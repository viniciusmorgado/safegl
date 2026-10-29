use crate::program;
use crate::program::BlockLayout;
#[cfg(safegl_desktop)]
use crate::program::ShaderStage;
use crate::texture;

use crate::uniforms::AsUniformValue;
use crate::uniforms::LayoutMismatchError;
use crate::uniforms::SamplerBehavior;
use crate::uniforms::UniformBlock;

use crate::buffer::BufferAnySlice;
#[cfg(any(safegl_gl42, safegl_gles31))]
use crate::uniforms::ImageUnitBehavior;

/// Type of a uniform in a program.
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum UniformType {
    Float,
    FloatVec2,
    FloatVec3,
    FloatVec4,
    Double,
    DoubleVec2,
    DoubleVec3,
    DoubleVec4,
    Int,
    IntVec2,
    IntVec3,
    IntVec4,
    UnsignedInt,
    UnsignedIntVec2,
    UnsignedIntVec3,
    UnsignedIntVec4,
    Int64,
    Int64Vec2,
    Int64Vec3,
    Int64Vec4,
    UnsignedInt64,
    UnsignedInt64Vec2,
    UnsignedInt64Vec3,
    UnsignedInt64Vec4,
    Bool,
    BoolVec2,
    BoolVec3,
    BoolVec4,
    FloatMat2,
    FloatMat3,
    FloatMat4,
    FloatMat2x3,
    FloatMat2x4,
    FloatMat3x2,
    FloatMat3x4,
    FloatMat4x2,
    FloatMat4x3,
    DoubleMat2,
    DoubleMat3,
    DoubleMat4,
    DoubleMat2x3,
    DoubleMat2x4,
    DoubleMat3x2,
    DoubleMat3x4,
    DoubleMat4x2,
    DoubleMat4x3,
    Sampler1d,
    ISampler1d,
    USampler1d,
    Sampler2d,
    ISampler2d,
    USampler2d,
    Sampler3d,
    ISampler3d,
    USampler3d,
    Sampler1dArray,
    ISampler1dArray,
    USampler1dArray,
    Sampler2dArray,
    ISampler2dArray,
    USampler2dArray,
    SamplerCube,
    ISamplerCube,
    USamplerCube,
    Sampler2dRect,
    ISampler2dRect,
    USampler2dRect,
    Sampler2dRectShadow,
    SamplerCubeArray,
    ISamplerCubeArray,
    USamplerCubeArray,
    SamplerBuffer,
    ISamplerBuffer,
    USamplerBuffer,
    Sampler2dMultisample,
    ISampler2dMultisample,
    USampler2dMultisample,
    Sampler2dMultisampleArray,
    ISampler2dMultisampleArray,
    USampler2dMultisampleArray,
    Sampler1dShadow,
    Sampler2dShadow,
    SamplerCubeShadow,
    Sampler1dArrayShadow,
    Sampler2dArrayShadow,
    SamplerCubeArrayShadow,
    Image1d,
    IImage1d,
    UImage1d,
    Image2d,
    IImage2d,
    UImage2d,
    Image3d,
    IImage3d,
    UImage3d,
    Image2dRect,
    IImage2dRect,
    UImage2dRect,
    ImageCube,
    IImageCube,
    UImageCube,
    ImageBuffer,
    IImageBuffer,
    UImageBuffer,
    Image1dArray,
    IImage1dArray,
    UImage1dArray,
    Image2dArray,
    IImage2dArray,
    UImage2dArray,
    ImageCubeArray,
    IImageCubeArray,
    UImageCubeArray,
    Image2dMultisample,
    IImage2dMultisample,
    UImage2dMultisample,
    Image2dMultisampleArray,
    IImage2dMultisampleArray,
    UImage2dMultisampleArray,
    AtomicCounterUint,
    /// `samplerExternalOES`, which samples an `ExternalTexture` (`GL_OES_EGL_image_external`).
    SamplerExternalOes,
    /// `__samplerExternal2DY2YEXT`, which samples the YUV values of an `ExternalTexture`
    /// without converting them (`GL_EXT_YUV_target`).
    SamplerExternal2dY2yExt,
    /// A type that SafeGL doesn't know, with the value OpenGL gives it. No value can be bound to
    /// a uniform of this type.
    Unknown(u32),
}

