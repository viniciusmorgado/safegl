#[macro_use]
extern crate safegl;

use safegl::{Api, CapabilitiesSource, Surface, Version};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn magnify_nearest_filtering() {
    // ignoring test on travis
    // TODO: find out why they are failing
    if ::std::env::var("TRAVIS").is_ok() {
        return;
    }

    let display = support::build_display();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);

    let program = program!(&display,
    330 => {
        vertex: "
                #version 330

                in vec2 position;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                }
            ",
        fragment: "
                #version 330

                out vec4 f_color;

                uniform sampler2D tex;

                void main() {
                    f_color = texture(tex, vec2(0.51, 0.0));
                }
            ",
    },
    100 => {
        vertex: "
                #version 100

                attribute lowp vec2 position;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                }
            ",
        fragment: "
                #version 100

                uniform lowp sampler2D tex;

                void main() {
                    gl_FragColor = texture2D(tex, vec2(0.51, 0.0));
                }
            ",
    })
    .unwrap();

    let texture_data = vec![vec![(0u8, 0, 0), (255, 255, 255)]];
    let texture = safegl::texture::Texture2d::new(&display, texture_data).unwrap();

    let uniforms = uniform! {
        tex: texture.sampled().magnify_filter(safegl::uniforms::MagnifySamplerFilter::Nearest)
    };

    let output = support::build_renderable_texture(&display);
    output.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    match output
        .as_surface()
        .draw(&vb, &ib, &program, &uniforms, &Default::default())
    {
        Ok(_) => (),
        Err(e) => panic!("{:?}", e),
    };

    let data: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
    assert_eq!(data[0][0], (255, 255, 255, 255));

    display.assert_no_error(None);
}

#[cfg(any(safegl_desktop, safegl_gles32))]
#[test]
fn border_color() {
    let display = support::build_display();
    // border colors need OpenGL ES 3.2
    if !support::glsl_supported(&display, 330, 320) {
        return;
    }
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let header = if support::is_gles(&display) {
        "#version 320 es\nprecision highp float;\n"
    } else {
        "#version 330\n"
    };
    let program = safegl::Program::from_source(
        &display,
        &format!(
            "{header}in vec2 position; void main() {{ gl_Position = vec4(position, 0.0, 1.0); }}"
        ),
        // a texture coordinate far outside the texture
        &format!(
            "{header}uniform sampler2D tex; out vec4 f_color;
                  void main() {{ f_color = texture(tex, vec2(4.0, 4.0)); }}"
        ),
        None,
    )
    .unwrap();

    let texture = support::build_unicolor_texture2d(&display, 1.0, 0.0, 0.0);
    let sampler = texture
        .sampled()
        .wrap_function(safegl::uniforms::SamplerWrapFunction::BorderClamp)
        .border_color([0.0, 1.0, 0.0, 1.0]);

    let output = support::build_renderable_texture(&display);
    output
        .as_surface()
        .draw(
            &vb,
            &ib,
            &program,
            &uniform! { tex: sampler },
            &Default::default(),
        )
        .unwrap();

    let data: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
    assert_eq!(data[0][0], (0, 255, 0, 255));
    display.assert_no_error(None);
}

#[test]
fn sampler_value_outlives_the_sampler() {
    let display = support::build_display();
    let texture = support::build_unicolor_texture2d(&display, 1.0, 0.0, 0.0);

    // the value borrows the texture, not the temporary sampler
    let value: safegl::uniforms::UniformValue<'_> = texture
        .sampled()
        .magnify_filter(safegl::uniforms::MagnifySamplerFilter::Nearest)
        .into();
    assert!(matches!(
        value,
        safegl::uniforms::UniformValue::Texture(_, Some(_))
    ));
}

/// Anisotropic filtering is core in OpenGL 4.6, whatever the extension strings.
#[test]
fn anisotropy_in_core_opengl_46() {
    let display = support::build_display();
    let extensions = display.get_extensions();

    let anisotropy = display.get_opengl_version() >= &Version(Api::Gl, 4, 6)
        || extensions.gl_arb_texture_filter_anisotropic
        || extensions.gl_ext_texture_filter_anisotropic;
    let max = display.get_capabilities().max_texture_max_anisotropy;
    assert_eq!(max.is_some(), anisotropy);
    if let Some(max) = max {
        assert!(max >= 2.0);
    }

    display.assert_no_error(None);
}

/// Draws a texture sampled with `sampler` and returns the result of the draw.
fn draw_sampled(
    display: &support::TestDisplay,
    sampler: safegl::uniforms::Sampler<'_, safegl::Texture2d>,
) -> Result<(), safegl::DrawError> {
    let (vb, ib) = support::build_rectangle_vb_ib(display);
    let program = support::program_from_legacy(
        display,
        "attribute vec2 position; varying vec2 v; \
         void main() { v = position; gl_Position = vec4(position, 0.0, 1.0); }",
        "uniform sampler2D tex; varying vec2 v; void main() { gl_FragColor = texture2D(tex, v); }",
    )
    .unwrap();
    let output = support::build_renderable_texture(display);
    output.as_surface().draw(
        &vb,
        &ib,
        &program,
        &uniform! { tex: sampler },
        &Default::default(),
    )
}

