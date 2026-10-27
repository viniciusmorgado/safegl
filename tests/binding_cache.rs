#[macro_use]
extern crate safegl;

use safegl::buffer::{Buffer, BufferMode, BufferType};
#[cfg(any(safegl_desktop, safegl_gles32))]
use safegl::CapabilitiesSource;
use safegl::Surface;

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

/// The `#version` line for desktop OpenGL, or for OpenGL ES with default precisions.
fn header(display: &Display, desktop: u32, es: u32) -> String {
    if support::is_gles(display) {
        format!("#version {es} es\nprecision highp float;\nprecision highp int;\n")
    } else {
        format!("#version {desktop}\n")
    }
}

/// The vertex shader, whose version must be the one of the fragment shader on OpenGL ES.
fn vertex(display: &Display, desktop: u32, es: u32) -> String {
    header(display, desktop, es)
        + "in vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }"
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
struct Data {
    value: [f32; 4],
}
implement_uniform_block!(Data, value);
implement_any_bit_pattern!(Data, value);

fn indexed_binding_preserves_readback(ty: BufferType, qualifier: &str, version: u32, es: u32) {
    let display = support::build_display();
    if !support::glsl_supported(&display, version, es) {
        return;
    }
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let output = safegl::texture::Texture2d::empty(&display, 1, 1).unwrap();
    let program = safegl::Program::from_source(
        &display,
        &vertex(&display, version, es),
        &(header(&display, version, es) + &format!("layout(std140) {qualifier} Block {{ vec4 value; }};\nout vec4 color;\nvoid main() {{ color = value; }}")),
        None,
    ).unwrap();
    let a_data = Data {
        value: [1.0, 0.0, 0.0, 1.0],
    };
    let b_data = Data {
        value: [0.0, 1.0, 0.0, 1.0],
    };
    let a = Buffer::new(&display, &a_data, ty, BufferMode::Dynamic).unwrap();
    let b = Buffer::new(&display, &b_data, ty, BufferMode::Dynamic).unwrap();

    for iteration in 0..2 {
        // Reading B establishes the generic binding. The first draw binds A to an
        // indexed slot; the second reuses that slot without issuing a GL bind.
        assert_eq!(b.read().unwrap(), b_data);
        output
            .as_surface()
            .draw(
                &vb,
                &ib,
                &program,
                &uniform! { Block: &a },
                &Default::default(),
            )
            .unwrap();
        if iteration == 0 {
            assert_eq!(b.read().unwrap(), b_data);
        }
        assert_eq!(a.read().unwrap(), a_data);
        assert_eq!(b.read().unwrap(), b_data);
    }
    // Writes must also use the correct generic binding after an indexed bind.
    output
        .as_surface()
        .draw(
            &vb,
            &ib,
            &program,
            &uniform! { Block: &b },
            &Default::default(),
        )
        .unwrap();
    a.write(&a_data);
    assert_eq!(b.read().unwrap(), b_data);
    display.assert_no_error(None);
}

#[test]
fn uniform_buffer_binding_cache() {
    indexed_binding_preserves_readback(BufferType::UniformBuffer, "uniform", 140, 300);
}

#[test]
#[cfg(any(safegl_gl43, safegl_gles31))]
fn shader_storage_binding_cache() {
    indexed_binding_preserves_readback(BufferType::ShaderStorageBuffer, "buffer", 430, 310);
}

#[test]
#[cfg(any(safegl_desktop, safegl_gles32))]
fn buffer_texture_binding_cache() {
    use safegl::texture::buffer_texture::{BufferTexture, BufferTextureType};
    use safegl::uniforms::UniformsStorage;

    let display = support::build_display();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let output = safegl::texture::Texture2d::empty(&display, 1, 1).unwrap();
    // buffer textures need OpenGL ES 3.2
    let buffer = match BufferTexture::new(
        &display,
        &[(1.0f32, 0.0, 0.0, 1.0)],
        BufferTextureType::Float,
    ) {
        Ok(buffer) => buffer,
        Err(safegl::texture::buffer_texture::CreationError::TextureCreationError(
            safegl::texture::buffer_texture::TextureCreationError::NotSupported,
        )) => return,
        Err(e) => panic!("{:?}", e),
    };
    let source =
        safegl::texture::Texture2d::new(&display, vec![vec![(0u8, 255u8, 0u8, 255u8)]]).unwrap();
    let fragment = header(&display, 140, 320)
        + if support::is_gles(&display) {
            "precision highp samplerBuffer;\n"
        } else {
            ""
        };
    let buffer_program = safegl::Program::from_source(&display, &vertex(&display, 140, 320),
        &(fragment.clone() + "uniform samplerBuffer data; out vec4 color;\nvoid main() { color = texelFetch(data, 0); }"), None).unwrap();
    let mixed_program = safegl::Program::from_source(&display, &vertex(&display, 140, 320),
        &(fragment + "uniform samplerBuffer data; uniform sampler2D image; out vec4 color;\nvoid main() { color = vec4(texelFetch(data, 0).rgb + texelFetch(image, ivec2(0), 0).rgb, 1.0); }"), None).unwrap();

    // Repeated buffer-texture draws must reuse a cached unit. Without caching,
    // they exhaust the unit list and overwrite a unit still cached as `source`.
    for _ in 0..=display.get_capabilities().max_combined_texture_image_units {
        output
            .as_surface()
            .draw(
                &vb,
                &ib,
                &buffer_program,
                &uniform! { data: &buffer },
                &Default::default(),
            )
            .unwrap();
    }
    // Visit the buffer first, then the 2D texture: stale cache entries would
    // assign two different sampler types to the same texture unit.
    let uniforms = UniformsStorage::new("image", &source).add("data", &buffer);
    output
        .as_surface()
        .draw(&vb, &ib, &mixed_program, &uniforms, &Default::default())
        .unwrap();
    display.assert_no_error(None);
    let pixels: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
    assert_eq!(pixels, vec![vec![(255, 255, 0, 255)]]);
}

#[test]
#[cfg(any(safegl_gl43, safegl_gles31))]
fn atomic_counter_binding_cache() {
    let display = support::build_display();
    if !support::glsl_supported(&display, 430, 310) {
        return;
    }
    let shader = safegl::program::ComputeShader::from_source(&display,
        &(header(&display, 430, 310) + "layout(local_size_x = 1) in;\nlayout(binding = 0) uniform atomic_uint counter;\nvoid main() { atomicCounterIncrement(counter); }")).unwrap();
    let a = Buffer::new(
        &display,
        &10u32,
        BufferType::AtomicCounterBuffer,
        BufferMode::Dynamic,
    )
    .unwrap();
    let b = Buffer::new(
        &display,
        &20u32,
        BufferType::AtomicCounterBuffer,
        BufferMode::Dynamic,
    )
    .unwrap();
    assert_eq!(b.read().unwrap(), 20);
    shader.execute(uniform! { counter: &a }, 1, 1, 1).unwrap();
    assert_eq!(b.read().unwrap(), 20);
    assert_eq!(a.read().unwrap(), 11);
    assert_eq!(b.read().unwrap(), 20);
    shader.execute(uniform! { counter: &a }, 1, 1, 1).unwrap();
    assert_eq!(a.read().unwrap(), 12);
    display.assert_no_error(None);
}

#[test]
#[cfg(any(safegl_gl43, safegl_gles31))]
fn atomic_counter_high_binding_point() {
    // binding the first buffer of a new context at a point above 1 grows the cached bindings by
    // several entries at once
    let display = support::build_display();
    if !support::glsl_supported(&display, 430, 310) {
        return;
    }
    let shader = safegl::program::ComputeShader::from_source(&display,
        &(header(&display, 430, 310) + "layout(local_size_x = 1) in;\nlayout(binding = 3) uniform atomic_uint counter;\nvoid main() { atomicCounterIncrement(counter); }")).unwrap();
    let counter = Buffer::new(
        &display,
        &5u32,
        BufferType::AtomicCounterBuffer,
        BufferMode::Dynamic,
    )
    .unwrap();
    shader.execute(uniform! { counter: &counter }, 1, 1, 1).unwrap();
    assert_eq!(counter.read().unwrap(), 6);
    display.assert_no_error(None);
}

#[test]
fn transform_feedback_generic_binding_does_not_change_indexed_binding() {
    #[derive(Clone, Copy, Debug, PartialEq)]
    #[repr(C)]
    struct Vertex {
        output_val: (f32, f32),
    }
    implement_vertex!(Vertex, output_val);
    implement_any_bit_pattern!(Vertex, output_val);

    let display = support::build_display();
    let (vb, ib) = support::build_rectangle_triangles(&display);
    let output = safegl::texture::Texture2d::empty(&display, 1, 1).unwrap();
    let vertex_shader = header(&display, 140, 300)
        + "in vec2 position; out vec2 output_val; void main() { output_val = position; gl_Position = vec4(position, 0.0, 1.0); }";
    let fragment_shader =
        header(&display, 140, 300) + "out vec4 color; void main() { color = vec4(1.0); }";
    let program = safegl::Program::new(
        &display,
        safegl::program::ProgramCreationInput::SourceCode {
            vertex_shader: &vertex_shader,
            fragment_shader: &fragment_shader,
            geometry_shader: None,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            transform_feedback_varyings: Some((
                vec!["output_val".into()],
                safegl::program::TransformFeedbackMode::Separate,
            )),
            outputs_srgb: false,
            uses_point_size: false,
            locations: Default::default(),
        },
    )
    .unwrap();
    let initial = [Vertex {
        output_val: (0.0, 0.0),
    }; 6];
    let mut a = Buffer::new(
        &display,
        &initial[..],
        BufferType::TransformFeedbackBuffer,
        BufferMode::Dynamic,
    )
    .unwrap();
    let mut b = Buffer::new(
        &display,
        &initial[..],
        BufferType::TransformFeedbackBuffer,
        BufferMode::Dynamic,
    )
    .unwrap();
    for buffer in [&mut a, &mut b] {
        // A generic read must not make SafeGL think this buffer is also bound
        // at indexed slot zero when starting the next capture.
        assert_eq!(buffer.read().unwrap(), initial);
        let session =
            safegl::vertex::TransformFeedback::new(&display, &program)
            .and_then(|transform_feedback| transform_feedback.with_buffer(buffer)).unwrap();
        output
            .as_surface()
            .draw(
                &vb,
                ib,
                &program,
                &uniform! {},
                &safegl::DrawParameters {
                    transform_feedback: Some(&session),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let expected = [
        (-1.0, 1.0),
        (1.0, 1.0),
        (-1.0, -1.0),
        (-1.0, -1.0),
        (1.0, 1.0),
        (1.0, -1.0),
    ];
    for buffer in [&a, &b] {
        let values: Vec<_> = buffer
            .read()
            .unwrap()
            .into_iter()
            .map(|v| v.output_val)
            .collect();
        assert_eq!(values, expected);
    }
    display.assert_no_error(None);
}
