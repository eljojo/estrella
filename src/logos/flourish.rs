//! # Victorian Flourish Sprite
//!
//! Small ornate decorative element inspired by Victorian typography.
//! Features a flower/rosette motif with scrollwork.

use super::LogoRaster;

/// Victorian flourish sprite generator.
pub struct Flourish;

impl Flourish {
    /// Sprite size in dots (width and height).
    pub const SIZE: u16 = 32;

    /// Generate the flourish sprite raster data.
    pub fn raster() -> LogoRaster {
        let size = Self::SIZE as usize;
        let width_bytes = size.div_ceil(8);
        let mut data = vec![0u8; width_bytes * size];

        let center = size as f32 / 2.0;

        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;

                if is_flourish_pixel(dx, dy, center) {
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

/// Victorian flourish pattern: central rosette with four curved petals.
fn is_flourish_pixel(dx: f32, dy: f32, center: f32) -> bool {
    use std::f32::consts::PI;

    let dist = (dx * dx + dy * dy).sqrt();
    let angle = dy.atan2(dx);

    // Central circle (rosette center)
    if dist < center * 0.25 {
        return true;
    }

    // Four petals at cardinal directions
    let petal_count = 4.0;
    let petal_angle = (angle * petal_count / 2.0).cos();

    // Petal shape: rounded lobes radiating from center
    let petal_radius = center * 0.4 + center * 0.35 * petal_angle.abs();
    if dist < petal_radius && petal_angle > 0.0 {
        return true;
    }

    // Decorative dots at corners (diagonal positions)
    let diagonal_angle = angle - PI / 4.0;
    let corner_dist = (diagonal_angle * 4.0).cos().abs();
    if corner_dist > 0.9 && dist > center * 0.6 && dist < center * 0.75 {
        // Small dots at 45° intervals
        let dot_angle = (angle * 4.0).sin();
        if dot_angle.abs() < 0.3 && dist < center * 0.7 {
            return true;
        }
    }

    // Outer ring frame
    if dist > center * 0.85 && dist < center * 0.95 {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flourish_dimensions() {
        let raster = Flourish::raster();
        assert_eq!(raster.width, 32);
        assert_eq!(raster.height, 32);
    }

    #[test]
    fn test_flourish_has_pixels() {
        let raster = Flourish::raster();
        let black_pixels: usize = raster
            .data
            .iter()
            .map(|byte| byte.count_ones() as usize)
            .sum();
        assert!(black_pixels > 100, "Flourish should have decorative pixels");
    }

    #[test]
    fn test_flourish_center_filled() {
        let raster = Flourish::raster();
        // Center should be part of rosette
        let center = 16;
        let byte_idx = center * (32 / 8) + center / 8;
        let bit_idx = 7 - (center % 8);
        let center_pixel = (raster.data[byte_idx] >> bit_idx) & 1;
        assert_eq!(center_pixel, 1, "Center rosette should be filled");
    }
}
