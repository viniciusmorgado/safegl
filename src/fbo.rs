/*!
Contains everything related to the internal handling of framebuffer objects.

*/
/*
Here are the rules taken from the official wiki:

Attachment Completeness

Each attachment point itctxt.framebuffer_objects must be complete according to these rules. Empty attachments
(attachments with no image attached) are complete by default. If an image is attached, it must
adhere to the following rules:

The source object for the image still exists and has the same type it was attached with.
The image has a non-zero width and height (the height of a 1D image is assumed to be 1). The
  width/height must also be less than GL_MAX_FRAMEBUFFER_WIDTH and GL_MAX_FRAMEBUFFER_HEIGHT
  respectively (if GL 4.3/ARB_framebuffer_no_attachments).
The layer for 3D or array textures attachments is less than the depth of the texture. It must
  also be less than GL_MAX_FRAMEBUFFER_LAYERS (if GL 4.3/ARB_framebuffer_no_attachments).
The number of samples must be less than GL_MAX_FRAMEBUFFER_SAMPLES (if
  GL 4.3/ARB_framebuffer_no_attachments).
The image's format must match the attachment point's requirements, as defined above.
  Color-renderable formats for color attachments, etc.

Completeness Rules

These are the rules for framebuffer completeness. The order of these rules matters.

If the target​ of glCheckFramebufferStatus references the Default Framebuffer (ie: FBO object
  number 0 is bound), and the default framebuffer does not exist, then you will get
  GL_FRAMEBUFFER_UNDEFINEZ. If the default framebuffer exists, then you always get
  GL_FRAMEBUFFER_COMPLETE. The rest of the rules apply when an FBO is bound.
All attachments must be attachment complete. (GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT when false).
There must be at least one image attached to the FBO, or if OpenGL 4.3 or
  ARB_framebuffer_no_attachment is available, the GL_FRAMEBUFFER_DEFAULT_WIDTH and
  GL_FRAMEBUFFER_DEFAULT_HEIGHT parameters of the framebuffer must both be non-zero.
  (GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT when false).
Each draw buffers must either specify color attachment points that have images attached or
  must be GL_NONE. (GL_FRAMEBUFFER_INCOMPLETE_DRAW_BUFFER when false). Note that this test is
  not performed if OpenGL 4.1 or ARB_ES2_compatibility is available.
If the read buffer is set, then it must specify an attachment point that has an image
  attached. (GL_FRAMEBUFFER_INCOMPLETE_READ_BUFFER when false). Note that this test is not
  performed if OpenGL 4.1 or ARB_ES2_compatibility is available.
All images must have the same number of multisample samples.
  (GL_FRAMEBUFFER_INCOMPLETE_MULTISAMPLE when false).
If a layered image is attached to one attachment, then all attachments must be layered
  attachments. The attached layers do not have to have the same number of layers, nor do the
  layers have to come from the same kind of texture (a cubemap color texture can be paired
  with an array depth texture) (GL_FRAMEBUFFER_INCOMPLETE_LAYER_TARGETS when false).

*/
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::hash::BuildHasherDefault;
use std::marker::PhantomData;
use std::{cmp, fmt, mem};

use fnv::FnvHasher;
use glow::HasContext;
use smallvec::SmallVec;

use crate::CapabilitiesSource;
use crate::GlObject;
use crate::TextureExt;

#[cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))]
use crate::framebuffer::InvalidateAttachments;
use crate::framebuffer::RenderBufferAny;
use crate::texture::CubeLayer;
use crate::image_format::TextureFormatRequest;
use crate::texture::TextureAny;
use crate::texture::TextureAnyImage;
use crate::texture::TextureAnyMipmap;
use crate::texture::TextureKind;

use crate::context::CommandContext;
use crate::context::Context;
use crate::gl;
use crate::version::Api;
use crate::version::Version;

