//! # Star Logo Generator
//!
//! Generates a 5-pointed star logo programmatically.

use super::LogoRaster;
use std::f32::consts::{PI, TAU};

/// Star logo generator.
pub struct Star;

impl Star {
    /// Logo size in dots (width and height).
    pub const SIZE: u16 = 96;

    /// Generate the star logo raster data.
    pub fn raster() -> LogoRaster {
        let size = Self::SIZE as usize;
        let width_bytes = size.div_ceil(8);
        let mut data = vec![0u8; width_bytes * size];

        let center = size as f32 / 2.0;
        let outer_radius = center * 0.9;
        let inner_radius = outer_radius * 0.382; // Golden ratio approximation

        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;

                if is_inside_star(dx, dy, outer_radius, inner_radius) {
                    let byte_idx = y * width_bytes + x / 8;
                    let bit_idx = 7 - (x % 8);
                    data[byte_idx] |= 1 << bit_idx;
                }
            }
        }

        LogoRaster {
            width: Self::SIZE,
            height: Self::SIZE,
            data,
        }
    }
}

/// Check if a point (relative to center) is inside a 5-pointed star.
///
/// The star has 5 outer points and 5 inner valleys.
/// We interpolate the edge radius based on angle.
fn is_inside_star(dx: f32, dy: f32, outer_r: f32, inner_r: f32) -> bool {
    // Distance from center
    let dist = (dx * dx + dy * dy).sqrt();

    // Quick reject if outside bounding circle
    if dist > outer_r {
        return false;
    }

    // Convert to angle (0 at top, clockwise)
    // atan2 gives angle from positive x-axis, counter-clockwise
    // We want 0 at top (negative y-axis), so adjust
    let angle = dy.atan2(dx);

    // 5 points means each sector is TAU/5 = 72 degrees
    let sector_angle = TAU / 5.0;

    // Offset so star point is at top (negative y direction)
    // A point is at -PI/2 (top), so we add PI/2 to normalize
    let adjusted = (angle + PI / 2.0).rem_euclid(TAU);

    // Which sector are we in? And how far through it?
    let local_angle = adjusted.rem_euclid(sector_angle);

    // Within a sector:
    // - At 0: outer point
    // - At sector_angle/2: inner valley
    // - At sector_angle: back to outer point
    let half = sector_angle / 2.0;
    let edge_radius = if local_angle < half {
        // Going from outer point to inner valley
        let t = local_angle / half;
        outer_r * (1.0 - t) + inner_r * t
    } else {
        // Going from inner valley to outer point
        let t = (local_angle - half) / half;
        inner_r * (1.0 - t) + outer_r * t
    };

    dist <= edge_radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_star_dimensions() {
        let raster = Star::raster();
        assert_eq!(raster.width, 96);
        assert_eq!(raster.height, 96);
    }

    #[test]
    fn test_star_data_size() {
        let raster = Star::raster();
        let expected_size = 96usize.div_ceil(8) * 96; // 12 bytes * 96 rows = 1152
        assert_eq!(raster.data.len(), expected_size);
    }

    #[test]
    fn test_star_has_pixels() {
        let raster = Star::raster();
        // The star should have some black pixels
        let black_pixels: usize = raster
            .data
            .iter()
            .map(|byte| byte.count_ones() as usize)
            .sum();
        // Rough sanity check: star should fill about 30-50% of the area
        let total_pixels = 96 * 96;
        assert!(black_pixels > total_pixels / 5, "Star has too few pixels");
        assert!(
            black_pixels < total_pixels * 3 / 4,
            "Star has too many pixels"
        );
    }

    #[test]
    fn test_star_center_is_filled() {
        let raster = Star::raster();
        // Center pixel should be black
        let center = 48;
        let byte_idx = center * 12 + center / 8;
        let bit_idx = 7 - (center % 8);
        let center_pixel = (raster.data[byte_idx] >> bit_idx) & 1;
        assert_eq!(center_pixel, 1, "Center of star should be filled");
    }

    #[test]
    fn test_star_corner_is_empty() {
        let raster = Star::raster();
        // Corner pixel (0,0) should be white
        let corner_pixel = (raster.data[0] >> 7) & 1;
        assert_eq!(corner_pixel, 0, "Corner of star should be empty");
    }
}
