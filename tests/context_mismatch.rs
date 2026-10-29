//! OpenGL objects can only be used with the context that created them: using them with another
//! context returns an error instead of using whatever object has the same name there.

#[macro_use]
extern crate safegl;

use safegl::Surface;

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn assert_mismatch(result: Result<(), safegl::DrawError>) {
    assert!(
        matches!(result, Err(safegl::DrawError::ContextMismatch { .. })),
        "{:?}",
        result
    );
}

#[test]
fn draw_with_objects_of_another_context() {
    let a = support::build_display();
    let b = support::build_display();

    let (vb_a, ib_a, program_a) = support::build_fullscreen_red_pipeline(&a);
    let (vb_b, ib_b, program_b) = support::build_fullscreen_red_pipeline(&b);
    let target = support::build_renderable_texture(&a);
    let uniforms = safegl::uniforms::EmptyUniforms;
    let params = Default::default();

    // everything from the right context works
    target
        .as_surface()
        .draw(&vb_a, &ib_a, &program_a, &uniforms, &params)
        .unwrap();

    // the program, the vertices or the indices of another context don't
    assert_mismatch(
        target
            .as_surface()
            .draw(&vb_a, &ib_a, &program_b, &uniforms, &params),
    );
    assert_mismatch(
        target
            .as_surface()
            .draw(&vb_b, &ib_a, &program_a, &uniforms, &params),
    );
    assert_mismatch(
        target
            .as_surface()
            .draw(&vb_a, &ib_b, &program_a, &uniforms, &params),
    );

    // nor does a query of another context
    if let Ok(query) = safegl::draw_parameters::AnySamplesPassedQuery::new(&b, false) {
        let params = safegl::DrawParameters {
            samples_passed_query: Some((&query).into()),
            ..Default::default()
        };
        assert_mismatch(
            target
                .as_surface()
                .draw(&vb_a, &ib_a, &program_a, &uniforms, &params),
        );
    }

    a.assert_no_error(None);
    b.assert_no_error(None);
}

#[test]
fn uniforms_of_another_context() {
    let a = support::build_display();
    let b = support::build_display();

    let (vb, ib, _) = support::build_fullscreen_red_pipeline(&a);
    let program = program!(&a,
        330 => {
            vertex: "#version 330\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
            fragment: "#version 330\nuniform sampler2D tex;\nlayout(std140) uniform Block { vec4 color; };\nout vec4 f_color;\nvoid main() { f_color = texture(tex, vec2(0.5)) + color; }",
        },
        300 es => {
            vertex: "#version 300 es\nin vec2 position;\nvoid main() { gl_Position = vec4(position, 0.0, 1.0); }",
            fragment: "#version 300 es\nprecision highp float;\nuniform sampler2D tex;\nlayout(std140) uniform Block { vec4 color; };\nout vec4 f_color;\nvoid main() { f_color = texture(tex, vec2(0.5)) + color; }",
        },
    )
    .unwrap();
    let target = support::build_renderable_texture(&a);

    let texture_a = support::build_unicolor_texture2d(&a, 1.0, 0.0, 0.0);
    let texture_b = support::build_unicolor_texture2d(&b, 1.0, 0.0, 0.0);
    let block_a = safegl::uniforms::UniformBuffer::new(&a, [0.0f32; 4]).unwrap();
    let block_b = safegl::uniforms::UniformBuffer::new(&b, [0.0f32; 4]).unwrap();

    let draw = |texture: &safegl::Texture2d, block: &safegl::uniforms::UniformBuffer<[f32; 4]>| {
        target.as_surface().draw(
            &vb,
            &ib,
            &program,
            &uniform! { tex: texture, Block: block },
            &Default::default(),
        )
    };

    draw(&texture_a, &block_a).unwrap();
    assert_mismatch(draw(&texture_b, &block_a));
    assert_mismatch(draw(&texture_a, &block_b));

    a.assert_no_error(None);
    b.assert_no_error(None);
}

#[test]
fn framebuffer_with_attachments_of_another_context() {
    let a = support::build_display();
    let b = support::build_display();

    let texture_b = support::build_renderable_texture(&b);
    let result = safegl::framebuffer::SimpleFrameBuffer::new(&a, &texture_b);
    assert!(matches!(
        result,
        Err(safegl::framebuffer::ValidationError::ContextMismatch)
    ));

    a.assert_no_error(None);
    b.assert_no_error(None);
}

#[test]
fn blit_between_contexts() {
    let a = support::build_display();
    let b = support::build_display();

    let texture_a = support::build_renderable_texture(&a);
    let texture_b = support::build_renderable_texture(&b);
    let other_a = support::build_renderable_texture(&a);

    let rect = safegl::Rect {
        left: 0,
        bottom: 0,
        width: 16,
        height: 16,
    };
    let target = safegl::BlitTarget {
        left: 0,
        bottom: 0,
        width: 16,
        height: 16,
    };
    let filter = safegl::uniforms::MagnifySamplerFilter::Nearest;

    texture_a
        .as_surface()
        .blit_color(&rect, &other_a.as_surface(), &target, filter)
        .unwrap();
    assert_eq!(
        texture_a
            .as_surface()
            .blit_color(&rect, &texture_b.as_surface(), &target, filter),
        Err(safegl::BlitError::ContextMismatch)
    );

    a.assert_no_error(None);
    b.assert_no_error(None);
}

#[test]
fn copies_between_contexts() {
    let a = support::build_display();
    let b = support::build_display();

    // buffers
    use safegl::buffer::{Buffer, BufferMode, BufferType};
    let buffer_a =
        Buffer::new(&a, &[1u32, 2, 3][..], BufferType::ArrayBuffer, BufferMode::Default).unwrap();
    let buffer_b =
        Buffer::new(&b, &[0u32, 0, 0][..], BufferType::ArrayBuffer, BufferMode::Default).unwrap();
    assert!(matches!(
        buffer_a.copy_to(&buffer_b),
        Err(safegl::buffer::CopyError::ContextMismatch)
    ));

    // images, which only the `gl46` tier copies
    let texture_a = safegl::Texture2d::empty(&a, 4, 4).unwrap();
    #[cfg(any(safegl_gl43, safegl_gles30))]
    {
        let texture_b = safegl::Texture2d::empty(&b, 4, 4).unwrap();
        let result = texture_a.main_level().copy_to(&texture_b.main_level());
        assert!(matches!(
            result,
            Err(safegl::texture::CopyImageError::ContextMismatch)
                | Err(safegl::texture::CopyImageError::NotSupported)
        ));
    }

    // pixel buffers
    let pixels_b = safegl::texture::pixel_buffer::PixelBuffer::<(u8, u8, u8, u8)>::new_empty(&b, 16);
    let image_a = texture_a.main_level().first_layer().into_image(None).unwrap();
    let rect = safegl::Rect {
        left: 0,
        bottom: 0,
        width: 4,
        height: 4,
    };
    assert!(matches!(
        image_a.raw_read_to_pixel_buffer(&rect, &pixels_b),
        Err(safegl::ReadError::ContextMismatch)
    ));
    assert!(matches!(
        texture_a
            .main_level()
            .raw_upload_from_pixel_buffer(pixels_b.as_slice(), 0..4, 0..4, 0..1),
        Err(safegl::texture::UploadError::ContextMismatch)
    ));

    a.assert_no_error(None);
    b.assert_no_error(None);
}
