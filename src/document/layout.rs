//! Emit logic for layout components: Divider, Spacer, BlankLine, Columns, Banner.

use super::types::{
    Banner, BlankLine, BorderStyle, ColumnAlign, Columns, Divider, DividerStyle, Spacer, Table,
};
use super::EmitContext;
use crate::ir::{Op, Program};
use crate::preview::ttf_font;
use crate::protocol::text::{Alignment, Font};
use crate::render::dither;

impl Divider {
    /// Emit IR ops for this divider component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        let width = self.width.unwrap_or(ctx.chars_per_line());
        let line = match self.style {
            DividerStyle::Dashed => "-".repeat(width),
            DividerStyle::Solid => "\u{2500}".repeat(width), // ─
            DividerStyle::Double => "\u{2550}".repeat(width), // ═
            DividerStyle::Equals => "=".repeat(width),
        };
        // Reset to Font A to ensure correct width (48 chars × 12 dots = 576 = full print width)
        ctx.push(Op::SetFont(Font::A));
        ctx.push(Op::SetAlign(Alignment::Left));
        ctx.push(Op::Text(line));
        ctx.push(Op::Newline);
    }
}

impl Spacer {
    /// Emit IR ops for this spacer component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        let units = if let Some(mm) = self.mm {
            (mm * 4.0).round().clamp(0.0, 255.0) as u8
        } else if let Some(lines) = self.lines {
            (lines as f32 * 3.0 * 4.0).round().clamp(0.0, 255.0) as u8
        } else {
            self.units.unwrap_or_default()
        };

        if units > 0 {
            ctx.push(Op::Feed { units });
        }
    }
}

impl BlankLine {
    /// Emit IR ops for this blank line component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        ctx.push(Op::Newline);
    }
}

impl Columns {
    /// Emit IR ops for this two-column layout component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        let width = self.width.unwrap_or(ctx.chars_per_line());
        let padding = width.saturating_sub(self.left.len() + self.right.len());
        let line = format!(
            "{}{:>width$}",
            self.left,
            self.right,
            width = padding + self.right.len()
        );

        // Reset to Font A to ensure correct width (48 chars × 12 dots = 576 = full print width)
        ctx.push(Op::SetFont(Font::A));
        ctx.push(Op::SetAlign(Alignment::Left));
        if self.bold {
            ctx.push(Op::SetBold(true));
        }
        if self.underline {
            ctx.push(Op::SetUnderline(true));
        }
        if self.invert {
            ctx.push(Op::SetInvert(true));
        }

        ctx.push(Op::Text(line));
        ctx.push(Op::Newline);

        if self.invert {
            ctx.push(Op::SetInvert(false));
        }
        if self.underline {
            ctx.push(Op::SetUnderline(false));
        }
        if self.bold {
            ctx.push(Op::SetBold(false));
        }
    }
}

impl Banner {
    /// Emit IR ops for this banner component.
    ///
    /// Renders a box-drawing frame around the content text, auto-sizing
    /// the width to be as large as possible while fitting the content.
    pub fn emit(&self, ctx: &mut EmitContext) {
        if let Some(ref font_name) = self.font {
            self.emit_with_custom_font(font_name, ctx);
            return;
        }

        let (size, total_width) = Self::fit(self.content.len(), self.size, self.border, ctx.print_width);
        let [h, w] = size;
        let font = if h == 0 && w == 0 { Font::B } else { Font::A };
        let esc_h = h.saturating_sub(1);
        let esc_w = w.saturating_sub(1);

        // Set style
        ctx.push(Op::SetFont(font));
        ctx.push(Op::SetAlign(Alignment::Left));
        if esc_h > 0 || esc_w > 0 {
            ctx.push(Op::SetSize {
                height: esc_h,
                width: esc_w,
            });
        }

        match self.border {
            BorderStyle::Shadow => self.emit_shadow(ctx, total_width),
            BorderStyle::Rule => self.emit_rule(ctx, total_width),
            BorderStyle::Heading => self.emit_heading(ctx, total_width),
            BorderStyle::Tag => self.emit_tag(ctx, total_width),
            _ => self.emit_boxed(ctx, total_width),
        }

        // Reset size if we changed it
        if esc_h > 0 || esc_w > 0 {
            ctx.push(Op::SetSize {
                height: 0,
                width: 0,
            });
        }
    }

