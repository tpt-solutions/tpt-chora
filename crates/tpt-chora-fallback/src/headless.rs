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

        match self.config.output_format {
            OutputFormat::RawRgba => Ok(pixels),
            OutputFormat::Png => self.encode_png(&pixels),
            OutputFormat::Jpeg => self.encode_jpeg(&pixels),
        }
    }

    fn encode_png(&self, pixels: &[u8]) -> Result<Vec<u8>, crate::FallbackError> {
        let mut output = Vec::new();
        {
            let mut encoder = png::Encoder::new(
                std::io::Cursor::new(&mut output),
                self.config.width,
                self.config.height,
            );
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .map_err(|e| crate::FallbackError::EncodeFailed(e.to_string()))?;
            writer
                .write_image_data(pixels)
                .map_err(|e| crate::FallbackError::EncodeFailed(e.to_string()))?;
        }
        Ok(output)
    }

    fn encode_jpeg(&self, pixels: &[u8]) -> Result<Vec<u8>, crate::FallbackError> {
        Ok(pixels.to_vec())
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
