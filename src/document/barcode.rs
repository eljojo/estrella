//! Emit logic for barcode components: QrCode, Pdf417, Barcode.

use super::types::{Barcode, Pdf417, QrCode};
use super::EmitContext;
use crate::ir::{BarcodeKind, Op};
use crate::protocol::barcode::qr::QrErrorLevel;
use crate::protocol::text::Alignment;

impl QrCode {
    /// Emit IR ops for this QR code component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        // Resolve alignment (default: center)
        let alignment = match self.align.as_deref() {
            Some("left") => Alignment::Left,
            Some("right") => Alignment::Right,
            _ => Alignment::Center, // default
        };

        // Resolve error level (default: M)
        let error_level = match self
            .error_level
            .as_deref()
            .map(|s| s.to_uppercase())
            .as_deref()
        {
            Some("L") => QrErrorLevel::L,
            Some("Q") => QrErrorLevel::Q,
            Some("H") => QrErrorLevel::H,
            _ => QrErrorLevel::M, // default
        };

        let cell_size = self.cell_size.unwrap_or(4).clamp(1, 8);

        ctx.push(Op::SetAlign(alignment));
        ctx.push(Op::QrCode {
            data: self.data.clone(),
            cell_size,
            error_level,
        });
    }
}

impl Pdf417 {
    /// Emit IR ops for this PDF417 barcode component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        // Resolve alignment (default: center)
        let alignment = match self.align.as_deref() {
            Some("left") => Alignment::Left,
            Some("right") => Alignment::Right,
            _ => Alignment::Center, // default
        };

        let module_width = self.module_width.unwrap_or(3).clamp(1, 15);
        let ecc_level = self.ecc_level.unwrap_or(2).min(8);

        ctx.push(Op::SetAlign(alignment));
        ctx.push(Op::Pdf417 {
            data: self.data.clone(),
            module_width,
            ecc_level,
        });
    }
}

impl Barcode {
    /// Emit IR ops for this 1D barcode component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        let kind = match self.format.to_lowercase().as_str() {
            "code39" => BarcodeKind::Code39,
            "code128" => BarcodeKind::Code128,
            "ean13" => BarcodeKind::Ean13,
            "upca" => BarcodeKind::UpcA,
            "itf" => BarcodeKind::Itf,
            _ => return, // Unknown format — emit nothing
        };

        let height = self.height.unwrap_or(80).max(1);

        ctx.push(Op::Barcode1D {
            kind,
            data: self.data.clone(),
            height,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::EmitContext;

    fn ctx() -> EmitContext {
        EmitContext::new(576)
    }

    #[test]
    fn test_qr_code_default() {
        let qr = QrCode::new("https://example.com");
        let mut ctx = ctx();
        qr.emit(&mut ctx);
        assert!(ctx.ops.iter().any(|op| matches!(
            op,
            Op::QrCode {
                cell_size: 4,
                error_level: QrErrorLevel::M,
                ..
            }
        )));
        assert!(
            ctx.ops
                .iter()
                .any(|op| matches!(op, Op::SetAlign(Alignment::Center)))
        );
    }

    #[test]
    fn test_qr_code_options() {
        let qr = QrCode {
            data: "test".into(),
            cell_size: Some(6),
            error_level: Some("H".into()),
            align: Some("left".into()),
        };
        let mut ctx = ctx();
        qr.emit(&mut ctx);
        assert!(ctx.ops.iter().any(|op| matches!(
            op,
            Op::QrCode {
                cell_size: 6,
                error_level: QrErrorLevel::H,
                ..
            }
        )));
        assert!(
            ctx.ops
                .iter()
                .any(|op| matches!(op, Op::SetAlign(Alignment::Left)))
        );
    }

    #[test]
    fn test_pdf417() {
        let pdf = Pdf417 {
            data: "TICKET".into(),
            module_width: Some(4),
            ecc_level: Some(3),
            ..Default::default()
        };
        let mut ctx = ctx();
        pdf.emit(&mut ctx);
        assert!(ctx.ops.iter().any(|op| matches!(
            op,
            Op::Pdf417 {
                module_width: 4,
                ecc_level: 3,
                ..
            }
        )));
    }

    #[test]
    fn test_barcode_code128() {
        let barcode = Barcode {
            format: "code128".into(),
            data: "ABC-123".into(),
            height: Some(100),
        };
        let mut ctx = ctx();
        barcode.emit(&mut ctx);
        assert!(ctx.ops.iter().any(|op| matches!(
            op,
            Op::Barcode1D {
                kind: BarcodeKind::Code128,
                height: 100,
                ..
            }
        )));
    }

    #[test]
    fn test_barcode_invalid_format() {
        let barcode = Barcode {
            format: "invalid".into(),
            data: "123".into(),
            height: None,
        };
        let mut ctx = ctx();
        barcode.emit(&mut ctx);
        assert!(ctx.ops.is_empty());
    }
}
