//! # Reaction-Diffusion
//!
//! Turing patterns creating organic blob/stripe formations like animal skins.
//!
//! ## Description
//!
//! Simulates a Gray-Scott reaction-diffusion system that produces organic
//! patterns similar to animal markings, coral structures, and other
//! biological patterns. Uses pre-computed steady-state approximation.

use crate::shader::*;
use rand::Rng;
use std::fmt;

/// Parameters for reaction-diffusion pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Feed rate (affects pattern type). Default: 0.055
    pub feed: f32,
    /// Kill rate (affects pattern type). Default: 0.062
    pub kill: f32,
    /// Pattern scale. Default: 0.008
    pub scale: f32,
    /// Number of noise octaves. Default: 4
    pub octaves: usize,
    /// Contrast adjustment. Default: 2.0
    pub contrast: f32,
    /// Seed for reproducibility. Default: 42
    pub seed: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            feed: 0.055,
            kill: 0.062,
            scale: 0.008,
            octaves: 4,
            contrast: 2.0,
            seed: 42,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        // Different feed/kill ratios produce different patterns:
        // stripes: f=0.022, k=0.051
        // spots: f=0.035, k=0.065
        // maze: f=0.029, k=0.057
        let pattern_type = rng.random_range(0..3);
        let (feed, kill) = match pattern_type {
            0 => (rng.random_range(0.020..0.028), rng.random_range(0.049..0.055)), // stripes
            1 => (rng.random_range(0.030..0.040), rng.random_range(0.060..0.070)), // spots
            _ => (rng.random_range(0.025..0.035), rng.random_range(0.055..0.063)), // maze
        };
        Self {
            feed,
            kill,
            scale: rng.random_range(0.005..0.015),
            octaves: rng.random_range(3..6),
            contrast: rng.random_range(1.5..3.0),
            seed: rng.random(),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "f={:.3} k={:.3} scale={:.3} contrast={:.1}",
            self.feed, self.kill, self.scale, self.contrast
        )
    }
}


/// Approximate reaction-diffusion steady state using noise-based simulation.
/// This creates patterns similar to true RD without expensive simulation.
pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32 * params.scale;
    let yf = y as f32 * params.scale;

    // Create base noise pattern
    let _n1 = fbm(xf, yf, params.octaves, params.seed);

    // Add domain warping for organic feel
    let warp_x = xf + fbm(xf + 50.0, yf + 50.0, 3, params.seed.wrapping_add(100)) * 2.0;
    let warp_y = yf + fbm(xf + 100.0, yf + 100.0, 3, params.seed.wrapping_add(200)) * 2.0;

    let n2 = fbm(warp_x, warp_y, params.octaves, params.seed.wrapping_add(300));

    // Create reaction-like threshold behavior
    // Feed rate affects the amount of "activator"
    // Kill rate affects how quickly pattern decays
    let threshold = 0.5 + (params.feed - 0.04) * 5.0;
    let sharpness = 10.0 + (params.kill - 0.05) * 100.0;

    // Sigmoid threshold to create sharp boundaries
    let pattern = 1.0 / (1.0 + (-(n2 - threshold) * sharpness).exp());

    // Add second scale for spots vs stripes
    let n3 = fbm(xf * 1.5 + 200.0, yf * 1.5, params.octaves - 1, params.seed.wrapping_add(400));
    let mix = params.feed / 0.06; // Higher feed = more mixed/spotted

    let combined = pattern * mix + n3 * (1.0 - mix) * pattern;

    // Apply contrast
    let centered = combined - 0.5;
    let contrasted = centered * params.contrast + 0.5;

    clamp01(contrasted)
}

/// Reaction-diffusion pattern.
#[derive(Debug, Clone)]
pub struct ReactionDiffusion {
    params: Params,
}

impl Default for ReactionDiffusion {
    fn default() -> Self {
        Self::golden()
    }
}

impl ReactionDiffusion {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for ReactionDiffusion {
    fn name(&self) -> &'static str {
        "reaction_diffusion"
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
            "feed" => self.params.feed = parse_f32(value)?,
            "kill" => self.params.kill = parse_f32(value)?,
            "scale" => self.params.scale = parse_f32(value)?,
            "octaves" => self.params.octaves = parse_usize(value)?,
            "contrast" => self.params.contrast = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            _ => return Err(format!("Unknown param '{}' for reaction_diffusion", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("feed", format!("{:.3}", self.params.feed)),
            ("kill", format!("{:.3}", self.params.kill)),
            ("scale", format!("{:.3}", self.params.scale)),
            ("octaves", self.params.octaves.to_string()),
            ("contrast", format!("{:.1}", self.params.contrast)),
            ("seed", self.params.seed.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("feed", "Feed Rate", 0.02, 0.04, 0.002)
                .with_description("Feed rate (affects pattern type)"),
            ParamSpec::slider("kill", "Kill Rate", 0.049, 0.07, 0.002)
                .with_description("Kill rate (affects pattern type)"),
            ParamSpec::slider("scale", "Scale", 0.005, 0.015, 0.001)
                .with_description("Pattern scale"),
            ParamSpec::int("octaves", "Octaves", Some(3), Some(6))
                .with_description("Number of noise octaves"),
            ParamSpec::slider("contrast", "Contrast", 1.5, 3.0, 0.1)
                .with_description("Contrast adjustment"),
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
