use std::collections::HashMap;

use ab_glyph::{FontRef, Glyph, GlyphId, PxScaleFont, ScaleFont};

#[derive(Debug, Clone)]
pub struct GlyphInfo {
    pub glyph_id: u16,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub x_advance: f32,
    pub y_advance: f32,
}

pub struct FontAtlas {
    pub texture_width: u32,
    pub texture_height: u32,
    pub pixel_data: Vec<u8>,
    pub glyphs: HashMap<u16, GlyphInfo>,
    pub font_size: f32,
}

pub struct SdfAtlasBuilder {
    font_data: Vec<u8>,
    font_size: f32,
    atlas_width: u32,
    atlas_height: u32,
    padding: u32,
    spread: f32,
}

impl SdfAtlasBuilder {
    pub fn new(font_data: Vec<u8>, font_size: f32) -> Self {
        Self {
            font_data,
            font_size,
            atlas_width: 1024,
            atlas_height: 1024,
            padding: 2,
            spread: 8.0,
        }
    }

    pub fn with_atlas_size(mut self, width: u32, height: u32) -> Self {
        self.atlas_width = width;
        self.atlas_height = height;
        self
    }

    pub fn with_padding(mut self, padding: u32) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_spread(mut self, spread: f32) -> Self {
        self.spread = spread;
        self
    }

    pub fn build(&self) -> Result<FontAtlas, crate::error::TextError> {
        let font = FontRef::try_from_slice(&self.font_data)
            .map_err(|e| crate::error::TextError::FontParse(e.to_string()))?;

        let scale_font = font.as_scaled(PxScale::from(self.font_size));
        let metrics = scale_font.metrics();

        let mut atlas_data = vec![0u8; (self.atlas_width * self.atlas_height * 4) as usize];
        let mut glyphs = HashMap::new();
        let mut cursor_x: u32 = 0;
        let mut cursor_y: u32 = 0;
        let mut row_height: u32 = 0;

        for glyph_id in 0u16..=255u16 {
            let glyph = Glyph::with_id(glyph_id);
            let outlined = scale_font.outline_glyph(glyph);

            if let Some(outline) = outlined {
                let bounds = outline.px_bounds();
                let glyph_width = (bounds.width() as u32 + self.padding * 2).max(1);
                let glyph_height = (bounds.height() as u32 + self.padding * 2).max(1);

                if cursor_x + glyph_width > self.atlas_width {
                    cursor_x = 0;
                    cursor_y += row_height;
                    row_height = 0;
                }

                if cursor_y + glyph_height > self.atlas_height {
                    continue;
                }

                let sdf_data =
                    generate_sdf(&outline, self.spread, glyph_width, self.padding);

                for y in 0..glyph_height {
                    for x in 0..glyph_width {
                        let atlas_idx =
                            ((cursor_y + y) * self.atlas_width + cursor_x + x) as usize * 4;
                        let sdf_idx = (y * glyph_width + x) as usize;
                        let val = sdf_data[sdf_idx];
                        atlas_data[atlas_idx] = val;
                        atlas_data[atlas_idx + 1] = val;
                        atlas_data[atlas_idx + 2] = val;
                        atlas_data[atlas_idx + 3] = 255;
                    }
                }

                glyphs.insert(
                    glyph_id,
                    GlyphInfo {
                        glyph_id,
                        x: cursor_x,
                        y: cursor_y,
                        width: glyph_width - self.padding * 2,
                        height: glyph_height - self.padding * 2,
                        x_offset: bounds.min.x as f32 - self.padding as f32,
                        y_offset: bounds.min.y as f32 - self.padding as f32,
                        x_advance: scale_font.h_advance(glyph),
                        y_advance: scale_font.v_advance(glyph),
                    },
                );

                cursor_x += glyph_width;
                row_height = row_height.max(glyph_height);
            }
        }

        Ok(FontAtlas {
            texture_width: self.atlas_width,
            texture_height: self.atlas_height,
            pixel_data: atlas_data,
            glyphs,
            font_size: self.font_size,
        })
    }
}

fn generate_sdf(
    outline: &ab_glyph::Outline,
    spread: f32,
    width: u32,
    padding: u32,
) -> Vec<u8> {
    let bounds = outline.px_bounds();
    let sdf_width = width;
    let sdf_height = (bounds.height() as u32 + padding * 2).max(1);
    let mut sdf = vec![128u8; (sdf_width * sdf_height) as usize];

    for y in 0..sdf_height {
        for x in 0..sdf_width {
            let px = bounds.min.x + (x as f32 - padding as f32);
            let py = bounds.min.y + (y as f32 - padding as f32);

            let mut min_dist = f32::MAX;
            for curve in &outline.curves {
                let dist = match curve {
                    ab_glyph::Curve::Line(line) => point_to_segment_dist(px, py, line),
                    ab_glyph::Curve::Quad(quad) => {
                        point_to_quad_dist(px, py, quad[0], quad[1], quad[2])
                    }
                };
                min_dist = min_dist.min(dist);
            }

            let inside = is_point_inside(px, py, &outline.curves);
            let signed_dist = if inside { -min_dist } else { min_dist };
            let normalized = ((signed_dist / spread) * 0.5 + 0.5).clamp(0.0, 1.0);
            sdf[(y * sdf_width + x) as usize] = (normalized * 255.0) as u8;
        }
    }

    sdf
}

fn point_to_segment_dist(px: f32, py: f32, end: [f32; 2]) -> f32 {
    let dx = end[0] - px;
    let dy = end[1] - py;
    (dx * dx + dy * dy).sqrt()
}

fn point_to_quad_dist(
    px: f32,
    py: f32,
    ctrl: [f32; 2],
    ctrl2: [f32; 2],
    end: [f32; 2],
) -> f32 {
    let mut min_dist = f32::MAX;
    let steps = 16;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let u = 1.0 - t;
        let x = u * u * u * px + 3.0 * u * u * t * ctrl[0] + 3.0 * u * t * t * ctrl2[0] + t * t * t * end[0];
        let y = u * u * u * py + 3.0 * u * u * t * ctrl[1] + 3.0 * u * t * t * ctrl2[1] + t * t * t * end[1];
        let dx = x - px;
        let dy = y - py;
        min_dist = min_dist.min((dx * dx + dy * dy).sqrt());
    }
    min_dist
}

fn is_point_inside(px: f32, py: f32, curves: &[ab_glyph::Curve]) -> bool {
    let mut inside = false;
    let mut last_x = 0.0f32;
    let mut last_y = 0.0f32;

    for curve in curves {
        let end = match curve {
            ab_glyph::Curve::Line(line) => line,
            ab_glyph::Curve::Quad(quad) => quad[2],
        };

        if (last_y > py) != (end[1] > py) {
            let x_intersect =
                last_x + (py - last_y) / (end[1] - last_y) * (end[0] - last_x);
            if px < x_intersect {
                inside = !inside;
            }
        }

        last_x = end[0];
        last_y = end[1];
    }

    inside
}
