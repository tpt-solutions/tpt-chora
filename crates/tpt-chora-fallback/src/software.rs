//! Tier 1 software rasterizer: renders a small immediate-mode scene graph of
//! filled/outlined primitives into an RGBA8 framebuffer entirely on the CPU.
//!
//! This is the crate's genuine CPU/software rasterization tier — it runs with
//! no GPU adapter, no windowing system, and no `wgpu` at all. It exists so a
//! tpt-chora scene can still be rendered (degraded, unaccelerated) on a
//! machine with no usable graphics driver, complementing the GPU-backed
//! `HeadlessRenderer` (Tier 2) and `DynamicFidelity` (Tier 3).

/// An immediate-mode drawing command. Commands are processed in order; clip
/// commands push/pop a stack of clip rectangles that bound every later draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Axis-aligned filled rectangle in device pixels.
    FillRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    },
    /// Filled circle with an explicit center and radius.
    FillCircle {
        cx: f32,
        cy: f32,
        radius: f32,
        color: [f32; 4],
    },
    /// Filled triangle given as three device-space vertices.
    FillTriangle {
        v0: [f32; 2],
        v1: [f32; 2],
        v2: [f32; 2],
        color: [f32; 4],
    },
    /// Axis-aligned stroked rectangle (outline only).
    StrokeRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        thickness: f32,
        color: [f32; 4],
    },
    /// Line segment stroked with a fixed thickness (device pixels).
    StrokeLine {
        p0: [f32; 2],
        p1: [f32; 2],
        thickness: f32,
        color: [f32; 4],
    },
    /// Push a clip rectangle onto the clip stack.
    PushClip {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Pop the most recent clip rectangle.
    PopClip,
}

/// CPU rasterizer producing an RGBA8 framebuffer.
pub struct SoftwareRasterizer {
    width: u32,
    height: u32,
    /// Supersampling factor per axis: `sample_rate^2` point samples per pixel.
    sample_rate: u32,
    buffer: Vec<u8>,
    clip_stack: Vec<[f32; 4]>,
}

impl SoftwareRasterizer {
    pub fn new(width: u32, height: u32) -> Self {
        Self::with_sample_rate(width, height, 2)
    }

    /// `sample_rate` of 1 disables antialiasing; higher values cost more.
    pub fn with_sample_rate(width: u32, height: u32, sample_rate: u32) -> Self {
        let buffer = vec![0u8; (width * height * 4) as usize];
        Self {
            width,
            height,
            sample_rate: sample_rate.max(1),
            buffer,
            clip_stack: Vec::new(),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Fills the whole canvas with an opaque RGBA color.
    pub fn clear(&mut self, color: [u8; 4]) {
        for pixel in self.buffer.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
    }

    /// Processes a command list in order against the current framebuffer.
    pub fn render(&mut self, commands: &[Command]) {
        for command in commands {
            match *command {
                Command::FillRect {
                    x,
                    y,
                    width,
                    height,
                    color,
                } => self.fill_rect(x, y, width, height, color),
                Command::FillCircle {
                    cx,
                    cy,
                    radius,
                    color,
                } => {
                    self.fill_circle(cx, cy, radius, color);
                }
                Command::FillTriangle { v0, v1, v2, color } => {
                    self.fill_triangle(v0, v1, v2, color);
                }
                Command::StrokeRect {
                    x,
                    y,
                    width,
                    height,
                    thickness,
                    color,
                } => {
                    let half = thickness / 2.0;
                    self.fill_rect(x - half, y - half, width + thickness, thickness, color);
                    self.fill_rect(
                        x - half,
                        y + height - half,
                        width + thickness,
                        thickness,
                        color,
                    );
                    self.fill_rect(x - half, y, thickness, height, color);
                    self.fill_rect(x + width - half, y, thickness, height, color);
                }
                Command::StrokeLine {
                    p0,
                    p1,
                    thickness,
                    color,
                } => {
                    self.stroke_line(p0, p1, thickness, color);
                }
                Command::PushClip {
                    x,
                    y,
                    width,
                    height,
                } => {
                    self.clip_stack.push([x, y, x + width, y + height]);
                }
                Command::PopClip => {
                    self.clip_stack.pop();
                }
            }
        }
    }

    /// Reads back the framebuffer as tightly-packed RGBA8 (row-major).
    pub fn to_rgba8(&self) -> &[u8] {
        &self.buffer
    }

    /// Reads back the framebuffer as `u8` RGBA values for a single pixel.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let idx = ((y * self.width + x) * 4) as usize;
        let px = &self.buffer[idx..idx + 4];
        [px[0], px[1], px[2], px[3]]
    }

    fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        let min = [x, y];
        let max = [x + width, y + height];
        self.rasterize(min, max, color, |px, py| {
            Self::point_in_rect(px, py, min, max)
        });
    }

