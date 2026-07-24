struct VolumetricParams {
    light_dir: vec3<f32>,
    light_intensity: f32,
    scatter_coefficient: f32,
    absorption_coefficient: f32,
    density: f32,
    step_size: f32,
    num_steps: u32,
    shadow_map_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> params: VolumetricParams;
@group(0) @binding(1) var scene_tex: texture_2d<f32>;
@group(0) @binding(2) var depth_tex: texture_depth_2d;
@group(0) @binding(3) var output_tex: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8)
fn volumetric_scatter(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(output_tex);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }

    let uv = vec2<f32>(f32(gid.x) / f32(dims.x), f32(gid.y) / f32(dims.y));
    let scene_color = textureLoad(scene_tex, gid.xy, 0).rgb;
    let depth = textureLoad(depth_tex, gid.xy, 0);

    let ray_origin = vec3<f32>(uv.x * 2.0 - 1.0, uv.y * 2.0 - 1.0, 0.0);
    let ray_dir = normalize(vec3<f32>(0.0, 0.0, 1.0));

    var scatter = vec3<f32>(0.0, 0.0, 0.0);
    var transmittance = 1.0;

    let step_size = params.step_size;
    let steps = params.num_steps;

    for (var i = 0u; i < steps; i++) {
        let t = f32(i) / f32(steps);
        let sample_pos = ray_origin + ray_dir * t;

        let n_dot_l = max(dot(normalize(sample_pos), params.light_dir), 0.0);
        let phase = 0.25 * (1.0 + n_dot_l);

        let sample_density = params.density * (1.0 - depth);
        let sample_scatter = params.scatter_coefficient * sample_density * phase;
        let sample_absorb = params.absorption_coefficient * sample_density;

        scatter += transmittance * sample_scatter * params.light_intensity * step_size;
        transmittance *= exp(-sample_absorb * step_size);
    }

    let result = scene_color + scatter;
    textureStore(output_tex, gid.xy, vec4<f32>(result, 1.0));
}
