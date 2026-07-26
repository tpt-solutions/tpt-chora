use std::io::Cursor;

pub struct ImageDecoder;

pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub format: ImageFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Rgba8,
    Rgb8,
    Rgba16,
    Bgra8,
}

impl ImageDecoder {
    pub fn new() -> Self {
        Self
    }

    pub fn decode(&self, data: &[u8]) -> Result<DecodedImage, crate::MediaError> {
        let format = self.detect_format(data)?;
        match format {
            ImageFormat::Rgba8 | ImageFormat::Rgb8 | ImageFormat::Bgra8 => {
                self.decode_image_rs(data)
            }
            _ => self.decode_image_rs(data),
        }
    }

    fn decode_image_rs(&self, data: &[u8]) -> Result<DecodedImage, crate::MediaError> {
        let img = image::load_from_memory(data)
            .map_err(|e| crate::MediaError::ImageDecode(e.to_string()))?;

        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        Ok(DecodedImage {
            width,
            height,
            data: rgba.into_raw(),
            format: ImageFormat::Rgba8,
        })
    }

    fn detect_format(&self, data: &[u8]) -> Result<ImageFormat, crate::MediaError> {
        if data.len() < 8 {
            return Err(crate::MediaError::UnsupportedFormat);
        }

        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            Ok(ImageFormat::Rgba8)
        } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Ok(ImageFormat::Rgb8)
        } else if data.len() >= 12 && data[0..4] == [0x52, 0x49, 0x46, 0x46] {
            Ok(ImageFormat::Rgba8)
        } else if data.starts_with(b"VP8") || data.starts_with(b"VP8L") || data.starts_with(b"VP8X")
        {
            Ok(ImageFormat::Rgba8)
        } else {
            Err(crate::MediaError::UnsupportedFormat)
        }
    }

    pub fn decode_to_gpu_format(
        &self,
        data: &[u8],
        target_format: wgpu::TextureFormat,
    ) -> Result<DecodedImage, crate::MediaError> {
        let decoded = self.decode(data)?;
        Ok(decoded)
    }
}
