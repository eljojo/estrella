//! # Text Components
//!
//! Components for displaying text with various styles.

use super::Component;
use crate::ir::Op;
use crate::protocol::text::{Alignment, Font};

/// A text component with optional styling.
///
/// ## Example
///
/// ```
/// use estrella::components::*;
///
/// // Simple text
/// let text = Text::new("Hello, World!");
///
/// // Styled text
/// let styled = Text::new("IMPORTANT")
///     .bold()
///     .center()
///     .size(2, 2);
/// ```
pub struct Text {
    content: String,
    newline: bool,
    bold: bool,
    underline: bool,
    upperline: bool,
    invert: bool,
    smoothing: Option<bool>,
    upside_down: bool,
    reduced: bool,
    scaled_width: u8,
    scaled_height: u8,
    font: Option<Font>,
    alignment: Option<Alignment>,
    height_mult: u8,
    width_mult: u8,
}

impl Text {
    /// Create a new text component.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            newline: true,
            bold: false,
            underline: false,
            upperline: false,
            invert: false,
            smoothing: None,
            upside_down: false,
            reduced: false,
            scaled_width: 0,
            scaled_height: 0,
            font: None,
            alignment: None,
            height_mult: 0,
            width_mult: 0,
        }
    }

    /// Create inline text (no trailing newline).
    pub fn inline(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            newline: false,
            bold: false,
            underline: false,
            upperline: false,
            invert: false,
            smoothing: None,
            upside_down: false,
            reduced: false,
            scaled_width: 0,
            scaled_height: 0,
            font: None,
            alignment: None,
            height_mult: 0,
            width_mult: 0,
        }
    }

    /// Add a newline after the text.
    pub fn line(mut self) -> Self {
        self.newline = true;
        self
    }

    /// Make text bold.
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Underline the text.
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Add upperline (line above text).
    pub fn upperline(mut self) -> Self {
        self.upperline = true;
        self
    }

    /// Invert the text (white on black).
    pub fn invert(mut self) -> Self {
        self.invert = true;
        self
    }

    /// Enable character smoothing.
    pub fn smoothing(mut self) -> Self {
        self.smoothing = Some(true);
        self
    }

    /// Disable character smoothing.
    pub fn no_smoothing(mut self) -> Self {
        self.smoothing = Some(false);
        self
    }

    /// Print text upside-down.
    pub fn upside_down(mut self) -> Self {
        self.upside_down = true;
        self
    }

    /// Print reduced (condensed) text.
    pub fn reduced(mut self) -> Self {
        self.reduced = true;
        self
    }

    /// Double the width only (uses ESC W command).
    pub fn double_width(mut self) -> Self {
        self.scaled_width = 1; // 0=1x, 1=2x
        self
    }

    /// Double the height only (uses ESC h command).
    pub fn double_height(mut self) -> Self {
        self.scaled_height = 1; // 0=1x, 1=2x
        self
    }

    /// Set character scale using ESC W (width) and ESC h (height).
    /// This uses separate width/height commands and may render differently than size().
    ///
    /// ## Parameters
    /// - height: 0=1x, 1=2x, 2=3x, ... 7=8x
    /// - width: 0=1x, 1=2x, 2=3x, ... 7=8x
    ///
    /// ## Example
    /// ```
    /// use estrella::components::Text;
    ///
    /// let big = Text::new("BIG").scale(1, 1);  // 2x2 using ESC W + ESC h
    /// let huge = Text::new("HUGE").scale(3, 3);  // 4x4
    /// ```
    pub fn scale(mut self, height: u8, width: u8) -> Self {
        self.scaled_height = height.min(7);
        self.scaled_width = width.min(7);
        self
    }

    /// Set the font.
    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Use Font A (12x24).
    pub fn font_a(mut self) -> Self {
        self.font = Some(Font::A);
        self
    }

    /// Use Font B (9x24).
    pub fn font_b(mut self) -> Self {
        self.font = Some(Font::B);
        self
    }

    /// Use Font C (9x17).
    pub fn font_c(mut self) -> Self {
        self.font = Some(Font::C);
        self
    }

    /// Center the text.
    pub fn center(mut self) -> Self {
        self.alignment = Some(Alignment::Center);
        self
    }

    /// Right-align the text.
    pub fn right(mut self) -> Self {
        self.alignment = Some(Alignment::Right);
        self
    }

    /// Left-align the text (explicit).
    pub fn left(mut self) -> Self {
        self.alignment = Some(Alignment::Left);
        self
    }

    /// Set character size multiplier.
    /// height/width: 0 = 1x, 1 = 2x, etc. Max 7 = 8x.
    pub fn size(mut self, height: u8, width: u8) -> Self {
        self.height_mult = height.min(7);
        self.width_mult = width.min(7);
        self
    }
}

