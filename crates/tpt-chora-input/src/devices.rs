use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DeviceCapability: u32 {
        const MOUSE = 0b0000_0001;
        const KEYBOARD = 0b0000_0010;
        const TOUCH = 0b0000_0100;
        const PEN = 0b0000_1000;
        const GAMEPAD = 0b0001_0000;
        const XR_CONTROLLER = 0b0010_0000;
        const EYE_TRACKING = 0b0100_0000;
        const GESTURE = 0b1000_0000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDeviceKind {
    Mouse,
    Keyboard,
    Touch,
    Pen,
    Gamepad,
    XrController,
}

#[derive(Debug, Clone)]
pub enum DeviceEvent {
    MouseMove { x: f32, y: f32 },
    MouseDown { button: MouseButton, x: f32, y: f32 },
    MouseUp { button: MouseButton, x: f32, y: f32 },
    MouseWheel { delta_x: f32, delta_y: f32 },
    KeyDown { key: KeyCode, modifiers: Modifiers },
    KeyUp { key: KeyCode, modifiers: Modifiers },
    TouchBegin { id: u64, x: f32, y: f32 },
    TouchMove { id: u64, x: f32, y: f32 },
    TouchEnd { id: u64, x: f32, y: f32 },
    PenDown { x: f32, y: f32, pressure: f32, tilt_x: f32, tilt_y: f32 },
    PenMove { x: f32, y: f32, pressure: f32, tilt_x: f32, tilt_y: f32 },
    PenUp { x: f32, y: f32 },
    GamepadButton { controller: u32, button: u32, pressed: bool },
    GamepadAxis { controller: u32, axis: u32, value: f32 },
    GazePoint { x: f32, y: f32, confidence: f32 },
    PinchStart { confidence: f32 },
    PinchEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    Space, Enter, Escape, Tab, Backspace, Delete,
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Home, End, PageUp, PageDown,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Shift, Control, Alt, Meta,
    Unknown(u32),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureIntent {
    Click,
    DoubleClick,
    LongPress,
    Drag,
    PinchZoom,
    Pan,
    Swipe { direction: SwipeDirection },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Default)]
pub struct InputState {
    pub mouse: MouseState,
    pub keyboard: KeyboardState,
    pub touch: TouchState,
    pub pen: PenState,
    pub gamepad: GamepadState,
    pub gaze: GazeState,
    pub active_devices: DeviceCapability,
}

#[derive(Debug, Default, Clone)]
pub struct MouseState {
    pub x: f32,
    pub y: f32,
    pub buttons: u8,
    pub scroll_x: f32,
    pub scroll_y: f32,
}

#[derive(Debug, Default, Clone)]
pub struct KeyboardState {
    pub keys_pressed: Vec<KeyCode>,
    pub modifiers: Modifiers,
}

#[derive(Debug, Default, Clone)]
pub struct TouchState {
    pub touches: Vec<TouchPoint>,
}

#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

#[derive(Debug, Default, Clone)]
pub struct PenState {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub active: bool,
}

#[derive(Debug, Default, Clone)]
pub struct GamepadState {
    pub axes: Vec<f32>,
    pub buttons: Vec<bool>,
}

#[derive(Debug, Default, Clone)]
pub struct GazeState {
    pub x: f32,
    pub y: f32,
    pub confidence: f32,
    pub active: bool,
}

pub struct InputEvent {
    pub device: InputDeviceKind,
    pub event: DeviceEvent,
    pub timestamp: u64,
}

pub struct InputDeviceInfo {
    pub kind: InputDeviceKind,
    pub capabilities: DeviceCapability,
    pub name: String,
    pub connected: bool,
}
