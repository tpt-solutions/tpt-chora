//! Ties the render graph, vector tessellation, and post-processing
//! pipeline together, and provides the headless off-screen entry point
//! used by the Phase 1 milestone (see `examples/triangle_and_path.rs`).
//! Headless/off-screen operation here doubles as the foundation for the
//! Tier 2 headless fallback (spec.txt, fallback strategy §Tier 2).

use std::rc::Rc;

use wgpu::util::DeviceExt;

use crate::error::RenderError;
use crate::graph::{GraphNode, NodeExecuteCtx, RenderGraph, ResourceId, TransientTextureDesc};
use crate::postprocess::{ColorGradeParams, PostProcessPipeline};
use crate::security::{SecurityContext, ViewportGuard};
use crate::vector::{circle_path, tessellate_cubics_gpu};

const SCENE_COLOR: ResourceId = ResourceId("scene_color");
const FINAL_COLOR: ResourceId = ResourceId("final_color");

/// Owns the wgpu instance/adapter/device/queue. Falls back through wgpu's
/// own backend list (Vulkan/Metal/DX12/GL) and, when no hardware adapter
/// is present, wgpu's software rasterizer (Tier 1 of the fallback
/// strategy) automatically.
#[derive(Debug)]
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuContext {
    pub fn new_headless() -> Result<Self, RenderError> {
        pollster::block_on(Self::new_headless_async())
    }

    async fn new_headless_async() -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
        {
            Some(a) => a,
            None => instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: None,
                    force_fallback_adapter: true,
                })
                .await
                .ok_or(RenderError::NoAdapter)?,
        };
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("chora-headless-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;
        Ok(Self { device, queue })
    }
}

/// The Core Rendering Engine's headless entry point: builds a render
/// graph each frame (scene pass -> post-process pass) and reads the
/// result back to CPU memory as RGBA8 pixels.
pub struct Renderer {
    ctx: GpuContext,
    width: u32,
    height: u32,
    scene_pipeline: Rc<wgpu::RenderPipeline>,
    scene_bgl: Rc<wgpu::BindGroupLayout>,
    postprocess: Rc<PostProcessPipeline>,
    color_format: wgpu::TextureFormat,
    security: SecurityContext,
}

