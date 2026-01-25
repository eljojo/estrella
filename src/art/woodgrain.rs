//! # Wood Grain
//!
//! Flowing parallel lines with knots creating natural wood texture.
//!
//! ## Description
//!
//! Simulates the appearance of wood grain with flowing parallel lines,
//! growth rings, and occasional knots. The pattern mimics the natural
//! variations found in different wood types.

use crate::shader::*;
use rand::Rng;
use std::fmt;

/// Parameters for wood grain pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Ring spacing. Default: 8.0
    pub ring_spacing: f32,
    /// Ring thickness. Default: 2.0
    pub ring_thickness: f32,
    /// Grain flow frequency. Default: 0.02
    pub flow_freq: f32,
    /// Grain flow amplitude. Default: 30.0
    pub flow_amp: f32,
    /// Number of knots. Default: 3
    pub num_knots: usize,
    /// Knot size. Default: 40.0
    pub knot_size: f32,
    /// Noise for grain variation. Default: 0.3
    pub noise_amount: f32,
    /// Seed for reproducibility. Default: 42
    pub seed: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            ring_spacing: 8.0,
            ring_thickness: 2.0,
            flow_freq: 0.02,
            flow_amp: 30.0,
            num_knots: 3,
            knot_size: 40.0,
            noise_amount: 0.3,
            seed: 42,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            ring_spacing: rng.random_range(5.0..15.0),
            ring_thickness: rng.random_range(1.0..4.0),
            flow_freq: rng.random_range(0.01..0.04),
            flow_amp: rng.random_range(15.0..50.0),
            num_knots: rng.random_range(0..6),
            knot_size: rng.random_range(25.0..60.0),
            noise_amount: rng.random_range(0.1..0.5),
            seed: rng.random(),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rings={:.1} flow={:.2} knots={} noise={:.2}",
            self.ring_spacing, self.flow_freq, self.num_knots, self.noise_amount
        )
    }
}

/// Generate knot positions.
fn get_knots(num: usize, width: usize, height: usize, seed: u32) -> Vec<(f32, f32)> {
    let mut knots = Vec::with_capacity(num);
    for i in 0..num {
        let kx = hash_f32(i as u32, seed) * width as f32;
        let ky = hash_f32(i as u32, seed.wrapping_add(1000)) * height as f32;
        knots.push((kx, ky));
    }
    knots
}

pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;

    // Flow field distortion
    let flow_x = fbm(xf * 0.005, yf * 0.005, 3, params.seed) - 0.5;
    let _flow_y = fbm(xf * 0.005 + 50.0, yf * 0.005 + 50.0, 3, params.seed.wrapping_add(100)) - 0.5;

    // Apply flow to create curved grain
    let distorted_x = xf + flow_x * params.flow_amp;
    let distorted_y = yf + (yf * params.flow_freq).sin() * params.flow_amp * 0.5;

    // Calculate base ring pattern (distance from vertical axis)
    let mut ring_dist = distorted_x;

    // Add influence from knots
    let knots = get_knots(params.num_knots, width, height, params.seed.wrapping_add(2000));
    for (kx, ky) in &knots {
        let dx = xf - kx;
        let dy = yf - ky;
        let dist = dist(xf, yf, *kx, *ky);

        if dist < params.knot_size * 2.0 {
            // Near a knot, rings curve around it
            let influence = 1.0 - (dist / (params.knot_size * 2.0)).min(1.0);
            let angle = dy.atan2(dx);
            ring_dist += influence * dist * 0.5 * angle.cos();
        }
    }

    // Add noise variation
    let noise = fbm(xf * 0.03, yf * 0.01, 2, params.seed.wrapping_add(3000));
    ring_dist += noise * params.noise_amount * params.ring_spacing;

    // Create rings
    let dist_to_ring = dist_from_cell_center(ring_dist, params.ring_spacing);

    // Ring intensity
    let half_thick = params.ring_thickness / 2.0;
    let ring_value = if dist_to_ring < half_thick {
        1.0
    } else if dist_to_ring < half_thick + 1.0 {
        1.0 - (dist_to_ring - half_thick)
    } else {
        0.0
    };

    // Add knot centers
    let mut knot_value = 0.0;
    for (kx, ky) in &knots {
        let dist = dist(xf, yf, *kx, *ky);

        if dist < params.knot_size {
            // Inside knot: concentric rings
            let knot_ring = (dist / (params.ring_spacing * 0.5)).fract();
            let knot_ring_dist = (knot_ring - 0.5).abs() * params.ring_spacing * 0.5;
            if knot_ring_dist < params.ring_thickness * 0.5 {
                knot_value = 1.0;
            }
            // Darken center
            if dist < params.knot_size * 0.3 {
                knot_value = 0.8;
            }
        }
    }

    // Fine grain texture
    let fine_grain = fbm(xf * 0.1 + 100.0, yf * 0.02, 2, params.seed.wrapping_add(4000));
    let grain_lines = ((distorted_y * 0.5).sin() * 0.5 + 0.5) * 0.1;

    clamp01(ring_value.max(knot_value) + fine_grain * 0.1 + grain_lines)
}

