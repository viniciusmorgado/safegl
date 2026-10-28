//! Compute shaders (glium#962).

#![cfg(any(safegl_gl43, safegl_gles31))]

#[macro_use]
extern crate safegl;

use safegl::buffer::{Buffer, BufferMode, BufferType};
use safegl::program::{ComputeCommand, ComputeShader};
use safegl::texture::{MipmapsOption, UncompressedFloatFormat};
use safegl::uniforms::{ImageUnitAccess, ImageUnitFormat};
use safegl::{DrawError, Surface, Texture2d};

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

/// Writes twice the index of each invocation into `values`.
const DOUBLE_INDICES: &str = "
    layout(local_size_x = 8) in;
    layout(std430) buffer Data { uint values[]; };
    void main() { values[gl_GlobalInvocationID.x] = gl_GlobalInvocationID.x * 2u; }
";

fn values(display: &Display) -> Buffer<[u32]> {
    Buffer::new(
        display,
        &[0u32; 64][..],
        BufferType::ShaderStorageBuffer,
        BufferMode::Default,
    )
    .unwrap()
}

#[test]
fn dispatch() {
    let display = support::build_display();
    let shader = match compute_shader(&display, DOUBLE_INDICES) {
        Some(shader) => shader,
        None => return,
    };
    let values = values(&display);

    shader.execute(uniform! { Data: &values }, 8, 1, 1).unwrap();

    let expected: Vec<u32> = (0..64).map(|index| index * 2).collect();
    assert_eq!(values.read().unwrap(), expected);

    display.assert_no_error(None);
}

#[test]
fn dispatch_indirect() {
    let display = support::build_display();
    let shader = match compute_shader(&display, DOUBLE_INDICES) {
        Some(shader) => shader,
        None => return,
    };
    let values = values(&display);
    let commands = Buffer::new(
        &display,
        &ComputeCommand {
            num_groups_x: 4,
            num_groups_y: 1,
            num_groups_z: 1,
        },
        BufferType::DispatchIndirectBuffer,
        BufferMode::Default,
    )
    .unwrap();

    shader
        .execute_indirect(uniform! { Data: &values }, commands.as_slice())
        .unwrap();

    // four groups of eight invocations
    let expected: Vec<u32> = (0..64)
        .map(|index| if index < 32 { index * 2 } else { 0 })
        .collect();
    assert_eq!(values.read().unwrap(), expected);

    display.assert_no_error(None);
}

#[test]
fn commands_written_by_a_compute_shader() {
    let display = support::build_display();
    let (writer, shader) = match (
        compute_shader(
            &display,
            "layout(local_size_x = 1) in;
             layout(std430) buffer Commands {
                 uint num_groups_x; uint num_groups_y; uint num_groups_z;
             };
             void main() { num_groups_x = 2u; num_groups_y = 1u; num_groups_z = 1u; }",
        ),
        compute_shader(&display, DOUBLE_INDICES),
    ) {
        (Some(writer), Some(shader)) => (writer, shader),
        _ => return,
    };
    let values = values(&display);
    let commands = Buffer::new(
        &display,
        &ComputeCommand {
            num_groups_x: 0,
            num_groups_y: 0,
            num_groups_z: 0,
        },
        BufferType::ShaderStorageBuffer,
        BufferMode::Default,
    )
    .unwrap();

    // the commands are only read after the shader that writes them is done
    writer
        .execute(uniform! { Commands: &commands }, 1, 1, 1)
        .unwrap();
    shader
        .execute_indirect(uniform! { Data: &values }, commands.as_slice())
        .unwrap();

    let expected: Vec<u32> = (0..64)
        .map(|index| if index < 16 { index * 2 } else { 0 })
        .collect();
    assert_eq!(values.read().unwrap(), expected);

    display.assert_no_error(None);
}

