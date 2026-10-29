//! EGL images, external textures and EGL fences, on the displays without window of EGL devices,
//! which share their images through dma-bufs.

#![cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]

#[macro_use]
extern crate safegl;

use std::time::Duration;

use safegl::backend::glutin::egl::{DmaBuf, Egl, EglError};
use safegl::backend::glutin::headless::{Headless, HeadlessError};
use safegl::backend::glutin::Display;
use safegl::glutin::surface::PbufferSurface;
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::{ExternalTexture, TextureCreationError, UncompressedFloatFormat};
use safegl::uniforms::SamplerWrapFunction;
use safegl::{DrawError, Program, Surface, Texture2d, VertexBuffer};

type Pixel = (u8, u8, u8, u8);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A display without window, and its EGL display; `None` without EGL devices.
fn display() -> Option<(Display<PbufferSurface>, Egl)> {
    let headless = match Headless::first() {
        Ok(headless) => headless,
        Err(HeadlessError::NoDevice) => return None,
        Err(error) => panic!("{error}"),
    };
    let display = headless.display(16, 16, Default::default()).unwrap();
    let egl = Egl::new(&display).unwrap();
    Some((display, egl))
}

/// A texture of 4x4 pixels whose rows have different colors.
fn source(display: &Display<PbufferSurface>) -> (Texture2d, Vec<Vec<Pixel>>) {
    let rows: Vec<Vec<Pixel>> = (0..4u8)
        .map(|row| vec![(row * 60, 255 - row * 60, 128, 255); 4])
        .collect();
    let texture = Texture2d::with_format(
        display,
        rows.clone(),
        UncompressedFloatFormat::U8U8U8U8,
        safegl::texture::MipmapsOption::NoMipmap,
    )
    .unwrap();
    (texture, rows)
}

/// A program that copies the external texture `image` to the target, in GLSL ES 3.00 with
/// `GL_OES_EGL_image_external_essl3`; `None` on desktop OpenGL and without the extension.
fn copy_program(display: &Display<PbufferSurface>) -> Option<Program> {
    use safegl::CapabilitiesSource;
    let extensions = display.get_extensions();
    let es = display.get_opengl_version().0 == safegl::Api::GlEs;
    if !es || !extensions.gl_oes_egl_image_external_essl3 {
        return None;
    }
    let vertex = "#version 300 es\nin vec2 position;\nout vec2 v_coordinates;\n\
                  void main() {\n\
                      v_coordinates = position * 0.5 + 0.5;\n\
                      gl_Position = vec4(position, 0.0, 1.0);\n\
                  }";
    let fragment = "#version 300 es\n#extension GL_OES_EGL_image_external_essl3 : require\n\
                    precision highp float;\nuniform samplerExternalOES image;\n\
                    in vec2 v_coordinates;\nout vec4 color;\n\
                    void main() { color = texture(image, v_coordinates); }";
    Some(Program::from_source(display, vertex, fragment, None).unwrap())
}

/// Draws the external texture over a 4x4 texture with `program`, and returns its pixels.
fn draw(
    display: &Display<PbufferSurface>,
    program: &Program,
    image: impl safegl::uniforms::AsUniformValue,
) -> Result<Vec<Vec<Pixel>>, DrawError> {
    let target = Texture2d::empty(display, 4, 4).unwrap();
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let triangle =
        VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap();
    target.as_surface().draw(
        &triangle,
        NoIndices(PrimitiveType::TrianglesList),
        program,
        &uniform! { image: image },
        &Default::default(),
    )?;
    Ok(target.read().unwrap())
}

