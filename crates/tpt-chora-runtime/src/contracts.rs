use glam::Mat4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GpuMeshHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GpuMaterialHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GpuTextureHandle(pub u64);

#[derive(Debug, Clone)]
pub struct ChoraVisualNode {
    pub transform: Mat4,
    pub geometry: GpuMeshHandle,
    pub material: GpuMaterialHandle,
    pub clip_mask: GpuTextureHandle,
    pub z_depth: f32,
    pub bounds: [f32; 4],
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub struct ChoraSemanticNode {
    pub role: AccessibilityRole,
    pub label: u64,
    pub state: u32,
    pub bounding_box_2d: [f32; 4],
    pub children: Vec<u64>,
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
    Generic,
}

pub struct ChoraVisualTree {
    nodes: Vec<ChoraVisualNode>,
    children: Vec<Vec<usize>>,
    parent: Vec<Option<usize>>,
    root: Option<usize>,
}

impl ChoraVisualTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            children: Vec::new(),
            parent: Vec::new(),
            root: None,
        }
    }

    pub fn add_node(&mut self, node: ChoraVisualNode) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(node);
        self.children.push(Vec::new());
        self.parent.push(None);
        if self.root.is_none() {
            self.root = Some(idx);
        }
        idx
    }

    pub fn add_child(&mut self, parent_idx: usize, node: ChoraVisualNode) -> usize {
        let child_idx = self.add_node(node);
        self.children[parent_idx].push(child_idx);
        self.parent[child_idx] = Some(parent_idx);
        child_idx
    }

    pub fn nodes(&self) -> &[ChoraVisualNode] {
        &self.nodes
    }

    pub fn set_root(&mut self, idx: usize) {
        self.root = Some(idx);
    }

    pub fn root(&self) -> Option<usize> {
        self.root
    }

    pub fn get_children(&self, idx: usize) -> &[usize] {
        &self.children[idx]
    }

    pub fn parent(&self, idx: usize) -> Option<usize> {
        self.parent[idx]
    }

    pub fn sort_by_z_depth(&mut self) {
        self.nodes.sort_by(|a, b| {
            a.z_depth
                .partial_cmp(&b.z_depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

pub struct ChoraSemanticTree {
    nodes: Vec<ChoraSemanticNode>,
    children: Vec<Vec<u64>>,
    parent: Vec<Option<u64>>,
    root: Option<usize>,
}

impl ChoraSemanticTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            children: Vec::new(),
            parent: Vec::new(),
            root: None,
        }
    }

    pub fn add_node(&mut self, node: ChoraSemanticNode) -> usize {
        let idx = self.nodes.len();
        let node_children = node.children.clone();
        self.nodes.push(node);
        self.children.push(node_children);
        self.parent.push(None);
        if self.root.is_none() {
            self.root = Some(idx);
        }
        idx
    }

    pub fn add_child(&mut self, parent_idx: usize, node: ChoraSemanticNode) -> usize {
        let child_idx = self.add_node(node);
        let child_node_id = self.nodes[child_idx].label;
        self.children[parent_idx].push(child_node_id);
        self.parent[child_idx] = Some(self.nodes[parent_idx].label);
        child_idx
    }

    pub fn nodes(&self) -> &[ChoraSemanticNode] {
        &self.nodes
    }

    pub fn root(&self) -> Option<usize> {
        self.root
    }

    pub fn get_children(&self, idx: usize) -> &[u64] {
        self.children.get(idx).map(|c| c.as_slice()).unwrap_or(&[])
    }

    pub fn parent(&self, idx: usize) -> Option<u64> {
        self.parent.get(idx).copied().flatten()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}
