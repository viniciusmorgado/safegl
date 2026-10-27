use glow::HasContext;

use crate::BlitError;
use crate::BlitTarget;
use crate::Rect;

use crate::context::CommandContext;
use crate::context::Context;
use crate::ContextExt;

use crate::fbo::FramebufferTarget;
use crate::fbo::FramebuffersContainer;

use crate::gl;
use crate::version::Api;

#[allow(clippy::too_many_arguments)]
pub fn blit(
    context: &Context,
    source_context: &Context,
    source: FramebufferTarget<'_>,
    target: FramebufferTarget<'_>,
    mask: gl::types::GLbitfield,
    src_rect: &Rect,
    target_rect: &BlitTarget,
    filter: gl::types::GLenum,
) -> Result<(), BlitError> {
    // OpenGL objects can only be used with the context that created them
    if !std::ptr::eq(context, source_context) {
        return Err(BlitError::ContextMismatch);
    }

    if (mask & gl::DEPTH_BUFFER_BIT != 0 || mask & gl::STENCIL_BUFFER_BIT != 0)
        && filter != gl::NEAREST
    {
        return Err(BlitError::LinearFilterOnDepthStencil);
    }

    // a framebuffer object of SafeGL is read from its first color attachment; the read buffer
    // of other framebuffers is theirs
    let source_read_buffer = match source {
        FramebufferTarget::Raw(None) => Some(gl::BACK),
        FramebufferTarget::Raw(Some(_)) => None,
        FramebufferTarget::Attachments(_) => Some(gl::COLOR_ATTACHMENT0),
    };

    unsafe {
        let mut ctxt = context.make_current();

        // FIXME: we don't draw on it
        let source = FramebuffersContainer::get_framebuffer_for_drawing(&mut ctxt, source);
        let target = FramebuffersContainer::get_framebuffer_for_drawing(&mut ctxt, target);

        // binding the framebuffers, which the checks query
        if ctxt.state.read_framebuffer != source {
            ctxt.glow.bind_framebuffer(glow::READ_FRAMEBUFFER, source);
            ctxt.state.read_framebuffer = source;
        }
        if let Some(read_buffer) = source_read_buffer {
            if mask & gl::COLOR_BUFFER_BIT != 0 {
                ctxt.glow.read_buffer(read_buffer);
            }
        }
        if ctxt.state.draw_framebuffer != target {
            ctxt.glow.bind_framebuffer(glow::DRAW_FRAMEBUFFER, target);
            ctxt.state.draw_framebuffer = target;
        }

        validate_blit(
            &mut ctxt,
            source,
            target,
            mask,
            src_rect,
            target_rect,
            filter,
        )?;

        // scissor testing influences blitting
        if ctxt.state.enabled_scissor_test {
            ctxt.glow.disable(glow::SCISSOR_TEST);
            ctxt.state.enabled_scissor_test = false;
        }

        // blitting copies the pixels as they are, while `GL_FRAMEBUFFER_SRGB` would convert them
        // between sRGB and linear values depending on the previous clear or draw call
        if (ctxt.version.0 == Api::Gl || ctxt.extensions.gl_ext_srgb_write_control)
            && ctxt.state.enabled_framebuffer_srgb
        {
            ctxt.glow.disable(glow::FRAMEBUFFER_SRGB);
            ctxt.state.enabled_framebuffer_srgb = false;
        }

        // doing the blit
        ctxt.glow.blit_framebuffer(
            src_rect.left as gl::types::GLint,
            src_rect.bottom as gl::types::GLint,
            (src_rect.left + src_rect.width) as gl::types::GLint,
            (src_rect.bottom + src_rect.height) as gl::types::GLint,
            target_rect.left as gl::types::GLint,
            target_rect.bottom as gl::types::GLint,
            (target_rect.left as i32 + target_rect.width) as gl::types::GLint,
            (target_rect.bottom as i32 + target_rect.height) as gl::types::GLint,
            mask,
            filter,
        );
    }

    Ok(())
}

/// The format of an attachment, as far as blitting is concerned.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct AttachmentFormat {
    /// `GL_FLOAT`, `GL_INT`, `GL_UNSIGNED_INT`, `GL_UNSIGNED_NORMALIZED` or
    /// `GL_SIGNED_NORMALIZED`.
    component_type: i32,
    /// The number of bits of each component: red, green, blue, alpha, depth and stencil.
    sizes: [i32; 6],
    /// `GL_LINEAR` or `GL_SRGB`.
    encoding: i32,
}

impl AttachmentFormat {
    fn is_integer(&self) -> bool {
        self.component_type as u32 == gl::INT || self.component_type as u32 == gl::UNSIGNED_INT
    }
}

