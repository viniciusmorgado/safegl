//! Makes the OpenGL state match the state that SafeGL has cached, after code outside of SafeGL
//! changed it.

use glow::HasContext;

use crate::context::CommandContext;
use crate::gl;
use crate::version::{is_webgl, Api, Version};

/// Issues the OpenGL calls that bring every piece of state that SafeGL caches back to the cached
/// value, or forgets the cached value where SafeGL sets the state again before each use anyway.
///
/// Running queries, transform feedback and conditional rendering aren't restored.
pub(crate) unsafe fn restore_state(ctxt: &mut CommandContext<'_>) {
    restore_capabilities(ctxt);
    restore_bindings(ctxt);
    restore_fixed_function(ctxt);

    // the other code may have bound an element array buffer, or changed the attributes, while a
    // vertex array object of SafeGL was bound: they are created again when needed
    crate::vertex_array_object::VertexAttributesSystem::purge_all(ctxt);

    // and it may have changed the uniforms of SafeGL's programs
    ctxt.state.uniform_values_generation += 1;
}

/// Enables or disables `cap`.
unsafe fn set(ctxt: &CommandContext<'_>, cap: gl::types::GLenum, enabled: bool) {
    if enabled {
        ctxt.glow.enable(cap);
    } else {
        ctxt.glow.disable(cap);
    }
}

unsafe fn restore_capabilities(ctxt: &mut CommandContext<'_>) {
    let state = &*ctxt.state;
    let desktop = ctxt.version.0 == Api::Gl;

    set(ctxt, gl::BLEND, state.enabled_blend);
    set(ctxt, gl::CULL_FACE, state.enabled_cull_face);
    set(ctxt, gl::DEPTH_TEST, state.enabled_depth_test);
    set(ctxt, gl::DITHER, state.enabled_dither);
    set(
        ctxt,
        gl::POLYGON_OFFSET_FILL,
        state.enabled_polygon_offset_fill,
    );
    set(
        ctxt,
        gl::RASTERIZER_DISCARD,
        state.enabled_rasterizer_discard,
    );
    set(
        ctxt,
        gl::SAMPLE_ALPHA_TO_COVERAGE,
        state.enabled_sample_alpha_to_coverage,
    );
    set(ctxt, gl::SAMPLE_COVERAGE, state.enabled_sample_coverage);
    set(ctxt, gl::SCISSOR_TEST, state.enabled_scissor_test);
    set(ctxt, gl::STENCIL_TEST, state.enabled_stencil_test);

    if desktop {
        set(ctxt, gl::MULTISAMPLE, state.enabled_multisample);
        set(
            ctxt,
            gl::POLYGON_OFFSET_LINE,
            state.enabled_polygon_offset_line,
        );
        set(
            ctxt,
            gl::POLYGON_OFFSET_POINT,
            state.enabled_polygon_offset_point,
        );
        set(ctxt, gl::LINE_SMOOTH, state.enabled_line_smooth);
        set(ctxt, gl::POLYGON_SMOOTH, state.enabled_polygon_smooth);
        set(
            ctxt,
            gl::PROGRAM_POINT_SIZE,
            state.enabled_program_point_size,
        );

        if state.enabled_depth_clamp_near == state.enabled_depth_clamp_far {
            set(ctxt, gl::DEPTH_CLAMP, state.enabled_depth_clamp_near);
        }
    }

    if desktop || ctxt.extensions.gl_ext_clip_cull_distance {
        let max_clip_distances = ctxt.glow.get_parameter_i32(gl::MAX_CLIP_DISTANCES).max(0);
        for i in 0..max_clip_distances.min(32) as u32 {
            set(
                ctxt,
                gl::CLIP_DISTANCE0 + i,
                state.enabled_clip_planes & (1 << i) != 0,
            );
        }
    }

    if ctxt.extensions.gl_amd_depth_clamp_separate {
        set(
            ctxt,
            gl::DEPTH_CLAMP_NEAR_AMD,
            state.enabled_depth_clamp_near,
        );
        set(ctxt, gl::DEPTH_CLAMP_FAR_AMD, state.enabled_depth_clamp_far);
    }

    if desktop || ctxt.extensions.gl_ext_srgb_write_control {
        set(ctxt, gl::FRAMEBUFFER_SRGB, state.enabled_framebuffer_srgb);
    }

    // always enabled on WebGL
    if (ctxt.version >= &Version(Api::Gl, 4, 3) || ctxt.version >= &Version(Api::GlEs, 3, 0))
        && !is_webgl()
    {
        set(
            ctxt,
            gl::PRIMITIVE_RESTART_FIXED_INDEX,
            state.enabled_primitive_fixed_restart,
        );
    }

    if (desktop || ctxt.version >= &Version(Api::GlEs, 3, 1)) && !is_webgl() {
        set(ctxt, gl::SAMPLE_MASK, state.enabled_sample_mask);
        ctxt.extra.SampleMaski(0, state.sample_mask);
    }

    if desktop || ctxt.version >= &Version(Api::GlEs, 3, 2) {
        set(ctxt, gl::SAMPLE_SHADING, state.enabled_sample_shading);
        ctxt.extra.MinSampleShading(state.min_sample_shading);
    } else if ctxt.extensions.gl_oes_sample_shading {
        set(ctxt, gl::SAMPLE_SHADING, state.enabled_sample_shading);
        ctxt.extra.MinSampleShadingOES(state.min_sample_shading);
    }
}

