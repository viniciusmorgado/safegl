//! The WebGL 2 extensions and capabilities that SafeGL reads from the browser.
#![cfg(target_arch = "wasm32")]

#[macro_use]
extern crate safegl;

use safegl::framebuffer::{RenderBuffer, RenderBufferCreationError};
use safegl::texture::{MipmapsOption, TextureFormat, UncompressedFloatFormat};
use safegl::{CapabilitiesSource, Surface, Texture2d};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test as test;

mod support;

/// The browser's context of a display.
fn webgl(display: &support::TestDisplay) -> web_sys::WebGl2RenderingContext {
    display
        .canvas()
        .get_context("webgl2")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap()
}

/// The extensions that the browser lists, without the ones that `SAFEGL_DISABLE_EXTENSIONS`
/// hides from SafeGL.
fn supported_extensions(display: &support::TestDisplay) -> Vec<String> {
    let hidden = support::disabled_extensions();
    webgl(display)
        .get_supported_extensions()
        .unwrap()
        .iter()
        .filter_map(|name| name.as_string())
        .filter(|name| !hidden.split_whitespace().any(|hidden| hidden == name))
        .collect()
}

/// Every WebGL 2 extension of the registry that SafeGL lists sets its field, without the `GL_`
/// prefix of the native names.
#[test]
fn extension_names_without_prefix() {
    let display = support::build_display();
    let supported = supported_extensions(&display);
    let e = support::extensions(&display);

    let fields = [
        ("EXT_clip_control", e.gl_ext_clip_control),
        ("EXT_color_buffer_float", e.gl_ext_color_buffer_float),
        ("EXT_color_buffer_half_float", e.gl_ext_color_buffer_half_float),
        ("EXT_conservative_depth", e.gl_ext_conservative_depth),
        ("EXT_depth_clamp", e.gl_ext_depth_clamp),
        ("EXT_disjoint_timer_query_webgl2", e.ext_disjoint_timer_query_webgl2),
        ("EXT_float_blend", e.gl_ext_float_blend),
        ("EXT_polygon_offset_clamp", e.gl_ext_polygon_offset_clamp),
        ("EXT_render_snorm", e.gl_ext_render_snorm),
        ("EXT_texture_compression_bptc", e.gl_ext_texture_compression_bptc),
        ("EXT_texture_compression_rgtc", e.gl_ext_texture_compression_rgtc),
        ("EXT_texture_filter_anisotropic", e.gl_ext_texture_filter_anisotropic),
        ("EXT_texture_mirror_clamp_to_edge", e.gl_ext_texture_mirror_clamp_to_edge),
        ("EXT_texture_norm16", e.gl_ext_texture_norm16),
        ("KHR_parallel_shader_compile", e.gl_khr_parallel_shader_compile),
        ("OES_draw_buffers_indexed", e.gl_oes_draw_buffers_indexed),
        ("OES_sample_variables", e.gl_oes_sample_variables),
        ("OES_shader_multisample_interpolation", e.gl_oes_shader_multisample_interpolation),
        ("OES_texture_float_linear", e.gl_oes_texture_float_linear),
        ("OVR_multiview2", e.gl_ovr_multiview2),
        ("WEBGL_blend_func_extended", e.webgl_blend_func_extended),
        ("WEBGL_clip_cull_distance", e.webgl_clip_cull_distance),
        ("WEBGL_compressed_texture_astc", e.webgl_compressed_texture_astc),
        ("WEBGL_compressed_texture_etc", e.webgl_compressed_texture_etc),
        ("WEBGL_compressed_texture_etc1", e.webgl_compressed_texture_etc1),
        ("WEBGL_compressed_texture_pvrtc", e.webgl_compressed_texture_pvrtc),
        ("WEBGL_compressed_texture_s3tc", e.webgl_compressed_texture_s3tc),
        ("WEBGL_compressed_texture_s3tc_srgb", e.webgl_compressed_texture_s3tc_srgb),
        ("WEBGL_debug_renderer_info", e.webgl_debug_renderer_info),
        (
            "WEBGL_draw_instanced_base_vertex_base_instance",
            e.webgl_draw_instanced_base_vertex_base_instance,
        ),
        ("WEBGL_lose_context", e.webgl_lose_context),
        ("WEBGL_multi_draw", e.webgl_multi_draw),
        (
            "WEBGL_multi_draw_instanced_base_vertex_base_instance",
            e.webgl_multi_draw_instanced_base_vertex_base_instance,
        ),
        ("WEBGL_polygon_mode", e.webgl_polygon_mode),
        ("WEBGL_provoking_vertex", e.webgl_provoking_vertex),
        ("WEBGL_render_shared_exponent", e.webgl_render_shared_exponent),
        ("WEBGL_stencil_texturing", e.webgl_stencil_texturing),
    ];
    for (name, field) in fields {
        assert_eq!(field, supported.contains(&name.to_owned()), "{}", name);
    }

    display.assert_no_error(None);
}

