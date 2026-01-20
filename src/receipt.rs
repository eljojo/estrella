//! # Receipt Builders
//!
//! Pre-built receipt templates demonstrating StarPRNT text capabilities.
//!
//! These generate command sequences that can be sent directly to the printer.

use crate::protocol::{
    barcode::{barcode1d, pdf417, qr},
    commands,
    text::{self, Font},
};

const LF: u8 = 0x0A;

/// Append text with line feed
fn text_line(data: &mut Vec<u8>, s: &str) {
    data.extend(s.as_bytes());
    data.push(LF);
}

/// Append a horizontal rule
fn hr(data: &mut Vec<u8>, ch: char, width: usize) {
    for _ in 0..width {
        data.push(ch as u8);
    }
    data.push(LF);
}

/// Reset all text styles to defaults
fn reset_styles(data: &mut Vec<u8>) {
    data.extend(text::invert_off());
    data.extend(text::underline_off());
    data.extend(text::upperline_off());
    data.extend(text::bold_off());
    data.extend(text::reduced_off());
    data.extend(text::size_normal());
    data.extend(text::smoothing_on());
    data.extend(text::align_left());
}

/// Generate a simple demo receipt (matches receipt.py)
///
/// Features demonstrated:
/// - Text alignment (left, center, right)
/// - Bold/emphasis
/// - Underline and upperline
/// - Inverted text (white on black)
/// - Size scaling
/// - Reduced printing (fine print)
/// - Upside-down text
pub fn demo_receipt() -> Vec<u8> {
    let mut data = Vec::new();

    // Initialize printer
    data.extend(commands::init());
    reset_styles(&mut data);

    // --- Header ---
    data.extend(text::align_center());
    data.extend(text::smoothing_on());
    data.extend(text::bold_on());
    data.extend(text::size(2, 2)); // 3x height and width
    text_line(&mut data, "CHURRA MART");
    data.extend(text::size_normal());
    data.extend(text::bold_off());

    data.extend(text::underline_on());
    text_line(&mut data, "starprnt style demo receipt");
    data.extend(text::underline_off());

    // Timestamp
    text_line(&mut data, "2026-01-20 12:00:00");
    data.extend(commands::feed_mm(3.0));

    // --- Inverted banner ---
    data.extend(text::invert_on());
    data.extend(text::bold_on());
    text_line(&mut data, "  TODAY ONLY: 0% OFF EVERYTHING  ");
    data.extend(text::bold_off());
    data.extend(text::invert_off());

    data.extend(commands::feed_mm(2.0));

    // --- Items table ---
    data.extend(text::align_left());
    data.extend(text::bold_on());
    text_line(&mut data, "ITEM                          CAD");
    data.extend(text::bold_off());
    hr(&mut data, '-', 48);

    text_line(&mut data, "Liminal Espresso              4.50");
    text_line(&mut data, "Basement Techno Vinyl        29.00");
    text_line(&mut data, "Thermal Paper (mystery)       7.25");
    text_line(&mut data, "Sticker: *****                2.00");

    hr(&mut data, '-', 48);
    data.extend(text::align_right());
    data.extend(text::bold_on());
    text_line(&mut data, "SUBTOTAL:  42.75");
    data.extend(text::bold_off());
    text_line(&mut data, "HST (13%):  5.56");
    data.extend(text::bold_on());
    data.extend(text::double_width_on());
    text_line(&mut data, "TOTAL:    48.31");
    data.extend(text::double_width_off());
    data.extend(text::bold_off());

    data.extend(commands::feed_mm(3.0));

    // --- Boxed thank you ---
    data.extend(text::align_center());
    data.extend(text::upperline_on());
    data.extend(text::underline_on());
    text_line(&mut data, "thank you for your vibes");
    data.extend(text::upperline_off());
    data.extend(text::underline_off());

    data.extend(commands::feed_mm(2.5));

    // --- Upside-down easter egg ---
    data.push(LF);
    data.extend(text::upside_down_on());
    data.extend(text::align_center());
    text_line(&mut data, "secret message from below");
    data.extend(text::upside_down_off());
    data.extend(text::align_left());

    data.extend(commands::feed_mm(2.5));

    // --- Fine print ---
    data.extend(text::reduced(1, 2)); // 67% width, 75% height
    data.extend(text::align_left());
    text_line(
        &mut data,
        "fine print: this receipt exists to show StarPRNT text styling.",
    );
    text_line(
        &mut data,
        "note: some options depend on printer spec / memory switch settings.",
    );
    data.extend(text::reduced_off());

    data.extend(commands::feed_mm(4.5));

    // --- Footer ---
    data.extend(text::align_center());
    data.extend(text::bold_on());
    text_line(&mut data, "COME BACK SOON");
    data.extend(text::bold_off());

    data.extend(commands::feed_mm(6.0));
    data.extend(commands::cut_full_feed());

    data
}

