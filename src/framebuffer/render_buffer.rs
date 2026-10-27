/*!

A render buffer is similar to a texture, but is optimized for usage as a draw target.

Contrary to a texture, you can't sample nor modify the content of a render buffer.
You should prefer render buffers over textures when you know that you don't need to read or modify
the data of the render buffer.

*/
use std::error::Error;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use glow::HasContext;

use crate::framebuffer::{ColorAttachment, ToColorAttachment};
use crate::framebuffer::{DepthAttachment, ToDepthAttachment};
use crate::framebuffer::{DepthStencilAttachment, ToDepthStencilAttachment};
use crate::framebuffer::{StencilAttachment, ToStencilAttachment};
use crate::texture::{
    DepthFormat, DepthStencilFormat, StencilFormat, TextureKind, UncompressedFloatFormat,
};

use crate::image_format;

use crate::backend::Facade;
use crate::context::{CommandContext, Context};
use crate::fbo::FramebuffersContainer;
use crate::gl;
use crate::ContextExt;
use crate::GlObject;

/// Error while creating a render buffer.
#[derive(Copy, Clone, Debug)]
pub enum CreationError {
    /// The requested format is not supported.
    FormatNotSupported,
}

impl fmt::Display for CreationError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::CreationError::*;
        let desc = match *self {
            FormatNotSupported => "The requested format is not supported",
        };
        fmt.write_str(desc)
    }
}

impl Error for CreationError {}

impl From<image_format::FormatNotSupportedError> for CreationError {
    fn from(_: image_format::FormatNotSupportedError) -> CreationError {
        CreationError::FormatNotSupported
    }
}

/// A render buffer is similar to a texture, but is optimized for usage as a draw target.
///
/// Contrary to a texture, you can't sample or modify the content of the `RenderBuffer`.
pub struct RenderBuffer {
    buffer: RenderBufferAny,
}

impl RenderBuffer {
    /// Builds a new render buffer.
    pub fn new<F: ?Sized>(
        facade: &F,
        format: UncompressedFloatFormat,
        width: u32,
        height: u32,
    ) -> Result<RenderBuffer, CreationError>
    where
        F: Facade,
    {
        let format = image_format::TextureFormatRequest::Specific(
            image_format::TextureFormat::UncompressedFloat(format),
        );
        let format = image_format::format_request_to_glenum(
            &facade.get_context(),
            format,
            image_format::RequestType::Renderbuffer,
        )?;

        Ok(RenderBuffer {
            buffer: RenderBufferAny::new(facade, format, TextureKind::Float, width, height, None),
        })
    }
    /// Builds a new render buffer with multisampling.
    pub fn new_multisample<F: ?Sized>(
        facade: &F,
        format: UncompressedFloatFormat,
        width: u32,
        height: u32,
        samples: u32,
    ) -> Result<RenderBuffer, CreationError>
    where
        F: Facade,
    {
        let format = image_format::TextureFormatRequest::Specific(
            image_format::TextureFormat::UncompressedFloat(format),
        );
        let format = image_format::format_request_to_glenum(
            &facade.get_context(),
            format,
            image_format::RequestType::Renderbuffer,
        )?;

        Ok(RenderBuffer {
            buffer: RenderBufferAny::new(
                facade,
                format,
                TextureKind::Float,
                width,
                height,
                Some(samples),
            ),
        })
    }
}

impl<'a> ToColorAttachment<'a> for &'a RenderBuffer {
    #[inline]
    fn to_color_attachment(self) -> ColorAttachment<'a> {
        ColorAttachment::RenderBuffer(self)
    }
}

impl Deref for RenderBuffer {
    type Target = RenderBufferAny;

    #[inline]
    fn deref(&self) -> &RenderBufferAny {
        &self.buffer
    }
}

impl DerefMut for RenderBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut RenderBufferAny {
        &mut self.buffer
    }
}

impl GlObject for RenderBuffer {
    type Id = glow::Renderbuffer;

