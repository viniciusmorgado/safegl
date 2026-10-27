//! Kinds of textures and formats: compressed formats, rectangle textures, texture views, ranges
//! of buffer textures, multisample textures and images.
// the helpers are unused at the levels that have none of the tests that need them
#![allow(dead_code)]

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::Texture2d;
use safegl::uniforms::Uniforms;
use safegl::{DrawError, Program, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;
type Pixel = (u8, u8, u8, u8);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A program whose fragment shader declares `declarations` and runs `body`, which writes
/// `color`, compiled as GLSL `desktop` (330, 430, ...) or GLSL ES `es` (300, 310, ...).
fn program(display: &Display, desktop: u32, es: u32, declarations: &str, body: &str) -> Program {
    let header = if support::is_gles(display) {
        format!(
            "#version {es} es\nprecision highp float;\nprecision highp int;\n\
             precision highp usampler2D;\nprecision highp isampler2D;\n"
        )
    } else {
        format!("#version {desktop} core\n")
    };
    let vertex = format!(
        "{header}in vec2 position;\nout vec2 uv;\n\
         void main() {{ uv = position * 0.5 + 0.5; gl_Position = vec4(position, 0.0, 1.0); }}"
    );
    let fragment =
        format!("{header}in vec2 uv;\n{declarations}\nout vec4 color;\nvoid main() {{ {body} }}");
    Program::from_source(display, &vertex, &fragment, None).unwrap()
}

/// Draws a fullscreen triangle with `program` into a target of 4 × 4 pixels and returns its
/// pixels.
fn draw(
    display: &Display,
    program: &Program,
    uniforms: &impl Uniforms,
) -> Result<Vec<Vec<Pixel>>, DrawError> {
    let target = Texture2d::empty(display, 4, 4).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let triangle =
        VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap();
    target.as_surface().draw(
        &triangle,
        NoIndices(PrimitiveType::TrianglesList),
        program,
        uniforms,
        &Default::default(),
    )?;
    Ok(target.read().unwrap())
}

/// An ETC2 block of 4 × 4 texels of a single color: the individual mode with the base colors
/// 8, 4 and 2 in both halves (`0x88`, `0x44` and `0x22` once extended to 8 bits), the modifier
/// table 0 and every texel at index 0, which adds 2.
const ETC2_BLOCK: [u8; 8] = [0x88, 0x44, 0x22, 0x00, 0, 0, 0, 0];
const ETC2_COLOR: Pixel = (0x88 + 2, 0x44 + 2, 0x22 + 2, 255);

#[cfg(any(safegl_gl43, safegl_gles30))]
#[test]
fn etc2_textures() {
    use safegl::texture::{CompressedFormat, CompressedMipmapsOption, CompressedTexture2d};

    let display = support::build_display();
    if !CompressedFormat::Etc2Rgb8.is_supported(&display) {
        return;
    }
    let texture = CompressedTexture2d::with_compressed_data(
        &display,
        &ETC2_BLOCK,
        4,
        4,
        CompressedFormat::Etc2Rgb8,
        CompressedMipmapsOption::NoMipmap,
    )
    .unwrap();
    let program = program(
        &display,
        330,
        300,
        "uniform sampler2D tex;",
        "color = texture(tex, uv);",
    );
    let pixels = draw(&display, &program, &uniform! { tex: &texture }).unwrap();
    assert!(
        pixels.iter().flatten().all(|pixel| *pixel == ETC2_COLOR),
        "{:?}",
        pixels[0][0]
    );

    // a block of 8 bytes for each 4 × 4 texels: a 6 × 6 texture has 4 blocks
    let result = CompressedTexture2d::with_compressed_data(
        &display,
        &[0u8; 4 * 8],
        6,
        6,
        CompressedFormat::Etc2Rgb8,
        CompressedMipmapsOption::NoMipmap,
    );
    assert_eq!(result.unwrap().get_width(), 6);
    let result = CompressedTexture2d::with_compressed_data(
        &display,
        &[0u8; 3 * 8],
        6,
        6,
        CompressedFormat::Etc2Rgb8,
        CompressedMipmapsOption::NoMipmap,
    );
    assert!(result.is_err());

    // EAC with two channels takes 16 bytes per block
    let texture = CompressedTexture2d::with_compressed_data(
        &display,
        &[0u8; 16],
        4,
        4,
        CompressedFormat::EacRg11Unsigned,
        CompressedMipmapsOption::NoMipmap,
    );
    assert!(texture.is_ok());

    display.assert_no_error(None);
}

#[cfg(any(safegl_gl43, safegl_gles30))]
#[test]
fn etc2_srgb_textures() {
    use safegl::texture::{CompressedMipmapsOption, CompressedSrgbFormat, CompressedSrgbTexture2d};

    let display = support::build_display();
    if !CompressedSrgbFormat::Etc2Rgb8.is_supported(&display) {
        return;
    }
    let texture = CompressedSrgbTexture2d::with_compressed_data(
        &display,
        &ETC2_BLOCK,
        4,
        4,
        CompressedSrgbFormat::Etc2Rgb8,
        CompressedMipmapsOption::NoMipmap,
    )
    .unwrap();
    let program = program(
        &display,
        330,
        300,
        "uniform sampler2D tex;",
        "color = texture(tex, uv);",
    );
    // sampling converts the sRGB colors to linear ones, which are darker
    let pixel = draw(&display, &program, &uniform! { tex: &texture }).unwrap()[0][0];
    assert!(
        pixel.0 < ETC2_COLOR.0 && pixel.1 < ETC2_COLOR.1,
        "{pixel:?}"
    );

    display.assert_no_error(None);
}

/// Multisample textures whose samples may be placed differently in each texel.
#[cfg(any(safegl_desktop, safegl_gles31))]
#[test]
fn variable_sample_locations() {
    use safegl::framebuffer::{DepthRenderBuffer, SimpleFrameBuffer, ValidationError};
    use safegl::glow::HasContext;
    use safegl::texture::{DepthFormat, Texture2dMultisample, UncompressedFloatFormat};
    use safegl::GlObject;

    let display = support::build_display();
    if !safegl::texture::is_texture_2d_multisample_supported(&display) {
        return;
    }
    let texture = Texture2dMultisample::empty_with_sample_locations(
        &display,
        UncompressedFloatFormat::U8U8U8U8,
        4,
        4,
        4,
        false,
    )
    .unwrap();
    assert!(!texture.has_fixed_sample_locations());
    let fixed = unsafe {
        let gl = display.glow_context();
        gl.bind_texture(safegl::glow::TEXTURE_2D_MULTISAMPLE, Some(texture.get_id()));
        let fixed = gl.get_tex_level_parameter_i32(
            safegl::glow::TEXTURE_2D_MULTISAMPLE,
            0,
            safegl::glow::TEXTURE_FIXED_SAMPLE_LOCATIONS,
        );
        gl.bind_texture(safegl::glow::TEXTURE_2D_MULTISAMPLE, None);
        fixed
    };
    display.restore_state();
    assert_eq!(fixed, 0);

    let mut framebuffer = SimpleFrameBuffer::new(&display, &texture).unwrap();
    framebuffer.clear_color(1.0, 0.0, 0.0, 1.0);

    // render buffers have fixed sample locations
    let depth = DepthRenderBuffer::new_multisample(&display, DepthFormat::I24, 4, 4, 4).unwrap();
    assert!(matches!(
        SimpleFrameBuffer::with_depth_buffer(&display, &texture, &depth),
        Err(ValidationError::Incomplete { .. })
    ));

    display.assert_no_error(None);
}

/// Invalidating a texture keeps it usable: it is only a hint.
#[cfg(safegl_gl43)]
#[test]
fn texture_invalidation() {
    use safegl::texture::{RegionOutOfBoundsError, Texture2dArray};

    let display = support::build_display();
    let texture = Texture2dArray::empty(&display, 4, 4, 3).unwrap();
    texture.main_level().invalidate();
    texture
        .main_level()
        .invalidate_region((1, 1, 2), (3, 3, 1))
        .unwrap();
    assert_eq!(
        texture.main_level().invalidate_region((1, 1, 2), (3, 3, 2)),
        Err(RegionOutOfBoundsError)
    );
    assert_eq!(
        texture.main_level().invalidate_region((5, 0, 0), (0, 0, 0)),
        Err(RegionOutOfBoundsError)
    );

    // the texture is written and read as usual afterwards
    let layer = texture.main_level().layer(1).unwrap();
    layer
        .write(
            safegl::Rect {
                left: 0,
                bottom: 0,
                width: 4,
                height: 4,
            },
            vec![vec![(1u8, 2u8, 3u8, 4u8); 4]; 4],
        )
        .unwrap();
    let image: safegl::texture::TextureAnyImage<'_> = layer.into();
    let rect = safegl::Rect {
        left: 0,
        bottom: 0,
        width: 4,
        height: 4,
    };
    let data: Vec<Vec<Pixel>> = image.raw_read(&rect).unwrap();
    assert_eq!(data[3][3], (1, 2, 3, 4));

    display.assert_no_error(None);
}

/// A buffer texture of a range of its buffer.
#[cfg(any(safegl_gl43, safegl_gles32))]
#[test]
fn buffer_texture_range() {
    use safegl::buffer::{Buffer, BufferMode, BufferType};
    use safegl::texture::buffer_texture::{BufferTexture, BufferTextureType, TextureCreationError};
    use safegl::CapabilitiesSource;

    let display = support::build_display();
    let Some(alignment) = display
        .get_capabilities()
        .limits
        .texture_buffer_offset_alignment
    else {
        return;
    };
    if !support::glsl_supported(&display, 330, 320) {
        return;
    }
    let element = std::mem::size_of::<u32>();
    let start = (alignment as usize).div_ceil(element).max(1);
    let data = (0..start as u32 + 64).collect::<Vec<u32>>();
    let buffer = || {
        Buffer::new(
            &display,
            data.as_slice(),
            BufferType::TextureBuffer,
            BufferMode::Default,
        )
        .unwrap()
    };

    let texture = BufferTexture::from_buffer_range(
        &display,
        buffer(),
        start..start + 4,
        BufferTextureType::Unsigned,
    )
    .map_err(|(error, _)| error)
    .unwrap();
    assert_eq!(texture.range(), start..start + 4);
    let precision = if support::is_gles(&display) {
        "precision highp usamplerBuffer;\n"
    } else {
        ""
    };
    let program = program(
        &display,
        330,
        320,
        &format!("{precision}uniform usamplerBuffer tex;"),
        "color = vec4(float(texelFetch(tex, 0).r % 256u), float(textureSize(tex)), \
         float(texelFetch(tex, 3).r % 256u), 255.0) / 255.0;",
    );
    let pixel = draw(&display, &program, &uniform! { tex: &texture }).unwrap()[0][0];
    assert_eq!(
        pixel,
        ((start % 256) as u8, 4, ((start + 3) % 256) as u8, 255)
    );

    let error = |range: std::ops::Range<usize>| {
        BufferTexture::from_buffer_range(&display, buffer(), range, BufferTextureType::Unsigned)
            .map(|_| ())
            .map_err(|(error, _)| error)
    };
    assert!(matches!(
        error(start..start + 65),
        Err(TextureCreationError::RangeOutOfBounds)
    ));
    if alignment as usize > element {
        assert!(matches!(
            error(1..5),
            Err(TextureCreationError::MisalignedRange { .. })
        ));
    }

    display.assert_no_error(None);
}

/// A program like `program`, or `None` if the context can't compile it, such as OpenGL ES
/// contexts without images in fragment shaders.
fn try_program(
    display: &Display,
    desktop: u32,
    es: u32,
    declarations: &str,
    body: &str,
) -> Option<Program> {
    if !support::glsl_supported(display, desktop, es) {
        return None;
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        program(display, desktop, es, declarations, body)
    }))
    .ok()
}

