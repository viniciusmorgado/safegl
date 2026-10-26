/*!
Framebuffers allow you to customize the color, depth and stencil buffers you will draw on.

In order to draw on a texture, use a `SimpleFrameBuffer`. This framebuffer is compatible with
shaders that write to `gl_FragColor`.

```no_run
# use safegl::texture::Texture2d;
# use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
# fn example<T>(display: safegl::Display<T>, texture: Texture2d)
#     where T: SurfaceTypeTrait + ResizeableSurface {
let framebuffer = safegl::framebuffer::SimpleFrameBuffer::new(&display, &texture);
// framebuffer.draw(...);    // draws over `texture`
# }
```

If, however, your shader wants to write to multiple color buffers at once, you must use
a `MultiOutputFrameBuffer`.

```no_run
# use safegl::texture::Texture2d;
# use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
# fn example<T>(display: safegl::Display<T>, texture1: Texture2d, texture2: Texture2d)
#     where T: SurfaceTypeTrait + ResizeableSurface {
let output = [ ("output1", &texture1), ("output2", &texture2) ];
let framebuffer = safegl::framebuffer::MultiOutputFrameBuffer::new(&display, output.iter().cloned());
// framebuffer.draw(...);

// example shader:
//
//     out vec4 output1;
//     out vec4 output2;
//
//     void main() {
//         output1 = vec4(0.0, 0.0, 0.5, 1.0);
//         output2 = vec4(1.0, 0.7, 1.0, 1.0);
//     }
# }
```

**Note**: depth-stencil attachments are not yet implemented.

# A note on restrictions

Some restrictions apply when you use framebuffers:

 - All textures must have an internal format that is renderable. Not all formats are supported.

 - All attachments must have the same number of samples, or must all have multisampling disabled.
   For example you can't create a texture with 4x multisampling, another texture with 2x
   multisampling, and draw on them simultaneously.

 - On old hardware all the framebuffer attachments must have the same dimensions (on more recent
   hardware the intersection between all the attachments is taken if all attachments don't have
   the same dimensions). You can use the `is_dimensions_mismatch_supported` function to check
   what the hardware supports.

 - You will get undefined results if you try to sample to a texture mipmap attached to the
   framebuffer that you are using. This is not enforced by safegl as it depends on your shader's
   source code. With the `gl46` feature, `texture::texture_barrier` makes it defined when each
   texel is read only by the draw that writes it, or after a barrier.

# Empty framebuffers

Modern OpenGL implementations support empty framebuffers. This is handled by SafeGL with the
`EmptyFrameBuffer` struct. Empty framebuffers are core in OpenGL 4.3, so this struct requires
the `gl46` Cargo feature.

You can check whether they are supported by calling `EmptyFrameBuffer::is_supported(&display)`.

# Layered framebuffers

Not yet supported

*/
use smallvec::SmallVec;
use std::rc::Rc;

use crate::texture::TextureAnyImage;

use crate::backend::Facade;
use crate::context::Context;
#[cfg(feature = "gl46")]
use crate::version::Api;
#[cfg(feature = "gl46")]
use crate::version::Version;
use crate::BlitMask;
#[cfg(feature = "gl46")]
use crate::CapabilitiesSource;

use crate::ops;
use crate::uniforms;
use crate::BlitTarget;
use crate::ContextExt;
use crate::FboAttachments;
use crate::Rect;
use crate::ToGlEnum;

use crate::DrawError;
use crate::{Program, Surface};

use crate::fbo;

pub use self::default_fb::{DefaultFramebuffer, DefaultFramebufferAttachment};
pub use self::external_fb::ExternalFramebuffer;
pub use self::render_buffer::CreationError as RenderBufferCreationError;
pub use self::render_buffer::{DepthRenderBuffer, RenderBuffer, RenderBufferAny};
pub use self::render_buffer::{DepthStencilRenderBuffer, StencilRenderBuffer};
pub use crate::fbo::is_dimensions_mismatch_supported;
pub use crate::fbo::ValidationError;
use crate::uniforms::MagnifySamplerFilter;

mod default_fb;
mod external_fb;
mod render_buffer;

