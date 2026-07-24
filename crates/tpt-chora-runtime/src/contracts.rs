use glam::{Mat4, Vec4};

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
    root: Option<usize>,
}

impl ChoraVisualTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
        }
    }

    pub fn add_node(&mut self, node: ChoraVisualNode) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(node);
        if self.root.is_none() {
            self.root = Some(idx);
        }
        idx
    }

    pub fn nodes(&self) -> &[ChoraVisualNode] {
        &self.nodes
    }

    pub fn set_root(&mut self, idx: usize) {
        self.root = Some(idx);
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
    root: Option<usize>,
}

impl ChoraSemanticTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
        }
    }

    pub fn add_node(&mut self, node: ChoraSemanticNode) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(node);
        if self.root.is_none() {
            self.root = Some(idx);
        }
        idx
    }

    pub fn nodes(&self) -> &[ChoraSemanticNode] {
        &self.nodes
    }
}
