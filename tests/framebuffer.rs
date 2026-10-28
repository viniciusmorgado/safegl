#[macro_use]
extern crate safegl;

use safegl::Surface;

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn no_depth_buffer_depth_test() {
    let display = support::build_display();
    let (vertex_buffer, index_buffer, program) = support::build_fullscreen_red_pipeline(&display);

    let texture = safegl::texture::Texture2d::empty_with_format(
        &display,
        safegl::texture::UncompressedFloatFormat::U8U8U8U8,
        safegl::texture::MipmapsOption::NoMipmap,
        128,
        128,
    )
    .unwrap();
    let mut framebuffer = safegl::framebuffer::SimpleFrameBuffer::new(&display, &texture).unwrap();

    let parameters = safegl::DrawParameters {
        depth: safegl::Depth {
            test: safegl::DepthTest::IfLess,
            ..Default::default()
        },
        ..Default::default()
    };

    match framebuffer.draw(
        &vertex_buffer,
        &index_buffer,
        &program,
        &safegl::uniforms::EmptyUniforms,
        &parameters,
    ) {
        Err(safegl::DrawError::NoDepthBuffer) => (),
        a => panic!("{:?}", a),
    };

    display.assert_no_error(None);
}

#[test]
fn no_depth_buffer_depth_write() {
    let display = support::build_display();
    let (vertex_buffer, index_buffer, program) = support::build_fullscreen_red_pipeline(&display);

    let texture = safegl::texture::Texture2d::empty_with_format(
        &display,
        safegl::texture::UncompressedFloatFormat::U8U8U8U8,
        safegl::texture::MipmapsOption::NoMipmap,
        128,
        128,
    )
    .unwrap();
    let mut framebuffer = safegl::framebuffer::SimpleFrameBuffer::new(&display, &texture).unwrap();

    let parameters = safegl::DrawParameters {
        depth: safegl::Depth {
            write: true,
            ..Default::default()
        },
        ..Default::default()
    };

    match framebuffer.draw(
        &vertex_buffer,
        &index_buffer,
        &program,
        &safegl::uniforms::EmptyUniforms,
        &parameters,
    ) {
        Err(safegl::DrawError::NoDepthBuffer) => (),
        a => panic!("{:?}", a),
    };

    display.assert_no_error(None);
}

#[test]
fn simple_dimensions() {
    let display = support::build_display();

    let texture = safegl::Texture2d::empty_with_format(
        &display,
        safegl::texture::UncompressedFloatFormat::U8U8U8U8,
        safegl::texture::MipmapsOption::NoMipmap,
        128,
        128,
    )
    .unwrap();

    let framebuffer = safegl::framebuffer::SimpleFrameBuffer::new(&display, &texture).unwrap();
    assert_eq!(framebuffer.get_dimensions(), (128, 128));

    display.assert_no_error(None);
}

