//! # Receipt Builders
//!
//! Pre-built receipt templates demonstrating StarPRNT text capabilities.
//!
//! These generate command sequences that can be sent directly to the printer.
//! Receipts are built using the declarative component system and optimized
//! for minimal byte output.

use crate::components::{
    Barcode, Columns, ComponentExt, Divider, LineItem, NvLogo, Pdf417, QrCode, Raw, Receipt,
    Spacer, Text, Total,
};
use crate::ir::{Op, Program};
use crate::protocol::text::Font;

// ============================================================================
// RECEIPT TEMPLATES
// ============================================================================

/// Generate a simple demo receipt.
///
/// Features demonstrated:
/// - Text alignment (left, center, right)
/// - Bold/emphasis
/// - Underline and upperline
/// - Inverted text (white on black)
/// - Size scaling
/// - Upside-down text
/// - Font selection
pub fn demo_receipt() -> Vec<u8> {
    Receipt::new()
        // Header
        .child(
            Text::new("CHURRA MART")
                .center()
                .bold()
                .size(2, 2), // Smoothing auto-enabled for scaled text
        )
        .child(
            Text::new("starprnt style demo receipt")
                .center()
                .underline(),
        )
        .child(Text::new("2026-01-20 12:00:00").center())
        .child(Spacer::mm(3.0))
        // Inverted banner
        .child(
            Text::new("  TODAY ONLY: 0% OFF EVERYTHING  ")
                .center()
                .invert()
                .bold(),
        )
        .child(Spacer::mm(2.0))
        // Items table
        .child(Columns::new("ITEM", "CAD").bold())
        .child(Divider::dashed())
        .child(LineItem::new("Liminal Espresso", 4.50))
        .child(LineItem::new("Basement Techno Vinyl", 29.00))
        .child(LineItem::new("Thermal Paper (mystery)", 7.25))
        .child(LineItem::new("Sticker: *****", 2.00))
        .child(Divider::dashed())
        // Totals
        .child(Total::labeled("SUBTOTAL:", 42.75).bold())
        .child(Total::labeled("HST (13%):", 5.56))
        .child(Total::labeled("TOTAL:", 48.31).bold().double_width())
        .child(Spacer::mm(3.0))
        // Boxed thank you
        .child(
            Text::new("thank you for your vibes")
                .center()
                .underline()
                .upperline(),
        )
        .child(Spacer::mm(2.5))
        // Upside down easter egg
        .child(Text::new("")) // blank line
        .child(
            Text::new("secret message from below")
                .center()
                .upside_down(),
        )
        .child(Spacer::mm(2.5))
        // Fine print
        .child(
            Text::new("fine print: this receipt exists to show StarPRNT text styling.")
                .left()
                .font(Font::B),
        )
        .child(
            Text::new("note: some options depend on printer spec / memory switch settings.")
                .font(Font::B),
        )
        .child(Spacer::mm(4.5))
        // Footer
        .child(Text::new("COME BACK SOON").center().bold())
        .child(Spacer::mm(6.0))
        .cut()
        .build()
}

