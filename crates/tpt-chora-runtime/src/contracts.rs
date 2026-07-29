use glam::Mat4;
pub use tpt_chora_a11y::semantic::AccessibilityRole;
pub use tpt_chora_render::HierarchicalZDepth;
pub use tpt_chora_render::ZDepthViolation;

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
    z_depth: f32,
    pub bounds: [f32; 4],
    pub visible: bool,
}

impl ChoraVisualNode {
    pub fn new(
        transform: Mat4,
        geometry: GpuMeshHandle,
        material: GpuMaterialHandle,
        clip_mask: GpuTextureHandle,
        bounds: [f32; 4],
    ) -> Self {
        Self {
            transform,
            geometry,
            material,
            clip_mask,
            z_depth: 0.0,
            bounds,
            visible: true,
        }
    }

    pub fn z_depth(&self) -> f32 {
        self.z_depth
    }

    pub fn set_z_depth(
        &mut self,
        z_depth_system: &HierarchicalZDepth,
        parent_z: f32,
        sibling_index: u32,
        has_modal_capability: bool,
    ) -> Result<(), ZDepthViolation> {
        self.z_depth = z_depth_system.compute_z(parent_z, sibling_index, has_modal_capability)?;
        Ok(())
    }

    /// Test-only escape hatch that sets `z_depth` directly, bypassing
    /// `HierarchicalZDepth::compute_z`'s modal-capability check. `cfg(test)`
    /// keeps this out of the compiled library entirely so no external caller
    /// can use it to bypass the security gate in real code — only this
    /// crate's own unit tests may construct nodes with an arbitrary z_depth.
    #[cfg(test)]
    fn with_z_depth_raw(mut self, z: f32) -> Self {
        self.z_depth = z;
        self
    }
}

#[derive(Debug, Clone)]
pub struct ChoraSemanticNode {
    pub role: AccessibilityRole,
    pub label: u64,
    pub state: u32,
    pub bounding_box_2d: [f32; 4],
    pub children: Vec<u64>,
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
            a.z_depth()
                .partial_cmp(&b.z_depth())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

impl Default for ChoraVisualTree {
    fn default() -> Self {
        Self::new()
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

impl Default for ChoraSemanticTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visual_node(z: f32) -> ChoraVisualNode {
        ChoraVisualNode::new(
            Mat4::IDENTITY,
            GpuMeshHandle(0),
            GpuMaterialHandle(0),
            GpuTextureHandle(0),
            [0.0; 4],
        )
        .with_z_depth_raw(z)
    }

    fn semantic_node(label: u64) -> ChoraSemanticNode {
        ChoraSemanticNode {
            role: AccessibilityRole::Button,
            label,
            state: 0,
            bounding_box_2d: [0.0; 4],
            children: Vec::new(),
        }
    }

    #[test]
    fn visual_tree_new_empty() {
        let t = ChoraVisualTree::new();
        assert!(t.nodes().is_empty());
        assert_eq!(t.root(), None);
    }

    #[test]
    fn visual_tree_add_node() {
        let mut t = ChoraVisualTree::new();
        let idx = t.add_node(visual_node(0.0));
        assert_eq!(idx, 0);
        assert_eq!(t.nodes().len(), 1);
        assert_eq!(t.root(), Some(0));
    }

    #[test]
    fn visual_tree_add_child() {
        let mut t = ChoraVisualTree::new();
        let parent = t.add_node(visual_node(0.0));
        let child = t.add_child(parent, visual_node(1.0));
        assert_eq!(t.get_children(parent), &[child]);
    }

    #[test]
    fn visual_tree_parent() {
        let mut t = ChoraVisualTree::new();
        let parent = t.add_node(visual_node(0.0));
        let child = t.add_child(parent, visual_node(1.0));
        assert_eq!(t.parent(child), Some(parent));
    }

    #[test]
    fn visual_tree_set_root() {
        let mut t = ChoraVisualTree::new();
        t.add_node(visual_node(0.0));
        let second = t.add_node(visual_node(1.0));
        t.set_root(second);
        assert_eq!(t.root(), Some(second));
    }

    #[test]
    fn visual_tree_sort_by_z_depth() {
        let mut t = ChoraVisualTree::new();
        t.add_node(visual_node(2.0));
        t.add_node(visual_node(1.0));
        t.add_node(visual_node(3.0));
        t.sort_by_z_depth();
        let depths: Vec<f32> = t.nodes().iter().map(|n| n.z_depth()).collect();
        assert_eq!(depths, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn semantic_tree_new_empty() {
        let t = ChoraSemanticTree::new();
        assert_eq!(t.node_count(), 0);
        assert_eq!(t.root(), None);
    }

    #[test]
    fn semantic_tree_add_node() {
        let mut t = ChoraSemanticTree::new();
        let idx = t.add_node(semantic_node(42));
        assert_eq!(idx, 0);
        assert_eq!(t.node_count(), 1);
        assert_eq!(t.root(), Some(0));
    }

    #[test]
    fn semantic_tree_get_children() {
        let mut t = ChoraSemanticTree::new();
        let parent = t.add_node(semantic_node(10));
        let child = t.add_child(parent, semantic_node(20));
        let children = t.get_children(parent);
        assert_eq!(children, &[20]);
        assert_eq!(child, 1);
    }

    #[test]
    fn semantic_tree_parent() {
        let mut t = ChoraSemanticTree::new();
        let parent = t.add_node(semantic_node(10));
        let child = t.add_child(parent, semantic_node(20));
        assert_eq!(t.parent(child), Some(10));
    }
}
