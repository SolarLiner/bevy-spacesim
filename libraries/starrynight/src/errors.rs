use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpectralTypeFromStrError {
    #[error("Empty string")]
    Empty,
    #[error("Unknown spectral type: {0:?}")]
    UnknownType(char),
}
