//! The functionality of OpenGL 4.0 to 4.6 that only exists in GLSL: each test compiles shaders
//! that use it, when the context has the version of GLSL, and checks what they compute.
#![cfg(safegl_desktop)]

#[macro_use]
extern crate safegl;

#[cfg(safegl_gl42)]
use safegl::buffer::{Buffer, BufferMode, BufferType};
use safegl::framebuffer::{DepthRenderBuffer, SimpleFrameBuffer};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::{DepthFormat, MipmapsOption, Texture2dMultisample, UncompressedFloatFormat};
#[cfg(safegl_gl42)]
use safegl::uniforms::{ImageUnitAccess, ImageUnitFormat};
use safegl::uniforms::{MagnifySamplerFilter, Uniforms};
use safegl::{
    Blend, BlendingFunction, Depth, DepthTest, DrawParameters, LinearBlendingFactor, Program,
    Surface, Texture2d, VertexBuffer,
};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Display = support::Backend;
type Pixel = (u8, u8, u8, u8);

const GREEN: Pixel = (0, 255, 0, 255);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// The vertex shader of the tests: a fullscreen triangle at the depth `depth`, with `v_uv` from
/// 0 to 1 over the target.
const VERTEX: &str = "
    in vec2 position;
    out vec2 v_uv;
    uniform float depth;
    void main() {
        v_uv = position * 0.5 + 0.5;
        gl_Position = vec4(position, depth, 1.0);
    }
";

/// A program with the vertex shader of the tests and a fragment shader that writes `color`,
/// both GLSL `version`, or a lower GLSL 4.x version with the `extensions` that give the same
/// functionality on drivers below `version`, or `None` if the context has neither.
fn program(
    display: &Display,
    version: u32,
    extensions: &[&str],
    fragment: &str,
) -> Option<Program> {
    let extensions: Vec<(&str, bool)> = extensions
        .iter()
        .map(|&name| (name, support::has_extension(display, name)))
        .collect();
    let header = support::glsl_header(display, version, &extensions)?;
    let fragment = format!("{header}in vec2 v_uv;\nout vec4 color;\n{fragment}");
    Some(Program::from_source(display, &format!("{header}{VERTEX}"), &fragment, None).unwrap())
}

fn triangle(display: &Display) -> VertexBuffer<Vertex> {
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap()
}

/// Draws the fullscreen triangle on `surface` at the depth 0.
fn draw_on<S: Surface>(
    display: &Display,
    surface: &mut S,
    program: &Program,
    uniforms: &impl Uniforms,
    parameters: &DrawParameters<'_>,
) {
    surface
        .draw(
            &triangle(display),
            NoIndices(PrimitiveType::TrianglesList),
            program,
            uniforms,
            parameters,
        )
        .unwrap();
}

/// Draws the fullscreen triangle on a 4x4 target cleared to black, and returns its pixels.
fn draw(display: &Display, program: &Program, uniforms: &impl Uniforms) -> Vec<Pixel> {
    let target = Texture2d::empty(display, 4, 4).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    draw_on(
        display,
        &mut target.as_surface(),
        program,
        uniforms,
        &Default::default(),
    );
    let data: Vec<Vec<Pixel>> = target.read().unwrap();
    data.into_iter().flatten().collect()
}

/// Checks that every pixel is green.
fn assert_green(pixels: &[Pixel]) {
    assert!(pixels.iter().all(|&p| p == GREEN), "{:?}", pixels);
}

#[test]
fn shader_group_vote() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        460,
        &["GL_ARB_shader_group_vote"],
        // the functions of the extension have a suffix
        "#if __VERSION__ < 460
        #define allInvocations allInvocationsARB
        #define anyInvocation anyInvocationARB
        #define allInvocationsEqual allInvocationsEqualARB
        #endif
        void main() {
            bool all_true = allInvocations(true);
            bool any_false = anyInvocation(false);
            bool equal = allInvocationsEqual(true);
            color = all_true && !any_false && equal ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);
        }",
    ) else {
        return;
    };
    assert_green(&draw(&display, &program, &uniform! { depth: 0.0f32 }));
    display.assert_no_error(None);
}

