#[macro_use]
extern crate safegl;

use safegl::{Display, Surface};
use glutin::surface::WindowSurface;
use std::io::Cursor;
use support::{ApplicationContext, State};

mod support;

struct Application {
    pub opengl_texture: safegl::Texture2d,
    pub dest_texture: safegl::Texture2d,
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "SafeGL blitting example";

    fn new(display: &Display<WindowSurface>) -> Self {
        // building a texture with "OpenGL" drawn on it
        let image = image::load(
            Cursor::new(&include_bytes!("../../tests/fixture/opengl.png")[..]),
            image::ImageFormat::Png,
        )
        .unwrap()
        .to_rgba8();
        let image_dimensions = image.dimensions();
        let image =
            safegl::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
        let opengl_texture = safegl::Texture2d::new(display, image).unwrap();

        // building a 1024x1024 empty texture
        let dest_texture = safegl::Texture2d::empty_with_format(
            display,
            safegl::texture::UncompressedFloatFormat::U8U8U8U8,
            safegl::texture::MipmapsOption::NoMipmap,
            1024,
            1024,
        )
        .unwrap();
        dest_texture
            .as_surface()
            .clear_color_srgb(0.0, 0.0, 0.0, 1.0);

        Self {
            opengl_texture,
            dest_texture,
        }
    }

    fn draw_frame(&mut self, display: &Display<WindowSurface>) {
        let frame = display.draw();
        if rand::random::<f64>() <= 0.016666 {
            let (left, bottom, dimensions): (f32, f32, f32) = rand::random();
            let dest_rect = safegl::BlitTarget {
                left: (left * self.dest_texture.get_width() as f32) as u32,
                bottom: (bottom * self.dest_texture.get_height().unwrap() as f32) as u32,
                width: (dimensions * self.dest_texture.get_width() as f32) as i32,
                height: (dimensions * self.dest_texture.get_height().unwrap() as f32) as i32,
            };

            self.opengl_texture.as_surface().blit_whole_color_to(
                &self.dest_texture.as_surface(),
                &dest_rect,
                safegl::uniforms::MagnifySamplerFilter::Linear,
            ).unwrap();
        }

        self.dest_texture
            .as_surface()
            .fill(&frame, safegl::uniforms::MagnifySamplerFilter::Linear).unwrap();
        frame.finish().unwrap();
    }
}

fn main() {
    State::<Application>::run_loop();
}