/// Generate a full demo receipt with barcodes.
///
/// Features demonstrated:
/// - Everything from demo_receipt()
/// - NV logo (if stored with key "A0")
/// - Font selection (A, B, C)
/// - Code39 barcode
/// - QR code
/// - PDF417 barcode
pub fn full_receipt() -> Vec<u8> {
    Receipt::new()
        // Set codepage
        .child(Raw::op(Op::SetCodepage(1)))
        // NV Logo (star from registry)
        .child(NvLogo::new("A1").center())
        .child(Spacer::mm(2.0))
        // Header
        .child(
            Text::new("CHURRA MART")
                .center()
                .bold()
                .double_height()
                .double_width(), // Smoothing auto-enabled for scaled text
        )
        .child(
            Text::new("StarPRNT style demo receipt")
                .center()
                .underline(),
        )
        .child(Text::new("2026-01-20 12:00:00").center())
        .child(Spacer::mm(2.5))
        // Inverted banner
        .child(
            Text::new(" TODAY ONLY: 0% OFF EVERYTHING ")
                .center()
                .invert(),
        )
        .child(Spacer::mm(2.0))
        // Font showcase
        .child(Divider::dashed())
        .child(Text::new("FONTS:").left().bold())
        .child(Text::new("Font A (12x24): THE QUICK BROWN FOX 0123456789").font(Font::A))
        .child(Text::new("Font B ( 9x24): THE QUICK BROWN FOX 0123456789").font(Font::B))
        .child(Text::new("Font C ( 9x17): THE QUICK BROWN FOX 0123456789").font(Font::C))
        .child(Spacer::mm(2.0))
        // Style showcase
        .child(Divider::dashed())
        .child(Text::new("STYLES:").left().bold())
        .child(Text::new("Normal text."))
        .child(Text::new("Emphasized (bold-ish).").bold())
        .child(Text::new("Underlined.").underline())
        .child(Text::new("White/black inverted.").invert())
        .child(Text::new("Smoothing ON (edges a bit softer).").smoothing())
        .child(Text::new("Double-wide.").double_width())
        .child(Text::new("Double-high.").double_height())
        .child(Text::new("BIG BIG").double_width().double_height())
        .child(Text::new("upside-down message").center().upside_down())
        .child(Spacer::mm(2.0))
        // Receipt body
        .child(Divider::dashed())
        .child(Columns::new("ITEM", "CAD").bold())
        .child(Divider::dashed())
        .child(LineItem::new("Liminal Espresso", 4.50))
        .child(LineItem::new("Basement Techno Vinyl", 29.00))
        .child(LineItem::new("Thermal Paper (mystery)", 7.25))
        .child(LineItem::new("Sticker: *****", 2.00))
        .child(Divider::dashed())
        // Totals
        .child(Total::labeled("SUBTOTAL:", 42.75))
        .child(Total::labeled("HST (13%):", 5.56))
        .child(Total::labeled("TOTAL:", 48.31).bold().double_width())
        .child(Spacer::mm(3.0))
        // Barcodes
        .child(Divider::dashed())
        .child(Text::new("CODES:").center().bold())
        .child(Text::new("1D Barcode (Code39 + HRI):").left())
        .child(Barcode::code39("CHURRA-2026-0001").height(80))
        .child(Spacer::mm(3.0))
        .child(Text::new("QR Code:").left())
        .child(QrCode::new("https://example.invalid/churra-mart").cell_size(6))
        .child(Spacer::mm(3.0))
        .child(Text::new("PDF417:").left())
        .child(
            Pdf417::new("CHURRA|MART|ORDER|2026-0001|TOTAL|48.31")
                .module_width(2)
                .ecc_level(3),
        )
        .child(Spacer::mm(4.0))
        // Footer
        .child(Divider::dashed())
        .child(Text::new("thank you for your vibes").center().underline())
        .child(Spacer::mm(2.0))
        .child(
            Text::new("fine print: this receipt exists to show StarPRNT text styling.")
                .left()
                .font(Font::B),
        )
        .child(
            Text::new("note: some options depend on printer spec / memory switch settings.")
                .font(Font::B),
        )
        .child(Text::new("tip: avoid Unicode unless you really know your code page.").font(Font::B))
        .child(Spacer::mm(6.0))
        .child(Text::new("COME BACK SOON").center().bold())
        .child(Spacer::mm(10.0))
        .cut()
        .build()
}

// ============================================================================
// LOOKUP FUNCTIONS
// ============================================================================

/// List available receipt templates
pub fn list_receipts() -> &'static [&'static str] {
    &["receipt", "receipt-full", "receipt-frame"]
}

/// Get receipt data by name
pub fn by_name(name: &str) -> Option<Vec<u8>> {
    match name.to_lowercase().as_str() {
        "receipt" => Some(demo_receipt()),
        "receipt-full" | "receipt_full" => Some(full_receipt()),
        "receipt-frame" | "receipt_frame" => Some(framed_receipt()),
        _ => None,
    }
}

/// Get receipt IR Program by name (for preview rendering).
pub fn program_by_name(name: &str) -> Option<crate::ir::Program> {
    use crate::components::ComponentExt;

    match name.to_lowercase().as_str() {
        "receipt" => Some(demo_receipt_component().compile()),
        "receipt-full" | "receipt_full" => Some(full_receipt_component().compile()),
        "receipt-frame" | "receipt_frame" => Some(framed_receipt_program()),
        _ => None,
    }
}

