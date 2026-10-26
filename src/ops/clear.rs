use glow::HasContext;

use crate::fbo::{self, FramebufferTarget};

use crate::context::Context;
use crate::ContextExt;
use crate::Rect;

use crate::draw_parameters::TimeElapsedQuery;
use crate::QueryExt;

use crate::gl;
use crate::texture::TextureKind;
use crate::Api;

pub fn clear(
    context: &Context,
    framebuffer: FramebufferTarget<'_>,
    rect: Option<&Rect>,
    color: Option<(f32, f32, f32, f32)>,
    color_srgb: bool,
    depth: Option<f32>,
    stencil: Option<i32>,
) {
    unsafe {
        let mut ctxt = context.make_current();

        let fbo_id =
            fbo::FramebuffersContainer::get_framebuffer_for_drawing(&mut ctxt, framebuffer);
        fbo::bind_framebuffer(&mut ctxt, fbo_id, true, false);

        if ctxt.state.enabled_rasterizer_discard {
            ctxt.glow.disable(glow::RASTERIZER_DISCARD);
            ctxt.state.enabled_rasterizer_discard = false;
        }

        // also undoes the per-draw-buffer masks of `DrawParameters::outputs`
        if ctxt.state.color_mask != (1, 1, 1, 1) || ctxt.state.indexed_color_mask {
            ctxt.state.color_mask = (1, 1, 1, 1);
            ctxt.state.indexed_color_mask = false;
            ctxt.glow.color_mask(true, true, true, true);
        }

        if ctxt.version.0 == Api::Gl || ctxt.extensions.gl_ext_srgb_write_control {
            if !color_srgb && !ctxt.state.enabled_framebuffer_srgb {
                ctxt.glow.enable(glow::FRAMEBUFFER_SRGB);
                ctxt.state.enabled_framebuffer_srgb = true;
            } else if color_srgb && ctxt.state.enabled_framebuffer_srgb {
                ctxt.glow.disable(glow::FRAMEBUFFER_SRGB);
                ctxt.state.enabled_framebuffer_srgb = false;
            }
        }

        TimeElapsedQuery::end_conditional_render(&mut ctxt);

        if let Some(rect) = rect {
            let rect = (
                rect.left as gl::types::GLint,
                rect.bottom as gl::types::GLint,
                rect.width as gl::types::GLsizei,
                rect.height as gl::types::GLsizei,
            );

            if ctxt.state.scissor != Some(rect) {
                ctxt.glow.scissor(rect.0, rect.1, rect.2, rect.3);
                ctxt.state.scissor = Some(rect);
            }

            if !ctxt.state.enabled_scissor_test {
                ctxt.glow.enable(glow::SCISSOR_TEST);
                ctxt.state.enabled_scissor_test = true;
            }
        } else if ctxt.state.enabled_scissor_test {
            ctxt.glow.disable(glow::SCISSOR_TEST);
            ctxt.state.enabled_scissor_test = false;
        }

        let mut flags = 0;

        if let Some(color) = color {
            let color = (
                color.0 as gl::types::GLclampf,
                color.1 as gl::types::GLclampf,
                color.2 as gl::types::GLclampf,
                color.3 as gl::types::GLclampf,
            );

            flags |= gl::COLOR_BUFFER_BIT;

            if ctxt.state.clear_color != color {
                ctxt.glow.clear_color(color.0, color.1, color.2, color.3);
                ctxt.state.clear_color = color;
            }
        }

        if let Some(depth) = depth {
            let depth = depth as gl::types::GLclampf;

            flags |= gl::DEPTH_BUFFER_BIT;

            if ctxt.state.clear_depth != depth {
                if ctxt.version.0 == Api::Gl {
                    ctxt.glow.clear_depth_f64(depth as gl::types::GLclampd);
                } else {
                    ctxt.glow.clear_depth_f32(depth);
                }

                ctxt.state.clear_depth = depth;
            }

            if !ctxt.state.depth_mask {
                ctxt.glow.depth_mask(true);
                ctxt.state.depth_mask = true;
            }
        }

        if let Some(stencil) = stencil {
            let stencil = stencil as gl::types::GLint;

            flags |= gl::STENCIL_BUFFER_BIT;

            if ctxt.state.clear_stencil != stencil {
                ctxt.glow.clear_stencil(stencil);
                ctxt.state.clear_stencil = stencil;
            }

            // the stencil write mask of the last draw call would mask the cleared value too
            if ctxt.state.stencil_mask_front != 0xffffffff
                || ctxt.state.stencil_mask_back != 0xffffffff
            {
                ctxt.glow.stencil_mask(0xffffffff);
                ctxt.state.stencil_mask_front = 0xffffffff;
                ctxt.state.stencil_mask_back = 0xffffffff;
            }
        }

        // `glClearColor` is undefined for integer color buffers, which are cleared one by one
        // with the color converted to integers
        if let (Some(color), FramebufferTarget::Attachments(attachments)) = (color, framebuffer) {
            let kinds = attachments.get_color_kinds();
            let integer = |kind| matches!(kind, TextureKind::Integral | TextureKind::Unsigned);
            if kinds.iter().any(|&(_, kind)| integer(kind)) {
                flags &= !gl::COLOR_BUFFER_BIT;

                for &(location, kind) in kinds {
                    let (r, g, b, a) = color;
                    match kind {
                        TextureKind::Integral => ctxt.glow.clear_buffer_i32_slice(
                            gl::COLOR,
                            location,
                            &[r as i32, g as i32, b as i32, a as i32],
                        ),
                        TextureKind::Unsigned => ctxt.glow.clear_buffer_u32_slice(
                            gl::COLOR,
                            location,
                            &[r as u32, g as u32, b as u32, a as u32],
                        ),
                        _ => ctxt
                            .glow
                            .clear_buffer_f32_slice(gl::COLOR, location, &[r, g, b, a]),
                    }
                }
            }
        }

        if flags != 0 {
            ctxt.glow.clear(flags);
        }
    }
}
