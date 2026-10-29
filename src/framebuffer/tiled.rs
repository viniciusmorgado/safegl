//! The render passes of tile-based GPUs, which most embedded GPUs are: pixel local storage
//! (`GL_EXT_shader_pixel_local_storage`) and tiled rendering (`GL_QCOM_tiled_rendering`).
//!
//! A pass borrows its surface, and draws go through the pass, which dereferences to the surface.
//! It stays open in OpenGL while the surface is the framebuffer bound for drawing: when SafeGL
//! binds another framebuffer, for a draw to another surface, a read or a blit, the pass is
//! suspended, and it starts again when the surface is drawn to.

use std::error::Error;
use std::fmt;
use std::ops::{Deref, DerefMut};

use glow::HasContext;

use crate::context::{CommandContext, Context};
use crate::fbo::{self, FramebufferPass, FramebufferPassKind, FramebufferTarget};
use crate::gl;
use crate::version::is_webgl;
use crate::{CapabilitiesSource, ContextExt, FboAttachments, Rect, Surface};

/// The surfaces that the render passes of tile-based GPUs are opened on: the framebuffers, the
/// frames and the default framebuffer.
///
/// A pass on a `MultiOutputFrameBuffer` stays open for the draws whose program writes the
/// outputs at locations in their order (the first output at location 0, and so on): the others
/// draw to another framebuffer object, and suspend the pass.
pub trait PassSurface: Surface + sealed::PassSurface {}

pub(crate) mod sealed {
    /// The context of a surface and the framebuffer that it draws to.
    pub trait PassSurface {
        fn pass_target(&self) -> (&crate::context::Context, crate::fbo::FramebufferTarget<'_>);
    }
}

macro_rules! pass_surface {
    ($($surface:ty),*) => {$(
        impl sealed::PassSurface for $surface {
            fn pass_target(&self) -> (&Context, FramebufferTarget<'_>) {
                (self.get_fbo_context(), self.get_attachments())
            }
        }

        impl PassSurface for $surface {}
    )*};
}

pass_surface!(
    crate::Frame,
    crate::framebuffer::DefaultFramebuffer,
    crate::framebuffer::SimpleFrameBuffer<'_>,
    crate::framebuffer::MultiOutputFrameBuffer<'_>
);

/// Error while opening a render pass of a tile-based GPU, or clearing its pixel local storage.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TilePassError {
    /// The backend doesn't have the extension of the pass: `GL_EXT_shader_pixel_local_storage`,
    /// `GL_EXT_shader_pixel_local_storage2` to clear the storage, or `GL_QCOM_tiled_rendering`.
    /// WebGL has none of them.
    NotSupported,
    /// The framebuffer is multisampled, where pixel local storage can't be enabled.
    Multisampled,
    /// The framebuffer has color attachments other than the first one, where pixel local storage
    /// needs `GL_EXT_shader_pixel_local_storage2`.
    TooManyColorAttachments,
    /// The size of the pixel local storage isn't a multiple of 4, is above
    /// `GL_MAX_SHADER_PIXEL_LOCAL_STORAGE_SIZE_EXT`, or doesn't fit next to the color attachments
    /// of the framebuffer (`GL_MAX_SHADER_COMBINED_LOCAL_STORAGE_SIZE_EXT`).
    SizeNotSupported,
    /// The values to clear go past the end of the pixel local storage.
    OutOfBounds,
    /// The context is lost: no pass can be opened in it.
    ContextLost,
}

impl fmt::Display for TilePassError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::TilePassError::*;
        let desc = match *self {
            NotSupported => "The backend doesn't support this render pass of tile-based GPUs",
            Multisampled => "Pixel local storage can't be enabled on a multisampled framebuffer",
            TooManyColorAttachments => {
                "Pixel local storage can't be enabled with several color attachments"
            }
            SizeNotSupported => "The size of the pixel local storage isn't supported",
            OutOfBounds => "The values go past the end of the pixel local storage",
            ContextLost => "The context is lost",
        };
        fmt.write_str(desc)
    }
}

impl Error for TilePassError {}

