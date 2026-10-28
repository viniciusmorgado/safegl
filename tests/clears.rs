//! Clears of single attachments: images of textures of every kind, and the outputs of
//! multi-output framebuffers.

#[macro_use]
extern crate safegl;

use safegl::framebuffer::{ClearBufferData, ClearBufferError, MultiOutputFrameBuffer};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::{DepthTexture2d, Texture2d, UnsignedTexture2d};
use safegl::{Program, Surface, VertexBuffer};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Display = support::Backend;
type Pixel = (u8, u8, u8, u8);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// Returns the color of a pixel of a target that a fragment shader filled with `body`.
fn sample(
    display: &Display,
    declarations: &str,
    body: &str,
    uniforms: &impl safegl::uniforms::Uniforms,
) -> Pixel {
    let header = if support::is_gles(display) {
        "#version 300 es\nprecision highp float;\nprecision highp usampler2D;\n"
    } else {
        "#version 330 core\n"
    };
    let program = Program::from_source(
        display,
        &format!(
            "{header}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}"
        ),
        &format!("{header}{declarations}\nout vec4 color;\nvoid main() {{ {body} }}"),
        None,
    )
    .unwrap();
    let target = Texture2d::empty(display, 2, 2).unwrap();
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let triangle =
        VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap();
    target
        .as_surface()
        .draw(
            &triangle,
            NoIndices(PrimitiveType::TrianglesList),
            &program,
            uniforms,
            &Default::default(),
        )
        .unwrap();
    let data: Vec<Vec<Pixel>> = target.read().unwrap();
    data[0][0]
}

#[test]
fn clear_images_of_every_kind() {
    let display = support::build_display();

    // integers, which `clear_color` can only give as converted floats
    let unsigned = UnsignedTexture2d::empty(&display, 2, 2).unwrap();
    let image = unsigned
        .main_level()
        .first_layer()
        .into_image(None)
        .unwrap();
    image.raw_clear_buffer([300u32, 7, 0, 1]).unwrap();
    let pixel = sample(
        &display,
        "uniform usampler2D tex;",
        "uvec4 v = texelFetch(tex, ivec2(0), 0);\n\
         color = vec4(float(v.r - 293u), float(v.g), 0.0, 1.0) / 255.0;",
        &uniform! { tex: &unsigned },
    );
    assert_eq!(pixel, (7, 7, 0, 1));

    // a depth, which the scissor box and the depth mask of earlier draws don't limit
    let depth = DepthTexture2d::empty(&display, 2, 2).unwrap();
    let image = depth.main_level().first_layer().into_image(None).unwrap();
    image.raw_clear_buffer(0.25).unwrap();
    let pixel = sample(
        &display,
        "uniform sampler2D tex;",
        "color = vec4(texelFetch(tex, ivec2(1), 0).r, 0.0, 0.0, 1.0);",
        &uniform! { tex: &depth },
    );
    assert!((63..=65).contains(&pixel.0), "{pixel:?}");

    // values of another kind are refused instead of making OpenGL errors
    assert_eq!(
        image.raw_clear_buffer([1.0, 0.0, 0.0, 1.0]),
        Err(ClearBufferError::KindMismatch)
    );
    let color = Texture2d::empty(&display, 2, 2).unwrap();
    let image = color.main_level().first_layer().into_image(None).unwrap();
    assert_eq!(
        image.raw_clear_buffer(ClearBufferData::Stencil(1)),
        Err(ClearBufferError::KindMismatch)
    );

    display.assert_no_error(None);
}

/// Stencil and depth-stencil textures are cleared with their stencil index.
#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn clear_depth_stencil_images() {
    use safegl::texture::{
        DepthStencilFormat, DepthStencilTexture2d, DepthStencilTextureMode, MipmapsOption,
    };

    let display = support::build_display();
    let texture = DepthStencilTexture2d::empty_with_format(
        &display,
        DepthStencilFormat::I24I8,
        MipmapsOption::NoMipmap,
        2,
        2,
    )
    .unwrap();
    let image = texture.main_level().first_layer().into_image(None).unwrap();
    image
        .raw_clear_buffer(ClearBufferData::DepthStencil(0.5, 9))
        .unwrap();
    if texture
        .set_depth_stencil_mode(DepthStencilTextureMode::Stencil)
        .is_err()
    {
        return;
    }
    let pixel = sample(
        &display,
        "uniform usampler2D tex;",
        "color = vec4(float(texelFetch(tex, ivec2(0), 0).r), 0.0, 0.0, 255.0) / 255.0;",
        &uniform! { tex: &texture },
    );
    assert_eq!(pixel.0, 9);

    display.assert_no_error(None);
}

#[test]
fn clear_one_output() {
    let display = support::build_display();
    let first = Texture2d::empty(&display, 2, 2).unwrap();
    let second = Texture2d::empty(&display, 2, 2).unwrap();
    let mut framebuffer =
        MultiOutputFrameBuffer::new(&display, [("first", &first), ("second", &second)]).unwrap();
    framebuffer.clear_color(1.0, 0.0, 0.0, 1.0);
    framebuffer
        .clear_output("second", [0.0, 1.0, 0.0, 1.0])
        .unwrap();

    let read = |texture: &Texture2d| {
        let data: Vec<Vec<Pixel>> = texture.read().unwrap();
        data[1][1]
    };
    assert_eq!(read(&first), (255, 0, 0, 255));
    assert_eq!(read(&second), (0, 255, 0, 255));

    assert_eq!(
        framebuffer.clear_output("third", [0.0; 4]),
        Err(ClearBufferError::OutputMissing)
    );
    assert_eq!(
        framebuffer.clear_output("first", [0u32; 4]),
        Err(ClearBufferError::KindMismatch)
    );

    display.assert_no_error(None);
}