/// Represents a value to bind to a uniform.
///
/// New variants can be added without a major version, so matches need a wildcard arm.
#[allow(missing_docs)]
#[derive(Copy)]
#[non_exhaustive]
pub enum UniformValue<'a> {
    /// Contains a handle to the buffer, and a function that indicates whether this buffer
    /// can be bound on a block with the given layout.
    ///
    /// The function is what makes binding the buffer safe, so this variant can only be built
    /// with `UniformValue::block`, from a typed buffer, or with `UniformValue::block_unchecked`.
    #[non_exhaustive]
    Block(
        BufferAnySlice<'a>,
        fn(&program::UniformBlock) -> Result<(), LayoutMismatchError>,
    ),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    Subroutine(ShaderStage, &'a str),
    /// Keeps the value that the uniform has in the program: the value of the previous draw
    /// command, or the initializer of the uniform in the shader. Only for uniforms that hold
    /// data: samplers, images and blocks are bound again by every draw command.
    Keep,
    SignedInt(i32),
    UnsignedInt(u32),
    Float(f32),
    /// 2x2 column-major matrix.
    Mat2([[f32; 2]; 2]),
    /// 3x3 column-major matrix.
    Mat3([[f32; 3]; 3]),
    /// 4x4 column-major matrix.
    Mat4([[f32; 4]; 4]),
    /// Column-major matrix with 2 columns and 3 rows (`mat2x3`).
    Mat2x3([[f32; 3]; 2]),
    /// Column-major matrix with 2 columns and 4 rows (`mat2x4`).
    Mat2x4([[f32; 4]; 2]),
    /// Column-major matrix with 3 columns and 2 rows (`mat3x2`).
    Mat3x2([[f32; 2]; 3]),
    /// Column-major matrix with 3 columns and 4 rows (`mat3x4`).
    Mat3x4([[f32; 4]; 3]),
    /// Column-major matrix with 4 columns and 2 rows (`mat4x2`).
    Mat4x2([[f32; 2]; 4]),
    /// Column-major matrix with 4 columns and 3 rows (`mat4x3`).
    Mat4x3([[f32; 3]; 4]),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    IntVec2([i32; 2]),
    IntVec3([i32; 3]),
    IntVec4([i32; 4]),
    UnsignedIntVec2([u32; 2]),
    UnsignedIntVec3([u32; 3]),
    UnsignedIntVec4([u32; 4]),
    Bool(bool),
    BoolVec2([bool; 2]),
    BoolVec3([bool; 3]),
    BoolVec4([bool; 4]),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    Double(f64),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    DoubleVec2([f64; 2]),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    DoubleVec3([f64; 3]),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    DoubleVec4([f64; 4]),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    DoubleMat2([[f64; 2]; 2]),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    DoubleMat3([[f64; 3]; 3]),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    DoubleMat4([[f64; 4]; 4]),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    Int64(i64),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    Int64Vec2([i64; 2]),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    Int64Vec3([i64; 3]),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    Int64Vec4([i64; 4]),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    UnsignedInt64(u64),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    UnsignedInt64Vec2([u64; 2]),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    UnsignedInt64Vec3([u64; 3]),
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    UnsignedInt64Vec4([u64; 4]),
    /// Column-major double-precision matrix with 2 columns and 3 rows (`dmat2x3`).
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    DoubleMat2x3([[f64; 3]; 2]),
    /// Column-major double-precision matrix with 2 columns and 4 rows (`dmat2x4`).
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    DoubleMat2x4([[f64; 4]; 2]),
    /// Column-major double-precision matrix with 3 columns and 2 rows (`dmat3x2`).
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    DoubleMat3x2([[f64; 2]; 3]),
    /// Column-major double-precision matrix with 3 columns and 4 rows (`dmat3x4`).
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    DoubleMat3x4([[f64; 4]; 3]),
    /// Column-major double-precision matrix with 4 columns and 2 rows (`dmat4x2`).
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    DoubleMat4x2([[f64; 2]; 4]),
    /// Column-major double-precision matrix with 4 columns and 3 rows (`dmat4x3`).
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    DoubleMat4x3([[f64; 3]; 4]),
    /// A texture sampled by a sampler uniform, with the sampling parameters to use, or `None` to
    /// use the default ones. It can be bound to the samplers whose type matches the kind of the
    /// texture (floating-point, integral, unsigned or depth) and its dimensions.
    Texture(&'a texture::TextureAny, Option<SamplerBehavior>),
    /// A buffer texture.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    BufferTexture(texture::buffer_texture::BufferTextureRef<'a>),
    /// A texture bound to an image unit, with how to bind it.
    #[cfg(any(safegl_gl42, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles31))))]
    Image(&'a texture::TextureAny, ImageUnitBehavior),
    /// A bindless handle of a texture, for a `sampler*` uniform declared with
    /// `layout(bindless_sampler)`.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    TextureHandle(texture::TextureHandle<'a>),
    /// A buffer texture bound to an image unit, with how to bind it. See
    /// `BufferTexture::image_unit`.
    #[cfg(any(safegl_gl42, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_gles32))))]
    BufferImage(texture::buffer_texture::BufferTextureRef<'a>, ImageUnitBehavior),
    /// The values of the elements of a uniform array: element `i` is given to the uniform
    /// `name[i]`. Built from slices, arrays and vectors of values, see `UniformArray`.
    Array(&'a dyn UniformArray),
}

/// The values of the elements of a uniform array, given to a uniform with `UniformValue::Array`.
///
/// It is implemented by slices, arrays and vectors of values. Their element `i` is given to the
/// uniform `name[i]`, so the elements can be samplers, images or arrays themselves, for arrays
/// of arrays. References to slices and arrays, and vectors, can be given to a uniform array
/// directly:
///
/// ```no_run
/// # #[macro_use] extern crate safegl;
/// # fn example(first: &safegl::Texture2d, second: &safegl::Texture2d) {
/// // `uniform float weights[3];` and `uniform sampler2D textures[2];` in the shader
/// let weights = [0.25f32, 0.5, 0.25];
/// let textures = [first.sampled(), second.sampled()];
/// let uniforms = uniform! { weights: &weights, textures: &textures };
/// # }
/// ```
///
/// Elements past the end of the array of the program are ignored, like the uniforms that the
/// program doesn't have: the compiler can shorten the arrays whose last elements aren't used.
pub trait UniformArray {
    /// Returns the number of elements.
    fn len(&self) -> usize;

    /// Returns true if there are no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the value of an element. `index` is smaller than `len()`.
    fn element(&self, index: usize) -> UniformValue<'_>;
}

impl<T> UniformArray for &[T]
where
    T: AsUniformValue,
{
    #[inline]
    fn len(&self) -> usize {
        <[T]>::len(self)
    }

    #[inline]
    fn element(&self, index: usize) -> UniformValue<'_> {
        self[index].as_uniform_value()
    }
}

impl<T, const N: usize> UniformArray for [T; N]
where
    T: AsUniformValue,
{
    #[inline]
    fn len(&self) -> usize {
        N
    }

    #[inline]
    fn element(&self, index: usize) -> UniformValue<'_> {
        self[index].as_uniform_value()
    }
}

impl<T> UniformArray for Vec<T>
where
    T: AsUniformValue,
{
    #[inline]
    fn len(&self) -> usize {
        Vec::len(self)
    }

    #[inline]
    fn element(&self, index: usize) -> UniformValue<'_> {
        self[index].as_uniform_value()
    }
}

impl<T> AsUniformValue for &[T]
where
    T: AsUniformValue,
{
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Array(self)
    }
}

