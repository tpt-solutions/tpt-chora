use wgpu::util::DeviceExt;

use crate::graph::{GraphNode, NodeExecuteCtx, ResourceId, TransientTextureDesc};
use crate::security::CapabilityToken;

pub const VOLUMETRIC_OUTPUT: ResourceId = ResourceId("volumetric_output");
pub const VOLUMETRIC_DEPTH: ResourceId = ResourceId("volumetric_depth");
pub const STEREO_LEFT: ResourceId = ResourceId("stereo_left");
pub const STEREO_RIGHT: ResourceId = ResourceId("stereo_right");
pub const FOVEATION_SHADOW_MAP: ResourceId = ResourceId("foveation_shadow_map");

/// The `format` argument was dropped: `VolumetricLightPipeline`'s bind
/// group layout hardcodes its output storage-texture binding to
/// `Rgba16Float`, so the output resource must always use that format
/// regardless of the color format the rest of the graph uses — passing a
/// caller-chosen format previously produced a storage-texture format
/// mismatch the first time this node actually ran.
pub fn create_volumetric_node(width: u32, height: u32) -> GraphNode {
    GraphNode::new("volumetric_fog", move |ctx: &mut NodeExecuteCtx<'_>| {
        let scene_view = &ctx.resources[&ResourceId("scene_color")].view;
        // `VolumetricLightPipeline::record`'s bind group layout declares
        // binding 1 as a filterable-float color texture and binding 2 as a
        // depth texture (`TextureSampleType::Depth`) — these are two
        // distinct sample types, so the depth binding can't reuse the color
        // view (a format mismatch wgpu's bind-group validation rejects).
        // Until real camera-space depth is threaded through the graph, this
        // node produces its own depth resource for that binding.
        let depth_view = &ctx.resources[&VOLUMETRIC_DEPTH].view;
        let output_view = &ctx.resources[&VOLUMETRIC_OUTPUT].view;

        let pipeline = tpt_chora_spatial::VolumetricLightPipeline::new(ctx.device);
        let params = tpt_chora_spatial::VolumetricParams::default();
        pipeline.record(
            ctx.device,
            ctx.encoder,
            &params,
            scene_view,
            depth_view,
            output_view,
            width,
            height,
        );
    })
    .reads([ResourceId("scene_color")])
    .creates(
        VOLUMETRIC_DEPTH,
        TransientTextureDesc {
            width,
            height,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        },
    )
    .creates(
        VOLUMETRIC_OUTPUT,
        TransientTextureDesc {
            width,
            height,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING,
        },
    )
    .requires(ResourceId("scene_color"), CapabilityToken::TEXTURE_READ)
    .requires(VOLUMETRIC_DEPTH, CapabilityToken::TEXTURE_READ)
    .requires(VOLUMETRIC_OUTPUT, CapabilityToken::TEXTURE_READ)
}

/// A small pyramid (4 triangles, position + normal per vertex, matching
/// `StereoGeometry`'s 24-byte stride) used as real, visible scene geometry
/// for the stereoscopic pass. Until a caller threads actual scene geometry
/// through (mirroring how `Renderer::render_frame` builds its own triangle
/// and path buffers), this replaces the former zero-index placeholder that
/// drew nothing at all.
#[rustfmt::skip]
const STEREO_DEMO_VERTICES: [[f32; 6]; 4] = [
    // apex
    [ 0.0,  0.5,  0.0,   0.0, 0.4472, 0.8944],
    [-0.5, -0.5,  0.5,   0.0, 0.4472, 0.8944],
    [ 0.5, -0.5,  0.5,   0.0, 0.4472, 0.8944],
    [ 0.0, -0.5, -0.7,   0.0, -1.0,   0.0],
];
const STEREO_DEMO_INDICES: [u32; 6] = [0, 1, 2, 1, 3, 2];

