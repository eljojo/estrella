//! # IR Opcodes
//!
//! This module defines the intermediate representation (IR) for receipt printing.
//! The IR is a sequence of opcodes that can be inspected, optimized, and compiled
//! to StarPRNT bytes.
//!
//! ## Design Philosophy
//!
//! The IR sits between declarative components and raw printer bytes:
//!
//! ```text
//! Components → IR (inspectable) → Optimizer → Codegen → Bytes
//! ```
//!
//! Each opcode represents a single, atomic operation. Style changes are
//! individual ops (not combined) to enable fine-grained optimization.

use crate::protocol::barcode::qr::QrErrorLevel;
use crate::protocol::text::{Alignment, Font};

/// Graphics rendering mode.
///
/// Controls how raster data is sent to the printer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphicsMode {
    /// Raster mode (ESC GS S) - arbitrary height, simpler.
    #[default]
    Raster,
    /// Band mode (ESC k) - 24-row chunks, more efficient for streaming.
    Band,
}

/// 1D barcode type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarcodeKind {
    Code39,
    Code128,
    Ean13,
    UpcA,
    Itf,
}

/// Style state tracked for optimization.
///
/// Represents the current text formatting state. Used by the optimizer
/// to eliminate redundant style changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleState {
    pub alignment: Alignment,
    pub font: Font,
    pub bold: bool,
    pub underline: bool,
    pub upperline: bool,
    pub invert: bool,
    pub smoothing: bool,
    pub upside_down: bool,
    pub reduced: bool,
    pub expanded_width: u8,
    pub expanded_height: u8,
    pub height_mult: u8,
    pub width_mult: u8,
}

impl Default for StyleState {
    fn default() -> Self {
        Self {
            alignment: Alignment::Left,
            font: Font::A,
            bold: false,
            underline: false,
            upperline: false,
            invert: false,
            smoothing: false,
            upside_down: false,
            reduced: false,
            expanded_width: 0,
            expanded_height: 0,
            height_mult: 0,
            width_mult: 0,
        }
    }
}

/// IR opcodes - the "bytecode" for receipt printing.
///
/// Each variant represents a single atomic operation. The IR can be:
/// - Inspected for debugging (`{:#?}`)
/// - Optimized to remove redundant operations
/// - Compiled to StarPRNT bytes
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    // ========== Printer Control ==========
    /// Initialize printer (ESC @). Resets to default state.
    Init,

    /// Cut paper. `partial: true` leaves a small hinge.
    Cut { partial: bool },

    /// Feed paper. Units are 1/4mm (4 units = 1mm).
    Feed { units: u8 },

    // ========== Style Changes ==========
    /// Set text alignment.
    SetAlign(Alignment),

    /// Set font (A, B, or C).
    SetFont(Font),

    /// Enable/disable bold.
    SetBold(bool),

    /// Enable/disable underline.
    SetUnderline(bool),

    /// Enable/disable inverted (white on black).
    SetInvert(bool),

    /// Set character size multiplier (ESC i n1 n2).
    /// height/width: 0 = 1x, 1 = 2x, ... 7 = 8x
    SetSize { height: u8, width: u8 },

    /// Set expanded width multiplier (ESC W n).
    /// 0 = 1x, 1 = 2x, ... 7 = 8x (spec dependent: may be limited to 5 or 7)
    SetExpandedWidth(u8),

    /// Set expanded height multiplier (ESC h n).
    /// 0 = 1x, 1 = 2x, ... 7 = 8x (spec dependent: may be limited to 5 or 7)
    SetExpandedHeight(u8),

    /// Enable/disable character smoothing.
    SetSmoothing(bool),

    /// Enable/disable upperline (line above text).
    SetUpperline(bool),

    /// Enable/disable upside-down printing.
    SetUpsideDown(bool),

    /// Enable/disable reduced (condensed) printing.
    SetReduced(bool),

    /// Set code page (character set).
    SetCodepage(u8),

    /// Reset all styles to default.
    ResetStyle,

    // ========== Content ==========
    /// Raw text (no trailing newline).
    Text(String),

    /// Line feed (newline).
    Newline,

    /// Raw bytes (for special characters or direct protocol access).
    Raw(Vec<u8>),

    // ========== Graphics ==========
    /// Raster graphics (ESC GS S). Arbitrary height.
    Raster {
        width: u16,
        height: u16,
        data: Vec<u8>,
    },

    /// Band graphics (ESC k). Fixed 24-row height.
    /// Data length must be `width_bytes * 24`.
    Band { width_bytes: u8, data: Vec<u8> },

    // ========== Barcodes ==========
    /// QR code.
    QrCode {
        data: String,
        cell_size: u8,
        error_level: QrErrorLevel,
    },

    /// PDF417 2D barcode.
    Pdf417 {
        data: String,
        module_width: u8,
        ecc_level: u8,
    },

    /// 1D barcode (Code39, Code128, etc).
    Barcode1D {
        kind: BarcodeKind,
        data: String,
        height: u8,
    },

    // ========== NV Graphics ==========
    /// Store image in printer's non-volatile memory.
    NvStore {
        key: String,
        width: u16,
        height: u16,
        data: Vec<u8>,
    },

    /// Print image from NV memory.
    NvPrint {
        key: String,
        scale_x: u8,
        scale_y: u8,
    },

    /// Delete image from NV memory.
    NvDelete { key: String },
}