/// Represents the attachments to use for an OpenGL framebuffer.
#[derive(Clone)]
pub enum FramebufferAttachments<'a> {
    /// Each attachment is a single image.
    Regular(FramebufferSpecificAttachments<RegularAttachment<'a>>),

    /// Each attachment is a layer of images.
    Layered(FramebufferSpecificAttachments<LayeredAttachment<'a>>),

    /// Each attachment is the views `base_view..base_view + views` of a level of a 2D array
    /// texture, which draws render to at once (`GL_OVR_multiview2`).
    Multiview {
        attachments: FramebufferSpecificAttachments<LayeredAttachment<'a>>,
        base_view: u32,
        views: u32,
    },

    /// An empty framebuffer.
    Empty {
        width: u32,
        height: u32,
        layers: Option<u32>,
        samples: Option<u32>,
        fixed_samples: bool,
    },
}

/// Describes a single non-layered framebuffer attachment.
#[derive(Copy, Clone)]
pub enum RegularAttachment<'a> {
    /// A texture.
    Texture(TextureAnyImage<'a>),
    /// A renderbuffer.
    RenderBuffer(&'a RenderBufferAny),
}

impl<'a> RegularAttachment<'a> {
    /// Returns true if the attachment was created by `context`.
    fn belongs_to(&self, context: &Context) -> bool {
        let owner: &Context = match self {
            RegularAttachment::Texture(t) => TextureExt::get_context(t.get_texture()),
            RegularAttachment::RenderBuffer(rb) => rb.get_context(),
        };
        std::ptr::eq(owner, context)
    }

    /// Returns the kind of attachment (float, integral, unsigned, depth, stencil, depthstencil).
    #[inline]
    pub fn kind(&self) -> TextureKind {
        match self {
            RegularAttachment::Texture(t) => t.get_texture().kind(),
            RegularAttachment::RenderBuffer(rb) => rb.kind(),
        }
    }
}

/// Describes a single layered framebuffer attachment: a whole level of a texture with layers.
#[derive(Copy, Clone)]
pub struct LayeredAttachment<'a>(pub(crate) TextureAnyMipmap<'a>);

impl<'a> LayeredAttachment<'a> {
    /// Returns true if the attachment was created by `context`.
    fn belongs_to(&self, context: &Context) -> bool {
        std::ptr::eq(&**TextureExt::get_context(self.0.get_texture()), context)
    }

    /// Returns the number of layers of the attachment: the faces of cube maps are layers, and
    /// so are the slices of 3D textures.
    fn layers(&self) -> u32 {
        use crate::texture::Dimensions;

        match self.0.get_texture().get_texture_type() {
            Dimensions::Texture1dArray { array_size, .. }
            | Dimensions::Texture2dArray { array_size, .. }
            | Dimensions::Texture2dMultisampleArray { array_size, .. } => array_size,
            Dimensions::Cubemap { .. } => 6,
            Dimensions::CubemapArray { array_size, .. } => 6 * array_size,
            _ => self.0.get_depth().unwrap_or(1),
        }
    }
}

/// Returns true if the backend has multiview framebuffers (`GL_OVR_multiview2`, or
/// `OVR_multiview2` on WebGL).
pub fn is_multiview_supported(ctxt: &CommandContext<'_>) -> bool {
    ctxt.extensions.gl_ovr_multiview2
}

/// Returns true if framebuffers render to textures with samples that the GPU resolves
/// (`GL_EXT_multisampled_render_to_texture`), which WebGL doesn't have.
pub fn is_multisampled_render_to_texture_supported(context: &Context) -> bool {
    let extensions = context.get_extensions();
    !cfg!(safegl_browser)
        && (extensions.gl_ext_multisampled_render_to_texture
            || extensions.gl_ext_multisampled_render_to_texture2)
}

/// Checks the attachments whose samples the GPU resolves when it writes them
/// (`GL_EXT_multisampled_render_to_texture`): the images given samples
/// (`TextureAnyImage::multisampled`) and the render buffers built with
/// `new_multisample_resolved`. Each attachment comes with whether it is the first color
/// attachment, the only one where the images given samples can be without
/// `GL_EXT_multisampled_render_to_texture2`. The other multisampled attachments of the
/// framebuffer can't have samples of their own.
fn check_resolved_samples(
    context: &Context,
    attachments: &[(bool, &RegularAttachment<'_>)],
) -> Result<(), ValidationError> {
    let mut resolved = false;
    let mut own_samples = false;
    for &(first_color, attachment) in attachments {
        match attachment {
            RegularAttachment::Texture(image) => {
                let Some(samples) = image.get_render_samples() else {
                    own_samples |= image.get_samples().is_some();
                    continue;
                };
                resolved = true;
                let second = context.get_extensions().gl_ext_multisampled_render_to_texture2;
                if !is_multisampled_render_to_texture_supported(context)
                    || (!first_color && !second)
                {
                    return Err(ValidationError::MultisampledRenderToTextureNotSupported);
                }
                let texture = image.get_texture();
                let bind_point = texture.get_bind_point();
                if bind_point != gl::TEXTURE_2D && bind_point != gl::TEXTURE_CUBE_MAP {
                    return Err(ValidationError::MultisampledRenderToTextureTarget);
                }
                let capabilities = context.get_capabilities();
                let mut max = capabilities.limits.max_samples.max(0) as u32;
                // the samples of a format are listed for render buffers
                if let TextureFormatRequest::Specific(format) = texture.get_requested_format() {
                    let format_samples = capabilities
                        .internal_formats_renderbuffers
                        .get(&format)
                        .and_then(|infos| infos.multisamples.as_ref());
                    if let Some(format_samples) = format_samples {
                        let format_max = format_samples.iter().copied().max().unwrap_or(0);
                        max = max.min(format_max.max(0) as u32);
                    }
                }
                if samples > max {
                    return Err(ValidationError::SamplesNotSupported);
                }
            }
            RegularAttachment::RenderBuffer(buffer) => {
                if buffer.is_multisample_resolved() {
                    resolved = true;
                } else {
                    own_samples |= buffer.get_samples().is_some_and(|samples| samples > 0);
                }
            }
        }
    }
    if resolved && own_samples {
        return Err(ValidationError::SamplesCountMismatch);
    }
    Ok(())
}

/// Returns true if the backend can attach whole textures with layers to framebuffers
/// (`glFramebufferTexture`, core in OpenGL 3.2 and OpenGL ES 3.2).
pub fn is_layered_supported(ctxt: &CommandContext<'_>) -> bool {
    ctxt.version.0 == Api::Gl
        || ctxt.version >= &Version(Api::GlEs, 3, 2)
        || ctxt.extensions.gl_ext_geometry_shader
}

impl<T> FramebufferSpecificAttachments<T> {
    /// Returns the values that `f` gives for the attachments.
    fn collect<R>(&self, mut f: impl FnMut(&T) -> Option<R>) -> SmallVec<[R; 5]> {
        let mut values = SmallVec::new();
        match self.depth_stencil {
            DepthStencilAttachments::None => (),
            DepthStencilAttachments::DepthAttachment(ref a)
            | DepthStencilAttachments::StencilAttachment(ref a)
            | DepthStencilAttachments::DepthStencilAttachment(ref a) => values.extend(f(a)),
            DepthStencilAttachments::DepthAndStencilAttachments(ref a, ref b) => {
                values.extend(f(a));
                values.extend(f(b));
            }
        }
        values.extend(self.colors.iter().filter_map(|(_, a)| f(a)));
        values
    }

    /// Returns true if `f` returns true for all the attachments.
    fn all(&self, mut f: impl FnMut(&T) -> bool) -> bool {
        let depth_stencil = match self.depth_stencil {
            DepthStencilAttachments::None => true,
            DepthStencilAttachments::DepthAttachment(ref a)
            | DepthStencilAttachments::StencilAttachment(ref a)
            | DepthStencilAttachments::DepthStencilAttachment(ref a) => f(a),
            DepthStencilAttachments::DepthAndStencilAttachments(ref a, ref b) => f(a) && f(b),
        };
        depth_stencil && self.colors.iter().all(|(_, a)| f(a))
    }
}

/// Depth and/or stencil attachment to use.
#[derive(Copy, Clone)]
pub enum DepthStencilAttachments<T> {
    /// No depth or stencil buffer.
    None,

    /// A depth attachment.
    DepthAttachment(T),

    /// A stencil attachment.
    StencilAttachment(T),

    /// A depth attachment and a stencil attachment.
    DepthAndStencilAttachments(T, T),

    /// A single attachment that serves as both depth and stencil buffer.
    DepthStencilAttachment(T),
}

/// Represents the attachments to use for an OpenGL framebuffer.
#[derive(Clone)]
pub struct FramebufferSpecificAttachments<T> {
    /// List of color attachments. The first parameter of the tuple is the index, and the
    /// second element is the attachment.
    pub colors: SmallVec<[(u32, T); 5]>,

    /// The depth and/or stencil attachment to use.
    pub depth_stencil: DepthStencilAttachments<T>,
}

impl<'a> FramebufferAttachments<'a> {
    /// After building a `FramebufferAttachments` struct, you must use this function
    /// to "compile" the attachments and make sure that they are valid together.
    #[inline]
    ///
    /// `ctxt` is the current context, through which the formats of the attachments are queried.
    pub fn validate(
        self,
        ctxt: &mut CommandContext<'_>,
    ) -> Result<ValidatedAttachments<'a>, ValidationError> {
        let context: &Context = ctxt.context;

        // OpenGL objects can only be used with the context that created them
        let owned = match self {
            FramebufferAttachments::Regular(ref a) => a.all(|a| a.belongs_to(context)),
            FramebufferAttachments::Layered(ref a)
            | FramebufferAttachments::Multiview { attachments: ref a, .. } => {
                a.all(|a| a.belongs_to(context))
            }
            FramebufferAttachments::Empty { .. } => true,
        };
        if !owned {
            return Err(ValidationError::ContextMismatch);
        }

        // the textures, whose shader writes must be visible to the framebuffer
        let textures = match self {
            FramebufferAttachments::Regular(ref a) => a.collect(|a| match *a {
                RegularAttachment::Texture(image) => Some(image.get_texture()),
                RegularAttachment::RenderBuffer(_) => None,
            }),
            FramebufferAttachments::Layered(ref a)
            | FramebufferAttachments::Multiview { attachments: ref a, .. } => {
                a.collect(|a| Some(a.0.get_texture()))
            }
            FramebufferAttachments::Empty { .. } => SmallVec::new(),
        };
        let mut validated = self.validate_attachments(ctxt)?;
        validated.textures = textures.into_vec();
        Ok(validated)
    }

    /// Validates the attachments, after checking that they belong to the current context.
    fn validate_attachments(
        self,
        ctxt: &mut CommandContext<'_>,
    ) -> Result<ValidatedAttachments<'a>, ValidationError> {
        let context: &Context = ctxt.context;

        match self {
            FramebufferAttachments::Regular(a) => {
                FramebufferAttachments::validate_regular(ctxt, a)
            }
            FramebufferAttachments::Layered(a) => {
                FramebufferAttachments::validate_layered(ctxt, a, None)
            }
            FramebufferAttachments::Multiview {
                attachments,
                base_view,
                views,
            } => {
                let views = Some((base_view, views));
                FramebufferAttachments::validate_layered(ctxt, attachments, views)
            }

            FramebufferAttachments::Empty {
                width,
                height,
                layers,
                samples,
                fixed_samples,
            } => {
                if context.get_opengl_version() >= &Version(Api::Gl, 4, 3)
                    || context.get_opengl_version() >= &Version(Api::GlEs, 3, 1)
                    || context.get_extensions().gl_arb_framebuffer_no_attachments
                {
                    if width == 0
                        || height == 0
                        || layers == Some(0)
                        || samples == Some(0)
                        || width
                        > context
                            .get_capabilities()
                            .max_framebuffer_width
                            .unwrap_or(0) as u32
                        || height
                            > context
                                .get_capabilities()
                                .max_framebuffer_height
                                .unwrap_or(0) as u32
                        || samples.unwrap_or(0)
                            > context
                                .get_capabilities()
                                .max_framebuffer_samples
                                .unwrap_or(0) as u32
                        || layers.unwrap_or(0)
                            > context
                                .get_capabilities()
                                .max_framebuffer_layers
                                .unwrap_or(0) as u32
                    {
                        return Err(ValidationError::EmptyFramebufferUnsupportedDimensions);
                    }

                    Ok(ValidatedAttachments {
                        raw: RawAttachments {
                            color: Vec::new(),
                            depth: None,
                            stencil: None,
                            depth_stencil: None,
                            default_width: Some(width),
                            default_height: Some(height),
                            default_layers: if context.get_opengl_version() <= &Version(Api::GlEs, 3, 1) {
                                None
                            } else {
                                Some(layers.unwrap_or(0))
                            },
                            default_samples: Some(samples.unwrap_or(0)),
                            default_samples_fixed: Some(fixed_samples),
                        },
                        color_kinds: SmallVec::new(),
                        dimensions: (width, height),
                        layers,
                        views: None,
                        depth_buffer_bits: None,
                        stencil_buffer_bits: None,
                        float32_colors: false,
                        textures: Vec::new(),
                        marker: PhantomData,
                    })
                } else {
                    Err(ValidationError::EmptyFramebufferObjectsNotSupported)
                }
            }
        }
    }

    fn validate_layered(
        ctxt: &mut CommandContext<'_>,
        FramebufferSpecificAttachments {
            colors,
            depth_stencil,
        }: FramebufferSpecificAttachments<LayeredAttachment<'a>>,
        views: Option<(u32, u32)>,
    ) -> Result<ValidatedAttachments<'a>, ValidationError> {
        let context: &Context = ctxt.context;

        if let Some((base_view, count)) = views {
            if !is_multiview_supported(ctxt) {
                return Err(ValidationError::MultiviewNotSupported);
            }
            // the views are layers of 2D array textures
            let mut attachments = colors.iter().map(|(_, attachment)| attachment);
            let mut depth_stencil_attachments = SmallVec::<[&LayeredAttachment<'a>; 2]>::new();
            match depth_stencil {
                DepthStencilAttachments::None => (),
                DepthStencilAttachments::DepthAttachment(ref a)
                | DepthStencilAttachments::StencilAttachment(ref a)
                | DepthStencilAttachments::DepthStencilAttachment(ref a) => {
                    depth_stencil_attachments.push(a)
                }
                DepthStencilAttachments::DepthAndStencilAttachments(ref a, ref b) => {
                    depth_stencil_attachments.extend([a, b])
                }
            }
            if !attachments
                .by_ref()
                .chain(depth_stencil_attachments.iter().copied())
                .all(|a| a.0.get_texture().get_bind_point() == gl::TEXTURE_2D_ARRAY)
            {
                return Err(ValidationError::MultiviewTextureNotSupported);
            }
            let max_views = unsafe { ctxt.glow.get_parameter_i32(gl::MAX_VIEWS_OVR) };
            let fits = colors
                .iter()
                .map(|(_, attachment)| attachment)
                .chain(depth_stencil_attachments.iter().copied())
                .all(|a| base_view.checked_add(count).is_some_and(|end| end <= a.layers()));
            // a single view isn't drawn the same way everywhere: Firefox refuses the programs
            // that declare one view, and Mesa the ones that declare none
            if count < 2 || count > max_views.max(0) as u32 || !fits {
                return Err(ValidationError::ViewsOutOfRange);
            }
        } else if !is_layered_supported(ctxt) {
            return Err(ValidationError::LayeredFramebuffersNotSupported);
        }

        // the color attachments must all be textures of the same kind
        let mut targets = colors
            .iter()
            .map(|(_, attachment)| attachment.0.get_texture().get_bind_point());
        if let Some(first) = targets.next() {
            if targets.any(|target| target != first) {
                return Err(ValidationError::LayerTargetsMismatch);
            }
        }

        // the framebuffer has the layers of the attachment that has the fewest
        let mut layers: Option<u32> = None;
        {
            let mut count = |attachment: &LayeredAttachment<'a>| {
                let count = attachment.layers();
                layers = Some(layers.map_or(count, |layers| layers.min(count)));
            };
            colors.iter().for_each(|(_, attachment)| count(attachment));
            match depth_stencil {
                DepthStencilAttachments::None => (),
                DepthStencilAttachments::DepthAttachment(ref a)
                | DepthStencilAttachments::StencilAttachment(ref a)
                | DepthStencilAttachments::DepthStencilAttachment(ref a) => count(a),
                DepthStencilAttachments::DepthAndStencilAttachments(ref a, ref b) => {
                    count(a);
                    count(b);
                }
            }
        }

        if let Some((_, count)) = views {
            layers = Some(count);
        }

        macro_rules! handle_tex {
            ($tex:ident, $dim:ident, $samples:ident, $num_bits:ident) => {{
                $num_bits = Some(
                    $tex.get_texture()
                        .get_internal_format_with(ctxt)
                        .map(|f| f.get_total_bits())
                        .ok()
                        .unwrap_or(24) as u16,
                ); // TODO: how to handle this?
                handle_tex!($tex, $dim, $samples)
            }};

            ($tex:ident, $dim:ident, $samples:ident) => {{
                check_renderable(context, $tex.get_texture())?;
                match &mut $samples {
                    &mut Some(samples) => {
                        if samples != $tex.get_samples().unwrap_or(0) {
                            return Err(ValidationError::SamplesCountMismatch);
                        }
                    }
                    s @ &mut None => {
                        *s = Some($tex.get_samples().unwrap_or(0));
                    }
                }

                match &mut $dim {
                    &mut Some((ref mut w, ref mut h)) => {
                        let height = $tex.get_height().unwrap_or(1);
                        // attachments with different dimensions are always supported
                        if *w != $tex.get_width() || *h != height {
                            *w = cmp::min(*w, $tex.get_width());
                            *h = cmp::min(*h, height);
                        }
                    }

                    dim @ &mut None => {
                        *dim = Some(($tex.get_width(), $tex.get_height().unwrap_or(1)));
                    }
                }

                RawAttachment::Texture {
                    texture: $tex.get_texture().get_id(),
                    bind_point: $tex.get_texture().get_bind_point(),
                    layer: None,
                    level: $tex.get_level(),
                    cubemap_layer: None,
                    views,
                    render_samples: None,
                }
            }};
        }

        let max_color_attachments = context.get_capabilities().max_color_attachments;
        if colors.len() > max_color_attachments as usize {
            return Err(ValidationError::TooManyColorAttachments {
                maximum: max_color_attachments as usize,
                obtained: colors.len(),
            });
        }

        let mut raw_attachments = RawAttachments {
            color: Vec::with_capacity(colors.len()),
            depth: None,
            stencil: None,
            depth_stencil: None,
            default_width: None,
            default_height: None,
            default_layers: None,
            default_samples: None,
            default_samples_fixed: None,
        };

        let mut dimensions = None;
        let mut depth_bits = None;
        let mut stencil_bits = None;
        let mut samples = None; // contains `0` if not multisampling and `None` if unknown

        let mut color_kinds = SmallVec::new();
        let mut float32_colors = false;
        for &(index, LayeredAttachment(ref attachment)) in colors.iter() {
            if index >= max_color_attachments as u32 {
                return Err(ValidationError::TooManyColorAttachments {
                    maximum: max_color_attachments as usize,
                    obtained: index as usize,
                });
            }
            color_kinds.push((index, attachment.get_texture().kind()));
            float32_colors |= is_float32(attachment.get_texture());
            raw_attachments
                .color
                .push((index, handle_tex!(attachment, dimensions, samples)));
        }

        match depth_stencil {
            DepthStencilAttachments::None => (),
            DepthStencilAttachments::DepthAttachment(LayeredAttachment(ref d)) => {
                raw_attachments.depth = Some(handle_tex!(d, dimensions, samples, depth_bits));
            }
            DepthStencilAttachments::StencilAttachment(LayeredAttachment(ref s)) => {
                raw_attachments.stencil = Some(handle_tex!(s, dimensions, samples, stencil_bits));
            }
            DepthStencilAttachments::DepthAndStencilAttachments(
                LayeredAttachment(ref d),
                LayeredAttachment(ref s),
            ) => {
                raw_attachments.depth = Some(handle_tex!(d, dimensions, samples, depth_bits));
                raw_attachments.stencil = Some(handle_tex!(s, dimensions, samples, stencil_bits));
            }
            DepthStencilAttachments::DepthStencilAttachment(LayeredAttachment(ref ds)) => {
                let depth_stencil_bits = ds.get_texture().get_depth_stencil_bits_with(ctxt);
                depth_bits = Some(depth_stencil_bits.0);
                stencil_bits = Some(depth_stencil_bits.1);
                raw_attachments.depth_stencil = Some(handle_tex!(ds, dimensions, samples));
            }
        }

        let dimensions = if let Some(dimensions) = dimensions {
            if dimensions.0 * dimensions.1 == 0 {
                return Err(ValidationError::EmptyFramebufferUnsupportedDimensions);
            }
            dimensions
        } else {
            // TODO: handle this
            return Err(ValidationError::EmptyFramebufferObjectsNotSupported);
        };

        Ok(ValidatedAttachments {
            raw: raw_attachments,
            color_kinds,
            dimensions,
            layers,
            views: views.map(|(_, count)| count),
            depth_buffer_bits: depth_bits,
            stencil_buffer_bits: stencil_bits,
            float32_colors,
            textures: Vec::new(),
            marker: PhantomData,
        })
    }

    fn validate_regular(
        ctxt: &mut CommandContext<'_>,
        FramebufferSpecificAttachments {
            colors,
            depth_stencil,
        }: FramebufferSpecificAttachments<RegularAttachment<'a>>,
    ) -> Result<ValidatedAttachments<'a>, ValidationError> {
        let context: &Context = ctxt.context;

        macro_rules! handle_tex {
            ($tex:ident, $dim:ident, $samples:ident, $num_bits:ident) => {{
                $num_bits = Some(
                    $tex.get_texture()
                        .get_internal_format_with(ctxt)
                        .map(|f| f.get_total_bits())
                        .ok()
                        .unwrap_or(24) as u16,
                ); // TODO: how to handle this?
                handle_tex!($tex, $dim, $samples)
            }};

            ($tex:ident, $dim:ident, $samples:ident) => {{
                check_renderable(context, $tex.get_texture())?;
                // the samples that the framebuffer renders to the image with
                let tex_samples = $tex.get_render_samples().or($tex.get_samples()).unwrap_or(0);
                match &mut $samples {
                    &mut Some(samples) => {
                        if samples != tex_samples {
                            return Err(ValidationError::SamplesCountMismatch);
                        }
                    }
                    s @ &mut None => {
                        *s = Some(tex_samples);
                    }
                }

                match &mut $dim {
                    &mut Some((ref mut w, ref mut h)) => {
                        let height = $tex.get_height().unwrap_or(1);
                        // attachments with different dimensions are always supported
                        if *w != $tex.get_width() || *h != height {
                            *w = cmp::min(*w, $tex.get_width());
                            *h = cmp::min(*h, height);
                        }
                    }

                    dim @ &mut None => {
                        *dim = Some(($tex.get_width(), $tex.get_height().unwrap_or(1)));
                    }
                }

                RawAttachment::Texture {
                    texture: $tex.get_texture().get_id(),
                    bind_point: $tex.get_texture().get_bind_point(),
                    layer: Some($tex.get_layer()),
                    level: $tex.get_level(),
                    cubemap_layer: $tex.get_cubemap_layer(),
                    views: None,
                    render_samples: $tex.get_render_samples(),
                }
            }};
        }

        macro_rules! handle_rb {
            ($rb:ident, $dim:ident, $samples:ident, $num_bits:ident) => {{
                $num_bits = Some(24); // FIXME: totally arbitrary
                handle_rb!($rb, $dim, $samples)
            }};

            ($rb:ident, $dim:ident, $samples:ident) => {{
                // render buffers are only created with formats that can be rendered to
                let dimensions = $rb.get_dimensions();

                match &mut $samples {
                    &mut Some(samples) => {
                        if samples != $rb.get_samples().unwrap_or(0) {
                            return Err(ValidationError::SamplesCountMismatch);
                        }
                    }
                    s @ &mut None => {
                        *s = Some($rb.get_samples().unwrap_or(0));
                    }
                }

                match &mut $dim {
                    &mut Some((ref mut w, ref mut h)) => {
                        // attachments with different dimensions are always supported
                        if *w != dimensions.0 || *h != dimensions.1 {
                            *w = cmp::min(*w, dimensions.0);
                            *h = cmp::min(*h, dimensions.1);
                        }
                    }

                    dim @ &mut None => {
                        *dim = Some((dimensions.0, dimensions.1));
                    }
                }

                RawAttachment::RenderBuffer($rb.get_id())
            }};
        }

        macro_rules! handle_atch {
            ($atch:ident, $($t:tt)*) => (
                match $atch {
                    &RegularAttachment::Texture(ref tex) => handle_tex!(tex, $($t)*),
                    &RegularAttachment::RenderBuffer(ref rb) => handle_rb!(rb, $($t)*),
                }
            );
        }

        let max_color_attachments = context.get_capabilities().max_color_attachments;
        if colors.len() > max_color_attachments as usize {
            return Err(ValidationError::TooManyColorAttachments {
                maximum: max_color_attachments as usize,
                obtained: colors.len(),
            });
        }

        let mut resolved = colors
            .iter()
            .map(|(index, attachment)| (*index == 0, attachment))
            .collect::<SmallVec<[_; 8]>>();
        match depth_stencil {
            DepthStencilAttachments::None => (),
            DepthStencilAttachments::DepthAttachment(ref a)
            | DepthStencilAttachments::StencilAttachment(ref a)
            | DepthStencilAttachments::DepthStencilAttachment(ref a) => resolved.push((false, a)),
            DepthStencilAttachments::DepthAndStencilAttachments(ref d, ref s) => {
                resolved.push((false, d));
                resolved.push((false, s));
            }
        }
        check_resolved_samples(context, &resolved)?;

        let mut raw_attachments = RawAttachments {
            color: Vec::with_capacity(colors.len()),
            depth: None,
            stencil: None,
            depth_stencil: None,
            default_width: None,
            default_height: None,
            default_layers: None,
            default_samples: None,
            default_samples_fixed: None,
        };

        let mut dimensions = None;
        let mut depth_bits = None;
        let mut stencil_bits = None;
        let mut samples = None; // contains `0` if not multisampling and `None` if unknown

        let mut color_kinds = SmallVec::new();
        let mut float32_colors = false;
        for &(index, ref attachment) in colors.iter() {
            if index >= max_color_attachments as u32 {
                return Err(ValidationError::TooManyColorAttachments {
                    maximum: max_color_attachments as usize,
                    obtained: index as usize,
                });
            }
            let (kind, float32) = match attachment {
                RegularAttachment::Texture(tex) => {
                    (tex.get_texture().kind(), is_float32(tex.get_texture()))
                }
                RegularAttachment::RenderBuffer(rb) => (rb.kind(), rb.get_format().is_float32()),
            };
            color_kinds.push((index, kind));
            float32_colors |= float32;
            raw_attachments
                .color
                .push((index, handle_atch!(attachment, dimensions, samples)));
        }

        match depth_stencil {
            DepthStencilAttachments::None => (),
            DepthStencilAttachments::DepthAttachment(ref d) => {
                raw_attachments.depth = Some(handle_atch!(d, dimensions, samples, depth_bits));
            }
            DepthStencilAttachments::StencilAttachment(ref s) => {
                raw_attachments.stencil = Some(handle_atch!(s, dimensions, samples, stencil_bits));
            }
            DepthStencilAttachments::DepthAndStencilAttachments(ref d, ref s) => {
                raw_attachments.depth = Some(handle_atch!(d, dimensions, samples, depth_bits));
                raw_attachments.stencil = Some(handle_atch!(s, dimensions, samples, stencil_bits));
            }
            DepthStencilAttachments::DepthStencilAttachment(ref ds) => {
                let depth_stencil_bits = match ds {
                    RegularAttachment::Texture(ref tex) => {
                        tex.get_texture().get_depth_stencil_bits_with(ctxt)
                    }
                    RegularAttachment::RenderBuffer(ref rb) => rb.get_depth_stencil_bits_with(ctxt),
                };
                depth_bits = Some(depth_stencil_bits.0);
                stencil_bits = Some(depth_stencil_bits.1);
                raw_attachments.depth_stencil = Some(handle_atch!(ds, dimensions, samples));
            }
        }

        let dimensions = if let Some(dimensions) = dimensions {
            if dimensions.0 * dimensions.1 == 0 {
                return Err(ValidationError::EmptyFramebufferUnsupportedDimensions);
            }
            dimensions
        } else {
            // TODO: handle this
            return Err(ValidationError::EmptyFramebufferObjectsNotSupported);
        };

        Ok(ValidatedAttachments {
            raw: raw_attachments,
            color_kinds,
            dimensions,
            layers: None,
            views: None,
            depth_buffer_bits: depth_bits,
            stencil_buffer_bits: stencil_bits,
            float32_colors,
            textures: Vec::new(),
            marker: PhantomData,
        })
    }
}

