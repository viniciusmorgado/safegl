//! Contains everything related to the default framebuffer.

use std::rc::Rc;

use crate::backend::Facade;
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
use crate::framebuffer::{
    ExternalFramebuffer, InvalidateAttachments, MultiOutputFrameBuffer, SimpleFrameBuffer,
};
use crate::index;
use crate::uniforms::MagnifySamplerFilter;
use crate::vertex;

/// One of the color attachments on the default framebuffer.
#[derive(Copy, Clone, Debug)]
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

    /// Like `invalidate`, but only inside `rect` (`glInvalidateSubFramebuffer`).
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
        // TODO: wrong attachment
        ops::clear(
            &self.context,
            fbo::FramebufferTarget::Raw(None),
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

        // TODO: wrong attachment
        ops::draw(
            &self.context,
            fbo::FramebufferTarget::Raw(None),
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
        target.blit_from_frame(&self.context, source_rect, target_rect, filter)
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
}

impl FboAttachments for DefaultFramebuffer {
    #[inline]
    fn get_fbo_context(&self) -> &Context {
        &self.context
    }

    #[inline]
    fn get_attachments(&self) -> fbo::FramebufferTarget<'_> {
        fbo::FramebufferTarget::Raw(None)
    }
}
