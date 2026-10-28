#[macro_use]
extern crate safegl;

use safegl::index::PrimitiveType;
use safegl::Surface;

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn multiple_buffers_source() {
    let display = support::build_display();

    let buffer1 = {
        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 2],
        }

        implement_vertex!(Vertex, position);

        safegl::VertexBuffer::new(
            &display,
            &[
                Vertex {
                    position: [-1.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, -1.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                },
            ],
        )
        .unwrap()
    };

    let buffer2 = {
        #[derive(Copy, Clone)]
        struct Vertex {
            color: [f32; 3],
        }

        implement_vertex!(Vertex, color);

        safegl::VertexBuffer::new(
            &display,
            &[
                Vertex {
                    color: [1.0, 0.0, 0.0],
                },
                Vertex {
                    color: [1.0, 0.0, 0.0],
                },
                Vertex {
                    color: [1.0, 0.0, 0.0],
                },
                Vertex {
                    color: [1.0, 0.0, 0.0],
                },
            ],
        )
        .unwrap()
    };

    let index_buffer =
        safegl::IndexBuffer::new(&display, PrimitiveType::TriangleStrip, &[0u16, 1, 2, 3]).unwrap();

    let program = program!(&display,
    330 => {
        vertex: "
                #version 330

                in vec2 position;
                in vec3 color;

                out vec3 v_color;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_color = color;
                }
            ",
        fragment: "
                #version 330
                out vec4 f_color;

                in vec3 v_color;

                void main() {
                    f_color = vec4(v_color, 1.0);
                }
            ",
    },
    100 => {
        vertex: "
                #version 100

                attribute lowp vec2 position;
                attribute lowp vec3 color;

                varying lowp vec3 v_color;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_color = color;
                }
            ",
        fragment: "
                #version 100
                varying lowp vec3 v_color;

                void main() {
                    gl_FragColor = vec4(v_color, 1.0);
                }
            ",
    })
    .unwrap();

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            (&buffer1, &buffer2),
            &index_buffer,
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    for row in data.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 0, 0, 255));
        }
    }

    display.assert_no_error(None);
}