/// Anisotropic filtering where the backend doesn't have it is an error, not a sampler without
/// it; above the maximum of the backend, it is clamped.
#[test]
fn anisotropy_needs_support() {
    let display = support::build_display();
    let texture = support::build_unicolor_texture2d(&display, 1.0, 0.0, 0.0);
    let result = draw_sampled(&display, texture.sampled().anisotropy(u16::MAX));
    if display.get_capabilities().max_texture_max_anisotropy.is_some() {
        result.unwrap();
    } else {
        assert!(matches!(
            result,
            Err(safegl::DrawError::SamplerNotSupported(
                safegl::uniforms::SamplerNotSupportedError::Anisotropy
            ))
        ));
    }
    display.assert_no_error(None);
}

/// Mirroring once is an error where the backend doesn't have it, not a clamp.
#[cfg(any(safegl_gl44, safegl_webgl))]
#[test]
fn mirror_clamp_needs_support() {
    use safegl::uniforms::{SamplerNotSupportedError, SamplerWrapFunction};

    let display = support::build_display();
    let texture = support::build_unicolor_texture2d(&display, 1.0, 0.0, 0.0);
    let result = draw_sampled(
        &display,
        texture.sampled().wrap_function(SamplerWrapFunction::MirrorClamp),
    );
    let extensions = support::extensions(&display);
    if support::gl_at_least(&display, 4, 4)
        || extensions.gl_arb_texture_mirror_clamp_to_edge
        || extensions.gl_ext_texture_mirror_clamp_to_edge
    {
        result.unwrap();
    } else {
        assert!(matches!(
            result,
            Err(safegl::DrawError::SamplerNotSupported(
                SamplerNotSupportedError::WrapFunction(SamplerWrapFunction::MirrorClamp)
            ))
        ));
    }
    display.assert_no_error(None);
}

/// Mirroring once then clamping (`MIRROR_CLAMP_TO_EDGE`) samples the other edge of a texture
/// one texture away, where clamping samples the near edge.
#[cfg(any(safegl_gl44, safegl_webgl))]
#[test]
fn mirror_clamp_samples_the_mirrored_edge() {
    use safegl::uniforms::{MagnifySamplerFilter, MinifySamplerFilter, SamplerWrapFunction};

    let display = support::build_display();
    let extensions = support::extensions(&display);
    if !(support::gl_at_least(&display, 4, 4)
        || extensions.gl_arb_texture_mirror_clamp_to_edge
        || extensions.gl_ext_texture_mirror_clamp_to_edge)
    {
        return;
    }
    // red then green
    let texture = safegl::Texture2d::new(
        &display,
        vec![vec![(255u8, 0u8, 0u8, 255u8), (0, 255, 0, 255)]],
    )
    .unwrap();
    let program = support::program_from_legacy(
        &display,
        "attribute vec2 position; void main() { gl_Position = vec4(position, 0.0, 1.0); }",
        // one texture to the left of it, mirrored onto its right edge
        "uniform sampler2D tex; void main() { gl_FragColor = texture2D(tex, vec2(-1.75, 0.5)); }",
    )
    .unwrap();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let sample = |wrap| {
        let output = support::build_renderable_texture(&display);
        let sampler = texture
            .sampled()
            .wrap_function(wrap)
            .minify_filter(MinifySamplerFilter::Nearest)
            .magnify_filter(MagnifySamplerFilter::Nearest);
        output
            .as_surface()
            .draw(&vb, &ib, &program, &uniform! { tex: sampler }, &Default::default())
            .unwrap();
        let pixels: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
        pixels[0][0]
    };
    assert_eq!(sample(SamplerWrapFunction::MirrorClamp), (0, 255, 0, 255));
    assert_eq!(sample(SamplerWrapFunction::Clamp), (255, 0, 0, 255));
    display.assert_no_error(None);
}

/// Cube map arrays are sampled by direction and layer (`samplerCubeArray`, GLSL 4.00).
#[cfg(safegl_desktop)]
#[test]
fn cube_map_arrays_are_sampled() {
    use safegl::framebuffer::SimpleFrameBuffer;
    use safegl::texture::{CubeLayer, CubemapArray};

    let display = support::build_display();
    if support::is_gles(&display) || !support::glsl_supported(&display, 400, 0) {
        return;
    }
    let cubes = match CubemapArray::empty(&display, 2, 2) {
        Ok(cubes) => cubes,
        Err(_) => return,
    };
    for layer in 0..2 {
        for face in [CubeLayer::PositiveX, CubeLayer::NegativeX] {
            let image = cubes.main_level().layer(layer).unwrap().image(face);
            let green = if layer == 1 && face == CubeLayer::PositiveX { 1.0 } else { 0.0 };
            SimpleFrameBuffer::new(&display, image)
                .unwrap()
                .clear_color(1.0 - green, green, 0.0, 1.0);
        }
    }
    let program = safegl::Program::from_source(
        &display,
        "#version 400\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
        "#version 400
        uniform samplerCubeArray cubes;
        out vec4 color;
        void main() { color = texture(cubes, vec4(1.0, 0.0, 0.0, 1.0)); }",
        None,
    )
    .unwrap();
    let (vb, ib) = support::build_rectangle_vb_ib(&display);
    let output = support::build_renderable_texture(&display);
    output
        .as_surface()
        .draw(&vb, &ib, &program, &uniform! { cubes: &cubes }, &Default::default())
        .unwrap();
    let pixels: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
    assert_eq!(pixels[0][0], (0, 255, 0, 255));
    display.assert_no_error(None);
}
