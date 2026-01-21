//! # Glitch Effect
//!
//! Blocky columns with horizontal scanlines creating a digital glitch aesthetic.
//!
//! ## Formula
//!
//! ```text
//! col = x / 12
//! base = sin(col * 0.7) * 0.5 + 0.5
//! wobble = 0.5 + 0.5 * sin((x + y * 7) / 15)
//! scan = 1.0 if (y % 24) < 2 else 0.0
//! v = max(0.55 * base + 0.45 * wobble, scan)
//! ```

use super::clamp01;

/// Parameters for the glitch effect.
#[derive(Debug, Clone)]
pub struct Params {
    /// Column width in pixels. Default: 12
    pub column_width: usize,
    /// Column frequency multiplier. Default: 0.7
    pub column_freq: f32,
    /// Wobble frequency divisor. Default: 15.0
    pub wobble_freq: f32,
    /// Wobble vertical multiplier. Default: 7.0
    pub wobble_vert: f32,
    /// Scanline period in rows. Default: 24
    pub scanline_period: usize,
    /// Scanline thickness in rows. Default: 2
    pub scanline_thickness: usize,
    /// Base weight in blend. Default: 0.55
    pub base_weight: f32,
    /// Wobble weight in blend. Default: 0.45
    pub wobble_weight: f32,
    /// Gamma correction. Default: 1.0
    pub gamma: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            column_width: 12,
            column_freq: 0.7,
            wobble_freq: 15.0,
            wobble_vert: 7.0,
            scanline_period: 24,
            scanline_thickness: 2,
            base_weight: 0.55,
            wobble_weight: 0.45,
            gamma: 1.0,
        }
    }
}

/// Compute glitch intensity at a pixel.
///
/// Returns intensity in [0.0, 1.0] with gamma applied.
pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let col = (x / params.column_width) as f32;

    // Base intensity varies by column
    let base = (col * params.column_freq).sin() * 0.5 + 0.5;

    // Wobble adds horizontal variation
    let wobble = 0.5 + 0.5 * ((xf + yf * params.wobble_vert) / params.wobble_freq).sin();

    // Scanlines: dark lines at regular intervals
    let scan_pos = y % params.scanline_period;
    let scan = if scan_pos < params.scanline_thickness {
        1.0
    } else {
        0.0
    };

    // Blend base and wobble, then overlay scanlines
    let blended = params.base_weight * base + params.wobble_weight * wobble;
    clamp01(blended.max(scan)).powf(params.gamma)
}

/// Glitch pattern with default parameters.
#[derive(Debug, Clone, Default)]
pub struct Glitch;

impl super::Pattern for Glitch {
    fn name(&self) -> &'static str {
        "glitch"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &Params::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shade_range() {
        let params = Params::default();
        for y in (0..500).step_by(50) {
            for x in (0..576).step_by(50) {
                let v = shade(x, y, 576, 500, &params);
                assert!(v >= 0.0 && v <= 1.0);
            }
        }
    }

    #[test]
    fn test_scanlines() {
        let params = Params::default();
        // Row 0 should have scanline
        let v0 = shade(100, 0, 576, 500, &params);
        // Row 12 should not have scanline
        let v12 = shade(100, 12, 576, 500, &params);
        assert!(v0 >= v12, "Scanline should be darker");
    }
}
