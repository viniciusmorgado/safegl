//! Transfers between a box of a texture level and a box of a larger client image, through the
//! pixel store parameters: the row length, image height and skips of the client image
//! (`GL_UNPACK_ROW_LENGTH`, `GL_PACK_SKIP_ROWS`, ...).

use glow::HasContext;

use crate::buffer::BufferAny;
use crate::context::CommandContext;
use crate::gl;
use crate::image_format::{self, ClientFormatAny, TextureFormat, TextureFormatRequest};
use crate::texture::any::{fits, image_extent, image_targets};
use crate::texture::{PixelValue, TextureAnyMipmap, UploadError};
#[cfg(safegl_gl42)]
use crate::version::{Api, Version};
use crate::{BufferExt, ContextExt, TextureExt};

/// The pixel store parameters of client images in memory (`GL_UNPACK_*` for writes,
/// `GL_PACK_*` for reads), all of which SafeGL keeps at their default between its calls.
pub(crate) struct ClientLayout {
    pub row_length: gl::types::GLenum,
    pub image_height: gl::types::GLenum,
    pub skip_pixels: gl::types::GLenum,
    pub skip_rows: gl::types::GLenum,
    pub skip_images: gl::types::GLenum,
}

pub(crate) const UNPACK: ClientLayout = ClientLayout {
    row_length: gl::UNPACK_ROW_LENGTH,
    image_height: gl::UNPACK_IMAGE_HEIGHT,
    skip_pixels: gl::UNPACK_SKIP_PIXELS,
    skip_rows: gl::UNPACK_SKIP_ROWS,
    skip_images: gl::UNPACK_SKIP_IMAGES,
};

pub(crate) const PACK: ClientLayout = ClientLayout {
    row_length: gl::PACK_ROW_LENGTH,
    image_height: gl::PACK_IMAGE_HEIGHT,
    skip_pixels: gl::PACK_SKIP_PIXELS,
    skip_rows: gl::PACK_SKIP_ROWS,
    skip_images: gl::PACK_SKIP_IMAGES,
};

impl ClientLayout {
    /// Sets the parameters for a box at `offset` of a client image of `size` texels.
    pub(crate) unsafe fn set(
        &self,
        ctxt: &mut CommandContext<'_>,
        size: (u32, u32, u32),
        offset: (u32, u32, u32),
    ) {
        ctxt.glow.pixel_store_i32(self.row_length, size.0 as i32);
        ctxt.glow.pixel_store_i32(self.image_height, size.1 as i32);
        ctxt.glow.pixel_store_i32(self.skip_pixels, offset.0 as i32);
        ctxt.glow.pixel_store_i32(self.skip_rows, offset.1 as i32);
        ctxt.glow.pixel_store_i32(self.skip_images, offset.2 as i32);
    }

    /// Puts the row length and the skipped pixels and rows back to their default, which are the
    /// parameters of reads on OpenGL ES and WebGL.
    pub(crate) unsafe fn reset_rows(&self, ctxt: &mut CommandContext<'_>) {
        for pname in [self.row_length, self.skip_pixels, self.skip_rows] {
            ctxt.glow.pixel_store_i32(pname, 0);
        }
    }

    /// Puts the parameters back to their default, tightly packed images.
    pub(crate) unsafe fn reset(&self, ctxt: &mut CommandContext<'_>) {
        for pname in [
            self.row_length,
            self.image_height,
            self.skip_pixels,
            self.skip_rows,
            self.skip_images,
        ] {
            ctxt.glow.pixel_store_i32(pname, 0);
        }
    }
}

/// Returns true if the texture was created with a compressed format.
fn is_compressed(format: TextureFormatRequest) -> bool {
    matches!(
        format,
        TextureFormatRequest::AnyCompressed
            | TextureFormatRequest::AnyCompressedSrgb
            | TextureFormatRequest::Specific(TextureFormat::CompressedFormat(_))
            | TextureFormatRequest::Specific(TextureFormat::CompressedSrgbFormat(_))
    )
}

#[cfg(safegl_gl42)]
/// Returns true if the box of `size` at `offset` starts on the 4 × 4 blocks of compressed
/// formats and ends on them or on the edge of `extent`.
fn is_block_aligned(offset: (u32, u32, u32), size: (u32, u32, u32), extent: (u32, u32, u32)) -> bool {
    let aligned = |offset: u32, size: u32, extent: u32| {
        offset % 4 == 0 && (size % 4 == 0 || offset + size == extent)
    };
    aligned(offset.0, size.0, extent.0) && aligned(offset.1, size.1, extent.1)
}

