#[derive(Debug, Clone)]
pub struct DirtyRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub struct DirtyRectTracker {
    current_rects: Vec<DirtyRect>,
    previous_rects: Vec<DirtyRect>,
    merge_threshold: f32,
}

impl DirtyRectTracker {
    pub fn new() -> Self {
        Self {
            current_rects: Vec::new(),
            previous_rects: Vec::new(),
            merge_threshold: 10.0,
        }
    }

    pub fn begin_frame(&mut self) {
        self.previous_rects = self.current_rects.clone();
        self.current_rects.clear();
    }

    pub fn mark_dirty(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let new_rect = DirtyRect { x, y, width, height };

        let mut merged = false;
        for existing in &mut self.current_rects {
            if Self::can_merge(existing, &new_rect, self.merge_threshold) {
                Self::merge_into(existing, &new_rect);
                merged = true;
                break;
            }
        }

        if !merged {
            self.current_rects.push(new_rect);
        }
    }

    fn can_merge(a: &DirtyRect, b: &DirtyRect, threshold: f32) -> bool {
        !(a.x + a.width + threshold < b.x
            || b.x + b.width + threshold < a.x
            || a.y + a.height + threshold < b.y
            || b.y + b.height + threshold < a.y)
    }

    fn merge_into(a: &mut DirtyRect, b: &DirtyRect) {
        let min_x = a.x.min(b.x);
        let min_y = a.y.min(b.y);
        let max_x = (a.x + a.width).max(b.x + b.width);
        let max_y = (a.y + a.height).max(b.y + b.height);
        a.x = min_x;
        a.y = min_y;
        a.width = max_x - min_x;
        a.height = max_y - min_y;
    }

    pub fn current_dirty_rects(&self) -> Vec<DirtyRect> {
        self.current_rects.clone()
    }

    pub fn has_dirty_rects(&self) -> bool {
        !self.current_rects.is_empty()
    }

    pub fn total_dirty_area(&self) -> f32 {
        self.current_rects
            .iter()
            .map(|r| r.width * r.height)
            .sum()
    }

    pub fn merge_all(&mut self) {
        if self.current_rects.len() <= 1 {
            return;
        }

        let mut merged = self.current_rects.remove(0);
        let remaining: Vec<DirtyRect> = self.current_rects.drain(..).collect();
        for rect in remaining {
            Self::merge_into(&mut merged, &rect);
        }
        self.current_rects.push(merged);
    }
}
