use crate::model::Model;
use anyhow::{Context, Result};
use glam::{Mat3, Mat4, Vec3};
use safegl::{
    backend::Facade,
    draw_parameters::{SamplesPassedQuery, Stencil, StencilOperation, StencilTest},
    framebuffer::{DepthStencilRenderBuffer, SimpleFrameBuffer},
    index::{NoIndices, PrimitiveType},
    texture::{
        DepthStencilFormat, MipmapsOption, Texture2dMultisample, UncompressedFloatFormat,
        UncompressedUintFormat, UnsignedTexture2dMultisample,
    },
    uniform, Surface,
};

const SAMPLES: u32 = 4;
/// Blinn–Phong with shininess 32 falls to half intensity when the normal is
/// acos(0.5^(1/32)) ≈ 0.21 rad away from the half vector. A normal that turns more
/// than a quarter of that per pixel squeezes the highlight into a few pixels.
const SPECULAR_THRESHOLD: f32 = 0.25 * 0.2077;

/// Which pixels pay for per-sample shading; every other pixel is shaded once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Geometric edges and alpha-coverage materials: the classic MSAA + SSAA hybrid.
    Edges,
    /// Edges, plus triangle interiors whose specular highlight would alias.
    EdgesSpecular,
    /// Every pixel, like the plain MSAA 4x visibility renderer (sparse-grid SSAA).
    Full,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Edges, Mode::EdgesSpecular, Mode::Full];

    pub fn name(self) -> &'static str {
        match self {
            Mode::Edges => "edges",
            Mode::EdgesSpecular => "edges-specular",
            Mode::Full => "full",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }

    pub fn next(self) -> Self {
        match self {
            Mode::Edges => Mode::EdgesSpecular,
            Mode::EdgesSpecular => Mode::Full,
            Mode::Full => Mode::Edges,
        }
    }
}

struct VisibilityBuffer {
    ids: UnsignedTexture2dMultisample,
    color: Texture2dMultisample,
    // Depth for rasterizing IDs; stencil carries the per-pixel/per-sample decision.
    depth_stencil: DepthStencilRenderBuffer,
    dimensions: (u32, u32),
}

impl VisibilityBuffer {
    fn new(display: &impl Facade, (width, height): (u32, u32)) -> Result<Self> {
        let buffer = Self {
            ids: UnsignedTexture2dMultisample::empty_with_format(
                display,
                UncompressedUintFormat::U32,
                MipmapsOption::NoMipmap,
                width,
                height,
                SAMPLES,
            )?,
            depth_stencil: DepthStencilRenderBuffer::new_multisample(
                display,
                DepthStencilFormat::I24I8,
                width,
                height,
                SAMPLES,
            )?,
            color: Texture2dMultisample::empty_with_format(
                display,
                UncompressedFloatFormat::F16F16F16F16,
                MipmapsOption::NoMipmap,
                width,
                height,
                SAMPLES,
            )?,
            dimensions: (width, height),
        };
        SimpleFrameBuffer::with_depth_stencil_buffer(display, &buffer.ids, &buffer.depth_stencil)?;
        println!("Visibility buffer: {width}x{height}, R32UI triangle/facing/alpha ID + depth24/stencil8; no attribute G-buffer; 4 samples; stencil-classified hybrid shading into RGBA16F");
        Ok(buffer)
    }
}

/// Fraction of window pixels shaded per sample, measured without stalling the GPU.
struct SupersampleCounter {
    pending: Option<(SamplesPassedQuery, u64, Mode)>,
    latest: Option<(f64, Mode)>,
}

pub struct VisibilityRenderer {
    visibility: safegl::Program,
    visibility_per_sample: safegl::Program,
    clear_ids: safegl::Program,
    classify: safegl::Program,
    classify_specular: safegl::Program,
    shade_pixel: safegl::Program,
    shade_sample: safegl::Program,
    present: safegl::Program,
    buffer: Option<VisibilityBuffer>,
    clusters: crate::clusters::Clusters,
    counter: SupersampleCounter,
}

