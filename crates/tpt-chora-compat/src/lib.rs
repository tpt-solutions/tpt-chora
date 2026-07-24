pub mod deployment;
pub mod css_parser;
pub mod eidos_transpiler;
pub mod web_component;
pub mod ffi_bridge;
pub mod error;

pub use error::CompatError;
pub use css_parser::{CssParser, ParsedCss, CssRule};
pub use eidos_transpiler::{EidosTranspiler, TranspileResult, Violation};
pub use web_component::{WebComponentConfig, ComponentBridge};
pub use ffi_bridge::{FfiBridge, WasmModule};
