//! Text rendering for preview.
//!
//! Implements character and text rendering with support for various styles.

use super::font::{generate_glyph, FontMetrics};
use super::PreviewRenderer;
use crate::protocol::text::{Alignment, Font};

impl PreviewRenderer {
    /// Render text with current style.
    pub(super) fn render_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        let char_width = self.state.effective_char_width();
        let char_height = self.state.effective_char_height();
        let line_height = char_height;

        // For upside-down text, reverse the character order so it reads correctly when flipped
        let chars: Vec<char> = if self.state.style.upside_down {
            text.chars().rev().collect()
        } else {
            text.chars().collect()
        };

        // Only apply alignment if we're at the start of a line (x == 0)
        // Otherwise, continue from current position
        if self.state.x == 0 {
            // Calculate text width for alignment
            let text_width = chars.len() * char_width;

            // Calculate starting x based on alignment (within print area)
            let start_x = match self.state.style.alignment {
                Alignment::Left => 0,
                Alignment::Center => {
                    if text_width < self.print_width {
                        (self.print_width - text_width) / 2
                    } else {
                        0
                    }
                }
                Alignment::Right => {
                    if text_width < self.print_width {
                        self.print_width - text_width
                    } else {
                        0
                    }
                }
            };

            self.state.x = start_x;
        }
        // else: continue from current x position

        // Ensure we have room for the text
        self.ensure_height(self.state.y + line_height);

        // Render each character
        for ch in chars {
            if ch == '\n' {
                self.state.x = 0;
                self.state.y += line_height;
                self.ensure_height(self.state.y + line_height);
                continue;
            }

            if self.state.x + char_width > self.print_width {
                // Wrap to next line
                self.state.x = 0;
                self.state.y += line_height;
                self.ensure_height(self.state.y + line_height);
            }

            self.render_char(ch);
            self.state.x += char_width;
        }
    }

    /// Render a single character at current position.
    /// state.x is in print coordinates (0 = left edge of printable area).
    fn render_char(&mut self, ch: char) {
        let font = self.state.style.font;
        let metrics = FontMetrics::for_font(font);
        let width_mult = self.state.total_width_mult();
        let height_mult = self.state.total_height_mult();

        // Get or generate the base glyph
        let glyph = self.get_glyph(font, ch);

        let base_x = self.state.x;
        let base_y = self.state.y;
        let char_pixel_width = metrics.char_width * width_mult;
        let char_pixel_height = metrics.char_height * height_mult;

        // Fill background first if inverted (black background)
        if self.state.style.invert {
            for py in base_y..(base_y + char_pixel_height) {
                for px in base_x..(base_x + char_pixel_width) {
                    self.set_print_pixel(px, py, true);
                }
            }
        }

        // Draw the glyph with scaling
        let upside_down = self.state.style.upside_down;
        for gy in 0..metrics.char_height {
            for gx in 0..metrics.char_width {
                let idx = gy * metrics.char_width + gx;
                let pixel_on = glyph.get(idx).copied().unwrap_or(0) != 0;

                // For inverted text, we draw white (false) where glyph is on
                // For normal text, we draw black (true) where glyph is on
                let draw_pixel = if self.state.style.invert {
                    !pixel_on // Draw white where glyph pixels are
                } else {
                    pixel_on // Draw black where glyph pixels are
                };

                // Only draw if there's something to draw
                // (for invert, we already filled background, now we "erase" the glyph shape)
                // (for normal, we just draw the black pixels)
                if (self.state.style.invert && pixel_on) || (!self.state.style.invert && pixel_on) {
                    // Calculate destination coordinates for upside-down (180° rotation)
                    // Flip both X and Y to achieve full 180° rotation
                    let dest_gx = if upside_down {
                        metrics.char_width - 1 - gx
                    } else {
                        gx
                    };
                    let dest_gy = if upside_down {
                        metrics.char_height - 1 - gy
                    } else {
                        gy
                    };

                    // Draw scaled pixel
                    for sy in 0..height_mult {
                        for sx in 0..width_mult {
                            let px = base_x + dest_gx * width_mult + sx;
                            let py = base_y + dest_gy * height_mult + sy;
                            self.set_print_pixel(px, py, draw_pixel);
                        }
                    }
                }
            }
        }

        // Draw bold (double-strike effect)
        if self.state.style.bold {
            for gy in 0..metrics.char_height {
                for gx in 0..metrics.char_width {
                    let idx = gy * metrics.char_width + gx;
                    let pixel_on = glyph.get(idx).copied().unwrap_or(0) != 0;

                    if pixel_on {
                        // Flip both X and Y for upside-down (180° rotation)
                        let dest_gx = if upside_down {
                            metrics.char_width - 1 - gx
                        } else {
                            gx
                        };
                        let dest_gy = if upside_down {
                            metrics.char_height - 1 - gy
                        } else {
                            gy
                        };

                        let draw_pixel = !self.state.style.invert;

                        for sy in 0..height_mult {
                            for sx in 0..width_mult {
                                // Offset by 1 pixel for bold effect
                                let offset = 1i32;
                                let px = (base_x + dest_gx * width_mult + sx) as i32 + offset;
                                let py = base_y + dest_gy * height_mult + sy;
                                if px >= 0 && (px as usize) < self.print_width {
                                    self.set_print_pixel(px as usize, py, draw_pixel);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Draw underline (at bottom for normal, at top for upside-down)
        if self.state.style.underline {
            let underline_y = if upside_down {
                base_y + 1
            } else {
                base_y + char_pixel_height - 2
            };
            for sx in 0..char_pixel_width {
                let px = base_x + sx;
                self.set_print_pixel(px, underline_y, !self.state.style.invert);
                self.set_print_pixel(px, underline_y + 1, !self.state.style.invert);
            }
        }

        // Draw upperline (at top for normal, at bottom for upside-down)
        if self.state.style.upperline {
            let upperline_y = if upside_down {
                base_y + char_pixel_height - 2
            } else {
                base_y
            };
            for sx in 0..char_pixel_width {
                let px = base_x + sx;
                self.set_print_pixel(px, upperline_y, !self.state.style.invert);
                self.set_print_pixel(px, upperline_y + 1, !self.state.style.invert);
            }
        }
    }

    /// Get or generate a glyph for the given font and character.
    fn get_glyph(&mut self, font: Font, ch: char) -> Vec<u8> {
        let key = (font, ch);
        if let Some(glyph) = self.font_cache.get(&key) {
            return glyph.to_vec();
        }

        let glyph = generate_glyph(font, ch);
        self.font_cache.insert(key, glyph.clone());
        glyph
    }

    /// Move to next line.
    pub(super) fn newline(&mut self) {
        self.state.x = 0;
        self.state.y += self.state.line_height();
        self.ensure_height(self.state.y);
    }
}
