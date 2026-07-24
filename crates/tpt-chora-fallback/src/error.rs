#[derive(Debug, thiserror::Error)]
pub enum FallbackError {
    #[error("headless renderer initialization failed: {0}")]
    InitFailed(String),
    #[error("frame encode failed: {0}")]
    EncodeFailed(String),
    #[error("no suitable fidelity profile for hardware")]
    NoFidelityProfile,
}
