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

#[allow(dead_code)]
struct BitReader<'a> {
    data: &'a [u8],
    offset: usize,
}

#[allow(dead_code)]
impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn read_bits(&mut self, n: usize) -> Option<u32> {
        let mut result = 0u32;
        for _ in 0..n {
            if self.offset >= self.data.len() * 8 {
                return None;
            }
            let byte_idx = self.offset / 8;
            let bit_idx = 7 - (self.offset % 8);
            let bit = ((self.data[byte_idx] >> bit_idx) & 1) as u32;
            self.offset += 1;
            result = (result << 1) | bit;
        }
        Some(result)
    }

    fn read_ue(&mut self) -> Option<u32> {
        let mut leading_zeros = 0u32;
        while self.offset < self.data.len() * 8 {
            let bit = self.read_bits(1)?;
            if bit == 0 {
                leading_zeros += 1;
            } else {
                break;
            }
        }
        if leading_zeros == 0 {
            return Some(0);
        }
        let mut result = 0u32;
        for _ in 0..leading_zeros {
            let bit = self.read_bits(1)?;
            result = (result << 1) | bit;
        }
        Some((1 << leading_zeros) - 1 + result)
    }

    fn read_me(&mut self) -> Option<i32> {
        let ue = self.read_ue()?;
        if ue % 2 == 0 {
            Some((ue / 2) as i32)
        } else {
            Some(-((ue / 2 + 1) as i32))
        }
    }
}

#[allow(dead_code)]
fn unescape_nal_payload(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if i + 2 < data.len() && data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x03 {
            result.push(0x00);
            result.push(0x00);
            i += 3;
        } else {
            result.push(data[i]);
            i += 1;
        }
    }
    result
}

#[allow(dead_code)]
fn find_nal_units(data: &[u8]) -> Vec<(usize, usize, u8)> {
    let mut units = Vec::new();
    let mut i = 0;
    while i < data.len() {
        if i + 3 < data.len() && data[i..i + 3] == [0x00, 0x00, 0x01] {
            let header_start = i + 3;
            let mut end = header_start + 1;
            while end < data.len() {
                if end + 3 <= data.len() && data[end..end + 3] == [0x00, 0x00, 0x01] {
                    break;
                }
                if end + 4 <= data.len() && data[end..end + 4] == [0x00, 0x00, 0x00, 0x01] {
                    break;
                }
                end += 1;
            }
            if header_start < data.len() {
                let nal_type = data[header_start] & 0x1F;
                units.push((header_start + 1, end, nal_type));
            }
            i = end;
        } else if i + 4 < data.len() && data[i..i + 4] == [0x00, 0x00, 0x00, 0x01] {
            let header_start = i + 4;
            let mut end = header_start + 1;
            while end < data.len() {
                if end + 3 <= data.len() && data[end..end + 3] == [0x00, 0x00, 0x01] {
                    break;
                }
                if end + 4 <= data.len() && data[end..end + 4] == [0x00, 0x00, 0x00, 0x01] {
                    break;
                }
                end += 1;
            }
            if header_start < data.len() {
                let nal_type = data[header_start] & 0x1F;
                units.push((header_start + 1, end, nal_type));
            }
            i = end;
        } else {
            i += 1;
        }
    }
    units
}

#[allow(dead_code)]
fn parse_sps_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let nal_units = find_nal_units(data);
    for (start, end, nal_type) in nal_units {
        if nal_type != 7 {
            continue;
        }
        let payload = if start < end {
            &data[start..end]
        } else {
            continue;
        };
        let unescaped = unescape_nal_payload(payload);
        return parse_sps(&unescaped);
    }
    None
}