#[test]
fn slice_draw_indices() {
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
    }

    implement_vertex!(Vertex, position);

    let display = support::build_display();
    let program = program!(&display,
    330 => {
        vertex: "
                #version 330

                in vec2 position;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                }
            ",
        fragment: "
                #version 330

                out vec4 f_color;

                void main() {
                    f_color = vec4(1.0, 0.0, 0.0, 1.0);
                }
            ",
    },
    100 => {
        vertex: "
                #version 100

                attribute lowp vec2 position;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                }
            ",
        fragment: "
                #version 100

                void main() {
                    gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
                }
            ",
    })
    .unwrap();

    let vb = safegl::VertexBuffer::new(
        &display,
        &[
            Vertex {
                position: [-1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
            },
            Vertex {
                position: [-1.0, -1.0],
            },
            Vertex {
                position: [1.0, -1.0],
            },
        ],
    )
    .unwrap();

    let indices =
        safegl::IndexBuffer::new(&display, PrimitiveType::TrianglesList, &[0u16, 1, 2]).unwrap();

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            vb.slice(1..4).unwrap(),
            &indices,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(data.last().unwrap()[0], (0, 0, 0, 0));
    assert_eq!(data[0].last().unwrap(), &(255, 0, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn slice_draw_noindices() {
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
    }

    implement_vertex!(Vertex, position);

    let display = support::build_display();
    let program = program!(&display,
    330 => {
        vertex: "
                #version 330

                in vec2 position;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                }
            ",
        fragment: "
                #version 330

                out vec4 f_color;

                void main() {
                    f_color = vec4(1.0, 0.0, 0.0, 1.0);
                }
            ",
    },
    100 => {
        vertex: "
                #version 100

                attribute lowp vec2 position;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                }
            ",
        fragment: "
                #version 100

                void main() {
                    gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
                }
            ",
    })
    .unwrap();

    let vb = safegl::VertexBuffer::new(
        &display,
        &[
            Vertex {
                position: [-1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
            },
            Vertex {
                position: [-1.0, -1.0],
            },
            Vertex {
                position: [1.0, -1.0],
            },
        ],
    )
    .unwrap();

    let indices = safegl::index::NoIndices(safegl::index::PrimitiveType::TrianglesList);

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            vb.slice(1..4).unwrap(),
            &indices,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(data.last().unwrap()[0], (0, 0, 0, 0));
    assert_eq!(data[0].last().unwrap(), &(255, 0, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn slice_draw_multiple() {
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
    }

    implement_vertex!(Vertex, position);

    #[derive(Copy, Clone)]
    struct Vertex2 {
        position2: [f32; 2],
    }

    implement_vertex!(Vertex2, position2);

    let display = support::build_display();
    let program = program!(&display,
    330 => {
        vertex: "
                #version 330

                in vec2 position;
                in vec2 position2;

                void main() {
                    if (position != position2) {
                        gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
                    } else {
                        gl_Position = vec4(position, 0.0, 1.0);
                    }
                }
            ",
        fragment: "
                #version 330

                out vec4 f_color;

                void main() {
                    f_color = vec4(1.0, 0.0, 0.0, 1.0);
                }
            ",
    },

    100 => {
        vertex: "
                #version 100

                attribute lowp vec2 position;
                attribute lowp vec2 position2;

                void main() {
                    if (position != position2) {
                        gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
                    } else {
                        gl_Position = vec4(position, 0.0, 1.0);
                    }
                }
            ",
        fragment: "
                #version 100

                void main() {
                    gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
                }
            ",
    })
    .unwrap();

    // the 3 last elements will be drawn
    let vb1 = safegl::VertexBuffer::new(
        &display,
        &[
            Vertex {
                position: [-1.0, 1.0],
            },
            Vertex {
                position: [-1.0, 1.0],
            },
            Vertex {
                position: [-1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
            },
            Vertex {
                position: [-1.0, -1.0],
            },
            Vertex {
                position: [1.0, -1.0],
            },
        ],
    )
    .unwrap();

    // the 3 last elements will be drawn
    let vb2 = safegl::VertexBuffer::new(
        &display,
        &[
            Vertex2 {
                position2: [-1.0, 1.0],
            },
            Vertex2 {
                position2: [-1.0, 1.0],
            },
            Vertex2 {
                position2: [-1.0, 1.0],
            },
            Vertex2 {
                position2: [-1.0, 1.0],
            },
            Vertex2 {
                position2: [-1.0, 1.0],
            },
            Vertex2 {
                position2: [1.0, 1.0],
            },
            Vertex2 {
                position2: [-1.0, -1.0],
            },
            Vertex2 {
                position2: [1.0, -1.0],
            },
        ],
    )
    .unwrap();

    let indices =
        safegl::IndexBuffer::new(&display, PrimitiveType::TrianglesList, &[2u16, 3, 4]).unwrap();

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            (vb1.slice(1..4).unwrap(), vb2.slice(3..6).unwrap()),
            &indices,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(data.last().unwrap()[0], (0, 0, 0, 0));
    assert_eq!(data[0].last().unwrap(), &(255, 0, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn attributes_marker() {
    let display = support::build_display();

    let program = program!(&display,
        140 => {
            vertex: "
                #version 140

                void main() {
                    if (gl_VertexID == 0) {
                        gl_Position = vec4(-1.0, 1.0, 0.0, 1.0);
                    } else if (gl_VertexID == 1) {
                        gl_Position = vec4(1.0, 1.0, 0.0, 1.0);
                    } else if (gl_VertexID == 2) {
                        gl_Position = vec4(-1.0, -1.0, 0.0, 1.0);
                    } else if (gl_VertexID == 3) {
                        gl_Position = vec4(1.0, -1.0, 0.0, 1.0);
                    }
                }
            ",
            fragment: "
                #version 140

                out vec4 color;

                void main() {
                    color = vec4(1.0, 0.0, 0.0, 1.0);
                }
            ",
        },
        300 es => {
            vertex: "
                #version 300 es

                void main() {
                    if (gl_VertexID == 0) {
                        gl_Position = vec4(-1.0, 1.0, 0.0, 1.0);
                    } else if (gl_VertexID == 1) {
                        gl_Position = vec4(1.0, 1.0, 0.0, 1.0);
                    } else if (gl_VertexID == 2) {
                        gl_Position = vec4(-1.0, -1.0, 0.0, 1.0);
                    } else if (gl_VertexID == 3) {
                        gl_Position = vec4(1.0, -1.0, 0.0, 1.0);
                    }
                }
            ",
            fragment: "
                #version 300 es
                precision mediump float;

                out vec4 color;

                void main() {
                    color = vec4(1.0, 0.0, 0.0, 1.0);
                }
            ",
        },
    )
    .unwrap();

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            safegl::vertex::EmptyVertexAttributes { len: 4 },
            &safegl::index::NoIndices(safegl::index::PrimitiveType::TriangleStrip),
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    for row in data.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 0, 0, 255));
        }
    }

    display.assert_no_error(None);
}

#[test]
fn attributes_marker_indices() {
    let display = support::build_display();

    let program = program!(&display,
        140 => {
            vertex: "
                #version 140

                void main() {
                    if (gl_VertexID == 0) {
                        gl_Position = vec4(-1.0, 1.0, 0.0, 1.0);
                    } else if (gl_VertexID == 1) {
                        gl_Position = vec4(1.0, 1.0, 0.0, 1.0);
                    } else if (gl_VertexID == 2) {
                        gl_Position = vec4(-1.0, -1.0, 0.0, 1.0);
                    } else if (gl_VertexID == 3) {
                        gl_Position = vec4(1.0, -1.0, 0.0, 1.0);
                    }
                }
            ",
            fragment: "
                #version 140

                out vec4 color;

                void main() {
                    color = vec4(1.0, 0.0, 0.0, 1.0);
                }
            ",
        },
        300 es => {
            vertex: "
                #version 300 es

                void main() {
                    if (gl_VertexID == 0) {
                        gl_Position = vec4(-1.0, 1.0, 0.0, 1.0);
                    } else if (gl_VertexID == 1) {
                        gl_Position = vec4(1.0, 1.0, 0.0, 1.0);
                    } else if (gl_VertexID == 2) {
                        gl_Position = vec4(-1.0, -1.0, 0.0, 1.0);
                    } else if (gl_VertexID == 3) {
                        gl_Position = vec4(1.0, -1.0, 0.0, 1.0);
                    }
                }
            ",
            fragment: "
                #version 300 es
                precision mediump float;

                out vec4 color;

                void main() {
                    color = vec4(1.0, 0.0, 0.0, 1.0);
                }
            ",
        },
    )
    .unwrap();

    let indices =
        safegl::index::IndexBuffer::new(&display, PrimitiveType::TriangleStrip, &[0u16, 1, 2, 3])
            .unwrap();

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            safegl::vertex::EmptyVertexAttributes { len: 4 },
            &indices,
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    for row in data.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 0, 0, 255));
        }
    }

    display.assert_no_error(None);
}