impl Renderer {
    pub fn new_headless(width: u32, height: u32) -> Result<Self, RenderError> {
        let ctx = GpuContext::new_headless()?;
        let color_format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let shader = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("chora-scene-shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/scene.wgsl").into()),
            });

        let scene_bgl = ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("chora-scene-bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("chora-scene-pl"),
                bind_group_layouts: &[&scene_bgl],
                push_constant_ranges: &[],
            });

        let scene_pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("chora-scene-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        }],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

        let postprocess =
            PostProcessPipeline::new(&ctx.device, color_format, ColorGradeParams::default());

        use crate::security::CapabilityToken;
        use crate::security::HierarchicalZDepth;
        let security = SecurityContext::new(
            0,
            CapabilityToken::TEXTURE_READ
                | CapabilityToken::TEXTURE_WRITE
                | CapabilityToken::UNIFORM_READ
                | CapabilityToken::RENDER_TARGET,
            [0.0, 0.0, width as f32, height as f32],
            HierarchicalZDepth::new(0.0, 1.0, 1000.0),
        );

        Ok(Self {
            ctx,
            width,
            height,
            scene_pipeline: Rc::new(scene_pipeline),
            scene_bgl: Rc::new(scene_bgl),
            postprocess: Rc::new(postprocess),
            color_format,
            security,
        })
    }

    pub fn set_color_grade(&self, params: ColorGradeParams) {
        self.postprocess.set_params(&self.ctx.queue, params);
    }

    /// Renders one frame (a triangle plus a GPU-tessellated vector path,
    /// through the render graph and the post-process pass) and returns
    /// tightly packed RGBA8 pixels, `width * height * 4` bytes.
    pub fn render_frame(&self) -> Result<Vec<u8>, RenderError> {
        let device = &self.ctx.device;
        let queue = &self.ctx.queue;

        let triangle_vertices: [[f32; 2]; 3] = [[-0.9, -0.7], [-0.5, 0.3], [-0.1, -0.7]];
        let triangle_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chora-triangle-vbuf"),
            contents: bytemuck::cast_slice(&triangle_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let triangle_color: [f32; 4] = [0.85, 0.25, 0.2, 1.0];
        let triangle_color_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chora-triangle-color"),
            contents: bytemuck::cast_slice(&triangle_color),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // The vector path: a circle built from 4 cubic Beziers, tessellated
        // on the GPU (spec.txt §2.1 "Vector Graphics"), then fan-triangulated
        // around its center for filling.
        let curves = circle_path([0.4, 0.0], 0.35);
        let path_points = tessellate_cubics_gpu(device, queue, &curves, 32)?;
        let mut path_vertices = Vec::with_capacity(path_points.len() + 1);
        path_vertices.push([0.4f32, 0.0]);
        path_vertices.extend(path_points.iter().copied());
        let path_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chora-path-vbuf"),
            contents: bytemuck::cast_slice(&path_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let mut path_indices: Vec<u16> = Vec::new();
        for i in 0..(path_points.len() as u16 - 1) {
            path_indices.push(0);
            path_indices.push(i + 1);
            path_indices.push(i + 2);
        }
        let path_index_count = path_indices.len() as u32;
        let path_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chora-path-ibuf"),
            contents: bytemuck::cast_slice(&path_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let path_color: [f32; 4] = [0.25, 0.45, 0.9, 1.0];
        let path_color_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chora-path-color"),
            contents: bytemuck::cast_slice(&path_color),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let scene_pipeline = self.scene_pipeline.clone();
        let scene_bgl = self.scene_bgl.clone();
        let postprocess = self.postprocess.clone();
        let width = self.width;
        let height = self.height;
        let color_format = self.color_format;
        let viewport = ViewportGuard::new(0.0, 0.0, width as f32, height as f32);

        let mut graph = RenderGraph::new();

        graph.add_node(
            GraphNode::new("scene", move |ctx: &mut NodeExecuteCtx<'_>| {
                let scene_view = &ctx.resources[&SCENE_COLOR].view;

                let tri_bg = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("chora-triangle-bg"),
                    layout: &scene_bgl,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: triangle_color_buf.as_entire_binding(),
                    }],
                });
                let path_bg = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("chora-path-bg"),
                    layout: &scene_bgl,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: path_color_buf.as_entire_binding(),
                    }],
                });

                let mut pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("chora-scene-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: scene_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.05,
                                g: 0.05,
                                b: 0.08,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                viewport.apply_scissor(&mut pass, height as f32);

                pass.set_pipeline(&scene_pipeline);

                pass.set_bind_group(0, &tri_bg, &[]);
                pass.set_vertex_buffer(0, triangle_vbuf.slice(..));
                pass.draw(0..3, 0..1);

                pass.set_bind_group(0, &path_bg, &[]);
                pass.set_vertex_buffer(0, path_vbuf.slice(..));
                pass.set_index_buffer(path_ibuf.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..path_index_count, 0, 0..1);
            })
            .creates(
                SCENE_COLOR,
                TransientTextureDesc {
                    width,
                    height,
                    format: color_format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                },
            )
            .requires(SCENE_COLOR, crate::security::CapabilityToken::TEXTURE_READ),
        );

        graph.add_node(
            GraphNode::new("postprocess", move |ctx: &mut NodeExecuteCtx<'_>| {
                let scene_view = &ctx.resources[&SCENE_COLOR].view;
                let final_view = &ctx.resources[&FINAL_COLOR].view;
                postprocess.record(ctx.device, ctx.encoder, scene_view, final_view);
            })
            .reads([SCENE_COLOR])
            .creates(
                FINAL_COLOR,
                TransientTextureDesc {
                    width,
                    height,
                    format: color_format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                },
            )
            .requires(SCENE_COLOR, crate::security::CapabilityToken::TEXTURE_READ)
            .requires(FINAL_COLOR, crate::security::CapabilityToken::TEXTURE_READ),
        );

        graph.execute(device, queue, Some(&self.security))?;

        let final_texture = graph
            .texture(FINAL_COLOR)
            .expect("postprocess node always allocates final_color");
        self.read_back_rgba(final_texture)
    }

    fn read_back_rgba(&self, texture: &wgpu::Texture) -> Result<Vec<u8>, RenderError> {
        let device = &self.ctx.device;
        let queue = &self.ctx.queue;
        let width = self.width;
        let height = self.height;

        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let buffer_size = (padded_bytes_per_row * height) as u64;

        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chora-readback-staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("chora-readback-encoder"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &staging,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| RenderError::Readback(format!("channel closed: {e}")))?
            .map_err(|e| RenderError::Readback(format!("map failed: {e}")))?;

        let data = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((unpadded_bytes_per_row * height) as usize);
        for row in 0..height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + unpadded_bytes_per_row as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        drop(data);
        staging.unmap();
        Ok(pixels)
    }
}
