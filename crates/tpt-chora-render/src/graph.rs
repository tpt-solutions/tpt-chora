//! The frame-scoped, dependency-tracked render graph (spec.txt §2.1).
//!
//! Nodes declare the transient resources they read, write, and create by
//! name. The graph topologically sorts nodes so producers always run before
//! their consumers, allocates each transient resource exactly once per
//! `execute`, and records every pass into a single command buffer so the
//! whole frame is submitted as one GPU submission.

use std::collections::{HashMap, HashSet};

use crate::error::RenderError;
use crate::security::{CapabilityGuard, CapabilityToken, SecurityContext};

/// A named handle to a transient resource owned by the graph for one execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub &'static str);

/// Description of a transient GPU texture a node wants the graph to allocate.
#[derive(Debug, Clone)]
pub struct TransientTextureDesc {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
}

/// A resolved transient texture, handed to nodes at execute time.
#[derive(Debug)]
pub struct TransientTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

/// Context passed to a node's execute closure.
///
/// `resources` is a plain public field rather than hidden behind an
/// accessor method: nodes routinely need to read `resources` (to look up
/// an input texture view) and mutably borrow `encoder` (to record into
/// it) in the same statement, and only direct field access lets the
/// borrow checker see those two borrows as disjoint.
#[derive(Debug)]
pub struct NodeExecuteCtx<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub resources: &'a HashMap<ResourceId, TransientTexture>,
}

impl<'a> NodeExecuteCtx<'a> {
    pub fn resource(&self, id: ResourceId) -> &TransientTexture {
        self.resources
            .get(&id)
            .unwrap_or_else(|| panic!("render graph: resource {:?} not allocated", id))
    }
}

type ExecuteFn = Box<dyn FnMut(&mut NodeExecuteCtx<'_>)>;

/// A single pass in the render graph.
pub struct GraphNode {
    pub name: &'static str,
    pub reads: Vec<ResourceId>,
    pub writes: Vec<ResourceId>,
    pub creates: Vec<(ResourceId, TransientTextureDesc)>,
    /// Declarative capability requirements: the `CapabilityToken`s this
    /// node's execute closure needs in order to legitimately access a
    /// given resource. Unlike `reads`/`writes`/`creates` (which describe
    /// resource *dependencies* for scheduling), this describes *security*
    /// requirements, and is pure data — checkable by `RenderGraph::lint_capabilities`
    /// without a GPU device or ever running the closure.
    pub requires: Vec<(ResourceId, CapabilityToken)>,
    execute: ExecuteFn,
}

impl GraphNode {
    pub fn new(name: &'static str, execute: impl FnMut(&mut NodeExecuteCtx<'_>) + 'static) -> Self {
        Self {
            name,
            reads: Vec::new(),
            writes: Vec::new(),
            creates: Vec::new(),
            requires: Vec::new(),
            execute: Box::new(execute),
        }
    }

    pub fn reads(mut self, ids: impl IntoIterator<Item = ResourceId>) -> Self {
        self.reads.extend(ids);
        self
    }

    pub fn writes(mut self, ids: impl IntoIterator<Item = ResourceId>) -> Self {
        self.writes.extend(ids);
        self
    }

    pub fn creates(mut self, id: ResourceId, desc: TransientTextureDesc) -> Self {
        self.creates.push((id, desc));
        self
    }

    pub fn requires(mut self, id: ResourceId, token: CapabilityToken) -> Self {
        self.requires.push((id, token));
        self
    }
}

/// A single failed declarative capability check found by
/// `RenderGraph::lint_capabilities`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityLintViolation {
    pub node: &'static str,
    pub resource: ResourceId,
    pub missing: CapabilityToken,
}

impl std::fmt::Display for CapabilityLintViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "node {:?} requires capability {:?} on resource {:?}, which its guard does not grant",
            self.node, self.missing, self.resource
        )
    }
}

impl std::error::Error for CapabilityLintViolation {}

