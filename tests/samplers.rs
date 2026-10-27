#[macro_use]
extern crate safegl;

use safegl::Surface;

mod support;

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
    110 => {
        vertex: "
                #version 110

                attribute vec2 position;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                }
            ",
        fragment: "
                #version 110

                uniform sampler2D texture;

                void main() {
                    gl_FragColor = texture2D(texture, vec2(0.51, 0.0));
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

                uniform lowp sampler2D texture;

                void main() {
                    gl_FragColor = texture2D(texture, vec2(0.51, 0.0));
                }
            ",
    })
    .unwrap();

    let texture_data = vec![vec![(0u8, 0, 0), (255, 255, 255)]];
    let texture = safegl::texture::Texture2d::new(&display, texture_data).unwrap();

    let uniforms = uniform! {
        texture: texture.sampled().magnify_filter(safegl::uniforms::MagnifySamplerFilter::Nearest)
    };

    let output = support::build_renderable_texture(&display);
    output.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    match output
        .as_surface()
        .draw(&vb, &ib, &program, &uniforms, &Default::default())
    {
        Ok(_) => (),
        Err(safegl::DrawError::SamplersNotSupported) => return,
        Err(e) => panic!("{:?}", e),
    };

    let data: Vec<Vec<(u8, u8, u8, u8)>> = output.read().unwrap();
    assert_eq!(data[0][0], (255, 255, 255, 255));

    display.assert_no_error(None);
}

#[test]
fn border_color() {
    let display = support::build_display();
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
