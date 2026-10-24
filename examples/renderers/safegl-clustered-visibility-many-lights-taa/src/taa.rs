use anyhow::{Context, Result};
use glam::{Mat4, Vec3};
use safegl::{
    backend::Facade,
    framebuffer::SimpleFrameBuffer,
    index::{NoIndices, PrimitiveType},
    texture::{DepthTexture2d, MipmapsOption, Texture2d, UncompressedFloatFormat},
    uniform, Surface,
};

pub type ColorPixels = Vec<Vec<(u8, u8, u8, u8)>>;

struct Targets {
    current: Texture2d,
    history: [Texture2d; 2],
    dimensions: (u32, u32),
}

pub struct Taa {
    resolve: safegl::Program,
    present: safegl::Program,
    targets: Option<Targets>,
    read_index: usize,
    frame: u32,
    previous_vp: Mat4,
    current_vp: Mat4,
    previous_eye: Option<Vec3>,
}

impl Taa {
    pub fn new(display: &impl Facade) -> Result<Self> {
        Ok(Self {
            resolve: safegl::Program::from_source(
                display,
                include_str!("../shaders/fullscreen.vert"),
                include_str!("../shaders/taa.frag"),
                None,
            )
            .context("Compiling TAA resolve shader")?,
            present: safegl::Program::new(
                display,
                safegl::program::ProgramCreationInput::SourceCode {
                    vertex_shader: include_str!("../shaders/fullscreen.vert"),
                    fragment_shader: include_str!("../shaders/present.frag"),
                    geometry_shader: None,
                    tessellation_control_shader: None,
                    tessellation_evaluation_shader: None,
                    transform_feedback_varyings: None,
                    outputs_srgb: false,
                    uses_point_size: false,
                },
            )
            .context("Compiling TAA presentation shader")?,
            targets: None,
            read_index: 0,
            frame: 0,
            previous_vp: Mat4::IDENTITY,
            current_vp: Mat4::IDENTITY,
            previous_eye: None,
        })
    }

    pub fn validation_pixels(&self) -> (ColorPixels, ColorPixels) {
        let targets = self
            .targets
            .as_ref()
            .expect("TAA rendered before validation");
        (
            targets.current.read(),
            targets.history[self.read_index].read(),
        )
    }

    pub fn reset(&mut self) {
        self.frame = 0;
        self.previous_eye = None;
    }

    pub fn prepare(
        &mut self,
        display: &impl Facade,
        dimensions: (u32, u32),
        view_projection: Mat4,
        eye: Vec3,
    ) -> Result<Mat4> {
        if self
            .targets
            .as_ref()
            .is_none_or(|t| t.dimensions != dimensions)
        {
            let texture = || {
                Texture2d::empty_with_format(
                    display,
                    UncompressedFloatFormat::F16F16F16F16,
                    MipmapsOption::NoMipmap,
                    dimensions.0,
                    dimensions.1,
                )
            };
            let targets = Targets {
                current: texture()?,
                history: [texture()?, texture()?],
                dimensions,
            };
            // Initialize history even though the first resolve bypasses it.
            for history in &targets.history {
                SimpleFrameBuffer::new(display, history)?.clear_color(0.0, 0.0, 0.0, 0.0);
            }
            self.targets = Some(targets);
            self.reset();
            println!(
                "TAA: {}x{}, 16-sample Halton jitter; history reset",
                dimensions.0, dimensions.1
            );
        }
        // Large camera jumps are cuts, not motion that should retain history.
        if self
            .previous_eye
            .is_some_and(|previous| previous.distance(eye) > 0.5)
        {
            self.reset();
        }
        let sample = self.frame % 16 + 1;
        let jitter = Vec3::new(
            (halton(sample, 2) - 0.5) * 2.0 / dimensions.0 as f32,
            (halton(sample, 3) - 0.5) * 2.0 / dimensions.1 as f32,
            0.0,
        );
        self.current_vp = Mat4::from_translation(jitter) * view_projection;
        self.previous_eye = Some(eye);
        Ok(self.current_vp)
    }

    pub fn current_color(&self) -> &Texture2d {
        &self
            .targets
            .as_ref()
            .expect("TAA prepared before lighting")
            .current
    }

    pub fn resolve(
        &mut self,
        display: &impl Facade,
        depth: &DepthTexture2d,
        output: &mut impl Surface,
    ) -> Result<()> {
        let targets = self.targets.as_ref().expect("TAA prepared before resolve");
        let write_index = 1 - self.read_index;
        let parameters = safegl::DrawParameters {
            multisampling: false,
            ..Default::default()
        };
        {
            let mut target = SimpleFrameBuffer::new(display, &targets.history[write_index])?;
            target
                .draw(
                    safegl::vertex::EmptyVertexAttributes { len: 3 },
                    NoIndices(PrimitiveType::TrianglesList),
                    &self.resolve,
                    &uniform! {
                        current_color: &targets.current,
                        scene_depth: depth,
                        inverse_current_vp: self.current_vp.inverse().to_cols_array_2d(),
                        history_color_depth: &targets.history[self.read_index],
                        current_vp: self.current_vp.to_cols_array_2d(),
                        previous_vp: self.previous_vp.to_cols_array_2d(),
                        history_valid: self.frame > 0,
                        history_weight: (self.frame as f32 / (self.frame as f32 + 1.0)).min(0.9),
                    },
                    &parameters,
                )
                .context("Resolving TAA history")?;
        }
        output
            .draw(
                safegl::vertex::EmptyVertexAttributes { len: 3 },
                NoIndices(PrimitiveType::TrianglesList),
                &self.present,
                &uniform! { resolved_color: &targets.history[write_index] },
                &parameters,
            )
            .context("Presenting TAA result")?;
        self.read_index = write_index;
        self.previous_vp = self.current_vp;
        self.frame = self.frame.wrapping_add(1);
        Ok(())
    }
}

fn halton(mut index: u32, base: u32) -> f32 {
    let mut fraction = 1.0;
    let mut result = 0.0;
    while index > 0 {
        fraction /= base as f32;
        result += fraction * (index % base) as f32;
        index /= base;
    }
    result
}