/// Generate a full demo receipt with barcodes (matches receipt2.py)
///
/// Features demonstrated:
/// - Everything from demo_receipt()
/// - Font selection (A, B, C)
/// - Code39 barcode
/// - QR code
/// - PDF417 barcode
pub fn full_receipt() -> Vec<u8> {
    let mut data = Vec::new();

    // Initialize printer
    data.extend(commands::init());
    data.extend(text::codepage_raw(1)); // CP437
    reset_styles(&mut data);

    // --- Header ---
    data.extend(text::align_center());
    data.extend(text::font(Font::A));
    data.extend(text::smoothing_on());
    data.extend(text::bold_on());
    data.extend(text::double_height_on());
    data.extend(text::double_width_on());
    text_line(&mut data, "CHURRA MART");
    reset_styles(&mut data);

    data.extend(text::align_center());
    data.extend(text::underline_on());
    text_line(&mut data, "StarPRNT style demo receipt");
    data.extend(text::underline_off());

    text_line(&mut data, "2026-01-20 12:00:00");
    data.extend(commands::feed_mm(2.5));

    // --- Inverted banner ---
    data.extend(text::align_center());
    data.extend(text::invert_on());
    text_line(&mut data, " TODAY ONLY: 0% OFF EVERYTHING ");
    data.extend(text::invert_off());

    data.extend(commands::feed_mm(2.0));

    // --- Font showcase ---
    data.extend(text::align_left());
    hr(&mut data, '-', 48);

    data.extend(text::bold_on());
    text_line(&mut data, "FONTS:");
    data.extend(text::bold_off());

    data.extend(text::font(Font::A));
    text_line(&mut data, "Font A (12x24): THE QUICK BROWN FOX 0123456789");
    data.extend(text::font(Font::B));
    text_line(&mut data, "Font B ( 9x24): THE QUICK BROWN FOX 0123456789");
    data.extend(text::font(Font::C));
    text_line(&mut data, "Font C ( 9x17): THE QUICK BROWN FOX 0123456789");
    data.extend(text::font(Font::A));

    data.extend(commands::feed_mm(2.0));

    // --- Style showcase ---
    hr(&mut data, '-', 48);
    data.extend(text::bold_on());
    text_line(&mut data, "STYLES:");
    data.extend(text::bold_off());

    text_line(&mut data, "Normal text.");

    data.extend(text::bold_on());
    text_line(&mut data, "Emphasized (bold-ish).");
    data.extend(text::bold_off());

    data.extend(text::underline_on());
    text_line(&mut data, "Underlined.");
    data.extend(text::underline_off());

    data.extend(text::invert_on());
    text_line(&mut data, "White/black inverted.");
    data.extend(text::invert_off());

    data.extend(text::smoothing_on());
    text_line(&mut data, "Smoothing ON (edges a bit softer).");
    data.extend(text::smoothing_off());

    data.extend(text::double_width_on());
    text_line(&mut data, "Double-wide.");
    data.extend(text::double_width_off());

    data.extend(text::double_height_on());
    text_line(&mut data, "Double-high.");
    data.extend(text::double_height_off());

    data.extend(text::double_width_on());
    data.extend(text::double_height_on());
    text_line(&mut data, "BIG BIG");
    reset_styles(&mut data);

    data.extend(text::upside_down_on());
    data.extend(text::align_center());
    text_line(&mut data, "upside-down message (SI)");
    data.extend(text::upside_down_off());
    data.extend(text::align_left());

    data.extend(commands::feed_mm(2.0));

    // --- Receipt body ---
    hr(&mut data, '-', 48);
    data.extend(text::align_left());
    data.extend(text::bold_on());
    text_line(&mut data, "ITEM                            CAD");
    data.extend(text::bold_off());
    hr(&mut data, '-', 48);

    text_line(&mut data, "Liminal Espresso               4.50");
    text_line(&mut data, "Basement Techno Vinyl         29.00");
    text_line(&mut data, "Thermal Paper (mystery)        7.25");
    text_line(&mut data, "Sticker: *****                 2.00");

    hr(&mut data, '-', 48);

    data.extend(text::align_right());
    text_line(&mut data, "SUBTOTAL:  42.75");
    text_line(&mut data, "HST (13%):  5.56");

    data.extend(text::bold_on());
    data.extend(text::double_width_on());
    text_line(&mut data, "TOTAL:    48.31");
    reset_styles(&mut data);

    data.extend(commands::feed_mm(3.0));

    // --- Barcodes ---
    hr(&mut data, '-', 48);
    data.extend(text::align_center());
    data.extend(text::bold_on());
    text_line(&mut data, "CODES:");
    data.extend(text::bold_off());

    data.extend(text::align_left());
    text_line(&mut data, "1D Barcode (Code39 + HRI):");
    data.extend(barcode1d::code39(b"CHURRA-2026-0001", 80));
    data.extend(commands::feed_mm(3.0));

    data.extend(text::align_left());
    text_line(&mut data, "QR Code:");
    data.extend(text::align_center());
    // QR code commands
    data.extend(qr::set_model(qr::QrModel::Model2));
    data.extend(qr::set_error_correction(qr::QrErrorLevel::M));
    data.extend(qr::set_cell_size(6));
    data.extend(qr::set_data(b"https://example.invalid/churra-mart"));
    data.extend(qr::print());
    data.extend(commands::feed_mm(3.0));

    data.extend(text::align_left());
    text_line(&mut data, "PDF417:");
    data.extend(text::align_center());
    // PDF417 commands
    data.extend(pdf417::set_size_ratio(6, 3));
    data.extend(pdf417::set_ecc_level(3));
    data.extend(pdf417::set_module_width(2));
    data.extend(pdf417::set_data(b"CHURRA|MART|ORDER|2026-0001|TOTAL|48.31"));
    data.extend(pdf417::print());

    data.extend(commands::feed_mm(4.0));

    // --- Footer ---
    hr(&mut data, '-', 48);
    data.extend(text::align_center());
    data.extend(text::underline_on());
    text_line(&mut data, "thank you for your vibes");
    data.extend(text::underline_off());

    data.extend(commands::feed_mm(2.0));

    data.extend(text::align_left());
    data.extend(text::font(Font::B));
    text_line(
        &mut data,
        "fine print: this receipt exists to show StarPRNT text styling.",
    );
    text_line(
        &mut data,
        "note: some options depend on printer spec / memory switch settings.",
    );
    text_line(
        &mut data,
        "tip: avoid Unicode unless you really know your code page.",
    );
    data.extend(text::font(Font::A));

    data.extend(commands::feed_mm(6.0));
    data.extend(text::align_center());
    data.extend(text::bold_on());
    text_line(&mut data, "COME BACK SOON");
    data.extend(text::bold_off());

    data.extend(commands::feed_mm(10.0));
    data.extend(commands::cut_full_feed());

    data
}

