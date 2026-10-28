//! Programs compiled and linked in the background (`Program::new_pending`), with
//! `KHR_parallel_shader_compile`, and at once without it.

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndices, PrimitiveType};
use safegl::program::{ProgramCreationInput, ShaderType};
use safegl::{PendingProgram, ProgramCreationError, Surface, Texture2d, VertexBuffer};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// The header of the shaders of the context.
fn header(display: &support::Backend) -> &'static str {
    if support::is_gles(display) {
        "#version 300 es\nprecision highp float;\n"
    } else {
        "#version 330\n"
    }
}

/// Starts building a program from the bodies of its shaders.
fn pending(
    display: &support::Backend,
    vertex: &str,
    fragment: &str,
) -> Result<PendingProgram, ProgramCreationError> {
    let header = header(display);
    let vertex = format!("{header}{vertex}");
    let fragment = format!("{header}{fragment}");
    safegl::Program::new_pending(
        display,
        ProgramCreationInput::SourceCode {
            vertex_shader: &vertex,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: &fragment,
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
            locations: Default::default(),
        },
    )
}

/// Waits for a pending program: in browsers by letting the event loop run, elsewhere by polling.
async fn ready(program: &PendingProgram) {
    support::until(|| program.is_ready()).await;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let start = std::time::Instant::now();
        while !program.is_ready() {
            assert!(start.elapsed().as_secs() < 10, "never ready");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}

support::async_test! {
    fn pending_program_draws() {
        let display = support::build_display();
        let program = pending(
            &display,
            "in vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
            "out vec4 color;\nvoid main() { color = vec4(1.0, 0.0, 0.0, 1.0); }",
        )
        .unwrap();
        ready(&program).await;
        assert!(program.is_ready());
        let program = program.finish().unwrap();
        assert!(program.get_attribute("position").is_some());

        let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
        let triangle =
            VertexBuffer::new(&display, &positions.map(|position| Vertex { position })).unwrap();
        let target = Texture2d::empty(&display, 4, 4).unwrap();
        target
            .as_surface()
            .draw(
                &triangle,
                NoIndices(PrimitiveType::TrianglesList),
                &program,
                &uniform! {},
                &Default::default(),
            )
            .unwrap();
        let pixels: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
        assert_eq!(pixels[2][2], (255, 0, 0, 255));

        display.assert_no_error(None);
    }
}

support::async_test! {
    fn pending_compilation_error() {
        let display = support::build_display();
        let program = pending(
            &display,
            "void main() { gl_Position = vec4(0.0); }",
            "out vec4 color;\nvoid main() { color = undefined_function(); }",
        )
        .unwrap();
        ready(&program).await;
        match program.finish() {
            Err(ProgramCreationError::CompilationError(_, ShaderType::Fragment)) => (),
            other => panic!("{:?}", other.err()),
        }

        display.assert_no_error(None);
    }
}

support::async_test! {
    fn pending_link_error() {
        let display = support::build_display();
        // the uniform `u` has two types, which only the linker finds out
        let program = pending(
            &display,
            "uniform float u;\nvoid main() { gl_Position = vec4(u); }",
            "uniform vec4 u;\nout vec4 color;\nvoid main() { color = u; }",
        )
        .unwrap();
        ready(&program).await;
        match program.finish() {
            Err(ProgramCreationError::LinkingError(_)) => (),
            other => panic!("{:?}", other.err()),
        }

        display.assert_no_error(None);
    }
}

/// A pending program can be dropped before it is ready, and the synchronous constructors report
/// the same errors.
#[test]
fn dropped_and_synchronous() {
    let display = support::build_display();
    let program = pending(
        &display,
        "void main() { gl_Position = vec4(0.0); }",
        "out vec4 color;\nvoid main() { color = vec4(1.0); }",
    )
    .unwrap();
    drop(program);

    let header = header(&display);
    let result = safegl::Program::from_source(
        &display,
        &format!("{header}void main() {{ gl_Position = vec4(0.0); }}"),
        &format!("{header}out vec4 color;\nvoid main() {{ color = undefined_function(); }}"),
        None,
    );
    match result {
        Err(ProgramCreationError::CompilationError(_, ShaderType::Fragment)) => (),
        other => panic!("{:?}", other.err()),
    }

    // the number of threads can only be set with the extension, and not in browsers
    let extensions = support::extensions(&display);
    let settable = !cfg!(target_arch = "wasm32")
        && (extensions.gl_khr_parallel_shader_compile
            || extensions.gl_arb_parallel_shader_compile);
    assert_eq!(display.set_shader_compiler_threads(Some(2)), settable);
    assert_eq!(display.set_shader_compiler_threads(None), settable);

    display.assert_no_error(None);
}
