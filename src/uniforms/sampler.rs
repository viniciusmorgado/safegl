use crate::gl;
use crate::ToGlEnum;

/// Function to use for out-of-bounds samples.
///
/// This is how GL must handle samples that are outside the texture.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SamplerWrapFunction {
    /// Samples at coord `x + 1` map to coord `x`.
    Repeat,

    /// Samples at coord `x + 1` map to coord `1 - x`.
    Mirror,

    /// Samples at coord `x + 1` map to coord `1`.
    Clamp,

    /// Use texture border.
    ///
    /// Requires OpenGL ES 3.2 or `GL_EXT_texture_border_clamp` on OpenGL ES, and isn't
    /// available on WebGL. Falls back to `Clamp` where unsupported.
    BorderClamp,

    /// Same as Mirror, but only for one repetition,
    ///
    /// Requires `GL_EXT_texture_mirror_clamp_to_edge` on OpenGL ES, and isn't available on
    /// WebGL. Falls back to `Clamp` where unsupported.
    MirrorClamp,
}

impl ToGlEnum for SamplerWrapFunction {
    #[inline]
    fn to_glenum(&self) -> gl::types::GLenum {
        match *self {
            SamplerWrapFunction::Repeat => gl::REPEAT,
            SamplerWrapFunction::Mirror => gl::MIRRORED_REPEAT,
            SamplerWrapFunction::Clamp => gl::CLAMP_TO_EDGE,
            SamplerWrapFunction::BorderClamp => gl::CLAMP_TO_BORDER,
            SamplerWrapFunction::MirrorClamp => gl::MIRROR_CLAMP_TO_EDGE,
        }
    }
}

/// The function that the GPU will use when loading the value of a texel.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum MagnifySamplerFilter {
    /// The nearest texel will be loaded.
    Nearest,

    /// All nearby texels will be loaded and their values will be merged.
    Linear,
}

impl ToGlEnum for MagnifySamplerFilter {
    #[inline]
    fn to_glenum(&self) -> gl::types::GLenum {
        match *self {
            MagnifySamplerFilter::Nearest => gl::NEAREST,
            MagnifySamplerFilter::Linear => gl::LINEAR,
        }
    }
}

/// The function that the GPU will use when loading the value of a texel.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum MinifySamplerFilter {
    /// The nearest texel will be loaded.
    ///
    /// Only uses the main texture, mipmaps are totally ignored.
    Nearest,

    /// All nearby texels will be loaded and their values will be merged.
    ///
    /// Only uses the main texture, mipmaps are totally ignored.
    Linear,

    /// The nearest texel of the nearest mipmap will be loaded.
    NearestMipmapNearest,

    /// Takes the nearest texel from the two nearest mipmaps, and merges them.
    LinearMipmapNearest,

    /// Same as `Linear`, but from the nearest mipmap.
    NearestMipmapLinear,

    /// Same as `Linear`, but from the two nearest mipmaps.
    LinearMipmapLinear,
}

impl ToGlEnum for MinifySamplerFilter {
    #[inline]
    fn to_glenum(&self) -> gl::types::GLenum {
        match *self {
            MinifySamplerFilter::Nearest => gl::NEAREST,
            MinifySamplerFilter::Linear => gl::LINEAR,
            MinifySamplerFilter::NearestMipmapNearest => gl::NEAREST_MIPMAP_NEAREST,
            MinifySamplerFilter::LinearMipmapNearest => gl::LINEAR_MIPMAP_NEAREST,
            MinifySamplerFilter::NearestMipmapLinear => gl::NEAREST_MIPMAP_LINEAR,
            MinifySamplerFilter::LinearMipmapLinear => gl::LINEAR_MIPMAP_LINEAR,
        }
    }
}

/// The depth texture comparison operation to use when comparing the r value to the value in the
/// currently bound texture.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum DepthTextureComparison {
    /// The r value is less than or equal to the texture value
    LessOrEqual,

    /// The r value is greater than or equal to the texture value
    GreaterOrEqual,

    /// The r value is less than to the texture value
    Less,

    /// The r value is greater than to the texture value
    Greater,

    /// The r value is equal to the texture value
    Equal,

    /// The r value is not equal to the texture value
    NotEqual,

    /// Always return 1.0 (true)
    Always,

    /// Never return 1.0 will return 0.0 (false)
    Never,
}

impl ToGlEnum for DepthTextureComparison {
    #[inline]
    fn to_glenum(&self) -> gl::types::GLenum {
        match *self {
            DepthTextureComparison::LessOrEqual => gl::LEQUAL,
            DepthTextureComparison::GreaterOrEqual => gl::GEQUAL,
            DepthTextureComparison::Less => gl::LESS,
            DepthTextureComparison::Greater => gl::GREATER,
            DepthTextureComparison::Equal => gl::EQUAL,
            DepthTextureComparison::NotEqual => gl::NOTEQUAL,
            DepthTextureComparison::Always => gl::ALWAYS,
            DepthTextureComparison::Never => gl::NEVER,
        }
    }
}