#[test]
fn simple_render_to_texture() {
    let display = support::build_display();
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);

    let texture = safegl::Texture2d::empty_with_format(
        &display,
        safegl::texture::UncompressedFloatFormat::U8U8U8U8,
        safegl::texture::MipmapsOption::NoMipmap,
        128,
        128,
    )
    .unwrap();

    let mut framebuffer = safegl::framebuffer::SimpleFrameBuffer::new(&display, &texture).unwrap();
    framebuffer
        .draw(
            &vb,
            &ib,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();

    let read_back: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();

    assert_eq!(read_back[0][0], (255, 0, 0, 255));
    assert_eq!(read_back[64][64], (255, 0, 0, 255));
    assert_eq!(read_back[127][127], (255, 0, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn depth_texture2d() {
    use std::iter;

    let display = support::build_display();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);

    // the program returns a Z coordinate between 0 (left of screen) and 1 (right of screen)
    let program = program!(&display,
    330 => {
        vertex: "
                #version 330

                in vec2 position;

                void main() {
                    gl_Position = vec4(position, position.x, 1.0);
                }
            ",
        fragment: "
                #version 330

                out vec4 f_color;

                void main() {
                    f_color = vec4(1.0, 1.0, 1.0, 1.0);
                }
            ",
    },
    100 => {
        vertex: "
                #version 100

                attribute lowp vec2 position;

                void main() {
                    gl_Position = vec4(position, position.x, 1.0);
                }
            ",
        fragment: "
                #version 100

                void main() {
                    gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
                }
            ",
    })
    .unwrap();

    // empty color attachment to put the data
    let color = safegl::Texture2d::empty_with_format(
        &display,
        safegl::texture::UncompressedFloatFormat::U8U8U8U8,
        safegl::texture::MipmapsOption::NoMipmap,
        128,
        128,
    )
    .unwrap();

    // depth texture with a value of 0.5 everywhere
    let depth_data = iter::repeat(iter::repeat(0.5f32).take(128).collect::<Vec<_>>())
        .take(128)
        .collect::<Vec<_>>();
    let depth = match safegl::texture::DepthTexture2d::new(&display, depth_data) {
        Err(safegl::texture::TextureCreationError::TypeNotSupported)
            | Err(safegl::texture::TextureCreationError::FormatNotSupported) => return,
        Err(e) => panic!("{:?}", e),
        Ok(t) => t,
    };

    // drawing with the `IfLess` depth test
    let mut framebuffer =
        safegl::framebuffer::SimpleFrameBuffer::with_depth_buffer(&display, &color, &depth).unwrap();
    let params = safegl::DrawParameters {
        depth: safegl::Depth {
            test: safegl::DepthTest::IfLess,
            ..Default::default()
        },
        ..Default::default()
    };

    framebuffer.clear_color(0.0, 0.0, 0.0, 1.0);
    framebuffer
        .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &params)
        .unwrap();

    // reading back the color
    let read_back: Vec<Vec<(u8, u8, u8, u8)>> = color.read().unwrap();

    assert_eq!(read_back[0][0], (255, 255, 255, 255));
    assert_eq!(read_back[127][127], (0, 0, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn multioutput() {
    let display = support::build_display();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);

    // OpenGL ES needs a location for each output when there are several
    let (header, color1, color2) = if support::is_gles(&display) {
        (
            "#version 300 es\nprecision highp float;\n",
            "layout(location = 0) out vec4 color1;",
            "layout(location = 1) out vec4 color2;",
        )
    } else {
        ("#version 330\n", "out vec4 color1;", "out vec4 color2;")
    };
    let program = safegl::Program::from_source(
        &display,
        &format!(
            "{header}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}"
        ),
        &format!(
            "{header}{color1}\n{color2}\n\
             void main() {{ color1 = vec4(1.0, 1.0, 1.0, 1.0); color2 = vec4(1.0, 0.0, 0.0, 1.0); }}"
        ),
        None,
    )
    .unwrap();

    // building two empty color attachments
    let color1 = safegl::Texture2d::empty_with_format(
        &display,
        safegl::texture::UncompressedFloatFormat::U8U8U8U8,
        safegl::texture::MipmapsOption::AutoGeneratedMipmaps,
        128,
        128,
    )
    .unwrap();
    color1.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);

    let color2 = safegl::Texture2d::empty_with_format(
        &display,
        safegl::texture::UncompressedFloatFormat::U8U8U8U8,
        safegl::texture::MipmapsOption::AutoGeneratedMipmaps,
        128,
        128,
    )
    .unwrap();
    color2.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);

    // building the framebuffer
    let mut framebuffer = safegl::framebuffer::MultiOutputFrameBuffer::new(
        &display,
        [("color1", &color1), ("color2", &color2)].iter().cloned(),
    )
    .unwrap();

    framebuffer
        .draw(
            &vb,
            &ib,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();

    // checking color1
    let read_back1: Vec<Vec<(u8, u8, u8, u8)>> = color1.read().unwrap();
    for row in read_back1.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 255, 255, 255));
        }
    }

    // checking color2
    let read_back2: Vec<Vec<(u8, u8, u8, u8)>> = color2.read().unwrap();
    for row in read_back2.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 0, 0, 255));
        }
    }

    display.assert_no_error(None);
}