/// The render graph: a set of nodes plus their declared resource dependencies.
#[derive(Default)]
pub struct RenderGraph {
    nodes: Vec<GraphNode>,
    resources: HashMap<ResourceId, TransientTexture>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            resources: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.push(node);
    }

    /// Topologically sorts nodes so every writer of a resource runs before
    /// any reader of that resource. Returns an error on cyclic dependencies.
    fn topo_order(&self) -> Result<Vec<usize>, RenderError> {
        let n = self.nodes.len();

        // producer_of[resource] = index of the node that creates/writes it.
        let mut producer_of: HashMap<ResourceId, usize> = HashMap::new();
        for (i, node) in self.nodes.iter().enumerate() {
            for (id, _) in &node.creates {
                producer_of.insert(*id, i);
            }
            for id in &node.writes {
                producer_of.entry(*id).or_insert(i);
            }
        }

        // Edge i -> j means "i must run before j".
        let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut in_degree = vec![0usize; n];
        for (j, node) in self.nodes.iter().enumerate() {
            for id in &node.reads {
                if let Some(&i) = producer_of.get(id) {
                    if i != j {
                        adjacency[i].push(j);
                        in_degree[j] += 1;
                    }
                }
            }
        }

        let mut queue: std::collections::VecDeque<usize> =
            (0..n).filter(|&i| in_degree[i] == 0).collect();
        let mut order = Vec::with_capacity(n);
        let mut visited = HashSet::new();

        while let Some(i) = queue.pop_front() {
            if !visited.insert(i) {
                continue;
            }
            order.push(i);
            for &j in &adjacency[i] {
                in_degree[j] -= 1;
                if in_degree[j] == 0 {
                    queue.push_back(j);
                }
            }
        }

        if order.len() != n {
            return Err(RenderError::GraphCycle);
        }
        Ok(order)
    }

    /// Allocates every declared transient resource, then runs each node's
    /// pass in dependency order, all inside one command encoder/submission.
    /// If `security` is provided, each node's resource access is validated
    /// against the capability guard before execution.
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        security: Option<&SecurityContext>,
    ) -> Result<(), RenderError> {
        if let Some(sec) = security {
            if let Err(violations) = self.lint_capabilities(&sec.capability) {
                let message = violations
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(RenderError::SecurityViolation(message));
            }
        }

        let order = self.topo_order()?;

        let mut resources: HashMap<ResourceId, TransientTexture> = HashMap::new();
        for node in &self.nodes {
            for (id, desc) in &node.creates {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(id.0),
                    size: wgpu::Extent3d {
                        width: desc.width,
                        height: desc.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: desc.format,
                    usage: desc.usage,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                resources.insert(*id, TransientTexture { texture, view });
            }
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("chora-render-graph-encoder"),
        });

        for &i in &order {
            if let Some(sec) = security {
                let node = &self.nodes[i];
                // A node is implicitly granted access to the resources it
                // creates (it's their producer/owner for this frame) before
                // its — or any later node's — reads/writes are validated,
                // so a downstream node can legitimately read what an
                // earlier node in the same graph produced.
                for (id, _) in &node.creates {
                    sec.capability
                        .grant_texture(id.0.as_ptr() as u64)
                        .map_err(|e| RenderError::SecurityViolation(e.to_string()))?;
                }
                let texture_ids: Vec<u64> = node
                    .reads
                    .iter()
                    .chain(node.writes.iter())
                    .map(|id| id.0.as_ptr() as u64)
                    .collect();
                sec.validate_node(&texture_ids, &[])?;
            }
            let node = &mut self.nodes[i];
            let mut ctx = NodeExecuteCtx {
                device,
                queue,
                encoder: &mut encoder,
                resources: &resources,
            };
            (node.execute)(&mut ctx);
        }

        queue.submit(std::iter::once(encoder.finish()));
        self.resources = resources;
        Ok(())
    }

    /// Build-time capability lint: checks every node's declared `requires`
    /// against `guard` without allocating any GPU resources, running any
    /// node's execute closure, or otherwise needing a `wgpu::Device` at
    /// all. Catches a component declaring a resource dependency without
    /// the capability token its closure will actually need at runtime,
    /// before the graph is ever executed.
    pub fn lint_capabilities(
        &self,
        guard: &CapabilityGuard,
    ) -> Result<(), Vec<CapabilityLintViolation>> {
        let violations: Vec<CapabilityLintViolation> = self
            .nodes
            .iter()
            .flat_map(|node| {
                node.requires.iter().filter_map(move |(id, token)| {
                    if guard.has_token(*token) {
                        None
                    } else {
                        Some(CapabilityLintViolation {
                            node: node.name,
                            resource: *id,
                            missing: *token,
                        })
                    }
                })
            })
            .collect();

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

// Resources stay alive on the graph after `execute` so callers can read back
// the final output texture before the next frame's `execute` reallocates them.
impl RenderGraph {
    pub fn texture(&self, id: ResourceId) -> Option<&wgpu::Texture> {
        self.resources.get(&id).map(|t| &t.texture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop_node(name: &'static str) -> GraphNode {
        GraphNode::new(name, |_| {})
    }

    #[test]
    fn empty_graph_topo_order() {
        let graph = RenderGraph::new();
        assert_eq!(graph.topo_order().unwrap(), Vec::<usize>::new());
    }

    #[test]
    fn single_node_no_deps() {
        let mut graph = RenderGraph::new();
        graph.add_node(noop_node("a"));
        let order = graph.topo_order().unwrap();
        assert_eq!(order, vec![0]);
    }

    #[test]
    fn acyclic_two_nodes_ordering() {
        let mut graph = RenderGraph::new();

        let a = GraphNode::new("a", |_| {}).creates(ResourceId("x"), dummy_tex_desc());
        let b = GraphNode::new("b", |_| {}).reads(vec![ResourceId("x")]);

        graph.add_node(a);
        graph.add_node(b);

        let order = graph.topo_order().unwrap();
        assert_eq!(order, vec![0, 1]);
    }

    #[test]
    fn cycle_detection() {
        let mut graph = RenderGraph::new();

        let a = GraphNode::new("a", |_| {})
            .creates(ResourceId("x"), dummy_tex_desc())
            .reads(vec![ResourceId("y")]);
        let b = GraphNode::new("b", |_| {})
            .creates(ResourceId("y"), dummy_tex_desc())
            .reads(vec![ResourceId("x")]);

        graph.add_node(a);
        graph.add_node(b);

        assert!(matches!(graph.topo_order(), Err(RenderError::GraphCycle)));
    }

    #[test]
    fn diamond_dependency() {
        let mut graph = RenderGraph::new();

        let a = GraphNode::new("a", |_| {}).creates(ResourceId("x"), dummy_tex_desc());
        let b = GraphNode::new("b", |_| {})
            .reads(vec![ResourceId("x")])
            .creates(ResourceId("y"), dummy_tex_desc());
        let c = GraphNode::new("c", |_| {}).reads(vec![ResourceId("x"), ResourceId("y")]);

        graph.add_node(a);
        graph.add_node(b);
        graph.add_node(c);

        let order = graph.topo_order().unwrap();
        let pos_of = |idx: usize| order.iter().position(|&i| i == idx).unwrap();

        assert!(pos_of(0) < pos_of(1));
        assert!(pos_of(0) < pos_of(2));
        assert!(pos_of(1) < pos_of(2));
    }

    #[test]
    fn self_dependency_ignored() {
        let mut graph = RenderGraph::new();

        let a = GraphNode::new("a", |_| {})
            .creates(ResourceId("x"), dummy_tex_desc())
            .reads(vec![ResourceId("x")]);

        graph.add_node(a);

        let order = graph.topo_order().unwrap();
        assert_eq!(order, vec![0]);
    }

    fn dummy_tex_desc() -> TransientTextureDesc {
        TransientTextureDesc {
            width: 1,
            height: 1,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        }
    }

    #[test]
    fn lint_capabilities_passes_when_guard_grants_required_tokens() {
        let mut graph = RenderGraph::new();
        let node = GraphNode::new("a", |_| {})
            .creates(ResourceId("x"), dummy_tex_desc())
            .requires(ResourceId("x"), CapabilityToken::TEXTURE_READ);
        graph.add_node(node);

        let guard = CapabilityGuard::new(0, CapabilityToken::TEXTURE_READ);
        assert!(graph.lint_capabilities(&guard).is_ok());
    }

    #[test]
    fn lint_capabilities_reports_missing_token() {
        let mut graph = RenderGraph::new();
        let node = GraphNode::new("a", |_| {})
            .creates(ResourceId("x"), dummy_tex_desc())
            .requires(ResourceId("x"), CapabilityToken::TEXTURE_WRITE);
        graph.add_node(node);

        // Guard only has TEXTURE_READ, not the TEXTURE_WRITE the node requires.
        let guard = CapabilityGuard::new(0, CapabilityToken::TEXTURE_READ);
        let violations = graph.lint_capabilities(&guard).unwrap_err();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].node, "a");
        assert_eq!(violations[0].resource, ResourceId("x"));
        assert_eq!(violations[0].missing, CapabilityToken::TEXTURE_WRITE);
        assert!(violations[0].to_string().contains("TEXTURE_WRITE"));
    }

    #[test]
    fn lint_capabilities_reports_every_violation_not_just_the_first() {
        let mut graph = RenderGraph::new();
        let a = GraphNode::new("a", |_| {})
            .creates(ResourceId("x"), dummy_tex_desc())
            .requires(ResourceId("x"), CapabilityToken::TEXTURE_WRITE);
        let b = GraphNode::new("b", |_| {})
            .creates(ResourceId("y"), dummy_tex_desc())
            .requires(ResourceId("y"), CapabilityToken::STORAGE_WRITE);
        graph.add_node(a);
        graph.add_node(b);

        let guard = CapabilityGuard::new(0, CapabilityToken::empty());
        let violations = graph.lint_capabilities(&guard).unwrap_err();
        assert_eq!(violations.len(), 2);
        assert!(violations.iter().any(|v| v.node == "a"));
        assert!(violations.iter().any(|v| v.node == "b"));
    }
}
