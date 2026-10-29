//! This file is not named `program.rs`, because executables that contain the string `program`
//! are treated in a special way by Windows.

#[macro_use]
extern crate safegl;

#[cfg(any(safegl_gl41, safegl_gles30))]
use safegl::Surface;

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn program_creation() {
    let display = support::build_display();

    // compiling shaders and linking them together
    support::program_from_legacy(
        &display,
        // vertex shader
        "
            #version 110

            uniform mat4 matrix;

            attribute vec2 position;
            attribute vec3 color;

            varying vec3 vColor;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0) * matrix;
                vColor = color;
            }
        ",
        // fragment shader
        "
            #version 110
            varying vec3 vColor;

            void main() {
                gl_FragColor = vec4(vColor, 1.0);
            }
        ",
    )
    .unwrap();

    display.assert_no_error(None);
}

#[test]
fn program_compilation_error() {
    let display = support::build_display();

    // compiling shaders and linking them together
    let program = safegl::Program::from_source(
        &display,
        // vertex shader
        "invalid glsl code",
        // fragment shader
        "
            #version 330

            out vec4 f_color;
            in vec3 vColor;

            void main() {
                f_color = vec4(vColor, 1.0);
            }
        ",
        // geometry shader
        None,
    );

    match program {
        Err(safegl::CompilationError(..)) => (),
        _ => panic!(),
    };

    display.assert_no_error(None);
}

#[test]
fn program_linking_error() {
    let display = support::build_display();
    let header = if support::is_gles(&display) {
        "#version 300 es\nprecision highp float;\n"
    } else {
        "#version 330\n"
    };

    // the uniform `u` is a `float` in the vertex shader and a `vec4` in the fragment shader,
    // which only the linker can find out (browsers check that every function called is defined
    // when they compile, and some drivers link outputs and inputs of different sizes)
    let program = safegl::Program::from_source(
        &display,
        &format!("{header}uniform float u;\nvoid main() {{ gl_Position = vec4(u); }}"),
        &format!("{header}uniform vec4 u;\nout vec4 color;\nvoid main() {{ color = u; }}"),
        None,
    );

    match program {
        Err(safegl::LinkingError(_)) => (),
        other => panic!("{:?}", other.err()),
    };

    display.assert_no_error(None);
}

#[test]
fn get_frag_data_location() {
    let display = support::build_display();

    let program = support::program_from_legacy(
        &display,
        "
            #version 110

            void main() {
                gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
            }
        ",
        "
            #version 130

            out vec4 color;

            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);
            }
        ",
    );

    let program = program.unwrap();

    assert!(program.get_frag_data_location("color").is_some());
    assert!(program.get_frag_data_location("unexisting").is_none());

    display.assert_no_error(None);
}

#[test]
fn get_uniform_blocks() {
    let display = support::build_display();

    if !display.is_glsl_version_supported(&safegl::Version(safegl::Api::Gl, 3, 3)) {
        return;
    }
    let program = safegl::Program::from_source(
        &display,
        "
            #version 330

            uniform MyBlock {
                vec3 position;
                float color[12];
            };

            void main() {
                gl_Position = vec4(position, color[2]);
            }
        ",
        "
            #version 330

            out vec4 color;

            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);
            }
        ",
        None,
    );

    let program = program.unwrap();

    let blocks = program.get_uniform_blocks();

    assert_eq!(blocks.len(), 1);
    assert!(blocks.get("MyBlock").is_some());

    let my_block = blocks.get("MyBlock").unwrap();
    assert!(my_block.size >= 3 * 4 + 4 * 12);

    if let safegl::program::BlockLayout::Struct { ref members } = my_block.layout {
        assert_eq!(members.len(), 2);

        let mut members = members.clone();
        members.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(members[1].0, "position");
        if let safegl::program::BlockLayout::BasicType {
            ty,
            offset_in_buffer,
        } = members[1].1
        {
            assert_eq!(ty, safegl::uniforms::UniformType::FloatVec3);
            assert_eq!(offset_in_buffer, 0);
        } else {
            panic!();
        }

        assert_eq!(members[0].0, "color");
        if let safegl::program::BlockLayout::Array {
            ref content,
            length,
            stride,
        } = members[0].1
        {
            assert_eq!(length, 12);
            assert!(stride >= 4);
            if let safegl::program::BlockLayout::BasicType {
                ty,
                offset_in_buffer,
            } = **content
            {
                assert_eq!(ty, safegl::uniforms::UniformType::Float);
                assert!(offset_in_buffer >= 4 * 3);
            } else {
                panic!();
            }
        } else {
            panic!();
        }
    } else {
        panic!();
    }

    display.assert_no_error(None);
}

