pub struct WebComponentConfig {
    pub name: String,
    pub shadow_dom: bool,
    pub attributes: Vec<String>,
    pub events: Vec<String>,
}

pub struct ComponentBridge {
    config: WebComponentConfig,
    buffer: Vec<u8>,
    event_log: Vec<(String, Vec<u8>)>,
    width: u32,
    height: u32,
    renderer: Option<tpt_chora_render::Renderer>,
}

impl ComponentBridge {
    pub fn new(config: WebComponentConfig) -> Self {
        Self {
            config,
            buffer: Vec::new(),
            event_log: Vec::new(),
            width: 0,
            height: 0,
            renderer: None,
        }
    }

    pub fn initialize(&mut self) {
        self.width = 300;
        self.height = 150;
        self.buffer = vec![0u8; (self.width * self.height * 4) as usize];
        self.renderer = tpt_chora_render::Renderer::new_headless(self.width, self.height).ok();
    }

    pub fn render(&mut self, width: u32, height: u32) -> Vec<u8> {
        // Re-create the headless renderer at the requested size instead of
        // silently returning placeholder pixels whenever a caller asks for
        // anything other than the size `initialize()` happened to pick.
        if self.renderer.is_none() || width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.buffer = vec![0u8; (width * height * 4) as usize];
            self.renderer = tpt_chora_render::Renderer::new_headless(width, height).ok();
        }

        if let Some(ref renderer) = self.renderer {
            if let Ok(pixels) = renderer.render_frame() {
                if pixels.len() == (width * height * 4) as usize {
                    return pixels;
                }
            }
        }

        // Only reached if the renderer itself couldn't be created or
        // couldn't render (e.g. no GPU adapter) — a real fallback, not a
        // silent one for a size mismatch that a resize should have fixed.
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let name_hash = self
            .config
            .name
            .bytes()
            .fold(0u8, |acc, b| acc.wrapping_add(b));

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let is_border = x == 0 || x == width - 1 || y == 0 || y == height - 1;

                if is_border {
                    pixels[idx] = name_hash;
                    pixels[idx + 1] = name_hash.wrapping_add(80);
                    pixels[idx + 2] = name_hash.wrapping_add(160);
                    pixels[idx + 3] = 255;
                } else {
                    pixels[idx] = 240;
                    pixels[idx + 1] = 240;
                    pixels[idx + 2] = 240;
                    pixels[idx + 3] = 255;
                }

                let header_height = 28.min(height);
                if y < header_height && x > 0 && x < width - 1 {
                    pixels[idx] = name_hash;
                    pixels[idx + 1] = name_hash.wrapping_add(50);
                    pixels[idx + 2] = name_hash.wrapping_add(100);
                    pixels[idx + 3] = 255;
                }
            }
        }

        pixels
    }

    pub fn handle_event(&mut self, event_type: &str, data: &[u8]) {
        self.event_log.push((event_type.to_string(), data.to_vec()));
    }

    pub fn event_log(&self) -> &[(String, Vec<u8>)] {
        &self.event_log
    }

    pub fn config(&self) -> &WebComponentConfig {
        &self.config
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bridge() -> ComponentBridge {
        ComponentBridge::new(WebComponentConfig {
            name: "test-widget".into(),
            shadow_dom: false,
            attributes: vec![],
            events: vec![],
        })
    }

    #[test]
    fn render_at_a_different_size_than_initialize_returns_correctly_sized_buffer() {
        let mut bridge = bridge();
        bridge.initialize();
        assert_eq!(bridge.render(300, 150).len(), 300 * 150 * 4);
        // Previously this silently returned placeholder pixels rather than
        // a re-rendered buffer for any size other than the 300x150 that
        // `initialize()` hardcoded.
        let pixels = bridge.render(64, 48);
        assert_eq!(pixels.len(), 64 * 48 * 4);
    }

    #[test]
    fn render_without_initialize_still_returns_correctly_sized_buffer() {
        let mut bridge = bridge();
        let pixels = bridge.render(32, 32);
        assert_eq!(pixels.len(), 32 * 32 * 4);
    }
}
