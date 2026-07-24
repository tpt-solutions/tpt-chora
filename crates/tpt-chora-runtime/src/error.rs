#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("GPU buffer binding failed")]
    BufferBindingFailed,
    #[error("archon page not found: {0}")]
    PageNotFound(u64),
    #[error("telos state transition failed: {0}")]
    TelosTransitionFailed(String),
    #[error("eidos proof verification failed")]
    EidosProofFailed,
    #[error("zero-copy mapping not supported for this data type")]
    ZeroCopyUnsupported,
}
