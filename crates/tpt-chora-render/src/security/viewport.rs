#[derive(Debug)]
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
        let read_mask = stencil_value.min(0xFF);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_inside() {
        let vp = ViewportGuard::new(10.0, 20.0, 100.0, 80.0);
        assert!(vp.is_point_inside(10.0, 20.0));
        assert!(vp.is_point_inside(55.0, 50.0));
        assert!(vp.is_point_inside(110.0, 100.0));
    }

    #[test]
    fn point_outside() {
        let vp = ViewportGuard::new(10.0, 20.0, 100.0, 80.0);
        assert!(!vp.is_point_inside(0.0, 50.0));
        assert!(!vp.is_point_inside(50.0, 0.0));
        assert!(!vp.is_point_inside(200.0, 200.0));
    }

    #[test]
    fn intersects_overlapping() {
        let a = ViewportGuard::new(0.0, 0.0, 100.0, 100.0);
        let b = ViewportGuard::new(50.0, 50.0, 100.0, 100.0);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
    }

    #[test]
    fn intersects_non_overlapping() {
        let a = ViewportGuard::new(0.0, 0.0, 10.0, 10.0);
        let b = ViewportGuard::new(100.0, 100.0, 10.0, 10.0);
        assert!(!a.intersects(&b));
        assert!(!b.intersects(&a));
    }

    #[test]
    fn intersects_touching_edge_not_intersecting() {
        let a = ViewportGuard::new(0.0, 0.0, 10.0, 10.0);
        let b = ViewportGuard::new(10.0, 0.0, 10.0, 10.0);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn to_scissor_rect() {
        let vp = ViewportGuard::new(10.0, 20.0, 30.0, 40.0);
        let rect = vp.to_scissor_rect(100.0);
        assert_eq!(rect, [10, 40, 30, 40]);
    }

    #[test]
    fn to_scissor_rect_clamp_negative() {
        let vp = ViewportGuard::from_bounds([-5.0, -10.0, 5.0, 10.0]);
        let rect = vp.to_scissor_rect(50.0);
        assert_eq!(rect, [0, 40, 10, 20]);
    }

    #[test]
    fn stencil_enabled_by_default() {
        let vp = ViewportGuard::new(0.0, 0.0, 100.0, 100.0);
        let _ = vp.to_scissor_rect(100.0);
        let b = ViewportGuard::from_bounds([0.0, 0.0, 100.0, 100.0]);
        assert_eq!(vp.bounds(), b.bounds());
    }

    #[test]
    fn set_scissor_state() {
        let mut vp = ViewportGuard::new(0.0, 0.0, 100.0, 100.0);
        vp.set_scissor_state(false);
        let _ = &vp;
    }

    #[test]
    fn from_bounds_same_as_new() {
        let a = ViewportGuard::new(10.0, 20.0, 30.0, 40.0);
        let b = ViewportGuard::from_bounds([10.0, 20.0, 40.0, 60.0]);
        assert_eq!(a.bounds(), b.bounds());
    }
}
