pub struct ViewportGuard {
    bounds: [f32; 4],
    scissor_enabled: bool,
    stencil_enabled: bool,
}

impl ViewportGuard {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            bounds: [x, y, x + width, y + height],
            scissor_enabled: true,
            stencil_enabled: true,
        }
    }

    pub fn from_bounds(bounds: [f32; 4]) -> Self {
        Self {
            bounds,
            scissor_enabled: true,
            stencil_enabled: true,
        }
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub fn set_scissor_state(&mut self, enabled: bool) {
        self.scissor_enabled = enabled;
    }

    pub fn set_stencil_state(&mut self, enabled: bool) {
        self.stencil_enabled = enabled;
    }

    pub fn is_point_inside(&self, x: f32, y: f32) -> bool {
        x >= self.bounds[0] && x <= self.bounds[2] && y >= self.bounds[1] && y <= self.bounds[3]
    }

    pub fn intersects(&self, other: &ViewportGuard) -> bool {
        self.bounds[0] < other.bounds[2]
            && self.bounds[2] > other.bounds[0]
            && self.bounds[1] < other.bounds[3]
            && self.bounds[3] > other.bounds[1]
    }

    pub fn to_scissor_rect(&self, screen_height: f32) -> [u32; 4] {
        let x = self.bounds[0].max(0.0) as u32;
        let y = (screen_height - self.bounds[3]).max(0.0) as u32;
        let width = (self.bounds[2] - self.bounds[0]).max(0.0) as u32;
        let height = (self.bounds[3] - self.bounds[1]).max(0.0) as u32;
        [x, y, width, height]
    }

    pub fn apply_scissor(&self, pass: &mut wgpu::RenderPass<'_>, screen_height: f32) {
        if self.scissor_enabled {
            let rect = self.to_scissor_rect(screen_height);
            pass.set_scissor_rect(rect[0], rect[1], rect[2], rect[3]);
        }
    }

    pub fn setup_stencil(
        &self,
        _device: &wgpu::Device,
        stencil_value: u32,
    ) -> Option<wgpu::DepthStencilState> {
        if !self.stencil_enabled {
            return None;
        }

        let compare = wgpu::CompareFunction::Equal;
        let pass_op = wgpu::StencilOperation::Keep;
        let read_mask = (stencil_value | 0xFF).min(0xFF) as u32;
        let write_mask = 0xFFu32;

        Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32FloatStencil8,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op,
                },
                back: wgpu::StencilFaceState {
                    compare,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op,
                },
                read_mask,
                write_mask,
            },
            bias: wgpu::DepthBiasState::default(),
        })
    }

    pub fn apply_stencil_reference(&self, pass: &mut wgpu::RenderPass<'_>, stencil_value: u32) {
        if self.stencil_enabled {
            pass.set_stencil_reference(stencil_value);
        }
    }
}