/// Shaders write single samples of multisample textures through `image2DMS`.
#[cfg(safegl_gl42)]
#[test]
fn multisample_images() {
    use safegl::framebuffer::SimpleFrameBuffer;
    use safegl::texture::{Texture2dMultisample, UncompressedFloatFormat};
    use safegl::uniforms::{ImageUnitAccess, ImageUnitError, ImageUnitFormat};
    use safegl::CapabilitiesSource;

    let display = support::build_display();
    if support::is_gles(&display) || !safegl::texture::is_texture_2d_multisample_supported(&display)
    {
        return;
    }
    let texture = Texture2dMultisample::empty_with_format(
        &display,
        UncompressedFloatFormat::U8U8U8U8,
        safegl::texture::MipmapsOption::NoMipmap,
        4,
        4,
        4,
    )
    .unwrap();
    let maximum = display
        .get_capabilities()
        .limits
        .max_image_samples
        .unwrap_or(0);
    let unit = match texture.image_unit(ImageUnitFormat::RGBA8) {
        Ok(unit) => unit.set_access(ImageUnitAccess::Write),
        Err(ImageUnitError::TooManySamples { samples: 4, .. }) => {
            assert!(maximum < 4);
            return;
        }
        Err(error) => panic!("{error}"),
    };
    let Some(write) = try_program(
        &display,
        430,
        0,
        "layout(rgba8) writeonly uniform image2DMS img;",
        "imageStore(img, ivec2(gl_FragCoord.xy), 2, vec4(0.0, 1.0, 0.0, 1.0)); color = vec4(0.0);",
    ) else {
        return;
    };
    SimpleFrameBuffer::new(&display, &texture)
        .unwrap()
        .clear_color(1.0, 0.0, 0.0, 1.0);
    draw(&display, &write, &uniform! { img: unit }).unwrap();

    let read = program(
        &display,
        430,
        0,
        "uniform sampler2DMS tex;\nuniform int sample_index;",
        "color = texelFetch(tex, ivec2(gl_FragCoord.xy), sample_index);",
    );
    let sample = |index: i32| {
        draw(
            &display,
            &read,
            &uniform! { tex: &texture, sample_index: index },
        )
        .unwrap()[1][1]
    };
    assert_eq!(sample(2), (0, 255, 0, 255));
    assert_eq!(sample(0), (255, 0, 0, 255));

    display.assert_no_error(None);
}

