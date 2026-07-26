use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StereoEye {
    Left,
    Right,
}

pub struct StereoView {
    pub eye: StereoEye,
    pub view: Mat4,
    pub projection: Mat4,
}

pub struct StereoGeometry {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub vertex_stride: u64,
}

pub struct StereoscopicRenderer {
    left_pipeline: wgpu::RenderPipeline,
    right_pipeline: wgpu::RenderPipeline,
    left_depth: wgpu::Texture,
    right_depth: wgpu::Texture,
    stereo_bgl: wgpu::BindGroupLayout,
    _width: u32,
    _height: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct StereoParams {
    view_projection: [[f32; 4]; 4],
    eye_offset: [f32; 4],
    separation: f32,
    convergence: f32,
    _pad: [f32; 2],
}

impl StereoscopicRenderer {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chora-stereo-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/stereo.wgsl").into()),
        });

        let stereo_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chora-stereo-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chora-stereo-pl"),
            bind_group_layouts: &[&stereo_bgl],
            push_constant_ranges: &[],
        });

        let create_pipeline = |label: &'static str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 24,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 0,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 12,
                                shader_location: 1,
                            },
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };

        let left_pipeline = create_pipeline("chora-stereo-pipeline-left");
        let right_pipeline = create_pipeline("chora-stereo-pipeline-right");

        let depth_desc = |label: &'static str| wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        let left_depth = device.create_texture(&depth_desc("chora-stereo-depth-left"));
        let right_depth = device.create_texture(&depth_desc("chora-stereo-depth-right"));

        Self {
            left_pipeline,
            right_pipeline,
            left_depth,
            right_depth,
            stereo_bgl,
            _width: width,
            _height: height,
        }
    }

    pub fn create_stereo_views(
        &self,
        camera_pos: Vec3,
        look_at: Vec3,
        up: Vec3,
        fov_y: f32,
        aspect: f32,
        near: f32,
        far: f32,
        eye_separation: f32,
        convergence_distance: f32,
    ) -> (StereoView, StereoView) {
        let forward = (look_at - camera_pos).normalize();
        let right = forward.cross(up).normalize();

        let left_eye = camera_pos - right * eye_separation * 0.5;
        let right_eye = camera_pos + right * eye_separation * 0.5;

        let left_view = Mat4::look_at_rh(left_eye, look_at, up);
        let right_view = Mat4::look_at_rh(right_eye, look_at, up);

        let left_offset = eye_separation * 0.5 * near / convergence_distance;
        let right_offset = -eye_separation * 0.5 * near / convergence_distance;

        let tan_half_fov = (fov_y * 0.5).tan();

        let left_proj = asymmetric_frustum(
            -tan_half_fov * aspect + left_offset,
            tan_half_fov * aspect + left_offset,
            -tan_half_fov,
            tan_half_fov,
            near,
            far,
        );
        let right_proj = asymmetric_frustum(
            -tan_half_fov * aspect + right_offset,
            tan_half_fov * aspect + right_offset,
            -tan_half_fov,
            tan_half_fov,
            near,
            far,
        );

        (
            StereoView {
                eye: StereoEye::Left,
                view: left_view,
                projection: left_proj,
            },
            StereoView {
                eye: StereoEye::Right,
                view: right_view,
                projection: right_proj,
            },
        )
    }

    pub fn record_pass(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target_left: &wgpu::TextureView,
        target_right: &wgpu::TextureView,
        stereo_view: &StereoView,
        geometry: &StereoGeometry,
    ) {
        let eye_offset = match stereo_view.eye {
            StereoEye::Left => [-1.0, 0.0, 0.0, 0.0],
            StereoEye::Right => [1.0, 0.0, 0.0, 0.0],
        };

        let params = StereoParams {
            view_projection: {
                let mat = stereo_view.projection * stereo_view.view;
                let cols = mat.to_cols_array();
                [
                    [cols[0], cols[1], cols[2], cols[3]],
                    [cols[4], cols[5], cols[6], cols[7]],
                    [cols[8], cols[9], cols[10], cols[11]],
                    [cols[12], cols[13], cols[14], cols[15]],
                ]
            },
            eye_offset,
            separation: 0.064,
            convergence: 1.0,
            _pad: [0.0; 2],
        };

        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chora-stereo-params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("chora-stereo-bg"),
            layout: &self.stereo_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buf.as_entire_binding(),
            }],
        });

        let (target, depth, pipeline) = match stereo_view.eye {
            StereoEye::Left => (
                target_left,
                self.left_depth
                    .create_view(&wgpu::TextureViewDescriptor::default()),
                &self.left_pipeline,
            ),
            StereoEye::Right => (
                target_right,
                self.right_depth
                    .create_view(&wgpu::TextureViewDescriptor::default()),
                &self.right_pipeline,
            ),
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("chora-stereo-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.set_vertex_buffer(0, geometry.vertex_buffer.slice(..));
        pass.set_index_buffer(geometry.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..geometry.index_count, 0, 0..1);
    }
}

fn asymmetric_frustum(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
    let sx = 2.0 * near / (right - left);
    let sy = 2.0 * near / (top - bottom);
    let sz = far / (near - far);
    let tx = near * (left + right) / (left - right);
    let ty = near * (bottom + top) / (bottom - top);

    Mat4::from_cols_array(&[
        sx,
        0.0,
        0.0,
        0.0,
        0.0,
        sy,
        0.0,
        0.0,
        tx,
        ty,
        sz,
        -1.0,
        0.0,
        0.0,
        near * sz,
        0.0,
    ])
}