#[test]
fn array_level() {
    let display = support::build_display();

    let texture = match safegl::texture::Texture2dArray::empty(&display, 128, 128, 4) {
        Ok(t) => t,
        Err(e) => panic!("{:?}", e),
    };

    let mut framebuffer = safegl::framebuffer::SimpleFrameBuffer::new(
        &display,
        texture.main_level().layer(2).unwrap(),
    )
    .unwrap();
    assert_eq!(framebuffer.get_dimensions(), (128, 128));

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    framebuffer
        .draw(
            &vb,
            &ib,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();

    // TODO: read the texture to see if it succeeded

    display.assert_no_error(None);
}

#[test]
fn cubemap_layer() {
    // ignoring test on travis
    // TODO: find out why they are failing
    if ::std::env::var("TRAVIS").is_ok() {
        return;
    }

    let display = support::build_display();

    let texture = match safegl::texture::Cubemap::empty(&display, 128) {
        Ok(t) => t,
        Err(e) => panic!("{:?}", e),
    };

    let mut framebuffer = safegl::framebuffer::SimpleFrameBuffer::new(
        &display,
        texture
            .main_level()
            .image(safegl::texture::CubeLayer::PositiveY),
    )
    .unwrap();
    assert_eq!(framebuffer.get_dimensions(), (128, 128));

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    framebuffer
        .draw(
            &vb,
            &ib,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();

    // TODO: read the texture to see if it succeeded

    display.assert_no_error(None);
}

#[test]
#[should_panic]
fn multi_color_attachments_maximum() {
    let display = support::build_display();

    let color_textures = (0..32)
        .map(|_| {
            safegl::Texture2d::empty_with_format(
                &display,
                safegl::texture::UncompressedFloatFormat::U8U8U8U8,
                safegl::texture::MipmapsOption::NoMipmap,
                128,
                128,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    let colors = (0..color_textures.len()).map(|i| ("attachment", &color_textures[i]));
    safegl::framebuffer::MultiOutputFrameBuffer::new(&display, colors).unwrap();
}

#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn empty_framebuffer_wrong_layers() {
    use safegl::framebuffer::{EmptyFrameBuffer, ValidationError};

    let display = support::build_display();

    // ignore the test
    if !EmptyFrameBuffer::is_supported(&display) {
        return;
    }

    let fb = EmptyFrameBuffer::new(&display, 256, 256, Some(0), None, true);
    assert!(matches!(
        fb,
        Err(ValidationError::EmptyFramebufferUnsupportedDimensions)
    ));
}

#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn empty_framebuffer_wrong_samples() {
    use safegl::framebuffer::{EmptyFrameBuffer, ValidationError};

    let display = support::build_display();

    // ignore the test
    if !EmptyFrameBuffer::is_supported(&display) {
        return;
    }

    let fb = EmptyFrameBuffer::new(&display, 256, 256, None, Some(0), true);
    assert!(matches!(
        fb,
        Err(ValidationError::EmptyFramebufferUnsupportedDimensions)
    ));
}

#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn empty_framebuffer_width_out_of_range() {
    use safegl::framebuffer::{EmptyFrameBuffer, ValidationError};

    let display = support::build_display();

    // ignore the test
    if !EmptyFrameBuffer::is_supported(&display) {
        return;
    }

    let _fb = match EmptyFrameBuffer::new(&display, 4294967295, 256, None, None, true) {
        Err(ValidationError::EmptyFramebufferUnsupportedDimensions) => (),
        _ => panic!(),
    };

    display.assert_no_error(None);
}

#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn empty_framebuffer_height_out_of_range() {
    use safegl::framebuffer::{EmptyFrameBuffer, ValidationError};

    let display = support::build_display();

    // ignore the test
    if !EmptyFrameBuffer::is_supported(&display) {
        return;
    }

    let _fb = match EmptyFrameBuffer::new(&display, 256, 4294967295, None, None, true) {
        Err(ValidationError::EmptyFramebufferUnsupportedDimensions) => (),
        _ => panic!(),
    };

    display.assert_no_error(None);
}

#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn empty_framebuffer_layers_out_of_range() {
    use safegl::framebuffer::{EmptyFrameBuffer, ValidationError};

    let display = support::build_display();

    // ignore the test
    if !EmptyFrameBuffer::is_layered_supported(&display) {
        return;
    }

    let _fb = match EmptyFrameBuffer::new(&display, 256, 256, Some(4294967295), None, true) {
        Err(ValidationError::EmptyFramebufferUnsupportedDimensions) => (),
        _ => panic!(),
    };

    display.assert_no_error(None);
}

