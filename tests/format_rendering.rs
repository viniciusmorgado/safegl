//! The formats that OpenGL ES and WebGL only render to, read, filter or blend through
//! extensions: 32-bit floats, signed normalized and 16-bit normalized formats, shared exponents.

#[macro_use]
extern crate safegl;

use safegl::framebuffer::SimpleFrameBuffer;
use safegl::texture::{MipmapsOption, UncompressedFloatFormat};
use safegl::uniforms::{MagnifySamplerFilter, MinifySamplerFilter};
use safegl::{Blend, DrawError, Surface, Texture2d};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// An empty texture of `format`, 4 × 4.
fn texture(display: &support::Backend, format: UncompressedFloatFormat) -> Option<Texture2d> {
    Texture2d::empty_with_format(display, format, MipmapsOption::NoMipmap, 4, 4).ok()
}

/// Blending into 32-bit floats needs `GL_EXT_float_blend` on OpenGL ES and WebGL.
#[test]
fn float_blending() {
    let display = support::build_display();
    let Some(texture) = texture(&display, UncompressedFloatFormat::F32F32F32F32) else {
        return;
    };
    let Ok(mut target) = SimpleFrameBuffer::new(&display, &texture) else {
        return;
    };
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    let parameters = safegl::DrawParameters {
        blend: Blend::alpha_blending(),
        ..Default::default()
    };
    let result = target.draw(&vb, &ib, &program, &uniform! {}, &parameters);
    if support::is_gles(&display) && !support::extensions(&display).gl_ext_float_blend {
        assert!(matches!(result, Err(DrawError::FloatBlendNotSupported)));
    } else {
        result.unwrap();
    }
    display.assert_no_error(None);
}

/// 32-bit floats are sampled without filtering where OpenGL ES and WebGL can't filter them,
/// instead of reading as zero.
#[test]
fn float_filtering() {
    let display = support::build_display();
    let data = vec![vec![(0.0f32, 0.0f32, 0.0f32, 1.0f32), (1.0, 1.0, 1.0, 1.0)]];
    let Ok(texture) = Texture2d::with_format(
        &display,
        data,
        UncompressedFloatFormat::F32F32F32F32,
        MipmapsOption::NoMipmap,
    ) else {
        return;
    };
    let target = Texture2d::empty(&display, 1, 1).unwrap();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let program = support::program_from_legacy(
        &display,
        "#version 110\nattribute vec2 position;\n\
         void main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 110\nuniform sampler2D tex;\n\
         void main() { gl_FragColor = texture2D(tex, vec2(0.5, 0.5)); }",
    )
    .unwrap();
    let sampler = texture
        .sampled()
        .minify_filter(MinifySamplerFilter::Linear)
        .magnify_filter(MagnifySamplerFilter::Linear);
    target
        .as_surface()
        .draw(
            &vb,
            &ib,
            &program,
            &uniform! { tex: sampler },
            &Default::default(),
        )
        .unwrap();
    let pixels: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    let red = pixels[0][0].0;
    if support::is_gles(&display) && !support::extensions(&display).gl_oes_texture_float_linear {
        // the nearest texel, between the two of them
        assert!(red == 0 || red == 255, "{}", red);
    } else {
        assert!((126..=129).contains(&red), "{}", red);
    }
    display.assert_no_error(None);
}

/// The signed normalized formats are render targets with `GL_EXT_render_snorm` on OpenGL ES and
/// WebGL, and are read as bytes.
#[test]
fn signed_normalized_targets() {
    let display = support::build_display();
    let Some(texture) = texture(&display, UncompressedFloatFormat::I8I8I8I8) else {
        return;
    };
    let renderable =
        !support::is_gles(&display) || support::extensions(&display).gl_ext_render_snorm;
    let target = SimpleFrameBuffer::new(&display, &texture);
    assert_eq!(target.is_ok(), renderable);
    if let Ok(mut target) = target {
        target.clear_color(-1.0, -0.5, 0.5, 1.0);
        let texels: Vec<Vec<(i8, i8, i8, i8)>> = texture.read_as().unwrap();
        let (r, g, b, a) = texels[0][0];
        assert_eq!((r, a), (-127, 127));
        assert!((-64..=-63).contains(&g) && (63..=64).contains(&b), "{:?}", texels[0][0]);
    }
    display.assert_no_error(None);
}

/// The 16-bit normalized formats are render targets with `GL_EXT_texture_norm16` on OpenGL ES
/// and WebGL, and are read as shorts.
#[test]
fn norm16_targets() {
    let display = support::build_display();
    let Some(texture) = texture(&display, UncompressedFloatFormat::U16U16U16U16) else {
        return;
    };
    let Ok(mut target) = SimpleFrameBuffer::new(&display, &texture) else {
        return;
    };
    target.clear_color(0.0, 0.5, 1.0, 1.0);
    let texels: Vec<Vec<(u16, u16, u16, u16)>> = texture.read_as().unwrap();
    let (r, g, b, a) = texels[0][0];
    assert_eq!((r, b, a), (0, 65535, 65535));
    assert!((32767..=32768).contains(&g), "{}", g);
    display.assert_no_error(None);
}

/// Shared exponents are render targets on WebGL with `WEBGL_render_shared_exponent`, and are read
/// as floats.
#[test]
fn shared_exponent_targets() {
    let display = support::build_display();
    let Some(texture) = texture(&display, UncompressedFloatFormat::F9F9F9) else {
        return;
    };
    let renderable = support::extensions(&display).webgl_render_shared_exponent;
    let target = SimpleFrameBuffer::new(&display, &texture);
    assert_eq!(target.is_ok(), renderable);
    if let Ok(mut target) = target {
        target.clear_color(0.5, 2.0, 0.25, 1.0);
        let texels: Vec<Vec<(f32, f32, f32, f32)>> = texture.read_as().unwrap();
        let (r, g, b, _) = texels[0][0];
        assert_eq!((r, g, b), (0.5, 2.0, 0.25));
    }
    display.assert_no_error(None);
}
