//! Framebuffers whose attachments are whole texture levels with their layers.

use std::rc::Rc;

use smallvec::SmallVec;

use crate::backend::Facade;
use crate::context::Context;
use crate::fbo;
use crate::framebuffer::{ExternalFramebuffer, MultiOutputFrameBuffer, SimpleFrameBuffer};
use crate::ops;
use crate::texture::TextureAnyMipmap;
use crate::uniforms::MagnifySamplerFilter;
use crate::{
    BlitError, BlitMask, BlitTarget, ContextExt, DrawError, FboAttachments, Program, Rect, Surface,
    ToGlEnum,
};

#[cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))]
use crate::framebuffer::InvalidateAttachments;
use crate::framebuffer::ValidationError;

/// A whole level of a texture with layers, attached as a layered color attachment: a level of
/// an array texture, a cube map, a cube map array or a 3D texture.
#[derive(Copy, Clone)]
pub struct LayeredColorAttachment<'a>(pub(crate) TextureAnyMipmap<'a>);

/// A whole level of a depth texture with layers, attached as a layered depth attachment.
#[derive(Copy, Clone)]
pub struct LayeredDepthAttachment<'a>(pub(crate) TextureAnyMipmap<'a>);

/// A whole level of a stencil texture with layers, attached as a layered stencil attachment.
#[derive(Copy, Clone)]
pub struct LayeredStencilAttachment<'a>(pub(crate) TextureAnyMipmap<'a>);

/// A whole level of a depth-stencil texture with layers, attached as a layered depth and stencil
/// attachment.
#[derive(Copy, Clone)]
pub struct LayeredDepthStencilAttachment<'a>(pub(crate) TextureAnyMipmap<'a>);

/// Trait for the texture levels with layers that can be layered color attachments.
///
/// Implemented by the mipmaps of the color textures with layers (`Texture2dArrayMipmap`,
/// `CubemapMipmap`, `Texture3dMipmap`, ...) and by references to the textures themselves, which
/// attach their main level.
pub trait ToLayeredColorAttachment<'a> {
    /// Builds the `LayeredColorAttachment`.
    fn to_layered_color_attachment(self) -> LayeredColorAttachment<'a>;
}

/// Trait for the depth texture levels with layers that can be layered depth attachments.
pub trait ToLayeredDepthAttachment<'a> {
    /// Builds the `LayeredDepthAttachment`.
    fn to_layered_depth_attachment(self) -> LayeredDepthAttachment<'a>;
}

/// Trait for the stencil texture levels with layers that can be layered stencil attachments.
pub trait ToLayeredStencilAttachment<'a> {
    /// Builds the `LayeredStencilAttachment`.
    fn to_layered_stencil_attachment(self) -> LayeredStencilAttachment<'a>;
}

/// Trait for the depth-stencil texture levels with layers that can be layered depth and stencil
/// attachments.
pub trait ToLayeredDepthStencilAttachment<'a> {
    /// Builds the `LayeredDepthStencilAttachment`.
    fn to_layered_depth_stencil_attachment(self) -> LayeredDepthStencilAttachment<'a>;
}

impl<'a> ToLayeredColorAttachment<'a> for LayeredColorAttachment<'a> {
    #[inline]
    fn to_layered_color_attachment(self) -> LayeredColorAttachment<'a> {
        self
    }
}

impl<'a> ToLayeredDepthAttachment<'a> for LayeredDepthAttachment<'a> {
    #[inline]
    fn to_layered_depth_attachment(self) -> LayeredDepthAttachment<'a> {
        self
    }
}

impl<'a> ToLayeredStencilAttachment<'a> for LayeredStencilAttachment<'a> {
    #[inline]
    fn to_layered_stencil_attachment(self) -> LayeredStencilAttachment<'a> {
        self
    }
}

impl<'a> ToLayeredDepthStencilAttachment<'a> for LayeredDepthStencilAttachment<'a> {
    #[inline]
    fn to_layered_depth_stencil_attachment(self) -> LayeredDepthStencilAttachment<'a> {
        self
    }
}

