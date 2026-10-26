/*!

Renders without a window: draws a triangle into a texture and saves it to a PNG file.

`Display` needs a window surface, so a headless program implements the `Backend` trait itself, like
the `manual-creation` example, with a context made current on a pbuffer surface that is never
shown. The pbuffer only gives the context something to be current on: the drawing goes into a
texture, whose size has nothing to do with it.

*/

#[macro_use]
extern crate safegl;

use glutin::config::{ConfigSurfaceTypes, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentContext, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{PbufferSurface, Surface as GlutinSurface, SurfaceAttributesBuilder};
use glutin_winit::DisplayBuilder;
use safegl::index::{NoIndices, PrimitiveType};
use safegl::Surface;
use winit::event_loop::EventLoop;

use std::ffi::{c_void, CString};
use std::num::NonZeroU32;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;

/// A context that is current on an invisible pbuffer surface.
struct Backend {
    context: PossiblyCurrentContext,
    surface: GlutinSurface<PbufferSurface>,
}

unsafe impl safegl::backend::Backend for Backend {
    // there is nothing to show
    fn swap_buffers(&self) -> Result<(), safegl::SwapBuffersError> {
        Ok(())
    }

    unsafe fn get_proc_address(&self, symbol: &str) -> *const c_void {
        let symbol = CString::new(symbol).unwrap();
        self.context.display().get_proc_address(&symbol) as *const _
    }

    fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        (1, 1)
    }

    fn resize(&self, _: (u32, u32)) {}

    fn is_current(&self) -> bool {
        self.context.is_current()
    }

    unsafe fn make_current(&self) {
        self.context.make_current(&self.surface).unwrap();
    }

    fn set_swap_interval(&self, _: safegl::backend::SwapInterval) {}
}

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}
implement_vertex!(Vertex, position, color);

fn main() {
    // the event loop only gives access to the display connection: no window is created
    let event_loop = EventLoop::builder().build().expect("event loop building");
    let template = ConfigTemplateBuilder::new().with_surface_type(ConfigSurfaceTypes::PBUFFER);
    let (_, gl_config) = DisplayBuilder::new()
        .build(&event_loop, template, |mut configs| configs.next().unwrap())
        .unwrap();
    let display = gl_config.display();

    // SafeGL needs at least OpenGL 4.1 or OpenGL ES 3.0
    let context = unsafe {
        display
            .create_context(
                &gl_config,
                &ContextAttributesBuilder::new()
                    .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 1))))
                    .build(None),
            )
            .or_else(|_| {
                display.create_context(
                    &gl_config,
                    &ContextAttributesBuilder::new()
                        .with_context_api(ContextApi::Gles(Some(Version::new(3, 0))))
                        .build(None),
                )
            })
            .expect("failed to create a context")
    };
    let surface = unsafe {
        display
            .create_pbuffer_surface(
                &gl_config,
                &SurfaceAttributesBuilder::<PbufferSurface>::new()
                    .build(NonZeroU32::new(1).unwrap(), NonZeroU32::new(1).unwrap()),
            )
            .unwrap()
    };
    let context = context.make_current(&surface).unwrap();

    let context = unsafe {
        safegl::backend::Context::new(Backend { context, surface }, true, Default::default())
    }
    .unwrap();

    // drawing into a texture
    let vertices = safegl::VertexBuffer::new(
        &context,
        &[
            Vertex {
                position: [-0.5, -0.5],
                color: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [0.0, 0.5],
                color: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.5, -0.5],
                color: [1.0, 0.0, 0.0],
            },
        ],
    )
    .unwrap();
    let program = program!(&context,
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
                in vec3 v_color;
                out vec4 f_color;
                void main() {
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
                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    v_color = color;
                }
            ",
            fragment: "
                #version 300 es
                precision mediump float;
                in vec3 v_color;
                out vec4 f_color;
                void main() {
                    f_color = vec4(v_color, 1.0);
                }
            ",
        },
    )
    .unwrap();

    let texture = safegl::Texture2d::empty(&context, WIDTH, HEIGHT).unwrap();
    let mut target = texture.as_surface();
    target.clear_color(0.0, 0.0, 0.0, 1.0);
    target
        .draw(
            &vertices,
            NoIndices(PrimitiveType::TrianglesList),
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();

    // reading the texture back and saving it, flipped since OpenGL's rows go from the bottom
    let image: safegl::texture::RawImage2d<'_, u8> = texture.read().unwrap();
    let image =
        image::ImageBuffer::from_raw(image.width, image.height, image.data.into_owned()).unwrap();
    let image = image::DynamicImage::ImageRgba8(image).flipv();
    let path = std::env::temp_dir().join("safegl-headless.png");
    image.save(&path).unwrap();
    println!("Saved {}", path.display());
}