#[test]
fn instancing() {
    let display = support::build_display();

    let buffer1 = {
        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 2],
        }

        implement_vertex!(Vertex, position);

        safegl::VertexBuffer::new(
            &display,
            &[
                Vertex {
                    position: [-1.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, -1.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                },
            ],
        )
        .unwrap()
    };

    let buffer2 = {
        #[derive(Copy, Clone)]
        struct Vertex {
            color: [f32; 3],
        }

        implement_vertex!(Vertex, color);

        safegl::vertex::VertexBuffer::new(
            &display,
            &[
                Vertex {
                    color: [0.0, 0.0, 1.0],
                },
                Vertex {
                    color: [0.0, 0.0, 1.0],
                },
                Vertex {
                    color: [0.0, 0.0, 1.0],
                },
                Vertex {
                    color: [1.0, 0.0, 0.0],
                },
            ],
        )
        .unwrap()
    };

    let buffer2 = buffer2.per_instance();

    let index_buffer =
        safegl::IndexBuffer::new(&display, PrimitiveType::TriangleStrip, &[0u16, 1, 2, 3]).unwrap();

    let program = program!(&display,
        330 => {
            vertex: "
                #version 330

                in vec2 position;
                in vec3 color;

                out vec3 v_color;
                flat out int instance;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_color = color;
                    instance = gl_InstanceID;
                }
            ",
            fragment: "
                #version 330
                in vec3 v_color;
                flat in int instance;
                out vec4 f_color;

                void main() {
                    if (instance != 3) {
                        discard;
                    }

                    f_color = vec4(v_color, 1.0);
                }
            ",
        },
        300 es => {
            vertex: "
                #version 300 es

                in vec2 position;
                in vec3 color;

                out vec3 v_color;
                flat out int instance;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_color = color;
                    instance = gl_InstanceID;
                }
            ",
            fragment: "
                #version 300 es
                precision mediump float;
                in vec3 v_color;
                flat in int instance;
                out vec4 f_color;

                void main() {
                    if (instance != 3) {
                        discard;
                    }

                    f_color = vec4(v_color, 1.0);
                }
            ",
        },
    )
    .unwrap();

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            (&buffer1, buffer2),
            &index_buffer,
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    for row in data.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(255, 0, 0, 255));
        }
    }

    display.assert_no_error(None);
}