#[test]
fn image_writes() {
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

    let unit = texture
        .image_unit(ImageUnitFormat::RGBA8)
        .unwrap()
        .set_access(ImageUnitAccess::Write);
    shader.execute(uniform! { image: unit }, 4, 4, 1).unwrap();

    // read back
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert!(data.iter().flatten().all(|&p| p == (255, 0, 0, 255)));

    // and sampled by a draw
    let program = support::program_from_legacy(
        &display,
        "#version 110\nattribute vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 110\nuniform sampler2D tex;\nvoid main() { gl_FragColor = texture2D(tex, vec2(0.5)); }",
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
            &uniform! { tex: &texture },
            &Default::default(),
        )
        .unwrap();
    let data: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
    assert_eq!(data[0][0], (255, 0, 0, 255));

    display.assert_no_error(None);
}

#[test]
fn too_many_work_groups() {
    let display = support::build_display();
    let shader = match compute_shader(&display, DOUBLE_INDICES) {
        Some(shader) => shader,
        None => return,
    };
    let values = values(&display);

    assert!(matches!(
        shader.execute(uniform! { Data: &values }, u32::MAX, 1, 1),
        Err(DrawError::WorkGroupCountTooLarge { .. })
    ));

    display.assert_no_error(None);
}

#[test]
fn not_supported() {
    let display = support::build_display();
    if ComputeShader::is_supported(&display) {
        return;
    }

    assert!(matches!(
        ComputeShader::from_source(&display, "#version 430\nvoid main() {}"),
        Err(safegl::ProgramCreationError::ShaderTypeNotSupported)
    ));

    display.assert_no_error(None);
}

#[test]
fn images_with_the_same_binding() {
    let display = support::build_display();
    let shader = match compute_shader(
        &display,
        "layout(local_size_x = 1, local_size_y = 1) in;
         layout(rgba8, binding = 1) writeonly uniform highp image2D first;
         layout(rgba8, binding = 1) writeonly uniform highp image2D second;
         void main() {
             imageStore(first, ivec2(gl_GlobalInvocationID.xy), vec4(1.0, 0.0, 0.0, 1.0));
             imageStore(second, ivec2(gl_GlobalInvocationID.xy), vec4(0.0, 1.0, 0.0, 1.0));
         }",
    ) {
        Some(shader) => shader,
        None => return,
    };
    let texture = || {
        Texture2d::empty_with_format(
            &display,
            UncompressedFloatFormat::U8U8U8U8,
            MipmapsOption::NoMipmap,
            2,
            2,
        )
        .unwrap()
    };
    let (first, second) = (texture(), texture());
    fn unit(texture: &Texture2d) -> safegl::uniforms::ImageUnit<'_, Texture2d> {
        texture
            .image_unit(ImageUnitFormat::RGBA8)
            .unwrap()
            .set_access(ImageUnitAccess::Write)
    }

    let result = shader.execute(
        uniform! { first: unit(&first), second: unit(&second) },
        2,
        2,
        1,
    );
    if support::is_gles(&display) {
        // OpenGL ES can't move the images to other units
        assert!(matches!(result, Err(DrawError::SharedBindingPoint { .. })));
    } else {
        // desktop OpenGL gives each image its own unit
        result.unwrap();
        let first: Vec<Vec<(u8, u8, u8, u8)>> = first.read().unwrap();
        let second: Vec<Vec<(u8, u8, u8, u8)>> = second.read().unwrap();
        assert_eq!(first[0][0], (255, 0, 0, 255));
        assert_eq!(second[0][0], (0, 255, 0, 255));
    }

    display.assert_no_error(None);
}

#[test]
fn work_group_size() {
    let display = support::build_display();
    let Some(shader) = compute_shader(
        &display,
        "layout(local_size_x = 4, local_size_y = 2, local_size_z = 3) in;\nvoid main() {}",
    ) else {
        return;
    };
    assert_eq!(shader.work_group_size(), [4, 2, 3]);
    let Some(shader) = compute_shader(&display, DOUBLE_INDICES) else {
        return;
    };
    assert_eq!(shader.work_group_size(), [8, 1, 1]);

    display.assert_no_error(None);
}