impl<'a> TextureAnyMipmap<'a> {
    /// Writes the box of `size` texels at `source_offset` of a client image of `source_size`
    /// texels (`source`, its rows from the bottom, then its slices) to the box at `offset` of
    /// this level, without copying the box out of the image first (`GL_UNPACK_ROW_LENGTH`,
    /// `GL_UNPACK_IMAGE_HEIGHT` and the skips).
    ///
    /// `offset` and `size` address layers and faces like `TextureAnyMipmap::clear_region`.
    /// Returns `UploadError::OutOfBounds` if a box isn't inside its image,
    /// `UploadError::DataSizeMismatch` if `source` doesn't have the texels of `source_size`,
    /// `UploadError::FormatNotSupported` if `P` can't be uploaded to the texture, which includes
    /// compressed textures (see `write_compressed_box_from`), and
    /// `UploadError::MultisampleTexture` for multisample textures.
    pub fn write_box_from<P: PixelValue>(
        &self,
        offset: (u32, u32, u32),
        size: (u32, u32, u32),
        source: &[P],
        source_size: (u32, u32, u32),
        source_offset: (u32, u32, u32),
    ) -> Result<(), UploadError> {
        let texture = self.get_texture();
        if self.get_samples().is_some() {
            return Err(UploadError::MultisampleTexture);
        }
        if !fits(offset, size, image_extent(self)) || !fits(source_offset, size, source_size) {
            return Err(UploadError::OutOfBounds);
        }
        let texels = source_size.0 as usize * source_size.1 as usize * source_size.2 as usize;
        if source.len() != texels {
            return Err(UploadError::DataSizeMismatch {
                expected: texels * std::mem::size_of::<P>(),
                obtained: std::mem::size_of_val(source),
            });
        }
        if is_compressed(texture.get_requested_format()) {
            return Err(UploadError::FormatNotSupported);
        }
        let (format, ty) = image_format::client_format_to_glenum(
            TextureExt::get_context(texture),
            ClientFormatAny::ClientFormat(P::get_format()),
            texture.get_requested_format(),
            false,
        )
        .map_err(|_| UploadError::FormatNotSupported)?;

        let mut ctxt = TextureExt::get_context(texture).make_current();
        // shader writes made through image units must be done before the image is changed
        texture.prepare_for_access(&mut ctxt, crate::TextureAccess::Update);
        let level = self.get_level() as i32;
        unsafe {
            BufferAny::unbind_pixel_unpack(&mut ctxt);
            if ctxt.state.pixel_store_unpack_alignment != 1 {
                ctxt.state.pixel_store_unpack_alignment = 1;
                ctxt.glow.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            }
            UNPACK.set(&mut ctxt, source_size, source_offset);
            let pixels = glow::PixelUnpackData::Slice(Some(std::slice::from_raw_parts(
                source.as_ptr() as *const u8,
                std::mem::size_of_val(source),
            )));

            let bind_point = texture.bind_to_current(&mut ctxt);
            let (x, y, z) = (offset.0 as i32, offset.1 as i32, offset.2 as i32);
            let (w, h, d) = (size.0 as i32, size.1 as i32, size.2 as i32);
            match bind_point {
                gl::TEXTURE_1D => ctxt.extra.TexSubImage1D(
                    bind_point,
                    level,
                    x,
                    w,
                    format,
                    ty,
                    source.as_ptr() as *const _,
                ),
                // the layers of 1D arrays are their rows
                gl::TEXTURE_2D | gl::TEXTURE_RECTANGLE | gl::TEXTURE_1D_ARRAY => ctxt
                    .glow
                    .tex_sub_image_2d(bind_point, level, x, y, w, h, format, ty, pixels),
                // the faces of a cube map are separate images, read from the slices of the
                // client image
                gl::TEXTURE_CUBE_MAP => {
                    let faces = image_targets(bind_point);
                    for i in 0..size.2 {
                        // 2D uploads ignore the skipped images: the rows of the slices
                        // before are skipped instead
                        let rows = source_offset.1 + (source_offset.2 + i) * source_size.1;
                        UNPACK.set(&mut ctxt, source_size, (source_offset.0, rows, 0));
                        ctxt.glow.tex_sub_image_2d(
                            faces[(offset.2 + i) as usize],
                            level,
                            x,
                            y,
                            w,
                            h,
                            format,
                            ty,
                            glow::PixelUnpackData::Slice(Some(std::slice::from_raw_parts(
                                source.as_ptr() as *const u8,
                                std::mem::size_of_val(source),
                            ))),
                        );
                    }
                }
                _ => ctxt
                    .glow
                    .tex_sub_image_3d(bind_point, level, x, y, z, w, h, d, format, ty, pixels),
            }
            UNPACK.reset(&mut ctxt);
        }
        Ok(())
    }

