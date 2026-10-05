//! End-to-end scenarios for `compress_bytes` / `compress_file`.

mod common;

use common::*;
use image::{DynamicImage, GenericImageView, Rgb};
use image_guard::api::compress::{
    compress_bytes, compress_file, image_info_bytes, image_info_file,
};
use image_guard::api::types::*;

fn opts(max_bytes: u32) -> CompressOptions {
    CompressOptions {
        max_bytes,
        ..Default::default()
    }
}

fn compress(bytes: &[u8], o: CompressOptions) -> Result<CompressResult, ImageGuardError> {
    compress_bytes(bytes.to_vec(), o)
}

/// Every successful result must satisfy the contract.
fn assert_contract(r: &CompressResult, o: &CompressOptions) {
    assert!(
        r.size_bytes <= o.max_bytes,
        "{} > {}",
        r.size_bytes,
        o.max_bytes
    );
    assert_eq!(r.size_bytes as usize, r.bytes.len());
    if let Some(mw) = o.max_width {
        assert!(r.width <= mw);
    }
    if let Some(mh) = o.max_height {
        assert!(r.height <= mh);
    }
    assert!(
        r.width <= r.original_width && r.height <= r.original_height,
        "never upscales"
    );
    if let Some(q) = r.quality {
        assert!((o.min_quality..=o.max_quality).contains(&q));
    }
    let img = decode(&r.bytes);
    assert_eq!(img.dimensions(), (r.width, r.height));
}

// ---------- The case from the requirements ----------

#[test]
fn small_image_with_bigger_limit_is_kept_as_is() {
    // ~100 KB image, user allows 500 KB → must stay ~100 KB, not grow.
    let input = jpeg(&photo(900, 700), 92);
    assert!(
        input.len() > 60 * 1024 && input.len() < 500 * 1024,
        "fixture {} bytes",
        input.len()
    );
    let o = opts(500 * KB);
    let r = compress(&input, o.clone()).unwrap();
    assert_contract(&r, &o);
    assert!(r.kept_original);
    assert_eq!(r.bytes, input, "no metadata → returned byte-for-byte");
    assert_eq!(r.attempts, 0);
}

#[test]
fn kept_original_has_gps_metadata_removed() {
    let exif = exif_orientation(1, b"GPS-SECRET-23.81N-90.41E");
    let input = jpeg_with_exif(&photo(400, 300), 85, Some(&exif));
    let o = opts(500 * KB);
    let r = compress(&input, o.clone()).unwrap();
    assert!(r.kept_original);
    assert!(r.bytes.len() < input.len());
    assert!(!r.bytes.windows(10).any(|w| w == b"GPS-SECRET"));
    // Pixels are untouched.
    assert_eq!(decode(&r.bytes).to_rgb8(), decode(&input).to_rgb8());
}

#[test]
fn metadata_kept_when_strip_disabled() {
    let exif = exif_orientation(1, b"GPS-SECRET");
    let input = jpeg_with_exif(&photo(400, 300), 85, Some(&exif));
    let o = CompressOptions {
        strip_metadata: false,
        ..opts(500 * KB)
    };
    let r = compress(&input, o).unwrap();
    assert_eq!(r.bytes, input);
}

#[test]
fn keep_original_disabled_reencodes_but_never_grows() {
    let input = jpeg(&photo(800, 600), 70);
    let o = CompressOptions {
        keep_original_if_fits: false,
        ..opts(500 * KB)
    };
    let r = compress(&input, o.clone()).unwrap();
    assert_contract(&r, &o);
    assert!(!r.kept_original);
    assert!(
        r.size_bytes as usize <= input.len(),
        "{} > {}",
        r.size_bytes,
        input.len()
    );
}

// ---------- Byte target ----------

#[test]
fn big_photo_to_50kb() {
    let input = jpeg(&photo(4000, 3000), 92);
    let o = opts(50 * KB);
    let r = compress(&input, o.clone()).unwrap();
    assert_contract(&r, &o);
    assert!(!r.kept_original);
    assert_eq!(r.original_width, 4000);
    eprintln!(
        "4000x3000 → {}x{} q={:?} {} B, {} attempts, {} ms",
        r.width, r.height, r.quality, r.size_bytes, r.attempts, r.elapsed_ms
    );
}

