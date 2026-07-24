struct SdfTextParams {
    screen_size: vec2<f32>,
    atlas_size: vec2<f32>,
    spread: f32,
}

struct VsIn {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> params: SdfTextParams;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let clip_x = (in.position.x / params.screen_size.x) * 2.0 - 1.0;
    let clip_y = 1.0 - (in.position.y / params.screen_size.y) * 2.0;
    out.pos = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    out.tex_coord = in.tex_coord;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let sdf = textureSample(atlas_tex, atlas_sampler, in.tex_coord).r;
    let smoothing = 1.0 / params.spread;
    let alpha = smoothstep(0.5 - smoothing, 0.5 + smoothing, sdf);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