/// SafeGL enables the extensions that shaders use: their `#extension` compiles.
#[test]
fn shader_extensions_are_enabled() {
    let display = support::build_display();
    let e = support::extensions(&display);

    let extensions = [
        ("GL_EXT_conservative_depth", e.gl_ext_conservative_depth),
        ("GL_OES_sample_variables", e.gl_oes_sample_variables),
        ("GL_OES_shader_multisample_interpolation", e.gl_oes_shader_multisample_interpolation),
    ];
    for name in extensions.into_iter().filter_map(|(name, has)| has.then_some(name)) {
        let fragment = format!(
            "#version 300 es\n#extension {} : require\nprecision mediump float;\n\
             out vec4 color;\nvoid main() {{ color = vec4(1.0); }}\n",
            name
        );
        let program = safegl::Program::from_source(
            &display,
            "#version 300 es\nvoid main() { gl_Position = vec4(0.0, 0.0, 0.0, 1.0); }\n",
            &fragment,
            None,
        );
        assert!(program.is_ok(), "{}: {:?}", name, program.err());
    }

    display.assert_no_error(None);
}

/// With `EXT_color_buffer_float`, 32-bit float textures are render targets: a clear writes values
/// outside of [0, 1] that read back exactly.
#[test]
fn float_render_targets() {
    let display = support::build_display();
    let texture = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::F32F32F32F32,
        MipmapsOption::NoMipmap,
        4,
        4,
    )
    .unwrap();

    let renderable = support::extensions(&display).gl_ext_color_buffer_float;
    let framebuffer = safegl::framebuffer::SimpleFrameBuffer::new(&display, &texture);
    assert_eq!(framebuffer.is_ok(), renderable);
    if let Ok(mut framebuffer) = framebuffer {
        framebuffer.clear_color(0.5, 2.0, -1.0, 1.0);
        let texels: Vec<Vec<(f32, f32, f32, f32)>> = texture.read_as().unwrap();
        assert_eq!(texels[3][3], (0.5, 2.0, -1.0, 1.0));
    }

    display.assert_no_error(None);
}

/// The browser lists the sample counts of the formats of render buffers: the largest one is
/// accepted, and more is an error.
#[test]
fn render_buffer_sample_counts() {
    let display = support::build_display();
    let capabilities = display.get_capabilities();

    let mut formats = vec![UncompressedFloatFormat::U8U8U8U8];
    if support::extensions(&display).gl_ext_color_buffer_float {
        formats.push(UncompressedFloatFormat::F32F32F32F32);
    }
    for format in formats {
        let samples = capabilities
            .internal_formats_renderbuffers
            .get(&TextureFormat::UncompressedFloat(format))
            .and_then(|infos| infos.multisamples.clone())
            .unwrap();
        let max = samples.iter().copied().max().unwrap_or(0) as u32;
        assert!(max <= capabilities.limits.max_samples as u32, "{:?}", format);
        if max > 0 {
            RenderBuffer::new_multisample(&display, format, 4, 4, max).unwrap();
        }
        assert_eq!(
            RenderBuffer::new_multisample(&display, format, 4, 4, max + 1).err(),
            Some(RenderBufferCreationError::SamplesNotSupported),
            "{:?}",
            format
        );
    }
    // RGBA8 is multisampled in every WebGL 2 implementation
    let rgba8 = &capabilities.internal_formats_renderbuffers
        [&TextureFormat::UncompressedFloat(UncompressedFloatFormat::U8U8U8U8)];
    assert!(rgba8.multisamples.as_ref().unwrap().iter().any(|&samples| samples >= 4));

    display.assert_no_error(None);
}

/// The renderer and the vendor are the real ones that `WEBGL_debug_renderer_info` gives, when the
/// browser has it.
#[test]
fn renderer_and_vendor() {
    let display = support::build_display();
    let capabilities = display.get_capabilities();
    assert!(!capabilities.renderer.is_empty());
    assert!(!capabilities.vendor.is_empty());

    if support::extensions(&display).webgl_debug_renderer_info {
        let gl = webgl(&display);
        let unmasked = |name| gl.get_parameter(name).unwrap().as_string().unwrap();
        assert_eq!(capabilities.renderer, unmasked(0x9246));
        assert_eq!(capabilities.vendor, unmasked(0x9245));
    }

    display.assert_no_error(None);
}