/// A sampler.
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Sampler<'t, T>(pub &'t T, pub SamplerBehavior);

impl<'t, T: 't> Sampler<'t, T> {
    /// Builds a new `Sampler` with default parameters.
    pub fn new(texture: &'t T) -> Sampler<'t, T> {
        Sampler(texture, Default::default())
    }

    /// Changes the wrap functions of all three coordinates.
    pub fn wrap_function(mut self, function: SamplerWrapFunction) -> Sampler<'t, T> {
        self.1.wrap_function = (function, function, function);
        self
    }

    /// Changes the minifying filter of the sampler.
    pub fn minify_filter(mut self, filter: MinifySamplerFilter) -> Sampler<'t, T> {
        self.1.minify_filter = filter;
        self
    }

    /// Changes the magnifying filter of the sampler.
    pub fn magnify_filter(mut self, filter: MagnifySamplerFilter) -> Sampler<'t, T> {
        self.1.magnify_filter = filter;
        self
    }

    /// Sets the depth texture comparison method.
    pub fn depth_texture_comparison(
        mut self,
        comparison: Option<DepthTextureComparison>,
    ) -> Sampler<'t, T> {
        self.1.depth_texture_comparison = comparison;
        self
    }

    /// Changes the magnifying filter of the sampler.
    pub fn anisotropy(mut self, level: u16) -> Sampler<'t, T> {
        self.1.max_anisotropy = level;
        self
    }

    /// Sets the color returned outside the texture with `SamplerWrapFunction::BorderClamp`,
    /// which is often white for shadow maps.
    pub fn border_color(mut self, color: [f32; 4]) -> Sampler<'t, T> {
        self.1.border_color = Some(color.into());
        self
    }
}

/// The color that a sampler returns outside the texture with `SamplerWrapFunction::BorderClamp`.
///
/// The components are stored as bits, so that `SamplerBehavior` can be hashed and compared.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct BorderColor([u32; 4]);

impl BorderColor {
    /// Returns the red, green, blue and alpha components.
    #[inline]
    pub fn to_f32(self) -> [f32; 4] {
        self.0.map(f32::from_bits)
    }
}

impl From<[f32; 4]> for BorderColor {
    #[inline]
    fn from(color: [f32; 4]) -> BorderColor {
        BorderColor(color.map(f32::to_bits))
    }
}

impl<'t, T: 't> Copy for Sampler<'t, T> {}

impl<'t, T: 't> Clone for Sampler<'t, T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Behavior of a sampler.
///
/// The default behavior, used by `texture.sampled()`, wraps with `Repeat`, like OpenGL and like
/// textures used without a sampler, minifies with `LinearMipmapLinear`, magnifies with `Linear`,
/// and has no depth comparison, no anisotropic filtering and no border color. Integer textures are
/// always sampled with the nearest filters.
///
/// A texture used as a uniform without a sampler (`uniform! { tex: &texture }`) keeps its own
/// parameters instead, which repeat the texture and filter it linearly, or with the nearest
/// filters for integer, depth and stencil textures.
// TODO: GL_TEXTURE_MIN_LOD, GL_TEXTURE_MAX_LOD, GL_TEXTURE_LOD_BIAS
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct SamplerBehavior {
    /// Functions to use for the X, Y, and Z coordinates.
    pub wrap_function: (
        SamplerWrapFunction,
        SamplerWrapFunction,
        SamplerWrapFunction,
    ),

    /// Filter to use when minifying the texture.
    pub minify_filter: MinifySamplerFilter,

    /// Filter to use when magnifying the texture.
    pub magnify_filter: MagnifySamplerFilter,

    /// The depth texture comparison function to use. Default value is None.
    pub depth_texture_comparison: Option<DepthTextureComparison>,

    /// `1` means no anisotropic filtering, any value above `1` sets the max anisotropy.
    ///
    /// ## Compatibility
    ///
    /// This parameter is always available. However it is ignored on hardware that does
    /// not support anisotropic filtering.
    ///
    /// If you set the value to a value higher than what the hardware supports, it will
    /// be clamped.
    pub max_anisotropy: u16,

    /// The color returned outside the texture with `SamplerWrapFunction::BorderClamp`. `None`
    /// keeps OpenGL's default, transparent black. Ignored where `BorderClamp` isn't supported.
    pub border_color: Option<BorderColor>,
}

impl Default for SamplerBehavior {
    #[inline]
    fn default() -> SamplerBehavior {
        SamplerBehavior {
            wrap_function: (
                SamplerWrapFunction::Repeat,
                SamplerWrapFunction::Repeat,
                SamplerWrapFunction::Repeat,
            ),
            minify_filter: MinifySamplerFilter::LinearMipmapLinear,
            magnify_filter: MagnifySamplerFilter::Linear,
            depth_texture_comparison: None,
            max_anisotropy: 1,
            border_color: None,
        }
    }
}
