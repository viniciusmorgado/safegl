#[macro_use]
extern crate safegl;

use safegl::draw_parameters::{Stencil, StencilTest};
use safegl::framebuffer::{SimpleFrameBuffer, ValidationError};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::{
    MipmapsOption, SrgbTexture2d, UncompressedFloatFormat, UncompressedUintFormat,
    UnsignedTexture2d,
};
use safegl::uniforms::{MagnifySamplerFilter, UniformBuffer};
use safegl::{BlitTarget, DrawError, Surface, Texture2d, VertexBuffer};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Display = support::Backend;

/// The `#version` line for desktop OpenGL, or for OpenGL ES with default precisions.
fn header(display: &Display, desktop: u32, es: u32) -> String {
    if support::is_gles(display) {
        format!("#version {es} es\nprecision highp float;\nprecision highp int;\nprecision highp usampler2D;\n")
    } else {
        format!("#version {desktop}\n")
    }
}

fn build_program(display: &Display, fragment: &str) -> safegl::Program {
    let vertex = header(display, 330, 300)
        + "in vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }";
    let fragment = header(display, 330, 300) + "out vec4 f_color;\n" + fragment;
    safegl::Program::from_source(display, &vertex, &fragment, None).unwrap()
}

fn pixel(texture: &Texture2d) -> (u8, u8, u8, u8) {
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    data[0][0]
}

#[test]
fn incomplete_framebuffer() {
    let display = support::build_display();

    // shared-exponent textures can't be rendered to
    let texture = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::F9F9F9,
        MipmapsOption::NoMipmap,
        2,
        2,
    )
    .unwrap();
    assert!(matches!(
        SimpleFrameBuffer::new(&display, &texture),
        Err(ValidationError::Incomplete { .. })
    ));
    display.assert_no_error(None);
}