/// The depth and stencil attachments of a `LayeredFrameBuffer`.
#[derive(Copy, Clone, Default)]
pub enum LayeredDepthStencil<'a> {
    /// No depth or stencil buffer.
    #[default]
    None,
    /// A depth buffer.
    Depth(LayeredDepthAttachment<'a>),
    /// A stencil buffer.
    Stencil(LayeredStencilAttachment<'a>),
    /// A depth buffer and a separate stencil buffer.
    DepthAndStencil(LayeredDepthAttachment<'a>, LayeredStencilAttachment<'a>),
    /// A single buffer that is both the depth and the stencil buffer.
    DepthStencil(LayeredDepthStencilAttachment<'a>),
}

impl<'a> LayeredDepthStencil<'a> {
    fn to_fbo(self) -> fbo::DepthStencilAttachments<fbo::LayeredAttachment<'a>> {
        use fbo::DepthStencilAttachments as A;
        use fbo::LayeredAttachment as L;

        match self {
            LayeredDepthStencil::None => A::None,
            LayeredDepthStencil::Depth(d) => A::DepthAttachment(L(d.0)),
            LayeredDepthStencil::Stencil(s) => A::StencilAttachment(L(s.0)),
            LayeredDepthStencil::DepthAndStencil(d, s) => {
                A::DepthAndStencilAttachments(L(d.0), L(s.0))
            }
            LayeredDepthStencil::DepthStencil(ds) => A::DepthStencilAttachment(L(ds.0)),
        }
    }
}

/// A framebuffer whose attachments are whole texture levels with their layers: the layers of
/// array textures, the faces of cube maps and cube map arrays, or the slices of 3D textures.
///
/// A geometry shader chooses the layer of each primitive by writing `gl_Layer` (the face of a
/// cube map is `gl_Layer % 6`), so that a single draw renders, for instance, the six faces of a
/// shadow cube map or the cascades of a shadow map array. Primitives without `gl_Layer` go to
/// layer 0, and a fragment shader can read `gl_Layer` too. Clears clear every layer.
///
/// The framebuffer has the layers of its attachment that has the fewest
/// (`LayeredFrameBuffer::layers`). Its color attachments must all be textures of the same kind,
/// or `ValidationError::LayerTargetsMismatch` is returned; its depth and stencil attachments
/// can be of another kind. Blits from it read layer 0.
///
/// Needs OpenGL ES 3.2 or `GL_EXT_geometry_shader` on OpenGL ES, otherwise creating it returns
/// `ValidationError::LayeredFramebuffersNotSupported`.
pub struct LayeredFrameBuffer<'a> {
    context: Rc<Context>,
    example_attachments: fbo::ValidatedAttachments<'a>,
    // the name of the fragment output of each color attachment, `None` for the single color
    // attachment of the constructors that don't take names, which is at location 0
    color_attachments: Vec<(Option<String>, fbo::LayeredAttachment<'a>)>,
    depth_stencil_attachments: fbo::DepthStencilAttachments<fbo::LayeredAttachment<'a>>,
}

impl<'a> LayeredFrameBuffer<'a> {
    /// Returns true if the backend supports layered framebuffers: desktop OpenGL, OpenGL ES 3.2
    /// or `GL_EXT_geometry_shader`.
    pub fn is_supported<F: ?Sized + Facade>(facade: &F) -> bool {
        fbo::is_layered_supported(&facade.get_context().make_current())
    }

    /// Creates a framebuffer with a single layered color attachment, written by the fragment
    /// output at location 0.
    pub fn new<F, C>(facade: &F, color: C) -> Result<LayeredFrameBuffer<'a>, ValidationError>
    where
        F: ?Sized + Facade,
        C: ToLayeredColorAttachment<'a>,
    {
        Self::new_impl(
            facade,
            vec![(None, color.to_layered_color_attachment())],
            LayeredDepthStencil::None,
        )
    }

    /// Creates a framebuffer with a layered color attachment and a layered depth buffer.
    pub fn with_depth_buffer<F, C, D>(
        facade: &F,
        color: C,
        depth: D,
    ) -> Result<LayeredFrameBuffer<'a>, ValidationError>
    where
        F: ?Sized + Facade,
        C: ToLayeredColorAttachment<'a>,
        D: ToLayeredDepthAttachment<'a>,
    {
        Self::new_impl(
            facade,
            vec![(None, color.to_layered_color_attachment())],
            LayeredDepthStencil::Depth(depth.to_layered_depth_attachment()),
        )
    }

    /// Creates a framebuffer with a layered color attachment and a layered stencil buffer.
    pub fn with_stencil_buffer<F, C, S>(
        facade: &F,
        color: C,
        stencil: S,
    ) -> Result<LayeredFrameBuffer<'a>, ValidationError>
    where
        F: ?Sized + Facade,
        C: ToLayeredColorAttachment<'a>,
        S: ToLayeredStencilAttachment<'a>,
    {
        Self::new_impl(
            facade,
            vec![(None, color.to_layered_color_attachment())],
            LayeredDepthStencil::Stencil(stencil.to_layered_stencil_attachment()),
        )
    }

    /// Creates a framebuffer with a layered color attachment and a layered depth-stencil
    /// buffer.
    pub fn with_depth_stencil_buffer<F, C, D>(
        facade: &F,
        color: C,
        depth_stencil: D,
    ) -> Result<LayeredFrameBuffer<'a>, ValidationError>
    where
        F: ?Sized + Facade,
        C: ToLayeredColorAttachment<'a>,
        D: ToLayeredDepthStencilAttachment<'a>,
    {
        Self::new_impl(
            facade,
            vec![(None, color.to_layered_color_attachment())],
            LayeredDepthStencil::DepthStencil(depth_stencil.to_layered_depth_stencil_attachment()),
        )
    }

    /// Creates a framebuffer with only a layered depth buffer, such as a shadow cube map.
    pub fn depth_only<F, D>(facade: &F, depth: D) -> Result<LayeredFrameBuffer<'a>, ValidationError>
    where
        F: ?Sized + Facade,
        D: ToLayeredDepthAttachment<'a>,
    {
        Self::new_impl(
            facade,
            Vec::new(),
            LayeredDepthStencil::Depth(depth.to_layered_depth_attachment()),
        )
    }

    /// Creates a framebuffer with several layered color attachments, each written by the
    /// fragment output of the given name, and the given depth and stencil attachments.
    pub fn with_outputs<F, I, C>(
        facade: &F,
        outputs: I,
        depth_stencil: LayeredDepthStencil<'a>,
    ) -> Result<LayeredFrameBuffer<'a>, ValidationError>
    where
        F: ?Sized + Facade,
        I: IntoIterator<Item = (&'a str, C)>,
        C: ToLayeredColorAttachment<'a>,
    {
        let colors = outputs
            .into_iter()
            .map(|(name, color)| (Some(name), color.to_layered_color_attachment()))
            .collect();
        Self::new_impl(facade, colors, depth_stencil)
    }

    fn new_impl<F: ?Sized + Facade>(
        facade: &F,
        colors: Vec<(Option<&str>, LayeredColorAttachment<'a>)>,
        depth_stencil: LayeredDepthStencil<'a>,
    ) -> Result<LayeredFrameBuffer<'a>, ValidationError> {
        let color_attachments = colors
            .into_iter()
            .map(|(name, color)| (name.map(str::to_owned), fbo::LayeredAttachment(color.0)))
            .collect::<Vec<_>>();
        let depth_stencil_attachments = depth_stencil.to_fbo();

        // the attachments as they would be with the outputs at the locations 0, 1, ...
        let example_colors = color_attachments
            .iter()
            .enumerate()
            .map(|(index, (_, attachment))| (index as u32, *attachment))
            .collect::<SmallVec<_>>();
        let mut ctxt = facade.get_context().make_current();
        let example_attachments =
            fbo::FramebufferAttachments::Layered(fbo::FramebufferSpecificAttachments {
                colors: example_colors,
                depth_stencil: depth_stencil_attachments,
            })
            .validate(&mut ctxt)?;
        fbo::FramebuffersContainer::check_complete(&mut ctxt, &example_attachments)?;
        drop(ctxt);

        Ok(LayeredFrameBuffer {
            context: facade.get_context().clone(),
            example_attachments,
            color_attachments,
            depth_stencil_attachments,
        })
    }

    /// Returns the number of layers of the framebuffer: the layers of its attachment that has
    /// the fewest.
    pub fn layers(&self) -> u32 {
        self.example_attachments
            .get_layers()
            .expect("the attachments of a layered framebuffer have layers")
    }

    /// Discards the content of the attachments selected by `which`, in every layer
    /// (`glInvalidateFramebuffer`).
    ///
    /// This is only a hint: it does nothing without OpenGL 4.3, OpenGL ES 3.0 or
    /// `GL_ARB_invalidate_subdata`.
    #[cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))))]
    pub fn invalidate(&self, which: InvalidateAttachments) {
        let mut ctxt = self.context.make_current();
        unsafe {
            fbo::FramebuffersContainer::invalidate(&mut ctxt, self.get_attachments(), which, None)
        }
    }

    /// Returns the attachments with the colors at the locations of their outputs in `program`.
    fn build_attachments(
        &self,
        program: &Program,
    ) -> Result<fbo::ValidatedAttachments<'_>, DrawError> {
        if self
            .color_attachments
            .iter()
            .all(|(name, _)| name.is_none())
        {
            return Ok(self.example_attachments.clone());
        }

        let mut colors = SmallVec::new();
        for (name, attachment) in &self.color_attachments {
            let name = name.as_deref().unwrap_or_default();
            let location = program.get_frag_data_location(name).ok_or_else(|| {
                DrawError::FragmentOutputMissing {
                    name: name.to_owned(),
                }
            })?;
            colors.push((location, *attachment));
        }

        // the same attachments were validated when the framebuffer was created
        Ok(
            fbo::FramebufferAttachments::Layered(fbo::FramebufferSpecificAttachments {
                colors,
                depth_stencil: self.depth_stencil_attachments,
            })
            .validate(&mut self.context.make_current())
            .expect("the attachments of the framebuffer were validated when it was created"),
        )
    }
}

impl<'a> Surface for LayeredFrameBuffer<'a> {
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
        program: &Program,
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
            fbo::FramebufferTarget::Attachments(&self.build_attachments(program)?),
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
        filter: MagnifySamplerFilter,
    ) -> Result<(), BlitError>
    where
        S: Surface,
    {
        target.blit_from_layered_framebuffer(self, source_rect, target_rect, filter)
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

    #[inline]
    fn blit_buffers_from_layered_framebuffer(
        &self,
        source: &LayeredFrameBuffer<'_>,
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

impl<'a> FboAttachments for LayeredFrameBuffer<'a> {
    #[inline]
    fn get_fbo_context(&self) -> &Context {
        &self.context
    }

    #[inline]
    fn get_attachments(&self) -> fbo::FramebufferTarget<'_> {
        fbo::FramebufferTarget::Attachments(&self.example_attachments)
    }
}
