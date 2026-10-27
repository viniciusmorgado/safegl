//! Contains everything related to the default framebuffer.

use std::rc::Rc;

use glow::HasContext;

use crate::backend::Facade;
use crate::gl;
use crate::version::Api;
use crate::context::Context;

use crate::ops;
use crate::uniforms;
use crate::BlitError;
use crate::BlitTarget;
use crate::ContextExt;
use crate::FboAttachments;
use crate::Rect;
use crate::ToGlEnum;
use crate::{BlitMask, DrawParameters};

use crate::DrawError;
use crate::{Program, Surface};

use crate::fbo;
#[cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))]
use crate::framebuffer::InvalidateAttachments;
use crate::framebuffer::{ExternalFramebuffer, MultiOutputFrameBuffer, SimpleFrameBuffer};
use crate::index;
use crate::uniforms::MagnifySamplerFilter;
use crate::vertex;

/// One of the color attachments on the default framebuffer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DefaultFramebufferAttachment {
    /// The backbuffer for the left eye. Equivalent to the backbuffer if stereoscopy is disabled.
    BackLeft,
    /// The backbuffer for the right eye. May not be present.
    BackRight,
    /// The frontbuffer for the left eye. Equivalent to the frontbuffer if stereoscopy is disabled.
    /// May not be accessible.
    FrontLeft,
    /// The frontbuffer for the right eye. May not be present or accessible.
    FrontRight,
}

impl DefaultFramebufferAttachment {
    fn to_glenum(self) -> gl::types::GLenum {
        match self {
            DefaultFramebufferAttachment::BackLeft => gl::BACK_LEFT,
            DefaultFramebufferAttachment::BackRight => gl::BACK_RIGHT,
            DefaultFramebufferAttachment::FrontLeft => gl::FRONT_LEFT,
            DefaultFramebufferAttachment::FrontRight => gl::FRONT_RIGHT,
        }
    }
}

/// The color buffer of the default framebuffer that `DefaultFramebuffer::new` was given doesn't
/// exist, like the front buffer of most window surfaces of EGL, the right buffers of contexts
/// without stereoscopy, and every buffer but the back one on OpenGL ES.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DefaultFramebufferAttachmentMissingError;

impl std::fmt::Display for DefaultFramebufferAttachmentMissingError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("The color buffer of the default framebuffer doesn't exist")
    }
}

impl std::error::Error for DefaultFramebufferAttachmentMissingError {}

/// A framebuffer which has only one color attachment.
pub struct DefaultFramebuffer {
    context: Rc<Context>,
    attachment: DefaultFramebufferAttachment,
}

impl DefaultFramebuffer {
    /// Discards the content of the attachments selected by `which`
    /// (`glInvalidateFramebuffer`).
    ///
    /// This tells the GPU that their content isn't needed any more, so that it doesn't have
    /// to keep it or write it back to memory, which saves bandwidth, especially on the
    /// tile-based GPUs of mobile devices. Their content is undefined afterwards, until they
    /// are drawn to or cleared. Typical uses are the depth buffer at the end of a frame, and
    /// intermediate targets once they have been read.
    ///
    /// This is only a hint: it does nothing without OpenGL 4.3, OpenGL ES 3.0 or
    /// `GL_ARB_invalidate_subdata`.
    #[cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))))]
    pub fn invalidate(&self, which: InvalidateAttachments) {
        let mut ctxt = self.context.make_current();
        unsafe {
            fbo::FramebuffersContainer::invalidate(
                &mut ctxt,
                fbo::FramebufferTarget::Raw(None),
                which,
                None,
            )
        }
    }

    /// Returns the position of each sample inside the pixels, from `[0.0, 0.0]` (the lower left
    /// corner) to `[1.0, 1.0]`, as the GPU places them (`glGetMultisamplefv(GL_SAMPLE_POSITION)`).
    /// The list is empty if the framebuffer isn't multisampled.
    ///
    /// Needs OpenGL ES 3.1 on OpenGL ES.
    #[cfg(any(safegl_desktop, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles31))))]
    pub fn sample_positions(
        &self,
    ) -> Result<Vec<[f32; 2]>, crate::framebuffer::SamplePositionsNotSupportedError> {
        let mut ctxt = self.context.make_current();
        fbo::sample_positions(&mut ctxt, fbo::FramebufferTarget::Raw(None))
    }