impl<T, const N: usize> AsUniformValue for &[T; N]
where
    T: AsUniformValue,
{
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Array(*self)
    }
}

impl<T> AsUniformValue for Vec<T>
where
    T: AsUniformValue,
{
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Array(self)
    }
}

/// Keeps the value that a uniform has in the program, instead of giving it a new one: the value
/// of the previous draw command, or the initializer of the uniform in the shader.
///
/// Every active uniform of a program must be given a value before drawing with it, or the draw
/// returns `DrawError::UniformMissing`. Pass `Keep` for the uniforms that already have the right
/// value:
///
/// ```no_run
/// # #[macro_use] extern crate safegl;
/// # fn example(program: &safegl::Program) {
/// // `uniform float scale = 2.0;` in the shader
/// let uniforms = uniform! { scale: safegl::uniforms::Keep, color: [1.0f32, 0.0, 0.0] };
/// # }
/// ```
#[derive(Debug, Copy, Clone)]
pub struct Keep;

impl AsUniformValue for Keep {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Keep
    }
}

/// A value that was already built, for example from a `Sampler`, can be passed as it is.
impl<'a> AsUniformValue for UniformValue<'a> {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        *self
    }
}

impl<'a> Clone for UniformValue<'a> {
    #[inline]
    fn clone(&self) -> UniformValue<'a> {
        *self
    }
}

impl<'a> UniformValue<'a> {
    /// Builds a value that binds a buffer to a uniform block or a shader storage block, after
    /// checking that the layout of `T` matches the layout of the block.
    #[inline]
    pub fn block<T>(buffer: crate::buffer::BufferSlice<'a, T>) -> UniformValue<'a>
    where
        T: ?Sized + UniformBlock + crate::buffer::Content,
    {
        fn matches<T>(block: &program::UniformBlock) -> Result<(), LayoutMismatchError>
        where
            T: ?Sized + UniformBlock,
        {
            T::matches(&block.layout, 0)
        }

        UniformValue::Block(buffer.as_slice_any(), matches::<T>)
    }

    /// Builds a value that binds a buffer to a uniform block or a shader storage block, with a
    /// function that checks whether the buffer matches the layout of the block.
    ///
    /// # Safety
    ///
    /// The function must only accept the layouts of blocks that the content of the buffer
    /// follows. In particular, it must not accept sampler or image members unless the buffer
    /// holds texture handles obtained from SafeGL at their offsets.
    #[inline]
    pub unsafe fn block_unchecked(
        buffer: BufferAnySlice<'a>,
        matches: fn(&program::UniformBlock) -> Result<(), LayoutMismatchError>,
    ) -> UniformValue<'a> {
        UniformValue::Block(buffer, matches)
    }

