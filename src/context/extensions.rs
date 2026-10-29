
use crate::version::is_webgl;

macro_rules! extensions {
    ($($string:expr => $field:ident,)+) => {
        /// Contains data about the list of extensions.
        ///
        /// WebGL names its extensions without the `GL_` prefix: the ones that OpenGL ES has too
        /// set the same fields (`EXT_color_buffer_float` sets `gl_ext_color_buffer_float`), and
        /// the ones that only WebGL has set the fields of their own name
        /// (`WEBGL_multi_draw` sets `webgl_multi_draw`).
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[non_exhaustive]
        pub struct ExtensionsList {
            $(
                #[allow(missing_docs)]
                pub $field: bool,
            )+
        }

        /// Returns the list of extensions supported by the backend.
        ///
        /// The version must match the one of the backend.
        ///
        /// *Safety*: the OpenGL context corresponding to `gl` must be current in the thread.
        ///
        /// ## Panic
        ///
        /// Can panic if the version number doesn't match the backend, leading to unloaded functions
        /// being called.
        ///
        pub fn get_extensions(gl: &glow::Context) -> ExtensionsList {
            let strings = glow::HasContext::supported_extensions(gl);

            let mut extensions = ExtensionsList {
                $(
                    $field: false,
                )+
            };

            let hidden = hidden_extensions();
            for extension in strings.iter() {
                if hidden.iter().any(|name| name == extension) {
                    continue;
                }
                // the names of browsers have no `GL_` prefix
                let prefixed = if is_webgl() {
                    format!("GL_{}", extension)
                } else {
                    String::new()
                };
                for name in [&extension[..], &prefixed[..]] {
                    match name {
                        $(
                            $string => extensions.$field = true,
                        )+
                        _ => ()
                    }
                }
            }

            extensions
        }
    }
}