/// The attachments of a framebuffer that `invalidate` and `invalidate_rect` discard.
///
/// ```
/// use safegl::framebuffer::InvalidateAttachments;
///
/// // only the depth and stencil buffers
/// let which = InvalidateAttachments { depth: true, stencil: true, ..Default::default() };
/// assert_eq!(which, InvalidateAttachments::DEPTH_STENCIL);
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct InvalidateAttachments {
    /// All the color attachments.
    pub color: bool,
    /// The depth attachment, or the depth of a depth-stencil attachment.
    pub depth: bool,
    /// The stencil attachment, or the stencil of a depth-stencil attachment.
    pub stencil: bool,
}

impl InvalidateAttachments {
    /// Every attachment.
    pub const ALL: InvalidateAttachments = InvalidateAttachments {
        color: true,
        depth: true,
        stencil: true,
    };

    /// The color attachments.
    pub const COLOR: InvalidateAttachments = InvalidateAttachments {
        color: true,
        depth: false,
        stencil: false,
    };

    /// The depth and stencil attachments.
    pub const DEPTH_STENCIL: InvalidateAttachments = InvalidateAttachments {
        color: false,
        depth: true,
        stencil: true,
    };
}

/// A framebuffer which has only one color attachment.
pub struct SimpleFrameBuffer<'a> {
    context: Rc<Context>,
    attachments: fbo::ValidatedAttachments<'a>,
}

impl<'a> SimpleFrameBuffer<'a> {
    /// Returns the framebuffer object that SafeGL draws to the attachments through, creating it
    /// if needed, so that code outside SafeGL can draw into them too (a video decoder, for
    /// example).
    ///
    /// The framebuffer object belongs to SafeGL, which deletes it with its attachments. Code
    /// outside SafeGL must not delete it or change its attachments, and must be followed by
    /// [`Context::restore_state`](crate::context::Context::restore_state) if it binds it.
    pub fn get_framebuffer_object(&self) -> glow::Framebuffer {
        let mut ctxt = self.context.make_current();
        fbo::FramebuffersContainer::get_framebuffer_for_drawing(&mut ctxt, self.get_attachments())
            .expect("a framebuffer with attachments always has a framebuffer object")
    }

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

    /// Creates a `SimpleFrameBuffer` with a single color attachment and no depth
    /// nor stencil buffer.
    #[inline]
    pub fn new<F: ?Sized, C>(facade: &F, color: C) -> Result<SimpleFrameBuffer<'a>, ValidationError>
    where
        C: ToColorAttachment<'a>,
        F: Facade,
    {
        SimpleFrameBuffer::new_impl(facade, Some(color.to_color_attachment()), None, None, None)
    }

    /// Creates a `SimpleFrameBuffer` with a single color attachment and a depth
    /// buffer, but no stencil buffer.
    #[inline]
    pub fn with_depth_buffer<F: ?Sized, C, D>(
        facade: &F,
        color: C,
        depth: D,
    ) -> Result<SimpleFrameBuffer<'a>, ValidationError>
    where
        C: ToColorAttachment<'a>,
        D: ToDepthAttachment<'a>,
        F: Facade,
    {
        SimpleFrameBuffer::new_impl(
            facade,
            Some(color.to_color_attachment()),
            Some(depth.to_depth_attachment()),
            None,
            None,
        )
    }

    /// Creates a `SimpleFrameBuffer` with a depth buffer and no color attachment
    /// nor stencil buffer.
    #[inline]
    pub fn depth_only<F: ?Sized, D>(
        facade: &F,
        depth: D,
    ) -> Result<SimpleFrameBuffer<'a>, ValidationError>
    where
        D: ToDepthAttachment<'a>,
        F: Facade,
    {
        SimpleFrameBuffer::new_impl(facade, None, Some(depth.to_depth_attachment()), None, None)
    }

    /// Creates a `SimpleFrameBuffer` with a single color attachment, a depth
    /// buffer, and a stencil buffer.
    #[inline]
    pub fn with_depth_and_stencil_buffer<F: ?Sized, C, D, S>(
        facade: &F,
        color: C,
        depth: D,
        stencil: S,
    ) -> Result<SimpleFrameBuffer<'a>, ValidationError>
    where
        C: ToColorAttachment<'a>,
        D: ToDepthAttachment<'a>,
        S: ToStencilAttachment<'a>,
        F: Facade,
    {
        SimpleFrameBuffer::new_impl(
            facade,
            Some(color.to_color_attachment()),
            Some(depth.to_depth_attachment()),
            Some(stencil.to_stencil_attachment()),
            None,
        )
    }

