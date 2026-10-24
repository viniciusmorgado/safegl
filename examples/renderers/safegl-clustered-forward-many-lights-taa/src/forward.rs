use crate::model::Model;
use anyhow::{Context, Result};
use glam::{Mat3, Mat4, Vec3};
use safegl::{
    backend::Facade,
    framebuffer::SimpleFrameBuffer,
    texture::{DepthFormat, DepthTexture2d, MipmapsOption},
    uniform, Surface,
};

struct Targets {
    depth: DepthTexture2d,
    dimensions: (u32, u32),
}

pub struct ClusteredForward {
    depth_program: safegl::Program,
    shading: safegl::Program,
    taa: crate::taa::Taa,
    clusters: crate::clusters::Clusters,
    targets: Option<Targets>,
}

fn color_program(display: &impl Facade, vertex: &str, fragment: &str) -> Result<safegl::Program> {
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

impl ClusteredForward {
    pub fn new(display: &impl Facade) -> Result<Self> {
        Ok(Self {
            depth_program: safegl::Program::from_source(
                display,
                include_str!("../shaders/depth.vert"),
                include_str!("../shaders/depth.frag"),
                None,
            )?,
            shading: color_program(
                display,
                include_str!("../shaders/forward.vert"),
                include_str!("../shaders/forward.frag"),
            )?,
            taa: crate::taa::Taa::new(display)?,
            clusters: crate::clusters::Clusters::new(display)?,
            targets: None,
        })
    }

    pub fn draw(
        &mut self,
        display: &impl Facade,
        model: &Model,
        camera: (Mat4, Mat4, Vec3),
        output: &mut impl Surface,
        all_lights: bool,
    ) -> Result<()> {
        let (projection, view, eye) = camera;
        let dimensions = output.get_dimensions();
        if self
            .targets
            .as_ref()
            .is_none_or(|t| t.dimensions != dimensions)
        {
            let (width, height) = dimensions;
            self.targets = Some(Targets {
                depth: DepthTexture2d::empty_with_format(
                    display,
                    DepthFormat::I24,
                    MipmapsOption::NoMipmap,
                    width,
                    height,
                )?,
                dimensions,
            });
            println!("Clustered Forward: {width}x{height}; single sample");
        }
        let targets = self.targets.as_ref().expect("Targets allocated above");
        let view_projection = self
            .taa
            .prepare(display, dimensions, projection * view, eye)?;
        let jittered_projection = view_projection * view.inverse();
        {
            let mut depth = SimpleFrameBuffer::depth_only(display, &targets.depth)?;
            depth.clear_depth(1.0);
            for mesh in &model.meshes {
                depth.draw(&mesh.vertices, &mesh.indices, &self.depth_program,
                    &uniform! {
                        model: model.transform.to_cols_array_2d(),
                        view_projection: view_projection.to_cols_array_2d(),
                        base_color_texture: mesh.texture.sampled()
                            .wrap_function(safegl::uniforms::SamplerWrapFunction::Repeat)
                            .minify_filter(safegl::uniforms::MinifySamplerFilter::LinearMipmapLinear)
                            .magnify_filter(safegl::uniforms::MagnifySamplerFilter::Linear),
                        base_color_factor: mesh.base_color,
                        alpha_mode: mesh.alpha_mode, alpha_cutoff: mesh.alpha_cutoff,
                    },
                    &safegl::DrawParameters {
                        depth: safegl::Depth { test: safegl::draw_parameters::DepthTest::IfLess, write: true, ..Default::default() },
                        backface_culling: culling(mesh.double_sided), multisampling: false, ..Default::default()
                    })?;
            }
        }
        self.clusters
            .update(display, dimensions, jittered_projection, view, all_lights)?;
        {
            let mut color = SimpleFrameBuffer::with_depth_buffer(
                display,
                self.taa.current_color(),
                &targets.depth,
            )?;
            color.clear_color(0.018, 0.024, 0.037, 1.0);
            let normal_matrix = Mat3::from_mat4(model.transform).inverse().transpose();
            for mesh in &model.meshes {
                let sampler = mesh
                    .texture
                    .sampled()
                    .wrap_function(safegl::uniforms::SamplerWrapFunction::Repeat)
                    .minify_filter(safegl::uniforms::MinifySamplerFilter::LinearMipmapLinear)
                    .magnify_filter(safegl::uniforms::MagnifySamplerFilter::Linear);
                color
                    .draw(
                        &mesh.vertices,
                        &mesh.indices,
                        &self.shading,
                        &uniform! {
                            model: model.transform.to_cols_array_2d(),
                            view_projection: view_projection.to_cols_array_2d(),
                            normal_matrix: normal_matrix.to_cols_array_2d(),
                            camera_position: eye.to_array(),
                            base_color_texture: sampler, base_color_factor: mesh.base_color, alpha_mode: mesh.alpha_mode, alpha_cutoff: mesh.alpha_cutoff,
                            grid: self.clusters.grid, near_far: [crate::clusters::NEAR, crate::clusters::FAR],
                            view: view.to_cols_array_2d(), all_lights: all_lights,
                            LightData: &self.clusters.lights, ClusterLists: &self.clusters.lists,
                        },
                        &safegl::DrawParameters {
                            depth: safegl::Depth {
                                test: safegl::draw_parameters::DepthTest::IfEqual,
                                write: false,
                                ..Default::default()
                            },
                            backface_culling: culling(mesh.double_sided),
                            multisampling: false,
                            ..Default::default()
                        },
                    )
                    .context("Drawing clustered Forward shading")?;
            }
        }
        self.taa.resolve(display, &targets.depth, output)?;
        Ok(())
    }

    pub fn reset_history(&mut self) {
        self.taa.reset();
    }
    pub fn validation_pixels(&self) -> (crate::taa::ColorPixels, crate::taa::ColorPixels) {
        self.taa.validation_pixels()
    }

    pub fn cluster_statistics(&self) -> Result<(f64, u32)> {
        self.clusters.statistics()
    }
}

fn culling(double_sided: bool) -> safegl::BackfaceCullingMode {
    if double_sided {
        safegl::BackfaceCullingMode::CullingDisabled
    } else {
        safegl::BackfaceCullingMode::CullClockwise
    }
}