    fn fill_circle(&mut self, cx: f32, cy: f32, radius: f32, color: [f32; 4]) {
        self.rasterize(
            [cx - radius, cy - radius],
            [cx + radius, cy + radius],
            color,
            |x, y| {
                let dx = x - cx;
                let dy = y - cy;
                dx * dx + dy * dy <= radius * radius
            },
        );
    }

    fn fill_triangle(&mut self, v0: [f32; 2], v1: [f32; 2], v2: [f32; 2], color: [f32; 4]) {
        let min_x = v0[0].min(v1[0]).min(v2[0]);
        let min_y = v0[1].min(v1[1]).min(v2[1]);
        let max_x = v0[0].max(v1[0]).max(v2[0]);
        let max_y = v0[1].max(v1[1]).max(v2[1]);
        self.rasterize([min_x, min_y], [max_x, max_y], color, |x, y| {
            Self::point_in_triangle(x, y, v0, v1, v2)
        });
    }

    fn stroke_line(&mut self, p0: [f32; 2], p1: [f32; 2], thickness: f32, color: [f32; 4]) {
        let radius = thickness / 2.0;
        let min_x = p0[0].min(p1[0]) - radius;
        let min_y = p0[1].min(p1[1]) - radius;
        let max_x = p0[0].max(p1[0]) + radius;
        let max_y = p0[1].max(p1[1]) + radius;
        self.rasterize([min_x, min_y], [max_x, max_y], color, |x, y| {
            Self::distance_to_segment(x, y, p0, p1) <= radius
        });
    }

    /// Rasterizes any shape given by a point-inclusion predicate over the
    /// bounding box `[min, max)`, antialiased by supersampling.
    fn rasterize(
        &mut self,
        min: [f32; 2],
        max: [f32; 2],
        color: [f32; 4],
        inside: impl Fn(f32, f32) -> bool,
    ) {
        let x0 = (min[0].floor().max(0.0)) as u32;
        let y0 = (min[1].floor().max(0.0)) as u32;
        let x1 = (max[0].ceil().min(self.width as f32)) as u32;
        let y1 = (max[1].ceil().min(self.height as f32)) as u32;

        let steps = self.sample_rate;
        let step_size = 1.0 / steps as f32;
        let offset = step_size / 2.0;

        for y in y0..y1 {
            for x in x0..x1 {
                let mut hits = 0u32;
                for sy in 0..steps {
                    for sx in 0..steps {
                        let px = x as f32 + sx as f32 * step_size + offset;
                        let py = y as f32 + sy as f32 * step_size + offset;
                        if self.inside_all_clips(px, py) && inside(px, py) {
                            hits += 1;
                        }
                    }
                }
                let coverage = hits as f32 / (steps * steps) as f32;
                if coverage > 0.0 {
                    self.blend_pixel(x, y, color, coverage);
                }
            }
        }
    }

    fn inside_all_clips(&self, x: f32, y: f32) -> bool {
        self.clip_stack
            .iter()
            .all(|&[x0, y0, x1, y1]| x >= x0 && x < x1 && y >= y0 && y < y1)
    }

