#[macro_use]
extern crate safegl;

use safegl::backend::Facade;
use safegl::glow::{self, HasContext};
use safegl::index::PrimitiveType;
use safegl::texture::Texture2dMultisample;
use safegl::uniforms::MagnifySamplerFilter;
use safegl::{BlitTarget, DrawError, Surface};

mod support;

const SIZE: u32 = 64;
const SAMPLES: u32 = 4;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

implement_vertex!(Vertex, position);

/// Draws a fullscreen quad whose fragments are red with the alpha given as a uniform.
struct Pipeline {
    vertices: safegl::VertexBuffer<Vertex>,
    indices: safegl::IndexBuffer<u8>,
    program: safegl::Program,
}

impl Pipeline {
    fn new(display: &safegl::Display<safegl::glutin::surface::WindowSurface>) -> Pipeline {
        let vertices = safegl::VertexBuffer::new(
            display,
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
            safegl::IndexBuffer::new(display, PrimitiveType::TriangleStrip, &[0u8, 1, 2, 3])
                .unwrap();
        let program = program!(display,
            330 => {
                vertex: "
                    #version 330 core
                    in vec2 position;
                    void main() { gl_Position = vec4(position, 0.0, 1.0); }
                ",
                fragment: "
                    #version 330 core
                    uniform float alpha;
                    out vec4 color;
                    void main() { color = vec4(1.0, 0.0, 0.0, alpha); }
                ",
            },
            300 es => {
                vertex: "
                    #version 300 es
                    in vec2 position;
                    void main() { gl_Position = vec4(position, 0.0, 1.0); }
                ",
                fragment: "
                    #version 300 es
                    precision mediump float;
                    uniform float alpha;
                    out vec4 color;
                    void main() { color = vec4(1.0, 0.0, 0.0, alpha); }
                ",
            },
        )
        .unwrap();

        Pipeline {
            vertices,
            indices,
            program,
        }
    }

    fn draw<S: Surface>(
        &self,
        target: &mut S,
        alpha: f32,
        params: &safegl::DrawParameters<'_>,
    ) -> Result<(), DrawError> {
        target.draw(
            &self.vertices,
            &self.indices,
            &self.program,
            &uniform! { alpha: alpha },
            params,
        )
    }
}

/// Resolves a multisample texture and returns the average of its red channel, from 0 to 255.
fn resolved_red(
    display: &safegl::Display<safegl::glutin::surface::WindowSurface>,
    texture: &Texture2dMultisample,
) -> f32 {
    let resolved = safegl::Texture2d::empty(display, SIZE, SIZE).unwrap();
    texture.as_surface().blit_whole_color_to(
        &resolved.as_surface(),
        &BlitTarget {
            left: 0,
            bottom: 0,
            width: SIZE as i32,
            height: SIZE as i32,
        },
        MagnifySamplerFilter::Nearest,
    ).unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = resolved.read().unwrap();
    let sum: u32 = data.iter().flatten().map(|pixel| pixel.0 as u32).sum();
    sum as f32 / (SIZE * SIZE) as f32
}

fn multisample_texture(
    display: &safegl::Display<safegl::glutin::surface::WindowSurface>,
) -> Texture2dMultisample {
    let texture = Texture2dMultisample::empty(display, SIZE, SIZE, SAMPLES).unwrap();
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
}

#[test]
fn alpha_to_coverage_covers_part_of_the_samples() {
    let display = support::build_display();
    let pipeline = Pipeline::new(&display);

    let texture = multisample_texture(&display);
    pipeline
        .draw(
            &mut texture.as_surface(),
            0.5,
            &safegl::DrawParameters {
                alpha_to_coverage: true,
                ..Default::default()
            },
        )
        .unwrap();
    let red = resolved_red(&display, &texture);
    assert!(
        (255.0 * 0.3..=255.0 * 0.7).contains(&red),
        "alpha 0.5 covered an average red of {red}"
    );

    // without alpha-to-coverage, every sample is written whatever the alpha
    let texture = multisample_texture(&display);
    pipeline
        .draw(&mut texture.as_surface(), 0.5, &Default::default())
        .unwrap();
    assert_eq!(resolved_red(&display, &texture), 255.0);

    // an alpha of 0 covers no sample at all
    let texture = multisample_texture(&display);
    pipeline
        .draw(
            &mut texture.as_surface(),
            0.0,
            &safegl::DrawParameters {
                alpha_to_coverage: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(resolved_red(&display, &texture), 0.0);

    display.assert_no_error(None);
}

#[test]
fn sample_mask_writes_only_the_selected_samples() {
    let display = support::build_display();
    let pipeline = Pipeline::new(&display);

    let texture = multisample_texture(&display);
    match pipeline.draw(
        &mut texture.as_surface(),
        1.0,
        &safegl::DrawParameters {
            sample_mask: Some(0b0011),
            ..Default::default()
        },
    ) {
        Err(DrawError::SampleMaskNotSupported) => return,
        result => result.unwrap(),
    }
    let red = resolved_red(&display, &texture);
    assert!(
        (red - 127.5).abs() <= 2.0,
        "two of four samples gave an average red of {red}"
    );

    let texture = multisample_texture(&display);
    pipeline
        .draw(
            &mut texture.as_surface(),
            1.0,
            &safegl::DrawParameters {
                sample_mask: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(resolved_red(&display, &texture), 0.0);

    // going back to `None` must disable the mask that the previous draw left enabled
    let texture = multisample_texture(&display);
    pipeline
        .draw(&mut texture.as_surface(), 1.0, &Default::default())
        .unwrap();
    assert_eq!(resolved_red(&display, &texture), 255.0);

    display.assert_no_error(None);
}

#[test]
fn sample_shading_sets_the_minimum_fraction() {
    let display = support::build_display();
    let pipeline = Pipeline::new(&display);
    let texture = multisample_texture(&display);

    let gl_state = |display: &safegl::Display<_>| unsafe {
        let gl = display.get_context().glow_context().clone();
        display.get_context().exec_in_context(move || {
            (
                gl.is_enabled(glow::SAMPLE_SHADING),
                gl.get_parameter_f32(glow::MIN_SAMPLE_SHADING_VALUE),
            )
        })
    };

    match pipeline.draw(
        &mut texture.as_surface(),
        1.0,
        &safegl::DrawParameters {
            sample_shading: Some(0.75),
            ..Default::default()
        },
    ) {
        Err(DrawError::SampleShadingNotSupported) => return,
        result => result.unwrap(),
    }
    assert_eq!(gl_state(&display), (true, 0.75));

    // values are clamped to 0..=1
    pipeline
        .draw(
            &mut texture.as_surface(),
            1.0,
            &safegl::DrawParameters {
                sample_shading: Some(4.0),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(gl_state(&display), (true, 1.0));

    pipeline
        .draw(&mut texture.as_surface(), 1.0, &Default::default())
        .unwrap();
    assert!(!gl_state(&display).0);
    assert_eq!(resolved_red(&display, &texture), 255.0);

    display.assert_no_error(None);
}
