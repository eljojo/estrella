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

use crate::shader::{clamp01, dist, gamma, lerp, wave_cos, wave_sin};
use async_trait::async_trait;
use rand::RngExt;
use std::fmt;

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
    /// Gamma correction. Default: 1.35
    pub gamma: f32,
    /// Border width in pixels. Default: 6.0
    pub border: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            scale: 6.5,
            drift: 85.0,
            wobble_mix: 0.25,
            gamma: 1.35,
            border: 6.0,
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
            gamma: 1.35,
            border: 0.0, // No border for logos
        }
    }

    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            center_x: rng.random_range(0.3..0.7),
            center_y: rng.random_range(0.3..0.7),
            scale: rng.random_range(4.0..10.0),
            drift: rng.random_range(50.0..150.0),
            wobble_mix: rng.random_range(0.1..0.4),
            gamma: rng.random_range(1.1..1.6),
            border: 6.0,
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "center=({:.2},{:.2}) scale={:.1} drift={:.0} wobble={:.2} gamma={:.2}",
            self.center_x, self.center_y, self.scale, self.drift, self.wobble_mix, self.gamma
        )
    }
}

/// Compute ripple intensity at a pixel.
///
/// Returns intensity in [0.0, 1.0] with gamma applied.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    // Border check
    if params.border > 0.0 && super::in_border(x, y, width, height, params.border) {
        return 1.0;
    }

    let xf = x as f32;
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;

    // Center point
    let cx = wf * params.center_x;
    let cy = hf * params.center_y;

    // Distance from center using shader primitive
    let r = dist(xf, yf, cx, cy);

    // Ripple: concentric circles with vertical drift
    let ripple = wave_cos(r, 1.0 / params.scale, -yf / params.drift);

    // Wobble: interference pattern
    let wobble = wave_sin(xf, 1.0 / 37.0, 0.7 * (yf / 53.0).cos());

    // Blend ripple and wobble
    let v = lerp(ripple, wobble, params.wobble_mix);

    gamma(clamp01(v), params.gamma)
}

/// Ripple pattern.
#[derive(Debug, Clone)]
pub struct Ripple {
    params: Params,
}

impl Default for Ripple {
    fn default() -> Self {
        Self::golden()
    }
}

impl Ripple {
    /// Create with golden (deterministic) params for reproducible output.
    pub fn golden() -> Self {
        Self {
            params: Params::default(),
        }
    }

    /// Create with randomized params for unique prints.
    pub fn random() -> Self {
        Self {
            params: Params::random(),
        }
    }

    /// Create with logo-optimized params.
    pub fn logo() -> Self {
        Self {
            params: Params::logo(),
        }
    }
}

#[async_trait]
impl super::Pattern for Ripple {
    fn name(&self) -> &'static str {
        "ripple"
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
            "center_x" => self.params.center_x = parse_f32(value)?,
            "center_y" => self.params.center_y = parse_f32(value)?,
            "scale" => self.params.scale = parse_f32(value)?,
            "drift" => self.params.drift = parse_f32(value)?,
            "wobble_mix" => self.params.wobble_mix = parse_f32(value)?,
            "gamma" => self.params.gamma = parse_f32(value)?,
            "border" => self.params.border = parse_f32(value)?,
            _ => {
                return Err(format!(
                    "Unknown param '{}' for ripple. Available: center_x, center_y, scale, drift, wobble_mix, gamma, border",
                    name
                ));
            }
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("center_x", format!("{:.2}", self.params.center_x)),
            ("center_y", format!("{:.2}", self.params.center_y)),
            ("scale", format!("{:.1}", self.params.scale)),
            ("drift", format!("{:.0}", self.params.drift)),
            ("wobble_mix", format!("{:.2}", self.params.wobble_mix)),
            ("gamma", format!("{:.2}", self.params.gamma)),
            ("border", format!("{:.1}", self.params.border)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("center_x", "Center X", 0.0, 1.0, 0.01)
                .with_description("Horizontal center position (0-1)"),
            ParamSpec::slider("center_y", "Center Y", 0.0, 1.0, 0.01)
                .with_description("Vertical center position (0-1)"),
            ParamSpec::slider("scale", "Scale", 2.0, 20.0, 0.5)
                .with_description("Ripple wavelength"),
            ParamSpec::slider("drift", "Drift", 20.0, 200.0, 5.0)
                .with_description("Vertical drift factor"),
            ParamSpec::slider("wobble_mix", "Wobble Mix", 0.0, 1.0, 0.01)
                .with_description("Blend with wobble pattern"),
            ParamSpec::slider("gamma", "Gamma", 0.5, 3.0, 0.05)
                .with_description("Contrast adjustment"),
            ParamSpec::slider("border", "Border", 0.0, 20.0, 1.0)
                .with_description("Border width in pixels"),
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
    fn test_logo_params() {
        let params = Params::logo();
        assert!((params.center_x - 0.52).abs() < 0.001);
        assert!((params.center_y - 0.35).abs() < 0.001);
        assert!((params.wobble_mix - 0.22).abs() < 0.001);
    }
}
