//! Hard limits. They protect the app from out-of-memory crashes and from
//! options that can never produce a useful image.

/// Smallest `max_bytes` a caller may ask for (1 KB).
pub const MIN_TARGET_BYTES: u32 = 1024;
/// Largest `max_bytes` a caller may ask for (100 MB).
pub const MAX_TARGET_BYTES: u32 = 100 * 1024 * 1024;

/// Inputs bigger than this are rejected before being read (100 MB).
pub const MAX_INPUT_BYTES: u64 = 100 * 1024 * 1024;

/// Allowed range for `max_width` / `max_height`.
pub const MAX_OPTION_DIMENSION: u32 = 16_384;
/// Allowed range for `min_dimension`.
pub const MAX_MIN_DIMENSION: u32 = 4_096;

/// Decoder limits: longest side, total pixels (100 MP) and memory.
pub const MAX_DECODE_SIDE: u32 = 30_000;
pub const MAX_DECODE_PIXELS: u64 = 100_000_000;
pub const MAX_DECODE_ALLOC: u64 = 512 * 1024 * 1024;
