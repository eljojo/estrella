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

use crate::shader::*;
use async_trait::async_trait;
use rand::RngExt;
use std::fmt;

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

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            scale: rng.random_range(4.0..10.0),
            drift: rng.random_range(50.0..120.0),
            wobble_mix: rng.random_range(0.15..0.4),
            gamma_light: rng.random_range(0.6..1.0),
            gamma_medium: rng.random_range(0.9..1.2),
            gamma_heavy: rng.random_range(1.3..1.8),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "scale={:.1} drift={:.0} wobble={:.2} gamma=({:.2},{:.2},{:.2})",
            self.scale,
            self.drift,
            self.wobble_mix,
            self.gamma_light,
            self.gamma_medium,
            self.gamma_heavy
        )
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

    // Compute base ripple pattern using shader primitives
    let (dx, dy) = center_coords(xf, yf, wf, hf);
    let r = dist(dx, dy, 0.0, 0.0);

    let ripple = wave_cos(r, 1.0 / params.scale, -yf / params.drift);
    let wobble = wave_sin(xf, 1.0 / 37.0, 0.7 * (yf / 53.0).cos());
    let v = lerp(ripple, wobble, params.wobble_mix);

    // Determine which density band we're in
    let band = band_index(y * 3, height);
    let g = match band {
        0 => params.gamma_light,
        1 => params.gamma_medium,
        _ => params.gamma_heavy,
    };

    gamma(v, g)
}

/// Density comparison pattern.
#[derive(Debug, Clone)]
pub struct Density {
    params: Params,
}

impl Default for Density {
    fn default() -> Self {
        Self::golden()
    }
}

impl Density {
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
impl super::Pattern for Density {
    fn name(&self) -> &'static str {
        "density"
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
            "scale" => self.params.scale = parse_f32(value)?,
            "drift" => self.params.drift = parse_f32(value)?,
            "wobble_mix" => self.params.wobble_mix = parse_f32(value)?,
            "gamma_light" => self.params.gamma_light = parse_f32(value)?,
            "gamma_medium" => self.params.gamma_medium = parse_f32(value)?,
            "gamma_heavy" => self.params.gamma_heavy = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for density", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("scale", format!("{:.1}", self.params.scale)),
            ("drift", format!("{:.0}", self.params.drift)),
            ("wobble_mix", format!("{:.2}", self.params.wobble_mix)),
            ("gamma_light", format!("{:.2}", self.params.gamma_light)),
            ("gamma_medium", format!("{:.2}", self.params.gamma_medium)),
            ("gamma_heavy", format!("{:.2}", self.params.gamma_heavy)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("scale", "Scale", 4.0, 10.0, 0.5)
                .with_description("Ripple scale (wavelength)"),
            ParamSpec::slider("drift", "Drift", 50.0, 120.0, 5.0)
                .with_description("Vertical drift divisor"),
            ParamSpec::slider("wobble_mix", "Wobble Mix", 0.15, 0.4, 0.01)
                .with_description("Wobble blend factor"),
            ParamSpec::slider("gamma_light", "Gamma Light", 0.6, 1.0, 0.05)
                .with_description("Gamma for light band (top)"),
            ParamSpec::slider("gamma_medium", "Gamma Medium", 0.9, 1.2, 0.05)
                .with_description("Gamma for medium band (middle)"),
            ParamSpec::slider("gamma_heavy", "Gamma Heavy", 1.3, 1.8, 0.05)
                .with_description("Gamma for heavy band (bottom)"),
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
