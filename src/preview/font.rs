//! Font metrics and glyph generation for preview rendering.
//!
//! Uses the Spleen bitmap font family for high-quality text rendering.

use crate::ir::StyleState;
use crate::protocol::text::Font;
use spleen_font::{PSF2Font, FONT_12X24, FONT_6X12};

/// Font dimensions for each font type.
#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    pub char_width: usize,
    pub char_height: usize,
    pub chars_per_line: usize,
}

impl FontMetrics {
    pub const FONT_A: FontMetrics = FontMetrics {
        char_width: 12,
        char_height: 24,
        chars_per_line: 48,
    };

    pub const FONT_B: FontMetrics = FontMetrics {
        char_width: 9,
        char_height: 24,
        chars_per_line: 64,
    };

    pub const FONT_C: FontMetrics = FontMetrics {
        char_width: 9,
        char_height: 17,
        chars_per_line: 64,
    };

    pub fn for_font(font: Font) -> FontMetrics {
        match font {
            Font::A => Self::FONT_A,
            Font::B => Self::FONT_B,
            Font::C => Self::FONT_C,
        }
    }
}

/// Render state tracking current style and position.
#[derive(Debug, Clone)]
pub struct RenderState {
    pub style: StyleState,
    pub x: usize,
    pub y: usize,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            style: StyleState::default(),
            x: 0,
            y: 0,
        }
    }
}

impl RenderState {
    pub fn reset(&mut self) {
        self.style = StyleState::default();
        self.x = 0;
        // Note: y position is not reset by Init
    }

    pub fn font_metrics(&self) -> FontMetrics {
        FontMetrics::for_font(self.style.font)
    }

    /// Get effective character width with size multipliers.
    pub fn effective_char_width(&self) -> usize {
        let base = self.font_metrics().char_width;
        let mult = self.total_width_mult();
        base * mult
    }

    /// Get effective character height with size multipliers.
    pub fn effective_char_height(&self) -> usize {
        let base = self.font_metrics().char_height;
        let mult = self.total_height_mult();
        base * mult
    }

    /// Get total width multiplier (combining size and expanded width).
    pub fn total_width_mult(&self) -> usize {
        let size_mult = (self.style.width_mult as usize) + 1;
        let expanded_mult = (self.style.expanded_width as usize) + 1;
        size_mult.max(expanded_mult)
    }

    /// Get total height multiplier (combining size and expanded height).
    pub fn total_height_mult(&self) -> usize {
        let size_mult = (self.style.height_mult as usize) + 1;
        let expanded_mult = (self.style.expanded_height as usize) + 1;
        size_mult.max(expanded_mult)
    }

    /// Get line height (character height).
    pub fn line_height(&self) -> usize {
        self.effective_char_height()
    }
}