    /// Creates a `SimpleFrameBuffer` with a depth buffer and a stencil buffer,
    /// but no color attachment.
    #[inline]
    pub fn depth_and_stencil_only<F: ?Sized, D, S>(
        facade: &F,
        depth: D,
        stencil: S,
    ) -> Result<SimpleFrameBuffer<'a>, ValidationError>
    where
        D: ToDepthAttachment<'a>,
        S: ToStencilAttachment<'a>,
        F: Facade,
    {
        SimpleFrameBuffer::new_impl(
            facade,
            None,
            Some(depth.to_depth_attachment()),
            Some(stencil.to_stencil_attachment()),
            None,
        )
    }

    /// Creates a `SimpleFrameBuffer` with a single color attachment and a stencil
    /// buffer, but no depth buffer.
    #[inline]
    pub fn with_stencil_buffer<F: ?Sized, C, S>(
        facade: &F,
        color: C,
        stencil: S,
    ) -> Result<SimpleFrameBuffer<'a>, ValidationError>
    where
        C: ToColorAttachment<'a>,
        S: ToStencilAttachment<'a>,
        F: Facade,
    {
        SimpleFrameBuffer::new_impl(
            facade,
            Some(color.to_color_attachment()),
            None,
            Some(stencil.to_stencil_attachment()),
            None,
        )
    }

    /// Creates a `SimpleFrameBuffer` with a stencil buffer and no color attachment
    /// nor depth buffer.
    #[inline]
    pub fn stencil_only<F: ?Sized, S>(
        facade: &F,
        stencil: S,
    ) -> Result<SimpleFrameBuffer<'a>, ValidationError>
    where
        S: ToStencilAttachment<'a>,
        F: Facade,
    {
        SimpleFrameBuffer::new_impl(
            facade,
            None,
            None,
            Some(stencil.to_stencil_attachment()),
            None,
        )
    }

    /// Creates a `SimpleFrameBuffer` with a single color attachment and a depth-stencil buffer.
    #[inline]
    pub fn with_depth_stencil_buffer<F: ?Sized, C, D>(
        facade: &F,
        color: C,
        depthstencil: D,
    ) -> Result<SimpleFrameBuffer<'a>, ValidationError>
    where
        C: ToColorAttachment<'a>,
        D: ToDepthStencilAttachment<'a>,
        F: Facade,
    {
        SimpleFrameBuffer::new_impl(
            facade,
            Some(color.to_color_attachment()),
            None,
            None,
            Some(depthstencil.to_depth_stencil_attachment()),
        )
    }

    /// Creates a `SimpleFrameBuffer` with a depth-stencil buffer and no color attachment.
    #[inline]
    pub fn depth_stencil_only<F: ?Sized, D>(
        facade: &F,
        depthstencil: D,
    ) -> Result<SimpleFrameBuffer<'a>, ValidationError>
    where
        D: ToDepthStencilAttachment<'a>,
        F: Facade,
    {
        SimpleFrameBuffer::new_impl(
            facade,
            None,
            None,
            None,
            Some(depthstencil.to_depth_stencil_attachment()),
        )
    }

