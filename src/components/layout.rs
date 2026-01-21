//! # Layout Components
//!
//! Components for controlling layout and spacing.

use super::Component;
use crate::ir::Op;
use crate::protocol::text::Alignment;

/// Divider style options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividerStyle {
    /// Dashed line (default): - - - - - - -
    #[default]
    Dashed,
    /// Solid line: ────────────
    Solid,
    /// Double line: ════════════
    Double,
    /// Equals line: ============
    Equals,
}

/// A horizontal divider line.
///
/// ## Example
///
/// ```
/// use estrella::components::{Divider, DividerStyle};
///
/// let dashed = Divider::dashed();
/// let solid = Divider::solid();
/// let double = Divider::new(DividerStyle::Double);
/// ```
pub struct Divider {
    style: DividerStyle,
    width: usize,
}

impl Divider {
    /// Create a divider with a specific style.
    pub fn new(style: DividerStyle) -> Self {
        Self { style, width: 48 }
    }

    /// Create a dashed divider.
    pub fn dashed() -> Self {
        Self::new(DividerStyle::Dashed)
    }

    /// Create a solid divider.
    pub fn solid() -> Self {
        Self::new(DividerStyle::Solid)
    }

    /// Create a double-line divider.
    pub fn double() -> Self {
        Self::new(DividerStyle::Double)
    }

    /// Create an equals-sign divider.
    pub fn equals() -> Self {
        Self::new(DividerStyle::Equals)
    }

    /// Set the character width.
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::dashed()
    }
}

impl Component for Divider {
    fn emit(&self, ops: &mut Vec<Op>) {
        let line = match self.style {
            DividerStyle::Dashed => "-".repeat(self.width),
            DividerStyle::Solid => "\u{2500}".repeat(self.width), // ─
            DividerStyle::Double => "\u{2550}".repeat(self.width), // ═
            DividerStyle::Equals => "=".repeat(self.width),
        };
        ops.push(Op::SetAlign(Alignment::Left));
        ops.push(Op::Text(line));
        ops.push(Op::Newline);
    }
}

/// A vertical spacer (paper feed).
///
/// ## Example
///
/// ```
/// use estrella::components::Spacer;
///
/// let small = Spacer::mm(2.0);
/// let large = Spacer::mm(10.0);
/// let lines = Spacer::lines(3);
/// ```
pub struct Spacer {
    units: u8, // 1/4mm units
}

impl Spacer {
    /// Create a spacer with a specific height in millimeters.
    pub fn mm(mm: f32) -> Self {
        let units = (mm * 4.0).round().clamp(0.0, 255.0) as u8;
        Self { units }
    }

    /// Create a spacer that's approximately N lines tall.
    /// Assumes ~3mm per line (24 dots at 203 DPI).
    pub fn lines(n: u8) -> Self {
        Self::mm(n as f32 * 3.0)
    }

    /// Create a spacer with raw 1/4mm units.
    pub fn units(units: u8) -> Self {
        Self { units }
    }
}

impl Component for Spacer {
    fn emit(&self, ops: &mut Vec<Op>) {
        if self.units > 0 {
            ops.push(Op::Feed { units: self.units });
        }
    }
}

/// A two-column layout (left and right aligned text on same line).
///
/// ## Example
///
/// ```
/// use estrella::components::Columns;
///
/// let row = Columns::new("Label:", "Value");
/// let receipt_line = Columns::new("Subtotal", "$19.99");
/// let header = Columns::new("ITEM", "PRICE").bold();
/// ```
pub struct Columns {
    left: String,
    right: String,
    width: usize,
    bold: bool,
    underline: bool,
    invert: bool,
}

impl Columns {
    /// Create a two-column row.
    pub fn new(left: impl Into<String>, right: impl Into<String>) -> Self {
        Self {
            left: left.into(),
            right: right.into(),
            width: 48,
            bold: false,
            underline: false,
            invert: false,
        }
    }

    /// Set the character width.
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Make the row bold.
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Underline the row.
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Invert the row (white on black).
    pub fn invert(mut self) -> Self {
        self.invert = true;
        self
    }
}

