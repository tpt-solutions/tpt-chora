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
    #[error("readback failed: {0}")]
    Readback(String),
    #[error("security violation: {0}")]
    SecurityViolation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_cycle_is_error() {
        let err: Box<dyn std::error::Error> = Box::new(RenderError::GraphCycle);
        assert_eq!(
            err.to_string(),
            "render graph has a cyclic resource dependency"
        );
    }

    #[test]
    fn no_adapter_is_error() {
        let err: Box<dyn std::error::Error> = Box::new(RenderError::NoAdapter);
        assert!(err.to_string().contains("no compatible GPU adapter"));
    }

    #[test]
    fn readback_variant_formats_correctly() {
        let err = RenderError::Readback("channel closed: disconnected".into());
        assert_eq!(
            err.to_string(),
            "readback failed: channel closed: disconnected"
        );
    }

    #[test]
    fn readback_is_error_trait() {
        let err: Box<dyn std::error::Error> =
            Box::new(RenderError::Readback("test failure".into()));
        assert!(err.source().is_none());
    }

    #[test]
    fn render_error_debug() {
        let err = RenderError::GraphCycle;
        let dbg = format!("{:?}", err);
        assert_eq!(dbg, "GraphCycle");
    }

    #[test]
    fn render_error_display_all_variants() {
        let cases: Vec<(RenderError, &str)> = vec![
            (
                RenderError::GraphCycle,
                "render graph has a cyclic resource dependency",
            ),
            (
                RenderError::NoAdapter,
                "no compatible GPU adapter found (wgpu could not find Vulkan/Metal/DX12/GL)",
            ),
            (
                RenderError::Readback("oops".into()),
                "readback failed: oops",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.to_string(), expected);
        }
    }
}