/// Generate a glyph bitmap for a character using Spleen font.
/// Returns a Vec<u8> where each byte is 0 (white) or 1 (black).
pub fn generate_glyph(font: Font, ch: char) -> Vec<u8> {
    let metrics = FontMetrics::for_font(font);
    let mut glyph = vec![0u8; metrics.char_width * metrics.char_height];

    // Select appropriate Spleen font based on target font
    // Font A: 12×24 → use Spleen 12x24 directly (exact match)
    // Font B: 9×24 → use Spleen 6x12 scaled 1.5x ≈ 9×18, then vertically stretched to 24
    // Font C: 9×17 → use Spleen 6x12 scaled 1.5x ≈ 9×18, then cropped to 17
    match font {
        Font::A => {
            // Use Spleen 12x24 directly (exact match)
            let mut spleen = PSF2Font::new(FONT_12X24).unwrap();
            let utf8_bytes = ch.to_string();

            if let Some(spleen_glyph) = spleen.glyph_for_utf8(utf8_bytes.as_bytes()) {
                for (row_y, row) in spleen_glyph.enumerate() {
                    for (col_x, on) in row.enumerate() {
                        let idx = row_y * metrics.char_width + col_x;
                        if idx < glyph.len() {
                            glyph[idx] = if on { 1 } else { 0 };
                        }
                    }
                }
            } else if let Some(fb) = fallback_glyph(ch, metrics.char_width, metrics.char_height) {
                glyph = fb;
            } else {
                // Fallback: draw a box for unknown chars
                draw_box(&mut glyph, metrics.char_width, metrics.char_height);
            }
        }
        Font::B | Font::C => {
            // Use Spleen 6x12 and scale to target size
            let mut spleen = PSF2Font::new(FONT_6X12).unwrap();
            let utf8_bytes = ch.to_string();

            if let Some(spleen_glyph) = spleen.glyph_for_utf8(utf8_bytes.as_bytes()) {
                // Collect the 6x12 bitmap
                let mut src_bitmap = vec![0u8; 6 * 12];
                for (row_y, row) in spleen_glyph.enumerate() {
                    for (col_x, on) in row.enumerate() {
                        if row_y < 12 && col_x < 6 {
                            src_bitmap[row_y * 6 + col_x] = if on { 1 } else { 0 };
                        }
                    }
                }

                // Scale from 6x12 to target size using nearest neighbor
                scale_bitmap(&src_bitmap, 6, 12, &mut glyph, metrics.char_width, metrics.char_height);
            } else if let Some(fb) = fallback_glyph(ch, metrics.char_width, metrics.char_height) {
                glyph = fb;
            } else {
                // Fallback: draw a box for unknown chars
                draw_box(&mut glyph, metrics.char_width, metrics.char_height);
            }
        }
    }

    glyph
}

/// Scale a bitmap from src dimensions to dst dimensions using nearest neighbor.
fn scale_bitmap(
    src: &[u8],
    src_w: usize,
    src_h: usize,
    dst: &mut [u8],
    dst_w: usize,
    dst_h: usize,
) {
    for dy in 0..dst_h {
        for dx in 0..dst_w {
            let sx = dx * src_w / dst_w;
            let sy = dy * src_h / dst_h;
            let src_idx = sy * src_w + sx;
            let dst_idx = dy * dst_w + dx;
            if src_idx < src.len() && dst_idx < dst.len() {
                dst[dst_idx] = src[src_idx];
            }
        }
    }
}

/// Draw a box outline in the glyph buffer.
fn draw_box(glyph: &mut [u8], width: usize, height: usize) {
    for x in 0..width {
        glyph[x] = 1;
        glyph[(height - 1) * width + x] = 1;
    }
    for y in 0..height {
        glyph[y * width] = 1;
        glyph[y * width + width - 1] = 1;
    }
}

/// Fill a rectangular region in a glyph buffer. Coordinates are clamped to bounds.
fn fill_rect(g: &mut [u8], stride: usize, x1: usize, y1: usize, x2: usize, y2: usize) {
    let h = g.len() / stride;
    for y in y1..y2.min(h) {
        for x in x1..x2.min(stride) {
            g[y * stride + x] = 1;
        }
    }
}

