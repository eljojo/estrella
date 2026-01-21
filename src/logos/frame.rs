//! # Frame Sprites
//!
//! Victorian-style decorative frame elements for borders.
//! Includes corners and tileable edges.

use super::LogoRaster;

/// Corner sprite size
const CORNER_SIZE: u16 = 32;
/// Edge sprite size (for horizontal/vertical bars)
const EDGE_SIZE: u16 = 32;

/// Top-left corner flourish.
pub struct CornerTopLeft;

impl CornerTopLeft {
    pub fn raster() -> LogoRaster {
        corner_sprite(false, false)
    }
}

/// Top-right corner flourish.
pub struct CornerTopRight;

impl CornerTopRight {
    pub fn raster() -> LogoRaster {
        corner_sprite(true, false)
    }
}

/// Bottom-left corner flourish.
pub struct CornerBottomLeft;

impl CornerBottomLeft {
    pub fn raster() -> LogoRaster {
        corner_sprite(false, true)
    }
}

/// Bottom-right corner flourish.
pub struct CornerBottomRight;

impl CornerBottomRight {
    pub fn raster() -> LogoRaster {
        corner_sprite(true, true)
    }
}

/// Horizontal edge flourish (tileable).
pub struct EdgeHorizontal;

impl EdgeHorizontal {
    pub fn raster() -> LogoRaster {
        let size = EDGE_SIZE as usize;
        let width_bytes = size.div_ceil(8);
        let mut data = vec![0u8; width_bytes * size];

        let center_y = size / 2;

        for y in 0..size {
            for x in 0..size {
                // Double horizontal lines (top and bottom)
                if y == center_y - 2 || y == center_y + 2 {
                    let byte_idx = y * width_bytes + x / 8;
                    let bit_idx = 7 - (x % 8);
                    data[byte_idx] |= 1 << bit_idx;
                }

                // Single middle line
                if y == center_y {
                    let byte_idx = y * width_bytes + x / 8;
                    let bit_idx = 7 - (x % 8);
                    data[byte_idx] |= 1 << bit_idx;
                }
            }
        }

        LogoRaster {
            width: EDGE_SIZE,
            height: EDGE_SIZE,
            data,
        }
    }
}

/// Vertical edge flourish (tileable).
pub struct EdgeVertical;

impl EdgeVertical {
    pub fn raster() -> LogoRaster {
        let size = EDGE_SIZE as usize;
        let width_bytes = size.div_ceil(8);
        let mut data = vec![0u8; width_bytes * size];

        let center_x = size / 2;

        for y in 0..size {
            for x in 0..size {
                // Triple vertical lines (left, center, right)
                if x == center_x - 2 || x == center_x || x == center_x + 2 {
                    let byte_idx = y * width_bytes + x / 8;
                    let bit_idx = 7 - (x % 8);
                    data[byte_idx] |= 1 << bit_idx;
                }
            }
        }

        LogoRaster {
            width: EDGE_SIZE,
            height: EDGE_SIZE,
            data,
        }
    }
}

/// Generate a corner sprite with optional flipping.
fn corner_sprite(flip_x: bool, flip_y: bool) -> LogoRaster {
    let size = CORNER_SIZE as usize;
    let width_bytes = size.div_ceil(8);
    let mut data = vec![0u8; width_bytes * size];

    for y in 0..size {
        for x in 0..size {
            // Map to canonical top-left corner, then flip
            let cx = if flip_x { size - 1 - x } else { x };
            let cy = if flip_y { size - 1 - y } else { y };

            if is_corner_pixel(cx, cy, size) {
                let byte_idx = y * width_bytes + x / 8;
                let bit_idx = 7 - (x % 8);
                data[byte_idx] |= 1 << bit_idx;
            }
        }
    }

    LogoRaster {
        width: CORNER_SIZE,
        height: CORNER_SIZE,
        data,
    }
}

/// Corner pattern: Ornamental bracket connecting horizontal and vertical edges.
fn is_corner_pixel(x: usize, y: usize, size: usize) -> bool {
    let center = size / 2;
    let line1 = center - 2;  // 14
    let line2 = center;       // 16
    let line3 = center + 2;   // 18

    // Horizontal lines (extend across full width to connect with edge tiles)
    let is_h_line = y == line1 || y == line2 || y == line3;

    // Vertical lines (extend down full height to connect with edge tiles)
    let is_v_line = x == line1 || x == line2 || x == line3;

    // Draw horizontal lines across entire width
    if is_h_line {
        return true;
    }

    // Draw vertical lines down entire height
    if is_v_line {
        return true;
    }

    // Decorative corner dot at (8, 8)
    let dx = x as i32 - 8;
    let dy = y as i32 - 8;
    if dx * dx + dy * dy <= 16 {  // radius 4
        return true;
    }

    false
}
