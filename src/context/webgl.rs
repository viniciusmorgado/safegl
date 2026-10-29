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
    "EXT_texture_sRGB_decode",
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

    /// `glFramebufferTextureMultiviewOVR` (`OVR_multiview2`): attaches the views of the level
    /// `level` of the 2D array texture bound to `GL_TEXTURE_2D_ARRAY` to the draw framebuffer.
    pub fn framebuffer_texture_multiview(
        &self,
        attachment: u32,
        level: u32,
        base_view: u32,
        views: u32,
    ) {
        let texture = self
            .gl
            .get_parameter(glow::TEXTURE_BINDING_2D_ARRAY)
            .unwrap_or(JsValue::NULL);
        let args = [
            glow::DRAW_FRAMEBUFFER.into(),
            attachment.into(),
            texture,
            level.into(),
            base_view.into(),
            views.into(),
        ];
        self.call("OVR_multiview2", "framebufferTextureMultiviewOVR", &args);
    }

    /// `glClipControlEXT` (`EXT_clip_control`).
    pub fn clip_control(&self, origin: u32, depth: u32) {
        self.call("EXT_clip_control", "clipControlEXT", &[origin.into(), depth.into()]);
    }

    /// `glDrawArraysInstancedBaseInstance` (`WEBGL_draw_instanced_base_vertex_base_instance`).
    pub fn draw_arrays_instanced_base_instance(
        &self,
        mode: u32,
        first: i32,
        count: i32,
        instances: i32,
        base_instance: u32,
    ) {
        let args = [
            mode.into(),
            first.into(),
            count.into(),
            instances.into(),
            base_instance.into(),
        ];
        self.call(
            "WEBGL_draw_instanced_base_vertex_base_instance",
            "drawArraysInstancedBaseInstanceWEBGL",
            &args,
        );
    }

    /// `glDrawElementsInstancedBaseVertexBaseInstance`
    /// (`WEBGL_draw_instanced_base_vertex_base_instance`), the indices at `offset` bytes in the
    /// bound index buffer.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_elements_instanced_base_vertex_base_instance(
        &self,
        mode: u32,
        count: i32,
        ty: u32,
        offset: i32,
        instances: i32,
        base_vertex: i32,
        base_instance: u32,
    ) {
        let args = [
            mode.into(),
            count.into(),
            ty.into(),
            offset.into(),
            instances.into(),
            base_vertex.into(),
            base_instance.into(),
        ];
        self.call(
            "WEBGL_draw_instanced_base_vertex_base_instance",
            "drawElementsInstancedBaseVertexBaseInstanceWEBGL",
            &args,
        );
    }

    /// `glMultiDrawArrays` (`WEBGL_multi_draw`).
    pub fn multi_draw_arrays(&self, mode: u32, firsts: &[i32], counts: &[i32]) {
        let args = [
            mode.into(),
            js_sys::Int32Array::from(firsts).into(),
            0.into(),
            js_sys::Int32Array::from(counts).into(),
            0.into(),
            (counts.len() as i32).into(),
        ];
        self.call("WEBGL_multi_draw", "multiDrawArraysWEBGL", &args);
    }

    /// `glMultiDrawElements` (`WEBGL_multi_draw`), the indices at `offsets` bytes in the bound
    /// index buffer.
    pub fn multi_draw_elements(&self, mode: u32, counts: &[i32], ty: u32, offsets: &[i32]) {
        let args = [
            mode.into(),
            js_sys::Int32Array::from(counts).into(),
            0.into(),
            ty.into(),
            js_sys::Int32Array::from(offsets).into(),
            0.into(),
            (counts.len() as i32).into(),
        ];
        self.call("WEBGL_multi_draw", "multiDrawElementsWEBGL", &args);
    }

    /// `glEnablei` or `glDisablei` (`OES_draw_buffers_indexed`).
    pub fn enable_indexed(&self, capability: u32, index: u32, enabled: bool) {
        let method = if enabled { "enableiOES" } else { "disableiOES" };
        let args = [capability.into(), index.into()];
        self.call("OES_draw_buffers_indexed", method, &args);
    }

    /// `glBlendEquationSeparatei` (`OES_draw_buffers_indexed`).
    pub fn blend_equation_indexed(&self, index: u32, color: u32, alpha: u32) {
        let args = [index.into(), color.into(), alpha.into()];
        self.call("OES_draw_buffers_indexed", "blendEquationSeparateiOES", &args);
    }

    /// `glBlendFuncSeparatei` (`OES_draw_buffers_indexed`): the source and destination factors
    /// of the colors, then of the alpha.
    pub fn blend_func_indexed(&self, index: u32, factors: (u32, u32, u32, u32)) {
        let (a, b, c, d) = factors;
        let args = [index.into(), a.into(), b.into(), c.into(), d.into()];
        self.call("OES_draw_buffers_indexed", "blendFuncSeparateiOES", &args);
    }

    /// `glColorMaski` (`OES_draw_buffers_indexed`).
    pub fn color_mask_indexed(&self, index: u32, mask: (bool, bool, bool, bool)) {
        let (r, g, b, a) = mask;
        let args = [index.into(), r.into(), g.into(), b.into(), a.into()];
        self.call("OES_draw_buffers_indexed", "colorMaskiOES", &args);
    }

    /// `glPolygonMode` for both faces (`WEBGL_polygon_mode`).
    pub fn polygon_mode(&self, mode: u32) {
        let args = [glow::FRONT_AND_BACK.into(), mode.into()];
        self.call("WEBGL_polygon_mode", "polygonModeWEBGL", &args);
    }

    /// `glProvokingVertex` (`WEBGL_provoking_vertex`).
    pub fn provoking_vertex(&self, convention: u32) {
        self.call(
            "WEBGL_provoking_vertex",
            "provokingVertexWEBGL",
            &[convention.into()],
        );
    }

    /// `glPolygonOffsetClampEXT` (`EXT_polygon_offset_clamp`).
    pub fn polygon_offset_clamp(&self, factor: f32, units: f32, clamp: f32) {
        let args = [factor.into(), units.into(), clamp.into()];
        self.call("EXT_polygon_offset_clamp", "polygonOffsetClampEXT", &args);
    }

    /// The browser's context.
    pub fn context(&self) -> &WebGl2RenderingContext {
        &self.gl
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

    /// `glGetFragDataIndex` on the current program (`WEBGL_blend_func_extended`): 1 for the
    /// second color output of dual-source blending, 0 for the others, -1 for none.
    pub fn frag_data_index_of_current_program(&self, name: &str) -> i32 {
        let Some(program) = self.current_program() else {
            return -1;
        };
        let args = [JsValue::from(program), JsValue::from_str(name)];
        let index = self.call("WEBGL_blend_func_extended", "getFragDataIndexWEBGL", &args);
        index.as_f64().unwrap_or(-1.0) as i32
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
