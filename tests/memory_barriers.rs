//! Shader writes to buffers and images, read afterwards in every other way: SafeGL inserts the
//! memory barrier that each read needs.

#![cfg(any(safegl_gl43, safegl_gles31))]

#[macro_use]
extern crate safegl;

use safegl::buffer::{Buffer, BufferMode, BufferType};
use safegl::program::ComputeShader;
use safegl::texture::{MipmapsOption, UncompressedFloatFormat};
use safegl::uniforms::{ImageUnitAccess, ImageUnitFormat};
use safegl::{Surface, Texture2d, VertexBuffer};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Display = support::Backend;

/// A compute shader with `source` after the `#version` line, or `None` without compute shaders.
fn compute_shader(display: &Display, source: &str) -> Option<ComputeShader> {
    if !ComputeShader::is_supported(display) || !support::glsl_supported(display, 430, 310) {
        return None;
    }
    let header = if support::is_gles(display) {
        "#version 310 es\n"
    } else {
        "#version 430\n"
    };
    Some(ComputeShader::from_source(display, &format!("{header}{source}")).unwrap())
}

/// Writes `index + 1` into each value.
const COUNT: &str = "
    layout(local_size_x = 8) in;
    layout(std430) buffer Data { uint values[]; };
    void main() { values[gl_GlobalInvocationID.x] = gl_GlobalInvocationID.x + 1u; }
";

fn expected_count() -> Vec<u32> {
    (1..=64).collect()
}

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);
implement_uniform_block!(Vertex, position);
implement_any_bit_pattern!(Vertex, position);

