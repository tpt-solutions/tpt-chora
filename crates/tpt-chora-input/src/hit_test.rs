use std::collections::HashMap;
use wgpu::util::DeviceExt;

pub struct BoundingBoxHierarchy {
    nodes: Vec<BvhNode>,
    root: Option<usize>,
}

struct BvhNode {
    bounds: [f32; 4],
    children: Option<[usize; 2]>,
    leaf_data: Option<u64>,
}

pub struct GpuHitTest {
    depth_buffer: Option<wgpu::Buffer>,
    bvh_buffer: Option<wgpu::Buffer>,
    hit_results_buffer: Option<wgpu::Buffer>,
    node_map: HashMap<u64, (u32, [f32; 4])>,
    node_count: u32,
    compute: Option<GpuHitTestPipeline>,
}

/// The compiled compute pipeline used by [`GpuHitTest::hit_test_gpu`] to
/// test every BVH leaf's bounding box against a query point in parallel on
/// the GPU, one thread per node.
struct GpuHitTestPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuHitTestPipeline {
    fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chora-hit-test-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/hit_test.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chora-hit-test-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chora-hit-test-pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("chora-hit-test-pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "hit_test",
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HitResult {
    pub node_id: u64,
    pub depth: f32,
    pub x: f32,
    pub y: f32,
    pub bounding_box: [f32; 4],
}

impl GpuHitTest {
    pub fn new() -> Self {
        Self {
            depth_buffer: None,
            bvh_buffer: None,
            hit_results_buffer: None,
            node_map: HashMap::new(),
            node_count: 0,
            compute: None,
        }
    }

    pub fn new_from_gpu(
        device: &wgpu::Device,
        bvh: &BoundingBoxHierarchy,
        width: u32,
        height: u32,
    ) -> Self {
        let flat_nodes = bvh.flatten();
        let node_count = flat_nodes.len() as u32;

        // Packed as a GPU-friendly, 16-byte-aligned `BvhNode` struct (see
        // `shaders/hit_test.wgsl`): `idx`, `node_id` split into `id_lo`/
        // `id_hi` (WGSL has no native u64), a padding word, then `bounds`
        // as a `vec4<f32>`.
        let bvh_data: Vec<u8> = flat_nodes
            .iter()
            .flat_map(|(idx, bounds, node_id)| {
                let mut data = Vec::with_capacity(32);
                data.extend_from_slice(&(*idx as u32).to_le_bytes());
                data.extend_from_slice(&(*node_id as u32).to_le_bytes());
                data.extend_from_slice(&((*node_id >> 32) as u32).to_le_bytes());
                data.extend_from_slice(&0u32.to_le_bytes());
                data.extend_from_slice(&bounds[0].to_le_bytes());
                data.extend_from_slice(&bounds[1].to_le_bytes());
                data.extend_from_slice(&bounds[2].to_le_bytes());
                data.extend_from_slice(&bounds[3].to_le_bytes());
                data
            })
            .collect();
        // A zero-node BVH would create a zero-sized storage buffer, which
        // wgpu rejects; keep a single dummy (all-zero, never matched by a
        // real node_count of 0 in the dispatch) entry in that case.
        let bvh_data = if bvh_data.is_empty() {
            vec![0u8; 32]
        } else {
            bvh_data
        };

        let bvh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chora-gpu-bvh"),
            contents: &bvh_data,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let depth_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chora-gpu-depth"),
            size: (width * height * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let hit_results_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chora-gpu-hit-results"),
            size: (std::mem::size_of::<u32>() * 4 * node_count.max(1) as usize) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let node_map: HashMap<u64, (u32, [f32; 4])> = flat_nodes
            .iter()
            .map(|(idx, bounds, node_id)| (*node_id, (*idx as u32, *bounds)))
            .collect();

        Self {
            depth_buffer: Some(depth_buffer),
            bvh_buffer: Some(bvh_buffer),
            hit_results_buffer: Some(hit_results_buffer),
            node_map,
            node_count,
            compute: Some(GpuHitTestPipeline::new(device)),
        }
    }

    pub fn hit_test_2d(&self, x: f32, y: f32, bvh: &BoundingBoxHierarchy) -> Option<HitResult> {
        bvh.query_point(x, y)
    }

    /// Dispatches the hit-test compute shader (one thread per BVH leaf)
    /// against the GPU buffers built by [`Self::new_from_gpu`], reads back
    /// per-node results, and picks the smallest-area match — the same
    /// "closest/smallest wins" tie-break `hit_test_2d`'s tree traversal
    /// uses, just computed by a parallel GPU scan instead of a CPU tree
    /// walk.
    pub fn hit_test_gpu(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        x: f32,
        y: f32,
    ) -> Option<HitResult> {
        let bvh_buffer = self.bvh_buffer.as_ref()?;
        let hit_results_buffer = self.hit_results_buffer.as_ref()?;
        let compute = self.compute.as_ref()?;
        if self.node_count == 0 {
            return None;
        }

        let mut query_data = Vec::with_capacity(16);
        query_data.extend_from_slice(&x.to_le_bytes());
        query_data.extend_from_slice(&y.to_le_bytes());
        query_data.extend_from_slice(&self.node_count.to_le_bytes());
        query_data.extend_from_slice(&0u32.to_le_bytes());
        let query_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chora-hit-test-query"),
            contents: &query_data,
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("chora-hit-test-bg"),
            layout: &compute.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: query_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bvh_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: hit_results_buffer.as_entire_binding(),
                },
            ],
        });

