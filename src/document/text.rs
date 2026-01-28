//! Emit logic for text components: Text, Header, LineItem, Total.

use super::types::{Header, LineItem, Text, Total};
use crate::ir::Op;
use crate::preview::ttf_font;
use crate::protocol::text::{Alignment, Font};
use crate::render::dither;

impl Text {
    /// Emit IR ops for this text component.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        if let Some(ref font_name) = self.font {
            self.emit_with_custom_font(font_name, ops);
            return;
        }

        // Resolve alignment: explicit `align` field > `center` bool > `right` bool
        let alignment = if let Some(ref align) = self.align {
            match align.as_str() {
                "center" => Some(Alignment::Center),
                "right" => Some(Alignment::Right),
                "left" => Some(Alignment::Left),
                _ => None,
            }
        } else if self.center {
            Some(Alignment::Center)
        } else if self.right {
            Some(Alignment::Right)
        } else {
            Some(Alignment::Left)
        };

        // Derive font and ESC i values from unified size field
        // size [0, 0] → Font B, no ESC i
        // size [1, 1] → Font A, no ESC i (default)
        // size [H, W] → Font A, ESC i [H-1, W-1]
        let [h, w] = self.size;
        let font = if h == 0 && w == 0 { Font::B } else { Font::A };
        let esc_h = h.saturating_sub(1);
        let esc_w = w.saturating_sub(1);

        // Resolve scale (ESC W / ESC h)
        let (scaled_width, scaled_height) = match self.scale {
            Some([h, w]) => (w, h),
            None => (
                if self.double_width { 1 } else { 0 },
                if self.double_height { 1 } else { 0 },
            ),
        };

        // Auto-enable smoothing for scaled text (unless explicitly disabled)
        let is_scaled = scaled_width > 0 || scaled_height > 0 || esc_h > 0 || esc_w > 0;
        let should_smooth = match self.smoothing {
            Some(explicit) => Some(explicit),
            None if is_scaled => Some(true),
            None => None,
        };

        // Emit style changes (order matters for compatibility)
        if let Some(align) = alignment {
            ops.push(Op::SetAlign(align));
        }
        ops.push(Op::SetFont(font));
        if let Some(enabled) = should_smooth {
            ops.push(Op::SetSmoothing(enabled));
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
        if scaled_width > 0 {
            ops.push(Op::SetExpandedWidth(scaled_width));
        }
        if scaled_height > 0 {
            ops.push(Op::SetExpandedHeight(scaled_height));
        }
        if esc_h > 0 || esc_w > 0 {
            ops.push(Op::SetSize {
                height: esc_h,
                width: esc_w,
            });
        }

        // Emit text
        ops.push(Op::Text(self.content.clone()));
        if !self.is_inline {
            ops.push(Op::Newline);
        }

        // Reset styles that were changed (reverse order)
        if esc_h > 0 || esc_w > 0 {
            ops.push(Op::SetSize {
                height: 0,
                width: 0,
            });
        }
        if scaled_height > 0 {
            ops.push(Op::SetExpandedHeight(0));
        }
        if scaled_width > 0 {
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
        // Reset auto-enabled smoothing (but not explicit smoothing)
        if should_smooth == Some(true) && self.smoothing.is_none() {
            ops.push(Op::SetSmoothing(false));
        }
        // Note: alignment and font are NOT reset - they persist
    }

    /// Emit text rendered with a custom TTF font as a raster image.
    fn emit_with_custom_font(&self, font_name: &str, ops: &mut Vec<Op>) {
        let pixel_height = ttf_font::size_to_pixel_height(self.size);
        let print_width: usize = 576;

        let rendered = ttf_font::render_ttf_text(
            &self.content,
            font_name,
            self.bold,
            pixel_height,
            print_width,
        );

        if rendered.width == 0 || rendered.height == 0 {
            return;
        }

        // Handle alignment: compute x offset within 576 dots
        let x_offset = if self.center || self.align.as_deref() == Some("center") {
            (print_width.saturating_sub(rendered.width)) / 2
        } else if self.right || self.align.as_deref() == Some("right") {
            print_width.saturating_sub(rendered.width)
        } else {
            0
        };

        // Dither the anti-aliased f32 buffer to 1-bit raster
        // Place the rendered text at the correct x offset within full print width
        let raster_data = dither::generate_raster(
            print_width,
            rendered.height,
            |x, y, _w, _h| {
                let local_x = x as i32 - x_offset as i32;
                if local_x < 0 || local_x >= rendered.width as i32 {
                    return 0.0;
                }
                let idx = y * rendered.width + local_x as usize;
                rendered.data.get(idx).copied().unwrap_or(0.0)
            },
            dither::DitheringAlgorithm::Atkinson,
        );

        // Handle invert: flip all bits
        let raster_data = if self.invert {
            raster_data.iter().map(|b| !b).collect()
        } else {
            raster_data
        };

        ops.push(Op::Raster {
            width: print_width as u16,
            height: rendered.height as u16,
            data: raster_data,
        });
    }
}

