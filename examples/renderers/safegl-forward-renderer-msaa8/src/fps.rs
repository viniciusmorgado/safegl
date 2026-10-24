use std::time::Instant;

use anyhow::Result;
use safegl::{backend::Facade, implement_vertex, index::NoIndices, uniform, Surface};

#[derive(Clone, Copy)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}
implement_vertex!(Vertex, position, color);

pub struct FpsCounter {
    program: safegl::Program,
    vertices: safegl::VertexBuffer<Vertex>,
    started: Option<Instant>,
    frames: u32,
    pub value: Option<f64>,
}

impl FpsCounter {
    pub fn new(display: &impl Facade) -> Result<Self> {
        let program = safegl::Program::from_source(
            display,
            r#"#version 330 core
            in vec2 position;
            in vec3 color;
            out vec3 text_color;
            uniform vec2 viewport;
            uniform float scale;
            void main() {
                vec2 p = position * scale / viewport;
                gl_Position = vec4(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
                text_color = color;
            }"#,
            r#"#version 330 core
            in vec3 text_color;
            out vec4 color;
            void main() { color = vec4(text_color, 1.0); }"#,
            None,
        )?;
        Ok(Self {
            program,
            vertices: safegl::VertexBuffer::new(display, &text_vertices("FPS: --"))?,
            started: None,
            frames: 0,
            value: None,
        })
    }

    // Measure completed window frames, excluding asset loading and startup.
    pub fn presented(&mut self, display: &impl Facade) -> Result<()> {
        let now = Instant::now();
        let Some(started) = self.started else {
            self.started = Some(now);
            return Ok(());
        };
        self.frames += 1;
        let seconds = now.duration_since(started).as_secs_f64();
        if seconds >= 0.5 {
            let fps = f64::from(self.frames) / seconds;
            self.vertices =
                safegl::VertexBuffer::new(display, &text_vertices(&format!("FPS: {fps:.0}")))?;
            self.value = Some(fps);
            self.started = Some(now);
            self.frames = 0;
        }
        Ok(())
    }

    pub fn draw(&self, target: &mut impl Surface, scale: f32) -> Result<()> {
        let (width, height) = target.get_dimensions();
        target.draw(
            &self.vertices,
            NoIndices(safegl::index::PrimitiveType::TrianglesList),
            &self.program,
            &uniform! { viewport: [width as f32, height as f32], scale: scale },
            &Default::default(),
        )?;
        Ok(())
    }
}

// A tiny 5x7 bitmap alphabet keeps the overlay independent of font/UI libraries.
fn glyph(c: char) -> [u8; 7] {
    match c {
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        ':' => [0, 4, 4, 0, 4, 4, 0],
        '-' => [0, 0, 0, 14, 0, 0, 0],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        _ => [0; 7],
    }
}

fn text_vertices(text: &str) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    let mut rectangle = |x: f32, y: f32, width: f32, height: f32, color| {
        for position in [
            [x, y],
            [x + width, y],
            [x, y + height],
            [x, y + height],
            [x + width, y],
            [x + width, y + height],
        ] {
            vertices.push(Vertex { position, color });
        }
    };
    rectangle(8.0, 8.0, text.len() as f32 * 12.0 + 14.0, 30.0, [0.008; 3]);
    for (i, c) in text.chars().enumerate() {
        for (y, row) in glyph(c).into_iter().enumerate() {
            for x in 0..5 {
                if row & (1 << (4 - x)) != 0 {
                    rectangle(
                        16.0 + i as f32 * 12.0 + x as f32 * 2.0,
                        16.0 + y as f32 * 2.0,
                        2.0,
                        2.0,
                        [1.0; 3],
                    );
                }
            }
        }
    }
    vertices
}
