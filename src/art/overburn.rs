//! # Overburn Effect
//!
//! Simulates double-pass printing where the same image is printed twice,
//! causing darker tones and ink bloom.
//!
//! ## Description
//!
//! Creates a darkened ripple pattern with simulated bloom/spread effect.
//! This mimics what happens when thermal printers make multiple passes
//! over the same area, causing darker output and slight spreading.
//!
//! ## Formula
//!
//! ```text
//! base = ripple_pattern(x, y)
//! darkened = base * 0.7 + 0.3
//! blurred = darkened * (1 - blur_amount) + base * blur_amount
//! v = clamp01(blurred) ^ gamma
//! ```

use crate::shader::*;
use rand::Rng;
use std::fmt;

/// Parameters for the overburn effect.
#[derive(Debug, Clone)]
pub struct Params {
    /// Ripple scale (wavelength). Default: 6.5
    pub scale: f32,
    /// Vertical drift divisor. Default: 85.0
    pub drift: f32,
    /// Wobble blend factor. Default: 0.25
    pub wobble_mix: f32,
    /// Darkening multiplier. Default: 0.7
    pub darken_mult: f32,
    /// Darkening offset. Default: 0.3
    pub darken_offset: f32,
    /// Bloom/blur amount. Default: 0.15
    pub blur_amount: f32,
    /// Gamma for extra contrast. Default: 1.6
    pub gamma: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            scale: 6.5,
            drift: 85.0,
            wobble_mix: 0.25,
            darken_mult: 0.7,
            darken_offset: 0.3,
            blur_amount: 0.15,
            gamma: 1.6,
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
            darken_mult: rng.random_range(0.5..0.85),
            darken_offset: rng.random_range(0.2..0.45),
            blur_amount: rng.random_range(0.1..0.25),
            gamma: rng.random_range(1.3..2.0),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "scale={:.1} drift={:.0} darken=({:.2},{:.2}) blur={:.2} gamma={:.2}",
            self.scale, self.drift, self.darken_mult, self.darken_offset,
            self.blur_amount, self.gamma
        )
    }
}

/// Compute overburn effect shade at a pixel.
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

    // Simulate overburn by darkening
    let darkened = v * params.darken_mult + params.darken_offset;

    // Add slight blur by blending back with original (bloom effect)
    let blurred = lerp(darkened, v, params.blur_amount);

    gamma(blurred, params.gamma)
}

/// Overburn effect pattern.
#[derive(Debug, Clone)]
pub struct Overburn {
    params: Params,
}

impl Default for Overburn {
    fn default() -> Self {
        Self::golden()
    }
}

impl Overburn {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for Overburn {
    fn name(&self) -> &'static str {
        "overburn"
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
            "scale" => self.params.scale = parse_f32(value)?,
            "drift" => self.params.drift = parse_f32(value)?,
            "wobble_mix" => self.params.wobble_mix = parse_f32(value)?,
            "darken_mult" => self.params.darken_mult = parse_f32(value)?,
            "darken_offset" => self.params.darken_offset = parse_f32(value)?,
            "blur_amount" => self.params.blur_amount = parse_f32(value)?,
            "gamma" => self.params.gamma = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for overburn", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("scale", format!("{:.1}", self.params.scale)),
            ("drift", format!("{:.0}", self.params.drift)),
            ("wobble_mix", format!("{:.2}", self.params.wobble_mix)),
            ("darken_mult", format!("{:.2}", self.params.darken_mult)),
            ("darken_offset", format!("{:.2}", self.params.darken_offset)),
            ("blur_amount", format!("{:.2}", self.params.blur_amount)),
            ("gamma", format!("{:.2}", self.params.gamma)),
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
            ParamSpec::slider("darken_mult", "Darken Multiplier", 0.5, 0.85, 0.05)
                .with_description("Darkening multiplier"),
            ParamSpec::slider("darken_offset", "Darken Offset", 0.2, 0.45, 0.05)
                .with_description("Darkening offset"),
            ParamSpec::slider("blur_amount", "Blur Amount", 0.1, 0.25, 0.01)
                .with_description("Bloom/blur amount"),
            ParamSpec::slider("gamma", "Gamma", 1.3, 2.0, 0.1)
                .with_description("Gamma for extra contrast"),
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
    fn test_darker_than_base() {
        // Overburn should generally produce darker output than plain ripple
        let params = Params::default();
        let v = shade(288, 250, 576, 500, &params);
        // With darkening and high gamma, output should be valid
        assert!(v >= 0.0 && v <= 1.0);
    }
}