/// Returns true if the backend has pixel local storage (`GL_EXT_shader_pixel_local_storage`).
pub fn is_pixel_local_storage_supported<C: ?Sized>(context: &C) -> bool
where
    C: CapabilitiesSource,
{
    !is_webgl() && context.get_extensions().gl_ext_shader_pixel_local_storage
}

/// Returns true if the backend has tiled rendering (`GL_QCOM_tiled_rendering`).
pub fn is_tiled_rendering_supported<C: ?Sized>(context: &C) -> bool
where
    C: CapabilitiesSource,
{
    !is_webgl() && context.get_extensions().gl_qcom_tiled_rendering
}

/// Binds the framebuffer of `target` for drawing, after closing the render pass of a tile-based
/// GPU that is open on it, and returns the framebuffer.
unsafe fn bind_for_pass(
    ctxt: &mut CommandContext<'_>,
    target: FramebufferTarget<'_>,
) -> Option<glow::Framebuffer> {
    let framebuffer = fbo::FramebuffersContainer::get_framebuffer_for_drawing(ctxt, target);
    fbo::bind_framebuffer(ctxt, framebuffer, true, false);
    // a pass is open on this framebuffer, through another surface of the same attachments
    fbo::suspend_framebuffer_pass(ctxt);
    ctxt.state.framebuffer_pass = None;
    framebuffer
}

/// Opens a render pass of a tile-based GPU, active in OpenGL, and returns its identifier.
fn open_pass(
    ctxt: &mut CommandContext<'_>,
    framebuffer: Option<glow::Framebuffer>,
    kind: FramebufferPassKind,
) -> u64 {
    let id = ctxt.state.next_framebuffer_pass;
    ctxt.state.next_framebuffer_pass += 1;
    ctxt.state.framebuffer_pass = Some(FramebufferPass {
        id,
        framebuffer,
        kind,
        active: true,
    });
    id
}

/// Closes the render pass `id` if it is still open, after `end` if it is active in OpenGL.
fn close_pass(context: &Context, id: u64, end: impl FnOnce(&mut CommandContext<'_>)) {
    let mut ctxt = context.make_current();
    let Some(pass) = ctxt.state.framebuffer_pass.filter(|pass| pass.id == id) else {
        return;
    };
    if pass.active && !ctxt.objects_lost() {
        end(&mut ctxt);
    }
    ctxt.state.framebuffer_pass = None;
}

/// Pixel local storage on a surface (`GL_EXT_shader_pixel_local_storage`): the fragment shaders
/// keep values for each pixel from a draw to the next, in the memory of the tile on the chip,
/// with the `__pixel_localEXT`, `__pixel_local_inEXT` and `__pixel_local_outEXT` blocks of GLSL.
///
/// Draws go through the storage, which dereferences to the surface, and a program that declares
/// pixel local storage draws only there: elsewhere, the draw returns
/// `DrawError::PixelLocalStorageNotEnabled`.
///
/// Values start undefined, unless they are cleared (`clear`, with
/// `GL_EXT_shader_pixel_local_storage2`): they are lost when the storage is dropped, and when
/// it is suspended by the binding of another framebuffer for drawing. Without
/// `GL_EXT_shader_pixel_local_storage2`, a fragment that writes a color output loses its pixel
/// local storage, and writing pixel local storage leaves the color of the pixel undefined.
pub struct PixelLocalStorage<'s, S: PassSurface + ?Sized> {
    surface: &'s mut S,
    id: u64,
    // the size of the storage, in 32-bit values
    words: u32,
}

