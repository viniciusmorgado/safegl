//! Texture uploads from the images of browsers: image, canvas and video elements, image bitmaps,
//! image data and offscreen canvases (`texImage2D` and `texSubImage2D` with a source).

use std::error::Error;
use std::fmt;

use wasm_bindgen::JsValue;
use web_sys::{
    HtmlCanvasElement, HtmlImageElement, HtmlVideoElement, ImageBitmap, ImageData,
    OffscreenCanvas, WebGl2RenderingContext,
};

use crate::buffer::BufferAny;
use crate::gl;
use crate::texture::{TextureAnyMipmap, TextureCreationError, UploadError};
use crate::{BufferExt, ContextExt, TextureExt};
use glow::HasContext;

mod sealed {
    pub trait Sealed {}
}

/// An image of the browser that a texture can be written from: an image, canvas or video
/// element, an image bitmap, image data or an offscreen canvas.
///
/// An image element must have loaded, and a video element must have a frame
/// (`HAVE_CURRENT_DATA`). The images of other origins can only be written with the permission of
/// their server (CORS), otherwise the browser refuses them.
pub trait WebImageSource: sealed::Sealed {
    /// The width and the height of the image in pixels.
    fn dimensions(&self) -> (u32, u32);

    /// Writes the image at `x`, `y` of the level `level` of the texture bound to `target`, as
    /// pixels of the client format `format` and type `ty`.
    #[doc(hidden)]
    fn upload(
        &self,
        gl: &WebGl2RenderingContext,
        target: u32,
        level: i32,
        offset: (i32, i32),
        client: (u32, u32),
    ) -> Result<(), JsValue>;
}

macro_rules! web_image_source {
    ($ty:ty, $upload:ident, |$image:ident| $dimensions:expr) => {
        impl sealed::Sealed for $ty {}

        impl WebImageSource for $ty {
            fn dimensions(&self) -> (u32, u32) {
                let $image = self;
                $dimensions
            }

            fn upload(
                &self,
                gl: &WebGl2RenderingContext,
                target: u32,
                level: i32,
                (x, y): (i32, i32),
                (format, ty): (u32, u32),
            ) -> Result<(), JsValue> {
                gl.$upload(target, level, x, y, format, ty, self)
            }
        }
    };
}

web_image_source!(
    HtmlImageElement,
    tex_sub_image_2d_with_u32_and_u32_and_html_image_element,
    |image| (image.natural_width(), image.natural_height())
);
web_image_source!(
    HtmlCanvasElement,
    tex_sub_image_2d_with_u32_and_u32_and_html_canvas_element,
    |canvas| (canvas.width(), canvas.height())
);
web_image_source!(
    HtmlVideoElement,
    tex_sub_image_2d_with_u32_and_u32_and_html_video_element,
    |video| (video.video_width(), video.video_height())
);
web_image_source!(
    ImageBitmap,
    tex_sub_image_2d_with_u32_and_u32_and_image_bitmap,
    |bitmap| (bitmap.width(), bitmap.height())
);
web_image_source!(
    ImageData,
    tex_sub_image_2d_with_u32_and_u32_and_image_data,
    |data| (data.width(), data.height())
);
web_image_source!(
    OffscreenCanvas,
    tex_sub_image_2d_with_u32_and_u32_and_offscreen_canvas,
    |canvas| (canvas.width(), canvas.height())
);

/// How the browser converts an image while it writes it to a texture (the pixel store of WebGL).
/// Image bitmaps are converted when they are created instead, and ignore these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebImageOptions {
    /// Flips the image vertically (`UNPACK_FLIP_Y_WEBGL`), so that its top row, which is first in
    /// the image, is at the top of the texture, whose first row is at the bottom. The default is
    /// `false`.
    pub flip_y: bool,

    /// Multiplies the colors by the alpha (`UNPACK_PREMULTIPLY_ALPHA_WEBGL`). The default is
    /// `false`.
    pub premultiply_alpha: bool,

    /// Converts the colors from the color space of the image to the one of the browser
    /// (`UNPACK_COLORSPACE_CONVERSION_WEBGL`). The default is `true`; `false` keeps the values
    /// stored in the image, for images that aren't colors, such as normal maps.
    pub colorspace_conversion: bool,
}

impl Default for WebImageOptions {
    fn default() -> WebImageOptions {
        WebImageOptions {
            flip_y: false,
            premultiply_alpha: false,
            colorspace_conversion: true,
        }
    }
}

// the pixel store of WebGL
pub(crate) const UNPACK_FLIP_Y_WEBGL: u32 = 0x9240;
pub(crate) const UNPACK_PREMULTIPLY_ALPHA_WEBGL: u32 = 0x9241;
pub(crate) const UNPACK_COLORSPACE_CONVERSION_WEBGL: u32 = 0x9243;
pub(crate) const BROWSER_DEFAULT_WEBGL: i32 = 0x9244;

