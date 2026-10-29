
#[macro_use]
extern crate safegl;

use safegl::backend::Facade;
use safegl::CapabilitiesSource;
use safegl::framebuffer::MultiOutputFrameBuffer;
use safegl::index::PrimitiveType;
use safegl::{
    Blend, BlendingFunction, DrawError, LinearBlendingFactor, OutputState, Surface, Texture2d,
};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const SIZE: u32 = 16;

type Display = support::Backend;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

implement_vertex!(Vertex, position);

/// Draws a fullscreen quad that writes the `value` uniform to two outputs.
struct Pipeline {
    vertices: safegl::VertexBuffer<Vertex>,
    indices: safegl::IndexBuffer<u8>,
    program: safegl::Program,
}

impl Pipeline {
    fn new(display: &Display) -> Pipeline {
        let vertices = safegl::VertexBuffer::new(
            display,
            &[
                Vertex {
                    position: [-1.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, -1.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                },
            ],
        )
        .unwrap();
        let indices =
            safegl::IndexBuffer::new(display, PrimitiveType::TriangleStrip, &[0u8, 1, 2, 3])
                .unwrap();
        let program = program!(display,
            330 => {
                vertex: "
                    #version 330 core
                    in vec2 position;
                    void main() { gl_Position = vec4(position, 0.0, 1.0); }
                ",
                fragment: "
                    #version 330 core
                    uniform vec4 value;
                    layout(location = 0) out vec4 first;
                    layout(location = 1) out vec4 second;
                    void main() {
                        first = value;
                        second = value;
                    }
                ",
            },
            300 es => {
                vertex: "
                    #version 300 es
                    in vec2 position;
                    void main() { gl_Position = vec4(position, 0.0, 1.0); }
                ",
                fragment: "
                    #version 300 es
                    precision mediump float;
                    uniform vec4 value;
                    layout(location = 0) out vec4 first;
                    layout(location = 1) out vec4 second;
                    void main() {
                        first = value;
                        second = value;
                    }
                ",
            },
        )
        .unwrap();

        Pipeline {
            vertices,
            indices,
            program,
        }
    }

    fn draw<S: Surface>(
        &self,
        target: &mut S,
        value: (f32, f32, f32, f32),
        params: &safegl::DrawParameters<'_>,
    ) -> Result<(), DrawError> {
        target.draw(
            &self.vertices,
            &self.indices,
            &self.program,
            &uniform! { value: value },
            params,
        )
    }
}

fn textures(display: &Display) -> (Texture2d, Texture2d) {
    (
        Texture2d::empty(display, SIZE, SIZE).unwrap(),
        Texture2d::empty(display, SIZE, SIZE).unwrap(),
    )
}

fn framebuffer<'a>(
    display: &Display,
    first: &'a Texture2d,
    second: &'a Texture2d,
) -> MultiOutputFrameBuffer<'a> {
    MultiOutputFrameBuffer::new(display, [("first", first), ("second", second)]).unwrap()
}

/// Asserts that every pixel of `texture` is `expected`, give or take 1 for rounding.
fn assert_texture(texture: &Texture2d, expected: (u8, u8, u8, u8)) {
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    for pixel in data.iter().flatten() {
        let close = |a: u8, b: u8| a.abs_diff(b) <= 1;
        assert!(
            close(pixel.0, expected.0)
                && close(pixel.1, expected.1)
                && close(pixel.2, expected.2)
                && close(pixel.3, expected.3),
            "got {pixel:?}, expected {expected:?}"
        );
    }
}

/// Returns true if the backend gives draw buffers their own blending and color mask: OpenGL 4.0,
/// OpenGL ES 3.2, or `GL_EXT_draw_buffers_indexed` or `GL_OES_draw_buffers_indexed` on OpenGL ES
/// 3.0 and 3.1 (`OES_draw_buffers_indexed` on WebGL).
fn per_output_supported(display: &Display) -> bool {
    let extensions = support::extensions(display);
    !support::is_gles(display)
        || (*display.get_opengl_version() >= safegl::Version(safegl::Api::GlEs, 3, 2)
            && !cfg!(target_arch = "wasm32"))
        || extensions.gl_ext_draw_buffers_indexed
        || extensions.gl_oes_draw_buffers_indexed
}

fn addition() -> Blend {
    let add = BlendingFunction::Addition {
        source: LinearBlendingFactor::One,
        destination: LinearBlendingFactor::One,
    };
    Blend {
        color: add,
        alpha: add,
        ..Default::default()
    }
}

#[test]
fn outputs_have_their_own_blending_and_mask() {
    let display = support::build_display();
    let pipeline = Pipeline::new(&display);
    let (first, second) = textures(&display);
    let mut target = framebuffer(&display, &first, &second);
    target.clear_color(0.25, 0.25, 0.25, 0.25);

    let outputs = [
        OutputState {
            color_mask: (true, false, true, false),
            ..Default::default()
        },
        OutputState {
            blend: addition(),
            ..Default::default()
        },
    ];
    match pipeline.draw(
        &mut target,
        (0.5, 0.5, 0.5, 0.5),
        &safegl::DrawParameters {
            outputs: &outputs,
            ..Default::default()
        },
    ) {
        Err(DrawError::PerOutputStateNotSupported) => {
            assert!(!per_output_supported(&display));
            return;
        }
        result => {
            assert!(per_output_supported(&display));
            result.unwrap()
        }
    }
    drop(target);

    assert_texture(&first, (128, 64, 128, 64));
    assert_texture(&second, (191, 191, 191, 191));

    display.assert_no_error(None);
}