impl Header {
    /// Emit IR ops for this header component.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        let variant = self.variant.as_deref().unwrap_or("normal");
        let text = match variant {
            "small" => Text {
                content: self.content.clone(),
                bold: true,
                center: true,
                ..Default::default()
            },
            _ => Text {
                content: self.content.clone(),
                bold: true,
                center: true,
                size: [2, 2],
                ..Default::default()
            },
        };
        text.emit(ops);
    }
}

impl LineItem {
    /// Emit IR ops for this line item component.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        let width = self.width.unwrap_or(48);
        let price_str = format!("{:.2}", self.price);
        let name_max_width = width.saturating_sub(price_str.len() + 1);
        let name = if self.name.len() > name_max_width {
            &self.name[..name_max_width]
        } else {
            &self.name
        };
        let padding = width.saturating_sub(name.len() + price_str.len());
        let line = format!(
            "{}{:>pad$}",
            name,
            price_str,
            pad = padding + price_str.len()
        );

        // Reset to Font A to ensure correct width (48 chars × 12 dots = 576 = full print width)
        ops.push(Op::SetFont(Font::A));
        ops.push(Op::SetAlign(Alignment::Left));
        ops.push(Op::Text(line));
        ops.push(Op::Newline);
    }
}

impl Total {
    /// Emit IR ops for this total component.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        let label = self.label.as_deref().unwrap_or("TOTAL:");
        let bold = self.bold.unwrap_or(self.label.is_none());
        let right_align = match self.align.as_deref() {
            Some("left") => false,
            _ => true, // default right
        };
        let scaled_width: u8 = if self.double_width { 1 } else { 0 };

        let amount_str = format!("{:.2}", self.amount);
        let line = format!("{}  {}", label, amount_str);

