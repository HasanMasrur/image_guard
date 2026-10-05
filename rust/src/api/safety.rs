//! NSFW detection functions callable from Dart.

use crate::api::types::{
    CompressOptions, ImageGuardError, ProcessResult, SafetyOptions, SafetyReport,
};
use crate::core::{io, model_store, pipeline, validate};

/// Loads (or replaces) the ONNX model. Takes ~100–300 ms; call once.
pub fn load_model(model: Vec<u8>) -> Result<(), ImageGuardError> {
    model_store::load(&model)
}

#[flutter_rust_bridge::frb(sync)]
pub fn is_model_loaded() -> bool {
    model_store::is_loaded()
}

/// Frees the model memory. `load_model` must be called again before use.
#[flutter_rust_bridge::frb(sync)]
pub fn unload_model() {
    model_store::unload()
}

pub fn classify_file(path: String, safety: SafetyOptions) -> Result<SafetyReport, ImageGuardError> {
    validate::validate_safety_options(&safety)?;
    let bytes = io::read_file(&path)?;
    pipeline::classify(&bytes, &safety)
}

pub fn classify_bytes(
    bytes: Vec<u8>,
    safety: SafetyOptions,
) -> Result<SafetyReport, ImageGuardError> {
    pipeline::classify(&bytes, &safety)
}

pub fn process_file(
    path: String,
    options: CompressOptions,
    safety: SafetyOptions,
) -> Result<ProcessResult, ImageGuardError> {
    validate::validate_options(&options)?;
    validate::validate_safety_options(&safety)?;
    let bytes = io::read_file(&path)?;
    pipeline::process(&bytes, &options, &safety)
}

pub fn process_bytes(
    bytes: Vec<u8>,
    options: CompressOptions,
    safety: SafetyOptions,
) -> Result<ProcessResult, ImageGuardError> {
    pipeline::process(&bytes, &options, &safety)
}
