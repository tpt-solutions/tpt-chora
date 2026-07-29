// `deny` rather than `forbid`: the optional `native-haptics-backends`
// feature calls real OS haptics APIs (CoreHaptics / Android Vibrator), which
// need `unsafe` FFI at their call sites (each annotated with its own
// `#[allow(unsafe_code)]` and a `// SAFETY:` justification) — everything
// else in this crate stays safe.
#![deny(unsafe_code)]

pub mod devices;
pub mod error;
pub mod haptics;
pub mod hit_test;
pub mod intent;

pub use devices::{
    DeviceCapability, DeviceEvent, GamepadState, GazeState, GestureIntent, InputDeviceInfo,
    InputDeviceKind, InputEvent, InputState, KeyboardState, MouseState, PenState, TouchState,
};
pub use error::InputError;
pub use haptics::{HapticFeedback, HapticPattern, HapticRouter};
pub use hit_test::{BoundingBoxHierarchy, GpuHitTest, HitResult};
pub use intent::{IntentResolver, InteractionIntent};