    /// Like `invalidate`, but only inside `rect` (`glInvalidateSubFramebuffer`).
    #[cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))))]
    pub fn invalidate_rect(&self, which: InvalidateAttachments, rect: &Rect) {
        let mut ctxt = self.context.make_current();
        unsafe {
            fbo::FramebuffersContainer::invalidate(
                &mut ctxt,
                fbo::FramebufferTarget::Raw(None),
                which,
                Some(rect),
            )
        }
    }

    /// Creates a `DefaultFramebuffer` with the back left buffer.
    #[inline]
    pub fn back_left<F: ?Sized>(facade: &F) -> DefaultFramebuffer
    where
        F: Facade,
    {
        DefaultFramebuffer {
            context: facade.get_context().clone(),
            attachment: DefaultFramebufferAttachment::BackLeft,
        }
    }

    /// Returns a framebuffer that draws into, and is read from, a color buffer of the default
    /// framebuffer (`glDrawBuffer` and `glReadBuffer`): the back or front buffer, of the left
    /// or right eye for stereoscopic contexts.
    ///
    /// Returns an error if the buffer doesn't exist: the front buffer of most window surfaces of
    /// EGL, the right buffers of contexts without stereoscopy, and every buffer but the back
    /// one on OpenGL ES.
    pub fn new<F: ?Sized>(
        facade: &F,
        attachment: DefaultFramebufferAttachment,
    ) -> Result<DefaultFramebuffer, DefaultFramebufferAttachmentMissingError>
    where
        F: Facade,
    {
        let context = facade.get_context();
        if attachment != DefaultFramebufferAttachment::BackLeft {
            let mut ctxt = context.make_current();
            if ctxt.version.0 != Api::Gl {
                return Err(DefaultFramebufferAttachmentMissingError);
            }
            let exists = unsafe {
                fbo::bind_framebuffer(&mut ctxt, None, true, false);
                ctxt.glow.get_framebuffer_attachment_parameter_i32(
                    glow::DRAW_FRAMEBUFFER,
                    attachment.to_glenum(),
                    glow::FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE,
                ) as u32
                    != gl::NONE
            };
            if !exists {
                return Err(DefaultFramebufferAttachmentMissingError);
            }
        }
        Ok(DefaultFramebuffer {
            context: context.clone(),
            attachment,
        })
    }

    /// Returns the color buffer that the framebuffer draws into.
    pub fn attachment(&self) -> DefaultFramebufferAttachment {
        self.attachment
    }
}

impl Surface for DefaultFramebuffer {
    #[inline]
    fn clear(
        &mut self,
        rect: Option<&Rect>,
        color: Option<(f32, f32, f32, f32)>,
        color_srgb: bool,
        depth: Option<f32>,
        stencil: Option<i32>,
    ) {
        ops::clear(
            &self.context,
            self.get_attachments(),
            None,
            color,
            color_srgb,
            depth,
            stencil,
        );
    }

    fn get_dimensions(&self) -> (u32, u32) {
        self.context.get_framebuffer_dimensions()
    }

    fn get_depth_buffer_bits(&self) -> Option<u16> {
        self.context.capabilities().depth_bits
    }

    fn get_stencil_buffer_bits(&self) -> Option<u16> {
        self.context.capabilities().stencil_bits
    }

    fn draw<'a, 'b, V, I, U>(
        &mut self,
        vertex_buffer: V,
        index_buffer: I,
        program: &Program,
        uniforms: &U,
        draw_parameters: &DrawParameters<'_>,
    ) -> Result<(), DrawError>
    where
        I: Into<index::IndicesSource<'a>>,
        U: uniforms::Uniforms,
        V: vertex::MultiVerticesSource<'b>,
    {
        if !self.has_depth_buffer()
            && (draw_parameters.depth.test.requires_depth_buffer() || draw_parameters.depth.write)
        {
            return Err(DrawError::NoDepthBuffer);
        }

        if !self.has_stencil_buffer() && draw_parameters.stencil.requires_stencil_buffer() {
            return Err(DrawError::NoStencilBuffer);
        }

        if let Some(viewport) = draw_parameters.viewport {
            if viewport.width > self.context.capabilities().max_viewport_dims.0 as u32 {
                return Err(DrawError::ViewportTooLarge);
            }
            if viewport.height > self.context.capabilities().max_viewport_dims.1 as u32 {
                return Err(DrawError::ViewportTooLarge);
            }
        }

        ops::draw(
            &self.context,
            self.get_attachments(),
            vertex_buffer,
            index_buffer.into(),
            program,
            uniforms,
            draw_parameters,
            self.get_dimensions(),
        )
    }

