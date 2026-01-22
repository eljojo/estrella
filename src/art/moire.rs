//! # Moiré Pattern
//!
//! Overlapping line grids at slight angles creating shimmering interference.
//!
//! ## Description
//!
//! Creates moiré interference patterns by overlaying multiple line grids
//! at different angles. The slight angular offset creates the characteristic
//! shimmering, wave-like optical illusion.

use super::clamp01;
use rand::Rng;
use std::f32::consts::PI;
use std::fmt;

/// Parameters for moiré pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Line spacing for first grid. Default: 6.0
    pub spacing1: f32,
    /// Line spacing for second grid. Default: 6.5
    pub spacing2: f32,
    /// Angle offset in degrees for second grid. Default: 3.0
    pub angle_offset: f32,
    /// Line thickness. Default: 2.0
    pub thickness: f32,
    /// Number of overlapping grids. Default: 2
    pub layers: usize,
    /// Center x offset. Default: 0.0
    pub center_x: f32,
    /// Center y offset. Default: 0.0
    pub center_y: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            spacing1: 6.0,
            spacing2: 6.5,
            angle_offset: 3.0,
            thickness: 2.0,
            layers: 2,
            center_x: 0.0,
            center_y: 0.0,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            spacing1: rng.random_range(4.0..10.0),
            spacing2: rng.random_range(4.0..10.0),
            angle_offset: rng.random_range(1.0..8.0),
            thickness: rng.random_range(1.0..3.0),
            layers: rng.random_range(2..4),
            center_x: rng.random_range(-50.0..50.0),
            center_y: rng.random_range(-50.0..50.0),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "spacing=({:.1},{:.1}) angle={:.1}° thick={:.1} layers={}",
            self.spacing1, self.spacing2, self.angle_offset, self.thickness, self.layers
        )
    }
}

/// Generate a line grid at a given angle.
fn line_grid(x: f32, y: f32, spacing: f32, angle_deg: f32, thickness: f32) -> f32 {
    let angle = angle_deg * PI / 180.0;
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    // Rotate coordinates
    let rotated = x * cos_a + y * sin_a;

    // Distance to nearest line
    let line_pos = rotated / spacing;
    let dist = (line_pos.fract() - 0.5).abs() * spacing;

    // Anti-aliased line
    let half_thick = thickness / 2.0;
    if dist < half_thick {
        1.0
    } else if dist < half_thick + 1.0 {
        1.0 - (dist - half_thick)
    } else {
        0.0
    }
}

pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let cx = width as f32 / 2.0 + params.center_x;
    let cy = height as f32 / 2.0 + params.center_y;

    let xf = x as f32 - cx;
    let yf = y as f32 - cy;

    let mut combined = 0.0;

    // First grid at 0 degrees
    let grid1 = line_grid(xf, yf, params.spacing1, 0.0, params.thickness);
    combined += grid1;

    // Second grid at slight angle
    let grid2 = line_grid(xf, yf, params.spacing2, params.angle_offset, params.thickness);
    combined += grid2;

    // Additional layers if specified
    if params.layers > 2 {
        let grid3 = line_grid(xf, yf, params.spacing1, 90.0, params.thickness);
        combined += grid3;
    }

    if params.layers > 3 {
        let grid4 = line_grid(xf, yf, params.spacing2, 90.0 + params.angle_offset, params.thickness);
        combined += grid4;
    }

    // Normalize based on number of layers
    clamp01(combined / params.layers as f32)
}

/// Moiré interference pattern.
#[derive(Debug, Clone)]
pub struct Moire {
    params: Params,
}

impl Default for Moire {
    fn default() -> Self {
        Self::golden()
    }
}

impl Moire {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for Moire {
    fn name(&self) -> &'static str {
        "moire"
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
        match name {
            "spacing1" => self.params.spacing1 = parse_f32(value)?,
            "spacing2" => self.params.spacing2 = parse_f32(value)?,
            "angle_offset" => self.params.angle_offset = parse_f32(value)?,
            "thickness" => self.params.thickness = parse_f32(value)?,
            "layers" => self.params.layers = parse_usize(value)?,
            "center_x" => self.params.center_x = parse_f32(value)?,
            "center_y" => self.params.center_y = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for moire", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("spacing1", format!("{:.1}", self.params.spacing1)),
            ("spacing2", format!("{:.1}", self.params.spacing2)),
            ("angle_offset", format!("{:.1}", self.params.angle_offset)),
            ("thickness", format!("{:.1}", self.params.thickness)),
            ("layers", self.params.layers.to_string()),
            ("center_x", format!("{:.1}", self.params.center_x)),
            ("center_y", format!("{:.1}", self.params.center_y)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("spacing1", "Spacing 1", 4.0, 10.0, 0.5)
                .with_description("Line spacing for first grid"),
            ParamSpec::slider("spacing2", "Spacing 2", 4.0, 10.0, 0.5)
                .with_description("Line spacing for second grid"),
            ParamSpec::slider("angle_offset", "Angle Offset", 1.0, 8.0, 0.5)
                .with_description("Angle offset in degrees for second grid"),
            ParamSpec::slider("thickness", "Thickness", 1.0, 3.0, 0.5)
                .with_description("Line thickness"),
            ParamSpec::int("layers", "Layers", Some(2), Some(4))
                .with_description("Number of overlapping grids"),
            ParamSpec::slider("center_x", "Center X", -50.0, 50.0, 5.0)
                .with_description("Center x offset"),
            ParamSpec::slider("center_y", "Center Y", -50.0, 50.0, 5.0)
                .with_description("Center y offset"),
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