unsafe fn restore_bindings(ctxt: &mut CommandContext<'_>) {
    ctxt.glow.use_program(ctxt.state.program);
    ctxt.glow.bind_vertex_array(ctxt.state.vertex_array);
    ctxt.glow
        .bind_framebuffer(gl::READ_FRAMEBUFFER, ctxt.state.read_framebuffer);
    ctxt.glow
        .bind_framebuffer(gl::DRAW_FRAMEBUFFER, ctxt.state.draw_framebuffer);
    ctxt.glow
        .bind_renderbuffer(gl::RENDERBUFFER, ctxt.state.renderbuffer);

    // the read buffer of the default framebuffer is set again before reading from it
    ctxt.state.default_framebuffer_read = None;

    crate::buffer::restore_bindings(ctxt);

    // The cached texture bindings don't record their target, so they are forgotten instead:
    // SafeGL binds every texture it uses before drawing. Sampler objects are unbound, because
    // SafeGL counts on no sampler being bound to the units of textures used without one.
    let units = ctxt.capabilities.max_combined_texture_image_units.max(0) as u32;
    for unit in 0..units {
        ctxt.glow.bind_sampler(unit, None);
    }
    for unit in ctxt.state.texture_units.iter_mut() {
        unit.texture = None;
        unit.sampler = None;
    }
    ctxt.glow
        .active_texture(gl::TEXTURE0 + ctxt.state.active_texture);

    ctxt.glow.pixel_store_i32(
        gl::UNPACK_ALIGNMENT,
        ctxt.state.pixel_store_unpack_alignment,
    );
    ctxt.glow
        .pixel_store_i32(gl::PACK_ALIGNMENT, ctxt.state.pixel_store_pack_alignment);
}