/// Shaders read and write buffer textures through `imageBuffer` uniforms.
#[cfg(any(safegl_gl42, safegl_gles32))]
#[test]
fn buffer_images() {
    use safegl::texture::buffer_texture::{BufferTexture, BufferTextureType};
    use safegl::uniforms::{ImageUnitError, ImageUnitFormat};

    let display = support::build_display();
    let texture = match BufferTexture::new(&display, &[0u32; 4], BufferTextureType::Unsigned) {
        Ok(texture) => texture,
        Err(_) => return,
    };
    let precision = if support::is_gles(&display) {
        "precision highp uimageBuffer;\n"
    } else {
        ""
    };
    let Some(count) = try_program(
        &display,
        430,
        320,
        &format!("{precision}layout(r32ui) uniform coherent uimageBuffer img;"),
        "imageAtomicAdd(img, 2, 1u); color = vec4(0.0);",
    ) else {
        return;
    };
    assert!(matches!(
        texture.image_unit(ImageUnitFormat::RGBA32UI),
        Err(ImageUnitError::BadFormatClass(32, 128))
    ));
    let unit = texture.image_unit(ImageUnitFormat::R32UI).unwrap();
    draw(&display, &count, &uniform! { img: unit }).unwrap();

    // every fragment of the 4 × 4 target added 1, which a read of the buffer sees
    let data = texture.read().unwrap();
    assert_eq!(data, vec![0, 0, 16, 0]);

    display.assert_no_error(None);
}

