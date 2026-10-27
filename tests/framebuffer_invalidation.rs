#![cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))]

#[macro_use]
extern crate safegl;

use safegl::framebuffer::{
    DepthStencilRenderBuffer, InvalidateAttachments, MultiOutputFrameBuffer, SimpleFrameBuffer,
};
use safegl::texture::DepthStencilFormat;
use safegl::{Rect, Surface, Texture2d};

mod support;

const SIZE: u32 = 16;

/// The content of invalidated attachments is undefined, so these tests check that invalidating
/// raises no OpenGL error and that the attachments can be drawn to afterwards.
fn assert_red(texture: &Texture2d) {
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    for pixel in data.iter().flatten() {
        assert_eq!(*pixel, (255, 0, 0, 255));
    }
}

#[test]
fn invalidate_simple_framebuffer() {
    let display = support::build_display();
    let (vertices, indices, program) = support::build_fullscreen_red_pipeline(&display);
    let color = Texture2d::empty(&display, SIZE, SIZE).unwrap();
    let depth_stencil =
        DepthStencilRenderBuffer::new(&display, DepthStencilFormat::I24I8, SIZE, SIZE).unwrap();
    let mut target =
        SimpleFrameBuffer::with_depth_stencil_buffer(&display, &color, &depth_stencil).unwrap();

    target.clear_color_and_depth((0.0, 0.0, 1.0, 1.0), 1.0);
    target.invalidate(InvalidateAttachments::DEPTH_STENCIL);
    target.invalidate(InvalidateAttachments::ALL);
    target.invalidate_rect(
        InvalidateAttachments::COLOR,
        &Rect {
            left: 2,
            bottom: 3,
            width: 4,
            height: 5,
        },
    );
    display.assert_no_error(None);

    target.clear_depth(1.0);
    target
        .draw(
            &vertices,
            &indices,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &safegl::DrawParameters {
                depth: safegl::Depth {
                    test: safegl::DepthTest::IfLess,
                    write: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    drop(target);
    assert_red(&color);

    display.assert_no_error(None);
}

#[test]
fn invalidate_depth_stencil_parts() {
    let display = support::build_display();
    let color = Texture2d::empty(&display, SIZE, SIZE).unwrap();
    let depth_stencil =
        DepthStencilRenderBuffer::new(&display, DepthStencilFormat::I24I8, SIZE, SIZE).unwrap();
    let target =
        SimpleFrameBuffer::with_depth_stencil_buffer(&display, &color, &depth_stencil).unwrap();

    // a depth-stencil attachment can be invalidated whole, or one part at a time
    for which in [
        InvalidateAttachments::DEPTH_STENCIL,
        InvalidateAttachments {
            depth: true,
            ..Default::default()
        },
        InvalidateAttachments {
            stencil: true,
            ..Default::default()
        },
        InvalidateAttachments::default(),
    ] {
        target.invalidate(which);
        display.assert_no_error(None);
    }
}

#[test]
fn invalidate_multi_output_framebuffer() {
    let display = support::build_display();
    let first = Texture2d::empty(&display, SIZE, SIZE).unwrap();
    let second = Texture2d::empty(&display, SIZE, SIZE).unwrap();
    let mut target =
        MultiOutputFrameBuffer::new(&display, [("first", &first), ("second", &second)]).unwrap();

    target.invalidate(InvalidateAttachments::ALL);
    target.invalidate_rect(
        InvalidateAttachments::COLOR,
        &Rect {
            left: 0,
            bottom: 0,
            width: SIZE,
            height: SIZE / 2,
        },
    );
    display.assert_no_error(None);

    target.clear_color(1.0, 0.0, 0.0, 1.0);
    drop(target);
    assert_red(&first);
    assert_red(&second);

    display.assert_no_error(None);
}

#[test]
fn invalidate_default_framebuffer() {
    let display = support::build_display();

    let mut frame = display.draw();
    frame.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
    // after the last draw of a frame, nothing reads the depth buffer any more
    frame.invalidate(InvalidateAttachments::DEPTH_STENCIL);
    frame.invalidate_rect(
        InvalidateAttachments::DEPTH_STENCIL,
        &Rect {
            left: 0,
            bottom: 0,
            width: 8,
            height: 8,
        },
    );
    frame.finish().unwrap();
    display.assert_no_error(None);

    let framebuffer = safegl::framebuffer::DefaultFramebuffer::back_left(&display);
    framebuffer.invalidate(InvalidateAttachments::COLOR);
    display.assert_no_error(None);
}