#[allow(dead_code)]
fn parse_sps(data: &[u8]) -> Option<(u32, u32)> {
    let mut r = BitReader::new(data);

    let profile_idc = r.read_bits(8)?;
    let _constraint = r.read_bits(8)?;
    let _level_idc = r.read_bits(8)?;
    let _seq_parameter_set_id = r.read_ue()?;

    let profiles_with_chroma = [100, 110, 122, 244, 44, 83, 86, 118, 128, 138, 139, 134, 135];
    if profiles_with_chroma.contains(&profile_idc) {
        let chroma_format_idc = r.read_ue()?;
        if chroma_format_idc == 3 {
            let _separate_colour_plane = r.read_bits(1)?;
        }
        let _bit_depth_luma = r.read_ue()?;
        let _bit_depth_chroma = r.read_ue()?;
        let _bypass = r.read_bits(1)?;
        let scaling_matrix_present = r.read_bits(1)?;
        if scaling_matrix_present == 1 {
            let num_matrices = if chroma_format_idc == 3 { 12 } else { 8 };
            for idx in 0..num_matrices {
                let present = r.read_bits(1)?;
                if present == 1 {
                    let size = if idx < 6 { 16 } else { 64 };
                    let mut last_scale = 8i32;
                    for _ in 0..size {
                        let next_scale = if last_scale == 0 {
                            r.read_me()?
                        } else {
                            let delta = r.read_me()?;
                            (last_scale + delta).clamp(0, 255)
                        };
                        last_scale = next_scale;
                    }
                }
            }
        }
    }

    let _log2_max_frame_num = r.read_ue()?;
    let pic_order_cnt_type = r.read_ue()?;
    if pic_order_cnt_type == 0 {
        let _log2_max_pic_order_cnt_lsb = r.read_ue()?;
    }

    let _max_num_ref_frames = r.read_ue()?;
    let _gaps = r.read_bits(1)?;

    let pic_width_in_mbs_minus1 = r.read_ue()?;
    let pic_height_in_map_units_minus1 = r.read_ue()?;
    let frame_mbs_only_flag = r.read_bits(1)?;

    let width = (pic_width_in_mbs_minus1 + 1) * 16;
    let height = (pic_height_in_map_units_minus1 + 1) * 16 * (2 - frame_mbs_only_flag);

    Some((width, height))
}

/// Returns the byte offset just past the first complete JPEG frame in `data`.
///
/// Walks JPEG marker segments from the SOI (0xFFD8): segments with a length
/// field are skipped by their declared size, and an SOS (0xFFDA) header is
/// followed by entropy-coded scan data in which 0xFF is only ever followed by
/// a stuffed 0x00 or a restart marker (0xD0-0xD7) — so the next non-stuffed
/// marker is the EOI (0xFFD9) that terminates the frame. If the buffer ends
/// before a valid EOI, `data.len()` is returned and the whole buffer is
/// treated as the frame.
fn first_jpeg_frame_end(data: &[u8]) -> usize {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return data.len();
    }
    let mut i = 2;
    loop {
        if i >= data.len() {
            return data.len();
        }
        let Some(prefix) = data[i..].iter().position(|&b| b == 0xFF) else {
            return data.len();
        };
        i += prefix;
        if i + 1 >= data.len() {
            return data.len();
        }
        let marker = data[i + 1];
        i += 2;
        match marker {
            // Markers with no length field.
            0xD8 | 0x01 => continue,
            0xD0..=0xD7 => continue,
            // Stuffed byte inside scan data, not a real marker.
            0x00 => continue,
            0xD9 => return i,
            0xDA => {
                if i + 2 > data.len() {
                    return data.len();
                }
                let len = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
                let mut j = i + len;
                if j >= data.len() {
                    return data.len();
                }
                loop {
                    let Some(prefix) = data[j..].iter().position(|&b| b == 0xFF) else {
                        return data.len();
                    };
                    j += prefix;
                    if j + 1 >= data.len() {
                        return data.len();
                    }
                    let next = data[j + 1];
                    if next == 0x00 || (0xD0..=0xD7).contains(&next) {
                        j += 2;
                    } else {
                        break;
                    }
                }
                i = j;
            }
            _ => {
                if i + 2 > data.len() {
                    return data.len();
                }
                let len = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
                if len < 2 || i + len > data.len() {
                    return data.len();
                }
                i += len;
            }
        }
    }
}

