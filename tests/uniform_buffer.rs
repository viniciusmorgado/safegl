#[macro_use]
extern crate safegl;

use safegl::Surface;

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn uniform_buffer_creation() {
    let display = support::build_display();

    let _ = safegl::uniforms::UniformBuffer::new(&display, 12);

    display.assert_no_error(None);
}

#[test]
fn uniform_buffer_mapping_read() {
    let display = support::build_display();

    let mut vb = match safegl::uniforms::UniformBuffer::new(&display, 12) {
        Err(e) => panic!("{:?}", e),
        Ok(b) => b,
    };

    let mapping = vb.map();
    assert_eq!(*mapping, 12);

    display.assert_no_error(None);
}

#[test]
fn uniform_buffer_mapping_write() {
    let display = support::build_display();

    let mut vb = match safegl::uniforms::UniformBuffer::new(&display, 6) {
        Err(e) => panic!("{:?}", e),
        Ok(b) => b,
    };

    {
        let mut mapping = vb.map();
        *mapping = 15;
    }

    let mapping = vb.map();
    assert_eq!(*mapping, 15);

    display.assert_no_error(None);
}

#[test]
fn uniform_buffer_read() {
    let display = support::build_display();

    let vb = match safegl::uniforms::UniformBuffer::new(&display, 12) {
        Err(e) => panic!("{:?}", e),
        Ok(b) => b,
    };

    let data = match vb.read() {
        Ok(r) => r,
        Err(safegl::buffer::ReadError::NotSupported) => return,
        e => e.unwrap(),
    };

    assert_eq!(data, 12);

    display.assert_no_error(None);
}

#[test]
fn uniform_buffer_write() {
    let display = support::build_display();

    let vb = match safegl::uniforms::UniformBuffer::new(&display, 5) {
        Err(e) => panic!("{:?}", e),
        Ok(b) => b,
    };

    vb.write(&24);

    let data = match vb.read() {
        Ok(r) => r,
        Err(safegl::buffer::ReadError::NotSupported) => return,
        e => e.unwrap(),
    };

    assert_eq!(data, 24);

    display.assert_no_error(None);
}

#[test]
fn block() {
    let display = support::build_display();

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
            #version 330
            layout(std140) uniform;

            uniform MyBlock {
                vec3 color;
            };

            void main() {
                gl_FragColor = vec4(color, 1.0);
            }
        ",
        None,
    );

    let program = program.unwrap();

    #[derive(Copy, Clone)]
    struct Data {
        color: (f32, f32, f32),
    }

    implement_uniform_block!(Data, color);
    implement_any_bit_pattern!(Data, color);

    let buffer = match safegl::uniforms::UniformBuffer::new(
        &display,
        Data {
            color: (1.0f32, 1.0f32, 0.0f32),
        },
    ) {
        Err(e) => panic!("{:?}", e),
        Ok(b) => b,
    };

    let uniforms = uniform! {
        MyBlock: &buffer
    };

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(&vb, &ib, &program, &uniforms, &Default::default())
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    for row in data.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 255, 0, 255));
        }
    }

    display.assert_no_error(None);
}

#[test]
fn block_wrong_type() {
    let display = support::build_display();

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
            #version 330
            layout(std140) uniform;

            uniform MyBlock {
                vec3 color;
            };

            void main() {
                gl_FragColor = vec4(color, 1.0);
            }
        ",
        None,
    );

    let program = program.unwrap();

    let buffer = match safegl::uniforms::UniformBuffer::new(&display, 2) {
        Err(e) => panic!("{:?}", e),
        Ok(b) => b,
    };

    let uniforms = uniform! {
        MyBlock: &buffer
    };

    let mut target = display.draw();
    target.clear_color(0.0, 0.0, 0.0, 0.0);

    match target.draw(&vb, &ib, &program, &uniforms, &Default::default()) {
        Err(safegl::DrawError::UniformBlockLayoutMismatch { ref name, .. })
            if name == &"MyBlock" =>
        {
            ()
        }
        a => panic!("{:?}", a),
    }

    target.finish().unwrap();

    display.assert_no_error(None);
}

#[test]
fn buffer_write() {
    let display = support::build_display();

    let mut buf = match safegl::uniforms::UniformBuffer::new(&display, (5, 3)) {
        Err(e) => panic!("{:?}", e),
        Ok(b) => b,
    };

    {
        let mut mapping = buf.map();
        mapping.1 = 8;
    }

    let mapping = buf.map();
    assert_eq!(*mapping, (5, 8));

    display.assert_no_error(None);
}

#[test]
fn persistent_block_race_condition() {
    let display = support::build_display();

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
            #version 330
            layout(std140) uniform;

            uniform MyBlock {
                vec3 color;
            };

            void main() {
                gl_FragColor = vec4(color, 1.0);
            }
        ",
        None,
    );

    let program = program.unwrap();

    #[derive(Copy, Clone)]
    struct Data {
        color: (f32, f32, f32),
    }

    implement_uniform_block!(Data, color);
    implement_any_bit_pattern!(Data, color);

    let mut buffer = match safegl::uniforms::UniformBuffer::new(
        &display,
        Data {
            color: (0.5f32, 0.5f32, 0.5f32),
        },
    ) {
        Err(e) => panic!("{:?}", e),
        Ok(b) => b,
    };

    // checking for synchronization issues by quickly drawing and modifying the buffer
    let texture = support::build_renderable_texture(&display);
    let mut target = texture.as_surface();
    target.clear_color(0.0, 0.0, 0.0, 0.0);
    for i in 0..1000 {
        {
            let mut mapping = buffer.map();
            mapping.color.0 = (i % 7) as f32 / 7.0;
            mapping.color.1 = (i % 5) as f32 / 5.0;
            mapping.color.2 = (i % 3) as f32 / 3.0;
        }

        target
            .draw(
                &vb,
                &ib,
                &program,
                &uniform! {
                    MyBlock: &buffer
                },
                &Default::default(),
            )
            .unwrap();
    }
    {
        let mut mapping = buffer.map();
        mapping.color.0 = 1.0;
        mapping.color.1 = 1.0;
        mapping.color.2 = 1.0;
    }
    target
        .draw(
            &vb,
            &ib,
            &program,
            &uniform! {
                MyBlock: &buffer
            },
            &Default::default(),
        )
        .unwrap();
    {
        let mut mapping = buffer.map();
        mapping.color.0 = 0.0;
        mapping.color.1 = 0.0;
        mapping.color.2 = 0.0;
    }

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    for row in data.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 255, 255, 255));
        }
    }

    display.assert_no_error(None);
}

#[test]
fn empty_uniform_buffer() {
    let display = support::build_display();

    let _ = match safegl::uniforms::UniformBuffer::new(&display, ()) {
        Err(e) => panic!("{:?}", e),
        Ok(b) => b,
    };

    display.assert_no_error(None);
}