/// A texture goes out as a dma-buf and comes back as an external texture and as the storage of
/// another texture, with the same pixels.
#[test]
fn dma_buf_round_trip() {
    let Some((display, egl)) = display() else {
        return;
    };
    let (texture, rows) = source(&display);
    let has = |name| egl.extensions().contains(name);
    if !has("EGL_KHR_gl_texture_2D_image")
        || !has("EGL_MESA_image_dma_buf_export")
        || !has("EGL_EXT_image_dma_buf_import")
    {
        return;
    }
    let image = egl.image_of_texture(&texture, 0).unwrap();
    assert_eq!((image.width(), image.height()), (4, 4));
    let dma_buf = image.export_dma_buf().unwrap();
    assert!(!dma_buf.planes.is_empty());
    let imported = egl.import_dma_buf(&dma_buf).unwrap();
    drop(dma_buf);

    // sampled as an external texture
    match (imported.external_texture(&display), copy_program(&display)) {
        (Ok(external), Some(program)) => {
            assert_eq!((external.width(), external.height()), (4, 4));
            assert_eq!(draw(&display, &program, &external).unwrap(), rows);
            assert_eq!(
                draw(&display, &program, external.sampled()).unwrap(),
                rows
            );
            check_sampler_rules(&display, &program, &external);
            // shaders write external textures in YUV with `GL_EXT_YUV_target`
            use safegl::CapabilitiesSource;
            let framebuffer = safegl::framebuffer::SimpleFrameBuffer::new(&display, &external);
            if !display.get_extensions().gl_ext_yuv_target {
                assert_eq!(
                    framebuffer.err(),
                    Some(safegl::framebuffer::ValidationError::FormatNotRenderable)
                );
            }
        }
        (Err(TextureCreationError::TypeNotSupported), _) => {
            assert!(!safegl::texture::is_external_texture_supported(&display));
        }
        (Ok(_), None) => (),
        (Err(error), _) => panic!("{error}"),
    }

    // the storage of another texture
    match imported.texture_2d(&display, UncompressedFloatFormat::U8U8U8U8) {
        Ok(copy) => {
            let pixels: Vec<Vec<Pixel>> = copy.read().unwrap();
            assert_eq!(pixels, rows);
        }
        Err(TextureCreationError::TypeNotSupported) => {
            use safegl::CapabilitiesSource;
            assert!(!display.get_extensions().gl_ext_egl_image_storage);
        }
        Err(error) => panic!("{error}"),
    }
    display.assert_no_error(None);
}

/// External textures are sampled without mipmaps, clamped to the edges unless the backend has
/// `GL_EXT_EGL_image_external_wrap_modes`.
fn check_sampler_rules(
    display: &Display<PbufferSurface>,
    program: &Program,
    external: &ExternalTexture,
) {
    use safegl::CapabilitiesSource;
    let mipmaps = external
        .sampled()
        .minify_filter(safegl::uniforms::MinifySamplerFilter::LinearMipmapLinear);
    assert!(matches!(
        draw(display, program, mipmaps),
        Err(DrawError::ExternalTextureSampler { .. })
    ));
    let repeat = external.sampled().wrap_function(SamplerWrapFunction::Repeat);
    let result = draw(display, program, repeat);
    if display.get_extensions().gl_ext_egl_image_external_wrap_modes {
        result.unwrap();
    } else {
        assert!(matches!(result, Err(DrawError::ExternalTextureSampler { .. })));
    }
}

/// Dma-bufs without planes or without size are refused before EGL sees them.
#[test]
fn invalid_dma_bufs() {
    let Some((_display, egl)) = display() else {
        return;
    };
    if !egl.extensions().contains("EGL_EXT_image_dma_buf_import") {
        return;
    }
    let empty = DmaBuf {
        width: 4,
        height: 4,
        fourcc: DmaBuf::fourcc(b"AB24"),
        modifier: None,
        planes: Vec::new(),
        color_space: None,
        range: None,
    };
    assert!(matches!(egl.import_dma_buf(&empty), Err(EglError::InvalidDmaBuf)));
}

/// EGL fences are signaled when the GPU has executed the commands before them, and native
/// fences go out as file descriptors and come back.
#[test]
fn fences() {
    let Some((display, egl)) = display() else {
        return;
    };
    let target = Texture2d::empty(&display, 4, 4).unwrap();
    target.as_surface().clear_color(1.0, 0.0, 0.0, 1.0);
    if egl.extensions().contains("EGL_KHR_fence_sync") {
        let fence = egl.fence().unwrap();
        assert!(fence.wait(Duration::from_secs(5)).unwrap());
    }
    if egl.extensions().contains("EGL_ANDROID_native_fence_sync") {
        let fence = egl.native_fence().unwrap();
        let fd = fence.native_fd().unwrap();
        let imported = egl.import_native_fence(fd).unwrap();
        if egl.extensions().contains("EGL_KHR_wait_sync") {
            imported.wait_on_gpu().unwrap();
        }
        assert!(imported.wait(Duration::from_secs(5)).unwrap());
    }
    display.assert_no_error(None);
}
