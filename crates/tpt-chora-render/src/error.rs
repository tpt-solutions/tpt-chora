#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("render graph has a cyclic resource dependency")]
    GraphCycle,
    #[error("no compatible GPU adapter found (wgpu could not find Vulkan/Metal/DX12/GL)")]
    NoAdapter,
    #[error("failed to request GPU device: {0}")]
    DeviceRequest(#[from] wgpu::RequestDeviceError),
    #[error("failed to map readback buffer: {0}")]
    BufferAsync(#[from] wgpu::BufferAsyncError),
}
