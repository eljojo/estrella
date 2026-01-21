//! # Mycelium Pattern
//!
//! Organic fungal network with branching hyphae creating web-like structures.
//!
//! ## Description
//!
//! Creates organic network patterns that mimic fungal mycelium growth.
//! Uses noise-based branching to create natural-looking interconnected
//! web structures with variable density and organic flow.

use super::clamp01;
use rand::Rng;
use std::fmt;

/// Parameters for the mycelium pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Noise scale for branch direction. Default: 0.015
    pub noise_scale: f32,
    /// Branch density. Default: 0.08
    pub density: f32,
    /// Line thickness. Default: 1.5
    pub thickness: f32,
    /// Seed for reproducibility. Default: 42
    pub seed: u32,
    /// Chaos/randomness in branching. Default: 0.4
    pub chaos: f32,
    /// Network connectivity. Default: 0.6
    pub connectivity: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            noise_scale: 0.015,
            density: 0.08,
            thickness: 1.5,
            seed: 42,
            chaos: 0.4,
            connectivity: 0.6,
        }
    }
}

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            noise_scale: rng.random_range(0.01..0.025),
            density: rng.random_range(0.05..0.12),
            thickness: rng.random_range(1.0..2.5),
            seed: rng.random(),
            chaos: rng.random_range(0.2..0.6),
            connectivity: rng.random_range(0.4..0.8),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "scale={:.3} density={:.2} chaos={:.2} seed={}",
            self.noise_scale, self.density, self.chaos, self.seed
        )
    }
}

/// Simple hash function.
fn hash(mut x: u32) -> u32 {
    x = x.wrapping_mul(0x45d9f3b);
    x ^= x >> 16;
    x = x.wrapping_mul(0x45d9f3b);
    x ^= x >> 16;
    x
}

/// Value noise.
fn noise2d(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - x.floor();
    let yf = y - y.floor();

    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);

    let h = |ix: i32, iy: i32| -> f32 {
        let n = hash(seed.wrapping_add((ix as u32).wrapping_mul(374761393))
            .wrapping_add((iy as u32).wrapping_mul(668265263)));
        (n as f32) / (u32::MAX as f32)
    };

    let n00 = h(xi, yi);
    let n10 = h(xi + 1, yi);
    let n01 = h(xi, yi + 1);
    let n11 = h(xi + 1, yi + 1);

    let nx0 = n00 * (1.0 - u) + n10 * u;
    let nx1 = n01 * (1.0 - u) + n11 * u;
    nx0 * (1.0 - v) + nx1 * v
}

/// Fractal noise.
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

/// Compute mycelium pattern intensity at a pixel.
pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;

    // Sample noise at multiple scales
    let nx = xf * params.noise_scale;
    let ny = yf * params.noise_scale;

    // Primary structure from noise
    let n1 = fbm(nx, ny, 4, params.seed);
    let n2 = fbm(nx * 1.7 + 50.0, ny * 1.7 + 50.0, 3, params.seed.wrapping_add(100));
    let n3 = fbm(nx * 2.3 + 100.0, ny * 2.3, 3, params.seed.wrapping_add(200));

    // Create branching structure using domain warping
    let warp_x = nx + n1 * params.chaos * 3.0;
    let warp_y = ny + n2 * params.chaos * 3.0;

    let warped = fbm(warp_x * 2.0, warp_y * 2.0, 4, params.seed.wrapping_add(300));

    // Create vein-like patterns
    let vein1 = ((warped * 30.0 * params.density).sin().abs()).powf(0.3);
    let vein2 = ((n3 * 25.0 * params.density + 1.5).sin().abs()).powf(0.4);

    // Combine veins with connectivity control
    let combined = vein1 * params.connectivity + vein2 * (1.0 - params.connectivity * 0.5);

    // Add fine detail
    let detail = fbm(nx * 5.0, ny * 5.0, 2, params.seed.wrapping_add(400));
    let with_detail = combined * (0.8 + detail * 0.4);

    // Threshold to create discrete lines
    let threshold = 0.7;
    let sharpness = 8.0;
    let line_value = 1.0 / (1.0 + (-(with_detail - threshold) * sharpness).exp());

    // Modulate by another noise layer for organic breaks
    let breaks = fbm(nx * 3.0 + 150.0, ny * 3.0, 2, params.seed.wrapping_add(500));
    let final_value = line_value * (0.3 + breaks * 0.7);

    clamp01(final_value)
}

/// Mycelium network pattern.
#[derive(Debug, Clone)]
pub struct Mycelium {
    params: Params,
}

impl Default for Mycelium {
    fn default() -> Self {
        Self::golden()
    }
}

impl Mycelium {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for Mycelium {
    fn name(&self) -> &'static str {
        "mycelium"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &self.params)
    }

    fn params_description(&self) -> String {
        self.params.to_string()
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        let parse_f32 = |v: &str| v.parse::<f32>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        let parse_u32 = |v: &str| v.parse::<u32>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        match name {
            "noise_scale" => self.params.noise_scale = parse_f32(value)?,
            "density" => self.params.density = parse_f32(value)?,
            "thickness" => self.params.thickness = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            "chaos" => self.params.chaos = parse_f32(value)?,
            "connectivity" => self.params.connectivity = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for mycelium", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("noise_scale", format!("{:.3}", self.params.noise_scale)),
            ("density", format!("{:.2}", self.params.density)),
            ("thickness", format!("{:.1}", self.params.thickness)),
            ("seed", self.params.seed.to_string()),
            ("chaos", format!("{:.2}", self.params.chaos)),
            ("connectivity", format!("{:.2}", self.params.connectivity)),
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
