//! The browser's WebGL 2 context, for the WebGL functions that glow doesn't have.

#[cfg(safegl_browser)]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(safegl_browser)]
use web_sys::{WebGl2RenderingContext, WebGlProgram};

/// The real vendor of `WEBGL_debug_renderer_info`.
#[cfg(safegl_browser)]
pub const UNMASKED_VENDOR_WEBGL: u32 = 0x9245;

/// The real renderer of `WEBGL_debug_renderer_info`.
#[cfg(safegl_browser)]
pub const UNMASKED_RENDERER_WEBGL: u32 = 0x9246;

/// The WebGL 2 extensions that SafeGL uses. Each one must be enabled with `getExtension` before
/// its functions, its constants and its GLSL `#extension` can be used.
#[cfg(safegl_browser)]
const EXTENSIONS: &[&str] = &[
    "EXT_clip_control",
    "EXT_color_buffer_float",
    "EXT_color_buffer_half_float",
    "EXT_conservative_depth",
    "EXT_depth_clamp",
    "EXT_disjoint_timer_query_webgl2",
    "EXT_float_blend",
    "EXT_polygon_offset_clamp",
    "EXT_render_snorm",
    "EXT_texture_compression_bptc",
    "EXT_texture_compression_rgtc",
    "EXT_texture_filter_anisotropic",
    "EXT_texture_mirror_clamp_to_edge",
    "EXT_texture_norm16",
    "KHR_parallel_shader_compile",
    "OES_draw_buffers_indexed",
    "OES_sample_variables",
    "OES_shader_multisample_interpolation",
    "OES_texture_float_linear",
    "OVR_multiview2",
    "WEBGL_blend_func_extended",
    "WEBGL_clip_cull_distance",
    "WEBGL_compressed_texture_astc",
    "WEBGL_compressed_texture_etc",
    "WEBGL_compressed_texture_etc1",
    "WEBGL_compressed_texture_pvrtc",
    "WEBGL_compressed_texture_s3tc",
    "WEBGL_compressed_texture_s3tc_srgb",
    "WEBGL_debug_renderer_info",
    "WEBGL_draw_instanced_base_vertex_base_instance",
    "WEBGL_lose_context",
    "WEBGL_multi_draw",
    "WEBGL_multi_draw_instanced_base_vertex_base_instance",
    "WEBGL_polygon_mode",
    "WEBGL_provoking_vertex",
    "WEBGL_render_shared_exponent",
    "WEBGL_stencil_texturing",
];

/// The browser's WebGL 2 context. Elsewhere, there is nothing in it.
pub struct WebGl {
    #[cfg(safegl_browser)]
    gl: WebGl2RenderingContext,

    /// The objects of the extensions that SafeGL enabled, which have their functions.
    #[cfg(safegl_browser)]
    extensions: Vec<(&'static str, js_sys::Object)>,
}

#[cfg(not(safegl_browser))]
impl WebGl {
    /// Nothing, outside of browsers.
    pub fn new() -> WebGl {
        WebGl {}
    }
}

#[cfg(safegl_browser)]
impl WebGl {
    /// Wraps the browser's context, and enables the extensions that SafeGL uses and that the
    /// browser has, except the ones that `hidden` names.
    pub fn new(gl: WebGl2RenderingContext, hidden: &[String]) -> WebGl {
        let supported = gl.get_supported_extensions().unwrap_or_default();
        let supported: Vec<String> = supported.iter().filter_map(|name| name.as_string()).collect();
        let extensions = EXTENSIONS
            .iter()
            .filter(|&&name| supported.iter().any(|supported| supported == name))
            .filter(|&&name| !hidden.iter().any(|hidden| hidden == name))
            .filter_map(|&name| Some((name, gl.get_extension(name).ok()??)))
            .collect();
        WebGl { gl, extensions }
    }

    /// Calls the function `method` of the object of `extension`, which SafeGL must have enabled.
    pub fn call(&self, extension: &str, method: &str, args: &[JsValue]) -> JsValue {
        let object = self
            .extensions
            .iter()
            .find(|(name, _)| *name == extension)
            .map(|(_, object)| object)
            .unwrap_or_else(|| panic!("SafeGL called {} without {}", method, extension));
        js_sys::Reflect::get(object, &JsValue::from_str(method))
            .ok()
            .and_then(|function| function.dyn_into::<js_sys::Function>().ok())
            .and_then(|function| {
                let args: js_sys::Array = args.iter().collect();
                function.apply(object, &args).ok()
            })
            .unwrap_or_else(|| panic!("{} has no function {}", extension, method))
    }

    /// `glPolygonOffsetClampEXT` (`EXT_polygon_offset_clamp`).
    pub fn polygon_offset_clamp(&self, factor: f32, units: f32, clamp: f32) {
        let args = [factor.into(), units.into(), clamp.into()];
        self.call("EXT_polygon_offset_clamp", "polygonOffsetClampEXT", &args);
    }

    /// The program that `glUseProgram` made current.
    fn current_program(&self) -> Option<WebGlProgram> {
        let program = self.gl.get_parameter(glow::CURRENT_PROGRAM).ok()?;
        program.dyn_into().ok()
    }

    /// `glGetActiveUniformsiv` on the current program: the value of `pname` for each uniform of
    /// `indices`. WebGL doesn't have `GL_UNIFORM_NAME_LENGTH`.
    pub fn active_uniforms_of_current_program(&self, indices: &[u32], pname: u32) -> Vec<i32> {
        let Some(program) = self.current_program() else {
            return vec![0; indices.len()];
        };
        let indices: js_sys::Array = indices.iter().map(|&index| JsValue::from(index)).collect();
        let values = self.gl.get_active_uniforms(&program, &indices, pname);
        // numbers, or booleans for `GL_UNIFORM_IS_ROW_MAJOR`
        js_sys::Array::from(&values)
            .iter()
            .map(|value| match value.as_bool() {
                Some(value) => value as i32,
                None => value.as_f64().unwrap_or(0.0) as i32,
            })
            .collect()
    }

    /// The sample counts of a renderable format for render buffers, the only target of WebGL's
    /// `getInternalformatParameter`, from the largest one.
    pub fn render_buffer_samples(&self, internal_format: u32) -> Vec<i32> {
        self.gl
            .get_internalformat_parameter(glow::RENDERBUFFER, internal_format, glow::SAMPLES)
            .ok()
            .and_then(|samples| samples.dyn_into::<js_sys::Int32Array>().ok())
            .map(|samples| samples.to_vec())
            .unwrap_or_default()
    }

    /// A string of `WEBGL_debug_renderer_info`: `UNMASKED_VENDOR_WEBGL` or
    /// `UNMASKED_RENDERER_WEBGL`.
    pub fn unmasked_string(&self, name: u32) -> Option<String> {
        self.gl.get_parameter(name).ok()?.as_string()
    }
}
