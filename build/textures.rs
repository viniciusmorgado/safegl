use std::io::Write;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TextureType {
    Regular,
    Compressed,
    Srgb,
    CompressedSrgb,
    Integral,
    Unsigned,
    Depth,
    Stencil,
    DepthStencil,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TextureDimensions {
    Texture1d,
    Texture2d,
    TextureRectangle,
    Texture2dMultisample,
    Texture3d,
    Texture1dArray,
    Texture2dArray,
    Texture2dMultisampleArray,
    Cubemap,
    CubemapArray,
}

impl TextureDimensions {
    fn is_array(&self) -> bool {
        match self {
            TextureDimensions::Texture1dArray => true,
            TextureDimensions::Texture2dArray => true,
            TextureDimensions::Texture2dMultisampleArray => true,
            TextureDimensions::CubemapArray => true,
            _ => false,
        }
    }

    fn is_multisample(&self) -> bool {
        match self {
            TextureDimensions::Texture2dMultisample => true,
            TextureDimensions::Texture2dMultisampleArray => true,
            _ => false,
        }
    }

    fn is_cube(&self) -> bool {
        match self {
            TextureDimensions::Cubemap => true,
            TextureDimensions::CubemapArray => true,
            _ => false,
        }
    }
}

/// The levels that have a kind of texture, as a `cfg` predicate, or `None` for every level.
fn level_gate(ty: &TextureType, dimensions: &TextureDimensions) -> Option<String> {
    let mut gates = Vec::new();
    match *dimensions {
        TextureDimensions::Texture1d
        | TextureDimensions::Texture1dArray
        | TextureDimensions::TextureRectangle => gates.push("safegl_desktop"),
        TextureDimensions::Texture2dMultisample => gates.push("any(safegl_desktop, safegl_gles31)"),
        TextureDimensions::Texture2dMultisampleArray | TextureDimensions::CubemapArray => {
            gates.push("any(safegl_desktop, safegl_gles32)")
        }
        _ => (),
    }
    // `GL_STENCIL_INDEX8` textures are core in OpenGL 4.4 and OpenGL ES 3.2
    if let TextureType::Stencil = *ty {
        gates.push("any(safegl_gl44, safegl_gles32)");
    }
    match gates.len() {
        0 => None,
        1 => Some(gates[0].to_owned()),
        _ => Some(format!("all({})", gates.join(", "))),
    }
}

pub fn build_texture_file<W: Write>(dest: &mut W) {
    build_texture(dest, TextureType::Regular, TextureDimensions::Texture1d);
    build_texture(dest, TextureType::Compressed, TextureDimensions::Texture1d);
    build_texture(dest, TextureType::Srgb, TextureDimensions::Texture1d);
    build_texture(
        dest,
        TextureType::CompressedSrgb,
        TextureDimensions::Texture1d,
    );
    build_texture(dest, TextureType::Integral, TextureDimensions::Texture1d);
    build_texture(dest, TextureType::Unsigned, TextureDimensions::Texture1d);
    build_texture(dest, TextureType::Depth, TextureDimensions::Texture1d);
    build_texture(dest, TextureType::Stencil, TextureDimensions::Texture1d);
    build_texture(
        dest,
        TextureType::DepthStencil,
        TextureDimensions::Texture1d,
    );
    build_texture(dest, TextureType::Regular, TextureDimensions::Texture2d);
    // rectangle textures have neither compressed nor stencil formats
    for ty in [
        TextureType::Regular,
        TextureType::Srgb,
        TextureType::Integral,
        TextureType::Unsigned,
        TextureType::Depth,
        TextureType::DepthStencil,
    ] {
        build_texture(dest, ty, TextureDimensions::TextureRectangle);
    }
    build_texture(dest, TextureType::Compressed, TextureDimensions::Texture2d);
    build_texture(dest, TextureType::Srgb, TextureDimensions::Texture2d);
    build_texture(
        dest,
        TextureType::CompressedSrgb,
        TextureDimensions::Texture2d,
    );
    build_texture(dest, TextureType::Integral, TextureDimensions::Texture2d);
    build_texture(dest, TextureType::Unsigned, TextureDimensions::Texture2d);
    build_texture(dest, TextureType::Depth, TextureDimensions::Texture2d);
    build_texture(dest, TextureType::Stencil, TextureDimensions::Texture2d);
    build_texture(
        dest,
        TextureType::DepthStencil,
        TextureDimensions::Texture2d,
    );
    build_texture(
        dest,
        TextureType::Regular,
        TextureDimensions::Texture2dMultisample,
    );
    build_texture(
        dest,
        TextureType::Integral,
        TextureDimensions::Texture2dMultisample,
    );
    build_texture(
        dest,
        TextureType::Srgb,
        TextureDimensions::Texture2dMultisample,
    );
    build_texture(
        dest,
        TextureType::Unsigned,
        TextureDimensions::Texture2dMultisample,
    );
    build_texture(
        dest,
        TextureType::Depth,
        TextureDimensions::Texture2dMultisample,
    );
    build_texture(
        dest,
        TextureType::Stencil,
        TextureDimensions::Texture2dMultisample,
    );
    build_texture(
        dest,
        TextureType::DepthStencil,
        TextureDimensions::Texture2dMultisample,
    );
    build_texture(dest, TextureType::Regular, TextureDimensions::Texture3d);
    build_texture(dest, TextureType::Compressed, TextureDimensions::Texture3d);
    build_texture(dest, TextureType::Srgb, TextureDimensions::Texture3d);
    build_texture(
        dest,
        TextureType::CompressedSrgb,
        TextureDimensions::Texture3d,
    );
    build_texture(dest, TextureType::Integral, TextureDimensions::Texture3d);
    build_texture(dest, TextureType::Unsigned, TextureDimensions::Texture3d);
    build_texture(dest, TextureType::Depth, TextureDimensions::Texture3d);
    //build_texture(dest, TextureType::Stencil, TextureDimensions::Texture3d);  // forbidden
    build_texture(
        dest,
        TextureType::DepthStencil,
        TextureDimensions::Texture3d,
    );
    build_texture(
        dest,
        TextureType::Regular,
        TextureDimensions::Texture1dArray,
    );
    build_texture(
        dest,
        TextureType::Compressed,
        TextureDimensions::Texture1dArray,
    );
    build_texture(dest, TextureType::Srgb, TextureDimensions::Texture1dArray);
    build_texture(
        dest,
        TextureType::CompressedSrgb,
        TextureDimensions::Texture1dArray,
    );
    build_texture(
        dest,
        TextureType::Integral,
        TextureDimensions::Texture1dArray,
    );
    build_texture(
        dest,
        TextureType::Unsigned,
        TextureDimensions::Texture1dArray,
    );
    build_texture(dest, TextureType::Depth, TextureDimensions::Texture1dArray);
    build_texture(
        dest,
        TextureType::Stencil,
        TextureDimensions::Texture1dArray,
    );
    build_texture(
        dest,
        TextureType::DepthStencil,
        TextureDimensions::Texture1dArray,
    );
    build_texture(
        dest,
        TextureType::Regular,
        TextureDimensions::Texture2dArray,
    );
    build_texture(
        dest,
        TextureType::Compressed,
        TextureDimensions::Texture2dArray,
    );
    build_texture(dest, TextureType::Srgb, TextureDimensions::Texture2dArray);
    build_texture(
        dest,
        TextureType::CompressedSrgb,
        TextureDimensions::Texture2dArray,
    );
    build_texture(
        dest,
        TextureType::Integral,
        TextureDimensions::Texture2dArray,
    );
    build_texture(
        dest,
        TextureType::Unsigned,
        TextureDimensions::Texture2dArray,
    );
    build_texture(dest, TextureType::Depth, TextureDimensions::Texture2dArray);
    build_texture(
        dest,
        TextureType::Stencil,
        TextureDimensions::Texture2dArray,
    );
    build_texture(
        dest,
        TextureType::DepthStencil,
        TextureDimensions::Texture2dArray,
    );
    build_texture(
        dest,
        TextureType::Regular,
        TextureDimensions::Texture2dMultisampleArray,
    );
    build_texture(
        dest,
        TextureType::Srgb,
        TextureDimensions::Texture2dMultisampleArray,
    );
    build_texture(
        dest,
        TextureType::Integral,
        TextureDimensions::Texture2dMultisampleArray,
    );
    build_texture(
        dest,
        TextureType::Unsigned,
        TextureDimensions::Texture2dMultisampleArray,
    );
    build_texture(
        dest,
        TextureType::Depth,
        TextureDimensions::Texture2dMultisampleArray,
    );
    build_texture(
        dest,
        TextureType::Stencil,
        TextureDimensions::Texture2dMultisampleArray,
    );
    build_texture(
        dest,
        TextureType::DepthStencil,
        TextureDimensions::Texture2dMultisampleArray,
    );
    build_texture(dest, TextureType::Regular, TextureDimensions::Cubemap);
    build_texture(dest, TextureType::Compressed, TextureDimensions::Cubemap);
    build_texture(dest, TextureType::Srgb, TextureDimensions::Cubemap);
    build_texture(
        dest,
        TextureType::CompressedSrgb,
        TextureDimensions::Cubemap,
    );
    build_texture(dest, TextureType::Integral, TextureDimensions::Cubemap);
    build_texture(dest, TextureType::Unsigned, TextureDimensions::Cubemap);
    build_texture(dest, TextureType::Depth, TextureDimensions::Cubemap);
    build_texture(dest, TextureType::Stencil, TextureDimensions::Cubemap);
    build_texture(dest, TextureType::DepthStencil, TextureDimensions::Cubemap);
    build_texture(dest, TextureType::Regular, TextureDimensions::CubemapArray);
    build_texture(
        dest,
        TextureType::Compressed,
        TextureDimensions::CubemapArray,
    );
    build_texture(dest, TextureType::Srgb, TextureDimensions::CubemapArray);
    build_texture(
        dest,
        TextureType::CompressedSrgb,
        TextureDimensions::CubemapArray,
    );
    build_texture(dest, TextureType::Integral, TextureDimensions::CubemapArray);
    build_texture(dest, TextureType::Unsigned, TextureDimensions::CubemapArray);
    build_texture(dest, TextureType::Depth, TextureDimensions::CubemapArray);
    build_texture(dest, TextureType::Stencil, TextureDimensions::CubemapArray);
    build_texture(
        dest,
        TextureType::DepthStencil,
        TextureDimensions::CubemapArray,
    );
}

fn build_texture<W: Write>(dest: &mut W, ty: TextureType, dimensions: TextureDimensions) {
    // building the name of the module
    let module_name: String = {
        let prefix = match ty {
            TextureType::Regular => "",
            TextureType::Compressed => "compressed_",
            TextureType::Srgb => "srgb_",
            TextureType::CompressedSrgb => "compressed_srgb_",
            TextureType::Integral => "integral_",
            TextureType::Unsigned => "unsigned_",
            TextureType::Depth => "depth_",
            TextureType::Stencil => "stencil_",
            TextureType::DepthStencil => "depth_stencil_",
        };

        let suffix = match dimensions {
            TextureDimensions::Texture1d => "texture1d",
            TextureDimensions::Texture2d => "texture2d",
            TextureDimensions::TextureRectangle => "texture_rectangle",
            TextureDimensions::Texture2dMultisample => "texture2d_multisample",
            TextureDimensions::Texture3d => "texture3d",
            TextureDimensions::Texture1dArray => "texture1d_array",
            TextureDimensions::Texture2dArray => "texture2d_array",
            TextureDimensions::Texture2dMultisampleArray => "texture2d_multisample_array",
            TextureDimensions::Cubemap => "cubemap",
            TextureDimensions::CubemapArray => "cubemap_array",
        };

        format!("{}{}", prefix, suffix)
    };

    // building the name of the texture type
    let name: String = {
        let prefix = match ty {
            TextureType::Regular => "",
            TextureType::Compressed => "Compressed",
            TextureType::Srgb => "Srgb",
            TextureType::CompressedSrgb => "CompressedSrgb",
            TextureType::Integral => "Integral",
            TextureType::Unsigned => "Unsigned",
            TextureType::Depth => "Depth",
            TextureType::Stencil => "Stencil",
            TextureType::DepthStencil => "DepthStencil",
        };

        let suffix = match dimensions {
            TextureDimensions::Texture1d => "Texture1d",
            TextureDimensions::Texture2d => "Texture2d",
            TextureDimensions::TextureRectangle => "TextureRectangle",
            TextureDimensions::Texture2dMultisample => "Texture2dMultisample",
            TextureDimensions::Texture3d => "Texture3d",
            TextureDimensions::Texture1dArray => "Texture1dArray",
            TextureDimensions::Texture2dArray => "Texture2dArray",
            TextureDimensions::Texture2dMultisampleArray => "Texture2dMultisampleArray",
            TextureDimensions::Cubemap => "Cubemap",
            TextureDimensions::CubemapArray => "CubemapArray",
        };

        format!("{}{}", prefix, suffix)
    };

    // the trait corresponding to the data source
    let data_source_trait = match dimensions {
        TextureDimensions::Texture1d | TextureDimensions::Texture1dArray => "Texture1dDataSource",
        TextureDimensions::Texture2d
        | TextureDimensions::TextureRectangle
        | TextureDimensions::Texture2dArray => "Texture2dDataSource",
        TextureDimensions::Texture3d => "Texture3dDataSource",
        TextureDimensions::Texture2dMultisample
        | TextureDimensions::Texture2dMultisampleArray
        | TextureDimensions::Cubemap
        | TextureDimensions::CubemapArray => "unreachable",
    };

    // the format enum corresponding to this texture
    let relevant_format = match ty {
        TextureType::Regular => "UncompressedFloatFormat",
        TextureType::Compressed => "CompressedFormat",
        TextureType::Srgb => "SrgbFormat",
        TextureType::CompressedSrgb => "CompressedSrgbFormat",
        TextureType::Integral => "UncompressedIntFormat",
        TextureType::Unsigned => "UncompressedUintFormat",
        TextureType::Depth => "DepthFormat",
        TextureType::Stencil => "StencilFormat",
        TextureType::DepthStencil => "DepthStencilFormat",
    };

    // the default format to use when none is specified
    let default_format = match ty {
        TextureType::Compressed => "TextureFormatRequest::AnyCompressed",
        TextureType::Regular => "TextureFormatRequest::AnyFloatingPoint",
        TextureType::CompressedSrgb => "TextureFormatRequest::AnyCompressedSrgb",
        TextureType::Srgb => "TextureFormatRequest::AnySrgb",
        TextureType::Integral => "TextureFormatRequest::AnyIntegral",
        TextureType::Unsigned => "TextureFormatRequest::AnyUnsigned",
        TextureType::Depth => "TextureFormatRequest::AnyDepth",
        TextureType::Stencil => "TextureFormatRequest::AnyStencil",
        TextureType::DepthStencil => "TextureFormatRequest::AnyDepthStencil",
    };

    // whether this is a internally compressed texture object
    let is_compressed = matches!(ty, TextureType::Compressed | TextureType::CompressedSrgb);

    let client_format_any_ty = match ty {
        TextureType::Compressed => "ClientFormatAny::CompressedFormat",
        TextureType::CompressedSrgb => "ClientFormatAny::CompressedSrgbFormat",
        _ => "ClientFormatAny::ClientFormat",
    };

    let mipmaps_option_ty = match ty {
        TextureType::Compressed | TextureType::CompressedSrgb => "CompressedMipmapsOption",
        _ => "MipmapsOption",
    };

    // mipmaps take memory and time to generate, so they are only created on request
    let mipmap_default = match ty {
        TextureType::Compressed | TextureType::CompressedSrgb => {
            "CompressedMipmapsOption::NoMipmap"
        }
        _ => "MipmapsOption::NoMipmap",
    };

    //
    let dimensions_parameters_input = match dimensions {
        TextureDimensions::Texture1d => "width: u32",
        TextureDimensions::Texture2d | TextureDimensions::TextureRectangle => {
            "width: u32, height: u32"
        }
        TextureDimensions::Texture2dMultisample => "width: u32, height: u32, samples: u32",
        TextureDimensions::Texture3d => "width: u32, height: u32, depth: u32",
        TextureDimensions::Texture1dArray => "width: u32, array_size: u32",
        TextureDimensions::Texture2dArray => "width: u32, height: u32, array_size: u32",
        TextureDimensions::Texture2dMultisampleArray => {
            "width: u32, height: u32, array_size: u32, samples: u32"
        }
        TextureDimensions::Cubemap => "dimension: u32",
        TextureDimensions::CubemapArray => "dimension: u32, array_size: u32",
    };

    let dimensions_parameters_passing = match dimensions {
        TextureDimensions::Texture1d => {
            "Dimensions::Texture1d { width: width }"
        },
        TextureDimensions::Texture2d => {
            "Dimensions::Texture2d { width: width, height: height }"
        },
        TextureDimensions::TextureRectangle => {
            "Dimensions::TextureRectangle { width: width, height: height }"
        },
        TextureDimensions::Texture2dMultisample => {
            "Dimensions::Texture2dMultisample { width: width, height: height, samples: samples }"
        },
        TextureDimensions::Texture3d => {
            "Dimensions::Texture3d { width: width, height: height, depth: depth }"
        },
        TextureDimensions::Texture1dArray => {
            "Dimensions::Texture1dArray { width: width, array_size: array_size }"
        },
        TextureDimensions::Texture2dArray => {
            "Dimensions::Texture2dArray { width: width, height: height, array_size: array_size }"
        },
        TextureDimensions::Texture2dMultisampleArray => {
            "Dimensions::Texture2dMultisampleArray { width: width, height: height, array_size: array_size, samples: samples }"
        },
        TextureDimensions::Cubemap => {
            "Dimensions::Cubemap { dimension: dimension }"
        },
        TextureDimensions::CubemapArray => {
            "Dimensions::CubemapArray { dimension: dimension, array_size: array_size }"
        },
    };

    // the attributes that compile the texture only at the levels that have it
    let gate = match level_gate(&ty, &dimensions) {
        Some(gate) => format!(
            "#[cfg({})]\n#[cfg_attr(docsrs, doc(cfg({})))]\n",
            gate, gate
        ),
        None => String::new(),
    };

    // writing the `use module::*;` statement
    writeln!(dest, "{}pub use self::{}::{};", gate, module_name, name).unwrap();
    write!(dest, "{}", gate).unwrap();

    // opening `mod module {`
    writeln!(dest, "
        /// Contains the implementation of `{}`.
        pub mod {} {{\
            // the list of imports we need depends on the texture type, don't bother with this
            #![allow(unused_imports)]

            use std::borrow::Cow;

            use crate::texture::any::{{self, TextureAny, TextureAnyLayer, TextureAnyMipmap}};
            use crate::texture::any::{{TextureAnyLayerMipmap, TextureAnyImage, Dimensions}};
            #[cfg(safegl_desktop)]
            use crate::texture::bindless::{{ResidentTexture, BindlessTexturesNotSupportedError}};
            use crate::texture::get_format::{{InternalFormat, InternalFormatType, GetFormatError}};
            use crate::texture::pixel_buffer::PixelBuffer;
            use crate::texture::{{TextureCreationError, Texture1dDataSource, Texture2dDataSource}};
            use crate::texture::{{Texture3dDataSource, Texture2dDataSink, MipmapsOption, CompressedMipmapsOption}};
            use crate::texture::{{RawImage1d, RawImage2d, RawImage3d, CubeLayer, UploadError}};
            use crate::ReadError;
            use crate::texture::pixel::PixelValue;

            use crate::image_format::{{ClientFormatAny, TextureFormatRequest}};
            use crate::image_format::{{UncompressedFloatFormat, UncompressedIntFormat}};
            use crate::image_format::{{CompressedFormat, DepthFormat, DepthStencilFormat, StencilFormat}};
            use crate::image_format::{{CompressedSrgbFormat, SrgbFormat, UncompressedUintFormat}};

            use crate::backend::Facade;
            use crate::uniforms::{{UniformValue, AsUniformValue, Sampler}};
            #[cfg(any(safegl_gl42, safegl_gles31))]
            use crate::uniforms::{{ImageUnit, ImageUnitError, ImageUnitFormat}};
            use crate::framebuffer;

            use crate::Rect;

            use crate::GlObject;
            use crate::TextureExt;
            use crate::TextureMipmapExt;
            use crate::gl;

    ", name, module_name).unwrap();

    // writing the struct with doc-comment
    (write!(dest, "/// ")).unwrap();
    (write!(
        dest,
        "{}",
        match dimensions {
            TextureDimensions::Texture1d
            | TextureDimensions::Texture2d
            | TextureDimensions::Texture2dMultisample
            | TextureDimensions::Texture3d
            | TextureDimensions::Cubemap => "A ",
            TextureDimensions::TextureRectangle => "A rectangle ",
            TextureDimensions::Texture1dArray
            | TextureDimensions::Texture2dArray
            | TextureDimensions::Texture2dMultisampleArray
            | TextureDimensions::CubemapArray => "An array of ",
        }
    ))
    .unwrap();
    if is_compressed {
        (write!(dest, "compressed ")).unwrap();
    }
    (write!(
        dest,
        "{}",
        match dimensions {
            TextureDimensions::Texture1d | TextureDimensions::Texture1dArray => "one-dimensional ",
            TextureDimensions::Texture2d
            | TextureDimensions::Texture2dArray
            | TextureDimensions::Texture2dMultisample
            | TextureDimensions::Texture2dMultisampleArray => {
                "two-dimensional "
            }
            // sampled with coordinates in texels, without mipmaps or repetition
            TextureDimensions::TextureRectangle => "",
            TextureDimensions::Texture3d => "three-dimensional ",
            TextureDimensions::Cubemap | TextureDimensions::CubemapArray => "cube ",
        }
    ))
    .unwrap();
    (write!(
        dest,
        "{}",
        match dimensions {
            TextureDimensions::Texture1d
            | TextureDimensions::Texture2d
            | TextureDimensions::TextureRectangle
            | TextureDimensions::Texture2dMultisample
            | TextureDimensions::Texture3d
            | TextureDimensions::Cubemap => "texture ",
            TextureDimensions::Texture1dArray
            | TextureDimensions::Texture2dArray
            | TextureDimensions::Texture2dMultisampleArray
            | TextureDimensions::CubemapArray => "textures ",
        }
    ))
    .unwrap();
    (write!(
        dest,
        "{}",
        match ty {
            TextureType::Regular | TextureType::Compressed => " containing floating-point data",
            TextureType::Srgb | TextureType::CompressedSrgb =>
                " containing sRGB floating-point data",
            TextureType::Integral => " containing signed integral data",
            TextureType::Unsigned => " containing unsigned integral data",
            TextureType::Depth => " containing depth data",
            TextureType::Stencil => " containing stencil data",
            TextureType::DepthStencil => " containing both depth and stencil data",
        }
    ))
    .unwrap();
    (writeln!(dest, ".")).unwrap();
    (writeln!(dest, "pub struct {}(TextureAny);", name)).unwrap();

    // `GlObject` trait impl
    (writeln!(
        dest,
        "
                impl GlObject for {} {{
                    type Id = glow::Texture;

                    #[inline]
                    fn get_id(&self) -> glow::Texture {{
                        self.0.get_id()
                    }}
                }}
            ",
        name
    ))
    .unwrap();

    // `Debug` trait impl
    (writeln!(
        dest,
        "
                impl ::std::fmt::Debug for {} {{
                    #[inline]
                    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error>
                    {{
                        self.0.fmt(f)
                    }}
                }}
            ",
        name
    ))
    .unwrap();

    // 'Deref' impl to common type.
    (writeln!(
        dest,
        "
                impl ::std::ops::Deref for {} {{
                    type Target = TextureAny;

                    #[inline]
                    fn deref<'a>(&'a self) -> &'a TextureAny {{
                        &self.0
                    }}
                }}
            ",
        name
    ))
    .unwrap();

    // `UniformValue` trait impl for samplers; rectangle textures can't repeat or have mipmaps, so
    // their samplers clamp them and filter them without mipmaps by default
    let default_sampler = if dimensions == TextureDimensions::TextureRectangle {
        "crate::uniforms::SamplerBehavior {
            wrap_function: (
                crate::uniforms::SamplerWrapFunction::Clamp,
                crate::uniforms::SamplerWrapFunction::Clamp,
                crate::uniforms::SamplerWrapFunction::Clamp,
            ),
            minify_filter: crate::uniforms::MinifySamplerFilter::Linear,
            ..Default::default()
        }"
    } else {
        "Default::default()"
    };
    {
        match ty {
            TextureType::Regular
            | TextureType::Compressed
            | TextureType::Srgb
            | TextureType::CompressedSrgb
            | TextureType::Integral
            | TextureType::Unsigned
            | TextureType::Depth
            | TextureType::Stencil
            | TextureType::DepthStencil => {
                (writeln!(dest, "
                            impl<'a> AsUniformValue for &'a {myname} {{
                                #[inline]
                                fn as_uniform_value(&self) -> UniformValue<'_> {{
                                    UniformValue::Texture(&***self, None)
                                }}
                            }}

                            impl AsUniformValue for {myname} {{
                                #[inline]
                                fn as_uniform_value(&self) -> UniformValue<'_> {{
                                    UniformValue::Texture(&**self, None)
                                }}
                            }}

                            impl<'a> AsUniformValue for Sampler<'a, {myname}> {{
                                #[inline]
                                fn as_uniform_value(&self) -> UniformValue<'_> {{
                                    UniformValue::Texture(&**self.0, Some(self.1))
                                }}
                            }}

                            /// Turns the sampler into a uniform value that borrows the texture
                            /// rather than the sampler, so that it can outlive the sampler.
                            impl<'a> From<Sampler<'a, {myname}>> for UniformValue<'a> {{
                                #[inline]
                                fn from(sampler: Sampler<'a, {myname}>) -> UniformValue<'a> {{
                                    UniformValue::Texture(&**sampler.0, Some(sampler.1))
                                }}
                            }}

                            impl {myname} {{
                                /// Builds a `Sampler` marker object that allows you to indicate
                                /// how the texture should be sampled from inside a shader.
                                ///
                                /// # Example
                                ///
                                /// ```no_run
                                /// # #[macro_use] extern crate safegl;
                                /// # fn example(texture: safegl::texture::Texture2d) {{
                                /// let uniforms = uniform! {{
                                ///     color_texture: texture.sampled().magnify_filter(safegl::uniforms::MagnifySamplerFilter::Nearest)
                                /// }};
                                /// # }}
                                /// ```
                                #[inline]
                                pub fn sampled(&self) -> Sampler<'_, {myname}> {{
                                    Sampler(self, {default_sampler})
                                }}
                            }}
                        ", myname = name, default_sampler = default_sampler)).unwrap();
            }
        }
    }

    // Generate implementations of image_unit: multisample images (`image2DMS`) only exist on
    // desktop OpenGL
    if ty == TextureType::Regular || ty == TextureType::Integral || ty == TextureType::Unsigned {
        let gate = if dimensions.is_multisample() {
            "safegl_gl42"
        } else {
            "any(safegl_gl42, safegl_gles31)"
        };
        writeln!(dest, "
                    #[cfg({gate})]
                    impl<'a> AsUniformValue for ImageUnit<'a, {myname}> {{
                        #[inline]
                        fn as_uniform_value(&self) -> UniformValue<'_>  {{
                            UniformValue::Image(&**self.0, self.1)
                        }}
                    }}

                    #[cfg({gate})]
                    impl {myname} {{
                        /// Builds an image unit marker object that allows you to indicate how the
                        /// texture should be bound to an image unit.
                        #[cfg_attr(docsrs, doc(cfg({gate})))]
                        #[inline]
                        pub fn image_unit(&self, format: ImageUnitFormat) -> Result<ImageUnit<'_, {myname}>, ImageUnitError> {{
                             ImageUnit::new(self, format)
                        }}
                    }}
                ", myname = name, gate = gate).unwrap();
    }

    // `ToXXXAttachment` trait impl
    if dimensions == TextureDimensions::Texture2d
        || dimensions == TextureDimensions::TextureRectangle
        || dimensions == TextureDimensions::Texture2dMultisample
        || dimensions == TextureDimensions::Texture1d
    {
        match ty {
            TextureType::Regular
            | TextureType::Srgb
            | TextureType::Integral
            | TextureType::Unsigned => {
                (writeln!(dest, "
                        impl<'t> crate::framebuffer::ToColorAttachment<'t> for &'t {name} {{
                            #[inline]
                            fn to_color_attachment(self) -> crate::framebuffer::ColorAttachment<'t> {{
                                crate::framebuffer::ColorAttachment::Texture(self.0.main_level().first_layer().into_image(None).unwrap())
                            }}
                        }}
                    ", name = name)).unwrap();
            }
            TextureType::Depth => {
                (writeln!(dest, "
                        impl<'t> crate::framebuffer::ToDepthAttachment<'t> for &'t {name} {{
                            #[inline]
                            fn to_depth_attachment(self) -> crate::framebuffer::DepthAttachment<'t> {{
                                crate::framebuffer::DepthAttachment::Texture(self.0.main_level().first_layer().into_image(None).unwrap())
                            }}
                        }}
                    ", name = name)).unwrap();
            }
            TextureType::Stencil => {
                (writeln!(dest, "
                        impl<'t> crate::framebuffer::ToStencilAttachment<'t> for &'t {name} {{
                            #[inline]
                            fn to_stencil_attachment(self) -> crate::framebuffer::StencilAttachment<'t> {{
                                crate::framebuffer::StencilAttachment::Texture(self.0.main_level().first_layer().into_image(None).unwrap())
                            }}
                        }}
                    ", name = name)).unwrap();
            }
            TextureType::DepthStencil => {
                (writeln!(dest, "
                        impl<'t> crate::framebuffer::ToDepthStencilAttachment<'t> for &'t {name} {{
                            #[inline]
                            fn to_depth_stencil_attachment(self) -> crate::framebuffer::DepthStencilAttachment<'t> {{
                                crate::framebuffer::DepthStencilAttachment::Texture(self.0.main_level().first_layer().into_image(None).unwrap())
                            }}
                        }}
                    ", name = name)).unwrap();
            }
            _ => (),
        }
    }

    // opening `impl Texture` block
    (writeln!(dest, "impl {} {{", name)).unwrap();

    // writing `get_internal_format`
    writeln!(
        dest,
        "
            /// Determines the internal format of this texture.
            ///
            /// The backend may not support querying the actual format, in which case an error
            /// is returned.
            #[inline]
            pub fn get_internal_format(&self) -> Result<InternalFormat, GetFormatError> {{
                self.0.get_internal_format()
            }}
        "
    )
    .unwrap();

    // writing the `new` function
    if !dimensions.is_multisample() && !dimensions.is_cube() {
        let param = match dimensions {
            TextureDimensions::Texture1d
            | TextureDimensions::Texture2d
            | TextureDimensions::TextureRectangle
            | TextureDimensions::Texture3d => "T",

            TextureDimensions::Texture1dArray | TextureDimensions::Texture2dArray => "Vec<T>",

            _ => unreachable!(),
        };

        let gen_doc = if is_compressed {
            "/// No mipmap level (except for the main level) will be allocated nor generated."
        } else {
            "/// No mipmap level (except for the main level) will be allocated nor generated: use \
             `with_mipmaps(facade, data, MipmapsOption::AutoGeneratedMipmaps)` to have them."
        };

        (writeln!(
            dest,
            "
                /// Builds a new texture by uploading data.
                ///
                {gen_doc}
                #[inline]
                pub fn new<'a, F: ?Sized, T>(facade: &F, data: {param})
                              -> Result<{name}, TextureCreationError>
                              where T: {data_source_trait}<'a>, F: Facade
                {{
                    {name}::new_impl(facade, data, None, {mipmap_default})
                }}
            ",
            data_source_trait = data_source_trait,
            param = param,
            name = name,
            mipmap_default = mipmap_default,
            gen_doc = gen_doc
        ))
        .unwrap();
    }

    // writing the `with_mipmaps` function
    if !dimensions.is_multisample() && !dimensions.is_cube() {
        let param = match dimensions {
            TextureDimensions::Texture1d
            | TextureDimensions::Texture2d
            | TextureDimensions::TextureRectangle
            | TextureDimensions::Texture3d => "T",

            TextureDimensions::Texture1dArray | TextureDimensions::Texture2dArray => "Vec<T>",

            _ => unreachable!(),
        };

        (writeln!(
            dest,
            "
                /// Builds a new texture by uploading data.
                #[inline]
                pub fn with_mipmaps<'a, F: ?Sized, T>(facade: &F, data: {param}, mipmaps: {mipmaps})
                                              -> Result<{name}, TextureCreationError>
                                              where T: {data_source_trait}<'a>, F: Facade
                {{
                    {name}::new_impl(facade, data, None, mipmaps)
                }}
            ",
            data_source_trait = data_source_trait,
            param = param,
            name = name,
            mipmaps = mipmaps_option_ty
        ))
        .unwrap();
    }

    // writing the `with_compressed_data` function
    if is_compressed && !dimensions.is_multisample() && !dimensions.is_cube() {
        let param = match dimensions {
            TextureDimensions::Texture1d
            | TextureDimensions::Texture2d
            | TextureDimensions::TextureRectangle
            | TextureDimensions::Texture3d => "&[u8]",

            TextureDimensions::Texture1dArray | TextureDimensions::Texture2dArray => "Vec<&[u8]>",

            _ => unreachable!(),
        };

        (writeln!(dest, "
                /// Builds a new texture with a specific format. The input data must also be of the
                /// specified compressed format.
                #[inline]
                pub fn with_compressed_data<F: ?Sized>(facade: &F, data: {param}, {dim_params},
                                                      format: {format}, mipmaps: {mipmaps})
                                                      -> Result<{name}, TextureCreationError>
                                                       where F: Facade
                {{
                    let data = Cow::Borrowed(data.as_ref());
                    let client_format = {client_format_any}(format);
                    Ok({name}(any::new_texture(facade, {default_format}, Some((client_format, data)),
                                                    mipmaps.into(), {dim_params_passing})?))
                }}
            ", dim_params = dimensions_parameters_input, dim_params_passing = dimensions_parameters_passing,
               param = param, client_format_any = client_format_any_ty,
               name = name, format = relevant_format, default_format = default_format,
               mipmaps = mipmaps_option_ty).unwrap());
    }

    // writing the `with_format` function
    if !dimensions.is_multisample() && !dimensions.is_cube() {
        let param = match dimensions {
            TextureDimensions::Texture1d
            | TextureDimensions::Texture2d
            | TextureDimensions::TextureRectangle
            | TextureDimensions::Texture3d => "T",

            TextureDimensions::Texture1dArray | TextureDimensions::Texture2dArray => "Vec<T>",

            _ => unreachable!(),
        };

        (writeln!(
            dest,
            "
                /// Builds a new texture with a specific format.
                #[inline]
                pub fn with_format<'a, F: ?Sized, T>(facade: &F, data: {param},
                                          format: {format}, mipmaps: {mipmaps})
                                          -> Result<{name}, TextureCreationError>
                                          where T: {data_source_trait}<'a>, F: Facade
                {{
                    {name}::new_impl(facade, data, Some(format), mipmaps)
                }}
            ",
            data_source_trait = data_source_trait,
            param = param,
            format = relevant_format,
            name = name,
            mipmaps = mipmaps_option_ty
        ))
        .unwrap();
    }

    // writing the `new_impl` function
    if !dimensions.is_multisample() && !dimensions.is_cube() {
        let param = match dimensions {
            TextureDimensions::Texture1d
            | TextureDimensions::Texture2d
            | TextureDimensions::TextureRectangle
            | TextureDimensions::Texture3d => "T",

            TextureDimensions::Texture1dArray | TextureDimensions::Texture2dArray => "Vec<T>",

            _ => unreachable!(),
        };

        (writeln!(
            dest,
            "
                #[inline]
                fn new_impl<'a, F: ?Sized, T>(facade: &F, data: {param},
                                   format: Option<{relevant_format}>, mipmaps: {mipmaps})
                                   -> Result<{name}, TextureCreationError>
                                   where T: {data_source_trait}<'a>, F: Facade
                {{
            ",
            data_source_trait = data_source_trait,
            param = param,
            name = name,
            relevant_format = relevant_format,
            mipmaps = mipmaps_option_ty
        ))
        .unwrap();

        // writing the `let format = ...` line
        (write!(
            dest,
            "let format = format.map(|f| {{
                           TextureFormatRequest::Specific(f.to_texture_format())
                       }}).unwrap_or({});",
            default_format
        ))
        .unwrap();

        match dimensions {
            TextureDimensions::Texture1d => (write!(dest, "
                    let RawImage1d {{ data, width, format: client_format }} = data.into_raw();
                ")).unwrap(),

            TextureDimensions::Texture2d | TextureDimensions::TextureRectangle => (write!(dest, "
                    let RawImage2d {{ data, width, height, format: client_format }} =
                                            data.into_raw();
                ")).unwrap(),

            TextureDimensions::Texture3d => (write!(dest, "
                    let RawImage3d {{ data, width, height, depth, format: client_format }} =
                                            data.into_raw();
                ")).unwrap(),

            TextureDimensions::Texture1dArray => (write!(dest, "
                    let vec_raw = data.into_iter().map(|e| e.into_raw()).collect();
                    let RawImage2d {{data, width, height: array_size, format: client_format }} = RawImage2d::from_vec_raw1d(&vec_raw);
                ")).unwrap(),   // TODO: panic if dimensions are inconsistent

            TextureDimensions::Texture2dArray => (write!(dest, "
                    let vec_raw = data.into_iter().map(|e| e.into_raw()).collect();
                    let RawImage3d {{data, width, height, depth: array_size, format: client_format }} = RawImage3d::from_vec_raw2d(&vec_raw);
                ")).unwrap(),   // TODO: panic if dimensions are inconsistent

            _ => unreachable!()
        }

        (write!(
            dest,
            "let client_format = ClientFormatAny::ClientFormat(client_format);"
        ))
        .unwrap();

        // writing the constructor
        (write!(
            dest,
            "Ok({}(any::new_texture(facade, format, \
                       Some((client_format, data)), mipmaps.into(), {}",
            name, dimensions_parameters_passing
        ))
        .unwrap();
        (writeln!(dest, ")?))")).unwrap();

        // end of "new" function block
        (writeln!(dest, "}}")).unwrap();
    }

    // writing the `empty` function
    if !is_compressed {
        // opening function
        (writeln!(
            dest,
            "
                /// Creates an empty texture.
                ///
                /// No mipmap level (except for the main level) will be allocated or generated.
                ///
                /// The texture will contain undefined data.
                #[inline]
                pub fn empty<F: ?Sized>(facade: &F, {dim_params})
                                -> Result<{name}, TextureCreationError>
                                where F: Facade
                {{
                    let format = {format};
            ",
            format = default_format,
            dim_params = dimensions_parameters_input,
            name = name,
        ))
        .unwrap();

        // writing the constructor
        (write!(dest, "any::new_texture::<_, u8>(facade, format, None, {mipmap}::NoMipmap.into(), {}).map(|t| {}(t))",
                dimensions_parameters_passing, name, mipmap = mipmaps_option_ty)).unwrap();

        // closing function
        (writeln!(dest, "}}")).unwrap();
    }

    // writing the `empty_with_format` function
    if true {
        // opening function
        (writeln!(dest, "
                /// Creates an empty texture with a specific format.
                ///
                /// The texture (and its mipmaps) will contain undefined data.
                #[inline]
                pub fn empty_with_format<F: ?Sized>(facade: &F, format: {format}, mipmaps: {mipmaps}, {dim_params}) -> Result<{name}, TextureCreationError> where F: Facade {{
                    let format = format.to_texture_format();
                    let format = TextureFormatRequest::Specific(format);
            ", format = relevant_format, dim_params = dimensions_parameters_input, name = name,
               mipmaps = mipmaps_option_ty)).unwrap();

        // writing the constructor
        (write!(
            dest,
            "let t = any::new_texture::<_, u8>(facade, format, None, mipmaps.into(), {});",
            dimensions_parameters_passing
        ))
        .unwrap();
        (writeln!(
            dest,
            "
            t.map(|t| {}(t))",
            name
        ))
        .unwrap();

        // closing function
        (writeln!(dest, "}}")).unwrap();
    }

    // writing the `view` function
    {
        let bind_point = match dimensions {
            TextureDimensions::Texture1d => "gl::TEXTURE_1D",
            TextureDimensions::Texture2d => "gl::TEXTURE_2D",
            TextureDimensions::TextureRectangle => "gl::TEXTURE_RECTANGLE",
            TextureDimensions::Texture2dMultisample => "gl::TEXTURE_2D_MULTISAMPLE",
            TextureDimensions::Texture3d => "gl::TEXTURE_3D",
            TextureDimensions::Texture1dArray => "gl::TEXTURE_1D_ARRAY",
            TextureDimensions::Texture2dArray => "gl::TEXTURE_2D_ARRAY",
            TextureDimensions::Texture2dMultisampleArray => "gl::TEXTURE_2D_MULTISAMPLE_ARRAY",
            TextureDimensions::Cubemap => "gl::TEXTURE_CUBE_MAP",
            TextureDimensions::CubemapArray => "gl::TEXTURE_CUBE_MAP_ARRAY",
        };
        (writeln!(dest, "
                /// Builds a texture that shares the storage of the levels `levels` and of the
                /// layers `layers` of `source` (`glTextureView`): writes through one are seen
                /// through the other. The layers of cube maps are their faces, in the order of
                /// `CubeLayer`.
                ///
                /// The view interprets the texels with `format`, which must have the size of
                /// the format of the source, as listed by the view classes of OpenGL (for
                /// instance `U8U8U8U8` as `U32` or as an sRGB format), or be the same.
                /// Its level 0 is the level `levels.start` of the source. Views can be made of
                /// views.
                ///
                /// Needs OpenGL 4.3 or `GL_ARB_texture_view`, and a source with immutable storage
                /// that is a texture of a compatible kind: two-dimensional textures, their arrays,
                /// cube maps and cube map arrays view each other, as do one-dimensional textures and
                /// their arrays, and multisample textures and their arrays; 3D and rectangle
                /// textures only view textures of their kind.
                #[cfg(safegl_gl43)]
                #[cfg_attr(docsrs, doc(cfg(safegl_gl43)))]
                pub fn view<S>(source: &S, format: {format}, levels: std::ops::Range<u32>,
                               layers: std::ops::Range<u32>)
                               -> Result<{name}, crate::texture::TextureViewError>
                    where S: std::ops::Deref<Target = TextureAny>
                {{
                    let format = TextureFormatRequest::Specific(format.to_texture_format());
                    any::new_texture_view(source, {bind_point}, format, levels, layers).map({name})
                }}
            ", format = relevant_format, name = name, bind_point = bind_point)).unwrap();
    }

    // writing the `empty_with_sample_locations` function of multisample textures
    if dimensions.is_multisample() {
        (writeln!(dest, "
                /// Creates an empty multisample texture with a specific format, whose samples are
                /// at the same positions in every texel if `fixed_sample_locations` is true, as
                /// with the other constructors, or may be placed differently in each texel
                /// otherwise (`GL_TEXTURE_FIXED_SAMPLE_LOCATIONS`), which some implementations
                /// use for better antialiasing.
                ///
                /// The attachments of a framebuffer must all have fixed sample locations or all
                /// have variable ones, and render buffers have fixed ones; otherwise creating the
                /// framebuffer returns `ValidationError::Incomplete`.
                #[inline]
                pub fn empty_with_sample_locations<F: ?Sized>(facade: &F, format: {format}, {dim_params}, fixed_sample_locations: bool) -> Result<{name}, TextureCreationError> where F: Facade {{
                    let format = TextureFormatRequest::Specific(format.to_texture_format());
                    any::new_texture_with_sample_locations::<_, u8>(facade, format, None, {mipmap}::NoMipmap.into(), {dim_passing}, fixed_sample_locations)
                        .map(|t| {name}(t))
                }}
            ", format = relevant_format, dim_params = dimensions_parameters_input, name = name,
               mipmap = mipmaps_option_ty, dim_passing = dimensions_parameters_passing)).unwrap();
    }

    // writing the `empty_with_mipmaps` function
    if !is_compressed {
        // opening function
        (writeln!(dest, "
                /// Creates an empty texture. Specifies whether is has mipmaps.
                ///
                /// The texture (and its mipmaps) will contain undefined data.
                #[inline]
                pub fn empty_with_mipmaps<F: ?Sized>(facade: &F, mipmaps: {mipmaps}, {dim_params}) -> Result<{name}, TextureCreationError> where F: Facade {{
                    let format = {format};
            ", format = default_format, dim_params = dimensions_parameters_input, name = name,
               mipmaps = mipmaps_option_ty)).unwrap();

        // writing the constructor
        (write!(
            dest,
            "any::new_texture::<_, u8>(facade, format, None, mipmaps.into(), {})",
            dimensions_parameters_passing
        ))
        .unwrap();
        (writeln!(dest, ".map(|t| {}(t))", name)).unwrap();

        // closing function
        (writeln!(dest, "}}")).unwrap();
    }

    // writing the 'from_id' function
    (writeln!(dest, "
                /// Builds a new texture reference from an existing, externally created OpenGL texture.
                /// If `owned` is true, this reference will take ownership of the texture and be responsible
                /// for cleaning it up. Otherwise, the texture must be cleaned up externally, but only
                /// after this reference's lifetime has ended.
                pub unsafe fn from_id<F: Facade + ?Sized>(facade: &F,
                                                 format: {format},
                                                 id: glow::Texture,
                                                 owned: bool,
                                                 mipmap: MipmapsOption,
                                                 ty: Dimensions)
                                                 -> {name} {{
                    let format = format.to_texture_format();
                    let format = TextureFormatRequest::Specific(format);
                    {name}(any::from_id(facade, format, id, owned, mipmap, ty))
                }}
        ", format = relevant_format, name = name)).unwrap();

    // dimensions getters
    write_dimensions_getters(dest, dimensions, "self.0", true);

    // writing the `as_surface` function
    if (dimensions == TextureDimensions::Texture2d
        || dimensions == TextureDimensions::TextureRectangle
        || dimensions == TextureDimensions::Texture2dMultisample)
        && (ty == TextureType::Regular
            || ty == TextureType::Integral
            || ty == TextureType::Unsigned)
    {
        (write!(
            dest,
            "
                /// Starts drawing on the texture.
                ///
                /// All the function calls to the framebuffer will draw on the texture instead
                /// of the screen.
                ///
                /// ## Low-level information
                ///
                /// The first time that this function is called, a FrameBuffer Object will be
                /// created and cached. The following calls to `as_surface` will load the existing
                /// FBO and re-use it. When the texture is destroyed, the FBO is destroyed too.
                ///
                #[inline]
                pub fn as_surface<'a>(&'a self) -> framebuffer::SimpleFrameBuffer<'a> {{
                    self.sync_shader_writes_for_surface();
                    framebuffer::SimpleFrameBuffer::new(self.0.get_context(), self).unwrap()
                }}

                /// Call this function if you write to this texture in a shader with Image Store
                /// operations, and you want to use the texture as a Surface or Framebuffer Object.
                ///
                /// This function issues a memory barrier if the texture has been written to in a
                /// shader via Image Store operations.
                #[inline]
                pub fn sync_shader_writes_for_surface(&self) {{
                    use crate::ContextExt;
                    let mut ctxt = self.0.get_context().make_current();
                    self.0.prepare_for_access(&mut ctxt, crate::TextureAccess::Framebuffer);
                }}
            "
        ))
        .unwrap();
    }

    // writing the `get_mipmap_levels` function
    (write!(
        dest,
        "
            /// Returns the number of mipmap levels of the texture.
            ///
            /// The minimum value is 1, since there is always a main texture.
            #[inline]
            pub fn get_mipmap_levels(&self) -> u32 {{
                self.0.get_mipmap_levels()
            }}
        "
    ))
    .unwrap();

    // writing the `read` function of depth textures
    if (dimensions == TextureDimensions::Texture2d
        || dimensions == TextureDimensions::TextureRectangle) && ty == TextureType::Depth {
        (write!(
            dest,
            r#"
                /// Reads the depth values of the texture to RAM.
                ///
                /// You should avoid doing this at all cost during performance-critical
                /// operations (for example, while you're drawing).
                ///
                /// Returns `ReadError::AttachmentTypeNotSupported` on OpenGL ES without
                /// `GL_NV_read_depth`, which can't read depth values, and
                /// `ReadError::ContextLost` if the context has been lost.
                #[inline]
                pub fn read<T>(&self) -> Result<T, ReadError> where T: Texture2dDataSink<f32> {{
                    self.0.main_level().read_2d()
                }}
            "#
        ))
        .unwrap();
    }

    // writing the `read` functions of 2D textures; the levels of every texture are read with
    // `read_volume` on desktop OpenGL
    let is_2d = dimensions == TextureDimensions::Texture2d
        || dimensions == TextureDimensions::TextureRectangle;
    if is_2d
        && (ty == TextureType::Regular
            || ty == TextureType::Srgb
            || ty == TextureType::Unsigned
            || is_compressed)
    {
        (write!(
            dest,
            r#"
                /// Reads the content of the texture to RAM. This method may only read `U8U8U8U8`
                /// data, as it is the only format guaranteed to be supported across all OpenGL
                /// versions.
                ///
                /// You should avoid doing this at all cost during performance-critical
                /// operations (for example, while you're drawing).
                /// Use `read_to_pixel_buffer` instead.
                ///
                /// Returns `ReadError::ContextLost` if the context has been lost.
                #[inline]
                pub fn read<T>(&self) -> Result<T, ReadError> where T: Texture2dDataSink<(u8, u8, u8, u8)> {{
                    self.read_as()
                }}
            "#
        ))
        .unwrap();

        (write!(
            dest,
            r#"
                /// Reads the content of the texture into a buffer in video memory. This method may
                /// only read `U8U8U8U8` data, as it is the only format guaranteed to be supported
                /// across all OpenGL versions.
                ///
                /// This operation copies the texture's data into a buffer in video memory
                /// (a pixel buffer). Contrary to the `read` function, this operation is
                /// done asynchronously and doesn't need a synchronization.
                #[inline]
                pub fn read_to_pixel_buffer(&self) -> Result<PixelBuffer<(u8, u8, u8, u8)>, ReadError> {{
                    self.read_to_pixel_buffer_as()
                }}
            "#
        ))
        .unwrap();
    }

    // signed integers have no `(u8, u8, u8, u8)` reads, which are of normalized values
    if is_2d
        && (ty == TextureType::Regular
            || ty == TextureType::Srgb
            || ty == TextureType::Unsigned
            || ty == TextureType::Integral
            || is_compressed)
    {

        (write!(dest, r#"
                /// Reads the content of the texture to RAM in the pixel format `P`, for example
                /// `(f32, f32, f32, f32)` to read the values of a floating-point texture exactly.
                ///
                /// You should avoid doing this at all cost during performance-critical
                /// operations (for example, while you're drawing).
                /// Use `read_to_pixel_buffer_as` instead.
                ///
                /// Desktop OpenGL reads every format (`glGetTexImage`), compressed ones
                /// decompressed. On OpenGL ES, which reads through a framebuffer and can only
                /// read one pixel format for each kind of texture, the pixels are converted on
                /// the CPU. Returns `ReadError::OutputFormatNotSupported` if `P` can't hold the
                /// values of the texture, like floats for an integral texture, and
                /// `ReadError::NotRenderable` on OpenGL ES for formats it can't render to.
                #[inline]
                pub fn read_as<T, P>(&self) -> Result<T, ReadError> where T: Texture2dDataSink<P>, P: PixelValue {{
                    self.0.main_level().read_2d()
                }}
            "#)).unwrap();

        (write!(dest, r#"
                /// Reads the content of the texture into a buffer in video memory, in the pixel
                /// format `P`.
                ///
                /// This operation copies the texture's data into a buffer in video memory
                /// (a pixel buffer). Contrary to the `read` function, this operation is
                /// done asynchronously and doesn't need a synchronization, except on OpenGL ES
                /// when `P` isn't the format that it reads: the pixels are then converted on the
                /// CPU.
                #[inline]
                pub fn read_to_pixel_buffer_as<P>(&self) -> Result<PixelBuffer<P>, ReadError> where P: PixelValue {{
                    self.0.main_level().read_2d_to_pixel_buffer()
                }}
            "#)).unwrap();
    }

    // writing the `read_compressed_data` function
    if is_compressed && !dimensions.is_array() {
        (write!(
            dest,
            r#"
                /// Reads the content of the texture to RAM without decompressing it before.
                ///
                /// You should avoid doing this at all cost during performance-critical
                /// operations (for example, while you're drawing).
                ///
                /// Returns the compressed format of the texture and the compressed data, gives
                /// `None` when the internal compression format is generic or unknown.
                #[inline]
                pub fn read_compressed_data(&self) -> Option<({format}, Vec<u8>)> {{
                    self.main_level().read_compressed_data()
                }}
            "#,
            format = relevant_format
        ))
        .unwrap();
    }

    // the types whose uncompressed data can be uploaded with `write`; stencil and depth-stencil
    // data have no client format
    let is_writable = matches!(
        ty,
        TextureType::Regular
            | TextureType::Srgb
            | TextureType::Integral
            | TextureType::Unsigned
            | TextureType::Depth
            | TextureType::Stencil
            | TextureType::DepthStencil
    );

    // writing the `write` function of one-dimensional and three-dimensional textures
    if is_writable && dimensions == TextureDimensions::Texture1d {
        (write!(
            dest,
            r#"
                /// Uploads some data in the texture, starting at the texel `offset`.
                ///
                /// Note that this may cause a synchronization if you use the texture right before
                /// or right after this call.
                ///
                /// Returns an error if the data doesn't fit in the texture, or if its size or
                /// format doesn't match.
                #[inline]
                pub fn write<'a, T>(&self, offset: u32, data: T) -> Result<(), UploadError>
                    where T: Texture1dDataSource<'a>
                {{
                    self.main_level().write(offset, data)
                }}
            "#
        ))
        .unwrap();
    }

    if is_writable && dimensions == TextureDimensions::Texture3d {
        (write!(
            dest,
            r#"
                /// Uploads some data in the texture, starting at the texel `offset` (x, y, z).
                ///
                /// Note that this may cause a synchronization if you use the texture right before
                /// or right after this call.
                ///
                /// Returns an error if the data doesn't fit in the texture, or if its size or
                /// format doesn't match.
                #[inline]
                pub fn write<'a, T>(&self, offset: (u32, u32, u32), data: T) -> Result<(), UploadError>
                    where T: Texture3dDataSource<'a>
                {{
                    self.main_level().write(offset, data)
                }}
            "#
        ))
        .unwrap();
    }

    // writing the `write` function
    if (dimensions == TextureDimensions::Texture2d
        || dimensions == TextureDimensions::TextureRectangle) && (is_writable || is_compressed) {
        let compressed_restrictions = if is_compressed {
            r#" ///
                /// For compressed textures, the region must start on the blocks of the format
                /// (4 × 4 texels) and end on them or on the edge of the image, otherwise
                /// `UploadError::MisalignedBlocks` is returned.
            "#
        } else {
            ""
        };

        (write!(
            dest,
            r#"
                /// Uploads some data in the texture.
                ///
                /// Note that this may cause a synchronization if you use the texture right before
                /// or right after this call. Prefer creating a whole new texture if you change a
                /// huge part of it.
                ///
                /// Returns an error if the dimensions of `data` don't match the `Rect`, if the
                /// `Rect` is out of the texture, or if the size or format of the data doesn't match.
                {compressed_restrictions}
                #[inline]
                pub fn write<'a, T>(&self, rect: Rect, data: T) -> Result<(), UploadError>
                    where T: {data_source_trait}<'a>
                {{
                    self.main_level().write(rect, data)
                }}
            "#,
            data_source_trait = data_source_trait,
            compressed_restrictions = compressed_restrictions
        ))
        .unwrap();
    }

    // writing the `write_compressed_data` function
    // TODO: implement for other types too
    if dimensions == TextureDimensions::Texture2d && is_compressed {
        (write!(dest, r#"
                /// Uploads some data in the texture by using a compressed format as input.
                ///
                /// Note that this may cause a synchronization if you use the texture right before
                /// or right after this call. Prefer creating a whole new texture if you change a
                /// huge part of it.
                ///
                /// Returns an error if the dimensions of `data` don't match the `Rect`, or if
                /// the size of the data doesn't match its format.
                ///
                /// The contents of any texel outside the region modified by the call are
                /// undefined.
                #[inline]
                pub fn write_compressed_data(&self, rect: Rect, data: &[u8],
                                             width: u32, height: u32, format: {format})
                                             -> Result<(), UploadError>
                {{
                    // FIXME is having width and height as parameter redundant as rect kinda of
                    // already provides them?
                    self.main_level().write_compressed_data(rect, data, width, height, format)
                }}
            "#, format = relevant_format)).unwrap();
    }

    // `resident_if_supported`
    (write!(
        dest,
        r#"
            /// Turns the texture into a `ResidentTexture`.
            ///
            /// This allows you to use the texture in a much more efficient way by storing
            /// a "reference to it" in a buffer (actually not a reference but a raw pointer).
            ///
            /// See the documentation of `ResidentTexture` for more infos.
            #[cfg(safegl_desktop)]
            #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
            #[inline]
            pub fn resident(self) -> Result<ResidentTexture, BindlessTexturesNotSupportedError> {{
                ResidentTexture::new(self.0)
            }}
        "#
    ))
    .unwrap();

    // writing the layer & mipmap access functions
    if dimensions.is_array() {
        (write!(
            dest,
            r#"
                /// Access the first layer of this texture.
                #[inline]
                pub fn first_layer(&self) -> {name}Layer<'_> {{
                    self.layer(0).unwrap()
                }}

                /// Access a single layer of this texture.
                #[inline]
                pub fn layer(&self, layer: u32) -> Option<{name}Layer<'_>> {{
                    self.0.layer(layer).map(|l| {name}Layer(l, self))
                }}
            "#,
            name = name
        ))
        .unwrap();
    }

    (write!(
        dest,
        r#"
            /// Access a single mipmap level of this texture.
            #[inline]
            pub fn mipmap(&self, level: u32) -> Option<{name}Mipmap<'_>> {{
                self.0.mipmap(level).map(|m| {name}Mipmap(m, self))
            }}

            /// Access the main mipmap level of this texture.
            #[inline]
            pub fn main_level(&self) -> {name}Mipmap<'_> {{
                self.mipmap(0).unwrap()
            }}
        "#,
        name = name
    ))
    .unwrap();

    // closing `impl Texture` block
    (writeln!(dest, "}}")).unwrap();

    // the `Layer` struct
    if dimensions.is_array() {
        // writing the struct
        (write!(
            dest,
            r#"
                /// Represents a single layer of a `{name}`.
                ///
                /// Can be obtained by calling `{name}::layer()`.
                #[derive(Copy, Clone)]
                pub struct {name}Layer<'t>(TextureAnyLayer<'t>, &'t {name});
            "#,
            name = name
        ))
        .unwrap();

        // opening `impl Layer` block
        (writeln!(dest, "impl<'t> {}Layer<'t> {{", name)).unwrap();

        // dimensions getters
        write_dimensions_getters(dest, dimensions, "(self.1).0", false);

        // writing the `get_layer` and `get_texture` functions
        (write!(
            dest,
            "
                /// Returns the corresponding texture.
                #[inline]
                pub fn get_texture(&self) -> &'t {name} {{
                    &self.1
                }}

                /// Returns the layer index.
                #[inline]
                pub fn get_layer(&self) -> u32 {{
                    self.0.get_layer()
                }}
            ",
            name = name
        ))
        .unwrap();

        // writing the `get_mipmap_levels` function
        (write!(
            dest,
            "
                /// Returns the number of mipmap levels of the texture.
                ///
                /// The minimum value is 1, since there is always a main texture.
                #[inline]
                pub fn get_mipmap_levels(&self) -> u32 {{
                    self.0.get_texture().get_mipmap_levels()
                }}
            "
        ))
        .unwrap();

        // writing the layer & mipmap access functions
        (write!(
            dest,
            r#"
            /// Access a single mipmap level of this layer.
            #[inline]
            pub fn mipmap(&self, level: u32) -> Option<{name}LayerMipmap<'t>> {{
                self.0.mipmap(level).map(|m| {name}LayerMipmap(m, self.1))
            }}

            /// Access the main mipmap level of this layer.
            #[inline]
            pub fn main_level(&self) -> {name}LayerMipmap<'t> {{
                self.mipmap(0).unwrap()
            }}
        "#,
            name = name
        ))
        .unwrap();

        // closing `impl Layer` block
        (writeln!(dest, "}}")).unwrap();
    }

    // the `Mipmap` struct
    {
        // writing the struct
        (write!(
            dest,
            r#"
                /// Represents a single mipmap level of a `{name}`.
                ///
                /// Can be obtained by calling `{name}::mipmap()`, `{name}::main_level()`,
                /// `{name}Layer::mipmap()` or `{name}Layer::main_level()`.
                #[derive(Copy, Clone)]
                pub struct {name}Mipmap<'t>(TextureAnyMipmap<'t>, &'t {name});

                impl<'a> ::std::ops::Deref for {name}Mipmap<'a> {{
                    type Target = TextureAnyMipmap<'a>;

                    #[inline]
                    fn deref(&self) -> &TextureAnyMipmap<'a> {{
                        &self.0
                    }}
                }}
            "#,
            name = name
        ))
        .unwrap();

        // opening `impl Mipmap` block
        (writeln!(dest, "impl<'t> {}Mipmap<'t> {{", name)).unwrap();

        // dimensions getters
        write_dimensions_getters(dest, dimensions, "self.0", true);

        // writing the `write` function for mipmaps of one-dimensional and three-dimensional
        // textures
        if is_writable && dimensions == TextureDimensions::Texture1d {
            (write!(dest, r#"
                    /// Uploads some data in the texture level, starting at the texel `offset`.
                    ///
                    /// Note that this may cause a synchronization if you use the texture right before
                    /// or right after this call.
                    ///
                    /// Returns an error if the data doesn't fit in the texture level, or if its
                    /// size or format doesn't match.
                    pub fn write<'a, T>(&self, offset: u32, data: T) -> Result<(), UploadError>
                        where T: Texture1dDataSource<'a>
                    {{
                        let RawImage1d {{ data, width, format: client_format }} = data.into_raw();
                        let client_format = ClientFormatAny::ClientFormat(client_format);

                        self.0.upload_texture(offset, 0, 0, (client_format, data), width, None, None,
                                              true)
                    }}
                "#)).unwrap();
        }

        if is_writable && dimensions == TextureDimensions::Texture3d {
            (write!(dest, r#"
                    /// Uploads some data in the texture level, starting at the texel `offset`
                    /// (x, y, z).
                    ///
                    /// Note that this may cause a synchronization if you use the texture right before
                    /// or right after this call.
                    ///
                    /// Returns an error if the data doesn't fit in the texture level, or if its
                    /// size or format doesn't match.
                    pub fn write<'a, T>(&self, offset: (u32, u32, u32), data: T) -> Result<(), UploadError>
                        where T: Texture3dDataSource<'a>
                    {{
                        let RawImage3d {{ data, width, height, depth, format: client_format }} =
                                                data.into_raw();
                        let client_format = ClientFormatAny::ClientFormat(client_format);

                        self.0.upload_texture(offset.0, offset.1, offset.2, (client_format, data),
                                              width, Some(height), Some(depth), true)
                    }}
                "#)).unwrap();
        }

        // writing the `write` function for mipmaps.
        if (dimensions == TextureDimensions::Texture2d
            || dimensions == TextureDimensions::TextureRectangle)
            && (is_writable || is_compressed)
        {
            let compressed_restrictions = if is_compressed {
                r#" ///
                    /// For compressed textures, the region must start on the blocks of the format
                    /// (4 × 4 texels) and end on them or on the edge of the image, otherwise
                    /// `UploadError::MisalignedBlocks` is returned.
                "#
            } else {
                ""
            };

            (write!(dest, r#"
                    /// Uploads some data in the texture level.
                    ///
                    /// Note that this may cause a synchronization if you use the texture right before
                    /// or right after this call.
                    ///
                    /// Returns an error if the dimensions of `data` don't match the `Rect`, if the
                    /// `Rect` is out of the level, or if the size or format of the data doesn't
                    /// match.
                    {compressed_restrictions}
                    pub fn write<'a, T>(&self, rect: Rect, data: T) -> Result<(), UploadError>
                        where T: {data_source_trait}<'a>
                    {{
                        let RawImage2d {{ data, width, height, format: client_format }} =
                                                data.into_raw();

                        if width != rect.width || height != rect.height {{
                            return Err(UploadError::OutOfBounds);
                        }}

                        let client_format = ClientFormatAny::ClientFormat(client_format);

                        self.0.upload_texture(rect.left, rect.bottom, 0, (client_format, data),
                                              width, Some(height), None, true)
                    }}
                "#, data_source_trait = data_source_trait,
                    compressed_restrictions = compressed_restrictions)).unwrap();
        }

        // writing the `write_compressed_data` function for mipmaps.
        // TODO: implement for other types too
        if dimensions == TextureDimensions::Texture2d && is_compressed {
            (write!(dest, r#"
                    /// Uploads some data in the texture level by using a compressed format as input.
                    ///
                    /// Note that this may cause a synchronization if you use the texture right before
                    /// or right after this call.
                    ///
                    /// Returns an error if the dimensions of `data` don't match the `Rect`, or if
                    /// the size of the data doesn't match its format.
                    ///
                    /// The contents of any texel outside the region modified by the call are
                    /// undefined.
                    pub fn write_compressed_data(&self, rect: Rect, data: &[u8],
                                                 width: u32, height: u32, format: {format})
                                                 -> Result<(), UploadError>
                    {{
                        if width != rect.width || height != rect.height {{
                            return Err(UploadError::OutOfBounds);
                        }}

                        let data = Cow::Borrowed(data.as_ref());
                        let client_format = {client_format_any}(format);

                        self.0.upload_texture(rect.left, rect.bottom, 0, (client_format, data),
                                              width, Some(height), None, false)
                    }}
                "#, format = relevant_format, client_format_any = client_format_any_ty)).unwrap();
        }

        // writing the `read_compressed_data` function for mipmaps
        if is_compressed && !dimensions.is_array() {
            (write!(dest, r#"
                    /// Reads the content of the texture level to RAM without decompressing it before.
                    ///
                    /// You should avoid doing this at all cost during performance-critical
                    /// operations (for example, while you're drawing).
                    ///
                    /// Returns the compressed format of the texture and the compressed data, gives
                    /// `None` when the internal compression format is generic or unknown.
                    #[inline]
                    pub fn read_compressed_data(&self) -> Option<({format}, Vec<u8>)> {{
                        match self.0.download_compressed_data() {{
                            Some(({client_format_any}(format), buf)) => Some((format, buf)),
                            None => None,
                            _ => unreachable!(),
                        }}
                    }}
                "#, format = relevant_format, client_format_any = client_format_any_ty)).unwrap();
        }

        // writing the `get_level` and `get_texture` functions
        (write!(
            dest,
            "
                /// Returns the corresponding texture.
                #[inline]
                pub fn get_texture(&self) -> &'t {name} {{
                    self.1
                }}

                /// Returns the texture level.
                #[inline]
                pub fn get_level(&self) -> u32 {{
                    self.0.get_level()
                }}
        ",
            name = name
        ))
        .unwrap();

        if dimensions.is_array() {
            (write!(
                dest,
                "
                    /// Access the first layer of this texture.
                    #[inline]
                    pub fn first_layer(&self) -> {name}LayerMipmap<'t> {{
                        self.layer(0).unwrap()
                    }}

                    /// Access a single layer of this texture.
                    #[inline]
                    pub fn layer(&self, layer: u32) -> Option<{name}LayerMipmap<'t>> {{
                        self.0.layer(layer).map(|l| {name}LayerMipmap(l, self.1))
                    }}
                ",
                name = name
            ))
            .unwrap();
        }

        if !dimensions.is_array() && dimensions.is_cube() {
            writeln!(
                dest,
                "/// Provides an object representing a single layer of this cubemap.
                pub fn image(&self, layer: CubeLayer) -> {name}Image<'t> {{
                    {name}Image(self.0.first_layer().into_image(Some(layer)).unwrap(), self.1)
                }}",
                name = name
            )
            .unwrap();
        }

        // closing `impl Mipmap` block
        (writeln!(dest, "}}")).unwrap();

        (writeln!(
            dest,
            "#[cfg(any(safegl_gl43, safegl_gles32))]
             impl<'t> From<{name}Mipmap<'t>> for crate::texture::CopyImageTarget<'t> {{
                 #[inline]
                 fn from(mipmap: {name}Mipmap<'t>) -> crate::texture::CopyImageTarget<'t> {{
                     crate::texture::CopyImageTarget::Texture(mipmap.0)
                 }}
             }}",
            name = name
        ))
        .unwrap();

        if !dimensions.is_array() && !dimensions.is_cube() {
            // into raw image
            (writeln!(
                dest,
                "impl<'t> Into<TextureAnyImage<'t>> for {name}Mipmap<'t> {{
                                fn into(self) -> TextureAnyImage<'t> {{
                                    self.0.first_layer().into_image(None).unwrap()
                                }}
                             }}",
                name = name
            ))
            .unwrap();
        }
    }

    // the `LayerMipmap` struct
    if dimensions.is_array() {
        // writing the struct
        (write!(
            dest,
            r#"
                /// Represents a single layer of a mipmap level of a `{name}`.
                #[derive(Copy, Clone)]
                pub struct {name}LayerMipmap<'t>(TextureAnyLayerMipmap<'t>, &'t {name});
            "#,
            name = name
        ))
        .unwrap();

        // opening `impl LayerMipmap` block
        (writeln!(dest, "impl<'t> {}LayerMipmap<'t> {{", name)).unwrap();

        // dimensions getters
        write_dimensions_getters(dest, dimensions, "self.0", false);

        // to the image struct
        if dimensions.is_cube() {
            writeln!(
                dest,
                "/// Provides an object representing a single layer of this cubemap.
                pub fn image(&self, layer: CubeLayer) -> {name}Image<'t> {{
                    {name}Image(self.0.into_image(Some(layer)).unwrap(), self.1)
                }}",
                name = name
            )
            .unwrap();
        }

        // writing the `write` function of layers
        if is_writable && dimensions == TextureDimensions::Texture1dArray {
            (write!(dest, r#"
                    /// Uploads some data in this layer, starting at the texel `offset`.
                    ///
                    /// Returns an error if the data doesn't fit in the layer, or if its size or
                    /// format doesn't match.
                    pub fn write<'a, T>(&self, offset: u32, data: T) -> Result<(), UploadError>
                        where T: Texture1dDataSource<'a>, T::Data: PixelValue
                    {{
                        let RawImage1d {{ data, width, format }} = data.into_raw();
                        let rect = Rect {{ left: offset, bottom: 0, width, height: 1 }};
                        self.0.into_image(None).unwrap()
                            .raw_write(&rect, RawImage2d {{ data, width, height: 1, format }})
                    }}
                "#)).unwrap();
        }

        if is_writable && dimensions == TextureDimensions::Texture2dArray {
            (write!(dest, r#"
                    /// Uploads some data in a rectangle of this layer.
                    ///
                    /// Returns an error if the dimensions of `data` don't match the `Rect`, if the
                    /// `Rect` is out of the layer, or if the size or format of the data doesn't
                    /// match.
                    pub fn write<'a, T>(&self, rect: Rect, data: T) -> Result<(), UploadError>
                        where T: Texture2dDataSource<'a>
                    {{
                        self.0.into_image(None).unwrap().raw_write(&rect, data)
                    }}
                "#)).unwrap();
        }

        // closing `impl LayerMipmap` block
        (writeln!(dest, "}}")).unwrap();

        // attachment traits
        if !dimensions.is_cube() {
            // into raw image
            (writeln!(
                dest,
                "impl<'t> Into<TextureAnyImage<'t>> for {name}LayerMipmap<'t> {{
                                fn into(self) -> TextureAnyImage<'t> {{
                                    self.0.into_image(None).unwrap()
                                }}
                             }}",
                name = name
            ))
            .unwrap();
        }
    }

    // the `Image` struct, only for cubemaps
    if dimensions.is_cube() {
        // writing the struct
        (write!(
            dest,
            r#"
                /// Represents a single image of a mipmap level of a layer of `{name}`.
                #[derive(Copy, Clone)]
                pub struct {name}Image<'t>(TextureAnyImage<'t>, &'t {name});
            "#,
            name = name
        ))
        .unwrap();

        // opening `impl Image` block
        (writeln!(dest, "impl<'t> {}Image<'t> {{", name)).unwrap();

        // dimensions getters
        write_dimensions_getters(dest, dimensions, "self.0", false);

        // writing the `write` function of cubemap faces
        if is_writable {
            (write!(dest, r#"
                    /// Uploads some data in a rectangle of this face.
                    ///
                    /// Returns an error if the dimensions of `data` don't match the `Rect`, if the
                    /// `Rect` is out of the face, or if the size or format of the data doesn't
                    /// match.
                    pub fn write<'a, T>(&self, rect: Rect, data: T) -> Result<(), UploadError>
                        where T: Texture2dDataSource<'a>
                    {{
                        self.0.raw_write(&rect, data)
                    }}
                "#)).unwrap();
        }

        // closing `impl Image` block
        (writeln!(dest, "}}")).unwrap();

        // into raw image
        (writeln!(
            dest,
            "impl<'t> Into<TextureAnyImage<'t>> for {name}Image<'t> {{
                            fn into(self) -> TextureAnyImage<'t> {{
                                self.0
                            }}
                         }}",
            name = name
        ))
        .unwrap();
    }

    // implement the attachments traits
    {
        let attachment_type = if dimensions.is_cube() {
            format!("{}Image", name)
        } else if dimensions.is_array() {
            format!("{}LayerMipmap", name)
        } else {
            format!("{}Mipmap", name)
        };

        match ty {
            TextureType::Regular
            | TextureType::Srgb
            | TextureType::Integral
            | TextureType::Unsigned => {
                (writeln!(dest, "
                        impl<'t> crate::framebuffer::ToColorAttachment<'t> for {ty}<'t> {{
                            #[inline]
                            fn to_color_attachment(self) -> crate::framebuffer::ColorAttachment<'t> {{
                                crate::framebuffer::ColorAttachment::Texture(self.into())
                            }}
                        }}
                    ", ty = attachment_type)).unwrap();
            }
            TextureType::Depth => {
                (writeln!(dest, "
                        impl<'t> crate::framebuffer::ToDepthAttachment<'t> for {ty}<'t> {{
                            #[inline]
                            fn to_depth_attachment(self) -> crate::framebuffer::DepthAttachment<'t> {{
                                crate::framebuffer::DepthAttachment::Texture(self.into())
                            }}
                        }}
                    ", ty = attachment_type)).unwrap();
            }
            TextureType::Stencil => {
                (writeln!(dest, "
                        impl<'t> crate::framebuffer::ToStencilAttachment<'t> for {ty}<'t> {{
                            #[inline]
                            fn to_stencil_attachment(self) -> crate::framebuffer::StencilAttachment<'t> {{
                                crate::framebuffer::StencilAttachment::Texture(self.into())
                            }}
                        }}
                    ", ty = attachment_type)).unwrap();
            }
            TextureType::DepthStencil => {
                (writeln!(dest, "
                        impl<'t> crate::framebuffer::ToDepthStencilAttachment<'t> for {ty}<'t> {{
                            #[inline]
                            fn to_depth_stencil_attachment(self) -> crate::framebuffer::DepthStencilAttachment<'t> {{
                                crate::framebuffer::DepthStencilAttachment::Texture(self.into())
                            }}
                        }}
                    ", ty = attachment_type)).unwrap();
            }
            _ => (),
        }
    }

    // implement the layered attachment traits for the levels of the textures with layers, and
    // for the textures themselves, which attach their main level
    if dimensions.is_array() || dimensions.is_cube() || dimensions == TextureDimensions::Texture3d
    {
        let (attachment, method) = match ty {
            TextureType::Regular
            | TextureType::Srgb
            | TextureType::Integral
            | TextureType::Unsigned => ("Color", "color"),
            TextureType::Depth => ("Depth", "depth"),
            TextureType::Stencil => ("Stencil", "stencil"),
            TextureType::DepthStencil => ("DepthStencil", "depth_stencil"),
            _ => ("", ""),
        };
        if !attachment.is_empty() {
            (writeln!(dest, "
                    impl<'t> crate::framebuffer::ToLayered{attachment}Attachment<'t> for {name}Mipmap<'t> {{
                        #[inline]
                        fn to_layered_{method}_attachment(self) -> crate::framebuffer::Layered{attachment}Attachment<'t> {{
                            crate::framebuffer::Layered{attachment}Attachment(self.0)
                        }}
                    }}

                    impl<'t> crate::framebuffer::ToLayered{attachment}Attachment<'t> for &'t {name} {{
                        #[inline]
                        fn to_layered_{method}_attachment(self) -> crate::framebuffer::Layered{attachment}Attachment<'t> {{
                            crate::framebuffer::Layered{attachment}Attachment(self.main_level().0)
                        }}
                    }}
                ", attachment = attachment, method = method, name = name)).unwrap();
        }
    }

    // closing `mod module {`
    writeln!(dest, "}}").unwrap();
}

