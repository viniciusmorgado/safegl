//! Various functions to detect whether texture types are supported.

use crate::version::Api;
use crate::version::Version;
use crate::CapabilitiesSource;

/// Returns true is one-dimensional textures are supported.
#[inline]
pub fn is_texture_1d_supported<C: ?Sized>(context: &C) -> bool
where
    C: CapabilitiesSource,
{
    context.get_version().0 == Api::Gl
}

/// Returns true is two-dimensional textures are supported.
///
/// This is a dummy function that always returns true, as 2d textures are always supported. This
/// function is just here for completeness.
#[inline]
pub fn is_texture_2d_supported<C: ?Sized>(_: &C) -> bool
where
    C: CapabilitiesSource,
{
    true
}

/// Returns true is three-dimensional textures are supported.
///
/// Always returns true, as 3d textures are core in all supported OpenGL and OpenGL ES versions.
#[inline]
pub fn is_texture_3d_supported<C: ?Sized>(_: &C) -> bool
where
    C: CapabilitiesSource,
{
    true
}

/// Returns true is one-dimensional texture arrays are supported.
#[inline]
pub fn is_texture_1d_array_supported<C: ?Sized>(context: &C) -> bool
where
    C: CapabilitiesSource,
{
    context.get_version().0 == Api::Gl
}

/// Returns true is two-dimensional texture arrays are supported.
///
/// Always returns true, as 2d texture arrays are core in all supported OpenGL and OpenGL ES
/// versions.
#[inline]
pub fn is_texture_2d_array_supported<C: ?Sized>(_: &C) -> bool
where
    C: CapabilitiesSource,
{
    true
}

/// Returns true is two-dimensional multisample textures are supported.
#[inline]
pub fn is_texture_2d_multisample_supported<C: ?Sized>(context: &C) -> bool
where
    C: CapabilitiesSource,
{
    context.get_version().0 == Api::Gl || context.get_version() >= &Version(Api::GlEs, 3, 1)
    // TODO: no gles extension?
}

/// Returns true is two-dimensional multisample texture arrays are supported.
#[inline]
pub fn is_texture_2d_multisample_array_supported<C: ?Sized>(context: &C) -> bool
where
    C: CapabilitiesSource,
{
    context.get_version().0 == Api::Gl
        || context.get_version() >= &Version(Api::GlEs, 3, 2)
        || context
            .get_extensions()
            .gl_oes_texture_storage_multisample_2d_array
}

/// Returns true is cubemaps are supported.
///
/// Always returns true, as cubemaps are core in all supported OpenGL and OpenGL ES versions.
#[inline]
pub fn is_cubemaps_supported<C: ?Sized>(_: &C) -> bool
where
    C: CapabilitiesSource,
{
    true
}

/// Returns true is cubemap arrays are supported.
#[inline]
pub fn is_cubemap_arrays_supported<C: ?Sized>(context: &C) -> bool
where
    C: CapabilitiesSource,
{
    context.get_version().0 == Api::Gl
        || context.get_version() >= &Version(Api::GlEs, 3, 2)
        || context.get_extensions().gl_ext_texture_cube_map_array
        || context.get_extensions().gl_oes_texture_cube_map_array
}
