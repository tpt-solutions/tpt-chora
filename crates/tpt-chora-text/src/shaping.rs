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
    let face = rustybuzz::Face::from_slice(font_data, 0)
        .ok_or_else(|| crate::error::TextError::FontParse("failed to parse font".into()))?;

    let scale_factor = font_size / face.units_per_em() as f32;

    let ordered_text = match direction {
        TextDirection::Ltr => text.to_string(),
        TextDirection::Rtl => reorder_rtl(text),
        TextDirection::Bidi => reorder_bidi(text),
    };

    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(&ordered_text);

    match direction {
        TextDirection::Ltr => buffer.set_direction(rustybuzz::Direction::LeftToRight),
        TextDirection::Rtl | TextDirection::Bidi => buffer.set_direction(rustybuzz::Direction::RightToLeft),
    }

    let shaped = rustybuzz::shape(&face, &[], buffer);

    let glyph_infos = shaped.glyph_infos();
    let glyph_positions = shaped.glyph_positions();

    let mut all_glyphs = Vec::new();
    let mut x_pos: f32 = 0.0;

    for (info, pos) in glyph_infos.iter().zip(glyph_positions.iter()) {
        let cluster_byte = info.cluster as usize;
        let codepoint = ordered_text
            .char_indices()
            .find(|&(byte_idx, _)| byte_idx == cluster_byte)
            .map(|(_, ch)| ch as u32)
            .unwrap_or(0);

        let glyph = ShapedGlyph {
            codepoint,
            glyph_id: info.glyph_id as u16,
            cluster_index: cluster_byte,
            x_advance: pos.x_advance as f32 * scale_factor,
            y_advance: pos.y_advance as f32 * scale_factor,
            x_offset: x_pos + pos.x_offset as f32 * scale_factor,
            y_offset: pos.y_offset as f32 * scale_factor,
        };
        all_glyphs.push(glyph);
        x_pos += pos.x_advance as f32 * scale_factor;
    }

    if direction == TextDirection::Rtl || direction == TextDirection::Bidi {
        normalize_rtl_positions(&mut all_glyphs, x_pos);
    }

    Ok(all_glyphs)
}

fn reorder_rtl(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    chars.reverse();
    chars.into_iter().collect()
}

fn reorder_bidi(text: &str) -> String {
    let bidi_info = unicode_bidi::BidiInfo::new(text, None);
    if bidi_info.paragraphs.is_empty() {
        return text.to_string();
    }

    let para = &bidi_info.paragraphs[0];
    let line = para.range.clone();
    let display = bidi_info.reorder_line(para, line);
    let result = display.into_owned();

    if result.is_empty() {
        text.to_string()
    } else {
        result
    }
}

fn normalize_rtl_positions(glyphs: &mut [ShapedGlyph], total_width: f32) {
    if glyphs.is_empty() {
        return;
    }

    let mut cursor = total_width;
    for glyph in glyphs.iter_mut() {
        cursor -= glyph.x_advance;
        glyph.x_offset = cursor;
    }
}
