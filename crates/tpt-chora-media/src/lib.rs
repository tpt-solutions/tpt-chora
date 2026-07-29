// `deny` rather than `forbid`: the optional `native-video-backends` feature
// calls real hardware video decode APIs (VA-API / VideoToolbox /
// MediaCodec), which need `unsafe` FFI at their call sites (each annotated
// with its own `#[allow(unsafe_code)]` and a `// SAFETY:` justification) —
// everything else in this crate stays safe.
#![deny(unsafe_code)]

pub mod decode;
pub mod error;
pub mod streaming;
pub mod texture;

pub use decode::{DecodedImage, ImageDecoder, VideoDecoder, VideoFrame};
pub use error::MediaError;
pub use streaming::{AssetPriority, AssetStreamer, StreamRequest};
pub use texture::{CachedTexture, GpuTextureCache};