#[cfg(safegl_gl42)]
#[test]
fn atomic_counter_operations() {
    let display = support::build_display();
    let program = |operation: &str| {
        program(
            &display,
            460,
            &["GL_ARB_shader_atomic_counter_ops", "GL_ARB_shader_atomic_counters"],
            &format!(
                // the functions of the extension have a suffix
                "#if __VERSION__ < 460\n\
                 #define atomicCounterAdd atomicCounterAddARB\n\
                 #define atomicCounterSubtract atomicCounterSubtractARB\n\
                 #define atomicCounterMax atomicCounterMaxARB\n\
                 #endif\n\
                 layout(binding = 0, offset = 0) uniform atomic_uint counter;\n\
                 void main() {{ {operation}; color = vec4(0.0, 1.0, 0.0, 1.0); }}"
            ),
        )
    };
    let (Some(add), Some(subtract), Some(maximum)) = (
        program("atomicCounterAdd(counter, 3u)"),
        program("atomicCounterSubtract(counter, 1u)"),
        program("atomicCounterMax(counter, 100u)"),
    ) else {
        return;
    };
    let counter = Buffer::new(
        &display,
        &0u32,
        BufferType::AtomicCounterBuffer,
        BufferMode::Default,
    )
    .unwrap();

    // 16 fragments
    let uniforms = uniform! { depth: 0.0f32, counter: &counter };
    draw(&display, &add, &uniforms);
    assert_eq!(counter.read().unwrap(), 48);
    draw(&display, &subtract, &uniforms);
    assert_eq!(counter.read().unwrap(), 32);
    draw(&display, &maximum, &uniforms);
    assert_eq!(counter.read().unwrap(), 100);

    display.assert_no_error(None);
}

#[test]
fn derivative_control() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        450,
        &["GL_ARB_derivative_control"],
        "void main() {
            bool fine = dFdxFine(gl_FragCoord.x) == 1.0 && dFdyFine(gl_FragCoord.y) == 1.0;
            bool coarse = dFdxCoarse(gl_FragCoord.x) == 1.0 && dFdyCoarse(gl_FragCoord.y) == 1.0;
            bool width = fwidthFine(gl_FragCoord.x) == 1.0 && fwidthCoarse(gl_FragCoord.y) == 1.0;
            color = fine && coarse && width ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);
        }",
    ) else {
        return;
    };
    assert_green(&draw(&display, &program, &uniform! { depth: 0.0f32 }));
    display.assert_no_error(None);
}

#[test]
fn helper_invocations() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        450,
        &["GL_ARB_ES3_1_compatibility"],
        "void main() { color = gl_HelperInvocation ? vec4(1.0) : vec4(0.0, 1.0, 0.0, 1.0); }",
    ) else {
        return;
    };
    // the fragments that are written aren't helper invocations
    assert_green(&draw(&display, &program, &uniform! { depth: 0.0f32 }));
    display.assert_no_error(None);
}

#[test]
fn texture_queries() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        450,
        &["GL_ARB_texture_query_levels", "GL_ARB_shader_texture_image_samples"],
        "uniform sampler2D tex;
        uniform sampler2DMS multisampled;
        void main() {
            bool levels = textureQueryLevels(tex) == 3;
            bool size = textureSize(tex, 1) == ivec2(2, 2);
            bool samples = textureSamples(multisampled) == 4;
            color = levels && size && samples ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);
        }",
    ) else {
        return;
    };
    let texture = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::EmptyMipmaps,
        4,
        4,
    )
    .unwrap();
    let multisampled = Texture2dMultisample::empty(&display, 4, 4, 4).unwrap();
    let uniforms = uniform! { depth: 0.0f32, tex: &texture, multisampled: &multisampled };
    assert_green(&draw(&display, &program, &uniforms));
    display.assert_no_error(None);
}