    #[inline]
    fn blit_color<S>(
        &self,
        source_rect: &Rect,
        target: &S,
        target_rect: &BlitTarget,
        filter: uniforms::MagnifySamplerFilter,
    ) -> Result<(), BlitError>
    where
        S: Surface,
    {
        target.blit_from_default_framebuffer(self, source_rect, target_rect, filter)
    }

    #[inline]
    fn blit_buffers_from_frame(
        &self,
        source: &Context,
        source_rect: &Rect,
        target_rect: &BlitTarget,
        filter: MagnifySamplerFilter,
        mask: BlitMask,
    ) -> Result<(), BlitError> {
        ops::blit(
            &self.context,
            source,
            fbo::FramebufferTarget::Raw(None),
            self.get_attachments(),
            mask.to_glenum(),
            source_rect,
            target_rect,
            filter.to_glenum(),
        )
    }

    #[inline]
    fn blit_buffers_from_simple_framebuffer(
        &self,
        source: &SimpleFrameBuffer<'_>,
        source_rect: &Rect,
        target_rect: &BlitTarget,
        filter: MagnifySamplerFilter,
        mask: BlitMask,
    ) -> Result<(), BlitError> {
        ops::blit(
            &self.context,
            source.get_fbo_context(),
            source.get_attachments(),
            self.get_attachments(),
            mask.to_glenum(),
            source_rect,
            target_rect,
            filter.to_glenum(),
        )
    }

    #[inline]
    fn blit_buffers_from_multioutput_framebuffer(
        &self,
        source: &MultiOutputFrameBuffer<'_>,
        source_rect: &Rect,
        target_rect: &BlitTarget,
        filter: MagnifySamplerFilter,
        mask: BlitMask,
    ) -> Result<(), BlitError> {
        ops::blit(
            &self.context,
            source.get_fbo_context(),
            source.get_attachments(),
            self.get_attachments(),
            mask.to_glenum(),
            source_rect,
            target_rect,
            filter.to_glenum(),
        )
    }

    #[inline]
    fn blit_buffers_from_external_framebuffer(
        &self,
        source: &ExternalFramebuffer,
        source_rect: &Rect,
        target_rect: &BlitTarget,
        filter: MagnifySamplerFilter,
        mask: BlitMask,
    ) -> Result<(), BlitError> {
        ops::blit(
            &self.context,
            source.get_fbo_context(),
            source.get_attachments(),
            self.get_attachments(),
            mask.to_glenum(),
            source_rect,
            target_rect,
            filter.to_glenum(),
        )
    }

    fn blit_buffers_from_default_framebuffer(
        &self,
        source: &crate::framebuffer::DefaultFramebuffer,
        source_rect: &Rect,
        target_rect: &BlitTarget,
        filter: MagnifySamplerFilter,
        mask: BlitMask,
    ) -> Result<(), BlitError> {
        ops::blit(
            &self.context,
            source.get_fbo_context(),
            source.get_attachments(),
            self.get_attachments(),
            mask.to_glenum(),
            source_rect,
            target_rect,
            filter.to_glenum(),
        )
    }

    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    fn blit_buffers_from_layered_framebuffer(
        &self,
        source: &crate::framebuffer::LayeredFrameBuffer<'_>,
        source_rect: &Rect,
        target_rect: &BlitTarget,
        filter: MagnifySamplerFilter,
        mask: BlitMask,
    ) -> Result<(), BlitError> {
        ops::blit(
            &self.context,
            source.get_fbo_context(),
            source.get_attachments(),
            self.get_attachments(),
            mask.to_glenum(),
            source_rect,
            target_rect,
            filter.to_glenum(),
        )
    }
}

impl FboAttachments for DefaultFramebuffer {
    #[inline]
    fn get_fbo_context(&self) -> &Context {
        &self.context
    }

    #[inline]
    fn get_attachments(&self) -> fbo::FramebufferTarget<'_> {
        match self.attachment {
            // the back buffer, as for `Frame`
            DefaultFramebufferAttachment::BackLeft => fbo::FramebufferTarget::Raw(None),
            attachment => fbo::FramebufferTarget::Default(attachment.to_glenum()),
        }
    }
}