/// List available receipt templates
pub fn list_receipts() -> &'static [&'static str] {
    &["receipt", "receipt-full"]
}

/// Get receipt data by name
pub fn by_name(name: &str) -> Option<Vec<u8>> {
    match name.to_lowercase().as_str() {
        "receipt" => Some(demo_receipt()),
        "receipt-full" | "receipt_full" => Some(full_receipt()),
        _ => None,
    }
}

/// Check if a name is a receipt template
pub fn is_receipt(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "receipt" | "receipt-full" | "receipt_full"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_receipt_not_empty() {
        let data = demo_receipt();
        assert!(!data.is_empty());
        // Should start with init command (ESC @)
        assert_eq!(&data[0..2], &[0x1B, 0x40]);
    }

    #[test]
    fn test_full_receipt_not_empty() {
        let data = full_receipt();
        assert!(!data.is_empty());
        // Should start with init command
        assert_eq!(&data[0..2], &[0x1B, 0x40]);
    }

    #[test]
    fn test_full_receipt_has_barcodes() {
        let data = full_receipt();
        // Should contain QR print command (ESC GS y P)
        let qr_print = [0x1B, 0x1D, b'y', b'P'];
        assert!(data.windows(4).any(|w| w == qr_print));
    }

    #[test]
    fn test_list_receipts() {
        let receipts = list_receipts();
        assert!(receipts.contains(&"receipt"));
        assert!(receipts.contains(&"receipt-full"));
    }

    #[test]
    fn test_by_name() {
        assert!(by_name("receipt").is_some());
        assert!(by_name("receipt-full").is_some());
        assert!(by_name("nonexistent").is_none());
    }

    #[test]
    fn test_is_receipt() {
        assert!(is_receipt("receipt"));
        assert!(is_receipt("receipt-full"));
        assert!(!is_receipt("ripple"));
    }
}
