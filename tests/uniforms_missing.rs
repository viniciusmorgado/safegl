//! Every uniform of a program must have a value when drawing with it (glium#1388).

#[macro_use]
extern crate safegl;

use safegl::uniforms::{Keep, UniformBuffer};
use safegl::{DrawError, Surface};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

fn build_program(display: &Display, fragment: &str) -> safegl::Program {
    let header = if support::is_gles(display) {
        "#version 300 es\nprecision highp float;\n"
    } else {
        "#version 330\n"
    };
    let vertex = format!(
        "{}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}",
        header
    );
    let fragment = format!("{}out vec4 f_color;\n{}", header, fragment);
    safegl::Program::from_source(display, &vertex, &fragment, None).unwrap()
}

fn draw<U: safegl::uniforms::Uniforms>(
    display: &Display,
    program: &safegl::Program,
    uniforms: &U,
) -> Result<(u8, u8, u8, u8), DrawError> {
    let (vb, ib) = support::build_rectangle_vb_ib(display);
    let texture = support::build_renderable_texture(display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(&vb, &ib, program, uniforms, &Default::default())?;
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    Ok(data[0][0])
}

#[test]
fn plain_uniforms_need_a_value_once() {
    let display = support::build_display();
    let program = build_program(
        &display,
        "uniform vec4 color;\nvoid main() { f_color = color; }",
    );

    // never given a value
    assert!(matches!(
        draw(&display, &program, &uniform! {}),
        Err(DrawError::UniformMissing { ref name }) if name == "color"
    ));

    // the program keeps the value of the previous draw
    let red = [1.0f32, 0.0, 0.0, 1.0];
    assert_eq!(draw(&display, &program, &uniform! { color: red }).unwrap(), (255, 0, 0, 255));
    assert_eq!(draw(&display, &program, &uniform! {}).unwrap(), (255, 0, 0, 255));
    assert_eq!(draw(&display, &program, &uniform! { color: Keep }).unwrap(), (255, 0, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn keep_the_initializer() {
    let display = support::build_display();
    if support::is_gles(&display) {
        // OpenGL ES has no uniform initializers
        return;
    }

    let program = build_program(
        &display,
        "uniform vec4 color = vec4(0.0, 1.0, 0.0, 1.0);\nvoid main() { f_color = color; }",
    );
    assert_eq!(draw(&display, &program, &uniform! { color: Keep }).unwrap(), (0, 255, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn samplers_need_a_value_every_time() {
    let display = support::build_display();
    let program = build_program(
        &display,
        "uniform sampler2D tex;\nvoid main() { f_color = texture(tex, vec2(0.5)); }",
    );
    let texture = support::build_unicolor_texture2d(&display, 0.0, 0.0, 1.0);

    assert_eq!(draw(&display, &program, &uniform! { tex: &texture }).unwrap(), (0, 0, 255, 255));

    // the texture unit of the previous draw could hold anything now
    assert!(matches!(
        draw(&display, &program, &uniform! {}),
        Err(DrawError::UniformBindingMissing { ref name }) if name == "tex"
    ));
    assert!(matches!(
        draw(&display, &program, &uniform! { tex: Keep }),
        Err(DrawError::UniformTypeMismatch { .. })
    ));

    display.assert_no_error(None);
}

#[test]
fn blocks_need_a_value_every_time() {
    let display = support::build_display();
    let program = build_program(
        &display,
        "layout(std140) uniform Light { vec4 color; };\nvoid main() { f_color = color; }",
    );
    let light = UniformBuffer::new(&display, [1.0f32, 1.0, 0.0, 1.0]).unwrap();

    assert_eq!(draw(&display, &program, &uniform! { Light: &light }).unwrap(), (255, 255, 0, 255));
    assert!(matches!(
        draw(&display, &program, &uniform! {}),
        Err(DrawError::UniformBindingMissing { ref name }) if name == "Light"
    ));

    // the members of blocks are given through the buffer of the block
    assert!(matches!(
        draw(&display, &program, &uniform! { Light: &light, color: [0.0f32; 4] }),
        Err(DrawError::UniformBlockMember { ref name }) if name == "color"
    ));

    display.assert_no_error(None);
}

#[cfg(safegl_desktop)]
#[test]
fn every_stage_needs_its_subroutines() {
    let display = support::build_display();
    if support::is_gles(&display)
        || *display.get_opengl_version() < safegl::Version(safegl::Api::Gl, 4, 0)
    {
        // subroutines need OpenGL 4.0
        return;
    }

    let program = safegl::Program::from_source(
        &display,
        "#version 400\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 400\nsubroutine vec4 color_t();\nsubroutine(color_t) vec4 red() { return vec4(1.0, 0.0, 0.0, 1.0); }\nsubroutine uniform color_t color;\nout vec4 f_color;\nvoid main() { f_color = color(); }",
        None,
    )
    .unwrap();

    assert!(matches!(
        draw(&display, &program, &uniform! {}),
        Err(DrawError::SubroutineUniformMissing { real_count: 0, .. })
    ));
    let red = ("red", safegl::program::ShaderStage::Fragment);
    assert_eq!(draw(&display, &program, &uniform! { color: red }).unwrap(), (255, 0, 0, 255));

    display.assert_no_error(None);
}