#[cfg(any(safegl_gl41, safegl_gles30))]
#[test]
fn get_program_binary() {
    let display = support::build_display();

    let program = support::program_from_legacy(
        &display,
        "
            #version 110

            uniform mat4 matrix;

            attribute vec2 position;
            attribute vec3 color;

            varying vec3 vColor;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0) * matrix;
                vColor = color;
            }
        ",
        "
            #version 110
            varying vec3 vColor;

            void main() {
                gl_FragColor = vec4(vColor, 1.0);
            }
        ",
    )
    .unwrap();

    let binary = match program.get_binary() {
        Err(safegl::program::GetBinaryError::NotSupported) => return,
        Err(e) => panic!("{:?}", e),
        Ok(bin) => bin,
    };

    assert!(binary.content.len() >= 1);

    display.assert_no_error(None);
}

#[cfg(any(safegl_gl41, safegl_gles30))]
#[test]
fn program_binary_reload() {
    let display = support::build_display();

    let program = support::program_from_legacy(
        &display,
        "
            #version 110

            uniform mat4 matrix;

            attribute vec2 position;
            attribute vec3 color;

            varying vec3 vColor;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0) * matrix;
                vColor = color;
            }
        ",
        "
            #version 110
            varying vec3 vColor;

            void main() {
                gl_FragColor = vec4(vColor, 1.0);
            }
        ",
    )
    .unwrap();

    let binary = match program.get_binary() {
        Err(safegl::program::GetBinaryError::NotSupported) => return,
        Err(e) => panic!("{:?}", e),
        Ok(bin) => bin,
    };

    let _program2 = safegl::Program::new(&display, binary).unwrap();

    display.assert_no_error(None);
}

#[cfg(any(safegl_gl41, safegl_gles30))]
#[test]
fn program_binary_working() {
    let display = support::build_display();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);

    let program_src = support::program_from_legacy(
        &display,
        "
            #version 110

            attribute vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ",
        "
            #version 110

            void main() {
                gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
            }
        ",
    )
    .unwrap();

    let binary = match program_src.get_binary() {
        Err(safegl::program::GetBinaryError::NotSupported) => return,
        Err(e) => panic!("{:?}", e),
        Ok(bin) => bin,
    };

    if !display.is_glsl_version_supported(&safegl::Version(safegl::Api::GlEs, 3, 0)) {
        return;
    }
    let program = safegl::Program::new(&display, binary).unwrap();

    let output = support::build_renderable_texture(&display);
    output.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    output
        .as_surface()
        .draw(&vb, &ib, &program, &uniform! {}, &Default::default())
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
    for row in data.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 0, 0, 255));
        }
    }

    display.assert_no_error(None);
}