#[test]
fn per_instance_length_mismatch() {
    let display = support::build_display();

    let buffer1 = {
        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 2],
        }

        implement_vertex!(Vertex, position);

        safegl::VertexBuffer::new(
            &display,
            &[
                Vertex {
                    position: [-1.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, -1.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                },
            ],
        )
        .unwrap()
    };

    let buffer2 = {
        #[derive(Copy, Clone)]
        struct Vertex {
            color: [f32; 3],
        }

        implement_vertex!(Vertex, color);

        safegl::vertex::VertexBuffer::new(
            &display,
            &[
                Vertex {
                    color: [0.0, 0.0, 1.0],
                },
                Vertex {
                    color: [0.0, 0.0, 1.0],
                },
                Vertex {
                    color: [0.0, 0.0, 1.0],
                },
                Vertex {
                    color: [1.0, 0.0, 0.0],
                },
            ],
        )
        .unwrap()
    };

    let buffer2 = buffer2.per_instance();

    let buffer3 = {
        #[derive(Copy, Clone)]
        struct Vertex {
            color: [f32; 3],
        }

        implement_vertex!(Vertex, color);

        safegl::vertex::VertexBuffer::new(
            &display,
            &[
                Vertex {
                    color: [0.0, 0.0, 1.0],
                },
                Vertex {
                    color: [0.0, 0.0, 1.0],
                },
                Vertex {
                    color: [0.0, 0.0, 1.0],
                },
            ],
        )
        .unwrap()
    };

    let buffer3 = buffer3.per_instance();

    let index_buffer =
        safegl::IndexBuffer::new(&display, PrimitiveType::TriangleStrip, &[0u16, 1, 2, 3]).unwrap();

    let program = program!(&display,
    330 => {
        vertex: "
                #version 330

                void main() {
                    gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
                }
            ",
        fragment: "
                #version 330

                out vec4 f_color;

                void main() {
                    f_color = vec4(0.0, 0.0, 0.0, 1.0);
                }
            ",
    },
    100 => {
        vertex: "
                #version 100

                void main() {
                    gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
                }
            ",
        fragment: "
                #version 100

                void main() {
                    gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
                }
            ",
    })
    .unwrap();

    let mut frame = display.draw();
    match frame.draw(
        (&buffer1, buffer2, buffer3),
        &index_buffer,
        &program,
        &uniform! {},
        &Default::default(),
    ) {
        Err(safegl::DrawError::InstancesCountMismatch) => (),
        a => panic!("{:?}", a),
    }

    frame.finish().unwrap();
    display.assert_no_error(None);
}

