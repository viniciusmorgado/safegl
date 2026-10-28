//! Multiview framebuffers (`GL_OVR_multiview2`), whose draws render to several views at once.

#[macro_use]
extern crate safegl;

use safegl::framebuffer::{
    LayeredDepthStencil, LayeredFrameBuffer, SimpleFrameBuffer, ToLayeredDepthAttachment,
    ValidationError,
};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::{DepthTexture2dArray, Texture2d, Texture2dArray, Texture3d, TextureAnyMipmap};
use safegl::{BlitError, DrawError, Program, Rect, Surface, VertexBuffer};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Pixel = (u8, u8, u8, u8);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

const SIZE: u32 = 4;

/// The header of the shaders of the context.
fn header(display: &support::Backend) -> &'static str {
    if support::is_gles(display) {
        "#version 300 es\n"
    } else {
        "#version 330\n"
    }
}

/// Returns true if the shaders of multiview programs compile. Firefox on NVIDIA drivers has
/// `OVR_multiview2`, but translates the shaders to GLSL 1.50, for which the driver refuses the
/// extension: there, only the errors that SafeGL returns without compiling are checked.
fn multiview_compiles(display: &support::Backend) -> bool {
    let header = header(display);
    let vertex = format!(
        "{header}#extension GL_OVR_multiview2 : require\nlayout(num_views = 2) in;\n\
         void main() {{ gl_Position = vec4(0.0); }}"
    );
    let fragment = format!("{header}precision mediump float;\nout vec4 color;\n\
                            void main() {{ color = vec4(1.0); }}");
    match Program::from_source(display, &vertex, &fragment, None) {
        Ok(_) => true,
        Err(error) if cfg!(target_arch = "wasm32") => {
            println!("multiview shaders don't compile: {error}");
            false
        }
        Err(error) => panic!("{error}"),
    }
}

/// A program whose vertex shader declares `views` views, if there are some, and whose draws are
/// red in the view 0 and green in the others.
fn program(display: &support::Backend, views: Option<u32>) -> Program {
    let header = header(display);
    let (extension, layout, view) = match views {
        Some(views) => (
            "#extension GL_OVR_multiview2 : require\n",
            format!("layout(num_views = {views}) in;\n"),
            "int(gl_ViewID_OVR)",
        ),
        None => ("", String::new(), "0"),
    };
    Program::from_source(
        display,
        &format!(
            "{header}{extension}{layout}in vec2 position;\nflat out int view;\n\
             void main() {{ view = {view}; gl_Position = vec4(position, 0.0, 1.0); }}"
        ),
        &format!(
            "{header}precision mediump float;\nflat in int view;\nout vec4 color;\n\
             void main() {{ color = view == 0 ? vec4(1.0, 0.0, 0.0, 1.0) \
             : vec4(0.0, 1.0, 0.0, 1.0); }}"
        ),
        None,
    )
    .unwrap()
}

/// Draws a triangle over the whole target with `program`.
fn draw<S: Surface>(
    display: &support::Backend,
    target: &mut S,
    program: &Program,
) -> Result<(), DrawError> {
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let triangle =
        VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap();
    target.draw(
        &triangle,
        NoIndices(PrimitiveType::TrianglesList),
        program,
        &uniform! {},
        &Default::default(),
    )
}

/// Reads a layer of the main level of a 2D array texture.
fn read(texture: &Texture2dArray, layer: u32) -> Vec<Vec<Pixel>> {
    let mipmap: &TextureAnyMipmap<'_> = &texture.main_level();
    let image = mipmap.layer(layer).unwrap().into_image(None).unwrap();
    let rect = Rect {
        left: 0,
        bottom: 0,
        width: SIZE,
        height: SIZE,
    };
    image.raw_read(&rect).unwrap()
}

/// Returns true if every pixel of `pixels` is `pixel`.
fn all(pixels: &[Vec<Pixel>], pixel: Pixel) -> bool {
    pixels.iter().flatten().all(|&p| p == pixel)
}