    #[inline]
    fn get_id(&self) -> glow::Renderbuffer {
        self.buffer.get_id()
    }
}

/// A render buffer is similar to a texture, but is optimized for usage as a draw target.
///
/// Contrary to a texture, you can't sample or modify the content of the `DepthRenderBuffer` directly.
pub struct DepthRenderBuffer {
    buffer: RenderBufferAny,
}

impl DepthRenderBuffer {
    /// Builds a new render buffer.
    pub fn new<F: ?Sized>(
        facade: &F,
        format: DepthFormat,
        width: u32,
        height: u32,
    ) -> Result<DepthRenderBuffer, CreationError>
    where
        F: Facade,
    {
        let format = image_format::TextureFormatRequest::Specific(
            image_format::TextureFormat::DepthFormat(format),
        );
        let format = image_format::format_request_to_glenum(
            &facade.get_context(),
            format,
            image_format::RequestType::Renderbuffer,
        )?;

        Ok(DepthRenderBuffer {
            buffer: RenderBufferAny::new(facade, format, TextureKind::Depth, width, height, None),
        })
    }
    /// Builds a new render buffer with multisampling.
    pub fn new_multisample<F: ?Sized>(
        facade: &F,
        format: DepthFormat,
        width: u32,
        height: u32,
        samples: u32,
    ) -> Result<DepthRenderBuffer, CreationError>
    where
        F: Facade,
    {
        let format = image_format::TextureFormatRequest::Specific(
            image_format::TextureFormat::DepthFormat(format),
        );
        let format = image_format::format_request_to_glenum(
            &facade.get_context(),
            format,
            image_format::RequestType::Renderbuffer,
        )?;

        Ok(DepthRenderBuffer {
            buffer: RenderBufferAny::new(
                facade,
                format,
                TextureKind::Depth,
                width,
                height,
                Some(samples),
            ),
        })
    }
}

impl<'a> ToDepthAttachment<'a> for &'a DepthRenderBuffer {
    fn to_depth_attachment(self) -> DepthAttachment<'a> {
        DepthAttachment::RenderBuffer(self)
    }
}

impl Deref for DepthRenderBuffer {
    type Target = RenderBufferAny;

    #[inline]
    fn deref(&self) -> &RenderBufferAny {
        &self.buffer
    }
}

impl DerefMut for DepthRenderBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut RenderBufferAny {
        &mut self.buffer
    }
}

impl GlObject for DepthRenderBuffer {
    type Id = glow::Renderbuffer;

    #[inline]
    fn get_id(&self) -> glow::Renderbuffer {
        self.buffer.get_id()
    }
}

/// A render buffer is similar to a texture, but is optimized for usage as a draw target.
///
/// Contrary to a texture, you can't sample or modify the content of the `StencilRenderBuffer` directly.
pub struct StencilRenderBuffer {
    buffer: RenderBufferAny,
}

impl StencilRenderBuffer {
    /// Builds a new render buffer.
    pub fn new<F: ?Sized>(
        facade: &F,
        format: StencilFormat,
        width: u32,
        height: u32,
    ) -> Result<StencilRenderBuffer, CreationError>
    where
        F: Facade,
    {
        let format = image_format::TextureFormatRequest::Specific(
            image_format::TextureFormat::StencilFormat(format),
        );
        let format = image_format::format_request_to_glenum(
            &facade.get_context(),
            format,
            image_format::RequestType::Renderbuffer,
        )?;

        Ok(StencilRenderBuffer {
            buffer: RenderBufferAny::new(facade, format, TextureKind::Stencil, width, height, None),
        })
    }
    /// Builds a new render buffer with multisampling.
    pub fn new_multisample<F: ?Sized>(
        facade: &F,
        format: StencilFormat,
        width: u32,
        height: u32,
        samples: u32,
    ) -> Result<StencilRenderBuffer, CreationError>
    where
        F: Facade,
    {
        let format = image_format::TextureFormatRequest::Specific(
            image_format::TextureFormat::StencilFormat(format),
        );
        let format = image_format::format_request_to_glenum(
            &facade.get_context(),
            format,
            image_format::RequestType::Renderbuffer,
        )?;

        Ok(StencilRenderBuffer {
            buffer: RenderBufferAny::new(
                facade,
                format,
                TextureKind::Stencil,
                width,
                height,
                Some(samples),
            ),
        })
    }
}

