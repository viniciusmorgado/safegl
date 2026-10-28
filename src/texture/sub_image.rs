//! Reads of boxes of texture levels (`glGetTextureSubImage`, OpenGL 4.5).


use glow::HasContext;

use crate::buffer::BufferAny;
use crate::context::CommandContext;
use crate::gl;
use crate::handles::RawName;
use crate::image_format::{self, ClientFormatAny};
use crate::ops::ReadError;
use crate::texture::any::{fits, image_extent};
use crate::texture::regions::PACK;
use crate::texture::{PixelValue, TextureAnyMipmap};
use crate::version::{Api, Version};
use crate::{BufferExt, ContextExt, GlObject, TextureExt};

/// Returns `ReadError::NotSupported` without `glGetTextureSubImage`, and prepares the texture to
/// be read.
fn prepare(mipmap: &TextureAnyMipmap<'_>, ctxt: &mut CommandContext<'_>) -> Result<(), ReadError> {
    if !(ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_get_texture_sub_image) {
        return Err(ReadError::NotSupported);
    }
    if mipmap.get_samples().is_some() {
        return Err(ReadError::NotSupported);
    }
    // shader writes made through image units must be done before the image is read
    mipmap
        .get_texture()
        .prepare_for_access(ctxt, crate::TextureAccess::Update);
    unsafe {
        BufferAny::unbind_pixel_pack(ctxt);
        if ctxt.state.pixel_store_pack_alignment != 1 {
            ctxt.state.pixel_store_pack_alignment = 1;
            ctxt.glow.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
        }
    }
    Ok(())
}

impl<'a> TextureAnyMipmap<'a> {
    /// Reads the texels of a box of this mipmap level (`glGetTextureSubImage`), converted to
    /// `P`: the rows of each slice from the bottom, then the slices from the first.
    ///
    /// `offset` and `size` are in texels and address layers and faces like
    /// `TextureAnyMipmap::clear_region`. Unlike reads through framebuffers, this reads every
    /// kind of texture: depth, stencil, integer, compressed (decompressed), 3D textures and all
    /// the layers of array textures, but not multisample textures.
    ///
    /// Needs OpenGL 4.5 or `GL_ARB_get_texture_sub_image`, otherwise returns
    /// `ReadError::NotSupported`. Returns `ReadError::OutOfBounds` if the box isn't inside the
    /// level, and `ReadError::OutputFormatNotSupported` if the texels can't be converted to `P`.
    #[cfg_attr(docsrs, doc(cfg(safegl_gl45)))]
    pub fn read_box<P: PixelValue>(
        &self,
        offset: (u32, u32, u32),
        size: (u32, u32, u32),
    ) -> Result<Vec<P>, ReadError> {
        let texture = self.get_texture();
        if !fits(offset, size, image_extent(self)) {
            return Err(ReadError::OutOfBounds);
        }
        let (format, ty) = image_format::client_format_to_glenum(
            TextureExt::get_context(texture),
            ClientFormatAny::ClientFormat(P::get_format()),
            texture.get_requested_format(),
            false,
        )
        .map_err(|_| ReadError::OutputFormatNotSupported)?;

        let mut ctxt = TextureExt::get_context(texture).make_current();
        prepare(self, &mut ctxt)?;
        let count = size.0 as usize * size.1 as usize * size.2 as usize;
        let mut data: Vec<P> = Vec::with_capacity(count);
        unsafe {
            // every bit pattern is a valid `P`
            std::ptr::write_bytes(data.as_mut_ptr(), 0, count);
            data.set_len(count);
            ctxt.extra.GetTextureSubImage(
                texture.get_id().raw_name(),
                self.get_level() as i32,
                offset.0 as i32,
                offset.1 as i32,
                offset.2 as i32,
                size.0 as i32,
                size.1 as i32,
                size.2 as i32,
                format,
                ty,
                (count * std::mem::size_of::<P>()) as i32,
                data.as_mut_ptr() as *mut _,
            );
        }
        Ok(data)
    }