/// Get the demo receipt component (for preview rendering).
fn demo_receipt_component() -> Receipt {
    Receipt::new()
        // Header
        .child(
            Text::new("CHURRA MART")
                .center()
                .bold()
                .size(2, 2),
        )
        .child(
            Text::new("starprnt style demo receipt")
                .center()
                .underline(),
        )
        .child(Text::new("2026-01-20 12:00:00").center())
        .child(Spacer::mm(3.0))
        // Inverted banner
        .child(
            Text::new("  TODAY ONLY: 0% OFF EVERYTHING  ")
                .center()
                .invert()
                .bold(),
        )
        .child(Spacer::mm(2.0))
        // Items table
        .child(Columns::new("ITEM", "CAD").bold())
        .child(Divider::dashed())
        .child(LineItem::new("Liminal Espresso", 4.50))
        .child(LineItem::new("Basement Techno Vinyl", 29.00))
        .child(LineItem::new("Thermal Paper (mystery)", 7.25))
        .child(LineItem::new("Sticker: *****", 2.00))
        .child(Divider::dashed())
        // Totals
        .child(Total::labeled("SUBTOTAL:", 42.75).bold())
        .child(Total::labeled("HST (13%):", 5.56))
        .child(Total::labeled("TOTAL:", 48.31).bold().double_width())
        .child(Spacer::mm(3.0))
        // Boxed thank you
        .child(
            Text::new("thank you for your vibes")
                .center()
                .underline()
                .upperline(),
        )
        .child(Spacer::mm(2.5))
        // Upside down easter egg
        .child(Text::new(""))
        .child(
            Text::new("secret message from below")
                .center()
                .upside_down(),
        )
        .child(Spacer::mm(2.5))
        // Fine print
        .child(
            Text::new("fine print: this receipt exists to show StarPRNT text styling.")
                .left()
                .font(Font::B),
        )
        .child(
            Text::new("note: some options depend on printer spec / memory switch settings.")
                .font(Font::B),
        )
        .child(Spacer::mm(4.5))
        // Footer
        .child(Text::new("COME BACK SOON").center().bold())
        .child(Spacer::mm(6.0))
        .cut()
}

/// Get the full receipt component (for preview rendering).
fn full_receipt_component() -> Receipt {
    Receipt::new()
        // Set codepage
        .child(Raw::op(Op::SetCodepage(1)))
        // NV Logo (star from registry)
        .child(NvLogo::new("A1").center())
        .child(Spacer::mm(2.0))
        // Header
        .child(
            Text::new("CHURRA MART")
                .center()
                .bold()
                .double_height()
                .double_width(),
        )
        .child(
            Text::new("StarPRNT style demo receipt")
                .center()
                .underline(),
        )
        .child(Text::new("2026-01-20 12:00:00").center())
        .child(Spacer::mm(2.5))
        // Inverted banner
        .child(
            Text::new(" TODAY ONLY: 0% OFF EVERYTHING ")
                .center()
                .invert(),
        )
        .child(Spacer::mm(2.0))
        // Font showcase
        .child(Divider::dashed())
        .child(Text::new("FONTS:").left().bold())
        .child(Text::new("Font A (12x24): THE QUICK BROWN FOX 0123456789").font(Font::A))
        .child(Text::new("Font B ( 9x24): THE QUICK BROWN FOX 0123456789").font(Font::B))
        .child(Text::new("Font C ( 9x17): THE QUICK BROWN FOX 0123456789").font(Font::C))
        .child(Spacer::mm(2.0))
        // Style showcase
        .child(Divider::dashed())
        .child(Text::new("STYLES:").left().bold())
        .child(Text::new("Normal text."))
        .child(Text::new("Emphasized (bold-ish).").bold())
        .child(Text::new("Underlined.").underline())
        .child(Text::new("White/black inverted.").invert())
        .child(Text::new("Smoothing ON (edges a bit softer).").smoothing())
        .child(Text::new("Double-wide.").double_width())
        .child(Text::new("Double-high.").double_height())
        .child(Text::new("BIG BIG").double_width().double_height())
        .child(Text::new("upside-down message").center().upside_down())
        .child(Spacer::mm(2.0))
        // Receipt body
        .child(Divider::dashed())
        .child(Columns::new("ITEM", "CAD").bold())
        .child(Divider::dashed())
        .child(LineItem::new("Liminal Espresso", 4.50))
        .child(LineItem::new("Basement Techno Vinyl", 29.00))
        .child(LineItem::new("Thermal Paper (mystery)", 7.25))
        .child(LineItem::new("Sticker: *****", 2.00))
        .child(Divider::dashed())
        // Totals
        .child(Total::labeled("SUBTOTAL:", 42.75))
        .child(Total::labeled("HST (13%):", 5.56))
        .child(Total::labeled("TOTAL:", 48.31).bold().double_width())
        .child(Spacer::mm(3.0))
        // Barcodes
        .child(Divider::dashed())
        .child(Text::new("CODES:").center().bold())
        .child(Text::new("1D Barcode (Code39 + HRI):").left())
        .child(Barcode::code39("CHURRA-2026-0001").height(80))
        .child(Spacer::mm(3.0))
        .child(Text::new("QR Code:").left())
        .child(QrCode::new("https://example.invalid/churra-mart").cell_size(6))
        .child(Spacer::mm(3.0))
        .child(Text::new("PDF417:").left())
        .child(
            Pdf417::new("CHURRA|MART|ORDER|2026-0001|TOTAL|48.31")
                .module_width(2)
                .ecc_level(3),
        )
        .child(Spacer::mm(4.0))
        // Footer
        .child(Divider::dashed())
        .child(Text::new("thank you for your vibes").center().underline())
        .child(Spacer::mm(2.0))
        .child(
            Text::new("fine print: this receipt exists to show StarPRNT text styling.")
                .left()
                .font(Font::B),
        )
        .child(
            Text::new("note: some options depend on printer spec / memory switch settings.")
                .font(Font::B),
        )
        .child(Text::new("tip: avoid Unicode unless you really know your code page.").font(Font::B))
        .child(Spacer::mm(6.0))
        .child(Text::new("COME BACK SOON").center().bold())
        .child(Spacer::mm(10.0))
        .cut()
}

