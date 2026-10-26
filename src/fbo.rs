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

use crate::framebuffer::{InvalidateAttachments, RenderBufferAny};
use crate::texture::CubeLayer;
use crate::texture::TextureAnyImage;
use crate::texture::TextureAnyMipmap;
use crate::texture::TextureKind;

use crate::context::CommandContext;
use crate::context::Context;
use crate::gl;
use crate::version::Api;
use crate::version::Version;

/// Returns true if the backend supports attachments with varying dimensions.
///
/// If this function returns `true` and you pass attachments with different dimensions, the
/// intersection between all the attachments will be used. If this function returns `false`, you'll
/// get an error instead.
pub fn is_dimensions_mismatch_supported<C: ?Sized>(_context: &C) -> bool
where
    C: CapabilitiesSource,
{
    // core since OpenGL 3.0 and OpenGL ES 2.0, which are below the minimum supported versions
    true
}

/// Represents the attachments to use for an OpenGL framebuffer.
#[derive(Clone)]
pub enum FramebufferAttachments<'a> {
    /// Each attachment is a single image.
    Regular(FramebufferSpecificAttachments<RegularAttachment<'a>>),

    /// Each attachment is a layer of images.
    Layered(FramebufferSpecificAttachments<LayeredAttachment<'a>>),

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

/// Describes a single layered framebuffer attachment.
#[derive(Copy, Clone)]
pub struct LayeredAttachment<'a>(TextureAnyMipmap<'a>);

impl<'a> LayeredAttachment<'a> {
    /// Returns true if the attachment was created by `context`.
    fn belongs_to(&self, context: &Context) -> bool {
        std::ptr::eq(&**TextureExt::get_context(self.0.get_texture()), context)
    }
}

impl<T> FramebufferSpecificAttachments<T> {
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
    pub fn validate(self, context: &Context) -> Result<ValidatedAttachments<'a>, ValidationError> {
        // OpenGL objects can only be used with the context that created them
        let owned = match self {
            FramebufferAttachments::Regular(ref a) => a.all(|a| a.belongs_to(context)),
            FramebufferAttachments::Layered(ref a) => a.all(|a| a.belongs_to(context)),
            FramebufferAttachments::Empty { .. } => true,
        };
        if !owned {
            return Err(ValidationError::ContextMismatch);
        }

        match self {
            FramebufferAttachments::Regular(a) => {
                FramebufferAttachments::validate_regular(context, a)
            }
            FramebufferAttachments::Layered(a) => {
                FramebufferAttachments::validate_layered(context, a)
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
                        depth_buffer_bits: None,
                        stencil_buffer_bits: None,
                        marker: PhantomData,
                    })
                } else {
                    Err(ValidationError::EmptyFramebufferObjectsNotSupported)
                }
            }
        }
    }

    fn validate_layered<C: ?Sized>(
        context: &C,
        FramebufferSpecificAttachments {
            colors,
            depth_stencil,
        }: FramebufferSpecificAttachments<LayeredAttachment<'a>>,
    ) -> Result<ValidatedAttachments<'a>, ValidationError>
    where
        C: CapabilitiesSource,
    {
        // TODO: make sure that all attachments are layered

        macro_rules! handle_tex {
            ($tex:ident, $dim:ident, $samples:ident, $num_bits:ident) => {{
                $num_bits = Some(
                    $tex.get_texture()
                        .get_internal_format()
                        .map(|f| f.get_total_bits())
                        .ok()
                        .unwrap_or(24) as u16,
                ); // TODO: how to handle this?
                handle_tex!($tex, $dim, $samples)
            }};

            ($tex:ident, $dim:ident, $samples:ident) => {{
                // TODO: check that internal format is renderable
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
        for &(index, LayeredAttachment(ref attachment)) in colors.iter() {
            if index >= max_color_attachments as u32 {
                return Err(ValidationError::TooManyColorAttachments {
                    maximum: max_color_attachments as usize,
                    obtained: index as usize,
                });
            }
            color_kinds.push((index, attachment.get_texture().kind()));
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
                let depth_stencil_bits = ds.get_texture().get_depth_stencil_bits();
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
            layers: None, // FIXME: count layers
            depth_buffer_bits: depth_bits,
            stencil_buffer_bits: stencil_bits,
            marker: PhantomData,
        })
    }

    fn validate_regular<C: ?Sized>(
        context: &C,
        FramebufferSpecificAttachments {
            colors,
            depth_stencil,
        }: FramebufferSpecificAttachments<RegularAttachment<'a>>,
    ) -> Result<ValidatedAttachments<'a>, ValidationError>
    where
        C: CapabilitiesSource,
    {
        macro_rules! handle_tex {
            ($tex:ident, $dim:ident, $samples:ident, $num_bits:ident) => {{
                $num_bits = Some(
                    $tex.get_texture()
                        .get_internal_format()
                        .map(|f| f.get_total_bits())
                        .ok()
                        .unwrap_or(24) as u16,
                ); // TODO: how to handle this?
                handle_tex!($tex, $dim, $samples)
            }};

            ($tex:ident, $dim:ident, $samples:ident) => {{
                // TODO: check that internal format is renderable
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
                    layer: Some($tex.get_layer()),
                    level: $tex.get_level(),
                    cubemap_layer: $tex.get_cubemap_layer(),
                }
            }};
        }

        macro_rules! handle_rb {
            ($rb:ident, $dim:ident, $samples:ident, $num_bits:ident) => {{
                $num_bits = Some(24); // FIXME: totally arbitrary
                handle_rb!($rb, $dim, $samples)
            }};

            ($rb:ident, $dim:ident, $samples:ident) => {{
                // TODO: check that internal format is renderable
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
        for &(index, ref attachment) in colors.iter() {
            if index >= max_color_attachments as u32 {
                return Err(ValidationError::TooManyColorAttachments {
                    maximum: max_color_attachments as usize,
                    obtained: index as usize,
                });
            }
            let kind = match attachment {
                RegularAttachment::Texture(tex) => tex.get_texture().kind(),
                RegularAttachment::RenderBuffer(rb) => rb.kind(),
            };
            color_kinds.push((index, kind));
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
                        tex.get_texture().get_depth_stencil_bits()
                    }
                    RegularAttachment::RenderBuffer(ref rb) => rb.get_depth_stencil_bits(),
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
            depth_buffer_bits: depth_bits,
            stencil_buffer_bits: stencil_bits,
            marker: PhantomData,
        })
    }
}

/// The framebuffer that an operation draws to, reads from or invalidates.
#[derive(Copy, Clone)]
pub enum FramebufferTarget<'a> {
    /// The default framebuffer (`None`), or a framebuffer object created outside SafeGL.
    Raw(Option<glow::Framebuffer>),
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
    depth_buffer_bits: Option<u16>,
    stencil_buffer_bits: Option<u16>,
    marker: PhantomData<&'a ()>,
}

