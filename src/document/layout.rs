//! Emit logic for layout components: Divider, Spacer, BlankLine, Columns, Banner.

use super::types::{Banner, BlankLine, BorderStyle, Columns, Divider, DividerStyle, Spacer};
use crate::ir::Op;
use crate::protocol::text::{Alignment, Font};

impl Divider {
    /// Emit IR ops for this divider component.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        let width = self.width.unwrap_or(48);
        let line = match self.style {
            DividerStyle::Dashed => "-".repeat(width),
            DividerStyle::Solid => "\u{2500}".repeat(width),   // ─
            DividerStyle::Double => "\u{2550}".repeat(width),  // ═
            DividerStyle::Equals => "=".repeat(width),
        };
        // Reset to Font A to ensure correct width (48 chars × 12 dots = 576 = full print width)
        ops.push(Op::SetFont(Font::A));
        ops.push(Op::SetAlign(Alignment::Left));
        ops.push(Op::Text(line));
        ops.push(Op::Newline);
    }
}

impl Spacer {
    /// Emit IR ops for this spacer component.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        let units = if let Some(mm) = self.mm {
            (mm * 4.0).round().clamp(0.0, 255.0) as u8
        } else if let Some(lines) = self.lines {
            (lines as f32 * 3.0 * 4.0).round().clamp(0.0, 255.0) as u8
        } else if let Some(units) = self.units {
            units
        } else {
            0
        };

        if units > 0 {
            ops.push(Op::Feed { units });
        }
    }
}

impl BlankLine {
    /// Emit IR ops for this blank line component.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        ops.push(Op::Newline);
    }
}

impl Columns {
    /// Emit IR ops for this two-column layout component.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        let width = self.width.unwrap_or(48);
        let padding = width.saturating_sub(self.left.len() + self.right.len());
        let line = format!(
            "{}{}",
            self.left,
            format!("{:>width$}", self.right, width = padding + self.right.len())
        );

        // Reset to Font A to ensure correct width (48 chars × 12 dots = 576 = full print width)
        ops.push(Op::SetFont(Font::A));
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

impl Banner {
    /// Emit IR ops for this banner component.
    ///
    /// Renders a box-drawing frame around the content text, auto-sizing
    /// the width to be as large as possible while fitting the content.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        let (size, total_width) = Self::fit(self.content.len(), self.size);
        let [h, w] = size;
        let font = if h == 0 && w == 0 { Font::B } else { Font::A };
        let esc_h = h.saturating_sub(1);
        let esc_w = w.saturating_sub(1);

        // Border characters (CP437 box drawing)
        let (tl, tr, bl, br, horiz, vert) = match self.border {
            BorderStyle::Single => ('\u{250C}', '\u{2510}', '\u{2514}', '\u{2518}', '\u{2500}', '\u{2502}'),
            BorderStyle::Double => ('\u{2554}', '\u{2557}', '\u{255A}', '\u{255D}', '\u{2550}', '\u{2551}'),
        };

        let inner = total_width - 2; // space inside the border
        let text = if self.content.len() > inner {
            &self.content[..inner]
        } else {
            &self.content
        };
        let pad = inner.saturating_sub(text.len());
        let pad_left = pad / 2;
        let pad_right = pad - pad_left;

        let h_bar: String = std::iter::repeat(horiz).take(inner).collect();
        let top = format!("{}{}{}", tl, h_bar, tr);
        let bot = format!("{}{}{}", bl, h_bar, br);
        let empty_line = format!("{}{}{}", vert, " ".repeat(inner), vert);
        let content_line = format!(
            "{}{}{}{}{}",
            vert,
            " ".repeat(pad_left),
            text,
            " ".repeat(pad_right),
            vert
        );

        // Set style
        ops.push(Op::SetFont(font));
        ops.push(Op::SetAlign(Alignment::Left));
        if esc_h > 0 || esc_w > 0 {
            ops.push(Op::SetSize {
                height: esc_h,
                width: esc_w,
            });
        }

        // Top border
        ops.push(Op::Text(top));
        ops.push(Op::Newline);

        // Padding lines above content
        for _ in 0..self.padding {
            ops.push(Op::Text(empty_line.clone()));
            ops.push(Op::Newline);
        }

        // Content line (bold if enabled)
        if self.bold {
            ops.push(Op::SetBold(true));
        }
        ops.push(Op::Text(content_line));
        ops.push(Op::Newline);
        if self.bold {
            ops.push(Op::SetBold(false));
        }

        // Padding lines below content
        for _ in 0..self.padding {
            ops.push(Op::Text(empty_line.clone()));
            ops.push(Op::Newline);
        }

        // Bottom border
        ops.push(Op::Text(bot));
        ops.push(Op::Newline);

        // Reset size if we changed it
        if esc_h > 0 || esc_w > 0 {
            ops.push(Op::SetSize {
                height: 0,
                width: 0,
            });
        }
    }