#[test]
fn get_transform_feedback_varyings() {
    let display = support::build_display();

    let (vertex_shader, fragment_shader) = if support::is_gles(&display) {
        (
            "#version 300 es
            out vec2 normal;
            out float color;
            void main() {
                normal = vec2(0.0, 0.0);
                color = 5.0;
                gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
            }",
            "#version 300 es
            precision mediump float;
            out vec4 color;
            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);
            }",
        )
    } else {
        (
            "
            #version 330

            out vec2 normal;
            out float color;

            void main() {
                normal = vec2(0.0, 0.0);
                color = 5.0;

                gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
            }
        ",
            "
            #version 330

            out vec4 color;

            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);
            }
        ",
        )
    };

    let source = safegl::program::ProgramCreationInput::SourceCode {
        tessellation_control_shader: None,
        tessellation_evaluation_shader: None,
        geometry_shader: None,
        outputs_srgb: false,
        uses_point_size: false,
        locations: Default::default(),

        vertex_shader,
        fragment_shader,

        transform_feedback_varyings: Some((
            vec!["normal".to_string(), "color".to_string()],
            safegl::program::TransformFeedbackMode::Separate,
        )),
    };

    let program = match safegl::Program::new(&display, source) {
        Ok(p) => p,
        Err(e) => panic!("{:?}", e),
    };

    assert_eq!(
        program.get_transform_feedback_buffers()[0],
        safegl::program::TransformFeedbackBuffer {
            id: 0,
            stride: 2 * 4,
            elements: vec![safegl::program::TransformFeedbackVarying {
                name: "normal".to_string(),
                offset: 0,
                size: 2 * 4,
                ty: safegl::vertex::AttributeType::F32F32,
            }]
        }
    );

    assert_eq!(
        program.get_transform_feedback_buffers()[1],
        safegl::program::TransformFeedbackBuffer {
            id: 1,
            stride: 4,
            elements: vec![safegl::program::TransformFeedbackVarying {
                name: "color".to_string(),
                offset: 0,
                size: 4,
                ty: safegl::vertex::AttributeType::F32,
            }]
        }
    );

    assert_eq!(program.get_transform_feedback_buffers().len(), 2);

    display.assert_no_error(None);
}

#[test]
fn get_transform_feedback_skipped_components_and_buffers() {
    let display = support::build_display();

    // `gl_SkipComponents` and `gl_NextBuffer` are desktop OpenGL 4.0 names
    if support::is_gles(&display) {
        return;
    }

    let source = safegl::program::ProgramCreationInput::SourceCode {
        tessellation_control_shader: None,
        tessellation_evaluation_shader: None,
        geometry_shader: None,
        outputs_srgb: false,
        uses_point_size: false,
        locations: Default::default(),

        vertex_shader: "
            #version 400

            out vec2 normal;
            out float color;
            out vec4 tangent;

            void main() {
                normal = vec2(0.0, 0.0);
                color = 5.0;
                tangent = vec4(1.0);
                gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
            }
        ",
        fragment_shader: "
            #version 400

            out vec4 color;

            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);
            }
        ",

        transform_feedback_varyings: Some((
            [
                "normal",
                "gl_SkipComponents2",
                "color",
                "gl_NextBuffer",
                "gl_SkipComponents1",
                "tangent",
                "gl_SkipComponents3",
            ]
            .iter()
            .map(|name| name.to_string())
            .collect(),
            safegl::program::TransformFeedbackMode::Interleaved,
        )),
    };

    let program = safegl::Program::new(&display, source).unwrap();

    assert_eq!(
        program.get_transform_feedback_buffers(),
        &[
            safegl::program::TransformFeedbackBuffer {
                id: 0,
                stride: (2 + 2 + 1) * 4,
                elements: vec![
                    safegl::program::TransformFeedbackVarying {
                        name: "normal".to_string(),
                        offset: 0,
                        size: 2 * 4,
                        ty: safegl::vertex::AttributeType::F32F32,
                    },
                    safegl::program::TransformFeedbackVarying {
                        name: "color".to_string(),
                        offset: (2 + 2) * 4,
                        size: 4,
                        ty: safegl::vertex::AttributeType::F32,
                    },
                ]
            },
            safegl::program::TransformFeedbackBuffer {
                id: 1,
                stride: (1 + 4 + 3) * 4,
                elements: vec![safegl::program::TransformFeedbackVarying {
                    name: "tangent".to_string(),
                    offset: 4,
                    size: 4 * 4,
                    ty: safegl::vertex::AttributeType::F32F32F32F32,
                }]
            },
        ]
    );

    display.assert_no_error(None);
}

