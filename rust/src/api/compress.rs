//! Functions callable from Dart. Each one is a thin wrapper around `core`.
//! Non-`sync` functions run on a Rust worker thread, so they never block the
//! Flutter UI isolate.

use crate::api::types::{CompressOptions, CompressResult, ImageGuardError, ImageInfo};
use crate::core::{io, pipeline, validate};

pub fn compress_file(
    path: String,
    options: CompressOptions,
) -> Result<CompressResult, ImageGuardError> {
    validate::validate_options(&options)?;
    let bytes = io::read_file(&path)?;
    pipeline::compress(&bytes, &options)
}

pub fn compress_bytes(
    bytes: Vec<u8>,
    options: CompressOptions,
) -> Result<CompressResult, ImageGuardError> {
    pipeline::compress(&bytes, &options)
}

pub fn image_info_file(path: String) -> Result<ImageInfo, ImageGuardError> {
    let bytes = io::read_file(&path)?;
    pipeline::image_info(&bytes)
}

pub fn image_info_bytes(bytes: Vec<u8>) -> Result<ImageInfo, ImageGuardError> {
    pipeline::image_info(&bytes)
}

/// Cheap check so Dart can reject bad options before reading any file.
#[flutter_rust_bridge::frb(sync)]
pub fn validate_compress_options(options: CompressOptions) -> Result<(), ImageGuardError> {
    validate::validate_options(&options)
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