impl<'a> ToStencilAttachment<'a> for &'a StencilRenderBuffer {
    #[inline]
    fn to_stencil_attachment(self) -> StencilAttachment<'a> {
        StencilAttachment::RenderBuffer(self)
    }
}

impl Deref for StencilRenderBuffer {
    type Target = RenderBufferAny;

    #[inline]
    fn deref(&self) -> &RenderBufferAny {
        &self.buffer
    }
}

impl DerefMut for StencilRenderBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut RenderBufferAny {
        &mut self.buffer
    }
}

impl GlObject for StencilRenderBuffer {
    type Id = glow::Renderbuffer;

    #[inline]
    fn get_id(&self) -> glow::Renderbuffer {
        self.buffer.get_id()
    }
}

/// A render buffer is similar to a texture, but is optimized for usage as a draw target.
///
/// Contrary to a texture, you can't sample or modify the content of the `DepthStencilRenderBuffer` directly.
pub struct DepthStencilRenderBuffer {
    buffer: RenderBufferAny,
}

impl DepthStencilRenderBuffer {
    /// Builds a new render buffer.
    pub fn new<F: ?Sized>(
        facade: &F,
        format: DepthStencilFormat,
        width: u32,
        height: u32,
    ) -> Result<DepthStencilRenderBuffer, CreationError>
    where
        F: Facade,
    {
        let format = image_format::TextureFormatRequest::Specific(
            image_format::TextureFormat::DepthStencilFormat(format),
        );
        let format = image_format::format_request_to_glenum(
            &facade.get_context(),
            format,
            image_format::RequestType::Renderbuffer,
        )?;

        Ok(DepthStencilRenderBuffer {
            buffer: RenderBufferAny::new(
                facade,
                format,
                TextureKind::DepthStencil,
                width,
                height,
                None,
            ),
        })
    }
    /// Builds a new render buffer with multisampling.
    pub fn new_multisample<F: ?Sized>(
        facade: &F,
        format: DepthStencilFormat,
        width: u32,
        height: u32,
        samples: u32,
    ) -> Result<DepthStencilRenderBuffer, CreationError>
    where
        F: Facade,
    {
        let format = image_format::TextureFormatRequest::Specific(
            image_format::TextureFormat::DepthStencilFormat(format),
        );
        let format = image_format::format_request_to_glenum(
            &facade.get_context(),
            format,
            image_format::RequestType::Renderbuffer,
        )?;

        Ok(DepthStencilRenderBuffer {
            buffer: RenderBufferAny::new(
                facade,
                format,
                TextureKind::DepthStencil,
                width,
                height,
                Some(samples),
            ),
        })
    }
}

impl<'a> ToDepthStencilAttachment<'a> for &'a DepthStencilRenderBuffer {
    #[inline]
    fn to_depth_stencil_attachment(self) -> DepthStencilAttachment<'a> {
        DepthStencilAttachment::RenderBuffer(self)
    }
}

impl Deref for DepthStencilRenderBuffer {
    type Target = RenderBufferAny;

    #[inline]
    fn deref(&self) -> &RenderBufferAny {
        &self.buffer
    }
}

impl DerefMut for DepthStencilRenderBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut RenderBufferAny {
        &mut self.buffer
    }
}

impl GlObject for DepthStencilRenderBuffer {
    type Id = glow::Renderbuffer;

    #[inline]
    fn get_id(&self) -> glow::Renderbuffer {
        self.buffer.get_id()
    }
}

/// A RenderBuffer of indeterminate type.
pub struct RenderBufferAny {
    context: Rc<Context>,
    id: glow::Renderbuffer,
    width: u32,
    height: u32,
    samples: Option<u32>,
    kind: TextureKind,
}