#[test]
fn stencil_test_without_stencil_buffer() {
    let display = support::build_display();
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    let texture = Texture2d::empty(&display, 2, 2).unwrap();

    let result = texture.as_surface().draw(
        &vb,
        &ib,
        &program,
        &safegl::uniforms::EmptyUniforms,
        &safegl::DrawParameters {
            stencil: Stencil {
                test_clockwise: StencilTest::AlwaysFail,
                test_counter_clockwise: StencilTest::AlwaysFail,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert!(matches!(result, Err(DrawError::NoStencilBuffer)));
    display.assert_no_error(None);
}

#[test]
fn blit_copies_pixels_as_they_are() {
    let display = support::build_display();
    // OpenGL ES always encodes the values written to sRGB targets
    if support::is_gles(&display) {
        return;
    }

    let source = Texture2d::new(&display, vec![vec![(128u8, 128u8, 128u8, 255u8); 2]; 2]).unwrap();
    let target = SrgbTexture2d::empty(&display, 2, 2).unwrap();

    // a clear of linear values enables `GL_FRAMEBUFFER_SRGB`
    let mut framebuffer = SimpleFrameBuffer::new(&display, &target).unwrap();
    framebuffer.clear_color(0.0, 0.0, 0.0, 0.0);
    source.as_surface().blit_whole_color_to(
        &framebuffer,
        &BlitTarget {
            left: 0,
            bottom: 0,
            width: 2,
            height: 2,
        },
        MagnifySamplerFilter::Nearest,
    ).unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    assert_eq!(data[0][0], (128, 128, 128, 255));
    display.assert_no_error(None);
}

#[test]
fn uniform_block_with_an_instance_name() {
    let display = support::build_display();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let program = build_program(
        &display,
        "layout(std140) uniform Light { vec4 color; } light;
         void main() { f_color = light.color; }",
    );

    #[derive(Copy, Clone)]
    struct Light {
        color: [f32; 4],
    }
    implement_uniform_block!(Light, color);
    let light = UniformBuffer::new(
        &display,
        Light {
            color: [0.0, 1.0, 0.0, 1.0],
        },
    )
    .unwrap();

    let texture = Texture2d::empty(&display, 2, 2).unwrap();
    texture
        .as_surface()
        .draw(
            &vb,
            &ib,
            &program,
            &uniform! { Light: &light },
            &Default::default(),
        )
        .unwrap();
    assert_eq!(pixel(&texture), (0, 255, 0, 255));
    display.assert_no_error(None);
}

#[cfg(any(safegl_desktop, safegl_gles32))]
#[test]
fn geometry_shader_input() {
    let display = support::build_display();
    let header_es = |stage| {
        if support::is_gles(&display) {
            format!("#version 320 es\nprecision highp float;\n{stage}")
        } else {
            format!("#version 330\n{stage}")
        }
    };
    // geometry shaders need OpenGL ES 3.2
    if !display.is_glsl_version_supported(&safegl::Version(safegl::Api::GlEs, 3, 2))
        && support::is_gles(&display)
    {
        return;
    }
    let program = safegl::Program::new(
        &display,
        safegl::program::ProgramCreationInput::SourceCode {
            vertex_shader: &header_es(
                "in vec2 position; void main() { gl_Position = vec4(position, 0.0, 1.0); }",
            ),
            geometry_shader: Some(&header_es(
                "layout(triangles) in; layout(triangle_strip, max_vertices = 3) out;
                 void main() {
                     for (int i = 0; i < 3; i++) { gl_Position = gl_in[i].gl_Position; EmitVertex(); }
                 }",
            )),
            fragment_shader: &header_es("out vec4 f_color; void main() { f_color = vec4(1.0); }"),
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            transform_feedback_varyings: None,
            outputs_srgb: false,
            uses_point_size: false,
            locations: Default::default(),
        },
    );
    let program = program.unwrap();
    let (vb, _) = support::build_rectangle_vb_ib(&display);
    let texture = Texture2d::empty(&display, 2, 2).unwrap();

    let draw = |primitives| {
        texture.as_surface().draw(
            &vb,
            NoIndices(primitives),
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
    };
    assert!(matches!(
        draw(PrimitiveType::Points),
        Err(DrawError::GeometryShaderInputMismatch)
    ));
    draw(PrimitiveType::TriangleStrip).unwrap();
    display.assert_no_error(None);
}

#[test]
fn sample_unsigned_texture_with_the_default_sampler() {
    let display = support::build_display();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let program = build_program(
        &display,
        "uniform usampler2D tex;
         void main() { f_color = vec4(texture(tex, vec2(0.5)).r == 5u ? 1.0 : 0.0, 0.0, 0.0, 1.0); }",
    );
    let unsigned = UnsignedTexture2d::with_format(
        &display,
        vec![vec![(5u8, 0u8, 0u8, 0u8); 2]; 2],
        UncompressedUintFormat::U8U8U8U8,
        MipmapsOption::NoMipmap,
    )
    .unwrap();

    // the default sampler has linear filters, which integer textures can't use
    let texture = Texture2d::empty(&display, 2, 2).unwrap();
    texture
        .as_surface()
        .draw(
            &vb,
            &ib,
            &program,
            &uniform! { tex: unsigned.sampled() },
            &Default::default(),
        )
        .unwrap();
    assert_eq!(pixel(&texture), (255, 0, 0, 255));
    display.assert_no_error(None);
}

#[test]
fn generic_vertex() {
    #[derive(Copy, Clone)]
    struct GenericVertex<T: Copy> {
        position: [T; 2],
    }
    implement_vertex!(GenericVertex<f32>, position);

    let display = support::build_display();
    let program = build_program(
        &display,
        "void main() { f_color = vec4(1.0, 0.0, 0.0, 1.0); }",
    );
    let vertices = VertexBuffer::new(
        &display,
        &[
            GenericVertex {
                position: [-1.0, -1.0],
            },
            GenericVertex {
                position: [3.0, -1.0],
            },
            GenericVertex {
                position: [-1.0, 3.0],
            },
        ],
    )
    .unwrap();

    let texture = Texture2d::empty(&display, 2, 2).unwrap();
    texture
        .as_surface()
        .draw(
            &vertices,
            NoIndices(PrimitiveType::TrianglesList),
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();
    assert_eq!(pixel(&texture), (255, 0, 0, 255));
    display.assert_no_error(None);
}

#[test]
fn uniform_type_mismatch_message() {
    let error = DrawError::UniformTypeMismatch {
        name: "color".to_owned(),
        expected: safegl::uniforms::UniformType::FloatVec4,
    };
    assert_eq!(
        error.to_string(),
        "The type of a uniform doesn't match what the program requires: `color` is a FloatVec4"
    );
}

#[test]
fn depth_buffer_of_16_bits() {
    let display = support::build_display();
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    let color = Texture2d::empty(&display, 2, 2).unwrap();
    let depth = safegl::framebuffer::DepthRenderBuffer::new(
        &display,
        safegl::texture::DepthFormat::I16,
        2,
        2,
    )
    .unwrap();

    // the driver either supports the combination, or SafeGL reports it before drawing
    match SimpleFrameBuffer::with_depth_buffer(&display, &color, &depth) {
        Ok(mut target) => {
            target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
            target
                .draw(
                    &vb,
                    &ib,
                    &program,
                    &safegl::uniforms::EmptyUniforms,
                    &safegl::DrawParameters {
                        depth: safegl::Depth {
                            test: safegl::DepthTest::IfLess,
                            write: true,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )
                .unwrap();
            drop(target);
            assert_eq!(pixel(&color), (255, 0, 0, 255));
        }
        Err(ValidationError::Incomplete { .. }) => (),
        Err(e) => panic!("{e}"),
    }
    display.assert_no_error(None);
}