/// The framebuffer that an operation draws to, reads from or invalidates.
#[derive(Copy, Clone)]
pub enum FramebufferTarget<'a> {
    /// The default framebuffer (`None`), drawing into its back buffer, or a framebuffer object
    /// created outside SafeGL.
    Raw(Option<glow::Framebuffer>),
    /// The default framebuffer, drawing into and reading from the color buffer given, such as
    /// `GL_FRONT_LEFT` (desktop OpenGL).
    Default(gl::types::GLenum),
    /// A framebuffer object with these attachments, which SafeGL creates when needed.
    Attachments(&'a ValidatedAttachments<'a>),
}

/// Represents attachments that have been validated and are usable.
#[derive(Clone)]
pub struct ValidatedAttachments<'a> {
    raw: RawAttachments,
    // the location and kind of each color attachment
    color_kinds: SmallVec<[(u32, TextureKind); 4]>,
    dimensions: (u32, u32),
    layers: Option<u32>,
    // the number of views of a multiview framebuffer (`GL_OVR_multiview2`)
    views: Option<u32>,
    depth_buffer_bits: Option<u16>,
    stencil_buffer_bits: Option<u16>,
    // whether a color attachment has a format of 32-bit floats
    float32_colors: bool,
    // the attached textures
    textures: Vec<&'a TextureAny>,
    marker: PhantomData<&'a ()>,
}

impl<'a> ValidatedAttachments<'a> {
    /// Calls `glMemoryBarrier(GL_FRAMEBUFFER_BARRIER_BIT)` if a shader wrote an attached texture
    /// through an image unit since the last barrier, before the framebuffer reads or writes it.
    pub fn prepare_for_framebuffer_access(&self, ctxt: &mut CommandContext<'_>) {
        for texture in &self.textures {
            texture.prepare_for_access(ctxt, crate::TextureAccess::Framebuffer);
        }
    }

