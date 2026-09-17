use std::io::Cursor;

use image::{DynamicImage, ImageBuffer, ImageFormat, Luma};
use rxing::{BarcodeFormat, EncodeHints, MultiFormatWriter, Writer};

use crate::{ZplError, ZplResult};

const MAX_DATA_BYTES: usize = 4096;
const MAX_PIXELS: u64 = 16_000_000;
const MAX_SVG_BYTES: usize = 8 * 1024 * 1024;

pub fn render_svg(kind: &str, data: &str, module_size: u32, quiet_zone: u32) -> ZplResult<String> {
    validate_layout(module_size, quiet_zone)?;
    let matrix = encode(kind, data, quiet_zone)?;
    let width = checked_dimension(matrix.width(), module_size)?;
    let height = checked_dimension(matrix.height(), module_size)?;
    let mut path = String::new();
    for y in 0..matrix.height() {
        let mut x = 0;
        while x < matrix.width() {
            if !matrix.get(x, y) {
                x += 1;
                continue;
            }
            let start = x;
            while x < matrix.width() && matrix.get(x, y) {
                x += 1;
            }
            path.push_str(&format!(
                "M{} {}h{}v{}H{}z",
                start * module_size,
                y * module_size,
                (x - start) * module_size,
                module_size,
                start * module_size
            ));
        }
    }
    let output = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\" shape-rendering=\"crispEdges\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/><path d=\"{path}\" fill=\"black\"/></svg>"
    );
    if output.len() > MAX_SVG_BYTES {
        return Err(ZplError::BackendError("BARCODE_OUTPUT_LIMIT".into()));
    }
    Ok(output)
}

pub fn render_png(kind: &str, data: &str, module_size: u32, quiet_zone: u32) -> ZplResult<Vec<u8>> {
    validate_layout(module_size, quiet_zone)?;
    let matrix = encode(kind, data, quiet_zone)?;
    let width = checked_dimension(matrix.width(), module_size)?;
    let height = checked_dimension(matrix.height(), module_size)?;
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(ZplError::BackendError("BARCODE_OUTPUT_LIMIT".into()));
    }
    let mut image = ImageBuffer::from_pixel(width, height, Luma([255u8]));
    for y in 0..matrix.height() {
        for x in 0..matrix.width() {
            if !matrix.get(x, y) {
                continue;
            }
            for py in y * module_size..(y + 1) * module_size {
                for px in x * module_size..(x + 1) * module_size {
                    image.put_pixel(px, py, Luma([0u8]));
                }
            }
        }
    }
    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageLuma8(image)
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|error| ZplError::BackendError(format!("PNG Encoding Error: {error}")))?;
    Ok(output.into_inner())
}

fn encode(kind: &str, data: &str, quiet_zone: u32) -> ZplResult<rxing::common::BitMatrix> {
    if data.is_empty() || data.len() > MAX_DATA_BYTES || module_type(kind).is_none() {
        return Err(ZplError::BackendError("BARCODE_INPUT_INVALID".into()));
    }
    let hints = EncodeHints {
        Margin: Some(quiet_zone.to_string()),
        ..EncodeHints::default()
    };
    MultiFormatWriter
        .encode_with_hints(data, &module_type(kind).unwrap(), 0, 0, &hints)
        .map_err(|error| ZplError::BackendError(format!("Barcode Generation Error: {error}")))
}

fn module_type(kind: &str) -> Option<BarcodeFormat> {
    match kind {
        "CODE_128" => Some(BarcodeFormat::CODE_128),
        "CODE_39" => Some(BarcodeFormat::CODE_39),
        "EAN_13" => Some(BarcodeFormat::EAN_13),
        "INTERLEAVED_2_OF_5" => Some(BarcodeFormat::ITF),
        "PDF417" => Some(BarcodeFormat::PDF_417),
        "AZTEC" => Some(BarcodeFormat::AZTEC),
        "DATAMATRIX" => Some(BarcodeFormat::DATA_MATRIX),
        "QR" => Some(BarcodeFormat::QR_CODE),
        _ => None,
    }
}

fn checked_dimension(modules: u32, module_size: u32) -> ZplResult<u32> {
    modules
        .checked_mul(module_size)
        .ok_or_else(|| ZplError::BackendError("BARCODE_OUTPUT_LIMIT".into()))
}

fn validate_layout(module_size: u32, quiet_zone: u32) -> ZplResult<()> {
    if !(1..=32).contains(&module_size) || quiet_zone > 64 {
        return Err(ZplError::BackendError("BARCODE_INPUT_INVALID".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_qr_svg_and_code128_png() {
        let svg = render_svg("QR", "https://yqn.com", 4, 4).unwrap();
        assert!(svg.starts_with("<svg "));
        assert!(svg.contains("<path d=\"M"));
        let png = render_png("CODE_128", "ABC-123", 2, 10).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn rejects_unknown_or_unbounded_input() {
        assert!(render_svg("UNKNOWN", "x", 1, 0).is_err());
        assert!(render_svg("QR", "", 1, 0).is_err());
        assert!(render_svg("QR", "x", 0, 0).is_err());
    }
}
