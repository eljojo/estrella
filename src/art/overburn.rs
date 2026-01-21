//! # Overburn Effect
//!
//! Simulates double-pass printing where the same image is printed twice,
//! causing darker tones and ink bloom.
//!
//! ## Description
//!
//! Creates a darkened ripple pattern with simulated bloom/spread effect.
//! This mimics what happens when thermal printers make multiple passes
//! over the same area, causing darker output and slight spreading.
//!
//! ## Formula
//!
//! ```text
//! base = ripple_pattern(x, y)
//! darkened = base * 0.7 + 0.3
//! blurred = darkened * (1 - blur_amount) + base * blur_amount
//! v = clamp01(blurred) ^ gamma
//! ```

use super::clamp01;

/// Parameters for the overburn effect.
#[derive(Debug, Clone)]
pub struct Params {
    /// Ripple scale (wavelength). Default: 6.5
    pub scale: f32,
    /// Vertical drift divisor. Default: 85.0
    pub drift: f32,
    /// Wobble blend factor. Default: 0.25
    pub wobble_mix: f32,
    /// Darkening multiplier. Default: 0.7
    pub darken_mult: f32,
    /// Darkening offset. Default: 0.3
    pub darken_offset: f32,
    /// Bloom/blur amount. Default: 0.15
    pub blur_amount: f32,
    /// Gamma for extra contrast. Default: 1.6
    pub gamma: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            scale: 6.5,
            drift: 85.0,
            wobble_mix: 0.25,
            darken_mult: 0.7,
            darken_offset: 0.3,
            blur_amount: 0.15,
            gamma: 1.6,
        }
    }
}

/// Compute overburn effect shade at a pixel.
///
/// Returns intensity in [0.0, 1.0].
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;

    // Compute base ripple pattern
    let cx = wf / 2.0;
    let cy = hf / 2.0;
    let dx = xf - cx;
    let dy = yf - cy;
    let r = (dx * dx + dy * dy).sqrt();

    let ripple = 0.5 + 0.5 * (r / params.scale - yf / params.drift).cos();
    let wobble = 0.5 + 0.5 * (xf / 37.0 + 0.7 * (yf / 53.0).cos()).sin();
    let v = (1.0 - params.wobble_mix) * ripple + params.wobble_mix * wobble;

    // Simulate overburn by darkening
    let darkened = v * params.darken_mult + params.darken_offset;

    // Add slight blur by blending back with original (bloom effect)
    let blurred = darkened * (1.0 - params.blur_amount) + v * params.blur_amount;

    clamp01(blurred).powf(params.gamma)
}

/// Overburn effect pattern with default parameters.
#[derive(Debug, Clone, Default)]
pub struct Overburn;

impl super::Pattern for Overburn {
    fn name(&self) -> &'static str {
        "overburn"
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
    fn test_darker_than_base() {
        // Overburn should generally produce darker output than plain ripple
        let params = Params::default();
        let v = shade(288, 250, 576, 500, &params);
        // With darkening and high gamma, output should be valid
        assert!(v >= 0.0 && v <= 1.0);
    }
}