/// A compiled IR program.
///
/// Contains a sequence of ops that can be optimized and compiled to bytes.
#[derive(Debug, Clone, Default)]
pub struct Program {
    pub ops: Vec<Op>,
}

impl Program {
    /// Create an empty program.
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// Create a program with an initial Init op.
    pub fn with_init() -> Self {
        Self {
            ops: vec![Op::Init],
        }
    }

    /// Add an op to the program.
    pub fn push(&mut self, op: Op) {
        self.ops.push(op);
    }

    /// Add multiple ops to the program.
    pub fn extend(&mut self, ops: impl IntoIterator<Item = Op>) {
        self.ops.extend(ops);
    }

    /// Get the number of ops in the program.
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Check if the program is empty.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Iterate over ops.
    pub fn iter(&self) -> impl Iterator<Item = &Op> {
        self.ops.iter()
    }
}

impl FromIterator<Op> for Program {
    fn from_iter<T: IntoIterator<Item = Op>>(iter: T) -> Self {
        Self {
            ops: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for Program {
    type Item = Op;
    type IntoIter = std::vec::IntoIter<Op>;

    fn into_iter(self) -> Self::IntoIter {
        self.ops.into_iter()
    }
}

impl<'a> IntoIterator for &'a Program {
    type Item = &'a Op;
    type IntoIter = std::slice::Iter<'a, Op>;

    fn into_iter(self) -> Self::IntoIter {
        self.ops.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_new() {
        let program = Program::new();
        assert!(program.is_empty());
    }

    #[test]
    fn test_program_with_init() {
        let program = Program::with_init();
        assert_eq!(program.len(), 1);
        assert_eq!(program.ops[0], Op::Init);
    }

    #[test]
    fn test_program_push() {
        let mut program = Program::new();
        program.push(Op::Init);
        program.push(Op::SetBold(true));
        program.push(Op::Text("Hello".into()));
        assert_eq!(program.len(), 3);
    }

    #[test]
    fn test_style_state_default() {
        let state = StyleState::default();
        assert_eq!(state.alignment, Alignment::Left);
        assert_eq!(state.font, Font::A);
        assert!(!state.bold);
        assert!(!state.underline);
        assert!(!state.invert);
        assert_eq!(state.height_mult, 0);
        assert_eq!(state.width_mult, 0);
    }

    #[test]
    fn test_graphics_mode_default() {
        let mode = GraphicsMode::default();
        assert_eq!(mode, GraphicsMode::Raster);
    }

    #[test]
    fn test_op_debug() {
        let op = Op::QrCode {
            data: "https://example.com".into(),
            cell_size: 4,
            error_level: QrErrorLevel::M,
        };
        let debug = format!("{:?}", op);
        assert!(debug.contains("QrCode"));
        assert!(debug.contains("example.com"));
    }
}
