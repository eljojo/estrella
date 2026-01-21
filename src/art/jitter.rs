//! # Jitter/Banding Effect
//!
//! Simulates the organic gradients and banding artifacts that occur when
//! inserting micro-feeds and delays between raster chunks.
//!
//! ## Description
//!
//! Creates horizontal bands that vary slightly in intensity, with visible
//! darker edges at band boundaries. This mimics the effect of printer
//! cooldown between chunks and slight mechanical jitter.
//!
//! ## Formula
//!
//! ```text
//! base = ripple_pattern(x, y)
//! band_num = y / band_height
//! band_variation = (sin(band_num * 0.3) + 1) / 2
//! band_mod = 0.9 + band_variation * 0.2
//! edge_darkening = 0.15 if at band boundary else 0.0
//! v = clamp01(base * band_mod + edge_darkening) ^ gamma
//! ```

use super::clamp01;

/// Parameters for the jitter/banding effect.
#[derive(Debug, Clone)]
pub struct Params {
    /// Ripple scale (wavelength). Default: 6.5
    pub scale: f32,
    /// Vertical drift divisor. Default: 85.0
    pub drift: f32,
    /// Wobble blend factor. Default: 0.25
    pub wobble_mix: f32,
    /// Height of each band in pixels. Default: 24
    pub band_height: usize,
    /// Variation multiplier base. Default: 0.3
    pub variation_freq: f32,
    /// Minimum band modifier. Default: 0.9
    pub mod_min: f32,
    /// Band modifier range. Default: 0.2
    pub mod_range: f32,
    /// Edge darkening amount. Default: 0.15
    pub edge_darken: f32,
    /// Number of edge rows. Default: 2
    pub edge_rows: usize,
    /// Gamma for contrast. Default: 1.3
    pub gamma: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            scale: 6.5,
            drift: 85.0,
            wobble_mix: 0.25,
            band_height: 24,
            variation_freq: 0.3,
            mod_min: 0.9,
            mod_range: 0.2,
            edge_darken: 0.15,
            edge_rows: 2,
            gamma: 1.3,
        }
    }
}

/// Compute jitter/banding effect shade at a pixel.
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

    // Create banding
    let band_num = y / params.band_height;
    let band_y = y % params.band_height;

    // Intensity varies slightly per band (simulates jitter/cooldown)
    let band_variation = ((band_num as f32 * params.variation_freq).sin() + 1.0) / 2.0;
    let band_mod = params.mod_min + band_variation * params.mod_range;

    // Add visible band boundaries (darker lines)
    let is_band_edge = band_y < params.edge_rows;
    let edge_darkening = if is_band_edge {
        params.edge_darken
    } else {
        0.0
    };

    let modified = (v * band_mod) + edge_darkening;
    clamp01(modified).powf(params.gamma)
}

/// Jitter/banding pattern with default parameters.
#[derive(Debug, Clone, Default)]
pub struct Jitter;

impl super::Pattern for Jitter {
    fn name(&self) -> &'static str {
        "jitter"
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
    fn test_band_edges() {
        let params = Params::default();
        // Row 0 should be an edge (darker)
        let v_edge = shade(288, 0, 576, 500, &params);
        // Row 12 should not be an edge
        let v_mid = shade(288, 12, 576, 500, &params);
        // Both should be valid
        assert!(v_edge >= 0.0 && v_edge <= 1.0);
        assert!(v_mid >= 0.0 && v_mid <= 1.0);
    }
}