    /// Emit a standard boxed banner (Single, Double, Heavy, Shade).
    fn emit_boxed(&self, ctx: &mut EmitContext, total_width: usize) {
        let (tl, tr, bl, br, horiz, vert) = match self.border {
            BorderStyle::Single | BorderStyle::Mixed => (
                '\u{250C}', '\u{2510}', '\u{2514}', '\u{2518}', '\u{2500}', '\u{2502}',
            ),
            BorderStyle::Double => (
                '\u{2554}', '\u{2557}', '\u{255A}', '\u{255D}', '\u{2550}', '\u{2551}',
            ),
            BorderStyle::Heavy => (
                '\u{2588}', '\u{2588}', '\u{2588}', '\u{2588}', '\u{2588}', '\u{2588}',
            ),
            BorderStyle::Shade => (
                '\u{2592}', '\u{2592}', '\u{2592}', '\u{2592}', '\u{2592}', '\u{2592}',
            ),
            BorderStyle::Shadow | BorderStyle::Rule | BorderStyle::Heading | BorderStyle::Tag => {
                unreachable!()
            }
        };

        let inner = total_width - 2;
        let text = if self.content.len() > inner {
            &self.content[..inner]
        } else {
            &self.content
        };
        let pad = inner.saturating_sub(text.len());
        let pad_left = pad / 2;
        let pad_right = pad - pad_left;

        let h_bar: String = std::iter::repeat_n(horiz, inner).collect();
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

        // Top border
        ctx.push(Op::Text(top));
        ctx.push(Op::Newline);

        // Padding lines above content
        for _ in 0..self.padding {
            ctx.push(Op::Text(empty_line.clone()));
            ctx.push(Op::Newline);
        }

        // Content line (bold if enabled)
        if self.bold {
            ctx.push(Op::SetBold(true));
        }
        ctx.push(Op::Text(content_line));
        ctx.push(Op::Newline);
        if self.bold {
            ctx.push(Op::SetBold(false));
        }

        // Padding lines below content
        for _ in 0..self.padding {
            ctx.push(Op::Text(empty_line.clone()));
            ctx.push(Op::Newline);
        }

        // Bottom border
        ctx.push(Op::Text(bot));
        ctx.push(Op::Newline);
    }

    /// Emit a shadow-style banner: single border + dark shade shadow.
    ///
    /// ```text
    /// ┌──────────┐
    /// │          │▓
    /// │  CHURRA  │▓
    /// │          │▓
    /// └──────────┘▓
    ///  ▓▓▓▓▓▓▓▓▓▓▓
    /// ```
    fn emit_shadow(&self, ctx: &mut EmitContext, total_width: usize) {
        let shadow = '\u{2593}'; // ▓

        // Shadow takes 1 char on right, so the box is (total_width - 1) wide
        let box_width = total_width - 1;
        let inner = box_width - 2;

        let text = if self.content.len() > inner {
            &self.content[..inner]
        } else {
            &self.content
        };
        let pad = inner.saturating_sub(text.len());
        let pad_left = pad / 2;
        let pad_right = pad - pad_left;

        let h_bar: String = std::iter::repeat_n('\u{2500}', inner).collect();
        let top = format!("\u{250C}{}\u{2510}", h_bar);
        let bot_with_shadow = format!("\u{2514}{}\u{2518}{}", h_bar, shadow);
        let empty_with_shadow = format!("\u{2502}{}\u{2502}{}", " ".repeat(inner), shadow);
        let content_line = format!(
            "\u{2502}{}{}{}\u{2502}{}",
            " ".repeat(pad_left),
            text,
            " ".repeat(pad_right),
            shadow
        );
        let shadow_bottom: String = format!(
            " {}",
            std::iter::repeat_n(shadow, box_width).collect::<String>()
        );

        // Top border (no shadow — creates depth illusion)
        ctx.push(Op::Text(top));
        ctx.push(Op::Newline);

        // Padding lines above content (with shadow on right)
        for _ in 0..self.padding {
            ctx.push(Op::Text(empty_with_shadow.clone()));
            ctx.push(Op::Newline);
        }

        // Content line (with shadow on right)
        if self.bold {
            ctx.push(Op::SetBold(true));
        }
        ctx.push(Op::Text(content_line));
        ctx.push(Op::Newline);
        if self.bold {
            ctx.push(Op::SetBold(false));
        }

        // Padding lines below content (with shadow on right)
        for _ in 0..self.padding {
            ctx.push(Op::Text(empty_with_shadow.clone()));
            ctx.push(Op::Newline);
        }

        // Bottom border with shadow
        ctx.push(Op::Text(bot_with_shadow));
        ctx.push(Op::Newline);

        // Shadow bottom row
        ctx.push(Op::Text(shadow_bottom));
        ctx.push(Op::Newline);
    }

    /// Emit a rule-style banner: `──── TEXT ──────────────────────` (single line).
    fn emit_rule(&self, ctx: &mut EmitContext, total_width: usize) {
        let text = &self.content;
        let text_len = text.len();
        // " TEXT " with 1 space on each side
        let text_with_spaces = text_len + 2;
        let remaining = total_width.saturating_sub(text_with_spaces);
        let left_rules = remaining / 2;
        let right_rules = remaining - left_rules;

        let line = format!(
            "{} {} {}",
            "\u{2500}".repeat(left_rules),
            text,
            "\u{2500}".repeat(right_rules),
        );

        if self.bold {
            ctx.push(Op::SetBold(true));
        }
        ctx.push(Op::Text(line));
        ctx.push(Op::Newline);
        if self.bold {
            ctx.push(Op::SetBold(false));
        }
    }

    /// Emit a heading-style banner: centered bold text + full-width rule below (2 lines).
    fn emit_heading(&self, ctx: &mut EmitContext, total_width: usize) {
        // Centered text line
        ctx.push(Op::SetAlign(Alignment::Center));
        if self.bold {
            ctx.push(Op::SetBold(true));
        }
        ctx.push(Op::Text(self.content.clone()));
        ctx.push(Op::Newline);
        if self.bold {
            ctx.push(Op::SetBold(false));
        }

        // Full-width rule below
        ctx.push(Op::SetAlign(Alignment::Left));
        let rule: String = "\u{2500}".repeat(total_width);
        ctx.push(Op::Text(rule));
        ctx.push(Op::Newline);
    }

