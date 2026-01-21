//! # Density Comparison Pattern
//!
//! Demonstrates print density by showing a ripple pattern at three different
//! gamma levels (light, medium, heavy).
//!
//! ## Description
//!
//! Divides the image into three horizontal bands, each showing the same
//! underlying ripple pattern but with different gamma correction:
//! - Top third: Light (gamma=0.8)
//! - Middle third: Medium (gamma=1.0)
//! - Bottom third: Heavy (gamma=1.5)
//!
//! Useful for calibrating printer density settings.

use super::clamp01;

/// Parameters for the density comparison pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Ripple scale (wavelength). Default: 6.5
    pub scale: f32,
    /// Vertical drift divisor. Default: 85.0
    pub drift: f32,
    /// Wobble blend factor. Default: 0.25
    pub wobble_mix: f32,
    /// Gamma for light band (top). Default: 0.8
    pub gamma_light: f32,
    /// Gamma for medium band (middle). Default: 1.0
    pub gamma_medium: f32,
    /// Gamma for heavy band (bottom). Default: 1.5
    pub gamma_heavy: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            scale: 6.5,
            drift: 85.0,
            wobble_mix: 0.25,
            gamma_light: 0.8,
            gamma_medium: 1.0,
            gamma_heavy: 1.5,
        }
    }
}

/// Compute density comparison shade at a pixel.
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

    // Determine which density band we're in
    let band = (y * 3) / height;
    let gamma = match band {
        0 => params.gamma_light,
        1 => params.gamma_medium,
        _ => params.gamma_heavy,
    };

    clamp01(v).powf(gamma)
}

/// Density comparison pattern with default parameters.
#[derive(Debug, Clone, Default)]
pub struct Density;

impl super::Pattern for Density {
    fn name(&self) -> &'static str {
        "density"
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
    fn test_bands() {
        let params = Params::default();
        let height = 300;
        // Sample from each band at the same x position
        let x = 288; // Center
        let v_light = shade(x, height / 6, 576, height, &params);
        let v_medium = shade(x, height / 2, 576, height, &params);
        let v_heavy = shade(x, height * 5 / 6, 576, height, &params);

        // All should be valid
        assert!(v_light >= 0.0 && v_light <= 1.0);
        assert!(v_medium >= 0.0 && v_medium <= 1.0);
        assert!(v_heavy >= 0.0 && v_heavy <= 1.0);
    }
}
