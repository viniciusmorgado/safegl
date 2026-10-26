#![cfg(feature = "gl46")]

#[macro_use]
extern crate safegl;

use safegl::framebuffer::{RenderBuffer, SimpleFrameBuffer};
use safegl::index::PrimitiveType;
use safegl::texture::{
    copy_image, texture_barrier, ClearTextureError, CompressedFormat, CompressedMipmapsOption,
    CompressedTexture2d, CopyImageError, DepthStencilFormat, DepthStencilTexture2d, DepthTexture2d,
    MipmapsOption, Texture2dMultisample, UncompressedFloatFormat, UncompressedUintFormat,
    UnsignedTexture2d,
};
use safegl::{Surface, Texture2d};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

const SIZE: u32 = 8;

/// Whether textures can be cleared and copied; the tests that need more check it themselves.
fn supported(display: &Display) -> bool {
    safegl::texture::is_clear_texture_supported(display)
        && safegl::texture::is_copy_image_supported(display)
}

/// Creating textures of float, integer and depth formats with `empty` and `empty_with_format`
/// fails on OpenGL ES, because SafeGL doesn't use `glTexStorage*` there; the tests that need
/// such textures only run on desktop OpenGL.
fn is_gles(display: &Display) -> bool {
    display.get_opengl_version().0 == safegl::Api::GlEs
}

fn read(texture: &Texture2d) -> Vec<Vec<(u8, u8, u8, u8)>> {
    texture.read()
}

fn assert_all(texture: &Texture2d, expected: (u8, u8, u8, u8)) {
    for pixel in read(texture).iter().flatten() {
        assert_eq!(*pixel, expected);
    }
}

/// A texture whose texel `(x, y)` is `(x, y, 0, 255)`.
fn numbered_texture(display: &Display) -> Texture2d {
    let data: Vec<Vec<(u8, u8, u8, u8)>> = (0..SIZE as u8)
        .map(|y| (0..SIZE as u8).map(|x| (x, y, 0, 255)).collect())
        .collect();
    Texture2d::with_format(
        display,
        data,
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::NoMipmap,
    )
    .unwrap()
}

fn rgba8(display: &Display) -> Texture2d {
    Texture2d::empty_with_format(
        display,
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::NoMipmap,
        SIZE,
        SIZE,
    )
    .unwrap()
}

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// Draws a fullscreen quad at window depth 0.5, which writes white, or the texel under it of
/// `canvas` plus 0.25 if `add` is true.
fn draw_quad<S: Surface>(
    display: &Display,
    target: &mut S,
    canvas: Option<&Texture2d>,
    params: &safegl::DrawParameters<'_>,
) {
    let vertices = safegl::VertexBuffer::new(
        display,
        &[[-1.0, 1.0], [1.0, 1.0], [-1.0, -1.0], [1.0, -1.0]].map(|position| Vertex { position }),
    )
    .unwrap();
    let indices =
        safegl::IndexBuffer::new(display, PrimitiveType::TriangleStrip, &[0u8, 1, 2, 3]).unwrap();
    let program = program!(display,
        330 => {
            vertex: "
                #version 330 core
                in vec2 position;
                void main() { gl_Position = vec4(position, 0.0, 1.0); }
            ",
            fragment: "
                #version 330 core
                uniform sampler2D canvas;
                uniform bool add;
                out vec4 color;
                void main() {
                    color = add ? texelFetch(canvas, ivec2(gl_FragCoord.xy), 0) + 0.25 : vec4(1.0);
                }
            ",
        },
        300 es => {
            vertex: "
                #version 300 es
                in vec2 position;
                void main() { gl_Position = vec4(position, 0.0, 1.0); }
            ",
            fragment: "
                #version 300 es
                precision mediump float;
                uniform sampler2D canvas;
                uniform bool add;
                out vec4 color;
                void main() {
                    color = add ? texelFetch(canvas, ivec2(gl_FragCoord.xy), 0) + 0.25 : vec4(1.0);
                }
            ",
        },
    )
    .unwrap();

    let blank = rgba8(display);
    target
        .draw(
            &vertices,
            &indices,
            &program,
            &uniform! { canvas: canvas.unwrap_or(&blank), add: canvas.is_some() },
            params,
        )
        .unwrap();
}

