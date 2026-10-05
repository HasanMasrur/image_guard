//! The size search: find the best-looking output that is ≤ the byte limit.
//!
//! 1. Fit inside maxWidth × maxHeight.
//! 2. JPEG: try maxQuality, then binary-search quality down to minQuality.
//!    PNG: lossless, a single encode per size.
//! 3. Still too big → shrink the resolution (always resampled from the source,
//!    never from an already-shrunk copy) and repeat.
//! 4. Longest side would drop below minDimension → `CannotMeetTarget`.

use image::DynamicImage;

use crate::api::types::{CompressOptions, ErrorCode, ImageGuardError, OutputFormat};
use crate::core::{encode, resize};

#[derive(Debug)]
pub struct Encoded {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub quality: Option<u8>,
    pub attempts: u32,
}

/// `cap` is the effective byte limit (≤ `options.max_bytes`).
pub fn encode_to_target(
    src: &DynamicImage,
    options: &CompressOptions,
    cap: u64,
) -> Result<Encoded, ImageGuardError> {
    let (mut w, mut h) = resize::fit_within(
        src.width(),
        src.height(),
        options.max_width,
        options.max_height,
    );
    let mut attempts = 0u32;

    loop {
        let img = resize::resize(src, w, h)?;
        let (found, smallest) = match options.format {
            OutputFormat::Jpeg => search_quality(&img, options, cap, &mut attempts)?,
            OutputFormat::Png => {
                let bytes = encode::encode_png(&img)?;
                attempts += 1;
                let len = bytes.len() as u64;
                (
                    if len <= cap {
                        Some((bytes, None))
                    } else {
                        None
                    },
                    len,
                )
            }
        };
        if let Some((bytes, quality)) = found {
            return Ok(Encoded {
                bytes,
                width: w,
                height: h,
                quality,
                attempts,
            });
        }

        let longest = w.max(h);
        if longest <= options.min_dimension {
            return Err(ImageGuardError::new(
                ErrorCode::CannotMeetTarget,
                format!(
                    "Cannot fit under {} bytes: the smallest result was {} bytes at {}x{} \
                     (minDimension {}, minQuality {}). Increase maxBytes or lower minDimension/minQuality.",
                    cap, smallest, w, h, options.min_dimension, options.min_quality
                ),
            ));
        }
        // JPEG/PNG size grows roughly with pixel count, so scale each side by
        // sqrt(target / current), with a margin and bounded step.
        let factor = ((cap as f64 / smallest as f64).sqrt() * 0.95).clamp(0.5, 0.9);
        let next = ((longest as f64 * factor).floor() as u32).max(options.min_dimension);
        (w, h) = resize::scale_to_longest(w, h, next.min(longest - 1));
    }
}

type Found = Option<(Vec<u8>, Option<u8>)>;