#[test]
fn storage_writes_then_vertex_fetches() {
    let display = support::build_display();
    let shader = match compute_shader(
        &display,
        "layout(local_size_x = 4) in;
         struct Vertex { vec2 position; };
         layout(std430) buffer Data { Vertex vertices[]; };
         void main() {
             uint i = gl_GlobalInvocationID.x;
             vertices[i].position = vec2(float(i % 2u) * 2.0 - 1.0, float(i / 2u) * 2.0 - 1.0);
         }",
    ) {
        Some(shader) => shader,
        None => return,
    };

    // the compute shader writes a fullscreen strip, which the draw reads as vertices
    let vertices = VertexBuffer::new(&display, &[Vertex { position: [0.0; 2] }; 4]).unwrap();
    shader
        .execute(uniform! { Data: &*vertices }, 1, 1, 1)
        .unwrap();

    let program = support::program_from_legacy(
        &display,
        "#version 110\nattribute vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 110\nvoid main() { gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0); }",
    )
    .unwrap();
    let output = support::build_renderable_texture(&display);
    output.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    output
        .as_surface()
        .draw(
            &vertices,
            safegl::index::NoIndices(safegl::index::PrimitiveType::TriangleStrip),
            &program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
    assert!(data.iter().flatten().all(|&p| p == (255, 0, 0, 255)));

    display.assert_no_error(None);
}

#[test]
fn storage_writes_then_copies() {
    let display = support::build_display();
    let shader = match compute_shader(&display, COUNT) {
        Some(shader) => shader,
        None => return,
    };
    let source = Buffer::new(
        &display,
        &[0u32; 64][..],
        BufferType::ShaderStorageBuffer,
        BufferMode::Default,
    )
    .unwrap();
    let destination = Buffer::new(
        &display,
        &[0u32; 64][..],
        BufferType::ArrayBuffer,
        BufferMode::Default,
    )
    .unwrap();

    shader.execute(uniform! { Data: &source }, 8, 1, 1).unwrap();
    source.copy_to(&destination).unwrap();

    assert_eq!(destination.read().unwrap(), expected_count());

    display.assert_no_error(None);
}

#[test]
fn storage_writes_then_persistent_mapping_reads() {
    let display = support::build_display();
    let shader = match compute_shader(&display, COUNT) {
        Some(shader) => shader,
        None => return,
    };
    let mut values = Buffer::new(
        &display,
        &[0u32; 64][..],
        BufferType::ShaderStorageBuffer,
        BufferMode::Persistent,
    )
    .unwrap();

    shader.execute(uniform! { Data: &values }, 8, 1, 1).unwrap();

    assert_eq!(&*values.map_read(), &expected_count()[..]);

    display.assert_no_error(None);
}

#[cfg(any(safegl_desktop, safegl_gles32))]
#[test]
fn storage_writes_then_buffer_texture_fetches() {
    use safegl::texture::buffer_texture::{BufferTexture, BufferTextureType};

    let display = support::build_display();
    let shader = match compute_shader(
        &display,
        "layout(local_size_x = 1) in;
         layout(std430) buffer Data { vec4 texels[]; };
         void main() { texels[0] = vec4(0.0, 1.0, 0.0, 1.0); }",
    ) {
        Some(shader) => shader,
        None => return,
    };
    let texture = match BufferTexture::new(
        &display,
        &[(1.0f32, 0.0, 0.0, 1.0)],
        BufferTextureType::Float,
    ) {
        Ok(texture) => texture,
        Err(safegl::texture::buffer_texture::CreationError::TextureCreationError(
            safegl::texture::buffer_texture::TextureCreationError::NotSupported,
        )) => return,
        Err(error) => panic!("{:?}", error),
    };

    shader
        .execute(uniform! { Data: &*texture }, 1, 1, 1)
        .unwrap();

    let (version, precision) = if support::is_gles(&display) {
        (
            "#version 320 es\n",
            "precision highp float;\nprecision highp samplerBuffer;\n",
        )
    } else {
        ("#version 140\n", "")
    };
    let program = safegl::Program::from_source(
        &display,
        &format!("{version}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}"),
        &format!("{version}{precision}uniform samplerBuffer data;\nout vec4 color;\nvoid main() {{ color = texelFetch(data, 0); }}"),
        None,
    )
    .unwrap();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let output = support::build_renderable_texture(&display);
    output
        .as_surface()
        .draw(
            &vb,
            &ib,
            &program,
            &uniform! { data: &texture },
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
    assert_eq!(data[0][0], (0, 255, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn image_stores_then_framebuffer_draws() {
    let display = support::build_display();
    let shader = match compute_shader(
        &display,
        "layout(local_size_x = 1, local_size_y = 1) in;
         layout(rgba8, binding = 0) writeonly uniform highp image2D image;
         void main() {
             imageStore(image, ivec2(gl_GlobalInvocationID.xy), vec4(1.0, 0.0, 0.0, 1.0));
         }",
    ) {
        Some(shader) => shader,
        None => return,
    };
    let texture = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::NoMipmap,
        4,
        4,
    )
    .unwrap();
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    let unit = texture
        .image_unit(ImageUnitFormat::RGBA8)
        .unwrap()
        .set_access(ImageUnitAccess::Write);
    shader.execute(uniform! { image: unit }, 4, 4, 1).unwrap();

    // the draw adds green to the red that the compute shader stored
    let program = support::program_from_legacy(
        &display,
        "#version 110\nattribute vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 110\nvoid main() { gl_FragColor = vec4(0.0, 1.0, 0.0, 1.0); }",
    )
    .unwrap();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let parameters = safegl::DrawParameters {
        blend: safegl::Blend {
            color: safegl::BlendingFunction::Addition {
                source: safegl::LinearBlendingFactor::One,
                destination: safegl::LinearBlendingFactor::One,
            },
            alpha: safegl::BlendingFunction::Addition {
                source: safegl::LinearBlendingFactor::One,
                destination: safegl::LinearBlendingFactor::One,
            },
            constant_value: (0.0, 0.0, 0.0, 0.0),
        },
        ..Default::default()
    };
    texture
        .as_surface()
        .draw(&vb, &ib, &program, &uniform! {}, &parameters)
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert!(data.iter().flatten().all(|&p| p == (255, 255, 0, 255)));

    display.assert_no_error(None);
}

#[test]
fn image_stores_then_uploads_and_mipmaps() {
    let display = support::build_display();
    let shader = match compute_shader(
        &display,
        "layout(local_size_x = 1, local_size_y = 1) in;
         layout(rgba8, binding = 0) writeonly uniform highp image2D image;
         void main() {
             imageStore(image, ivec2(gl_GlobalInvocationID.xy), vec4(0.0, 0.0, 1.0, 1.0));
         }",
    ) {
        Some(shader) => shader,
        None => return,
    };
    let texture = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::EmptyMipmaps,
        4,
        4,
    )
    .unwrap();

    let unit = texture
        .image_unit(ImageUnitFormat::RGBA8)
        .unwrap()
        .set_access(ImageUnitAccess::Write);
    shader.execute(uniform! { image: unit }, 4, 4, 1).unwrap();

    // an upload over a part of the stored image, then levels generated from the whole image
    texture
        .write(
            safegl::Rect {
                left: 0,
                bottom: 0,
                width: 2,
                height: 4,
            },
            vec![vec![(255u8, 0u8, 0u8, 255u8); 2]; 4],
        )
        .unwrap();
    texture.generate_mipmaps().unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    for row in &data {
        assert_eq!(
            &row[..],
            &[
                (255, 0, 0, 255),
                (255, 0, 0, 255),
                (0, 0, 255, 255),
                (0, 0, 255, 255)
            ]
        );
    }
    let level: Vec<Vec<(u8, u8, u8, u8)>> = texture
        .mipmap(2)
        .unwrap()
        .first_layer()
        .into_image(None)
        .unwrap()
        .raw_read(&safegl::Rect {
            left: 0,
            bottom: 0,
            width: 1,
            height: 1,
        })
        .unwrap();
    // the average of red and blue halves
    let (r, g, b, a) = level[0][0];
    assert!((120..=135).contains(&r) && g == 0 && (120..=135).contains(&b) && a == 255);

    display.assert_no_error(None);
}