#[test]
fn get_output_primitives_simple() {
    let display = support::build_display();

    let program = support::program_from_legacy(
        &display,
        "
            #version 110

            void main() {
                gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
            }
        ",
        "
            #version 110

            void main() {
                gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
            }
        ",
    );

    let program = program.unwrap();

    assert!(program.get_output_primitives().is_none());
    display.assert_no_error(None);
}

// TODO: add tests for get_output_primitives with geometry shader, TES, and both

#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn ssbos() {
    let display = support::build_display();

    if !display.is_glsl_version_supported(&safegl::Version(safegl::Api::Gl, 4, 3)) {
        return;
    }
    let program = safegl::Program::from_source(
        &display,
        "
            #version 430

            buffer MyBuffer {
                vec3 position;
                float color[12];
            };

            void main() {
                gl_Position = vec4(position, color[2]);
            }
        ",
        "
            #version 330

            out vec4 color;

            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);
            }
        ",
        None,
    );

    let program = program.unwrap();

    let blocks = program.get_shader_storage_blocks();

    assert_eq!(blocks.len(), 1);
    assert!(blocks.get("MyBuffer").is_some());

    let my_block = blocks.get("MyBuffer").unwrap();
    assert!(my_block.size >= 3 * 4 + 4 * 12);

    if let safegl::program::BlockLayout::Struct { ref members } = my_block.layout {
        assert_eq!(members.len(), 2);

        let mut members = members.clone();
        members.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(members[1].0, "position");
        if let safegl::program::BlockLayout::BasicType {
            ty,
            offset_in_buffer,
        } = members[1].1
        {
            assert_eq!(ty, safegl::uniforms::UniformType::FloatVec3);
            assert_eq!(offset_in_buffer, 0);
        } else {
            panic!();
        }

        assert_eq!(members[0].0, "color");
        if let safegl::program::BlockLayout::Array {
            ref content,
            length,
            stride,
        } = members[0].1
        {
            assert_eq!(length, 12);
            assert!(stride >= 4);
            if let safegl::program::BlockLayout::BasicType {
                ty,
                offset_in_buffer,
            } = **content
            {
                assert_eq!(ty, safegl::uniforms::UniformType::Float);
                assert!(offset_in_buffer >= 4 * 3);
            } else {
                panic!();
            }
        } else {
            panic!();
        }
    } else {
        panic!();
    }

    display.assert_no_error(None);
}

