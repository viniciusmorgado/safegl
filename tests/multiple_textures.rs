//! Programs that sample several textures, each on its own texture unit (glium#366).

#[macro_use]
extern crate safegl;

use safegl::framebuffer::SimpleFrameBuffer;
use safegl::texture::{CubeLayer, Cubemap, Texture2dArray};
use safegl::uniforms::{MagnifySamplerFilter, Sampler};
use safegl::Surface;

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

fn draw<U: safegl::uniforms::Uniforms>(
    display: &Display,
    program: &safegl::Program,
    uniforms: &U,
) -> (u8, u8, u8, u8) {
    let (vb, ib) = support::build_rectangle_vb_ib(display);
    let texture = support::build_renderable_texture(display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(&vb, &ib, program, uniforms, &Default::default())
        .unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    data[0][0]
}

fn program(display: &Display, fragment: &str) -> safegl::Program {
    support::program_from_legacy(
        display,
        "#version 110\nattribute vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        fragment,
    )
    .unwrap()
}

#[test]
fn three_textures() {
    let display = support::build_display();
    // each texture gives one channel, so that a texture on the wrong unit shows
    let program = program(
        &display,
        "#version 110
         uniform sampler2D red;
         uniform sampler2D green;
         uniform sampler2D blue;
         void main() {
             gl_FragColor = vec4(texture2D(red, vec2(0.5)).r, texture2D(green, vec2(0.5)).g,
                 texture2D(blue, vec2(0.5)).b, 1.0);
         }",
    );
    let red = support::build_unicolor_texture2d(&display, 1.0, 0.0, 0.0);
    let green = support::build_unicolor_texture2d(&display, 0.0, 1.0, 0.0);
    let blue = support::build_unicolor_texture2d(&display, 0.0, 0.0, 1.0);

    assert_eq!(
        draw(
            &display,
            &program,
            &uniform! { red: &red, green: &green, blue: &blue }
        ),
        (255, 255, 255, 255)
    );

    // the units are bound again when the textures move between uniforms
    assert_eq!(
        draw(
            &display,
            &program,
            &uniform! { red: &green, green: &blue, blue: &red }
        ),
        (0, 0, 0, 255)
    );
    assert_eq!(
        draw(
            &display,
            &program,
            &uniform! { red: &red, green: &blue, blue: &blue }
        ),
        (255, 0, 255, 255)
    );

    // the same texture can be given to several uniforms, with different samplers
    let nearest = |texture| Sampler::new(texture).magnify_filter(MagnifySamplerFilter::Nearest);
    assert_eq!(
        draw(
            &display,
            &program,
            &uniform! { red: nearest(&red), green: &green, blue: nearest(&blue) }
        ),
        (255, 255, 255, 255)
    );

    display.assert_no_error(None);
}

#[test]
fn several_kinds_of_textures() {
    let display = support::build_display();
    let header = if support::is_gles(&display) {
        "#version 300 es\nprecision highp float;\nprecision highp sampler2DArray;\n"
    } else {
        "#version 330\n"
    };
    let program = safegl::Program::from_source(
        &display,
        &format!(
            "{header}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}"
        ),
        &format!(
            "{header}uniform sampler2D flat_texture;
             uniform sampler2DArray layers;
             uniform samplerCube cube;
             out vec4 color;
             void main() {{
                 color = vec4(texture(flat_texture, vec2(0.5)).r,
                     texture(layers, vec3(0.5, 0.5, 1.0)).g, texture(cube, vec3(1.0, 0.0, 0.0)).b,
                     1.0);
             }}"
        ),
        None,
    )
    .unwrap();

    let flat_texture = support::build_unicolor_texture2d(&display, 1.0, 0.0, 0.0);
    // the second layer is green
    let layers = Texture2dArray::new(
        &display,
        vec![
            vec![vec![(0u8, 0u8, 0u8, 255u8)]],
            vec![vec![(0u8, 255u8, 0u8, 255u8)]],
        ],
    )
    .unwrap();
    // every face is blue
    let cube = Cubemap::empty(&display, 1).unwrap();
    for layer in [
        CubeLayer::PositiveX,
        CubeLayer::NegativeX,
        CubeLayer::PositiveY,
        CubeLayer::NegativeY,
        CubeLayer::PositiveZ,
        CubeLayer::NegativeZ,
    ] {
        SimpleFrameBuffer::new(&display, cube.main_level().image(layer))
            .unwrap()
            .clear_color(0.0, 0.0, 1.0, 1.0);
    }

    assert_eq!(
        draw(
            &display,
            &program,
            &uniform! { flat_texture: &flat_texture, layers: &layers, cube: &cube }
        ),
        (255, 255, 255, 255)
    );

    display.assert_no_error(None);
}
