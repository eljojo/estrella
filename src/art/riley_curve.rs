//! # Riley Curve Pattern
//!
//! Bridget Riley-inspired curved bands creating depth and movement.
//!
//! ## Description
//!
//! Creates parallel curved stripes that bulge outward from the center,
//! creating the illusion of a curved surface. Inspired by Riley's work
//! and the sweeping curves seen in brutalist architecture.

use crate::shader::*;
use async_trait::async_trait;
use rand::Rng;
use std::fmt;

/// Parameters for the Riley curve pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Base stripe width in pixels. Default: 12.0
    pub stripe_width: f32,
    /// Curve strength (bulge amount). Default: 80.0
    pub curve_strength: f32,
    /// Curve falloff (how quickly bulge fades). Default: 2.0
    pub curve_falloff: f32,
    /// Number of curve centers. Default: 1
    pub num_curves: usize,
    /// Secondary wave amplitude. Default: 15.0
    pub wave_amplitude: f32,
    /// Secondary wave frequency. Default: 0.025
    pub wave_freq: f32,
    /// Stripe orientation in degrees. Default: 0.0
    pub rotation: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            stripe_width: 12.0,
            curve_strength: 80.0,
            curve_falloff: 2.0,
            num_curves: 1,
            wave_amplitude: 15.0,
            wave_freq: 0.025,
            rotation: 0.0,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            stripe_width: rng.random_range(8.0..18.0),
            curve_strength: rng.random_range(40.0..120.0),
            curve_falloff: rng.random_range(1.5..3.0),
            num_curves: rng.random_range(1..4),
            wave_amplitude: rng.random_range(5.0..25.0),
            wave_freq: rng.random_range(0.015..0.04),
            rotation: rng.random_range(0.0..180.0),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "stripe={:.0} curve=({:.0},{:.1}) waves=({:.0},{:.3})",
            self.stripe_width,
            self.curve_strength,
            self.curve_falloff,
            self.wave_amplitude,
            self.wave_freq
        )
    }
}

/// Compute Riley curve intensity at a pixel.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let wf = width as f32;
    let hf = height as f32;

    // Center-relative coordinates
    let (xc, yc) = center_coords(x as f32, y as f32, wf, hf);

    // Apply rotation
    let (rx, ry) = rotate_deg(xc, yc, params.rotation);

    // Calculate curve displacement
    let mut displacement = 0.0;

    for i in 0..params.num_curves {
        // Distribute curve centers
        let curve_offset = if params.num_curves > 1 {
            (i as f32 - (params.num_curves - 1) as f32 / 2.0) * hf * 0.4
        } else {
            0.0
        };

        let dist_from_curve = (ry - curve_offset).abs() / (hf * 0.5);
        let curve_factor = (-dist_from_curve.powf(params.curve_falloff)).exp();
        displacement += params.curve_strength * curve_factor;
    }

    // Add secondary wave
    displacement += params.wave_amplitude * (ry * params.wave_freq).sin();

    // Alternating stripes with displacement
    if stripes(rx + displacement, params.stripe_width) {
        1.0
    } else {
        0.0
    }
}

/// Riley curved stripes pattern.
#[derive(Debug, Clone)]
pub struct RileyCurve {
    params: Params,
}

impl Default for RileyCurve {
    fn default() -> Self {
        Self::golden()
    }
}

impl RileyCurve {
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
impl super::Pattern for RileyCurve {
    fn name(&self) -> &'static str {
        "riley_curve"
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
        let parse_usize = |v: &str| {
            v.parse::<usize>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        match name {
            "stripe_width" => self.params.stripe_width = parse_f32(value)?,
            "curve_strength" => self.params.curve_strength = parse_f32(value)?,
            "curve_falloff" => self.params.curve_falloff = parse_f32(value)?,
            "num_curves" => self.params.num_curves = parse_usize(value)?,
            "wave_amplitude" => self.params.wave_amplitude = parse_f32(value)?,
            "wave_freq" => self.params.wave_freq = parse_f32(value)?,
            "rotation" => self.params.rotation = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for riley_curve", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("stripe_width", format!("{:.1}", self.params.stripe_width)),
            (
                "curve_strength",
                format!("{:.1}", self.params.curve_strength),
            ),
            ("curve_falloff", format!("{:.1}", self.params.curve_falloff)),
            ("num_curves", self.params.num_curves.to_string()),
            (
                "wave_amplitude",
                format!("{:.1}", self.params.wave_amplitude),
            ),
            ("wave_freq", format!("{:.3}", self.params.wave_freq)),
            ("rotation", format!("{:.1}", self.params.rotation)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("stripe_width", "Stripe Width", 8.0, 18.0, 1.0)
                .with_description("Base stripe width in pixels"),
            ParamSpec::slider("curve_strength", "Curve Strength", 40.0, 120.0, 5.0)
                .with_description("How much the stripes bulge"),
            ParamSpec::slider("curve_falloff", "Curve Falloff", 1.5, 3.0, 0.1)
                .with_description("How quickly the bulge fades"),
            ParamSpec::int("num_curves", "Num Curves", Some(1), Some(3))
                .with_description("Number of bulge centers"),
            ParamSpec::slider("wave_amplitude", "Wave Amplitude", 5.0, 25.0, 1.0)
                .with_description("Secondary wave strength"),
            ParamSpec::slider("wave_freq", "Wave Frequency", 0.015, 0.04, 0.005)
                .with_description("Secondary wave frequency"),
            ParamSpec::slider("rotation", "Rotation", 0.0, 180.0, 15.0)
                .with_description("Stripe orientation in degrees"),
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