#[test]
fn gather_with_offsets() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        400,
        &[],
        "uniform sampler2D tex;
        void main() {
            const ivec2 offsets[4] = ivec2[4](ivec2(0), ivec2(1, 0), ivec2(0, 1), ivec2(1));
            vec4 gathered = textureGatherOffsets(tex, vec2(0.25), offsets);
            vec4 expected = vec4(
                textureGatherOffset(tex, vec2(0.25), ivec2(0)).w,
                textureGatherOffset(tex, vec2(0.25), ivec2(1, 0)).w,
                textureGatherOffset(tex, vec2(0.25), ivec2(0, 1)).w,
                textureGatherOffset(tex, vec2(0.25), ivec2(1)).w
            );
            color = gathered == expected ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);
        }",
    ) else {
        return;
    };
    // a different red in each texel
    let data: Vec<Vec<(u8, u8, u8, u8)>> = (0..4u8)
        .map(|y| (0..4u8).map(|x| (x * 16 + y * 64, 0, 0, 255)).collect())
        .collect();
    let texture = Texture2d::new(&display, data).unwrap();
    let uniforms = uniform! { depth: 0.0f32, tex: texture.sampled() };
    assert_green(&draw(&display, &program, &uniforms));
    display.assert_no_error(None);
}

#[test]
fn interpolation_functions() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        400,
        &[],
        "void main() {
            // without multisampling, every sample is at the center of the pixel
            bool center = distance(interpolateAtOffset(v_uv, vec2(0.0)), v_uv) < 1e-5;
            bool at_centroid = distance(interpolateAtCentroid(v_uv), v_uv) < 1e-5;
            bool at_sample = distance(interpolateAtSample(v_uv, 0), v_uv) < 1e-5;
            vec2 moved = interpolateAtOffset(v_uv, vec2(0.25, 0.0)) - v_uv;
            bool offset = abs(moved.x - 0.25 / 4.0 * 2.0 / 2.0) < 1e-4 && abs(moved.y) < 1e-5;
            color = center && at_centroid && at_sample && offset
                ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);
        }",
    ) else {
        return;
    };
    assert_green(&draw(&display, &program, &uniform! { depth: 0.0f32 }));
    display.assert_no_error(None);
}

#[test]
fn sample_built_ins() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        400,
        &[],
        "void main() {
            // using `gl_SampleID` shades each sample, whose bit alone is in the mask
            bool mask = gl_SampleMaskIn[0] == (1 << gl_SampleID);
            bool position = all(greaterThanEqual(gl_SamplePosition, vec2(0.0)))
                && all(lessThan(gl_SamplePosition, vec2(1.0)));
            color = mask && position ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);
        }",
    ) else {
        return;
    };
    let multisampled = Texture2dMultisample::empty(&display, 4, 4, 4).unwrap();
    multisampled.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    draw_on(
        &display,
        &mut multisampled.as_surface(),
        &program,
        &uniform! { depth: 0.0f32 },
        &Default::default(),
    );
    let resolved = Texture2d::empty(&display, 4, 4).unwrap();
    multisampled
        .as_surface()
        .fill(&resolved.as_surface(), MagnifySamplerFilter::Nearest)
        .unwrap();
    let data: Vec<Vec<Pixel>> = resolved.read().unwrap();
    assert_green(&data.into_iter().flatten().collect::<Vec<_>>());
    display.assert_no_error(None);
}