/// A draw renders each view with its `gl_ViewID_OVR`, and a clear clears every view.
#[test]
fn views_are_drawn() {
    let display = support::build_display();
    let texture = Texture2dArray::empty(&display, SIZE, SIZE, 3).unwrap();
    let framebuffer = LayeredFrameBuffer::multiview(&display, &texture, Default::default(), 1..3);
    if !LayeredFrameBuffer::is_multiview_supported(&display) {
        assert!(matches!(framebuffer, Err(ValidationError::MultiviewNotSupported)));
        return;
    }
    let mut framebuffer = framebuffer.unwrap();
    if !multiview_compiles(&display) {
        return;
    }
    assert_eq!(framebuffer.views(), Some(2));
    assert_eq!(framebuffer.layers(), 2);
    assert_eq!(framebuffer.get_dimensions(), (SIZE, SIZE));

    // the layer 0 isn't a view
    let mut layer0 = SimpleFrameBuffer::new(&display, texture.main_level().layer(0).unwrap())
        .unwrap();
    layer0.clear_color(0.0, 0.0, 1.0, 1.0);
    framebuffer.clear_color(0.0, 0.0, 0.0, 1.0);
    assert!(all(&read(&texture, 1), (0, 0, 0, 255)));
    assert!(all(&read(&texture, 2), (0, 0, 0, 255)));

    draw(&display, &mut framebuffer, &program(&display, Some(2))).unwrap();
    assert!(all(&read(&texture, 0), (0, 0, 255, 255)));
    assert!(all(&read(&texture, 1), (255, 0, 0, 255)));
    assert!(all(&read(&texture, 2), (0, 255, 0, 255)));

    display.assert_no_error(None);
}

/// The views of a multiview framebuffer with a depth buffer of the same views.
#[test]
fn views_with_depth() {
    let display = support::build_display();
    if !LayeredFrameBuffer::is_multiview_supported(&display) {
        return;
    }
    if !multiview_compiles(&display) {
        return;
    }
    let color = Texture2dArray::empty(&display, SIZE, SIZE, 2).unwrap();
    let depth = DepthTexture2dArray::empty(&display, SIZE, SIZE, 2).unwrap();
    let depth_stencil = LayeredDepthStencil::Depth((&depth).to_layered_depth_attachment());
    let mut framebuffer =
        LayeredFrameBuffer::multiview(&display, &color, depth_stencil, 0..2).unwrap();
    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
    draw(&display, &mut framebuffer, &program(&display, Some(2))).unwrap();
    assert!(all(&read(&color, 0), (255, 0, 0, 255)));
    assert!(all(&read(&color, 1), (0, 255, 0, 255)));

    display.assert_no_error(None);
}

/// The program declares the views of the framebuffer, which can't capture nor be read.
#[test]
fn views_mismatch() {
    let display = support::build_display();
    if !LayeredFrameBuffer::is_multiview_supported(&display) {
        return;
    }
    if !multiview_compiles(&display) {
        return;
    }
    let texture = Texture2dArray::empty(&display, SIZE, SIZE, 2).unwrap();
    let mut framebuffer =
        LayeredFrameBuffer::multiview(&display, &texture, Default::default(), 0..2).unwrap();

    // a program without views draws to the framebuffers that aren't multiview
    assert!(matches!(
        draw(&display, &mut framebuffer, &program(&display, None)),
        Err(DrawError::ViewsMismatch {
            framebuffer: Some(2),
            program: None
        })
    ));
    display.assert_no_error(Some("a program without views"));
    // a program of views only draws to a multiview framebuffer of as many, even of one view:
    // WebGL refuses the others
    let target = Texture2d::empty(&display, SIZE, SIZE).unwrap();
    for views in [2, 1] {
        assert!(matches!(
            draw(&display, &mut target.as_surface(), &program(&display, Some(views))),
            Err(DrawError::ViewsMismatch {
                framebuffer: None,
                program: Some(v)
            }) if v == views
        ));
    }
    display.assert_no_error(Some("programs of views"));

    // blits can't read several views
    let rect = Rect {
        left: 0,
        bottom: 0,
        width: SIZE,
        height: SIZE,
    };
    let blit_target = safegl::BlitTarget {
        left: 0,
        bottom: 0,
        width: SIZE as i32,
        height: SIZE as i32,
    };
    let blit = framebuffer.blit_color(
        &rect,
        &target.as_surface(),
        &blit_target,
        safegl::uniforms::MagnifySamplerFilter::Nearest,
    );
    assert_eq!(blit, Err(BlitError::Multiview));

    display.assert_no_error(None);
}

/// The views are layers of 2D array textures, within their layers and the limit of the backend.
#[test]
fn invalid_views() {
    let display = support::build_display();
    if !LayeredFrameBuffer::is_multiview_supported(&display) {
        return;
    }
    let texture = Texture2dArray::empty(&display, SIZE, SIZE, 2).unwrap();
    for views in [1..3, 0..0, 0..1, 2..2, 0..64] {
        assert!(
            matches!(
                LayeredFrameBuffer::multiview(&display, &texture, Default::default(), views.clone()),
                Err(ValidationError::ViewsOutOfRange)
            ),
            "{views:?}"
        );
    }
    let volume = Texture3d::empty(&display, SIZE, SIZE, 2).unwrap();
    assert!(matches!(
        LayeredFrameBuffer::multiview(&display, &volume, Default::default(), 0..2),
        Err(ValidationError::MultiviewTextureNotSupported)
    ));

    display.assert_no_error(None);
}
