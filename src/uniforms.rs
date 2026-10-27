/*!
A uniform is a global variable in your program. In order to draw something, you will need to
give `SafeGL` the values of all your uniforms. Objects that implement the `Uniform` trait are
here to do that.

The currently preferred way to do this is to use the `uniform!` macro provided by SafeGL:

```no_run
# use safegl::uniform;
# use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
# fn example<T>(display: safegl::Display<T>) where T: SurfaceTypeTrait + ResizeableSurface {
# let tex: f32 = 0.0;
# let matrix: f32 = 0.0;
let uniforms = uniform! {
    texture: tex,
    matrix: matrix
};
# }
```

Each field must implement the `UniformValue` trait for this to work.

## Samplers

In order to customize the way a texture is being sampled, you must use a `Sampler`.

```no_run
# use safegl::uniform;
# use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
# fn example<T>(display: safegl::Display<T>, texture: safegl::texture::Texture2d)
#     where T: SurfaceTypeTrait + ResizeableSurface {
let uniforms = uniform! {
    texture: safegl::uniforms::Sampler::new(&texture)
                        .magnify_filter(safegl::uniforms::MagnifySamplerFilter::Nearest)
};
# }
```

## Blocks

In GLSL, you can choose to use a uniform *block*. When you use a block, you first need to
upload the content of this block in the video memory thanks to a `UniformBuffer`. Then you
can link the buffer to the name of the block, just like any other uniform.

```no_run
# use safegl::uniform;
# use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
# fn example<T>(display: safegl::Display<T>, texture: safegl::texture::Texture2d)
#     where T: SurfaceTypeTrait + ResizeableSurface {
let program = safegl::Program::from_source(&display,
    "
        #version 110

        attribute vec2 position;

        void main() {
            gl_Position = vec4(position, 0.0, 1.0);
        }
    ",
    "
        #version 330
        uniform layout(std140);

        uniform MyBlock {
            vec3 color;
        };

        void main() {
            gl_FragColor = vec4(color, 1.0);
        }
    ",
    None);

let buffer = safegl::uniforms::UniformBuffer::new(&display, (0.5f32, 0.5f32, 0.5f32)).unwrap();

let uniforms = uniform! {
    MyBlock: &buffer
};
# }
```

## Subroutines
OpenGL allows the use of subroutines, which are like function pointers. Subroutines can be used
to change the functionality of a shader program at runtime. This method is usually a lot faster
than using multiple programs that are switched during execution.

A subroutine uniform is unique per shader stage, and not per program.

```no_run
# use safegl::uniform;
# use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
# fn example<T>(display: safegl::Display<T>, texture: safegl::texture::Texture2d)
#     where T: SurfaceTypeTrait + ResizeableSurface {
let program = safegl::Program::from_source(&display,
    "
        #version 150
        in vec2 position;
        void main() {
            gl_Position = vec4(position, 0.0, 1.0);
        }
    ",
    "
        #version 150
        #extension GL_ARB_shader_subroutine : require
        out vec4 fragColor;
        subroutine vec4 modify_t(vec4 color);
        subroutine uniform modify_t modify_color;

        subroutine(modify_t) vec4 delete_r(vec4 color)
        {
          return vec4(0, color.g, color.b, color.a);
        }

        subroutine(modify_t) vec4 delete_b(vec4 color)
        {
          return vec4(color.r, color.g, 0, color.a);
        }

        void main()
        {
            vec4 white= vec4(1, 1, 1, 1);
            fragColor = modify_color(white);
        }
    ", None);

    let uniforms = uniform! {
        modify_color: ("delete_b", safegl::program::ShaderStage::Fragment)
    };
# }
```
*/
pub use self::buffer::UniformBuffer;
#[cfg(feature = "gl46")]
#[cfg_attr(docsrs, doc(cfg(feature = "gl46")))]
pub use self::image_unit::{ImageUnit, ImageUnitBehavior};
#[cfg(feature = "gl46")]
#[cfg_attr(docsrs, doc(cfg(feature = "gl46")))]
pub use self::image_unit::{ImageUnitAccess, ImageUnitError, ImageUnitFormat};
pub use self::sampler::{
    DepthTextureComparison, MagnifySamplerFilter, MinifySamplerFilter, SamplerWrapFunction,
};
pub use self::sampler::{BorderColor, Sampler, SamplerBehavior};
pub use self::uniforms::{DynamicUniforms, EmptyUniforms, UniformsStorage};
pub(crate) use self::value::{is_opaque, is_shadow_sampler};
pub use self::value::Keep;
pub use self::value::{UniformType, UniformValue};

