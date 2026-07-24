use crate::gpu_timing::GpuTimer;
use crate::dirty_rect::DirtyRectTracker;
use crate::heatmap::OverdrawHeatmap;
use crate::color_proof::{ColorBlindnessMode, ColorProof};

pub struct ChoraInspector {
    config: InspectorConfig,
    gpu_timer: GpuTimer,
    dirty_tracker: DirtyRectTracker,
    heatmap: OverdrawHeatmap,
    color_proof: ColorBlindnessMode,
    visible: bool,
    draw_call_count: u32,
    triangle_count: u32,
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
        Self {
            config: InspectorConfig::default(),
            gpu_timer: GpuTimer::new(device),
            dirty_tracker: DirtyRectTracker::new(),
            heatmap: OverdrawHeatmap::new(width, height),
            color_proof: ColorBlindnessMode::None,
            visible: false,
            draw_call_count: 0,
            triangle_count: 0,
        }
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

    pub fn end_frame(&mut self) -> InspectorFrameData {
        InspectorFrameData {
            draw_calls: self.draw_call_count,
            triangles: self.triangle_count,
            dirty_rects: self.dirty_tracker.current_dirty_rects(),
            gpu_timings: self.gpu_timer.readback(),
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
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        screen_width: u32,
        screen_height: u32,
    ) {
    }
}

#[derive(Debug, Clone)]
pub struct InspectorFrameData {
    pub draw_calls: u32,
    pub triangles: u32,
    pub dirty_rects: Vec<crate::dirty_rect::DirtyRect>,
    pub gpu_timings: Vec<crate::gpu_timing::TimingResult>,
}
