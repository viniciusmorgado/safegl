//! SafeGL can be used from the code that it calls: closures given to `exec_in_context`,
//! implementations of `Uniforms` and `AsUniformValue`, and debug callbacks.

#[macro_use]
extern crate safegl;

use safegl::uniforms::{UniformValue, Uniforms};
use safegl::Surface;
#[cfg(not(target_arch = "wasm32"))]
use safegl::CapabilitiesSource;
use std::cell::RefCell;

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn safegl_inside_exec_in_context() {
    let display = support::build_display();

    let value = unsafe {
        display.exec_in_context(|| {
            // creating, reading and dropping objects, and restoring the state
            let buffer = safegl::VertexBuffer::new(&*display, &[1u32, 2, 3].map(|v| Value { v }))
                .unwrap();
            let read = buffer.read().unwrap();
            display.restore_state();
            read[2].v
        })
    };
    assert_eq!(value, 3);

    display.assert_no_error(None);
}

#[derive(Copy, Clone)]
struct Value {
    v: u32,
}
implement_vertex!(Value, v);
implement_any_bit_pattern!(Value, v);

/// Uniforms that create the texture they give the first time they are visited.
struct LazyUniforms<'d> {
    display: &'d support::TestDisplay,
    texture: RefCell<Option<Box<safegl::Texture2d>>>,
}

impl<'d> Uniforms for LazyUniforms<'d> {
    fn visit_values<'a, F: FnMut(&'a str, UniformValue<'a>)>(&'a self, mut output: F) {
        if self.texture.borrow().is_none() {
            let texture = support::build_unicolor_texture2d(self.display, 0.0, 1.0, 0.0);
            *self.texture.borrow_mut() = Some(Box::new(texture));
        }
        // the box is never replaced, so the texture lives as long as `self`
        let texture: &'a safegl::Texture2d =
            unsafe { &*(&**self.texture.borrow().as_ref().unwrap() as *const _) };
        output("tex", UniformValue::Texture(texture, None));
    }
}

#[test]
fn safegl_inside_uniforms() {
    let display = support::build_display();

    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let program = program!(&display,
        330 => {
            vertex: "#version 330\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
            fragment: "#version 330\nuniform sampler2D tex;\nout vec4 f_color;\nvoid main() { f_color = texture(tex, vec2(0.5)); }",
        },
        300 es => {
            vertex: "#version 300 es\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
            fragment: "#version 300 es\nprecision highp float;\nuniform sampler2D tex;\nout vec4 f_color;\nvoid main() { f_color = texture(tex, vec2(0.5)); }",
        },
    )
    .unwrap();

    let uniforms = LazyUniforms {
        display: &display,
        texture: RefCell::new(None),
    };
    let target = support::build_renderable_texture(&display);
    target
        .as_surface()
        .draw(&vb, &ib, &program, &uniforms, &Default::default())
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    assert_eq!(data[0][0], (0, 255, 0, 255));

    display.assert_no_error(None);
}

// the panic is caught, which needs panics that unwind (not in browsers)
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn panicking_debug_callback() {
    if !support::contexts_check_errors() {
        return;
    }
    {
        // with only `GL_ARB_debug_output`, the driver may keep the messages of contexts that
        // aren't debug contexts, and nothing can turn them on
        let display = support::build_display();
        if !support::gl_at_least(&display, 4, 3)
            && !support::is_gles(&display)
            && !support::extensions(&display).gl_khr_debug
            && !display.get_capabilities().debug
        {
            return;
        }
    }
    let display = support::build_display_with_debug(
        safegl::debug::DebugCallbackBehavior::Custom {
            callback: Box::new(|_, ty, _, _, _, message| {
                if matches!(ty, safegl::debug::MessageType::Error) {
                    panic!("OpenGL error: {}", message);
                }
            }),
            synchronous: true,
        },
    );

    // an invalid enum raises an error; the panic of the callback can't unwind through OpenGL, so
    // it is resumed when the closure returns
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        display.exec_in_context(|| {
            use safegl::glow::HasContext;
            display.glow_context().enable(0x1234_5678);
        })
    }));
    let payload = result.unwrap_err();
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .unwrap_or_default();
    assert!(message.starts_with("OpenGL error"), "{}", message);

    // the context is still usable, once the error is consumed
    let error = unsafe {
        display.exec_in_context(|| {
            use safegl::glow::HasContext;
            display.glow_context().get_error()
        })
    };
    assert_eq!(error, safegl::glow::INVALID_ENUM);
    let _ = display.get_free_video_memory();
    display.assert_no_error(None);
}
