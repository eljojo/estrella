//! # Waves Effect
//!
//! Multi-oscillator interference pattern creating flowing wave effects.
//!
//! ## Formula
//!
//! ```text
//! horiz = sin(x / 19.0 + 0.7 * sin(y / 37.0))
//! vert = cos(y / 23.0 + 0.9 * cos(x / 41.0))
//! radial = cos(r * 24.0 - y / 29.0)
//! v = 0.45 * horiz + 0.30 * vert + 0.25 * radial
//! ```

use super::clamp01;

/// Parameters for the waves effect.
#[derive(Debug, Clone)]
pub struct Params {
    /// Horizontal wave frequency divisor. Default: 19.0
    pub horiz_freq: f32,
    /// Vertical wave frequency divisor. Default: 23.0
    pub vert_freq: f32,
    /// Radial wave frequency. Default: 24.0
    pub radial_freq: f32,
    /// Horizontal wave weight. Default: 0.45
    pub horiz_weight: f32,
    /// Vertical wave weight. Default: 0.30
    pub vert_weight: f32,
    /// Radial wave weight. Default: 0.25
    pub radial_weight: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            horiz_freq: 19.0,
            vert_freq: 23.0,
            radial_freq: 24.0,
            horiz_weight: 0.45,
            vert_weight: 0.35,
            radial_weight: 0.20,
        }
    }
}

/// Compute waves shade at a pixel.
///
/// Returns intensity in [0.0, 1.0] before gamma correction.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;

    // Normalized coordinates in [-1, 1]
    let nx = 2.0 * xf / wf - 1.0;
    let ny = 2.0 * yf / hf - 1.0;
    let r = (nx * nx + ny * ny).sqrt();

    // Horizontal waves with vertical modulation
    let horiz = (xf / params.horiz_freq + 0.7 * (yf / 37.0).sin()).sin();

    // Vertical waves with horizontal modulation
    let vert = (yf / params.vert_freq + 0.9 * (xf / 41.0).cos()).cos();

    // Radial waves with vertical drift
    let radial = (r * params.radial_freq - yf / 29.0).cos();

    // Normalize to [0, 1]
    let horiz_norm = 0.5 + 0.5 * horiz;
    let vert_norm = 0.5 + 0.5 * vert;
    let radial_norm = 0.5 + 0.5 * radial;

    // Weighted blend
    let v = params.horiz_weight * horiz_norm
        + params.vert_weight * vert_norm
        + params.radial_weight * radial_norm;

    clamp01(v)
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
}
