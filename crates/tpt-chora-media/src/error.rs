#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("image decode failed: {0}")]
    ImageDecode(String),
    #[error("unsupported image format")]
    UnsupportedFormat,
    #[error("texture cache full")]
    TextureCacheFull,
    #[error("asset stream error: {0}")]
    StreamError(String),
    #[error("video decode not available on this platform")]
    VideoDecodeUnavailable,
    #[error("GPU texture creation failed: {0}")]
    TextureCreation(String),
}