    /// Find the largest size that fits the content.
    ///
    /// Returns `([h, w], total_chars_per_line)`.
    /// Cascades width from `max_size` down to 1, then falls back to Font B.
    pub fn fit(content_len: usize, max_size: u8) -> ([u8; 2], usize) {
        // Need at least 2 chars for border + some inner space
        let border_overhead = 2; // left + right border chars

        // Try each width from max down to 1 (Font A with ESC i)
        for w in (1..=max_size).rev() {
            let chars_per_line = 48 / w as usize;
            let usable = chars_per_line.saturating_sub(border_overhead);
            if content_len <= usable {
                return ([max_size, w], chars_per_line);
            }
        }

        // Font B fallback: 64 chars per line
        ([0, 0], 64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashed_divider() {
        let div = Divider {
            style: DividerStyle::Dashed,
            width: Some(10),
        };
        let mut ops = Vec::new();
        div.emit(&mut ops);
        assert!(ops.iter().any(|op| *op == Op::Text("----------".into())));
    }

    #[test]
    fn test_equals_divider() {
        let div = Divider {
            style: DividerStyle::Equals,
            width: Some(5),
        };
        let mut ops = Vec::new();
        div.emit(&mut ops);
        assert!(ops.iter().any(|op| *op == Op::Text("=====".into())));
    }

    #[test]
    fn test_spacer_mm() {
        let spacer = Spacer::mm(5.0);
        let mut ops = Vec::new();
        spacer.emit(&mut ops);
        assert!(ops.iter().any(|op| *op == Op::Feed { units: 20 }));
    }

    #[test]
    fn test_spacer_lines() {
        let spacer = Spacer::lines(2);
        let mut ops = Vec::new();
        spacer.emit(&mut ops);
        assert!(ops.iter().any(|op| *op == Op::Feed { units: 24 }));
    }

    #[test]
    fn test_columns() {
        let cols = Columns {
            left: "Left".into(),
            right: "Right".into(),
            width: Some(20),
            ..Default::default()
        };
        let mut ops = Vec::new();
        cols.emit(&mut ops);
        let has_columns = ops.iter().any(|op| {
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
        let cols = Columns {
            left: "ITEM".into(),
            right: "PRICE".into(),
            bold: true,
            ..Default::default()
        };
        let mut ops = Vec::new();
        cols.emit(&mut ops);
        assert!(ops.contains(&Op::SetBold(true)));
        assert!(ops.contains(&Op::SetBold(false)));
    }

    #[test]
    fn test_blank_line() {
        let blank = BlankLine {};
        let mut ops = Vec::new();
        blank.emit(&mut ops);
        assert!(ops.iter().any(|op| *op == Op::Newline));
    }

    // ========================================================================
    // Banner tests
    // ========================================================================

    #[test]
    fn test_banner_fit_short_text() {
        // "HELLO" (5 chars) fits at size 3×3 (16 chars/line, 14 usable)
        let (size, total) = Banner::fit(5, 3);
        assert_eq!(size, [3, 3]);
        assert_eq!(total, 16);
    }

    #[test]
    fn test_banner_fit_medium_text() {
        // 15 chars won't fit at 3×3 (14 usable) but fits at 3×2 (22 usable)
        let (size, total) = Banner::fit(15, 3);
        assert_eq!(size, [3, 2]);
        assert_eq!(total, 24);
    }

    #[test]
    fn test_banner_fit_long_text() {
        // 47 chars won't fit at 3×1 (46 usable) → Font B (62 usable)
        let (size, total) = Banner::fit(47, 3);
        assert_eq!(size, [0, 0]);
        assert_eq!(total, 64);
    }

    #[test]
    fn test_banner_fit_size_0() {
        // max_size 0 → always Font B
        let (size, total) = Banner::fit(5, 0);
        assert_eq!(size, [0, 0]);
        assert_eq!(total, 64);
    }

    #[test]
    fn test_banner_emit_basic() {
        let banner = Banner::new("TEST");
        let mut ops = Vec::new();
        banner.emit(&mut ops);

        // Should have font, alignment, size set, bold, and box-drawing text
        assert!(ops.contains(&Op::SetFont(Font::A)));
        assert!(ops.contains(&Op::SetAlign(Alignment::Left)));
        assert!(ops.contains(&Op::SetBold(true)));
        assert!(ops.contains(&Op::SetBold(false)));

        // Should have the content text with the border chars
        let has_content = ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.contains("TEST")
            } else {
                false
            }
        });
        assert!(has_content, "Banner should contain the content text");

        // Should have box-drawing border
        let has_top_border = ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.starts_with('\u{250C}') && s.ends_with('\u{2510}')
            } else {
                false
            }
        });
        assert!(has_top_border, "Banner should have top border");
    }

    #[test]
    fn test_banner_double_border() {
        let banner = Banner {
            content: "HI".into(),
            border: BorderStyle::Double,
            ..Default::default()
        };
        let mut ops = Vec::new();
        banner.emit(&mut ops);

        let has_double_top = ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.starts_with('\u{2554}') && s.ends_with('\u{2557}')
            } else {
                false
            }
        });
        assert!(has_double_top, "Banner should have double-line top border");
    }

    #[test]
    fn test_banner_font_b_fallback() {
        // Content too long for any Font A size → Font B
        let long = "A".repeat(50);
        let banner = Banner {
            content: long,
            size: 3,
            ..Default::default()
        };
        let mut ops = Vec::new();
        banner.emit(&mut ops);
        assert!(ops.contains(&Op::SetFont(Font::B)));
        assert!(!ops.iter().any(|op| matches!(op, Op::SetSize { .. })));
    }
}
