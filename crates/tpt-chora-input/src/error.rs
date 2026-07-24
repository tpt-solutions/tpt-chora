#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("no device available for input type")]
    NoDevice,
    #[error("hit test buffer creation failed")]
    HitTestBufferCreation,
    #[error("haptic feedback not supported on this platform")]
    HapticNotSupported,
}
