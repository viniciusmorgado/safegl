//! Framebuffer objects that weren't created by SafeGL.

use std::rc::Rc;

use glow::HasContext;

use crate::backend::Facade;
use crate::context::Context;

use crate::gl;
use crate::ops;
use crate::uniforms;
use crate::BlitTarget;
use crate::ContextExt;
use crate::FboAttachments;
use crate::Rect;
use crate::ToGlEnum;
use crate::{BlitMask, DrawParameters};

use crate::DrawError;
use crate::{Program, Surface};

use crate::fbo;
use crate::framebuffer::{InvalidateAttachments, MultiOutputFrameBuffer, SimpleFrameBuffer};
use crate::index;
use crate::uniforms::MagnifySamplerFilter;
use crate::vertex;

/// A framebuffer object created outside SafeGL, that SafeGL can draw to, clear, blit to and from,
/// and invalidate.
///
/// GUI toolkits that let OpenGL draw into a widget (GTK's `GLArea`, for example), or frontends
/// that give a framebuffer to their cores, draw through a framebuffer object of their own instead
/// of the default framebuffer.
///
/// ```no_run
/// # use safegl::glow::HasContext;
/// # fn example(display: &safegl::Display<safegl::glutin::surface::WindowSurface>) {
/// # let (width, height) = (800, 600);
/// use safegl::Surface;
/// // the framebuffer the toolkit bound before calling us
/// let id = unsafe { display.exec_in_context(|| {
///     display.glow_context().get_parameter_framebuffer(safegl::glow::DRAW_FRAMEBUFFER_BINDING)
/// }) }.unwrap();
/// let mut target = unsafe {
///     safegl::framebuffer::ExternalFramebuffer::new(display, id, (width, height))
/// };
/// target.clear_color(0.0, 0.0, 0.0, 1.0);
/// # }
/// ```
pub struct ExternalFramebuffer {
    context: Rc<Context>,
    id: glow::Framebuffer,
    dimensions: (u32, u32),
    depth_buffer_bits: Option<u16>,
    stencil_buffer_bits: Option<u16>,
}

impl ExternalFramebuffer {
    /// Wraps the framebuffer object `id`, whose attachments are `dimensions` large. The sizes of
    /// its depth and stencil buffers are queried from it.
    ///
    /// # Safety
    ///
    /// `id` must be a complete framebuffer object of the context of `facade`. It must not be
    /// deleted, and its attachments must not be changed, while the returned object exists.
    pub unsafe fn new<F>(
        facade: &F,
        id: glow::Framebuffer,
        dimensions: (u32, u32),
    ) -> ExternalFramebuffer
    where
        F: Facade + ?Sized,
    {
        let context = facade.get_context().clone();

        let (depth_buffer_bits, stencil_buffer_bits) = {
            let mut ctxt = context.make_current();
            fbo::bind_framebuffer(&mut ctxt, Some(id), false, true);
            // `bind_framebuffer` binds `GL_FRAMEBUFFER` on OpenGL ES and WebGL
            let target = if ctxt.version.0 == crate::version::Api::Gl {
                gl::READ_FRAMEBUFFER
            } else {
                gl::FRAMEBUFFER
            };

            let bits = |attachment, size| {
                let ty = ctxt.glow.get_framebuffer_attachment_parameter_i32(
                    target,
                    attachment,
                    gl::FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE,
                );
                if ty == gl::NONE as i32 {
                    return None;
                }
                let bits = ctxt
                    .glow
                    .get_framebuffer_attachment_parameter_i32(target, attachment, size);
                if bits > 0 {
                    Some(bits as u16)
                } else {
                    None
                }
            };

            (
                bits(gl::DEPTH_ATTACHMENT, gl::FRAMEBUFFER_ATTACHMENT_DEPTH_SIZE),
                bits(
                    gl::STENCIL_ATTACHMENT,
                    gl::FRAMEBUFFER_ATTACHMENT_STENCIL_SIZE,
                ),
            )
        };

        ExternalFramebuffer {
            context,
            id,
            dimensions,
            depth_buffer_bits,
            stencil_buffer_bits,
        }
    }

    /// Returns the framebuffer object.
    #[inline]
    pub fn get_id(&self) -> glow::Framebuffer {
        self.id
    }

    /// Discards the content of the attachments selected by `which`
    /// (`glInvalidateFramebuffer`). The color attachment is the first one.
    ///
    /// This is only a hint: it does nothing without OpenGL 4.3, OpenGL ES 3.0 or
    /// `GL_ARB_invalidate_subdata`.
    pub fn invalidate(&self, which: InvalidateAttachments) {
        let mut ctxt = self.context.make_current();
        unsafe {
            fbo::FramebuffersContainer::invalidate(&mut ctxt, self.get_attachments(), which, None)
        }
    }

    /// Like `invalidate`, but only inside `rect` (`glInvalidateSubFramebuffer`).
    pub fn invalidate_rect(&self, which: InvalidateAttachments, rect: &Rect) {
        let mut ctxt = self.context.make_current();
        unsafe {
            fbo::FramebuffersContainer::invalidate(
                &mut ctxt,
                self.get_attachments(),
                which,
                Some(rect),
            )
        }
    }
}

impl Surface for ExternalFramebuffer {
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
            rect,
            color,
            color_srgb,
            depth,
            stencil,
        );
    }

    #[inline]
    fn get_dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    #[inline]
    fn get_depth_buffer_bits(&self) -> Option<u16> {
        self.depth_buffer_bits
    }

    #[inline]
    fn get_stencil_buffer_bits(&self) -> Option<u16> {
        self.stencil_buffer_bits
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
    ) where
        S: Surface,
    {
        target.blit_from_external_framebuffer(self, source_rect, target_rect, filter)
    }

    #[inline]
    fn blit_buffers_from_frame(
        &self,
        source_rect: &Rect,
        target_rect: &BlitTarget,
        filter: MagnifySamplerFilter,
        mask: BlitMask,
    ) {
        ops::blit(
            &self.context,
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
    ) {
        ops::blit(
            &self.context,
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
    ) {
        ops::blit(
            &self.context,
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
    ) {
        ops::blit(
            &self.context,
            source.get_attachments(),
            self.get_attachments(),
            mask.to_glenum(),
            source_rect,
            target_rect,
            filter.to_glenum(),
        )
    }
}

impl FboAttachments for ExternalFramebuffer {
    #[inline]
    fn get_attachments(&self) -> fbo::FramebufferTarget<'_> {
        fbo::FramebufferTarget::Raw(Some(self.id))
    }
}
