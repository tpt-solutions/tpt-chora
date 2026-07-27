use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct AccessibilityState: u32 {
        const FOCUSED = 0b0000_0001;
        const DISABLED = 0b0000_0010;
        const EXPANDED = 0b0000_0100;
        const COLLAPSED = 0b0000_1000;
        const CHECKED = 0b0001_0000;
        const UNCHECKED = 0b0010_0000;
        const INDETERMINATE = 0b0100_0000;
        const SELECTED = 0b1000_0000;
        const HIDDEN = 0b1_0000_0000;
        const READ_ONLY = 0b10_0000_0000;
        const REQUIRED = 0b100_0000_0000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessibilityRole {
    Button,
    Link,
    Heading,
    Text,
    Image,
    TextField,
    TextArea,
    CheckBox,
    RadioButton,
    Slider,
    ProgressBar,
    ComboBox,
    ListBox,
    MenuItem,
    Menu,
    Tab,
    TabPanel,
    Dialog,
    AlertDialog,
    Document,
    Group,
    Region,
    Table,
    TableRow,
    TableCell,
    List,
    ListItem,
    Tree,
    TreeItem,
    Toolbar,
    StatusIndicator,
    Separator,
    Scrollbar,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemanticNodeId(pub u64);

#[derive(Debug, Clone)]
pub struct SemanticNode {
    pub id: SemanticNodeId,
    pub role: AccessibilityRole,
    pub label: String,
    pub description: String,
    pub state: AccessibilityState,
    pub bounds: [f32; 4],
    pub children: Vec<SemanticNodeId>,
    pub parent: Option<SemanticNodeId>,
    pub z_depth: f32,
}

pub struct SemanticIR {
    nodes: Vec<SemanticNode>,
    root_id: Option<SemanticNodeId>,
}

impl SemanticIR {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root_id: None,
        }
    }

    pub fn add_node(&mut self, node: SemanticNode) -> SemanticNodeId {
        let id = node.id;
        self.nodes.push(node);
        if self.root_id.is_none() {
            self.root_id = Some(id);
        }
        id
    }

    pub fn set_root(&mut self, id: SemanticNodeId) {
        self.root_id = Some(id);
    }

    pub fn get_node(&self, id: SemanticNodeId) -> Option<&SemanticNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_node_mut(&mut self, id: SemanticNodeId) -> Option<&mut SemanticNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn root(&self) -> Option<&SemanticNode> {
        self.root_id.and_then(|id| self.get_node(id))
    }

    pub fn nodes(&self) -> &[SemanticNode] {
        &self.nodes
    }

    pub fn flatten_tree(&self) -> Vec<&SemanticNode> {
        let mut result = Vec::new();
        if let Some(root) = self.root() {
            self.flatten_subtree(root, &mut result);
        }
        result
    }

    fn flatten_subtree<'a>(&'a self, node: &'a SemanticNode, result: &mut Vec<&'a SemanticNode>) {
        result.push(node);
        for &child_id in &node.children {
            if let Some(child) = self.get_node(child_id) {
                self.flatten_subtree(child, result);
            }
        }
    }

    pub fn serialize_for_bridge(&self) -> Vec<BridgeNode> {
        self.flatten_tree()
            .iter()
            .map(|node| BridgeNode {
                id: node.id.0,
                role: node.role,
                label: node.label.clone(),
                state: node.state,
                bounds: node.bounds,
                children: node.children.iter().map(|id| id.0).collect(),
            })
            .collect()
    }
}

impl Default for SemanticIR {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct BridgeNode {
    pub id: u64,
    pub role: AccessibilityRole,
    pub label: String,
    pub state: AccessibilityState,
    pub bounds: [f32; 4],
    pub children: Vec<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: u64) -> SemanticNode {
        SemanticNode {
            id: SemanticNodeId(id),
            role: AccessibilityRole::Generic,
            label: format!("node_{id}"),
            description: String::new(),
            state: AccessibilityState::empty(),
            bounds: [0.0, 0.0, 100.0, 100.0],
            children: Vec::new(),
            parent: None,
            z_depth: 0.0,
        }
    }

    fn make_node_with_children(id: u64, children: Vec<SemanticNodeId>) -> SemanticNode {
        let mut node = make_node(id);
        node.children = children;
        node
    }

