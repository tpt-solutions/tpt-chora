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

#[derive(Debug, Clone)]
pub struct BridgeNode {
    pub id: u64,
    pub role: AccessibilityRole,
    pub label: String,
    pub state: AccessibilityState,
    pub bounds: [f32; 4],
    pub children: Vec<u64>,
}