#[test]
fn complex_layout() {
    let display = support::build_display();

    if !display.is_glsl_version_supported(&safegl::Version(safegl::Api::Gl, 3, 3)) {
        return;
    }
    let program = safegl::Program::from_source(
        &display,
        "
            #version 330
            layout(std140) uniform;

            struct Foo {
                ivec3 a[3];
                int b;
            };

            uniform MyBlock {
                vec3 position;
                float color[12];
                Foo foo[2];
                Foo bar;
            };

            void main() {
                gl_Position = vec4(position, 1.0);
            }
        ",
        "
            #version 330

            out vec4 color;

            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);
            }
        ",
        None,
    );

    let program = program.unwrap();

    let blocks = program.get_uniform_blocks();

    assert_eq!(blocks.len(), 1);
    assert!(blocks.get("MyBlock").is_some());

    let my_block = blocks.get("MyBlock").unwrap();
    //assert_eq!(my_block.size, 3 * 4 + 4 * 12 + (3 * 4 + 4) * 3 + );      // TODO: sort offsets out

    if let safegl::program::BlockLayout::Struct { ref members } = my_block.layout {
        assert_eq!(members.len(), 4);

        let mut members = members.clone();
        members.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(members[0].0, "bar");
        if let safegl::program::BlockLayout::Struct { ref members } = members[0].1 {
            assert_eq!(members.len(), 2);
            let mut members = members.clone();
            members.sort_by(|a, b| a.0.cmp(&b.0));

            assert_eq!(members[0].0, "a");
            if let safegl::program::BlockLayout::Array {
                ref content,
                length,
                stride,
            } = members[0].1
            {
                assert_eq!(length, 3);
                assert_eq!(stride, 16);
                if let safegl::program::BlockLayout::BasicType { ty, .. } = **content {
                    assert_eq!(ty, safegl::uniforms::UniformType::IntVec3);
                    //assert_eq!(offset_in_buffer, 4 * 3);      // TODO: sort offsets out
                } else {
                    panic!();
                }
            }

            assert_eq!(members[1].0, "b");
            if let safegl::program::BlockLayout::BasicType { ty, .. } = members[1].1 {
                assert_eq!(ty, safegl::uniforms::UniformType::Int);
                //assert_eq!(offset_in_buffer, 0);      // TODO: sort offsets out
            } else {
                panic!();
            }
        } else {
            panic!();
        }

        assert_eq!(members[1].0, "color");
        if let safegl::program::BlockLayout::Array {
            ref content,
            length,
            stride,
        } = members[1].1
        {
            assert_eq!(length, 12);
            assert_eq!(stride, 16);
            if let safegl::program::BlockLayout::BasicType { ty, .. } = **content {
                assert_eq!(ty, safegl::uniforms::UniformType::Float);
                //assert_eq!(offset_in_buffer, 4 * 3);      // TODO: sort offsets out
            } else {
                panic!();
            }
        } else {
            panic!();
        }

        assert_eq!(members[2].0, "foo");
        if let safegl::program::BlockLayout::Array {
            ref content,
            length,
            stride,
        } = members[2].1
        {
            assert_eq!(length, 2);
            assert_eq!(stride, 64);

            if let safegl::program::BlockLayout::Struct { ref members } = **content {
                assert_eq!(members.len(), 2);
                let mut members = members.clone();
                members.sort_by(|a, b| a.0.cmp(&b.0));

                assert_eq!(members[0].0, "a");
                if let safegl::program::BlockLayout::Array {
                    ref content,
                    length,
                    stride,
                } = members[0].1
                {
                    assert_eq!(length, 3);
                    assert_eq!(stride, 16);
                    if let safegl::program::BlockLayout::BasicType { ty, .. } = **content {
                        assert_eq!(ty, safegl::uniforms::UniformType::IntVec3);
                        //assert!(offset_in_buffer >= 4 * 3);       // TODO: sort offsets out
                    } else {
                        panic!();
                    }
                }

                assert_eq!(members[1].0, "b");
                if let safegl::program::BlockLayout::BasicType { ty, .. } = members[1].1 {
                    assert_eq!(ty, safegl::uniforms::UniformType::Int);
                    //assert_eq!(offset_in_buffer, 0);      // TODO: sort offsets out
                } else {
                    panic!();
                }
            } else {
                panic!();
            }
        } else {
            panic!();
        }

        assert_eq!(members[3].0, "position");
        if let safegl::program::BlockLayout::BasicType { ty, .. } = members[3].1 {
            assert_eq!(ty, safegl::uniforms::UniformType::FloatVec3);
            //assert_eq!(offset_in_buffer, 0);      // TODO: sort offsets out
        } else {
            panic!();
        }
    } else {
        panic!();
    }

    display.assert_no_error(None);
}

