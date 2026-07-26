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
        let mut decoded = self.decode(data)?;

        match target_format {
            wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Bgra8Unorm => {
                for chunk in decoded.data.chunks_exact_mut(4) {
                    chunk.swap(0, 2);
                }
                decoded.format = ImageFormat::Bgra8;
            }
            wgpu::TextureFormat::R8Unorm => {
                let gray: Vec<u8> = decoded
                    .data
                    .chunks_exact(4)
                    .map(|rgba| {
                        let r = rgba[0] as f32;
                        let g = rgba[1] as f32;
                        let b = rgba[2] as f32;
                        (0.299 * r + 0.587 * g + 0.114 * b) as u8
                    })
                    .collect();
                let w = decoded.width;
                let h = decoded.height;
                decoded = DecodedImage {
                    width: w,
                    height: h,
                    data: gray,
                    format: ImageFormat::Rgba8,
                };
            }
            wgpu::TextureFormat::Rgba16Float => {
                let mut float_data = Vec::with_capacity(decoded.data.len() * 2);
                for &byte in &decoded.data {
                    let f = byte as f32 / 255.0;
                    let half = f16_from_f32(f);
                    float_data.extend_from_slice(&half.to_le_bytes());
                }
                decoded.data = float_data;
            }
            _ => {}
        }

        Ok(decoded)
    }
}

fn f16_from_f32(val: f32) -> u16 {
    let bits = val.to_bits();
    let sign = (bits >> 16) as u16 & 0x8000;
    let exp = ((bits >> 23) as i32 - 127 + 15).max(0).min(31) as u16;
    let mantissa = ((bits >> 13) & 0x3FF) as u16;

    if exp == 0 && mantissa == 0 {
        sign
    } else if exp == 31 {
        sign | 0x7C00 | mantissa
    } else {
        sign | (exp << 10) | mantissa
    }
}