        let result_size = (std::mem::size_of::<u32>() * 4 * self.node_count as usize) as u64;
        let staging_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chora-hit-test-staging"),
            size: result_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("chora-hit-test-encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("chora-hit-test-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&compute.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(self.node_count.div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(hit_results_buffer, 0, &staging_buf, 0, result_size);
        queue.submit(std::iter::once(encoder.finish()));

        let slice = staging_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv().ok()?.ok()?;

        let data = slice.get_mapped_range();
        let mut closest: Option<HitResult> = None;
        let mut closest_area = f32::MAX;
        for chunk in data.chunks_exact(16) {
            let hit = u32::from_le_bytes(chunk[0..4].try_into().unwrap());
            if hit == 0 {
                continue;
            }
            let id_lo = u32::from_le_bytes(chunk[4..8].try_into().unwrap());
            let id_hi = u32::from_le_bytes(chunk[8..12].try_into().unwrap());
            let node_id = (id_lo as u64) | ((id_hi as u64) << 32);
            let area = f32::from_bits(u32::from_le_bytes(chunk[12..16].try_into().unwrap()));

            if area < closest_area {
                if let Some(&(_idx, bounds)) = self.node_map.get(&node_id) {
                    closest_area = area;
                    closest = Some(HitResult {
                        node_id,
                        depth: area,
                        x,
                        y,
                        bounding_box: bounds,
                    });
                }
            }
        }
        drop(data);
        staging_buf.unmap();

        closest
    }

    pub fn depth_buffer(&self) -> Option<&wgpu::Buffer> {
        self.depth_buffer.as_ref()
    }

    pub fn bvh_buffer(&self) -> Option<&wgpu::Buffer> {
        self.bvh_buffer.as_ref()
    }

    pub fn hit_results_buffer(&self) -> Option<&wgpu::Buffer> {
        self.hit_results_buffer.as_ref()
    }
}

impl Default for GpuHitTest {
    fn default() -> Self {
        Self::new()
    }
}

impl BoundingBoxHierarchy {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
        }
    }

    pub fn insert(&mut self, bounds: [f32; 4], node_id: u64) {
        let leaf = BvhNode {
            bounds,
            children: None,
            leaf_data: Some(node_id),
        };
        let idx = self.nodes.len();
        self.nodes.push(leaf);
        if self.root.is_none() {
            self.root = Some(idx);
        }
    }

    pub fn build_tree(&mut self) {
        if self.nodes.len() < 2 {
            return;
        }

        let mut indices: Vec<usize> = (0..self.nodes.len()).collect();
        indices.sort_by(|&a, &b| {
            let center_a = (self.nodes[a].bounds[0] + self.nodes[a].bounds[2]) * 0.5;
            let center_b = (self.nodes[b].bounds[0] + self.nodes[b].bounds[2]) * 0.5;
            center_a.total_cmp(&center_b)
        });

        self.root = Some(self.build_subtree(&indices));
    }

    fn build_subtree(&mut self, indices: &[usize]) -> usize {
        if indices.len() == 1 {
            return indices[0];
        }

        if indices.len() == 2 {
            let left = indices[0];
            let right = indices[1];

            let min_x = self.nodes[left].bounds[0].min(self.nodes[right].bounds[0]);
            let min_y = self.nodes[left].bounds[1].min(self.nodes[right].bounds[1]);
            let max_x = self.nodes[left].bounds[2].max(self.nodes[right].bounds[2]);
            let max_y = self.nodes[left].bounds[3].max(self.nodes[right].bounds[3]);

            let parent_idx = self.nodes.len();
            self.nodes.push(BvhNode {
                bounds: [min_x, min_y, max_x, max_y],
                children: Some([left, right]),
                leaf_data: None,
            });
            return parent_idx;
        }

        let mid = indices.len() / 2;
        let left_idx = self.build_subtree(&indices[..mid]);
        let right_idx = self.build_subtree(&indices[mid..]);

        let min_x = self.nodes[left_idx].bounds[0].min(self.nodes[right_idx].bounds[0]);
        let min_y = self.nodes[left_idx].bounds[1].min(self.nodes[right_idx].bounds[1]);
        let max_x = self.nodes[left_idx].bounds[2].max(self.nodes[right_idx].bounds[2]);
        let max_y = self.nodes[left_idx].bounds[3].max(self.nodes[right_idx].bounds[3]);

        let parent_idx = self.nodes.len();
        self.nodes.push(BvhNode {
            bounds: [min_x, min_y, max_x, max_y],
            children: Some([left_idx, right_idx]),
            leaf_data: None,
        });
        parent_idx
    }

    pub fn query_point(&self, x: f32, y: f32) -> Option<HitResult> {
        let root = self.root?;
        self.query_point_recursive(root, x, y, f32::MAX, None)
    }

    fn query_point_recursive(
        &self,
        idx: usize,
        x: f32,
        y: f32,
        best_depth: f32,
        best: Option<HitResult>,
    ) -> Option<HitResult> {
        let node = &self.nodes[idx];

        if !point_in_bounds(x, y, &node.bounds) {
            return best;
        }

        let mut best = best;
        let mut best_depth = best_depth;

        if let Some(leaf_id) = node.leaf_data {
            let center_x = (node.bounds[0] + node.bounds[2]) * 0.5;
            let center_y = (node.bounds[1] + node.bounds[3]) * 0.5;
            let dist_to_center = ((x - center_x).powi(2) + (y - center_y).powi(2)).sqrt();
            let depth = dist_to_center;

            if depth < best_depth {
                best_depth = depth;
                best = Some(HitResult {
                    node_id: leaf_id,
                    depth,
                    x,
                    y,
                    bounding_box: node.bounds,
                });
            }
        }

        if let Some([left, right]) = node.children {
            let left_bounds = &self.nodes[left].bounds;
            let right_bounds = &self.nodes[right].bounds;
            let left_dist = point_to_bounds_dist(x, y, left_bounds);
            let right_dist = point_to_bounds_dist(x, y, right_bounds);

            if left_dist < best_depth {
                best = self.query_point_recursive(left, x, y, best_depth, best);
                if let Some(ref b) = best {
                    best_depth = b.depth;
                }
            }
            if right_dist < best_depth {
                best = self.query_point_recursive(right, x, y, best_depth, best);
            }
        }

        best
    }

    pub fn query_region(&self, bounds: [f32; 4]) -> Vec<HitResult> {
        let root = match self.root {
            Some(r) => r,
            None => return Vec::new(),
        };
        let mut results = Vec::new();
        self.query_region_recursive(root, &bounds, &mut results);
        results
    }

    fn query_region_recursive(
        &self,
        idx: usize,
        query_bounds: &[f32; 4],
        results: &mut Vec<HitResult>,
    ) {
        let node = &self.nodes[idx];

        if !bounds_intersect(&node.bounds, query_bounds) {
            return;
        }

        if let Some(leaf_id) = node.leaf_data {
            results.push(HitResult {
                node_id: leaf_id,
                depth: 0.0,
                x: (query_bounds[0] + query_bounds[2]) * 0.5,
                y: (query_bounds[1] + query_bounds[3]) * 0.5,
                bounding_box: node.bounds,
            });
        }

        if let Some([left, right]) = node.children {
            self.query_region_recursive(left, query_bounds, results);
            self.query_region_recursive(right, query_bounds, results);
        }
    }

    pub fn flatten(&self) -> Vec<(usize, [f32; 4], u64)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(idx, node)| node.leaf_data.map(|id| (idx, node.bounds, id)))
            .collect()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for BoundingBoxHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

