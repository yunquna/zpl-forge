//! Bounded horizontal CJK/Latin shaping shared by PDF positioning and measurement.
use crate::{ZplError, ZplResult};
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

pub(crate) struct Glyph {
    pub id: u16,
    pub source: String,
    pub x: f64,
    pub y: f64,
}
pub(crate) struct Run {
    pub glyphs: Vec<Glyph>,
    pub advance: f64,
    pub units: f64,
}
fn unsupported() -> ZplError {
    ZplError::FontError("Unsupported shaped text or font".into())
}

pub(crate) fn shape(raw: &[u8], text: &str) -> ZplResult<Run> {
    if text.chars().count() > 4096 {
        return Err(ZplError::SecurityLimitExceeded(
            "Shaped field character limit".into(),
        ));
    }
    let face = rustybuzz::Face::from_slice(raw, 0).ok_or_else(unsupported)?;
    if face.is_variable() || face.tables().glyf.is_none() {
        return Err(unsupported());
    }
    let mut previous = None;
    for ch in text.chars() {
        if !matches!(
            ch.script(),
            Script::Latin
                | Script::Han
                | Script::Hiragana
                | Script::Katakana
                | Script::Hangul
                | Script::Common
                | Script::Inherited
        ) || ch.is_control()
        {
            return Err(unsupported());
        }
        if matches!(ch as u32, 0xFE00..=0xFE0F | 0xE0100..=0xE01EF) {
            if previous
                .and_then(|base| face.glyph_variation_index(base, ch))
                .is_none()
            {
                return Err(unsupported());
            }
        } else if matches!(ch as u32, 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0xFEFF)
        {
            return Err(unsupported());
        }
        previous = Some(ch);
    }
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(rustybuzz::Direction::LeftToRight);
    buffer.set_language("zh-Hans".parse().unwrap());
    buffer.guess_segment_properties();
    let output = rustybuzz::shape(&face, &[], buffer);
    let infos = output.glyph_infos();
    let mut glyphs = Vec::with_capacity(infos.len());
    let mut cursor = 0.0;
    for (index, (info, pos)) in infos.iter().zip(output.glyph_positions()).enumerate() {
        if info.glyph_id == 0 {
            return Err(unsupported());
        }
        let cluster = info.cluster as usize;
        let first = index == 0 || infos[index - 1].cluster != info.cluster;
        let source = if first {
            let end = infos
                .iter()
                .skip(index + 1)
                .find(|i| i.cluster != info.cluster)
                .map_or(text.len(), |i| i.cluster as usize);
            text.get(cluster..end).ok_or_else(unsupported)?.to_string()
        } else {
            String::new()
        };
        glyphs.push(Glyph {
            id: info.glyph_id as u16,
            source,
            x: cursor + f64::from(pos.x_offset),
            y: f64::from(pos.y_offset),
        });
        cursor += f64::from(pos.x_advance);
    }
    Ok(Run {
        glyphs,
        advance: cursor,
        units: f64::from(face.units_per_em()),
    })
}

/// Unicode line opportunities; emergency breaks only at complete graphemes.
pub(crate) fn wrap(text: &str, width: u32, measure: impl Fn(&str) -> u32) -> Vec<String> {
    let mut lines = Vec::new();
    for segment in text.split("\\&") {
        if width == 0 {
            lines.push(segment.to_string());
            continue;
        }
        let opportunities: std::collections::HashSet<_> = unicode_linebreak::linebreaks(segment)
            .map(|(i, _)| i)
            .collect();
        let mut start = 0;
        let mut last_break = None;
        for (offset, grapheme) in segment.grapheme_indices(true) {
            let end = offset + grapheme.len();
            if offset > start && measure(&segment[start..end]) > width {
                let split = last_break.filter(|&i| i > start).unwrap_or(offset);
                lines.push(segment[start..split].trim_end().to_string());
                start = split;
                last_break = None;
                // The tail after a word boundary may itself exceed the width.
                if offset > start && measure(&segment[start..end]) > width {
                    lines.push(segment[start..offset].trim_end().to_string());
                    start = offset;
                }
            }
            if opportunities.contains(&end) {
                last_break = Some(end);
            }
        }
        lines.push(segment[start..].trim_end().to_string());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emergency_wrap_keeps_combining_clusters() {
        let lines = wrap("e\u{301}e\u{301}e\u{301}", 1, |s| {
            s.graphemes(true).count() as u32
        });
        assert_eq!(lines, ["e\u{301}", "e\u{301}", "e\u{301}"]);
    }
    #[test]
    fn cjk_breaks_and_explicit_lines_preserve_text() {
        let lines = wrap("中文标签\\&上海", 2, |s| {
            s.graphemes(true).count() as u32
        });
        assert_eq!(lines, ["中文", "标签", "上海"]);
    }
}
