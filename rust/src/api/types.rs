//! Types shared between Rust and Dart. flutter_rust_bridge generates a Dart
//! class for every type in this file.

/// Output encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Lossy. Size is controlled by quality first, then by resolution.
    Jpeg,
    /// Lossless. Size is controlled by resolution only (quality is ignored).
    Png,
}

/// Format detected from the input bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Jpeg,
    Png,
    WebP,
    Gif,
    Bmp,
}

/// Everything the caller can tune. All values are validated by
/// [`crate::core::validate::validate_options`] before any work is done.
#[derive(Debug, Clone, PartialEq)]
pub struct CompressOptions {
    /// Hard upper limit for the output size in bytes.
    pub max_bytes: u32,
    /// Output will be at most this wide (aspect ratio is kept, never upscaled).
    pub max_width: Option<u32>,
    /// Output will be at most this tall (aspect ratio is kept, never upscaled).
    pub max_height: Option<u32>,
    pub format: OutputFormat,
    /// Lowest JPEG quality the search may use (1–100).
    pub min_quality: u8,
    /// Highest JPEG quality the search may use (1–100).
    pub max_quality: u8,
    /// The search never shrinks the longest side below this many pixels;
    /// if the target still cannot be met, `CannotMeetTarget` is returned.
    pub min_dimension: u32,
    /// If the input already satisfies every limit (size, resolution, format)
    /// it is returned as-is instead of being re-encoded.
    pub keep_original_if_fits: bool,
    /// Remove EXIF/XMP/IPTC/text metadata (e.g. GPS location).
    pub strip_metadata: bool,
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self {
            max_bytes: 50 * 1024,
            max_width: None,
            max_height: None,
            format: OutputFormat::Jpeg,
            min_quality: 40,
            max_quality: 90,
            min_dimension: 64,
            keep_original_if_fits: true,
            strip_metadata: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompressResult {
    pub bytes: Vec<u8>,
    pub size_bytes: u32,
    pub width: u32,
    pub height: u32,
    pub format: OutputFormat,
    /// JPEG quality that was used; `None` for PNG or when the original was kept.
    pub quality: Option<u8>,
    pub original_size_bytes: u32,
    /// Width/height of the input after EXIF orientation was applied.
    pub original_width: u32,
    pub original_height: u32,
    pub original_format: InputFormat,
    /// `true` when the input already fit and was returned without re-encoding.
    pub kept_original: bool,
    /// Number of encode attempts the size search needed.
    pub attempts: u32,
    pub elapsed_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageInfo {
    pub format: InputFormat,
    /// Width/height after EXIF orientation.
    pub width: u32,
    pub height: u32,
    pub size_bytes: u32,
    /// `true` when the image has an EXIF rotation/flip that viewers apply.
    pub has_orientation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidOptions,
    EmptyInput,
    FileNotFound,
    FileReadFailed,
    InputTooLarge,
    UnsupportedFormat,
    CorruptImage,
    ImageTooLarge,
    CannotMeetTarget,
    EncodingFailed,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code:?}: {message}")]
pub struct ImageGuardError {
    pub code: ErrorCode,
    pub message: String,
}

impl ImageGuardError {
    pub(crate) fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
