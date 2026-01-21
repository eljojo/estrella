//! # Ripple Effect
//!
//! Concentric circles emanating from a center point, combined with wobble interference.
//!
//! ## Formula
//!
//! ```text
//! r = sqrt((x - cx)² + (y - cy)²)  // Distance from center
//! ripple = 0.5 + 0.5 * cos(r / scale - y / drift)
//! wobble = 0.5 + 0.5 * sin(x / 37.0 + 0.7 * cos(y / 53.0))
//! v = ripple * (1 - wobble_mix) + wobble * wobble_mix
//! ```

use super::clamp01;

/// Parameters for the ripple effect.
#[derive(Debug, Clone)]
pub struct Params {
    /// Center X as fraction of width (0.0-1.0). Default: 0.5
    pub center_x: f32,
    /// Center Y as fraction of height (0.0-1.0). Default: 0.5
    pub center_y: f32,
    /// Ripple wavelength - larger = wider rings. Default: 6.5
    pub scale: f32,
    /// Vertical drift factor - creates downward motion effect. Default: 85.0
    pub drift: f32,
    /// Wobble blend (0.0 = pure ripple, 1.0 = pure wobble). Default: 0.25
    pub wobble_mix: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            scale: 6.5,
            drift: 85.0,
            wobble_mix: 0.25,
        }
    }
}

impl Params {
    /// Parameters matching spec/nv_logo_store.py for logo generation.
    pub fn logo() -> Self {
        Self {
            center_x: 0.52,
            center_y: 0.35,
            scale: 6.5,
            drift: 85.0,
            wobble_mix: 0.22,
        }
    }
}

/// Compute ripple shade at a pixel.
///
/// Returns intensity in [0.0, 1.0] before gamma correction.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;

    // Center point
    let cx = wf * params.center_x;
    let cy = hf * params.center_y;

    // Distance from center
    let dx = xf - cx;
    let dy = yf - cy;
    let r = (dx * dx + dy * dy).sqrt();

    // Ripple: concentric circles with vertical drift
    let ripple = 0.5 + 0.5 * (r / params.scale - yf / params.drift).cos();

    // Wobble: interference pattern
    let wobble = 0.5 + 0.5 * (xf / 37.0 + 0.7 * (yf / 53.0).cos()).sin();

    // Blend ripple and wobble
    let v = ripple * (1.0 - params.wobble_mix) + wobble * params.wobble_mix;

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

    #[test]
    fn test_logo_params() {
        let params = Params::logo();
        assert!((params.center_x - 0.52).abs() < 0.001);
        assert!((params.center_y - 0.35).abs() < 0.001);
        assert!((params.wobble_mix - 0.22).abs() < 0.001);
    }
}
