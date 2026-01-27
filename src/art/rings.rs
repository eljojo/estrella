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

use crate::shader::{center_coords, clamp01, dist, gamma, wave_cos, wave_sin};
use rand::Rng;
use async_trait::async_trait;
use std::fmt;

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

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            ring_freq: rng.random_range(20.0..50.0),
            drift: rng.random_range(15.0..40.0),
            diag_freq: rng.random_range(15.0..35.0),
            ring_weight: rng.random_range(0.5..0.8),
            diag_weight: rng.random_range(0.2..0.5),
            gamma: rng.random_range(0.9..1.4),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ring={:.1} drift={:.1} diag={:.1} weights=({:.2},{:.2}) gamma={:.2}",
            self.ring_freq, self.drift, self.diag_freq,
            self.ring_weight, self.diag_weight, self.gamma
        )
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

    // Center-relative coordinates using shader primitive
    let (cx, cy) = center_coords(xf, yf, wf, hf);
    // Normalized to [-1, 1] by dividing by half-dimensions
    let nx = cx / (wf * 0.5);
    let ny = cy / (hf * 0.5);
    let r = dist(nx, ny, 0.0, 0.0);

    // Rings emanating from center
    let rings = wave_cos(r, params.ring_freq, -yf / params.drift);

    // Diagonal wave pattern
    let diag = wave_sin(xf - 2.0 * yf, 1.0 / params.diag_freq, 0.0);

    // Blend
    let blended = params.ring_weight * rings + params.diag_weight * diag;

    gamma(clamp01(blended), params.gamma)
}

/// Rings pattern.
#[derive(Debug, Clone)]
pub struct Rings {
    params: Params,
}

impl Default for Rings {
    fn default() -> Self {
        Self::golden()
    }
}

impl Rings {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

#[async_trait]
impl super::Pattern for Rings {
    fn name(&self) -> &'static str {
        "rings"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &self.params)
    }

    fn params_description(&self) -> String {
        self.params.to_string()
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        let parse_f32 = |v: &str| v.parse::<f32>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        match name {
            "ring_freq" => self.params.ring_freq = parse_f32(value)?,
            "drift" => self.params.drift = parse_f32(value)?,
            "diag_freq" => self.params.diag_freq = parse_f32(value)?,
            "ring_weight" => self.params.ring_weight = parse_f32(value)?,
            "diag_weight" => self.params.diag_weight = parse_f32(value)?,
            "gamma" => self.params.gamma = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for rings", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("ring_freq", format!("{:.1}", self.params.ring_freq)),
            ("drift", format!("{:.1}", self.params.drift)),
            ("diag_freq", format!("{:.1}", self.params.diag_freq)),
            ("ring_weight", format!("{:.2}", self.params.ring_weight)),
            ("diag_weight", format!("{:.2}", self.params.diag_weight)),
            ("gamma", format!("{:.2}", self.params.gamma)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("ring_freq", "Ring Frequency", 20.0, 50.0, 1.0)
                .with_description("Ring frequency multiplier"),
            ParamSpec::slider("drift", "Drift", 15.0, 40.0, 1.0)
                .with_description("Vertical drift divisor"),
            ParamSpec::slider("diag_freq", "Diagonal Frequency", 15.0, 35.0, 1.0)
                .with_description("Diagonal frequency divisor"),
            ParamSpec::slider("ring_weight", "Ring Weight", 0.5, 0.8, 0.01)
                .with_description("Ring weight in blend"),
            ParamSpec::slider("diag_weight", "Diagonal Weight", 0.2, 0.5, 0.01)
                .with_description("Diagonal weight in blend"),
            ParamSpec::slider("gamma", "Gamma", 0.9, 1.4, 0.05)
                .with_description("Gamma correction"),
        ]
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
