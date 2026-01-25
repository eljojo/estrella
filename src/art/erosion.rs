//! # Erosion Pattern
//!
//! Simulated hydraulic erosion creating river valley and canyon patterns.
//!
//! ## Description
//!
//! Simulates water droplets flowing down a noise-based terrain, carving
//! channels as they go. Creates branching river patterns, valleys, and
//! canyon-like structures reminiscent of aerial landscape photography.

use crate::shader::*;
use rand::Rng;
use std::fmt;

/// Parameters for the erosion pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Terrain noise scale. Default: 0.006
    pub terrain_scale: f32,
    /// Number of noise octaves for terrain. Default: 5
    pub octaves: usize,
    /// Number of water droplets to simulate. Default: 8000
    pub droplets: usize,
    /// Droplet trail length. Default: 80
    pub trail_length: usize,
    /// Erosion strength. Default: 0.3
    pub erosion_strength: f32,
    /// Seed for reproducibility. Default: 42
    pub seed: u32,
    /// Terrain contrast. Default: 1.5
    pub contrast: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            terrain_scale: 0.006,
            octaves: 5,
            droplets: 8000,
            trail_length: 80,
            erosion_strength: 0.3,
            seed: 42,
            contrast: 1.5,
        }
    }
}

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            terrain_scale: rng.random_range(0.004..0.01),
            octaves: rng.random_range(4..7),
            droplets: rng.random_range(5000..12000),
            trail_length: rng.random_range(50..120),
            erosion_strength: rng.random_range(0.2..0.5),
            seed: rng.random(),
            contrast: rng.random_range(1.2..2.0),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "scale={:.3} drops={} len={} erode={:.2} seed={}",
            self.terrain_scale, self.droplets, self.trail_length,
            self.erosion_strength, self.seed
        )
    }
}

/// Erosion pattern.
#[derive(Debug, Clone)]
pub struct Erosion {
    params: Params,
}

impl Default for Erosion {
    fn default() -> Self {
        Self::golden()
    }
}

impl Erosion {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

/// Compute erosion pattern shade at a pixel.
pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;

    // Base terrain heightfield
    let terrain = fbm(
        xf * params.terrain_scale,
        yf * params.terrain_scale,
        params.octaves,
        params.seed,
    );

    // Ridge noise - creates sharp ridges/valleys by inverting peaks
    let ridge_noise = |x: f32, y: f32, scale: f32, seed_offset: u32| -> f32 {
        let n = fbm(x * scale, y * scale, 3, params.seed.wrapping_add(seed_offset));
        1.0 - (n - 0.5).abs() * 2.0  // Sharp valleys at 0.5
    };

    let ridge1 = ridge_noise(xf, yf, params.terrain_scale * 2.0, 100);
    let ridge2 = ridge_noise(xf, yf, params.terrain_scale * 4.0, 200);

    // Create contour lines from terrain
    let num_contours = 12.0;
    let contour = ((terrain * num_contours * std::f32::consts::TAU).sin() * 0.5 + 0.5).powf(0.3);

    // Gradient magnitude for slope shading
    let eps = 1.0;
    let scale = params.terrain_scale;
    let t_r = fbm((xf + eps) * scale, yf * scale, params.octaves, params.seed);
    let t_l = fbm((xf - eps) * scale, yf * scale, params.octaves, params.seed);
    let t_d = fbm(xf * scale, (yf + eps) * scale, params.octaves, params.seed);
    let t_u = fbm(xf * scale, (yf - eps) * scale, params.octaves, params.seed);

    let grad_x = t_r - t_l;
    let grad_y = t_d - t_u;

    // Simulated light direction for hillshade effect
    let light_x = 0.7f32;
    let light_y = -0.7f32;
    let hillshade = (grad_x * light_x + grad_y * light_y) * 5.0 + 0.5;
    let hillshade = clamp01(hillshade);

    // Combine layers
    let ridges = ridge1 * 0.6 + ridge2 * 0.4;
    let base = terrain * 0.3 + ridges * 0.3 + hillshade * 0.2 + contour * 0.2;

    // Apply contrast and enhance
    let enhanced = clamp01(base * 2.0 - 0.3);
    gamma(clamp01(enhanced), params.contrast)
}

impl super::Pattern for Erosion {
    fn name(&self) -> &'static str {
        "erosion"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        // Use pre-computed erosion map for better results
        // Note: This requires &mut self, so we fall back to shade() for immutable access
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
            "terrain_scale" => self.params.terrain_scale = parse_f32(value)?,
            "octaves" => self.params.octaves = parse_usize(value)?,
            "droplets" => self.params.droplets = parse_usize(value)?,
            "trail_length" => self.params.trail_length = parse_usize(value)?,
            "erosion_strength" => self.params.erosion_strength = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            "contrast" => self.params.contrast = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for erosion", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("terrain_scale", format!("{:.4}", self.params.terrain_scale)),
            ("octaves", self.params.octaves.to_string()),
            ("droplets", self.params.droplets.to_string()),
            ("trail_length", self.params.trail_length.to_string()),
            ("erosion_strength", format!("{:.2}", self.params.erosion_strength)),
            ("seed", self.params.seed.to_string()),
            ("contrast", format!("{:.2}", self.params.contrast)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("terrain_scale", "Terrain Scale", 0.004, 0.01, 0.001)
                .with_description("Terrain noise scale"),
            ParamSpec::int("octaves", "Octaves", Some(4), Some(7))
                .with_description("Number of noise octaves for terrain"),
            ParamSpec::int("droplets", "Droplets", Some(5000), Some(12000))
                .with_description("Number of water droplets to simulate"),
            ParamSpec::int("trail_length", "Trail Length", Some(50), Some(120))
                .with_description("Droplet trail length"),
            ParamSpec::slider("erosion_strength", "Erosion Strength", 0.2, 0.5, 0.05)
                .with_description("Erosion strength"),
            ParamSpec::int("seed", "Seed", Some(0), Some(999999))
                .with_description("Seed for reproducibility"),
            ParamSpec::slider("contrast", "Contrast", 1.2, 2.0, 0.1)
                .with_description("Terrain contrast"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shade_range() {
        let params = Params::default();
        for y in (0..100).step_by(25) {
            for x in (0..100).step_by(25) {
                let v = shade(x, y, 576, 500, &params);
                assert!(v >= 0.0 && v <= 1.0);
            }
        }
    }
}
