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
}
