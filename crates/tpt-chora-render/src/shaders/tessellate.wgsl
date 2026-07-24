struct Params {
    segments_per_curve: u32,
    curve_count: u32,
};

struct Bezier {
    p0: vec2<f32>,
    p1: vec2<f32>,
    p2: vec2<f32>,
    p3: vec2<f32>,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> curves: array<Bezier>;
@group(0) @binding(2) var<storage, read_write> out_points: array<vec2<f32>>;

// Evaluates a cubic Bezier at parameter t via the Bernstein/De Casteljau
// closed form: B(t) = (1-t)^3 p0 + 3(1-t)^2 t p1 + 3(1-t) t^2 p2 + t^3 p3.
@compute @workgroup_size(64)
fn tessellate(@builtin(global_invocation_id) gid: vec3<u32>) {
    let points_per_curve = params.segments_per_curve + 1u;
    let total = points_per_curve * params.curve_count;
    let idx = gid.x;
    if (idx >= total) {
        return;
    }

    let curve_index = idx / points_per_curve;
    let point_index = idx % points_per_curve;
    let t = f32(point_index) / f32(params.segments_per_curve);

    let b = curves[curve_index];
    let one_minus_t = 1.0 - t;
    let a0 = one_minus_t * one_minus_t * one_minus_t;
    let a1 = 3.0 * one_minus_t * one_minus_t * t;
    let a2 = 3.0 * one_minus_t * t * t;
    let a3 = t * t * t;

    out_points[idx] = a0 * b.p0 + a1 * b.p1 + a2 * b.p2 + a3 * b.p3;
}
