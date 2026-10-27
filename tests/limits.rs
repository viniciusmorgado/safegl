//! The limits of the context: every value at least the minimum of the specification, and `None`
//! exactly when the backend lacks the feature that brings it.

#[macro_use]
extern crate safegl;

use safegl::framebuffer::{RenderBuffer, RenderBufferCreationError};
use safegl::texture::UncompressedFloatFormat;
use safegl::{Api, CapabilitiesSource, Limits, StageLimits, Version};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

/// Desktop OpenGL `major.minor` or above.
fn gl(display: &Display, major: u8, minor: u8) -> bool {
    display.get_opengl_version() >= &Version(Api::Gl, major, minor)
}

/// OpenGL ES `major.minor` or above.
// unused at the OpenGL ES 3.0 level
#[allow(dead_code)]
fn gles(display: &Display, major: u8, minor: u8) -> bool {
    display.get_opengl_version() >= &Version(Api::GlEs, major, minor)
}

/// The minimum of the specification of the API of the display.
fn min(display: &Display, desktop: i64, es: i64) -> i64 {
    if support::is_gles(display) {
        es
    } else {
        desktop
    }
}

/// Checks the limits that every stage has.
fn check_stage(display: &Display, stage: &StageLimits) {
    assert!(stage.max_uniform_components >= min(display, 512, 512) as i32);
    assert!(stage.max_combined_uniform_components >= stage.max_uniform_components as i64);
    assert!(stage.max_uniform_blocks >= 12);
    assert!(stage.max_texture_image_units >= 16);
    if let Some(input) = stage.max_input_components {
        assert!(input >= 60);
    }
    if let Some(output) = stage.max_output_components {
        assert!(output >= 64);
    }
}

#[test]
fn stage_limits() {
    let display = support::build_display();
    let limits: &Limits = &display.get_capabilities().limits;

    check_stage(&display, &limits.vertex);
    assert_eq!(limits.vertex.max_input_components, None);
    assert!(limits.vertex.max_output_components.is_some());
    check_stage(&display, &limits.fragment);
    assert!(limits.fragment.max_input_components.is_some());
    assert_eq!(limits.fragment.max_output_components, None);
    assert!(limits.max_combined_uniform_blocks >= min(&display, 60, 24) as i32);

    #[cfg(any(safegl_desktop, safegl_gles32))]
    {
        let extensions = display.get_extensions();
        let tessellation = !support::is_gles(&display)
            || gles(&display, 3, 2)
            || extensions.gl_ext_tessellation_shader
            || extensions.gl_oes_tessellation_shader;
        assert_eq!(limits.tess_control.is_some(), tessellation);
        assert_eq!(limits.tess_evaluation.is_some(), tessellation);
        assert_eq!(limits.max_tess_gen_level.is_some(), tessellation);
        for stage in limits.tess_control.iter().chain(&limits.tess_evaluation) {
            check_stage(&display, stage);
            assert!(stage.max_input_components.is_some());
            assert!(stage.max_output_components.is_some());
        }
        if tessellation {
            assert!(limits.max_tess_gen_level.unwrap() >= 64);
            assert!(limits.max_tess_patch_components.unwrap() >= 120);
            assert!(limits.max_tess_control_total_output_components.unwrap() >= 2048);
        }

        let geometry = !support::is_gles(&display)
            || gles(&display, 3, 2)
            || extensions.gl_ext_geometry_shader
            || extensions.gl_oes_geometry_shader;
        assert_eq!(limits.geometry.is_some(), geometry);
        if let Some(stage) = &limits.geometry {
            check_stage(&display, stage);
            assert!(limits.max_geometry_output_vertices.unwrap() >= 256);
            assert!(limits.max_geometry_total_output_components.unwrap() >= 1024);
            assert!(limits.max_geometry_shader_invocations.unwrap() >= 32);
        }
    }

    #[cfg(any(safegl_gl43, safegl_gles31))]
    {
        let extensions = display.get_extensions();
        let compute =
            gl(&display, 4, 3) || gles(&display, 3, 1) || extensions.gl_arb_compute_shader;
        assert_eq!(limits.compute.is_some(), compute);
        assert_eq!(limits.max_compute_work_group_size.is_some(), compute);
        if let Some(stage) = &limits.compute {
            assert!(stage.max_uniform_components >= 512);
            assert!(stage.max_uniform_blocks >= 12);
            assert!(stage.max_texture_image_units >= 16);
            assert_eq!(stage.max_input_components, None);
            assert_eq!(stage.max_output_components, None);
            assert!(stage.max_shader_storage_blocks.unwrap() >= 4);
            assert!(stage.max_image_uniforms.unwrap() >= 4);
            assert!(stage.max_atomic_counter_buffers.unwrap() >= 1);
            assert!(stage.max_atomic_counters.unwrap() >= 8);

            let (x, y, z) = limits.max_compute_work_group_size.unwrap();
            let minimum = min(&display, 1024, 128) as i32;
            assert!(x >= minimum && y >= minimum && z >= 64);
            assert!(limits.max_compute_work_group_invocations.unwrap() >= minimum);
            assert!(
                limits.max_compute_shared_memory_size.unwrap()
                    >= min(&display, 32768, 16384) as i32
            );
        }
    }

    display.assert_no_error(None);
}