/// Highest quality in [min, max] whose output fits; also returns the smallest
/// size seen (used to choose the next resolution).
fn search_quality(
    img: &DynamicImage,
    o: &CompressOptions,
    cap: u64,
    attempts: &mut u32,
) -> Result<(Found, u64), ImageGuardError> {
    let mut try_q = |q: u8| -> Result<Vec<u8>, ImageGuardError> {
        *attempts += 1;
        encode::encode_jpeg(img, q)
    };

    // Lowest quality first: if even that is too big, this resolution is
    // hopeless and we move on after a single encode.
    let bottom = try_q(o.min_quality)?;
    let smallest = bottom.len() as u64;
    if smallest > cap || o.min_quality == o.max_quality {
        let found = (smallest <= cap).then_some((bottom, Some(o.min_quality)));
        return Ok((found, smallest));
    }

    let top = try_q(o.max_quality)?;
    if top.len() as u64 <= cap {
        return Ok((Some((top, Some(o.max_quality))), smallest));
    }

    let (mut lo, mut hi) = (o.min_quality as i32 + 1, o.max_quality as i32 - 1);
    let mut best: Found = Some((bottom, Some(o.min_quality)));
    while lo <= hi {
        let q = ((lo + hi) / 2) as u8;
        let bytes = try_q(q)?;
        if bytes.len() as u64 <= cap {
            best = Some((bytes, Some(q)));
            lo = q as i32 + 1;
        } else {
            hi = q as i32 - 1;
        }
    }
    Ok((best, smallest))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pseudo-random noise: the hardest content to compress.
    fn noise(w: u32, h: u32) -> DynamicImage {
        let mut s = 0x1234_5678u32;
        DynamicImage::ImageRgb8(image::RgbImage::from_fn(w, h, |_, _| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            image::Rgb([s as u8, (s >> 8) as u8, (s >> 16) as u8])
        }))
    }

    fn opts(max_bytes: u32) -> CompressOptions {
        CompressOptions {
            max_bytes,
            ..Default::default()
        }
    }

    #[test]
    fn fits_under_target() {
        let r = encode_to_target(&noise(1200, 900), &opts(50 * 1024), 50 * 1024).unwrap();
        assert!(r.bytes.len() as u64 <= 50 * 1024);
        assert!(
            r.width < 1200,
            "noise at 1200px cannot be 50 KB, must shrink"
        );
    }

    #[test]
    fn uses_best_quality_that_fits() {
        let img = noise(300, 300);
        let o = opts(30 * 1024);
        let r = encode_to_target(&img, &o, o.max_bytes as u64).unwrap();
        if let Some(q) = r.quality {
            if q < o.max_quality {
                // One step higher must not fit, otherwise the search was lazy.
                let bigger =
                    encode::encode_jpeg(&resize::resize(&img, r.width, r.height).unwrap(), q + 1)
                        .unwrap();
                assert!(bigger.len() as u64 > o.max_bytes as u64);
            }
        }
    }

    #[test]
    fn respects_max_dimensions() {
        let o = CompressOptions {
            max_width: Some(500),
            max_height: Some(500),
            max_bytes: 10 * 1024 * 1024,
            ..Default::default()
        };
        let r = encode_to_target(&noise(1000, 800), &o, o.max_bytes as u64).unwrap();
        assert_eq!((r.width, r.height), (500, 400));
    }

    #[test]
    fn impossible_target_errors() {
        let o = CompressOptions {
            max_bytes: 1024,
            min_dimension: 1000,
            ..Default::default()
        };
        let e = encode_to_target(&noise(1200, 1200), &o, o.max_bytes as u64).unwrap_err();
        assert_eq!(e.code, ErrorCode::CannotMeetTarget);
        assert!(e.message.contains("smallest result"));
    }

    #[test]
    fn never_goes_below_min_dimension() {
        let o = CompressOptions {
            max_bytes: 4 * 1024,
            min_dimension: 200,
            ..Default::default()
        };
        match encode_to_target(&noise(800, 600), &o, o.max_bytes as u64) {
            Ok(r) => assert!(r.width.max(r.height) >= 200),
            Err(e) => assert_eq!(e.code, ErrorCode::CannotMeetTarget),
        }
    }

    #[test]
    fn png_shrinks_resolution() {
        let o = CompressOptions {
            max_bytes: 40 * 1024,
            format: OutputFormat::Png,
            ..Default::default()
        };
        let r = encode_to_target(&noise(400, 400), &o, o.max_bytes as u64).unwrap();
        assert!(r.bytes.len() as u64 <= o.max_bytes as u64);
        assert!(r.quality.is_none());
        assert!(r.width < 400);
    }

    #[test]
    fn fixed_quality_when_min_equals_max() {
        let o = CompressOptions {
            min_quality: 70,
            max_quality: 70,
            max_bytes: 30 * 1024,
            ..Default::default()
        };
        let r = encode_to_target(&noise(600, 600), &o, o.max_bytes as u64).unwrap();
        assert_eq!(r.quality, Some(70));
    }
}
