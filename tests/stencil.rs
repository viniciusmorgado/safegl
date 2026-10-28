//! The stencil tests and operations of `DrawParameters::stencil` (glium#719).

#[macro_use]
extern crate safegl;

use safegl::draw_parameters::{Depth, DepthTest, Stencil, StencilOperation, StencilTest};
use safegl::framebuffer::{DepthStencilRenderBuffer, SimpleFrameBuffer};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::DepthStencilFormat;
use safegl::{DrawParameters, Surface, Texture2d, VertexBuffer};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Display = support::Backend;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A triangle covering the whole target, counter-clockwise on it, or clockwise if `reversed`.
fn fullscreen_triangle(display: &Display, reversed: bool) -> VertexBuffer<Vertex> {
    let mut vertices = vec![
        Vertex {
            position: [-1.0, -1.0],
        },
        Vertex {
            position: [3.0, -1.0],
        },
        Vertex {
            position: [-1.0, 3.0],
        },
    ];
    if reversed {
        vertices.reverse();
    }
    VertexBuffer::new(display, &vertices).unwrap()
}

/// A target with a color texture and a depth-stencil buffer.
struct Target {
    color: Texture2d,
    depth_stencil: DepthStencilRenderBuffer,
}

impl Target {
    fn new(display: &Display) -> Target {
        Target {
            color: Texture2d::empty(display, 4, 4).unwrap(),
            depth_stencil: DepthStencilRenderBuffer::new(display, DepthStencilFormat::I24I8, 4, 4)
                .unwrap(),
        }
    }

    fn framebuffer<'a>(&'a self, display: &Display) -> SimpleFrameBuffer<'a> {
        SimpleFrameBuffer::with_depth_stencil_buffer(display, &self.color, &self.depth_stencil)
            .unwrap()
    }

    /// Draws a red triangle with `stencil`, over black, and returns whether it was drawn.
    fn draw(&self, display: &Display, triangle: &VertexBuffer<Vertex>, stencil: Stencil) -> bool {
        self.draw_with(display, triangle, stencil, Default::default())
    }

    fn draw_with(
        &self,
        display: &Display,
        triangle: &VertexBuffer<Vertex>,
        stencil: Stencil,
        depth: Depth,
    ) -> bool {
        let (_, _, program) = support::build_fullscreen_red_pipeline(display);
        let mut framebuffer = self.framebuffer(display);
        framebuffer.clear_color(0.0, 0.0, 0.0, 0.0);
        framebuffer
            .draw(
                triangle,
                NoIndices(PrimitiveType::TrianglesList),
                &program,
                &uniform! {},
                &DrawParameters {
                    stencil,
                    depth,
                    ..Default::default()
                },
            )
            .unwrap();
        let pixels: Vec<Vec<(u8, u8, u8, u8)>> = self.color.read().unwrap();
        pixels[2][2] == (255, 0, 0, 255)
    }

    /// Returns whether the stencil buffer holds `value`, by drawing with an equality test.
    fn holds(&self, display: &Display, value: i32) -> bool {
        let triangle = fullscreen_triangle(display, false);
        self.draw(
            display,
            &triangle,
            both_faces(StencilTest::IfEqual { mask: 0xff }, value),
        )
    }

    fn clear_stencil(&self, display: &Display, value: i32) {
        self.framebuffer(display)
            .clear_depth_and_stencil(1.0, value);
    }
}

/// The same stencil test and reference value for both faces.
fn both_faces(test: StencilTest, reference: i32) -> Stencil {
    Stencil {
        test_clockwise: test,
        reference_value_clockwise: reference,
        test_counter_clockwise: test,
        reference_value_counter_clockwise: reference,
        ..Default::default()
    }
}

#[test]
fn stencil_tests() {
    let display = support::build_display();
    let target = Target::new(&display);
    let triangle = fullscreen_triangle(&display, false);
    target.clear_stencil(&display, 5);

    let mask = 0xff;
    for (test, reference, passes) in [
        (StencilTest::AlwaysPass, 0, true),
        (StencilTest::AlwaysFail, 0, false),
        (StencilTest::IfLess { mask }, 4, true),
        (StencilTest::IfLess { mask }, 5, false),
        (StencilTest::IfLessOrEqual { mask }, 5, true),
        (StencilTest::IfLessOrEqual { mask }, 6, false),
        (StencilTest::IfMore { mask }, 6, true),
        (StencilTest::IfMore { mask }, 5, false),
        (StencilTest::IfMoreOrEqual { mask }, 5, true),
        (StencilTest::IfMoreOrEqual { mask }, 4, false),
        (StencilTest::IfEqual { mask }, 5, true),
        (StencilTest::IfEqual { mask }, 4, false),
        (StencilTest::IfNotEqual { mask }, 4, true),
        (StencilTest::IfNotEqual { mask }, 5, false),
        // the mask applies to the reference and to the stored value
        (StencilTest::IfEqual { mask: 0x0f }, 0x35, true),
        (StencilTest::IfEqual { mask: 0xf0 }, 0x15, false),
        (StencilTest::IfMore { mask: 0x04 }, 0x0c, false),
    ] {
        assert_eq!(
            target.draw(&display, &triangle, both_faces(test, reference)),
            passes,
            "{test:?} with the reference {reference:#x}"
        );
    }

    display.assert_no_error(None);
}