/// The extensions that the context must treat as missing, from the space-separated names of
/// `SAFEGL_DISABLE_EXTENSIONS`, so that tests run the paths of drivers without them on drivers
/// that can't hide them (Mesa can't disable some extensions). Only debug builds read it: from the
/// environment of the program, or, in browsers, which have no environment, from the environment
/// of the build.
pub(crate) fn hidden_extensions() -> Vec<String> {
    if !cfg!(debug_assertions) {
        return Vec::new();
    }
    let names = std::env::var("SAFEGL_DISABLE_EXTENSIONS")
        .ok()
        .or_else(|| option_env!("SAFEGL_DISABLE_EXTENSIONS").map(str::to_owned));
    names
        .map(|names| names.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default()
}

extensions! {
    "GL_AMD_depth_clamp_separate" => gl_amd_depth_clamp_separate,
    "GL_AMD_query_buffer_object" => gl_amd_query_buffer_object,
    "GL_ARB_base_instance" => gl_arb_base_instance,
    "GL_ARB_bindless_texture" => gl_arb_bindless_texture,
    "GL_ARB_buffer_storage" => gl_arb_buffer_storage,
    "GL_ARB_clear_texture" => gl_arb_clear_texture,
    "GL_ARB_clip_control" => gl_arb_clip_control,
    "GL_ARB_cull_distance" => gl_arb_cull_distance,
    "GL_ARB_compressed_texture_pixel_storage" => gl_arb_compressed_texture_pixel_storage,
    "GL_ARB_compute_shader" => gl_arb_compute_shader,
    "GL_ARB_conditional_render_inverted" => gl_arb_conditional_render_inverted,
    "GL_ARB_copy_image" => gl_arb_copy_image,
    "GL_ARB_debug_output" => gl_arb_debug_output,
    "GL_ARB_clear_buffer_object" => gl_arb_clear_buffer_object,
    "GL_ARB_direct_state_access" => gl_arb_direct_state_access,
    "GL_ARB_ES2_compatibility" => gl_arb_es2_compatibility,
    "GL_ARB_enhanced_layouts" => gl_arb_enhanced_layouts,
    "GL_ARB_ES3_compatibility" => gl_arb_es3_compatibility,
    "GL_ARB_explicit_uniform_location" => gl_arb_explicit_uniform_location,
    "GL_ARB_ES3_1_compatibility" => gl_arb_es3_1_compatibility,
    "GL_ARB_ES3_2_compatibility" => gl_arb_es3_2_compatibility,
    "GL_ARB_framebuffer_no_attachments" => gl_arb_framebuffer_no_attachments,
    "GL_ARB_get_program_binary" => gl_arb_get_program_binary,
    "GL_ARB_gl_spirv" => gl_arb_gl_spirv,
    "GL_ARB_spirv_extensions" => gl_arb_spirv_extensions,
    "GL_ARB_gpu_shader_int64" => gl_arb_gpu_shader_int64,
    "GL_ARB_internalformat_query" => gl_arb_internalformat_query,
    "GL_ARB_internalformat_query2" => gl_arb_internalformat_query2,
    "GL_ARB_map_buffer_alignment" => gl_arb_map_buffer_alignment,
    "GL_ARB_get_texture_sub_image" => gl_arb_get_texture_sub_image,
    "GL_ARB_invalidate_subdata" => gl_arb_invalidate_subdata,
    "GL_ARB_polygon_offset_clamp" => gl_arb_polygon_offset_clamp,
    "GL_ARB_program_interface_query" => gl_arb_program_interface_query,
    "GL_ARB_query_buffer_object" => gl_arb_query_buffer_object,
    "GL_ARB_indirect_parameters" => gl_arb_indirect_parameters,
    "GL_ARB_multi_draw_indirect" => gl_arb_multi_draw_indirect,
    "GL_ARB_pipeline_statistics_query" => gl_arb_pipeline_statistics_query,
    "GL_ARB_transform_feedback_overflow_query" => gl_arb_transform_feedback_overflow_query,
    "GL_ARB_robustness" => gl_arb_robustness,
    "GL_ARB_robust_buffer_access_behavior" => gl_arb_robust_buffer_access_behavior,
    "GL_ARB_separate_shader_objects" => gl_arb_separate_shader_objects,
    "GL_ARB_shader_atomic_counters" => gl_arb_shader_atomic_counters,
    "GL_ARB_shader_draw_parameters" => gl_arb_shader_draw_parameters,
    "GL_ARB_shader_image_load_store" => gl_arb_shader_image_load_store,
    "GL_ARB_shader_storage_buffer_object" => gl_arb_shader_storage_buffer_object,
    "GL_ARB_stencil_texturing" => gl_arb_stencil_texturing,
    "GL_ARB_texture_barrier" => gl_arb_texture_barrier,
    "GL_ARB_texture_buffer_range" => gl_arb_texture_buffer_range,
    "GL_ARB_texture_compression_bptc" => gl_arb_texture_compression_bptc,
    "GL_ARB_texture_stencil8" => gl_arb_texture_stencil8,
    "GL_ARB_texture_storage" => gl_arb_texture_storage,
    "GL_ARB_texture_view" => gl_arb_texture_view,
    "GL_ARB_texture_storage_multisample" => gl_arb_texture_storage_multisample,
    "GL_ARB_transform_feedback_instanced" => gl_arb_transform_feedback_instanced,
    "GL_ARB_vertex_attrib_64bit" => gl_arb_vertex_attrib_64bit,
    "GL_ARB_vertex_attrib_binding" => gl_arb_vertex_attrib_binding,
    "GL_ARB_viewport_array" => gl_arb_viewport_array,
    "GL_ARB_vertex_type_10f_11f_11f_rev" => gl_arb_vertex_type_10f_11f_11f_rev,
    "GL_ATI_meminfo" => gl_ati_meminfo,
    "GL_EXT_base_instance" => gl_ext_base_instance,
    "GL_EXT_blend_func_extended" => gl_ext_blend_func_extended,
    "GL_EXT_buffer_storage" => gl_ext_buffer_storage,
    "GL_EXT_clear_texture" => gl_ext_clear_texture,
    "GL_EXT_clip_control" => gl_ext_clip_control,
    "GL_EXT_clip_cull_distance" => gl_ext_clip_cull_distance,
    "GL_EXT_compressed_ETC1_RGB8_sub_texture" => gl_ext_compressed_etc1_rgb8_sub_texture,
    "GL_EXT_conservative_depth" => gl_ext_conservative_depth,
    "GL_EXT_copy_image" => gl_ext_copy_image,
    "GL_EXT_draw_buffers_indexed" => gl_ext_draw_buffers_indexed,
    "GL_EXT_draw_elements_base_vertex" => gl_ext_draw_elements_base_vertex,
    "GL_OES_copy_image" => gl_oes_copy_image,
    "GL_EXT_depth_clamp" => gl_ext_depth_clamp,
    "GL_EXT_tessellation_shader" => gl_ext_tessellation_shader,
    "GL_EXT_color_buffer_float" => gl_ext_color_buffer_float,
    "GL_EXT_color_buffer_half_float" => gl_ext_color_buffer_half_float,
    "GL_EXT_debug_label" => gl_ext_debug_label,
    "GL_EXT_debug_marker" => gl_ext_debug_marker,
    "GL_EXT_direct_state_access" => gl_ext_direct_state_access,
    "GL_EXT_memory_object" => gl_ext_memory_object,
    "GL_EXT_memory_object_fd" => gl_ext_memory_object_fd,
    "GL_EXT_disjoint_timer_query" => gl_ext_disjoint_timer_query,
    "GL_EXT_float_blend" => gl_ext_float_blend,
    "GL_EXT_geometry_shader" => gl_ext_geometry_shader,
    "GL_EXT_multi_draw_arrays" => gl_ext_multi_draw_arrays,
    "GL_EXT_multi_draw_indirect" => gl_ext_multi_draw_indirect,
    "GL_EXT_multisampled_render_to_texture" => gl_ext_multisampled_render_to_texture,
    "GL_EXT_multisampled_render_to_texture2" => gl_ext_multisampled_render_to_texture2,
    "GL_EXT_occlusion_query_boolean" => gl_ext_occlusion_query_boolean,
    "GL_EXT_polygon_offset_clamp" => gl_ext_polygon_offset_clamp,
    "GL_EXT_primitive_bounding_box" => gl_ext_primitive_bounding_box,
    "GL_EXT_provoking_vertex" => gl_ext_provoking_vertex,
    "GL_EXT_render_snorm" => gl_ext_render_snorm,
    "GL_EXT_robustness" => gl_ext_robustness,
    "GL_EXT_sRGB_write_control" => gl_ext_srgb_write_control,
    "GL_EXT_semaphore" => gl_ext_semaphore,
    "GL_EXT_semaphore_fd" => gl_ext_semaphore_fd,
    "GL_EXT_texture_border_clamp" => gl_ext_texture_border_clamp,
    "GL_EXT_texture_buffer" => gl_ext_texture_buffer,
    "GL_EXT_texture_compression_astc_decode_mode" => gl_ext_texture_compression_astc_decode_mode,
    "GL_EXT_texture_compression_astc_decode_mode_rgb9e5" =>
        gl_ext_texture_compression_astc_decode_mode_rgb9e5,
    "GL_EXT_texture_compression_bptc" => gl_ext_texture_compression_bptc,
    "GL_EXT_texture_compression_rgtc" => gl_ext_texture_compression_rgtc,
    "GL_EXT_texture_compression_s3tc" => gl_ext_texture_compression_s3tc,
    "GL_EXT_texture_cube_map_array" => gl_ext_texture_cube_map_array,
    "GL_EXT_texture_filter_anisotropic" => gl_ext_texture_filter_anisotropic,
    "GL_ARB_texture_filter_anisotropic" => gl_arb_texture_filter_anisotropic,
    "GL_EXT_texture_mirror_clamp_to_edge" => gl_ext_texture_mirror_clamp_to_edge,
    "GL_ARB_texture_mirror_clamp_to_edge" => gl_arb_texture_mirror_clamp_to_edge,
    "GL_EXT_texture_norm16" => gl_ext_texture_norm16,
    "GL_EXT_texture_sRGB" => gl_ext_texture_srgb,
    "GL_EXT_texture_format_sRGB_override" => gl_ext_texture_format_srgb_override,
    "GL_EXT_texture_sRGB_decode" => gl_ext_texture_srgb_decode,
    "GL_EXT_texture_sRGB_R8" => gl_ext_texture_srgb_r8,
    "GL_EXT_texture_sRGB_RG8" => gl_ext_texture_srgb_rg8,
    "GL_EXT_texture_view" => gl_ext_texture_view,
    "GL_OES_texture_view" => gl_oes_texture_view,
    "GL_OES_viewport_array" => gl_oes_viewport_array,
    "GL_KHR_debug" => gl_khr_debug,
    "GL_IMG_texture_compression_pvrtc" => gl_img_texture_compression_pvrtc,
    "GL_IMG_texture_compression_pvrtc2" => gl_img_texture_compression_pvrtc2,
    "GL_EXT_pvrtc_sRGB" => gl_ext_pvrtc_srgb,
    "GL_AMD_compressed_ATC_texture" => gl_amd_compressed_atc_texture,
    "GL_KHR_no_error" => gl_khr_no_error,
    "GL_KHR_parallel_shader_compile" => gl_khr_parallel_shader_compile,
    "GL_ARB_parallel_shader_compile" => gl_arb_parallel_shader_compile,
    "GL_KHR_context_flush_control" => gl_khr_context_flush_control,
    "GL_KHR_robustness" => gl_khr_robustness,
    "GL_KHR_texture_compression_astc_hdr" => gl_khr_texture_compression_astc_hdr,
    "GL_KHR_texture_compression_astc_ldr" => gl_khr_texture_compression_astc_ldr,
    "GL_KHR_texture_compression_astc_sliced_3d" => gl_khr_texture_compression_astc_sliced_3d,
    "GL_OES_texture_compression_astc" => gl_oes_texture_compression_astc,
    "GL_KHR_robust_buffer_access_behavior" => gl_khr_robust_buffer_access_behavior,
    "GL_NV_conditional_render" => gl_nv_conditional_render,
    "GL_NV_internalformat_sample_query" => gl_nv_internalformat_sample_query,
    "GL_NV_read_depth" => gl_nv_read_depth,
    "GL_NV_read_stencil" => gl_nv_read_stencil,
    "GL_NV_read_depth_stencil" => gl_nv_read_depth_stencil,
    "GL_NV_shader_atomic_counters" => gl_nv_shader_atomic_counters,
    "GL_NV_shader_storage_buffer_object" => gl_nv_shader_storage_buffer_object,
    "GL_NV_texture_barrier" => gl_nv_texture_barrier,
    "GL_NV_vertex_attrib_integer_64bit" => gl_nv_vertex_attrib_integer_64bit,
    "GL_NVX_gpu_memory_info" => gl_nvx_gpu_memory_info,
    "GL_OES_compressed_ETC1_RGB8_texture" => gl_oes_compressed_etc1_rgb8_texture,
    "GL_OES_draw_buffers_indexed" => gl_oes_draw_buffers_indexed,
    "GL_OES_draw_elements_base_vertex" => gl_oes_draw_elements_base_vertex,
    "GL_OES_geometry_shader" => gl_oes_geometry_shader,
    "GL_OES_primitive_bounding_box" => gl_oes_primitive_bounding_box,
    "GL_OES_sample_shading" => gl_oes_sample_shading,
    "GL_OES_sample_variables" => gl_oes_sample_variables,
    "GL_OES_shader_multisample_interpolation" => gl_oes_shader_multisample_interpolation,
    "GL_OES_tessellation_shader" => gl_oes_tessellation_shader,
    "GL_OES_texture_border_clamp" => gl_oes_texture_border_clamp,
    "GL_OES_texture_buffer" => gl_oes_texture_buffer,
    "GL_OES_texture_cube_map_array" => gl_oes_texture_cube_map_array,
    "GL_OES_texture_float_linear" => gl_oes_texture_float_linear,
    "GL_OES_texture_stencil8" => gl_oes_texture_stencil8,
    "GL_OES_texture_storage_multisample_2d_array" => gl_oes_texture_storage_multisample_2d_array,
    "GL_OVR_multiview2" => gl_ovr_multiview2,
    "GL_KHR_blend_equation_advanced" => gl_khr_blend_equation_advanced,
    "GL_KHR_blend_equation_advanced_coherent" => gl_khr_blend_equation_advanced_coherent,
    "GL_EXT_shader_framebuffer_fetch_non_coherent" =>
        gl_ext_shader_framebuffer_fetch_non_coherent,
    "GL_EXT_shader_pixel_local_storage" => gl_ext_shader_pixel_local_storage,
    "GL_EXT_shader_pixel_local_storage2" => gl_ext_shader_pixel_local_storage2,
    "GL_QCOM_tiled_rendering" => gl_qcom_tiled_rendering,
    // extensions of GLSL ES only, which shaders enable with `#extension`
    "GL_EXT_gpu_shader5" => gl_ext_gpu_shader5,
    "GL_OES_gpu_shader5" => gl_oes_gpu_shader5,
    "GL_EXT_shader_io_blocks" => gl_ext_shader_io_blocks,
    "GL_OES_shader_io_blocks" => gl_oes_shader_io_blocks,
    "GL_OES_shader_image_atomic" => gl_oes_shader_image_atomic,
    "GL_EXT_shader_non_constant_global_initializers" =>
        gl_ext_shader_non_constant_global_initializers,
    "GL_NV_shader_noperspective_interpolation" => gl_nv_shader_noperspective_interpolation,
    "GL_EXT_geometry_point_size" => gl_ext_geometry_point_size,
    "GL_EXT_tessellation_point_size" => gl_ext_tessellation_point_size,
    "GL_EXT_shader_group_vote" => gl_ext_shader_group_vote,
    "GL_EXT_shader_integer_mix" => gl_ext_shader_integer_mix,
    "GL_EXT_texture_shadow_lod" => gl_ext_texture_shadow_lod,
    "GL_EXT_shader_framebuffer_fetch" => gl_ext_shader_framebuffer_fetch,
    "GL_ARM_shader_framebuffer_fetch" => gl_arm_shader_framebuffer_fetch,
    "GL_ARM_shader_framebuffer_fetch_depth_stencil" =>
        gl_arm_shader_framebuffer_fetch_depth_stencil,
    // the extensions that only WebGL has
    "EXT_disjoint_timer_query_webgl2" => ext_disjoint_timer_query_webgl2,
    "WEBGL_blend_func_extended" => webgl_blend_func_extended,
    "WEBGL_clip_cull_distance" => webgl_clip_cull_distance,
    "WEBGL_compressed_texture_astc" => webgl_compressed_texture_astc,
    "WEBGL_compressed_texture_etc" => webgl_compressed_texture_etc,
    "WEBGL_compressed_texture_etc1" => webgl_compressed_texture_etc1,
    "WEBGL_compressed_texture_pvrtc" => webgl_compressed_texture_pvrtc,
    "WEBGL_compressed_texture_s3tc" => webgl_compressed_texture_s3tc,
    "WEBGL_compressed_texture_s3tc_srgb" => webgl_compressed_texture_s3tc_srgb,
    "WEBGL_debug_renderer_info" => webgl_debug_renderer_info,
    "WEBGL_draw_instanced_base_vertex_base_instance" =>
        webgl_draw_instanced_base_vertex_base_instance,
    "WEBGL_lose_context" => webgl_lose_context,
    "WEBGL_multi_draw" => webgl_multi_draw,
    "WEBGL_multi_draw_instanced_base_vertex_base_instance" =>
        webgl_multi_draw_instanced_base_vertex_base_instance,
    "WEBGL_polygon_mode" => webgl_polygon_mode,
    "WEBGL_provoking_vertex" => webgl_provoking_vertex,
    "WEBGL_render_shared_exponent" => webgl_render_shared_exponent,
    "WEBGL_stencil_texturing" => webgl_stencil_texturing,
}

impl ExtensionsList {
    /// Returns true if the backend has the timer queries of OpenGL ES
    /// (`GL_EXT_disjoint_timer_query`) or of WebGL (`EXT_disjoint_timer_query_webgl2`).
    pub(crate) fn disjoint_timer_query(&self) -> bool {
        self.gl_ext_disjoint_timer_query || self.ext_disjoint_timer_query_webgl2
    }
}