/// Sets the pixel store of WebGL for `options`.
unsafe fn set_options(gl: &glow::Context, options: WebImageOptions) {
    gl.pixel_store_bool(UNPACK_FLIP_Y_WEBGL, options.flip_y);
    gl.pixel_store_bool(UNPACK_PREMULTIPLY_ALPHA_WEBGL, options.premultiply_alpha);
    let conversion = if options.colorspace_conversion {
        BROWSER_DEFAULT_WEBGL
    } else {
        gl::NONE as i32
    };
    gl.pixel_store_i32(UNPACK_COLORSPACE_CONVERSION_WEBGL, conversion);
}

/// Resets the pixel store of WebGL to its defaults, which other code may change.
pub(crate) unsafe fn reset_options(gl: &glow::Context) {
    set_options(gl, WebImageOptions::default());
}

impl<'t> TextureAnyMipmap<'t> {
    /// Writes an image of the browser to this level at `left` and `bottom`, the image keeping its
    /// size. See `WebImageOptions` for the conversions.
    ///
    /// The texture must be a 2D texture of 8-bit normalized texels (`U8`, `U8U8`, `U8U8U8` or
    /// `U8U8U8U8`, in linear or sRGB colors, or `U5U6U5`, `U4U4U4U4` or `U5U5U5U1`), which keep
    /// the components of the image that they have, or `UploadError::FormatNotSupported` is
    /// returned. Returns `UploadError::OutOfBounds` if the image doesn't fit, and
    /// `UploadError::ImageRefused` if the browser refuses the image: an image of another origin
    /// without permission, or one that hasn't loaded.
    pub fn write_web_image<S>(
        &self,
        left: u32,
        bottom: u32,
        source: &S,
        options: WebImageOptions,
    ) -> Result<(), UploadError>
    where
        S: WebImageSource + ?Sized,
    {
        let texture = self.get_texture();
        // the client formats that WebGL accepts for images with the internal format of the
        // texture, sized or not (without texture storage)
        let client = match texture.created_format() {
            gl::RGBA8 | gl::SRGB8_ALPHA8 | gl::RGBA4 | gl::RGB5_A1 | gl::RGBA => gl::RGBA,
            gl::RGB8 | gl::SRGB8 | gl::RGB565 | gl::RGB => gl::RGB,
            gl::RG8 | gl::RG => gl::RG,
            gl::R8 | gl::RED => gl::RED,
            _ => return Err(UploadError::FormatNotSupported),
        };
        if texture.get_bind_point() != gl::TEXTURE_2D {
            return Err(UploadError::FormatNotSupported);
        }
        let (width, height) = source.dimensions();
        let fits = |offset: u32, size: u32, extent: u32| {
            offset.checked_add(size).is_some_and(|end| end <= extent)
        };
        if !fits(left, width, self.get_width())
            || !fits(bottom, height, self.get_height().unwrap_or(1))
        {
            return Err(UploadError::OutOfBounds);
        }

        let mut ctxt = TextureExt::get_context(texture).make_current();
        if ctxt.objects_lost() {
            return Ok(());
        }
        texture.prepare_for_access(&mut ctxt, crate::TextureAccess::Update);
        let result = unsafe {
            BufferAny::unbind_pixel_unpack(&mut ctxt);
            let bind_point = texture.bind_to_current(&mut ctxt);
            set_options(ctxt.glow, options);
            let level = self.get_level() as i32;
            let offset = (left as i32, bottom as i32);
            let client = (client, gl::UNSIGNED_BYTE);
            let result = source.upload(ctxt.webgl.context(), bind_point, level, offset, client);
            set_options(ctxt.glow, WebImageOptions::default());
            result
        };
        // the exceptions of the browser are `DOMException`s, whose message tells the reason
        result.map_err(|error| UploadError::ImageRefused {
            message: js_sys::Reflect::get(&error, &JsValue::from_str("message"))
                .ok()
                .and_then(|message| message.as_string())
                .unwrap_or_default(),
        })
    }
}

/// Error that can happen when building a texture from an image of the browser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebImageError {
    /// The texture couldn't be created.
    Creation(TextureCreationError),
    /// The image couldn't be written to the texture.
    Upload(UploadError),
}

impl fmt::Display for WebImageError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebImageError::Creation(error) => write!(fmt, "Couldn't create the texture: {}", error),
            WebImageError::Upload(error) => write!(fmt, "Couldn't write the image: {}", error),
        }
    }
}

impl Error for WebImageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WebImageError::Creation(error) => Some(error),
            WebImageError::Upload(error) => Some(error),
        }
    }
}

impl From<TextureCreationError> for WebImageError {
    fn from(error: TextureCreationError) -> WebImageError {
        WebImageError::Creation(error)
    }
}

impl From<UploadError> for WebImageError {
    fn from(error: UploadError) -> WebImageError {
        WebImageError::Upload(error)
    }
}