#[test]
fn general_limits() {
    let display = support::build_display();
    let limits = &display.get_capabilities().limits;

    assert!(limits.max_vertex_attribs >= 16);
    assert!(limits.max_varying_components >= 60);
    assert!(limits.max_elements_indices >= 0);
    assert!(limits.max_elements_vertices >= 0);
    assert!(limits.max_transform_feedback_interleaved_components >= 64);
    assert!(limits.max_transform_feedback_separate_attribs >= 4);
    assert!(limits.max_transform_feedback_separate_components >= 4);
    assert!(limits.subpixel_bits >= 4);
    assert!(limits.point_size_range.0 <= 1.0 && limits.point_size_range.1 >= 1.0);
    assert!(limits.aliased_line_width_range.0 <= 1.0);
    assert!(limits.aliased_line_width_range.1 >= 1.0);
    assert!(limits.max_samples >= 4);
    assert!(limits.max_renderbuffer_size >= 2048);
    assert!(limits.max_texture_lod_bias >= 2.0);
    assert!(limits.min_program_texel_offset <= -8);
    assert!(limits.max_program_texel_offset >= 7);
    assert!(limits.max_uniform_block_size >= 16384);
    // unsigned, and all ones on some drivers
    let _: u64 = limits.max_server_wait_timeout;

    #[cfg(any(safegl_gl41, safegl_gles30))]
    {
        let extensions = display.get_extensions();
        let es2_compatibility =
            support::is_gles(&display) || gl(&display, 4, 1) || extensions.gl_arb_es2_compatibility;
        assert_eq!(limits.max_varying_vectors.is_some(), es2_compatibility);
        assert_eq!(
            limits.num_shader_binary_formats.is_some(),
            es2_compatibility
        );
        if es2_compatibility {
            assert!(limits.max_varying_vectors.unwrap() >= 15);
            assert!(limits.max_vertex_uniform_vectors.unwrap() >= 256);
            assert!(limits.max_fragment_uniform_vectors.unwrap() >= 224);
            assert!(limits.num_shader_binary_formats.unwrap() >= 0);
        }

        let program_binary = support::is_gles(&display)
            || gl(&display, 4, 1)
            || extensions.gl_arb_get_program_binary;
        assert_eq!(limits.num_program_binary_formats.is_some(), program_binary);
    }

    #[cfg(any(safegl_gl43, safegl_gles30))]
    {
        let extensions = display.get_extensions();
        let es3_compatibility =
            support::is_gles(&display) || gl(&display, 4, 3) || extensions.gl_arb_es3_compatibility;
        assert_eq!(limits.max_element_index.is_some(), es3_compatibility);
        if let Some(index) = limits.max_element_index {
            assert!(index >= (1 << 24) - 1);
        }
    }

    #[cfg(any(safegl_desktop, safegl_gles31))]
    if !support::is_gles(&display) || gles(&display, 3, 1) {
        assert!(limits.max_color_texture_samples.unwrap() >= 1);
        assert!(limits.max_depth_texture_samples.unwrap() >= 1);
        assert!(limits.max_integer_samples.unwrap() >= 1);
        assert!(limits.max_sample_mask_words.unwrap() >= 1);
        assert!(limits.min_program_texture_gather_offset.unwrap() <= -8);
        assert!(limits.max_program_texture_gather_offset.unwrap() >= 7);
    } else {
        assert_eq!(limits.max_color_texture_samples, None);
        assert_eq!(limits.max_program_texture_gather_offset, None);
    }

    #[cfg(any(safegl_desktop, safegl_gles32))]
    if let Some(offset) = limits.min_fragment_interpolation_offset {
        assert!(offset <= -0.5);
        assert!(limits.max_fragment_interpolation_offset.unwrap() >= 0.5);
        assert!(limits.fragment_interpolation_offset_bits.unwrap() >= 4);
    } else {
        assert!(support::is_gles(&display) && !gles(&display, 3, 2));
    }

    #[cfg(any(safegl_gl44, safegl_gles32))]
    assert_eq!(
        limits.primitive_restart_for_patches.is_some(),
        gl(&display, 4, 4) || gles(&display, 3, 2)
    );

    display.assert_no_error(None);
}

