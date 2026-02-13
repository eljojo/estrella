//! # Zebra Pattern
//!
//! Undulating organic stripes inspired by Bridget Riley's zebra-like works.
//!
//! ## Description
//!
//! Creates flowing black and white stripes that twist and undulate across
//! the image, reminiscent of both Riley's op art and natural zebra patterns.
//! Multiple wave components create organic, almost liquid movement.

use crate::shader::*;
use async_trait::async_trait;
use rand::RngExt;
use std::f32::consts::PI;
use std::fmt;

/// Parameters for the zebra pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Base stripe width in pixels. Default: 18.0
    pub stripe_width: f32,
    /// Primary wave amplitude. Default: 40.0
    pub wave1_amp: f32,
    /// Primary wave frequency. Default: 0.012
    pub wave1_freq: f32,
    /// Secondary wave amplitude. Default: 20.0
    pub wave2_amp: f32,
    /// Secondary wave frequency. Default: 0.025
    pub wave2_freq: f32,
    /// Tertiary wave amplitude (adds fine detail). Default: 8.0
    pub wave3_amp: f32,
    /// Tertiary wave frequency. Default: 0.06
    pub wave3_freq: f32,
    /// Base stripe direction in degrees. Default: 90.0 (vertical)
    pub direction: f32,
    /// Phase offset. Default: 0.0
    pub phase: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            stripe_width: 18.0,
            wave1_amp: 40.0,
            wave1_freq: 0.012,
            wave2_amp: 20.0,
            wave2_freq: 0.025,
            wave3_amp: 8.0,
            wave3_freq: 0.06,
            direction: 90.0,
            phase: 0.0,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            stripe_width: rng.random_range(10.0..28.0),
            wave1_amp: rng.random_range(20.0..60.0),
            wave1_freq: rng.random_range(0.008..0.02),
            wave2_amp: rng.random_range(10.0..35.0),
            wave2_freq: rng.random_range(0.015..0.04),
            wave3_amp: rng.random_range(3.0..15.0),
            wave3_freq: rng.random_range(0.04..0.1),
            direction: rng.random_range(0.0..180.0),
            phase: rng.random_range(0.0..PI * 2.0),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "stripe={:.0} waves=({:.0},{:.0},{:.0}) dir={:.0}°",
            self.stripe_width, self.wave1_amp, self.wave2_amp, self.wave3_amp, self.direction
        )
    }
}

/// Compute zebra intensity at a pixel.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let wf = width as f32;
    let hf = height as f32;

    // Center-relative coordinates
    let (dx, dy) = center_coords(x as f32, y as f32, wf, hf);

    // Rotate coordinates based on direction
    // u is along stripe direction, v is perpendicular
    let (u, v) = rotate_deg(dx, dy, -params.direction);

    // Calculate wave displacement (perpendicular to stripes)
    let wave1 = params.wave1_amp * (u * params.wave1_freq + params.phase).sin();
    let wave2 = params.wave2_amp * (u * params.wave2_freq + v * 0.005 + params.phase * 0.7).sin();
    let wave3 = params.wave3_amp * (u * params.wave3_freq + params.phase * 1.3).sin();

    let total_displacement = wave1 + wave2 + wave3;

    // Stripe pattern with displacement
    if stripes(v + total_displacement, params.stripe_width) {
        1.0
    } else {
        0.0
    }
}

/// Zebra stripe pattern.
#[derive(Debug, Clone)]
pub struct Zebra {
    params: Params,
}

impl Default for Zebra {
    fn default() -> Self {
        Self::golden()
    }
}

impl Zebra {
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
impl super::Pattern for Zebra {
    fn name(&self) -> &'static str {
        "zebra"
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
            "stripe_width" => self.params.stripe_width = parse_f32(value)?,
            "wave1_amp" => self.params.wave1_amp = parse_f32(value)?,
            "wave1_freq" => self.params.wave1_freq = parse_f32(value)?,
            "wave2_amp" => self.params.wave2_amp = parse_f32(value)?,
            "wave2_freq" => self.params.wave2_freq = parse_f32(value)?,
            "wave3_amp" => self.params.wave3_amp = parse_f32(value)?,
            "wave3_freq" => self.params.wave3_freq = parse_f32(value)?,
            "direction" => self.params.direction = parse_f32(value)?,
            "phase" => self.params.phase = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for zebra", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("stripe_width", format!("{:.1}", self.params.stripe_width)),
            ("wave1_amp", format!("{:.1}", self.params.wave1_amp)),
            ("wave1_freq", format!("{:.3}", self.params.wave1_freq)),
            ("wave2_amp", format!("{:.1}", self.params.wave2_amp)),
            ("wave2_freq", format!("{:.3}", self.params.wave2_freq)),
            ("wave3_amp", format!("{:.1}", self.params.wave3_amp)),
            ("wave3_freq", format!("{:.3}", self.params.wave3_freq)),
            ("direction", format!("{:.1}", self.params.direction)),
            ("phase", format!("{:.2}", self.params.phase)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("stripe_width", "Stripe Width", 10.0, 28.0, 1.0)
                .with_description("Base stripe width in pixels"),
            ParamSpec::slider("wave1_amp", "Wave 1 Amp", 20.0, 60.0, 5.0)
                .with_description("Primary wave amplitude"),
            ParamSpec::slider("wave1_freq", "Wave 1 Freq", 0.008, 0.02, 0.001)
                .with_description("Primary wave frequency"),
            ParamSpec::slider("wave2_amp", "Wave 2 Amp", 10.0, 35.0, 2.5)
                .with_description("Secondary wave amplitude"),
            ParamSpec::slider("wave2_freq", "Wave 2 Freq", 0.015, 0.04, 0.002)
                .with_description("Secondary wave frequency"),
            ParamSpec::slider("wave3_amp", "Wave 3 Amp", 3.0, 15.0, 1.0)
                .with_description("Tertiary wave amplitude"),
            ParamSpec::slider("wave3_freq", "Wave 3 Freq", 0.04, 0.1, 0.005)
                .with_description("Tertiary wave frequency"),
            ParamSpec::slider("direction", "Direction", 0.0, 180.0, 15.0)
                .with_description("Base stripe direction in degrees"),
            ParamSpec::slider("phase", "Phase", 0.0, std::f32::consts::TAU, 0.1)
                .with_description("Phase offset"),
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
