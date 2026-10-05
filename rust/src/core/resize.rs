use std::borrow::Cow;

use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer};
use image::DynamicImage;

use crate::api::types::{ErrorCode, ImageGuardError};

/// Largest size that fits inside `max_w` × `max_h`, keeping the aspect ratio.
/// Never upscales and never returns 0.
pub fn fit_within(w: u32, h: u32, max_w: Option<u32>, max_h: Option<u32>) -> (u32, u32) {
    let sx = max_w.map_or(1.0, |m| m as f64 / w as f64);
    let sy = max_h.map_or(1.0, |m| m as f64 / h as f64);
    let s = sx.min(sy).min(1.0);
    if s >= 1.0 {
        return (w, h);
    }
    let nw = ((w as f64 * s).round() as u32).clamp(1, max_w.unwrap_or(u32::MAX));
    let nh = ((h as f64 * s).round() as u32).clamp(1, max_h.unwrap_or(u32::MAX));
    (nw, nh)
}

/// Scales `(w, h)` so the longest side becomes `longest`.
pub fn scale_to_longest(w: u32, h: u32, longest: u32) -> (u32, u32) {
    let s = longest as f64 / w.max(h) as f64;
    (
        ((w as f64 * s).round() as u32).max(1),
        ((h as f64 * s).round() as u32).max(1),
    )
}

/// High-quality (Lanczos3, SIMD) resize. Returns the input untouched when the
/// size does not change.
pub fn resize(
    src: &DynamicImage,
    w: u32,
    h: u32,
) -> Result<Cow<'_, DynamicImage>, ImageGuardError> {
    if src.width() == w && src.height() == h {
        return Ok(Cow::Borrowed(src));
    }
    let mut dst = DynamicImage::new(w, h, src.color());
    Resizer::new()
        .resize(
            src,
            &mut dst,
            &ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3)),
        )
        .map_err(|e| ImageGuardError::new(ErrorCode::Internal, format!("Resize failed: {e}")))?;
    Ok(Cow::Owned(dst))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_keeps_aspect_ratio() {
        assert_eq!(fit_within(4000, 3000, Some(1600), Some(1600)), (1600, 1200));
        assert_eq!(fit_within(3000, 4000, Some(1600), Some(1600)), (1200, 1600));
        assert_eq!(fit_within(4000, 3000, Some(1080), None), (1080, 810));
        assert_eq!(fit_within(4000, 3000, None, Some(300)), (400, 300));
    }

    #[test]
    fn fit_never_upscales() {
        assert_eq!(fit_within(800, 600, Some(1600), Some(1600)), (800, 600));
        assert_eq!(fit_within(800, 600, None, None), (800, 600));
    }

    #[test]
    fn fit_extreme_panorama_never_zero() {
        let (w, h) = fit_within(30000, 10, Some(100), Some(100));
        assert_eq!(w, 100);
        assert!(h >= 1);
    }

    #[test]
    fn resize_changes_dimensions() {
        let img = DynamicImage::new_rgb8(400, 200);
        let out = resize(&img, 100, 50).unwrap();
        assert_eq!((out.width(), out.height()), (100, 50));
        assert!(matches!(resize(&img, 400, 200).unwrap(), Cow::Borrowed(_)));
    }

    #[test]
    fn resize_rgba() {
        let img = DynamicImage::new_rgba8(64, 64);
        let out = resize(&img, 10, 10).unwrap();
        assert!(out.as_rgba8().is_some());
    }
}
