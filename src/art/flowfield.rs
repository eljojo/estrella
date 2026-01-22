//! # Flow Field Pattern
//!
//! Flowing lines following noise-based vector fields, creating organic swirling patterns.
//!
//! ## Description
//!
//! Creates swirling, organic patterns reminiscent of smoke, water currents,
//! or Van Gogh's Starry Night. Uses layered noise to create directional flow
//! that's rendered as varying line density.

use super::clamp01;
use rand::Rng;
use std::fmt;

/// Parameters for the flow field pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Noise scale (smaller = larger features). Default: 0.008
    pub noise_scale: f32,
    /// Number of noise octaves. Default: 4
    pub octaves: usize,
    /// Line frequency (higher = more lines). Default: 0.15
    pub line_freq: f32,
    /// Line sharpness. Default: 3.0
    pub sharpness: f32,
    /// Turbulence amount. Default: 0.5
    pub turbulence: f32,
    /// Seed for reproducibility. Default: 42
    pub seed: u32,
    /// Gamma correction. Default: 1.2
    pub gamma: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            noise_scale: 0.008,
            octaves: 4,
            line_freq: 0.15,
            sharpness: 3.0,
            turbulence: 0.5,
            seed: 42,
            gamma: 1.2,
        }
    }
}

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            noise_scale: rng.random_range(0.005..0.015),
            octaves: rng.random_range(3..6),
            line_freq: rng.random_range(0.1..0.25),
            sharpness: rng.random_range(2.0..5.0),
            turbulence: rng.random_range(0.3..0.8),
            seed: rng.random(),
            gamma: rng.random_range(1.0..1.5),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "scale={:.3} oct={} freq={:.2} turb={:.2} seed={}",
            self.noise_scale, self.octaves, self.line_freq, self.turbulence, self.seed
        )
    }
}

/// Simple hash function for deterministic randomness.
fn hash(mut x: u32) -> u32 {
    x = x.wrapping_mul(0x45d9f3b);
    x ^= x >> 16;
    x = x.wrapping_mul(0x45d9f3b);
    x ^= x >> 16;
    x
}

/// Value noise at a point.
fn noise2d(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - x.floor();
    let yf = y - y.floor();

    // Smoothstep interpolation
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);

    // Hash corners
    let h = |ix: i32, iy: i32| -> f32 {
        let n = hash(seed.wrapping_add((ix as u32).wrapping_mul(374761393))
            .wrapping_add((iy as u32).wrapping_mul(668265263)));
        (n as f32) / (u32::MAX as f32)
    };

    let n00 = h(xi, yi);
    let n10 = h(xi + 1, yi);
    let n01 = h(xi, yi + 1);
    let n11 = h(xi + 1, yi + 1);

    // Bilinear interpolation
    let nx0 = n00 * (1.0 - u) + n10 * u;
    let nx1 = n01 * (1.0 - u) + n11 * u;
    nx0 * (1.0 - v) + nx1 * v
}

/// Fractal Brownian Motion noise.
fn fbm(x: f32, y: f32, octaves: usize, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut max_value = 0.0;

    for i in 0..octaves {
        value += amplitude * noise2d(x * frequency, y * frequency, seed.wrapping_add(i as u32 * 1000));
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / max_value
}

/// Compute flow field intensity at a pixel.
///
/// Returns intensity in [0.0, 1.0].
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;

    // Normalized coordinates
    let nx = xf * params.noise_scale;
    let ny = yf * params.noise_scale;

    // Get flow direction from noise
    let angle = fbm(nx, ny, params.octaves, params.seed) * std::f32::consts::TAU;

    // Add turbulence layer
    let turb_x = fbm(nx * 2.0 + 100.0, ny * 2.0, params.octaves, params.seed.wrapping_add(500));
    let turb_y = fbm(nx * 2.0, ny * 2.0 + 100.0, params.octaves, params.seed.wrapping_add(1000));

    // Displace coordinates along flow direction
    let flow_x = xf + angle.cos() * 50.0 * params.turbulence + turb_x * 30.0 * params.turbulence;
    let flow_y = yf + angle.sin() * 50.0 * params.turbulence + turb_y * 30.0 * params.turbulence;

    // Create line pattern perpendicular to flow
    let line_angle = angle + std::f32::consts::FRAC_PI_2;
    let line_coord = flow_x * line_angle.cos() + flow_y * line_angle.sin();

    // Convert to line pattern
    let line_phase = (line_coord * params.line_freq).sin();
    let line_value = (line_phase * params.sharpness).tanh() * 0.5 + 0.5;

    // Add some variation based on position
    let variation = fbm(nx * 3.0 + 200.0, ny * 3.0 + 200.0, 2, params.seed.wrapping_add(2000));
    let final_value = line_value * (0.7 + variation * 0.3);

    // Edge fade
    let edge_x = (xf / wf).min(1.0 - xf / wf) * 10.0;
    let edge_y = (yf / hf).min(1.0 - yf / hf) * 10.0;
    let edge_fade = edge_x.min(edge_y).min(1.0);

    clamp01(final_value * edge_fade).powf(params.gamma)
}

/// Flow field pattern.
#[derive(Debug, Clone)]
pub struct Flowfield {
    params: Params,
}

impl Default for Flowfield {
    fn default() -> Self {
        Self::golden()
    }
}

impl Flowfield {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for Flowfield {
    fn name(&self) -> &'static str {
        "flowfield"
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
            "noise_scale" => self.params.noise_scale = parse_f32(value)?,
            "octaves" => self.params.octaves = parse_usize(value)?,
            "line_freq" => self.params.line_freq = parse_f32(value)?,
            "sharpness" => self.params.sharpness = parse_f32(value)?,
            "turbulence" => self.params.turbulence = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            "gamma" => self.params.gamma = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for flowfield", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("noise_scale", format!("{:.4}", self.params.noise_scale)),
            ("octaves", self.params.octaves.to_string()),
            ("line_freq", format!("{:.2}", self.params.line_freq)),
            ("sharpness", format!("{:.1}", self.params.sharpness)),
            ("turbulence", format!("{:.2}", self.params.turbulence)),
            ("seed", self.params.seed.to_string()),
            ("gamma", format!("{:.2}", self.params.gamma)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("noise_scale", "Noise Scale", 0.005, 0.015, 0.001)
                .with_description("Noise scale (smaller = larger features)"),
            ParamSpec::int("octaves", "Octaves", Some(3), Some(6))
                .with_description("Number of noise octaves"),
            ParamSpec::slider("line_freq", "Line Frequency", 0.1, 0.25, 0.01)
                .with_description("Line frequency (higher = more lines)"),
            ParamSpec::slider("sharpness", "Sharpness", 2.0, 5.0, 0.5)
                .with_description("Line sharpness"),
            ParamSpec::slider("turbulence", "Turbulence", 0.3, 0.8, 0.05)
                .with_description("Turbulence amount"),
            ParamSpec::int("seed", "Seed", Some(0), Some(999999))
                .with_description("Seed for reproducibility"),
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

    #[test]
    fn test_noise_range() {
        for i in 0..100 {
            let v = noise2d(i as f32 * 0.1, i as f32 * 0.07, 42);
            assert!(v >= 0.0 && v <= 1.0);
        }
    }
}
