//! `DrawParameters::base_instance`, added to the start of the per-instance slices.
#![cfg(safegl_gl42)]

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndices, PrimitiveType};
use safegl::{DrawError, DrawParameters, IndexBuffer, Program, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Position {
    position: [f32; 2],
}
implement_vertex!(Position, position);

#[derive(Copy, Clone)]
struct Color {
    color: [f32; 3],
}
implement_vertex!(Color, color);

/// A triangle that covers the whole viewport.
const TRIANGLE: [[f32; 2]; 3] = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];

/// The number of instance colors.
const COUNT: usize = 8;

/// The red of the instance color `i`, which is different for each color once read back.
fn red(i: usize) -> u8 {
    (i * 30 + 10) as u8
}

/// A program that draws the color of the instance, or green without per-instance color.
fn program(display: &Display, per_instance: bool) -> Program {
    let (input, color) = if per_instance {
        ("in vec3 color;", "color")
    } else {
        ("", "vec3(0.0, 1.0, 0.0)")
    };
    let body = format!(
        "in vec2 position;
        {input}
        out vec3 v_color;
        void main() {{
            gl_Position = vec4(position, 0.0, 1.0);
            v_color = {color};
        }}"
    );
    let fragment = "in vec3 v_color;
        out vec4 f_color;
        void main() { f_color = vec4(v_color, 1.0); }";
    program!(display,
        330 => {
            vertex: &format!("#version 330\n{body}"),
            fragment: &format!("#version 330\n{fragment}"),
        },
        300 es => {
            vertex: &format!("#version 300 es\n{body}"),
            fragment: &format!("#version 300 es\nprecision mediump float;\n{fragment}"),
        },
    )
    .unwrap()
}

/// A buffer of the positions of the triangle, after `skipped` other vertices, and the buffer of
/// the instance colors.
fn buffers(display: &Display, skipped: usize) -> (VertexBuffer<Position>, VertexBuffer<Color>) {
    let positions: Vec<Position> = std::iter::repeat_n([0.0, 0.0], skipped)
        .chain(TRIANGLE)
        .map(|position| Position { position })
        .collect();
    let colors: Vec<Color> = (0..COUNT)
        .map(|i| Color {
            color: [red(i) as f32 / 255.0, 0.0, 1.0],
        })
        .collect();
    (
        VertexBuffer::new(display, &positions).unwrap(),
        VertexBuffer::new(display, &colors).unwrap(),
    )
}

fn parameters(base_instance: u32) -> DrawParameters<'static> {
    DrawParameters {
        base_instance,
        ..Default::default()
    }
}

/// Returns `false` if the draw failed because the backend doesn't support base instances.
fn drawn(result: Result<(), DrawError>) -> bool {
    match result {
        Ok(()) => true,
        Err(DrawError::BaseInstanceNotSupported) => false,
        Err(e) => panic!("{e:?}"),
    }
}

/// The color of the center of `texture`.
fn center(texture: &safegl::Texture2d) -> (u8, u8, u8, u8) {
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    data[data.len() / 2][data[0].len() / 2]
}

#[test]
fn base_instance_picks_the_per_instance_element() {
    let display = support::build_display();
    let program = program(&display, true);
    let (positions, colors) = buffers(&display, 0);
    let texture = support::build_renderable_texture(&display);

    for i in 0..COUNT {
        // one instance, at the start of the buffer
        let slice = colors.slice(0..1).unwrap();
        texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
        let result = texture.as_surface().draw(
            (&positions, slice.per_instance()),
            NoIndices(PrimitiveType::TrianglesList),
            &program,
            &uniform! {},
            &parameters(i as u32),
        );
        if !drawn(result) {
            return;
        }
        assert_eq!(center(&texture), (red(i), 0, 255, 255), "base instance {i}");
    }

    display.assert_no_error(None);
}

