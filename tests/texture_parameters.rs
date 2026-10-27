//! Parameters of textures and samplers: swizzle, mipmap range, depth-stencil texture mode,
//! levels of detail and border colors.

#[macro_use]
extern crate safegl;

use safegl::index::{NoIndices, PrimitiveType};
use safegl::texture::{MipmapRangeError, MipmapsOption, Texture2d};
use safegl::uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Uniforms};
use safegl::{DrawError, Program, Surface, VertexBuffer};

mod support;

type Display = safegl::Display<safegl::glutin::surface::WindowSurface>;
type Pixel = (u8, u8, u8, u8);

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A program whose fragment shader declares `declarations` and runs `body`, which writes
/// `color`, compiled as GLSL `desktop` (330, 430, ...) or GLSL ES `es` (300, 310, ...).
fn program(display: &Display, desktop: u32, es: u32, declarations: &str, body: &str) -> Program {
    let header = if support::is_gles(display) {
        format!(
            "#version {es} es\nprecision highp float;\nprecision highp usampler2D;\n\
             precision highp isampler2D;\n"
        )
    } else {
        format!("#version {desktop} core\n")
    };
    let vertex = format!(
        "{header}in vec2 position;\nout vec2 uv;\n\
         void main() {{ uv = position * 0.5 + 0.5; gl_Position = vec4(position, 0.0, 1.0); }}"
    );
    let fragment =
        format!("{header}in vec2 uv;\n{declarations}\nout vec4 color;\nvoid main() {{ {body} }}");
    Program::from_source(display, &vertex, &fragment, None).unwrap()
}

/// Draws a fullscreen triangle with `program` and returns the color of a pixel.
fn draw(
    display: &Display,
    program: &Program,
    uniforms: &impl Uniforms,
) -> Result<Pixel, DrawError> {
    let target = Texture2d::empty(display, 4, 4).unwrap();
    target.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    let positions = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let triangle =
        VertexBuffer::new(display, &positions.map(|position| Vertex { position })).unwrap();
    target.as_surface().draw(
        &triangle,
        NoIndices(PrimitiveType::TrianglesList),
        program,
        uniforms,
        &Default::default(),
    )?;
    let data: Vec<Vec<Pixel>> = target.read().unwrap();
    Ok(data[1][1])
}

#[cfg(any(safegl_desktop, safegl_gles30))]
#[test]
fn swizzle() {
    use safegl::texture::{Swizzle, SwizzleSource};

    let display = support::build_display();
    let texture = Texture2d::new(&display, vec![vec![(100u8, 20u8, 30u8, 50u8); 2]; 2]).unwrap();
    let program = program(
        &display,
        330,
        300,
        "uniform sampler2D tex;",
        "color = texture(tex, uv);",
    );
    let sample = || draw(&display, &program, &uniform! { tex: &texture }).unwrap();

    assert_eq!(texture.swizzle(), Swizzle::default());
    let gray = Swizzle {
        r: SwizzleSource::Red,
        g: SwizzleSource::Red,
        b: SwizzleSource::Red,
        a: SwizzleSource::One,
    };
    texture.set_swizzle(gray);
    assert_eq!(texture.swizzle(), gray);
    assert_eq!(sample(), (100, 100, 100, 255));

    texture.set_swizzle(Swizzle {
        r: SwizzleSource::Alpha,
        g: SwizzleSource::Blue,
        b: SwizzleSource::Green,
        a: SwizzleSource::Zero,
    });
    assert_eq!(sample(), (50, 30, 20, 0));

    // reads aren't swizzled
    let data: Vec<Vec<Pixel>> = texture.read().unwrap();
    assert_eq!(data[0][0], (100, 20, 30, 50));

    display.assert_no_error(None);
}

/// A texture of 4 × 4 texels with 3 levels, each of its own color: level `n` is `(n × 100, 0, 0,
/// 255)`.
fn levels(display: &Display) -> Texture2d {
    let texture =
        Texture2d::empty_with_mipmaps(display, MipmapsOption::EmptyMipmaps, 4, 4).unwrap();
    assert_eq!(texture.get_mipmap_levels(), 3);
    for level in 0..3 {
        let mipmap = texture.mipmap(level).unwrap();
        let size = mipmap.width();
        mipmap
            .write(
                safegl::Rect {
                    left: 0,
                    bottom: 0,
                    width: size,
                    height: size,
                },
                vec![vec![((level * 100) as u8, 0u8, 0u8, 255u8); size as usize]; size as usize],
            )
            .unwrap();
    }
    texture
}

#[test]
fn mipmap_range() {
    let display = support::build_display();
    let texture = levels(&display);
    let program = program(
        &display,
        330,
        300,
        "uniform sampler2D tex;\nuniform float lod;",
        "color = textureLod(tex, uv, lod);",
    );
    let sampler = texture
        .sampled()
        .minify_filter(MinifySamplerFilter::NearestMipmapNearest)
        .magnify_filter(MagnifySamplerFilter::Nearest);
    let sample = |lod: f32| {
        draw(&display, &program, &uniform! { tex: sampler, lod: lod })
            .unwrap()
            .0
    };

    assert_eq!(texture.mipmap_range(), (0, 2));
    assert_eq!(sample(0.0), 0);
    assert_eq!(sample(2.0), 200);

    // the levels are counted from the base level, and stop at the maximum level
    texture.set_mipmap_range(1, 1).unwrap();
    assert_eq!(texture.mipmap_range(), (1, 1));
    assert_eq!(sample(0.0), 100);
    assert_eq!(sample(2.0), 100);

    assert_eq!(
        texture.set_mipmap_range(2, 1),
        Err(MipmapRangeError::BaseAfterMax)
    );
    assert_eq!(
        texture.set_mipmap_range(0, 3),
        Err(MipmapRangeError::LevelOutOfRange { levels: 3 })
    );
    assert_eq!(texture.mipmap_range(), (1, 1));

    texture.set_mipmap_range(0, 2).unwrap();
    assert_eq!(sample(1.0), 100);

    display.assert_no_error(None);
}

