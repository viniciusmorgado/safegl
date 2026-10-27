//! Layered framebuffers, whose geometry shaders send each primitive to a layer with `gl_Layer`.
#![cfg(any(safegl_desktop, safegl_gles32))]

#[macro_use]
extern crate safegl;

use safegl::draw_parameters::DepthTest;
use safegl::framebuffer::{
    LayeredDepthStencil, LayeredFrameBuffer, ToLayeredColorAttachment, ToLayeredDepthAttachment,
    ValidationError,
};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::{
    CubeLayer, Cubemap, DepthTexture2dArray, Texture2dArray, Texture3d, TextureAnyMipmap,
};
use safegl::{Depth, DrawParameters, Program, Rect, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;
type Pixel = (u8, u8, u8, u8);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

const SIZE: u32 = 4;

/// A program whose geometry shader sends the triangle to the layers 0 to `layers - 1`, each with
/// the red `(layer + 1) × 16` and the blue `blue`, at the depth `depth`. `None` without layered
/// framebuffers.
fn program(display: &Display, layers: u32, read_layer: bool) -> Option<Program> {
    if !LayeredFrameBuffer::is_supported(display) || !support::glsl_supported(display, 400, 320) {
        return None;
    }
    // the fragment shader reads `gl_Layer` since GLSL 4.30 and GLSL ES 3.20
    if read_layer && !support::glsl_supported(display, 430, 320) {
        return None;
    }
    let geometry = format!(
        "
        layout(triangles, invocations = {layers}) in;
        layout(triangle_strip, max_vertices = 3) out;
        flat out int layer;
        void main() {{
            for (int i = 0; i < 3; i++) {{
                gl_Position = gl_in[i].gl_Position;
                gl_Layer = gl_InvocationID;
                layer = gl_InvocationID;
                EmitVertex();
            }}
            EndPrimitive();
        }}"
    );
    let layer = if read_layer { "gl_Layer" } else { "layer" };
    let fragment = format!(
        "
        flat in int layer;
        uniform float blue;
        out vec4 color;
        void main() {{ color = vec4(float({layer} + 1) * 16.0 / 255.0, 0.0, blue, 1.0); }}"
    );
    let vertex = "
        in vec2 position;
        uniform float depth;
        void main() { gl_Position = vec4(position, depth, 1.0); }";
    let desktop = if read_layer {
        "#version 430 core\n"
    } else {
        "#version 400 core\n"
    };
    let es = "#version 320 es\nprecision highp float;\n";
    let [dv, dg, df, ev, eg, ef] = [
        (desktop, vertex),
        (desktop, geometry.as_str()),
        (desktop, fragment.as_str()),
        (es, vertex),
        (es, geometry.as_str()),
        (es, fragment.as_str()),
    ]
    .map(|(header, source)| format!("{header}{source}"));
    let program = program!(display,
        400 => {
            vertex: &dv,
            geometry: dg.as_str(),
            fragment: &df,
        },
        320 es => {
            vertex: &ev,
            geometry: eg.as_str(),
            fragment: &ef,
        },
    )
    .unwrap();
    Some(program)
}

/// Draws a fullscreen triangle with `program` into `framebuffer`.
fn draw(
    display: &Display,
    framebuffer: &mut LayeredFrameBuffer<'_>,
    program: &Program,
    blue: f32,
    depth: f32,
    parameters: &DrawParameters<'_>,
) {
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let triangle =
        VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap();
    framebuffer
        .draw(
            &triangle,
            NoIndices(PrimitiveType::TrianglesList),
            program,
            &uniform! { blue: blue, depth: depth },
            parameters,
        )
        .unwrap();
}

/// Reads a layer of a texture level, or a face of a cube map.
fn read(mipmap: &TextureAnyMipmap<'_>, layer: u32, face: Option<CubeLayer>) -> Vec<Vec<Pixel>> {
    let image = mipmap.layer(layer).unwrap().into_image(face).unwrap();
    let rect = Rect {
        left: 0,
        bottom: 0,
        width: image.get_width(),
        height: image.get_height().unwrap_or(1),
    };
    image.raw_read(&rect).unwrap()
}

fn red(layer: u32) -> u8 {
    ((layer + 1) * 16) as u8
}

#[test]
fn array_layers() {
    let display = support::build_display();
    let Some(program) = program(&display, 3, false) else {
        return;
    };
    let texture = Texture2dArray::empty(&display, SIZE, SIZE, 3).unwrap();
    let mut framebuffer = LayeredFrameBuffer::new(&display, &texture).unwrap();
    assert_eq!(framebuffer.layers(), 3);
    assert_eq!(framebuffer.get_dimensions(), (SIZE, SIZE));

    framebuffer.clear_color(0.0, 0.0, 0.0, 1.0);
    draw(
        &display,
        &mut framebuffer,
        &program,
        0.0,
        0.0,
        &Default::default(),
    );
    for layer in 0..3 {
        let pixels = read(&texture.main_level(), layer, None);
        assert!(
            pixels
                .iter()
                .flatten()
                .all(|p| *p == (red(layer), 0, 0, 255)),
            "{layer}"
        );
    }

    // a clear clears every layer
    framebuffer.clear_color(0.0, 0.0, 1.0, 1.0);
    for layer in 0..3 {
        let pixels = read(&texture.main_level(), layer, None);
        assert!(pixels.iter().flatten().all(|p| *p == (0, 0, 255, 255)));
    }

    display.assert_no_error(None);
}

/// The fragment shader reads the layer that the geometry shader chose.
#[test]
fn fragment_shader_reads_the_layer() {
    let display = support::build_display();
    let Some(program) = program(&display, 2, true) else {
        return;
    };
    let texture = Texture2dArray::empty(&display, SIZE, SIZE, 2).unwrap();
    let mut framebuffer = LayeredFrameBuffer::new(&display, &texture).unwrap();
    draw(
        &display,
        &mut framebuffer,
        &program,
        0.0,
        0.0,
        &Default::default(),
    );
    for layer in 0..2 {
        assert_eq!(
            read(&texture.main_level(), layer, None)[0][0],
            (red(layer), 0, 0, 255)
        );
    }

    display.assert_no_error(None);
}

/// The faces of cube maps are layers, in the order of `CubeLayer`.
#[test]
fn cube_map_faces() {
    let display = support::build_display();
    let Some(program) = program(&display, 6, false) else {
        return;
    };
    let texture = Cubemap::empty(&display, SIZE).unwrap();
    let mut framebuffer = LayeredFrameBuffer::new(&display, &texture).unwrap();
    assert_eq!(framebuffer.layers(), 6);
    draw(
        &display,
        &mut framebuffer,
        &program,
        0.0,
        0.0,
        &Default::default(),
    );
    let faces = [
        CubeLayer::PositiveX,
        CubeLayer::NegativeX,
        CubeLayer::PositiveY,
        CubeLayer::NegativeY,
        CubeLayer::PositiveZ,
        CubeLayer::NegativeZ,
    ];
    for (layer, face) in faces.into_iter().enumerate() {
        let pixels = read(&texture.main_level(), 0, Some(face));
        assert_eq!(pixels[0][0], (red(layer as u32), 0, 0, 255), "{face:?}");
    }

    display.assert_no_error(None);
}

/// The slices of 3D textures are layers.
#[test]
fn texture_3d_slices() {
    let display = support::build_display();
    let Some(program) = program(&display, 4, false) else {
        return;
    };
    let texture = Texture3d::empty(&display, SIZE, SIZE, 4).unwrap();
    let mut framebuffer = LayeredFrameBuffer::new(&display, &texture).unwrap();
    assert_eq!(framebuffer.layers(), 4);
    draw(
        &display,
        &mut framebuffer,
        &program,
        0.0,
        0.0,
        &Default::default(),
    );
    for layer in 0..4 {
        assert_eq!(
            read(&texture.main_level(), layer, None)[0][0],
            (red(layer), 0, 0, 255)
        );
    }

    display.assert_no_error(None);
}

/// A layered depth buffer has a layer for each color layer, and the framebuffer has the layers of
/// its attachment that has the fewest.
#[test]
fn layered_depth_buffer() {
    let display = support::build_display();
    let Some(program) = program(&display, 2, false) else {
        return;
    };
    let color = Texture2dArray::empty(&display, SIZE, SIZE, 3).unwrap();
    let depth = DepthTexture2dArray::empty(&display, SIZE, SIZE, 2).unwrap();
    let mut framebuffer = LayeredFrameBuffer::with_depth_buffer(&display, &color, &depth).unwrap();
    assert_eq!(framebuffer.layers(), 2);

    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
    let parameters = DrawParameters {
        depth: Depth {
            test: DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        ..Default::default()
    };
    // a near triangle, then a far blue one that fails the depth test in both layers
    draw(&display, &mut framebuffer, &program, 0.0, -0.5, &parameters);
    draw(&display, &mut framebuffer, &program, 1.0, 0.5, &parameters);
    for layer in 0..2 {
        assert_eq!(
            read(&color.main_level(), layer, None)[0][0],
            (red(layer), 0, 0, 255)
        );
    }

    // a depth-only framebuffer, like a shadow map array
    let mut framebuffer = LayeredFrameBuffer::depth_only(&display, &depth).unwrap();
    assert_eq!(framebuffer.layers(), 2);
    framebuffer.clear_depth(1.0);

    display.assert_no_error(None);
}

/// Several color outputs, each with its layered attachment.
#[test]
fn layered_outputs() {
    let display = support::build_display();
    if !LayeredFrameBuffer::is_supported(&display) || !support::glsl_supported(&display, 400, 320) {
        return;
    }
    let geometry = "
        layout(triangles, invocations = 2) in;
        layout(triangle_strip, max_vertices = 3) out;
        flat out int layer;
        void main() {
            for (int i = 0; i < 3; i++) {
                gl_Position = gl_in[i].gl_Position;
                gl_Layer = gl_InvocationID;
                layer = gl_InvocationID;
                EmitVertex();
            }
            EndPrimitive();
        }";
    let fragment = "
        flat in int layer;
        layout(location = 0) out vec4 first;
        layout(location = 1) out vec4 second;
        void main() {
            first = vec4(float(layer + 1) * 16.0 / 255.0, 0.0, 0.0, 1.0);
            second = vec4(0.0, float(layer + 1) * 16.0 / 255.0, 0.0, 1.0);
        }";
    let vertex = "
        in vec2 position;
        void main() { gl_Position = vec4(position, 0.0, 1.0); }";
    let desktop = "#version 400 core\n#extension GL_ARB_explicit_attrib_location : enable\n";
    let es = "#version 320 es\nprecision highp float;\n";
    let [dv, dg, df, ev, eg, ef] = [
        (desktop, vertex),
        (desktop, geometry),
        (desktop, fragment),
        (es, vertex),
        (es, geometry),
        (es, fragment),
    ]
    .map(|(header, source)| format!("{header}{source}"));
    let program = program!(&display,
        400 => { vertex: &dv, geometry: dg.as_str(), fragment: &df },
        320 es => { vertex: &ev, geometry: eg.as_str(), fragment: &ef },
    )
    .unwrap();

    let first = Texture2dArray::empty(&display, SIZE, SIZE, 2).unwrap();
    let second = Texture2dArray::empty(&display, SIZE, SIZE, 2).unwrap();
    let mut framebuffer = LayeredFrameBuffer::with_outputs(
        &display,
        [("second", &second), ("first", &first)],
        LayeredDepthStencil::None,
    )
    .unwrap();
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let triangle =
        VertexBuffer::new(&display, &positions.map(|position| Vertex { position })).unwrap();
    framebuffer
        .draw(
            &triangle,
            NoIndices(PrimitiveType::TrianglesList),
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();
    for layer in 0..2 {
        assert_eq!(
            read(&first.main_level(), layer, None)[0][0],
            (red(layer), 0, 0, 255)
        );
        assert_eq!(
            read(&second.main_level(), layer, None)[0][0],
            (0, red(layer), 0, 255)
        );
    }

    display.assert_no_error(None);
}

/// The layered color attachments must be textures of the same kind.
#[test]
fn layer_targets_must_match() {
    let display = support::build_display();
    if !LayeredFrameBuffer::is_supported(&display) {
        return;
    }
    let array = Texture2dArray::empty(&display, SIZE, SIZE, 6).unwrap();
    let cube = Cubemap::empty(&display, SIZE).unwrap();
    let result = LayeredFrameBuffer::with_outputs(
        &display,
        [
            ("first", (&array).to_layered_color_attachment()),
            ("second", (&cube).to_layered_color_attachment()),
        ],
        LayeredDepthStencil::None,
    );
    assert!(matches!(result, Err(ValidationError::LayerTargetsMismatch)));

    // the depth attachment can be of another kind
    let depth = DepthTexture2dArray::empty(&display, SIZE, SIZE, 6).unwrap();
    let framebuffer = LayeredFrameBuffer::with_outputs(
        &display,
        [("color", &cube)],
        LayeredDepthStencil::Depth((&depth).to_layered_depth_attachment()),
    );
    // some drivers refuse mixing cube maps and arrays with an incomplete status
    match framebuffer {
        Ok(framebuffer) => assert_eq!(framebuffer.layers(), 6),
        Err(ValidationError::Incomplete { .. }) => (),
        Err(error) => panic!("{error}"),
    }

    display.assert_no_error(None);
}

/// The layers of cube map arrays are the faces of their cube maps, six by six.
#[test]
fn cube_map_array_faces() {
    use safegl::texture::CubemapArray;

    let display = support::build_display();
    let Some(program) = program(&display, 12, false) else {
        return;
    };
    let texture = match CubemapArray::empty(&display, SIZE, 2) {
        Ok(texture) => texture,
        // OpenGL ES 3.2 has cube map arrays, but drivers of OpenGL 4.0 may lack the format
        Err(safegl::texture::TextureCreationError::FormatNotSupported) => return,
        Err(error) => panic!("{error}"),
    };
    let mut framebuffer = LayeredFrameBuffer::new(&display, &texture).unwrap();
    assert_eq!(framebuffer.layers(), 12);
    draw(
        &display,
        &mut framebuffer,
        &program,
        0.0,
        0.0,
        &Default::default(),
    );
    for layer in 0..12 {
        // `layer` of a cube map array mipmap is a cube map, whose faces come in `CubeLayer` order
        let face = [
            CubeLayer::PositiveX,
            CubeLayer::NegativeX,
            CubeLayer::PositiveY,
            CubeLayer::NegativeY,
            CubeLayer::PositiveZ,
            CubeLayer::NegativeZ,
        ][layer as usize % 6];
        let pixels = read(&texture.main_level(), layer / 6, Some(face));
        assert_eq!(pixels[0][0], (red(layer), 0, 0, 255), "{layer}");
    }

    display.assert_no_error(None);
}
