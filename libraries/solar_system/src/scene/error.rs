use thiserror::Error;

#[derive(Debug, Error)]
pub enum SceneLoadError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Camera target not found: {0:?}")]
    CameraTargetNotFound(String),
}