impl<'s, S: PassSurface + ?Sized> PixelLocalStorage<'s, S> {
    /// Enables pixel local storage of `size` bytes, a multiple of 4, on `surface`. The fragment
    /// shaders that draw there declare at most as much.
    ///
    /// With `GL_EXT_shader_pixel_local_storage2`, the framebuffer gets this size
    /// (`glFramebufferPixelLocalStorageSizeEXT`), and can have several color attachments.
    pub fn new(surface: &'s mut S, size: u32) -> Result<Self, TilePassError> {
        let (context, target) = surface.pass_target();
        let mut ctxt = context.make_current();
        if is_webgl() || !ctxt.extensions.gl_ext_shader_pixel_local_storage {
            return Err(TilePassError::NotSupported);
        }
        if ctxt.objects_lost() {
            return Err(TilePassError::ContextLost);
        }
        let second = ctxt.extensions.gl_ext_shader_pixel_local_storage2;
        let max = unsafe {
            ctxt.glow
                .get_parameter_i32(gl::MAX_SHADER_PIXEL_LOCAL_STORAGE_SIZE_EXT)
        };
        if size % 4 != 0 || size > max.max(0) as u32 {
            return Err(TilePassError::SizeNotSupported);
        }
        if let FramebufferTarget::Attachments(attachments) = target {
            if !second && attachments.color_locations().any(|location| location != 0) {
                return Err(TilePassError::TooManyColorAttachments);
            }
        }

        unsafe {
            let framebuffer = bind_for_pass(&mut ctxt, target);
            if ctxt.glow.get_parameter_i32(gl::SAMPLE_BUFFERS) != 0 {
                return Err(TilePassError::Multisampled);
            }
            #[cfg(not(safegl_browser))]
            if second {
                ctxt.extra
                    .FramebufferPixelLocalStorageSizeEXT(gl::DRAW_FRAMEBUFFER, size as i32);
                // the storage and the color attachments share the memory of the tile
                let status = ctxt.glow.check_framebuffer_status(gl::DRAW_FRAMEBUFFER);
                if status != gl::FRAMEBUFFER_COMPLETE {
                    ctxt.extra
                        .FramebufferPixelLocalStorageSizeEXT(gl::DRAW_FRAMEBUFFER, 0);
                    return Err(TilePassError::SizeNotSupported);
                }
            }
            ctxt.glow.enable(gl::SHADER_PIXEL_LOCAL_STORAGE_EXT);
            let id = open_pass(&mut ctxt, framebuffer, FramebufferPassKind::PixelLocalStorage);
            drop(ctxt);
            Ok(PixelLocalStorage {
                surface,
                id,
                words: size / 4,
            })
        }
    }

    /// Sets the 32-bit values of the pixel local storage of every pixel from `offset`, counted
    /// in 32-bit values, to `values` (`glClearPixelLocalStorageuiEXT`). Needs
    /// `GL_EXT_shader_pixel_local_storage2`.
    pub fn clear(&mut self, offset: u32, values: &[u32]) -> Result<(), TilePassError> {
        let (context, target) = self.surface.pass_target();
        let mut ctxt = context.make_current();
        if !ctxt.extensions.gl_ext_shader_pixel_local_storage2 {
            return Err(TilePassError::NotSupported);
        }
        if offset as u64 + values.len() as u64 > self.words as u64 {
            return Err(TilePassError::OutOfBounds);
        }
        if ctxt.objects_lost() {
            return Err(TilePassError::ContextLost);
        }
        unsafe {
            // binding the framebuffer enables the storage again if it was suspended
            let framebuffer =
                fbo::FramebuffersContainer::get_framebuffer_for_drawing(&mut ctxt, target);
            fbo::bind_framebuffer(&mut ctxt, framebuffer, true, false);
            #[cfg(not(safegl_browser))]
            ctxt.extra.ClearPixelLocalStorageuiEXT(
                offset as i32,
                values.len() as i32,
                values.as_ptr(),
            );
        }
        Ok(())
    }
}

impl<S: PassSurface + ?Sized> Deref for PixelLocalStorage<'_, S> {
    type Target = S;

    fn deref(&self) -> &S {
        self.surface
    }
}

impl<S: PassSurface + ?Sized> DerefMut for PixelLocalStorage<'_, S> {
    fn deref_mut(&mut self) -> &mut S {
        self.surface
    }
}

impl<S: PassSurface + ?Sized> Drop for PixelLocalStorage<'_, S> {
    fn drop(&mut self) {
        close_pass(self.surface.pass_target().0, self.id, |ctxt| unsafe {
            ctxt.glow.disable(gl::SHADER_PIXEL_LOCAL_STORAGE_EXT)
        });
    }
}

