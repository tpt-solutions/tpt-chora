struct ColorGradeParams {
    exposure: f32,
    contrast: f32,
    saturation: f32,
    gamma: f32,
};

@group(0) @binding(0) var<uniform> params: ColorGradeParams;
@group(0) @binding(1) var scene_tex: texture_2d<f32>;
@group(0) @binding(2) var scene_sampler: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VsOut;
    let p = positions[vi];
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, 1.0 - (p.y + 1.0) * 0.5);
    return out;
}

// The built-in post-processing pipeline (spec.txt §2.1): a single
// color-grading pass (exposure/contrast/saturation/gamma) driven by
// parameters that will come from tpt-eidos's visual constraints once the
// eidos integration (Phase 7) lands. Bloom, depth of field, and motion
// blur are additional passes layered the same way once camera-space
// scene data (depth, velocity buffers) exists to drive them.
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var color = textureSample(scene_tex, scene_sampler, in.uv).rgb;
    color = color * params.exposure;
    color = (color - vec3<f32>(0.5)) * params.contrast + vec3<f32>(0.5);
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = mix(vec3<f32>(luma), color, params.saturation);
    color = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / params.gamma));
    return vec4<f32>(color, 1.0);
}
