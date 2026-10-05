//! NSFW classifier: MobileNetV2 (GantMan nsfw_model, MIT) run with `tract`,
//! a pure-Rust inference engine, so no native ML library has to be linked.
//! The model ships in tract's NNEF format (converted from ONNX at dev time by
//! `examples/onnx_to_nnef.rs`), so the app does not carry an ONNX parser.
//!
//! Input: 1×224×224×3 RGB, values 0–1 (Keras `rescale=1/255`).
//! Output: softmax over [drawings, hentai, neutral, porn, sexy].

use std::io::Cursor;
use std::time::Instant;

use image::DynamicImage;
use tract_nnef::prelude::*;

use crate::api::types::{
    ErrorCode, ImageGuardError, SafetyOptions, SafetyReport, SafetyScores, Verdict,
};
use crate::core::{decode, resize};

pub const INPUT_SIZE: u32 = 224;
pub const CLASS_COUNT: usize = 5;

pub struct Classifier {
    plan: Arc<TypedRunnableModel>,
}

fn load_err(e: impl std::fmt::Display) -> ImageGuardError {
    ImageGuardError::new(
        ErrorCode::ModelLoadFailed,
        format!("Cannot load NSFW model: {e}"),
    )
}

impl Classifier {
    pub fn load(model: &[u8]) -> Result<Self, ImageGuardError> {
        let size = INPUT_SIZE as usize;
        let plan = tract_nnef::nnef()
            .model_for_read(&mut Cursor::new(model))
            .map_err(load_err)?
            .into_optimized()
            .map_err(load_err)?
            .into_runnable()
            .map_err(load_err)?;
        let classifier = Self { plan };
        // Fail now (not on the first photo) if the output is not 5 classes.
        classifier
            .run(vec![0.5; size * size * 3])
            .map_err(load_err)?;
        Ok(classifier)
    }

    pub fn scores(&self, img: &DynamicImage) -> Result<SafetyScores, ImageGuardError> {
        let input = preprocess(img)?;
        let p = self.run(input)?;
        Ok(SafetyScores {
            drawings: p[0],
            hentai: p[1],
            neutral: p[2],
            porn: p[3],
            sexy: p[4],
        })
    }

    fn run(&self, input: Vec<f32>) -> Result<[f32; CLASS_COUNT], ImageGuardError> {
        let internal = |e: TractError| {
            ImageGuardError::new(ErrorCode::Internal, format!("Inference failed: {e}"))
        };
        let size = INPUT_SIZE as usize;
        let tensor = Tensor::from_shape(&[1, size, size, 3], &input).map_err(internal)?;
        let out = self.plan.run(tvec!(tensor.into())).map_err(internal)?;
        let view = out[0].to_plain_array_view::<f32>().map_err(internal)?;
        let values: Vec<f32> = view.iter().copied().collect();
        if values.len() != CLASS_COUNT {
            return Err(ImageGuardError::new(
                ErrorCode::ModelLoadFailed,
                format!("Model has {} outputs, expected {CLASS_COUNT}", values.len()),
            ));
        }
        Ok([values[0], values[1], values[2], values[3], values[4]])
    }

    pub fn classify(
        &self,
        img: &DynamicImage,
        o: &SafetyOptions,
    ) -> Result<SafetyReport, ImageGuardError> {
        let started = Instant::now();
        let scores = self.scores(img)?;
        Ok(report(scores, o, started.elapsed().as_millis() as u32))
    }
}

/// Squash to 224×224 (the model was trained without keeping aspect ratio),
/// flatten transparency onto white, scale to 0–1, NHWC order.
pub fn preprocess(img: &DynamicImage) -> Result<Vec<f32>, ImageGuardError> {
    let rgb = if img.as_rgb8().is_some() {
        std::borrow::Cow::Borrowed(img)
    } else {
        std::borrow::Cow::Owned(decode::normalize(img.clone(), false))
    };
    let small = resize::resize(&rgb, INPUT_SIZE, INPUT_SIZE)?;
    let small = small.as_rgb8().expect("normalized to RGB8");
    Ok(small.as_raw().iter().map(|&v| v as f32 / 255.0).collect())
}

pub fn nsfw_score(s: &SafetyScores, o: &SafetyOptions) -> f32 {
    (s.porn + s.hentai + s.sexy * o.suggestive_weight).clamp(0.0, 1.0)
}

pub fn report(scores: SafetyScores, o: &SafetyOptions, elapsed_ms: u32) -> SafetyReport {
    let nsfw = nsfw_score(&scores, o);
    let verdict = if nsfw >= o.reject_threshold {
        Verdict::Unsafe
    } else if nsfw >= o.review_threshold {
        Verdict::Uncertain
    } else {
        Verdict::Safe
    };
    SafetyReport {
        verdict,
        nsfw_score: nsfw,
        scores,
        elapsed_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scores(porn: f32, hentai: f32, sexy: f32) -> SafetyScores {
        let neutral = (1.0 - porn - hentai - sexy).max(0.0);
        SafetyScores {
            drawings: 0.0,
            hentai,
            neutral,
            porn,
            sexy,
        }
    }

    #[test]
    fn verdict_thresholds() {
        let o = SafetyOptions::default();
        assert_eq!(report(scores(0.0, 0.0, 0.0), &o, 0).verdict, Verdict::Safe);
        assert_eq!(report(scores(0.29, 0.0, 0.0), &o, 0).verdict, Verdict::Safe);
        assert_eq!(
            report(scores(0.30, 0.0, 0.0), &o, 0).verdict,
            Verdict::Uncertain
        );
        assert_eq!(
            report(scores(0.69, 0.0, 0.0), &o, 0).verdict,
            Verdict::Uncertain
        );
        assert_eq!(
            report(scores(0.70, 0.0, 0.0), &o, 0).verdict,
            Verdict::Unsafe
        );
        assert_eq!(
            report(scores(0.0, 0.95, 0.0), &o, 0).verdict,
            Verdict::Unsafe
        );
    }

    #[test]
    fn suggestive_weight() {
        let s = scores(0.0, 0.0, 0.9);
        let half = SafetyOptions::default();
        assert!((nsfw_score(&s, &half) - 0.45).abs() < 1e-6);
        let full = SafetyOptions {
            suggestive_weight: 1.0,
            ..Default::default()
        };
        assert_eq!(report(s, &full, 0).verdict, Verdict::Unsafe);
        let none = SafetyOptions {
            suggestive_weight: 0.0,
            ..Default::default()
        };
        assert_eq!(report(s, &none, 0).verdict, Verdict::Safe);
    }

    #[test]
    fn score_is_clamped() {
        let s = scores(0.9, 0.9, 0.9);
        assert_eq!(nsfw_score(&s, &SafetyOptions::default()), 1.0);
    }

    #[test]
    fn preprocess_shape_and_range() {
        let img = DynamicImage::new_rgba8(500, 300);
        let v = preprocess(&img).unwrap();
        assert_eq!(v.len(), 224 * 224 * 3);
        // Transparent → white → 1.0
        assert!(v.iter().all(|&x| (x - 1.0).abs() < 1e-6));
    }

    #[test]
    fn garbage_model_fails_cleanly() {
        let e = Classifier::load(b"definitely not onnx").err().unwrap();
        assert_eq!(e.code, ErrorCode::ModelLoadFailed);
    }
}
