use crate::semantic::{BridgeNode, SemanticIR};

pub struct A11yBridge {
    #[cfg(target_os = "windows")]
    uiautomation: Option<WindowsUIAutomation>,
    #[cfg(target_os = "macos")]
    nsaccessibility: Option<MacOSAccessibility>,
    #[cfg(target_os = "android")]
    uiautomator: Option<AndroidUIAutomator>,
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "android")))]
    stub: Option<StubBridge>,
}

#[derive(Debug, Clone)]
pub enum A11yBridgeEvent {
    FocusChanged(u64),
    ValueChanged(u64, String),
    StateChanged(u64),
    Announcement(String),
}

#[derive(Debug, Clone)]
pub struct A11yTreeUpdate {
    pub nodes: Vec<BridgeNode>,
    pub focused_node: Option<u64>,
}

#[cfg(target_os = "windows")]
struct WindowsUIAutomation;

#[cfg(target_os = "macos")]
struct MacOSAccessibility;

#[cfg(target_os = "android")]
struct AndroidUIAutomator;

struct StubBridge;

impl A11yBridge {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            uiautomation: Some(WindowsUIAutomation),
            #[cfg(target_os = "macos")]
            nsaccessibility: Some(MacOSAccessibility),
            #[cfg(target_os = "android")]
            uiautomator: Some(AndroidUIAutomator),
            #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "android")))]
            stub: Some(StubBridge),
        }
    }

    pub fn update_tree(&self, ir: &SemanticIR) -> Result<(), crate::A11yError> {
        let nodes = ir.serialize_for_bridge();
        let focused = ir.root().and_then(|r| r.children.first().map(|&id| id.0));
        let update = A11yTreeUpdate {
            nodes,
            focused_node: focused,
        };
        self.push_update(&update)
    }

    pub fn push_update(&self, _update: &A11yTreeUpdate) -> Result<(), crate::A11yError> {
        Ok(())
    }

    pub fn announce(&self, _message: &str) -> Result<(), crate::A11yError> {
        Ok(())
    }

    pub fn set_focus(&self, _node_id: u64) -> Result<(), crate::A11yError> {
        Ok(())
    }
}
