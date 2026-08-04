/// Shared output encoding for both the GPU-backed `HeadlessRenderer` and the
/// CPU-only `SoftwareRenderer`.
use crate::{FallbackError, OutputFormat};

/// Encodes tightly-packed RGBA8 pixels into the requested container format.
pub fn encode_pixels(
    pixels: &[u8],
    width: u32,
    height: u32,
    format: OutputFormat,
) -> Result<Vec<u8>, FallbackError> {
    match format {
        OutputFormat::RawRgba => Ok(pixels.to_vec()),
        OutputFormat::Png => encode_png(pixels, width, height),
        OutputFormat::Jpeg => encode_jpeg(pixels, width, height),
    }
}

fn encode_png(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, FallbackError> {
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(std::io::Cursor::new(&mut output), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| FallbackError::EncodeFailed(e.to_string()))?;
        writer
            .write_image_data(pixels)
            .map_err(|e| FallbackError::EncodeFailed(e.to_string()))?;
    }
    Ok(output)
}

fn encode_jpeg(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, FallbackError> {
    let rgb: Vec<u8> = pixels
        .chunks_exact(4)
        .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]])
        .collect();

    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
    use image::ImageEncoder;
    encoder
        .write_image(&rgb, width, height, image::ColorType::Rgb8.into())
        .map_err(|e| FallbackError::EncodeFailed(e.to_string()))?;

    Ok(buf.into_inner())
}
