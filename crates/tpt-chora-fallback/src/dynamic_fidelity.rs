#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FidelityLevel {
    Ultra = 0,
    High = 1,
    Medium = 2,
    Low = 3,
    Minimum = 4,
}

pub struct FidelityProfile {
    pub level: FidelityLevel,
    pub shadows_enabled: bool,
    pub post_processing_enabled: bool,
    pub sdf_fonts: bool,
    pub max_fps: u32,
    pub shadow_map_size: u32,
    pub max_texture_size: u32,
    pub msaa_samples: u32,
    pub volumetric_lighting: bool,
    pub foveated_rendering: bool,
}

pub struct DynamicFidelity {
    current_level: FidelityLevel,
    profiles: Vec<FidelityProfile>,
    frame_time_threshold_ms: f64,
    adaptation_speed: f32,
    current_score: f32,
}

impl DynamicFidelity {
    pub fn new() -> Self {
        Self {
            current_level: FidelityLevel::High,
            profiles: Self::default_profiles(),
            frame_time_threshold_ms: 16.67,
            adaptation_speed: 0.1,
            current_score: 1.0,
        }
    }

    fn default_profiles() -> Vec<FidelityProfile> {
        vec![
            FidelityProfile {
                level: FidelityLevel::Ultra,
                shadows_enabled: true,
                post_processing_enabled: true,
                sdf_fonts: true,
                max_fps: 120,
                shadow_map_size: 4096,
                max_texture_size: 8192,
                msaa_samples: 4,
                volumetric_lighting: true,
                foveated_rendering: false,
            },
            FidelityProfile {
                level: FidelityLevel::High,
                shadows_enabled: true,
                post_processing_enabled: true,
                sdf_fonts: true,
                max_fps: 60,
                shadow_map_size: 2048,
                max_texture_size: 4096,
                msaa_samples: 2,
                volumetric_lighting: true,
                foveated_rendering: false,
            },
            FidelityProfile {
                level: FidelityLevel::Medium,
                shadows_enabled: true,
                post_processing_enabled: true,
                sdf_fonts: true,
                max_fps: 60,
                shadow_map_size: 1024,
                max_texture_size: 2048,
                msaa_samples: 1,
                volumetric_lighting: false,
                foveated_rendering: false,
            },
            FidelityProfile {
                level: FidelityLevel::Low,
                shadows_enabled: false,
                post_processing_enabled: false,
                sdf_fonts: false,
                max_fps: 30,
                shadow_map_size: 512,
                max_texture_size: 1024,
                msaa_samples: 1,
                volumetric_lighting: false,
                foveated_rendering: false,
            },
            FidelityProfile {
                level: FidelityLevel::Minimum,
                shadows_enabled: false,
                post_processing_enabled: false,
                sdf_fonts: false,
                max_fps: 30,
                shadow_map_size: 256,
                max_texture_size: 512,
                msaa_samples: 1,
                volumetric_lighting: false,
                foveated_rendering: false,
            },
        ]
    }

    pub fn current_profile(&self) -> &FidelityProfile {
        self.profiles
            .iter()
            .find(|p| p.level == self.current_level)
            .unwrap_or(&self.profiles[1])
    }

    pub fn update(&mut self, frame_time_ms: f64) {
        let target_score = if frame_time_ms > self.frame_time_threshold_ms * 1.5 {
            0.0
        } else if frame_time_ms < self.frame_time_threshold_ms * 0.8 {
            1.0
        } else {
            0.5
        };

        self.current_score += (target_score - self.current_score) * self.adaptation_speed;

        let new_level = if self.current_score < 0.2 {
            FidelityLevel::Minimum
        } else if self.current_score < 0.4 {
            FidelityLevel::Low
        } else if self.current_score < 0.6 {
            FidelityLevel::Medium
        } else if self.current_score < 0.8 {
            FidelityLevel::High
        } else {
            FidelityLevel::Ultra
        };

        if new_level != self.current_level {
            self.current_level = new_level;
        }
    }

    pub fn current_level(&self) -> FidelityLevel {
        self.current_level
    }

    pub fn set_level(&mut self, level: FidelityLevel) {
        self.current_level = level;
    }
}