impl Component for Text {
    fn emit(&self, ops: &mut Vec<Op>) {
        // Emit style changes (order matters for compatibility)
        if let Some(align) = self.alignment {
            ops.push(Op::SetAlign(align));
        }
        if let Some(font) = self.font {
            ops.push(Op::SetFont(font));
        }
        if let Some(smoothing) = self.smoothing {
            ops.push(Op::SetSmoothing(smoothing));
        }
        if self.bold {
            ops.push(Op::SetBold(true));
        }
        if self.underline {
            ops.push(Op::SetUnderline(true));
        }
        if self.upperline {
            ops.push(Op::SetUpperline(true));
        }
        if self.invert {
            ops.push(Op::SetInvert(true));
        }
        if self.upside_down {
            ops.push(Op::SetUpsideDown(true));
        }
        if self.reduced {
            ops.push(Op::SetReduced(true));
        }
        if self.scaled_width > 0 {
            ops.push(Op::SetExpandedWidth(self.scaled_width));
        }
        if self.scaled_height > 0 {
            ops.push(Op::SetExpandedHeight(self.scaled_height));
        }
        if self.height_mult > 0 || self.width_mult > 0 {
            ops.push(Op::SetSize {
                height: self.height_mult,
                width: self.width_mult,
            });
        }

        // Emit text
        ops.push(Op::Text(self.content.clone()));
        if self.newline {
            ops.push(Op::Newline);
        }

        // Reset styles that were changed (reverse order)
        if self.height_mult > 0 || self.width_mult > 0 {
            ops.push(Op::SetSize {
                height: 0,
                width: 0,
            });
        }
        if self.scaled_height > 0 {
            ops.push(Op::SetExpandedHeight(0));
        }
        if self.scaled_width > 0 {
            ops.push(Op::SetExpandedWidth(0));
        }
        if self.reduced {
            ops.push(Op::SetReduced(false));
        }
        if self.upside_down {
            ops.push(Op::SetUpsideDown(false));
        }
        if self.invert {
            ops.push(Op::SetInvert(false));
        }
        if self.upperline {
            ops.push(Op::SetUpperline(false));
        }
        if self.underline {
            ops.push(Op::SetUnderline(false));
        }
        if self.bold {
            ops.push(Op::SetBold(false));
        }
        // Note: alignment, font, smoothing are NOT reset - they persist
    }
}

/// A header component (centered, bold, large text).
///
/// ## Example
///
/// ```
/// use estrella::components::Header;
///
/// let header = Header::new("CHURRA MART");
/// ```
pub struct Header {
    text: Text,
}

impl Header {
    /// Create a new header.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            text: Text::new(content).center().bold().size(1, 1),
        }
    }

    /// Create a smaller header (normal size, still bold and centered).
    pub fn small(content: impl Into<String>) -> Self {
        Self {
            text: Text::new(content).center().bold(),
        }
    }
}

impl Component for Header {
    fn emit(&self, ops: &mut Vec<Op>) {
        self.text.emit(ops);
    }
}

/// A line item component (name on left, price on right).
///
/// ## Example
///
/// ```
/// use estrella::components::LineItem;
///
/// let item = LineItem::new("Espresso", 4.50);
/// ```
pub struct LineItem {
    name: String,
    price: f64,
    width: usize, // Character width for formatting
}

impl LineItem {
    /// Create a new line item.
    pub fn new(name: impl Into<String>, price: f64) -> Self {
        Self {
            name: name.into(),
            price,
            width: 48, // Default for Font A at 72mm
        }
    }

    /// Set the character width for formatting.
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }
}

impl Component for LineItem {
    fn emit(&self, ops: &mut Vec<Op>) {
        let price_str = format!("{:.2}", self.price);
        let name_max_width = self.width.saturating_sub(price_str.len() + 1);
        let name = if self.name.len() > name_max_width {
            &self.name[..name_max_width]
        } else {
            &self.name
        };
        let padding = self.width.saturating_sub(name.len() + price_str.len());
        let line = format!(
            "{}{:>pad$}",
            name,
            price_str,
            pad = padding + price_str.len()
        );

        ops.push(Op::SetAlign(Alignment::Left));
        ops.push(Op::Text(line));
        ops.push(Op::Newline);
    }
}

/// A total component (label and amount, typically right-aligned).
///
/// ## Example
///
/// ```
/// use estrella::components::Total;
///
/// let total = Total::new(19.99);
/// let custom = Total::labeled("SUBTOTAL:", 15.99);
/// ```
pub struct Total {
    label: String,
    amount: f64,
    bold: bool,
    scaled_width: u8,
    right_align: bool,
}

