use std::cmp::Ordering;

/// Describes a version.
///
/// A version can only be compared to another version if they belong to the same API.
/// For example, both `Version(Gl, 3, 0) >= Version(GlEs, 3, 0)` and `Version(GlEs, 3, 0) >=
/// Version(Gl, 3, 0)` return `false`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Version(pub Api, pub u8, pub u8);

/// Describes an OpenGL-related API.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Api {
    /// Regular OpenGL.
    Gl,
    /// OpenGL embedded system.
    GlEs,
}

impl PartialOrd for Version {
    #[inline]
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        if self.0 != other.0 {
            return None;
        }

        match self.1.cmp(&other.1) {
            Ordering::Equal => Some(self.2.cmp(&other.2)),
            a => Some(a),
        }
    }
}

/// Returns true if the crate is compiled for WebGL (`wasm32` without Emscripten).
///
/// WebGL 2 is reported as OpenGL ES 3.0, but it lacks or rejects several things that native
/// OpenGL ES 3.0 has (buffer mapping, blocking fence waits, `PRIMITIVE_RESTART_FIXED_INDEX`,
/// ...), and the table of extra entry points (`gl::Extra`) is never loaded.
#[inline]
pub(crate) const fn is_webgl() -> bool {
    cfg!(all(target_family = "wasm", not(target_os = "emscripten")))
}

/// Obtains the OpenGL version of a `glow` context.
///
/// WebGL 2 is reported by glow as OpenGL ES 3.0.
pub fn get_gl_version(gl: &glow::Context) -> Version {
    use glow::HasContext;

    let version = gl.version();
    let api = if version.is_embedded {
        Api::GlEs
    } else {
        Api::Gl
    };
    let clamp = |value: u32| u8::try_from(value).unwrap_or(u8::MAX);

    Version(api, clamp(version.major), clamp(version.minor))
}

/// Given an API version, this function returns the GLSL version that the implementation is
/// required to support, which has the same number from OpenGL 3.3 and OpenGL ES 3.0.
///
/// # Panic
///
/// Panics if the version is below OpenGL 4.0 or OpenGL ES 3.0, which SafeGL doesn't support.
pub fn get_supported_glsl_version(gl_version: &Version) -> Version {
    let minimum = match gl_version.0 {
        Api::Gl => Version(Api::Gl, 4, 0),
        Api::GlEs => Version(Api::GlEs, 3, 0),
    };
    assert!(
        *gl_version >= minimum,
        "SafeGL doesn't support {:?} {}.{}",
        gl_version.0,
        gl_version.1,
        gl_version.2
    );
    *gl_version
}

#[cfg(test)]
mod tests {
    use super::{get_supported_glsl_version, Api, Version};

    #[test]
    fn glsl_versions_match_the_api_versions() {
        for version in [
            Version(Api::Gl, 4, 0),
            Version(Api::Gl, 4, 6),
            Version(Api::GlEs, 3, 0),
            Version(Api::GlEs, 3, 2),
        ] {
            assert_eq!(get_supported_glsl_version(&version), version);
        }
    }

    #[test]
    #[should_panic]
    fn unsupported_gl_version() {
        get_supported_glsl_version(&Version(Api::Gl, 3, 3));
    }

    #[test]
    #[should_panic]
    fn unsupported_gles_version() {
        get_supported_glsl_version(&Version(Api::GlEs, 2, 0));
    }
}
