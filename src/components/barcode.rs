//! # Barcode Components
//!
//! Components for rendering QR codes, PDF417, and 1D barcodes.

use super::Component;
use crate::ir::{BarcodeKind, Op};
use crate::protocol::barcode::qr::QrErrorLevel;
use crate::protocol::text::Alignment;

/// A QR code component.
///
/// ## Example
///
/// ```
/// use estrella::components::QrCode;
///
/// // Simple QR code
/// let qr = QrCode::new("https://example.com");
///
/// // With options
/// let qr = QrCode::new("https://example.com")
///     .cell_size(6)
///     .error_level_high();
/// ```
pub struct QrCode {
    data: String,
    cell_size: u8,
    error_level: QrErrorLevel,
    alignment: Alignment,
}

impl QrCode {
    /// Create a new QR code with the given data.
    /// Defaults to center alignment.
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            cell_size: 4,
            error_level: QrErrorLevel::M,
            alignment: Alignment::Center,
        }
    }

    /// Set the cell (module) size in dots (1-8).
    pub fn cell_size(mut self, size: u8) -> Self {
        self.cell_size = size.clamp(1, 8);
        self
    }

    /// Set error correction level L (~7% recovery).
    pub fn error_level_low(mut self) -> Self {
        self.error_level = QrErrorLevel::L;
        self
    }

    /// Set error correction level M (~15% recovery) - default.
    pub fn error_level_medium(mut self) -> Self {
        self.error_level = QrErrorLevel::M;
        self
    }

    /// Set error correction level Q (~25% recovery).
    pub fn error_level_quartile(mut self) -> Self {
        self.error_level = QrErrorLevel::Q;
        self
    }

    /// Set error correction level H (~30% recovery).
    pub fn error_level_high(mut self) -> Self {
        self.error_level = QrErrorLevel::H;
        self
    }

    /// Center the QR code (default).
    pub fn center(mut self) -> Self {
        self.alignment = Alignment::Center;
        self
    }

    /// Left-align the QR code.
    pub fn left(mut self) -> Self {
        self.alignment = Alignment::Left;
        self
    }

    /// Right-align the QR code.
    pub fn right(mut self) -> Self {
        self.alignment = Alignment::Right;
        self
    }
}

impl Component for QrCode {
    fn emit(&self, ops: &mut Vec<Op>) {
        ops.push(Op::SetAlign(self.alignment));
        ops.push(Op::QrCode {
            data: self.data.clone(),
            cell_size: self.cell_size,
            error_level: self.error_level,
        });
    }
}

/// A PDF417 2D barcode component.
///
/// ## Example
///
/// ```
/// use estrella::components::Pdf417;
///
/// let barcode = Pdf417::new("Hello, PDF417!")
///     .module_width(3)
///     .ecc_level(2);
/// ```
pub struct Pdf417 {
    data: String,
    module_width: u8,
    ecc_level: u8,
    alignment: Alignment,
}

impl Pdf417 {
    /// Create a new PDF417 barcode.
    /// Defaults to center alignment.
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            module_width: 3,
            ecc_level: 2,
            alignment: Alignment::Center,
        }
    }

    /// Set the module width in dots (1-15).
    pub fn module_width(mut self, width: u8) -> Self {
        self.module_width = width.clamp(1, 15);
        self
    }

    /// Set the error correction level (0-8).
    pub fn ecc_level(mut self, level: u8) -> Self {
        self.ecc_level = level.min(8);
        self
    }

    /// Center the PDF417 barcode (default).
    pub fn center(mut self) -> Self {
        self.alignment = Alignment::Center;
        self
    }

    /// Left-align the PDF417 barcode.
    pub fn left(mut self) -> Self {
        self.alignment = Alignment::Left;
        self
    }

    /// Right-align the PDF417 barcode.
    pub fn right(mut self) -> Self {
        self.alignment = Alignment::Right;
        self
    }
}

impl Component for Pdf417 {
    fn emit(&self, ops: &mut Vec<Op>) {
        ops.push(Op::SetAlign(self.alignment));
        ops.push(Op::Pdf417 {
            data: self.data.clone(),
            module_width: self.module_width,
            ecc_level: self.ecc_level,
        });
    }
}

/// A 1D barcode component.
///
/// ## Example
///
/// ```
/// use estrella::components::Barcode;
///
/// // Code39 barcode
/// let barcode = Barcode::code39("HELLO-123");
///
/// // Code128 with custom height
/// let barcode = Barcode::code128("Hello World").height(100);
/// ```
pub struct Barcode {
    data: String,
    kind: BarcodeKind,
    height: u8,
}

impl Barcode {
    /// Create a Code39 barcode.
    ///
    /// Code39 supports: A-Z, 0-9, space, - . $ / % +
    pub fn code39(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            kind: BarcodeKind::Code39,
            height: 80,
        }
    }

    /// Create a Code128 barcode.
    ///
    /// Code128 supports full ASCII.
    pub fn code128(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            kind: BarcodeKind::Code128,
            height: 80,
        }
    }

    /// Create an EAN-13 barcode.
    ///
    /// EAN-13 requires 12-13 digits.
    pub fn ean13(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            kind: BarcodeKind::Ean13,
            height: 80,
        }
    }

    /// Create a UPC-A barcode.
    ///
    /// UPC-A requires 11-12 digits.
    pub fn upca(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            kind: BarcodeKind::UpcA,
            height: 80,
        }
    }

    /// Create an ITF (Interleaved 2 of 5) barcode.
    ///
    /// ITF requires an even number of digits.
    pub fn itf(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            kind: BarcodeKind::Itf,
            height: 80,
        }
    }

    /// Set the barcode height in dots (1-255).
    pub fn height(mut self, height: u8) -> Self {
        self.height = height.max(1);
        self
    }
}

impl Component for Barcode {
    fn emit(&self, ops: &mut Vec<Op>) {
        ops.push(Op::Barcode1D {
            kind: self.kind,
            data: self.data.clone(),
            height: self.height,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::ComponentExt;

    #[test]
    fn test_qr_code() {
        let qr = QrCode::new("https://example.com");
        let ir = qr.compile();

        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::QrCode {
                cell_size: 4,
                error_level: QrErrorLevel::M,
                ..
            }
        )));
    }

    #[test]
    fn test_qr_code_with_options() {
        let qr = QrCode::new("test").cell_size(6).error_level_high();
        let ir = qr.compile();

        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::QrCode {
                cell_size: 6,
                error_level: QrErrorLevel::H,
                ..
            }
        )));
    }

    #[test]
    fn test_pdf417() {
        let pdf = Pdf417::new("Hello").module_width(4).ecc_level(3);
        let ir = pdf.compile();

        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::Pdf417 {
                module_width: 4,
                ecc_level: 3,
                ..
            }
        )));
    }

    #[test]
    fn test_barcode_code39() {
        let barcode = Barcode::code39("TEST-123").height(100);
        let ir = barcode.compile();

        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::Barcode1D {
                kind: BarcodeKind::Code39,
                height: 100,
                ..
            }
        )));
    }

    #[test]
    fn test_barcode_code128() {
        let barcode = Barcode::code128("Hello World");
        let ir = barcode.compile();

        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::Barcode1D {
                kind: BarcodeKind::Code128,
                ..
            }
        )));
    }

    #[test]
    fn test_barcode_ean13() {
        let barcode = Barcode::ean13("5901234123457");
        let ir = barcode.compile();

        assert!(ir.ops.iter().any(|op| matches!(
            op,
            Op::Barcode1D {
                kind: BarcodeKind::Ean13,
                ..
            }
        )));
    }
}
