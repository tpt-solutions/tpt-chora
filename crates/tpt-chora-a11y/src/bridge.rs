use crate::semantic::{BridgeNode, SemanticIR};

pub struct A11yBridge {
    last_update: Option<A11yTreeUpdate>,
    announcements: Vec<String>,
    focused_node: Option<u64>,
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
        Self {
            last_update: None,
            announcements: Vec::new(),
            focused_node: None,
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
        Ok(())
    }

    pub fn announce(&mut self, message: &str) -> Result<(), crate::A11yError> {
        self.announcements.push(message.to_string());
        Ok(())
    }

    pub fn set_focus(&mut self, node_id: u64) -> Result<(), crate::A11yError> {
        if let Some(ref update) = self.last_update {
            if !update.nodes.iter().any(|n| n.id == node_id) {
                return Err(crate::A11yError::NodeNotFound(node_id));
            }
        }
        self.focused_node = Some(node_id);
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