    /// Writes the compressed blocks of the box of `size` texels at `source_offset` of a
    /// compressed client image of `source_size` texels (`source`, as the blocks of a texture of
    /// that size are stored) to the box at `offset` of this level of a compressed texture,
    /// without copying the blocks out of the image first (`GL_UNPACK_COMPRESSED_BLOCK_SIZE` and
    /// the other block parameters).
    ///
    /// The boxes must start on the 4 × 4 blocks of the formats of SafeGL and end on them or on
    /// the edge of their images, or `UploadError::MisalignedBlocks` is returned.
    ///
    /// Needs OpenGL 4.2 or `GL_ARB_compressed_texture_pixel_storage`, otherwise returns
    /// `UploadError::NotSupported`. See `write_box_from` for the other errors.
    #[cfg(safegl_gl42)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl42)))]
    pub fn write_compressed_box_from(
        &self,
        offset: (u32, u32, u32),
        size: (u32, u32, u32),
        source: &[u8],
        source_size: (u32, u32, u32),
        source_offset: (u32, u32, u32),
    ) -> Result<(), UploadError> {
        let texture = self.get_texture();
        let extent = image_extent(self);
        if !fits(offset, size, extent) || !fits(source_offset, size, source_size) {
            return Err(UploadError::OutOfBounds);
        }
        if !is_compressed(texture.get_requested_format()) {
            return Err(UploadError::FormatNotSupported);
        }
        if !is_block_aligned(offset, size, extent)
            || !is_block_aligned(source_offset, size, source_size)
        {
            return Err(UploadError::MisalignedBlocks);
        }

        let mut ctxt = TextureExt::get_context(texture).make_current();
        if !(ctxt.version >= &Version(Api::Gl, 4, 2)
            || ctxt.extensions.gl_arb_compressed_texture_pixel_storage)
        {
            return Err(UploadError::NotSupported);
        }
        texture.prepare_for_access(&mut ctxt, crate::TextureAccess::Update);
        let level = self.get_level() as i32;
        unsafe {
            let bind_point = texture.bind_to_current(&mut ctxt);
            let faces = image_targets(bind_point);
            let internal_format =
                ctxt.glow
                    .get_tex_level_parameter_i32(faces[0], level, gl::TEXTURE_INTERNAL_FORMAT)
                    as gl::types::GLenum;
            let format = ClientFormatAny::from_internal_compressed_format(internal_format)
                .ok_or(UploadError::FormatNotSupported)?;
            let block_bytes = format
                .get_buffer_size(4, Some(4), None, None)
                .ok_or(UploadError::FormatNotSupported)?;
            let blocks = |texels: u32| texels.div_ceil(4) as usize;
            let expected = blocks(source_size.0) * blocks(source_size.1) * block_bytes
                * source_size.2 as usize;
            if source.len() != expected {
                return Err(UploadError::DataSizeMismatch {
                    expected,
                    obtained: source.len(),
                });
            }

            BufferAny::unbind_pixel_unpack(&mut ctxt);
            ctxt.glow.pixel_store_i32(gl::UNPACK_COMPRESSED_BLOCK_WIDTH, 4);
            ctxt.glow.pixel_store_i32(gl::UNPACK_COMPRESSED_BLOCK_HEIGHT, 4);
            ctxt.glow.pixel_store_i32(gl::UNPACK_COMPRESSED_BLOCK_DEPTH, 1);
            ctxt.glow
                .pixel_store_i32(gl::UNPACK_COMPRESSED_BLOCK_SIZE, block_bytes as i32);

            // the size given to OpenGL is the size of the blocks of the box itself; the blocks
            // are read from the client image with the skips, inside `source`
            let image_size = blocks(size.0) * blocks(size.1) * block_bytes;
            let blocks_of = |count: usize| {
                glow::CompressedPixelUnpackData::Slice(&source[..image_size * count])
            };
            let (x, y, z) = (offset.0 as i32, offset.1 as i32, offset.2 as i32);
            let (w, h) = (size.0 as i32, size.1 as i32);
            match bind_point {
                gl::TEXTURE_2D => {
                    UNPACK.set(&mut ctxt, source_size, source_offset);
                    ctxt.glow.compressed_tex_sub_image_2d(
                        bind_point,
                        level,
                        x,
                        y,
                        w,
                        h,
                        internal_format,
                        blocks_of(1),
                    );
                }
                gl::TEXTURE_CUBE_MAP => {
                    for i in 0..size.2 {
                        // 2D uploads ignore the skipped images: the rows of the slices
                        // before are skipped instead
                        let rows = source_offset.1 + (source_offset.2 + i) * source_size.1;
                        UNPACK.set(&mut ctxt, source_size, (source_offset.0, rows, 0));
                        ctxt.glow.compressed_tex_sub_image_2d(
                            faces[(offset.2 + i) as usize],
                            level,
                            x,
                            y,
                            w,
                            h,
                            internal_format,
                            blocks_of(1),
                        );
                    }
                }
                _ => {
                    UNPACK.set(&mut ctxt, source_size, source_offset);
                    ctxt.glow.compressed_tex_sub_image_3d(
                        bind_point,
                        level,
                        x,
                        y,
                        z,
                        w,
                        h,
                        size.2 as i32,
                        internal_format,
                        blocks_of(size.2 as usize),
                    );
                }
            }
            UNPACK.reset(&mut ctxt);
            for pname in [
                gl::UNPACK_COMPRESSED_BLOCK_WIDTH,
                gl::UNPACK_COMPRESSED_BLOCK_HEIGHT,
                gl::UNPACK_COMPRESSED_BLOCK_DEPTH,
                gl::UNPACK_COMPRESSED_BLOCK_SIZE,
            ] {
                ctxt.glow.pixel_store_i32(pname, 0);
            }
        }
        Ok(())
    }
}
