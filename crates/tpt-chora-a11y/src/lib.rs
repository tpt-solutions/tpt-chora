// `deny` rather than `forbid`: the optional `native-a11y-backends` feature
// calls real OS accessibility APIs (Windows UIA / macOS NSAccessibility /
// Android AccessibilityNodeInfo), which need `unsafe` FFI at their call
// sites (each annotated with its own `#[allow(unsafe_code)]` and a
// `// SAFETY:` justification) — everything else in this crate stays safe.
#![deny(unsafe_code)]

pub mod bridge;
pub mod error;
pub mod focus;
pub mod semantic;

pub use bridge::{A11yBridge, A11yBridgeEvent, A11yTreeUpdate};
pub use error::A11yError;
pub use focus::{FocusDirection, FocusResult, FocusTraversal};
pub use semantic::{
    AccessibilityRole, AccessibilityState, SemanticIR, SemanticNode, SemanticNodeId,
};