    /// Returns true if this value can be used with a uniform of the given type.
    pub fn is_usable_with(&self, ty: &UniformType) -> bool {
        match (self, *ty) {
            (&UniformValue::Keep, ty) => !is_opaque(ty),
            (&UniformValue::Bool(_), UniformType::Bool) => true,
            (&UniformValue::SignedInt(_), UniformType::Int) => true,
            (&UniformValue::UnsignedInt(_), UniformType::UnsignedInt) => true,
            (&UniformValue::Float(_), UniformType::Float) => true,
            (&UniformValue::Mat2(_), UniformType::FloatMat2) => true,
            (&UniformValue::Mat3(_), UniformType::FloatMat3) => true,
            (&UniformValue::Mat4(_), UniformType::FloatMat4) => true,
            (&UniformValue::Mat2x3(_), UniformType::FloatMat2x3) => true,
            (&UniformValue::Mat2x4(_), UniformType::FloatMat2x4) => true,
            (&UniformValue::Mat3x2(_), UniformType::FloatMat3x2) => true,
            (&UniformValue::Mat3x4(_), UniformType::FloatMat3x4) => true,
            (&UniformValue::Mat4x2(_), UniformType::FloatMat4x2) => true,
            (&UniformValue::Mat4x3(_), UniformType::FloatMat4x3) => true,
            (&UniformValue::Vec2(_), UniformType::FloatVec2) => true,
            (&UniformValue::Vec3(_), UniformType::FloatVec3) => true,
            (&UniformValue::Vec4(_), UniformType::FloatVec4) => true,
            (&UniformValue::IntVec2(_), UniformType::IntVec2) => true,
            (&UniformValue::IntVec3(_), UniformType::IntVec3) => true,
            (&UniformValue::IntVec4(_), UniformType::IntVec4) => true,
            (&UniformValue::UnsignedIntVec2(_), UniformType::UnsignedIntVec2) => true,
            (&UniformValue::UnsignedIntVec3(_), UniformType::UnsignedIntVec3) => true,
            (&UniformValue::UnsignedIntVec4(_), UniformType::UnsignedIntVec4) => true,
            (&UniformValue::BoolVec2(_), UniformType::BoolVec2) => true,
            (&UniformValue::BoolVec3(_), UniformType::BoolVec3) => true,
            (&UniformValue::BoolVec4(_), UniformType::BoolVec4) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::Double(_), UniformType::Double) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::DoubleMat2(_), UniformType::DoubleMat2) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::DoubleMat3(_), UniformType::DoubleMat3) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::DoubleMat4(_), UniformType::DoubleMat4) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::DoubleVec2(_), UniformType::DoubleVec2) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::DoubleVec3(_), UniformType::DoubleVec3) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::DoubleVec4(_), UniformType::DoubleVec4) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::Int64(_), UniformType::Int64) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::Int64Vec2(_), UniformType::Int64Vec2) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::Int64Vec3(_), UniformType::Int64Vec3) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::Int64Vec4(_), UniformType::Int64Vec4) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::UnsignedInt64(_), UniformType::UnsignedInt64) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::UnsignedInt64Vec2(_), UniformType::UnsignedInt64Vec2) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::UnsignedInt64Vec3(_), UniformType::UnsignedInt64Vec3) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::UnsignedInt64Vec4(_), UniformType::UnsignedInt64Vec4) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::DoubleMat2x3(_), UniformType::DoubleMat2x3) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::DoubleMat2x4(_), UniformType::DoubleMat2x4) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::DoubleMat3x2(_), UniformType::DoubleMat3x2) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::DoubleMat3x4(_), UniformType::DoubleMat3x4) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::DoubleMat4x2(_), UniformType::DoubleMat4x2) => true,
            #[cfg(safegl_desktop)]
            (&UniformValue::DoubleMat4x3(_), UniformType::DoubleMat4x3) => true,
            (&UniformValue::Texture(texture, _), ty) => sampler_types(texture).contains(&ty),
            // the texture of the handle is checked when it is bound
            #[cfg(safegl_desktop)]
            (&UniformValue::TextureHandle(_), ty) => is_opaque(ty),
            #[cfg(any(safegl_desktop, safegl_gles32))]
            (&UniformValue::BufferTexture(tex), UniformType::SamplerBuffer) => {
                tex.get_texture_type() == texture::buffer_texture::BufferTextureType::Float
            }
            #[cfg(any(safegl_desktop, safegl_gles32))]
            (&UniformValue::BufferTexture(tex), UniformType::ISamplerBuffer) => {
                tex.get_texture_type() == texture::buffer_texture::BufferTextureType::Integral
            }
            #[cfg(any(safegl_desktop, safegl_gles32))]
            (&UniformValue::BufferTexture(tex), UniformType::USamplerBuffer) => {
                tex.get_texture_type() == texture::buffer_texture::BufferTextureType::Unsigned
            }
            #[cfg(any(safegl_gl42, safegl_gles31))]
            (&UniformValue::Image(texture, _), ty) => image_types(texture).contains(&ty),
            #[cfg(any(safegl_gl42, safegl_gles32))]
            (&UniformValue::BufferImage(texture, _), ty) => {
                use crate::texture::buffer_texture::BufferTextureType;
                let expected = match texture.get_texture_type() {
                    BufferTextureType::Float => UniformType::ImageBuffer,
                    BufferTextureType::Integral => UniformType::IImageBuffer,
                    BufferTextureType::Unsigned => UniformType::UImageBuffer,
                };
                ty == expected
            }
            _ => false,
        }
    }
}

