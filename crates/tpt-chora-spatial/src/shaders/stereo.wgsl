struct StereoParams {
    view_projection: mat4x4<f32>,
    eye_offset: vec4<f32>,
    separation: f32,
    convergence: f32,
};

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
};

@group(0) @binding(0) var<uniform> params: StereoParams;

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let eye_adjusted = vec4<f32>(in.position + params.eye_offset.xyz, 1.0);
    out.pos = params.view_projection * eye_adjusted;
    out.normal = in.normal;
    out.world_pos = in.position;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.3, 0.8, 0.5));
    let ambient = 0.2;
    let diffuse = max(dot(in.normal, light_dir), 0.0);
    let brightness = ambient + diffuse * 0.8;
    return vec4<f32>(brightness, brightness, brightness, 1.0);
}
