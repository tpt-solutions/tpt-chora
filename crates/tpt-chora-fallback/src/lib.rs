#![forbid(unsafe_code)]

pub mod dynamic_fidelity;
mod encoding;
pub mod error;
pub mod headless;
pub mod software;

pub use dynamic_fidelity::{DynamicFidelity, FidelityLevel, FidelityProfile, FidelitySettings};
pub use error::FallbackError;
pub use headless::{HeadlessConfig, HeadlessRenderer, OutputFormat};
pub use software::{Command, SoftwareRasterizer, SoftwareRenderer};