#[test]
fn base_instance_adds_to_the_slices_of_indexed_draws() {
    let display = support::build_display();
    let program = program(&display, true);
    // the triangle is after 3 other vertices, so that the draw has a base vertex too
    let (positions, colors) = buffers(&display, 3);
    let indices = IndexBuffer::new(&display, PrimitiveType::TrianglesList, &[0u16, 1, 2]).unwrap();
    let texture = support::build_renderable_texture(&display);

    for start in 0..3 {
        for base_instance in 0..3 {
            let vertices = positions.slice(3..6).unwrap();
            let instances = colors.slice(start..start + 1).unwrap();
            texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
            let result = texture.as_surface().draw(
                (vertices, instances.per_instance()),
                &indices,
                &program,
                &uniform! {},
                &parameters(base_instance as u32),
            );
            if !drawn(result) {
                return;
            }
            let element = start + base_instance;
            assert_eq!(
                center(&texture),
                (red(element), 0, 255, 255),
                "slice at {start}, base instance {base_instance}"
            );
        }
    }

    display.assert_no_error(None);
}

#[test]
fn base_instance_without_per_instance_buffers_draws_one_instance() {
    let display = support::build_display();
    let program = program(&display, false);
    let (positions, _) = buffers(&display, 0);
    let texture = support::build_renderable_texture(&display);

    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    let result = texture.as_surface().draw(
        &positions,
        NoIndices(PrimitiveType::TrianglesList),
        &program,
        &uniform! {},
        &parameters(5),
    );
    if !drawn(result) {
        return;
    }
    assert_eq!(center(&texture), (0, 255, 0, 255));

    display.assert_no_error(None);
}

/// Ranges are drawn one by one with the base instance, which `glMultiDrawArrays` and
/// `glMultiDrawElementsBaseVertex` don't have.
#[test]
fn base_instance_applies_to_draw_ranges() {
    use safegl::index::NoIndicesRanges;

    let display = support::build_display();
    // OpenGL ES has no draws of ranges
    if support::is_gles(&display) {
        return;
    }
    let program = program(&display, true);
    let (positions, colors) = buffers(&display, 0);
    let indices = IndexBuffer::new(&display, PrimitiveType::TrianglesList, &[0u16, 1, 2]).unwrap();
    let texture = support::build_renderable_texture(&display);
    let ranges = [0..3];

    for base_instance in [0u32, 4] {
        let instances = colors.slice(0..1).unwrap();
        texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
        let result = texture.as_surface().draw(
            (&positions, instances.per_instance()),
            NoIndicesRanges(PrimitiveType::TrianglesList, &ranges),
            &program,
            &uniform! {},
            &parameters(base_instance),
        );
        if !drawn(result) {
            return;
        }
        let expected = (red(base_instance as usize), 0, 255, 255);
        assert_eq!(center(&texture), expected, "arrays, base instance {base_instance}");

        let instances = colors.slice(0..1).unwrap();
        texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
        let result = texture.as_surface().draw(
            (&positions, instances.per_instance()),
            indices.ranges(&ranges),
            &program,
            &uniform! {},
            &parameters(base_instance),
        );
        drawn(result);
        assert_eq!(center(&texture), expected, "elements, base instance {base_instance}");
    }

    display.assert_no_error(None);
}

/// The commands of multidraw buffers give their own base instance: one in the parameters too
/// is an error instead of being ignored. Multidraw buffers are drawn from the gl43 level.
#[cfg(safegl_gl43)]
#[test]
fn base_instance_of_commands_is_not_overridden() {
    use safegl::index::{DrawCommandsNoIndicesBuffer, IndicesSource};

    let display = support::build_display();
    let program = program(&display, true);
    let (positions, colors) = buffers(&display, 0);
    let commands = match DrawCommandsNoIndicesBuffer::empty(&display, 1) {
        Ok(commands) => commands,
        Err(_) => return,
    };
    let texture = support::build_renderable_texture(&display);
    let source: IndicesSource<'_> = commands.with_primitive_type(PrimitiveType::TrianglesList);
    let result = texture.as_surface().draw(
        (&positions, colors.per_instance()),
        source,
        &program,
        &uniform! {},
        &parameters(1),
    );
    match result {
        Err(DrawError::BaseInstanceNotApplicable) | Err(DrawError::BaseInstanceNotSupported) => (),
        other => panic!("{other:?}"),
    }

    display.assert_no_error(None);
}