    /// Source-over alpha blend of `color` (premultiplied by `coverage`) onto
    /// the existing pixel. Colors are straight (non-premultiplied) RGBA in
    /// 0..=1, so alpha handling is explicit.
    fn blend_pixel(&mut self, x: u32, y: u32, color: [f32; 4], coverage: f32) {
        let idx = ((y * self.width + x) * 4) as usize;
        let dst = &self.buffer[idx..idx + 4];
        let dst_a = dst[3] as f32 / 255.0;
        let src_a = (color[3] * coverage).clamp(0.0, 1.0);

        let out_a = src_a + dst_a * (1.0 - src_a);
        let out = if out_a <= 0.0 {
            [0.0; 3]
        } else {
            [
                (color[0] * src_a + (dst[0] as f32 / 255.0) * dst_a * (1.0 - src_a)) / out_a,
                (color[1] * src_a + (dst[1] as f32 / 255.0) * dst_a * (1.0 - src_a)) / out_a,
                (color[2] * src_a + (dst[2] as f32 / 255.0) * dst_a * (1.0 - src_a)) / out_a,
            ]
        };

        self.buffer[idx] = (out[0].clamp(0.0, 1.0) * 255.0) as u8;
        self.buffer[idx + 1] = (out[1].clamp(0.0, 1.0) * 255.0) as u8;
        self.buffer[idx + 2] = (out[2].clamp(0.0, 1.0) * 255.0) as u8;
        self.buffer[idx + 3] = (out_a.clamp(0.0, 1.0) * 255.0) as u8;
    }

    fn point_in_rect(x: f32, y: f32, min: [f32; 2], max: [f32; 2]) -> bool {
        x >= min[0] && x < max[0] && y >= min[1] && y < max[1]
    }

    fn point_in_triangle(px: f32, py: f32, v0: [f32; 2], v1: [f32; 2], v2: [f32; 2]) -> bool {
        let d1 = Self::edge_sign(px, py, v0, v1);
        let d2 = Self::edge_sign(px, py, v1, v2);
        let d3 = Self::edge_sign(px, py, v2, v0);
        let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
        let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
        !(has_neg && has_pos)
    }

    fn edge_sign(px: f32, py: f32, a: [f32; 2], b: [f32; 2]) -> f32 {
        (px - b[0]) * (a[1] - b[1]) - (a[0] - b[0]) * (py - b[1])
    }

    fn distance_to_segment(px: f32, py: f32, a: [f32; 2], b: [f32; 2]) -> f32 {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len_sq = dx * dx + dy * dy;
        let t = if len_sq <= f32::EPSILON {
            0.0
        } else {
            (((px - a[0]) * dx + (py - a[1]) * dy) / len_sq).clamp(0.0, 1.0)
        };
        let cx = a[0] + t * dx;
        let cy = a[1] + t * dy;
        let ox = px - cx;
        let oy = py - cy;
        (ox * ox + oy * oy).sqrt()
    }
}

impl Default for SoftwareRasterizer {
    fn default() -> Self {
        Self::new(320, 240)
    }
}

/// CPU-only renderer mirroring the `HeadlessRenderer` API. Renders an
/// immediate-mode command list into a software framebuffer and encodes the
/// result without ever touching a GPU adapter.
pub struct SoftwareRenderer {
    rasterizer: SoftwareRasterizer,
    clear_color: [u8; 4],
    output_format: crate::OutputFormat,
}

impl SoftwareRenderer {
    pub fn new(width: u32, height: u32, output_format: crate::OutputFormat) -> Self {
        Self {
            rasterizer: SoftwareRasterizer::new(width, height),
            clear_color: [0, 0, 0, 255],
            output_format,
        }
    }

    pub fn with_clear_color(
        width: u32,
        height: u32,
        output_format: crate::OutputFormat,
        clear_color: [u8; 4],
    ) -> Self {
        Self {
            rasterizer: SoftwareRasterizer::new(width, height),
            clear_color,
            output_format,
        }
    }

    pub fn width(&self) -> u32 {
        self.rasterizer.width()
    }

