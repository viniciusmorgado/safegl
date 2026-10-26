#[macro_use]
extern crate safegl;

use glow::HasContext;
use safegl::framebuffer::{ExternalFramebuffer, InvalidateAttachments, SimpleFrameBuffer};
use safegl::{GlObject, Surface, Texture2d};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

const SIZE: u32 = 4;

/// Creates a framebuffer object outside SafeGL, the way a GUI toolkit would, with `texture` as
/// its color attachment.
fn foreign_framebuffer(display: &Display, texture: &Texture2d) -> glow::Framebuffer {
    let framebuffer = unsafe {
        display.exec_in_context(|| {
            let gl = display.glow_context();
            let framebuffer = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture.get_id()),
                0,
            );
            framebuffer
        })
    };
    display.restore_state();
    framebuffer
}

fn pixel(texture: &Texture2d) -> (u8, u8, u8, u8) {
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    data[1][1]
}

#[test]
fn draw_and_clear() {
    let display = support::build_display();
    let texture = Texture2d::empty(&display, SIZE, SIZE).unwrap();
    let id = foreign_framebuffer(&display, &texture);
    let mut target = unsafe { ExternalFramebuffer::new(&display, id, (SIZE, SIZE)) };
    assert_eq!(target.get_dimensions(), (SIZE, SIZE));
    assert!(!target.has_depth_buffer());

    target.clear_color(0.0, 0.0, 1.0, 1.0);
    assert_eq!(pixel(&texture), (0, 0, 255, 255));

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    target
        .draw(
            &vb,
            &ib,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();
    assert_eq!(pixel(&texture), (255, 0, 0, 255));

    // no depth buffer
    let result = target.draw(
        &vb,
        &ib,
        &program,
        &safegl::uniforms::EmptyUniforms,
        &safegl::DrawParameters {
            depth: safegl::Depth {
                test: safegl::DepthTest::IfLess,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert!(matches!(result, Err(safegl::DrawError::NoDepthBuffer)));

    target.invalidate(InvalidateAttachments::ALL);
    display.assert_no_error(None);
}

#[test]
fn blit_to_and_from() {
    let display = support::build_display();
    let foreign = Texture2d::empty(&display, SIZE, SIZE).unwrap();
    let id = foreign_framebuffer(&display, &foreign);
    let target = unsafe { ExternalFramebuffer::new(&display, id, (SIZE, SIZE)) };

    let green = support::build_unicolor_texture2d(&display, 0.0, 1.0, 0.0);
    green.as_surface().blit_whole_color_to(
        &target,
        &safegl::BlitTarget {
            left: 0,
            bottom: 0,
            width: SIZE as i32,
            height: SIZE as i32,
        },
        safegl::uniforms::MagnifySamplerFilter::Nearest,
    ).unwrap();
    assert_eq!(pixel(&foreign), (0, 255, 0, 255));

    let copy = Texture2d::empty(&display, SIZE, SIZE).unwrap();
    target.blit_whole_color_to(
        &copy.as_surface(),
        &safegl::BlitTarget {
            left: 0,
            bottom: 0,
            width: SIZE as i32,
            height: SIZE as i32,
        },
        safegl::uniforms::MagnifySamplerFilter::Nearest,
    ).unwrap();
    assert_eq!(pixel(&copy), (0, 255, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn framebuffer_object_of_a_simple_framebuffer() {
    let display = support::build_display();
    let texture = Texture2d::empty(&display, SIZE, SIZE).unwrap();
    let framebuffer = SimpleFrameBuffer::new(&display, &texture).unwrap();
    let id = framebuffer.get_framebuffer_object();

    // code outside SafeGL clears the texture through SafeGL's framebuffer object
    unsafe {
        display.exec_in_context(|| {
            let gl = display.glow_context();
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(id));
            gl.disable(glow::SCISSOR_TEST);
            gl.color_mask(true, true, true, true);
            gl.clear_color(1.0, 1.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        });
    }
    display.restore_state();

    assert_eq!(pixel(&texture), (255, 255, 0, 255));
    display.assert_no_error(None);
}