    #[test]
    fn new_creates_empty_ir() {
        let ir = SemanticIR::new();
        assert!(ir.root_id.is_none());
        assert!(ir.nodes().is_empty());
    }

    #[test]
    fn add_node_single_becomes_root() {
        let mut ir = SemanticIR::new();
        let id = ir.add_node(make_node(1));
        assert_eq!(id, SemanticNodeId(1));
        assert_eq!(ir.nodes().len(), 1);
        assert_eq!(ir.root_id, Some(SemanticNodeId(1)));
    }

    #[test]
    fn add_node_second_preserves_root() {
        let mut ir = SemanticIR::new();
        ir.add_node(make_node(1));
        ir.add_node(make_node(2));
        assert_eq!(ir.nodes().len(), 2);
        assert_eq!(ir.root_id, Some(SemanticNodeId(1)));
    }

    #[test]
    fn set_root_changes_root() {
        let mut ir = SemanticIR::new();
        ir.add_node(make_node(1));
        ir.add_node(make_node(2));
        ir.set_root(SemanticNodeId(2));
        assert_eq!(ir.root_id, Some(SemanticNodeId(2)));
        assert_eq!(ir.root().unwrap().id, SemanticNodeId(2));
    }

    #[test]
    fn get_node_found() {
        let mut ir = SemanticIR::new();
        ir.add_node(make_node(42));
        assert!(ir.get_node(SemanticNodeId(42)).is_some());
    }

    #[test]
    fn get_node_not_found() {
        let mut ir = SemanticIR::new();
        ir.add_node(make_node(1));
        assert!(ir.get_node(SemanticNodeId(999)).is_none());
    }

    #[test]
    fn get_node_mut_modifies_label() {
        let mut ir = SemanticIR::new();
        ir.add_node(make_node(1));
        let node = ir.get_node_mut(SemanticNodeId(1)).unwrap();
        node.label = "modified".to_string();
        assert_eq!(ir.get_node(SemanticNodeId(1)).unwrap().label, "modified");
    }

    #[test]
    fn root_returns_first_added_node() {
        let mut ir = SemanticIR::new();
        ir.add_node(make_node(10));
        ir.add_node(make_node(20));
        assert_eq!(ir.root().unwrap().id, SemanticNodeId(10));
    }

    #[test]
    fn flatten_tree_single_node() {
        let mut ir = SemanticIR::new();
        ir.add_node(make_node(1));
        let flat = ir.flatten_tree();
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].id, SemanticNodeId(1));
    }

    #[test]
    fn flatten_tree_hierarchy_depth_first() {
        let mut ir = SemanticIR::new();
        let child1 = SemanticNodeId(2);
        let child2 = SemanticNodeId(3);
        ir.add_node(make_node_with_children(1, vec![child1, child2]));
        ir.add_node(make_node(2));
        ir.add_node(make_node(3));
        let flat = ir.flatten_tree();
        assert_eq!(flat.len(), 3);
        assert_eq!(flat[0].id, SemanticNodeId(1));
        assert_eq!(flat[1].id, SemanticNodeId(2));
        assert_eq!(flat[2].id, SemanticNodeId(3));
    }

    #[test]
    fn serialize_for_bridge_matches_fields() {
        let mut ir = SemanticIR::new();
        let mut node = make_node(10);
        node.role = AccessibilityRole::Button;
        node.label = "Submit".to_string();
        node.state = AccessibilityState::FOCUSED | AccessibilityState::REQUIRED;
        node.bounds = [10.0, 20.0, 200.0, 50.0];
        node.children = vec![SemanticNodeId(20)];
        ir.add_node(node);
        ir.add_node(make_node(20));

        let bridge = ir.serialize_for_bridge();
        assert_eq!(bridge.len(), 2);

        let root_bridge = &bridge[0];
        assert_eq!(root_bridge.id, 10);
        assert_eq!(root_bridge.role, AccessibilityRole::Button);
        assert_eq!(root_bridge.label, "Submit");
        assert!(root_bridge.state.contains(AccessibilityState::FOCUSED));
        assert!(root_bridge.state.contains(AccessibilityState::REQUIRED));
        assert_eq!(root_bridge.bounds, [10.0, 20.0, 200.0, 50.0]);
        assert_eq!(root_bridge.children, vec![20]);

        let child_bridge = &bridge[1];
        assert_eq!(child_bridge.id, 20);
        assert!(child_bridge.children.is_empty());
    }
}
