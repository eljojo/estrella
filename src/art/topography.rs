//! # Topography Effect
//!
//! Contour lines like elevation on a topographic map.
//!
//! ## Formula
//!
//! ```text
//! t = sin(x / 17) + sin(y / 29) + sin((x + y) / 41)
//! t_wrapped = t - floor(t)  // fmod to [0, 1)
//! contours = abs(t_wrapped - 0.5) * 2  // 0 at midline
//! v = (1 - contours) ^ 2.2  // invert so lines are dark
//! ```

use super::clamp01;

/// Parameters for the topography effect.
#[derive(Debug, Clone)]
pub struct Params {
    /// First wave frequency divisor. Default: 17.0
    pub freq1: f32,
    /// Second wave frequency divisor. Default: 29.0
    pub freq2: f32,
    /// Third (diagonal) wave frequency divisor. Default: 41.0
    pub freq3: f32,
    /// Gamma for contour sharpness. Default: 2.2
    pub gamma: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            freq1: 17.0,
            freq2: 29.0,
            freq3: 41.0,
            gamma: 2.2,
        }
    }
}

/// Compute topography shade at a pixel.
///
/// Returns intensity in [0.0, 1.0] with internal gamma applied.
pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;

    let t = (xf / params.freq1).sin() + (yf / params.freq2).sin() + ((xf + yf) / params.freq3).sin();

    // Create contour lines by mapping to periodic bands
    let t_wrapped = t - t.floor(); // fmod to [0, 1)
    let contours = (t_wrapped - 0.5).abs() * 2.0; // 0 at midline

    // Invert so contour lines are dark
    clamp01(1.0 - contours).powf(params.gamma)
}

/// Topography pattern with default parameters.
#[derive(Debug, Clone, Default)]
pub struct Topography;

impl super::Pattern for Topography {
    fn name(&self) -> &'static str {
        "topography"
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
}
