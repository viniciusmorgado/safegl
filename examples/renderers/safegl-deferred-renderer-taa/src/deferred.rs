use anyhow::{Context, Result};
use glam::{Mat3, Mat4, Vec3};
use safegl::{
    backend::Facade,
    framebuffer::{DepthRenderBuffer, MultiOutputFrameBuffer},
    index::{NoIndices, PrimitiveType},
    texture::{DepthFormat, MipmapsOption, Texture2d, UncompressedFloatFormat},
    uniform, Surface,
};

use crate::model::Model;

struct GBuffer {
    position: Texture2d,
    normal_specular: Texture2d,
    albedo: Texture2d,
    depth: DepthRenderBuffer,
    dimensions: (u32, u32),
}

impl GBuffer {
    fn new(display: &impl Facade, (width, height): (u32, u32)) -> Result<Self> {
        let texture = |format| {
            Texture2d::empty_with_format(display, format, MipmapsOption::NoMipmap, width, height)
        };
        let buffer = Self {
            position: texture(UncompressedFloatFormat::F16F16F16F16)?,
            normal_specular: texture(UncompressedFloatFormat::F16F16F16F16)?,
            albedo: texture(UncompressedFloatFormat::U8U8U8U8)?,
            depth: DepthRenderBuffer::new(display, DepthFormat::I24, width, height)?,
            dimensions: (width, height),
        };
        // Check MRT/depth compatibility immediately, including after resize.
        buffer.framebuffer(display)?;
        println!("G-buffer: {width}x{height}, position RGBA16F, normal/specular RGBA16F, albedo RGBA8, depth24; single sample");
        Ok(buffer)
    }

    fn framebuffer<'a>(&'a self, display: &impl Facade) -> Result<MultiOutputFrameBuffer<'a>> {
        Ok(MultiOutputFrameBuffer::with_depth_buffer(
            display,
            [
                ("g_position", &self.position),
                ("g_normal_specular", &self.normal_specular),
                ("g_albedo", &self.albedo),
            ],
            &self.depth,
        )?)
    }
}

pub struct DeferredRenderer {
    geometry: safegl::Program,
    lighting: safegl::Program,
    gbuffer: Option<GBuffer>,
    taa: crate::taa::Taa,
}

impl DeferredRenderer {
    pub fn new(display: &impl Facade) -> Result<Self> {
        Ok(Self {
            geometry: safegl::Program::from_source(
                display,
                include_str!("../shaders/geometry.vert"),
                include_str!("../shaders/geometry.frag"),
                None,
            )
            .context("Compiling G-buffer geometry shaders")?,
            lighting: safegl::Program::new(
                display,
                safegl::program::ProgramCreationInput::SourceCode {
                    vertex_shader: include_str!("../shaders/lighting.vert"),
                    fragment_shader: include_str!("../shaders/lighting.frag"),
                    geometry_shader: None,
                    tessellation_control_shader: None,
                    tessellation_evaluation_shader: None,
                    transform_feedback_varyings: None,
                    // Lighting writes linear RGB; enable framebuffer sRGB encoding.
                    outputs_srgb: false,
                    uses_point_size: false,
                },
            )
            .context("Compiling deferred lighting shaders")?,
            gbuffer: None,
            taa: crate::taa::Taa::new(display)?,
        })
    }

    pub fn validation_pixels(&self) -> (crate::taa::ColorPixels, crate::taa::ColorPixels) {
        self.taa.validation_pixels()
    }

    pub fn reset_history(&mut self) {
        self.taa.reset();
    }

    pub fn draw(
        &mut self,
        display: &impl Facade,
        model: &Model,
        view_projection: Mat4,
        eye: Vec3,
        output: &mut impl Surface,
    ) -> Result<()> {
        let dimensions = output.get_dimensions();
        if self
            .gbuffer
            .as_ref()
            .is_none_or(|g| g.dimensions != dimensions)
        {
            self.gbuffer = Some(GBuffer::new(display, dimensions).context("Allocating G-buffer")?);
        }
        let view_projection = self
            .taa
            .prepare(display, dimensions, view_projection, eye)?;
        let gbuffer = self.gbuffer.as_ref().expect("G-buffer allocated above");
        {
            let mut target = gbuffer.framebuffer(display)?;
            // Position alpha is a coverage flag; zero marks the background.
            target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
            let normal_matrix = Mat3::from_mat4(model.transform).inverse().transpose();
            for mesh in &model.meshes {
                let sampler = mesh
                    .texture
                    .sampled()
                    .wrap_function(safegl::uniforms::SamplerWrapFunction::Repeat)
                    .minify_filter(safegl::uniforms::MinifySamplerFilter::LinearMipmapLinear)
                    .magnify_filter(safegl::uniforms::MagnifySamplerFilter::Linear);
                let uniforms = uniform! {
                    model: model.transform.to_cols_array_2d(),
                    view_projection: view_projection.to_cols_array_2d(),
                    normal_matrix: normal_matrix.to_cols_array_2d(),
                    base_color_texture: sampler,
                    base_color_factor: mesh.base_color, alpha_mode: mesh.alpha_mode, alpha_cutoff: mesh.alpha_cutoff,
                };
                let parameters = safegl::DrawParameters {
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
                    multisampling: false,
                    ..Default::default()
                };
                target
                    .draw(
                        &mesh.vertices,
                        &mesh.indices,
                        &self.geometry,
                        &uniforms,
                        &parameters,
                    )
                    .context("Drawing G-buffer geometry pass")?;
            }
        }
        // Shade into linear HDR color before temporal filtering and presentation.
        let mut lit =
            safegl::framebuffer::SimpleFrameBuffer::new(display, self.taa.current_color())?;
        lit.draw(
            safegl::vertex::EmptyVertexAttributes { len: 3 },
            NoIndices(PrimitiveType::TrianglesList),
            &self.lighting,
            &uniform! {
                position_texture: &gbuffer.position,
                normal_specular_texture: &gbuffer.normal_specular,
                albedo_texture: &gbuffer.albedo,
                camera_position: eye.to_array(),
                light_direction: [0.5_f32, 0.8, 1.0],
                light_color: [1.0_f32, 0.96, 0.90],
            },
            &safegl::DrawParameters {
                multisampling: false,
                ..Default::default()
            },
        )
        .context("Drawing deferred lighting pass")?;
        drop(lit);
        self.taa.resolve(display, &gbuffer.position, output)?;
        Ok(())
    }
}