    pub fn height(&self) -> u32 {
        self.rasterizer.height()
    }

    pub fn output_format(&self) -> crate::OutputFormat {
        self.output_format
    }

    pub fn rasterizer(&self) -> &SoftwareRasterizer {
        &self.rasterizer
    }

    /// Clears the framebuffer, renders `commands`, and encodes the result.
    pub fn render_frame(&mut self, commands: &[Command]) -> Result<Vec<u8>, crate::FallbackError> {
        self.rasterizer.clear(self.clear_color);
        self.rasterizer.render(commands);
        crate::encoding::encode_pixels(
            self.rasterizer.to_rgba8(),
            self.width(),
            self.height(),
            self.output_format,
        )
    }

    pub fn render_frame_to_file(
        &mut self,
        commands: &[Command],
        path: &std::path::Path,
    ) -> Result<(), crate::FallbackError> {
        let data = self.render_frame(commands)?;
        std::fs::write(path, data)
            .map_err(|e| crate::FallbackError::EncodeFailed(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opaque(color: [u8; 3]) -> [f32; 4] {
        [
            color[0] as f32 / 255.0,
            color[1] as f32 / 255.0,
            color[2] as f32 / 255.0,
            1.0,
        ]
    }

    #[test]
    fn clears_to_background_color() {
        let mut r = SoftwareRasterizer::new(8, 8);
        r.clear([10, 20, 30, 255]);
        assert_eq!(r.pixel(0, 0), [10, 20, 30, 255]);
        assert_eq!(r.pixel(7, 7), [10, 20, 30, 255]);
        assert_eq!(r.to_rgba8().len(), 8 * 8 * 4);
    }

    #[test]
    fn fills_a_full_canvas_rect() {
        let mut r = SoftwareRasterizer::new(16, 16);
        r.clear([0, 0, 0, 255]);
        r.render(&[Command::FillRect {
            x: 0.0,
            y: 0.0,
            width: 16.0,
            height: 16.0,
            color: opaque([255, 0, 0]),
        }]);
        assert_eq!(r.pixel(3, 3), [255, 0, 0, 255]);
        assert_eq!(r.pixel(15, 15), [255, 0, 0, 255]);
    }

    #[test]
    fn partially_covered_pixel_is_antialiased() {
        // A 1x1 rect covering exactly half of the left column of the center
        // pixel produces roughly 50% coverage at sample_rate 2 (2 samples hit).
        let mut r = SoftwareRasterizer::with_sample_rate(4, 4, 2);
        r.clear([0, 0, 0, 255]);
        r.render(&[Command::FillRect {
            x: 0.0,
            y: 1.0,
            width: 0.5,
            height: 2.0,
            color: opaque([0, 255, 0]),
        }]);
        let px = r.pixel(0, 2);
        assert_eq!(px[3], 255); // opaque src, coverage blend over opaque black
                                // Coverage 0.5 -> green channel ~128 (+- 32).
        assert!(
            (px[1] as i32 - 128).abs() <= 32,
            "expected ~50% green coverage, got {:?}",
            px
        );
    }

    #[test]
    fn offscreen_geometry_is_clipped_to_framebuffer() {
        let mut r = SoftwareRasterizer::new(8, 8);
        r.clear([0, 0, 0, 255]);
        // Rect mostly outside the canvas; only the visible sliver is drawn.
        r.render(&[Command::FillRect {
            x: -4.0,
            y: 2.0,
            width: 16.0,
            height: 4.0,
            color: opaque([0, 0, 255]),
        }]);
        assert_eq!(r.pixel(0, 3), [0, 0, 255, 255]);
        assert_eq!(r.pixel(7, 3), [0, 0, 255, 255]);
        assert_eq!(r.pixel(0, 0), [0, 0, 0, 255]);
        assert_eq!(r.pixel(0, 7), [0, 0, 0, 255]);
    }

    #[test]
    fn explicit_clip_rect_limits_drawing() {
        let mut r = SoftwareRasterizer::new(8, 8);
        r.clear([0, 0, 0, 255]);
        r.render(&[
            Command::PushClip {
                x: 0.0,
                y: 0.0,
                width: 4.0,
                height: 8.0,
            },
            Command::FillRect {
                x: 0.0,
                y: 0.0,
                width: 8.0,
                height: 8.0,
                color: opaque([255, 255, 0]),
            },
            Command::PopClip,
        ]);
        assert_eq!(r.pixel(2, 2), [255, 255, 0, 255]);
        assert_eq!(r.pixel(5, 2), [0, 0, 0, 255]);
    }

    #[test]
    fn alpha_blends_over_existing_content() {
        let mut r = SoftwareRasterizer::new(4, 4);
        r.clear([255, 0, 0, 255]);
        r.render(&[Command::FillRect {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 4.0,
            color: [0.0, 0.0, 1.0, 0.5],
        }]);
        // 50% blue over opaque red -> (127, 0, 127).
        let px = r.pixel(2, 2);
        assert!((px[0] as i32 - 127).abs() <= 4, "got {:?}", px);
        assert!((px[2] as i32 - 127).abs() <= 4, "got {:?}", px);
        assert_eq!(px[3], 255);
    }

    #[test]
    fn triangle_covers_only_its_interior() {
        let mut r = SoftwareRasterizer::new(4, 4);
        r.clear([0, 0, 0, 255]);
        // Right triangle spanning the whole canvas: vertices (0,0) (4,0) (0,4).
        r.render(&[Command::FillTriangle {
            v0: [0.0, 0.0],
            v1: [4.0, 0.0],
            v2: [0.0, 4.0],
            color: opaque([0, 255, 0]),
        }]);
        assert_eq!(r.pixel(0, 0), [0, 255, 0, 255]);
        assert_eq!(r.pixel(3, 3), [0, 0, 0, 255]);
        // Fully inside the triangle away from edges: (0,2).
        assert_eq!(r.pixel(0, 2), [0, 255, 0, 255]);
        // Fully outside: (3,1) has x + y >= 4 for every sample.
        assert_eq!(r.pixel(3, 1), [0, 0, 0, 255]);
    }

    #[test]
    fn stroke_line_draws_thick_segment() {
        let mut r = SoftwareRasterizer::new(8, 8);
        r.clear([0, 0, 0, 255]);
        r.render(&[Command::StrokeLine {
            p0: [1.0, 4.0],
            p1: [6.0, 4.0],
            thickness: 2.0,
            color: opaque([255, 255, 255]),
        }]);
        assert_eq!(r.pixel(3, 4), [255, 255, 255, 255]);
        assert_eq!(r.pixel(3, 3), [255, 255, 255, 255]);
        assert_eq!(r.pixel(3, 1), [0, 0, 0, 255]);
    }

    #[test]
    fn software_renderer_encodes_png() {
        let mut r = SoftwareRenderer::new(6, 4, crate::OutputFormat::Png);
        let png = r
            .render_frame(&[Command::FillRect {
                x: 0.0,
                y: 0.0,
                width: 6.0,
                height: 4.0,
                color: opaque([0, 128, 255]),
            }])
            .expect("render should succeed");
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));

        let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
            .expect("output should be a valid PNG");
        assert_eq!(decoded.width(), 6);
        assert_eq!(decoded.height(), 4);
        let rgb = decoded.to_rgb8();
        assert_eq!(rgb.get_pixel(3, 2), &image::Rgb([0, 128, 255]));
    }

    #[test]
    fn software_renderer_encodes_raw_rgba() {
        let mut r = SoftwareRenderer::new(2, 2, crate::OutputFormat::RawRgba);
        let raw = r
            .render_frame(&[Command::FillCircle {
                cx: 0.5,
                cy: 0.5,
                radius: 10.0,
                color: opaque([255, 0, 0]),
            }])
            .expect("render should succeed");
        assert_eq!(raw.len(), 2 * 2 * 4);
        assert_eq!(&raw[0..4], &[255, 0, 0, 255]);
    }
}