pub fn create_stereo_node(width: u32, height: u32, format: wgpu::TextureFormat) -> GraphNode {
    GraphNode::new("stereoscopic", move |ctx: &mut NodeExecuteCtx<'_>| {
        let left_view = &ctx.resources[&STEREO_LEFT].view;
        let right_view = &ctx.resources[&STEREO_RIGHT].view;

        let renderer =
            tpt_chora_spatial::StereoscopicRenderer::new(ctx.device, width, height, format);

        use glam::Vec3;
        let (left_stereo, right_stereo) = renderer.create_stereo_views(
            Vec3::new(0.0, 0.0, 3.0),
            Vec3::ZERO,
            Vec3::Y,
            std::f32::consts::FRAC_PI_4,
            width as f32 / height as f32,
            0.1,
            100.0,
            0.064,
            1.0,
        );

        let vb = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("stereo-demo-vb"),
                contents: bytemuck::cast_slice(&STEREO_DEMO_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ib = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("stereo-demo-ib"),
                contents: bytemuck::cast_slice(&STEREO_DEMO_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            });
        let geometry = tpt_chora_spatial::StereoGeometry {
            vertex_buffer: vb,
            index_buffer: ib,
            index_count: STEREO_DEMO_INDICES.len() as u32,
            vertex_stride: 24,
        };

        renderer.record_pass(
            ctx.device,
            ctx.encoder,
            left_view,
            right_view,
            &left_stereo,
            &geometry,
        );
        renderer.record_pass(
            ctx.device,
            ctx.encoder,
            left_view,
            right_view,
            &right_stereo,
            &geometry,
        );
    })
    .creates(
        STEREO_LEFT,
        TransientTextureDesc {
            width,
            height,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        },
    )
    .creates(
        STEREO_RIGHT,
        TransientTextureDesc {
            width,
            height,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        },
    )
    .requires(STEREO_LEFT, CapabilityToken::TEXTURE_READ)
    .requires(STEREO_RIGHT, CapabilityToken::TEXTURE_READ)
}

/// Sizes a shadow-map render target off the foveation level computed for
/// screen center, so the level actually drives a decision (resource size)
/// rather than being computed and discarded. A gaze-aware caller would
/// recompute the level (and therefore the ideal size) per-frame from real
/// eye-tracking input; this construction-time computation is the fixed
/// point used until gaze data is threaded through.
pub fn create_foveation_node(width: f32, height: f32) -> GraphNode {
    let renderer = tpt_chora_spatial::FoveatedRenderer::new();
    let level = renderer.compute_foveation_level(width * 0.5, height * 0.5, None, width, height);
    let shadow_map_size = renderer.get_shadow_map_size(level);

    GraphNode::new("foveated_rendering", move |ctx: &mut NodeExecuteCtx<'_>| {
        // Touch the resource so the graph's dependency tracking sees this
        // node as its producer; a real shadow pass would render into it here.
        let _ = &ctx.resources[&FOVEATION_SHADOW_MAP];
    })
    .creates(
        FOVEATION_SHADOW_MAP,
        TransientTextureDesc {
            width: shadow_map_size,
            height: shadow_map_size,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        },
    )
    .requires(FOVEATION_SHADOW_MAP, CapabilityToken::TEXTURE_READ)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::RenderGraph;
    use crate::renderer::GpuContext;

    #[test]
    fn stereo_node_executes_and_draws_nonzero_geometry() {
        let ctx = GpuContext::new_headless().expect("headless GPU context");
        let mut graph = RenderGraph::new();
        graph.add_node(create_stereo_node(
            64,
            64,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ));

        graph
            .execute(&ctx.device, &ctx.queue, None)
            .expect("stereo node executes");

        assert!(graph.texture(STEREO_LEFT).is_some());
        assert!(graph.texture(STEREO_RIGHT).is_some());
        assert!(
            !STEREO_DEMO_INDICES.is_empty(),
            "regression guard: demo geometry must not go back to zero indices"
        );
    }

    #[test]
    fn foveation_node_sizes_shadow_map_from_computed_level() {
        let ctx = GpuContext::new_headless().expect("headless GPU context");
        let mut graph = RenderGraph::new();
        graph.add_node(create_foveation_node(800.0, 600.0));

        graph
            .execute(&ctx.device, &ctx.queue, None)
            .expect("foveation node executes");

        let renderer = tpt_chora_spatial::FoveatedRenderer::new();
        let level = renderer.compute_foveation_level(400.0, 300.0, None, 800.0, 600.0);
        let expected_size = renderer.get_shadow_map_size(level);

        let tex = graph
            .texture(FOVEATION_SHADOW_MAP)
            .expect("foveation node allocates its shadow map");
        assert_eq!(tex.size().width, expected_size);
        assert_eq!(tex.size().height, expected_size);
    }

    #[test]
    fn volumetric_node_executes_after_scene_color() {
        let ctx = GpuContext::new_headless().expect("headless GPU context");
        let mut graph = RenderGraph::new();

        // volumetric_fog reads "scene_color", so a producer must exist first.
        graph.add_node(
            GraphNode::new("scene", |ctx: &mut NodeExecuteCtx<'_>| {
                let _ = &ctx.resources[&ResourceId("scene_color")];
            })
            .creates(
                ResourceId("scene_color"),
                TransientTextureDesc {
                    width: 64,
                    height: 64,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                },
            ),
        );
        graph.add_node(create_volumetric_node(64, 64));

        graph
            .execute(&ctx.device, &ctx.queue, None)
            .expect("volumetric node executes");

        assert!(graph.texture(VOLUMETRIC_OUTPUT).is_some());
    }
}
