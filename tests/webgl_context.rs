//! The WebGL context of a canvas: its attributes, its size on HiDPI screens, its loss and its
//! restoration, and its debug output.
#![cfg(target_arch = "wasm32")]

#[macro_use]
extern crate safegl;

use std::cell::RefCell;
use std::rc::Rc;

use safegl::backend::web::{PowerPreference, WebDisplay, WebDisplayCreationError};
use safegl::{CapabilitiesSource, Surface, Texture2d};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test as test;

mod support;

fn canvas() -> web_sys::HtmlCanvasElement {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas: web_sys::HtmlCanvasElement = document
        .create_element("canvas")
        .unwrap()
        .dyn_into()
        .unwrap();
    canvas.set_width(16);
    canvas.set_height(16);
    canvas
}

fn webgl(display: &WebDisplay) -> web_sys::WebGl2RenderingContext {
    display
        .canvas()
        .get_context("webgl2")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap()
}

/// The `WEBGL_lose_context` extension of a display, which loses and restores its context.
fn lose_context(display: &WebDisplay) -> web_sys::WebglLoseContext {
    // the objects of extensions aren't instances of a class
    webgl(display)
        .get_extension("WEBGL_lose_context")
        .unwrap()
        .unwrap()
        .unchecked_into()
}

/// The builder gives the context its attributes, which the default framebuffer reflects.
#[test]
fn context_attributes() {
    let display = WebDisplay::builder(canvas())
        .stencil(true)
        .depth(false)
        .antialias(false)
        .alpha(false)
        .premultiplied_alpha(false)
        .preserve_drawing_buffer(true)
        .power_preference(PowerPreference::LowPower)
        .desynchronized(false)
        .build()
        .unwrap();

    let attributes = webgl(&display).get_context_attributes().unwrap();
    assert_eq!(attributes.get_stencil(), Some(true));
    assert_eq!(attributes.get_antialias(), Some(false));
    let capabilities = display.get_capabilities();
    assert_eq!(capabilities.stencil_bits, Some(8));
    assert!(matches!(capabilities.depth_bits, None | Some(0)));

    // the browser's defaults: no stencil buffer
    let display = WebDisplay::from_canvas(canvas()).unwrap();
    assert!(matches!(
        display.get_capabilities().stencil_bits,
        None | Some(0)
    ));
    display.assert_no_error(None);
}

/// The drawing buffer gets a pixel per device pixel of the canvas on the screen.
#[test]
fn resize_to_display() {
    let display = WebDisplay::from_canvas(canvas()).unwrap();
    assert_eq!(display.resize_to_display((100.0, 50.0), 2.0), (200, 100));
    assert_eq!(display.canvas().width(), 200);
    assert_eq!(display.get_framebuffer_dimensions(), (200, 100));
    assert_eq!(display.resize_to_display((10.2, 0.1), 1.5), (15, 1));
    display.assert_no_error(None);
}

/// While the context is lost, objects can't be created and the commands do nothing; once the
/// browser restored it, a new display draws with it, and the objects of the lost context belong
/// to the old display.
#[wasm_bindgen_test::wasm_bindgen_test]
async fn loss_and_restoration() {
    let display = WebDisplay::from_canvas(canvas()).unwrap();
    let old_texture = Texture2d::empty(&display, 4, 4).unwrap();
    let lose = lose_context(&display);

    lose.lose_context();
    assert!(display.is_context_lost());
    let texture = Texture2d::empty(&display, 4, 4);
    assert!(
        matches!(
            texture,
            Err(safegl::texture::TextureCreationError::ContextLost)
        ),
        "{:?}",
        texture.err()
    );
    let primitives = safegl::index::PrimitiveType::TrianglesList;
    assert!(matches!(
        safegl::IndexBuffer::<u16>::empty(&display, primitives, 4),
        Err(safegl::index::BufferCreationError::BufferCreationError(
            safegl::buffer::BufferCreationError::ContextLost
        ))
    ));
    // the commands do nothing, and the frame tells the loss
    old_texture.as_surface().clear_color(1.0, 0.0, 0.0, 1.0);
    let mut frame = display.draw();
    frame.clear_color(1.0, 0.0, 0.0, 1.0);
    assert_eq!(frame.finish(), Err(safegl::SwapBuffersError::ContextLost));
    assert!(matches!(
        display.restore(),
        Err(WebDisplayCreationError::ContextNotRestored)
    ));

    lose.restore_context();
    support::until(|| display.is_context_restored()).await;
    let restored = display.restore().unwrap();
    assert!(!restored.is_context_lost());
    assert!(display.is_context_lost());

    let texture = Texture2d::empty(&restored, 4, 4).unwrap();
    texture.as_surface().clear_color(0.0, 1.0, 0.0, 1.0);
    let pixels: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(pixels[3][3], (0, 255, 0, 255));

    // the objects of the lost context are dropped without errors in the restored one
    drop(old_texture);
    drop(display);
    restored.assert_no_error(None);
}

/// The debug callback gets the errors that WebGL raises, after each operation.
#[test]
fn debug_output_polls_errors() {
    use safegl::debug::{DebugCallbackBehavior, MessageType, Severity, Source};
    use safegl::glow::HasContext;

    let messages = Rc::new(RefCell::new(Vec::new()));
    let sink = messages.clone();
    let callback = Box::new(
        move |source, ty, severity, id, _: bool, message: &str| {
            sink.borrow_mut()
                .push((source, ty, severity, id, message.to_owned()));
        },
    );
    let display = WebDisplay::builder(canvas())
        .debug_callback_behavior(DebugCallbackBehavior::Custom {
            callback,
            synchronous: true,
        })
        .build()
        .unwrap();

    // an invalid capability raises `GL_INVALID_ENUM`, which the next operation reports
    unsafe { display.glow_context().enable(0x1234) };
    let texture = Texture2d::empty(&display, 2, 2).unwrap();
    let messages = messages.borrow();
    assert_eq!(messages.len(), 1, "{:?}", *messages);
    let (source, ty, severity, id, message) = &messages[0];
    assert_eq!(
        (*source, *ty, *severity, *id),
        (Source::Api, MessageType::Error, Severity::High, 0x0500)
    );
    assert_eq!(message, "GL_INVALID_ENUM");
    drop(texture);
}
