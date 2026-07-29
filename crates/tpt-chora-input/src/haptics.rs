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
        #[cfg(all(
            feature = "native-haptics-backends",
            any(target_os = "macos", target_os = "ios")
        ))]
        {
            use objc2::ffi::{objc_msgSend, objc_msgSend_stret};
            use objc2::runtime::{Object, Sel};
            use std::ffi::c_void;

            extern "C" {
                fn objc_getClass(name: *const u8) -> *mut Object;
                fn sel_getUid(name: *const u8) -> Sel;
            }

            let kCHHapticEngine = b"CHHapticEngine\0" as *const u8;
            let kCHHapticPatternPlayer = b"CHHapticPatternPlayer\0" as *const u8;
            let kCHHapticPattern = b"CHHapticPattern\0" as *const u8;
            let kCHHapticEngineStart = b"startAndReturnError:\0" as *const u8;
            let kCHHapticEngineCreatePlayer = b"createPlayerWithPattern:error:\0" as *const u8;
            let kCHHapticPatternPlayerStart = b"startAtTime:error:\0" as *const u8;

            let events = Self::translate_pattern(_pattern);
            let total_duration_ms: u64 = events.iter().map(|e| e.duration_ms + e.delay_ms).sum();

            unsafe {
                let engine_class = objc_getClass(kCHHapticEngine as *const u8);
                let start_sel = sel_getUid(kCHHapticEngineStart as *const u8);
                let create_player_sel = sel_getUid(kCHHapticEngineCreatePlayer as *const u8);
                let start_player_sel = sel_getUid(kCHHapticPatternPlayerStart as *const u8);

                let _engine: *mut c_void =
                    objc_msgSend(engine_class, start_sel, std::ptr::null_mut::<*mut c_void>());
                let _player: *mut c_void = objc_msgSend(
                    _engine,
                    create_player_sel,
                    std::ptr::null_mut::<*mut c_void>(),
                    std::ptr::null_mut::<*mut c_void>(),
                );
                objc_msgSend(
                    _player,
                    start_player_sel,
                    0.0,
                    std::ptr::null_mut::<*mut c_void>(),
                );
                std::thread::sleep(std::time::Duration::from_millis(total_duration_ms));
            }
            Ok(())
        }
        #[cfg(all(
            not(feature = "native-haptics-backends"),
            any(target_os = "macos", target_os = "ios")
        ))]
        {
            std::thread::sleep(std::time::Duration::from_millis(10));
            Ok(())
        }
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        {
            Err(crate::InputError::HapticNotSupported)
        }
    }

    fn play_android(&self, _pattern: &HapticPattern) -> Result<(), crate::InputError> {
        #[cfg(all(feature = "native-haptics-backends", target_os = "android"))]
        {
            use jni::JNIEnv;
            let events = Self::translate_pattern(_pattern);
            let total_duration_ms: u64 = events.iter().map(|e| e.duration_ms + e.delay_ms).sum();
            let activity = ndk::android::activity::AndroidActivity::from_env(
                unsafe { jni::JavaVM::from_raw(std::ptr::null_mut() as _) }
                    .unwrap()
                    .get_env()
                    .unwrap(),
            );
            let vib_service =
                activity.get_system_service(ndk::android::activity::SystemService::VibratorService);
            if let Ok(vib) = vib_service {
                let _ = vib.vibrate(total_duration_ms);
            }
            Ok(())
        }
        #[cfg(all(not(feature = "native-haptics-backends"), target_os = "android"))]
        {
            std::thread::sleep(std::time::Duration::from_millis(10));
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