#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn unsized_array() {
    let display = support::build_display();

    if !display.is_glsl_version_supported(&safegl::Version(safegl::Api::Gl, 4, 3)) {
        return;
    }
    let program = safegl::Program::from_source(
        &display,
        "
            #version 430

            struct Foo {
                ivec3 a[3];
                int b;
            };

            layout(std140) buffer MyBlock {
                vec3 position;
                Foo foo[1];
                Foo bar[];
            };

            void main() {
                gl_Position = vec4(position, 1.0);
            }
        ",
        "
            #version 140

            out vec4 color;

            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);
            }
        ",
        None,
    );

    let program = program.unwrap();

    let blocks = program.get_shader_storage_blocks();

    assert_eq!(blocks.len(), 1);
    let my_block = blocks.get("MyBlock").unwrap();

    assert_eq!(
        my_block.layout,
        safegl::program::BlockLayout::Struct {
            members: vec![
                (
                    "position".to_string(),
                    safegl::program::BlockLayout::BasicType {
                        ty: safegl::uniforms::UniformType::FloatVec3,
                        offset_in_buffer: 0,
                    }
                ),
                (
                    "foo".to_string(),
                    safegl::program::BlockLayout::Array {
                        content: Box::new(safegl::program::BlockLayout::Struct {
                            members: vec![
                                (
                                    "a".to_string(),
                                    safegl::program::BlockLayout::Array {
                                        content: Box::new(safegl::program::BlockLayout::BasicType {
                                            ty: safegl::uniforms::UniformType::IntVec3,
                                            offset_in_buffer: 16,
                                        }),
                                        length: 3,
                                        stride: 16,
                                    }
                                ),
                                (
                                    "b".to_string(),
                                    safegl::program::BlockLayout::BasicType {
                                        ty: safegl::uniforms::UniformType::Int,
                                        offset_in_buffer: 64,
                                    }
                                ),
                            ],
                        }),
                        length: 1,
                        stride: 64,
                    }
                ),
                (
                    "bar".to_string(),
                    safegl::program::BlockLayout::DynamicSizedArray {
                        content: Box::new(safegl::program::BlockLayout::Struct {
                            members: vec![
                                (
                                    "a".to_string(),
                                    safegl::program::BlockLayout::Array {
                                        content: Box::new(safegl::program::BlockLayout::BasicType {
                                            ty: safegl::uniforms::UniformType::IntVec3,
                                            offset_in_buffer: 80,
                                        }),
                                        length: 3,
                                        stride: 16,
                                    }
                                ),
                                (
                                    "b".to_string(),
                                    safegl::program::BlockLayout::BasicType {
                                        ty: safegl::uniforms::UniformType::Int,
                                        offset_in_buffer: 128,
                                    }
                                ),
                            ],
                        }),
                        stride: 64,
                    }
                ),
            ]
        }
    );

    display.assert_no_error(None);
}

#[test]
fn array_layout_offsets() {
    let display = support::build_display();

    if !display.is_glsl_version_supported(&safegl::Version(safegl::Api::Gl, 3, 3)) {
        return;
    }
    let program = safegl::Program::from_source(
        &display,
        "
            #version 330

            struct Foo {
                vec2 pos;
                vec2 dir;
                float speed;
            };

            layout(std140)
            uniform MyBlock {
                Foo data[256];
            };

            void main() {
                gl_Position = vec4(data[0].pos, 0.0, 1.0);
            }
        ",
        "
            #version 140

            out vec4 color;

            void main() {
                color = vec4(1.0, 1.0, 1.0, 1.0);
            }
        ",
        None,
    );

    let program = program.unwrap();

    let blocks = program.get_uniform_blocks();

    assert_eq!(blocks.len(), 1);
    let my_block = blocks.get("MyBlock").unwrap();

    assert_eq!(
        my_block.layout,
        safegl::program::BlockLayout::Struct {
            members: vec![(
                "data".to_string(),
                safegl::program::BlockLayout::Array {
                    content: Box::new(safegl::program::BlockLayout::Struct {
                        members: vec![
                            (
                                "pos".to_string(),
                                safegl::program::BlockLayout::BasicType {
                                    ty: safegl::uniforms::UniformType::FloatVec2,
                                    offset_in_buffer: 0,
                                }
                            ),
                            (
                                "dir".to_string(),
                                safegl::program::BlockLayout::BasicType {
                                    ty: safegl::uniforms::UniformType::FloatVec2,
                                    offset_in_buffer: 8,
                                }
                            ),
                            (
                                "speed".to_string(),
                                safegl::program::BlockLayout::BasicType {
                                    ty: safegl::uniforms::UniformType::Float,
                                    offset_in_buffer: 16,
                                }
                            ),
                        ],
                    }),
                    length: 256,
                    stride: 32,
                }
            ),]
        }
    );

    display.assert_no_error(None);
}

