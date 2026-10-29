//! The state of single draw buffers: `glEnablei`, `glBlend*i` and `glColorMaski`, core in OpenGL
//! 4.0 and OpenGL ES 3.2, given by `GL_EXT_draw_buffers_indexed` and `GL_OES_draw_buffers_indexed`
//! on OpenGL ES 3.0 and 3.1, and by `OES_draw_buffers_indexed` on WebGL.

use crate::context::CommandContext;
use crate::gl;
use crate::version::{Api, Version};

#[cfg(not(safegl_browser))]
use glow::HasContext;

/// Returns true if the backend can set the state of single draw buffers.
pub(crate) fn is_supported(ctxt: &CommandContext<'_>) -> bool {
    ctxt.version.0 == Api::Gl
        || (ctxt.version >= &Version(Api::GlEs, 3, 2) && !crate::version::is_webgl())
        || ctxt.extensions.gl_ext_draw_buffers_indexed
        || ctxt.extensions.gl_oes_draw_buffers_indexed
}

/// The functions that set the state of single draw buffers.
#[cfg(not(safegl_browser))]
enum Entry {
    Core,
    Ext,
    Oes,
}

#[cfg(not(safegl_browser))]
fn entry(ctxt: &CommandContext<'_>) -> Entry {
    if ctxt.version.0 == Api::Gl || ctxt.version >= &Version(Api::GlEs, 3, 2) {
        Entry::Core
    } else if ctxt.extensions.gl_ext_draw_buffers_indexed {
        Entry::Ext
    } else {
        Entry::Oes
    }
}

/// Enables or disables `capability` for the draw buffer `index`.
pub(crate) unsafe fn set_enabled(
    ctxt: &CommandContext<'_>,
    capability: gl::types::GLenum,
    index: u32,
    enabled: bool,
) {
    #[cfg(safegl_browser)]
    ctxt.webgl.enable_indexed(capability, index, enabled);
    #[cfg(not(safegl_browser))]
    match (entry(ctxt), enabled) {
        (Entry::Core, true) => ctxt.glow.enable_draw_buffer(capability, index),
        (Entry::Core, false) => ctxt.glow.disable_draw_buffer(capability, index),
        (Entry::Ext, true) => ctxt.extra.EnableiEXT(capability, index),
        (Entry::Ext, false) => ctxt.extra.DisableiEXT(capability, index),
        (Entry::Oes, true) => ctxt.extra.EnableiOES(capability, index),
        (Entry::Oes, false) => ctxt.extra.DisableiOES(capability, index),
    }
}

/// Sets the blend equations of color and alpha of the draw buffer `index`. An advanced equation
/// is the same for both, and set by itself.
pub(crate) unsafe fn blend_equation(
    ctxt: &CommandContext<'_>,
    index: u32,
    (color, alpha): (gl::types::GLenum, gl::types::GLenum),
) {
    #[cfg(safegl_browser)]
    ctxt.webgl.blend_equation_indexed(index, color, alpha);
    #[cfg(not(safegl_browser))]
    if super::blend::is_advanced(color) {
        match entry(ctxt) {
            Entry::Core => ctxt.glow.blend_equation_draw_buffer(index, color),
            Entry::Ext => ctxt.extra.BlendEquationiEXT(index, color),
            Entry::Oes => ctxt.extra.BlendEquationiOES(index, color),
        }
    } else {
        match entry(ctxt) {
            Entry::Core => ctxt
                .glow
                .blend_equation_separate_draw_buffer(index, color, alpha),
            Entry::Ext => ctxt.extra.BlendEquationSeparateiEXT(index, color, alpha),
            Entry::Oes => ctxt.extra.BlendEquationSeparateiOES(index, color, alpha),
        }
    }
}

/// Sets the blend factors of the draw buffer `index`: source and destination color, then source
/// and destination alpha.
pub(crate) unsafe fn blend_func(
    ctxt: &CommandContext<'_>,
    index: u32,
    func: (gl::types::GLenum, gl::types::GLenum, gl::types::GLenum, gl::types::GLenum),
) {
    #[cfg(safegl_browser)]
    ctxt.webgl.blend_func_indexed(index, func);
    #[cfg(not(safegl_browser))]
    match entry(ctxt) {
        Entry::Core => ctxt
            .glow
            .blend_func_separate_draw_buffer(index, func.0, func.1, func.2, func.3),
        Entry::Ext => ctxt
            .extra
            .BlendFuncSeparateiEXT(index, func.0, func.1, func.2, func.3),
        Entry::Oes => ctxt
            .extra
            .BlendFuncSeparateiOES(index, func.0, func.1, func.2, func.3),
    }
}

/// Sets the color mask of the draw buffer `index`.
pub(crate) unsafe fn color_mask(
    ctxt: &CommandContext<'_>,
    index: u32,
    mask: (bool, bool, bool, bool),
) {
    #[cfg(safegl_browser)]
    ctxt.webgl.color_mask_indexed(index, mask);
    #[cfg(not(safegl_browser))]
    {
        let (r, g, b, a) = (mask.0 as u8, mask.1 as u8, mask.2 as u8, mask.3 as u8);
        match entry(ctxt) {
            Entry::Core => ctxt
                .glow
                .color_mask_draw_buffer(index, mask.0, mask.1, mask.2, mask.3),
            Entry::Ext => ctxt.extra.ColorMaskiEXT(index, r, g, b, a),
            Entry::Oes => ctxt.extra.ColorMaskiOES(index, r, g, b, a),
        }
    }
}
