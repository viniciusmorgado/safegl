//! The features of tile-based GPUs: advanced blend equations, reads of the framebuffer by
//! fragment shaders, multisampled rendering to textures, pixel local storage and tiled
//! rendering.

#[macro_use]
extern crate safegl;

use safegl::draw_parameters::{is_advanced_blending_supported, AdvancedBlending};
use safegl::framebuffer::{
    ColorAttachment, DepthRenderBuffer, MultiOutputFrameBuffer, PixelLocalStorage,
    RenderBuffer, RenderBufferCreationError, SimpleFrameBuffer, TilePassError, TilePreserve,
    TiledRendering, ValidationError,
};
use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::{
    DepthFormat, MipmapsOption, Texture2dArray, TextureAnyImage, UncompressedFloatFormat,
};
use safegl::{
    Blend, BlendingFunction, Depth, DepthTest, DrawError, DrawParameters, Program, Rect,
    Surface, Texture2d, VertexBuffer,
};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Pixel = (u8, u8, u8, u8);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// The first line of the shaders, GLSL ES 3.00 (3.20 with the `gles32` extension, which gives
/// that version) or GLSL 3.30, then `#extension` lines that require the `extensions`, and the
/// precisions.
fn header(display: &support::Backend, extensions: &[&str]) -> String {
    let mut header = match (support::is_gles(display), extensions) {
        (true, ["gles32"]) => return "#version 320 es\nprecision highp float;\n".to_owned(),
        (true, _) => "#version 300 es\n".to_owned(),
        (false, _) => "#version 330\n".to_owned(),
    };
    for extension in extensions {
        header.push_str(&format!("#extension {extension} : require\n"));
    }
    header + "precision highp float;\n"
}

/// A program that draws `positions`, whose fragment shader is `fragment` with the `#extension`
/// lines of `extensions`.
fn program(display: &support::Backend, extensions: &[&str], fragment: &str) -> Program {
    let vertex = format!(
        "{}in vec2 position;\nvoid main() {{ gl_Position = vec4(position, 0.0, 1.0); }}",
        header(display, &[])
    );
    let fragment = format!("{}{fragment}", header(display, extensions));
    Program::from_source(display, &vertex, &fragment, None).unwrap()
}

/// Draws a triangle with `program`.
fn draw<S: Surface>(
    display: &support::Backend,
    surface: &mut S,
    positions: [[f32; 2]; 3],
    program: &Program,
    parameters: &DrawParameters<'_>,
) -> Result<(), DrawError> {
    let triangle =
        VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap();
    surface.draw(
        &triangle,
        NoIndices(PrimitiveType::TrianglesList),
        program,
        &uniform! {},
        parameters,
    )
}

/// A triangle that covers the whole surface.
const FULLSCREEN: [[f32; 2]; 3] = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];

/// Returns true if `pixel` is `expected`, give or take one in each component.
fn close(pixel: Pixel, expected: Pixel) -> bool {
    let near = |a: u8, b: u8| a.abs_diff(b) <= 1;
    near(pixel.0, expected.0)
        && near(pixel.1, expected.1)
        && near(pixel.2, expected.2)
        && near(pixel.3, expected.3)
}

fn all_pixels(texture: &Texture2d, expected: Pixel) -> bool {
    let pixels: Vec<Vec<Pixel>> = texture.read().unwrap();
    pixels.iter().flatten().all(|&pixel| close(pixel, expected))
}