/// Sampling a depth-stencil texture returns its depth or its stencil index.
#[cfg(any(safegl_gl43, safegl_gles31))]
#[test]
fn depth_stencil_texture_mode() {
    use safegl::framebuffer::SimpleFrameBuffer;
    use safegl::texture::{
        DepthStencilFormat, DepthStencilTexture2d, DepthStencilTextureMode,
        DepthStencilTextureModeError,
    };

    let display = support::build_display();
    if !support::glsl_supported(&display, 430, 310) {
        return;
    }
    let depth_stencil = DepthStencilTexture2d::empty_with_format(
        &display,
        DepthStencilFormat::I24I8,
        MipmapsOption::NoMipmap,
        4,
        4,
    )
    .unwrap();
    let color = Texture2d::empty(&display, 4, 4).unwrap();
    SimpleFrameBuffer::with_depth_stencil_buffer(&display, &color, &depth_stencil)
        .unwrap()
        .clear_depth_and_stencil(0.25, 7);

    match depth_stencil.set_depth_stencil_mode(DepthStencilTextureMode::Stencil) {
        Err(DepthStencilTextureModeError::NotSupported) => return,
        result => result.unwrap(),
    }
    assert_eq!(
        depth_stencil.depth_stencil_mode(),
        DepthStencilTextureMode::Stencil
    );
    let stencil = program(
        &display,
        430,
        310,
        "uniform usampler2D tex;",
        "color = vec4(texelFetch(tex, ivec2(0), 0)) / 255.0;",
    );
    // with the texture's own filters, and with a sampler whose filters are linear
    let sampled = depth_stencil
        .sampled()
        .magnify_filter(MagnifySamplerFilter::Linear);
    assert_eq!(
        draw(&display, &stencil, &uniform! { tex: &depth_stencil })
            .unwrap()
            .0,
        7
    );
    assert_eq!(
        draw(&display, &stencil, &uniform! { tex: sampled })
            .unwrap()
            .0,
        7
    );

    // a float sampler no longer samples it
    let depth = program(
        &display,
        430,
        310,
        "uniform sampler2D tex;",
        "color = texture(tex, vec2(0.5));",
    );
    assert!(matches!(
        draw(&display, &depth, &uniform! { tex: &depth_stencil }),
        Err(DrawError::UniformTypeMismatch { .. })
    ));

    depth_stencil
        .set_depth_stencil_mode(DepthStencilTextureMode::Depth)
        .unwrap();
    let red = draw(&display, &depth, &uniform! { tex: &depth_stencil })
        .unwrap()
        .0;
    assert!((63..=65).contains(&red), "{red}");

    let not_depth_stencil = Texture2d::empty(&display, 1, 1).unwrap();
    assert_eq!(
        not_depth_stencil.set_depth_stencil_mode(DepthStencilTextureMode::Stencil),
        Err(DepthStencilTextureModeError::NotDepthStencil)
    );

    display.assert_no_error(None);
}

/// Stencil textures are sampled as unsigned integers.
#[cfg(any(safegl_gl44, safegl_gles32))]
#[test]
fn stencil_texture_sampling() {
    use safegl::framebuffer::SimpleFrameBuffer;
    use safegl::texture::{StencilFormat, StencilTexture2d, TextureCreationError};

    let display = support::build_display();
    if !support::glsl_supported(&display, 330, 320) {
        return;
    }
    let stencil = match StencilTexture2d::empty_with_format(
        &display,
        StencilFormat::I8,
        MipmapsOption::NoMipmap,
        4,
        4,
    ) {
        Ok(texture) => texture,
        Err(TextureCreationError::TypeNotSupported | TextureCreationError::FormatNotSupported) => {
            return
        }
        Err(error) => panic!("{error:?}"),
    };
    let color = Texture2d::empty(&display, 4, 4).unwrap();
    SimpleFrameBuffer::with_stencil_buffer(&display, &color, &stencil)
        .unwrap()
        .clear_stencil(9);

    let program = program(
        &display,
        330,
        320,
        "uniform usampler2D tex;",
        "color = vec4(texture(tex, uv)) / 255.0;",
    );
    assert_eq!(
        draw(&display, &program, &uniform! { tex: &stencil })
            .unwrap()
            .0,
        9
    );
    // a sampler with linear filters samples it with the nearest ones
    let sampler = stencil
        .sampled()
        .magnify_filter(MagnifySamplerFilter::Linear);
    assert_eq!(
        draw(&display, &program, &uniform! { tex: sampler })
            .unwrap()
            .0,
        9
    );

    display.assert_no_error(None);
}