impl Component for Columns {
    fn emit(&self, ops: &mut Vec<Op>) {
        let padding = self
            .width
            .saturating_sub(self.left.len() + self.right.len());
        let line = format!(
            "{}{}",
            self.left,
            format!("{:>width$}", self.right, width = padding + self.right.len())
        );

        ops.push(Op::SetAlign(Alignment::Left));
        if self.bold {
            ops.push(Op::SetBold(true));
        }
        if self.underline {
            ops.push(Op::SetUnderline(true));
        }
        if self.invert {
            ops.push(Op::SetInvert(true));
        }

        ops.push(Op::Text(line));
        ops.push(Op::Newline);

        if self.invert {
            ops.push(Op::SetInvert(false));
        }
        if self.underline {
            ops.push(Op::SetUnderline(false));
        }
        if self.bold {
            ops.push(Op::SetBold(false));
        }
    }
}

/// An empty line (just a newline character).
pub struct BlankLine;

impl BlankLine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BlankLine {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for BlankLine {
    fn emit(&self, ops: &mut Vec<Op>) {
        ops.push(Op::Newline);
    }
}

/// Raw bytes or ops - escape hatch for direct protocol access.
///
/// ## Example
///
/// ```
/// use estrella::components::Raw;
/// use estrella::ir::Op;
///
/// // From raw bytes
/// let raw = Raw::bytes(vec![0x1B, 0x40]);
///
/// // From ops
/// let raw = Raw::ops(vec![Op::Init, Op::SetBold(true)]);
/// ```
pub struct Raw {
    ops: Vec<Op>,
}

impl Raw {
    /// Create from raw bytes.
    pub fn bytes(data: Vec<u8>) -> Self {
        Self {
            ops: vec![Op::Raw(data)],
        }
    }

    /// Create from IR ops.
    pub fn ops(ops: Vec<Op>) -> Self {
        Self { ops }
    }

    /// Create a single op.
    pub fn op(op: Op) -> Self {
        Self { ops: vec![op] }
    }
}

impl Component for Raw {
    fn emit(&self, ops: &mut Vec<Op>) {
        ops.extend(self.ops.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::ComponentExt;

    #[test]
    fn test_dashed_divider() {
        let div = Divider::dashed().width(10);
        let ir = div.compile();
        assert!(ir.ops.iter().any(|op| *op == Op::Text("----------".into())));
    }

    #[test]
    fn test_equals_divider() {
        let div = Divider::equals().width(5);
        let ir = div.compile();
        assert!(ir.ops.iter().any(|op| *op == Op::Text("=====".into())));
    }

    #[test]
    fn test_spacer_mm() {
        let spacer = Spacer::mm(5.0);
        let ir = spacer.compile();
        // 5mm = 20 units (5 * 4)
        assert!(ir.ops.iter().any(|op| *op == Op::Feed { units: 20 }));
    }

    #[test]
    fn test_spacer_lines() {
        let spacer = Spacer::lines(2);
        let ir = spacer.compile();
        // 2 lines ≈ 6mm = 24 units
        assert!(ir.ops.iter().any(|op| *op == Op::Feed { units: 24 }));
    }

    #[test]
    fn test_columns() {
        let cols = Columns::new("Left", "Right").width(20);
        let ir = cols.compile();
        // Should have formatted text
        let has_columns = ir.ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.starts_with("Left") && s.ends_with("Right") && s.len() == 20
            } else {
                false
            }
        });
        assert!(has_columns);
    }

    #[test]
    fn test_columns_bold() {
        let cols = Columns::new("ITEM", "PRICE").bold();
        let ir = cols.compile();
        assert!(ir.ops.contains(&Op::SetBold(true)));
        assert!(ir.ops.contains(&Op::SetBold(false)));
    }

    #[test]
    fn test_columns_underline() {
        let cols = Columns::new("A", "B").underline();
        let ir = cols.compile();
        assert!(ir.ops.contains(&Op::SetUnderline(true)));
        assert!(ir.ops.contains(&Op::SetUnderline(false)));
    }

    #[test]
    fn test_columns_invert() {
        let cols = Columns::new("A", "B").invert();
        let ir = cols.compile();
        assert!(ir.ops.contains(&Op::SetInvert(true)));
        assert!(ir.ops.contains(&Op::SetInvert(false)));
    }

    #[test]
    fn test_blank_line() {
        let blank = BlankLine::new();
        let ir = blank.compile();
        assert!(ir.ops.iter().any(|op| *op == Op::Newline));
    }
}
