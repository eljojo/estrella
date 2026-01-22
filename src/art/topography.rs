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
use rand::Rng;
use std::fmt;

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

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            freq1: rng.random_range(10.0..30.0),
            freq2: rng.random_range(18.0..45.0),
            freq3: rng.random_range(25.0..60.0),
            gamma: rng.random_range(1.8..2.8),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "freq=({:.1},{:.1},{:.1}) gamma={:.2}",
            self.freq1, self.freq2, self.freq3, self.gamma
        )
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

/// Topography pattern.
#[derive(Debug, Clone)]
pub struct Topography {
    params: Params,
}

impl Default for Topography {
    fn default() -> Self {
        Self::golden()
    }
}

impl Topography {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for Topography {
    fn name(&self) -> &'static str {
        "topography"
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
            "gamma" => self.params.gamma = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for topography", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("freq1", format!("{:.1}", self.params.freq1)),
            ("freq2", format!("{:.1}", self.params.freq2)),
            ("freq3", format!("{:.1}", self.params.freq3)),
            ("gamma", format!("{:.2}", self.params.gamma)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("freq1", "Frequency 1", 10.0, 30.0, 1.0)
                .with_description("First wave frequency divisor"),
            ParamSpec::slider("freq2", "Frequency 2", 18.0, 45.0, 1.0)
                .with_description("Second wave frequency divisor"),
            ParamSpec::slider("freq3", "Frequency 3", 25.0, 60.0, 1.0)
                .with_description("Third (diagonal) wave frequency divisor"),
            ParamSpec::slider("gamma", "Gamma", 1.8, 2.8, 0.1)
                .with_description("Gamma for contour sharpness"),
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
