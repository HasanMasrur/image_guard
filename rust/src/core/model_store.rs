//! The loaded model, shared by every thread. Loading is done once; inference
//! only needs a read lock, so many images can be classified in parallel.

use std::sync::{Arc, RwLock};

use crate::api::types::{ErrorCode, ImageGuardError};
use crate::core::classifier::Classifier;

static MODEL: RwLock<Option<Arc<Classifier>>> = RwLock::new(None);

pub fn load(model: &[u8]) -> Result<(), ImageGuardError> {
    let classifier = Arc::new(Classifier::load(model)?);
    *MODEL.write().unwrap_or_else(|e| e.into_inner()) = Some(classifier);
    Ok(())
}

pub fn get() -> Result<Arc<Classifier>, ImageGuardError> {
    MODEL
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .ok_or_else(|| {
            ImageGuardError::new(
                ErrorCode::ModelNotLoaded,
                "NSFW model is not loaded. Call SafeImage.initialize() first.",
            )
        })
}

pub fn is_loaded() -> bool {
    MODEL.read().unwrap_or_else(|e| e.into_inner()).is_some()
}

pub fn unload() {
    *MODEL.write().unwrap_or_else(|e| e.into_inner()) = None;
}
