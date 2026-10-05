//! Rust core of the `image_guard` Flutter package.
//!
//! * [`api`] — the only module exposed to Dart (via flutter_rust_bridge).
//! * [`core`] — decode / resize / encode / target-size search, pure Rust and
//!   unit-testable without Flutter.

pub mod api;
pub mod core;
mod frb_generated;