    /// Emit a tag-style banner: `■ TEXT` (single line, left-aligned).
    fn emit_tag(&self, ctx: &mut EmitContext, _total_width: usize) {
        let line = format!("\u{25A0} {}", self.content);

        if self.bold {
            ctx.push(Op::SetBold(true));
        }
        ctx.push(Op::Text(line));
        ctx.push(Op::Newline);
        if self.bold {
            ctx.push(Op::SetBold(false));
        }
    }

    /// Emit a banner with custom font: render the banner frame using standard
    /// bitmap path, then composite TTF-rendered text content over the frame.
    fn emit_with_custom_font(&self, font_name: &str, ctx: &mut EmitContext) {
        // Render the banner frame with spaces instead of real text — same length
        // preserves the fit() result (same expansion, same frame geometry) while
        // leaving the interior blank for clean TTF compositing.
        let mut sub_ctx = EmitContext::new(ctx.print_width);
        let mut plain_banner = self.clone();
        plain_banner.font = None;
        plain_banner.content = " ".repeat(self.content.len());
        plain_banner.emit(&mut sub_ctx);

        if sub_ctx.ops.is_empty() {
            return;
        }

        // Render the bitmap banner to raw pixels
        let program = Program { ops: sub_ctx.ops };
        let Ok(raw) = crate::preview::render_raw(&program) else {
            return;
        };

        let width = raw.width;
        let height = raw.height;
        let width_bytes = width.div_ceil(8);

        // Convert 1-bit banner to f32 intensity buffer
        let mut buffer = vec![0.0f32; width * height];
        for y in 0..height {
            for x in 0..width {
                let byte_idx = y * width_bytes + x / 8;
                let bit_idx = 7 - (x % 8);
                let is_black = (raw.data.get(byte_idx).copied().unwrap_or(0) >> bit_idx) & 1 == 1;
                if is_black {
                    buffer[y * width + x] = 1.0;
                }
            }
        }

        // Use the actual fitted size — fit() may cascade width or fall back to Font B
        let (fitted_size, _) = Self::fit(self.content.len(), self.size, self.border, ctx.print_width);
        let pixel_height = ttf_font::size_to_pixel_height(fitted_size);
        let text_render =
            ttf_font::render_ttf_text(&self.content, font_name, self.bold, pixel_height, width);

        // Center the TTF text both horizontally and vertically within the banner
        let text_x = (width.saturating_sub(text_render.width)) / 2;
        let text_y = (height.saturating_sub(text_render.height)) / 2;

        // Composite TTF text over the banner (replace the bitmap text region)
        for ty in 0..text_render.height {
            for tx in 0..text_render.width {
                let dst_x = text_x + tx;
                let dst_y = text_y + ty;
                if dst_x < width && dst_y < height {
                    let src_idx = ty * text_render.width + tx;
                    let coverage = text_render.data.get(src_idx).copied().unwrap_or(0.0);
                    if coverage > 0.0 {
                        let dst_idx = dst_y * width + dst_x;
                        // Overwrite with TTF text (treat as Normal blend)
                        buffer[dst_idx] = coverage;
                    }
                }
            }
        }

        // Dither the composite to 1-bit (Atkinson for smooth AA text)
        let raster_data = dither::generate_raster(
            width,
            height,
            |x, y, _w, _h| buffer[y * width + x],
            dither::DitheringAlgorithm::Atkinson,
        );

        ctx.push(Op::Raster {
            width: width as u16,
            height: height as u16,
            data: raster_data,
        });
    }

    /// Find the largest size that fits the content.
    ///
    /// Returns `([h, w], total_chars_per_line)`.
    /// Cascades width from `max_size` down to 1, then falls back to Font B.
    pub fn fit(content_len: usize, max_size: u8, border: BorderStyle, print_width: usize) -> ([u8; 2], usize) {
        let border_overhead = match border {
            BorderStyle::Shadow => 3, // left + right + shadow column
            BorderStyle::Tag => 2,    // "■ " prefix
            BorderStyle::Rule | BorderStyle::Heading => 0, // rules fill remaining space
            _ => 2,                   // left + right
        };

        // Try each width from max down to 1 (Font A with ESC i)
        for w in (1..=max_size).rev() {
            let chars_per_line = (print_width / 12) / w as usize;
            let usable = chars_per_line.saturating_sub(border_overhead);
            if content_len <= usable {
                return ([max_size, w], chars_per_line);
            }
        }

        // Font B fallback
        ([0, 0], print_width / 9)
    }
}

// ============================================================================
// Table
// ============================================================================

/// Box-drawing character set for table borders.
struct TableChars {
    tl: char,
    tr: char,
    bl: char,
    br: char,
    horiz: char,
    vert: char,
    t_down: char,
    t_up: char,
    t_right: char,
    t_left: char,
    cross: char,
}