    /// Returns the locations of the color attachments.
    pub fn color_locations(&self) -> impl Iterator<Item = u32> + '_ {
        self.color_kinds.iter().map(|&(location, _)| location)
    }

    /// Returns true if a color attachment has a format of 32-bit floats, which OpenGL ES and
    /// WebGL only blend into with `GL_EXT_float_blend`.
    pub fn has_float32_colors(&self) -> bool {
        self.float32_colors
    }

    /// Returns true if a level of the `textures` given is attached, with the levels of each one,
    /// from the first to the second included.
    pub fn attaches_any(&self, textures: &[(glow::Texture, (u32, u32))]) -> bool {
        let raw = &self.raw;
        let attachments = raw.color.iter().map(|(_, attachment)| attachment);
        attachments
            .chain(&raw.depth)
            .chain(&raw.stencil)
            .chain(&raw.depth_stencil)
            .any(|attachment| match *attachment {
                RawAttachment::Texture { texture, level, .. } => {
                    textures.iter().any(|&(sampled, (first, last))| {
                        sampled == texture && (first..=last).contains(&level)
                    })
                }
                RawAttachment::RenderBuffer(_) => false,
            })
    }

    /// Returns the fragment output location and the kind of each color attachment.
    #[inline]
    pub fn get_color_kinds(&self) -> &[(u32, TextureKind)] {
        &self.color_kinds
    }

    /// Returns `true` if the framebuffer is layered.
    #[inline]
    pub fn is_layered(&self) -> bool {
        self.layers.is_some()
    }

    /// Returns the number of layers of a layered framebuffer.
    #[inline]
    pub fn get_layers(&self) -> Option<u32> {
        self.layers
    }

    /// Returns the number of views of a multiview framebuffer, `None` for the others.
    #[inline]
    pub fn get_views(&self) -> Option<u32> {
        self.views
    }

    /// Returns the dimensions that the framebuffer will have if you use these attachments.
    #[inline]
    pub fn get_dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    /// Returns the number of bits of precision of the depth buffer, or `None` if there is no
    /// depth buffer. Also works for depth-stencil buffers.
    #[inline]
    pub fn get_depth_buffer_bits(&self) -> Option<u16> {
        self.depth_buffer_bits
    }

    /// Returns the number of bits of precision of the stencil buffer, or `None` if there is no
    /// stencil buffer. Also works for depth-stencil buffers.
    #[inline]
    pub fn get_stencil_buffer_bits(&self) -> Option<u16> {
        self.stencil_buffer_bits
    }
}

/// An error that can happen while validating attachments.
///
/// New variants can be added without a major version, so matches need a wildcard arm.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationError {
    /// You requested an empty framebuffer object, but they are not supported.
    EmptyFramebufferObjectsNotSupported,

    /// The requested characteristics of an empty framebuffer object are out of range.
    EmptyFramebufferUnsupportedDimensions,

    /// The backend doesn't support attachments with various dimensions.
    ///
    /// Note that almost all OpenGL implementations support attachments with various dimensions.
    /// Only very old versions don't.
    DimensionsMismatchNotSupported,

    /// All attachments must have the same number of samples.
    SamplesCountMismatch,

    /// Backends only support a certain number of color attachments.
    TooManyColorAttachments {
        /// Maximum number of attachments.
        maximum: usize,
        /// Number of attachments that were given.
        obtained: usize,
    },

    /// OpenGL doesn't support this combination of attachments: `glCheckFramebufferStatus`
    /// didn't return `GL_FRAMEBUFFER_COMPLETE`.
    Incomplete {
        /// The value that `glCheckFramebufferStatus` returned, such as
        /// `GL_FRAMEBUFFER_UNSUPPORTED`.
        status: u32,
    },

    /// An attachment belongs to another context than the framebuffer. OpenGL objects can only be
    /// used with the context that created them.
    ContextMismatch,

    /// Whole textures with layers can't be attached: OpenGL ES 3.2 or `GL_EXT_geometry_shader`
    /// is needed.
    LayeredFramebuffersNotSupported,

    /// The layered color attachments aren't all textures of the same kind, such as 2D array
    /// textures, cube maps or 3D textures.
    LayerTargetsMismatch,

    /// An attached texture has a format that the backend can't render to: a compressed format,
    /// `F9F9F9`, or on OpenGL ES and WebGL a format that needs an extension, such as the float
    /// formats without `GL_EXT_color_buffer_float`.
    FormatNotRenderable,

    /// Multiview framebuffers need `GL_OVR_multiview2` (`OVR_multiview2` on WebGL).
    MultiviewNotSupported,

    /// The attachments of multiview framebuffers must be levels of 2D array textures.
    MultiviewTextureNotSupported,

    /// The views of a multiview framebuffer are more than the layers of an attachment, more than
    /// `GL_MAX_VIEWS_OVR`, or fewer than two: a single layer is attached on its own instead.
    ViewsOutOfRange,

    /// An image is rendered to with samples (`TextureAnyImage::multisampled`), which needs
    /// `GL_EXT_multisampled_render_to_texture`, and `GL_EXT_multisampled_render_to_texture2`
    /// elsewhere than at the first color attachment. WebGL doesn't have them.
    MultisampledRenderToTextureNotSupported,

    /// An image rendered to with samples isn't of a texture of two dimensions or of a face of a
    /// cube map, without samples of its own.
    MultisampledRenderToTextureTarget,

    /// An image is rendered to with more samples than `Limits::max_samples`, or than the largest
    /// number of samples of its format.
    SamplesNotSupported,
}

/// Returns true if `texture` has a format of 32-bit floats.
fn is_float32(texture: &TextureAny) -> bool {
    match texture.get_requested_format() {
        TextureFormatRequest::Specific(format) => format.is_float32(),
        _ => false,
    }
}

/// Returns `FormatNotRenderable` if `texture` has a format that the backend can't render to,
/// which would make the framebuffer incomplete.
fn check_renderable(context: &Context, texture: &TextureAny) -> Result<(), ValidationError> {
    match texture.get_requested_format() {
        TextureFormatRequest::Specific(format) if !format.is_renderable(context) => {
            Err(ValidationError::FormatNotRenderable)
        }
        _ => Ok(()),
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::ValidationError::*;
        let desc = match self {
            EmptyFramebufferObjectsNotSupported => {
                "You requested an empty framebuffer object, but they are not supported"
            }
            EmptyFramebufferUnsupportedDimensions => {
                "The requested characteristics of an empty framebuffer object are out of range"
            }
            DimensionsMismatchNotSupported => {
                "The backend doesn't support attachments with various dimensions"
            }
            SamplesCountMismatch => "All attachments must have the same number of samples",
            TooManyColorAttachments { .. } => {
                "Backends only support a certain number of color attachments"
            }
            Incomplete { .. } => "OpenGL doesn't support this combination of attachments",
            ContextMismatch => "An attachment belongs to another context than the framebuffer",
            LayeredFramebuffersNotSupported => "Layered framebuffers are not supported",
            LayerTargetsMismatch => "The layered color attachments are textures of different kinds",
            FormatNotRenderable => "An attached texture has a format that can't be rendered to",
            MultiviewNotSupported => "Multiview framebuffers are not supported",
            MultiviewTextureNotSupported => {
                "The attachments of multiview framebuffers must be 2D array textures"
            }
            ViewsOutOfRange => "The views are out of the range of the attachments or the backend",
            MultisampledRenderToTextureNotSupported => {
                "The backend can't render to this attachment with samples that it resolves"
            }
            MultisampledRenderToTextureTarget => {
                "Images rendered to with samples are of 2D textures or of faces of cube maps"
            }
            SamplesNotSupported => "The number of samples isn't supported for this format",
        };
        match self {
            TooManyColorAttachments {
                ref maximum,
                ref obtained,
            } => write!(fmt, "{}: found {}, maximum: {}", desc, obtained, maximum),
            Incomplete { status } => write!(fmt, "{} (status {:#06x})", desc, status),
            _ => fmt.write_str(desc),
        }
    }
}

impl Error for ValidationError {}

/// Data structure stored in the hashmap.
///
/// These attachments are guaranteed to be valid.
#[derive(Hash, Clone, Eq, PartialEq)]
struct RawAttachments {
    // for each frag output the location, the attachment to use
    color: Vec<(u32, RawAttachment)>,
    depth: Option<RawAttachment>,
    stencil: Option<RawAttachment>,
    depth_stencil: Option<RawAttachment>,

    // values to set through `glFramebufferParameteri`, they are `None` if they should not be set
    default_width: Option<u32>,
    default_height: Option<u32>,
    default_layers: Option<u32>,
    default_samples: Option<u32>,
    default_samples_fixed: Option<bool>,
}

/// Single attachment of `RawAttachments`.
#[derive(Hash, Copy, Clone, Eq, PartialEq)]
enum RawAttachment {
    /// A texture.
    Texture {
        // a GLenum like `TEXTURE_2D`, `TEXTURE_3D`, etc.
        bind_point: gl::types::GLenum, // TODO: Dimensions instead
        // id of the texture
        texture: glow::Texture,
        // if `Some`, use a regular attachment ; if `None`, use a layered attachment
        // if `None`, the texture **must** be an array, cubemap, or texture 3d
        layer: Option<u32>,
        // mipmap level
        level: u32,
        // layer of the cubemap, if this is a cubemap
        cubemap_layer: Option<CubeLayer>,
        // the first view and the number of views of a multiview attachment of a 2D array
        // texture, whose `layer` is `None`
        views: Option<(u32, u32)>,
        // the samples that the framebuffer renders to the texture with, which the GPU resolves
        // (`GL_EXT_multisampled_render_to_texture`)
        render_samples: Option<u32>,
    },

    /// A renderbuffer with its ID.
    RenderBuffer(glow::Renderbuffer),
}

/// Data to pass to the `clear_buffer` function.
#[derive(Debug, Copy, Clone)]
pub enum ClearBufferData {
    /// Suitable for float attachments.
    Float([f32; 4]),
    /// Suitable for integral textures.
    Integral([i32; 4]),
    /// Suitable for unsigned textures.
    Unsigned([u32; 4]),
    /// Suitable for depth attachments.
    Depth(f32),
    /// Suitable for stencil attachments.
    Stencil(i32),
    /// Suitable for depth-stencil attachments.
    DepthStencil(f32, i32),
}

impl From<[f32; 4]> for ClearBufferData {
    #[inline]
    fn from(data: [f32; 4]) -> ClearBufferData {
        ClearBufferData::Float(data)
    }
}

impl From<[i32; 4]> for ClearBufferData {
    #[inline]
    fn from(data: [i32; 4]) -> ClearBufferData {
        ClearBufferData::Integral(data)
    }
}

impl From<[u32; 4]> for ClearBufferData {
    #[inline]
    fn from(data: [u32; 4]) -> ClearBufferData {
        ClearBufferData::Unsigned(data)
    }
}

/// An error of the clears of single attachments (`raw_clear_buffer`,
/// `MultiOutputFrameBuffer::clear_output`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClearBufferError {
    /// The value doesn't match the kind of the attachment: floats for float and normalized
    /// attachments, integers of the same signedness for integer ones, a depth, a stencil index,
    /// or both.
    KindMismatch,
    /// The attachment can't be put in a framebuffer.
    Framebuffer(ValidationError),
    /// The framebuffer has no output of this name.
    OutputMissing,
}

impl fmt::Display for ClearBufferError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClearBufferError::KindMismatch => {
                fmt.write_str("The value doesn't match the kind of the attachment")
            }
            ClearBufferError::Framebuffer(error) => write!(fmt, "{error}"),
            ClearBufferError::OutputMissing => {
                fmt.write_str("The framebuffer has no output of this name")
            }
        }
    }
}

impl Error for ClearBufferError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ClearBufferError::Framebuffer(error) => Some(error),
            _ => None,
        }
    }
}

impl From<f32> for ClearBufferData {
    /// A depth.
    #[inline]
    fn from(depth: f32) -> ClearBufferData {
        ClearBufferData::Depth(depth)
    }
}

/// Manages all the framebuffer objects.
///
/// `cleanup` **must** be called when destroying the container, otherwise `Drop` will panic.
pub struct FramebuffersContainer {
    framebuffers:
        RefCell<HashMap<RawAttachments, FrameBufferObject, BuildHasherDefault<FnvHasher>>>,
}