use std::error::Error;
use std::fmt;

use crate::buffer::Buffer;
use crate::buffer::Content as BufferContent;
use crate::program;
use crate::program::BlockLayout;

mod bind;
pub(crate) use self::bind::CollectedUniforms;
mod buffer;
#[cfg(feature = "gl46")]
mod image_unit;
mod sampler;
mod uniforms;
mod value;

/// Object that contains the values of all the uniforms to bind to a program.
///
/// Objects of this type can be passed to the `draw()` function.
pub trait Uniforms {
    /// Calls the parameter once with the name and value of each uniform.
    ///
    /// The names live as long as `self`, so that SafeGL can collect the values before it starts
    /// to issue OpenGL commands: the implementations of this trait and of `AsUniformValue` can
    /// create or read SafeGL objects.
    fn visit_values<'a, F: FnMut(&'a str, UniformValue<'a>)>(&'a self, _: F);
}

/// Error about a block layout mismatch.
#[derive(Clone, Debug)]
pub enum LayoutMismatchError {
    /// There is a mismatch in the type of one element.
    TypeMismatch {
        /// Type expected by the shader.
        expected: UniformType,
        /// Type that you gave.
        obtained: UniformType,
    },

    /// The expected layout is totally different from what we have.
    LayoutMismatch {
        /// Layout expected by the shader.
        expected: BlockLayout,
        /// Layout of the input.
        obtained: BlockLayout,
    },

    /// The type of data is good, but there is a misalignment.
    OffsetMismatch {
        /// Expected offset of a member.
        expected: usize,
        /// Offset of the same member in the input.
        obtained: usize,
    },

    /// There is a mismatch in a submember of this layout.
    ///
    /// This is kind of a hierarchy inside the `LayoutMismatchError`s.
    MemberMismatch {
        /// Name of the field.
        member: String,
        /// The sub-error.
        err: Box<LayoutMismatchError>,
    },

    /// A field is missing in either the expected of the input data layout.
    MissingField {
        /// Name of the field.
        name: String,
    },
}

impl Error for LayoutMismatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use self::LayoutMismatchError::*;
        match *self {
            MemberMismatch { ref err, .. } => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl fmt::Display for LayoutMismatchError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        use self::LayoutMismatchError::*;
        let desc = match *self {
            TypeMismatch { .. } => "There is a mismatch in the type of one element",
            LayoutMismatch { .. } => "The expected layout is totally different from what we have",
            OffsetMismatch { .. } => "The type of data is good, but there is a misalignment",
            MemberMismatch { .. } => "There is a mismatch in a submember of this layout",
            MissingField { .. } => {
                "A field is missing in either the expected of the input data layout"
            }
        };
        match *self {
            //duplicate Patternmatching, different Types can't be condensed
            TypeMismatch {
                ref expected,
                ref obtained,
            } => write!(
                fmt,
                "{}, got: {:?}, expected: {:?}",
                desc, obtained, expected,
            ),
            LayoutMismatch {
                ref expected,
                ref obtained,
            } => write!(
                fmt,
                "{}, got: {:?}, expected: {:?}",
                desc, obtained, expected,
            ),
            OffsetMismatch {
                ref expected,
                ref obtained,
            } => write!(fmt, "{}, got: {}, expected: {}", desc, obtained, expected,),
            MemberMismatch {
                ref member,
                ref err,
            } => write!(fmt, "{}, {}: {}", desc, member, err,),
            MissingField { ref name } => write!(fmt, "{}: {}", desc, name,),
        }
    }
}

/// Value that can be used as the value of a uniform.
///
/// This includes buffers and textures for example.
pub trait AsUniformValue {
    /// Builds a `UniformValue`.
    fn as_uniform_value(&self) -> UniformValue<'_>;
}

// TODO: no way to bind a slice
impl<'a, T: ?Sized> AsUniformValue for &'a Buffer<T>
where
    T: UniformBlock + BufferContent,
{
    #[inline]
    fn as_uniform_value(&self) -> UniformValue<'_> {
        UniformValue::block(self.as_slice())
    }
}

/// Objects that are suitable for being inside a uniform block or a SSBO.
///
/// Use `implement_uniform_block!` to implement it for a struct.
///
/// # Safety
///
/// `matches` and `build_layout` must describe the layout of the type in memory. In particular,
/// only types that hold texture handles obtained from SafeGL, like `TextureHandle`, may match the
/// sampler and image types of a block: the GPU uses the bytes of these members as bindless
/// handles, and a handle that doesn't come from a resident texture makes it sample garbage or
/// crash.
pub unsafe trait UniformBlock {
    // TODO: `: Copy`, but unsized structs don't impl `Copy`
    /// Checks whether the uniforms' layout matches the given block if `Self` starts at
    /// the given offset.
    fn matches(_: &BlockLayout, base_offset: usize) -> Result<(), LayoutMismatchError>;

