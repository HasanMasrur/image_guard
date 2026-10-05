//! End-to-end flow used by the Dart API.

use std::time::Instant;

use crate::api::types::{
    CompressOptions, CompressResult, ImageGuardError, ImageInfo, InputFormat, OutputFormat,
};
use crate::core::{decode, metadata, resize, target, validate};
use image::metadata::Orientation;

pub fn compress(
    bytes: &[u8],
    options: &CompressOptions,
) -> Result<CompressResult, ImageGuardError> {
    let started = Instant::now();
    validate::validate_options(options)?;
    // Always decode fully: this proves the file is a real, intact image
    // before anything is handed back to the app (or uploaded).
    let decoded = decode::decode(bytes)?;
    let (ow, oh) = (decoded.image.width(), decoded.image.height());
    let original_size = bytes.len() as u32; // ≤ MAX_INPUT_BYTES, checked by decode

    let finish = |out: Vec<u8>, w, h, quality, kept_original, attempts| CompressResult {
        size_bytes: out.len() as u32,
        bytes: out,
        width: w,
        height: h,
        format: options.format,
        quality,
        original_size_bytes: original_size,
        original_width: ow,
        original_height: oh,
        original_format: decoded.format,
        kept_original,
        attempts,
        elapsed_ms: started.elapsed().as_millis().min(u32::MAX as u128) as u32,
    };

    let same_format = matches!(
        (decoded.format, options.format),
        (InputFormat::Jpeg, OutputFormat::Jpeg) | (InputFormat::Png, OutputFormat::Png)
    );
    let fits_resolution =
        resize::fit_within(ow, oh, options.max_width, options.max_height) == (ow, oh);

    // Case: the input already satisfies everything (e.g. 100 KB photo with a
    // 500 KB limit) → return it as-is, never make it bigger or worse.
    if options.keep_original_if_fits
        && same_format
        && fits_resolution
        && original_size <= options.max_bytes
    {
        // Stripping EXIF would also drop a rotation flag, so a rotated image
        // must be re-encoded upright instead.
        let rotation_ok =
            decoded.orientation == Orientation::NoTransforms || !options.strip_metadata;
        if rotation_ok {
            let out = if options.strip_metadata {
                match decoded.format {
                    InputFormat::Jpeg => metadata::strip_jpeg(bytes),
                    _ => metadata::strip_png(bytes),
                }
            } else {
                Some(bytes.to_vec())
            };
            if let Some(out) = out {
                return Ok(finish(out, ow, oh, None, true, 0));
            }
        }
    }

    // Re-encoding an image that already fit must not make it bigger than it was.
    let cap = if same_format && fits_resolution {
        options.max_bytes.min(original_size)
    } else {
        options.max_bytes
    } as u64;

    let keep_alpha = options.format == OutputFormat::Png;
    let image = decode::normalize(decoded.image, keep_alpha);
    let enc = target::encode_to_target(&image, options, cap)?;
    Ok(finish(
        enc.bytes,
        enc.width,
        enc.height,
        enc.quality,
        false,
        enc.attempts,
    ))
}

pub fn image_info(bytes: &[u8]) -> Result<ImageInfo, ImageGuardError> {
    let p = decode::probe(bytes)?;
    Ok(ImageInfo {
        format: p.format,
        width: p.width,
        height: p.height,
        size_bytes: bytes.len() as u32,
        has_orientation: p.orientation != Orientation::NoTransforms,
    })
}