fn table_chars(style: BorderStyle) -> TableChars {
    match style {
        BorderStyle::Single
        | BorderStyle::Mixed
        | BorderStyle::Shadow
        | BorderStyle::Rule
        | BorderStyle::Heading
        | BorderStyle::Tag => TableChars {
            tl: '\u{250C}',
            tr: '\u{2510}',
            bl: '\u{2514}',
            br: '\u{2518}',
            horiz: '\u{2500}',
            vert: '\u{2502}',
            t_down: '\u{252C}',
            t_up: '\u{2534}',
            t_right: '\u{251C}',
            t_left: '\u{2524}',
            cross: '\u{253C}',
        },
        BorderStyle::Double => TableChars {
            tl: '\u{2554}',
            tr: '\u{2557}',
            bl: '\u{255A}',
            br: '\u{255D}',
            horiz: '\u{2550}',
            vert: '\u{2551}',
            t_down: '\u{2566}',
            t_up: '\u{2569}',
            t_right: '\u{2560}',
            t_left: '\u{2563}',
            cross: '\u{256C}',
        },
        BorderStyle::Heavy => TableChars {
            tl: '\u{2588}',
            tr: '\u{2588}',
            bl: '\u{2588}',
            br: '\u{2588}',
            horiz: '\u{2588}',
            vert: '\u{2588}',
            t_down: '\u{2588}',
            t_up: '\u{2588}',
            t_right: '\u{2588}',
            t_left: '\u{2588}',
            cross: '\u{2588}',
        },
        BorderStyle::Shade => TableChars {
            tl: '\u{2592}',
            tr: '\u{2592}',
            bl: '\u{2592}',
            br: '\u{2592}',
            horiz: '\u{2592}',
            vert: '\u{2592}',
            t_down: '\u{2592}',
            t_up: '\u{2592}',
            t_right: '\u{2592}',
            t_left: '\u{2592}',
            cross: '\u{2592}',
        },
    }
}

/// Build a horizontal rule line: `left` + (fill × col_width+2) + `junction` + ... + `right`.
fn horizontal_line(
    left: char,
    fill: char,
    junction: char,
    right: char,
    col_widths: &[usize],
) -> String {
    let mut line = String::new();
    line.push(left);
    for (i, &w) in col_widths.iter().enumerate() {
        for _ in 0..(w + 2) {
            line.push(fill);
        }
        if i < col_widths.len() - 1 {
            line.push(junction);
        }
    }
    line.push(right);
    line
}

/// Build a data row: `vert` + ` cell ` + `vert` + ... + `vert`.
fn data_row(
    vert: char,
    cells: &[String],
    col_widths: &[usize],
    align: &[ColumnAlign],
    num_cols: usize,
) -> String {
    let mut line = String::new();
    line.push(vert);
    for (i, &w) in col_widths.iter().enumerate().take(num_cols) {
        let cell = cells.get(i).map(|s| s.as_str()).unwrap_or("");
        let truncated = if cell.len() > w { &cell[..w] } else { cell };
        let alignment = align.get(i).copied().unwrap_or(ColumnAlign::Left);
        let padded = match alignment {
            ColumnAlign::Left => format!(" {:<width$} ", truncated, width = w),
            ColumnAlign::Right => format!(" {:>width$} ", truncated, width = w),
            ColumnAlign::Center => format!(" {:^width$} ", truncated, width = w),
        };
        line.push_str(&padded);
        if i < num_cols - 1 {
            line.push(vert);
        }
    }
    line.push(vert);
    line
}

/// Compute column widths distributed proportionally to max content widths.
fn compute_col_widths(num_cols: usize, max_widths: &[usize], total_width: usize) -> Vec<usize> {
    if num_cols == 0 {
        return vec![];
    }

    // Borders: (num_cols + 1) border chars, Padding: 2 per column (1 space each side)
    let overhead = (num_cols + 1) + (2 * num_cols);
    let available = total_width.saturating_sub(overhead);

    if available == 0 {
        return vec![0; num_cols];
    }

    let total_content: usize = max_widths.iter().sum();

    if total_content == 0 {
        // Equal distribution when no content
        let each = available / num_cols;
        let remainder = available % num_cols;
        let mut widths = vec![each; num_cols];
        for w in widths.iter_mut().take(remainder) {
            *w += 1;
        }
        return widths;
    }

    // Proportional distribution
    let mut widths = vec![0usize; num_cols];
    let mut assigned = 0;

    for i in 0..num_cols {
        widths[i] = (max_widths[i] * available) / total_content;
        if widths[i] == 0 && max_widths[i] > 0 && available > assigned {
            widths[i] = 1;
        }
        assigned += widths[i];
    }

    // Distribute remainder to widest columns first
    let mut remainder = available.saturating_sub(assigned);
    if remainder > 0 {
        let mut indices: Vec<usize> = (0..num_cols).collect();
        indices.sort_by(|&a, &b| max_widths[b].cmp(&max_widths[a]));
        for &i in &indices {
            if remainder == 0 {
                break;
            }
            widths[i] += 1;
            remainder -= 1;
        }
    }

    widths
}

