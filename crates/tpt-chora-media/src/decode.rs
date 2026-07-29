/// Largest width/height (in pixels) `ImageDecoder` will decode. Rejecting
/// oversized images before the full decode guards against decompression
/// bombs — a small compressed file (e.g. a crafted PNG) that expands to a
/// multi-gigabyte pixel buffer.
const MAX_IMAGE_DIMENSION: u32 = 16_384;

/// Largest decoded RGBA byte budget `ImageDecoder` will allocate.
const MAX_DECODED_BYTES: u64 = 256 * 1024 * 1024;

pub struct ImageDecoder;

pub struct VideoDecoder {
    backend: VideoDecodeBackend,
}

enum VideoDecodeBackend {
    VaApi,
    VideoToolbox,
    MediaCodec,
    SoftwareFallback,
}

pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub format: ImageFormat,
    pub presentation_timestamp_us: u64,
}

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

/// `image::Limits` is `#[non_exhaustive]`, so it can't be built with struct-
/// literal syntax outside the `image` crate; construct the default and
/// mutate the fields we care about instead.
fn decode_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODED_BYTES);
    limits
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
        // Setting `Limits` before `decode()` (rather than calling the
        // `image::load_from_memory` free function, which decodes with no
        // caps) rejects oversized/decompression-bomb images by their
        // declared header dimensions before the full pixel buffer is
        // allocated.
        let mut reader = image::ImageReader::new(std::io::Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| crate::MediaError::ImageDecode(e.to_string()))?;
        reader.limits(decode_limits());

        let img = reader
            .decode()
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
        } else if (data.len() >= 12 && data[0..4] == [0x52, 0x49, 0x46, 0x46])
            || data.starts_with(b"VP8")
            || data.starts_with(b"VP8L")
            || data.starts_with(b"VP8X")
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

impl Default for ImageDecoder {
    fn default() -> Self {
        Self::new()
    }
}

fn f16_from_f32(val: f32) -> u16 {
    let bits = val.to_bits();
    let sign = (bits >> 16) as u16 & 0x8000;
    let exp = ((bits >> 23) as i32 - 127 + 15).clamp(0, 31) as u16;
    let mantissa = ((bits >> 13) & 0x3FF) as u16;

    if exp == 0 && mantissa == 0 {
        sign
    } else if exp == 31 {
        sign | 0x7C00 | mantissa
    } else {
        sign | (exp << 10) | mantissa
    }
}

impl VideoDecoder {
    pub fn new() -> Self {
        let backend = if cfg!(target_os = "linux") {
            VideoDecodeBackend::VaApi
        } else if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
            VideoDecodeBackend::VideoToolbox
        } else if cfg!(target_os = "android") {
            VideoDecodeBackend::MediaCodec
        } else {
            VideoDecodeBackend::SoftwareFallback
        };
        Self { backend }
    }

    pub fn decode_frame(&self, _encoded_data: &[u8]) -> Result<VideoFrame, crate::MediaError> {
        match &self.backend {
            VideoDecodeBackend::VaApi => Err(crate::MediaError::VideoDecodeUnavailable),
            VideoDecodeBackend::VideoToolbox => Err(crate::MediaError::VideoDecodeUnavailable),
            VideoDecodeBackend::MediaCodec => Err(crate::MediaError::VideoDecodeUnavailable),
            VideoDecodeBackend::SoftwareFallback => Err(crate::MediaError::VideoDecodeUnavailable),
        }
    }

    pub fn backend_name(&self) -> &'static str {
        match &self.backend {
            VideoDecodeBackend::VaApi => "VA-API",
            VideoDecodeBackend::VideoToolbox => "VideoToolbox",
            VideoDecodeBackend::MediaCodec => "MediaCodec",
            VideoDecodeBackend::SoftwareFallback => "software-fallback",
        }
    }
}

impl Default for VideoDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_png(width: u32, height: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([x as u8, y as u8, 0, 255])
        });
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("encode test PNG");
        bytes
    }

    #[test]
    fn decodes_a_normal_sized_image() {
        let data = encode_png(4, 4);
        let decoded = ImageDecoder::new().decode(&data).expect("decode");
        assert_eq!((decoded.width, decoded.height), (4, 4));
        assert_eq!(decoded.data.len(), 4 * 4 * 4);
    }

    #[test]
    fn rejects_images_exceeding_the_max_dimension() {
        // One dimension over the cap; the other stays tiny so the encoded
        // fixture itself is small.
        let data = encode_png(MAX_IMAGE_DIMENSION + 1, 1);
        let err = match ImageDecoder::new().decode(&data) {
            Ok(_) => panic!("expected oversized image to be rejected"),
            Err(e) => e,
        };
        assert!(
            err.to_string().to_lowercase().contains("limit"),
            "expected a limits error, got: {err}"
        );
    }

    #[test]
    fn accepts_an_image_exactly_at_the_max_dimension() {
        let data = encode_png(1, MAX_IMAGE_DIMENSION);
        let decoded = ImageDecoder::new().decode(&data).expect("decode");
        assert_eq!(decoded.height, MAX_IMAGE_DIMENSION);
    }
}