        // Reset to Font A to ensure correct width
        ops.push(Op::SetFont(Font::A));
        if right_align {
            ops.push(Op::SetAlign(Alignment::Right));
        }
        if bold {
            ops.push(Op::SetBold(true));
        }
        if scaled_width > 0 {
            ops.push(Op::SetExpandedWidth(scaled_width));
        }
        ops.push(Op::Text(line));
        ops.push(Op::Newline);
        if scaled_width > 0 {
            ops.push(Op::SetExpandedWidth(0));
        }
        if bold {
            ops.push(Op::SetBold(false));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_text() {
        let text = Text::new("Hello");
        let mut ops = Vec::new();
        text.emit(&mut ops);
        assert!(ops.iter().any(|op| *op == Op::Text("Hello".into())));
        assert!(ops.iter().any(|op| *op == Op::Newline));
    }

    #[test]
    fn test_inline_text() {
        let text = Text {
            content: "Hello".into(),
            is_inline: true,
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);
        assert!(ops.iter().any(|op| *op == Op::Text("Hello".into())));
        assert!(!ops.iter().any(|op| *op == Op::Newline));
    }

    #[test]
    fn test_bold_text() {
        let text = Text {
            content: "Bold".into(),
            bold: true,
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);
        let bold_on = ops.iter().position(|op| *op == Op::SetBold(true));
        let text_pos = ops.iter().position(|op| *op == Op::Text("Bold".into()));
        let bold_off = ops.iter().position(|op| *op == Op::SetBold(false));
        assert!(bold_on.unwrap() < text_pos.unwrap());
        assert!(text_pos.unwrap() < bold_off.unwrap());
    }

    #[test]
    fn test_centered_text() {
        let text = Text {
            content: "C".into(),
            center: true,
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);
        assert!(ops.iter().any(|op| *op == Op::SetAlign(Alignment::Center)));
    }

    #[test]
    fn test_align_field_overrides_center() {
        let text = Text {
            content: "R".into(),
            align: Some("right".into()),
            center: true, // should be ignored
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);
        assert!(ops.iter().any(|op| *op == Op::SetAlign(Alignment::Right)));
        assert!(!ops.iter().any(|op| *op == Op::SetAlign(Alignment::Center)));
    }

    #[test]
    fn test_header_normal() {
        let header = Header::new("STORE");
        let mut ops = Vec::new();
        header.emit(&mut ops);
        assert!(ops.iter().any(|op| *op == Op::SetAlign(Alignment::Center)));
        assert!(ops.iter().any(|op| *op == Op::SetBold(true)));
        assert!(ops.iter().any(|op| *op == Op::SetSize { height: 1, width: 1 }));
    }

    #[test]
    fn test_header_small() {
        let header = Header {
            content: "small".into(),
            variant: Some("small".into()),
        };
        let mut ops = Vec::new();
        header.emit(&mut ops);
        assert!(ops.iter().any(|op| *op == Op::SetBold(true)));
        assert!(!ops.iter().any(|op| matches!(op, Op::SetSize { .. })));
    }

    #[test]
    fn test_line_item() {
        let item = LineItem::new("Coffee", 4.50);
        let mut ops = Vec::new();
        item.emit(&mut ops);
        let has_formatted_line = ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.contains("Coffee") && s.contains("4.50")
            } else {
                false
            }
        });
        assert!(has_formatted_line);
    }

    #[test]
    fn test_total_default() {
        let total = Total::new(19.99);
        let mut ops = Vec::new();
        total.emit(&mut ops);
        assert!(ops.iter().any(|op| *op == Op::SetBold(true)));
        assert!(ops.iter().any(|op| *op == Op::SetAlign(Alignment::Right)));
        let has_total = ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.contains("TOTAL:") && s.contains("19.99")
            } else {
                false
            }
        });
        assert!(has_total);
    }

    #[test]
    fn test_total_labeled_not_bold() {
        let total = Total {
            amount: 0.99,
            label: Some("TAX:".into()),
            bold: Some(false),
            ..Default::default()
        };
        let mut ops = Vec::new();
        total.emit(&mut ops);
        assert!(!ops.iter().any(|op| *op == Op::SetBold(true)));
    }

    #[test]
    fn test_auto_smoothing_with_scale() {
        let text = Text {
            content: "BIG".into(),
            scale: Some([1, 1]),
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);
        assert!(ops.contains(&Op::SetSmoothing(true)));
        assert!(ops.contains(&Op::SetSmoothing(false)));
    }

    #[test]
    fn test_auto_smoothing_with_size() {
        let text = Text {
            content: "BIG".into(),
            size: [2, 2],
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);
        assert!(ops.contains(&Op::SetSmoothing(true)));
        assert!(ops.contains(&Op::SetSmoothing(false)));
    }

    #[test]
    fn test_no_auto_smoothing_normal_text() {
        let text = Text::new("normal");
        let mut ops = Vec::new();
        text.emit(&mut ops);
        assert!(!ops.iter().any(|op| matches!(op, Op::SetSmoothing(_))));
    }

    #[test]
    fn test_explicit_smoothing_not_reset() {
        let text = Text {
            content: "smooth".into(),
            smoothing: Some(true),
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);
        assert!(ops.contains(&Op::SetSmoothing(true)));
        assert!(!ops.contains(&Op::SetSmoothing(false)));
    }

    #[test]
    fn test_size_0_uses_font_b() {
        let text = Text {
            content: "small".into(),
            size: [0, 0],
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);
        assert!(ops.contains(&Op::SetFont(Font::B)));
        assert!(!ops.iter().any(|op| matches!(op, Op::SetSize { .. })));
    }

    #[test]
    fn test_size_1_uses_font_a_no_expansion() {
        let text = Text::new("normal");
        let mut ops = Vec::new();
        text.emit(&mut ops);
        assert!(ops.contains(&Op::SetFont(Font::A)));
        assert!(!ops.iter().any(|op| matches!(op, Op::SetSize { .. })));
    }

    #[test]
    fn test_size_3_triple_expansion() {
        let text = Text {
            content: "huge".into(),
            size: [3, 3],
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);
        assert!(ops.contains(&Op::SetFont(Font::A)));
        assert!(ops.contains(&Op::SetSize { height: 2, width: 2 }));
    }

    #[test]
    fn test_size_asymmetric() {
        let text = Text {
            content: "tall".into(),
            size: [3, 1],
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);
        assert!(ops.contains(&Op::SetFont(Font::A)));
        // [3, 1] → ESC i [2, 0] — height expansion only
        assert!(ops.contains(&Op::SetSize { height: 2, width: 0 }));
    }
}