#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn empty_framebuffer_samples_out_of_range() {
    use safegl::framebuffer::{EmptyFrameBuffer, ValidationError};

    let display = support::build_display();

    // ignore the test
    if !EmptyFrameBuffer::is_supported(&display) {
        return;
    }

    let _fb = match EmptyFrameBuffer::new(&display, 256, 256, None, Some(4294967295), true) {
        Err(ValidationError::EmptyFramebufferUnsupportedDimensions) => (),
        _ => panic!(),
    };

    display.assert_no_error(None);
}

#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn empty_framebuffer_simple_draw() {
    use safegl::framebuffer::EmptyFrameBuffer;

    let display = support::build_display();
    let (vertex_buffer, index_buffer, program) = support::build_fullscreen_red_pipeline(&display);

    // ignore the test
    if !EmptyFrameBuffer::is_supported(&display) {
        return;
    }

    let mut fb = EmptyFrameBuffer::new(&display, 256, 256, None, None, true).unwrap();
    fb.clear_color(0.0, 0.0, 0.0, 0.0);
    fb.draw(
        &vertex_buffer,
        &index_buffer,
        &program,
        &safegl::uniforms::EmptyUniforms,
        &Default::default(),
    )
    .unwrap();

    display.assert_no_error(None);
}

/// The outputs can be listed in any order: each color is written by the output of its name,
/// whatever its location. OpenGL ES only accepts the color attachment n as the draw buffer n.
#[test]
fn multioutput_in_another_order() {
    use safegl::framebuffer::MultiOutputFrameBuffer;

    let display = support::build_display();
    let program = safegl::program!(&display,
        330 => {
            vertex: "#version 330 core
                in vec2 position;
                void main() { gl_Position = vec4(position, 0.0, 1.0); }",
            fragment: "#version 330 core
                layout(location = 0) out vec4 first;
                layout(location = 1) out vec4 second;
                void main() {
                    first = vec4(1.0, 0.0, 0.0, 1.0);
                    second = vec4(0.0, 1.0, 0.0, 1.0);
                }",
        },
        300 es => {
            vertex: "#version 300 es
                in vec2 position;
                void main() { gl_Position = vec4(position, 0.0, 1.0); }",
            fragment: "#version 300 es
                precision mediump float;
                layout(location = 0) out vec4 first;
                layout(location = 1) out vec4 second;
                void main() {
                    first = vec4(1.0, 0.0, 0.0, 1.0);
                    second = vec4(0.0, 1.0, 0.0, 1.0);
                }",
        },
    )
    .unwrap();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let first = safegl::Texture2d::empty(&display, 4, 4).unwrap();
    let second = safegl::Texture2d::empty(&display, 4, 4).unwrap();
    let mut framebuffer =
        MultiOutputFrameBuffer::new(&display, [("second", &second), ("first", &first)]).unwrap();
    framebuffer.clear_color(0.0, 0.0, 0.0, 0.0);
    framebuffer
        .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &Default::default())
        .unwrap();

    let read = |texture: &safegl::Texture2d| {
        let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
        data[2][2]
    };
    assert_eq!(read(&first), (255, 0, 0, 255));
    assert_eq!(read(&second), (0, 255, 0, 255));

    display.assert_no_error(None);
}