#[test]
fn advanced_blending() {
    let display = support::build_display();
    let supported = is_advanced_blending_supported(&display);
    let khr = support::extensions(&display).gl_khr_blend_equation_advanced;
    let extensions: &[&str] = match (supported, khr) {
        (false, _) => &[],
        (true, true) => &["GL_KHR_blend_equation_advanced"],
        // OpenGL ES 3.2 without the extension: the qualifiers are core in GLSL ES 3.20
        (true, false) => &["gles32"],
    };
    let layout = if supported {
        "layout(blend_support_multiply) out;\nlayout(blend_support_screen) out;\n"
    } else {
        ""
    };
    let program = program(
        &display,
        extensions,
        &format!("{layout}out vec4 color;\nvoid main() {{ color = vec4(0.25, 1.0, 0.0, 1.0); }}"),
    );
    let target = Texture2d::empty(&display, 4, 4).unwrap();
    // 128 and 102: the values are chosen so that no blend ends halfway between two bytes
    target.as_surface().clear_color(0.5, 0.4, 0.0, 1.0);
    let parameters = |blend| DrawParameters {
        blend,
        ..Default::default()
    };

    let multiply = parameters(Blend::advanced(AdvancedBlending::Multiply));
    let mut surface = target.as_surface();
    let result = draw(&display, &mut surface, FULLSCREEN, &program, &multiply);
    if !supported {
        assert!(matches!(result, Err(DrawError::AdvancedBlendingNotSupported)));
        display.assert_no_error(None);
        return;
    }
    result.unwrap();
    // opaque colors are multiplied
    assert!(all_pixels(&target, (32, 102, 0, 255)));

    // again, the second draw blends with what the first wrote
    draw(&display, &mut surface, FULLSCREEN, &program, &multiply).unwrap();
    assert!(all_pixels(&target, (8, 102, 0, 255)));
    // source + destination - source × destination
    let screen = parameters(Blend::advanced(AdvancedBlending::Screen));
    draw(&display, &mut surface, FULLSCREEN, &program, &screen).unwrap();
    assert!(all_pixels(&target, (70, 255, 0, 255)));

    let mismatch = parameters(Blend {
        alpha: BlendingFunction::AlwaysReplace,
        ..Blend::advanced(AdvancedBlending::Multiply)
    });
    assert!(matches!(
        draw(&display, &mut surface, FULLSCREEN, &program, &mismatch),
        Err(DrawError::AdvancedBlendingMismatch)
    ));
    let undeclared = parameters(Blend::advanced(AdvancedBlending::Darken));
    assert!(matches!(
        draw(&display, &mut surface, FULLSCREEN, &program, &undeclared),
        Err(DrawError::AdvancedBlendingNotDeclared {
            equation: AdvancedBlending::Darken
        })
    ));

    // a single color output
    let outputs = program_with_two_outputs(&display, extensions);
    let first = Texture2d::empty(&display, 4, 4).unwrap();
    let second = Texture2d::empty(&display, 4, 4).unwrap();
    let mut framebuffer =
        MultiOutputFrameBuffer::new(&display, [("color", &first), ("other", &second)]).unwrap();
    assert!(matches!(
        draw(&display, &mut framebuffer, FULLSCREEN, &outputs, &multiply),
        Err(DrawError::AdvancedBlendingTooManyOutputs)
    ));

    display.assert_no_error(None);
}

/// A program that blends with every advanced equation, and writes two color outputs.
fn program_with_two_outputs(display: &support::Backend, extensions: &[&str]) -> Program {
    program(
        display,
        extensions,
        "layout(blend_support_all_equations) out;\n\
         layout(location = 0) out vec4 color;\nlayout(location = 1) out vec4 other;\n\
         void main() { color = vec4(1.0); other = vec4(1.0); }",
    )
}