#[test]
fn clear_color_texture() {
    let display = support::build_display();
    if !supported(&display) {
        return;
    }
    let texture = rgba8(&display);

    texture
        .main_level()
        .clear((0.25f32, 0.5f32, 0.75f32, 1.0f32))
        .unwrap();
    assert_all(&texture, (64, 128, 191, 255));

    // integers are normalized
    texture
        .main_level()
        .clear((255u8, 0u8, 0u8, 255u8))
        .unwrap();
    assert_all(&texture, (255, 0, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn clear_region() {
    let display = support::build_display();
    if !supported(&display) {
        return;
    }
    let texture = rgba8(&display);
    texture.main_level().clear((0u8, 0u8, 0u8, 255u8)).unwrap();

    texture
        .main_level()
        .clear_region((2, 4, 0), (3, 2, 1), (255u8, 255u8, 255u8, 255u8))
        .unwrap();

    for (y, row) in read(&texture).iter().enumerate() {
        for (x, pixel) in row.iter().enumerate() {
            let inside = (2..5).contains(&x) && (4..6).contains(&y);
            let expected = if inside { 255 } else { 0 };
            assert_eq!(
                *pixel,
                (expected, expected, expected, 255),
                "texel ({x}, {y})"
            );
        }
    }

    assert_eq!(
        texture.main_level().clear_region((6, 0, 0), (3, 1, 1), 0u8),
        Err(ClearTextureError::OutOfBounds)
    );

    display.assert_no_error(None);
}

#[test]
fn clear_unsigned_texture() {
    let display = support::build_display();
    if !supported(&display) || is_gles(&display) {
        return;
    }
    let texture = UnsignedTexture2d::empty_with_format(
        &display,
        UncompressedUintFormat::U32U32U32U32,
        MipmapsOption::NoMipmap,
        SIZE,
        SIZE,
    )
    .unwrap();

    texture
        .main_level()
        .clear((1u32, 2u32, 3u32, 4u32))
        .unwrap();
    let data: Vec<Vec<(u32, u32, u32, u32)>> = unsafe { texture.unchecked_read() };
    for pixel in data.iter().flatten() {
        assert_eq!(*pixel, (1, 2, 3, 4));
    }

    // integer textures need integer values
    assert_eq!(
        texture.main_level().clear((1.0f32, 0.0f32, 0.0f32, 0.0f32)),
        Err(ClearTextureError::FormatNotSupported)
    );

    display.assert_no_error(None);
}

#[test]
fn clear_depth_texture() {
    let display = support::build_display();
    if !supported(&display) || is_gles(&display) {
        return;
    }
    let color = rgba8(&display);
    let depth = DepthTexture2d::empty(&display, SIZE, SIZE).unwrap();
    let params = safegl::DrawParameters {
        depth: safegl::Depth {
            test: safegl::DepthTest::IfLess,
            ..Default::default()
        },
        ..Default::default()
    };

    // the quad at depth 0.5 is behind 0.25 and in front of 0.75
    depth.main_level().clear(0.25f32).unwrap();
    let mut target = SimpleFrameBuffer::with_depth_buffer(&display, &color, &depth).unwrap();
    target.clear_color(0.0, 0.0, 0.0, 1.0);
    draw_quad(&display, &mut target, None, &params);
    drop(target);
    assert_all(&color, (0, 0, 0, 255));

    depth.main_level().clear(0.75f32).unwrap();
    let mut target = SimpleFrameBuffer::with_depth_buffer(&display, &color, &depth).unwrap();
    draw_quad(&display, &mut target, None, &params);
    drop(target);
    assert_all(&color, (255, 255, 255, 255));

    // depth textures need an f32
    assert_eq!(
        depth.main_level().clear(1u8),
        Err(ClearTextureError::FormatNotSupported)
    );

    display.assert_no_error(None);
}

#[test]
fn clear_depth_stencil_texture() {
    let display = support::build_display();
    if !supported(&display) {
        return;
    }
    let color = rgba8(&display);
    let depth_stencil = DepthStencilTexture2d::empty_with_format(
        &display,
        DepthStencilFormat::I24I8,
        MipmapsOption::NoMipmap,
        SIZE,
        SIZE,
    )
    .unwrap();

    depth_stencil
        .main_level()
        .clear_depth_stencil(0.75, 7)
        .unwrap();

    // passes the depth test only if the depth is above 0.5, and the stencil test only if the
    // stencil is 7
    let params = safegl::DrawParameters {
        depth: safegl::Depth {
            test: safegl::DepthTest::IfLess,
            ..Default::default()
        },
        stencil: safegl::draw_parameters::Stencil {
            test_clockwise: safegl::StencilTest::IfEqual { mask: 0xff },
            test_counter_clockwise: safegl::StencilTest::IfEqual { mask: 0xff },
            reference_value_clockwise: 7,
            reference_value_counter_clockwise: 7,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut target =
        SimpleFrameBuffer::with_depth_stencil_buffer(&display, &color, &depth_stencil).unwrap();
    target.clear_color(0.0, 0.0, 0.0, 1.0);
    draw_quad(&display, &mut target, None, &params);
    drop(target);
    assert_all(&color, (255, 255, 255, 255));

    // `clear` doesn't take depth-stencil values
    assert_eq!(
        depth_stencil.main_level().clear(0.5f32),
        Err(ClearTextureError::FormatNotSupported)
    );

    display.assert_no_error(None);
}

#[test]
fn compressed_textures_cant_be_cleared() {
    let display = support::build_display();
    if !supported(&display) {
        return;
    }
    let texture = CompressedTexture2d::empty_with_format(
        &display,
        CompressedFormat::S3tcDxt5Alpha,
        CompressedMipmapsOption::NoMipmap,
        SIZE,
        SIZE,
    )
    .unwrap();

    assert_eq!(
        texture.main_level().clear((0u8, 0u8, 0u8, 0u8)),
        Err(ClearTextureError::FormatNotSupported)
    );
}

#[test]
fn copy_region_between_textures() {
    let display = support::build_display();
    if !supported(&display) {
        return;
    }
    let source = numbered_texture(&display);
    let destination = rgba8(&display);
    destination
        .main_level()
        .clear((0u8, 0u8, 0u8, 0u8))
        .unwrap();

    copy_image(
        source.main_level(),
        (1, 2, 0),
        destination.main_level(),
        (4, 4, 0),
        (4, 3, 1),
    )
    .unwrap();

    for (y, row) in read(&destination).iter().enumerate() {
        for (x, pixel) in row.iter().enumerate() {
            let expected = if (4..8).contains(&x) && (4..7).contains(&y) {
                (x as u8 - 3, y as u8 - 2, 0, 255)
            } else {
                (0, 0, 0, 0)
            };
            assert_eq!(*pixel, expected, "texel ({x}, {y})");
        }
    }

    // the whole level
    destination
        .main_level()
        .copy_to(&source.main_level())
        .unwrap();
    assert_eq!(read(&source), read(&destination));

    display.assert_no_error(None);
}

#[test]
fn copy_checks_regions() {
    let display = support::build_display();
    if !supported(&display) {
        return;
    }
    let source = numbered_texture(&display);
    let small = Texture2d::empty(&display, SIZE / 2, SIZE / 2).unwrap();

    assert_eq!(
        source.main_level().copy_to(&small.main_level()),
        Err(CopyImageError::OutOfBounds)
    );
    assert_eq!(
        copy_image(
            source.main_level(),
            (6, 0, 0),
            small.main_level(),
            (0, 0, 0),
            (4, 1, 1)
        ),
        Err(CopyImageError::OutOfBounds)
    );

    // within one texture, regions must not overlap
    assert_eq!(
        copy_image(
            source.main_level(),
            (0, 0, 0),
            source.main_level(),
            (2, 2, 0),
            (4, 4, 1)
        ),
        Err(CopyImageError::OverlappingRegions)
    );
    copy_image(
        source.main_level(),
        (0, 0, 0),
        source.main_level(),
        (4, 4, 0),
        (4, 4, 1),
    )
    .unwrap();
    assert_eq!(read(&source)[5][6], (2, 1, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn copy_checks_formats_and_samples() {
    let display = support::build_display();
    if !supported(&display) || is_gles(&display) {
        return;
    }
    let source = numbered_texture(&display);

    // 32 bits per texel: compatible, the bits are copied as they are
    let float = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::F32,
        MipmapsOption::NoMipmap,
        SIZE,
        SIZE,
    )
    .unwrap();
    source.main_level().copy_to(&float.main_level()).unwrap();
    float.main_level().copy_to(&source.main_level()).unwrap();
    assert_eq!(read(&source)[3][5], (5, 3, 0, 255));

    // 16 bits per texel
    let half = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::F16,
        MipmapsOption::NoMipmap,
        SIZE,
        SIZE,
    )
    .unwrap();
    assert_eq!(
        source.main_level().copy_to(&half.main_level()),
        Err(CopyImageError::IncompatibleFormats)
    );

    let multisample = Texture2dMultisample::empty(&display, SIZE, SIZE, 4).unwrap();
    assert_eq!(
        source.main_level().copy_to(&multisample.main_level()),
        Err(CopyImageError::SampleCountMismatch)
    );

    display.assert_no_error(None);
}

#[test]
fn copy_between_compressed_and_uncompressed() {
    let display = support::build_display();
    if !supported(&display) || is_gles(&display) {
        return;
    }

    // a DXT5 block is 128 bits, like a texel of RGBA32UI: 2x2 texels become 8x8 texels
    let blocks = UnsignedTexture2d::empty_with_format(
        &display,
        UncompressedUintFormat::U32U32U32U32,
        MipmapsOption::NoMipmap,
        2,
        2,
    )
    .unwrap();
    blocks.main_level().clear((0u32, 0u32, 0u32, 0u32)).unwrap();
    let compressed = CompressedTexture2d::empty_with_format(
        &display,
        CompressedFormat::S3tcDxt5Alpha,
        CompressedMipmapsOption::NoMipmap,
        SIZE,
        SIZE,
    )
    .unwrap();

    copy_image(
        blocks.main_level(),
        (0, 0, 0),
        compressed.main_level(),
        (0, 0, 0),
        (2, 2, 1),
    )
    .unwrap();
    copy_image(
        compressed.main_level(),
        (4, 0, 0),
        blocks.main_level(),
        (0, 1, 0),
        (4, 4, 1),
    )
    .unwrap();

    // regions of compressed images start on blocks
    assert_eq!(
        copy_image(
            compressed.main_level(),
            (2, 0, 0),
            blocks.main_level(),
            (0, 0, 0),
            (4, 4, 1)
        ),
        Err(CopyImageError::UnalignedCompressedRegion)
    );
    // a DXT5 block isn't the size of an RGBA8 texel
    let rgba = rgba8(&display);
    assert_eq!(
        copy_image(
            compressed.main_level(),
            (0, 0, 0),
            rgba.main_level(),
            (0, 0, 0),
            (4, 4, 1)
        ),
        Err(CopyImageError::IncompatibleFormats)
    );

    display.assert_no_error(None);
}

#[test]
fn copy_from_render_buffer() {
    let display = support::build_display();
    if !supported(&display) {
        return;
    }
    let render_buffer =
        RenderBuffer::new(&display, UncompressedFloatFormat::U8U8U8U8, SIZE, SIZE).unwrap();
    SimpleFrameBuffer::new(&display, &render_buffer)
        .unwrap()
        .clear_color(0.0, 1.0, 0.0, 1.0);

    let texture = rgba8(&display);
    copy_image(
        &render_buffer,
        (0, 0, 0),
        texture.main_level(),
        (0, 0, 0),
        (SIZE, SIZE, 1),
    )
    .unwrap();
    assert_all(&texture, (0, 255, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn texture_barrier_makes_writes_visible() {
    let display = support::build_display();
    if !supported(&display) {
        return;
    }
    if !safegl::texture::is_texture_barrier_supported(&display) {
        assert!(texture_barrier(&display).is_err());
        return;
    }

    let canvas = rgba8(&display);
    canvas.main_level().clear((0u8, 0u8, 0u8, 0u8)).unwrap();

    // each draw reads the texel it writes, and adds 0.25
    for _ in 0..2 {
        draw_quad(
            &display,
            &mut canvas.as_surface(),
            Some(&canvas),
            &Default::default(),
        );
        texture_barrier(&display).unwrap();
    }
    assert_all(&canvas, (128, 128, 128, 128));

    display.assert_no_error(None);
}