/// The limits that only desktop OpenGL has.
#[cfg(safegl_desktop)]
#[test]
fn desktop_limits() {
    let display = support::build_display();
    let limits = &display.get_capabilities().limits;

    if support::is_gles(&display) {
        assert_eq!(limits.max_vertex_streams, None);
        assert_eq!(limits.max_subroutines, None);
        assert_eq!(limits.query_counter_bits, None);
        return;
    }

    assert!(limits.max_vertex_streams.unwrap() >= 4);
    assert!(limits.max_clip_distances.unwrap() >= 8);
    assert!(limits.point_size_granularity.unwrap() >= 0.0);
    assert!(limits.smooth_line_width_range.unwrap().1 >= 1.0);
    assert!(limits.smooth_line_width_granularity.unwrap() >= 0.0);
    assert!(limits.max_dual_source_draw_buffers.unwrap() >= 1);
    assert!(limits.max_rectangle_texture_size.unwrap() >= 1024);
    assert!(limits.max_subroutines.unwrap() >= 256);
    assert!(limits.max_subroutine_uniform_locations.unwrap() >= 1024);

    let bits = limits.query_counter_bits.unwrap();
    assert!(bits.time_elapsed >= 30);
    assert!(bits.timestamp >= 30);
    assert!(bits.samples_passed >= 0);
    assert!(bits.primitives_generated >= 0);
    assert!(bits.transform_feedback_primitives_written >= 0);

    display.assert_no_error(None);
}

/// The limits of image load and store, atomic counters and shader storage buffers.
#[cfg(any(safegl_gl42, safegl_gles31))]
#[test]
fn shader_resource_limits() {
    let display = support::build_display();
    let limits = &display.get_capabilities().limits;
    let extensions = display.get_extensions();

    let images =
        gl(&display, 4, 2) || gles(&display, 3, 1) || extensions.gl_arb_shader_image_load_store;
    assert_eq!(limits.max_image_units.is_some(), images);
    assert_eq!(limits.fragment.max_image_uniforms.is_some(), images);
    if images {
        assert!(limits.max_image_units.unwrap() >= min(&display, 8, 4) as i32);
        assert!(limits.max_combined_image_uniforms.unwrap() >= min(&display, 8, 4) as i32);
        assert!(limits.max_combined_shader_output_resources.unwrap() >= 4);
    }

    let atomic_counters =
        gl(&display, 4, 2) || gles(&display, 3, 1) || extensions.gl_arb_shader_atomic_counters;
    assert_eq!(
        limits.max_atomic_counter_buffer_size.is_some(),
        atomic_counters
    );
    if atomic_counters {
        assert!(limits.max_atomic_counter_buffer_size.unwrap() >= 32);
        assert!(limits.max_combined_atomic_counter_buffers.unwrap() >= 1);
        assert!(limits.max_combined_atomic_counters.unwrap() >= 8);
    }

    #[cfg(safegl_gl42)]
    if !support::is_gles(&display) {
        let alignment = gl(&display, 4, 2) || extensions.gl_arb_map_buffer_alignment;
        assert_eq!(limits.min_map_buffer_alignment.is_some(), alignment);
        if alignment {
            assert!(limits.min_map_buffer_alignment.unwrap() >= 64);
        }
        assert_eq!(limits.max_image_samples.is_some(), images);
    }

    #[cfg(any(safegl_gl43, safegl_gles31))]
    {
        let storage = gl(&display, 4, 3)
            || gles(&display, 3, 1)
            || extensions.gl_arb_shader_storage_buffer_object;
        assert_eq!(limits.max_shader_storage_block_size.is_some(), storage);
        if storage {
            assert!(limits.max_shader_storage_block_size.unwrap() >= 1 << 24);
            assert!(limits.max_combined_shader_storage_blocks.unwrap() >= 4);
            assert!(limits.fragment.max_shader_storage_blocks.unwrap() >= 0);
        }

        let locations = gl(&display, 4, 3)
            || gles(&display, 3, 1)
            || extensions.gl_arb_explicit_uniform_location;
        assert_eq!(limits.max_uniform_locations.is_some(), locations);
        if locations {
            assert!(limits.max_uniform_locations.unwrap() >= 1024);
        }
    }

    display.assert_no_error(None);
}