/// A `DefaultFramebuffer` draws into the color buffer it was given, and blits from it.
#[test]
fn default_framebuffer_buffers() {
    use safegl::framebuffer::{
        DefaultFramebuffer, DefaultFramebufferAttachment, DefaultFramebufferAttachmentMissingError,
    };
    use safegl::uniforms::MagnifySamplerFilter;
    use safegl::BlitTarget;

    let display = support::build_display();
    let mut back =
        DefaultFramebuffer::new(&display, DefaultFramebufferAttachment::BackLeft).unwrap();
    let front = DefaultFramebuffer::new(&display, DefaultFramebufferAttachment::FrontLeft);
    if support::is_gles(&display) {
        assert_eq!(front.err(), Some(DefaultFramebufferAttachmentMissingError));
        return;
    }
    // contexts without stereoscopy have no right buffers
    let stereo = unsafe {
        use safegl::glow::HasContext;
        display.glow_context().get_parameter_bool(safegl::glow::STEREO)
    };
    if !stereo {
        let right = DefaultFramebuffer::new(&display, DefaultFramebufferAttachment::BackRight);
        assert!(right.is_err());
    }
    // the front buffer of most window surfaces of EGL doesn't exist
    let Ok(mut front) = front else {
        return;
    };

    back.clear_color(1.0, 0.0, 0.0, 1.0);
    front.clear_color(0.0, 1.0, 0.0, 1.0);
    let copy = |source: &DefaultFramebuffer| {
        let target = safegl::Texture2d::empty(&display, 4, 4).unwrap();
        source
            .blit_whole_color_to(
                &target.as_surface(),
                &BlitTarget {
                    left: 0,
                    bottom: 0,
                    width: 4,
                    height: 4,
                },
                MagnifySamplerFilter::Nearest,
            )
            .unwrap();
        let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
        data[0][0]
    };
    assert_eq!(copy(&front), (0, 255, 0, 255));
    assert_eq!(copy(&back), (255, 0, 0, 255));

    display.assert_no_error(None);
}

/// Render buffers can be outputs of a framebuffer with several outputs, like textures.
#[test]
fn multioutput_render_buffers() {
    use safegl::framebuffer::{MultiOutputFrameBuffer, RenderBuffer, SimpleFrameBuffer};
    use safegl::texture::UncompressedFloatFormat;

    let display = support::build_display();
    let first = RenderBuffer::new(&display, UncompressedFloatFormat::U8U8U8U8, 4, 4).unwrap();
    let second = RenderBuffer::new(&display, UncompressedFloatFormat::U8U8U8U8, 4, 4).unwrap();
    let framebuffer =
        MultiOutputFrameBuffer::new(&display, [("first", &first), ("second", &second)]).unwrap();
    framebuffer.clear_output("second", [0.0f32, 1.0, 0.0, 1.0]).unwrap();

    // the render buffer is read through a blit to a texture
    let target = safegl::Texture2d::empty(&display, 4, 4).unwrap();
    SimpleFrameBuffer::new(&display, &second)
        .unwrap()
        .blit_whole_color_to(
            &target.as_surface(),
            &safegl::BlitTarget {
                left: 0,
                bottom: 0,
                width: 4,
                height: 4,
            },
            safegl::uniforms::MagnifySamplerFilter::Nearest,
        )
        .unwrap();
    let read: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    assert_eq!(read[0][0], (0, 255, 0, 255));
    display.assert_no_error(None);
}