unsafe fn restore_fixed_function(ctxt: &mut CommandContext<'_>) {
    let state = &mut *ctxt.state;
    let desktop = ctxt.version.0 == Api::Gl;

    let (r, g, b, a) = state.clear_color;
    ctxt.glow.clear_color(r, g, b, a);
    if desktop {
        ctxt.glow.clear_depth_f64(state.clear_depth as f64);
    } else {
        ctxt.glow.clear_depth_f32(state.clear_depth);
    }
    ctxt.glow.clear_stencil(state.clear_stencil);

    // the global calls also undo the masks and blending of single draw buffers, so the next
    // draw call issues them again
    let (r, g, b, a) = state.color_mask;
    ctxt.glow.color_mask(r != 0, g != 0, b != 0, a != 0);
    state.indexed_color_mask = true;
    ctxt.glow
        .blend_equation_separate(state.blend_equation.0, state.blend_equation.1);
    let (src_rgb, dst_rgb, src_alpha, dst_alpha) = state.blend_func;
    ctxt.glow
        .blend_func_separate(src_rgb, dst_rgb, src_alpha, dst_alpha);
    let (r, g, b, a) = state.blend_color;
    ctxt.glow.blend_color(r, g, b, a);
    state.indexed_blend = true;

    ctxt.glow.depth_func(state.depth_func);
    ctxt.glow.depth_mask(state.depth_mask);
    if desktop {
        ctxt.glow
            .depth_range_f64(state.depth_range.0 as f64, state.depth_range.1 as f64);
    } else {
        ctxt.glow
            .depth_range_f32(state.depth_range.0, state.depth_range.1);
    }

    for (face, func, mask, op) in [
        (
            gl::FRONT,
            state.stencil_func_front,
            state.stencil_mask_front,
            state.stencil_op_front,
        ),
        (
            gl::BACK,
            state.stencil_func_back,
            state.stencil_mask_back,
            state.stencil_op_back,
        ),
    ] {
        ctxt.glow
            .stencil_func_separate(face, func.0, func.1, func.2);
        ctxt.glow.stencil_mask_separate(face, mask);
        ctxt.glow.stencil_op_separate(face, op.0, op.1, op.2);
    }

    if let Some((x, y, width, height)) = state.viewport {
        ctxt.glow.viewport(x, y, width, height);
    }
    if let Some((x, y, width, height)) = state.scissor {
        ctxt.glow.scissor(x, y, width, height);
    }

    ctxt.glow.line_width(state.line_width);
    ctxt.glow.cull_face(state.cull_face);
    let (factor, units) = state.polygon_offset;
    ctxt.glow.polygon_offset(factor, units);

    if desktop {
        ctxt.extra.PointSize(state.point_size);
        ctxt.glow
            .polygon_mode(gl::FRONT_AND_BACK, state.polygon_mode);
        ctxt.glow.hint(gl::LINE_SMOOTH_HINT, state.smooth.0);
        ctxt.glow.hint(gl::POLYGON_SMOOTH_HINT, state.smooth.1);
        ctxt.extra
            .ClampColor(gl::CLAMP_READ_COLOR, state.clamp_color);
        ctxt.extra.ProvokingVertex(state.provoking_vertex);
    } else if ctxt.extensions.gl_ext_provoking_vertex {
        ctxt.extra.ProvokingVertexEXT(state.provoking_vertex);
    }

    if ctxt.version >= &Version(Api::Gl, 4, 6) || ctxt.extensions.gl_arb_polygon_offset_clamp {
        ctxt.extra
            .PolygonOffsetClamp(factor, units, state.polygon_offset_clamp);
    } else if ctxt.extensions.gl_ext_polygon_offset_clamp {
        ctxt.extra
            .PolygonOffsetClampEXT(factor, units, state.polygon_offset_clamp);
    }

    if ctxt.capabilities.max_patch_vertices.is_some() {
        super::set_patch_vertices(
            (ctxt.glow, ctxt.extra),
            ctxt.version,
            ctxt.extensions,
            state.patch_patch_vertices,
        );
    }

    let bb = state.primitive_bounding_box;
    if ctxt.version >= &Version(Api::GlEs, 3, 2) {
        ctxt.extra
            .PrimitiveBoundingBox(bb.0, bb.1, bb.2, bb.3, bb.4, bb.5, bb.6, bb.7);
    } else if ctxt.extensions.gl_arb_es3_2_compatibility {
        ctxt.extra
            .PrimitiveBoundingBoxARB(bb.0, bb.1, bb.2, bb.3, bb.4, bb.5, bb.6, bb.7);
    } else if ctxt.extensions.gl_oes_primitive_bounding_box {
        ctxt.extra
            .PrimitiveBoundingBoxOES(bb.0, bb.1, bb.2, bb.3, bb.4, bb.5, bb.6, bb.7);
    } else if ctxt.extensions.gl_ext_primitive_bounding_box {
        ctxt.extra
            .PrimitiveBoundingBoxEXT(bb.0, bb.1, bb.2, bb.3, bb.4, bb.5, bb.6, bb.7);
    }

    #[cfg(safegl_gl45)]
    if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_clip_control {
        ctxt.extra
            .ClipControl(state.clip_control.0, state.clip_control.1);
    }
}
