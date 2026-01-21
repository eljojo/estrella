//! # Rings Effect
//!
//! Concentric rings emanating from center with diagonal interference.
//!
//! ## Formula
//!
//! ```text
//! r = sqrt(nx² + ny²)  // normalized radius
//! rings = 0.5 + 0.5 * cos(r * 30 - y / 25)
//! diag = 0.5 + 0.5 * sin((x - 2*y) / 23)
//! v = 0.65 * rings + 0.35 * diag
//! ```

use super::clamp01;

/// Parameters for the rings effect.
#[derive(Debug, Clone)]
pub struct Params {
    /// Ring frequency multiplier. Default: 30.0
    pub ring_freq: f32,
    /// Vertical drift divisor. Default: 25.0
    pub drift: f32,
    /// Diagonal frequency divisor. Default: 23.0
    pub diag_freq: f32,
    /// Ring weight in blend. Default: 0.65
    pub ring_weight: f32,
    /// Diagonal weight in blend. Default: 0.35
    pub diag_weight: f32,
    /// Gamma correction. Default: 1.1
    pub gamma: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            ring_freq: 30.0,
            drift: 25.0,
            diag_freq: 23.0,
            ring_weight: 0.65,
            diag_weight: 0.35,
            gamma: 1.1,
        }
    }
}

/// Compute rings shade at a pixel.
///
/// Returns intensity in [0.0, 1.0] with internal gamma applied.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;

    // Normalized coordinates in [-1, 1]
    let nx = (xf - wf * 0.5) / (wf * 0.5);
    let ny = (yf - hf * 0.5) / (hf * 0.5);
    let r = (nx * nx + ny * ny).sqrt();

    // Rings emanating from center
    let rings = 0.5 + 0.5 * (r * params.ring_freq - yf / params.drift).cos();

    // Diagonal wave pattern
    let diag = 0.5 + 0.5 * ((xf - 2.0 * yf) / params.diag_freq).sin();

    // Blend
    let blended = params.ring_weight * rings + params.diag_weight * diag;

    clamp01(blended).powf(params.gamma)
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
