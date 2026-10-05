use std::fs;
use std::io::ErrorKind;

use crate::api::types::{ErrorCode, ImageGuardError};
use crate::core::limits::MAX_INPUT_BYTES;

/// Reads a file after checking it exists, is a regular file and is not too big.
pub fn read_file(path: &str) -> Result<Vec<u8>, ImageGuardError> {
    if path.trim().is_empty() {
        return Err(ImageGuardError::new(
            ErrorCode::FileNotFound,
            "File path is empty",
        ));
    }
    let meta = fs::metadata(path).map_err(|e| map_io(e, path))?;
    if !meta.is_file() {
        return Err(ImageGuardError::new(
            ErrorCode::FileReadFailed,
            format!("Not a regular file: {path}"),
        ));
    }
    check_input_len(meta.len())?;
    let bytes = fs::read(path).map_err(|e| map_io(e, path))?;
    check_input_len(bytes.len() as u64)?;
    Ok(bytes)
}

pub fn check_input_len(len: u64) -> Result<(), ImageGuardError> {
    if len == 0 {
        return Err(ImageGuardError::new(
            ErrorCode::EmptyInput,
            "Input is empty (0 bytes)",
        ));
    }
    if len > MAX_INPUT_BYTES {
        return Err(ImageGuardError::new(
            ErrorCode::InputTooLarge,
            format!("Input is {len} bytes; the limit is {MAX_INPUT_BYTES} bytes (100 MB)"),
        ));
    }
    Ok(())
}

fn map_io(e: std::io::Error, path: &str) -> ImageGuardError {
    match e.kind() {
        ErrorKind::NotFound => {
            ImageGuardError::new(ErrorCode::FileNotFound, format!("File not found: {path}"))
        }
        _ => ImageGuardError::new(
            ErrorCode::FileReadFailed,
            format!("Cannot read {path}: {e}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file() {
        assert_eq!(
            read_file("/definitely/not/here.jpg").unwrap_err().code,
            ErrorCode::FileNotFound
        );
    }

    #[test]
    fn empty_path() {
        assert_eq!(read_file("  ").unwrap_err().code, ErrorCode::FileNotFound);
    }

    #[test]
    fn directory_is_rejected() {
        let dir = std::env::temp_dir();
        assert_eq!(
            read_file(dir.to_str().unwrap()).unwrap_err().code,
            ErrorCode::FileReadFailed
        );
    }

    #[test]
    fn empty_file() {
        let p = std::env::temp_dir().join("image_guard_empty_test.jpg");
        fs::write(&p, b"").unwrap();
        assert_eq!(
            read_file(p.to_str().unwrap()).unwrap_err().code,
            ErrorCode::EmptyInput
        );
        let _ = fs::remove_file(p);
    }

    #[test]
    fn length_limits() {
        assert!(check_input_len(1).is_ok());
        assert!(check_input_len(MAX_INPUT_BYTES).is_ok());
        assert_eq!(
            check_input_len(MAX_INPUT_BYTES + 1).unwrap_err().code,
            ErrorCode::InputTooLarge
        );
    }
}
