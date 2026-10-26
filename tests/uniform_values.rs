#[macro_use]
extern crate safegl;

use safegl::uniforms::UniformsStorage;
use safegl::Surface;

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;

/// The `#version` line for desktop OpenGL, or for OpenGL ES with a default precision.
fn header(display: &Display) -> &'static str {
    if support::is_gles(display) {
        "#version 300 es\nprecision highp float;\n"
    } else {
        "#version 330\n"
    }
}

fn build_program(display: &Display, fragment: &str) -> safegl::Program {
    let vertex = format!(
        "{}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}",
        header(display)
    );
    let fragment = format!("{}out vec4 f_color;\n{}", header(display), fragment);
    safegl::Program::from_source(display, &vertex, &fragment, None).unwrap()
}

/// Draws a fullscreen rectangle with `uniforms` and returns the color of a pixel.
fn draw<U: safegl::uniforms::Uniforms>(
    display: &Display,
    program: &safegl::Program,
    uniforms: &U,
) -> (u8, u8, u8, u8) {
    let (vb, ib) = support::build_rectangle_vb_ib(display);
    let texture = support::build_renderable_texture(display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(&vb, &ib, program, uniforms, &Default::default())
        .unwrap();

    display.assert_no_error(None);
    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    data[0][0]
}

#[test]
fn uniform_array_elements() {
    let display = support::build_display();
    let program = build_program(
        &display,
        "uniform vec4 colors[3];\nvoid main() { f_color = colors[0] + colors[1] + colors[2]; }",
    );

    // the name of the array refers to its first element, as in OpenGL
    let first = program.get_uniform("colors").unwrap();
    assert_eq!(
        first.location,
        program.get_uniform("colors[0]").unwrap().location
    );
    assert!(first.size.is_none());
    assert!(program.get_uniform("colors[2]").is_some());

    let uniforms = UniformsStorage::new("colors", [1.0, 0.0, 0.0, 0.0f32])
        .add("colors[1]", [0.0, 1.0, 0.0, 0.0f32])
        .add("colors[2]", [0.0, 0.0, 1.0, 1.0f32]);
    assert_eq!(draw(&display, &program, &uniforms), (255, 255, 255, 255));

    // each element has its own location
    let uniforms = UniformsStorage::new("colors[2]", [0.0, 0.0, 1.0, 1.0f32])
        .add("colors[0]", [0.0f32; 4])
        .add("colors[1]", [0.0f32; 4]);
    assert_eq!(draw(&display, &program, &uniforms), (0, 0, 255, 255));
}

#[test]
fn non_square_matrices() {
    let display = support::build_display();

    // 4 columns of 3 rows: the product with the first axis is the first column
    let program = build_program(
        &display,
        "uniform mat4x3 m;\nvoid main() { f_color = vec4(m * vec4(1.0, 0.0, 0.0, 0.0), 1.0); }",
    );
    let m: [[f32; 3]; 4] = [
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
    ];
    assert_eq!(
        draw(&display, &program, &uniform! { m: m }),
        (0, 255, 0, 255)
    );

    // 3 columns of 2 rows: the product with the last axis is the last column
    let program = build_program(
        &display,
        "uniform mat3x2 m;\nvoid main() { f_color = vec4(m * vec3(0.0, 0.0, 1.0), 1.0, 1.0); }",
    );
    let m: [[f32; 2]; 3] = [[0.0, 0.0], [0.0, 0.0], [1.0, 0.0]];
    assert_eq!(
        draw(&display, &program, &uniform! { m: m }),
        (255, 0, 255, 255)
    );

    // a square matrix doesn't match a non-square uniform
    let square = [[0.0f32; 3]; 3];
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let texture = support::build_renderable_texture(&display);
    let result = texture.as_surface().draw(
        &vb,
        &ib,
        &program,
        &uniform! { m: square },
        &Default::default(),
    );
    assert!(matches!(
        result,
        Err(safegl::DrawError::UniformTypeMismatch { .. })
    ));
}

/// Draws a fullscreen rectangle with `uniforms` and returns the result of the draw command.
fn try_draw<U: safegl::uniforms::Uniforms>(
    display: &Display,
    program: &safegl::Program,
    uniforms: &U,
) -> Result<(), safegl::DrawError> {
    let (vb, ib) = support::build_rectangle_vb_ib(display);
    let texture = support::build_renderable_texture(display);
    texture
        .as_surface()
        .draw(&vb, &ib, program, uniforms, &Default::default())
}

#[test]
fn shadow_samplers_need_a_depth_comparison() {
    use safegl::uniforms::DepthTextureComparison;

    let display = support::build_display();
    let precision = if support::is_gles(&display) {
        "precision highp sampler2DShadow;\n"
    } else {
        ""
    };
    let shadow = build_program(
        &display,
        &format!(
            "{}uniform sampler2DShadow tex;\nvoid main() {{ f_color = vec4(texture(tex, vec3(0.5, 0.5, 0.5))); }}",
            precision
        ),
    );
    let plain = build_program(
        &display,
        "uniform sampler2D tex;\nvoid main() { f_color = texture(tex, vec2(0.5, 0.5)); }",
    );

    let depth = safegl::texture::DepthTexture2d::empty(&display, 4, 4).unwrap();
    let color = support::build_unicolor_texture2d(&display, 1.0, 0.0, 0.0);
    let comparing = depth
        .sampled()
        .depth_texture_comparison(Some(DepthTextureComparison::LessOrEqual));

    // a shadow sampler without a comparison returns undefined values
    assert!(matches!(
        try_draw(&display, &shadow, &uniform! { tex: &depth }),
        Err(safegl::DrawError::DepthComparisonMismatch { .. })
    ));
    try_draw(&display, &shadow, &uniform! { tex: comparing }).unwrap();

    // and so does a plain sampler with one
    assert!(matches!(
        try_draw(&display, &plain, &uniform! { tex: comparing }),
        Err(safegl::DrawError::DepthComparisonMismatch { .. })
    ));
    try_draw(&display, &plain, &uniform! { tex: &depth }).unwrap();
    try_draw(&display, &plain, &uniform! { tex: &color }).unwrap();

    display.assert_no_error(None);
}

#[test]
fn texture_kind_and_dimensions_must_match_the_sampler() {
    let display = support::build_display();
    let program = build_program(
        &display,
        "uniform sampler2D tex;\nvoid main() { f_color = texture(tex, vec2(0.5, 0.5)); }",
    );

    let array = safegl::texture::Texture2dArray::empty(&display, 4, 4, 2).unwrap();
    let integral = safegl::texture::IntegralTexture2d::empty(&display, 4, 4).unwrap();
    for result in [
        try_draw(&display, &program, &uniform! { tex: &array }),
        try_draw(&display, &program, &uniform! { tex: &integral }),
    ] {
        assert!(matches!(
            result,
            Err(safegl::DrawError::UniformTypeMismatch { .. })
        ));
    }

    display.assert_no_error(None);
}

#[test]
fn multisample_array_texture() {
    let display = support::build_display();

    let texture = match safegl::texture::Texture2dMultisampleArray::empty(&display, 4, 4, 2, 4) {
        Ok(texture) => texture,
        // multisample array textures need OpenGL 3.2 or OpenGL ES 3.2
        Err(_) => return,
    };
    let source = if support::is_gles(&display) {
        "#version 320 es\nprecision highp float;\nprecision highp sampler2DMSArray;\n"
    } else {
        "#version 330\n"
    };
    let vertex = format!(
        "{}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}",
        source
    );
    let fragment = format!(
        "{}uniform sampler2DMSArray tex;\nout vec4 f_color;\nvoid main() {{ f_color = texelFetch(tex, ivec3(0, 0, 1), 3); }}",
        source
    );
    let program = safegl::Program::from_source(&display, &vertex, &fragment, None).unwrap();

    try_draw(&display, &program, &uniform! { tex: &texture }).unwrap();

    display.assert_no_error(None);
}

#[test]
fn double_precision_non_square_matrices() {
    let display = support::build_display();
    let version = *display.get_opengl_version();
    if version.0 != safegl::Api::Gl || version < safegl::Version(safegl::Api::Gl, 4, 0) {
        // double precision needs OpenGL 4.0
        return;
    }

    let program = safegl::Program::from_source(
        &display,
        "#version 400\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 400\nuniform dmat3x2 m;\nout vec4 f_color;\nvoid main() { f_color = vec4(vec2(m * dvec3(0.0, 0.0, 1.0)), 1.0, 1.0); }",
        None,
    )
    .unwrap();
    let m: [[f64; 2]; 3] = [[0.0, 0.0], [0.0, 0.0], [1.0, 0.0]];
    assert_eq!(
        draw(&display, &program, &uniform! { m: m }),
        (255, 0, 255, 255)
    );
}

#[test]
fn int64_uniforms() {
    use safegl::backend::Facade;
    use safegl::CapabilitiesSource;

    let display = support::build_display();
    if !display.get_context().get_extensions().gl_arb_gpu_shader_int64 {
        return;
    }

    let program = safegl::Program::from_source(
        &display,
        "#version 400\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 400\n#extension GL_ARB_gpu_shader_int64 : require\nuniform int64_t value;\nuniform u64vec2 pair;\nout vec4 f_color;\nvoid main() { f_color = vec4(value == -5000000000l ? 1.0 : 0.0, pair.y == 6000000000ul ? 1.0 : 0.0, 0.0, 1.0); }",
        None,
    )
    .unwrap();
    assert_eq!(
        draw(
            &display,
            &program,
            &uniform! { value: -5_000_000_000i64, pair: [1u64, 6_000_000_000u64] }
        ),
        (255, 255, 0, 255)
    );
}

#[cfg(feature = "gl46")]
#[test]
fn block_buffer_offsets() {
    use safegl::backend::Facade;
    use safegl::uniforms::UniformValue;
    use safegl::CapabilitiesSource;

    let display = support::build_display();
    let alignment = display
        .get_context()
        .get_capabilities()
        .shader_storage_buffer_offset_alignment;
    if display.get_context().get_capabilities().max_indexed_shader_storage_buffer == 0 {
        return;
    }
    let header = if support::is_gles(&display) {
        "#version 310 es\nprecision highp float;\n"
    } else {
        "#version 430\n"
    };
    let program = safegl::Program::from_source(
        &display,
        &format!("{}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}", header),
        &format!("{}layout(std430) buffer Colors {{ vec4 colors[]; }};\nout vec4 f_color;\nvoid main() {{ f_color = colors[0]; }}", header),
        None,
    )
    .unwrap();

    // one alignment step of colors, then the color to draw
    let step = alignment.max(16) / 16;
    let mut colors = vec![[0.0f32, 0.0, 0.0, 0.0]; step + 1];
    colors[step] = [0.0, 1.0, 0.0, 1.0];
    let colors = safegl::buffer::Buffer::new(
        &display,
        &colors[..],
        safegl::buffer::BufferType::ShaderStorageBuffer,
        safegl::buffer::BufferMode::Default,
    )
    .unwrap();

    // a slice that starts at an aligned offset can be bound
    let aligned = UniformValue::block(colors.slice(step..step + 1).unwrap());
    assert_eq!(
        draw(&display, &program, &UniformsStorage::new("Colors", aligned)),
        (0, 255, 0, 255)
    );

    // one that doesn't, can't
    if alignment > 16 {
        let unaligned = UniformValue::block(colors.slice(1..2).unwrap());
        assert!(matches!(
            try_draw(&display, &program, &UniformsStorage::new("Colors", unaligned)),
            Err(safegl::DrawError::BufferOffsetNotAligned { .. })
        ));
    }

    display.assert_no_error(None);
}

#[test]
fn struct_uniforms() {
    let display = support::build_display();
    let program = build_program(
        &display,
        "struct Light { vec4 color; float intensity; };\nuniform Light light;\nuniform Light lights[2];\nvoid main() { f_color = light.color * light.intensity + lights[1].color * lights[1].intensity; }",
    );

    let uniforms = uniform! {
        "light.color": [1.0f32, 0.0, 0.0, 1.0],
        "light.intensity": 1.0f32,
        "lights[0].color": [0.0f32, 0.0, 0.0, 0.0],
        "lights[0].intensity": 0.0f32,
        "lights[1].color": [0.0f32, 0.0, 1.0, 0.0],
        "lights[1].intensity": 1.0f32,
    };
    assert_eq!(draw(&display, &program, &uniforms), (255, 0, 255, 255));
}
