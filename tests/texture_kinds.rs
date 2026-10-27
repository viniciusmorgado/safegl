//! Kinds of textures and formats: compressed formats, rectangle textures, texture views, ranges
//! of buffer textures, multisample textures and images.
// the helpers are unused at the levels that have none of the tests that need them
#![allow(dead_code)]

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::Texture2d;
use safegl::uniforms::Uniforms;
use safegl::{DrawError, Program, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;
type Pixel = (u8, u8, u8, u8);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A program whose fragment shader declares `declarations` and runs `body`, which writes
/// `color`, compiled as GLSL `desktop` (330, 430, ...) or GLSL ES `es` (300, 310, ...).
fn program(display: &Display, desktop: u32, es: u32, declarations: &str, body: &str) -> Program {
    let header = if support::is_gles(display) {
        format!(
            "#version {es} es\nprecision highp float;\nprecision highp int;\n\
             precision highp usampler2D;\nprecision highp isampler2D;\n"
        )
    } else {
        format!("#version {desktop} core\n")
    };
    let vertex = format!(
        "{header}in vec2 position;\nout vec2 uv;\n\
         void main() {{ uv = position * 0.5 + 0.5; gl_Position = vec4(position, 0.0, 1.0); }}"
    );
    let fragment =
        format!("{header}in vec2 uv;\n{declarations}\nout vec4 color;\nvoid main() {{ {body} }}");
    Program::from_source(display, &vertex, &fragment, None).unwrap()
}

/// Draws a fullscreen triangle with `program` into a target of 4 × 4 pixels and returns its
/// pixels.
fn draw(
    display: &Display,
    program: &Program,
    uniforms: &impl Uniforms,
) -> Result<Vec<Vec<Pixel>>, DrawError> {
    let target = Texture2d::empty(display, 4, 4).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let triangle =
        VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap();
    target.as_surface().draw(
        &triangle,
        NoIndices(PrimitiveType::TrianglesList),
        program,
        uniforms,
        &Default::default(),
    )?;
    Ok(target.read().unwrap())
}

/// An ETC2 block of 4 × 4 texels of a single color: the individual mode with the base colors
/// 8, 4 and 2 in both halves (`0x88`, `0x44` and `0x22` once extended to 8 bits), the modifier
/// table 0 and every texel at index 0, which adds 2.
const ETC2_BLOCK: [u8; 8] = [0x88, 0x44, 0x22, 0x00, 0, 0, 0, 0];
const ETC2_COLOR: Pixel = (0x88 + 2, 0x44 + 2, 0x22 + 2, 255);

#[cfg(any(safegl_gl43, safegl_gles30))]
#[test]
fn etc2_textures() {
    use safegl::texture::{CompressedFormat, CompressedMipmapsOption, CompressedTexture2d};

    let display = support::build_display();
    if !CompressedFormat::Etc2Rgb8.is_supported(&display) {
        return;
    }
    let texture = CompressedTexture2d::with_compressed_data(
        &display,
        &ETC2_BLOCK,
        4,
        4,
        CompressedFormat::Etc2Rgb8,
        CompressedMipmapsOption::NoMipmap,
    )
    .unwrap();
    let program = program(
        &display,
        330,
        300,
        "uniform sampler2D tex;",
        "color = texture(tex, uv);",
    );
    let pixels = draw(&display, &program, &uniform! { tex: &texture }).unwrap();
    assert!(
        pixels.iter().flatten().all(|pixel| *pixel == ETC2_COLOR),
        "{:?}",
        pixels[0][0]
    );

    // a block of 8 bytes for each 4 × 4 texels: a 6 × 6 texture has 4 blocks
    let result = CompressedTexture2d::with_compressed_data(
        &display,
        &[0u8; 4 * 8],
        6,
        6,
        CompressedFormat::Etc2Rgb8,
        CompressedMipmapsOption::NoMipmap,
    );
    assert_eq!(result.unwrap().get_width(), 6);
    let result = CompressedTexture2d::with_compressed_data(
        &display,
        &[0u8; 3 * 8],
        6,
        6,
        CompressedFormat::Etc2Rgb8,
        CompressedMipmapsOption::NoMipmap,
    );
    assert!(result.is_err());

    // EAC with two channels takes 16 bytes per block
    let texture = CompressedTexture2d::with_compressed_data(
        &display,
        &[0u8; 16],
        4,
        4,
        CompressedFormat::EacRg11Unsigned,
        CompressedMipmapsOption::NoMipmap,
    );
    assert!(texture.is_ok());

    display.assert_no_error(None);
}

#[cfg(any(safegl_gl43, safegl_gles30))]
#[test]
fn etc2_srgb_textures() {
    use safegl::texture::{CompressedMipmapsOption, CompressedSrgbFormat, CompressedSrgbTexture2d};

    let display = support::build_display();
    if !CompressedSrgbFormat::Etc2Rgb8.is_supported(&display) {
        return;
    }
    let texture = CompressedSrgbTexture2d::with_compressed_data(
        &display,
        &ETC2_BLOCK,
        4,
        4,
        CompressedSrgbFormat::Etc2Rgb8,
        CompressedMipmapsOption::NoMipmap,
    )
    .unwrap();
    let program = program(
        &display,
        330,
        300,
        "uniform sampler2D tex;",
        "color = texture(tex, uv);",
    );
    // sampling converts the sRGB colors to linear ones, which are darker
    let pixel = draw(&display, &program, &uniform! { tex: &texture }).unwrap()[0][0];
    assert!(
        pixel.0 < ETC2_COLOR.0 && pixel.1 < ETC2_COLOR.1,
        "{pixel:?}"
    );

    display.assert_no_error(None);
}

/// Multisample textures whose samples may be placed differently in each texel.
#[cfg(any(safegl_desktop, safegl_gles31))]
#[test]
fn variable_sample_locations() {
    use safegl::framebuffer::{DepthRenderBuffer, SimpleFrameBuffer, ValidationError};
    use safegl::glow::HasContext;
    use safegl::texture::{DepthFormat, Texture2dMultisample, UncompressedFloatFormat};
    use safegl::GlObject;

    let display = support::build_display();
    if !safegl::texture::is_texture_2d_multisample_supported(&display) {
        return;
    }
    let texture = Texture2dMultisample::empty_with_sample_locations(
        &display,
        UncompressedFloatFormat::U8U8U8U8,
        4,
        4,
        4,
        false,
    )
    .unwrap();
    assert!(!texture.has_fixed_sample_locations());
    let fixed = unsafe {
        let gl = display.glow_context();
        gl.bind_texture(safegl::glow::TEXTURE_2D_MULTISAMPLE, Some(texture.get_id()));
        let fixed = gl.get_tex_level_parameter_i32(
            safegl::glow::TEXTURE_2D_MULTISAMPLE,
            0,
            safegl::glow::TEXTURE_FIXED_SAMPLE_LOCATIONS,
        );
        gl.bind_texture(safegl::glow::TEXTURE_2D_MULTISAMPLE, None);
        fixed
    };
    display.restore_state();
    assert_eq!(fixed, 0);

    let mut framebuffer = SimpleFrameBuffer::new(&display, &texture).unwrap();
    framebuffer.clear_color(1.0, 0.0, 0.0, 1.0);

    // render buffers have fixed sample locations
    let depth = DepthRenderBuffer::new_multisample(&display, DepthFormat::I24, 4, 4, 4).unwrap();
    assert!(matches!(
        SimpleFrameBuffer::with_depth_buffer(&display, &texture, &depth),
        Err(ValidationError::Incomplete { .. })
    ));

    display.assert_no_error(None);
}
