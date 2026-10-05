//! Separate test binary: the model is never loaded here.

use image_guard::api::safety::{classify_bytes, is_model_loaded, load_model, process_bytes};
use image_guard::api::types::*;

#[test]
fn calls_fail_cleanly_without_model() {
    assert!(!is_model_loaded());
    let e = classify_bytes(vec![1, 2, 3], SafetyOptions::default()).unwrap_err();
    assert_eq!(e.code, ErrorCode::ModelNotLoaded);
    let e = process_bytes(
        vec![1, 2, 3],
        CompressOptions::default(),
        SafetyOptions::default(),
    )
    .unwrap_err();
    assert_eq!(e.code, ErrorCode::ModelNotLoaded);

    let e = load_model(b"not an onnx model".to_vec()).unwrap_err();
    assert_eq!(e.code, ErrorCode::ModelLoadFailed);
    assert!(
        !is_model_loaded(),
        "a failed load must not leave a broken model behind"
    );
}