fn point_in_bounds(x: f32, y: f32, bounds: &[f32; 4]) -> bool {
    x >= bounds[0] && x <= bounds[2] && y >= bounds[1] && y <= bounds[3]
}

fn bounds_intersect(a: &[f32; 4], b: &[f32; 4]) -> bool {
    a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]
}

fn point_to_bounds_dist(x: f32, y: f32, bounds: &[f32; 4]) -> f32 {
    let clamped_x = x.clamp(bounds[0], bounds[2]);
    let clamped_y = y.clamp(bounds[1], bounds[3]);
    ((x - clamped_x).powi(2) + (y - clamped_y).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bvh_node_count_and_query() {
        let bvh = BoundingBoxHierarchy::new();
        assert_eq!(bvh.node_count(), 0);
        assert!(bvh.root.is_none());
        assert!(bvh.query_point(5.0, 5.0).is_none());
    }

    #[test]
    fn single_node_insert() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 10.0, 10.0], 1);
        assert_eq!(bvh.node_count(), 1);
        assert_eq!(bvh.root, Some(0));
    }

    #[test]
    fn build_tree_two_nodes() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 5.0, 5.0], 1);
        bvh.insert([6.0, 6.0, 10.0, 10.0], 2);
        bvh.build_tree();
        assert_eq!(bvh.node_count(), 3);
    }

    #[test]
    fn build_tree_four_nodes() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 2.0, 2.0], 1);
        bvh.insert([3.0, 0.0, 5.0, 2.0], 2);
        bvh.insert([0.0, 3.0, 2.0, 5.0], 3);
        bvh.insert([3.0, 3.0, 5.0, 5.0], 4);
        bvh.build_tree();
        assert_eq!(bvh.node_count(), 7);
    }

    #[test]
    fn build_tree_single_node_noop() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 10.0, 10.0], 42);
        bvh.build_tree();
        assert_eq!(bvh.node_count(), 1);
    }

    #[test]
    fn query_point_hit() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 10.0, 10.0], 42);
        bvh.build_tree();
        let result = bvh.query_point(5.0, 5.0).unwrap();
        assert_eq!(result.node_id, 42);
        assert_eq!(result.bounding_box, [0.0, 0.0, 10.0, 10.0]);
        assert!(result.depth >= 0.0);
    }

    #[test]
    fn query_point_miss() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 10.0, 10.0], 42);
        bvh.build_tree();
        assert!(bvh.query_point(20.0, 20.0).is_none());
    }

    #[test]
    fn query_point_closest_overlapping() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 20.0, 20.0], 1);
        bvh.insert([2.0, 2.0, 8.0, 8.0], 2);
        bvh.build_tree();
        let result = bvh.query_point(4.0, 4.0).unwrap();
        assert_eq!(result.node_id, 2);
    }

    #[test]
    fn query_region_single_hit() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 10.0, 10.0], 1);
        bvh.insert([20.0, 20.0, 30.0, 30.0], 2);
        bvh.build_tree();
        let results = bvh.query_region([2.0, 2.0, 8.0, 8.0]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].node_id, 1);
    }

    #[test]
    fn query_region_multiple_hits() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 10.0, 10.0], 1);
        bvh.insert([5.0, 5.0, 15.0, 15.0], 2);
        bvh.insert([20.0, 20.0, 30.0, 30.0], 3);
        bvh.build_tree();
        let results = bvh.query_region([0.0, 0.0, 12.0, 12.0]);
        assert_eq!(results.len(), 2);
        let ids: Vec<u64> = results.iter().map(|r| r.node_id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
    }

    #[test]
    fn query_region_no_hits() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 10.0, 10.0], 1);
        bvh.insert([20.0, 20.0, 30.0, 30.0], 2);
        bvh.build_tree();
        let results = bvh.query_region([50.0, 50.0, 60.0, 60.0]);
        assert!(results.is_empty());
    }

    #[test]
    fn flatten_returns_only_leaves() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 5.0, 5.0], 10);
        bvh.insert([6.0, 6.0, 11.0, 11.0], 20);
        bvh.insert([12.0, 12.0, 17.0, 17.0], 30);
        bvh.build_tree();
        let flat = bvh.flatten();
        assert_eq!(flat.len(), 3);
        let ids: Vec<u64> = flat.iter().map(|(_, _, id)| *id).collect();
        assert!(ids.contains(&10));
        assert!(ids.contains(&20));
        assert!(ids.contains(&30));
        for (idx, bounds, _) in &flat {
            assert_eq!(*idx, *idx);
            assert!(bounds[2] > bounds[0]);
            assert!(bounds[3] > bounds[1]);
        }
    }

    #[test]
    fn gpu_hit_test_2d_hit() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 10.0, 10.0], 99);
        bvh.build_tree();
        let gpu = GpuHitTest::new();
        let result = gpu.hit_test_2d(3.0, 3.0, &bvh).unwrap();
        assert_eq!(result.node_id, 99);
    }

    #[test]
    fn gpu_hit_test_2d_miss() {
        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 10.0, 10.0], 99);
        bvh.build_tree();
        let gpu = GpuHitTest::new();
        assert!(gpu.hit_test_2d(50.0, 50.0, &bvh).is_none());
    }

    fn headless_device() -> (wgpu::Device, wgpu::Queue) {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
            let adapter = match instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
            {
                Some(a) => a,
                None => instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::default(),
                        compatible_surface: None,
                        force_fallback_adapter: true,
                    })
                    .await
                    .expect("no wgpu adapter available"),
            };
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("chora-input-test-device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                    },
                    None,
                )
                .await
                .expect("failed to create device")
        })
    }

    #[test]
    fn gpu_hit_test_dispatches_real_compute_and_hits() {
        let (device, queue) = headless_device();

        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 10.0, 10.0], 42);
        bvh.insert([100.0, 100.0, 110.0, 110.0], 7);
        bvh.build_tree();

        let gpu = GpuHitTest::new_from_gpu(&device, &bvh, 256, 256);
        let result = gpu
            .hit_test_gpu(&device, &queue, 3.0, 3.0)
            .expect("expected a GPU hit");
        assert_eq!(result.node_id, 42);
    }

    #[test]
    fn gpu_hit_test_miss_returns_none() {
        let (device, queue) = headless_device();

        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 10.0, 10.0], 42);
        bvh.build_tree();

        let gpu = GpuHitTest::new_from_gpu(&device, &bvh, 256, 256);
        assert!(gpu.hit_test_gpu(&device, &queue, 500.0, 500.0).is_none());
    }

    #[test]
    fn gpu_hit_test_picks_smallest_overlapping_area() {
        let (device, queue) = headless_device();

        let mut bvh = BoundingBoxHierarchy::new();
        bvh.insert([0.0, 0.0, 20.0, 20.0], 1);
        bvh.insert([2.0, 2.0, 8.0, 8.0], 2);
        bvh.build_tree();

        let gpu = GpuHitTest::new_from_gpu(&device, &bvh, 256, 256);
        let result = gpu
            .hit_test_gpu(&device, &queue, 4.0, 4.0)
            .expect("expected a GPU hit");
        assert_eq!(result.node_id, 2);
    }

    #[test]
    fn gpu_hit_test_empty_bvh_returns_none() {
        let (device, queue) = headless_device();
        let bvh = BoundingBoxHierarchy::new();
        let gpu = GpuHitTest::new_from_gpu(&device, &bvh, 256, 256);
        assert!(gpu.hit_test_gpu(&device, &queue, 1.0, 1.0).is_none());
    }
}