fn write_dimensions_getters<W: Write>(
    dest: &mut W,
    dimensions: TextureDimensions,
    accessor: &str,
    write_array_size: bool,
) {
    writeln!(
        dest,
        r#"
        /// Returns the width of that image.
        #[inline]
        pub fn width(&self) -> u32 {{
            {}.get_width()
        }}
    "#,
        accessor
    )
    .unwrap();

    match dimensions {
        TextureDimensions::Texture2d
        | TextureDimensions::Texture2dMultisample
        | TextureDimensions::Texture3d
        | TextureDimensions::Texture2dArray
        | TextureDimensions::Texture2dMultisampleArray
        | TextureDimensions::Cubemap
        | TextureDimensions::CubemapArray => {
            writeln!(
                dest,
                r#"
                /// Returns the height of that image.
                #[inline]
                pub fn height(&self) -> u32 {{
                    {}.get_height().unwrap()
                }}
            "#,
                accessor
            )
            .unwrap();
        }
        _ => (),
    };

    match dimensions {
        TextureDimensions::Texture3d => {
            writeln!(
                dest,
                r#"
                /// Returns the depth of that image.
                #[inline]
                pub fn depth(&self) -> u32 {{
                    {}.get_depth().unwrap()
                }}
            "#,
                accessor
            )
            .unwrap();
        }
        _ => (),
    };

    if write_array_size {
        match dimensions {
            TextureDimensions::Texture2dArray
            | TextureDimensions::Texture2dMultisampleArray
            | TextureDimensions::CubemapArray => {
                writeln!(
                    dest,
                    r#"
                    /// Returns the number of array layers.
                    #[inline]
                    pub fn array_size(&self) -> u32 {{
                        {}.get_array_size().unwrap()
                    }}
                "#,
                    accessor
                )
                .unwrap();
            }
            _ => (),
        }
    };

    match dimensions {
        TextureDimensions::Texture2dMultisample | TextureDimensions::Texture2dMultisampleArray => {
            writeln!(
                dest,
                r#"
                /// Returns the number of samples of that image.
                #[inline]
                pub fn samples(&self) -> u32 {{
                    {}.get_samples().unwrap()
                }}
            "#,
                accessor
            )
            .unwrap();
        }
        _ => (),
    };

    match dimensions {
        TextureDimensions::Texture2d
        | TextureDimensions::Texture2dMultisample
        | TextureDimensions::Texture2dArray
        | TextureDimensions::Texture2dMultisampleArray => {
            writeln!(
                dest,
                r#"
                /// Returns the width and height of that image.
                #[inline]
                pub fn dimensions(&self) -> (u32, u32) {{
                    (self.width(), self.height())
                }}
            "#
            )
            .unwrap();
        }
        TextureDimensions::Texture3d => {
            writeln!(
                dest,
                r#"
                /// Returns the width, height and depth of that image.
                #[inline]
                pub fn dimensions(&self) -> (u32, u32, u32) {{
                    (self.width(), self.height(), self.depth())
                }}
            "#
            )
            .unwrap();
        }
        TextureDimensions::Cubemap | TextureDimensions::CubemapArray => {
            writeln!(
                dest,
                r#"
                /// Returns the dimension of that image.
                #[inline]
                pub fn dimensions(&self) -> u32 {{
                    self.width()
                }}
            "#
            )
            .unwrap();
        }
        _ => (),
    };
}
