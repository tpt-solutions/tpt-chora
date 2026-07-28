#[derive(Debug, thiserror::Error)]
pub enum CompatError {
    #[error("CSS parse error at line {line}: {message}")]
    CssParseError { line: u32, message: String },
    #[error("transpile failed: {0}")]
    TranspileFailed(String),
    #[error("safety violation: {0}")]
    SafetyViolation(String),
    #[error("Wasm module load failed: {0}")]
    WasmLoadFailed(String),
    #[error("FFI bridge error: {0}")]
    FfiError(String),
    #[error("Wasm module exceeded its resource budget: {0}")]
    ResourceLimitExceeded(String),
}
