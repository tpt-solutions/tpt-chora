pub mod headless;
pub mod dynamic_fidelity;
pub mod error;

pub use error::FallbackError;
pub use headless::{HeadlessRenderer, HeadlessConfig, OutputFormat};
pub use dynamic_fidelity::{FidelityLevel, FidelityProfile, DynamicFidelity};
