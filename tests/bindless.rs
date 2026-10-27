//! Bindless textures: handles of resident textures given to `sampler*` uniforms.
#![cfg(safegl_desktop)]

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::{TextureHandle, UnsignedTexture2d};
use safegl::uniforms::{MagnifySamplerFilter, SamplerBehavior};
use safegl::{DrawError, Program, Surface, Texture2d, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A program whose fragment shader writes `body` with the bindless sampler `tex` of type `ty`, or
/// `None` without bindless textures.
fn program(display: &Display, ty: &str, body: &str) -> Option<Program> {
    let header = "#version 400 core\n#extension GL_ARB_bindless_texture : require\n";
    Program::from_source(
        display,
        &format!(
            "{header}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}"
        ),
        &format!(
            "{header}layout(bindless_sampler) uniform {ty} tex;\nout vec4 color;\n\
             void main() {{ {body} }}"
        ),
        None,
    )
    .ok()
}

fn draw(
    display: &Display,
    program: &Program,
    handle: TextureHandle<'_>,
) -> Result<(u8, u8, u8, u8), DrawError> {
    let target = Texture2d::empty(display, 2, 2).unwrap();
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let triangle =
        VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap();
    target.as_surface().draw(
        &triangle,
        NoIndices(PrimitiveType::TrianglesList),
        program,
        &uniform! { tex: handle },
        &Default::default(),
    )?;
    let data: Vec<Vec<(u8, u8, u8, u8)>> = target.read().unwrap();
    Ok(data[0][0])
}

#[test]
fn bindless_uniforms() {
    let display = support::build_display();
    if support::is_gles(&display) {
        return;
    }
    let texture = Texture2d::new(&display, vec![vec![(255u8, 0u8, 0u8, 255u8); 2]; 2]).unwrap();
    let Ok(texture) = texture.resident() else {
        // without `GL_ARB_bindless_texture`
        return;
    };
    let Some(sample) = program(&display, "sampler2D", "color = texture(tex, vec2(0.5));") else {
        return;
    };
    let nearest = SamplerBehavior {
        magnify_filter: MagnifySamplerFilter::Nearest,
        ..Default::default()
    };
    let handle = TextureHandle::new(&texture, &nearest);
    assert_eq!(draw(&display, &sample, handle).unwrap(), (255, 0, 0, 255));
    // the same sampler gives the same handle
    let again = TextureHandle::new(&texture, &nearest);
    assert_eq!(draw(&display, &sample, again).unwrap(), (255, 0, 0, 255));

    // the type of the uniform is checked against the texture
    let Some(three_d) = program(&display, "sampler3D", "color = texture(tex, vec3(0.5));") else {
        return;
    };
    assert!(matches!(
        draw(&display, &three_d, handle),
        Err(DrawError::UniformTypeMismatch { .. })
    ));

    // integer textures are sampled without filtering whatever the sampler
    let unsigned = UnsignedTexture2d::new(&display, vec![vec![(7u8, 0u8, 0u8, 0u8); 2]; 2])
        .unwrap()
        .resident()
        .unwrap();
    let integer = program(
        &display,
        "usampler2D",
        "color = vec4(float(texture(tex, vec2(0.5)).r) / 255.0, 0.0, 0.0, 1.0);",
    )
    .unwrap();
    let linear = TextureHandle::new(&unsigned, &SamplerBehavior::default());
    assert_eq!(draw(&display, &integer, linear).unwrap(), (7, 0, 0, 255));

    display.assert_no_error(None);
}