    fn new_impl<F: ?Sized>(
        facade: &F,
        color: Option<ColorAttachment<'a>>,
        depth: Option<DepthAttachment<'a>>,
        stencil: Option<StencilAttachment<'a>>,
        depthstencil: Option<DepthStencilAttachment<'a>>,
    ) -> Result<SimpleFrameBuffer<'a>, ValidationError>
    where
        F: Facade,
    {
        let color = color.map(|color| match color {
            ColorAttachment::Texture(tex) => fbo::RegularAttachment::Texture(tex),
            ColorAttachment::RenderBuffer(buffer) => fbo::RegularAttachment::RenderBuffer(buffer),
        });

        let depth = depth.map(|depth| match depth {
            DepthAttachment::Texture(tex) => fbo::RegularAttachment::Texture(tex),
            DepthAttachment::RenderBuffer(buffer) => fbo::RegularAttachment::RenderBuffer(buffer),
        });

        let stencil = stencil.map(|stencil| match stencil {
            StencilAttachment::Texture(tex) => fbo::RegularAttachment::Texture(tex),
            StencilAttachment::RenderBuffer(buffer) => fbo::RegularAttachment::RenderBuffer(buffer),
        });

        let depthstencil = depthstencil.map(|depthstencil| match depthstencil {
            DepthStencilAttachment::Texture(tex) => fbo::RegularAttachment::Texture(tex),
            DepthStencilAttachment::RenderBuffer(buffer) => {
                fbo::RegularAttachment::RenderBuffer(buffer)
            }
        });

        let attachments =
            fbo::FramebufferAttachments::Regular(fbo::FramebufferSpecificAttachments {
                colors: if let Some(color) = color {
                    let mut v = SmallVec::new();
                    v.push((0, color));
                    v
                } else {
                    SmallVec::new()
                },
                depth_stencil: if let (Some(depth), Some(stencil)) = (depth, stencil) {
                    fbo::DepthStencilAttachments::DepthAndStencilAttachments(depth, stencil)
                } else if let Some(depth) = depth {
                    fbo::DepthStencilAttachments::DepthAttachment(depth)
                } else if let Some(stencil) = stencil {
                    fbo::DepthStencilAttachments::StencilAttachment(stencil)
                } else if let Some(depthstencil) = depthstencil {
                    fbo::DepthStencilAttachments::DepthStencilAttachment(depthstencil)
                } else {
                    fbo::DepthStencilAttachments::None
                },
            });

        let attachments = attachments.validate(facade)?;
        fbo::FramebuffersContainer::check_complete(
            &mut facade.get_context().make_current(),
            &attachments,
        )?;

        Ok(SimpleFrameBuffer {
            context: facade.get_context().clone(),
            attachments,
        })
    }
}

impl<'a> Surface for SimpleFrameBuffer<'a> {
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
            fbo::FramebufferTarget::Attachments(&self.attachments),
            rect,
            color,
            color_srgb,
            depth,
            stencil,
        );
    }

    #[inline]
    fn get_dimensions(&self) -> (u32, u32) {
        self.attachments.get_dimensions()
    }

    #[inline]
    fn get_depth_buffer_bits(&self) -> Option<u16> {
        self.attachments.get_depth_buffer_bits()
    }

    #[inline]
    fn get_stencil_buffer_bits(&self) -> Option<u16> {
        self.attachments.get_stencil_buffer_bits()
    }

    fn draw<'b, 'v, V, I, U>(
        &mut self,
        vb: V,
        ib: I,
        program: &crate::Program,
        uniforms: &U,
        draw_parameters: &crate::DrawParameters<'_>,
    ) -> Result<(), DrawError>
    where
        I: Into<crate::index::IndicesSource<'b>>,
        U: crate::uniforms::Uniforms,
        V: crate::vertex::MultiVerticesSource<'v>,
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
            fbo::FramebufferTarget::Attachments(&self.attachments),
            vb,
            ib.into(),
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
        target.blit_from_simple_framebuffer(self, source_rect, target_rect, filter)
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

impl<'a> FboAttachments for SimpleFrameBuffer<'a> {
    #[inline]
    fn get_attachments(&self) -> fbo::FramebufferTarget<'_> {
        fbo::FramebufferTarget::Attachments(&self.attachments)
    }
}

/// A framebuffer which has multiple color attachments.
pub struct MultiOutputFrameBuffer<'a> {
    context: Rc<Context>,
    example_attachments: fbo::ValidatedAttachments<'a>,
    color_attachments: Vec<(String, fbo::RegularAttachment<'a>)>,
    depth_stencil_attachments: fbo::DepthStencilAttachments<fbo::RegularAttachment<'a>>,
}

impl<'a> MultiOutputFrameBuffer<'a> {
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

