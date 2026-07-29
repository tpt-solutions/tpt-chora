#![forbid(unsafe_code)]

pub mod error;
pub mod foveated;
mod panned_source;
pub mod spatial_audio;
pub mod stereoscopic;
pub mod volumetric;

pub use error::SpatialError;
pub use foveated::{FoveatedRenderer, FoveationLevel, GazeTarget};
pub use spatial_audio::{AudioSource, HrtfParams, SpatialAudioEngine, SpatialAudioOutput};
pub use stereoscopic::{StereoEye, StereoGeometry, StereoView, StereoscopicRenderer};
pub use volumetric::{VolumetricLightPipeline, VolumetricParams};