/// Wood grain pattern.
#[derive(Debug, Clone)]
pub struct Woodgrain {
    params: Params,
}

impl Default for Woodgrain {
    fn default() -> Self {
        Self::golden()
    }
}

impl Woodgrain {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for Woodgrain {
    fn name(&self) -> &'static str {
        "woodgrain"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &self.params)
    }

    fn params_description(&self) -> String {
        self.params.to_string()
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        let parse_f32 = |v: &str| v.parse::<f32>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        let parse_usize = |v: &str| v.parse::<usize>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        let parse_u32 = |v: &str| v.parse::<u32>().map_err(|e| format!("Invalid value '{}': {}", v, e));

        match name {
            "ring_spacing" => self.params.ring_spacing = parse_f32(value)?,
            "ring_thickness" => self.params.ring_thickness = parse_f32(value)?,
            "flow_freq" => self.params.flow_freq = parse_f32(value)?,
            "flow_amp" => self.params.flow_amp = parse_f32(value)?,
            "num_knots" => self.params.num_knots = parse_usize(value)?,
            "knot_size" => self.params.knot_size = parse_f32(value)?,
            "noise_amount" => self.params.noise_amount = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            _ => return Err(format!("Unknown param '{}' for woodgrain", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("ring_spacing", format!("{:.1}", self.params.ring_spacing)),
            ("ring_thickness", format!("{:.1}", self.params.ring_thickness)),
            ("flow_freq", format!("{:.3}", self.params.flow_freq)),
            ("flow_amp", format!("{:.1}", self.params.flow_amp)),
            ("num_knots", self.params.num_knots.to_string()),
            ("knot_size", format!("{:.1}", self.params.knot_size)),
            ("noise_amount", format!("{:.2}", self.params.noise_amount)),
            ("seed", self.params.seed.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("ring_spacing", "Ring Spacing", 5.0, 15.0, 0.5)
                .with_description("Ring spacing"),
            ParamSpec::slider("ring_thickness", "Ring Thickness", 1.0, 4.0, 0.5)
                .with_description("Ring thickness"),
            ParamSpec::slider("flow_freq", "Flow Frequency", 0.01, 0.04, 0.005)
                .with_description("Grain flow frequency"),
            ParamSpec::slider("flow_amp", "Flow Amplitude", 15.0, 50.0, 5.0)
                .with_description("Grain flow amplitude"),
            ParamSpec::int("num_knots", "Number of Knots", Some(0), Some(6))
                .with_description("Number of knots"),
            ParamSpec::slider("knot_size", "Knot Size", 25.0, 60.0, 5.0)
                .with_description("Knot size"),
            ParamSpec::slider("noise_amount", "Noise Amount", 0.1, 0.5, 0.05)
                .with_description("Noise for grain variation"),
            ParamSpec::int("seed", "Seed", Some(0), Some(999999))
                .with_description("Seed for reproducibility"),
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
                assert!(v >= 0.0 && v <= 1.0, "value {} out of range at ({}, {})", v, x, y);
            }
        }
    }
}
