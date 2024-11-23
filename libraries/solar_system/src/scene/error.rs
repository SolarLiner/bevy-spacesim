use std::ops;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SceneLoadError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    ParseError(#[from] serde_yaml::Error),
    #[error("Camera target not found: {0:?}")]
    CameraTargetNotFound(String),
}

#[derive(Debug, Error)]
pub enum DurationFromStrError {
    #[error("Failed to parse duration string: {:?}", &.0[.1.clone()])]
    MalformedString(String, ops::Range<usize>),
}