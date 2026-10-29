/*!

Renders without a window, and without a window system: draws a triangle on a display without
window and saves its frame to a PNG file.

The display (`backend::glutin::headless`) is the EGL display of a device, a GPU or the software
renderer of Mesa (`EGL_EXT_platform_device`), whose frames go to a pbuffer of a fixed size. It
works on servers, in CI and on embedded boards without X11 or Wayland.

*/

#[cfg(safegl_egl)]
#[macro_use]
extern crate safegl;

#[cfg(safegl_egl)]
#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}
#[cfg(safegl_egl)]
implement_vertex!(Vertex, position, color);

#[cfg(safegl_egl)]
fn main() {
    use safegl::backend::glutin::headless::{devices, Headless};
    use safegl::index::{NoIndices, PrimitiveType};
    use safegl::Surface;

    const WIDTH: u32 = 256;
    const HEIGHT: u32 = 256;

    for device in devices().expect("the EGL devices can't be listed") {
        println!("EGL device: {:?} {:?}", device.vendor(), device.name());
    }
    let headless = Headless::first().expect("no EGL device creates a context of the levels");
    let display = headless.display(WIDTH, HEIGHT, Default::default()).unwrap();
    println!("Drawing with {}", display.get_opengl_renderer_string());

    let vertices = safegl::VertexBuffer::new(
        &display,
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
    let program = program!(&display,
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

    let mut frame = display.draw();
    frame.clear_color(0.0, 0.0, 0.0, 1.0);
    frame
        .draw(
            &vertices,
            NoIndices(PrimitiveType::TrianglesList),
            &program,
            &safegl::uniforms::EmptyUniforms,
            &Default::default(),
        )
        .unwrap();
    frame.finish().unwrap();

    // reading the frame back and saving it, flipped since OpenGL's rows go from the bottom
    let image: safegl::texture::RawImage2d<'_, u8> = display.read_front_buffer().unwrap();
    let image =
        image::ImageBuffer::from_raw(image.width, image.height, image.data.into_owned()).unwrap();
    let image = image::DynamicImage::ImageRgba8(image).flipv();
    let path = std::env::temp_dir().join("safegl-headless.png");
    image.save(&path).unwrap();
    println!("Saved {}", path.display());
}

#[cfg(not(safegl_egl))]
fn main() {
    eprintln!("this example needs EGL, with the `egl` feature, on Linux, Android or Windows");
}