#[test]
fn instanced_geometry_shaders() {
    let display = support::build_display();
    if support::is_gles(&display) || !support::glsl_supported(&display, 400, 0) {
        return;
    }
    let header = "#version 400 core\n";
    let geometry = format!(
        "{header}layout(triangles, invocations = 2) in;
        layout(triangle_strip, max_vertices = 3) out;
        out vec4 g_color;
        void main() {{
            for (int i = 0; i < 3; i++) {{
                // each invocation writes a channel
                g_color = gl_InvocationID == 0
                    ? vec4(1.0, 0.0, 0.0, 1.0) : vec4(0.0, 1.0, 0.0, 1.0);
                gl_Position = gl_in[i].gl_Position;
                EmitVertex();
            }}
            EndPrimitive();
        }}"
    );
    let program = Program::new(
        &display,
        safegl::program::ProgramCreationInput::SourceCode {
            vertex_shader: &format!("{header}{VERTEX}"),
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: Some(geometry.as_str().into()),
            fragment_shader: &format!(
                "{header}in vec4 g_color;\nout vec4 color;\nvoid main() {{ color = g_color; }}"
            ),
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
            locations: Default::default(),
        },
    )
    .unwrap();

    // the two invocations add their channels
    let add = BlendingFunction::Addition {
        source: LinearBlendingFactor::One,
        destination: LinearBlendingFactor::One,
    };
    let parameters = DrawParameters {
        blend: Blend {
            color: add,
            alpha: add,
            ..Default::default()
        },
        ..Default::default()
    };
    let target = Texture2d::empty(&display, 4, 4).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    draw_on(
        &display,
        &mut target.as_surface(),
        &program,
        &uniform! { depth: 0.0f32 },
        &parameters,
    );
    let data: Vec<Vec<Pixel>> = target.read().unwrap();
    assert!(data.iter().flatten().all(|&p| p == (255, 255, 0, 255)));
    display.assert_no_error(None);
}

#[test]
fn double_precision() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        400,
        &[],
        "uniform double value;
        uniform dvec2 pair;
        void main() {
            double sum = value * 2.0lf + pair.x * pair.y;
            color = vec4(float(sum), 0.0, 0.0, 1.0);
        }",
    ) else {
        return;
    };
    // 0.1 × 2 + 0.5 × 0.4
    let uniforms = uniform! { depth: 0.0f32, value: 0.1f64, pair: [0.5f64, 0.4] };
    let pixels = draw(&display, &program, &uniforms);
    assert!(
        pixels.iter().all(|&p| p == (102, 0, 0, 255)),
        "{:?}",
        pixels
    );
    display.assert_no_error(None);
}

#[test]
fn explicit_uniform_locations() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        430,
        &["GL_ARB_explicit_uniform_location"],
        "layout(location = 3) uniform vec4 tint;
        layout(location = 7) uniform float scale;
        void main() { color = tint * scale; }",
    ) else {
        return;
    };
    assert_eq!(program.get_uniform("tint").unwrap().location, 3);
    assert_eq!(program.get_uniform("scale").unwrap().location, 7);
    let uniforms = uniform! { depth: 0.0f32, tint: [0.0f32, 0.5, 0.0, 0.5], scale: 2.0f32 };
    assert_green(&draw(&display, &program, &uniforms));
    display.assert_no_error(None);
}

/// A 4x4 color target with a depth buffer cleared to 1.
fn depth_target(display: &Display) -> (Texture2d, DepthRenderBuffer) {
    let color = Texture2d::empty(display, 4, 4).unwrap();
    let depth = DepthRenderBuffer::new(display, DepthFormat::I24, 4, 4).unwrap();
    (color, depth)
}

#[test]
fn conservative_depth() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        420,
        &["GL_ARB_conservative_depth"],
        "layout(depth_greater) out float gl_FragDepth;
        void main() {
            gl_FragDepth = gl_FragCoord.z + 0.25;
            color = vec4(0.0, 1.0, 0.0, 1.0);
        }",
    ) else {
        return;
    };
    let (color, depth) = depth_target(&display);
    let mut framebuffer = SimpleFrameBuffer::with_depth_buffer(&display, &color, &depth).unwrap();
    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
    let parameters = DrawParameters {
        depth: Depth {
            test: DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        ..Default::default()
    };
    // drawn at 0.5 + 0.25: in front of the clear depth
    draw_on(
        &display,
        &mut framebuffer,
        &program,
        &uniform! { depth: 0.0f32 },
        &parameters,
    );
    let data: Vec<Vec<Pixel>> = color.read().unwrap();
    assert_green(&data.into_iter().flatten().collect::<Vec<_>>());
    display.assert_no_error(None);
}

