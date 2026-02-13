//! # Stipple
//!
//! Dot density shading like pen & ink illustration.
//!
//! ## Description
//!
//! Creates stippling patterns where dot density varies to represent
//! different tonal values, similar to traditional pen & ink illustration
//! and pointillism techniques.

use crate::shader::*;
use async_trait::async_trait;
use rand::RngExt;
use std::fmt;

/// Parameters for stipple pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Dot spacing (grid size). Default: 4.0
    pub spacing: f32,
    /// Maximum dot radius. Default: 1.8
    pub max_radius: f32,
    /// Minimum dot radius. Default: 0.3
    pub min_radius: f32,
    /// Noise scale for tonal variation. Default: 0.015
    pub noise_scale: f32,
    /// Dot position jitter. Default: 0.3
    pub jitter: f32,
    /// Seed for reproducibility. Default: 42
    pub seed: u32,
    /// Contrast adjustment. Default: 1.2
    pub contrast: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            spacing: 4.0,
            max_radius: 1.8,
            min_radius: 0.3,
            noise_scale: 0.015,
            jitter: 0.3,
            seed: 42,
            contrast: 1.2,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            spacing: rng.random_range(3.0..6.0),
            max_radius: rng.random_range(1.2..2.5),
            min_radius: rng.random_range(0.2..0.5),
            noise_scale: rng.random_range(0.008..0.025),
            jitter: rng.random_range(0.1..0.5),
            seed: rng.random(),
            contrast: rng.random_range(0.8..1.5),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "spacing={:.1} radius=({:.1}-{:.1}) jitter={:.2}",
            self.spacing, self.min_radius, self.max_radius, self.jitter
        )
    }
}

pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;

    // Get grid cell
    let (cell_x, cell_y) = grid_cell(xf, yf, params.spacing);

    // Check nearby cells for dots
    let mut min_dist_val = f32::MAX;

    for dy in -1..=1 {
        for ddx in -1..=1 {
            let cx = cell_x + ddx;
            let cy = cell_y + dy;

            // Dot center with jitter
            let jitter_x = (hash2_f32(cx as u32, cy as u32, params.seed) - 0.5)
                * params.jitter
                * params.spacing;
            let jitter_y = (hash2_f32(cx as u32, cy as u32, params.seed.wrapping_add(1000)) - 0.5)
                * params.jitter
                * params.spacing;

            let dot_x = (cx as f32 + 0.5) * params.spacing + jitter_x;
            let dot_y = (cy as f32 + 0.5) * params.spacing + jitter_y;

            // Get tonal value at dot position to determine radius
            let tone = fbm(
                dot_x * params.noise_scale,
                dot_y * params.noise_scale,
                4,
                params.seed.wrapping_add(5000),
            );

            // Apply contrast
            let adjusted_tone = contrast(tone, 0.5, params.contrast);

            // Dot radius based on tone (darker = larger dot)
            let radius =
                params.min_radius + adjusted_tone * (params.max_radius - params.min_radius);

            // Distance to dot center
            let d = dist(xf, yf, dot_x, dot_y);

            // Normalized distance (0 at center, 1 at edge)
            let norm_dist = d / radius;
            if norm_dist < min_dist_val {
                min_dist_val = norm_dist;
            }
        }
    }

    // Anti-aliased dot
    aa_edge(min_dist_val, 1.0, 0.5)
}

/// Stipple dot pattern.
#[derive(Debug, Clone)]
pub struct Stipple {
    params: Params,
}

impl Default for Stipple {
    fn default() -> Self {
        Self::golden()
    }
}

impl Stipple {
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
impl super::Pattern for Stipple {
    fn name(&self) -> &'static str {
        "stipple"
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
        let parse_u32 = |v: &str| {
            v.parse::<u32>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };

        match name {
            "spacing" => self.params.spacing = parse_f32(value)?,
            "max_radius" => self.params.max_radius = parse_f32(value)?,
            "min_radius" => self.params.min_radius = parse_f32(value)?,
            "noise_scale" => self.params.noise_scale = parse_f32(value)?,
            "jitter" => self.params.jitter = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            "contrast" => self.params.contrast = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for stipple", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("spacing", format!("{:.1}", self.params.spacing)),
            ("max_radius", format!("{:.1}", self.params.max_radius)),
            ("min_radius", format!("{:.1}", self.params.min_radius)),
            ("noise_scale", format!("{:.3}", self.params.noise_scale)),
            ("jitter", format!("{:.2}", self.params.jitter)),
            ("seed", self.params.seed.to_string()),
            ("contrast", format!("{:.2}", self.params.contrast)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("spacing", "Spacing", 3.0, 6.0, 0.5)
                .with_description("Dot spacing (grid size)"),
            ParamSpec::slider("max_radius", "Max Radius", 1.2, 2.5, 0.1)
                .with_description("Maximum dot radius"),
            ParamSpec::slider("min_radius", "Min Radius", 0.2, 0.5, 0.05)
                .with_description("Minimum dot radius"),
            ParamSpec::slider("noise_scale", "Noise Scale", 0.008, 0.025, 0.001)
                .with_description("Noise scale for tonal variation"),
            ParamSpec::slider("jitter", "Jitter", 0.1, 0.5, 0.05)
                .with_description("Dot position jitter"),
            ParamSpec::int("seed", "Seed", Some(0), Some(999999))
                .with_description("Seed for reproducibility"),
            ParamSpec::slider("contrast", "Contrast", 0.8, 1.5, 0.1)
                .with_description("Contrast adjustment"),
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
                assert!(
                    v >= 0.0 && v <= 1.0,
                    "value {} out of range at ({}, {})",
                    v,
                    x,
                    y
                );
            }
        }
    }
}