/// The types of the samplers that can sample a texture.
pub(crate) fn sampler_types(texture: &texture::TextureAny) -> &'static [UniformType] {
    use crate::texture::Dimensions as D;
    use crate::texture::TextureKind as K;
    use UniformType::*;

    // depth-stencil textures in stencil mode are sampled like stencil textures
    let kind = match texture.kind() {
        K::DepthStencil if texture.samples_stencil() => K::Stencil,
        kind => kind,
    };
    match (kind, texture.dimensions()) {
        // the images of external textures are sampled as floats, or as raw YUV values
        (_, D::External { .. }) => &[SamplerExternalOes, SamplerExternal2dY2yExt],
        (K::Float, D::Texture1d { .. }) => &[Sampler1d],
        (K::Float, D::Texture1dArray { .. }) => &[Sampler1dArray],
        (K::Float, D::Texture2d { .. }) => &[Sampler2d],
        (K::Float, D::TextureRectangle { .. }) => &[Sampler2dRect],
        (K::Float, D::Texture2dArray { .. }) => &[Sampler2dArray],
        (K::Float, D::Texture2dMultisample { .. }) => &[Sampler2dMultisample],
        (K::Float, D::Texture2dMultisampleArray { .. }) => &[Sampler2dMultisampleArray],
        (K::Float, D::Texture3d { .. }) => &[Sampler3d],
        (K::Float, D::Cubemap { .. }) => &[SamplerCube],
        (K::Float, D::CubemapArray { .. }) => &[SamplerCubeArray],

        (K::Integral, D::Texture1d { .. }) => &[ISampler1d],
        (K::Integral, D::Texture1dArray { .. }) => &[ISampler1dArray],
        (K::Integral, D::Texture2d { .. }) => &[ISampler2d],
        (K::Integral, D::TextureRectangle { .. }) => &[ISampler2dRect],
        (K::Integral, D::Texture2dArray { .. }) => &[ISampler2dArray],
        (K::Integral, D::Texture2dMultisample { .. }) => &[ISampler2dMultisample],
        (K::Integral, D::Texture2dMultisampleArray { .. }) => &[ISampler2dMultisampleArray],
        (K::Integral, D::Texture3d { .. }) => &[ISampler3d],
        (K::Integral, D::Cubemap { .. }) => &[ISamplerCube],
        (K::Integral, D::CubemapArray { .. }) => &[ISamplerCubeArray],

        // stencil textures are sampled as unsigned integers
        (K::Unsigned | K::Stencil, D::Texture1d { .. }) => &[USampler1d],
        (K::Unsigned | K::Stencil, D::Texture1dArray { .. }) => &[USampler1dArray],
        (K::Unsigned | K::Stencil, D::Texture2d { .. }) => &[USampler2d],
        (K::Unsigned | K::Stencil, D::TextureRectangle { .. }) => &[USampler2dRect],
        (K::Unsigned | K::Stencil, D::Texture2dArray { .. }) => &[USampler2dArray],
        (K::Unsigned | K::Stencil, D::Texture2dMultisample { .. }) => &[USampler2dMultisample],
        (K::Unsigned | K::Stencil, D::Texture2dMultisampleArray { .. }) => {
            &[USampler2dMultisampleArray]
        }
        (K::Unsigned | K::Stencil, D::Texture3d { .. }) => &[USampler3d],
        (K::Unsigned | K::Stencil, D::Cubemap { .. }) => &[USamplerCube],
        (K::Unsigned | K::Stencil, D::CubemapArray { .. }) => &[USamplerCubeArray],

        // depth textures are read by shadow samplers, with a depth comparison, or as floats
        (K::Depth | K::DepthStencil, D::Texture1d { .. }) => &[Sampler1d, Sampler1dShadow],
        (K::Depth | K::DepthStencil, D::Texture1dArray { .. }) => {
            &[Sampler1dArray, Sampler1dArrayShadow]
        }
        (K::Depth | K::DepthStencil, D::Texture2d { .. }) => &[Sampler2d, Sampler2dShadow],
        (K::Depth | K::DepthStencil, D::TextureRectangle { .. }) => {
            &[Sampler2dRect, Sampler2dRectShadow]
        }
        (K::Depth | K::DepthStencil, D::Texture2dArray { .. }) => {
            &[Sampler2dArray, Sampler2dArrayShadow]
        }
        (K::Depth | K::DepthStencil, D::Texture2dMultisample { .. }) => &[Sampler2dMultisample],
        (K::Depth | K::DepthStencil, D::Texture2dMultisampleArray { .. }) => {
            &[Sampler2dMultisampleArray]
        }
        (K::Depth | K::DepthStencil, D::Texture3d { .. }) => &[Sampler3d],
        (K::Depth | K::DepthStencil, D::Cubemap { .. }) => &[SamplerCube, SamplerCubeShadow],
        (K::Depth | K::DepthStencil, D::CubemapArray { .. }) => {
            &[SamplerCubeArray, SamplerCubeArrayShadow]
        }
    }
}

/// Returns true if the type is a sampler, an image or an atomic counter: the value of the uniform
/// refers to a texture unit, an image unit or a buffer binding point.
pub(crate) fn is_opaque(ty: UniformType) -> bool {
    use UniformType::*;
    !matches!(
        ty,
        Float
            | FloatVec2
            | FloatVec3
            | FloatVec4
            | Double
            | DoubleVec2
            | DoubleVec3
            | DoubleVec4
            | Int
            | IntVec2
            | IntVec3
            | IntVec4
            | UnsignedInt
            | UnsignedIntVec2
            | UnsignedIntVec3
            | UnsignedIntVec4
            | Int64
            | Int64Vec2
            | Int64Vec3
            | Int64Vec4
            | UnsignedInt64
            | UnsignedInt64Vec2
            | UnsignedInt64Vec3
            | UnsignedInt64Vec4
            | Bool
            | BoolVec2
            | BoolVec3
            | BoolVec4
            | FloatMat2
            | FloatMat3
            | FloatMat4
            | FloatMat2x3
            | FloatMat2x4
            | FloatMat3x2
            | FloatMat3x4
            | FloatMat4x2
            | FloatMat4x3
            | DoubleMat2
            | DoubleMat3
            | DoubleMat4
            | DoubleMat2x3
            | DoubleMat2x4
            | DoubleMat3x2
            | DoubleMat3x4
            | DoubleMat4x2
            | DoubleMat4x3
    )
}

/// Returns true if the sampler type compares the depth of the texture with a reference value.
pub(crate) fn is_shadow_sampler(ty: UniformType) -> bool {
    matches!(
        ty,
        UniformType::Sampler1dShadow
            | UniformType::Sampler2dShadow
            | UniformType::Sampler2dRectShadow
            | UniformType::SamplerCubeShadow
            | UniformType::Sampler1dArrayShadow
            | UniformType::Sampler2dArrayShadow
            | UniformType::SamplerCubeArrayShadow
    )
}