fn program(display: &impl Facade, vertex: &str, fragment: &str) -> Result<safegl::Program> {
    Ok(safegl::Program::new(
        display,
        safegl::program::ProgramCreationInput::SourceCode {
            vertex_shader: vertex,
            fragment_shader: fragment,
            geometry_shader: None,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            transform_feedback_varyings: None,
            outputs_srgb: false,
            uses_point_size: false,
        },
    )?)
}

/// Prepends the version line and one preprocessor switch to a shared source.
fn variant(define: &str, source: &str) -> String {
    format!("#version 460 core\n#define {define}\n{source}")
}

/// Stencil state applied identically to both facings of fullscreen triangles. The
/// reference is always 1: stencil 1 means "shade per sample", 0 "shade once".
fn stencil(test: StencilTest, pass: StencilOperation) -> Stencil {
    Stencil {
        test_clockwise: test,
        reference_value_clockwise: 1,
        depth_pass_operation_clockwise: pass,
        test_counter_clockwise: test,
        reference_value_counter_clockwise: 1,
        depth_pass_operation_counter_clockwise: pass,
        ..Default::default()
    }
}

impl VisibilityRenderer {
    pub fn new(display: &impl Facade) -> Result<Self> {
        let fullscreen = include_str!("../shaders/fullscreen.vert");
        let shade = include_str!("../shaders/shade.glsl");
        let visibility_vertex = include_str!("../shaders/visibility.vert");
        let visibility_fragment = include_str!("../shaders/visibility.frag");
        Ok(Self {
            visibility: program(
                display,
                &variant("PIXEL_COVERAGE", visibility_vertex),
                &variant("PIXEL_COVERAGE", visibility_fragment),
            )
            .context("Compiling visibility shaders")?,
            visibility_per_sample: program(
                display,
                &variant("SAMPLE_COVERAGE", visibility_vertex),
                &variant("SAMPLE_COVERAGE", visibility_fragment),
            )
            .context("Compiling per-sample alpha-coverage visibility shaders")?,
            clear_ids: program(
                display,
                fullscreen,
                include_str!("../shaders/clear_ids.frag"),
            )?,
            classify: program(
                display,
                fullscreen,
                include_str!("../shaders/classify.frag"),
            )
            .context("Compiling edge classification")?,
            classify_specular: program(
                display,
                fullscreen,
                &variant("SHADING_RATE_CLASSIFY", shade),
            )
            .context("Compiling specular classification")?,
            shade_pixel: program(display, fullscreen, &variant("SHADING_RATE_PIXEL", shade))
                .context("Compiling per-pixel attribute reconstruction and shading")?,
            shade_sample: program(display, fullscreen, &variant("SHADING_RATE_SAMPLE", shade))
                .context("Compiling per-sample attribute reconstruction and shading")?,
            present: program(display, fullscreen, include_str!("../shaders/present.frag"))?,
            buffer: None,
            clusters: crate::clusters::Clusters::new(display)?,
            counter: SupersampleCounter {
                pending: None,
                latest: None,
            },
        })
    }

    pub fn cluster_statistics(&self) -> Result<(f64, u32)> {
        self.clusters.statistics()
    }

    /// Latest completed measurement of the share of pixels shaded per sample.
    pub fn supersampled_fraction(&self) -> Option<(f64, Mode)> {
        self.counter.latest
    }

