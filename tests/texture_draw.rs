#[macro_use]
extern crate safegl;

use safegl::GlObject;
use safegl::Surface;

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

macro_rules! create_program {
    ($display:expr, $glsl_ty:expr, $glsl_value:expr) => {{
        let header = if support::is_gles(&$display) {
            "#version 300 es\nprecision highp float;\nprecision highp int;\n"
        } else {
            "#version 330\n"
        };
        safegl::Program::from_source(
            &$display,
            &format!(
                "{}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}",
                header
            ),
            &format!(
                "{}out {} color;\nvoid main() {{ color = {}; }}",
                header, $glsl_ty, $glsl_value
            ),
            None,
        )
        .unwrap()
    }};
}

macro_rules! draw_and_validate {
    ($display: expr, $program: expr, $texture:expr, $vb:expr, $ib:expr, $rust_value:expr) => {{
        $texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
        $texture
            .as_surface()
            .draw(
                &$vb,
                &$ib,
                &$program,
                &uniform! { texture: &$texture },
                &Default::default(),
            )
            .unwrap();

        $display.assert_no_error(None);

        let data: Vec<Vec<(u8, u8, u8, u8)>> = $texture.read().unwrap();
        for row in data.iter() {
            for pixel in row.iter() {
                assert_eq!(pixel, &$rust_value);
            }
        }

        $display.assert_no_error(None);
    }};
}

macro_rules! texture_draw_test {
    ($test_name:ident, $tex_ty:ident, [$($dims:expr),+], $glsl_ty:expr, $glsl_value:expr,
     $rust_value:expr) => (
        #[test]
        fn $test_name() {
            let display = support::build_display();
            let (vb, ib) = support::build_rectangle_vb_ib(&display);

            let program = create_program!(display, $glsl_ty, $glsl_value);

            let texture = safegl::texture::$tex_ty::empty(&display, $($dims),+).unwrap();

            draw_and_validate!(display, program, texture, vb, ib, $rust_value);
        }
    );
}

macro_rules! unowned_draw_test {
    ($test_name:ident, $tex_ty:ident, $format_ty:ident, $format_value:ident, [$($dims:expr),+], $glsl_ty:expr, $glsl_value:expr,
     $rust_value:expr) => (
        #[test]
        fn $test_name() {
            let display = support::build_display();
            let (vb, ib) = support::build_rectangle_vb_ib(&display);

            let program = create_program!(display, $glsl_ty, $glsl_value);

            let empty_texture = safegl::texture::$tex_ty::empty_with_format(&display,
                                                                           safegl::texture::$format_ty::$format_value,
                                                                           safegl::texture::MipmapsOption::NoMipmap,
                                                                           $($dims),+).unwrap();
            let texture = unsafe {
                safegl::texture::$tex_ty::from_id(&display,
                                                 safegl::texture::$format_ty::$format_value,
                                                 empty_texture.get_id(),
                                                 false,
                                                 safegl::texture::MipmapsOption::NoMipmap,
                                                 empty_texture.get_texture_type())
            };

            draw_and_validate!(display, program, texture, vb, ib, $rust_value);
        }
    );
}

texture_draw_test!(
    texture_2d_draw,
    Texture2d,
    [1024, 1024],
    "vec4",
    "vec4(1.0, 0.0, 1.0, 0.0)",
    (255, 0, 255, 0)
);
unowned_draw_test!(
    texture_2d_draw_unowned,
    Texture2d,
    UncompressedFloatFormat,
    F32F32F32F32,
    [1024, 1024],
    "vec4",
    "vec4(1.0, 0.0, 1.0, 0.0)",
    (255, 0, 255, 0)
);
