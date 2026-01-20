//! # Code Generation
//!
//! Converts IR programs to StarPRNT protocol bytes.

use super::ops::{BarcodeKind, Op, Program};
use crate::printer::PrinterConfig;
use crate::protocol::{barcode, commands, graphics, nv_graphics, text};

impl Program {
    /// Compile the IR program to StarPRNT bytes.
    ///
    /// Uses the default printer configuration (TSP650II).
    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes_with_config(&PrinterConfig::TSP650II)
    }

    /// Compile the IR program to StarPRNT bytes with a specific printer config.
    pub fn to_bytes_with_config(&self, _config: &PrinterConfig) -> Vec<u8> {
        let mut out = Vec::new();

        for op in &self.ops {
            match op {
                // ===== Printer Control =====
                Op::Init => {
                    out.extend(commands::init());
                }
                Op::Cut { partial } => {
                    if *partial {
                        out.extend(commands::cut_partial_feed());
                    } else {
                        out.extend(commands::cut_full_feed());
                    }
                }
                Op::Feed { units } => {
                    out.extend(commands::feed_units(*units));
                }

                // ===== Style Changes =====
                Op::SetAlign(align) => {
                    out.extend(text::align(*align));
                }
                Op::SetFont(font) => {
                    out.extend(text::font(*font));
                }
                Op::SetBold(enabled) => {
                    if *enabled {
                        out.extend(text::bold_on());
                    } else {
                        out.extend(text::bold_off());
                    }
                }
                Op::SetUnderline(enabled) => {
                    if *enabled {
                        out.extend(text::underline_on());
                    } else {
                        out.extend(text::underline_off());
                    }
                }
                Op::SetInvert(enabled) => {
                    if *enabled {
                        out.extend(text::invert_on());
                    } else {
                        out.extend(text::invert_off());
                    }
                }
                Op::SetSize { height, width } => {
                    out.extend(text::size(*height, *width));
                }
                Op::SetExpandedWidth(mult) => {
                    out.extend(text::expanded_width(*mult));
                }
                Op::SetExpandedHeight(mult) => {
                    out.extend(text::expanded_height(*mult));
                }
                Op::SetSmoothing(enabled) => {
                    if *enabled {
                        out.extend(text::smoothing_on());
                    } else {
                        out.extend(text::smoothing_off());
                    }
                }
                Op::SetUpperline(enabled) => {
                    if *enabled {
                        out.extend(text::upperline_on());
                    } else {
                        out.extend(text::upperline_off());
                    }
                }
                Op::SetUpsideDown(enabled) => {
                    if *enabled {
                        out.extend(text::upside_down_on());
                    } else {
                        out.extend(text::upside_down_off());
                    }
                }
                Op::SetReduced(enabled) => {
                    if *enabled {
                        out.extend(text::reduced(1, 1)); // Horizontal and vertical reduction
                    } else {
                        out.extend(text::reduced_off());
                    }
                }
                Op::SetCodepage(page) => {
                    out.extend(text::codepage_raw(*page));
                }
                Op::ResetStyle => {
                    out.extend(text::TextStyle::reset());
                }

                // ===== Content =====
                Op::Text(s) => {
                    out.extend(s.as_bytes());
                }
                Op::Newline => {
                    out.push(0x0A);
                }
                Op::Raw(bytes) => {
                    out.extend(bytes);
                }

                // ===== Graphics =====
                Op::Raster {
                    width,
                    height,
                    data,
                } => {
                    // Chunk large raster images to avoid printer buffer overflow
                    // Max chunk: 256 rows (matches golden test behavior)
                    let width_bytes = width.div_ceil(8) as usize;
                    let chunk_rows = 256usize;
                    let total_height = *height as usize;

                    let mut row_offset = 0;
                    while row_offset < total_height {
                        let chunk_height = (total_height - row_offset).min(chunk_rows);
                        let byte_start = row_offset * width_bytes;
                        let byte_end = (row_offset + chunk_height) * width_bytes;
                        let chunk_data = &data[byte_start..byte_end];

                        out.extend(graphics::raster(*width, chunk_height as u16, chunk_data));
                        row_offset += chunk_height;
                    }
                }
                Op::Band { width_bytes, data } => {
                    // Band mode: 24-row chunks with feed after each band
                    // Matches Python sick.py behavior
                    let band_size = *width_bytes as usize * 24;

                    for chunk in data.chunks(band_size) {
                        if chunk.len() == band_size {
                            out.extend(graphics::band(*width_bytes, chunk));
                        } else {
                            // Pad last band to 24 rows with white
                            let mut padded = chunk.to_vec();
                            padded.resize(band_size, 0x00);
                            out.extend(graphics::band(*width_bytes, &padded));
                        }
                        // Feed 3mm after each band (12 units = 3mm)
                        out.extend(commands::feed_units(12));
                    }
                }

                // ===== Barcodes =====
                Op::QrCode {
                    data,
                    cell_size,
                    error_level,
                } => {
                    out.extend(barcode::qr::generate(
                        data.as_bytes(),
                        *cell_size,
                        *error_level,
                    ));
                }
                Op::Pdf417 {
                    data,
                    module_width,
                    ecc_level,
                } => {
                    out.extend(barcode::pdf417::generate(
                        data.as_bytes(),
                        *module_width,
                        *ecc_level,
                    ));
                }
                Op::Barcode1D { kind, data, height } => {
                    let barcode_fn = match kind {
                        BarcodeKind::Code39 => barcode::barcode1d::code39,
                        BarcodeKind::Code128 => barcode::barcode1d::code128,
                        BarcodeKind::Ean13 => barcode::barcode1d::ean13,
                        BarcodeKind::UpcA => barcode::barcode1d::upca,
                        BarcodeKind::Itf => barcode::barcode1d::itf,
                    };
                    out.extend(barcode_fn(data.as_bytes(), *height));
                }

                // ===== NV Graphics =====
                Op::NvStore {
                    key,
                    width,
                    height,
                    data,
                } => {
                    if let Some(cmd) = nv_graphics::define(key, *width, *height, data) {
                        out.extend(cmd);
                    }
                }
                Op::NvPrint {
                    key,
                    scale_x,
                    scale_y,
                } => {
                    if let Some(cmd) = nv_graphics::print(key, *scale_x, *scale_y) {
                        out.extend(cmd);
                    }
                }
                Op::NvDelete { key } => {
                    if let Some(cmd) = nv_graphics::erase(key) {
                        out.extend(cmd);
                    }
                }
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::text::Alignment;

    #[test]
    fn test_empty_program() {
        let program = Program::new();
        let bytes = program.to_bytes();
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_init_only() {
        let program = Program::with_init();
        let bytes = program.to_bytes();
        assert_eq!(bytes, vec![0x1B, 0x40]);
    }

    #[test]
    fn test_simple_text() {
        let mut program = Program::with_init();
        program.push(Op::Text("Hello".into()));
        program.push(Op::Newline);

        let bytes = program.to_bytes();
        assert!(bytes.starts_with(&[0x1B, 0x40])); // Init
        assert!(bytes.ends_with(&[b'H', b'e', b'l', b'l', b'o', 0x0A]));
    }

    #[test]
    fn test_styled_text() {
        let mut program = Program::with_init();
        program.push(Op::SetAlign(Alignment::Center));
        program.push(Op::SetBold(true));
        program.push(Op::Text("HEADER".into()));
        program.push(Op::Newline);
        program.push(Op::SetBold(false));

        let bytes = program.to_bytes();

        // Should contain init
        assert!(bytes.starts_with(&[0x1B, 0x40]));
        // Should contain center align (ESC GS a 1)
        assert!(bytes.windows(4).any(|w| w == [0x1B, 0x1D, 0x61, 0x01]));
        // Should contain bold on (ESC E)
        assert!(bytes.windows(2).any(|w| w == [0x1B, 0x45]));
        // Should contain bold off (ESC F)
        assert!(bytes.windows(2).any(|w| w == [0x1B, 0x46]));
    }

    #[test]
    fn test_cut() {
        let mut program = Program::with_init();
        program.push(Op::Cut { partial: false });

        let bytes = program.to_bytes();
        // Full cut with feed: ESC d 2
        assert!(bytes.ends_with(&[0x1B, 0x64, 0x02]));
    }

    #[test]
    fn test_partial_cut() {
        let mut program = Program::with_init();
        program.push(Op::Cut { partial: true });

        let bytes = program.to_bytes();
        // Partial cut with feed: ESC d 3
        assert!(bytes.ends_with(&[0x1B, 0x64, 0x03]));
    }

    #[test]
    fn test_feed() {
        let mut program = Program::new();
        program.push(Op::Feed { units: 20 }); // 5mm

        let bytes = program.to_bytes();
        // Feed: ESC J 20
        assert_eq!(bytes, vec![0x1B, 0x4A, 20]);
    }

    #[test]
    fn test_raster_graphics() {
        let mut program = Program::new();
        // Small 8x2 raster (1 byte wide, 2 rows)
        let data = vec![0xFF, 0xAA];
        program.push(Op::Raster {
            width: 8,
            height: 2,
            data,
        });

        let bytes = program.to_bytes();
        // Raster command: ESC GS S 1 1 0 2 0 0 data
        assert!(bytes.starts_with(&[0x1B, 0x1D, 0x53]));
    }

    #[test]
    fn test_qr_code() {
        let mut program = Program::new();
        program.push(Op::QrCode {
            data: "https://example.com".into(),
            cell_size: 4,
            error_level: barcode::qr::QrErrorLevel::M,
        });

        let bytes = program.to_bytes();
        // Should contain QR model command (ESC GS y S 0)
        assert!(
            bytes
                .windows(5)
                .any(|w| w == [0x1B, 0x1D, 0x79, 0x53, 0x30])
        );
        // Should contain QR print command (ESC GS y P)
        assert!(bytes.windows(4).any(|w| w == [0x1B, 0x1D, 0x79, 0x50]));
    }

    #[test]
    fn test_raw_bytes() {
        let mut program = Program::new();
        program.push(Op::Raw(vec![0x01, 0x02, 0x03]));

        let bytes = program.to_bytes();
        assert_eq!(bytes, vec![0x01, 0x02, 0x03]);
    }
}