#[cfg(safegl_gl42)]
#[test]
fn early_fragment_tests() {
    let display = support::build_display();
    let Some(counting) = program(
        &display,
        420,
        &["GL_ARB_shader_image_load_store", "GL_ARB_shader_atomic_counters"],
        "layout(early_fragment_tests) in;
        layout(binding = 0, offset = 0) uniform atomic_uint counter;
        void main() {
            atomicCounterIncrement(counter);
            color = vec4(0.0, 1.0, 0.0, 1.0);
        }",
    ) else {
        return;
    };
    let plain = program(&display, 400, &[], "void main() { color = vec4(1.0); }").unwrap();
    let counter = Buffer::new(
        &display,
        &0u32,
        BufferType::AtomicCounterBuffer,
        BufferMode::Default,
    )
    .unwrap();
    let (color, depth) = depth_target(&display);
    let mut framebuffer = SimpleFrameBuffer::with_depth_buffer(&display, &color, &depth).unwrap();
    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
    let parameters = DrawParameters {
        depth: Depth {
            test: DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        ..Default::default()
    };

    // an occluder in front, then fragments behind it, whose shaders don't run
    draw_on(
        &display,
        &mut framebuffer,
        &plain,
        &uniform! { depth: -0.5f32 },
        &parameters,
    );
    let behind = uniform! { depth: 0.5f32, counter: &counter };
    draw_on(&display, &mut framebuffer, &counting, &behind, &parameters);
    assert_eq!(counter.read().unwrap(), 0);

    // fragments in front of it run
    let in_front = uniform! { depth: -0.9f32, counter: &counter };
    draw_on(
        &display,
        &mut framebuffer,
        &counting,
        &in_front,
        &parameters,
    );
    assert_eq!(counter.read().unwrap(), 16);

    display.assert_no_error(None);
}

#[cfg(safegl_gl42)]
#[test]
fn atomic_exchange_on_float_images() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        450,
        // float exchanges come with GLSL 4.50, and the binding of images with GLSL 4.20
        &[
            "GL_ARB_ES3_1_compatibility",
            "GL_ARB_shader_image_load_store",
            "GL_ARB_shading_language_420pack",
        ],
        "layout(r32f, binding = 0) uniform image2D image;
        void main() {
            if (ivec2(gl_FragCoord.xy) == ivec2(0)) {
                float previous = imageAtomicExchange(image, ivec2(0), 2.5);
                color = previous == 1.0 ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);
            } else {
                color = vec4(0.0, 1.0, 0.0, 1.0);
            }
        }",
    ) else {
        return;
    };
    let image = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::F32,
        MipmapsOption::NoMipmap,
        1,
        1,
    )
    .unwrap();
    image.as_surface().clear_color(1.0, 0.0, 0.0, 0.0);
    let unit = image
        .image_unit(ImageUnitFormat::R32F)
        .unwrap()
        .set_access(ImageUnitAccess::ReadWrite);
    assert_green(&draw(
        &display,
        &program,
        &uniform! { depth: 0.0f32, image: unit },
    ));
    let data: Vec<Vec<(f32, f32, f32, f32)>> = image.read_as().unwrap();
    assert_eq!(data[0][0].0, 2.5);
    display.assert_no_error(None);
}

/// The level of detail that a sampling would use (`textureQueryLod`, GLSL 4.00): a 4x4 texture
/// drawn over 4x4 pixels is sampled at its level 0.
#[test]
fn texture_query_lod() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        400,
        &[],
        "uniform sampler2D tex;
        void main() {
            vec2 lod = textureQueryLod(tex, v_uv);
            color = abs(lod.x) < 0.01 ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);
        }",
    ) else {
        return;
    };
    let texture = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::EmptyMipmaps,
        4,
        4,
    )
    .unwrap();
    let uniforms = uniform! { depth: 0.0f32, tex: &texture };
    assert_green(&draw(&display, &program, &uniforms));
    display.assert_no_error(None);
}

/// The bit functions, `fma` and `precise` of GLSL 4.00.
#[test]
fn bit_functions_and_fused_operations() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        400,
        &[],
        "void main() {
            bool bits = bitfieldExtract(0xf0u, 4, 4) == 0xfu
                && bitfieldInsert(0u, 3u, 2, 2) == 12u
                && bitfieldReverse(1u) == 0x80000000u
                && bitCount(7u) == 3
                && findLSB(8) == 3
                && findMSB(8) == 3;
            uint carry;
            bool extended = uaddCarry(0xffffffffu, 1u, carry) == 0u && carry == 1u;
            precise float fused = fma(2.0, 3.0, 1.0);
            color = bits && extended && fused == 7.0 ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);
        }",
    ) else {
        return;
    };
    assert_green(&draw(&display, &program, &uniform! { depth: 0.0f32 }));
    display.assert_no_error(None);
}