    /// Builds the `BlockLayout` corresponding to the current object.
    fn build_layout(base_offset: usize) -> BlockLayout;
}

unsafe impl<T> UniformBlock for [T]
where
    T: UniformBlock,
{
    fn matches(layout: &BlockLayout, base_offset: usize) -> Result<(), LayoutMismatchError> {
        if let BlockLayout::Struct { members } = layout {
            if members.len() == 1 {
                return Self::matches(&members[0].1, base_offset);
            }
        }

        if let BlockLayout::DynamicSizedArray { content } = layout {
            <T as UniformBlock>::matches(content, base_offset).map_err(|err| {
                LayoutMismatchError::MemberMismatch {
                    member: "<dynamic array content>".to_owned(),
                    err: Box::new(err),
                }
            })
        } else if let BlockLayout::Array { content, .. } = layout {
            <T as UniformBlock>::matches(content, base_offset).map_err(|err| {
                LayoutMismatchError::MemberMismatch {
                    member: "<dynamic array content>".to_owned(),
                    err: Box::new(err),
                }
            })
        } else {
            Err(LayoutMismatchError::LayoutMismatch {
                expected: layout.clone(),
                obtained: <Self as UniformBlock>::build_layout(base_offset),
            })
        }
    }

    #[inline]
    fn build_layout(base_offset: usize) -> BlockLayout {
        BlockLayout::DynamicSizedArray {
            content: Box::new(<T as UniformBlock>::build_layout(base_offset)),
        }
    }
}

macro_rules! impl_uniform_block_array {
    ($len:expr) => {
        unsafe impl<T> UniformBlock for [T; $len]
        where
            T: UniformBlock,
        {
            fn matches(
                layout: &program::BlockLayout,
                base_offset: usize,
            ) -> Result<(), LayoutMismatchError> {
                if let &BlockLayout::Struct { ref members } = layout {
                    if members.len() == 1 {
                        return Self::matches(&members[0].1, base_offset);
                    }
                }

                if let &BlockLayout::Array {
                    ref content,
                    length,
                } = layout
                {
                    if let Err(err) = T::matches(content, base_offset) {
                        return Err(LayoutMismatchError::MemberMismatch {
                            member: "<array content>".to_owned(),
                            err: Box::new(err),
                        });
                    }

                    if length != $len {
                        return Err(LayoutMismatchError::LayoutMismatch {
                            expected: layout.clone(),
                            obtained: Self::build_layout(base_offset),
                        });
                    }

                    Ok(())
                } else {
                    Err(LayoutMismatchError::LayoutMismatch {
                        expected: layout.clone(),
                        obtained: Self::build_layout(base_offset),
                    })
                }
            }

            #[inline]
            fn build_layout(base_offset: usize) -> program::BlockLayout {
                BlockLayout::Array {
                    content: Box::new(T::build_layout(base_offset)),
                    length: $len,
                }
            }
        }
    };
}

impl_uniform_block_array!(5);
impl_uniform_block_array!(6);
impl_uniform_block_array!(7);
impl_uniform_block_array!(8);
impl_uniform_block_array!(9);
impl_uniform_block_array!(10);
impl_uniform_block_array!(11);
impl_uniform_block_array!(12);
impl_uniform_block_array!(13);
impl_uniform_block_array!(14);
impl_uniform_block_array!(15);
impl_uniform_block_array!(16);
impl_uniform_block_array!(17);
impl_uniform_block_array!(18);
impl_uniform_block_array!(19);
impl_uniform_block_array!(20);
impl_uniform_block_array!(21);
impl_uniform_block_array!(22);
impl_uniform_block_array!(23);
impl_uniform_block_array!(24);
impl_uniform_block_array!(25);
impl_uniform_block_array!(26);
impl_uniform_block_array!(27);
impl_uniform_block_array!(28);
impl_uniform_block_array!(29);
impl_uniform_block_array!(30);
impl_uniform_block_array!(31);
impl_uniform_block_array!(32);
impl_uniform_block_array!(64);
impl_uniform_block_array!(128);
impl_uniform_block_array!(256);
impl_uniform_block_array!(512);
impl_uniform_block_array!(1024);
impl_uniform_block_array!(2048);
impl_uniform_block_array!(4096);
