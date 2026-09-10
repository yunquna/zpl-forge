use std::{collections::HashMap, sync::Arc};
use zpl_forge::forge::pdf_native::PdfNativeBackend;
use zpl_forge::{FontManager, Resolution, Unit, ZplEngine};

#[test]
fn maxicode_rejects_unsupported_modes_and_sequences() {
    for command in [
        "^BD5",
        "^BD6",
        "^BD4,2,2",
        "^BD4,0,1",
        "^BDoops",
        "^BD4,1,1,2",
    ] {
        assert!(
            ZplEngine::new(
                &format!("^XA^FO20,20{command}^FDHELLO^FS^XZ"),
                Unit::Inches(4.0),
                Unit::Inches(3.0),
                Resolution::Dpi203
            )
            .is_err()
        );
    }
}

/// Opt-in artifact qualification uses a locally provisioned licensed CJK font.
#[test]
#[ignore = "requires provisioned MAXICODE_CJK_FONT and MAXICODE_OUTPUT_DIR"]
fn maxicode_cjk_qualification() {
    let font_path = std::env::var("MAXICODE_CJK_FONT").unwrap();
    let out = std::env::var("MAXICODE_OUTPUT_DIR").unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let mut fonts = FontManager::default();
    fonts
        .register_font("Noto Sans SC", &std::fs::read(font_path).unwrap(), '0', '0')
        .unwrap();
    let fonts = Arc::new(fonts);
    for (mode, data) in [
        (2, "002840336091062[)>_1E01_1D961Z12345678_1DUPSN_1E_04"),
        (3, "001124K1A0B1[)>_1E01_1D961Z12345678_1DUPSN_1E_04"),
        (4, "HELLO MAXICODE 123456789"),
    ] {
        for (dpi, res) in [
            (203, Resolution::Dpi203),
            (300, Resolution::Dpi300),
            (600, Resolution::Dpi600),
        ] {
            let zpl = format!(
                "^XA^CI28^FO30,20^A0N,30,30^FD中文箱标 Shipping Label^FS^FO30,90^BY6^BD{mode}^FH^FD{data}^FS^FO30,740^BY2^BCN,70,N,N,N^FD123456789012^FS^XZ"
            );
            let mut engine =
                ZplEngine::new(&zpl, Unit::Inches(4.0), Unit::Inches(5.0), res).unwrap();
            engine.set_fonts(fonts.clone());
            let pdf = engine
                .render(
                    PdfNativeBackend::new().with_unicode_fonts(),
                    &HashMap::new(),
                )
                .unwrap();
            let doc = lopdf::Document::load_mem(&pdf).unwrap();
            assert!(doc.objects.values().all(|o| {
                o.as_stream()
                    .ok()
                    .and_then(|s| s.dict.get(b"Subtype").ok())
                    .and_then(|v| v.as_name().ok())
                    != Some(b"Image")
            }));
            assert!(doc.extract_text(&[1]).unwrap().contains("中文箱标"));
            std::fs::write(format!("{out}/mode{mode}-{dpi}.pdf"), pdf).unwrap();
            std::fs::write(
                format!("{out}/mode{mode}-{dpi}.png"),
                engine.to_png().unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn maxicode_default_and_by_invariance() {
    let render = |command: &str| {
        let e = ZplEngine::new(
            &format!("^XA^FO30,90{command}^FH^FD002840336091062[)>_1E01_1D96TRACK_1E_04^FS^XZ"),
            Unit::Inches(4.0),
            Unit::Inches(3.0),
            Resolution::Dpi203,
        )
        .unwrap();
        e.to_png().unwrap()
    };
    assert_eq!(render("^BD"), render("^BY6^BD2,1,1"));
    assert_eq!(render("^BD\n"), render("^BD2\n"));
}