/// Procedurally draw fallback glyphs for characters missing from the Spleen font.
/// Returns None if the character has no fallback, falling through to draw_box().
fn fallback_glyph(ch: char, w: usize, h: usize) -> Option<Vec<u8>> {
    let mut g = vec![0u8; w * h];
    let cx = w / 2;
    let cy = h / 2;

    // Single box-drawing line parameters (2px thickness)
    let s1 = cx.saturating_sub(1);
    let s2 = cx + 1;
    let t1 = cy.saturating_sub(1);
    let t2 = cy + 1;

    // Double box-drawing line parameters (2px thickness each, 2px gap)
    let dl = cx.saturating_sub(3); // double left
    let dr = cx.saturating_sub(1); // double left end
    let dl2 = cx + 1;              // double right start
    let dr2 = cx + 3;              // double right end
    let dt = cy.saturating_sub(3); // double top
    let db = cy.saturating_sub(1); // double top end
    let dt2 = cy + 1;              // double bottom start
    let db2 = cy + 3;              // double bottom end

    match ch {
        // ── Single box-drawing ──
        '\u{2500}' => { // ─ horizontal
            fill_rect(&mut g, w, 0, t1, w, t2);
        }
        '\u{2502}' => { // │ vertical
            fill_rect(&mut g, w, s1, 0, s2, h);
        }
        '\u{250C}' => { // ┌ top-left
            fill_rect(&mut g, w, s1, t1, w, t2);
            fill_rect(&mut g, w, s1, t1, s2, h);
        }
        '\u{2510}' => { // ┐ top-right
            fill_rect(&mut g, w, 0, t1, s2, t2);
            fill_rect(&mut g, w, s1, t1, s2, h);
        }
        '\u{2514}' => { // └ bottom-left
            fill_rect(&mut g, w, s1, t1, w, t2);
            fill_rect(&mut g, w, s1, 0, s2, t2);
        }
        '\u{2518}' => { // ┘ bottom-right
            fill_rect(&mut g, w, 0, t1, s2, t2);
            fill_rect(&mut g, w, s1, 0, s2, t2);
        }

        // ══ Double box-drawing ══
        '\u{2550}' => { // ═ horizontal
            fill_rect(&mut g, w, 0, dt, w, db);
            fill_rect(&mut g, w, 0, dt2, w, db2);
        }
        '\u{2551}' => { // ║ vertical
            fill_rect(&mut g, w, dl, 0, dr, h);
            fill_rect(&mut g, w, dl2, 0, dr2, h);
        }
        '\u{2554}' => { // ╔ top-left
            fill_rect(&mut g, w, dl, dt, w, db);   // outer horiz
            fill_rect(&mut g, w, dl, dt, dr, h);    // outer vert
            fill_rect(&mut g, w, dl2, dt2, w, db2); // inner horiz
            fill_rect(&mut g, w, dl2, dt2, dr2, h); // inner vert
        }
        '\u{2557}' => { // ╗ top-right
            fill_rect(&mut g, w, 0, dt, dr2, db);   // outer horiz
            fill_rect(&mut g, w, dl2, dt, dr2, h);   // outer vert
            fill_rect(&mut g, w, 0, dt2, dr, db2);   // inner horiz
            fill_rect(&mut g, w, dl, dt2, dr, h);    // inner vert
        }
        '\u{255A}' => { // ╚ bottom-left
            fill_rect(&mut g, w, dl, dt2, w, db2);  // outer horiz
            fill_rect(&mut g, w, dl, 0, dr, db2);   // outer vert
            fill_rect(&mut g, w, dl2, dt, w, db);    // inner horiz
            fill_rect(&mut g, w, dl2, 0, dr2, db);  // inner vert
        }
        '\u{255D}' => { // ╝ bottom-right
            fill_rect(&mut g, w, 0, dt2, dr2, db2); // outer horiz
            fill_rect(&mut g, w, dl2, 0, dr2, db2); // outer vert
            fill_rect(&mut g, w, 0, dt, dr, db);     // inner horiz
            fill_rect(&mut g, w, dl, 0, dr, db);    // inner vert
        }

        // «» Guillemets ──
        '\u{00AB}' | '\u{00BB}' => { // « »
            let top = h / 4;
            let bot = h * 3 / 4;
            let mid = (top + bot) / 2;
            let half = mid - top;
            if half == 0 { return None; }

            // Two chevrons with different x offsets
            let (tip1, arm1, tip2, arm2) = if ch == '\u{00AB}' {
                // « left-pointing: tip on left, arm on right
                (1usize, w / 2 - 1, w / 2, w - 2)
            } else {
                // » right-pointing: tip on right, arm on left
                (w / 2, 1usize, w - 2, w / 2 + 1)
            };

            for y in top..bot {
                let dist = if y < mid { mid - y } else { y - mid + 1 };
                let dist = dist.min(half);

                for (tip, arm) in [(tip1, arm1), (tip2, arm2)] {
                    let x = if ch == '\u{00AB}' {
                        // <: center=tip(left), edges=arm(right)
                        tip + (arm - tip) * dist / half
                    } else {
                        // >: center=tip(right), edges=arm(left)
                        arm + (tip - arm) * (half - dist) / half
                    };
                    // Draw 2px wide
                    if x < w { g[y * w + x] = 1; }
                    if x + 1 < w { g[y * w + x + 1] = 1; }
                }
            }
        }

        // ── Single T-junctions + cross ──
        '\u{252C}' => { // ┬ T-down: full horiz + bottom vert
            fill_rect(&mut g, w, 0, t1, w, t2);
            fill_rect(&mut g, w, s1, t1, s2, h);
        }
        '\u{2534}' => { // ┴ T-up: full horiz + top vert
            fill_rect(&mut g, w, 0, t1, w, t2);
            fill_rect(&mut g, w, s1, 0, s2, t2);
        }
        '\u{251C}' => { // ├ T-right: full vert + right horiz
            fill_rect(&mut g, w, s1, 0, s2, h);
            fill_rect(&mut g, w, s1, t1, w, t2);
        }
        '\u{2524}' => { // ┤ T-left: full vert + left horiz
            fill_rect(&mut g, w, s1, 0, s2, h);
            fill_rect(&mut g, w, 0, t1, s2, t2);
        }
        '\u{253C}' => { // ┼ cross: full horiz + full vert
            fill_rect(&mut g, w, 0, t1, w, t2);
            fill_rect(&mut g, w, s1, 0, s2, h);
        }

        // ══ Double T-junctions + cross ══
        '\u{2566}' => { // ╦ T-down: full double horiz + bottom double vert
            fill_rect(&mut g, w, 0, dt, w, db);
            fill_rect(&mut g, w, 0, dt2, w, db2);
            fill_rect(&mut g, w, dl, dt, dr, h);
            fill_rect(&mut g, w, dl2, dt2, dr2, h);
        }
        '\u{2569}' => { // ╩ T-up: full double horiz + top double vert
            fill_rect(&mut g, w, 0, dt, w, db);
            fill_rect(&mut g, w, 0, dt2, w, db2);
            fill_rect(&mut g, w, dl, 0, dr, db2);
            fill_rect(&mut g, w, dl2, 0, dr2, db);
        }
        '\u{2560}' => { // ╠ T-right: full double vert + right double horiz
            fill_rect(&mut g, w, dl, 0, dr, h);
            fill_rect(&mut g, w, dl2, 0, dr2, h);
            fill_rect(&mut g, w, dl, dt, w, db);
            fill_rect(&mut g, w, dl2, dt2, w, db2);
        }
        '\u{2563}' => { // ╣ T-left: full double vert + left double horiz
            fill_rect(&mut g, w, dl, 0, dr, h);
            fill_rect(&mut g, w, dl2, 0, dr2, h);
            fill_rect(&mut g, w, 0, dt, dr2, db);
            fill_rect(&mut g, w, 0, dt2, dr, db2);
        }
        '\u{256C}' => { // ╬ cross: full double horiz + full double vert
            fill_rect(&mut g, w, 0, dt, w, db);
            fill_rect(&mut g, w, 0, dt2, w, db2);
            fill_rect(&mut g, w, dl, 0, dr, h);
            fill_rect(&mut g, w, dl2, 0, dr2, h);
        }

        // ── Mixed junctions (single vert + double horiz) ──
        '\u{255E}' => { // ╞ single vert + right double horiz
            fill_rect(&mut g, w, s1, 0, s2, h);
            fill_rect(&mut g, w, s1, dt, w, db);
            fill_rect(&mut g, w, s1, dt2, w, db2);
        }
        '\u{2561}' => { // ╡ single vert + left double horiz
            fill_rect(&mut g, w, s1, 0, s2, h);
            fill_rect(&mut g, w, 0, dt, s2, db);
            fill_rect(&mut g, w, 0, dt2, s2, db2);
        }
        '\u{256A}' => { // ╪ single vert + full double horiz
            fill_rect(&mut g, w, s1, 0, s2, h);
            fill_rect(&mut g, w, 0, dt, w, db);
            fill_rect(&mut g, w, 0, dt2, w, db2);
        }

        // ■ Filled square ──
        '\u{25A0}' => {
            let mx = w / 6;
            let my = h / 6;
            fill_rect(&mut g, w, mx, my, w - mx, h - my);
        }

        _ => return None,
    }

    Some(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_metrics() {
        assert_eq!(FontMetrics::FONT_A.char_width, 12);
        assert_eq!(FontMetrics::FONT_A.char_height, 24);
        assert_eq!(FontMetrics::FONT_B.char_width, 9);
        assert_eq!(FontMetrics::FONT_C.char_height, 17);
    }

    #[test]
    fn test_generate_glyph() {
        let glyph = generate_glyph(Font::A, 'A');
        assert_eq!(glyph.len(), 12 * 24);
        // Should have some black pixels
        assert!(glyph.iter().any(|&p| p != 0));
    }

    #[test]
    fn test_spleen_char_coverage() {
        let mut font = PSF2Font::new(FONT_12X24).unwrap();
        let chars = ['«', '»', '░', '▒', '▓', '█', '─', '│', '┌', '°', '■'];
        for ch in &chars {
            let s = ch.to_string();
            let found = font.glyph_for_utf8(s.as_bytes()).is_some();
            eprintln!("{} (U+{:04X}): {}", ch, *ch as u32, if found { "FOUND" } else { "MISSING" });
        }
    }

    #[test]
    fn test_fallback_glyphs_not_boxes() {
        // All characters that should have fallback glyphs (not empty box outlines)
        let chars = [
            '─', '│', '┌', '┐', '└', '┘',       // single box-drawing
            '┬', '┴', '├', '┤', '┼',             // single T-junctions + cross
            '═', '║', '╔', '╗', '╚', '╝',       // double box-drawing
            '╦', '╩', '╠', '╣', '╬',             // double T-junctions + cross
            '╞', '╡', '╪',                        // mixed junctions
            '«', '»',                             // guillemets
            '■',                                   // filled square
        ];

        for ch in &chars {
            let glyph = generate_glyph(Font::A, *ch);
            assert_eq!(glyph.len(), 12 * 24);

            // Should have black pixels (not empty)
            let black_count: usize = glyph.iter().filter(|&&p| p != 0).count();
            assert!(black_count > 0, "{} (U+{:04X}) has no black pixels", ch, *ch as u32);

            // Should NOT be identical to a box outline (draw_box fills edges).
            // Compare against actual draw_box output to verify the fallback was used.
            let mut box_glyph = vec![0u8; 12 * 24];
            draw_box(&mut box_glyph, 12, 24);
            assert_ne!(
                glyph, box_glyph,
                "{} (U+{:04X}) is identical to box fallback",
                ch, *ch as u32
            );
        }
    }

    #[test]
    fn test_fallback_box_drawing_structure() {
        // ─ should have pixels in the center rows, none at top/bottom
        let horiz = generate_glyph(Font::A, '─');
        // Top row should be empty
        assert!(horiz[..12].iter().all(|&p| p == 0), "─ top row should be empty");
        // Center row should have pixels
        let center_row = 11 * 12;
        assert!(horiz[center_row..center_row + 12].iter().any(|&p| p != 0), "─ center should have pixels");

        // │ should have pixels in center columns, none at left/right edges
        let vert = generate_glyph(Font::A, '│');
        // First column should be empty (center is at col 5-6)
        assert!(vert[0] == 0, "│ left edge should be empty");
        // Center column should have pixels
        assert!(vert[5] != 0 || vert[6] != 0, "│ center should have pixels");
    }
}
