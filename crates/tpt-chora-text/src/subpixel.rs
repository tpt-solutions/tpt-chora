use crate::atlas::FontAtlas;

pub struct SubPixelConfig {
    pub enabled: bool,
    pub distribution: [f32; 3],
    pub gamma: f32,
}

impl Default for SubPixelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            distribution: [0.2, 0.6, 0.2],
            gamma: 2.2,
        }
    }
}

pub fn build_subpixel_vertices(
    atlas: &FontAtlas,
    glyph_id: u16,
    x: f32,
    y: f32,
    color: [f32; 4],
    config: &SubPixelConfig,
) -> Option<(Vec<crate::sdf::TextVertex>, Vec<u32>)> {
    if !config.enabled {
        return build_standard_vertices(atlas, glyph_id, x, y, color);
    }
    let info = atlas.glyphs.get(&glyph_id)?;

    let atlas_w = atlas.texture_width as f32;
    let atlas_h = atlas.texture_height as f32;

    let u0 = info.x as f32 / atlas_w;
    let v0 = info.y as f32 / atlas_h;
    let u1 = (info.x + info.width) as f32 / atlas_w;
    let v1 = (info.y + info.height) as f32 / atlas_h;

    let x0 = x + info.x_offset;
    let y0 = y + info.y_offset;
    let x1 = x0 + info.width as f32;
    let y1 = y0 + info.height as f32;

    let gamma_corrected_color = [
        color[0].powf(1.0 / config.gamma),
        color[1].powf(1.0 / config.gamma),
        color[2].powf(1.0 / config.gamma),
        color[3],
    ];

    let vertices = vec![
        crate::sdf::TextVertex {
            position: [x0, y0],
            tex_coord: [u0, v0],
            color: gamma_corrected_color,
        },
        crate::sdf::TextVertex {
            position: [x1, y0],
            tex_coord: [u1, v0],
            color: gamma_corrected_color,
        },
        crate::sdf::TextVertex {
            position: [x1, y1],
            tex_coord: [u1, v1],
            color: gamma_corrected_color,
        },
        crate::sdf::TextVertex {
            position: [x0, y1],
            tex_coord: [u0, v1],
            color: gamma_corrected_color,
        },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    Some((vertices, indices))
}

fn build_standard_vertices(
    atlas: &FontAtlas,
    glyph_id: u16,
    x: f32,
    y: f32,
    color: [f32; 4],
) -> Option<(Vec<crate::sdf::TextVertex>, Vec<u32>)> {
    let info = atlas.glyphs.get(&glyph_id)?;

    let atlas_w = atlas.texture_width as f32;
    let atlas_h = atlas.texture_height as f32;

    let u0 = info.x as f32 / atlas_w;
    let v0 = info.y as f32 / atlas_h;
    let u1 = (info.x + info.width) as f32 / atlas_w;
    let v1 = (info.y + info.height) as f32 / atlas_h;

    let x0 = x + info.x_offset;
    let y0 = y + info.y_offset;
    let x1 = x0 + info.width as f32;
    let y1 = y0 + info.height as f32;

    let vertices = vec![
        crate::sdf::TextVertex {
            position: [x0, y0],
            tex_coord: [u0, v0],
            color,
        },
        crate::sdf::TextVertex {
            position: [x1, y0],
            tex_coord: [u1, v0],
            color,
        },
        crate::sdf::TextVertex {
            position: [x1, y1],
            tex_coord: [u1, v1],
            color,
        },
        crate::sdf::TextVertex {
            position: [x0, y1],
            tex_coord: [u0, v1],
            color,
        },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];
    Some((vertices, indices))
}

pub fn build_glyph_vertices(
    atlas: &FontAtlas,
    shaped_glyphs: &[crate::shaping::ShapedGlyph],
    start_x: f32,
    start_y: f32,
    color: [f32; 4],
    config: &SubPixelConfig,
) -> (Vec<crate::sdf::TextVertex>, Vec<u32>) {
    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();

    for glyph in shaped_glyphs.iter() {
        let info = atlas.glyphs.get(&(glyph.glyph_id));
        if let Some(_info) = info {
            if let Some((verts, indices)) = build_subpixel_vertices(
                atlas,
                glyph.glyph_id,
                start_x + glyph.x_offset,
                start_y + glyph.y_offset,
                color,
                config,
            ) {
                let base = all_vertices.len() as u32;
                all_vertices.extend(verts);
                all_indices.extend(indices.iter().map(|&idx| idx + base));
            }
        }
    }

    (all_vertices, all_indices)
}