/// The packing functions of GLSL 4.20 (`GL_ARB_shading_language_packing` below).
#[test]
fn packing_functions() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        420,
        &["GL_ARB_shading_language_packing"],
        "void main() {
            bool halves = unpackHalf2x16(packHalf2x16(vec2(1.5, -2.0))) == vec2(1.5, -2.0);
            bool snorm = unpackSnorm2x16(packSnorm2x16(vec2(1.0, -1.0))) == vec2(1.0, -1.0);
            bool unorm = unpackUnorm4x8(packUnorm4x8(vec4(0.0, 1.0, 0.0, 1.0)))
                == vec4(0.0, 1.0, 0.0, 1.0);
            color = halves && snorm && unorm ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);
        }",
    ) else {
        return;
    };
    assert_green(&draw(&display, &program, &uniform! { depth: 0.0f32 }));
    display.assert_no_error(None);
}

/// Initializer lists, `.length()` on arrays and vectors, and swizzles of scalars, of GLSL 4.20
/// (`GL_ARB_shading_language_420pack` below).
#[test]
fn shading_language_420pack() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        420,
        &["GL_ARB_shading_language_420pack"],
        "void main() {
            const float values[3] = { 1.0, 2.0, 3.0 };
            vec3 vector = { 0.0, 1.0, 0.0 };
            float one = 1.0;
            bool lengths = values.length() == 3 && vector.length() == 3;
            bool swizzle = one.xx == vec2(1.0);
            color = lengths && swizzle && values[2] == 3.0
                ? vec4(vector, one)
                : vec4(1.0);
        }",
    ) else {
        return;
    };
    assert_green(&draw(&display, &program, &uniform! { depth: 0.0f32 }));
    display.assert_no_error(None);
}

/// The size of images (`imageSize`, GLSL 4.30 or `GL_ARB_shader_image_size`) and the samples of
/// multisample images (`imageSamples`, GLSL 4.50 or `GL_ARB_shader_texture_image_samples`).
#[cfg(safegl_gl42)]
#[test]
fn image_size_and_samples() {
    let display = support::build_display();
    let Some(size) = program(
        &display,
        430,
        &[
            "GL_ARB_shader_image_size",
            "GL_ARB_shader_image_load_store",
            "GL_ARB_shading_language_420pack",
        ],
        "layout(rgba8, binding = 0) readonly uniform image2D image;
        void main() {
            color = imageSize(image) == ivec2(4, 2) ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);
        }",
    ) else {
        return;
    };
    let image = Texture2d::empty(&display, 4, 2).unwrap();
    let unit = image
        .image_unit(ImageUnitFormat::RGBA8)
        .unwrap()
        .set_access(ImageUnitAccess::Read);
    assert_green(&draw(&display, &size, &uniform! { depth: 0.0f32, image: unit }));

    let Some(samples) = program(
        &display,
        450,
        &[
            "GL_ARB_shader_texture_image_samples",
            "GL_ARB_shader_image_load_store",
            "GL_ARB_shading_language_420pack",
        ],
        "layout(rgba8, binding = 0) readonly uniform image2DMS image;
        void main() {
            color = imageSamples(image) == 4 ? vec4(0.0, 1.0, 0.0, 1.0) : vec4(1.0);
        }",
    ) else {
        return;
    };
    let multisampled = Texture2dMultisample::empty(&display, 4, 4, 4).unwrap();
    let unit = match multisampled.image_unit(ImageUnitFormat::RGBA8) {
        Ok(unit) => unit.set_access(ImageUnitAccess::Read),
        // multisample images need the limit of their samples
        Err(_) => return,
    };
    assert_green(&draw(&display, &samples, &uniform! { depth: 0.0f32, image: unit }));
    display.assert_no_error(None);
}
