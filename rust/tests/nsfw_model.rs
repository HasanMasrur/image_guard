//! Tests with the real bundled model (assets/models/*.onnx).

mod common;

use std::path::PathBuf;

use common::*;
use image_guard::api::safety::{classify_bytes, is_model_loaded, load_model, process_bytes};
use image_guard::api::types::*;
use image_guard::core::model_store;

fn model_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../assets/models/nsfw_mobilenet_v2_140_224.nnef.tar")
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn ensure_model() {
    if !is_model_loaded() {
        load_model(std::fs::read(model_path()).expect("run tool/convert_model.py first")).unwrap();
    }
}

#[test]
fn matches_original_tflite_model() {
    ensure_model();
    let reference: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixtures().join("reference.json")).unwrap())
            .unwrap();
    let model = model_store::get().unwrap();
    let mut worst = 0f32;
    for (name, expected) in reference.as_object().unwrap() {
        let img = image::open(fixtures().join(format!("{name}.png"))).unwrap();
        assert_eq!(
            (img.width(), img.height()),
            (224, 224),
            "fixture must not need resizing"
        );
        let s = model.scores(&img).unwrap();
        for (label, got) in [
            ("drawings", s.drawings),
            ("hentai", s.hentai),
            ("neutral", s.neutral),
            ("porn", s.porn),
            ("sexy", s.sexy),
        ] {
            let want = expected[label].as_f64().unwrap() as f32;
            worst = worst.max((want - got).abs());
            assert!(
                (want - got).abs() < 2e-3,
                "{name}/{label}: tflite {want} vs rust {got}"
            );
        }
    }
    eprintln!("max diff vs TFLite: {worst:e}");
}

#[test]
fn probabilities_sum_to_one() {
    ensure_model();
    let r = classify_bytes(jpeg(&photo(640, 480), 90), SafetyOptions::default()).unwrap();
    let s = r.scores;
    let sum = s.drawings + s.hentai + s.neutral + s.porn + s.sexy;
    assert!((sum - 1.0).abs() < 1e-3, "sum {sum}");
    assert!((0.0..=1.0).contains(&r.nsfw_score));
}

#[test]
fn plain_images_are_not_unsafe() {
    ensure_model();
    for (name, bytes) in [
        ("photo", jpeg(&photo(800, 600), 90)),
        (
            "white",
            png_rgb(&image::RgbImage::from_pixel(
                300,
                300,
                image::Rgb([255, 255, 255]),
            )),
        ),
        ("transparent", png_rgba(&transparent_square(128))),
    ] {
        let r = classify_bytes(bytes, SafetyOptions::default()).unwrap();
        assert_ne!(r.verdict, Verdict::Unsafe, "{name}: {r:?}");
    }
}

#[test]
fn rotation_does_not_change_much() {
    // EXIF-rotated copy of the same picture is classified upright.
    ensure_model();
    let img = photo(400, 300);
    let plain = classify_bytes(jpeg(&img, 95), SafetyOptions::default()).unwrap();
    let rotated_pixels = image::imageops::rotate270(&img);
    let tagged = jpeg_with_exif(&rotated_pixels, 95, Some(&exif_orientation(6, b"")));
    let rotated = classify_bytes(tagged, SafetyOptions::default()).unwrap();
    assert!(
        (plain.nsfw_score - rotated.nsfw_score).abs() < 0.05,
        "{plain:?} vs {rotated:?}"
    );
}

#[test]
fn process_compresses_safe_and_skips_unsafe() {
    ensure_model();
    let input = jpeg(&photo(2000, 1500), 90);
    let o = CompressOptions {
        max_bytes: 50 * KB,
        ..Default::default()
    };

    let lenient = SafetyOptions {
        reject_threshold: 1.0,
        review_threshold: 1.0,
        ..Default::default()
    };
    let r = process_bytes(input.clone(), o.clone(), lenient).unwrap();
    assert_eq!(r.safety.verdict, Verdict::Safe);
    assert!(r.image.unwrap().size_bytes <= 50 * KB);

    let everything_unsafe = SafetyOptions {
        reject_threshold: 0.0,
        review_threshold: 0.0,
        ..Default::default()
    };
    let r = process_bytes(input.clone(), o.clone(), everything_unsafe.clone()).unwrap();
    assert_eq!(r.safety.verdict, Verdict::Unsafe);
    assert!(r.image.is_none());

    let r = process_bytes(
        input,
        o,
        SafetyOptions {
            compress_unsafe: true,
            ..everything_unsafe
        },
    )
    .unwrap();
    assert!(r.image.is_some());
}

#[test]
fn process_validates_everything() {
    ensure_model();
    let bad_safety = SafetyOptions {
        reject_threshold: 3.0,
        ..Default::default()
    };
    let e = process_bytes(
        jpeg(&photo(10, 10), 90),
        CompressOptions::default(),
        bad_safety,
    )
    .unwrap_err();
    assert_eq!(e.code, ErrorCode::InvalidOptions);
    let bad_opts = CompressOptions {
        max_bytes: 1,
        ..Default::default()
    };
    let e =
        process_bytes(jpeg(&photo(10, 10), 90), bad_opts, SafetyOptions::default()).unwrap_err();
    assert_eq!(e.code, ErrorCode::InvalidOptions);
    let e = process_bytes(
        b"nope".to_vec(),
        CompressOptions::default(),
        SafetyOptions::default(),
    )
    .unwrap_err();
    assert_eq!(e.code, ErrorCode::UnsupportedFormat);
}

#[test]
fn parallel_inference_is_consistent() {
    ensure_model();
    let input = std::sync::Arc::new(jpeg(&photo(500, 400), 90));
    let scores: Vec<f32> = (0..8)
        .map(|_| {
            let input = input.clone();
            std::thread::spawn(move || {
                classify_bytes(input.to_vec(), SafetyOptions::default())
                    .unwrap()
                    .nsfw_score
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect();
    assert!(
        scores.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-6),
        "{scores:?}"
    );
}

#[test]
fn inference_speed() {
    ensure_model();
    let input = jpeg(&photo(4000, 3000), 90);
    let t = std::time::Instant::now();
    let r = process_bytes(
        input,
        CompressOptions::default(),
        SafetyOptions {
            reject_threshold: 1.0,
            review_threshold: 1.0,
            ..Default::default()
        },
    )
    .unwrap();
    eprintln!(
        "12 MP process: total {} ms, inference {} ms",
        t.elapsed().as_millis(),
        r.safety.elapsed_ms
    );
}