    /// Reads the texels of a box of this mipmap level into the box at `dest_offset` of a client
    /// image of `dest_size` texels (`dest`, its rows from the bottom, then its slices), converted
    /// to `P`, without reading into a separate buffer first (`GL_PACK_ROW_LENGTH`,
    /// `GL_PACK_IMAGE_HEIGHT` and the skips). The rest of `dest` is left as it is.
    ///
    /// Returns `ReadError::OutOfBounds` if a box isn't inside its image, and
    /// `ReadError::BufferTooSmall` if `dest` doesn't have the texels of `dest_size`. See
    /// `read_box` for the requirements and the other errors.
    #[cfg_attr(docsrs, doc(cfg(safegl_gl45)))]
    pub fn read_box_into<P: PixelValue>(
        &self,
        offset: (u32, u32, u32),
        size: (u32, u32, u32),
        dest: &mut [P],
        dest_size: (u32, u32, u32),
        dest_offset: (u32, u32, u32),
    ) -> Result<(), ReadError> {
        let texture = self.get_texture();
        if !fits(offset, size, image_extent(self)) || !fits(dest_offset, size, dest_size) {
            return Err(ReadError::OutOfBounds);
        }
        if dest.len() != dest_size.0 as usize * dest_size.1 as usize * dest_size.2 as usize {
            return Err(ReadError::BufferTooSmall);
        }
        let (format, ty) = image_format::client_format_to_glenum(
            TextureExt::get_context(texture),
            ClientFormatAny::ClientFormat(P::get_format()),
            texture.get_requested_format(),
            false,
        )
        .map_err(|_| ReadError::OutputFormatNotSupported)?;

        let mut ctxt = TextureExt::get_context(texture).make_current();
        prepare(self, &mut ctxt)?;
        unsafe {
            PACK.set(&mut ctxt, dest_size, dest_offset);
            ctxt.extra.GetTextureSubImage(
                texture.get_id().raw_name(),
                self.get_level() as i32,
                offset.0 as i32,
                offset.1 as i32,
                offset.2 as i32,
                size.0 as i32,
                size.1 as i32,
                size.2 as i32,
                format,
                ty,
                std::mem::size_of_val(dest) as i32,
                dest.as_mut_ptr() as *mut _,
            );
            PACK.reset(&mut ctxt);
        }
        Ok(())
    }

    /// Reads the compressed blocks of a box of this mipmap level of a compressed texture
    /// (`glGetCompressedTextureSubImage`), as they are stored.
    ///
    /// The box must start on the blocks of the format (see `CompressedFormat::block_size`) and
    /// end on them or on the edge of the level, or `ReadError::MisalignedBlocks` is
    /// returned. Returns `ReadError::OutputFormatNotSupported` if the texture isn't compressed in
    /// a format that SafeGL knows. See `read_box` for the requirements.
    #[cfg_attr(docsrs, doc(cfg(safegl_gl45)))]
    pub fn read_compressed_box(
        &self,
        offset: (u32, u32, u32),
        size: (u32, u32, u32),
    ) -> Result<Vec<u8>, ReadError> {
        let texture = self.get_texture();
        let extent = image_extent(self);
        if !fits(offset, size, extent) {
            return Err(ReadError::OutOfBounds);
        }
        let mut ctxt = TextureExt::get_context(texture).make_current();
        prepare(self, &mut ctxt)?;
        let format = unsafe {
            let bind_point = texture.bind_to_current(&mut ctxt);
            let face = if bind_point == gl::TEXTURE_CUBE_MAP {
                gl::TEXTURE_CUBE_MAP_POSITIVE_X
            } else {
                bind_point
            };
            ctxt.glow.get_tex_level_parameter_i32(
                face,
                self.get_level() as i32,
                gl::TEXTURE_INTERNAL_FORMAT,
            ) as gl::types::GLenum
        };
        let format = ClientFormatAny::from_internal_compressed_format(format)
            .ok_or(ReadError::OutputFormatNotSupported)?;
        let (block_width, block_height, _) =
            format.block_size().ok_or(ReadError::OutputFormatNotSupported)?;
        let aligned = |offset: u32, size: u32, extent: u32, block: u32| {
            offset % block == 0 && (size % block == 0 || offset + size == extent)
        };
        if !aligned(offset.0, size.0, extent.0, block_width)
            || !aligned(offset.1, size.1, extent.1, block_height)
        {
            return Err(ReadError::MisalignedBlocks);
        }
        let bytes = format
            .get_buffer_size(size.0, Some(size.1), None, Some(size.2))
            .ok_or(ReadError::OutputFormatNotSupported)?;
        let mut data = vec![0u8; bytes];
        unsafe {
            ctxt.extra.GetCompressedTextureSubImage(
                texture.get_id().raw_name(),
                self.get_level() as i32,
                offset.0 as i32,
                offset.1 as i32,
                offset.2 as i32,
                size.0 as i32,
                size.1 as i32,
                size.2 as i32,
                bytes as i32,
                data.as_mut_ptr() as *mut _,
            );
        }
        Ok(data)
    }
}
