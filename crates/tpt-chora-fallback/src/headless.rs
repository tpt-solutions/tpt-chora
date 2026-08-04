pub struct HeadlessRenderer {
    renderer: tpt_chora_render::Renderer,
    config: HeadlessConfig,
}

#[derive(Debug, Clone)]
pub struct HeadlessConfig {
    pub width: u32,
    pub height: u32,
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Png,
    Jpeg,
    RawRgba,
}

impl HeadlessRenderer {
    pub fn new(config: HeadlessConfig) -> Result<Self, crate::FallbackError> {
        let renderer = tpt_chora_render::Renderer::new_headless(config.width, config.height)
            .map_err(|e| crate::FallbackError::InitFailed(e.to_string()))?;

        Ok(Self { renderer, config })
    }

    pub fn render_frame(&self) -> Result<Vec<u8>, crate::FallbackError> {
        let pixels = self
            .renderer
            .render_frame()
            .map_err(|e| crate::FallbackError::EncodeFailed(e.to_string()))?;

        crate::encoding::encode_pixels(
            &pixels,
            self.config.width,
            self.config.height,
            self.config.output_format,
        )
    }

    pub fn render_frame_to_file(&self, path: &std::path::Path) -> Result<(), crate::FallbackError> {
        let data = self.render_frame()?;
        std::fs::write(path, data)
            .map_err(|e| crate::FallbackError::EncodeFailed(e.to_string()))?;
        Ok(())
    }

    pub fn config(&self) -> &HeadlessConfig {
        &self.config
    }
}
