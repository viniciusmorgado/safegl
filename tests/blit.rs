#[macro_use]
extern crate safegl;

use safegl::framebuffer::SimpleFrameBuffer;
use safegl::uniforms::MagnifySamplerFilter;
use safegl::{BlitMask, BlitTarget, Rect, Surface};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn blit_texture_to_window() {
    let display = support::build_display();

    let src_rect = Rect {
        left: 0,
        bottom: 0,
        width: 2,
        height: 2,
    };

    let dest_rect = BlitTarget {
        left: 1,
        bottom: 1,
        width: 2,
        height: 2,
    };

    let texture = support::build_unicolor_texture2d(&display, 0.0, 1.0, 0.0);

    let target = support::build_renderable_texture(&display);
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    texture.as_surface().blit_color(
        &src_rect,
        &target.as_surface(),
        &dest_rect,
        safegl::uniforms::MagnifySamplerFilter::Nearest,
    ).unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();

    assert_eq!(data[1][1], (0, 255, 0, 255));
    assert_eq!(data[1][2], (0, 255, 0, 255));
    assert_eq!(data[2][1], (0, 255, 0, 255));
    assert_eq!(data[2][2], (0, 255, 0, 255));

    assert_eq!(data[0][0], (0, 0, 0, 0));

    assert_eq!(data[0][1], (0, 0, 0, 0));
    assert_eq!(data[0][2], (0, 0, 0, 0));

    assert_eq!(data[1][0], (0, 0, 0, 0));
    assert_eq!(data[2][0], (0, 0, 0, 0));

    assert_eq!(data[3][1], (0, 0, 0, 0));
    assert_eq!(data[3][2], (0, 0, 0, 0));

    assert_eq!(data[2][3], (0, 0, 0, 0));
    assert_eq!(data[1][3], (0, 0, 0, 0));

    assert_eq!(data[3][3], (0, 0, 0, 0));

    display.assert_no_error(None);
}

#[test]
fn blit_color_and_depth_buffer() {
    let display = support::build_display();

    // source frame buffer
    let src_tex_color = support::build_unicolor_texture2d(&display, 0.0, 0.5, 1.0);
    let src_tex_depth = support::build_constant_depth_texture(&display, 0.5);
    let src_frame_buffer =
        SimpleFrameBuffer::with_depth_buffer(&display, &src_tex_color, &src_tex_depth).unwrap();

    // destination frame buffer
    let dst_tex_color = support::build_unicolor_texture2d(&display, 0.0, 0.0, 0.0);
    let dst_tex_depth = support::build_constant_depth_texture(&display, 0.0);
    let dst_frame_buffer =
        SimpleFrameBuffer::with_depth_buffer(&display, &dst_tex_color, &dst_tex_depth).unwrap();

    // blit
    let src_rect = Rect {
        left: 0,
        bottom: 0,
        width: 2,
        height: 2,
    };
    let dst_rect = BlitTarget {
        left: 0,
        bottom: 0,
        width: 2,
        height: 2,
    };
    dst_frame_buffer.blit_buffers_from_simple_framebuffer(
        &src_frame_buffer,
        &src_rect,
        &dst_rect,
        MagnifySamplerFilter::Nearest,
        BlitMask::color_and_depth(),
    ).unwrap();

    // check result
    let color_data: Vec<Vec<(u8, u8, u8, u8)>> = dst_tex_color.read().unwrap();
    assert_eq!(
        color_data,
        vec![
            vec![(0, 127, 255, 255), (0, 127, 255, 255),],
            vec![(0, 127, 255, 255), (0, 127, 255, 255),],
        ]
    );
    // todo: how to check dst_tex_depth? There is no .read() on a DepthTexture2d...
    display.assert_no_error(None);
}

#[test]
fn blit_from_multi_output_framebuffer() {
    let display = support::build_display();

    let first = support::build_unicolor_texture2d(&display, 0.0, 0.0, 1.0);
    let second = support::build_unicolor_texture2d(&display, 1.0, 0.0, 0.0);
    let source = safegl::framebuffer::MultiOutputFrameBuffer::new(
        &display,
        [("first", &first), ("second", &second)],
    )
    .unwrap();

    // blits read from the first output
    let target = safegl::Texture2d::empty(&display, 2, 2).unwrap();
    source.blit_whole_color_to(
        &target.as_surface(),
        &BlitTarget {
            left: 0,
            bottom: 0,
            width: 2,
            height: 2,
        },
        MagnifySamplerFilter::Nearest,
    ).unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    assert_eq!(data[0][0], (0, 0, 255, 255));
    display.assert_no_error(None);
}

fn rect(size: u32) -> Rect {
    Rect {
        left: 0,
        bottom: 0,
        width: size,
        height: size,
    }
}

fn target(size: i32) -> BlitTarget {
    BlitTarget {
        left: 0,
        bottom: 0,
        width: size,
        height: size,
    }
}

