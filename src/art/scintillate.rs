//! # Scintillating Grid Pattern
//!
//! Hermann grid variant where dots at intersections appear to flicker.
//!
//! ## Description
//!
//! Creates a grid of dark lines on a light background with white dots at
//! intersections. When viewed, the intersections not directly looked at
//! appear to contain ghostly dark dots that "scintillate" or flicker.
//! This exploits lateral inhibition in the human visual system.

use crate::shader::*;
use async_trait::async_trait;
use rand::RngExt;
use std::fmt;

/// Parameters for the scintillating grid pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Grid cell size in pixels. Default: 32.0
    pub cell_size: f32,
    /// Line thickness. Default: 8.0
    pub line_thickness: f32,
    /// Dot radius at intersections. Default: 4.0
    pub dot_radius: f32,
    /// Background intensity (0=white, 1=black). Default: 0.0
    pub background: f32,
    /// Line intensity. Default: 0.7
    pub line_intensity: f32,
    /// Dot intensity (usually white/light). Default: 0.0
    pub dot_intensity: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            cell_size: 32.0,
            line_thickness: 8.0,
            dot_radius: 4.0,
            background: 0.0,
            line_intensity: 0.7,
            dot_intensity: 0.0,
        }
    }
}

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let cell = rng.random_range(24.0..48.0);
        let line = rng.random_range(6.0..12.0);
        Self {
            cell_size: cell,
            line_thickness: line,
            dot_radius: rng.random_range(line * 0.4..line * 0.7),
            background: rng.random_range(0.0..0.15),
            line_intensity: rng.random_range(0.6..0.9),
            dot_intensity: rng.random_range(0.0..0.1),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cell={:.0} line={:.1} dot={:.1}",
            self.cell_size, self.line_thickness, self.dot_radius
        )
    }
}

/// Compute scintillating grid intensity at a pixel.
///
/// Returns intensity in [0.0, 1.0].
pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;

    // Grid position
    let gx = xf / params.cell_size;
    let gy = yf / params.cell_size;

    // Distance from cell center
    let dx = dist_from_cell_center(xf, params.cell_size);
    let dy = dist_from_cell_center(yf, params.cell_size);

    let half_line = params.line_thickness / 2.0;

    // Check if we're on a grid line
    let on_h_line = dy.abs() > params.cell_size / 2.0 - half_line;
    let on_v_line = dx.abs() > params.cell_size / 2.0 - half_line;

    // Distance to nearest intersection
    let nearest_ix = (gx + 0.5).floor() * params.cell_size;
    let nearest_iy = (gy + 0.5).floor() * params.cell_size;
    let dist_to_intersection = dist(xf, yf, nearest_ix, nearest_iy);

    // White dot at intersection
    if dist_to_intersection < params.dot_radius {
        let edge_dist = params.dot_radius - dist_to_intersection;
        if edge_dist > 1.0 {
            return params.dot_intensity;
        } else {
            // Anti-alias dot edge
            let t = edge_dist;
            return clamp01(lerp(params.line_intensity, params.dot_intensity, t));
        }
    }

    // Grid lines
    if on_h_line || on_v_line {
        return params.line_intensity;
    }

    // Background
    params.background
}

/// Scintillating grid pattern.
#[derive(Debug, Clone)]
pub struct Scintillate {
    params: Params,
}

impl Default for Scintillate {
    fn default() -> Self {
        Self::golden()
    }
}

impl Scintillate {
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
impl super::Pattern for Scintillate {
    fn name(&self) -> &'static str {
        "scintillate"
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
            "cell_size" => self.params.cell_size = parse_f32(value)?,
            "line_thickness" => self.params.line_thickness = parse_f32(value)?,
            "dot_radius" => self.params.dot_radius = parse_f32(value)?,
            "background" => self.params.background = parse_f32(value)?,
            "line_intensity" => self.params.line_intensity = parse_f32(value)?,
            "dot_intensity" => self.params.dot_intensity = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for scintillate", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("cell_size", format!("{:.1}", self.params.cell_size)),
            (
                "line_thickness",
                format!("{:.1}", self.params.line_thickness),
            ),
            ("dot_radius", format!("{:.1}", self.params.dot_radius)),
            ("background", format!("{:.2}", self.params.background)),
            (
                "line_intensity",
                format!("{:.2}", self.params.line_intensity),
            ),
            ("dot_intensity", format!("{:.2}", self.params.dot_intensity)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("cell_size", "Cell Size", 24.0, 48.0, 1.0)
                .with_description("Grid cell size in pixels"),
            ParamSpec::slider("line_thickness", "Line Thickness", 6.0, 12.0, 0.5)
                .with_description("Line thickness"),
            ParamSpec::slider("dot_radius", "Dot Radius", 2.4, 8.4, 0.2)
                .with_description("Dot radius at intersections"),
            ParamSpec::slider("background", "Background", 0.0, 0.15, 0.01)
                .with_description("Background intensity (0=white, 1=black)"),
            ParamSpec::slider("line_intensity", "Line Intensity", 0.6, 0.9, 0.05)
                .with_description("Line intensity"),
            ParamSpec::slider("dot_intensity", "Dot Intensity", 0.0, 0.1, 0.01)
                .with_description("Dot intensity (usually white/light)"),
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
