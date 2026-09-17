use std::sync::Arc;
use wasm_bindgen::prelude::*;
use zpl_forge::{FontManager, Resolution, Unit, ZplEngine};

/// Owns reusable fonts. Call free() when the JS owner no longer needs it.
/// All input/output stays in memory; the library performs no network or file I/O.
#[wasm_bindgen]
pub struct ZplRenderer {
    fonts: Arc<FontManager>,
}

#[wasm_bindgen]
impl ZplRenderer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            fonts: Arc::new(FontManager::default()),
        }
    }

    /// Register one caller-provided TrueType font for ZPL font identifier 0.
    /// Registration replaces the prior custom font, bounding retained memory.
    #[wasm_bindgen(js_name = setDefaultFont)]
    pub fn set_default_font(&mut self, bytes: &[u8]) -> Result<(), JsError> {
        if bytes.is_empty() || bytes.len() > 16 * 1024 * 1024 {
            return Err(JsError::new("FONT_SIZE_LIMIT"));
        }
        if !bytes.starts_with(&[0, 1, 0, 0]) && !bytes.starts_with(b"true") {
            return Err(JsError::new("FONT_INVALID"));
        }
        let mut fonts = FontManager::default();
        fonts
            .register_font("ExternalDefault", bytes, '0', '0')
            .map_err(|_| JsError::new("FONT_INVALID"))?;
        self.fonts = Arc::new(fonts);
        Ok(())
    }

    #[wasm_bindgen(js_name = renderPdf)]
    pub fn render_pdf(
        &self,
        zpl: &str,
        width: u32,
        height: u32,
        dpi: u32,
    ) -> Result<Vec<u8>, JsError> {
        let engine = self.engine(zpl, width, height, dpi)?;
        let output = engine
            .render(
                zpl_forge::forge::pdf_native::PdfNativeBackend::new().with_unicode_fonts(),
                &std::collections::HashMap::new(),
            )
            .map_err(map_render_error)?;
        check_output(output)
    }

    /// Opt-in CJK/Latin shaped PDF. The existing renderPdf and renderPng remain unchanged.
    #[wasm_bindgen(js_name = renderShapedPdf)]
    pub fn render_shaped_pdf(
        &self,
        zpl: &str,
        width: u32,
        height: u32,
        dpi: u32,
    ) -> Result<Vec<u8>, JsError> {
        let mut engine = self.engine(zpl, width, height, dpi)?;
        engine.set_fonts(Arc::new((*self.fonts).clone().with_pdf_shaping()));
        check_output(
            engine
                .render(
                    zpl_forge::forge::pdf_native::PdfNativeBackend::new().with_unicode_fonts(),
                    &std::collections::HashMap::new(),
                )
                .map_err(map_render_error)?,
        )
    }

    /// Render one template with a JSON array of string-variable maps, one map per page.
    #[wasm_bindgen(js_name = renderPdfPages)]
    pub fn render_pdf_pages(
        &self,
        zpl: &str,
        width: u32,
        height: u32,
        dpi: u32,
        pages_json: &str,
    ) -> Result<Vec<u8>, JsError> {
        if pages_json.len() > 64 * 1024 {
            return Err(JsError::new("ZPL_VARIABLES_LIMIT"));
        }
        let pages: Vec<std::collections::HashMap<String, String>> =
            serde_json::from_str(pages_json).map_err(|_| JsError::new("ZPL_VARIABLES_INVALID"))?;
        if pages.is_empty() || pages.len() > 32 {
            return Err(JsError::new("ZPL_PAGE_LIMIT"));
        }
        let engine = self.engine(zpl, width, height, dpi)?;
        check_output(
            engine
                .render_pages(
                    zpl_forge::forge::pdf_native::PdfNativeBackend::new().with_unicode_fonts(),
                    &pages,
                )
                .map_err(map_render_error)?,
        )
    }

    #[wasm_bindgen(js_name = renderPng)]
    pub fn render_png(
        &self,
        zpl: &str,
        width: u32,
        height: u32,
        dpi: u32,
    ) -> Result<Vec<u8>, JsError> {
        let engine = self.engine(zpl, width, height, dpi)?;
        let output = engine.to_png().map_err(map_render_error)?;
        check_output(output)
    }

    #[wasm_bindgen(js_name = renderBarcodeSvg)]
    pub fn render_barcode_svg(
        &self,
        kind: &str,
        data: &str,
        module_size: u32,
        quiet_zone: u32,
    ) -> Result<String, JsError> {
        zpl_forge::standalone_barcode::render_svg(kind, data, module_size, quiet_zone)
            .map_err(|_| JsError::new("BARCODE_RENDER_FAILED"))
    }

    #[wasm_bindgen(js_name = renderBarcodePng)]
    pub fn render_barcode_png(
        &self,
        kind: &str,
        data: &str,
        module_size: u32,
        quiet_zone: u32,
    ) -> Result<Vec<u8>, JsError> {
        check_output(
            zpl_forge::standalone_barcode::render_png(kind, data, module_size, quiet_zone)
                .map_err(|_| JsError::new("BARCODE_RENDER_FAILED"))?,
        )
    }
}

