pub mod dynamic_fidelity;
pub mod error;
pub mod headless;

pub use dynamic_fidelity::{DynamicFidelity, FidelityLevel, FidelityProfile, FidelitySettings};
pub use error::FallbackError;
pub use headless::{HeadlessConfig, HeadlessRenderer, OutputFormat};
