//! Emit logic for text components: Text, Header, LineItem, Total.

use super::types::{Header, LineItem, Text, Total};
use crate::ir::Op;
use crate::preview::{FontMetrics, emoji, generate_glyph, ttf_font};
use crate::protocol::text::{Alignment, Font};
use crate::render::dither;

impl Text {
    /// Emit IR ops for this text component.
    pub fn emit(&self, ops: &mut Vec<Op>) {
        // Priority 1: Custom font specified → TTF rendering
        if let Some(ref font_name) = self.font {
            // With custom font, also handle emoji if present
            if emoji::contains_emoji(&self.content) {
                self.emit_with_font_and_emoji(font_name, ops);
            } else {
                self.emit_with_custom_font(font_name, ops);
            }
            return;
        }

        // Priority 2: Contains emoji (no custom font) → bitmap font + emoji sprites
        if emoji::contains_emoji(&self.content) {
            self.emit_with_emoji(ops);
            return;
        }

        // Default: standard text ops (no graphics rendering)

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

    /// Emit text with emoji using bitmap fonts (no custom font specified).
    ///
    /// Uses the standard bitmap font system (Spleen) for regular characters
    /// and emoji sprites for supported emoji. Both are 1-bit, so no dithering needed.
    fn emit_with_emoji(&self, ops: &mut Vec<Op>) {
        let print_width: usize = 576;

        // Determine font based on size field
        let [h, _w] = self.size;
        let font = if h == 0 { Font::B } else { Font::A };
        let metrics = FontMetrics::for_font(font);

        // Calculate size multiplier for scaling
        let height_mult = h.max(1) as usize;
        let char_height = metrics.char_height * height_mult;
        let char_width = metrics.char_width * height_mult;

        // Parse text into segments (handles both single-char and keycap emoji)
        let segments = emoji::parse_text(&self.content);

        // First pass: calculate total width and collect glyphs
        let mut glyphs: Vec<GlyphData> = Vec::new();
        let mut total_width = 0usize;

        for segment in &segments {
            match segment {
                emoji::TextSegment::Emoji(ch) => {
                    if let Some(sprite) = emoji::get_emoji_bitmap(*ch, char_height) {
                        glyphs.push(GlyphData::Emoji {
                            width: sprite.width,
                            height: sprite.height,
                            data: sprite.data,
                        });
                        total_width += sprite.width;
                    }
                }
                emoji::TextSegment::KeycapEmoji(seq) => {
                    if let Some(sprite) = emoji::get_keycap_bitmap(seq, char_height) {
                        glyphs.push(GlyphData::Emoji {
                            width: sprite.width,
                            height: sprite.height,
                            data: sprite.data,
                        });
                        total_width += sprite.width;
                    }
                }
                emoji::TextSegment::Text(text) => {
                    for ch in text.chars() {
                        let glyph = generate_glyph(font, ch);
                        glyphs.push(GlyphData::Char {
                            width: metrics.char_width,
                            height: metrics.char_height,
                            data: glyph,
                            scale: height_mult,
                        });
                        total_width += char_width;
                    }
                }
            }
        }

        if glyphs.is_empty() || total_width == 0 {
            return;
        }

        // Handle alignment
        let x_offset = if self.center || self.align.as_deref() == Some("center") {
            (print_width.saturating_sub(total_width)) / 2
        } else if self.right || self.align.as_deref() == Some("right") {
            print_width.saturating_sub(total_width)
        } else {
            0
        };

        // Create output buffer (1 byte per pixel for now, will pack later)
        let mut buffer = vec![0u8; print_width * char_height];
        let mut cursor_x = x_offset;

        // Second pass: render glyphs into buffer
        for glyph in &glyphs {
            match glyph {
                GlyphData::Emoji {
                    width,
                    height,
                    data,
                } => {
                    // Copy emoji sprite to buffer
                    for y in 0..*height {
                        for x in 0..*width {
                            let src_idx = y * width + x;
                            let dst_x = cursor_x + x;
                            let dst_y = y;
                            if dst_x < print_width && dst_y < char_height {
                                let dst_idx = dst_y * print_width + dst_x;
                                if data[src_idx] != 0 {
                                    buffer[dst_idx] = 1;
                                }
                            }
                        }
                    }
                    cursor_x += width;
                }
                GlyphData::Char {
                    width,
                    height,
                    data,
                    scale,
                } => {
                    // Copy scaled character glyph to buffer
                    for src_y in 0..*height {
                        for src_x in 0..*width {
                            let src_idx = src_y * width + src_x;
                            if data[src_idx] != 0 {
                                // Scale up the pixel
                                for dy in 0..*scale {
                                    for dx in 0..*scale {
                                        let dst_x = cursor_x + src_x * scale + dx;
                                        let dst_y = src_y * scale + dy;
                                        if dst_x < print_width && dst_y < char_height {
                                            let dst_idx = dst_y * print_width + dst_x;
                                            buffer[dst_idx] = 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    cursor_x += width * scale;
                }
            }
        }

        // Apply bold by duplicating pixels to the right
        if self.bold {
            for y in 0..char_height {
                for x in (1..print_width).rev() {
                    let idx = y * print_width + x;
                    let prev_idx = idx - 1;
                    if buffer[prev_idx] != 0 {
                        buffer[idx] = 1;
                    }
                }
            }
        }

        // Pack into 1-bit raster data
        let width_bytes = print_width.div_ceil(8);
        let mut raster_data = vec![0u8; width_bytes * char_height];

        for y in 0..char_height {
            for x in 0..print_width {
                let src_idx = y * print_width + x;
                let is_black = buffer[src_idx] != 0;

                // Invert if requested
                let pixel = if self.invert { !is_black } else { is_black };

                if pixel {
                    let byte_idx = y * width_bytes + x / 8;
                    let bit_idx = 7 - (x % 8);
                    raster_data[byte_idx] |= 1 << bit_idx;
                }
            }
        }

        ops.push(Op::Raster {
            width: print_width as u16,
            height: char_height as u16,
            data: raster_data,
        });
    }

    /// Emit text with emoji using TTF fonts (custom font specified).
    ///
    /// Uses TTF rendering for regular characters and emoji sprites for emoji.
    /// Both produce f32 buffers, dithered with Atkinson.
    fn emit_with_font_and_emoji(&self, font_name: &str, ops: &mut Vec<Op>) {
        let pixel_height = ttf_font::size_to_pixel_height(self.size);
        let print_width: usize = 576;
        let target_height = pixel_height.ceil() as usize;

        // Parse text into segments (handles both single-char and keycap emoji)
        let parsed_segments = emoji::parse_text(&self.content);

        // Convert to our internal segment format
        let mut segments: Vec<ContentSegment> = Vec::new();
        for seg in parsed_segments {
            match seg {
                emoji::TextSegment::Text(text) => {
                    segments.push(ContentSegment::Text(text));
                }
                emoji::TextSegment::Emoji(ch) => {
                    segments.push(ContentSegment::Emoji(ch));
                }
                emoji::TextSegment::KeycapEmoji(seq) => {
                    segments.push(ContentSegment::Keycap(seq));
                }
            }
        }

        // Calculate total width
        let mut total_width = 0usize;
        let mut segment_widths: Vec<usize> = Vec::new();

        for segment in &segments {
            let width = match segment {
                ContentSegment::Text(text) => {
                    let rendered = ttf_font::render_ttf_text(
                        text,
                        font_name,
                        self.bold,
                        pixel_height,
                        print_width,
                    );
                    rendered.width
                }
                ContentSegment::Emoji(ch) => emoji::get_emoji_grayscale(*ch, target_height)
                    .map(|s| s.width)
                    .unwrap_or(0),
                ContentSegment::Keycap(seq) => emoji::get_keycap_grayscale(seq, target_height)
                    .map(|s| s.width)
                    .unwrap_or(0),
            };
            segment_widths.push(width);
            total_width += width;
        }

        if total_width == 0 {
            return;
        }

        // Handle alignment
        let x_offset = if self.center || self.align.as_deref() == Some("center") {
            (print_width.saturating_sub(total_width)) / 2
        } else if self.right || self.align.as_deref() == Some("right") {
            print_width.saturating_sub(total_width)
        } else {
            0
        };

        // Create f32 intensity buffer
        let mut buffer = vec![0.0f32; print_width * target_height];
        let mut cursor_x = x_offset;

        // Render each segment
        for (segment, &width) in segments.iter().zip(segment_widths.iter()) {
            match segment {
                ContentSegment::Text(text) => {
                    let rendered = ttf_font::render_ttf_text(
                        text,
                        font_name,
                        self.bold,
                        pixel_height,
                        print_width,
                    );
                    // Copy rendered text to buffer
                    for y in 0..rendered.height.min(target_height) {
                        for x in 0..rendered.width {
                            let src_idx = y * rendered.width + x;
                            let dst_x = cursor_x + x;
                            if dst_x < print_width {
                                let dst_idx = y * print_width + dst_x;
                                buffer[dst_idx] =
                                    rendered.data.get(src_idx).copied().unwrap_or(0.0);
                            }
                        }
                    }
                }
                ContentSegment::Emoji(ch) => {
                    if let Some(sprite) = emoji::get_emoji_grayscale(*ch, target_height) {
                        // Copy emoji sprite to buffer
                        for y in 0..sprite.height.min(target_height) {
                            for x in 0..sprite.width {
                                let src_idx = y * sprite.width + x;
                                let dst_x = cursor_x + x;
                                if dst_x < print_width {
                                    let dst_idx = y * print_width + dst_x;
                                    buffer[dst_idx] =
                                        sprite.data.get(src_idx).copied().unwrap_or(0.0);
                                }
                            }
                        }
                    }
                }
                ContentSegment::Keycap(seq) => {
                    if let Some(sprite) = emoji::get_keycap_grayscale(seq, target_height) {
                        // Copy keycap emoji sprite to buffer
                        for y in 0..sprite.height.min(target_height) {
                            for x in 0..sprite.width {
                                let src_idx = y * sprite.width + x;
                                let dst_x = cursor_x + x;
                                if dst_x < print_width {
                                    let dst_idx = y * print_width + dst_x;
                                    buffer[dst_idx] =
                                        sprite.data.get(src_idx).copied().unwrap_or(0.0);
                                }
                            }
                        }
                    }
                }
            }
            cursor_x += width;
        }

        // Dither to 1-bit raster
        let raster_data = dither::generate_raster(
            print_width,
            target_height,
            |x, y, _w, _h| {
                let idx = y * print_width + x;
                buffer.get(idx).copied().unwrap_or(0.0)
            },
            dither::DitheringAlgorithm::Atkinson,
        );

        // Handle invert
        let raster_data = if self.invert {
            raster_data.iter().map(|b| !b).collect()
        } else {
            raster_data
        };

        ops.push(Op::Raster {
            width: print_width as u16,
            height: target_height as u16,
            data: raster_data,
        });
    }
}

/// Glyph data for compositing text with emoji.
enum GlyphData {
    /// Emoji sprite (already at target size).
    Emoji {
        width: usize,
        height: usize,
        data: Vec<u8>,
    },
    /// Character from bitmap font (may need scaling).
    Char {
        width: usize,
        height: usize,
        data: Vec<u8>,
        scale: usize,
    },
}

/// Content segment for mixed text/emoji rendering.
enum ContentSegment {
    Text(String),
    Emoji(char),
    Keycap(String),
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
        assert!(ops.iter().any(|op| *op
            == Op::SetSize {
                height: 1,
                width: 1
            }));
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
        assert!(ops.contains(&Op::SetSize {
            height: 2,
            width: 2
        }));
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
        assert!(ops.contains(&Op::SetSize {
            height: 2,
            width: 0
        }));
    }

    // ============================================================================
    // EMOJI TESTS
    // ============================================================================

    #[test]
    fn test_text_with_emoji_emits_raster() {
        // Text with emoji should emit Raster op (not Text op)
        let text = Text {
            content: "Hello ☀ World".into(),
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);

        // Should have a Raster op
        assert!(
            ops.iter().any(|op| matches!(op, Op::Raster { .. })),
            "Text with emoji should emit Raster op"
        );
        // Should NOT have a Text op (emoji forces graphics mode)
        assert!(
            !ops.iter().any(|op| matches!(op, Op::Text(_))),
            "Text with emoji should not emit Text op"
        );
    }

    #[test]
    fn test_text_without_emoji_emits_text() {
        // Text without emoji should emit normal Text op
        let text = Text::new("Hello World");
        let mut ops = Vec::new();
        text.emit(&mut ops);

        // Should have a Text op
        assert!(
            ops.iter().any(|op| matches!(op, Op::Text(_))),
            "Text without emoji should emit Text op"
        );
        // Should NOT have a Raster op
        assert!(
            !ops.iter().any(|op| matches!(op, Op::Raster { .. })),
            "Text without emoji should not emit Raster op"
        );
    }

    #[test]
    fn test_text_with_emoji_and_custom_font() {
        // Text with emoji AND custom font should use TTF+emoji path
        let text = Text {
            content: "Heart ❤ love".into(),
            font: Some("ibm".into()),
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);

        // Should have a Raster op
        assert!(
            ops.iter().any(|op| matches!(op, Op::Raster { .. })),
            "Text with emoji and custom font should emit Raster op"
        );
    }

    #[test]
    fn test_emoji_raster_has_content() {
        // Verify the raster has actual content (non-zero data)
        let text = Text {
            content: "☀".into(),
            ..Default::default()
        };
        let mut ops = Vec::new();
        text.emit(&mut ops);

        if let Some(Op::Raster {
            width,
            height,
            data,
        }) = ops.iter().find(|op| matches!(op, Op::Raster { .. }))
        {
            assert!(*width > 0, "Raster should have non-zero width");
            assert!(*height > 0, "Raster should have non-zero height");
            // Should have some black pixels
            assert!(
                data.iter().any(|&b| b != 0),
                "Emoji raster should have some black pixels"
            );
        } else {
            panic!("Expected Raster op");
        }
    }
}