impl Total {
    /// Create a new total with default "TOTAL:" label.
    pub fn new(amount: f64) -> Self {
        Self {
            label: "TOTAL:".into(),
            amount,
            bold: true,
            scaled_width: 0,
            right_align: true,
        }
    }

    /// Create a total with a custom label.
    pub fn labeled(label: impl Into<String>, amount: f64) -> Self {
        Self {
            label: label.into(),
            amount,
            bold: false,
            scaled_width: 0,
            right_align: true,
        }
    }

    /// Set whether the total is bold (default: true for Total::new).
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Disable bold.
    pub fn not_bold(mut self) -> Self {
        self.bold = false;
        self
    }

    /// Enable double width for emphasis.
    pub fn double_width(mut self) -> Self {
        self.scaled_width = 1; // 0=1x, 1=2x
        self
    }

    /// Use left alignment instead of right.
    pub fn left(mut self) -> Self {
        self.right_align = false;
        self
    }
}

impl Component for Total {
    fn emit(&self, ops: &mut Vec<Op>) {
        // Format: "LABEL:  VALUE" with two spaces between
        let amount_str = format!("{:.2}", self.amount);
        let line = format!("{}  {}", self.label, amount_str);

        if self.right_align {
            ops.push(Op::SetAlign(Alignment::Right));
        }
        if self.bold {
            ops.push(Op::SetBold(true));
        }
        if self.scaled_width > 0 {
            ops.push(Op::SetExpandedWidth(self.scaled_width));
        }
        ops.push(Op::Text(line));
        ops.push(Op::Newline);
        if self.scaled_width > 0 {
            ops.push(Op::SetExpandedWidth(0));
        }
        if self.bold {
            ops.push(Op::SetBold(false));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::ComponentExt;

    #[test]
    fn test_simple_text() {
        let text = Text::new("Hello");
        let ir = text.compile();
        assert!(ir.ops.iter().any(|op| *op == Op::Text("Hello".into())));
        assert!(ir.ops.iter().any(|op| *op == Op::Newline));
    }

    #[test]
    fn test_inline_text() {
        let text = Text::inline("Hello");
        let ir = text.compile();
        assert!(ir.ops.iter().any(|op| *op == Op::Text("Hello".into())));
        assert!(!ir.ops.iter().any(|op| *op == Op::Newline));
    }

    #[test]
    fn test_bold_text() {
        let text = Text::new("Bold").bold();
        let ir = text.compile();
        // Should have SetBold(true) before text and SetBold(false) after
        let ops: Vec<_> = ir.ops.iter().collect();
        let bold_on_pos = ops.iter().position(|op| **op == Op::SetBold(true));
        let text_pos = ops.iter().position(|op| **op == Op::Text("Bold".into()));
        let bold_off_pos = ops.iter().position(|op| **op == Op::SetBold(false));

        assert!(bold_on_pos.is_some());
        assert!(text_pos.is_some());
        assert!(bold_off_pos.is_some());
        assert!(bold_on_pos.unwrap() < text_pos.unwrap());
        assert!(text_pos.unwrap() < bold_off_pos.unwrap());
    }

    #[test]
    fn test_centered_text() {
        let text = Text::new("Centered").center();
        let ir = text.compile();
        assert!(
            ir.ops
                .iter()
                .any(|op| *op == Op::SetAlign(Alignment::Center))
        );
    }

    #[test]
    fn test_header() {
        let header = Header::new("STORE");
        let ir = header.compile();
        // Should be centered, bold, and have size
        assert!(
            ir.ops
                .iter()
                .any(|op| *op == Op::SetAlign(Alignment::Center))
        );
        assert!(ir.ops.iter().any(|op| *op == Op::SetBold(true)));
        assert!(ir.ops.iter().any(|op| *op
            == Op::SetSize {
                height: 1,
                width: 1
            }));
    }

    #[test]
    fn test_line_item() {
        let item = LineItem::new("Coffee", 4.50);
        let ir = item.compile();
        // Should contain text with both name and price
        let has_formatted_line = ir.ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.contains("Coffee") && s.contains("4.50")
            } else {
                false
            }
        });
        assert!(has_formatted_line);
    }

    #[test]
    fn test_total() {
        let total = Total::new(19.99);
        let ir = total.compile();
        // Should contain TOTAL: and amount, and be bold, right-aligned
        let has_total_line = ir.ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.contains("TOTAL:") && s.contains("19.99")
            } else {
                false
            }
        });
        assert!(has_total_line);
        assert!(ir.ops.iter().any(|op| *op == Op::SetBold(true)));
        assert!(
            ir.ops
                .iter()
                .any(|op| *op == Op::SetAlign(Alignment::Right))
        );
    }
}