impl Table {
    /// Emit IR ops for this table component.
    pub fn emit(&self, ctx: &mut EmitContext) {
        let total_width = self.width.unwrap_or(ctx.chars_per_line());

        // Determine number of columns
        let num_cols = {
            let from_headers = self.headers.as_ref().map(|h| h.len()).unwrap_or(0);
            let from_rows = self.rows.iter().map(|r| r.len()).max().unwrap_or(0);
            from_headers.max(from_rows)
        };

        if num_cols == 0 {
            return;
        }

        // Compute max content width per column
        let mut max_widths = vec![0usize; num_cols];
        if let Some(ref headers) = self.headers {
            for (i, h) in headers.iter().enumerate() {
                max_widths[i] = max_widths[i].max(h.len());
            }
        }
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < num_cols {
                    max_widths[i] = max_widths[i].max(cell.len());
                }
            }
        }

        let col_widths = compute_col_widths(num_cols, &max_widths, total_width);
        let chars = table_chars(self.border);

        ctx.push(Op::SetFont(Font::A));
        ctx.push(Op::SetAlign(Alignment::Left));

        // Top border: ┌──┬──┐
        let top = horizontal_line(chars.tl, chars.horiz, chars.t_down, chars.tr, &col_widths);
        ctx.push(Op::Text(top));
        ctx.push(Op::Newline);

        // Header row
        if let Some(ref headers) = self.headers {
            ctx.push(Op::SetBold(true));
            let header_row = data_row(chars.vert, headers, &col_widths, &self.align, num_cols);
            ctx.push(Op::Text(header_row));
            ctx.push(Op::Newline);
            ctx.push(Op::SetBold(false));

            // Header separator: ├──┼──┤ or ╞══╪══╡ for mixed
            let sep = if matches!(self.border, BorderStyle::Mixed) {
                horizontal_line('\u{255E}', '\u{2550}', '\u{256A}', '\u{2561}', &col_widths)
            } else {
                horizontal_line(
                    chars.t_right,
                    chars.horiz,
                    chars.cross,
                    chars.t_left,
                    &col_widths,
                )
            };
            ctx.push(Op::Text(sep));
            ctx.push(Op::Newline);
        }

        // Data rows
        for (i, row) in self.rows.iter().enumerate() {
            let row_text = data_row(chars.vert, row, &col_widths, &self.align, num_cols);
            ctx.push(Op::Text(row_text));
            ctx.push(Op::Newline);

            // Row separator between rows, not after last
            if self.row_separator && i < self.rows.len() - 1 {
                let sep = horizontal_line(
                    chars.t_right,
                    chars.horiz,
                    chars.cross,
                    chars.t_left,
                    &col_widths,
                );
                ctx.push(Op::Text(sep));
                ctx.push(Op::Newline);
            }
        }

        // Bottom border: └──┴──┘
        let bottom = horizontal_line(chars.bl, chars.horiz, chars.t_up, chars.br, &col_widths);
        ctx.push(Op::Text(bottom));
        ctx.push(Op::Newline);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EmitContext {
        EmitContext::new(576)
    }

    #[test]
    fn test_dashed_divider() {
        let div = Divider {
            style: DividerStyle::Dashed,
            width: Some(10),
        };
        let mut ctx = ctx();
        div.emit(&mut ctx);
        assert!(ctx.ops.iter().any(|op| *op == Op::Text("----------".into())));
    }

    #[test]
    fn test_equals_divider() {
        let div = Divider {
            style: DividerStyle::Equals,
            width: Some(5),
        };
        let mut ctx = ctx();
        div.emit(&mut ctx);
        assert!(ctx.ops.iter().any(|op| *op == Op::Text("=====".into())));
    }

    #[test]
    fn test_spacer_mm() {
        let spacer = Spacer::mm(5.0);
        let mut ctx = ctx();
        spacer.emit(&mut ctx);
        assert!(ctx.ops.iter().any(|op| *op == Op::Feed { units: 20 }));
    }

    #[test]
    fn test_spacer_lines() {
        let spacer = Spacer::lines(2);
        let mut ctx = ctx();
        spacer.emit(&mut ctx);
        assert!(ctx.ops.iter().any(|op| *op == Op::Feed { units: 24 }));
    }

    #[test]
    fn test_columns() {
        let cols = Columns {
            left: "Left".into(),
            right: "Right".into(),
            width: Some(20),
            ..Default::default()
        };
        let mut ctx = ctx();
        cols.emit(&mut ctx);
        let has_columns = ctx.ops.iter().any(|op| {
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
        let mut ctx = ctx();
        cols.emit(&mut ctx);
        assert!(ctx.ops.contains(&Op::SetBold(true)));
        assert!(ctx.ops.contains(&Op::SetBold(false)));
    }

    #[test]
    fn test_blank_line() {
        let blank = BlankLine {};
        let mut ctx = ctx();
        blank.emit(&mut ctx);
        assert!(ctx.ops.iter().any(|op| *op == Op::Newline));
    }

    // ========================================================================
    // Banner tests
    // ========================================================================

    #[test]
    fn test_banner_fit_short_text() {
        // "HELLO" (5 chars) fits at size 3×3 (16 chars/line, 14 usable)
        let (size, total) = Banner::fit(5, 3, BorderStyle::Single, 576);
        assert_eq!(size, [3, 3]);
        assert_eq!(total, 16);
    }

    #[test]
    fn test_banner_fit_medium_text() {
        // 15 chars won't fit at 3×3 (14 usable) but fits at 3×2 (22 usable)
        let (size, total) = Banner::fit(15, 3, BorderStyle::Single, 576);
        assert_eq!(size, [3, 2]);
        assert_eq!(total, 24);
    }

    #[test]
    fn test_banner_fit_long_text() {
        // 47 chars won't fit at 3×1 (46 usable) → Font B (62 usable)
        let (size, total) = Banner::fit(47, 3, BorderStyle::Single, 576);
        assert_eq!(size, [0, 0]);
        assert_eq!(total, 64);
    }

    #[test]
    fn test_banner_fit_size_0() {
        // max_size 0 → always Font B
        let (size, total) = Banner::fit(5, 0, BorderStyle::Single, 576);
        assert_eq!(size, [0, 0]);
        assert_eq!(total, 64);
    }

    #[test]
    fn test_banner_emit_basic() {
        let banner = Banner::new("TEST");
        let mut ctx = ctx();
        banner.emit(&mut ctx);

        // Should have font, alignment, size set, bold, and box-drawing text
        assert!(ctx.ops.contains(&Op::SetFont(Font::A)));
        assert!(ctx.ops.contains(&Op::SetAlign(Alignment::Left)));
        assert!(ctx.ops.contains(&Op::SetBold(true)));
        assert!(ctx.ops.contains(&Op::SetBold(false)));

        // Should have the content text with the border chars
        let has_content = ctx.ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.contains("TEST")
            } else {
                false
            }
        });
        assert!(has_content, "Banner should contain the content text");

        // Should have box-drawing border
        let has_top_border = ctx.ops.iter().any(|op| {
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
        let mut ctx = ctx();
        banner.emit(&mut ctx);

        let has_double_top = ctx.ops.iter().any(|op| {
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
        let mut ctx = ctx();
        banner.emit(&mut ctx);
        assert!(ctx.ops.contains(&Op::SetFont(Font::B)));
        assert!(!ctx.ops.iter().any(|op| matches!(op, Op::SetSize { .. })));
    }

    #[test]
    fn test_banner_heavy_border() {
        let banner = Banner {
            content: "HI".into(),
            border: BorderStyle::Heavy,
            ..Default::default()
        };
        let mut ctx = ctx();
        banner.emit(&mut ctx);

        // Top line should be all full-block chars (█)
        let first_text = ctx.ops
            .iter()
            .find_map(|op| {
                if let Op::Text(s) = op {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert!(
            first_text.chars().all(|c| c == '\u{2588}'),
            "Heavy border top should be all full-block chars"
        );
    }

    #[test]
    fn test_banner_shade_border() {
        let banner = Banner {
            content: "HI".into(),
            border: BorderStyle::Shade,
            ..Default::default()
        };
        let mut ctx = ctx();
        banner.emit(&mut ctx);

        // Top line should be all medium-shade chars (▒)
        let first_text = ctx.ops
            .iter()
            .find_map(|op| {
                if let Op::Text(s) = op {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert!(
            first_text.chars().all(|c| c == '\u{2592}'),
            "Shade border top should be all medium-shade chars"
        );
    }

    #[test]
    fn test_banner_shadow_border() {
        let banner = Banner {
            content: "HI".into(),
            border: BorderStyle::Shadow,
            ..Default::default()
        };
        let mut ctx = ctx();
        banner.emit(&mut ctx);

        // Top border: single-line, no shadow
        let first_text = ctx.ops
            .iter()
            .find_map(|op| {
                if let Op::Text(s) = op {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert!(
            first_text.starts_with('\u{250C}'),
            "Shadow top should start with ┌"
        );
        assert!(
            first_text.ends_with('\u{2510}'),
            "Shadow top should end with ┐ (no shadow)"
        );

        // Content line should end with shadow char ▓
        let content_text = ctx.ops
            .iter()
            .find_map(|op| {
                if let Op::Text(s) = op {
                    if s.contains("HI") {
                        Some(s.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap();
        assert!(
            content_text.ends_with('\u{2593}'),
            "Shadow content line should end with ▓"
        );

        // Last text op should be the shadow bottom row
        let last_text = ctx.ops
            .iter()
            .rev()
            .find_map(|op| {
                if let Op::Text(s) = op {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert!(
            last_text.starts_with(' '),
            "Shadow bottom row starts with space"
        );
        assert!(
            last_text.chars().skip(1).all(|c| c == '\u{2593}'),
            "Shadow bottom row should be all ▓ after leading space"
        );
    }

    #[test]
    fn test_banner_fit_shadow_overhead() {
        // Shadow has overhead of 3 instead of 2
        // At size 3×3: 16 chars/line, 13 usable (16-3)
        // Content of 14 won't fit at 3×3 but fits at 3×2 (24-3 = 21 usable)
        let (size, total) = Banner::fit(14, 3, BorderStyle::Shadow, 576);
        assert_eq!(size, [3, 2]);
        assert_eq!(total, 24);

        // Content of 13 fits at 3×3 (16-3 = 13 usable)
        let (size, total) = Banner::fit(13, 3, BorderStyle::Shadow, 576);
        assert_eq!(size, [3, 3]);
        assert_eq!(total, 16);
    }

    #[test]
    fn test_banner_mixed_border() {
        // Mixed border should render the same as Single for banners
        let banner = Banner {
            content: "HI".into(),
            border: BorderStyle::Mixed,
            ..Default::default()
        };
        let mut ctx = ctx();
        banner.emit(&mut ctx);
        let has_single_top = ctx.ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.starts_with('\u{250C}') && s.ends_with('\u{2510}')
            } else {
                false
            }
        });
        assert!(has_single_top, "Mixed banner should use single-line border");
    }

    #[test]
    fn test_banner_rule() {
        let banner = Banner {
            content: "WEATHER".into(),
            border: BorderStyle::Rule,
            size: 2,
            ..Default::default()
        };
        let mut ctx = ctx();
        banner.emit(&mut ctx);

        // Should produce a single text line + newline (plus style ops)
        let texts: Vec<&str> = ctx.ops
            .iter()
            .filter_map(|op| {
                if let Op::Text(s) = op {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            texts.len(),
            1,
            "Rule banner should emit exactly 1 text line"
        );

        let line = texts[0];
        assert!(
            line.contains("WEATHER"),
            "Rule line should contain the text"
        );
        assert!(
            line.contains('\u{2500}'),
            "Rule line should contain ─ characters"
        );
        // Text should be surrounded by spaces
        assert!(
            line.contains(" WEATHER "),
            "Text should have spaces around it"
        );
    }

    #[test]
    fn test_banner_heading() {
        let banner = Banner {
            content: "FORECAST".into(),
            border: BorderStyle::Heading,
            size: 2,
            ..Default::default()
        };
        let mut ctx = ctx();
        banner.emit(&mut ctx);

        let texts: Vec<&str> = ctx.ops
            .iter()
            .filter_map(|op| {
                if let Op::Text(s) = op {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(texts.len(), 2, "Heading banner should emit 2 text lines");
        assert_eq!(texts[0], "FORECAST");
        // Second line is all ─
        assert!(
            texts[1].chars().all(|c| c == '\u{2500}'),
            "Second line should be all ─"
        );

        // Should set center alignment for text
        assert!(ctx.ops.contains(&Op::SetAlign(Alignment::Center)));
        // Should reset to left for the rule
        assert!(
            ctx.ops.iter()
                .filter(|op| matches!(op, Op::SetAlign(Alignment::Left)))
                .count()
                >= 1
        );
    }

    #[test]
    fn test_banner_tag() {
        let banner = Banner {
            content: "GROCERIES".into(),
            border: BorderStyle::Tag,
            size: 2,
            ..Default::default()
        };
        let mut ctx = ctx();
        banner.emit(&mut ctx);

        let texts: Vec<&str> = ctx.ops
            .iter()
            .filter_map(|op| {
                if let Op::Text(s) = op {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(texts.len(), 1, "Tag banner should emit exactly 1 text line");
        assert_eq!(texts[0], "\u{25A0} GROCERIES");
    }

    #[test]
    fn test_banner_fit_rule() {
        // Rule has 0 border overhead
        let (size, total) = Banner::fit(5, 3, BorderStyle::Rule, 576);
        assert_eq!(size, [3, 3]);
        assert_eq!(total, 16);
        // All 16 chars usable (no border deduction)
        let usable = total; // 0 overhead
        assert!(5 <= usable);
    }

    #[test]
    fn test_banner_fit_tag() {
        // Tag has 2 chars overhead ("■ " prefix)
        let (size, total) = Banner::fit(5, 3, BorderStyle::Tag, 576);
        assert_eq!(size, [3, 3]);
        assert_eq!(total, 16);
    }

    // ========================================================================
    // Table tests
    // ========================================================================

    #[test]
    fn test_table_basic() {
        let table = Table {
            rows: vec![vec!["A".into(), "B".into()], vec!["C".into(), "D".into()]],
            width: Some(20),
            ..Default::default()
        };
        let mut ctx = ctx();
        table.emit(&mut ctx);

        // Should set Font A and Left alignment
        assert!(ctx.ops.contains(&Op::SetFont(Font::A)));
        assert!(ctx.ops.contains(&Op::SetAlign(Alignment::Left)));

        // Collect all text ops
        let texts: Vec<&str> = ctx.ops
            .iter()
            .filter_map(|op| {
                if let Op::Text(s) = op {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();

        // Top border: ┌...┬...┐
        assert!(texts[0].starts_with('\u{250C}'), "Top starts with ┌");
        assert!(texts[0].ends_with('\u{2510}'), "Top ends with ┐");
        assert!(texts[0].contains('\u{252C}'), "Top has ┬ junction");

        // Data rows contain cells
        assert!(texts[1].contains("A"));
        assert!(texts[1].contains("B"));
        assert!(texts[2].contains("C"));
        assert!(texts[2].contains("D"));

        // Bottom border: └...┴...┘
        let last = texts.last().unwrap();
        assert!(last.starts_with('\u{2514}'), "Bottom starts with └");
        assert!(last.ends_with('\u{2518}'), "Bottom ends with ┘");
        assert!(last.contains('\u{2534}'), "Bottom has ┴ junction");
    }

    #[test]
    fn test_table_with_headers() {
        let table = Table {
            headers: Some(vec!["Name".into(), "Price".into()]),
            rows: vec![vec!["Coffee".into(), "$4.50".into()]],
            width: Some(30),
            ..Default::default()
        };
        let mut ctx = ctx();
        table.emit(&mut ctx);

        // Header should be bold
        assert!(ctx.ops.contains(&Op::SetBold(true)));
        assert!(ctx.ops.contains(&Op::SetBold(false)));

        // Should have header separator (├...┼...┤)
        let has_separator = ctx.ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.starts_with('\u{251C}') && s.contains('\u{253C}') && s.ends_with('\u{2524}')
            } else {
                false
            }
        });
        assert!(has_separator, "Should have header separator ├┼┤");
    }

    #[test]
    fn test_table_column_widths() {
        // Proportional: "LongColumn" (10) vs "B" (1) → 10:1 ratio
        let widths = compute_col_widths(2, &[10, 1], 20);
        assert!(widths[0] > widths[1], "Wider content gets more space");
        assert_eq!(
            widths.iter().sum::<usize>(),
            20 - 3 - 4, // total - 3 borders - 4 padding
            "Widths sum to available space"
        );
    }

    #[test]
    fn test_table_alignment() {
        let table = Table {
            rows: vec![vec!["L".into(), "C".into(), "R".into()]],
            align: vec![ColumnAlign::Left, ColumnAlign::Center, ColumnAlign::Right],
            width: Some(30),
            ..Default::default()
        };
        let mut ctx = ctx();
        table.emit(&mut ctx);

        let row = ctx.ops
            .iter()
            .find_map(|op| {
                if let Op::Text(s) = op {
                    if s.contains("L") && s.contains("C") && s.contains("R") {
                        Some(s.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .expect("Should have data row");

        // Split on │ to get cells
        let cells: Vec<&str> = row.split('\u{2502}').collect();
        // First and last are empty (before first │ and after last │)
        assert!(
            cells[1].starts_with(" L"),
            "Left-aligned cell starts with ' L'"
        );
        assert!(
            cells[3].ends_with("R "),
            "Right-aligned cell ends with 'R '"
        );
    }

    #[test]
    fn test_table_double_border() {
        let table = Table {
            rows: vec![vec!["X".into()]],
            border: BorderStyle::Double,
            width: Some(20),
            ..Default::default()
        };
        let mut ctx = ctx();
        table.emit(&mut ctx);

        let texts: Vec<&str> = ctx.ops
            .iter()
            .filter_map(|op| {
                if let Op::Text(s) = op {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();

        assert!(texts[0].starts_with('\u{2554}'), "Double top starts with ╔");
        assert!(texts[0].ends_with('\u{2557}'), "Double top ends with ╗");
        assert!(texts[0].contains('\u{2550}'), "Double top has ═ fill");
    }

    #[test]
    fn test_table_mixed_border() {
        let table = Table {
            headers: Some(vec!["H1".into(), "H2".into()]),
            rows: vec![vec!["A".into(), "B".into()]],
            border: BorderStyle::Mixed,
            width: Some(20),
            ..Default::default()
        };
        let mut ctx = ctx();
        table.emit(&mut ctx);

        // Top border should be single (┌)
        let texts: Vec<&str> = ctx.ops
            .iter()
            .filter_map(|op| {
                if let Op::Text(s) = op {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(texts[0].starts_with('\u{250C}'), "Mixed top uses single ┌");

        // Header separator should use mixed chars ╞═╪═╡
        let has_mixed_sep = ctx.ops.iter().any(|op| {
            if let Op::Text(s) = op {
                s.starts_with('\u{255E}') && s.contains('\u{256A}') && s.ends_with('\u{2561}')
            } else {
                false
            }
        });
        assert!(
            has_mixed_sep,
            "Mixed border should use ╞═╪═╡ header separator"
        );
    }

    #[test]
    fn test_table_row_separator() {
        let table = Table {
            rows: vec![vec!["A".into()], vec!["B".into()], vec!["C".into()]],
            row_separator: true,
            width: Some(20),
            ..Default::default()
        };
        let mut ctx = ctx();
        table.emit(&mut ctx);

        // Count separator lines (├...┤) — should be 2 (between 3 rows)
        let sep_count = ctx.ops
            .iter()
            .filter(|op| {
                if let Op::Text(s) = op {
                    s.starts_with('\u{251C}') && s.ends_with('\u{2524}')
                } else {
                    false
                }
            })
            .count();
        assert_eq!(sep_count, 2, "Should have 2 row separators between 3 rows");
    }

    #[test]
    fn test_table_uneven_rows() {
        // Rows with different numbers of cells
        let table = Table {
            headers: Some(vec!["A".into(), "B".into(), "C".into()]),
            rows: vec![
                vec!["1".into()],                         // 1 cell, 3 columns
                vec!["1".into(), "2".into(), "3".into()], // 3 cells
            ],
            width: Some(30),
            ..Default::default()
        };
        let mut ctx = ctx();
        table.emit(&mut ctx);

        // Short row should still produce a valid line with 3 │ separators
        let first_data_row = ctx.ops.iter().find_map(|op| {
            if let Op::Text(s) = op {
                if s.contains("1") && !s.contains("2") && s.starts_with('\u{2502}') {
                    Some(s.clone())
                } else {
                    None
                }
            } else {
                None
            }
        });
        assert!(first_data_row.is_some(), "Short row should still render");
    }

    #[test]
    fn test_table_empty_rows() {
        let table = Table {
            rows: vec![],
            width: Some(20),
            ..Default::default()
        };
        let mut ctx = ctx();
        table.emit(&mut ctx);
        // Empty table with no rows and no headers → no output
        assert!(ctx.ops.is_empty(), "Empty table should produce no ops");
    }
}
