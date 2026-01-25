//! # Cross-Hatch
//!
//! Engraving-style overlapping line shading.
//!
//! ## Description
//!
//! Creates cross-hatching patterns similar to those used in traditional
//! engraving and pen & ink illustration. Multiple layers of parallel
//! lines at different angles create varying tonal densities.

use crate::shader::*;
use rand::Rng;
use std::fmt;

/// Parameters for cross-hatch pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Line spacing. Default: 6.0
    pub spacing: f32,
    /// Line thickness. Default: 1.5
    pub thickness: f32,
    /// Number of hatch layers (1-4). Default: 3
    pub layers: usize,
    /// Base angle in degrees. Default: 45.0
    pub base_angle: f32,
    /// Angle between layers. Default: 30.0
    pub layer_angle: f32,
    /// Line wobble amount. Default: 0.0
    pub wobble: f32,
    /// Tonal variation frequency. Default: 0.01
    pub tone_freq: f32,
    /// Seed for wobble. Default: 42
    pub seed: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            spacing: 6.0,
            thickness: 1.5,
            layers: 3,
            base_angle: 45.0,
            layer_angle: 30.0,
            wobble: 0.0,
            tone_freq: 0.01,
            seed: 42,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            spacing: rng.random_range(4.0..10.0),
            thickness: rng.random_range(1.0..2.5),
            layers: rng.random_range(2..5),
            base_angle: rng.random_range(0.0..90.0),
            layer_angle: rng.random_range(20.0..45.0),
            wobble: rng.random_range(0.0..1.0),
            tone_freq: rng.random_range(0.005..0.02),
            seed: rng.random(),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "spacing={:.1} thick={:.1} layers={} angle={:.0}°",
            self.spacing, self.thickness, self.layers, self.base_angle
        )
    }
}

/// Render a single hatch layer.
fn hatch_layer(x: f32, y: f32, angle_deg: f32, spacing: f32, thickness: f32, wobble: f32, seed: u32) -> f32 {
    // Add wobble
    let wobble_offset = if wobble > 0.0 {
        let n = noise2d(x * 0.05, y * 0.05, seed);
        (n - 0.5) * wobble * 5.0
    } else {
        0.0
    };

    // Project onto rotated axis (line perpendicular distance)
    let angle = angle_deg * std::f32::consts::PI / 180.0;
    let rotated = x * angle.cos() + y * angle.sin() + wobble_offset;

    // Distance from cell center
    let d = dist_from_cell_center(rotated, spacing);

    // Anti-aliased line
    aa_edge(d, thickness / 2.0, 0.5)
}

pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;

    // Compute tonal value that determines how many layers to show
    let tone = noise2d(xf * params.tone_freq, yf * params.tone_freq, params.seed);
    let tone = tone * 0.5 + 0.25; // Remap to 0.25-0.75 range

    // Determine active layers based on tone
    let active_layers = ((1.0 - tone) * params.layers as f32).ceil() as usize;

    let mut combined: f32 = 0.0;
    let mut max_possible: f32 = 0.0;

    for i in 0..params.layers.min(4) {
        let angle = params.base_angle + i as f32 * params.layer_angle;

        // Adjust spacing per layer for variety
        let layer_spacing = params.spacing * (1.0 + i as f32 * 0.1);

        let layer_value = hatch_layer(
            xf,
            yf,
            angle,
            layer_spacing,
            params.thickness,
            params.wobble,
            params.seed.wrapping_add(i as u32 * 1000),
        );

        // Apply layer based on tone
        if i < active_layers {
            combined += layer_value;
        }
        max_possible += 1.0;
    }

    // Normalize
    clamp01(combined / max_possible.max(1.0) * 1.5)
}

/// Cross-hatch engraving pattern.
#[derive(Debug, Clone)]
pub struct Crosshatch {
    params: Params,
}

impl Default for Crosshatch {
    fn default() -> Self {
        Self::golden()
    }
}

impl Crosshatch {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for Crosshatch {
    fn name(&self) -> &'static str {
        "crosshatch"
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
            "spacing" => self.params.spacing = parse_f32(value)?,
            "thickness" => self.params.thickness = parse_f32(value)?,
            "layers" => self.params.layers = parse_usize(value)?,
            "base_angle" => self.params.base_angle = parse_f32(value)?,
            "layer_angle" => self.params.layer_angle = parse_f32(value)?,
            "wobble" => self.params.wobble = parse_f32(value)?,
            "tone_freq" => self.params.tone_freq = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            _ => return Err(format!("Unknown param '{}' for crosshatch", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("spacing", format!("{:.1}", self.params.spacing)),
            ("thickness", format!("{:.1}", self.params.thickness)),
            ("layers", self.params.layers.to_string()),
            ("base_angle", format!("{:.0}", self.params.base_angle)),
            ("layer_angle", format!("{:.0}", self.params.layer_angle)),
            ("wobble", format!("{:.2}", self.params.wobble)),
            ("tone_freq", format!("{:.3}", self.params.tone_freq)),
            ("seed", self.params.seed.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("spacing", "Spacing", 4.0, 10.0, 0.5)
                .with_description("Line spacing"),
            ParamSpec::slider("thickness", "Thickness", 1.0, 2.5, 0.1)
                .with_description("Line thickness"),
            ParamSpec::int("layers", "Layers", Some(2), Some(4))
                .with_description("Number of hatch layers (1-4)"),
            ParamSpec::slider("base_angle", "Base Angle", 0.0, 90.0, 5.0)
                .with_description("Base angle in degrees"),
            ParamSpec::slider("layer_angle", "Layer Angle", 20.0, 45.0, 1.0)
                .with_description("Angle between layers"),
            ParamSpec::slider("wobble", "Wobble", 0.0, 1.0, 0.05)
                .with_description("Line wobble amount"),
            ParamSpec::slider("tone_freq", "Tone Frequency", 0.005, 0.02, 0.001)
                .with_description("Tonal variation frequency"),
            ParamSpec::int("seed", "Seed", Some(0), Some(999999))
                .with_description("Seed for wobble"),
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
