//! Optional TrueType CID font embedding. Subsets are for PDF use only.
use std::collections::BTreeMap;

use ab_glyph::{Font, FontArc};
use lopdf::{Document, FontData, Object, ObjectId, Stream, dictionary};
use subsetter::GlyphRemapper;

use crate::{ZplError, ZplResult};

pub(super) fn embed(
    doc: &mut Document,
    raw: &[u8],
    chars: &BTreeMap<(u16, String), u16>,
) -> ZplResult<ObjectId> {
    // CIDFontType2/FontFile2 requires TrueType outlines. Do not mislabel CFF/OTC.
    if !raw.starts_with(&[0, 1, 0, 0]) && !raw.starts_with(b"true") {
        return Err(ZplError::FontError(
            "Unicode PDF requires a TrueType font".into(),
        ));
    }
    let face = FontArc::try_from_vec(raw.to_vec())
        .map_err(|_| ZplError::FontError("Invalid TrueType font".into()))?;
    let units = face.units_per_em().unwrap_or(1000.0) as f64;
    let scale = |value: f64| (value * 1000.0 / units).round() as i64;
    let mut remapper = GlyphRemapper::new();
    let mut cid_to_gid = vec![0u8; (chars.len() + 1) * 2];
    let mut widths = vec![Object::Integer(0); chars.len()];
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let entries: Vec<_> = chars.iter().collect();
    for chunk in entries.chunks(100) {
        let mapped_count = chunk.iter().filter(|(ch, _)| !ch.1.is_empty()).count();
        if mapped_count > 0 {
            cmap.push_str(&format!("{mapped_count} beginbfchar\n"));
        }
        for &(ch, cid) in chunk {
            let gid = ch.0;
            let mapped = remapper.remap(gid);
            let offset = usize::from(*cid) * 2;
            cid_to_gid[offset..offset + 2].copy_from_slice(&mapped.to_be_bytes());
            widths[usize::from(*cid) - 1] =
                scale(face.h_advance_unscaled(ab_glyph::GlyphId(gid)) as f64).into();
            let unicode =
                ch.1.encode_utf16()
                    .map(|unit| format!("{unit:04X}"))
                    .collect::<String>();
            if !unicode.is_empty() {
                cmap.push_str(&format!("<{cid:04X}> <{unicode}>\n"));
            }
        }
        if mapped_count > 0 {
            cmap.push_str("endbfchar\n");
        }
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    let subset = subsetter::subset(raw, 0, &remapper)
        .map_err(|_| ZplError::FontError("TrueType subsetting failed".into()))?;
    // Distinct subset names within a document, without random IDs or timestamps.
    let mut ordinal = doc.max_id;
    let mut prefix = [b'A'; 6];
    for letter in &mut prefix {
        *letter += (ordinal % 26) as u8;
        ordinal /= 26;
    }
    let name = format!("{}+ZplSubset", String::from_utf8_lossy(&prefix));
    let metrics = FontData::new(raw, name.clone());
    let file = doc.add_object(Stream::new(
        dictionary! { "Length1" => subset.len() as i64 },
        subset,
    ));
    let descriptor = doc.add_object(dictionary! {
        "Type" => "FontDescriptor", "FontName" => Object::Name(name.clone().into_bytes()),
        "Flags" => 4_i64,
        "FontBBox" => vec![scale(metrics.font_bbox.0 as f64).into(), scale(metrics.font_bbox.1 as f64).into(), scale(metrics.font_bbox.2 as f64).into(), scale(metrics.font_bbox.3 as f64).into()],
        "ItalicAngle" => metrics.italic_angle, "Ascent" => scale(metrics.ascent as f64),
        "Descent" => scale(metrics.descent as f64), "CapHeight" => scale(metrics.cap_height as f64),
        "StemV" => 80_i64, "FontFile2" => file,
    });
    let mapping = doc.add_object(Stream::new(dictionary! {}, cid_to_gid));
    let unicode = doc.add_object(Stream::new(dictionary! {}, cmap.into_bytes()));
    let descendant = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "CIDFontType2",
        "BaseFont" => Object::Name(name.clone().into_bytes()),
        "CIDSystemInfo" => dictionary! { "Registry" => Object::string_literal("Adobe"), "Ordering" => Object::string_literal("Identity"), "Supplement" => 0_i64 },
        "FontDescriptor" => descriptor, "CIDToGIDMap" => mapping,
        "W" => vec![Object::Integer(1), Object::Array(widths)],
    });
    Ok(doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type0", "BaseFont" => Object::Name(name.into_bytes()),
        "Encoding" => "Identity-H", "DescendantFonts" => vec![Object::Reference(descendant)],
        "ToUnicode" => unicode,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::pdf_native::PdfNativeBackend;
    use crate::{Resolution, Unit, ZplEngine};
    use std::collections::HashMap;

    #[test]
    fn subset_preserves_text_across_pages() {
        let engine = ZplEngine::new(
            "^XA^FO20,20^AAN,30,30^FD{{text}}^FS^XZ",
            Unit::Dots(816),
            Unit::Dots(1216),
            Resolution::Dpi203,
        )
        .unwrap();
        let pages = [
            HashMap::from([("text".into(), "Subset 1".into())]),
            HashMap::from([("text".into(), "Page 2".into())]),
        ];
        let bytes = engine
            .render_pages(PdfNativeBackend::new().with_unicode_fonts(), &pages)
            .unwrap();
        let doc = Document::load_mem(&bytes).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
        assert!(doc.extract_text(&[1]).unwrap().contains("Subset 1"));
        assert!(doc.extract_text(&[2]).unwrap().contains("Page 2"));
        assert!(bytes.len() < 16 * 1024, "font should be subsetted");
    }

    #[test]
    fn missing_unicode_glyph_fails_instead_of_substituting() {
        let engine = ZplEngine::new(
            "^XA^FO20,20^AAN,30,30^FD\u{10ffff}^FS^XZ",
            Unit::Dots(816),
            Unit::Dots(1216),
            Resolution::Dpi203,
        )
        .unwrap();
        assert!(matches!(
            engine.render(
                PdfNativeBackend::new().with_unicode_fonts(),
                &HashMap::new()
            ),
            Err(ZplError::FontError(_))
        ));
    }
}