impl<'a> ValidatedAttachments<'a> {
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
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
            FramebufferTarget::Raw(framebuffer) => framebuffer,
            FramebufferTarget::Attachments(attachments) => {
                Some(FramebuffersContainer::get_framebuffer(ctxt, attachments))
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
            depth_buffer_bits: None,
            stencil_buffer_bits: None,
            marker: PhantomData,
        };

        let framebuffer = FramebuffersContainer::get_framebuffer_for_drawing(
            ctxt,
            FramebufferTarget::Attachments(&attachments),
        );
        bind_framebuffer(ctxt, framebuffer, false, true);
        ctxt.glow.read_buffer(read_buffer); // TODO: cache
    }

    /// Calls `glClearBuffer` on a framebuffer that contains the attachment.
    ///
    /// # Panic
    ///
    /// Panics if `data` is incompatible with the kind of attachment.
    ///
    /// # Unsafety
    ///
    /// After calling this function, you **must** make sure to call `purge_texture`
    /// and/or `purge_renderbuffer` when one of the attachment is destroyed.
    pub unsafe fn clear_buffer<D>(
        ctxt: &mut CommandContext<'_>,
        attachment: &RegularAttachment<'_>,
        data: D,
    ) where
        D: Into<ClearBufferData>,
    {
        // TODO: look for an existing framebuffer with this attachment

        let data = data.into();

        let fb = FramebufferAttachments::Regular(FramebufferSpecificAttachments {
            colors: {
                let mut v = SmallVec::new();
                v.push((0, *attachment));
                v
            },
            depth_stencil: DepthStencilAttachments::None,
        })
        .validate(ctxt.context)
        .unwrap();
        let fb = FramebuffersContainer::get_framebuffer_for_drawing(
            ctxt,
            FramebufferTarget::Attachments(&fb),
        );

        // TODO: use DSA if supported
        // TODO: what if glClearBuffer is not supported?

        bind_framebuffer(ctxt, fb, true, false);

        // `glClearBuffer` obeys the color mask, including the per-draw-buffer masks of
        // `DrawParameters::outputs`
        if ctxt.state.color_mask != (1, 1, 1, 1) || ctxt.state.indexed_color_mask {
            ctxt.state.color_mask = (1, 1, 1, 1);
            ctxt.state.indexed_color_mask = false;
            ctxt.glow.color_mask(true, true, true, true);
        }

        match (attachment.kind(), data) {
            (TextureKind::Float, ClearBufferData::Float(data)) => {
                ctxt.glow.clear_buffer_f32_slice(glow::COLOR, 0, &data);
            }
            (TextureKind::Integral, ClearBufferData::Integral(data)) => {
                ctxt.glow.clear_buffer_i32_slice(glow::COLOR, 0, &data);
            }
            (TextureKind::Unsigned, ClearBufferData::Unsigned(data)) => {
                ctxt.glow.clear_buffer_u32_slice(glow::COLOR, 0, &data);
            }
            (TextureKind::Depth, _) => {
                unimplemented!() // TODO: can't work with the code above ^
            }
            (TextureKind::Stencil, _) => {
                unimplemented!() // TODO: can't work with the code above ^
            }
            (TextureKind::DepthStencil, _) => {
                unimplemented!() // TODO: can't work with the code above ^
            }
            _ => {
                panic!("The data passed to `clear_buffer` does not match the kind of attachment");
            }
        }
    }

    /// Discards the content of some attachments of a framebuffer (`glInvalidateFramebuffer`), in
    /// the whole framebuffer or in `rect`. Does nothing if the backend doesn't support it, since
    /// this is only a hint.
    ///
    /// # Unsafety
    ///
    /// After calling this function, you **must** make sure to call `purge_texture`
    /// and/or `purge_renderbuffer` when one of the attachment is destroyed.
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
                    // `FrameBufferObject::new` attaches the n-th color at `COLOR_ATTACHMENTn`
                    list.extend((0..raw.color.len() as u32).map(|i| gl::COLOR_ATTACHMENT0 + i));
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
            FramebufferTarget::Raw(None) => {
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
        FramebuffersContainer::get_framebuffer(ctxt, attachments);
        let status = ctxt.framebuffer_objects.framebuffers.borrow()[&attachments.raw].status;
        if status == gl::FRAMEBUFFER_COMPLETE {
            Ok(())
        } else {
            Err(ValidationError::Incomplete { status })
        }
    }

    fn get_framebuffer(
        ctxt: &mut CommandContext<'_>,
        attachments: &ValidatedAttachments<'_>,
    ) -> glow::Framebuffer {
        // TODO: use entries API
        let mut framebuffers = ctxt.framebuffer_objects.framebuffers.borrow_mut();
        if let Some(value) = framebuffers.get(&attachments.raw) {
            return value.id;
        }

        let new_fbo = FrameBufferObject::new(ctxt, &attachments.raw);
        let new_fbo_id = new_fbo.id;
        framebuffers.insert(attachments.raw.clone(), new_fbo);
        new_fbo_id
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
    fn new(mut ctxt: &mut CommandContext<'_>, attachments: &RawAttachments) -> FrameBufferObject {
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
                ctxt.glow
                    .create_named_framebuffer()
                    .expect("failed to create a framebuffer object")
            } else {
                let id = ctxt
                    .glow
                    .create_framebuffer()
                    .expect("failed to create a framebuffer object");
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

        // attaching the attachments, and building the list of enums to pass to `glDrawBuffers`
        let mut raw_attachments = Vec::with_capacity(attachments.color.len());
        for (attachment_pos, &(pos_in_drawbuffers, atchmnt)) in attachments.color.iter().enumerate()
        {
            if attachment_pos >= ctxt.capabilities.max_color_attachments as usize {
                panic!("Trying to attach a color buffer to slot {}, but the hardware only supports {} bind points",
                    attachment_pos, ctxt.capabilities.max_color_attachments);
            }
            unsafe {
                attach(
                    &mut ctxt,
                    gl::COLOR_ATTACHMENT0 + attachment_pos as u32,
                    id,
                    atchmnt,
                )
            };

            while raw_attachments.len() <= pos_in_drawbuffers as usize {
                raw_attachments.push(gl::NONE);
            }
            raw_attachments[pos_in_drawbuffers as usize] =
                gl::COLOR_ATTACHMENT0 + attachment_pos as u32;
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

        FrameBufferObject {
            id,
            current_read_buffer: gl::BACK,
            status,
        }
    }

    /// Destroys the FBO. Must be called, or things will leak.
    fn destroy(self, ctxt: &mut CommandContext<'_>) {
        // unbinding framebuffer
        if ctxt.state.draw_framebuffer == Some(self.id) {
            ctxt.state.draw_framebuffer = None;
        }

        if ctxt.state.read_framebuffer == Some(self.id) {
            ctxt.state.read_framebuffer = None;
        }

        // deleting
        unsafe { ctxt.glow.delete_framebuffer(self.id) };
    }
}

impl GlObject for FrameBufferObject {
    type Id = glow::Framebuffer;

    #[inline]
    fn get_id(&self) -> glow::Framebuffer {
        self.id
    }
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
            layer,
            bind_point,
            cubemap_layer,
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
                            gl::TEXTURE_1D_ARRAY
                            | gl::TEXTURE_2D_ARRAY
                            | gl::TEXTURE_2D_MULTISAMPLE_ARRAY => {
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
                        // note that this should have been detected earlier
                        panic!("Layered framebuffers are not supported");
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
                        // note that this should have been detected earlier
                        panic!("Layered framebuffers are not supported");
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
