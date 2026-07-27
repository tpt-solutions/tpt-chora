use wgpu::util::DeviceExt;

use crate::graph::{GraphNode, NodeExecuteCtx, ResourceId, TransientTextureDesc};

pub const VOLUMETRIC_OUTPUT: ResourceId = ResourceId("volumetric_output");
pub const STEREO_LEFT: ResourceId = ResourceId("stereo_left");
pub const STEREO_RIGHT: ResourceId = ResourceId("stereo_right");

pub fn create_volumetric_node(
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> GraphNode {
    GraphNode::new("volumetric_fog", move |ctx: &mut NodeExecuteCtx<'_> {
        let scene_view = &ctx.resources[&ResourceId("scene_color")].view;
        let depth_view = scene_view;
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
        VOLUMETRIC_OUTPUT,
        TransientTextureDesc {
            width,
            height,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING,
        },
    )
}

pub fn create_stereo_node(
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> GraphNode {
    GraphNode::new("stereoscopic", move |ctx: &mut NodeExecuteCtx<'_> {
        let left_view = &ctx.resources[&STEREO_LEFT].view;
        let right_view = &ctx.resources[&STEREO_RIGHT].view;

        let renderer = tpt_chora_spatial::StereoscopicRenderer::new(
            ctx.device,
            width,
            height,
            format,
        );

        use glam::{Mat4, Vec3};
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

        let vb = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("stereo-placeholder-vb"),
            contents: bytemuck::cast_slice(&[0f32; 6]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ib = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("stereo-placeholder-ib"),
            contents: bytemuck::cast_slice(&[0u32; 3]),
            usage: wgpu::BufferUsages::INDEX,
        });
        let geometry = tpt_chora_spatial::StereoGeometry {
            vertex_buffer: vb,
            index_buffer: ib,
            index_count: 0,
            vertex_stride: 24,
        };

        renderer.record_pass(ctx.device, ctx.encoder, left_view, right_view, &left_stereo, &geometry);
        renderer.record_pass(ctx.device, ctx.encoder, left_view, right_view, &right_stereo, &geometry);
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
}

pub fn create_foveation_node(
    width: f32,
    height: f32,
) -> GraphNode {
    GraphNode::new("foveated_rendering", move |ctx: &mut NodeExecuteCtx<'_> {
        let renderer = tpt_chora_spatial::FoveatedRenderer::new();
        let _level = renderer.compute_foveation_level(
            width * 0.5,
            height * 0.5,
            None,
            width,
            height,
        );
    })
}
