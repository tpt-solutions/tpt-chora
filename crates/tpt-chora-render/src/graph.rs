//! The frame-scoped, dependency-tracked render graph (spec.txt §2.1).
//!
//! Nodes declare the transient resources they read, write, and create by
//! name. The graph topologically sorts nodes so producers always run before
//! their consumers, allocates each transient resource exactly once per
//! `execute`, and records every pass into a single command buffer so the
//! whole frame is submitted as one GPU submission.

use std::collections::{HashMap, HashSet};

use crate::error::RenderError;

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
    execute: ExecuteFn,
}

impl GraphNode {
    pub fn new(name: &'static str, execute: impl FnMut(&mut NodeExecuteCtx<'_>) + 'static) -> Self {
        Self {
            name,
            reads: Vec::new(),
            writes: Vec::new(),
            creates: Vec::new(),
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
}

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
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), RenderError> {
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
}

// Resources stay alive on the graph after `execute` so callers can read back
// the final output texture before the next frame's `execute` reallocates them.
impl RenderGraph {
    pub fn texture(&self, id: ResourceId) -> Option<&wgpu::Texture> {
        self.resources.get(&id).map(|t| &t.texture)
    }
}