#[test]
fn normalized_vertex() {
    let display = support::build_display();

    let buffer1 = {
        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 2],
        }

        implement_vertex!(Vertex, position);

        safegl::VertexBuffer::new(
            &display,
            &[
                Vertex {
                    position: [-1.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, -1.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                },
            ],
        )
        .unwrap()
    };

    let buffer2 = {
        #[derive(Copy, Clone)]
        struct Vertex {
            color: [u8; 3],
        }

        implement_vertex!(Vertex, color normalize(true));

        safegl::VertexBuffer::new(
            &display,
            &[
                Vertex { color: [0, 0, 0] },
                Vertex { color: [128, 0, 0] },
                Vertex { color: [0, 128, 0] },
                Vertex { color: [0, 0, 128] },
            ],
        )
        .unwrap()
    };

    let index_buffer =
        safegl::IndexBuffer::new(&display, PrimitiveType::TriangleStrip, &[0u16, 1, 2, 3]).unwrap();

    let program = program!(&display,
    330 => {
        vertex: "
                #version 330

                in vec2 position;
                in vec3 color;

                out vec3 v_color;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_color = color;
                }
            ",
        fragment: "
                #version 330
                out vec4 f_color;

                in vec3 v_color;

                void main() {
                    f_color = vec4(v_color, 1.0);
                }
            ",
    },
    100 => {
        vertex: "
                #version 100

                attribute lowp vec2 position;
                attribute lowp vec3 color;

                varying lowp vec3 v_color;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_color = color;
                }
            ",
        fragment: "
                #version 100
                varying lowp vec3 v_color;

                void main() {
                    gl_FragColor = vec4(v_color, 1.0);
                }
            ",
    })
    .unwrap();

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(
            (&buffer1, &buffer2),
            &index_buffer,
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    // If the data wasn't normalized, the 128 would be 255 instead
    assert_eq!(data[0][0], (0, 128, 0, 255));

    display.assert_no_error(None);
}

/// A buffer of 2 colors with a divisor of 3 colors 6 instances, drawn in 6 columns.
#[test]
fn instance_divisor() {
    use std::num::NonZeroU32;

    let display = support::build_display();

    #[derive(Copy, Clone)]
    struct Corner {
        corner: [f32; 2],
    }
    implement_vertex!(Corner, corner);
    #[derive(Copy, Clone)]
    struct Column {
        column: f32,
    }
    implement_vertex!(Column, column);
    #[derive(Copy, Clone)]
    struct Color {
        color: [f32; 3],
    }
    implement_vertex!(Color, color);

    let corners =
        [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]].map(|corner| Corner { corner });
    let corners = safegl::VertexBuffer::new(&display, &corners).unwrap();
    let columns: Vec<Column> = (0..6).map(|column| Column { column: column as f32 }).collect();
    let columns = safegl::VertexBuffer::new(&display, &columns).unwrap();
    let colors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].map(|color| Color { color });
    let colors = safegl::VertexBuffer::new(&display, &colors).unwrap();

    let program = support::program_from_legacy(
        &display,
        "#version 110
         attribute vec2 corner;
         attribute float column;
         attribute vec3 color;
         varying vec3 v_color;
         void main() {
             v_color = color;
             vec2 position = vec2((column + corner.x) / 3.0 - 1.0, corner.y * 2.0 - 1.0);
             gl_Position = vec4(position, 0.0, 1.0);
         }",
        "#version 110
         varying vec3 v_color;
         void main() { gl_FragColor = vec4(v_color, 1.0); }",
    )
    .unwrap();

    let target = safegl::Texture2d::empty(&display, 6, 1).unwrap();
    target
        .as_surface()
        .draw(
            (
                &corners,
                columns.per_instance(),
                colors
                    .per_instance()
                    .with_divisor(NonZeroU32::new(3).unwrap()),
            ),
            &safegl::index::NoIndices(safegl::index::PrimitiveType::TriangleStrip),
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    let red = (255, 0, 0, 255);
    let green = (0, 255, 0, 255);
    assert_eq!(data[0], [red, red, red, green, green, green]);

    // the colors don't provide 6 instances without the divisor
    let result = target.as_surface().draw(
        (&corners, columns.per_instance(), colors.per_instance()),
        &safegl::index::NoIndices(safegl::index::PrimitiveType::TriangleStrip),
        &program,
        &uniform! {},
        &Default::default(),
    );
    assert!(matches!(result, Err(safegl::DrawError::InstancesCountMismatch)));

    display.assert_no_error(None);
}
