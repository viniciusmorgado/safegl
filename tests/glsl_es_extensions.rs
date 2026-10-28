//! The extensions of OpenGL ES and WebGL that only exist in GLSL ES: conservative depth, sample
//! variables and multisample interpolation. Desktop OpenGL has them in GLSL 4.x, which the
//! `glsl_coverage` tests cover.

#[macro_use]
extern crate safegl;

use safegl::framebuffer::{DepthRenderBuffer, SimpleFrameBuffer};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::DepthFormat;
use safegl::{Depth, DepthTest, DrawParameters, Program, Surface, Texture2d, VertexBuffer};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Pixel = (u8, u8, u8, u8);

const GREEN: Pixel = (0, 255, 0, 255);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A program of GLSL ES 3.00 whose vertex shader draws a fullscreen triangle whose `v_one` is 1,
/// and whose fragment shader `fragment` has the `#extension` directive of `extension`; `None` on
/// desktop OpenGL and without the extension.
fn program(
    display: &support::Backend,
    extension: &str,
    has: bool,
    fragment: &str,
) -> Option<Program> {
    if !support::is_gles(display) || !has {
        return None;
    }
    let vertex = "#version 300 es\nin vec2 position;\nout float v_one;\n\
                  void main() { v_one = 1.0; gl_Position = vec4(position, 0.0, 1.0); }";
    let fragment = format!(
        "#version 300 es\n#extension {extension} : require\nprecision highp float;\n\
         in float v_one;\nout vec4 color;\n{fragment}"
    );
    Some(Program::from_source(display, vertex, &fragment, None).unwrap())
}

/// Draws the fullscreen triangle with `program` on `surface`.
fn draw<S: Surface>(
    display: &support::Backend,
    surface: &mut S,
    program: &Program,
    parameters: &DrawParameters<'_>,
) {
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let triangle =
        VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap();
    surface
        .draw(
            &triangle,
            NoIndices(PrimitiveType::TrianglesList),
            program,
            &uniform! {},
            parameters,
        )
        .unwrap();
}

/// Returns true if every pixel of `texture` is green.
fn all_green(texture: &Texture2d) -> bool {
    let pixels: Vec<Vec<Pixel>> = texture.read().unwrap();
    pixels.iter().flatten().all(|&pixel| pixel == GREEN)
}

/// `GL_EXT_conservative_depth` (`EXT_conservative_depth` on WebGL): `gl_FragDepth` declared
/// `depth_greater`, written above the depth of the fragment and below the cleared depth.
#[test]
fn conservative_depth() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_ext_conservative_depth;
    let Some(program) = program(
        &display,
        "GL_EXT_conservative_depth",
        has,
        "layout(depth_greater) out highp float gl_FragDepth;\n\
         void main() {\n\
             gl_FragDepth = gl_FragCoord.z + 0.25;\n\
             color = vec4(0.0, v_one, 0.0, 1.0);\n\
         }",
    ) else {
        return;
    };
    let color = Texture2d::empty(&display, 4, 4).unwrap();
    let depth = DepthRenderBuffer::new(&display, DepthFormat::I24, 4, 4).unwrap();
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
    // drawn at 0.5 + 0.25, in front of the cleared depth
    draw(&display, &mut framebuffer, &program, &parameters);
    assert!(all_green(&color));

    display.assert_no_error(None);
}

/// `GL_OES_sample_variables`: a single-sample target has one sample, of index 0.
#[test]
fn sample_variables() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_oes_sample_variables;
    let Some(program) = program(
        &display,
        "GL_OES_sample_variables",
        has,
        "void main() {\n\
             bool single = gl_SampleID == 0 && gl_NumSamples == 1;\n\
             color = single ? vec4(0.0, v_one, 0.0, 1.0) : vec4(1.0, 0.0, 0.0, 1.0);\n\
         }",
    ) else {
        return;
    };
    let target = Texture2d::empty(&display, 4, 4).unwrap();
    draw(&display, &mut target.as_surface(), &program, &Default::default());
    assert!(all_green(&target));

    display.assert_no_error(None);
}

/// `GL_OES_shader_multisample_interpolation`: an input interpolated at an offset of a constant
/// value keeps it.
#[test]
fn multisample_interpolation() {
    let display = support::build_display();
    let has = support::extensions(&display).gl_oes_shader_multisample_interpolation;
    let Some(program) = program(
        &display,
        "GL_OES_shader_multisample_interpolation",
        has,
        "void main() {\n\
             float one = interpolateAtOffset(v_one, vec2(0.25, -0.25));\n\
             color = vec4(0.0, one, 0.0, 1.0);\n\
         }",
    ) else {
        return;
    };
    let target = Texture2d::empty(&display, 4, 4).unwrap();
    draw(&display, &mut target.as_surface(), &program, &Default::default());
    assert!(all_green(&target));

    display.assert_no_error(None);
}
