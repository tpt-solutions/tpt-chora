use crate::devices::DeviceEvent;

pub struct HapticRouter {
    patterns: Vec<HapticPattern>,
    active_pattern: Option<usize>,
    platform: PlatformHaptics,
}

#[derive(Debug, Clone)]
pub enum HapticPattern {
    Light,
    Medium,
    Heavy,
    Selection,
    Success,
    Warning,
    Error,
    Custom { pattern: Vec<HapticEvent> },
}

#[derive(Debug, Clone, Copy)]
pub struct HapticEvent {
    pub intensity: f32,
    pub duration_ms: u64,
    pub delay_ms: u64,
}

pub struct HapticFeedback {
    pub pattern: HapticPattern,
    pub repeat: bool,
}

enum PlatformHaptics {
    CoreHaptics,
    AndroidVibrator,
    XrControllerRumble { controller_id: u32 },
    Unsupported,
}

impl HapticRouter {
    pub fn new() -> Self {
        let platform = Self::detect_platform();
        Self {
            patterns: Vec::new(),
            active_pattern: None,
            platform,
        }
    }

    fn detect_platform() -> PlatformHaptics {
        if cfg!(target_os = "ios") || cfg!(target_os = "macos") {
            PlatformHaptics::CoreHaptics
        } else if cfg!(target_os = "android") {
            PlatformHaptics::AndroidVibrator
        } else {
            PlatformHaptics::Unsupported
        }
    }

    pub fn route_event(&self, event: &DeviceEvent) -> Option<HapticPattern> {
        match event {
            DeviceEvent::MouseDown { .. } => Some(HapticPattern::Light),
            DeviceEvent::KeyDown { .. } => Some(HapticPattern::Selection),
            DeviceEvent::TouchBegin { .. } => Some(HapticPattern::Medium),
            DeviceEvent::PenDown { pressure, .. } => {
                if *pressure > 0.8 {
                    Some(HapticPattern::Heavy)
                } else {
                    Some(HapticPattern::Light)
                }
            }
            DeviceEvent::PinchStart { .. } => Some(HapticPattern::Selection),
            _ => None,
        }
    }

    pub fn play(&self, pattern: &HapticPattern) -> Result<(), crate::InputError> {
        match &self.platform {
            PlatformHaptics::CoreHaptics => self.play_corehaptics(pattern),
            PlatformHaptics::AndroidVibrator => self.play_android(pattern),
            PlatformHaptics::XrControllerRumble { .. } => self.play_xr_rumble(pattern),
            PlatformHaptics::Unsupported => Ok(()),
        }
    }

    fn play_corehaptics(&self, _pattern: &HapticPattern) -> Result<(), crate::InputError> {
        Ok(())
    }

    fn play_android(&self, _pattern: &HapticPattern) -> Result<(), crate::InputError> {
        Ok(())
    }

    fn play_xr_rumble(&self, _pattern: &HapticPattern) -> Result<(), crate::InputError> {
        Ok(())
    }
}
