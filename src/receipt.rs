//! # Receipt Builders
//!
//! Pre-built receipt templates demonstrating StarPRNT text capabilities.
//!
//! These generate command sequences that can be sent directly to the printer.
//! Receipts are built using the declarative component system and optimized
//! for minimal byte output.

use crate::components::{
    Barcode, Columns, ComponentExt, Divider, LineItem, Markdown, NvLogo, Pdf417, QrCode, Raw,
    Receipt, Spacer, Text, Total,
};
use crate::ir::Op;
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

/// Generate a demo receipt using Markdown syntax.
///
/// Features demonstrated:
/// - Markdown headers (H1, H2, H3)
/// - Bold, italic (rendered as underline), inline code
/// - Unordered lists
/// - Ordered lists
/// - Links
/// - Horizontal rules
/// - Paragraphs and spacing
pub fn markdown_demo() -> Vec<u8> {
    markdown_demo_component().build()
}

// ============================================================================
// LOOKUP FUNCTIONS
// ============================================================================

/// List available receipt templates
pub fn list_receipts() -> &'static [&'static str] {
    &["receipt", "receipt-full", "markdown"]
}

/// Get receipt data by name
pub fn by_name(name: &str) -> Option<Vec<u8>> {
    match name.to_lowercase().as_str() {
        "receipt" => Some(demo_receipt()),
        "receipt-full" | "receipt_full" => Some(full_receipt()),
        "markdown" => Some(markdown_demo()),
        _ => None,
    }
}

/// Get receipt IR Program by name (for preview rendering).
pub fn program_by_name(name: &str) -> Option<crate::ir::Program> {
    use crate::components::ComponentExt;

    match name.to_lowercase().as_str() {
        "receipt" => Some(demo_receipt_component().compile()),
        "receipt-full" | "receipt_full" => Some(full_receipt_component().compile()),
        "markdown" => Some(markdown_demo_component().compile()),
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

/// Get the markdown demo component (for preview rendering).
fn markdown_demo_component() -> Receipt {
    Receipt::new()
        .child(Markdown::new(
            r#"# Coffee Shop

Date: 2026-01-20

Order: 1234

## Items

- Espresso ($3.50)
- Croissant ($4.00)
- Oat milk upgrade ($0.50)

### Payment

1. Subtotal: $8.00
2. Tax (13%): $1.04
3. **Total: $9.04**

Thank *you* for your purchase!

Visit us at `coffeeshop.example`
"#,
        ))
        .cut()
}

/// Check if a name is a receipt template
pub fn is_receipt(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "receipt" | "receipt-full" | "receipt_full" | "markdown"
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