#[test]
fn stencil_operations() {
    let display = support::build_display();
    let target = Target::new(&display);
    let triangle = fullscreen_triangle(&display, false);

    for (operation, initial, expected) in [
        (StencilOperation::Keep, 5, 5),
        (StencilOperation::Zero, 5, 0),
        (StencilOperation::Replace, 5, 9),
        (StencilOperation::Increment, 5, 6),
        (StencilOperation::Increment, 255, 255),
        (StencilOperation::IncrementWrap, 5, 6),
        (StencilOperation::IncrementWrap, 255, 0),
        (StencilOperation::Decrement, 5, 4),
        (StencilOperation::Decrement, 0, 0),
        (StencilOperation::DecrementWrap, 5, 4),
        (StencilOperation::DecrementWrap, 0, 255),
        (StencilOperation::Invert, 5, 250),
    ] {
        // the operation when the stencil test fails
        target.clear_stencil(&display, initial);
        target.draw(
            &display,
            &triangle,
            Stencil {
                fail_operation_clockwise: operation,
                fail_operation_counter_clockwise: operation,
                ..both_faces(StencilTest::AlwaysFail, 9)
            },
        );
        assert!(
            target.holds(&display, expected),
            "{operation:?} on a failed stencil test, from {initial}"
        );

        // when the stencil test passes but the depth test fails
        target.clear_stencil(&display, initial);
        target.draw_with(
            &display,
            &triangle,
            Stencil {
                pass_depth_fail_operation_clockwise: operation,
                pass_depth_fail_operation_counter_clockwise: operation,
                ..both_faces(StencilTest::AlwaysPass, 9)
            },
            // the depth buffer is cleared to 1.0
            Depth {
                test: DepthTest::IfMore,
                ..Default::default()
            },
        );
        assert!(
            target.holds(&display, expected),
            "{operation:?} on a failed depth test, from {initial}"
        );

        // when both pass
        target.clear_stencil(&display, initial);
        target.draw(
            &display,
            &triangle,
            Stencil {
                depth_pass_operation_clockwise: operation,
                depth_pass_operation_counter_clockwise: operation,
                ..both_faces(StencilTest::AlwaysPass, 9)
            },
        );
        assert!(
            target.holds(&display, expected),
            "{operation:?} on passed tests, from {initial}"
        );
    }

    display.assert_no_error(None);
}

#[test]
fn write_mask() {
    let display = support::build_display();
    let target = Target::new(&display);
    let triangle = fullscreen_triangle(&display, false);

    // only the low bits of the reference are written
    target.clear_stencil(&display, 0x50);
    target.draw(
        &display,
        &triangle,
        Stencil {
            depth_pass_operation_clockwise: StencilOperation::Replace,
            depth_pass_operation_counter_clockwise: StencilOperation::Replace,
            write_mask_clockwise: 0x0f,
            write_mask_counter_clockwise: 0x0f,
            ..both_faces(StencilTest::AlwaysPass, 0xaa)
        },
    );
    assert!(target.holds(&display, 0x5a));

    display.assert_no_error(None);
}

#[test]
fn faces() {
    let display = support::build_display();
    let target = Target::new(&display);
    let counter_clockwise = fullscreen_triangle(&display, false);
    let clockwise = fullscreen_triangle(&display, true);

    let stencil = Stencil {
        test_clockwise: StencilTest::AlwaysFail,
        test_counter_clockwise: StencilTest::AlwaysPass,
        ..Default::default()
    };
    assert!(target.draw(&display, &counter_clockwise, stencil));
    assert!(!target.draw(&display, &clockwise, stencil));

    // each face has its own reference value, write mask and operations
    let stencil = Stencil {
        reference_value_clockwise: 0x0f,
        write_mask_clockwise: 0x03,
        depth_pass_operation_clockwise: StencilOperation::Replace,
        reference_value_counter_clockwise: 0xf0,
        write_mask_counter_clockwise: 0x30,
        depth_pass_operation_counter_clockwise: StencilOperation::Invert,
        ..Default::default()
    };
    target.clear_stencil(&display, 0);
    target.draw(&display, &clockwise, stencil);
    assert!(target.holds(&display, 0x03));
    target.clear_stencil(&display, 0);
    target.draw(&display, &counter_clockwise, stencil);
    assert!(target.holds(&display, 0x30));

    display.assert_no_error(None);
}