    /// Creates a new `MultiOutputFrameBuffer`.
    ///
    /// # Panic
    ///
    /// Panics if all attachments don't have the same dimensions.
    #[inline]
    pub fn new<F: ?Sized, I, A>(
        facade: &F,
        color_attachments: I,
    ) -> Result<MultiOutputFrameBuffer<'a>, ValidationError>
    where
        F: Facade,
        I: IntoIterator<Item = (&'a str, A)>,
        A: ToColorAttachment<'a>,
    {
        MultiOutputFrameBuffer::new_impl(facade, color_attachments, None, None, None)
    }

    /// Creates a `MultiOutputFrameBuffer` with a depth buffer.
    ///
    /// # Panic
    ///
    /// Panics if all attachments don't have the same dimensions.
    #[inline]
    pub fn with_depth_buffer<F: ?Sized, D, I, A>(
        facade: &F,
        color_attachments: I,
        depth: D,
    ) -> Result<MultiOutputFrameBuffer<'a>, ValidationError>
    where
        F: Facade,
        D: ToDepthAttachment<'a>,
        I: IntoIterator<Item = (&'a str, A)>,
        A: ToColorAttachment<'a>,
    {
        MultiOutputFrameBuffer::new_impl(
            facade,
            color_attachments,
            Some(depth.to_depth_attachment()),
            None,
            None,
        )
    }

    /// Creates a `MultiOutputFrameBuffer` with a depth buffer, and a stencil buffer.
    ///
    /// # Panic
    ///
    /// Panics if all attachments don't have the same dimensions.
    #[inline]
    pub fn with_depth_and_stencil_buffer<A, F: ?Sized, I, D, S>(
        facade: &F,
        color: I,
        depth: D,
        stencil: S,
    ) -> Result<MultiOutputFrameBuffer<'a>, ValidationError>
    where
        D: ToDepthAttachment<'a>,
        I: IntoIterator<Item = (&'a str, A)>,
        S: ToStencilAttachment<'a>,
        A: ToColorAttachment<'a>,
        F: Facade,
    {
        MultiOutputFrameBuffer::new_impl(
            facade,
            color,
            Some(depth.to_depth_attachment()),
            Some(stencil.to_stencil_attachment()),
            None,
        )
    }

    /// Creates a `MultiOutputFrameBuffer` with a stencil buffer, but no depth buffer.
    ///
    /// # Panic
    ///
    /// Panics if all attachments don't have the same dimensions.
    #[inline]
    pub fn with_stencil_buffer<A, F: ?Sized, I, S>(
        facade: &F,
        color: I,
        stencil: S,
    ) -> Result<MultiOutputFrameBuffer<'a>, ValidationError>
    where
        S: ToStencilAttachment<'a>,
        F: Facade,
        I: IntoIterator<Item = (&'a str, A)>,
        A: ToColorAttachment<'a>,
    {
        MultiOutputFrameBuffer::new_impl(
            facade,
            color,
            None,
            Some(stencil.to_stencil_attachment()),
            None,
        )
    }

    /// Creates a `MultiOutputFrameBuffer` with a depth-stencil buffer.
    ///
    /// # Panic
    ///
    /// Panics if all attachments don't have the same dimensions.
    #[inline]
    pub fn with_depth_stencil_buffer<A, F: ?Sized, I, D>(
        facade: &F,
        color: I,
        depthstencil: D,
    ) -> Result<MultiOutputFrameBuffer<'a>, ValidationError>
    where
        D: ToDepthStencilAttachment<'a>,
        F: Facade,
        I: IntoIterator<Item = (&'a str, A)>,
        A: ToColorAttachment<'a>,
    {
        MultiOutputFrameBuffer::new_impl(
            facade,
            color,
            None,
            None,
            Some(depthstencil.to_depth_stencil_attachment()),
        )
    }

    fn new_impl<F: ?Sized, I, A>(
        facade: &F,
        color: I,
        depth: Option<DepthAttachment<'a>>,
        stencil: Option<StencilAttachment<'a>>,
        depthstencil: Option<DepthStencilAttachment<'a>>,
    ) -> Result<MultiOutputFrameBuffer<'a>, ValidationError>
    where
        F: Facade,
        I: IntoIterator<Item = (&'a str, A)>,
        A: ToColorAttachment<'a>,
    {
        let color = color
            .into_iter()
            .map(|(name, tex)| {
                let atch = tex.to_color_attachment();
                let atch = if let ColorAttachment::Texture(t) = atch {
                    t
                } else {
                    panic!()
                };
                (name.to_owned(), fbo::RegularAttachment::Texture(atch))
            })
            .collect::<Vec<_>>();

        let example_color = {
            let mut v = SmallVec::new();
            for e in color
                .iter()
                .enumerate()
                .map(|(index, &(_, tex))| (index as u32, tex))
            {
                v.push(e);
            }
            v
        };

        let depth = depth.map(|depth| match depth {
            DepthAttachment::Texture(tex) => fbo::RegularAttachment::Texture(tex),
            DepthAttachment::RenderBuffer(buffer) => fbo::RegularAttachment::RenderBuffer(buffer),
        });

        let stencil = stencil.map(|stencil| match stencil {
            StencilAttachment::Texture(tex) => fbo::RegularAttachment::Texture(tex),
            StencilAttachment::RenderBuffer(buffer) => fbo::RegularAttachment::RenderBuffer(buffer),
        });

        let depthstencil = depthstencil.map(|depthstencil| match depthstencil {
            DepthStencilAttachment::Texture(tex) => fbo::RegularAttachment::Texture(tex),
            DepthStencilAttachment::RenderBuffer(buffer) => {
                fbo::RegularAttachment::RenderBuffer(buffer)
            }
        });

        let depth_stencil_attachments = if let (Some(depth), Some(stencil)) = (depth, stencil) {
            fbo::DepthStencilAttachments::DepthAndStencilAttachments(depth, stencil)
        } else if let Some(depth) = depth {
            fbo::DepthStencilAttachments::DepthAttachment(depth)
        } else if let Some(stencil) = stencil {
            fbo::DepthStencilAttachments::StencilAttachment(stencil)
        } else if let Some(depthstencil) = depthstencil {
            fbo::DepthStencilAttachments::DepthStencilAttachment(depthstencil)
        } else {
            fbo::DepthStencilAttachments::None
        };

        let example_attachments =
            fbo::FramebufferAttachments::Regular(fbo::FramebufferSpecificAttachments {
                colors: example_color,
                depth_stencil: depth_stencil_attachments,
            })
            .validate(facade)?;
        fbo::FramebuffersContainer::check_complete(
            &mut facade.get_context().make_current(),
            &example_attachments,
        )?;

        Ok(MultiOutputFrameBuffer {
            context: facade.get_context().clone(),
            example_attachments,
            color_attachments: color,
            depth_stencil_attachments,
        })
    }

    fn build_attachments(&self, program: &Program) -> fbo::ValidatedAttachments<'_> {
        let mut colors = SmallVec::new();

        for &(ref name, attachment) in self.color_attachments.iter() {
            let location = match program.get_frag_data_location(&name) {
                Some(l) => l,
                None => panic!(
                    "The fragment output `{}` was not found in the program",
                    name
                ),
            };

            colors.push((location, attachment));
        }

        fbo::FramebufferAttachments::Regular(fbo::FramebufferSpecificAttachments {
            colors,
            depth_stencil: self.depth_stencil_attachments,
        })
        .validate(&self.context)
        .unwrap()
    }
}

impl<'a> Surface for MultiOutputFrameBuffer<'a> {
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
            fbo::FramebufferTarget::Attachments(&self.example_attachments),
            rect,
            color,
            color_srgb,
            depth,
            stencil,
        );
    }

    #[inline]
    fn get_dimensions(&self) -> (u32, u32) {
        self.example_attachments.get_dimensions()
    }

    #[inline]
    fn get_depth_buffer_bits(&self) -> Option<u16> {
        self.example_attachments.get_depth_buffer_bits()
    }

    #[inline]
    fn get_stencil_buffer_bits(&self) -> Option<u16> {
        self.example_attachments.get_stencil_buffer_bits()
    }

    fn draw<'i, 'v, V, I, U>(
        &mut self,
        vb: V,
        ib: I,
        program: &crate::Program,
        uniforms: &U,
        draw_parameters: &crate::DrawParameters<'_>,
    ) -> Result<(), DrawError>
    where
        I: Into<crate::index::IndicesSource<'i>>,
        U: crate::uniforms::Uniforms,
        V: crate::vertex::MultiVerticesSource<'v>,
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
            fbo::FramebufferTarget::Attachments(&self.build_attachments(program)),
            vb,
            ib.into(),
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
        target.blit_from_multioutput_framebuffer(self, source_rect, target_rect, filter)
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

impl<'a> FboAttachments for MultiOutputFrameBuffer<'a> {
    #[inline]
    fn get_attachments(&self) -> fbo::FramebufferTarget<'_> {
        // blits read from the first output
        fbo::FramebufferTarget::Attachments(&self.example_attachments)
    }
}

/// A framebuffer with no attachment at all.
///
/// Note that this is only supported on recent hardware.
#[cfg(feature = "gl46")]
#[cfg_attr(docsrs, doc(cfg(feature = "gl46")))]
pub struct EmptyFrameBuffer {
    context: Rc<Context>,
    attachments: fbo::ValidatedAttachments<'static>,
}

#[cfg(feature = "gl46")]
impl<'a> EmptyFrameBuffer {
    /// Returns true if empty framebuffers are supported by the backend.
    pub fn is_supported<C: ?Sized>(context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        context.get_version() >= &Version(Api::Gl, 4, 3)
            || context.get_version() >= &Version(Api::GlEs, 3, 1)
            || context.get_extensions().gl_arb_framebuffer_no_attachments
    }

    /// Returns true if layered empty framebuffers are supported by the backend.
    pub fn is_layered_supported<C: ?Sized>(context: &C) -> bool
    where
        C: CapabilitiesSource,
    {
        context.get_version() >= &Version(Api::Gl, 4, 3)
            || context.get_version() >= &Version(Api::GlEs, 3, 2)
            || context.get_extensions().gl_arb_framebuffer_no_attachments
    }

    /// Returns the maximum width of empty framebuffers that the backend supports, or `None` if
    /// empty framebuffers are not supported.
    pub fn get_max_supported_width<C: ?Sized>(context: &C) -> Option<u32>
    where
        C: CapabilitiesSource,
    {
        context
            .get_capabilities()
            .max_framebuffer_width
            .map(|v| v as u32)
    }

    /// Returns the maximum height of empty framebuffers that the backend supports, or `None` if
    /// empty framebuffers are not supported.
    pub fn get_max_supported_height<C: ?Sized>(context: &C) -> Option<u32>
    where
        C: CapabilitiesSource,
    {
        context
            .get_capabilities()
            .max_framebuffer_height
            .map(|v| v as u32)
    }

    /// Returns the maximum number of samples of empty framebuffers that the backend supports,
    /// or `None` if empty framebuffers are not supported.
    pub fn get_max_supported_samples<C: ?Sized>(context: &C) -> Option<u32>
    where
        C: CapabilitiesSource,
    {
        context
            .get_capabilities()
            .max_framebuffer_samples
            .map(|v| v as u32)
    }

    /// Returns the maximum number of layers of empty framebuffers that the backend supports,
    /// or `None` if layered empty framebuffers are not supported.
    pub fn get_max_supported_layers<C: ?Sized>(context: &C) -> Option<u32>
    where
        C: CapabilitiesSource,
    {
        context
            .get_capabilities()
            .max_framebuffer_layers
            .map(|v| v as u32)
    }

    /// Creates a `EmptyFrameBuffer`.
    ///
    /// # Panic
    ///
    /// Panics if `layers` or `samples` is equal to `Some(0)`.
    ///
    #[inline]
    pub fn new<F: ?Sized>(
        facade: &F,
        width: u32,
        height: u32,
        layers: Option<u32>,
        samples: Option<u32>,
        fixed_samples: bool,
    ) -> Result<EmptyFrameBuffer, ValidationError>
    where
        F: Facade,
    {
        let context = facade.get_context();

        let attachments = fbo::FramebufferAttachments::Empty {
            width,
            height,
            layers,
            samples,
            fixed_samples,
        };

        let attachments = attachments.validate(context)?;
        fbo::FramebuffersContainer::check_complete(&mut context.make_current(), &attachments)?;

        Ok(EmptyFrameBuffer {
            context: context.clone(),
            attachments,
        })
    }
}

#[cfg(feature = "gl46")]
impl Surface for EmptyFrameBuffer {
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
            fbo::FramebufferTarget::Attachments(&self.attachments),
            rect,
            color,
            color_srgb,
            depth,
            stencil,
        );
    }

