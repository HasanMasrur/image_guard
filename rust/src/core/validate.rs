use crate::api::types::{CompressOptions, ErrorCode, ImageGuardError, SafetyOptions};
use crate::core::limits::*;

fn invalid(message: String) -> ImageGuardError {
    ImageGuardError::new(ErrorCode::InvalidOptions, message)
}

/// Rejects options that are out of range or contradict each other.
pub fn validate_options(o: &CompressOptions) -> Result<(), ImageGuardError> {
    if o.max_bytes < MIN_TARGET_BYTES || o.max_bytes > MAX_TARGET_BYTES {
        return Err(invalid(format!(
            "maxBytes must be between {} and {} bytes (1 KB – 100 MB), got {}",
            MIN_TARGET_BYTES, MAX_TARGET_BYTES, o.max_bytes
        )));
    }
    for (name, q) in [("minQuality", o.min_quality), ("maxQuality", o.max_quality)] {
        if !(1..=100).contains(&q) {
            return Err(invalid(format!(
                "{name} must be between 1 and 100, got {q}"
            )));
        }
    }
    if o.min_quality > o.max_quality {
        return Err(invalid(format!(
            "minQuality ({}) must not be greater than maxQuality ({})",
            o.min_quality, o.max_quality
        )));
    }
    if o.min_dimension == 0 || o.min_dimension > MAX_MIN_DIMENSION {
        return Err(invalid(format!(
            "minDimension must be between 1 and {MAX_MIN_DIMENSION}, got {}",
            o.min_dimension
        )));
    }
    for (name, v) in [("maxWidth", o.max_width), ("maxHeight", o.max_height)] {
        if let Some(v) = v {
            if v == 0 || v > MAX_OPTION_DIMENSION {
                return Err(invalid(format!(
                    "{name} must be between 1 and {MAX_OPTION_DIMENSION}, got {v}"
                )));
            }
            if v < o.min_dimension {
                return Err(invalid(format!(
                    "{name} ({v}) must not be smaller than minDimension ({})",
                    o.min_dimension
                )));
            }
        }
    }
    Ok(())
}

pub fn validate_safety_options(o: &SafetyOptions) -> Result<(), ImageGuardError> {
    for (name, v) in [
        ("rejectThreshold", o.reject_threshold),
        ("reviewThreshold", o.review_threshold),
        ("suggestiveWeight", o.suggestive_weight),
    ] {
        if !(0.0..=1.0).contains(&v) {
            return Err(invalid(format!(
                "{name} must be between 0.0 and 1.0, got {v}"
            )));
        }
    }
    if o.review_threshold > o.reject_threshold {
        return Err(invalid(format!(
            "reviewThreshold ({}) must not be greater than rejectThreshold ({})",
            o.review_threshold, o.reject_threshold
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_defaults_valid() {
        assert!(validate_safety_options(&SafetyOptions::default()).is_ok());
    }

    #[test]
    fn safety_ranges() {
        let base = SafetyOptions::default();
        for bad in [
            SafetyOptions {
                reject_threshold: 1.1,
                ..base.clone()
            },
            SafetyOptions {
                review_threshold: -0.1,
                ..base.clone()
            },
            SafetyOptions {
                suggestive_weight: 2.0,
                ..base.clone()
            },
            SafetyOptions {
                reject_threshold: f32::NAN,
                ..base.clone()
            },
            SafetyOptions {
                review_threshold: 0.8,
                reject_threshold: 0.5,
                ..base.clone()
            },
        ] {
            assert_eq!(
                validate_safety_options(&bad).unwrap_err().code,
                ErrorCode::InvalidOptions,
                "{bad:?}"
            );
        }
        assert!(validate_safety_options(&SafetyOptions {
            review_threshold: 0.5,
            reject_threshold: 0.5,
            ..base
        })
        .is_ok());
    }

    fn err_code(o: CompressOptions) -> Option<ErrorCode> {
        validate_options(&o).err().map(|e| e.code)
    }

    #[test]
    fn defaults_are_valid() {
        assert!(validate_options(&CompressOptions::default()).is_ok());
    }

    #[test]
    fn max_bytes_bounds() {
        let base = CompressOptions::default();
        assert_eq!(
            err_code(CompressOptions {
                max_bytes: 0,
                ..base.clone()
            }),
            Some(ErrorCode::InvalidOptions)
        );
        assert_eq!(
            err_code(CompressOptions {
                max_bytes: 1023,
                ..base.clone()
            }),
            Some(ErrorCode::InvalidOptions)
        );
        assert_eq!(
            err_code(CompressOptions {
                max_bytes: 1024,
                ..base.clone()
            }),
            None
        );
        assert_eq!(
            err_code(CompressOptions {
                max_bytes: MAX_TARGET_BYTES,
                ..base.clone()
            }),
            None
        );
        assert_eq!(
            err_code(CompressOptions {
                max_bytes: MAX_TARGET_BYTES + 1,
                ..base
            }),
            Some(ErrorCode::InvalidOptions)
        );
    }

    #[test]
    fn quality_bounds_and_order() {
        let base = CompressOptions::default();
        assert!(err_code(CompressOptions {
            min_quality: 0,
            ..base.clone()
        })
        .is_some());
        assert!(err_code(CompressOptions {
            max_quality: 101,
            ..base.clone()
        })
        .is_some());
        assert!(err_code(CompressOptions {
            min_quality: 80,
            max_quality: 70,
            ..base.clone()
        })
        .is_some());
        assert!(err_code(CompressOptions {
            min_quality: 75,
            max_quality: 75,
            ..base.clone()
        })
        .is_none());
        assert!(err_code(CompressOptions {
            min_quality: 1,
            max_quality: 100,
            ..base
        })
        .is_none());
    }

    #[test]
    fn dimension_bounds() {
        let base = CompressOptions::default();
        assert!(err_code(CompressOptions {
            max_width: Some(0),
            ..base.clone()
        })
        .is_some());
        assert!(err_code(CompressOptions {
            max_height: Some(MAX_OPTION_DIMENSION + 1),
            ..base.clone()
        })
        .is_some());
        assert!(err_code(CompressOptions {
            min_dimension: 0,
            ..base.clone()
        })
        .is_some());
        assert!(err_code(CompressOptions {
            min_dimension: MAX_MIN_DIMENSION + 1,
            ..base.clone()
        })
        .is_some());
        // maxWidth smaller than minDimension is contradictory.
        assert!(err_code(CompressOptions {
            max_width: Some(100),
            min_dimension: 200,
            ..base.clone()
        })
        .is_some());
        assert!(err_code(CompressOptions {
            max_width: Some(1080),
            max_height: Some(1080),
            ..base
        })
        .is_none());
    }

    #[test]
    fn message_names_the_field() {
        let e = validate_options(&CompressOptions {
            max_quality: 0,
            ..Default::default()
        })
        .unwrap_err();
        assert!(e.message.contains("maxQuality"), "{}", e.message);
    }
}
