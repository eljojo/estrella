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

use crate::shader::{clamp01, dist, gamma};
use rand::Rng;
use async_trait::async_trait;
use std::fmt;

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

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            freq1: rng.random_range(7.0..18.0),
            freq2: rng.random_range(12.0..30.0),
            freq3: rng.random_range(8.0..20.0),
            freq4: rng.random_range(5.0..15.0),
            center_x: rng.random_range(0.2..0.8),
            center_y: rng.random_range(0.1..0.5),
            gamma: rng.random_range(1.0..1.5),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "freq=({:.1},{:.1},{:.1},{:.1}) center=({:.2},{:.2}) gamma={:.2}",
            self.freq1, self.freq2, self.freq3, self.freq4,
            self.center_x, self.center_y, self.gamma
        )
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

    // Multiple overlapping sine waves using shader distance primitive
    let plasma = (xf / params.freq1).sin()
        + ((xf + yf) / params.freq2).sin()
        + (yf / params.freq3).cos()
        + (dist(xf, yf, cx, cy) / params.freq4).sin();

    // Normalize from roughly [-4, 4] to [0, 1]
    let normalized = (plasma + 4.0) / 8.0;

    // Apply gamma for contrast using shader primitive
    gamma(clamp01(normalized), params.gamma)
}

/// Plasma pattern.
#[derive(Debug, Clone)]
pub struct Plasma {
    params: Params,
}

impl Default for Plasma {
    fn default() -> Self {
        Self::golden()
    }
}

impl Plasma {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

#[async_trait]
impl super::Pattern for Plasma {
    fn name(&self) -> &'static str {
        "plasma"
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
            "freq1" => self.params.freq1 = parse_f32(value)?,
            "freq2" => self.params.freq2 = parse_f32(value)?,
            "freq3" => self.params.freq3 = parse_f32(value)?,
            "freq4" => self.params.freq4 = parse_f32(value)?,
            "center_x" => self.params.center_x = parse_f32(value)?,
            "center_y" => self.params.center_y = parse_f32(value)?,
            "gamma" => self.params.gamma = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for plasma", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("freq1", format!("{:.1}", self.params.freq1)),
            ("freq2", format!("{:.1}", self.params.freq2)),
            ("freq3", format!("{:.1}", self.params.freq3)),
            ("freq4", format!("{:.1}", self.params.freq4)),
            ("center_x", format!("{:.2}", self.params.center_x)),
            ("center_y", format!("{:.2}", self.params.center_y)),
            ("gamma", format!("{:.2}", self.params.gamma)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("freq1", "Frequency 1", 7.0, 18.0, 0.5)
                .with_description("First sine wave frequency divisor"),
            ParamSpec::slider("freq2", "Frequency 2", 12.0, 30.0, 0.5)
                .with_description("Second (diagonal) sine wave frequency divisor"),
            ParamSpec::slider("freq3", "Frequency 3", 8.0, 20.0, 0.5)
                .with_description("Vertical cosine wave frequency divisor"),
            ParamSpec::slider("freq4", "Frequency 4", 5.0, 15.0, 0.5)
                .with_description("Radial sine wave frequency divisor"),
            ParamSpec::slider("center_x", "Center X", 0.2, 0.8, 0.01)
                .with_description("Center X as fraction of width"),
            ParamSpec::slider("center_y", "Center Y", 0.1, 0.5, 0.01)
                .with_description("Center Y as fraction of height"),
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
