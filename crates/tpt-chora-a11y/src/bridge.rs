use crate::semantic::{BridgeNode, SemanticIR};

pub struct A11yBridge {
    last_update: Option<A11yTreeUpdate>,
    announcements: Vec<String>,
    focused_node: Option<u64>,
    os_backend: OsBackend,
}

enum OsBackend {
    WindowsUiAutomation,
    MacOsNsAccessibility,
    AndroidAccessibilityNodeInfo,
    InMemory,
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

impl A11yBridge {
    pub fn new() -> Self {
        let os_backend = if cfg!(target_os = "windows") {
            OsBackend::WindowsUiAutomation
        } else if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
            OsBackend::MacOsNsAccessibility
        } else if cfg!(target_os = "android") {
            OsBackend::AndroidAccessibilityNodeInfo
        } else {
            OsBackend::InMemory
        };
        Self {
            last_update: None,
            announcements: Vec::new(),
            focused_node: None,
            os_backend,
        }
    }

    pub fn update_tree(&mut self, ir: &SemanticIR) -> Result<(), crate::A11yError> {
        let nodes = ir.serialize_for_bridge();
        let focused = ir.root().and_then(|r| r.children.first().map(|&id| id.0));
        let update = A11yTreeUpdate {
            nodes,
            focused_node: focused,
        };
        self.push_update(&update)
    }

    pub fn push_update(&mut self, update: &A11yTreeUpdate) -> Result<(), crate::A11yError> {
        if let Some(focused_id) = update.focused_node {
            if !update.nodes.iter().any(|n| n.id == focused_id) {
                return Err(crate::A11yError::NodeNotFound(focused_id));
            }
        }

        self.focused_node = update.focused_node;
        self.last_update = Some(update.clone());

        match &self.os_backend {
            OsBackend::WindowsUiAutomation => {
                self.sync_to_windows_uia(&update.nodes);
            }
            OsBackend::MacOsNsAccessibility => {
                self.sync_to_macos_nsaccessibility(&update.nodes);
            }
            OsBackend::AndroidAccessibilityNodeInfo => {
                self.sync_to_android_node_info(&update.nodes);
            }
            OsBackend::InMemory => {}
        }

        Ok(())
    }

    fn sync_to_windows_uia(&self, _nodes: &[BridgeNode]) {
        // Windows UIAutomation integration:
        // Create IRawElementProviderSimple for each BridgeNode.
        // Set IAccessibleValue, IAccessible states, and
        // UIA_ControlTypePropertyId from AccessibilityRole.
        // Call IRawElementProviderFragmentRoot::Navigate for tree structure.
    }

    fn sync_to_macos_nsaccessibility(&self, _nodes: &[BridgeNode]) {
        // macOS NSAccessibility integration:
        // For each BridgeNode, set accessibilityRole, accessibilityLabel,
        // accessibilityValue, accessibilityFrame on the NSView.
        // Post NSAccessibilityNotificationPostedNotification for
        // focus/value changes.
    }

    fn sync_to_android_node_info(&self, _nodes: &[BridgeNode]) {
        // Android AccessibilityNodeInfo integration:
        // For each BridgeNode, create AccessibilityNodeInfoCompat,
        // set className/role, contentDescription/label, stateDescription,
        // and childNodeIds. Post via AccessibilityManager.sendAccessibilityEvent.
    }

    pub fn announce(&mut self, message: &str) -> Result<(), crate::A11yError> {
        self.announcements.push(message.to_string());
        match &self.os_backend {
            OsBackend::WindowsUiAutomation => {
                // IUIAutomation::RaiseAutomationEvent(UIA_AutomationPropertyChangedEventId)
            }
            OsBackend::MacOsNsAccessibility => {
                // NSAccessibilityPostNotification(NSAccessibilityAnnouncementRequestedNotification)
            }
            OsBackend::AndroidAccessibilityNodeInfo => {
                // AccessibilityManager.sendAccessibilityEvent(TYPE_ANNOUNCEMENT)
            }
            OsBackend::InMemory => {}
        }
        Ok(())
    }

    pub fn set_focus(&mut self, node_id: u64) -> Result<(), crate::A11yError> {
        if let Some(ref update) = self.last_update {
            if !update.nodes.iter().any(|n| n.id == node_id) {
                return Err(crate::A11yError::NodeNotFound(node_id));
            }
        }
        self.focused_node = Some(node_id);
        match &self.os_backend {
            OsBackend::WindowsUiAutomation => {
                // IUIAutomation::SetFocus() on the element matching node_id
            }
            OsBackend::MacOsNsAccessibility => {
                // NSAccessibilityPostNotification(element, NSAccessibilityFocusedUIElementChangedNotification)
            }
            OsBackend::AndroidAccessibilityNodeInfo => {
                // AccessibilityNodeInfoCompat.performAction(ACTION_ACCESSIBILITY_FOCUS)
            }
            OsBackend::InMemory => {}
        }
        Ok(())
    }

    pub fn last_update(&self) -> Option<&A11yTreeUpdate> {
        self.last_update.as_ref()
    }

    pub fn announcements(&self) -> &[String] {
        &self.announcements
    }

    pub fn focused_node(&self) -> Option<u64> {
        self.focused_node
    }

    pub fn drain_announcements(&mut self) -> Vec<String> {
        std::mem::take(&mut self.announcements)
    }
}

impl Default for A11yBridge {
    fn default() -> Self {
        Self::new()
    }
}
