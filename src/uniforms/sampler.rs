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
    /// available on WebGL. Sampling with it where it's unsupported is an error
    /// (`SamplerNotSupportedError::WrapFunction`).
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    BorderClamp,

    /// Same as Mirror, but only for one repetition,
    ///
    /// Requires OpenGL 4.4, `GL_ARB_texture_mirror_clamp_to_edge` or, on OpenGL ES,
    /// `GL_EXT_texture_mirror_clamp_to_edge` (`EXT_texture_mirror_clamp_to_edge` on WebGL).
    /// Sampling with it where it's unsupported is an
    /// error (`SamplerNotSupportedError::WrapFunction`).
    #[cfg(any(safegl_gl44, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl44, safegl_webgl))))]
    MirrorClamp,
}

impl ToGlEnum for SamplerWrapFunction {
    #[inline]
    fn to_glenum(&self) -> gl::types::GLenum {
        match *self {
            SamplerWrapFunction::Repeat => gl::REPEAT,
            SamplerWrapFunction::Mirror => gl::MIRRORED_REPEAT,
            SamplerWrapFunction::Clamp => gl::CLAMP_TO_EDGE,
            #[cfg(any(safegl_desktop, safegl_gles32))]
            SamplerWrapFunction::BorderClamp => gl::CLAMP_TO_BORDER,
            #[cfg(any(safegl_gl44, safegl_webgl))]
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

    /// Sets the lowest and the highest level of detail that the sampler uses. See
    /// `LevelOfDetail`.
    pub fn lod_range(mut self, min: f32, max: f32) -> Sampler<'t, T> {
        self.1.level_of_detail.min = min;
        self.1.level_of_detail.max = max;
        self
    }

    /// Sets the bias added to the level of detail. See `LevelOfDetail::bias`.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub fn lod_bias(mut self, bias: f32) -> Sampler<'t, T> {
        self.1.level_of_detail.bias = bias;
        self
    }

    /// Sets the color returned outside the texture with `SamplerWrapFunction::BorderClamp`,
    /// which is often white for shadow maps.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub fn border_color(mut self, color: [f32; 4]) -> Sampler<'t, T> {
        self.1.border_color = Some(color.into());
        self
    }

    /// Sets the border color of a signed integer texture. See `border_color`.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub fn border_color_int(mut self, color: [i32; 4]) -> Sampler<'t, T> {
        self.1.border_color = Some(color.into());
        self
    }

    /// Sets the border color of an unsigned integer or stencil texture. See `border_color`.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub fn border_color_uint(mut self, color: [u32; 4]) -> Sampler<'t, T> {
        self.1.border_color = Some(color.into());
        self
    }
}

/// The color that a sampler returns outside the texture with `SamplerWrapFunction::BorderClamp`.
///
/// A border color of floats is for float, normalized, depth and sRGB textures; the border color
/// of an integer texture is made of integers of the same signedness, and that of a stencil
/// texture of unsigned integers. Sampling a texture with a border color of another kind returns
/// `DrawError::BorderColorMismatch`.
///
/// The components are stored as bits, so that `SamplerBehavior` can be hashed and compared.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg(any(safegl_desktop, safegl_gles32))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
pub struct BorderColor {
    bits: [u32; 4],
    kind: BorderColorKind,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg(any(safegl_desktop, safegl_gles32))]
enum BorderColorKind {
    Float,
    Int,
    UnsignedInt,
}

/// The components of a `BorderColor`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg(any(safegl_desktop, safegl_gles32))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
pub enum BorderColorValue {
    /// Floats, for float, normalized, depth and sRGB textures.
    Float([f32; 4]),
    /// Signed integers, for signed integer textures.
    Int([i32; 4]),
    /// Unsigned integers, for unsigned integer and stencil textures.
    UnsignedInt([u32; 4]),
}

#[cfg(any(safegl_desktop, safegl_gles32))]
impl BorderColor {
    /// Returns the red, green, blue and alpha components, converted to floats if they are
    /// integers.
    #[inline]
    pub fn to_f32(self) -> [f32; 4] {
        match self.value() {
            BorderColorValue::Float(color) => color,
            BorderColorValue::Int(color) => color.map(|c| c as f32),
            BorderColorValue::UnsignedInt(color) => color.map(|c| c as f32),
        }
    }

    /// Returns the red, green, blue and alpha components.
    #[inline]
    pub fn value(self) -> BorderColorValue {
        match self.kind {
            BorderColorKind::Float => BorderColorValue::Float(self.bits.map(f32::from_bits)),
            BorderColorKind::Int => BorderColorValue::Int(self.bits.map(|c| c as i32)),
            BorderColorKind::UnsignedInt => BorderColorValue::UnsignedInt(self.bits),
        }
    }

    /// Returns true if the border color can be used with `texture`, whose components are of the
    /// same kind.
    pub(crate) fn fits(self, texture: &crate::texture::TextureAny) -> bool {
        use crate::texture::TextureKind;

        let kind = match texture.kind() {
            TextureKind::Integral => BorderColorKind::Int,
            TextureKind::Unsigned | TextureKind::Stencil => BorderColorKind::UnsignedInt,
            TextureKind::DepthStencil if texture.samples_stencil() => BorderColorKind::UnsignedInt,
            _ => BorderColorKind::Float,
        };
        self.kind == kind
    }
}

