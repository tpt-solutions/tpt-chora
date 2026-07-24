use ab_glyph::{FontRef, PxScale};
use rustybuzz::shape;

pub struct ShapedGlyph {
    pub codepoint: u32,
    pub glyph_id: u16,
    pub cluster_index: usize,
    pub x_advance: f32,
    pub y_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    Ltr,
    Rtl,
    Bidi,
}

pub fn shaped_text(
    font_data: &[u8],
    text: &str,
    direction: TextDirection,
    font_size: f32,
) -> Result<Vec<ShapedGlyph>, crate::error::TextError> {
    let font = FontRef::try_from_slice(font_data)
        .map_err(|e| crate::error::TextError::FontParse(e.to_string()))?;

    let bidi_info = unicode_bidi::BidiInfo::new(text, None);
    let paragraph = bidi_info
        .paragraphs
        .first()
        .ok_or_else(|| crate::error::TextError::ShapingFailed("no paragraph found".into()))?;

    let (levels, runs) = bidi_info.reorder_consecutive(&paragraph.range);

    let mut all_glyphs = Vec::new();
    let scale = PxScale::from(font_size);

    for run in runs {
        let run_text = &text[run.clone()];
        let face = rustybuzz::Face::from_face(|_, index| {
            let font_ref = FontRef::try_from_slice(font_data).unwrap();
            rustybuzz::Face::from_index(font_ref, index)
        });

        let mut shape_input = rustybuzz::ShapeBuffer::new();
        let is_rtl = levels
            .get(run.start)
            .map_or(false, |l| l.is_rtl());

        shape_input.push(run_text, None);
        let shaped = shape(&face, shape_input, None);

        let glyph_infos = shaped.glyph_infos();
        let glyph_positions = shaped.glyph_positions();

        let mut x_pos: f32 = 0.0;
        for (info, pos) in glyph_infos.iter().zip(glyph_positions.iter()) {
            let glyph = ShapedGlyph {
                codepoint: run_text
                    .chars()
                    .nth(info.cluster as usize)
                    .map_or(0, |c| c as u32),
                glyph_id: info.glyph_id,
                cluster_index: info.cluster as usize,
                x_advance: pos.x_advance as f32,
                y_advance: pos.y_advance as f32,
                x_offset: x_pos + pos.x_offset as f32,
                y_offset: pos.y_offset as f32,
            };
            all_glyphs.push(glyph);
            x_pos += pos.x_advance as f32;
        }
    }

    Ok(all_glyphs)
}
