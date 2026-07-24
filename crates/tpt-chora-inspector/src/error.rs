#[derive(Debug, thiserror::Error)]
pub enum InspectorError {
    #[error("GPU timer query not supported")]
    TimerQueryUnsupported,
    #[error("hot reload watch failed: {0}")]
    WatchFailed(String),
    #[error("inspector overlay render failed: {0}")]
    OverlayRenderFailed(String),
}