    #[inline]
    fn get_dimensions(&self) -> (u32, u32) {
        self.attachments.get_dimensions()
    }

    #[inline]
    fn get_depth_buffer_bits(&self) -> Option<u16> {
        None
    }

    #[inline]
    fn get_stencil_buffer_bits(&self) -> Option<u16> {
        None
    }

    fn draw<'b, 'v, V, I, U>(
        &mut self,
        vb: V,
        ib: I,
        program: &crate::Program,
        uniforms: &U,
        draw_parameters: &crate::DrawParameters<'_>,
    ) -> Result<(), DrawError>
    where
        I: Into<crate::index::IndicesSource<'b>>,
        U: crate::uniforms::Uniforms,
        V: crate::vertex::MultiVerticesSource<'v>,
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
            fbo::FramebufferTarget::Attachments(&self.attachments),
            vb,
            ib.into(),
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
        unimplemented!() // TODO:
    }

    #[inline]
    fn blit_buffers_from_frame(
        &self,
        source_rect: &Rect,
        target_rect: &BlitTarget,
        filter: uniforms::MagnifySamplerFilter,
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
        filter: uniforms::MagnifySamplerFilter,
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
        filter: uniforms::MagnifySamplerFilter,
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
        filter: uniforms::MagnifySamplerFilter,
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

#[cfg(feature = "gl46")]
impl FboAttachments for EmptyFrameBuffer {
    #[inline]
    fn get_attachments(&self) -> fbo::FramebufferTarget<'_> {
        fbo::FramebufferTarget::Attachments(&self.attachments)
    }
}