/// Get the framed receipt Program (for preview rendering).
fn framed_receipt_program() -> Program {
    const PAPER_WIDTH_MM: f32 = 72.0;
    const SPRITE_SIZE_DOTS: u16 = 32;
    const DOTS_PER_MM: f32 = 8.0;

    // Calculate actual content height needed
    // Top border + side borders + bottom border
    const NUM_VERTICAL_TILES: u16 = 8; // Number of vertical edge sprites per side (more room for content)
    const CONTENT_HEIGHT_DOTS: u16 = SPRITE_SIZE_DOTS // top
        + (NUM_VERTICAL_TILES * SPRITE_SIZE_DOTS) // sides
        + SPRITE_SIZE_DOTS; // bottom

    // Convert dimensions to 1/8mm units for page mode
    let region_width = (PAPER_WIDTH_MM * 8.0) as u16;
    let region_height = (CONTENT_HEIGHT_DOTS as f32 / DOTS_PER_MM * 8.0) as u16;

    // Helper: dots to mm
    let dots_to_mm = |dots: u16| -> f32 { dots as f32 / DOTS_PER_MM };
    let mm_to_eighth = |mm: f32| -> u16 { (mm * 8.0) as u16 };

    let mut program = Program::new();
    program.push(Op::Init);

    // Enter page mode
    program.push(Op::PageModeEnter);
    program.push(Op::PageModeSetRegion {
        x: 0,
        y: 0,
        width: region_width,
        height: region_height,
    });
    program.push(Op::PageModeSetDirection(0)); // Left-to-right, top-left origin

    // === TOP BORDER ===
    let y_top = 0;

    // Top-left corner (0, 0)
    program.push(Op::SetAbsolutePosition(0));
    program.push(Op::PageModeSetPositionY(mm_to_eighth(dots_to_mm(y_top))));
    program.push(Op::NvPrint {
        key: "C0".to_string(),
        scale_x: 1,
        scale_y: 1,
    });

    // Calculate right edge position
    let right_edge_dots = (PAPER_WIDTH_MM * DOTS_PER_MM) as u16 - SPRITE_SIZE_DOTS;

    // Top edge tiles
    let mut x = SPRITE_SIZE_DOTS;
    while x < right_edge_dots {
        program.push(Op::SetAbsolutePosition(x));
        program.push(Op::PageModeSetPositionY(mm_to_eighth(dots_to_mm(y_top))));
        program.push(Op::NvPrint {
            key: "C4".to_string(),
            scale_x: 1,
            scale_y: 1,
        });
        x += SPRITE_SIZE_DOTS;
    }

    // Top-right corner
    program.push(Op::SetAbsolutePosition(right_edge_dots));
    program.push(Op::PageModeSetPositionY(mm_to_eighth(dots_to_mm(y_top))));
    program.push(Op::NvPrint {
        key: "C1".to_string(),
        scale_x: 1,
        scale_y: 1,
    });

    // === SIDE BORDERS ===
    // Vertical edges on left and right sides
    let mut y = SPRITE_SIZE_DOTS; // Start below top corners
    let y_bottom_start = CONTENT_HEIGHT_DOTS - SPRITE_SIZE_DOTS; // Stop above bottom corners

    while y < y_bottom_start {
        // Left edge
        program.push(Op::SetAbsolutePosition(0));
        program.push(Op::PageModeSetPositionY(mm_to_eighth(dots_to_mm(y))));
        program.push(Op::NvPrint {
            key: "C5".to_string(),
            scale_x: 1,
            scale_y: 1,
        });

        // Right edge
        program.push(Op::SetAbsolutePosition(right_edge_dots));
        program.push(Op::PageModeSetPositionY(mm_to_eighth(dots_to_mm(y))));
        program.push(Op::NvPrint {
            key: "C5".to_string(),
            scale_x: 1,
            scale_y: 1,
        });

        y += SPRITE_SIZE_DOTS;
    }

    // === CONTENT ===
    // Position text slightly above center to ensure both lines fit
    // Font A at 2x scale = 48 dots tall per line
    const TEXT_LINE_HEIGHT: u16 = 48;
    let text_start_y_dots = (CONTENT_HEIGHT_DOTS / 2) - TEXT_LINE_HEIGHT;
    let text_start_y_mm = dots_to_mm(text_start_y_dots);

    program.push(Op::SetAbsolutePosition((PAPER_WIDTH_MM * DOTS_PER_MM / 2.0) as u16));
    program.push(Op::PageModeSetPositionY(mm_to_eighth(text_start_y_mm)));
    program.push(Op::SetAlign(crate::protocol::text::Alignment::Center));
    program.push(Op::SetBold(true));
    program.push(Op::SetSize { width: 2, height: 2 });
    program.push(Op::Text("PAGE MODE".to_string()));
    program.push(Op::Newline);
    program.push(Op::Text("SPRITES".to_string()));
    program.push(Op::SetSize { width: 1, height: 1 });
    program.push(Op::SetBold(false));

    // === BOTTOM BORDER ===
    let y_bottom_mm = dots_to_mm(CONTENT_HEIGHT_DOTS - SPRITE_SIZE_DOTS);

    // Bottom-left corner
    program.push(Op::SetAbsolutePosition(0));
    program.push(Op::PageModeSetPositionY(mm_to_eighth(y_bottom_mm)));
    program.push(Op::NvPrint {
        key: "C2".to_string(),
        scale_x: 1,
        scale_y: 1,
    });

    // Bottom edge tiles
    let mut x = SPRITE_SIZE_DOTS;
    while x < right_edge_dots {
        program.push(Op::SetAbsolutePosition(x));
        program.push(Op::PageModeSetPositionY(mm_to_eighth(y_bottom_mm)));
        program.push(Op::NvPrint {
            key: "C4".to_string(),
            scale_x: 1,
            scale_y: 1,
        });
        x += SPRITE_SIZE_DOTS;
    }

    // Bottom-right corner
    program.push(Op::SetAbsolutePosition(right_edge_dots));
    program.push(Op::PageModeSetPositionY(mm_to_eighth(y_bottom_mm)));
    program.push(Op::NvPrint {
        key: "C3".to_string(),
        scale_x: 1,
        scale_y: 1,
    });

    // Print and exit page mode
    program.push(Op::PageModePrintAndExit);

    program.push(Op::Feed { units: 24 });
    program.push(Op::Cut { partial: false });

    program
}

/// Framed receipt using page mode for proper sprite positioning.
///
/// This version uses page mode which allows true X,Y positioning
/// of sprites, so they can tile horizontally and vertically properly.
pub fn framed_receipt() -> Vec<u8> {
    framed_receipt_program().to_bytes()
}

/// Check if a name is a receipt template
pub fn is_receipt(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "receipt" | "receipt-full" | "receipt_full" | "receipt-frame" | "receipt_frame"
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

    #[test]
    fn test_demo_receipt_size() {
        let data = demo_receipt();
        // Component-based version should be ~803 bytes (optimized)
        assert!(
            data.len() <= 850,
            "demo_receipt should be optimized: {} bytes",
            data.len()
        );
    }

    #[test]
    fn test_full_receipt_size() {
        let data = full_receipt();
        // Component-based version should be ~1710 bytes (optimized)
        // Includes NvLogo print command (12 bytes) + spacer
        // Optimizer now removes redundant smoothing/style ops
        assert!(
            data.len() <= 1750,
            "full_receipt should be optimized: {} bytes",
            data.len()
        );
    }
}