#[test]
fn many_targets_all_respected() {
    let input = jpeg(&photo(2000, 1500), 90);
    for kb in [5, 10, 20, 50, 100, 200, 300] {
        let o = opts(kb * KB);
        let r = compress(&input, o.clone()).unwrap_or_else(|e| panic!("{kb} KB: {e}"));
        assert_contract(&r, &o);
    }
}

#[test]
fn bigger_target_gives_better_or_equal_result() {
    let input = jpeg(&photo(2000, 1500), 92);
    let small = compress(&input, opts(30 * KB)).unwrap();
    let large = compress(&input, opts(150 * KB)).unwrap();
    let pixels = |r: &CompressResult| r.width as u64 * r.height as u64;
    assert!(pixels(&large) >= pixels(&small));
    assert!(large.size_bytes > small.size_bytes);
}

#[test]
fn noise_needs_downscale_but_still_fits() {
    let input = png_rgb(&noise(1000, 1000));
    let o = opts(20 * KB);
    let r = compress(&input, o.clone()).unwrap();
    assert_contract(&r, &o);
    assert!(r.width < 1000);
}

#[test]
fn impossible_target_returns_cannot_meet_target() {
    let input = png_rgb(&noise(1500, 1500));
    let o = CompressOptions {
        min_dimension: 1500,
        ..opts(KB)
    };
    let e = compress(&input, o).unwrap_err();
    assert_eq!(e.code, ErrorCode::CannotMeetTarget);
}

#[test]
fn smallest_allowed_target_1kb() {
    let input = jpeg(&photo(1200, 900), 90);
    let o = opts(KB);
    match compress(&input, o.clone()) {
        Ok(r) => assert_contract(&r, &o),
        Err(e) => assert_eq!(e.code, ErrorCode::CannotMeetTarget),
    }
}

// ---------- Resolution ----------

#[test]
fn resolution_limit_keeps_aspect_ratio() {
    let input = jpeg(&photo(4000, 3000), 90);
    let o = CompressOptions {
        max_width: Some(1080),
        max_height: Some(1080),
        ..opts(10 * 1024 * KB)
    };
    let r = compress(&input, o.clone()).unwrap();
    assert_contract(&r, &o);
    assert_eq!((r.width, r.height), (1080, 810));
}

#[test]
fn only_width_limit() {
    let input = jpeg(&photo(3000, 4000), 90);
    let o = CompressOptions {
        max_width: Some(600),
        ..opts(10 * 1024 * KB)
    };
    let r = compress(&input, o).unwrap();
    assert_eq!((r.width, r.height), (600, 800));
}

#[test]
fn resolution_and_size_together() {
    let input = jpeg(&photo(4000, 3000), 90);
    let o = CompressOptions {
        max_width: Some(1280),
        max_height: Some(1280),
        ..opts(50 * KB)
    };
    let r = compress(&input, o.clone()).unwrap();
    assert_contract(&r, &o);
}

#[test]
fn small_image_is_never_upscaled() {
    let input = jpeg(&photo(200, 100), 90);
    let o = CompressOptions {
        max_width: Some(4000),
        max_height: Some(4000),
        keep_original_if_fits: false,
        ..opts(500 * KB)
    };
    let r = compress(&input, o).unwrap();
    assert_eq!((r.width, r.height), (200, 100));
}

#[test]
fn oversized_resolution_forces_resize_even_if_bytes_fit() {
    let input = jpeg(&photo(2000, 1000), 30); // small file, big resolution
    let o = CompressOptions {
        max_width: Some(500),
        ..opts(5 * 1024 * KB)
    };
    let r = compress(&input, o).unwrap();
    assert!(!r.kept_original);
    assert_eq!((r.width, r.height), (500, 250));
}

// ---------- EXIF orientation ----------

/// 40x20 image with a red block in the top-left corner.
fn marked() -> image::RgbImage {
    image::RgbImage::from_fn(40, 20, |x, y| {
        if x < 8 && y < 8 {
            Rgb([255, 0, 0])
        } else {
            Rgb([0, 0, 255])
        }
    })
}

