use glow::HasContext;

use crate::BlitError;
use crate::BlitTarget;
use crate::Rect;

use crate::context::Context;
use crate::ContextExt;

use crate::fbo::FramebufferTarget;
use crate::fbo::FramebuffersContainer;

use crate::gl;
use crate::version::Api;
use crate::version::Version;

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

    unsafe {
        let mut ctxt = context.make_current();

        // FIXME: we don't draw on it
        let source = FramebuffersContainer::get_framebuffer_for_drawing(&mut ctxt, source);
        let target = FramebuffersContainer::get_framebuffer_for_drawing(&mut ctxt, target);

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

        // trying to do a named blit if possible
        if ctxt.version >= &Version(Api::Gl, 4, 5) {
            ctxt.glow.blit_named_framebuffer(
                source,
                target,
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

            return Ok(());
        }

        // binding source framebuffer
        if ctxt.state.read_framebuffer != source {
            ctxt.glow.bind_framebuffer(glow::READ_FRAMEBUFFER, source);
            ctxt.state.read_framebuffer = source;
        }

        // binding target framebuffer
        if ctxt.state.draw_framebuffer != target {
            ctxt.glow.bind_framebuffer(glow::DRAW_FRAMEBUFFER, target);
            ctxt.state.draw_framebuffer = target;
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
