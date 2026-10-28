//! Reads of whole texture levels (`glGetTexImage`), which every version of desktop OpenGL has.

use std::borrow::Cow;

use glow::HasContext;

use crate::buffer::BufferAny;
use crate::context::CommandContext;
use crate::gl;
use crate::image_format::{self, ClientFormatAny};
use crate::ops::ReadError;
use crate::texture::any::{image_extent, image_targets};
use crate::texture::pixel_buffer::{store_infos, PixelBuffer};
use crate::texture::{PixelValue, Texture3dDataSink, TextureAnyMipmap};
use crate::version::{Api, Version};
use crate::handles::RawName;
use crate::{BufferExt, ContextExt, GlObject, TextureExt};

impl<'a> TextureAnyMipmap<'a> {
    /// Reads the whole level into a `Texture3dDataSink`, such as `Vec<Vec<Vec<P>>>`, converted to
    /// `P`: the slices of a 3D texture, the layers of an array texture or the faces of a cube
    /// map, each made of its rows from the bottom (`glGetTexImage`).
    ///
    /// Unlike reads through framebuffers, this reads every kind of texture: depth, stencil,
    /// depth-stencil, integer, compressed (decompressed) and formats that can't be rendered to,
    /// but not multisample textures, which return `ReadError::NotSupported`. Returns
    /// `ReadError::OutputFormatNotSupported` if the texels can't be converted to `P`, and
    /// `ReadError::ContextLost` if the context has been lost.
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub fn read_volume<P, T>(&self) -> Result<T, ReadError>
    where
        P: PixelValue,
        T: Texture3dDataSink<P>,
    {
        let (width, height, depth) = image_extent(self);
        let data = self.read_level::<P>()?;
        Ok(T::from_raw(Cow::Owned(data), width, height, depth))
    }

    /// Reads the texels of the whole level, in the order of `read_volume`.
    pub(crate) fn read_level<P: PixelValue>(&self) -> Result<Vec<P>, ReadError> {
        let (width, height, depth) = image_extent(self);
        let count = width as usize * height as usize * depth as usize;
        let mut data: Vec<P> = Vec::with_capacity(count);
        unsafe {
            // every bit pattern is a valid `P`
            std::ptr::write_bytes(data.as_mut_ptr(), 0, count);
            data.set_len(count);
        }
        let pointer = data.as_mut_ptr() as usize;
        self.get_level_images::<P>(None, pointer as *mut _, |ctxt, target, level, (format, ty), offset, size| unsafe {
            let pixels = (pointer + offset) as *mut _;
            // the robust entry point never writes past the end of the buffer
            if ctxt.extensions.gl_arb_robustness {
                ctxt.extra.GetnTexImageARB(target, level, format, ty, size, pixels);
            } else {
                let image = std::slice::from_raw_parts_mut(pixels as *mut u8, size as usize);
                ctxt.glow
                    .get_tex_image(target, level, format, ty, glow::PixelPackData::Slice(Some(image)));
            }
        })?;
        Ok(data)
    }

    /// Reads the texels of the whole level into `buffer`, in the order of `read_volume`.
    pub(crate) fn read_level_to_pixel_buffer<P: PixelValue>(
        &self,
        buffer: &PixelBuffer<P>,
    ) -> Result<(), ReadError> {
        let (width, height, depth) = image_extent(self);
        if buffer.len() < width as usize * height as usize * depth as usize {
            return Err(ReadError::BufferTooSmall);
        }
        if !std::rc::Rc::ptr_eq(buffer.get_context(), TextureExt::get_context(self.get_texture()))
        {
            return Err(ReadError::ContextMismatch);
        }
        self.get_level_images::<P>(Some(buffer), std::ptr::null_mut(), |ctxt, target, level, (format, ty), offset, _| unsafe {
            ctxt.glow.get_tex_image(
                target,
                level,
                format,
                ty,
                glow::PixelPackData::BufferOffset(offset as u32),
            );
        })?;
        store_infos(buffer, (width, height * depth));
        Ok(())
    }

    /// Prepares the reads of the images of the level and calls `read` for each one, with its
    /// target, the level, the format and type of `P`, and the offset and size in bytes of the
    /// image in the output: the faces of a cube map are read one by one, the layers of an
    /// array texture and the slices of a 3D texture at once. The output is `buffer` if there's
    /// one, client memory otherwise.
    ///
    /// With OpenGL 4.5 or `GL_ARB_get_texture_sub_image`, the whole level is read at once with
    /// `glGetTextureSubImage` instead, which bounds its writes by the size of the output, and
    /// `read` isn't called. (Mesa's `glGetnTexImage` writes nothing.)
    fn get_level_images<P: PixelValue>(
        &self,
        buffer: Option<&PixelBuffer<P>>,
        output: *mut std::ffi::c_void,
        mut read: impl FnMut(
            &mut CommandContext<'_>,
            gl::types::GLenum,
            i32,
            (gl::types::GLenum, gl::types::GLenum),
            usize,
            i32,
        ),
    ) -> Result<(), ReadError> {
        let texture = self.get_texture();
        if self.get_samples().is_some() {
            return Err(ReadError::NotSupported);
        }
        let format = image_format::client_format_to_glenum(
            TextureExt::get_context(texture),
            ClientFormatAny::ClientFormat(P::get_format()),
            texture.get_requested_format(),
            false,
        )
        .map_err(|_| ReadError::OutputFormatNotSupported)?;

        let mut ctxt = TextureExt::get_context(texture).make_current();
        if ctxt.state.lost_context {
            return Err(ReadError::ContextLost);
        }
        // shader writes made through image units must be done before the image is read
        texture.prepare_for_access(&mut ctxt, crate::TextureAccess::Update);

        let (width, height, depth) = image_extent(self);
        let bytes = width as usize * height as usize * depth as usize * std::mem::size_of::<P>();
        unsafe {
            match buffer {
                Some(buffer) => buffer.prepare_and_bind_for_pixel_pack(&mut ctxt),
                None => BufferAny::unbind_pixel_pack(&mut ctxt),
            }
            if ctxt.state.pixel_store_pack_alignment != 1 {
                ctxt.state.pixel_store_pack_alignment = 1;
                ctxt.glow.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            }

            if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_get_texture_sub_image
            {
                let (format, ty) = format;
                let pixels = match buffer {
                    Some(_) => std::ptr::null_mut(),
                    None => output,
                };
                ctxt.extra.GetTextureSubImage(
                    texture.get_id().raw_name(),
                    self.get_level() as i32,
                    0,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    depth as i32,
                    format,
                    ty,
                    bytes as i32,
                    pixels,
                );
                return Ok(());
            }

            let bind_point = texture.bind_to_current(&mut ctxt);
            let targets = image_targets(bind_point);
            let image = bytes / targets.len();
            let level = self.get_level() as i32;
            for (i, target) in targets.into_iter().enumerate() {
                read(&mut ctxt, target, level, format, i * image, image as i32);
            }
        }
        Ok(())
    }
}