#[test]
fn a_draw_without_outputs_restores_every_draw_buffer() {
    let display = support::build_display();
    let pipeline = Pipeline::new(&display);
    let (first, second) = textures(&display);
    let mut target = framebuffer(&display, &first, &second);
    target.clear_color(0.25, 0.25, 0.25, 0.25);

    // the global state blends and masks green; the outputs undo it on one buffer each
    let params = safegl::DrawParameters {
        blend: addition(),
        color_mask: (true, false, true, true),
        outputs: &[
            OutputState::default(),
            OutputState {
                blend: addition(),
                color_mask: (true, true, true, true),
            },
        ],
        ..Default::default()
    };
    match pipeline.draw(&mut target, (0.5, 0.5, 0.5, 0.5), &params) {
        Err(DrawError::PerOutputStateNotSupported) => {
            assert!(!per_output_supported(&display));
            return;
        }
        result => {
            assert!(per_output_supported(&display));
            result.unwrap()
        }
    }

    // The cached global state didn't change, so only the per-buffer flags tell SafeGL that
    // the buffers must be reset.
    pipeline
        .draw(
            &mut target,
            (0.5, 0.5, 0.5, 0.5),
            &safegl::DrawParameters {
                blend: addition(),
                color_mask: (true, false, true, true),
                ..Default::default()
            },
        )
        .unwrap();
    drop(target);

    // first: replaced by 0.5, then 0.5 added with green masked
    assert_texture(&first, (255, 128, 255, 255));
    // second: 0.25 + 0.5, then 0.5 added with green masked
    assert_texture(&second, (255, 191, 255, 255));

    display.assert_no_error(None);
}

#[test]
fn clearing_ignores_the_masks_of_outputs() {
    let display = support::build_display();
    let pipeline = Pipeline::new(&display);
    let (first, second) = textures(&display);
    let mut target = framebuffer(&display, &first, &second);

    let outputs = [
        OutputState::default(),
        OutputState {
            color_mask: (false, false, false, false),
            ..Default::default()
        },
    ];
    match pipeline.draw(
        &mut target,
        (1.0, 0.0, 0.0, 1.0),
        &safegl::DrawParameters {
            outputs: &outputs,
            ..Default::default()
        },
    ) {
        Err(DrawError::PerOutputStateNotSupported) => {
            assert!(!per_output_supported(&display));
            return;
        }
        result => {
            assert!(per_output_supported(&display));
            result.unwrap()
        }
    }

    target.clear_color(0.0, 0.0, 1.0, 1.0);
    drop(target);

    assert_texture(&first, (0, 0, 255, 255));
    assert_texture(&second, (0, 0, 255, 255));

    display.assert_no_error(None);
}

#[test]
fn outputs_equal_to_the_global_state_need_no_support() {
    let display = support::build_display();
    let pipeline = Pipeline::new(&display);
    let (first, second) = textures(&display);
    let mut target = framebuffer(&display, &first, &second);

    // entries that don't override anything are accepted everywhere
    let outputs = [OutputState::default(); 2];
    pipeline
        .draw(
            &mut target,
            (0.0, 1.0, 0.0, 1.0),
            &safegl::DrawParameters {
                outputs: &outputs,
                ..Default::default()
            },
        )
        .unwrap();
    drop(target);

    assert_texture(&first, (0, 255, 0, 255));
    assert_texture(&second, (0, 255, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn too_many_outputs_are_rejected() {
    let display = support::build_display();
    let pipeline = Pipeline::new(&display);
    let (first, second) = textures(&display);
    let mut target = framebuffer(&display, &first, &second);

    let max = display.get_context().get_capabilities().max_draw_buffers as usize;
    let outputs = vec![OutputState::default(); max + 1];
    match pipeline.draw(
        &mut target,
        (1.0, 1.0, 1.0, 1.0),
        &safegl::DrawParameters {
            outputs: &outputs,
            ..Default::default()
        },
    ) {
        Err(DrawError::OutputStateIndexOutOfBounds) => (),
        result => panic!("{result:?}"),
    }

    display.assert_no_error(None);
}

#[test]
fn outputs_share_one_blend_constant() {
    let display = support::build_display();
    let pipeline = Pipeline::new(&display);
    let (first, second) = textures(&display);
    let mut target = framebuffer(&display, &first, &second);

    let constant = |value: f32| {
        let function = BlendingFunction::Addition {
            source: LinearBlendingFactor::ConstantColor,
            destination: LinearBlendingFactor::Zero,
        };
        Blend {
            color: function,
            alpha: function,
            constant_value: (value, value, value, value),
        }
    };

    let outputs = [
        OutputState {
            blend: constant(0.5),
            ..Default::default()
        },
        OutputState {
            blend: constant(0.25),
            ..Default::default()
        },
    ];
    match pipeline.draw(
        &mut target,
        (1.0, 1.0, 1.0, 1.0),
        &safegl::DrawParameters {
            outputs: &outputs,
            ..Default::default()
        },
    ) {
        Err(DrawError::BlendingParameterNotSupported) => (),
        Err(DrawError::PerOutputStateNotSupported) => {
            assert!(!per_output_supported(&display));
            return;
        }
        result => panic!("{result:?}"),
    }

    // the same constant on both outputs is fine
    let outputs = [
        OutputState {
            blend: constant(0.5),
            ..Default::default()
        },
        OutputState::default(),
    ];
    pipeline
        .draw(
            &mut target,
            (1.0, 1.0, 1.0, 1.0),
            &safegl::DrawParameters {
                outputs: &outputs,
                ..Default::default()
            },
        )
        .unwrap();
    drop(target);

    assert_texture(&first, (128, 128, 128, 128));
    assert_texture(&second, (255, 255, 255, 255));

    display.assert_no_error(None);
}
