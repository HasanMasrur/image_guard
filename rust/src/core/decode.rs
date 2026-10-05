use std::io::Cursor;

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageError, ImageFormat, ImageReader, Limits, RgbImage};

use crate::api::types::{ErrorCode, ImageGuardError, InputFormat};
use crate::core::limits::*;

pub struct Decoded {
    /// Pixels with EXIF orientation already applied.
    pub image: DynamicImage,
    pub format: InputFormat,
    pub orientation: Orientation,
}

/// Header-only information (no pixel decoding).
pub struct Probe {
    pub format: InputFormat,
    /// Width/height after orientation.
    pub width: u32,
    pub height: u32,
    pub orientation: Orientation,
}

pub fn probe(bytes: &[u8]) -> Result<Probe, ImageGuardError> {
    let (reader, format) = reader(bytes)?;
    let mut decoder = reader.into_decoder().map_err(map_image_error)?;
    let (w, h) = decoder.dimensions();
    check_dimensions(w, h)?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let (width, height) = oriented_dimensions(w, h, orientation);
    Ok(Probe {
        format,
        width,
        height,
        orientation,
    })
}

pub fn decode(bytes: &[u8]) -> Result<Decoded, ImageGuardError> {
    let (reader, format) = reader(bytes)?;
    let mut decoder = reader.into_decoder().map_err(map_image_error)?;
    let (w, h) = decoder.dimensions();
    check_dimensions(w, h)?;
    // A broken EXIF block should not make a valid picture unusable.
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder).map_err(map_image_error)?;
    image.apply_orientation(orientation);
    Ok(Decoded {
        image,
        format,
        orientation,
    })
}

/// Converts any pixel layout to 8-bit RGB, or RGBA when `keep_alpha` is set
/// and the image is actually transparent. Transparent pixels are flattened
/// onto white when alpha is dropped (JPEG has no alpha channel).
pub fn normalize(image: DynamicImage, keep_alpha: bool) -> DynamicImage {
    if !image.color().has_alpha() {
        return DynamicImage::ImageRgb8(image.into_rgb8());
    }
    let rgba = image.into_rgba8();
    if keep_alpha {
        return DynamicImage::ImageRgba8(rgba);
    }
    let (w, h) = rgba.dimensions();
    let mut out = RgbImage::new(w, h);
    for (src, dst) in rgba.pixels().zip(out.pixels_mut()) {
        let a = src[3] as u32;
        for c in 0..3 {
            dst[c] = ((src[c] as u32 * a + 255 * (255 - a) + 127) / 255) as u8;
        }
    }
    DynamicImage::ImageRgb8(out)
}

pub fn oriented_dimensions(w: u32, h: u32, o: Orientation) -> (u32, u32) {
    match o {
        Orientation::Rotate90
        | Orientation::Rotate270
        | Orientation::Rotate90FlipH
        | Orientation::Rotate270FlipH => (h, w),
        _ => (w, h),
    }
}

type Reader<'a> = ImageReader<Cursor<&'a [u8]>>;

fn reader(bytes: &[u8]) -> Result<(Reader<'_>, InputFormat), ImageGuardError> {
    crate::core::io::check_input_len(bytes.len() as u64)?;
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| ImageGuardError::new(ErrorCode::Internal, e.to_string()))?;
    let format =
        match reader.format() {
            Some(ImageFormat::Jpeg) => InputFormat::Jpeg,
            Some(ImageFormat::Png) => InputFormat::Png,
            Some(ImageFormat::WebP) => InputFormat::WebP,
            Some(ImageFormat::Gif) => InputFormat::Gif,
            Some(ImageFormat::Bmp) => InputFormat::Bmp,
            Some(other) => {
                return Err(ImageGuardError::new(
                    ErrorCode::UnsupportedFormat,
                    format!("{other:?} images are not supported (use JPEG, PNG, WebP, GIF or BMP)"),
                ))
            }
            None => return Err(ImageGuardError::new(
                ErrorCode::UnsupportedFormat,
                "Unknown image format (HEIC is not supported; JPEG, PNG, WebP, GIF and BMP are)",
            )),
        };
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DECODE_SIDE);
    limits.max_image_height = Some(MAX_DECODE_SIDE);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    reader.limits(limits);
    Ok((reader, format))
}

fn check_dimensions(w: u32, h: u32) -> Result<(), ImageGuardError> {
    if w == 0 || h == 0 {
        return Err(ImageGuardError::new(
            ErrorCode::CorruptImage,
            "Image has zero width or height",
        ));
    }
    if w > MAX_DECODE_SIDE || h > MAX_DECODE_SIDE || w as u64 * h as u64 > MAX_DECODE_PIXELS {
        return Err(ImageGuardError::new(
            ErrorCode::ImageTooLarge,
            format!(
                "Image is {w}x{h}; the limit is {MAX_DECODE_SIDE} px per side and 100 megapixels"
            ),
        ));
    }
    Ok(())
}

pub fn map_image_error(e: ImageError) -> ImageGuardError {
    match e {
        ImageError::Limits(e) => ImageGuardError::new(ErrorCode::ImageTooLarge, e.to_string()),
        ImageError::Unsupported(e) => {
            ImageGuardError::new(ErrorCode::UnsupportedFormat, e.to_string())
        }
        ImageError::Decoding(e) => ImageGuardError::new(ErrorCode::CorruptImage, e.to_string()),
        // Truncated files surface as I/O errors from the in-memory cursor.
        ImageError::IoError(e) => ImageGuardError::new(ErrorCode::CorruptImage, e.to_string()),
        other => ImageGuardError::new(ErrorCode::Internal, other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn normalize_flattens_alpha_onto_white() {
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(0, 0, Rgba([0, 0, 0, 0])); // fully transparent → white
        img.put_pixel(1, 0, Rgba([10, 20, 30, 255])); // opaque → unchanged
        let out = normalize(DynamicImage::ImageRgba8(img), false);
        let rgb = out.as_rgb8().unwrap();
        assert_eq!(rgb.get_pixel(0, 0).0, [255, 255, 255]);
        assert_eq!(rgb.get_pixel(1, 0).0, [10, 20, 30]);
    }

    #[test]
    fn normalize_keeps_alpha_when_asked() {
        let img = RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 4]));
        assert!(normalize(DynamicImage::ImageRgba8(img), true)
            .as_rgba8()
            .is_some());
    }

    #[test]
    fn normalize_opaque_to_rgb() {
        let img = DynamicImage::new_luma8(3, 3);
        assert!(normalize(img, true).as_rgb8().is_some());
    }

    #[test]
    fn rotation_swaps_dimensions() {
        assert_eq!(oriented_dimensions(4, 3, Orientation::Rotate90), (3, 4));
        assert_eq!(oriented_dimensions(4, 3, Orientation::Rotate180), (4, 3));
        assert_eq!(
            oriented_dimensions(4, 3, Orientation::Rotate270FlipH),
            (3, 4)
        );
    }

    #[test]
    fn garbage_is_unsupported() {
        let e = decode(b"this is not an image at all").err().unwrap();
        assert_eq!(e.code, ErrorCode::UnsupportedFormat);
    }

    #[test]
    fn empty_input() {
        assert_eq!(decode(b"").err().unwrap().code, ErrorCode::EmptyInput);
    }
}
