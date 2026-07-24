#[derive(Debug, thiserror::Error)]
pub enum A11yError {
    #[error("semantic tree has no root node")]
    NoRoot,
    #[error("node not found: {0}")]
    NodeNotFound(u64),
    #[error("focus traversal would trap the user")]
    FocusTrap,
    #[error("OS accessibility bridge not available on this platform")]
    BridgeUnavailable,
    #[error("semantic node has invalid role transition")]
    InvalidRoleTransition,
}