#[test]
fn rotated_photo_is_made_upright() {
    // Orientation 6 = viewer must rotate 90° clockwise.
    let input = jpeg_with_exif(&marked(), 95, Some(&exif_orientation(6, b"")));
    let o = opts(500 * KB);
    let r = compress(&input, o.clone()).unwrap();
    assert_contract(&r, &o);
    assert!(
        !r.kept_original,
        "EXIF is stripped so pixels must be rotated"
    );
    assert_eq!((r.width, r.height), (20, 40));
    assert_eq!((r.original_width, r.original_height), (20, 40));
    // After a 90° clockwise turn the red corner is at the top-right.
    let img = decode(&r.bytes).to_rgb8();
    let p = img.get_pixel(17, 2);
    assert!(
        p[0] > 150 && p[2] < 100,
        "expected red at top-right, got {p:?}"
    );
}

#[test]
fn rotated_photo_kept_when_metadata_kept() {
    let input = jpeg_with_exif(&marked(), 95, Some(&exif_orientation(6, b"")));
    let o = CompressOptions {
        strip_metadata: false,
        ..opts(500 * KB)
    };
    let r = compress(&input, o).unwrap();
    assert!(r.kept_original);
    assert_eq!(r.bytes, input);
}

#[test]
fn broken_exif_does_not_fail() {
    let mut exif = exif_orientation(6, b"");
    exif.truncate(12);
    let input = jpeg_with_exif(&photo(100, 80), 90, Some(&exif));
    assert!(compress(&input, opts(500 * KB)).is_ok());
}

// ---------- Formats ----------

#[test]
fn png_with_alpha_to_jpeg_flattens_on_white() {
    let input = png_rgba(&transparent_square(64));
    let r = compress(&input, opts(100 * KB)).unwrap();
    assert_eq!(r.format, OutputFormat::Jpeg);
    assert_eq!(r.original_format, InputFormat::Png);
    let img = decode(&r.bytes).to_rgb8();
    let p = img.get_pixel(5, 32);
    assert!(
        p.0.iter().all(|&c| c > 240),
        "transparent area should be white, got {p:?}"
    );
}

#[test]
fn png_to_png_keeps_alpha() {
    let input = png_rgba(&transparent_square(64));
    let o = CompressOptions {
        format: OutputFormat::Png,
        keep_original_if_fits: false,
        ..opts(100 * KB)
    };
    let r = compress(&input, o).unwrap();
    assert!(decode(&r.bytes).color().has_alpha());
}

#[test]
fn png_passthrough() {
    let input = png_rgb(&photo(64, 64));
    let o = CompressOptions {
        format: OutputFormat::Png,
        ..opts(500 * KB)
    };
    let r = compress(&input, o).unwrap();
    assert!(r.kept_original);
}

#[test]
fn jpeg_input_png_output_is_not_passthrough() {
    let input = jpeg(&photo(64, 64), 90);
    let o = CompressOptions {
        format: OutputFormat::Png,
        ..opts(500 * KB)
    };
    let r = compress(&input, o).unwrap();
    assert!(!r.kept_original);
    assert_eq!(&r.bytes[1..4], b"PNG");
}

#[test]
fn other_input_formats() {
    let img = DynamicImage::ImageRgb8(photo(300, 200));
    for (fmt, expected) in [
        (image::ImageFormat::Gif, InputFormat::Gif),
        (image::ImageFormat::Bmp, InputFormat::Bmp),
        (image::ImageFormat::WebP, InputFormat::WebP),
    ] {
        let input = encode_as(&img, fmt);
        let o = opts(30 * KB);
        let r = compress(&input, o.clone()).unwrap_or_else(|e| panic!("{fmt:?}: {e}"));
        assert_eq!(r.original_format, expected);
        assert_contract(&r, &o);
    }
}

#[test]
fn grayscale_and_16_bit_inputs() {
    let gray = DynamicImage::ImageLuma8(DynamicImage::ImageRgb8(photo(200, 200)).to_luma8());
    let deep = DynamicImage::ImageRgb16(DynamicImage::ImageRgb8(photo(200, 200)).to_rgb16());
    for img in [gray, deep] {
        let input = encode_as(&img, image::ImageFormat::Png);
        let o = CompressOptions {
            keep_original_if_fits: false,
            ..opts(20 * KB)
        };
        let r = compress(&input, o.clone()).unwrap();
        assert_contract(&r, &o);
    }
}

#[test]
fn one_pixel_image() {
    let input = png_rgb(&photo(1, 1));
    let r = compress(&input, opts(KB)).unwrap();
    assert_eq!((r.width, r.height), (1, 1));
}

