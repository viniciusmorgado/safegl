use anyhow::{ensure, Result};
use safegl::{
    backend::Facade,
    framebuffer::SimpleFrameBuffer,
    index::{NoIndices, PrimitiveType},
    texture::{MipmapsOption, Texture2d, UncompressedFloatFormat},
    uniform, Surface,
};
pub struct Ssaa {
    program: safegl::Program,
    color: Option<Texture2d>,
    dimensions: (u32, u32),
}
impl Ssaa {
    pub fn new(display: &impl Facade) -> Result<Self> {
        Ok(Self {
            program: safegl::Program::new(
                display,
                safegl::program::ProgramCreationInput::SourceCode {
                    vertex_shader: include_str!("../shaders/ssaa.vert"),
                    fragment_shader: include_str!("../shaders/ssaa.frag"),
                    geometry_shader: None,
                    tessellation_control_shader: None,
                    tessellation_evaluation_shader: None,
                    transform_feedback_varyings: None,
                    outputs_srgb: false,
                    uses_point_size: false,
                },
            )?,
            color: None,
            dimensions: (0, 0),
        })
    }
    pub fn prepare(&mut self, display: &impl Facade, dimensions: (u32, u32)) -> Result<()> {
        if self.dimensions != dimensions {
            let width = dimensions
                .0
                .checked_mul(2)
                .ok_or_else(|| anyhow::anyhow!("SSAA width overflow"))?;
            let height = dimensions
                .1
                .checked_mul(2)
                .ok_or_else(|| anyhow::anyhow!("SSAA height overflow"))?;
            self.color = Some(Texture2d::empty_with_format(
                display,
                UncompressedFloatFormat::F16F16F16F16,
                MipmapsOption::NoMipmap,
                width,
                height,
            )?);
            self.dimensions = dimensions;
            println!(
                "SSAA 4x: {}x{} output, {width}x{height} shading, linear 2x2 box resolve",
                dimensions.0, dimensions.1
            );
        }
        Ok(())
    }
    pub fn color(&self) -> &Texture2d {
        self.color.as_ref().expect("SSAA prepared")
    }
    pub fn resolve(&self, output: &mut impl Surface) -> Result<()> {
        ensure!(
            output.get_dimensions() == self.dimensions,
            "SSAA output dimensions changed during frame"
        );
        output.draw(
            safegl::vertex::EmptyVertexAttributes { len: 3 },
            NoIndices(PrimitiveType::TrianglesList),
            &self.program,
            &uniform! {source_color: self.color()},
            &safegl::DrawParameters {
                multisampling: false,
                ..Default::default()
            },
        )?;
        Ok(())
    }
}
pub fn validate(display: &impl Facade) -> Result<()> {
    let mut ssaa = Ssaa::new(display)?;
    ssaa.prepare(display, (1, 1))?;
    ensure!(
        ssaa.color().dimensions() == (2, 2),
        "SSAA must double both dimensions"
    );
    ssaa.color().write(
        safegl::Rect {
            left: 0,
            bottom: 0,
            width: 2,
            height: 2,
        },
        safegl::texture::RawImage2d::from_raw_rgba(
            vec![
                1.0_f32, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0,
            ],
            (2, 2),
        ),
    );
    let output = Texture2d::empty_with_format(
        display,
        UncompressedFloatFormat::F16F16F16F16,
        MipmapsOption::NoMipmap,
        1,
        1,
    )?;
    ssaa.resolve(&mut SimpleFrameBuffer::new(display, &output)?)?;
    let pixels: Vec<Vec<(u8, u8, u8, u8)>> = output.read();
    let p = pixels[0][0];
    ensure!(
        [p.0, p.1, p.2].into_iter().all(|v| v.abs_diff(64) <= 1),
        "SSAA must average all four samples in linear space"
    );
    println!("SSAA GPU resolve validation passed");
    Ok(())
}