impl FramebuffersContainer {
    /// Initializes the container.
    #[inline]
    pub fn new() -> FramebuffersContainer {
        FramebuffersContainer {
            framebuffers: RefCell::new(HashMap::with_hasher(Default::default())),
        }
    }

    /// Destroys all framebuffer objects. This is used when using a new context for example.
    pub fn purge_all(ctxt: &mut CommandContext<'_>) {
        let mut other = HashMap::with_hasher(Default::default());
        mem::swap(
            &mut *ctxt.framebuffer_objects.framebuffers.borrow_mut(),
            &mut other,
        );

        for (_, obj) in other.into_iter() {
            obj.destroy(ctxt);
        }
    }

    /// Destroys all framebuffer objects that contain a precise texture.
    #[inline]
    pub fn purge_texture(ctxt: &mut CommandContext<'_>, texture: glow::Texture) {
        FramebuffersContainer::purge_if(
            ctxt,
            |a| matches!(a, &RawAttachment::Texture { texture: id, .. } if id == texture),
        );
    }

    /// Destroys all framebuffer objects that contain a precise renderbuffer.
    #[inline]
    pub fn purge_renderbuffer(ctxt: &mut CommandContext<'_>, renderbuffer: glow::Renderbuffer) {
        FramebuffersContainer::purge_if(ctxt, |a| a == &RawAttachment::RenderBuffer(renderbuffer));
    }