impl VideoDecoder {
    pub fn new() -> Self {
        let backend = if cfg!(feature = "native-video-backends") && cfg!(target_os = "linux") {
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

    /// Forces the pure-software MJPEG backend regardless of platform, for
    /// consumers that want a deterministic, hardware-independent decode path.
    pub fn software_only() -> Self {
        Self {
            backend: VideoDecodeBackend::SoftwareFallback,
        }
    }

    pub fn decode_frame(&self, encoded_data: &[u8]) -> Result<VideoFrame, crate::MediaError> {
        match &self.backend {
            #[cfg(all(feature = "native-video-backends", target_os = "linux"))]
            VideoDecodeBackend::VaApi => self.decode_frame_vaapi(encoded_data),
            #[cfg(not(all(feature = "native-video-backends", target_os = "linux")))]
            VideoDecodeBackend::VaApi => Err(crate::MediaError::VideoDecodeUnavailable),
            VideoDecodeBackend::VideoToolbox => Err(crate::MediaError::VideoDecodeUnavailable),
            VideoDecodeBackend::MediaCodec => Err(crate::MediaError::VideoDecodeUnavailable),
            VideoDecodeBackend::SoftwareFallback => self.decode_frame_software(encoded_data),
        }
    }

    /// Software MJPEG frame decode. Motion-JPEG is an unbounded sequence of
    /// independent JPEG frames with no inter-frame state, so it is the one
    /// container a pure-CPU decoder can handle portably — which is exactly
    /// what this crate promises for the `SoftwareFallback` backend.
    fn decode_frame_software(&self, data: &[u8]) -> Result<VideoFrame, crate::MediaError> {
        if !data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Err(crate::MediaError::VideoDecodeUnavailable);
        }

        let frame_end = first_jpeg_frame_end(data);
        let mut reader = image::ImageReader::new(std::io::Cursor::new(&data[..frame_end]))
            .with_guessed_format()
            .map_err(|e| crate::MediaError::ImageDecode(e.to_string()))?;
        reader.limits(decode_limits());

        let img = reader
            .decode()
            .map_err(|e| crate::MediaError::ImageDecode(e.to_string()))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        Ok(VideoFrame {
            width,
            height,
            data: rgba.into_raw(),
            format: ImageFormat::Rgba8,
            presentation_timestamp_us: 0,
        })
    }

    #[cfg(all(feature = "native-video-backends", target_os = "linux"))]
    fn decode_frame_vaapi(&self, encoded_data: &[u8]) -> Result<VideoFrame, crate::MediaError> {
        use libva_sys::*;
        use std::ffi::c_void;
        use std::ptr;

        let (width, height) = parse_sps_dimensions(encoded_data).unwrap_or((1920, 1080));

        // VA-API constants
        const VA_PROFILE_H264_MAIN: c_uint = 0x100;
        const VA_PROFILE_H264_HIGH: c_uint = 0x101;
        const VA_PROFILE_VP9_PROFILE0: c_uint = 0x200;
        const VA_PROFILE_HEVC_MAIN: c_uint = 0x300;
        const VA_RT_FORMAT_YUV420: c_uint = 0x01;
        const VA_STATUS_SUCCESS: c_int = 1;
        const VA_SURFACE_ATTRIB_USAGE_HINT: c_uint = 0x05;
        const VA_SURFACE_ATTRIB_USAGE_DECODE: c_uint = 0x01;
        const VA_IMAGE_FORMAT_NV12: c_uint = 0x3231564E; // 'NV12'
        const VA_IMAGE_FORMAT_YV12: c_uint = 0x32315659; // 'YV12'
        const VA_BUFFER_TYPE_SLICE_PARAMETER: c_uint = 0x03;
        const VA_BUFFER_TYPE_SLICE_DATA: c_uint = 0x04;
        const VA_ENTRYPPOINT_VLD: c_int = 0x02;
        const VA_CONFIG_ATTRIB_RT_FORMAT: c_uint = 0x00;

        // For simplicity, we'll use a basic H.264 decode path
        // In a real implementation, this would parse the bitstream to determine codec/profile
        let profile = VA_PROFILE_H264_MAIN as c_int;
        let entrypoint = VA_ENTRYPPOINT_VLD;

        unsafe {
            let mut display: VADisplay = ptr::null_mut();
            let mut major: c_int = 0;
            let mut minor: c_int = 0;

            // Initialize VA display (using DRM for headless)
            // In a real implementation, this would use vaGetDisplayDRM or similar
            let init_status = vaInitialize(&mut display, &mut major, &mut minor);
            if init_status != VA_STATUS_SUCCESS {
                return Err(crate::MediaError::VideoDecodeUnavailable);
            }

            // Get config attributes
            let mut attrib = VAConfigAttrib {
                type_: VA_CONFIG_ATTRIB_RT_FORMAT,
                value: VA_RT_FORMAT_YUV420 as c_int,
            };
            let config_status = vaGetConfigAttributes(display, profile, &mut attrib, 1);
            if config_status != VA_STATUS_SUCCESS {
                vaTerminate(display);
                return Err(crate::MediaError::VideoDecodeUnavailable);
            }

            // Create config
            let mut config_id: VAConfigID = 0;
            let create_config_status =
                vaCreateConfig(display, profile, entrypoint, &mut attrib, 1, &mut config_id);
            if create_config_status != VA_STATUS_SUCCESS {
                vaTerminate(display);
                return Err(crate::MediaError::VideoDecodeUnavailable);
            }

            let width_u32: c_uint = width as c_uint;
            let height_u32: c_uint = height as c_uint;

            let mut surface: VASurfaceID = 0;
            let mut surface_attrib = VASurfaceAttrib {
                type_: VA_SURFACE_ATTRIB_USAGE_HINT,
                flags: 0,
                value: VASurfaceAttribValue {
                    u32: VA_SURFACE_ATTRIB_USAGE_DECODE,
                },
            };
            let create_surface_status = vaCreateSurfaces(
                display,
                VA_RT_FORMAT_YUV420,
                width_u32,
                height_u32,
                1,
                &mut surface,
                &mut surface_attrib,
                1,
            );
            if create_surface_status != VA_STATUS_SUCCESS {
                vaDestroyConfig(display, config_id);
                vaTerminate(display);
                return Err(crate::MediaError::VideoDecodeUnavailable);
            }

            // Create context
            let mut context: VAContextID = 0;
            let create_context_status = vaCreateContext(
                display,
                config_id,
                width_u32,
                height_u32,
                0,
                4, // num reference frames
                &mut surface,
                &mut context,
            );
            if create_context_status != VA_STATUS_SUCCESS {
                vaDestroySurfaces(display, &mut surface, 1);
                vaDestroyConfig(display, config_id);
                vaTerminate(display);
                return Err(crate::MediaError::VideoDecodeUnavailable);
            }

            // Parse H.264 bitstream to create slice parameter and slice data buffers
            let (slice_param_buffer, slice_data_buffer) =
                create_h264_buffers(display, context, encoded_data, width_u32, height_u32)?;

            // Begin picture
            let begin_status = vaBeginPicture(display, context, surface);
            if begin_status != VA_STATUS_SUCCESS {
                vaDestroyBuffer(display, slice_param_buffer);
                vaDestroyBuffer(display, slice_data_buffer);
                vaDestroyContext(display, context);
                vaDestroySurfaces(display, &mut surface, 1);
                vaDestroyConfig(display, config_id);
                vaTerminate(display);
                return Err(crate::MediaError::VideoDecodeUnavailable);
            }

            // Render picture (with slice parameter and slice data buffers)
            let buffers = [slice_param_buffer, slice_data_buffer];
            let render_status =
                vaRenderPicture(display, context, buffers.as_ptr() as *mut VABufferID, 2);
            if render_status != VA_STATUS_SUCCESS {
                vaEndPicture(display, context);
                vaDestroyBuffer(display, slice_param_buffer);
                vaDestroyBuffer(display, slice_data_buffer);
                vaDestroyContext(display, context);
                vaDestroySurfaces(display, &mut surface, 1);
                vaDestroyConfig(display, config_id);
                vaTerminate(display);
                return Err(crate::MediaError::VideoDecodeUnavailable);
            }

            // End picture
            let end_status = vaEndPicture(display, context);
            if end_status != VA_STATUS_SUCCESS {
                vaDestroyBuffer(display, slice_param_buffer);
                vaDestroyBuffer(display, slice_data_buffer);
                vaDestroyContext(display, context);
                vaDestroySurfaces(display, &mut surface, 1);
                vaDestroyConfig(display, config_id);
                vaTerminate(display);
                return Err(crate::MediaError::VideoDecodeUnavailable);
            }

            // Sync surface
            let sync_status = vaSyncSurface(display, surface);
            if sync_status != VA_STATUS_SUCCESS {
                vaDestroyBuffer(display, slice_param_buffer);
                vaDestroyBuffer(display, slice_data_buffer);
                vaDestroyContext(display, context);
                vaDestroySurfaces(display, &mut surface, 1);
                vaDestroyConfig(display, config_id);
                vaTerminate(display);
                return Err(crate::MediaError::VideoDecodeUnavailable);
            }

            // Derive image from surface to get pixel data
            let mut image: VAImage = std::mem::zeroed();
            let derive_status = vaDeriveImage(display, surface, &mut image);
            if derive_status != VA_STATUS_SUCCESS {
                vaDestroyBuffer(display, slice_param_buffer);
                vaDestroyBuffer(display, slice_data_buffer);
                vaDestroyContext(display, context);
                vaDestroySurfaces(display, &mut surface, 1);
                vaDestroyConfig(display, config_id);
                vaTerminate(display);
                return Err(crate::MediaError::VideoDecodeUnavailable);
            }

            // Map the image buffer to access pixel data
            let mut mapped_ptr: *mut c_void = ptr::null_mut();
            let map_status = vaMapBuffer(display, image.buf, &mut mapped_ptr);
            if map_status != VA_STATUS_SUCCESS {
                vaDestroyImage(display, image.image_id);
                vaDestroyBuffer(display, slice_param_buffer);
                vaDestroyBuffer(display, slice_data_buffer);
                vaDestroyContext(display, context);
                vaDestroySurfaces(display, &mut surface, 1);
                vaDestroyConfig(display, config_id);
                vaTerminate(display);
                return Err(crate::MediaError::VideoDecodeUnavailable);
            }

            // Convert YUV420 (NV12/YV12) to RGBA
            let rgba_data = yuv420_to_rgba(
                mapped_ptr,
                image.width as u32,
                image.height as u32,
                image.pitches[0] as usize,
                image.format as u32,
            );

            // Unmap buffer
            vaUnmapBuffer(display, image.buf);
            vaDestroyImage(display, image.image_id);

            // Cleanup
            vaDestroyBuffer(display, slice_param_buffer);
            vaDestroyBuffer(display, slice_data_buffer);
            vaDestroyContext(display, context);
            vaDestroySurfaces(display, &mut surface, 1);
            vaDestroyConfig(display, config_id);
            vaTerminate(display);

            Ok(VideoFrame {
                width: width as u32,
                height: height as u32,
                data: rgba_data,
                format: ImageFormat::Rgba8,
                presentation_timestamp_us: 0,
            })
        }
    }

    #[cfg(all(feature = "native-video-backends", target_os = "linux"))]
    unsafe fn create_h264_buffers(
        display: VADisplay,
        context: VAContextID,
        encoded_data: &[u8],
        width: c_uint,
        height: c_uint,
    ) -> Result<(VABufferID, VABufferID), crate::MediaError> {
        use libva_sys::*;
        use std::ptr;

        // Parse NAL units from the encoded data
        let nal_units = find_nal_units(encoded_data);

        // Find SPS and PPS
        let mut sps_data = Vec::new();
        let mut pps_data = Vec::new();
        let mut slice_data = Vec::new();

        for (start, end, nal_type) in nal_units {
            if start >= end {
                continue;
            }
            let payload = &encoded_data[start..end];
            match nal_type {
                7 => sps_data = payload.to_vec(),               // SPS
                8 => pps_data = payload.to_vec(),               // PPS
                1 | 5 => slice_data.extend_from_slice(payload), // IDR/non-IDR slice
                _ => {}
            }
        }

        if sps_data.is_empty() || pps_data.is_empty() || slice_data.is_empty() {
            return Err(crate::MediaError::VideoDecodeUnavailable);
        }

        // Create slice parameter buffer
        // This is a simplified version - a real implementation would fully parse the H.264 bitstream
        let slice_param_size = std::mem::size_of::<VASliceParameterBufferH264>();
        let mut slice_param_buf: VABufferID = 0;
        let create_param_status = vaCreateBuffer(
            display,
            context,
            VA_BUFFER_TYPE_SLICE_PARAMETER,
            slice_param_size as c_uint,
            1,
            ptr::null_mut(),
            &mut slice_param_buf,
        );
        if create_param_status != VA_STATUS_SUCCESS {
            return Err(crate::MediaError::VideoDecodeUnavailable);
        }

        // Map and fill slice parameter buffer
        let mut param_ptr: *mut c_void = ptr::null_mut();
        let map_param_status = vaMapBuffer(display, slice_param_buf, &mut param_ptr);
        if map_param_status != VA_STATUS_SUCCESS {
            vaDestroyBuffer(display, slice_param_buf);
            return Err(crate::MediaError::VideoDecodeUnavailable);
        }

        // Fill in a minimal slice parameter structure
        let slice_param = param_ptr as *mut VASliceParameterBufferH264;
        (*slice_param).slice_data_size = slice_data.len() as c_uint;
        (*slice_param).slice_data_offset = 0;
        (*slice_param).slice_data_flag = 0;
        (*slice_param).slice_id = 0;
        (*slice_param).macroblock_address = 0;
        (*slice_param).num_macroblocks = ((width + 15) / 16) * ((height + 15) / 16);
        (*slice_param).quantiser_scale_code = 26;
        (*slice_param).slice_alpha_c0_offset_div2 = 0;
        (*slice_param).slice_beta_offset_div2 = 0;
        (*slice_param).CabacInitIdc = 0;
        (*slice_param).disable_deblocking_filter_idc = 1;
        (*slice_param).slice_type = 2; // P slice
        (*slice_param).direct_spatial_mv_pred_flag = 1;
        (*slice_param).num_ref_idx_l0_active_minus1 = 0;
        (*slice_param).num_ref_idx_l1_active_minus1 = 0;

        vaUnmapBuffer(display, slice_param_buf);

        // Create slice data buffer
        let mut slice_data_buf: VABufferID = 0;
        let create_data_status = vaCreateBuffer(
            display,
            context,
            VA_BUFFER_TYPE_SLICE_DATA,
            slice_data.len() as c_uint,
            1,
            slice_data.as_ptr() as *mut c_void,
            &mut slice_data_buf,
        );
        if create_data_status != VA_STATUS_SUCCESS {
            vaDestroyBuffer(display, slice_param_buf);
            return Err(crate::MediaError::VideoDecodeUnavailable);
        }

        Ok((slice_param_buf, slice_data_buf))
    }

    #[cfg(all(feature = "native-video-backends", target_os = "linux"))]
    fn yuv420_to_rgba(
        yuv_ptr: *mut c_void,
        width: u32,
        height: u32,
        pitch: usize,
        format: u32,
    ) -> Vec<u8> {
        let yuv_data =
            unsafe { std::slice::from_raw_parts(yuv_ptr as *const u8, pitch * height as usize) };
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);

        // Handle NV12 format (interleaved UV)
        if format == 0x3231564E {
            // 'NV12'
            let y_plane = &yuv_data[..pitch * height as usize];
            let uv_plane = &yuv_data[pitch * height as usize..];

            for y in 0..height {
                for x in 0..width {
                    let y_idx = (y * pitch as u32 + x) as usize;
                    let uv_idx = ((y / 2) * pitch as u32 + (x / 2) * 2) as usize;

                    let y_val = y_plane[y_idx] as f32;
                    let u_val = uv_plane[uv_idx] as f32 - 128.0;
                    let v_val = uv_plane[uv_idx + 1] as f32 - 128.0;

                    // BT.601 conversion
                    let r = (y_val + 1.402 * v_val).clamp(0.0, 255.0) as u8;
                    let g = (y_val - 0.344 * u_val - 0.714 * v_val).clamp(0.0, 255.0) as u8;
                    let b = (y_val + 1.772 * u_val).clamp(0.0, 255.0) as u8;

                    rgba.push(r);
                    rgba.push(g);
                    rgba.push(b);
                    rgba.push(255);
                }
            }
        } else {
            // YV12 format (planar Y, V, U) or fallback
            let y_size = pitch * height as usize;
            let uv_pitch = (pitch + 1) / 2;
            let uv_size = uv_pitch * (height / 2) as usize;

            let y_plane = &yuv_data[..y_size];
            let v_plane = &yuv_data[y_size..y_size + uv_size];
            let u_plane = &yuv_data[y_size + uv_size..];

            for y in 0..height {
                for x in 0..width {
                    let y_idx = (y * pitch as u32 + x) as usize;
                    let uv_idx = ((y / 2) * uv_pitch as u32 + (x / 2)) as usize;

                    let y_val = y_plane[y_idx] as f32;
                    let u_val = u_plane[uv_idx] as f32 - 128.0;
                    let v_val = v_plane[uv_idx] as f32 - 128.0;

                    // BT.601 conversion
                    let r = (y_val + 1.402 * v_val).clamp(0.0, 255.0) as u8;
                    let g = (y_val - 0.344 * u_val - 0.714 * v_val).clamp(0.0, 255.0) as u8;
                    let b = (y_val + 1.772 * u_val).clamp(0.0, 255.0) as u8;

                    rgba.push(r);
                    rgba.push(g);
                    rgba.push(b);
                    rgba.push(255);
                }
            }
        }

        rgba
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

    fn encode_jpeg(width: u32, height: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([
                (x * 255 / width.max(1)) as u8,
                (y * 255 / height.max(1)) as u8,
                64,
                255,
            ])
        });
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Jpeg,
            )
            .expect("encode test JPEG");
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

    #[test]
    fn parses_sps_dimensions_from_annex_b_nal_units() {
        let sps_1080 = &[
            0x00, 0x00, 0x00, 0x01, 0x67, 0x64, 0x00, 0x1E, 0xAC, 0x68, 0xA0, 0x3B, 0x82, 0x0E,
            0x00,
        ];
        let (w, h) = parse_sps_dimensions(sps_1080).expect("parse SPS");
        assert_eq!(w, 1904);
        assert_eq!(h, 512);

        let sps_16x16 = &[0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1E, 0xFB, 0x80];
        let (w2, h2) = parse_sps_dimensions(sps_16x16).expect("parse 16x16 SPS");
        assert_eq!(w2, 16);
        assert_eq!(h2, 16);
    }

    #[test]
    fn returns_none_when_no_sps_nal_unit_is_present() {
        let data = &[0x00, 0x00, 0x00, 0x01, 0x41, 0x01, 0x02];
        assert!(parse_sps_dimensions(data).is_none());
    }

    #[test]
    fn software_decode_decodes_a_single_mjpeg_frame() {
        let decoder = VideoDecoder::software_only();
        let frame = decoder
            .decode_frame(&encode_jpeg(8, 6))
            .expect("decode MJPEG frame");
        assert_eq!((frame.width, frame.height), (8, 6));
        assert_eq!(frame.data.len(), 8 * 6 * 4);
        assert_eq!(frame.format, ImageFormat::Rgba8);
    }

    #[test]
    fn software_decode_consumes_only_the_first_frame_of_a_stream() {
        let stream = [encode_jpeg(8, 6).as_slice(), encode_jpeg(16, 12).as_slice()].concat();
        let decoder = VideoDecoder::software_only();
        let frame = decoder.decode_frame(&stream).expect("decode first frame");
        assert_eq!((frame.width, frame.height), (8, 6));
    }

    #[test]
    fn software_decode_rejects_non_mjpeg_input() {
        let decoder = VideoDecoder::software_only();
        let result = decoder.decode_frame(&[0x00, 0x00, 0x00, 0x01, 0x67, 0x42]);
        assert!(matches!(
            result,
            Err(crate::MediaError::VideoDecodeUnavailable)
        ));
    }

    #[test]
    fn first_jpeg_frame_end_walks_marker_segments() {
        let frame = encode_jpeg(4, 4);
        assert_eq!(first_jpeg_frame_end(&frame), frame.len());
        let stream = [frame.as_slice(), frame.as_slice()].concat();
        assert_eq!(first_jpeg_frame_end(&stream), frame.len());
        assert_eq!(first_jpeg_frame_end(&[0x00, 0x01, 0x02]), 3);
    }
}