/// The fragment shader reads the color of the pixel and adds to it, three times
/// (`GL_EXT_shader_framebuffer_fetch` and its variant without coherence).
#[test]
fn framebuffer_fetch() {
    let display = support::build_display();
    let extensions = support::extensions(&display);
    for (extension, has, qualifier) in [
        ("GL_EXT_shader_framebuffer_fetch", extensions.gl_ext_shader_framebuffer_fetch, ""),
        (
            "GL_EXT_shader_framebuffer_fetch_non_coherent",
            extensions.gl_ext_shader_framebuffer_fetch_non_coherent,
            ", noncoherent",
        ),
    ] {
        if !has || !support::is_gles(&display) {
            continue;
        }
        let program = program(
            &display,
            &[extension],
            &format!(
                "layout(location = 0{qualifier}) inout vec4 color;\n\
                 void main() {{ color += vec4(0.25, 0.0, 0.0, 0.0); }}"
            ),
        );
        let target = Texture2d::empty(&display, 4, 4).unwrap();
        target.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);
        let mut surface = target.as_surface();
        for _ in 0..3 {
            draw(&display, &mut surface, FULLSCREEN, &program, &Default::default()).unwrap();
        }
        assert!(all_pixels(&target, (191, 0, 0, 255)), "{extension}");
    }
    display.assert_no_error(None);
}

/// The fragment shader reads the depth of the pixel
/// (`GL_ARM_shader_framebuffer_fetch_depth_stencil`).
#[test]
fn depth_framebuffer_fetch() {
    let display = support::build_display();
    let extension = "GL_ARM_shader_framebuffer_fetch_depth_stencil";
    let has = support::extensions(&display).gl_arm_shader_framebuffer_fetch_depth_stencil;
    // llvmpipe (Mesa 26.2) crashes in LLVM when it compiles a shader that reads the depth
    let llvmpipe = display.get_opengl_renderer_string().contains("llvmpipe");
    if !has || !support::is_gles(&display) || llvmpipe {
        return;
    }
    let program = program(
        &display,
        &[extension],
        "out vec4 color;\nvoid main() { color = vec4(gl_LastFragDepthARM, 0.0, 0.0, 1.0); }",
    );
    let target = Texture2d::empty(&display, 4, 4).unwrap();
    let depth = DepthRenderBuffer::new(&display, DepthFormat::I24, 4, 4).unwrap();
    let mut framebuffer = SimpleFrameBuffer::with_depth_buffer(&display, &target, &depth).unwrap();
    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 0.25);
    draw(&display, &mut framebuffer, FULLSCREEN, &program, &Default::default()).unwrap();
    assert!(all_pixels(&target, (64, 0, 0, 255)));
    display.assert_no_error(None);
}

/// The number of pixels of `texture` whose red is neither 0 nor 255: the edges of triangles
/// drawn with samples, which cover part of them.
fn partly_covered(texture: &Texture2d) -> usize {
    let pixels: Vec<Vec<Pixel>> = texture.read().unwrap();
    pixels
        .iter()
        .flatten()
        .filter(|pixel| pixel.0 != 0 && pixel.0 != 255)
        .count()
}

/// A triangle with a slanted edge over a texture of 16x16 pixels.
const SLANTED: [[f32; 2]; 3] = [[-1.0, -1.0], [1.0, -0.6], [-1.0, 1.0]];

