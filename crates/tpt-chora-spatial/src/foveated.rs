pub struct FoveatedRenderer {
    inner_radius: f32,
    mid_radius: f32,
    outer_radius: f32,
    inner_samples: u32,
    mid_samples: u32,
    outer_samples: u32,
    enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GazeTarget {
    pub x: f32,
    pub y: f32,
}

impl FoveatedRenderer {
    pub fn new() -> Self {
        Self {
            inner_radius: 0.15,
            mid_radius: 0.35,
            outer_radius: 0.55,
            inner_samples: 1,
            mid_samples: 2,
            outer_samples: 4,
            enabled: true,
        }
    }

    pub fn with_radii(mut self, inner: f32, mid: f32, outer: f32) -> Self {
        self.inner_radius = inner;
        self.mid_radius = mid;
        self.outer_radius = outer;
        self
    }

    pub fn with_sampling(mut self, inner: u32, mid: u32, outer: u32) -> Self {
        self.inner_samples = inner;
        self.mid_samples = mid;
        self.outer_samples = outer;
        self
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn compute_foveation_level(
        &self,
        pixel_x: f32,
        pixel_y: f32,
        gaze: Option<&GazeTarget>,
        screen_width: f32,
        screen_height: f32,
    ) -> FoveationLevel {
        if !self.enabled {
            return FoveationLevel::Full;
        }

        let gaze = match gaze {
            Some(g) => g,
            None => {
                return FoveationLevel::Full;
            }
        };

        let ndc_x = (pixel_x / screen_width) * 2.0 - 1.0;
        let ndc_y = (pixel_y / screen_height) * 2.0 - 1.0;
        let gaze_x = gaze.x;
        let gaze_y = gaze.y;

        let dx = ndc_x - gaze_x;
        let dy = ndc_y - gaze_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < self.inner_radius {
            FoveationLevel::Inner(self.inner_samples)
        } else if dist < self.mid_radius {
            FoveationLevel::Mid(self.mid_samples)
        } else if dist < self.outer_radius {
            FoveationLevel::Outer(self.outer_samples)
        } else {
            FoveationLevel::Full
        }
    }

    pub fn get_shadow_map_size(&self, level: FoveationLevel) -> u32 {
        match level {
            FoveationLevel::Inner(_) => 2048,
            FoveationLevel::Mid(_) => 1024,
            FoveationLevel::Outer(_) => 512,
            FoveationLevel::Full => 256,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FoveationLevel {
    Inner(u32),
    Mid(u32),
    Outer(u32),
    Full,
}
