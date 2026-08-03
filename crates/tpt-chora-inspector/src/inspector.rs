use crate::color_proof::ColorBlindnessMode;
use crate::dirty_rect::DirtyRectTracker;
use crate::gpu_timing::GpuTimer;
use crate::heatmap::OverdrawHeatmap;
use std::borrow::Cow;

pub struct ChoraInspector {
    config: InspectorConfig,
    gpu_timer: GpuTimer,
    dirty_tracker: DirtyRectTracker,
    heatmap: OverdrawHeatmap,
    color_proof: ColorBlindnessMode,
    visible: bool,
    draw_call_count: u32,
    triangle_count: u32,
    overlay_pipeline: Option<wgpu::RenderPipeline>,
}

#[derive(Debug, Clone)]
pub struct InspectorConfig {
    pub show_gpu_timing: bool,
    pub show_dirty_rects: bool,
    pub show_overdraw_heatmap: bool,
    pub show_color_proof: bool,
    pub show_a11y_tree: bool,
    pub overlay_opacity: f32,
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            show_gpu_timing: true,
            show_dirty_rects: true,
            show_overdraw_heatmap: false,
            show_color_proof: false,
            show_a11y_tree: false,
            overlay_opacity: 0.8,
        }
    }
}

impl ChoraInspector {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let overlay_pipeline = Self::create_overlay_pipeline(device, width, height);
        Self {
            config: InspectorConfig::default(),
            gpu_timer: GpuTimer::new(device, 256),
            dirty_tracker: DirtyRectTracker::new(),
            heatmap: OverdrawHeatmap::new(width, height),
            color_proof: ColorBlindnessMode::None,
            visible: false,
            draw_call_count: 0,
            triangle_count: 0,
            overlay_pipeline,
        }
    }

    fn create_overlay_pipeline(
        device: &wgpu::Device,
        _width: u32,
        _height: u32,
    ) -> Option<wgpu::RenderPipeline> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("inspector-overlay-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                r#"
                struct VertexOutput {
                    @builtin(position) position: vec4<f32>,
                    @location(0) color: vec4<f32>,
                };

                @vertex
                fn vs_main(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
                    var positions = array<vec2<f32>, 6>(
                        vec2<f32>(-1.0, -1.0),
                        vec2<f32>( 1.0, -1.0),
                        vec2<f32>( 1.0,  0.92),
                        vec2<f32>(-1.0, -1.0),
                        vec2<f32>( 1.0,  0.92),
                        vec2<f32>(-1.0,  0.92),
                    );
                    var colors = array<vec4<f32>, 6>(
                        vec4<f32>(0.0, 0.0, 0.0, 0.7),
                        vec4<f32>(0.0, 0.0, 0.0, 0.7),
                        vec4<f32>(0.0, 0.0, 0.0, 0.7),
                        vec4<f32>(0.0, 0.0, 0.0, 0.7),
                        vec4<f32>(0.0, 0.0, 0.0, 0.7),
                        vec4<f32>(0.0, 0.0, 0.0, 0.7),
                    );

                    var out: VertexOutput;
                    out.position = vec4<f32>(positions[vertex_idx], 0.0, 1.0);
                    out.color = colors[vertex_idx];
                    return out;
                }

                @fragment
                fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                    return in.color;
                }
                "#,
            )),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("inspector-overlay-pl"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        Some(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("inspector-overlay-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            }),
        )
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_config(&mut self, config: InspectorConfig) {
        self.config = config;
    }

    pub fn begin_frame(&mut self) {
        self.draw_call_count = 0;
        self.triangle_count = 0;
        self.dirty_tracker.begin_frame();
    }

    pub fn record_draw_call(&mut self, triangles: u32) {
        self.draw_call_count += 1;
        self.triangle_count += triangles;
    }

    pub fn record_dirty_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.dirty_tracker.mark_dirty(x, y, width, height);
    }

    pub fn end_frame(&mut self, device: &wgpu::Device) -> InspectorFrameData {
        InspectorFrameData {
            draw_calls: self.draw_call_count,
            triangles: self.triangle_count,
            dirty_rects: self.dirty_tracker.current_dirty_rects(),
            gpu_timings: self.gpu_timer.readback(device),
        }
    }

    pub fn set_color_proof(&mut self, mode: ColorBlindnessMode) {
        self.color_proof = mode;
    }

    pub fn get_color_proof_matrix(&self) -> [f32; 9] {
        self.color_proof.simulation_matrix()
    }

    pub fn render_overlay(
        &self,
        _device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        _screen_width: u32,
        _screen_height: u32,
    ) {
        if !self.visible {
            return;
        }

        if let Some(ref pipeline) = self.overlay_pipeline {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("inspector-overlay-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(pipeline);
            pass.draw(0..6, 0..1);
        }
    }

    pub fn gpu_timer(&self) -> &GpuTimer {
        &self.gpu_timer
    }

    pub fn heatmap(&self) -> &OverdrawHeatmap {
        &self.heatmap
    }

    pub fn heatmap_mut(&mut self) -> &mut OverdrawHeatmap {
        &mut self.heatmap
    }
}

#[derive(Debug, Clone)]
pub struct InspectorFrameData {
    pub draw_calls: u32,
    pub triangles: u32,
    pub dirty_rects: Vec<crate::dirty_rect::DirtyRect>,
    pub gpu_timings: Vec<crate::gpu_timing::TimingResult>,
}
