#[derive(Debug, Clone)]
pub struct HitResult {
    pub node_id: u64,
    pub depth: f32,
    pub x: f32,
    pub y: f32,
    pub bounding_box: [f32; 4],
}

pub struct BoundingBoxHierarchy {
    nodes: Vec<BvhNode>,
}

struct BvhNode {
    bounds: [f32; 4],
    children: Option<[usize; 2]>,
    leaf_data: Option<u64>,
}

pub struct GpuHitTest {
    depth_buffer: Option<u32>,
    bvh_buffer: Option<u32>,
    hit_results_buffer: Option<u32>,
}

impl GpuHitTest {
    pub fn new() -> Self {
        Self {
            depth_buffer: None,
            bvh_buffer: None,
            hit_results_buffer: None,
        }
    }

    pub fn hit_test_2d(&self, x: f32, y: f32, bvh: &BoundingBoxHierarchy) -> Option<HitResult> {
        bvh.query_point(x, y)
    }

    pub fn hit_test_gpu(&self, _x: f32, _y: f32, _width: u32, _height: u32) -> Option<HitResult> {
        None
    }
}

impl BoundingBoxHierarchy {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn insert(&mut self, bounds: [f32; 4], node_id: u64) {
        let leaf = BvhNode {
            bounds,
            children: None,
            leaf_data: Some(node_id),
        };
        self.nodes.push(leaf);
    }

    pub fn query_point(&self, x: f32, y: f32) -> Option<HitResult> {
        let mut closest: Option<HitResult> = None;
        let mut closest_depth = f32::MAX;

        for node in &self.nodes {
            if let Some(leaf_id) = node.leaf_data {
                if x >= node.bounds[0]
                    && x <= node.bounds[2]
                    && y >= node.bounds[1]
                    && y <= node.bounds[3]
                {
                    let depth = 0.0;
                    if depth < closest_depth {
                        closest_depth = depth;
                        closest = Some(HitResult {
                            node_id: leaf_id,
                            depth,
                            x,
                            y,
                            bounding_box: node.bounds,
                        });
                    }
                }
            }
        }

        closest
    }

    pub fn query_region(&self, bounds: [f32; 4]) -> Vec<HitResult> {
        self.nodes
            .iter()
            .filter_map(|node| {
                node.leaf_data.and_then(|leaf_id| {
                    if bounds[0] <= node.bounds[2]
                        && bounds[2] >= node.bounds[0]
                        && bounds[1] <= node.bounds[3]
                        && bounds[3] >= node.bounds[1]
                    {
                        Some(HitResult {
                            node_id: leaf_id,
                            depth: 0.0,
                            x: (bounds[0] + bounds[2]) * 0.5,
                            y: (bounds[1] + bounds[3]) * 0.5,
                            bounding_box: node.bounds,
                        })
                    } else {
                        None
                    }
                })
            })
            .collect()
    }
}
