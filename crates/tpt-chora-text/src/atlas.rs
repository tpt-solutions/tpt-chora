use std::collections::HashMap;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};

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

#[derive(Debug, Clone)]
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

        let padding = self.padding;
        let spread = self.spread;
        let atlas_w = self.atlas_width;
        let atlas_h = self.atlas_height;

        let mut atlas_data = vec![0u8; (atlas_w * atlas_h) as usize];
        let mut glyphs = HashMap::new();
        let mut cursor_x: u32 = 0;
        let mut cursor_y: u32 = 0;
        let mut row_height: u32 = 0;

        for glyph_id in 0u16..=255u16 {
            let glyph_id_ab = ab_glyph::GlyphId(glyph_id);
            let advance = scale_font.h_advance(glyph_id_ab);
            if advance <= 0.0 {
                continue;
            }

            let glyph = glyph_id_ab.with_scale_and_position(PxScale::from(self.font_size), ab_glyph::point(0.0, 0.0));

            let outline = font.outline_glyph(glyph);
            let (pixel_width, pixel_height, rasterized_bitmap) = if let Some(ref outline) = outline {
                let bounds = outline.px_bounds();
                let w = (bounds.width().ceil() as u32).max(1);
                let h = (bounds.height().ceil() as u32).max(1);

                let mut bitmap = vec![0.0f32; (w * h) as usize];
                outline.draw(|x, y, coverage| {
                    let idx = (y * w + x) as usize;
                    if idx < bitmap.len() {
                        bitmap[idx] = coverage;
                    }
                });
                (w, h, bitmap)
            } else {
                let w = (advance.ceil() as u32).max(1);
                let h = (self.font_size.ceil() as u32).max(1);
                (w, h, vec![0.0f32; (w * h) as usize])
            };

            let glyph_w = pixel_width + padding * 2;
            let glyph_h = pixel_height + padding * 2;

            if cursor_x + glyph_w > atlas_w {
                cursor_x = 0;
                cursor_y += row_height;
                row_height = 0;
            }

            if cursor_y + glyph_h > atlas_h {
                continue;
            }

            let sdf_data = compute_sdf(&rasterized_bitmap, pixel_width, pixel_height, spread as i32);

            for sy in 0..pixel_height {
                for sx in 0..pixel_width {
                    let atlas_x = cursor_x + padding + sx;
                    let atlas_y = cursor_y + padding + sy;
                    let sdf_val = sdf_data[(sy * pixel_width + sx) as usize];
                    let idx = (atlas_y * atlas_w + atlas_x) as usize;
                    if idx < atlas_data.len() {
                        atlas_data[idx] = (sdf_val * 255.0).clamp(0.0, 255.0) as u8;
                    }
                }
            }

            let glyph_id_u16 = glyph_id;
            glyphs.insert(
                glyph_id_u16,
                GlyphInfo {
                    glyph_id: glyph_id_u16,
                    x: cursor_x + padding,
                    y: cursor_y + padding,
                    width: pixel_width,
                    height: pixel_height,
                    x_offset: 0.0,
                    y_offset: -(pixel_height as f32),
                    x_advance: advance,
                    y_advance: 0.0,
                },
            );

            cursor_x += glyph_w;
            row_height = row_height.max(glyph_h);
        }

        Ok(FontAtlas {
            texture_width: atlas_w,
            texture_height: atlas_h,
            pixel_data: atlas_data,
            glyphs,
            font_size: self.font_size,
        })
    }
}

fn compute_sdf(bitmap: &[f32], width: u32, height: u32, spread: i32) -> Vec<f32> {
    let w = width as i32;
    let h = height as i32;
    let mut sdf = vec![0.5f32; (width * height) as usize];

    let outer_sdf = compute_distance_field(bitmap, width, height, false);
    let inner_sdf = compute_distance_field(bitmap, width, height, true);

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let dist = outer_sdf[idx] - inner_sdf[idx];
            let normalized = 0.5 + dist / (2.0 * spread as f32);
            sdf[idx] = normalized.clamp(0.0, 1.0);
        }
    }

    sdf
}

fn compute_distance_field(bitmap: &[f32], width: u32, height: u32, invert: bool) -> Vec<f32> {
    let w = width as i32;
    let h = height as i32;
    let mut dist = vec![0.0f32; (width * height) as usize];
    let big = 1e10f32;

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let inside = if invert {
                1.0 - bitmap[idx]
            } else {
                bitmap[idx]
            };
            dist[idx] = if inside > 0.5 { 0.0 } else { big };
        }
    }

    for y in 0..h {
        for x in 1..w {
            let idx = (y * w + x) as usize;
            let prev = (y * w + (x - 1)) as usize;
            let d = dist[prev] + 1.0;
            if d < dist[idx] {
                dist[idx] = d;
            }
        }
        for x in (0..w - 1).rev() {
            let idx = (y * w + x) as usize;
            let next = (y * w + (x + 1)) as usize;
            let d = dist[next] + 1.0;
            if d < dist[idx] {
                dist[idx] = d;
            }
        }
    }

    for x in 0..w {
        for y in 1..h {
            let idx = (y * w + x) as usize;
            let prev = ((y - 1) * w + x) as usize;
            let d = dist[prev] + 1.0;
            if d < dist[idx] {
                dist[idx] = d;
            }
        }
        for y in (0..h - 1).rev() {
            let idx = (y * w + x) as usize;
            let next = ((y + 1) * w + x) as usize;
            let d = dist[next] + 1.0;
            if d < dist[idx] {
                dist[idx] = d;
            }
        }
    }

    dist
}
