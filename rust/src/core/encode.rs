use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ImageEncoder};
use jpeg_encoder::{ColorType, Encoder, SamplingFactor};

use crate::api::types::{ErrorCode, ImageGuardError};

fn enc_err(e: impl std::fmt::Display) -> ImageGuardError {
    ImageGuardError::new(ErrorCode::EncodingFailed, e.to_string())
}

/// Progressive JPEG with optimized Huffman tables (smallest output for a given
/// quality). Any alpha must have been flattened before calling this.
pub fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, ImageGuardError> {
    let owned;
    let rgb = match img.as_rgb8() {
        Some(rgb) => rgb,
        None => {
            owned = img.to_rgb8();
            &owned
        }
    };
    let (w, h) = rgb.dimensions();
    if w > u16::MAX as u32 || h > u16::MAX as u32 {
        return Err(enc_err(format!(
            "{w}x{h} is too large for JPEG (max 65535)"
        )));
    }
    let mut out = Vec::with_capacity(rgb.len() / 8);
    let mut enc = Encoder::new(&mut out, quality.clamp(1, 100));
    enc.set_progressive(true);
    enc.set_optimized_huffman_tables(true);
    enc.set_sampling_factor(SamplingFactor::R_4_2_0);
    enc.encode(rgb.as_raw(), w as u16, h as u16, ColorType::Rgb)
        .map_err(enc_err)?;
    Ok(out)
}

pub fn encode_png(img: &DynamicImage) -> Result<Vec<u8>, ImageGuardError> {
    let mut out = Vec::new();
    PngEncoder::new_with_quality(&mut out, CompressionType::Best, FilterType::Adaptive)
        .write_image(
            img.as_bytes(),
            img.width(),
            img.height(),
            img.color().into(),
        )
        .map_err(enc_err)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        }))
    }

    #[test]
    fn jpeg_roundtrip() {
        let bytes = encode_jpeg(&gradient(120, 80), 80).unwrap();
        assert_eq!(&bytes[..2], &[0xFF, 0xD8]);
        let back = image::load_from_memory(&bytes).unwrap();
        assert_eq!((back.width(), back.height()), (120, 80));
    }

    #[test]
    fn lower_quality_is_smaller() {
        let img = gradient(400, 300);
        let hi = encode_jpeg(&img, 95).unwrap().len();
        let lo = encode_jpeg(&img, 30).unwrap().len();
        assert!(lo < hi, "q30={lo} q95={hi}");
    }

    #[test]
    fn jpeg_accepts_rgba_input() {
        let img = DynamicImage::new_rgba8(10, 10);
        assert!(encode_jpeg(&img, 80).is_ok());
    }

    #[test]
    fn png_roundtrip_keeps_alpha() {
        let img = DynamicImage::new_rgba8(16, 16);
        let bytes = encode_png(&img).unwrap();
        let back = image::load_from_memory(&bytes).unwrap();
        assert!(back.color().has_alpha());
    }
}