    /// Destroys all framebuffer objects that match a certain condition.
    fn purge_if<F>(ctxt: &mut CommandContext<'_>, condition: F)
    where
        F: Fn(&RawAttachment) -> bool,
    {
        let mut framebuffers = ctxt.framebuffer_objects.framebuffers.borrow_mut();

        let mut attachments = Vec::with_capacity(0);
        for (key, _) in framebuffers.iter() {
            if key.color.iter().any(|&(_, ref id)| condition(id)) {
                attachments.push(key.clone());
                continue;
            }

            if let Some(ref atch) = key.depth {
                if condition(atch) {
                    attachments.push(key.clone());
                    continue;
                }
            }

            if let Some(ref atch) = key.stencil {
                if condition(atch) {
                    attachments.push(key.clone());
                    continue;
                }
            }

            if let Some(ref atch) = key.depth_stencil {
                if condition(atch) {
                    attachments.push(key.clone());
                    continue;
                }
            }
        }

        for atch in attachments.into_iter() {
            framebuffers.remove(&atch).unwrap().destroy(ctxt);
        }
    }

    /// Destroys all framebuffer objects.
    ///
    /// This is very similar to `purge_all`, but optimized for when the container will soon
    /// be destroyed.
    pub fn cleanup(ctxt: &mut CommandContext<'_>) {
        let mut other = HashMap::with_hasher(Default::default());
        mem::swap(
            &mut *ctxt.framebuffer_objects.framebuffers.borrow_mut(),
            &mut other,
        );

        for (_, obj) in other.into_iter() {
            obj.destroy(ctxt);
        }
    }

    ///
    /// # Unsafety
    ///
    /// After calling this function, you **must** make sure to call `purge_texture`
    /// and/or `purge_renderbuffer` when one of the attachment is destroyed.
    #[inline]
    pub fn get_framebuffer_for_drawing(
        ctxt: &mut CommandContext<'_>,
        target: FramebufferTarget<'_>,
    ) -> Option<glow::Framebuffer> {
        match target {
            FramebufferTarget::Raw(None) => {
                select_default_draw_buffer(ctxt, gl::BACK);
                None
            }
            FramebufferTarget::Default(buffer) => {
                select_default_draw_buffer(ctxt, buffer);
                None
            }
            FramebufferTarget::Raw(framebuffer) => framebuffer,
            FramebufferTarget::Attachments(attachments) => {
                attachments.prepare_for_framebuffer_access(ctxt);
                FramebuffersContainer::get_framebuffer(ctxt, attachments)
            }
        }
    }

    /// Binds the default framebuffer to `GL_READ_FRAMEBUFFER` or `GL_FRAMEBUFFER` so that it
    /// becomes the target of `glReadPixels`, `glCopyTexImage2D`, etc.
    // TODO: use an enum for the read buffer instead
    #[inline]
    pub fn bind_default_framebuffer_for_reading(
        ctxt: &mut CommandContext<'_>,
        read_buffer: gl::types::GLenum,
    ) {
        // OpenGL ES and WebGL only accept `GL_BACK` and `GL_NONE` for the default framebuffer
        let read_buffer = if ctxt.version.0 != Api::Gl && read_buffer != gl::NONE {
            gl::BACK
        } else {
            read_buffer
        };

        unsafe { bind_framebuffer(ctxt, None, false, true) };
        unsafe { ctxt.glow.read_buffer(read_buffer) }; // TODO: cache
    }

    /// Binds a framebuffer to `GL_READ_FRAMEBUFFER` or `GL_FRAMEBUFFER` so that it becomes the
    /// target of `glReadPixels`, `glCopyTexImage2D`, etc.
    ///
    /// # Unsafety
    ///
    /// After calling this function, you **must** make sure to call `purge_texture`
    /// and/or `purge_renderbuffer` when one of the attachment is destroyed.
    pub unsafe fn bind_framebuffer_for_reading(
        ctxt: &mut CommandContext<'_>,
        attachment: &RegularAttachment<'_>,
    ) {
        // TODO: restore this optimisation
        /*for (attachments, fbo) in ctxt.framebuffer_objects.framebuffers.borrow_mut().iter() {
            for &(key, ref atc) in attachments.color.iter() {
                if atc == attachment {
                    return (fbo.get_id(), gl::COLOR_ATTACHMENT0 + key);
                }
            }
        }*/

        if let RegularAttachment::Texture(image) = attachment {
            image
                .get_texture()
                .prepare_for_access(ctxt, crate::TextureAccess::Framebuffer);
        }

        // depth and stencil images are read through their own attachment points
        let kind = match attachment {
            RegularAttachment::Texture(image) => image.get_texture().kind(),
            RegularAttachment::RenderBuffer(buffer) => buffer.kind(),
        };
        // the attachments are described directly instead of being validated, since validating
        // a depth or stencil attachment queries its format, which needs the context that the
        // caller already holds
        let (raw, dimensions) = match attachment {
            RegularAttachment::Texture(image) => (
                RawAttachment::Texture {
                    texture: image.get_texture().get_id(),
                    bind_point: image.get_texture().get_bind_point(),
                    layer: Some(image.get_layer()),
                    level: image.get_level(),
                    cubemap_layer: image.get_cubemap_layer(),
                    views: None,
                    render_samples: None,
                },
                (image.get_width(), image.get_height().unwrap_or(1)),
            ),
            RegularAttachment::RenderBuffer(buffer) => (
                RawAttachment::RenderBuffer(buffer.get_id()),
                buffer.get_dimensions(),
            ),
        };
        let mut raw_attachments = RawAttachments {
            color: Vec::new(),
            depth: None,
            stencil: None,
            depth_stencil: None,
            default_width: None,
            default_height: None,
            default_layers: None,
            default_samples: None,
            default_samples_fixed: None,
        };
        let mut color_kinds = SmallVec::new();
        let read_buffer = match kind {
            TextureKind::Depth => {
                raw_attachments.depth = Some(raw);
                glow::NONE
            }
            TextureKind::Stencil => {
                raw_attachments.stencil = Some(raw);
                glow::NONE
            }
            TextureKind::DepthStencil => {
                raw_attachments.depth_stencil = Some(raw);
                glow::NONE
            }
            TextureKind::Float | TextureKind::Integral | TextureKind::Unsigned => {
                raw_attachments.color.push((0, raw));
                color_kinds.push((0, kind));
                glow::COLOR_ATTACHMENT0
            }
        };

        let attachments = ValidatedAttachments {
            raw: raw_attachments,
            color_kinds,
            dimensions,
            layers: None,
            views: None,
            depth_buffer_bits: None,
            stencil_buffer_bits: None,
            float32_colors: false,
            textures: Vec::new(),
            marker: PhantomData,
        };

        let framebuffer = FramebuffersContainer::get_framebuffer_for_drawing(
            ctxt,
            FramebufferTarget::Attachments(&attachments),
        );
        bind_framebuffer(ctxt, framebuffer, false, true);
        ctxt.glow.read_buffer(read_buffer); // TODO: cache
    }

    /// Sets every texel of the attachment to `data` with `glClearBuffer`, through a framebuffer
    /// that contains only the attachment, ignoring the scissor box and the write masks.
    ///
    /// Returns an error if `data` doesn't match the kind of the attachment.
    ///
    /// # Unsafety
    ///
    /// After calling this function, you **must** make sure to call `purge_texture`
    /// and/or `purge_renderbuffer` when one of the attachment is destroyed.
    pub unsafe fn clear_buffer(
        ctxt: &mut CommandContext<'_>,
        attachment: &RegularAttachment<'_>,
        data: ClearBufferData,
    ) -> Result<(), ClearBufferError> {
        let kind = attachment.kind();
        let matches = matches!(
            (kind, data),
            (TextureKind::Float, ClearBufferData::Float(_))
                | (TextureKind::Integral, ClearBufferData::Integral(_))
                | (TextureKind::Unsigned, ClearBufferData::Unsigned(_))
                | (TextureKind::Depth, ClearBufferData::Depth(_))
                | (TextureKind::Stencil, ClearBufferData::Stencil(_))
                | (TextureKind::DepthStencil, ClearBufferData::DepthStencil(..))
        );
        if !matches {
            return Err(ClearBufferError::KindMismatch);
        }

        // the framebuffer has the attachment where its kind goes
        let mut colors = SmallVec::new();
        let depth_stencil = match kind {
            TextureKind::Depth => DepthStencilAttachments::DepthAttachment(*attachment),
            TextureKind::Stencil => DepthStencilAttachments::StencilAttachment(*attachment),
            TextureKind::DepthStencil => {
                DepthStencilAttachments::DepthStencilAttachment(*attachment)
            }
            _ => {
                colors.push((0, *attachment));
                DepthStencilAttachments::None
            }
        };
        let fb = FramebufferAttachments::Regular(FramebufferSpecificAttachments {
            colors,
            depth_stencil,
        })
        .validate(ctxt)
        .map_err(ClearBufferError::Framebuffer)?;
        let fb = FramebuffersContainer::get_framebuffer_for_drawing(
            ctxt,
            FramebufferTarget::Attachments(&fb),
        );
        bind_framebuffer(ctxt, fb, true, false);

        // `glClearBuffer` obeys the masks, the scissor box, the rasterizer discard, the
        // conditional rendering and the sRGB conversion, none of which the clear wants
        if ctxt.state.color_mask != (1, 1, 1, 1) || ctxt.state.indexed_color_mask {
            ctxt.state.color_mask = (1, 1, 1, 1);
            ctxt.state.indexed_color_mask = false;
            ctxt.glow.color_mask(true, true, true, true);
        }
        if !ctxt.state.depth_mask {
            ctxt.glow.depth_mask(true);
            ctxt.state.depth_mask = true;
        }
        if ctxt.state.stencil_mask_front != 0xffffffff || ctxt.state.stencil_mask_back != 0xffffffff
        {
            ctxt.glow.stencil_mask(0xffffffff);
            ctxt.state.stencil_mask_front = 0xffffffff;
            ctxt.state.stencil_mask_back = 0xffffffff;
        }
        if ctxt.state.enabled_scissor_test {
            ctxt.glow.disable(glow::SCISSOR_TEST);
            ctxt.state.enabled_scissor_test = false;
        }
        if ctxt.state.enabled_rasterizer_discard {
            ctxt.glow.disable(glow::RASTERIZER_DISCARD);
            ctxt.state.enabled_rasterizer_discard = false;
        }
        if ctxt.state.enabled_framebuffer_srgb {
            ctxt.glow.disable(glow::FRAMEBUFFER_SRGB);
            ctxt.state.enabled_framebuffer_srgb = false;
        }
        <crate::draw_parameters::AnySamplesPassedQuery as crate::QueryExt>::end_conditional_render(
            ctxt,
        );

        match data {
            ClearBufferData::Float(data) => ctxt.glow.clear_buffer_f32_slice(glow::COLOR, 0, &data),
            ClearBufferData::Integral(data) => {
                ctxt.glow.clear_buffer_i32_slice(glow::COLOR, 0, &data)
            }
            ClearBufferData::Unsigned(data) => {
                ctxt.glow.clear_buffer_u32_slice(glow::COLOR, 0, &data)
            }
            ClearBufferData::Depth(depth) => {
                ctxt.glow.clear_buffer_f32_slice(glow::DEPTH, 0, &[depth])
            }
            ClearBufferData::Stencil(stencil) => {
                ctxt.glow.clear_buffer_i32_slice(glow::STENCIL, 0, &[stencil])
            }
            ClearBufferData::DepthStencil(depth, stencil) => {
                ctxt.glow
                    .clear_buffer_depth_stencil(glow::DEPTH_STENCIL, 0, depth, stencil)
            }
        }
        Ok(())
    }

    /// Discards the content of some attachments of a framebuffer (`glInvalidateFramebuffer`), in
    /// the whole framebuffer or in `rect`. Does nothing if the backend doesn't support it, since
    /// this is only a hint.
    ///
    /// # Unsafety
    ///
    /// After calling this function, you **must** make sure to call `purge_texture`
    /// and/or `purge_renderbuffer` when one of the attachment is destroyed.
    #[cfg(any(safegl_gl43, safegl_gles30, safegl_webgl))]
    pub unsafe fn invalidate(
        ctxt: &mut CommandContext<'_>,
        target: FramebufferTarget<'_>,
        which: InvalidateAttachments,
        rect: Option<&crate::Rect>,
    ) {
        // core in OpenGL 4.3, OpenGL ES 3.0 and WebGL 2
        if !(ctxt.version >= &Version(Api::Gl, 4, 3)
            || ctxt.version >= &Version(Api::GlEs, 3, 0)
            || ctxt.extensions.gl_arb_invalidate_subdata)
        {
            return;
        }

        let mut list = SmallVec::<[gl::types::GLenum; 8]>::new();
        match target {
            FramebufferTarget::Attachments(attachments) => {
                let raw = &attachments.raw;
                if which.color {
                    // `FrameBufferObject::new` attaches the color of location n at
                    // `COLOR_ATTACHMENTn`
                    list.extend(
                        raw.color
                            .iter()
                            .map(|&(location, _)| gl::COLOR_ATTACHMENT0 + location),
                    );
                }
                if which.depth && raw.depth.is_some() {
                    list.push(gl::DEPTH_ATTACHMENT);
                }
                if which.stencil && raw.stencil.is_some() {
                    list.push(gl::STENCIL_ATTACHMENT);
                }
                if raw.depth_stencil.is_some() {
                    // either part of a depth-stencil attachment can be invalidated alone
                    match (which.depth, which.stencil) {
                        (true, true) => list.push(gl::DEPTH_STENCIL_ATTACHMENT),
                        (true, false) => list.push(gl::DEPTH_ATTACHMENT),
                        (false, true) => list.push(gl::STENCIL_ATTACHMENT),
                        (false, false) => (),
                    }
                }
            }
            // the attachments of a framebuffer object created outside SafeGL are unknown, but
            // invalidating an attachment point that is empty does nothing
            FramebufferTarget::Raw(Some(_)) => {
                if which.color {
                    list.push(gl::COLOR_ATTACHMENT0);
                }
                if which.depth {
                    list.push(gl::DEPTH_ATTACHMENT);
                }
                if which.stencil {
                    list.push(gl::STENCIL_ATTACHMENT);
                }
            }
            FramebufferTarget::Raw(None) | FramebufferTarget::Default(_) => {
                if which.color {
                    list.push(gl::COLOR);
                }
                if which.depth {
                    list.push(gl::DEPTH);
                }
                if which.stencil {
                    list.push(gl::STENCIL);
                }
            }
        }
        if list.is_empty() {
            return;
        }

        let framebuffer = FramebuffersContainer::get_framebuffer_for_drawing(ctxt, target);
        bind_framebuffer(ctxt, framebuffer, true, false);
        // `bind_framebuffer` binds `GL_FRAMEBUFFER` on OpenGL ES and WebGL
        let target = if ctxt.version.0 == Api::Gl {
            gl::DRAW_FRAMEBUFFER
        } else {
            gl::FRAMEBUFFER
        };

        match rect {
            Some(rect) => ctxt.glow.invalidate_sub_framebuffer(
                target,
                &list,
                rect.left as gl::types::GLint,
                rect.bottom as gl::types::GLint,
                rect.width as gl::types::GLsizei,
                rect.height as gl::types::GLsizei,
            ),
            None => ctxt.glow.invalidate_framebuffer(target, &list),
        }
    }

    ///
    /// # Unsafety
    ///
    /// After calling this function, you **must** make sure to call `purge_texture`
    /// and/or `purge_renderbuffer` when one of the attachment is destroyed.
    /// Creates the framebuffer object of `attachments` if needed, and returns an error if OpenGL
    /// considers it incomplete.
    ///
    /// # Unsafety
    ///
    /// After calling this function, you **must** make sure to call `purge_texture`
    /// and/or `purge_renderbuffer` when one of the attachment is destroyed.
    pub fn check_complete(
        ctxt: &mut CommandContext<'_>,
        attachments: &ValidatedAttachments<'_>,
    ) -> Result<(), ValidationError> {
        // a lost context has nothing to check: its commands do nothing
        if FramebuffersContainer::get_framebuffer(ctxt, attachments).is_none() {
            return Ok(());
        }
        let status = ctxt.framebuffer_objects.framebuffers.borrow()[&attachments.raw].status;
        if status == gl::FRAMEBUFFER_COMPLETE {
            Ok(())
        } else {
            Err(ValidationError::Incomplete { status })
        }
    }

    /// Returns the framebuffer object of `attachments`, created if needed, or `None` if the
    /// context is lost: the commands then go to the default framebuffer, where they do nothing.
    fn get_framebuffer(
        ctxt: &mut CommandContext<'_>,
        attachments: &ValidatedAttachments<'_>,
    ) -> Option<glow::Framebuffer> {
        // TODO: use entries API
        let mut framebuffers = ctxt.framebuffer_objects.framebuffers.borrow_mut();
        if let Some(value) = framebuffers.get(&attachments.raw) {
            return Some(value.id);
        }

        // the objects of a lost context are gone, whether the browser creates them or not
        if ctxt.objects_lost() {
            return None;
        }
        let new_fbo = FrameBufferObject::new(ctxt, &attachments.raw)?;
        let new_fbo_id = new_fbo.id;
        framebuffers.insert(attachments.raw.clone(), new_fbo);
        Some(new_fbo_id)
    }
}

impl Drop for FramebuffersContainer {
    #[inline]
    fn drop(&mut self) {
        if self.framebuffers.borrow().len() != 0 {
            panic!()
        }
    }
}

/// A framebuffer object.
struct FrameBufferObject {
    id: glow::Framebuffer,
    current_read_buffer: gl::types::GLenum,
    // what `glCheckFramebufferStatus` returned once the attachments were attached
    status: gl::types::GLenum,
}

impl FrameBufferObject {
    /// Builds a new FBO.
    ///
    /// # Panic
    ///
    /// Panics if anything wrong or not supported is detected with the raw attachments.
    ///
    fn new(
        mut ctxt: &mut CommandContext<'_>,
        attachments: &RawAttachments,
    ) -> Option<FrameBufferObject> {
        if attachments.color.len() > ctxt.capabilities.max_draw_buffers as usize {
            panic!(
                "Trying to attach {} color buffers, but the hardware only supports {}",
                attachments.color.len(),
                ctxt.capabilities.max_draw_buffers
            );
        }

        // building the FBO
        let id = unsafe {
            if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_direct_state_access
            {
                ctxt.glow.create_named_framebuffer().ok()?
            } else {
                let id = ctxt.glow.create_framebuffer().ok()?;
                bind_framebuffer(&mut ctxt, Some(id), true, false);
                id
            }
        };

        // framebuffer parameters
        // TODO: DSA
        if let Some(width) = attachments.default_width {
            unsafe { bind_framebuffer(&mut ctxt, Some(id), true, false) }; // TODO: remove once DSA is used
            if ctxt.version >= &Version(Api::Gl, 4, 3)
                || ctxt.version >= &Version(Api::GlEs, 3, 1)
                || ctxt.extensions.gl_arb_framebuffer_no_attachments
            {
                unsafe {
                    ctxt.extra.FramebufferParameteri(
                        gl::DRAW_FRAMEBUFFER,
                        gl::FRAMEBUFFER_DEFAULT_WIDTH,
                        width as gl::types::GLint,
                    );
                }
            } else {
                unreachable!();
            }
        }
        if let Some(height) = attachments.default_height {
            unsafe { bind_framebuffer(&mut ctxt, Some(id), true, false) }; // TODO: remove once DSA is used
            if ctxt.version >= &Version(Api::Gl, 4, 3)
                || ctxt.version >= &Version(Api::GlEs, 3, 1)
                || ctxt.extensions.gl_arb_framebuffer_no_attachments
            {
                unsafe {
                    ctxt.extra.FramebufferParameteri(
                        gl::DRAW_FRAMEBUFFER,
                        gl::FRAMEBUFFER_DEFAULT_HEIGHT,
                        height as gl::types::GLint,
                    );
                }
            } else {
                unreachable!();
            }
        }
        if let Some(layers) = attachments.default_layers {
            unsafe { bind_framebuffer(&mut ctxt, Some(id), true, false) }; // TODO: remove once DSA is used
            if ctxt.version >= &Version(Api::Gl, 4, 3)
                || ctxt.version >= &Version(Api::GlEs, 3, 2)
                || ctxt.extensions.gl_arb_framebuffer_no_attachments
            {
                unsafe {
                    ctxt.extra.FramebufferParameteri(
                        gl::DRAW_FRAMEBUFFER,
                        gl::FRAMEBUFFER_DEFAULT_LAYERS,
                        layers as gl::types::GLint,
                    );
                }
            } else {
                unreachable!();
            }
        }
        if let Some(samples) = attachments.default_samples {
            unsafe { bind_framebuffer(&mut ctxt, Some(id), true, false) }; // TODO: remove once DSA is used
            if ctxt.version >= &Version(Api::Gl, 4, 3)
                || ctxt.version >= &Version(Api::GlEs, 3, 1)
                || ctxt.extensions.gl_arb_framebuffer_no_attachments
            {
                unsafe {
                    ctxt.extra.FramebufferParameteri(
                        gl::DRAW_FRAMEBUFFER,
                        gl::FRAMEBUFFER_DEFAULT_SAMPLES,
                        samples as gl::types::GLint,
                    );
                }
            } else {
                unreachable!();
            }
        }
        if let Some(samples_fixed) = attachments.default_samples_fixed {
            unsafe { bind_framebuffer(&mut ctxt, Some(id), true, false) }; // TODO: remove once DSA is used
            if ctxt.version >= &Version(Api::Gl, 4, 3)
                || ctxt.version >= &Version(Api::GlEs, 3, 1)
                || ctxt.extensions.gl_arb_framebuffer_no_attachments
            {
                unsafe {
                    ctxt.extra.FramebufferParameteri(
                        gl::DRAW_FRAMEBUFFER,
                        gl::FRAMEBUFFER_DEFAULT_FIXED_SAMPLE_LOCATIONS,
                        if samples_fixed { 1 } else { 0 },
                    );
                }
            } else {
                unreachable!();
            }
        }

        // attaching the attachments, and building the list of enums to pass to `glDrawBuffers`:
        // the color written by the output at location `n` goes to `GL_COLOR_ATTACHMENTn`, since
        // OpenGL ES only accepts `GL_COLOR_ATTACHMENTn` or `GL_NONE` as the n-th draw buffer
        let mut raw_attachments = Vec::with_capacity(attachments.color.len());
        for &(location, atchmnt) in attachments.color.iter() {
            if location >= ctxt.capabilities.max_color_attachments as u32 {
                panic!("Trying to attach a color buffer to slot {}, but the hardware only supports {} bind points",
                    location, ctxt.capabilities.max_color_attachments);
            }
            unsafe { attach(&mut ctxt, gl::COLOR_ATTACHMENT0 + location, id, atchmnt) };

            while raw_attachments.len() <= location as usize {
                raw_attachments.push(gl::NONE);
            }
            raw_attachments[location as usize] = gl::COLOR_ATTACHMENT0 + location;
        }
        if let Some(depth) = attachments.depth {
            unsafe { attach(&mut ctxt, gl::DEPTH_ATTACHMENT, id, depth) };
        }
        if let Some(stencil) = attachments.stencil {
            unsafe { attach(&mut ctxt, gl::STENCIL_ATTACHMENT, id, stencil) };
        }
        if let Some(depth_stencil) = attachments.depth_stencil {
            unsafe { attach(&mut ctxt, gl::DEPTH_STENCIL_ATTACHMENT, id, depth_stencil) };
        }

        // calling `glDrawBuffers` if necessary
        if raw_attachments != &[gl::COLOR_ATTACHMENT0] {
            if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_direct_state_access
            {
                unsafe {
                    ctxt.glow
                        .named_framebuffer_draw_buffers(Some(id), &raw_attachments);
                }
            } else {
                unsafe {
                    bind_framebuffer(&mut ctxt, Some(id), true, false);
                    ctxt.glow.draw_buffers(&raw_attachments);
                }
            }
        }

        let status = unsafe {
            if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_direct_state_access
            {
                ctxt.glow
                    .check_named_framebuffer_status(Some(id), gl::DRAW_FRAMEBUFFER)
            } else {
                bind_framebuffer(ctxt, Some(id), true, false);
                // `bind_framebuffer` binds `GL_FRAMEBUFFER` on OpenGL ES and WebGL
                ctxt.glow
                    .check_framebuffer_status(if ctxt.version.0 == Api::Gl {
                        gl::DRAW_FRAMEBUFFER
                    } else {
                        gl::FRAMEBUFFER
                    })
            }
        };

        Some(FrameBufferObject {
            id,
            current_read_buffer: gl::BACK,
            status,
        })
    }

