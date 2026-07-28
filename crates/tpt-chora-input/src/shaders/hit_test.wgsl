// GPU-accelerated 2D point hit-testing: one thread tests one BVH leaf's
// bounding box against the query point, in parallel. The CPU side then
// scans the (small) per-node results to pick the smallest-area match,
// mirroring `BoundingBoxHierarchy::query_point`'s "closest/smallest wins"
// tie-break without needing a tree traversal on the GPU.

struct BvhNode {
    idx: u32,
    id_lo: u32,
    id_hi: u32,
    _pad: u32,
    bounds: vec4<f32>,
};

struct QueryParams {
    x: f32,
    y: f32,
    node_count: u32,
    _pad: u32,
};

struct HitResult {
    hit: u32,
    id_lo: u32,
    id_hi: u32,
    area_bits: u32,
};

@group(0) @binding(0) var<uniform> query: QueryParams;
@group(0) @binding(1) var<storage, read> nodes: array<BvhNode>;
@group(0) @binding(2) var<storage, read_write> results: array<HitResult>;

@compute @workgroup_size(64)
fn hit_test(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= query.node_count) {
        return;
    }

    let b = nodes[i].bounds;
    let inside = query.x >= b.x && query.x <= b.z && query.y >= b.y && query.y <= b.w;

    var out: HitResult;
    out.id_lo = nodes[i].id_lo;
    out.id_hi = nodes[i].id_hi;
    if (inside) {
        out.hit = 1u;
        out.area_bits = bitcast<u32>((b.z - b.x) * (b.w - b.y));
    } else {
        out.hit = 0u;
        out.area_bits = bitcast<u32>(3.40282347e38);
    }
    results[i] = out;
}