#[cfg(any(safegl_desktop, safegl_gles32))]
impl From<[f32; 4]> for BorderColor {
    #[inline]
    fn from(color: [f32; 4]) -> BorderColor {
        BorderColor {
            bits: color.map(f32::to_bits),
            kind: BorderColorKind::Float,
        }
    }
}

#[cfg(any(safegl_desktop, safegl_gles32))]
impl From<[i32; 4]> for BorderColor {
    #[inline]
    fn from(color: [i32; 4]) -> BorderColor {
        BorderColor {
            bits: color.map(|c| c as u32),
            kind: BorderColorKind::Int,
        }
    }
}

#[cfg(any(safegl_desktop, safegl_gles32))]
impl From<[u32; 4]> for BorderColor {
    #[inline]
    fn from(color: [u32; 4]) -> BorderColor {
        BorderColor {
            bits: color,
            kind: BorderColorKind::UnsignedInt,
        }
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
    /// Anisotropic filtering requires OpenGL 4.6, `GL_ARB_texture_filter_anisotropic` or
    /// `GL_EXT_texture_filter_anisotropic`: a value above 1 without them is an error
    /// (`SamplerNotSupportedError::Anisotropy`).
    ///
    /// A value higher than what the hardware supports is clamped to its maximum
    /// (`Capabilities::max_texture_max_anisotropy`).
    pub max_anisotropy: u16,

    /// The color returned outside the texture with `SamplerWrapFunction::BorderClamp`. `None`
    /// keeps OpenGL's default, transparent black. A color where `BorderClamp` isn't supported
    /// is an error (`SamplerNotSupportedError::WrapFunction`).
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub border_color: Option<BorderColor>,

    /// The levels of detail that the sampler can use, and the bias added to the level it
    /// computes. Default value is every level and no bias.
    pub level_of_detail: LevelOfDetail,
}

/// The levels of detail of a sampler (`GL_TEXTURE_MIN_LOD`, `GL_TEXTURE_MAX_LOD` and
/// `GL_TEXTURE_LOD_BIAS`).
///
/// The level of detail is the mipmap level that sampling reads, in fractions of levels relative
/// to the base level of the texture. It is clamped to `min..=max` after the bias is added.
#[derive(Debug, Clone, Copy)]
pub struct LevelOfDetail {
    /// The lowest level of detail, `-1000.0` by default.
    pub min: f32,
    /// The highest level of detail, `1000.0` by default.
    pub max: f32,
    /// The bias added to the level of detail that the GPU computes, `0.0` by default. Positive
    /// values sample smaller levels, which blurs; negative ones sharpen. It is clamped to
    /// `Limits::max_texture_lod_bias`. Shaders can add their own bias on every API.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub bias: f32,
}

impl LevelOfDetail {
    fn bits(&self) -> [u32; 3] {
        #[cfg(safegl_desktop)]
        let bias = self.bias;
        #[cfg(not(safegl_desktop))]
        let bias = 0.0f32;
        [self.min.to_bits(), self.max.to_bits(), bias.to_bits()]
    }
}

impl Default for LevelOfDetail {
    fn default() -> LevelOfDetail {
        LevelOfDetail {
            min: -1000.0,
            max: 1000.0,
            #[cfg(safegl_desktop)]
            bias: 0.0,
        }
    }
}

// compared bit by bit, so that `SamplerBehavior` can be hashed and compared
impl PartialEq for LevelOfDetail {
    fn eq(&self, other: &LevelOfDetail) -> bool {
        self.bits() == other.bits()
    }
}

impl Eq for LevelOfDetail {}

impl std::hash::Hash for LevelOfDetail {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bits().hash(state);
    }
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
            #[cfg(any(safegl_desktop, safegl_gles32))]
            border_color: None,
            level_of_detail: Default::default(),
        }
    }
}

/// A sampler that the backend can't sample with, instead of a sampler that silently samples
/// differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SamplerNotSupportedError {
    /// A wrap function that the backend doesn't have, or a border color where `BorderClamp`
    /// isn't supported. See the requirements of each `SamplerWrapFunction`.
    WrapFunction(SamplerWrapFunction),

    /// Anisotropic filtering (`max_anisotropy` above 1) without OpenGL 4.6,
    /// `GL_ARB_texture_filter_anisotropic` or `GL_EXT_texture_filter_anisotropic`.
    Anisotropy,
}

impl std::fmt::Display for SamplerNotSupportedError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SamplerNotSupportedError::WrapFunction(function) => {
                write!(fmt, "The wrap function {:?} is not supported by the backend", function)
            }
            SamplerNotSupportedError::Anisotropy => {
                fmt.write_str("Anisotropic filtering is not supported by the backend")
            }
        }
    }
}

impl std::error::Error for SamplerNotSupportedError {}
