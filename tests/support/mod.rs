/*!
Test supports module.

*/

#![allow(dead_code)]

use safegl::backend::Facade;
use safegl::index::PrimitiveType;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::*;

/// Defines a test whose body waits, with `support::until(|| ready).await`, for the results of
/// queries and fences, which WebGL only gives in a later task of the browser's event loop: an
/// `async` test in browsers, and elsewhere a test run to completion at once, since there the
/// results can be waited for.
#[allow(unused_macros)]
macro_rules! async_test {
    ($(#[$attribute:meta])* fn $name:ident() $body:block) => {
        #[cfg(not(target_arch = "wasm32"))]
        $(#[$attribute])*
        #[test]
        fn $name() {
            support::block_on(async $body)
        }

        #[cfg(target_arch = "wasm32")]
        $(#[$attribute])*
        #[wasm_bindgen_test::wasm_bindgen_test]
        async fn $name() $body
    };
}
#[allow(unused_imports)]
pub(crate) use async_test;

/// Returns true if the context compiles GLSL `desktop` on desktop OpenGL, or GLSL `es` ES on
/// OpenGL ES, written like `#version` numbers (430, 310).
///
/// A context can have the API of a feature through an extension without the GLSL version that
/// the tests write their shaders in.
pub fn glsl_supported(display: &Backend, desktop: u32, es: u32) -> bool {
    let version = if is_gles(display) {
        safegl::Version(safegl::Api::GlEs, (es / 100) as u8, (es / 10 % 10) as u8)
    } else {
        safegl::Version(safegl::Api::Gl, (desktop / 100) as u8, (desktop / 10 % 10) as u8)
    };
    display.is_glsl_version_supported(&version)
}

/// The first lines of a desktop shader that needs GLSL `desktop` or the GLSL `extensions` that
/// give the same functionality: `#version desktop` when the context compiles that version,
/// otherwise the highest GLSL 4.x version of the context with an `#extension ... : require`
/// line for each extension, when the context has all of them.
///
/// The flags of `extensions` come from `get_extensions()`. Returns `None` on OpenGL ES, and when
/// the context has neither the version nor the extensions.
pub fn glsl_header(
    display: &Backend,
    desktop: u32,
    extensions: &[(&str, bool)],
) -> Option<String> {
    if is_gles(display) {
        return None;
    }
    if glsl_supported(display, desktop, 0) {
        return Some(format!("#version {desktop}\n"));
    }
    if extensions.is_empty() || extensions.iter().any(|&(_, supported)| !supported) {
        return None;
    }
    let base = (400..desktop)
        .step_by(10)
        .rev()
        .find(|&version| glsl_supported(display, version, 0))?;
    let mut header = format!("#version {base}\n");
    for (name, _) in extensions {
        header.push_str(&format!("#extension {name} : require\n"));
    }
    Some(header)
}

/// Returns true on desktop OpenGL `major.minor` or above, for the tests of features that a lower
/// driver can only have through an extension.
pub fn gl_at_least(display: &Backend, major: u8, minor: u8) -> bool {
    *display.get_opengl_version() >= safegl::Version(safegl::Api::Gl, major, minor)
}

/// The extensions of the context, as SafeGL sees them (without the ones that
/// `SAFEGL_DISABLE_EXTENSIONS` hides).
pub fn extensions(display: &Backend) -> safegl::ExtensionsList {
    use safegl::CapabilitiesSource;
    *display.get_context().get_extensions()
}

/// Returns true if the driver has the extension `name`, like `GL_ARB_derivative_control`, for
/// the extensions that only shaders use, which SafeGL doesn't list. The extensions that
/// `SAFEGL_DISABLE_EXTENSIONS` hides are missing, like for SafeGL.
pub fn has_extension(display: &Backend, name: &str) -> bool {
    use safegl::glow::HasContext;
    let hidden = disabled_extensions();
    if hidden.split_whitespace().any(|hidden| hidden == name) {
        return false;
    }
    unsafe { display.glow_context().supported_extensions().contains(name) }
}

/// Returns true if the display uses OpenGL ES, for the few tests of features it doesn't have.
pub fn is_gles(display: &Backend) -> bool {
    display.get_opengl_version().0 == safegl::Api::GlEs
}

/// Builds a 2x2 unicolor texture.
pub fn build_unicolor_texture2d<F: ?Sized>(
    facade: &F,
    red: f32,
    green: f32,
    blue: f32,
) -> safegl::Texture2d
where
    F: Facade,
{
    let color = (
        (red * 255.0) as u8,
        (green * 255.0) as u8,
        (blue * 255.0) as u8,
    );

    safegl::texture::Texture2d::new(facade, vec![vec![color, color], vec![color, color]]).unwrap()
}

/// Builds a 2x2 depth texture.
pub fn build_constant_depth_texture<F>(facade: &F, depth: f32) -> safegl::texture::DepthTexture2d
where
    F: Facade + ?Sized,
{
    safegl::texture::DepthTexture2d::new(facade, vec![vec![depth, depth], vec![depth, depth]])
        .unwrap()
}

/// Builds a vertex buffer, index buffer, and program, to draw red `(1.0, 0.0, 0.0, 1.0)` to the whole screen.
pub fn build_fullscreen_red_pipeline<F: ?Sized>(
    facade: &F,
) -> (
    safegl::vertex::VertexBufferAny,
    safegl::index::IndexBufferAny,
    safegl::Program,
)
where
    F: Facade,
{
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
    }

    implement_vertex!(Vertex, position);

    (
        safegl::VertexBuffer::new(
            facade,
            &[
                Vertex {
                    position: [-1.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, -1.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                },
            ],
        )
        .unwrap()
        .into(),
        safegl::IndexBuffer::new(facade, PrimitiveType::TriangleStrip, &[0u8, 1, 2, 3])
            .unwrap()
            .into(),
        program!(facade,
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

                    void main() {
                        f_color = vec4(1.0, 0.0, 0.0, 1.0);
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

                    void main() {
                        gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
                    }
                ",
            },
        )
        .unwrap(),
    )
}

/// Builds a vertex buffer and an index buffer corresponding to a rectangle.
///
/// The vertex buffer has the "position" attribute of type "vec2".
pub fn build_rectangle_vb_ib<F: ?Sized>(
    facade: &F,
) -> (
    safegl::vertex::VertexBufferAny,
    safegl::index::IndexBufferAny,
)
where
    F: Facade,
{
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
    }

    implement_vertex!(Vertex, position);

    (
        safegl::VertexBuffer::new(
            facade,
            &[
                Vertex {
                    position: [-1.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, -1.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                },
            ],
        )
        .unwrap()
        .into(),
        safegl::IndexBuffer::new(facade, PrimitiveType::TriangleStrip, &[0u8, 1, 2, 3])
            .unwrap()
            .into(),
    )
}

/// Same as `build_rectangle_vb_ib`, as a list of two triangles drawn without indices, for
/// transform feedback on OpenGL ES 3.0 and 3.1, which only captures such draws.
pub fn build_rectangle_triangles<F: ?Sized>(
    facade: &F,
) -> (safegl::vertex::VertexBufferAny, safegl::index::NoIndices)
where
    F: Facade,
{
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
    }

    implement_vertex!(Vertex, position);

    let vertex = |x, y| Vertex { position: [x, y] };
    let vertices = [
        vertex(-1.0, 1.0),
        vertex(1.0, 1.0),
        vertex(-1.0, -1.0),
        vertex(-1.0, -1.0),
        vertex(1.0, 1.0),
        vertex(1.0, -1.0),
    ];
    (
        safegl::VertexBuffer::new(facade, &vertices).unwrap().into(),
        safegl::index::NoIndices(PrimitiveType::TrianglesList),
    )
}

/// Builds a texture suitable for rendering.
pub fn build_renderable_texture<F: ?Sized>(facade: &F) -> safegl::Texture2d
where
    F: Facade,
{
    safegl::Texture2d::empty(facade, 1024, 1024).unwrap()
}

/// Translates a GLSL 1.10 shader to the GLSL of the context: `#version 330` on desktop OpenGL,
/// and `#version 300 es` with default precisions on OpenGL ES, whose drivers don't all accept
/// GLSL 1.10.
pub fn modern_glsl(display: &Backend, source: &str, fragment: bool) -> String {
    /// Replaces the identifier `word` by `by` where it isn't part of a longer identifier.
    fn replace_word(line: &str, word: &str, by: &str) -> String {
        let is_identifier = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let mut out = String::with_capacity(line.len());
        let mut rest = line;
        while let Some(position) = rest.find(word) {
            let before = rest[..position].chars().next_back();
            let after = rest[position + word.len()..].chars().next();
            out.push_str(&rest[..position]);
            if before.is_some_and(is_identifier) || after.is_some_and(is_identifier) {
                out.push_str(word);
            } else {
                out.push_str(by);
            }
            rest = &rest[position + word.len()..];
        }
        out.push_str(rest);
        out
    }

    let mut out = String::from(if is_gles(display) {
        "#version 300 es\nprecision highp float;\nprecision highp int;\n"
    } else {
        "#version 330\n"
    });
    // the shaders that declare their own output don't get another one, which would need a
    // location on OpenGL ES
    if fragment && source.contains("gl_FragColor") {
        out.push_str("out vec4 support_frag_color;\n");
    }
    for line in source.lines() {
        if line.trim_start().starts_with("#version") {
            continue;
        }
        let line = replace_word(line, "attribute", "in");
        let line = replace_word(&line, "varying", if fragment { "in" } else { "out" });
        let line = replace_word(&line, "gl_FragColor", "support_frag_color");
        let line = replace_word(&line, "texture2D", "texture");
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Builds a program from GLSL 1.10 shaders translated with `modern_glsl`.
pub fn program_from_legacy(
    display: &Backend,
    vertex: &str,
    fragment: &str,
) -> Result<safegl::Program, safegl::ProgramCreationError> {
    safegl::Program::from_source(
        display,
        &modern_glsl(display, vertex, false),
        &modern_glsl(display, fragment, true),
        None,
    )
}