#[cfg(any(safegl_gl41, safegl_gles30))]
#[test]
fn precision_of_shaders() {
    use safegl::program::{PrecisionQualifier, ShaderType};

    let display = support::build_display();

    let highp = match display
        .shader_precision_format(ShaderType::Fragment, PrecisionQualifier::HighFloat)
    {
        Some(highp) => highp,
        None => {
            use safegl::CapabilitiesSource;
            assert!(!support::is_gles(&display));
            assert!(!display.get_extensions().gl_arb_es2_compatibility);
            return;
        }
    };
    // the highp floats of the fragment shaders of OpenGL ES 3.0 are 32-bit floats
    assert!(highp.range.0 >= 126 && highp.range.1 >= 127);
    assert!(highp.precision >= 23);

    let int = display
        .shader_precision_format(ShaderType::Vertex, PrecisionQualifier::HighInt)
        .unwrap();
    assert!(int.range.1 >= 30);
    assert_eq!(int.precision, 0);

    assert_eq!(
        display.shader_precision_format(ShaderType::Compute, PrecisionQualifier::HighFloat),
        None
    );

    display.assert_no_error(None);
}

/// Every qualifier of the vertex and fragment shaders has at least the precision of OpenGL ES 3.0,
/// whose `mediump` and `lowp` are less precise on mobile GPUs than on desktop drivers, which
/// compute them as `highp`. The formats are printed, for the runs on devices
/// (`--nocapture`).
#[cfg(any(safegl_gl41, safegl_gles30))]
#[test]
fn precision_of_every_qualifier() {
    use safegl::program::{PrecisionQualifier as Q, ShaderType};

    let display = support::build_display();
    // the minimum range and precision of each qualifier in OpenGL ES 3.0
    let minimums = [
        (Q::HighFloat, (126, 127), 23),
        (Q::MediumFloat, (14, 14), 10),
        (Q::LowFloat, (1, 1), 8),
        (Q::HighInt, (31, 30), 0),
        (Q::MediumInt, (15, 14), 0),
        (Q::LowInt, (8, 7), 0),
    ];
    for shader in [ShaderType::Vertex, ShaderType::Fragment] {
        for (qualifier, range, precision) in minimums {
            let Some(format) = display.shader_precision_format(shader, qualifier) else {
                return;
            };
            eprintln!("{shader:?} {qualifier:?}: {format:?}");
            assert!(
                format.range.0 >= range.0 && format.range.1 >= range.1,
                "{shader:?} {qualifier:?}: {format:?}"
            );
            assert!(format.precision >= precision, "{shader:?} {qualifier:?}: {format:?}");
        }
    }
    display.assert_no_error(None);
}

/// Programs are linked with `GL_PROGRAM_BINARY_RETRIEVABLE_HINT`, so that their binaries can be
/// retrieved on every driver.
#[cfg(any(safegl_gl41, safegl_gles30))]
#[test]
fn programs_are_linked_with_the_binary_hint() {
    use safegl::glow::HasContext;
    use safegl::GlObject;

    let display = support::build_display();
    if !safegl::program::is_binary_supported(&display) {
        return;
    }
    let (_, _, program) = support::build_fullscreen_red_pipeline(&display);
    let hint = unsafe {
        display.glow_context().get_program_parameter_i32(
            program.get_id(),
            safegl::glow::PROGRAM_BINARY_RETRIEVABLE_HINT,
        )
    };
    assert_eq!(hint, 1);

    display.assert_no_error(None);
}