/// The types of the image uniforms that a texture can be bound to.
#[cfg(any(safegl_gl42, safegl_gles31))]
fn image_types(texture: &texture::TextureAny) -> &'static [UniformType] {
    use crate::texture::Dimensions as D;
    use crate::texture::TextureKind as K;
    use UniformType::*;

    match (texture.kind(), texture.dimensions()) {
        // external textures aren't images
        (_, D::External { .. }) => &[],
        (K::Float, D::Texture1d { .. }) => &[Image1d],
        (K::Float, D::Texture1dArray { .. }) => &[Image1dArray],
        (K::Float, D::Texture2d { .. }) => &[Image2d],
        (K::Float, D::TextureRectangle { .. }) => &[Image2dRect],
        (K::Float, D::Texture2dArray { .. }) => &[Image2dArray],
        (K::Float, D::Texture2dMultisample { .. }) => &[Image2dMultisample],
        (K::Float, D::Texture2dMultisampleArray { .. }) => &[Image2dMultisampleArray],
        (K::Float, D::Texture3d { .. }) => &[Image3d],
        (K::Float, D::Cubemap { .. }) => &[ImageCube],
        (K::Float, D::CubemapArray { .. }) => &[ImageCubeArray],

        (K::Integral, D::Texture1d { .. }) => &[IImage1d],
        (K::Integral, D::Texture1dArray { .. }) => &[IImage1dArray],
        (K::Integral, D::Texture2d { .. }) => &[IImage2d],
        (K::Integral, D::TextureRectangle { .. }) => &[IImage2dRect],
        (K::Integral, D::Texture2dArray { .. }) => &[IImage2dArray],
        (K::Integral, D::Texture2dMultisample { .. }) => &[IImage2dMultisample],
        (K::Integral, D::Texture2dMultisampleArray { .. }) => &[IImage2dMultisampleArray],
        (K::Integral, D::Texture3d { .. }) => &[IImage3d],
        (K::Integral, D::Cubemap { .. }) => &[IImageCube],
        (K::Integral, D::CubemapArray { .. }) => &[IImageCubeArray],

        (K::Unsigned, D::Texture1d { .. }) => &[UImage1d],
        (K::Unsigned, D::Texture1dArray { .. }) => &[UImage1dArray],
        (K::Unsigned, D::Texture2d { .. }) => &[UImage2d],
        (K::Unsigned, D::TextureRectangle { .. }) => &[UImage2dRect],
        (K::Unsigned, D::Texture2dArray { .. }) => &[UImage2dArray],
        (K::Unsigned, D::Texture2dMultisample { .. }) => &[UImage2dMultisample],
        (K::Unsigned, D::Texture2dMultisampleArray { .. }) => &[UImage2dMultisampleArray],
        (K::Unsigned, D::Texture3d { .. }) => &[UImage3d],
        (K::Unsigned, D::Cubemap { .. }) => &[UImageCube],
        (K::Unsigned, D::CubemapArray { .. }) => &[UImageCubeArray],

        // depth and stencil formats can't be bound to image units
        (K::Depth | K::Stencil | K::DepthStencil, _) => &[],
    }
}

macro_rules! impl_uniform_block_basic {
    ($ty:ty, $uniform_ty:expr) => {
        unsafe impl UniformBlock for $ty {
            fn matches(
                layout: &program::BlockLayout,
                base_offset: usize,
            ) -> Result<(), LayoutMismatchError> {
                // a block with a single member matches the type of the member, like for arrays
                if let BlockLayout::Struct { members } = layout {
                    if members.len() == 1 {
                        return Self::matches(&members[0].1, base_offset);
                    }
                }

                if let &BlockLayout::BasicType {
                    ty,
                    offset_in_buffer,
                } = layout
                {
                    if ty != $uniform_ty {
                        return Err(LayoutMismatchError::TypeMismatch {
                            expected: ty,
                            obtained: $uniform_ty,
                        });
                    }

                    if offset_in_buffer != base_offset {
                        return Err(LayoutMismatchError::OffsetMismatch {
                            expected: offset_in_buffer,
                            obtained: base_offset,
                        });
                    }

                    Ok(())
                } else {
                    Err(LayoutMismatchError::LayoutMismatch {
                        expected: layout.clone(),
                        obtained: BlockLayout::BasicType {
                            ty: $uniform_ty,
                            offset_in_buffer: base_offset,
                        },
                    })
                }
            }

            #[inline]
            fn build_layout(base_offset: usize) -> program::BlockLayout {
                BlockLayout::BasicType {
                    ty: $uniform_ty,
                    offset_in_buffer: base_offset,
                }
            }
        }
    };
}

impl AsUniformValue for i8 {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::SignedInt(*self as i32)
    }
}

impl AsUniformValue for u8 {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedInt(*self as u32)
    }
}

impl AsUniformValue for i16 {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::SignedInt(*self as i32)
    }
}

impl AsUniformValue for u16 {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedInt(*self as u32)
    }
}

impl AsUniformValue for i32 {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::SignedInt(*self as i32)
    }
}

impl_uniform_block_basic!(i32, UniformType::Int);

impl AsUniformValue for [i32; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::IntVec2(*self)
    }
}

impl_uniform_block_basic!([i32; 2], UniformType::IntVec2);

impl AsUniformValue for (i32, i32) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::IntVec2([self.0, self.1])
    }
}

impl_uniform_block_basic!((i32, i32), UniformType::IntVec2);

impl AsUniformValue for [i32; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::IntVec3(*self)
    }
}

impl_uniform_block_basic!([i32; 3], UniformType::IntVec3);