impl RenderBufferAny {
    /// Builds a new render buffer.
    fn new<F: ?Sized>(
        facade: &F,
        format: gl::types::GLenum,
        kind: TextureKind,
        width: u32,
        height: u32,
        samples: Option<u32>,
    ) -> RenderBufferAny
    where
        F: Facade,
    {
        unsafe {
            // TODO: check that dimensions don't exceed GL_MAX_RENDERBUFFER_SIZE
            // FIXME: gles2 only supports very few formats
            let mut ctxt = facade.get_context().make_current();

            let id = ctxt
                .glow
                .create_renderbuffer()
                .expect("failed to create a renderbuffer object");
            ctxt.glow.bind_renderbuffer(glow::RENDERBUFFER, Some(id));
            ctxt.state.renderbuffer = Some(id);

            if let Some(samples) = samples {
                ctxt.glow.renderbuffer_storage_multisample(
                    glow::RENDERBUFFER,
                    samples as gl::types::GLsizei,
                    format,
                    width as gl::types::GLsizei,
                    height as gl::types::GLsizei,
                );
            } else {
                ctxt.glow.renderbuffer_storage(
                    glow::RENDERBUFFER,
                    format,
                    width as gl::types::GLsizei,
                    height as gl::types::GLsizei,
                );
            }

            RenderBufferAny {
                context: facade.get_context().clone(),
                id,
                width,
                height,
                samples,
                kind,
            }
        }
    }

    /// Returns the dimensions of the render buffer.
    #[inline]
    pub fn get_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the number of samples of the render buffer, or `None` if multisampling isn't
    /// enabled.
    #[inline]
    pub fn get_samples(&self) -> Option<u32> {
        self.samples
    }

    /// Returns the context used to create this renderbuffer.
    #[inline]
    pub fn get_context(&self) -> &Rc<Context> {
        &self.context
    }

    /// Returns the kind of renderbuffer.
    #[inline]
    pub fn kind(&self) -> TextureKind {
        self.kind
    }

    /// Determines the number of depth and stencil bits in the format of this render buffer.
    pub fn get_depth_stencil_bits(&self) -> (u16, u16) {
        self.get_depth_stencil_bits_with(&mut self.context.make_current())
    }

    /// Same as `get_depth_stencil_bits`, with the context already current.
    pub(crate) fn get_depth_stencil_bits_with(&self, ctxt: &mut CommandContext<'_>) -> (u16, u16) {
        unsafe {
            // `glGetRenderbufferParameteriv` is core in OpenGL 3.0 and OpenGL ES 2.0
            if ctxt.state.renderbuffer != Some(self.id) {
                ctxt.glow
                    .bind_renderbuffer(glow::RENDERBUFFER, Some(self.id));
                ctxt.state.renderbuffer = Some(self.id);
            }
            let depth_bits: gl::types::GLint = ctxt
                .glow
                .get_renderbuffer_parameter_i32(glow::RENDERBUFFER, glow::RENDERBUFFER_DEPTH_SIZE);
            let stencil_bits: gl::types::GLint = ctxt.glow.get_renderbuffer_parameter_i32(
                glow::RENDERBUFFER,
                glow::RENDERBUFFER_STENCIL_SIZE,
            );
            (depth_bits as u16, stencil_bits as u16)
        }
    }
}

impl Drop for RenderBufferAny {
    fn drop(&mut self) {
        unsafe {
            let mut ctxt = self.context.make_current();

            // removing FBOs which contain this buffer
            FramebuffersContainer::purge_renderbuffer(&mut ctxt, self.id);

            if ctxt.state.renderbuffer == Some(self.id) {
                ctxt.state.renderbuffer = None;
            }

            ctxt.glow.delete_renderbuffer(self.id);
        }
    }
}

impl GlObject for RenderBufferAny {
    type Id = glow::Renderbuffer;

    #[inline]
    fn get_id(&self) -> glow::Renderbuffer {
        self.id
    }
}