    /// Destroys the FBO. Must be called, or things will leak.
    fn destroy(self, ctxt: &mut CommandContext<'_>) {
        // the render pass of a tile-based GPU on the framebuffer ends with it
        if ctxt
            .state
            .framebuffer_pass
            .is_some_and(|pass| pass.framebuffer == Some(self.id))
        {
            unsafe { suspend_framebuffer_pass(ctxt) };
            ctxt.state.framebuffer_pass = None;
        }

        // unbinding framebuffer
        if ctxt.state.draw_framebuffer == Some(self.id) {
            ctxt.state.draw_framebuffer = None;
        }

        if ctxt.state.read_framebuffer == Some(self.id) {
            ctxt.state.read_framebuffer = None;
        }

        // deleting
        if !ctxt.objects_lost() {
            unsafe { ctxt.glow.delete_framebuffer(self.id) }
        }
    }
}

impl GlObject for FrameBufferObject {
    type Id = glow::Framebuffer;

    #[inline]
    fn get_id(&self) -> glow::Framebuffer {
        self.id
    }
}

/// Returns the positions of the samples inside the pixels of `framebuffer`, from 0 to 1, or
/// nothing if it isn't multisampled (`glGetMultisamplefv(GL_SAMPLE_POSITION)`).
#[cfg(any(safegl_desktop, safegl_gles31))]
pub fn sample_positions(
    ctxt: &mut CommandContext<'_>,
    framebuffer: FramebufferTarget<'_>,
) -> Result<Vec<[f32; 2]>, crate::framebuffer::SamplePositionsNotSupportedError> {
    // core in OpenGL 3.2 and OpenGL ES 3.1
    if !(ctxt.version.0 == Api::Gl || ctxt.version >= &Version(Api::GlEs, 3, 1)) {
        return Err(crate::framebuffer::SamplePositionsNotSupportedError);
    }

    let id = FramebuffersContainer::get_framebuffer_for_drawing(ctxt, framebuffer);
    unsafe { bind_framebuffer(ctxt, id, true, false) };
    if unsafe { ctxt.glow.get_parameter_i32(gl::SAMPLE_BUFFERS) } == 0 {
        return Ok(Vec::new());
    }
    let samples = unsafe { ctxt.glow.get_parameter_i32(gl::SAMPLES) }.max(0) as u32;
    Ok((0..samples)
        .map(|i| {
            let mut position = [0.0; 2];
            unsafe {
                ctxt.extra
                    .GetMultisamplefv(gl::SAMPLE_POSITION, i, position.as_mut_ptr())
            };
            position
        })
        .collect())
}

/// Returns the number of draw buffers of `framebuffer` that aren't `GL_NONE`. A framebuffer
/// object created outside SafeGL must be bound for drawing.
pub fn draw_buffers_count(
    ctxt: &mut CommandContext<'_>,
    framebuffer: FramebufferTarget<'_>,
) -> usize {
    match framebuffer {
        FramebufferTarget::Raw(None) | FramebufferTarget::Default(_) => 1,
        FramebufferTarget::Attachments(attachments) => attachments.get_color_kinds().len(),
        FramebufferTarget::Raw(Some(_)) => (0..ctxt.capabilities.max_draw_buffers.max(0) as u32)
            .filter(|i| unsafe {
                ctxt.glow.get_parameter_i32(gl::DRAW_BUFFER0 + i) as u32 != gl::NONE
            })
            .count(),
    }
}

/// Makes `buffer` the color buffer that draws into the default framebuffer write
/// (`glDrawBuffer`), binding the default framebuffer for drawing. OpenGL ES only draws into the
/// back buffer.
fn select_default_draw_buffer(ctxt: &mut CommandContext<'_>, buffer: gl::types::GLenum) {
    if ctxt.version.0 != Api::Gl || ctxt.state.default_draw_buffer == buffer {
        return;
    }
    unsafe {
        bind_framebuffer(ctxt, None, true, false);
        ctxt.glow.draw_buffer(buffer);
    }
    ctxt.state.default_draw_buffer = buffer;
}

/// Binds a framebuffer object, either for drawing, reading, or both.
///
/// # Safety
///
/// The id of the FBO must be valid.
///
pub unsafe fn bind_framebuffer(
    ctxt: &mut CommandContext<'_>,
    fbo_id: Option<glow::Framebuffer>,
    draw: bool,
    read: bool,
) {
    // OpenGL ES binds framebuffers for reading to `GL_FRAMEBUFFER`, for drawing too
    let draws = draw || (read && ctxt.version.0 != Api::Gl);
    if draws && ctxt.state.draw_framebuffer != fbo_id {
        suspend_framebuffer_pass(ctxt);
    }
    bind_framebuffer_without_pass(ctxt, fbo_id, draw, read);
    resume_framebuffer_pass(ctxt);
}

/// A render pass of a tile-based GPU that a surface keeps open on a framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramebufferPass {
    /// The identifier of the object that opened the pass.
    pub id: u64,
    /// The framebuffer of the pass, `None` for the default one.
    pub framebuffer: Option<glow::Framebuffer>,
    /// What the pass does.
    pub kind: FramebufferPassKind,
    /// Whether the pass is active in OpenGL, rather than suspended while another framebuffer is
    /// bound for drawing.
    pub active: bool,
}

/// What a render pass of a tile-based GPU does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramebufferPassKind {
    /// Pixel local storage is enabled (`GL_EXT_shader_pixel_local_storage`).
    PixelLocalStorage,
    /// The rendering is limited to a tile, `(x, y, width, height)` (`GL_QCOM_tiled_rendering`).
    Tiling((u32, u32, u32, u32)),
}

/// All the buffers that `glStartTilingQCOM` and `glEndTilingQCOM` can preserve.
pub const PRESERVE_ALL_TILE_BUFFERS: gl::types::GLbitfield = 0xFFFF_FFFF;

/// Ends the render pass of a tile-based GPU in OpenGL, which `resume_framebuffer_pass` starts
/// again when its framebuffer is bound for drawing: pixel local storage is disabled, and its
/// content is lost, and the tiled rendering ends, preserving everything.
pub unsafe fn suspend_framebuffer_pass(ctxt: &mut CommandContext<'_>) {
    let Some(pass) = ctxt.state.framebuffer_pass.filter(|pass| pass.active) else {
        return;
    };
    #[cfg(not(safegl_browser))]
    match pass.kind {
        FramebufferPassKind::PixelLocalStorage => {
            ctxt.glow.disable(gl::SHADER_PIXEL_LOCAL_STORAGE_EXT)
        }
        FramebufferPassKind::Tiling(_) => ctxt.extra.EndTilingQCOM(PRESERVE_ALL_TILE_BUFFERS),
    }
    ctxt.state.framebuffer_pass = Some(FramebufferPass {
        active: false,
        ..pass
    });
}

/// Starts again the render pass of a tile-based GPU that `suspend_framebuffer_pass` ended, if
/// its framebuffer is bound for drawing.
unsafe fn resume_framebuffer_pass(ctxt: &mut CommandContext<'_>) {
    let Some(pass) = ctxt.state.framebuffer_pass else {
        return;
    };
    if pass.active || ctxt.state.draw_framebuffer != pass.framebuffer {
        return;
    }
    #[cfg(not(safegl_browser))]
    match pass.kind {
        FramebufferPassKind::PixelLocalStorage => {
            ctxt.glow.enable(gl::SHADER_PIXEL_LOCAL_STORAGE_EXT)
        }
        FramebufferPassKind::Tiling((x, y, width, height)) => {
            ctxt.extra
                .StartTilingQCOM(x, y, width, height, PRESERVE_ALL_TILE_BUFFERS)
        }
    }
    ctxt.state.framebuffer_pass = Some(FramebufferPass {
        active: true,
        ..pass
    });
}

/// Binds a framebuffer without suspending nor resuming the render pass of a tile-based GPU.
unsafe fn bind_framebuffer_without_pass(
    ctxt: &mut CommandContext<'_>,
    fbo_id: Option<glow::Framebuffer>,
    draw: bool,
    read: bool,
) {
    if draw && read {
        if ctxt.state.draw_framebuffer != fbo_id || ctxt.state.read_framebuffer != fbo_id {
            ctxt.glow.bind_framebuffer(glow::FRAMEBUFFER, fbo_id);
            ctxt.state.draw_framebuffer = fbo_id;
            ctxt.state.read_framebuffer = fbo_id;
        }
    } else {
        if draw && ctxt.state.draw_framebuffer != fbo_id {
            if ctxt.version.0 == Api::Gl {
                ctxt.glow.bind_framebuffer(glow::DRAW_FRAMEBUFFER, fbo_id);
                ctxt.state.draw_framebuffer = fbo_id;
            } else {
                ctxt.glow.bind_framebuffer(glow::FRAMEBUFFER, fbo_id);
                ctxt.state.draw_framebuffer = fbo_id;
                ctxt.state.read_framebuffer = fbo_id;
            }
        }

        if read && ctxt.state.read_framebuffer != fbo_id {
            if ctxt.version.0 == Api::Gl {
                ctxt.glow.bind_framebuffer(glow::READ_FRAMEBUFFER, fbo_id);
                ctxt.state.read_framebuffer = fbo_id;
            } else {
                ctxt.glow.bind_framebuffer(glow::FRAMEBUFFER, fbo_id);
                ctxt.state.draw_framebuffer = fbo_id;
                ctxt.state.read_framebuffer = fbo_id;
            }
        }
    }
}