#[test]
fn multisample_resolve() {
    use safegl::framebuffer::RenderBuffer;
    use safegl::texture::UncompressedFloatFormat;

    let display = support::build_display();
    let multisample =
        match RenderBuffer::new_multisample(&display, UncompressedFloatFormat::U8U8U8U8, 16, 16, 4)
        {
            Ok(buffer) => buffer,
            Err(e) => panic!("{:?}", e),
        };
    let mut source = SimpleFrameBuffer::new(&display, &multisample).unwrap();
    source.clear_color(0.0, 1.0, 0.0, 1.0);

    // resolving the samples to a target of the same size works
    let resolved = support::build_renderable_texture(&display);
    source
        .blit_color(&rect(16), &resolved.as_surface(), &target(16), MagnifySamplerFilter::Nearest)
        .unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = resolved.read().unwrap();
    assert_eq!(data[0][0], (0, 255, 0, 255));

    // but not with scaling
    assert_eq!(
        source.blit_color(&rect(16), &resolved.as_surface(), &target(8), MagnifySamplerFilter::Nearest),
        Err(safegl::BlitError::ScaledMultisample)
    );

    // OpenGL ES can't blit to a multisampled target
    let other = RenderBuffer::new_multisample(&display, UncompressedFloatFormat::U8U8U8U8, 16, 16, 4)
        .unwrap();
    let other = SimpleFrameBuffer::new(&display, &other).unwrap();
    let result =
        source.blit_color(&rect(16), &other, &target(16), MagnifySamplerFilter::Nearest);
    if support::is_gles(&display) {
        assert_eq!(result, Err(safegl::BlitError::MultisampleTarget));
    } else {
        result.unwrap();
    }

    display.assert_no_error(None);
}

#[test]
fn integer_blits() {
    let display = support::build_display();

    let unsigned = safegl::texture::UnsignedTexture2d::empty(&display, 16, 16).unwrap();
    let unsigned_target = safegl::texture::UnsignedTexture2d::empty(&display, 16, 16).unwrap();
    let source = SimpleFrameBuffer::new(&display, &unsigned).unwrap();
    let unsigned_target = SimpleFrameBuffer::new(&display, &unsigned_target).unwrap();
    let float_target = support::build_renderable_texture(&display);

    source
        .blit_color(&rect(16), &unsigned_target, &target(16), MagnifySamplerFilter::Nearest)
        .unwrap();
    assert_eq!(
        source.blit_color(&rect(16), &unsigned_target, &target(8), MagnifySamplerFilter::Linear),
        Err(safegl::BlitError::LinearFilterOnIntegers)
    );
    assert_eq!(
        source.blit_color(&rect(16), &float_target.as_surface(), &target(16), MagnifySamplerFilter::Nearest),
        Err(safegl::BlitError::FormatMismatch)
    );

    display.assert_no_error(None);
}

#[test]
fn depth_formats_must_match() {
    use safegl::framebuffer::DepthRenderBuffer;
    use safegl::texture::DepthFormat;

    let display = support::build_display();

    let color = support::build_renderable_texture(&display);
    let other_color = support::build_renderable_texture(&display);
    let depth16 = DepthRenderBuffer::new(&display, DepthFormat::I16, 1024, 1024).unwrap();
    let depth32 = DepthRenderBuffer::new(&display, DepthFormat::F32, 1024, 1024).unwrap();
    let other16 = DepthRenderBuffer::new(&display, DepthFormat::I16, 1024, 1024).unwrap();
    let source = SimpleFrameBuffer::with_depth_buffer(&display, &color, &depth16).unwrap();
    let same = SimpleFrameBuffer::with_depth_buffer(&display, &other_color, &other16).unwrap();
    let different = SimpleFrameBuffer::with_depth_buffer(&display, &other_color, &depth32).unwrap();

    let mask = BlitMask {
        color: false,
        depth: true,
        stencil: false,
    };
    source
        .blit_buffers_from_simple_framebuffer(&same, &rect(16), &target(16), MagnifySamplerFilter::Nearest, mask)
        .unwrap();
    assert_eq!(
        source.blit_buffers_from_simple_framebuffer(&different, &rect(16), &target(16), MagnifySamplerFilter::Nearest, mask),
        Err(safegl::BlitError::FormatMismatch)
    );

    display.assert_no_error(None);
}

#[test]
fn blit_to_itself() {
    let display = support::build_display();

    let texture = support::build_renderable_texture(&display);
    let surface = texture.as_surface();
    let result = surface.blit_color(
        &rect(8),
        &surface,
        &BlitTarget {
            left: 16,
            bottom: 16,
            width: 8,
            height: 8,
        },
        MagnifySamplerFilter::Nearest,
    );
    if support::is_gles(&display) {
        assert_eq!(result, Err(safegl::BlitError::SameSourceAndTarget));
    } else {
        result.unwrap();
    }

    display.assert_no_error(None);
}

/// An empty framebuffer has no color to copy.
#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn blit_from_empty_framebuffer() {
    use safegl::framebuffer::EmptyFrameBuffer;

    let display = support::build_display();
    if !EmptyFrameBuffer::is_supported(&display) {
        return;
    }
    let empty = EmptyFrameBuffer::new(&display, 4, 4, None, None, true).unwrap();
    let target = support::build_renderable_texture(&display);
    assert_eq!(
        empty.blit_whole_color_to(
            &target.as_surface(),
            &BlitTarget {
                left: 0,
                bottom: 0,
                width: 4,
                height: 4,
            },
            MagnifySamplerFilter::Nearest,
        ),
        Err(safegl::BlitError::NoColorBuffer)
    );

    display.assert_no_error(None);
}
