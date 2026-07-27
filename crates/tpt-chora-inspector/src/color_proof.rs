#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorBlindnessMode {
    None,
    Protanopia,
    Deuteranopia,
    Tritanopia,
    Achromatopsia,
    HighContrast,
}

impl ColorBlindnessMode {
    pub fn simulation_matrix(&self) -> [f32; 9] {
        match self {
            Self::None => [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            Self::Protanopia => [0.567, 0.433, 0.0, 0.558, 0.442, 0.0, 0.0, 0.242, 0.758],
            Self::Deuteranopia => [0.625, 0.375, 0.0, 0.7, 0.3, 0.0, 0.0, 0.3, 0.7],
            Self::Tritanopia => [0.95, 0.05, 0.0, 0.0, 0.433, 0.567, 0.0, 0.475, 0.525],
            Self::Achromatopsia => [
                0.299, 0.587, 0.114, 0.299, 0.587, 0.114, 0.299, 0.587, 0.114,
            ],
            Self::HighContrast => [1.5, 0.0, 0.0, 0.0, 1.5, 0.0, 0.0, 0.0, 1.5],
        }
    }
}

pub struct ColorProof {
    mode: ColorBlindnessMode,
    shader_params: [f32; 12],
}

impl ColorProof {
    pub fn new() -> Self {
        Self {
            mode: ColorBlindnessMode::None,
            shader_params: [0.0; 12],
        }
    }

    pub fn set_mode(&mut self, mode: ColorBlindnessMode) {
        self.mode = mode;
        self.recompute_params();
    }

    fn recompute_params(&mut self) {
        let matrix = self.mode.simulation_matrix();
        for (i, &val) in matrix.iter().enumerate() {
            self.shader_params[i] = val;
        }
        self.shader_params[9] = match self.mode {
            ColorBlindnessMode::HighContrast => 1.0,
            _ => 0.0,
        };
    }

    pub fn get_params(&self) -> [f32; 12] {
        self.shader_params
    }

    pub fn current_mode(&self) -> ColorBlindnessMode {
        self.mode
    }
}

impl Default for ColorProof {
    fn default() -> Self {
        Self::new()
    }
}
