//! The browser runs SafeGL: a WebGL 2 context on a canvas, a texture cleared and read back.
#![cfg(target_arch = "wasm32")]

use safegl::backend::web::WebDisplay;
use safegl::Surface;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn clear_and_read_a_texture() {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas: web_sys::HtmlCanvasElement = document
        .create_element("canvas")
        .unwrap()
        .dyn_into()
        .unwrap();
    canvas.set_width(16);
    canvas.set_height(16);
    let display = WebDisplay::from_canvas(canvas).unwrap();

    let texture = safegl::Texture2d::empty(&display, 4, 4).unwrap();
    texture.as_surface().clear_color(0.0, 1.0, 0.0, 1.0);
    let pixels: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(pixels[3][3], (0, 255, 0, 255));
    display.assert_no_error(None);
}
