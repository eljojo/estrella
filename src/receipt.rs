//! # Receipt Builders
//!
//! Pre-built receipt templates demonstrating StarPRNT text capabilities.
//!
//! These generate command sequences that can be sent directly to the printer.
//! Receipts are built using the unified Document model and optimized
//! for minimal byte output.
//!
//! ## Date Handling
//!
//! - `demo_receipt()`, `full_receipt()`, etc. use the **current date** for live printing
//! - `demo_receipt_golden()`, etc. use a **fixed date** for reproducible golden tests

use chrono::Local;

use crate::document::{Component, Document, Markdown};

/// Fixed date used for golden tests (ensures reproducible output)
pub const GOLDEN_TEST_DATE: &str = "2026-01-20";
pub const GOLDEN_TEST_DATETIME: &str = "2026-01-20 12:00:00";

/// Get the current date as a string (YYYY-MM-DD format)
pub fn current_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Get the current datetime as a string (YYYY-MM-DD HH:MM:SS format)
pub fn current_datetime() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

// ============================================================================
// RECEIPT TEMPLATES
// ============================================================================

/// Generate a simple demo receipt with the current date/time.
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
    demo_receipt_doc(&current_datetime()).build()
}

/// Generate a simple demo receipt with a fixed date (for golden tests).
pub fn demo_receipt_golden() -> Vec<u8> {
    demo_receipt_doc(GOLDEN_TEST_DATETIME).build()
}

/// JSON fixture for the demo receipt.
const RECEIPT_JSON: &str = include_str!("fixtures/receipt.json");

/// JSON fixture for the full receipt.
const RECEIPT_FULL_JSON: &str = include_str!("fixtures/receipt-full.json");

/// Load a receipt Document from a JSON fixture, injecting the datetime variable.
fn load_fixture(json: &str, datetime: &str) -> Document {
    let mut doc: Document =
        serde_json::from_str(json).expect("Invalid receipt fixture JSON");
    doc.variables
        .insert("datetime".to_string(), datetime.to_string());
    doc
}

/// Build a simple demo receipt Document with a specific datetime string.
fn demo_receipt_doc(datetime: &str) -> Document {
    load_fixture(RECEIPT_JSON, datetime)
}

/// Generate a full demo receipt with barcodes, using the current date/time.
///
/// Features demonstrated:
/// - Everything from demo_receipt()
/// - NV logo (if stored with key "A1")
/// - Font selection (A, B, C)
/// - Code39 barcode
/// - QR code
/// - PDF417 barcode
pub fn full_receipt() -> Vec<u8> {
    full_receipt_doc(&current_datetime()).build()
}

/// Generate a full demo receipt with a fixed date (for golden tests).
pub fn full_receipt_golden() -> Vec<u8> {
    full_receipt_doc(GOLDEN_TEST_DATETIME).build()
}

/// Build a full demo receipt Document with a specific datetime string.
fn full_receipt_doc(datetime: &str) -> Document {
    load_fixture(RECEIPT_FULL_JSON, datetime)
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
    markdown_demo_doc(&current_date()).build()
}

/// Generate a markdown demo receipt with a fixed date (for golden tests).
pub fn markdown_demo_golden() -> Vec<u8> {
    markdown_demo_doc(GOLDEN_TEST_DATE).build()
}

/// Build a markdown demo Document with a specific date.
fn markdown_demo_doc(date: &str) -> Document {
    let content = format!(
        r#"# Markdown *Kitchen* Sink

## All **Heading** Levels

### H3 with `inline code`

#### H4 *italic* heading

##### H5 heading

###### H6 tiny heading

---

## Text Formatting

This is **bold text** and this is *italic text*.

You can combine **bold and *italic* together**.

Here's some `inline code` in a sentence.

Visit [our website](https://example.com) for more.

---

## Lists

### Unordered

- First item
- Second with **bold**
- Third with *emphasis*
  - Nested item
  - Another nested

### Ordered

1. Step one
2. Step **two**
3. Step three
   1. Sub-step A
   2. Sub-step B

---

## Receipt Example

Date: {} | Order: #1234

### Items

- Espresso ($3.50)
- Croissant ($4.00)
- Oat milk (+$0.50)

**Subtotal**: $8.00
**Tax (13%)**: $1.04

### Total: $9.04

---

Thank *you* for your purchase!
"#,
        date
    );
    Document {
        document: vec![Component::Markdown(Markdown::new(&content))],
        cut: true,
        interpolate: false,
        ..Default::default()
    }
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

/// Get receipt IR Program by name (uses current date for live preview).
pub fn program_by_name(name: &str) -> Option<crate::ir::Program> {
    match name.to_lowercase().as_str() {
        "receipt" => Some(demo_receipt_doc(&current_datetime()).compile()),
        "receipt-full" | "receipt_full" => {
            Some(full_receipt_doc(&current_datetime()).compile())
        }
        "markdown" => Some(markdown_demo_doc(&current_date()).compile()),
        _ => None,
    }
}

/// Get receipt IR Program by name with fixed date (for golden tests).
pub fn program_by_name_golden(name: &str) -> Option<crate::ir::Program> {
    match name.to_lowercase().as_str() {
        "receipt" => Some(demo_receipt_doc(GOLDEN_TEST_DATETIME).compile()),
        "receipt-full" | "receipt_full" => {
            Some(full_receipt_doc(GOLDEN_TEST_DATETIME).compile())
        }
        "markdown" => Some(markdown_demo_doc(GOLDEN_TEST_DATE).compile()),
        _ => None,
    }
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
        // Document-based version should be similarly sized to old component version
        assert!(
            data.len() <= 1000,
            "demo_receipt should be optimized: {} bytes",
            data.len()
        );
    }

    #[test]
    fn test_full_receipt_size() {
        let data = full_receipt();
        // Document-based version should be similarly sized
        // (slightly smaller since SetCodepage(1) was dropped)
        assert!(
            data.len() <= 1900,
            "full_receipt should be optimized: {} bytes",
            data.len()
        );
    }
}
