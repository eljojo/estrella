//! # Plasma Effect
//!
//! Multiple overlapping sine waves creating moire/interference patterns.
//!
//! ## Formula
//!
//! ```text
//! plasma = sin(x / 11) + sin((x + y) / 19) + cos(y / 13) + sin(hypot(x - cx, y - cy) / 9)
//! normalized = (plasma + 4) / 8
//! ```

use super::clamp01;

/// Parameters for the plasma effect.
#[derive(Debug, Clone)]
pub struct Params {
    /// First sine wave frequency divisor. Default: 11.0
    pub freq1: f32,
    /// Second (diagonal) sine wave frequency divisor. Default: 19.0
    pub freq2: f32,
    /// Vertical cosine wave frequency divisor. Default: 13.0
    pub freq3: f32,
    /// Radial sine wave frequency divisor. Default: 9.0
    pub freq4: f32,
    /// Center X as fraction of width. Default: 0.35
    pub center_x: f32,
    /// Center Y as fraction of height. Default: 0.2
    pub center_y: f32,
    /// Gamma correction. Default: 1.2
    pub gamma: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            freq1: 11.0,
            freq2: 19.0,
            freq3: 13.0,
            freq4: 9.0,
            center_x: 0.35,
            center_y: 0.2,
            gamma: 1.2,
        }
    }
}

/// Compute plasma shade at a pixel.
///
/// Returns intensity in [0.0, 1.0] with internal gamma applied.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;

    let cx = wf * params.center_x;
    let cy = hf * params.center_y;

    // Multiple overlapping sine waves
    let plasma = (xf / params.freq1).sin()
        + ((xf + yf) / params.freq2).sin()
        + (yf / params.freq3).cos()
        + ((xf - cx).hypot(yf - cy) / params.freq4).sin();

    // Normalize from roughly [-4, 4] to [0, 1]
    let normalized = (plasma + 4.0) / 8.0;

    // Apply gamma for contrast
    clamp01(normalized).powf(params.gamma)
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