#[test]
fn extreme_panorama() {
    let input = jpeg(&photo(12000, 300), 85);
    let o = opts(50 * KB);
    let r = compress(&input, o.clone()).unwrap();
    assert_contract(&r, &o);
}

// ---------- Bad input ----------

#[test]
fn invalid_options_are_reported_before_decoding() {
    let e = compress(
        b"garbage",
        CompressOptions {
            max_bytes: 10,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(e.code, ErrorCode::InvalidOptions);
}

#[test]
fn empty_bytes() {
    assert_eq!(
        compress(b"", opts(50 * KB)).unwrap_err().code,
        ErrorCode::EmptyInput
    );
}

#[test]
fn not_an_image() {
    let e = compress(b"%PDF-1.7 this is a pdf", opts(50 * KB)).unwrap_err();
    assert_eq!(e.code, ErrorCode::UnsupportedFormat);
}

#[test]
fn heic_like_header_is_unsupported() {
    let mut heic = vec![0, 0, 0, 0x18];
    heic.extend_from_slice(b"ftypheic\0\0\0\0mif1heic");
    assert_eq!(
        compress(&heic, opts(50 * KB)).unwrap_err().code,
        ErrorCode::UnsupportedFormat
    );
}

#[test]
fn truncated_jpeg_is_corrupt_or_still_valid_output() {
    let full = jpeg(&photo(600, 400), 90);
    for cut in [10, 100, full.len() / 3] {
        match compress(&full[..cut], opts(50 * KB)) {
            Ok(r) => {
                decode(&r.bytes);
            }
            Err(e) => assert!(
                matches!(
                    e.code,
                    ErrorCode::CorruptImage | ErrorCode::UnsupportedFormat
                ),
                "cut={cut}: {e}"
            ),
        }
    }
}

#[test]
fn huge_dimensions_rejected_without_allocating() {
    let e = compress(&png_header_only(29_000, 29_000), opts(50 * KB)).unwrap_err();
    assert_eq!(e.code, ErrorCode::ImageTooLarge);
    let e = compress(&png_header_only(40_000, 10), opts(50 * KB)).unwrap_err();
    assert_eq!(e.code, ErrorCode::ImageTooLarge);
}

#[test]
fn zero_dimension_header_rejected() {
    let e = compress(&png_header_only(0, 100), opts(50 * KB)).unwrap_err();
    assert!(
        matches!(
            e.code,
            ErrorCode::CorruptImage | ErrorCode::UnsupportedFormat
        ),
        "{e}"
    );
}

// ---------- File API ----------

#[test]
fn file_api() {
    let dir = std::env::temp_dir().join("image_guard_file_api");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("photo.jpg");
    std::fs::write(&path, jpeg(&photo(1600, 1200), 90)).unwrap();
    let p = path.to_str().unwrap().to_string();

    let info = image_info_file(p.clone()).unwrap();
    assert_eq!(
        (info.format, info.width, info.height),
        (InputFormat::Jpeg, 1600, 1200)
    );

    let o = opts(40 * KB);
    let r = compress_file(p, o.clone()).unwrap();
    assert_contract(&r, &o);

    assert_eq!(
        compress_file("/nope/x.jpg".into(), o.clone())
            .unwrap_err()
            .code,
        ErrorCode::FileNotFound
    );
    // Invalid options win over a missing file (nothing is read).
    let bad = CompressOptions {
        min_quality: 0,
        ..o
    };
    assert_eq!(
        compress_file("/nope/x.jpg".into(), bad).unwrap_err().code,
        ErrorCode::InvalidOptions
    );
}

#[test]
fn image_info_reports_rotation() {
    let input = jpeg_with_exif(&marked(), 90, Some(&exif_orientation(8, b"")));
    let info = image_info_bytes(input).unwrap();
    assert!(info.has_orientation);
    assert_eq!((info.width, info.height), (20, 40));
}

// ---------- Concurrency ----------

#[test]
fn parallel_calls_are_safe() {
    let input = std::sync::Arc::new(jpeg(&photo(1500, 1000), 90));
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let input = input.clone();
            std::thread::spawn(move || {
                let o = opts((20 + i * 10) * KB);
                let r = compress_bytes(input.to_vec(), o.clone()).unwrap();
                assert_contract(&r, &o);
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}
