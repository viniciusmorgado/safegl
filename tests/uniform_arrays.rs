//! Uniform arrays given from slices, arrays and vectors: data, samplers, blocks and arrays of
//! arrays.

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndices, PrimitiveType};
use safegl::uniforms::{Keep, UniformBuffer, Uniforms};
use safegl::{DrawError, Program, Surface, Texture2d, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;
type Pixel = (u8, u8, u8, u8);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A program whose fragment shader declares `declarations` and runs `body`, which writes
/// `color`, compiled as GLSL `desktop` or GLSL ES `es`, or `None` if the context doesn't have
/// these versions.
fn program(
    display: &Display,
    desktop: u32,
    es: u32,
    declarations: &str,
    body: &str,
) -> Option<Program> {
    if !support::glsl_supported(display, desktop, es) {
        return None;
    }
    let header = if support::is_gles(display) {
        format!("#version {es} es\nprecision highp float;\n")
    } else {
        format!("#version {desktop} core\n")
    };
    let vertex = format!(
        "{header}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}"
    );
    let fragment = format!("{header}{declarations}\nout vec4 color;\nvoid main() {{ {body} }}");
    Some(Program::from_source(display, &vertex, &fragment, None).unwrap())
}

/// Draws a fullscreen triangle with `program` and returns the color of a pixel.
fn draw(
    display: &Display,
    program: &Program,
    uniforms: &impl Uniforms,
) -> Result<Pixel, DrawError> {
    let target = Texture2d::empty(display, 2, 2).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let triangle =
        VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap();
    target.as_surface().draw(
        &triangle,
        NoIndices(PrimitiveType::TrianglesList),
        program,
        uniforms,
        &Default::default(),
    )?;
    let data: Vec<Vec<Pixel>> = target.read().unwrap();
    Ok(data[0][0])
}

#[test]
fn data_arrays() {
    let display = support::build_display();
    let program = program(
        &display,
        330,
        300,
        "uniform float weights[3];\nuniform vec4 colors[2];",
        "color = (colors[0] + colors[1]) * (weights[0] + weights[1] + weights[2]);",
    )
    .unwrap();

    // from a reference to an array, a slice and a vector
    let weights = [0.25f32, 0.25, 0.5];
    let colors = vec![[0.2f32, 0.0, 0.4, 0.0], [0.2, 1.0, 0.0, 1.0]];
    let pixel = draw(
        &display,
        &program,
        &uniform! { weights: &weights, colors: &colors[..] },
    )
    .unwrap();
    assert_eq!(pixel, (102, 255, 102, 255));
    let pixel = draw(
        &display,
        &program,
        &uniform! { weights: &weights, colors: colors.clone() },
    )
    .unwrap();
    assert_eq!(pixel, (102, 255, 102, 255));

    // the elements keep their values
    let partial = [0.0f32, 0.4, 0.0];
    assert_eq!(
        draw(
            &display,
            &program,
            &uniform! { weights: &partial, colors: Keep }
        )
        .unwrap(),
        (41, 102, 41, 102)
    );

    // an element given on its own, with the others kept
    let uniforms = uniform! { weights: Keep, "weights[2]": 0.6f32, colors: Keep };
    assert_eq!(
        draw(&display, &program, &uniforms).unwrap(),
        (102, 255, 102, 255)
    );

    // the elements must have the type of the array
    let integers = [1i32, 2, 3];
    assert!(matches!(
        draw(&display, &program, &uniform! { weights: &integers, colors: Keep }),
        Err(DrawError::UniformTypeMismatch { name, .. }) if name == "weights[0]"
    ));

    display.assert_no_error(None);
}

#[test]
fn missing_elements() {
    let display = support::build_display();
    let program = program(
        &display,
        330,
        300,
        "uniform float weights[3];",
        "color = vec4(weights[0] + weights[1] + weights[2]);",
    )
    .unwrap();

    // a program that was never given the last element
    let short = [0.5f32, 0.5];
    assert!(matches!(
        draw(&display, &program, &uniform! { weights: &short }),
        Err(DrawError::UniformMissing { name }) if name == "weights[2]"
    ));

    // elements past the end of the array are ignored
    let long = [0.25f32, 0.25, 0.5, 1.0, 1.0];
    assert_eq!(
        draw(&display, &program, &uniform! { weights: &long }).unwrap(),
        (255, 255, 255, 255)
    );

    display.assert_no_error(None);
}

#[test]
fn sampler_arrays() {
    let display = support::build_display();
    let program = program(
        &display,
        330,
        300,
        "uniform sampler2D textures[2];",
        "color = texture(textures[0], vec2(0.5)) + texture(textures[1], vec2(0.5));",
    )
    .unwrap();
    let red = Texture2d::new(&display, vec![vec![(255u8, 0u8, 0u8, 0u8); 2]; 2]).unwrap();
    let blue = Texture2d::new(&display, vec![vec![(0u8, 0u8, 255u8, 255u8); 2]; 2]).unwrap();

    // textures and samplers
    assert_eq!(
        draw(&display, &program, &uniform! { textures: &[&red, &blue] }).unwrap(),
        (255, 0, 255, 255)
    );
    let samplers = [red.sampled(), red.sampled()];
    assert_eq!(
        draw(&display, &program, &uniform! { textures: &samplers }).unwrap(),
        (255, 0, 0, 0)
    );

    // every element of a sampler array needs a texture
    assert!(matches!(
        draw(&display, &program, &uniform! { textures: &[&red] }),
        Err(DrawError::UniformBindingMissing { name }) if name == "textures[1]"
    ));

    display.assert_no_error(None);
}

#[test]
fn block_arrays() {
    let display = support::build_display();
    let program = program(
        &display,
        330,
        300,
        "layout(std140) uniform Colors { vec4 value; } colors[2];",
        "color = colors[0].value + colors[1].value;",
    )
    .unwrap();
    let green = UniformBuffer::new(&display, [0.0f32, 1.0, 0.0, 0.0]).unwrap();
    let alpha = UniformBuffer::new(&display, [0.0f32, 0.0, 0.0, 1.0]).unwrap();

    // the blocks of an array are named after the block
    assert_eq!(
        draw(&display, &program, &uniform! { Colors: &[&green, &alpha] }).unwrap(),
        (0, 255, 0, 255)
    );

    display.assert_no_error(None);
}

#[test]
fn arrays_of_arrays() {
    let display = support::build_display();
    let Some(program) = program(
        &display,
        430,
        310,
        "uniform float grid[2][3];",
        "color = vec4(grid[0][0] + grid[0][2], grid[1][1], grid[1][2], 1.0);",
    ) else {
        return;
    };

    // the inner arrays are references, since arrays of floats are vectors
    let grid = [&[0.25f32, 0.0, 0.75], &[0.0, 0.6, 0.0]];
    assert_eq!(
        draw(&display, &program, &uniform! { grid: &grid }).unwrap(),
        (255, 153, 0, 255)
    );

    display.assert_no_error(None);
}
