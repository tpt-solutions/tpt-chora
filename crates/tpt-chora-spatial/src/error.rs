#[derive(Debug, thiserror::Error)]
pub enum SpatialError {
    #[error("no GPU adapter found for stereoscopic rendering")]
    NoAdapter,
    #[error("device request failed: {0}")]
    DeviceRequest(#[from] wgpu::RequestDeviceError),
    #[error("volumetric pipeline creation failed: {0}")]
    PipelineCreation(String),
    #[error("spatial audio initialization failed: {0}")]
    AudioInit(String),
    #[error("foveated rendering requires eye-tracking data")]
    NoEyeTracking,
}
