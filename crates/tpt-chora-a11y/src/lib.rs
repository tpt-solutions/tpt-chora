pub mod semantic;
pub mod bridge;
pub mod focus;
pub mod error;

pub use error::A11yError;
pub use semantic::{
    AccessibilityRole, AccessibilityState, SemanticIR, SemanticNode,
    SemanticNodeId,
};
pub use bridge::{A11yBridge, A11yBridgeEvent, A11yTreeUpdate};
pub use focus::{FocusTraversal, FocusDirection, FocusResult};