/// Returns the format of an attachment of the framebuffer bound to `target`, or `None` if it
/// has nothing attached there.
unsafe fn attachment_format(
    ctxt: &CommandContext<'_>,
    target: u32,
    attachment: u32,
) -> Option<AttachmentFormat> {
    // desktop OpenGL names the buffers of the default framebuffer by their side
    let attachment = match attachment {
        gl::BACK if ctxt.version.0 == Api::Gl => gl::BACK_LEFT,
        gl::FRONT if ctxt.version.0 == Api::Gl => gl::FRONT_LEFT,
        attachment => attachment,
    };
    let parameter = |pname| {
        ctxt.glow
            .get_framebuffer_attachment_parameter_i32(target, attachment, pname)
    };

    if parameter(glow::FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE) as u32 == gl::NONE {
        return None;
    }

    // stencil attachments have no component type
    let component_type = if attachment == gl::STENCIL || attachment == gl::STENCIL_ATTACHMENT {
        gl::UNSIGNED_INT as i32
    } else {
        parameter(glow::FRAMEBUFFER_ATTACHMENT_COMPONENT_TYPE)
    };

    Some(AttachmentFormat {
        component_type,
        sizes: [
            parameter(glow::FRAMEBUFFER_ATTACHMENT_RED_SIZE),
            parameter(glow::FRAMEBUFFER_ATTACHMENT_GREEN_SIZE),
            parameter(glow::FRAMEBUFFER_ATTACHMENT_BLUE_SIZE),
            parameter(glow::FRAMEBUFFER_ATTACHMENT_ALPHA_SIZE),
            parameter(glow::FRAMEBUFFER_ATTACHMENT_DEPTH_SIZE),
            parameter(glow::FRAMEBUFFER_ATTACHMENT_STENCIL_SIZE),
        ],
        encoding: parameter(glow::FRAMEBUFFER_ATTACHMENT_COLOR_ENCODING),
    })
}

/// Returns the number of samples of the framebuffer bound to `GL_DRAW_FRAMEBUFFER`, which is 0
/// if it isn't multisampled.
unsafe fn draw_framebuffer_samples(ctxt: &CommandContext<'_>) -> i32 {
    if ctxt.glow.get_parameter_i32(glow::SAMPLE_BUFFERS) > 0 {
        ctxt.glow.get_parameter_i32(glow::SAMPLES)
    } else {
        0
    }
}

/// Checks the rules of `glBlitFramebuffer` that would raise `GL_INVALID_OPERATION`. The source
/// must be bound to `GL_READ_FRAMEBUFFER` and the target to `GL_DRAW_FRAMEBUFFER`.
unsafe fn validate_blit(
    ctxt: &mut CommandContext<'_>,
    source: Option<glow::Framebuffer>,
    target: Option<glow::Framebuffer>,
    mask: gl::types::GLbitfield,
    src_rect: &Rect,
    target_rect: &BlitTarget,
    filter: gl::types::GLenum,
) -> Result<(), BlitError> {
    let es = ctxt.version.0 != Api::Gl;

    // the number of samples of a framebuffer is only given for the draw framebuffer
    let target_samples = draw_framebuffer_samples(ctxt);
    ctxt.glow.bind_framebuffer(glow::DRAW_FRAMEBUFFER, source);
    let source_samples = draw_framebuffer_samples(ctxt);
    ctxt.glow.bind_framebuffer(glow::DRAW_FRAMEBUFFER, target);

    let multisample = source_samples > 0 || target_samples > 0;
    if es && target_samples > 0 {
        return Err(BlitError::MultisampleTarget);
    }
    if source_samples > 0 && target_samples > 0 && source_samples != target_samples {
        return Err(BlitError::SampleCountMismatch);
    }
    if multisample
        && (target_rect.width != src_rect.width as i32
            || target_rect.height != src_rect.height as i32)
    {
        return Err(BlitError::ScaledMultisample);
    }

    // OpenGL ES can't blit a buffer to itself
    if es && source == target {
        return Err(BlitError::SameSourceAndTarget);
    }

    if mask & gl::COLOR_BUFFER_BIT != 0 {
        let read_buffer = ctxt.glow.get_parameter_i32(glow::READ_BUFFER) as u32;
        let read = if read_buffer == gl::NONE {
            None
        } else {
            attachment_format(ctxt, glow::READ_FRAMEBUFFER, read_buffer)
        };

        if let Some(read) = read {
            if read.is_integer() && filter == gl::LINEAR {
                return Err(BlitError::LinearFilterOnIntegers);
            }

            let draw_buffers = ctxt.capabilities.max_draw_buffers.max(1) as u32;
            for index in 0..draw_buffers {
                let draw_buffer =
                    ctxt.glow.get_parameter_i32(glow::DRAW_BUFFER0 + index) as u32;
                if draw_buffer == gl::NONE {
                    continue;
                }
                let draw = match attachment_format(ctxt, glow::DRAW_FRAMEBUFFER, draw_buffer) {
                    Some(draw) => draw,
                    None => continue,
                };

                // integers are only copied to integers, and multisample blits copy samples as
                // they are
                if read.is_integer() != draw.is_integer() || (multisample && read != draw) {
                    return Err(BlitError::FormatMismatch);
                }
            }
        }
    }

    // the depth and stencil buffers must have the same format, when both framebuffers have them
    for (bit, attachment) in [
        (gl::DEPTH_BUFFER_BIT, gl::DEPTH_ATTACHMENT),
        (gl::STENCIL_BUFFER_BIT, gl::STENCIL_ATTACHMENT),
    ] {
        if mask & bit == 0 {
            continue;
        }
        let name = |framebuffer: Option<glow::Framebuffer>| match framebuffer {
            None if attachment == gl::DEPTH_ATTACHMENT => gl::DEPTH,
            None => gl::STENCIL,
            Some(_) => attachment,
        };
        let read = attachment_format(ctxt, glow::READ_FRAMEBUFFER, name(source));
        let draw = attachment_format(ctxt, glow::DRAW_FRAMEBUFFER, name(target));
        if let (Some(read), Some(draw)) = (read, draw) {
            let relevant = |format: AttachmentFormat| {
                if attachment == gl::DEPTH_ATTACHMENT {
                    (format.component_type, format.sizes[4])
                } else {
                    (0, format.sizes[5])
                }
            };
            if relevant(read) != relevant(draw) {
                return Err(BlitError::FormatMismatch);
            }
        }
    }

    Ok(())
}