/// Describes an attachment for a color buffer.
#[derive(Copy, Clone)]
pub enum ColorAttachment<'a> {
    /// A texture.
    Texture(TextureAnyImage<'a>),
    /// A render buffer.
    RenderBuffer(&'a RenderBuffer),
}

/// Trait for objects that can be used as color attachments.
pub trait ToColorAttachment<'a> {
    /// Builds the `ColorAttachment`.
    fn to_color_attachment(self) -> ColorAttachment<'a>;
}

impl<'a> ToColorAttachment<'a> for ColorAttachment<'a> {
    #[inline]
    fn to_color_attachment(self) -> ColorAttachment<'a> {
        self
    }
}

/// Describes an attachment for a depth buffer.
#[derive(Copy, Clone)]
pub enum DepthAttachment<'a> {
    /// A texture.
    Texture(TextureAnyImage<'a>),
    /// A render buffer.
    RenderBuffer(&'a DepthRenderBuffer),
}

/// Trait for objects that can be used as depth attachments.
pub trait ToDepthAttachment<'a> {
    /// Builds the `DepthAttachment`.
    fn to_depth_attachment(self) -> DepthAttachment<'a>;
}

impl<'a> ToDepthAttachment<'a> for DepthAttachment<'a> {
    #[inline]
    fn to_depth_attachment(self) -> DepthAttachment<'a> {
        self
    }
}

