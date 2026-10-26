#[macro_use]
extern crate safegl;

mod support;

// Tests for support of the cube map array type in programs.
fn program_support_for_cube_map_array(type_prefix: &str) {
    let display = support::build_display();

    // cube map arrays are core in OpenGL 4.0 and OpenGL ES 3.2
    let header = if support::is_gles(&display) {
        if *display.get_opengl_version() < safegl::Version(safegl::Api::GlEs, 3, 2) {
            return;
        }
        format!(
            "#version 320 es\nprecision highp float;\nprecision highp {}samplerCubeArray;\n",
            type_prefix
        )
    } else {
        if *display.get_opengl_version() < safegl::Version(safegl::Api::Gl, 4, 0) {
            return;
        }
        "#version 400\n".to_owned()
    };

    let _ = safegl::Program::from_source(
        &display,
        &format!(
            "{}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}",
            header
        ),
        &format!(
            "{}uniform {}samplerCubeArray cube_textures;\nout vec4 color;\n\
             void main() {{ color = vec4(texture(cube_textures, vec4(0, 0, 0, 0))); }}",
            header, type_prefix
        ),
        None,
    )
    .unwrap();
}

#[test]
fn program_support_for_cube_map_array_float() {
    program_support_for_cube_map_array("");
}

#[test]
fn program_support_for_cube_map_array_unsigned() {
    program_support_for_cube_map_array("u");
}

#[test]
fn program_support_for_cube_map_array_integral() {
    program_support_for_cube_map_array("i");
}
