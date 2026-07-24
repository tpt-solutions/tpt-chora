#[derive(Debug, thiserror::Error)]
pub enum TextError {
    #[error("no font loaded")]
    NoFont,
    #[error("font parsing failed: {0}")]
    FontParse(String),
    #[error("glyph not found for codepoint U+{0:04X}")]
    GlyphNotFound(u32),
    #[error("atlas full: need {0}x{1} but only {2}x{3} remaining")]
    AtlasFull(u32, u32, u32, u32),
    #[error("shaping failed: {0}")]
    ShapingFailed(String),
    #[error("GPU texture creation failed: {0}")]
    TextureCreation(String),
}
