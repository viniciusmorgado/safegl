use crate::model::Model;
use anyhow::{Context, Result};
use glam::{Mat3, Mat4, Vec3};
use safegl::{
    backend::Facade,
    framebuffer::{DepthRenderBuffer, SimpleFrameBuffer},
    index::{NoIndices, PrimitiveType},
    texture::{DepthFormat, MipmapsOption, UncompressedUintFormat, UnsignedTexture2d},
    uniform, Surface,
};

struct VisibilityBuffer {
    ids: UnsignedTexture2d,
    depth: DepthRenderBuffer,
    dimensions: (u32, u32),
}

impl VisibilityBuffer {
    fn new(display: &impl Facade, (width, height): (u32, u32)) -> Result<Self> {
        let buffer = Self {
            ids: UnsignedTexture2d::empty_with_format(
                display,
                UncompressedUintFormat::U32,
                MipmapsOption::NoMipmap,
                width,
                height,
            )?,
            depth: DepthRenderBuffer::new(display, DepthFormat::I24, width, height)?,
            dimensions: (width, height),
        };
        SimpleFrameBuffer::with_depth_buffer(display, &buffer.ids, &buffer.depth)?;
        println!("Visibility buffer: {width}x{height}, R32UI triangle/facing ID + depth24; no attribute G-buffer; single sample");
        Ok(buffer)
    }
}

pub struct VisibilityRenderer {
    visibility: safegl::Program,
    clear_ids: safegl::Program,
    shading: safegl::Program,
    buffer: Option<VisibilityBuffer>,
}

impl VisibilityRenderer {
    pub fn new(display: &impl Facade) -> Result<Self> {
        Ok(Self {
            visibility: safegl::Program::from_source(
                display,
                include_str!("../shaders/visibility.vert"),
                include_str!("../shaders/visibility.frag"),
                None,
            )
            .context("Compiling visibility shaders")?,
            clear_ids: safegl::Program::from_source(
                display,
                include_str!("../shaders/fullscreen.vert"),
                include_str!("../shaders/clear_ids.frag"),
                None,
            )?,
            shading: safegl::Program::new(
                display,
                safegl::program::ProgramCreationInput::SourceCode {
                    vertex_shader: include_str!("../shaders/fullscreen.vert"),
                    fragment_shader: include_str!("../shaders/shade.frag"),
                    geometry_shader: None,
                    tessellation_control_shader: None,
                    tessellation_evaluation_shader: None,
                    transform_feedback_varyings: None,
                    outputs_srgb: false,
                    uses_point_size: false,
                },
            )
            .context("Compiling visibility attribute reconstruction and shading")?,
            buffer: None,
        })
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
            multisampling: false,
            ..Default::default()
        };
        {
            let mut target =
                SimpleFrameBuffer::with_depth_buffer(display, &buffer.ids, &buffer.depth)?;
            target.clear_depth(1.0);
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
                target
                    .draw(
                        safegl::vertex::EmptyVertexAttributes {
                            len: mesh.vertex_count,
                        },
                        &mesh.indices,
                        &self.visibility,
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
                            multisampling: false,
                            ..Default::default()
                        },
                    )
                    .context("Rasterizing visibility IDs")?;
            }
        }
        output.clear_color(0.018, 0.024, 0.037, 1.0);
        let normal_matrix = Mat3::from_mat4(model.transform).inverse().transpose();
        for mesh in &model.meshes {
            let sampler = mesh
                .texture
                .sampled()
                .wrap_function(safegl::uniforms::SamplerWrapFunction::Repeat)
                .minify_filter(safegl::uniforms::MinifySamplerFilter::LinearMipmapLinear)
                .magnify_filter(safegl::uniforms::MagnifySamplerFilter::Linear);
            output
                .draw(
                    safegl::vertex::EmptyVertexAttributes { len: 3 },
                    NoIndices(PrimitiveType::TrianglesList),
                    &self.shading,
                    &uniform! {
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
                        light_direction: [0.5_f32, 0.8, 1.0],
                        light_color: [1.0_f32, 0.96, 0.90],
                    },
                    &fullscreen,
                )
                .context("Reconstructing attributes and shading visible triangles")?;
        }
        Ok(())
    }
}
