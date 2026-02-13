//! # Vasarely Pattern
//!
//! Victor Vasarely-inspired op art with grid distortion creating a 3D sphere illusion.
//!
//! ## Description
//!
//! Creates a regular grid that bulges outward from the center, creating the
//! optical illusion of a sphere emerging from the surface. The distortion
//! follows a spherical mapping function.

use crate::shader::*;
use async_trait::async_trait;
use rand::RngExt;
use std::fmt;

/// Parameters for the Vasarely pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Grid cell size in pixels. Default: 20.0
    pub cell_size: f32,
    /// Line thickness. Default: 2.0
    pub line_thickness: f32,
    /// Sphere radius as fraction of min dimension. Default: 0.35
    pub sphere_radius: f32,
    /// Bulge strength (how much the sphere protrudes). Default: 0.6
    pub bulge_strength: f32,
    /// Center X as fraction of width. Default: 0.5
    pub center_x: f32,
    /// Center Y as fraction of height. Default: 0.5
    pub center_y: f32,
    /// Whether to invert colors inside sphere. Default: true
    pub invert_sphere: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            cell_size: 20.0,
            line_thickness: 2.0,
            sphere_radius: 0.35,
            bulge_strength: 0.6,
            center_x: 0.5,
            center_y: 0.5,
            invert_sphere: true,
        }
    }
}

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            cell_size: rng.random_range(15.0..30.0),
            line_thickness: rng.random_range(1.5..4.0),
            sphere_radius: rng.random_range(0.25..0.45),
            bulge_strength: rng.random_range(0.4..0.8),
            center_x: rng.random_range(0.3..0.7),
            center_y: rng.random_range(0.3..0.7),
            invert_sphere: rng.random_bool(0.7),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cell={:.0} thick={:.1} radius={:.2} bulge={:.2}",
            self.cell_size, self.line_thickness, self.sphere_radius, self.bulge_strength
        )
    }
}

/// Compute Vasarely pattern intensity at a pixel.
///
/// Returns intensity in [0.0, 1.0].
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;

    // Center of the sphere
    let cx = wf * params.center_x;
    let cy = hf * params.center_y;

    // Distance from center (normalized)
    let dx = xf - cx;
    let dy = yf - cy;
    let min_dim = wf.min(hf);
    let sphere_r = min_dim * params.sphere_radius;

    let sphere_dist = dist(xf, yf, cx, cy);
    let normalized_dist = sphere_dist / sphere_r;

    // Calculate distorted coordinates
    let (grid_x, grid_y, in_sphere) = if normalized_dist < 1.0 {
        // Inside sphere - apply spherical bulge distortion
        // Map to sphere surface using inverse stereographic-like projection
        let z = (1.0 - normalized_dist * normalized_dist).sqrt();
        let bulge_factor = 1.0 + params.bulge_strength * z;

        let distorted_x = cx + dx * bulge_factor;
        let distorted_y = cy + dy * bulge_factor;
        (distorted_x, distorted_y, true)
    } else {
        // Outside sphere - no distortion
        (xf, yf, false)
    };

    // Distance from cell center
    let dx_grid = dist_from_cell_center(grid_x, params.cell_size);
    let dy_grid = dist_from_cell_center(grid_y, params.cell_size);
    let dist_to_line = dx_grid.min(dy_grid);

    // Line rendering with anti-aliasing
    let half_thick = params.line_thickness / 2.0;
    let line_intensity = aa_edge(dist_to_line, half_thick, 1.0);

    // Optionally invert colors inside sphere
    if in_sphere && params.invert_sphere {
        invert(line_intensity)
    } else {
        line_intensity
    }
}

/// Vasarely op-art pattern.
#[derive(Debug, Clone)]
pub struct Vasarely {
    params: Params,
}

impl Default for Vasarely {
    fn default() -> Self {
        Self::golden()
    }
}

impl Vasarely {
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
impl super::Pattern for Vasarely {
    fn name(&self) -> &'static str {
        "vasarely"
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
        let parse_bool = |v: &str| {
            v.parse::<bool>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        match name {
            "cell_size" => self.params.cell_size = parse_f32(value)?,
            "line_thickness" => self.params.line_thickness = parse_f32(value)?,
            "sphere_radius" => self.params.sphere_radius = parse_f32(value)?,
            "bulge_strength" => self.params.bulge_strength = parse_f32(value)?,
            "center_x" => self.params.center_x = parse_f32(value)?,
            "center_y" => self.params.center_y = parse_f32(value)?,
            "invert_sphere" => self.params.invert_sphere = parse_bool(value)?,
            _ => return Err(format!("Unknown param '{}' for vasarely", name)),
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
            ("sphere_radius", format!("{:.2}", self.params.sphere_radius)),
            (
                "bulge_strength",
                format!("{:.2}", self.params.bulge_strength),
            ),
            ("center_x", format!("{:.2}", self.params.center_x)),
            ("center_y", format!("{:.2}", self.params.center_y)),
            ("invert_sphere", self.params.invert_sphere.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("cell_size", "Cell Size", 15.0, 30.0, 1.0)
                .with_description("Grid cell size in pixels"),
            ParamSpec::slider("line_thickness", "Line Thickness", 1.5, 4.0, 0.5)
                .with_description("Line thickness"),
            ParamSpec::slider("sphere_radius", "Sphere Radius", 0.25, 0.45, 0.01)
                .with_description("Sphere radius as fraction of min dimension"),
            ParamSpec::slider("bulge_strength", "Bulge Strength", 0.4, 0.8, 0.05)
                .with_description("How much the sphere protrudes"),
            ParamSpec::slider("center_x", "Center X", 0.3, 0.7, 0.01)
                .with_description("Center X as fraction of width"),
            ParamSpec::slider("center_y", "Center Y", 0.3, 0.7, 0.01)
                .with_description("Center Y as fraction of height"),
            ParamSpec::bool("invert_sphere", "Invert Sphere")
                .with_description("Invert colors inside sphere"),
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
