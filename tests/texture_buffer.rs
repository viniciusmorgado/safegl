#![cfg(any(safegl_desktop, safegl_gles32))]

#[macro_use]
extern crate safegl;

use safegl::texture::buffer_texture::BufferTexture;
use safegl::texture::buffer_texture::BufferTextureType;
use safegl::Surface;

mod support;

#[test]
fn empty() {
    let display = support::build_display();

    let texture = BufferTexture::empty(&display, 32, BufferTextureType::Float);
    let texture: BufferTexture<(u8, u8, u8, u8)> = match texture {
        Ok(t) => t,
        Err(safegl::texture::buffer_texture::CreationError::TextureCreationError(
                safegl::texture::buffer_texture::TextureCreationError::NotSupported,
            )) => return,
        Err(e) => panic!("{:?}", e),
    };

    display.assert_no_error(None);
    drop(texture);
    display.assert_no_error(None);
}

#[test]
fn sample() {
    let display = support::build_display();

    let data = &[(255, 0, 255, 255)];
    let buf_tex = BufferTexture::new(&display, data, BufferTextureType::Float);
    let buf_tex: BufferTexture<(u8, u8, u8, u8)> = match buf_tex {
        Ok(t) => t,
        Err(safegl::texture::buffer_texture::CreationError::TextureCreationError(
                safegl::texture::buffer_texture::TextureCreationError::NotSupported,
            )) => return,
        Err(e) => panic!("{:?}", e),
    };

    let (vb, ib) = support::build_rectangle_vb_ib(&display);

    if !display.is_glsl_version_supported(&safegl::Version(safegl::Api::Gl, 3, 3)) {
        return;
    }
    let program = safegl::Program::from_source(
        &display,
        "
            #version 330

            in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ",
        "
            #version 140

            uniform samplerBuffer tex;

            void main() {
                gl_FragColor = texelFetch(tex, 0);
            }
        ",
        None,
    );
    let program = program.unwrap();

    let output = support::build_renderable_texture(&display);
    output.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    output
        .as_surface()
        .draw(
            &vb,
            &ib,
            &program,
            &uniform! { tex: &buf_tex },
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
    for row in data.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 0, 255, 255));
        }
    }

    display.assert_no_error(None);
}

#[test]
fn wrong_type() {
    let display = support::build_display();

    let data = &[(255, 0, 255, 255)];
    let buf_tex = BufferTexture::new(&display, data, BufferTextureType::Float);
    let buf_tex: BufferTexture<(u8, u8, u8, u8)> = match buf_tex {
        Ok(t) => t,
        Err(safegl::texture::buffer_texture::CreationError::TextureCreationError(
                safegl::texture::buffer_texture::TextureCreationError::NotSupported,
            )) => return,
        Err(e) => panic!("{:?}", e),
    };

    let (vb, ib) = support::build_rectangle_vb_ib(&display);

    if !display.is_glsl_version_supported(&safegl::Version(safegl::Api::Gl, 3, 3)) {
        return;
    }
    let program = safegl::Program::from_source(
        &display,
        "
            #version 330

            in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ",
        "
            #version 140

            uniform isamplerBuffer tex;

            void main() {
                gl_FragColor = texelFetch(tex, 0);
            }
        ",
        None,
    );
    let program = program.unwrap();

    let mut frame = display.draw();
    let result = frame.draw(
        &vb,
        &ib,
        &program,
        &uniform! { tex: &buf_tex },
        &Default::default(),
    );

    match result {
        Err(safegl::DrawError::UniformTypeMismatch { .. }) => (),
        _ => panic!(),
    };

    frame.finish().unwrap();

    display.assert_no_error(None);
}

/// Buffer textures of three 32-bit components, which only desktop OpenGL has.
#[cfg(safegl_desktop)]
#[test]
fn rgb32_texels() {
    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }
    let data = [(0.25f32, 0.5f32, 0.75f32), (1.0, 0.0, 1.0)];
    let texture: BufferTexture<(f32, f32, f32)> =
        match BufferTexture::new(&display, &data, BufferTextureType::Float) {
            Ok(texture) => texture,
            Err(e) => panic!("{:?}", e),
        };
    let program = support::program_from_legacy(
        &display,
        "attribute vec2 position; void main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "uniform samplerBuffer tex;
        void main() { gl_FragColor = vec4(texelFetch(tex, 1).rgb, 1.0); }",
    )
    .unwrap();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let output = support::build_renderable_texture(&display);
    output
        .as_surface()
        .draw(&vb, &ib, &program, &uniform! { tex: &texture }, &Default::default())
        .unwrap();
    let pixels: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
    assert_eq!(pixels[0][0], (255, 0, 255, 255));
    display.assert_no_error(None);
}