/// Describes an attachment for a stencil buffer.
#[derive(Copy, Clone)]
pub enum StencilAttachment<'a> {
    /// A texture.
    Texture(TextureAnyImage<'a>),
    /// A render buffer.
    RenderBuffer(&'a StencilRenderBuffer),
}

/// Trait for objects that can be used as stencil attachments.
pub trait ToStencilAttachment<'a> {
    /// Builds the `StencilAttachment`.
    fn to_stencil_attachment(self) -> StencilAttachment<'a>;
}

impl<'a> ToStencilAttachment<'a> for StencilAttachment<'a> {
    #[inline]
    fn to_stencil_attachment(self) -> StencilAttachment<'a> {
        self
    }
}

/// Describes an attachment for a depth and stencil buffer.
#[derive(Copy, Clone)]
pub enum DepthStencilAttachment<'a> {
    /// A texture.
    Texture(TextureAnyImage<'a>),
    /// A render buffer.
    RenderBuffer(&'a DepthStencilRenderBuffer),
}

/// Trait for objects that can be used as depth and stencil attachments.
pub trait ToDepthStencilAttachment<'a> {
    /// Builds the `DepthStencilAttachment`.
    fn to_depth_stencil_attachment(self) -> DepthStencilAttachment<'a>;
}

impl<'a> ToDepthStencilAttachment<'a> for DepthStencilAttachment<'a> {
    #[inline]
    fn to_depth_stencil_attachment(self) -> DepthStencilAttachment<'a> {
        self
    }
}
