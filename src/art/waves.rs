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

use crate::shader::{clamp01, dist, gamma, normalize};
use async_trait::async_trait;
use rand::Rng;
use std::fmt;

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
    /// Gamma correction. Default: 1.25
    pub gamma: f32,
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
            gamma: 1.25,
        }
    }
}

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            horiz_freq: rng.random_range(12.0..30.0),
            vert_freq: rng.random_range(15.0..35.0),
            radial_freq: rng.random_range(15.0..40.0),
            horiz_weight: rng.random_range(0.3..0.5),
            vert_weight: rng.random_range(0.25..0.45),
            radial_weight: rng.random_range(0.15..0.35),
            gamma: rng.random_range(1.0..1.5),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "horiz={:.1} vert={:.1} radial={:.1} weights=({:.2},{:.2},{:.2}) gamma={:.2}",
            self.horiz_freq,
            self.vert_freq,
            self.radial_freq,
            self.horiz_weight,
            self.vert_weight,
            self.radial_weight,
            self.gamma
        )
    }
}

/// Compute waves intensity at a pixel.
///
/// Returns intensity in [0.0, 1.0] with gamma applied.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;

    // Normalized coordinates in [-1, 1] using shader primitive
    let nx = normalize(xf, wf);
    let ny = normalize(yf, hf);
    let r = dist(nx, ny, 0.0, 0.0);

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

    gamma(clamp01(v), params.gamma)
}

/// Waves pattern.
#[derive(Debug, Clone)]
pub struct Waves {
    params: Params,
}

impl Default for Waves {
    fn default() -> Self {
        Self::golden()
    }
}

impl Waves {
    pub fn golden() -> Self {
        Self {
            params: Params::default(),
        }
    }

    pub fn random() -> Self {
        Self {
            params: Params::random(),
        }
    }
}

#[async_trait]
impl super::Pattern for Waves {
    fn name(&self) -> &'static str {
        "waves"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &self.params)
    }

    fn params_description(&self) -> String {
        self.params.to_string()
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        let parse_f32 = |v: &str| {
            v.parse::<f32>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        match name {
            "horiz_freq" => self.params.horiz_freq = parse_f32(value)?,
            "vert_freq" => self.params.vert_freq = parse_f32(value)?,
            "radial_freq" => self.params.radial_freq = parse_f32(value)?,
            "horiz_weight" => self.params.horiz_weight = parse_f32(value)?,
            "vert_weight" => self.params.vert_weight = parse_f32(value)?,
            "radial_weight" => self.params.radial_weight = parse_f32(value)?,
            "gamma" => self.params.gamma = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for waves", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("horiz_freq", format!("{:.1}", self.params.horiz_freq)),
            ("vert_freq", format!("{:.1}", self.params.vert_freq)),
            ("radial_freq", format!("{:.1}", self.params.radial_freq)),
            ("horiz_weight", format!("{:.2}", self.params.horiz_weight)),
            ("vert_weight", format!("{:.2}", self.params.vert_weight)),
            ("radial_weight", format!("{:.2}", self.params.radial_weight)),
            ("gamma", format!("{:.2}", self.params.gamma)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("horiz_freq", "Horizontal Frequency", 12.0, 30.0, 0.5)
                .with_description("Horizontal wave frequency divisor"),
            ParamSpec::slider("vert_freq", "Vertical Frequency", 15.0, 35.0, 0.5)
                .with_description("Vertical wave frequency divisor"),
            ParamSpec::slider("radial_freq", "Radial Frequency", 15.0, 40.0, 0.5)
                .with_description("Radial wave frequency"),
            ParamSpec::slider("horiz_weight", "Horizontal Weight", 0.3, 0.5, 0.01)
                .with_description("Weight of horizontal waves in blend"),
            ParamSpec::slider("vert_weight", "Vertical Weight", 0.25, 0.45, 0.01)
                .with_description("Weight of vertical waves in blend"),
            ParamSpec::slider("radial_weight", "Radial Weight", 0.15, 0.35, 0.01)
                .with_description("Weight of radial waves in blend"),
            ParamSpec::slider("gamma", "Gamma", 1.0, 1.5, 0.05)
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