    /// Blocks until the measurement for the frame just drawn is available.
    pub fn wait_supersampled_fraction(&mut self) -> Option<(f64, Mode)> {
        let (query, pixels, mode) = self.counter.pending.take()?;
        let fraction = f64::from(query.get()) / (pixels * u64::from(SAMPLES)) as f64;
        self.counter.latest = Some((fraction, mode));
        self.counter.latest
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        display: &impl Facade,
        model: &Model,
        camera: (Mat4, Mat4, Vec3),
        output: &mut impl Surface,
        all_lights: bool,
        mode: Mode,
        highlight_samples: bool,
    ) -> Result<()> {
        let (projection, view, eye) = camera;
        let view_projection = projection * view;
        let dimensions = output.get_dimensions();
        self.clusters
            .update(display, dimensions, projection, view, all_lights)?;
        if self
            .buffer
            .as_ref()
            .is_none_or(|b| b.dimensions != dimensions)
        {
            self.buffer = Some(VisibilityBuffer::new(display, dimensions)?);
        }
        let buffer = self
            .buffer
            .as_ref()
            .expect("Visibility buffer allocated above");
        let fullscreen = safegl::DrawParameters {
            multisampling: true,
            ..Default::default()
        };
        {
            let mut target = SimpleFrameBuffer::with_depth_stencil_buffer(
                display,
                &buffer.ids,
                &buffer.depth_stencil,
            )?;
            target.clear_depth_and_stencil(1.0, 0);
            // Integer color attachments require an integer clear. A uint-output
            // fullscreen shader avoids safegl's floating-point color-clear path.
            target.draw(
                safegl::vertex::EmptyVertexAttributes { len: 3 },
                NoIndices(PrimitiveType::TrianglesList),
                &self.clear_ids,
                &safegl::uniforms::EmptyUniforms,
                &fullscreen,
            )?;
            for mesh in &model.meshes {
                let program = if mesh.alpha_mode == 0 {
                    &self.visibility
                } else {
                    &self.visibility_per_sample
                };
                target
                    .draw(
                        safegl::vertex::EmptyVertexAttributes {
                            len: mesh.vertex_count,
                        },
                        &mesh.indices,
                        program,
                        &uniform! {
                            attributes: &mesh.attributes,
                            base_color_texture: mesh.texture.sampled()
                                .wrap_function(safegl::uniforms::SamplerWrapFunction::Repeat)
                                .minify_filter(safegl::uniforms::MinifySamplerFilter::LinearMipmapLinear)
                                .magnify_filter(safegl::uniforms::MagnifySamplerFilter::Linear),
                            base_color_factor: mesh.base_color,
                            alpha_mode: mesh.alpha_mode, alpha_cutoff: mesh.alpha_cutoff,
                            model: model.transform.to_cols_array_2d(),
                            view_projection: view_projection.to_cols_array_2d(),
                            triangle_base: mesh.triangle_base,
                        },
                        &safegl::DrawParameters {
                            depth: safegl::Depth {
                                test: safegl::draw_parameters::DepthTest::IfLess,
                                write: true,
                                ..Default::default()
                            },
                            backface_culling: if mesh.double_sided {
                                safegl::BackfaceCullingMode::CullingDisabled
                            } else {
                                safegl::BackfaceCullingMode::CullClockwise
                            },
                            multisampling: true,
                            ..Default::default()
                        },
                    )
                    .context("Rasterizing visibility IDs")?;
            }
        }
        // Read the previous measurement once the GPU has it, then start a new one.
        if let Some((query, pixels, measured)) = self.counter.pending.take() {
            if query.is_ready() {
                let fraction = f64::from(query.get()) / (pixels * u64::from(SAMPLES)) as f64;
                self.counter.latest = Some((fraction, measured));
            } else {
                self.counter.pending = Some((query, pixels, measured));
            }
        }
        let query = if mode == Mode::Full {
            // Nothing is classified; every pixel is shaded per sample.
            self.counter.pending = None;
            self.counter.latest = Some((1.0, mode));
            None
        } else if self.counter.pending.is_none() {
            Some(SamplesPassedQuery::new(display)?)
        } else {
            None
        };
        let mut shaded = SimpleFrameBuffer::with_depth_stencil_buffer(
            display,
            &buffer.color,
            &buffer.depth_stencil,
        )?;
        shaded.clear_color(0.018, 0.024, 0.037, 1.0);
        let normal_matrix = Mat3::from_mat4(model.transform).inverse().transpose();
        let uniforms: Vec<_> = model
            .meshes
            .iter()
            .map(|mesh| {
                let sampler = mesh
                    .texture
                    .sampled()
                    .wrap_function(safegl::uniforms::SamplerWrapFunction::Repeat)
                    .minify_filter(safegl::uniforms::MinifySamplerFilter::LinearMipmapLinear)
                    .magnify_filter(safegl::uniforms::MagnifySamplerFilter::Linear);
                uniform! {
                    visibility_texture: &buffer.ids,
                    index_lookup: &mesh.index_lookup,
                    attributes: &mesh.attributes,
                    base_color_texture: sampler,
                    triangle_base: mesh.triangle_base,
                    triangle_count: mesh.triangle_count,
                    model: model.transform.to_cols_array_2d(),
                    view_projection: view_projection.to_cols_array_2d(),
                    normal_matrix: normal_matrix.to_cols_array_2d(),
                    base_color_factor: mesh.base_color, alpha_mode: mesh.alpha_mode, alpha_cutoff: mesh.alpha_cutoff,
                    camera_position: eye.to_array(),
                    view: view.to_cols_array_2d(),
                    grid: self.clusters.grid,
                    near_far: [crate::clusters::NEAR, crate::clusters::FAR],
                    all_lights: all_lights,
                    highlight_samples: highlight_samples,
                    specular_threshold: SPECULAR_THRESHOLD,
                    LightData: &self.clusters.lights,
                    ClusterLists: &self.clusters.lists,
                }
            })
            .collect();
        if mode != Mode::Full {
            // Marks only pixels not marked yet, so each kept pixel adds its four
            // samples to the query exactly once.
            let mark = safegl::DrawParameters {
                color_mask: (false, false, false, false),
                stencil: stencil(
                    StencilTest::IfNotEqual { mask: 0xff },
                    StencilOperation::Replace,
                ),
                samples_passed_query: query.as_ref().map(Into::into),
                ..fullscreen.clone()
            };
            // Classification finishes for every mesh before any shading reads it.
            for (mesh, uniforms) in model.meshes.iter().zip(&uniforms) {
                shaded
                    .draw(
                        safegl::vertex::EmptyVertexAttributes { len: 3 },
                        NoIndices(PrimitiveType::TrianglesList),
                        &self.classify,
                        &uniform! {
                            visibility_texture: &buffer.ids,
                            index_lookup: &mesh.index_lookup,
                            triangle_base: mesh.triangle_base,
                            triangle_count: mesh.triangle_count,
                        },
                        &mark,
                    )
                    .context("Classifying edge and alpha-coverage pixels")?;
                if mode == Mode::EdgesSpecular {
                    shaded
                        .draw(
                            safegl::vertex::EmptyVertexAttributes { len: 3 },
                            NoIndices(PrimitiveType::TrianglesList),
                            &self.classify_specular,
                            uniforms,
                            &mark,
                        )
                        .context("Classifying specular-aliasing pixels")?;
                }
            }
        }
        let (single, per_sample) = if mode == Mode::Full {
            (None, StencilTest::AlwaysPass)
        } else {
            (
                Some(StencilTest::IfNotEqual { mask: 0xff }),
                StencilTest::IfEqual { mask: 0xff },
            )
        };
        for uniforms in &uniforms {
            if let Some(test) = single {
                shaded
                    .draw(
                        safegl::vertex::EmptyVertexAttributes { len: 3 },
                        NoIndices(PrimitiveType::TrianglesList),
                        &self.shade_pixel,
                        uniforms,
                        &safegl::DrawParameters {
                            stencil: stencil(test, StencilOperation::Keep),
                            ..fullscreen.clone()
                        },
                    )
                    .context("Shading continuous pixels once")?;
            }
            shaded
                .draw(
                    safegl::vertex::EmptyVertexAttributes { len: 3 },
                    NoIndices(PrimitiveType::TrianglesList),
                    &self.shade_sample,
                    uniforms,
                    &safegl::DrawParameters {
                        stencil: stencil(per_sample, StencilOperation::Keep),
                        ..fullscreen.clone()
                    },
                )
                .context("Shading supersampled pixels per sample")?;
        }
        drop(shaded);
        if let Some(query) = query {
            let pixels = u64::from(dimensions.0) * u64::from(dimensions.1);
            self.counter.pending = Some((query, pixels, mode));
        }
        output
            .draw(
                safegl::vertex::EmptyVertexAttributes { len: 3 },
                NoIndices(PrimitiveType::TrianglesList),
                &self.present,
                &uniform! { color_texture: &buffer.color },
                &safegl::DrawParameters {
                    multisampling: false,
                    ..Default::default()
                },
            )
            .context("Resolving four linear shaded samples")?;
        Ok(())
    }
}