impl AsUniformValue for (i32, i32, i32) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::IntVec3([self.0, self.1, self.2])
    }
}

impl_uniform_block_basic!((i32, i32, i32), UniformType::IntVec3);

impl AsUniformValue for [i32; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::IntVec4(*self)
    }
}

impl_uniform_block_basic!([i32; 4], UniformType::IntVec4);

impl AsUniformValue for (i32, i32, i32, i32) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::IntVec4([self.0, self.1, self.2, self.3])
    }
}

impl_uniform_block_basic!((i32, i32, i32, i32), UniformType::IntVec4);

impl AsUniformValue for u32 {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedInt(*self as u32)
    }
}

impl_uniform_block_basic!(u32, UniformType::UnsignedInt);

impl AsUniformValue for [u32; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedIntVec2(*self)
    }
}

impl_uniform_block_basic!([u32; 2], UniformType::UnsignedIntVec2);

impl AsUniformValue for (u32, u32) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedIntVec2([self.0, self.1])
    }
}

impl_uniform_block_basic!((u32, u32), UniformType::UnsignedIntVec2);

impl AsUniformValue for [u32; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedIntVec3(*self)
    }
}

impl_uniform_block_basic!([u32; 3], UniformType::UnsignedIntVec3);

impl AsUniformValue for (u32, u32, u32) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedIntVec3([self.0, self.1, self.2])
    }
}

impl_uniform_block_basic!((u32, u32, u32), UniformType::UnsignedIntVec3);

impl AsUniformValue for [u32; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedIntVec4(*self)
    }
}

impl_uniform_block_basic!([u32; 4], UniformType::UnsignedIntVec4);

impl AsUniformValue for (u32, u32, u32, u32) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedIntVec4([self.0, self.1, self.2, self.3])
    }
}

impl_uniform_block_basic!((u32, u32, u32, u32), UniformType::UnsignedIntVec4);

impl AsUniformValue for bool {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Bool(*self)
    }
}

impl_uniform_block_basic!(bool, UniformType::Bool);

impl AsUniformValue for [bool; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::BoolVec2(*self)
    }
}

impl_uniform_block_basic!([bool; 2], UniformType::BoolVec2);

impl AsUniformValue for (bool, bool) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::BoolVec2([self.0, self.1])
    }
}

impl_uniform_block_basic!((bool, bool), UniformType::BoolVec2);

impl AsUniformValue for [bool; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::BoolVec3(*self)
    }
}

impl_uniform_block_basic!([bool; 3], UniformType::BoolVec3);

impl AsUniformValue for (bool, bool, bool) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::BoolVec3([self.0, self.1, self.2])
    }
}

impl_uniform_block_basic!((bool, bool, bool), UniformType::BoolVec3);

impl AsUniformValue for [bool; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::BoolVec4(*self)
    }
}

impl_uniform_block_basic!([bool; 4], UniformType::BoolVec4);

impl AsUniformValue for (bool, bool, bool, bool) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::BoolVec4([self.0, self.1, self.2, self.3])
    }
}

impl_uniform_block_basic!((bool, bool, bool, bool), UniformType::BoolVec4);

impl AsUniformValue for f32 {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Float(*self)
    }
}

impl_uniform_block_basic!(f32, UniformType::Float);

impl AsUniformValue for [[f32; 2]; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Mat2(*self)
    }
}

impl_uniform_block_basic!([[f32; 2]; 2], UniformType::FloatMat2);

impl AsUniformValue for [[f32; 3]; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Mat3(*self)
    }
}

impl_uniform_block_basic!([[f32; 3]; 3], UniformType::FloatMat3);

impl AsUniformValue for [[f32; 4]; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Mat4(*self)
    }
}

impl_uniform_block_basic!([[f32; 4]; 4], UniformType::FloatMat4);

impl AsUniformValue for [[f32; 3]; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Mat2x3(*self)
    }
}

impl AsUniformValue for [[f32; 4]; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Mat2x4(*self)
    }
}

impl AsUniformValue for [[f32; 2]; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Mat3x2(*self)
    }
}

impl AsUniformValue for [[f32; 4]; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Mat3x4(*self)
    }
}

impl AsUniformValue for [[f32; 2]; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Mat4x2(*self)
    }
}

impl AsUniformValue for [[f32; 3]; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Mat4x3(*self)
    }
}

impl AsUniformValue for (f32, f32) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Vec2([self.0, self.1])
    }
}

impl_uniform_block_basic!((f32, f32), UniformType::FloatVec2);

impl AsUniformValue for (f32, f32, f32) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Vec3([self.0, self.1, self.2])
    }
}

impl_uniform_block_basic!((f32, f32, f32), UniformType::FloatVec3);

impl AsUniformValue for (f32, f32, f32, f32) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Vec4([self.0, self.1, self.2, self.3])
    }
}

impl_uniform_block_basic!((f32, f32, f32, f32), UniformType::FloatVec4);

impl AsUniformValue for [f32; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Vec2(*self)
    }
}

impl_uniform_block_basic!([f32; 2], UniformType::FloatVec2);

impl AsUniformValue for [f32; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Vec3(*self)
    }
}

impl_uniform_block_basic!([f32; 3], UniformType::FloatVec3);

impl AsUniformValue for [f32; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Vec4(*self)
    }
}

impl_uniform_block_basic!([f32; 4], UniformType::FloatVec4);

