use crate::devices::GestureIntent;

pub struct IntentResolver {
    last_click_time: u64,
    last_click_x: f32,
    last_click_y: f32,
    double_click_threshold_ms: u64,
    long_press_threshold_ms: u64,
    drag_threshold: f32,
    touch_start_x: f32,
    touch_start_y: f32,
    touch_start_time: u64,
    dragging: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum InteractionIntent {
    Click,
    DoubleClick,
    LongPress,
    DragStart,
    DragMove,
    DragEnd,
    SwipeUp,
    SwipeDown,
    SwipeLeft,
    SwipeRight,
    PinchZoomStart,
    PinchZoomMove,
    PinchZoomEnd,
    PanStart,
    PanMove,
    PanEnd,
    Hover,
    Focus,
    Blur,
}

impl IntentResolver {
    pub fn new() -> Self {
        Self {
            last_click_time: 0,
            last_click_x: 0.0,
            last_click_y: 0.0,
            double_click_threshold_ms: 300,
            long_press_threshold_ms: 500,
            drag_threshold: 5.0,
            touch_start_x: 0.0,
            touch_start_y: 0.0,
            touch_start_time: 0,
            dragging: false,
        }
    }

    pub fn resolve(
        &mut self,
        gesture: GestureIntent,
        x: f32,
        y: f32,
        timestamp: u64,
    ) -> InteractionIntent {
        match gesture {
            GestureIntent::Click => {
                if !self.dragging
                    && timestamp - self.last_click_time < self.double_click_threshold_ms
                    && (x - self.last_click_x).abs() < self.drag_threshold
                    && (y - self.last_click_y).abs() < self.drag_threshold
                {
                    self.last_click_time = 0;
                    InteractionIntent::DoubleClick
                } else {
                    self.last_click_time = timestamp;
                    self.last_click_x = x;
                    self.last_click_y = y;
                    InteractionIntent::Click
                }
            }
            GestureIntent::LongPress => {
                self.touch_start_x = x;
                self.touch_start_y = y;
                self.touch_start_time = timestamp;
                self.dragging = false;
                InteractionIntent::LongPress
            }
            GestureIntent::Drag => {
                let dx = x - self.touch_start_x;
                let dy = y - self.touch_start_y;
                let dist = (dx * dx + dy * dy).sqrt();

                if !self.dragging && dist > self.drag_threshold {
                    self.dragging = true;
                    InteractionIntent::DragStart
                } else if self.dragging {
                    InteractionIntent::DragMove
                } else {
                    InteractionIntent::Hover
                }
            }
            GestureIntent::PinchZoom => InteractionIntent::PinchZoomStart,
            GestureIntent::Pan => InteractionIntent::PanStart,
            GestureIntent::DoubleClick => InteractionIntent::DoubleClick,
            GestureIntent::Swipe { direction } => match direction {
                crate::devices::SwipeDirection::Up => InteractionIntent::SwipeUp,
                crate::devices::SwipeDirection::Down => InteractionIntent::SwipeDown,
                crate::devices::SwipeDirection::Left => InteractionIntent::SwipeLeft,
                crate::devices::SwipeDirection::Right => InteractionIntent::SwipeRight,
            },
        }
    }

    pub fn end_drag(&mut self) -> InteractionIntent {
        self.dragging = false;
        InteractionIntent::DragEnd
    }
}