/// The buffers that tiled rendering preserves (`GL_QCOM_tiled_rendering`): when it starts, the
/// tile gets their content from memory, and when it ends, their content in the tile is written
/// to memory. The content of the others is undefined in the tile.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct TilePreserve {
    /// The color attachments, a bit for each, from the bit 0 for the first one to the bit 7.
    pub colors: u8,
    /// The depth buffer.
    pub depth: bool,
    /// The stencil buffer.
    pub stencil: bool,
    /// The samples of multisampled buffers.
    pub multisample: bool,
}

impl TilePreserve {
    /// Every buffer.
    pub const ALL: TilePreserve = TilePreserve {
        colors: 0xFF,
        depth: true,
        stencil: true,
        multisample: true,
    };

    /// No buffer.
    pub const NONE: TilePreserve = TilePreserve {
        colors: 0,
        depth: false,
        stencil: false,
        multisample: false,
    };

    fn mask(self) -> gl::types::GLbitfield {
        let mut mask = self.colors as gl::types::GLbitfield;
        if self.depth {
            mask |= 0xFF00;
        }
        if self.stencil {
            mask |= 0xFF_0000;
        }
        if self.multisample {
            mask |= 0xFF00_0000;
        }
        mask
    }
}

/// Tiled rendering on a surface (`GL_QCOM_tiled_rendering`): the draws and the clears change
/// only the pixels of a rectangle, which the GPU keeps in the memory of the chip from the start
/// of the rendering to its end, without writing the rest of the surface.
///
/// Draws go through the rendering, which dereferences to the surface. The viewport and the
/// scissor box stay relative to the surface. The rendering ends when it is dropped, preserving
/// every buffer, or with `finish`, which chooses them. When another framebuffer is bound for
/// drawing, it ends preserving every buffer, and starts again preserving them when the surface
/// is drawn to.
pub struct TiledRendering<'s, S: PassSurface + ?Sized> {
    surface: &'s mut S,
    id: u64,
    // the buffers to preserve at the end
    end: gl::types::GLbitfield,
}

impl<'s, S: PassSurface + ?Sized> TiledRendering<'s, S> {
    /// Starts tiled rendering of the rectangle `tile` of `surface` (`glStartTilingQCOM`),
    /// preserving the buffers of `preserve`. The part of the rectangle outside the surface is
    /// ignored.
    pub fn new(
        surface: &'s mut S,
        tile: &Rect,
        preserve: TilePreserve,
    ) -> Result<Self, TilePassError> {
        let (context, target) = surface.pass_target();
        let mut ctxt = context.make_current();
        if is_webgl() || !ctxt.extensions.gl_qcom_tiled_rendering {
            return Err(TilePassError::NotSupported);
        }
        if ctxt.objects_lost() {
            return Err(TilePassError::ContextLost);
        }
        let rect = (tile.left, tile.bottom, tile.width, tile.height);
        unsafe {
            let framebuffer = bind_for_pass(&mut ctxt, target);
            #[cfg(not(safegl_browser))]
            ctxt.extra
                .StartTilingQCOM(rect.0, rect.1, rect.2, rect.3, preserve.mask());
            let id = open_pass(&mut ctxt, framebuffer, FramebufferPassKind::Tiling(rect));
            drop(ctxt);
            Ok(TiledRendering {
                surface,
                id,
                end: fbo::PRESERVE_ALL_TILE_BUFFERS,
            })
        }
    }

    /// Ends the tiled rendering (`glEndTilingQCOM`), preserving the buffers of `preserve`.
    pub fn finish(mut self, preserve: TilePreserve) {
        self.end = preserve.mask();
    }
}

impl<S: PassSurface + ?Sized> Deref for TiledRendering<'_, S> {
    type Target = S;

    fn deref(&self) -> &S {
        self.surface
    }
}

impl<S: PassSurface + ?Sized> DerefMut for TiledRendering<'_, S> {
    fn deref_mut(&mut self) -> &mut S {
        self.surface
    }
}

impl<S: PassSurface + ?Sized> Drop for TiledRendering<'_, S> {
    fn drop(&mut self) {
        let end = self.end;
        close_pass(self.surface.pass_target().0, self.id, |_ctxt| {
            #[cfg(not(safegl_browser))]
            unsafe {
                _ctxt.extra.EndTilingQCOM(end)
            }
            #[cfg(safegl_browser)]
            let _ = end;
        });
    }
}