/// Rectangle textures are sampled with coordinates in texels, without repetition or mipmaps.
#[cfg(safegl_desktop)]
#[test]
fn rectangle_textures() {
    use safegl::texture::{MipmapsOption, TextureCreationError, TextureRectangle};
    use safegl::uniforms::{MinifySamplerFilter, SamplerWrapFunction};

    let display = support::build_display();
    if support::is_gles(&display) {
        assert!(matches!(
            TextureRectangle::empty(&display, 4, 4),
            Err(TextureCreationError::TypeNotSupported)
        ));
        return;
    }
    // texel (x, y) is (x × 60, y × 60, 0, 255)
    let texels = (0..4u8)
        .map(|y| {
            (0..4u8)
                .map(|x| (x * 60, y * 60, 0u8, 255u8))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let texture = TextureRectangle::new(&display, texels.clone()).unwrap();
    let read: Vec<Vec<Pixel>> = texture.read().unwrap();
    assert_eq!(read, texels);

    let program = program(
        &display,
        330,
        0,
        "uniform sampler2DRect tex;",
        "color = texture(tex, gl_FragCoord.xy);",
    );
    assert_eq!(
        draw(&display, &program, &uniform! { tex: &texture }).unwrap(),
        texels
    );
    let sampler = texture.sampled();
    assert_eq!(
        draw(&display, &program, &uniform! { tex: sampler }).unwrap(),
        texels
    );

    // samplers that repeat or filter with mipmaps can't sample them
    for sampler in [
        texture.sampled().wrap_function(SamplerWrapFunction::Repeat),
        texture
            .sampled()
            .minify_filter(MinifySamplerFilter::LinearMipmapLinear),
    ] {
        assert!(matches!(
            draw(&display, &program, &uniform! { tex: sampler }),
            Err(DrawError::RectangleTextureSampler { .. })
        ));
    }

    // they have no mipmaps
    assert!(matches!(
        TextureRectangle::empty_with_mipmaps(&display, MipmapsOption::EmptyMipmaps, 4, 4),
        Err(TextureCreationError::MipmapsNotSupported)
    ));

    // and are drawn to like other textures
    let target = TextureRectangle::empty(&display, 3, 5).unwrap();
    target.as_surface().clear_color(0.0, 1.0, 0.0, 1.0);
    let read: Vec<Vec<Pixel>> = target.read().unwrap();
    assert_eq!(read, vec![vec![(0, 255, 0, 255); 3]; 5]);

    display.assert_no_error(None);
}

/// Texture views share the storage of their source, with another kind, format, range of levels
/// or range of layers.
#[cfg(safegl_gl43)]
#[test]
fn texture_views() {
    use safegl::texture::{
        Cubemap, MipmapsOption, SrgbFormat, SrgbTexture2d, Texture2dArray, Texture3d,
        TextureViewError, UncompressedFloatFormat, UncompressedUintFormat, UnsignedTexture2d,
    };

    let display = support::build_display();
    let source = Texture2d::with_format(
        &display,
        vec![vec![(1u8, 2u8, 3u8, 4u8); 4]; 4],
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::NoMipmap,
    )
    .unwrap();
    // an RGBA8 texel read as a single 32-bit unsigned integer
    let as_uint = match UnsignedTexture2d::view(&source, UncompressedUintFormat::U32, 0..1, 0..1) {
        Err(TextureViewError::NotSupported) => return,
        result => result.unwrap(),
    };
    let uint_program = program(
        &display,
        330,
        0,
        "uniform usampler2D tex;",
        "uint v = texelFetch(tex, ivec2(0), 0).r;\n\
         color = vec4(float(v & 255u), float((v >> 8) & 255u), float((v >> 16) & 255u), \
         float(v >> 24)) / 255.0;",
    );
    assert_eq!(
        draw(&display, &uint_program, &uniform! { tex: &as_uint }).unwrap()[0][0],
        (1, 2, 3, 4)
    );

    // drawing into a view writes the source
    let view = Texture2d::view(&source, UncompressedFloatFormat::U8U8U8U8, 0..1, 0..1).unwrap();
    view.as_surface().clear_color(0.0, 1.0, 0.0, 1.0);
    let data: Vec<Vec<Pixel>> = source.read().unwrap();
    assert_eq!(data[2][2], (0, 255, 0, 255));

    // an sRGB view of linear texels samples them darker: 128 in sRGB is about 55 in linear
    let gray = Texture2d::with_format(
        &display,
        vec![vec![(128u8, 128u8, 128u8, 255u8); 4]; 4],
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::NoMipmap,
    )
    .unwrap();
    let srgb = SrgbTexture2d::view(&gray, SrgbFormat::U8U8U8U8, 0..1, 0..1).unwrap();
    let sample = program(
        &display,
        330,
        0,
        "uniform sampler2D tex;",
        "color = texture(tex, uv);",
    );
    let pixel = draw(&display, &sample, &uniform! { tex: &srgb }).unwrap()[0][0];
    assert!((54..=56).contains(&pixel.0), "{pixel:?}");
    assert_eq!(pixel.3, 255);

    // a layer of an array texture as a two-dimensional texture
    let array = Texture2dArray::with_format(
        &display,
        (0..3u8)
            .map(|layer| vec![vec![(layer * 100, 0u8, 0u8, 255u8); 2]; 2])
            .collect(),
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::NoMipmap,
    )
    .unwrap();
    let layer = Texture2d::view(&array, UncompressedFloatFormat::U8U8U8U8, 0..1, 2..3).unwrap();
    let data: Vec<Vec<Pixel>> = layer.read().unwrap();
    assert_eq!(data[0][0], (200, 0, 0, 255));

    // a range of levels: the view's level 0 is the source's level 1
    let levels = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::EmptyMipmaps,
        4,
        4,
    )
    .unwrap();
    levels
        .mipmap(1)
        .unwrap()
        .write(
            safegl::Rect {
                left: 0,
                bottom: 0,
                width: 2,
                height: 2,
            },
            vec![vec![(9u8, 8u8, 7u8, 6u8); 2]; 2],
        )
        .unwrap();
    let small = Texture2d::view(&levels, UncompressedFloatFormat::U8U8U8U8, 1..3, 0..1).unwrap();
    assert_eq!((small.width(), small.get_mipmap_levels()), (2, 2));
    let data: Vec<Vec<Pixel>> = small.read().unwrap();
    assert_eq!(data[1][1], (9, 8, 7, 6));

    // six layers of an array as a cube map
    let six = Texture2dArray::empty_with_format(
        &display,
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::NoMipmap,
        2,
        2,
        6,
    )
    .unwrap();
    let cube = Cubemap::view(&six, UncompressedFloatFormat::U8U8U8U8, 0..1, 0..6).unwrap();
    assert_eq!(cube.dimensions(), 2);

    // errors
    let f = UncompressedFloatFormat::U8U8U8U8;
    assert_eq!(
        Texture2d::view(&source, UncompressedFloatFormat::U16, 0..1, 0..1).err(),
        Some(TextureViewError::IncompatibleFormat)
    );
    assert_eq!(
        Texture3d::view(&source, f, 0..1, 0..1).err(),
        Some(TextureViewError::IncompatibleTarget)
    );
    assert_eq!(
        Texture2d::view(&source, f, 0..2, 0..1).err(),
        Some(TextureViewError::LevelsOutOfRange)
    );
    assert_eq!(
        Texture2d::view(&array, f, 0..1, 1..3).err(),
        Some(TextureViewError::LayersOutOfRange)
    );
    assert_eq!(
        Cubemap::view(&array, f, 0..1, 0..3).err(),
        Some(TextureViewError::LayersOutOfRange)
    );

    // a texture created outside SafeGL with `glTexImage2D` has mutable storage
    let mutable = unsafe {
        use safegl::glow::HasContext;
        let gl = display.glow_context();
        let id = gl.create_texture().unwrap();
        gl.bind_texture(safegl::glow::TEXTURE_2D, Some(id));
        gl.tex_image_2d(
            safegl::glow::TEXTURE_2D,
            0,
            safegl::glow::RGBA8 as i32,
            2,
            2,
            0,
            safegl::glow::RGBA,
            safegl::glow::UNSIGNED_BYTE,
            safegl::glow::PixelUnpackData::Slice(None),
        );
        display.restore_state();
        Texture2d::from_id(
            &display,
            f,
            id,
            true,
            MipmapsOption::NoMipmap,
            safegl::texture::Dimensions::Texture2d {
                width: 2,
                height: 2,
            },
        )
    };
    assert_eq!(
        Texture2d::view(&mutable, f, 0..1, 0..1).err(),
        Some(TextureViewError::SourceNotImmutable)
    );

    display.assert_no_error(None);
}

/// Textures compatible with the image formats of their class only refuse the other formats of
/// their size.
#[cfg(safegl_gl42)]
#[test]
fn image_format_classes() {
    use safegl::glow::HasContext;
    use safegl::texture::{MipmapsOption, UncompressedFloatFormat};
    use safegl::uniforms::{ImageUnitError, ImageUnitFormat};
    use safegl::GlObject;

    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }
    let texture = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::F11F11F10,
        MipmapsOption::NoMipmap,
        2,
        2,
    )
    .unwrap();
    let by_class = unsafe {
        let gl = display.glow_context();
        gl.bind_texture(safegl::glow::TEXTURE_2D, Some(texture.get_id()));
        let ty = gl.get_tex_parameter_i32(
            safegl::glow::TEXTURE_2D,
            safegl::glow::IMAGE_FORMAT_COMPATIBILITY_TYPE,
        ) as u32;
        display.restore_state();
        ty == safegl::glow::IMAGE_FORMAT_COMPATIBILITY_BY_CLASS
    };

    assert!(texture.image_unit(ImageUnitFormat::R11FG11FB10F).is_ok());
    let other = texture.image_unit(ImageUnitFormat::RGBA8);
    if by_class {
        assert!(matches!(other, Err(ImageUnitError::FormatClassMismatch)));
    } else {
        assert!(other.is_ok());
    }
    // a format of another size is always refused
    assert!(matches!(
        texture.image_unit(ImageUnitFormat::RG32F),
        Err(ImageUnitError::BadFormatClass(32, 64))
    ));

    display.assert_no_error(None);
}