#[test]
fn multisampled_render_to_texture() {
    let display = support::build_display();
    let extensions = support::extensions(&display);
    let supported = extensions.gl_ext_multisampled_render_to_texture
        || extensions.gl_ext_multisampled_render_to_texture2;
    let white = program(
        &display,
        &[],
        "out vec4 color;\nvoid main() { color = vec4(1.0); }",
    );
    let texture = Texture2d::empty_with_format(
        &display,
        UncompressedFloatFormat::U8U8U8U8,
        MipmapsOption::NoMipmap,
        16,
        16,
    )
    .unwrap();
    let image: TextureAnyImage<'_> = texture.main_level().into();
    let depth = DepthRenderBuffer::new_multisample_resolved(&display, DepthFormat::I24, 16, 16, 4);
    if !supported {
        assert_eq!(
            SimpleFrameBuffer::new(&display, ColorAttachment::Texture(image.multisampled(4)))
                .err(),
            Some(ValidationError::MultisampledRenderToTextureNotSupported)
        );
        assert_eq!(
            depth.err(),
            Some(RenderBufferCreationError::ResolvedMultisampleNotSupported)
        );
        display.assert_no_error(None);
        return;
    }
    let depth = depth.unwrap();
    assert!(depth.is_multisample_resolved());

    // the edge is smoothed by the samples, which are resolved into the texture
    let mut framebuffer = SimpleFrameBuffer::with_depth_buffer(
        &display,
        ColorAttachment::Texture(image.multisampled(4)),
        &depth,
    )
    .unwrap();
    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
    let parameters = DrawParameters {
        depth: Depth {
            test: DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        ..Default::default()
    };
    draw(&display, &mut framebuffer, SLANTED, &white, &parameters).unwrap();
    drop(framebuffer);
    assert!(partly_covered(&texture) > 0);

    // without samples, every pixel is covered or not
    let mut framebuffer = SimpleFrameBuffer::new(&display, &texture).unwrap();
    framebuffer.clear_color(0.0, 0.0, 0.0, 1.0);
    draw(&display, &mut framebuffer, SLANTED, &white, &Default::default()).unwrap();
    assert_eq!(partly_covered(&texture), 0);

    // the multisampled attachments are all resolved, with the same samples
    let own = DepthRenderBuffer::new_multisample(&display, DepthFormat::I24, 16, 16, 4).unwrap();
    assert_eq!(
        SimpleFrameBuffer::with_depth_buffer(
            &display,
            ColorAttachment::Texture(image.multisampled(4)),
            &own
        )
        .err(),
        Some(ValidationError::SamplesCountMismatch)
    );
    assert_eq!(
        SimpleFrameBuffer::with_depth_buffer(
            &display,
            ColorAttachment::Texture(image.multisampled(2)),
            &depth
        )
        .err(),
        Some(ValidationError::SamplesCountMismatch)
    );
    assert_eq!(
        SimpleFrameBuffer::new(&display, ColorAttachment::Texture(image.multisampled(1000)))
            .err(),
        Some(ValidationError::SamplesNotSupported)
    );
    let array = Texture2dArray::empty(&display, 16, 16, 2).unwrap();
    let layer: TextureAnyImage<'_> = array.main_level().layer(0).unwrap().into();
    assert_eq!(
        SimpleFrameBuffer::new(&display, ColorAttachment::Texture(layer.multisampled(4))).err(),
        Some(ValidationError::MultisampledRenderToTextureTarget)
    );

    // render buffers resolved by the GPU are attachments of their own
    let color = RenderBuffer::new_multisample_resolved(
        &display,
        UncompressedFloatFormat::U8U8U8U8,
        16,
        16,
        4,
    )
    .unwrap();
    let mut framebuffer = SimpleFrameBuffer::with_depth_buffer(&display, &color, &depth).unwrap();
    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
    draw(&display, &mut framebuffer, SLANTED, &white, &parameters).unwrap();
    drop(framebuffer);

    // the second color attachment needs `GL_EXT_multisampled_render_to_texture2`
    let second = Texture2d::empty(&display, 16, 16).unwrap();
    let second: TextureAnyImage<'_> = second.main_level().into();
    let outputs = MultiOutputFrameBuffer::new(
        &display,
        [
            ("color", ColorAttachment::Texture(image.multisampled(4))),
            ("other", ColorAttachment::Texture(second.multisampled(4))),
        ],
    );
    if extensions.gl_ext_multisampled_render_to_texture2 {
        outputs.unwrap();
    } else {
        assert_eq!(
            outputs.err(),
            Some(ValidationError::MultisampledRenderToTextureNotSupported)
        );
    }

    display.assert_no_error(None);
}

/// Pixel local storage counts the draws of each pixel, which a last draw writes to the color.
#[test]
fn pixel_local_storage() {
    let display = support::build_display();
    let texture = Texture2d::empty(&display, 4, 4).unwrap();
    let mut framebuffer = SimpleFrameBuffer::new(&display, &texture).unwrap();
    // the storage starts at zero over pixels that are zero
    framebuffer.clear_color(0.0, 0.0, 0.0, 0.0);
    if !safegl::framebuffer::is_pixel_local_storage_supported(&display) {
        assert_eq!(
            PixelLocalStorage::new(&mut framebuffer, 4).err(),
            Some(TilePassError::NotSupported)
        );
        display.assert_no_error(None);
        return;
    }
    let extension = ["GL_EXT_shader_pixel_local_storage"];
    let count = program(
        &display,
        &extension,
        "__pixel_localEXT Storage { layout(r32f) highp float count; };\n\
         void main() { count += 0.25; }",
    );
    let resolve = program(
        &display,
        &extension,
        "__pixel_local_inEXT Storage { layout(r32f) highp float count; };\n\
         out vec4 color;\nvoid main() { color = vec4(count, 0.0, 0.0, 1.0); }",
    );
    assert!(matches!(
        draw(&display, &mut framebuffer, FULLSCREEN, &count, &Default::default()),
        Err(DrawError::PixelLocalStorageNotEnabled)
    ));
    assert_eq!(
        PixelLocalStorage::new(&mut framebuffer, 6).err(),
        Some(TilePassError::SizeNotSupported)
    );

    let second = support::extensions(&display).gl_ext_shader_pixel_local_storage2;
    let mut storage = PixelLocalStorage::new(&mut framebuffer, 4).unwrap();
    // the second version leaves the storage undefined until it is cleared
    if second {
        storage.clear(0, &[0]).unwrap();
    }
    for _ in 0..3 {
        draw(&display, &mut *storage, FULLSCREEN, &count, &Default::default()).unwrap();
    }
    draw(&display, &mut *storage, FULLSCREEN, &resolve, &Default::default()).unwrap();
    let past_the_end = if second {
        TilePassError::OutOfBounds
    } else {
        TilePassError::NotSupported
    };
    assert_eq!(storage.clear(0, &[0, 0]), Err(past_the_end));
    drop(storage);
    assert!(all_pixels(&texture, (191, 0, 0, 255)));

    display.assert_no_error(None);
}

/// Tiled rendering of the left half of a texture: the draws change only that half.
#[test]
fn tiled_rendering() {
    let display = support::build_display();
    let texture = Texture2d::empty(&display, 4, 4).unwrap();
    let mut framebuffer = SimpleFrameBuffer::new(&display, &texture).unwrap();
    framebuffer.clear_color(0.0, 0.0, 0.0, 1.0);
    let tile = Rect {
        left: 0,
        bottom: 0,
        width: 2,
        height: 4,
    };
    if !safegl::framebuffer::is_tiled_rendering_supported(&display) {
        assert_eq!(
            TiledRendering::new(&mut framebuffer, &tile, TilePreserve::ALL).err(),
            Some(TilePassError::NotSupported)
        );
        display.assert_no_error(None);
        return;
    }
    let red = program(
        &display,
        &[],
        "out vec4 color;\nvoid main() { color = vec4(1.0, 0.0, 0.0, 1.0); }",
    );
    let mut tiled = TiledRendering::new(&mut framebuffer, &tile, TilePreserve::ALL).unwrap();
    draw(&display, &mut *tiled, FULLSCREEN, &red, &Default::default()).unwrap();
    // a read of another texture suspends the rendering, which starts again for the next draw
    let other = Texture2d::empty(&display, 1, 1).unwrap();
    let _: Vec<Vec<Pixel>> = other.read().unwrap();
    draw(&display, &mut *tiled, FULLSCREEN, &red, &Default::default()).unwrap();
    tiled.finish(TilePreserve::ALL);

    let pixels: Vec<Vec<Pixel>> = texture.read().unwrap();
    for row in &pixels {
        assert!(row[..2].iter().all(|&pixel| pixel == (255, 0, 0, 255)));
        assert!(row[2..].iter().all(|&pixel| pixel == (0, 0, 0, 255)));
    }
    display.assert_no_error(None);
}