//TODO bool, i32, u32 and f64 should also be implemented as cgmath and nalgebra variants (i.e. nalgebra::Vec3<f64>).
// Start of double type variants
#[cfg(safegl_desktop)]
impl AsUniformValue for f64 {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Double(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!(f64, UniformType::Double);

#[cfg(safegl_desktop)]
impl AsUniformValue for [f64; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleVec2(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!([f64; 2], UniformType::DoubleVec2);

#[cfg(safegl_desktop)]
impl AsUniformValue for (f64, f64) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleVec2([self.0, self.1])
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!((f64, f64), UniformType::DoubleVec2);

#[cfg(safegl_desktop)]
impl AsUniformValue for [f64; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleVec3(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!([f64; 3], UniformType::DoubleVec3);

#[cfg(safegl_desktop)]
impl AsUniformValue for (f64, f64, f64) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleVec3([self.0, self.1, self.2])
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!((f64, f64, f64), UniformType::DoubleVec3);

#[cfg(safegl_desktop)]
impl AsUniformValue for [f64; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleVec4(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!([f64; 4], UniformType::DoubleVec4);

#[cfg(safegl_desktop)]
impl AsUniformValue for (f64, f64, f64, f64) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleVec4([self.0, self.1, self.2, self.3])
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!((f64, f64, f64, f64), UniformType::DoubleVec4);

#[cfg(safegl_desktop)]
impl AsUniformValue for [[f64; 2]; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleMat2(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!([[f64; 2]; 2], UniformType::DoubleMat2);

#[cfg(safegl_desktop)]
impl AsUniformValue for [[f64; 3]; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleMat3(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!([[f64; 3]; 3], UniformType::DoubleMat3);

#[cfg(safegl_desktop)]
impl AsUniformValue for [[f64; 4]; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleMat4(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!([[f64; 4]; 4], UniformType::DoubleMat4);

#[cfg(safegl_desktop)]
impl AsUniformValue for [[f64; 3]; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleMat2x3(*self)
    }
}

#[cfg(safegl_desktop)]
impl AsUniformValue for [[f64; 4]; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleMat2x4(*self)
    }
}

#[cfg(safegl_desktop)]
impl AsUniformValue for [[f64; 2]; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleMat3x2(*self)
    }
}

#[cfg(safegl_desktop)]
impl AsUniformValue for [[f64; 4]; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleMat3x4(*self)
    }
}

#[cfg(safegl_desktop)]
impl AsUniformValue for [[f64; 2]; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleMat4x2(*self)
    }
}

#[cfg(safegl_desktop)]
impl AsUniformValue for [[f64; 3]; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::DoubleMat4x3(*self)
    }
}
#[cfg(safegl_desktop)]
impl AsUniformValue for i64 {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Int64(*self as i64)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!(i64, UniformType::Int);

#[cfg(safegl_desktop)]
impl AsUniformValue for [i64; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Int64Vec2(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!([i64; 2], UniformType::Int64Vec2);

#[cfg(safegl_desktop)]
impl AsUniformValue for (i64, i64) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Int64Vec2([self.0, self.1])
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!((i64, i64), UniformType::Int64Vec2);

#[cfg(safegl_desktop)]
impl AsUniformValue for [i64; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Int64Vec3(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!([i64; 3], UniformType::Int64Vec3);

#[cfg(safegl_desktop)]
impl AsUniformValue for (i64, i64, i64) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Int64Vec3([self.0, self.1, self.2])
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!((i64, i64, i64), UniformType::Int64Vec3);

#[cfg(safegl_desktop)]
impl AsUniformValue for [i64; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Int64Vec4(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!([i64; 4], UniformType::Int64Vec4);

#[cfg(safegl_desktop)]
impl AsUniformValue for (i64, i64, i64, i64) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Int64Vec4([self.0, self.1, self.2, self.3])
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!((i64, i64, i64, i64), UniformType::Int64Vec4);

#[cfg(safegl_desktop)]
impl AsUniformValue for u64 {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedInt64(*self as u64)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!(u64, UniformType::UnsignedInt64);

#[cfg(safegl_desktop)]
impl AsUniformValue for [u64; 2] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedInt64Vec2(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!([u64; 2], UniformType::UnsignedInt64Vec2);

#[cfg(safegl_desktop)]
impl AsUniformValue for (u64, u64) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedInt64Vec2([self.0, self.1])
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!((u64, u64), UniformType::UnsignedInt64Vec2);

#[cfg(safegl_desktop)]
impl AsUniformValue for [u64; 3] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedInt64Vec3(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!([u64; 3], UniformType::UnsignedInt64Vec3);

#[cfg(safegl_desktop)]
impl AsUniformValue for (u64, u64, u64) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedInt64Vec3([self.0, self.1, self.2])
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!((u64, u64, u64), UniformType::UnsignedInt64Vec3);

#[cfg(safegl_desktop)]
impl AsUniformValue for [u64; 4] {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedInt64Vec4(*self)
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!([u64; 4], UniformType::UnsignedInt64Vec4);

#[cfg(safegl_desktop)]
impl AsUniformValue for (u64, u64, u64, u64) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::UnsignedInt64Vec4([self.0, self.1, self.2, self.3])
    }
}

#[cfg(safegl_desktop)]
impl_uniform_block_basic!((u64, u64, u64, u64), UniformType::UnsignedInt64Vec4);

// Subroutines
#[cfg(safegl_desktop)]
impl<'a> AsUniformValue for (&'a str, ShaderStage) {
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::Subroutine(self.1, self.0)
    }
}
