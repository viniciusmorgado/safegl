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

/// Returns true is one-dimensional texture arrays are supported.
#[inline]
pub fn is_texture_1d_array_supported<C: ?Sized>(context: &C) -> bool
where
    C: CapabilitiesSource,
{
    context.get_version().0 == Api::Gl
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