/// Attaches something to a framebuffer object.
///
/// # Panic
///
/// - Panics if `layer` is `None` and layered attachments are not supported.
/// - Panics if `layer` is `None` and the texture is not an array or a 3D texture.
/// - Panics if the texture is an array and attaching an array is not supported.
///
/// # Safety
///
/// All parameters must be valid.
///
/// Attaches a whole texture level with its layers on OpenGL ES, where `glFramebufferTexture` is
/// core in OpenGL ES 3.2 and `glFramebufferTextureEXT` comes with `GL_EXT_geometry_shader`.
/// `validate_layered` checked that one of them exists.
unsafe fn attach_layered_es(
    ctxt: &mut CommandContext<'_>,
    slot: gl::types::GLenum,
    id: glow::Framebuffer,
    texture: glow::Texture,
    level: u32,
) {
    use crate::handles::RawName;

    bind_framebuffer(ctxt, Some(id), true, true);
    if ctxt.version >= &Version(Api::GlEs, 3, 2) {
        ctxt.glow
            .framebuffer_texture(glow::FRAMEBUFFER, slot, Some(texture), level as gl::types::GLint);
    } else {
        ctxt.extra.FramebufferTextureEXT(
            gl::FRAMEBUFFER,
            slot,
            texture.raw_name(),
            level as gl::types::GLint,
        );
    }
}

/// Attaches the views `base_view..base_view + views` of a level of a 2D array texture
/// (`GL_OVR_multiview2`), which `validate_layered` checked.
unsafe fn attach_multiview(
    ctxt: &mut CommandContext<'_>,
    slot: gl::types::GLenum,
    id: glow::Framebuffer,
    texture: glow::Texture,
    level: u32,
    base_view: u32,
    views: u32,
) {
    bind_framebuffer(ctxt, Some(id), true, false);
    if cfg!(safegl_browser) {
        // WebGL takes the texture object, which is the one bound
        #[cfg(safegl_browser)]
        {
            let unit = ctxt.state.active_texture as usize;
            if ctxt.state.texture_units[unit].texture != Some(texture) {
                ctxt.glow.bind_texture(gl::TEXTURE_2D_ARRAY, Some(texture));
                ctxt.state.texture_units[unit].texture = Some(texture);
            }
            ctxt.webgl.framebuffer_texture_multiview(slot, level, base_view, views);
        }
    } else {
        use crate::handles::RawName;

        ctxt.extra.FramebufferTextureMultiviewOVR(
            gl::DRAW_FRAMEBUFFER,
            slot,
            texture.raw_name(),
            level as gl::types::GLint,
            base_view as gl::types::GLint,
            views as gl::types::GLsizei,
        );
    }
}

unsafe fn attach(
    ctxt: &mut CommandContext<'_>,
    slot: gl::types::GLenum,
    id: glow::Framebuffer,
    attachment: RawAttachment,
) {
    match attachment {
        RawAttachment::Texture {
            texture: tex_id,
            level,
            views: Some((base_view, views)),
            ..
        } => attach_multiview(ctxt, slot, id, tex_id, level, base_view, views),

        RawAttachment::Texture {
            texture: tex_id,
            level,
            bind_point,
            cubemap_layer,
            render_samples: Some(samples),
            ..
        } => {
            // a level of a 2D texture or of a face of a cube map
            let target = match cubemap_layer {
                Some(face) => {
                    gl::TEXTURE_CUBE_MAP_POSITIVE_X + face.get_layer_index() as gl::types::GLenum
                }
                None => bind_point,
            };
            bind_framebuffer(ctxt, Some(id), true, true);
            #[cfg(not(safegl_browser))]
            {
                use crate::handles::RawName;

                ctxt.extra.FramebufferTexture2DMultisampleEXT(
                    gl::FRAMEBUFFER,
                    slot,
                    target,
                    tex_id.raw_name(),
                    level as gl::types::GLint,
                    samples as gl::types::GLsizei,
                );
            }
            #[cfg(safegl_browser)]
            let _ = (slot, target, level, samples);
        }

        RawAttachment::Texture {
            texture: tex_id,
            level,
            layer,
            bind_point,
            cubemap_layer,
            views: None,
            render_samples: None,
        } => {
            match bind_point {
                // these textures can't be layered
                gl::TEXTURE_2D
                | gl::TEXTURE_2D_MULTISAMPLE
                | gl::TEXTURE_1D
                | gl::TEXTURE_RECTANGLE => {
                    assert_eq!(layer, Some(0));
                    debug_assert!(cubemap_layer.is_none());

                    if ctxt.version >= &Version(Api::Gl, 4, 5)
                        || ctxt.extensions.gl_arb_direct_state_access
                    {
                        ctxt.glow.named_framebuffer_texture(
                            Some(id),
                            slot,
                            Some(tex_id),
                            level as gl::types::GLint,
                        );
                    } else if ctxt.version.0 == Api::Gl {
                        bind_framebuffer(ctxt, Some(id), true, false);
                        ctxt.glow.framebuffer_texture(
                            glow::DRAW_FRAMEBUFFER,
                            slot,
                            Some(tex_id),
                            level as gl::types::GLint,
                        );
                    } else {
                        bind_framebuffer(ctxt, Some(id), true, true);
                        // OpenGL ES 3.1+ also accepts multisample textures here
                        assert!(
                            bind_point == gl::TEXTURE_2D
                                || bind_point == gl::TEXTURE_2D_MULTISAMPLE
                        );
                        ctxt.glow.framebuffer_texture_2d(
                            glow::FRAMEBUFFER,
                            slot,
                            bind_point,
                            Some(tex_id),
                            level as gl::types::GLint,
                        );
                    }
                }

                // non-layered attachments
                gl::TEXTURE_1D_ARRAY
                | gl::TEXTURE_2D_ARRAY
                | gl::TEXTURE_2D_MULTISAMPLE_ARRAY
                | gl::TEXTURE_3D
                | gl::TEXTURE_CUBE_MAP_ARRAY
                    if layer.is_some() =>
                {
                    let layer = if bind_point == gl::TEXTURE_CUBE_MAP_ARRAY {
                        layer.unwrap() * 6
                            + cubemap_layer.unwrap().get_layer_index() as gl::types::GLenum
                    } else {
                        layer.unwrap()
                    };

                    if ctxt.version >= &Version(Api::Gl, 4, 5)
                        || ctxt.extensions.gl_arb_direct_state_access
                    {
                        ctxt.glow.named_framebuffer_texture_layer(
                            Some(id),
                            slot,
                            Some(tex_id),
                            level as gl::types::GLint,
                            layer as gl::types::GLint,
                        );
                    } else {
                        bind_framebuffer(ctxt, Some(id), true, false);

                        match bind_point {
                            // the layer of a cube map array is `6 × cube map + face`
                            gl::TEXTURE_1D_ARRAY
                            | gl::TEXTURE_2D_ARRAY
                            | gl::TEXTURE_2D_MULTISAMPLE_ARRAY
                            | gl::TEXTURE_CUBE_MAP_ARRAY => {
                                ctxt.glow.framebuffer_texture_layer(
                                    glow::DRAW_FRAMEBUFFER,
                                    slot,
                                    Some(tex_id),
                                    level as gl::types::GLint,
                                    layer as gl::types::GLint,
                                );
                            }

                            gl::TEXTURE_3D if ctxt.version.0 == Api::Gl => {
                                ctxt.glow.framebuffer_texture_3d(
                                    glow::DRAW_FRAMEBUFFER,
                                    slot,
                                    bind_point,
                                    Some(tex_id),
                                    level as gl::types::GLint,
                                    layer as gl::types::GLint,
                                );
                            }

                            // OpenGL ES attaches the slices of 3D textures like layers
                            gl::TEXTURE_3D => {
                                ctxt.glow.framebuffer_texture_layer(
                                    glow::DRAW_FRAMEBUFFER,
                                    slot,
                                    Some(tex_id),
                                    level as gl::types::GLint,
                                    layer as gl::types::GLint,
                                );
                            }

                            _ => panic!("Attaching a texture array is not supported"),
                        }
                    }
                }

                // layered attachments
                gl::TEXTURE_1D_ARRAY
                | gl::TEXTURE_2D_ARRAY
                | gl::TEXTURE_2D_MULTISAMPLE_ARRAY
                | gl::TEXTURE_3D
                | gl::TEXTURE_CUBE_MAP_ARRAY
                    if layer.is_none() =>
                {
                    if ctxt.version >= &Version(Api::Gl, 4, 5)
                        || ctxt.extensions.gl_arb_direct_state_access
                    {
                        ctxt.glow.named_framebuffer_texture(
                            Some(id),
                            slot,
                            Some(tex_id),
                            level as gl::types::GLint,
                        );
                    } else if ctxt.version.0 == Api::Gl {
                        bind_framebuffer(ctxt, Some(id), true, false);
                        ctxt.glow.framebuffer_texture(
                            glow::DRAW_FRAMEBUFFER,
                            slot,
                            Some(tex_id),
                            level as gl::types::GLint,
                        );
                    } else {
                        attach_layered_es(ctxt, slot, id, tex_id, level);
                    }
                }

                // non-layered cubemaps
                gl::TEXTURE_CUBE_MAP if layer.is_some() => {
                    let bind_point = gl::TEXTURE_CUBE_MAP_POSITIVE_X
                        + cubemap_layer.unwrap().get_layer_index() as gl::types::GLenum;

                    if ctxt.version.0 == Api::Gl {
                        bind_framebuffer(ctxt, Some(id), true, false);
                        ctxt.glow.framebuffer_texture_2d(
                            glow::DRAW_FRAMEBUFFER,
                            slot,
                            bind_point,
                            Some(tex_id),
                            level as gl::types::GLint,
                        );
                    } else {
                        bind_framebuffer(ctxt, Some(id), true, true);
                        ctxt.glow.framebuffer_texture_2d(
                            glow::FRAMEBUFFER,
                            slot,
                            bind_point,
                            Some(tex_id),
                            level as gl::types::GLint,
                        );
                    }
                }

                // layered cubemaps
                gl::TEXTURE_CUBE_MAP if layer.is_none() => {
                    if ctxt.version >= &Version(Api::Gl, 4, 5)
                        || ctxt.extensions.gl_arb_direct_state_access
                    {
                        ctxt.glow.named_framebuffer_texture(
                            Some(id),
                            slot,
                            Some(tex_id),
                            level as gl::types::GLint,
                        );
                    } else if ctxt.version.0 == Api::Gl {
                        bind_framebuffer(ctxt, Some(id), true, false);
                        ctxt.glow.framebuffer_texture(
                            glow::DRAW_FRAMEBUFFER,
                            slot,
                            Some(tex_id),
                            level as gl::types::GLint,
                        );
                    } else {
                        attach_layered_es(ctxt, slot, id, tex_id, level);
                    }
                }

                _ => unreachable!(),
            }
        }

        // renderbuffers are straight-forward
        RawAttachment::RenderBuffer(renderbuffer) => {
            if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_direct_state_access
            {
                ctxt.glow.named_framebuffer_renderbuffer(
                    Some(id),
                    slot,
                    glow::RENDERBUFFER,
                    Some(renderbuffer),
                );
            } else if ctxt.version.0 == Api::Gl {
                bind_framebuffer(ctxt, Some(id), true, false);
                ctxt.glow.framebuffer_renderbuffer(
                    glow::DRAW_FRAMEBUFFER,
                    slot,
                    glow::RENDERBUFFER,
                    Some(renderbuffer),
                );
            } else {
                bind_framebuffer(ctxt, Some(id), true, true);
                ctxt.glow.framebuffer_renderbuffer(
                    glow::DRAW_FRAMEBUFFER,
                    slot,
                    glow::RENDERBUFFER,
                    Some(renderbuffer),
                );
            }
        }
    }
}
