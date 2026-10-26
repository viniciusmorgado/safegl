//! The exported macros work when they are called by their path, without `#[macro_use]`.

extern crate safegl;

#[derive(Copy, Clone)]
#[allow(dead_code)]
struct Light {
    color: [f32; 4],
}
safegl::implement_uniform_block!(Light, color);
safegl::implement_vertex!(Light, color);

#[allow(dead_code)]
fn program(display: &safegl::Display<safegl::glutin::surface::WindowSurface>) -> safegl::Program {
    safegl::program!(display,
        330 => {
            vertex: "#version 330\nvoid main() { gl_Position = vec4(0.0); }",
            fragment: "#version 330\nout vec4 f_color;\nvoid main() { f_color = vec4(1.0); }",
        },
    )
    .unwrap()
}

#[test]
fn macros_called_by_path() {
    fn is_uniform_block<T: safegl::uniforms::UniformBlock>() {}
    fn is_vertex<T: safegl::Vertex>() {}
    is_uniform_block::<Light>();
    is_vertex::<Light>();
}