#[cfg(safegl_gl41)]
#[test]
fn viewport_limits() {
    let display = support::build_display();
    let limits = &display.get_capabilities().limits;

    let viewport_array = gl(&display, 4, 1) || display.get_extensions().gl_arb_viewport_array;
    assert_eq!(limits.max_viewports.is_some(), viewport_array);
    assert_eq!(
        limits.viewport_index_provoking_vertex.is_some(),
        viewport_array
    );
    if viewport_array {
        assert!(limits.max_viewports.unwrap() >= 16);
        assert!(limits.viewport_subpixel_bits.unwrap() >= 0);
        let (low, high) = limits.viewport_bounds_range.unwrap();
        assert!(low <= -32768.0 && high >= 32767.0);
        assert!(limits.layer_provoking_vertex.is_some());
    }

    display.assert_no_error(None);
}

#[cfg(safegl_gl45)]
#[test]
fn cull_distance_limits() {
    let display = support::build_display();
    let limits = &display.get_capabilities().limits;
    let extensions = display.get_extensions();

    let cull_distance = gl(&display, 4, 5)
        || extensions.gl_arb_cull_distance
        || extensions.gl_ext_clip_cull_distance;
    assert_eq!(limits.max_cull_distances.is_some(), cull_distance);
    if cull_distance {
        assert!(limits.max_cull_distances.unwrap() >= 8);
        assert!(limits.max_combined_clip_and_cull_distances.unwrap() >= 8);
    }

    display.assert_no_error(None);
}

#[cfg(any(safegl_gl43, safegl_gles32))]
#[test]
fn debug_limits() {
    let display = support::build_display();
    let limits = &display.get_capabilities().limits;

    let debug = gl(&display, 4, 3) || gles(&display, 3, 2) || display.get_extensions().gl_khr_debug;
    assert_eq!(limits.max_label_length.is_some(), debug);
    if debug {
        assert!(limits.max_debug_message_length.unwrap() >= 1);
        assert!(limits.max_debug_logged_messages.unwrap() >= 1);
        assert!(limits.max_debug_group_stack_depth.unwrap() >= 64);
        assert!(limits.max_label_length.unwrap() >= 256);
    }

    if let Some(alignment) = limits.texture_buffer_offset_alignment {
        assert!((1..=256).contains(&alignment));
    }

    display.assert_no_error(None);
}

#[test]
fn render_buffer_above_limits() {
    let display = support::build_display();
    let limits = &display.get_capabilities().limits;
    let format = UncompressedFloatFormat::U8U8U8U8;

    let size = limits.max_renderbuffer_size as u32 + 1;
    assert_eq!(
        RenderBuffer::new(&display, format, size, 1).err(),
        Some(RenderBufferCreationError::DimensionsNotSupported)
    );
    assert_eq!(
        RenderBuffer::new(&display, format, 1, size).err(),
        Some(RenderBufferCreationError::DimensionsNotSupported)
    );

    let samples = limits.max_samples as u32 + 1;
    assert_eq!(
        RenderBuffer::new_multisample(&display, format, 4, 4, samples).err(),
        Some(RenderBufferCreationError::SamplesNotSupported)
    );

    RenderBuffer::new(&display, format, 4, 4).unwrap();
    RenderBuffer::new_multisample(&display, format, 4, 4, 4).unwrap();

    display.assert_no_error(None);
}
