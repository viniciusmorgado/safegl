use crate::model::Model;
use anyhow::{Context, Result};
use glam::{Mat3, Mat4, Vec3};
use safegl::{
    backend::Facade,
    buffer::{Buffer, BufferMode, BufferType},
    framebuffer::SimpleFrameBuffer,
    index::{NoIndices, PrimitiveType},
    texture::{DepthFormat, DepthTexture2d, MipmapsOption, Texture2d, UncompressedFloatFormat},
    uniform, Surface,
};

pub const LIGHT_COUNT: usize = 256;
const TILE_SIZE: u32 = 32;
const TILE_STRIDE: usize = LIGHT_COUNT + 1;

struct Targets {
    depth: DepthTexture2d,
    color: Texture2d,
    lists: Buffer<[u32]>,
    dimensions: (u32, u32),
    tiles: (u32, u32),
}

pub struct ForwardPlus {
    depth_program: safegl::Program,
    cull: safegl::program::ComputeShader,
    shading: safegl::Program,
    present: safegl::Program,
    lights: Buffer<[[f32; 4]]>,
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

impl ForwardPlus {
    pub fn new(display: &impl Facade) -> Result<Self> {
        let mut lights = Vec::with_capacity(LIGHT_COUNT * 2);
        let palette = [
            [1.0, 0.38, 0.18],
            [0.22, 0.5, 1.0],
            [0.35, 1.0, 0.55],
            [1.0, 0.75, 0.35],
        ];
        // Fixed 8x8x4 world-space grid: identical scene on every run.
        for z in 0..4 {
            for y in 0..8 {
                for x in 0..8 {
                    lights.push([
                        -1.4 + x as f32 * 0.4,
                        -1.25 + y as f32 * (2.5 / 7.0),
                        -0.9 + z as f32 * 0.7,
                        0.55,
                    ]);
                    let color = palette[(x + y * 3 + z) % palette.len()];
                    lights.push([color[0], color[1], color[2], 1.3]);
                }
            }
        }
        Ok(Self {
            depth_program: safegl::Program::from_source(
                display,
                include_str!("../shaders/depth.vert"),
                include_str!("../shaders/depth.frag"),
                None,
            )?,
            cull: safegl::program::ComputeShader::from_source(
                display,
                include_str!("../shaders/cull.comp"),
            )
            .context("Compiling tiled light culling")?,
            shading: color_program(
                display,
                include_str!("../shaders/forward.vert"),
                include_str!("../shaders/forward.frag"),
            )?,
            present: color_program(
                display,
                include_str!("../shaders/present.vert"),
                include_str!("../shaders/present.frag"),
            )?,
            lights: Buffer::new(
                display,
                &lights[..],
                BufferType::ShaderStorageBuffer,
                BufferMode::Immutable,
            )?,
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
            let tiles = (width.div_ceil(TILE_SIZE), height.div_ceil(TILE_SIZE));
            self.targets = Some(Targets {
                depth: DepthTexture2d::empty_with_format(
                    display,
                    DepthFormat::I24,
                    MipmapsOption::NoMipmap,
                    width,
                    height,
                )?,
                color: Texture2d::empty_with_format(
                    display,
                    UncompressedFloatFormat::F16F16F16F16,
                    MipmapsOption::NoMipmap,
                    width,
                    height,
                )?,
                lists: Buffer::empty_array(
                    display,
                    BufferType::ShaderStorageBuffer,
                    tiles.0 as usize * tiles.1 as usize * TILE_STRIDE,
                    BufferMode::Dynamic,
                )?,
                dimensions,
                tiles,
            });
            println!(
                "Forward+: {width}x{height}, {}x{} tiles of 32x32 internal pixels (16x16 output); 256 point lights",
                tiles.0, tiles.1
            );
        }
        let targets = self.targets.as_ref().expect("Targets allocated above");
        let view_projection = projection * view;
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
        if !all_lights {
            self.cull.execute(
                uniform! {
                    scene_depth: &targets.depth,
                    inverse_projection: projection.inverse().to_cols_array_2d(),
                    view: view.to_cols_array_2d(),
                    viewport: [dimensions.0, dimensions.1],
                    tiles_x: targets.tiles.0,
                    LightData: &self.lights,
                    TileLists: &targets.lists,
                },
                targets.tiles.0,
                targets.tiles.1,
                1,
            );
        }
        {
            let mut color =
                SimpleFrameBuffer::with_depth_buffer(display, &targets.color, &targets.depth)?;
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
                            tiles_x: targets.tiles.0, all_lights: all_lights,
                            LightData: &self.lights, TileLists: &targets.lists,
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
                    .context("Drawing tiled Forward+ shading")?;
            }
        }
        output.draw(
            safegl::vertex::EmptyVertexAttributes { len: 3 },
            NoIndices(PrimitiveType::TrianglesList),
            &self.present,
            &uniform! { color_texture: &targets.color },
            &safegl::DrawParameters {
                multisampling: false,
                ..Default::default()
            },
        )?;
        Ok(())
    }

    pub fn tile_statistics(&self) -> Result<(f64, u32)> {
        let data = self
            .targets
            .as_ref()
            .context("No rendered frame")?
            .lists
            .read()?;
        let mut occupied = 0;
        let mut sum = 0;
        let mut maximum = 0;
        for tile in data.as_chunks::<TILE_STRIDE>().0 {
            let count = tile[0];
            anyhow::ensure!(count <= LIGHT_COUNT as u32, "Tile overflow");
            for &index in &tile[1..1 + count as usize] {
                anyhow::ensure!(index < LIGHT_COUNT as u32, "Invalid light index");
            }
            if count > 0 {
                occupied += 1;
                sum += count;
                maximum = maximum.max(count);
            }
        }
        Ok((sum as f64 / occupied.max(1) as f64, maximum))
    }
}

fn culling(double_sided: bool) -> safegl::BackfaceCullingMode {
    if double_sided {
        safegl::BackfaceCullingMode::CullingDisabled
    } else {
        safegl::BackfaceCullingMode::CullClockwise
    }
}