impl Default for ZplRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl ZplRenderer {
    fn engine(&self, zpl: &str, width: u32, height: u32, dpi: u32) -> Result<ZplEngine, JsError> {
        validate(zpl, width, height, dpi).map_err(JsError::new)?;
        let resolution = match dpi {
            203 => Resolution::Dpi203,
            300 => Resolution::Dpi300,
            600 => Resolution::Dpi600,
            _ => unreachable!(),
        };
        let mut engine = ZplEngine::new(zpl, Unit::Dots(width), Unit::Dots(height), resolution)
            .map_err(|_| JsError::new("ZPL_INVALID"))?;
        engine.set_fonts(Arc::clone(&self.fonts));
        Ok(engine)
    }
}

fn validate(zpl: &str, width: u32, height: u32, dpi: u32) -> Result<(), &'static str> {
    if zpl.is_empty() || zpl.len() > 64 * 1024 {
        return Err("ZPL_INPUT_LIMIT");
    }
    if !zpl.trim().starts_with("^XA")
        || !zpl.trim().ends_with("^XZ")
        || zpl.matches("^XA").count() != 1
        || zpl.matches("^XZ").count() != 1
    {
        return Err("ZPL_SINGLE_FORMAT_REQUIRED");
    }
    if width == 0
        || height == 0
        || width > 4096
        || height > 4096
        || u64::from(width) * u64::from(height) > 4_194_304
    {
        return Err("ZPL_CANVAS_LIMIT");
    }
    if !matches!(dpi, 203 | 300 | 600) {
        return Err("ZPL_RESOLUTION_INVALID");
    }
    Ok(())
}

fn map_render_error(error: zpl_forge::ZplError) -> JsError {
    let code = match error {
        zpl_forge::ZplError::FontError(_) => "ZPL_FONT_UNSUPPORTED",
        zpl_forge::ZplError::SecurityLimitExceeded(_) => "ZPL_RESOURCE_LIMIT",
        _ => "ZPL_RENDER_FAILED",
    };
    JsError::new(code)
}

fn check_output(output: Vec<u8>) -> Result<Vec<u8>, JsError> {
    if output.len() > 16 * 1024 * 1024 {
        return Err(JsError::new("ZPL_OUTPUT_LIMIT"));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_limits_before_parsing() {
        assert_eq!(validate("", 816, 1216, 203), Err("ZPL_INPUT_LIMIT"));
        assert_eq!(validate("^XA^XZ", 4096, 4096, 203), Err("ZPL_CANVAS_LIMIT"));
        assert_eq!(
            validate("^XA^XZ", 816, 1216, 200),
            Err("ZPL_RESOLUTION_INVALID")
        );
        assert!(validate("^XA^XZ", 816, 1216, 203).is_ok());
    }
}
