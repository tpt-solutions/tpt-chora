use crate::devices::DeviceEvent;

pub struct HapticRouter {
    platform: PlatformHaptics,
    last_events: Vec<HapticEvent>,
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
    #[allow(dead_code)]
    XrControllerRumble {
        controller_id: u32,
    },
    Unsupported,
}

impl HapticRouter {
    pub fn new() -> Self {
        let platform = Self::detect_platform();
        Self {
            platform,
            last_events: Vec::new(),
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
            DeviceEvent::GamepadButton { pressed: true, .. } => Some(HapticPattern::Medium),
            DeviceEvent::GamepadAxis { value, .. } if value.abs() > 0.8 => {
                Some(HapticPattern::Light)
            }
            _ => None,
        }
    }

    pub fn play(&mut self, pattern: &HapticPattern) -> Result<(), crate::InputError> {
        let events = Self::translate_pattern(pattern);
        self.last_events = events;

        match &self.platform {
            PlatformHaptics::CoreHaptics => self.play_corehaptics(pattern),
            PlatformHaptics::AndroidVibrator => self.play_android(pattern),
            PlatformHaptics::XrControllerRumble { .. } => self.play_xr_rumble(pattern),
            PlatformHaptics::Unsupported => Ok(()),
        }
    }

    pub fn translate_pattern(pattern: &HapticPattern) -> Vec<HapticEvent> {
        match pattern {
            HapticPattern::Light => vec![HapticEvent {
                intensity: 0.3,
                duration_ms: 10,
                delay_ms: 0,
            }],
            HapticPattern::Medium => vec![HapticEvent {
                intensity: 0.6,
                duration_ms: 20,
                delay_ms: 0,
            }],
            HapticPattern::Heavy => vec![HapticEvent {
                intensity: 1.0,
                duration_ms: 40,
                delay_ms: 0,
            }],
            HapticPattern::Selection => vec![HapticEvent {
                intensity: 0.2,
                duration_ms: 5,
                delay_ms: 0,
            }],
            HapticPattern::Success => vec![
                HapticEvent {
                    intensity: 0.5,
                    duration_ms: 15,
                    delay_ms: 0,
                },
                HapticEvent {
                    intensity: 0.0,
                    duration_ms: 10,
                    delay_ms: 15,
                },
                HapticEvent {
                    intensity: 0.8,
                    duration_ms: 25,
                    delay_ms: 25,
                },
            ],
            HapticPattern::Warning => vec![
                HapticEvent {
                    intensity: 0.7,
                    duration_ms: 20,
                    delay_ms: 0,
                },
                HapticEvent {
                    intensity: 0.0,
                    duration_ms: 15,
                    delay_ms: 20,
                },
                HapticEvent {
                    intensity: 0.7,
                    duration_ms: 20,
                    delay_ms: 35,
                },
            ],
            HapticPattern::Error => vec![
                HapticEvent {
                    intensity: 1.0,
                    duration_ms: 30,
                    delay_ms: 0,
                },
                HapticEvent {
                    intensity: 0.0,
                    duration_ms: 10,
                    delay_ms: 30,
                },
                HapticEvent {
                    intensity: 1.0,
                    duration_ms: 30,
                    delay_ms: 40,
                },
                HapticEvent {
                    intensity: 0.0,
                    duration_ms: 10,
                    delay_ms: 70,
                },
                HapticEvent {
                    intensity: 1.0,
                    duration_ms: 50,
                    delay_ms: 80,
                },
            ],
            HapticPattern::Custom { pattern } => pattern.clone(),
        }
    }

    fn play_corehaptics(&self, _pattern: &HapticPattern) -> Result<(), crate::InputError> {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            let events = Self::translate_pattern(_pattern);
            for event in &events {
                if event.delay_ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(event.delay_ms));
                }
                std::thread::sleep(std::time::Duration::from_millis(event.duration_ms));
            }
            Ok(())
        }
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        {
            Err(crate::InputError::HapticNotSupported)
        }
    }

    fn play_android(&self, _pattern: &HapticPattern) -> Result<(), crate::InputError> {
        #[cfg(target_os = "android")]
        {
            let events = Self::translate_pattern(_pattern);
            for event in &events {
                if event.delay_ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(event.delay_ms));
                }
                std::thread::sleep(std::time::Duration::from_millis(event.duration_ms));
            }
            Ok(())
        }
        #[cfg(not(target_os = "android"))]
        {
            Err(crate::InputError::HapticNotSupported)
        }
    }

    fn play_xr_rumble(&self, _pattern: &HapticPattern) -> Result<(), crate::InputError> {
        Err(crate::InputError::HapticNotSupported)
    }

    pub fn last_events(&self) -> &[HapticEvent] {
        &self.last_events
    }
}

impl Default for HapticRouter {
    fn default() -> Self {
        Self::new()
    }
}
