//! # Vasarely Bubbles Pattern
//!
//! Victor Vasarely-inspired multiple spheres emerging from a checkerboard.
//!
//! ## Description
//!
//! Creates a checkerboard pattern with multiple spherical distortions,
//! like bubbles pushing up through the surface. Inspired by Vasarely's
//! kinetic art and the CBC logo aesthetic of spheres emerging from grids.

use crate::shader::*;
use rand::Rng;
use std::fmt;

/// A single bubble/sphere distortion.
#[derive(Debug, Clone)]
pub struct Bubble {
    /// X position as fraction of width (0.0-1.0)
    pub x: f32,
    /// Y position as fraction of height (0.0-1.0)
    pub y: f32,
    /// Radius as fraction of min dimension
    pub radius: f32,
    /// Bulge strength
    pub strength: f32,
}

/// Parameters for the Vasarely bubbles pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Grid cell size in pixels. Default: 16.0
    pub cell_size: f32,
    /// Line thickness for grid. Default: 0.0 (checkerboard, not grid lines)
    pub line_thickness: f32,
    /// Bubble definitions
    pub bubbles: Vec<Bubble>,
    /// Whether to invert inside bubbles. Default: true
    pub invert_bubbles: bool,
    /// Whether to use checkerboard (true) or grid lines (false). Default: true
    pub checkerboard: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            cell_size: 16.0,
            line_thickness: 0.0,
            bubbles: vec![
                Bubble { x: 0.3, y: 0.35, radius: 0.25, strength: 0.7 },
                Bubble { x: 0.7, y: 0.5, radius: 0.2, strength: 0.6 },
                Bubble { x: 0.5, y: 0.75, radius: 0.15, strength: 0.5 },
            ],
            invert_bubbles: true,
            checkerboard: true,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let num_bubbles = rng.random_range(2..6);
        let mut bubbles = Vec::with_capacity(num_bubbles);

        for _ in 0..num_bubbles {
            bubbles.push(Bubble {
                x: rng.random_range(0.15..0.85),
                y: rng.random_range(0.15..0.85),
                radius: rng.random_range(0.1..0.3),
                strength: rng.random_range(0.4..0.8),
            });
        }

        Self {
            cell_size: rng.random_range(10.0..24.0),
            line_thickness: if rng.random_bool(0.3) { rng.random_range(1.0..3.0) } else { 0.0 },
            bubbles,
            invert_bubbles: rng.random_bool(0.8),
            checkerboard: rng.random_bool(0.7),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cell={:.0} bubbles={} mode={}",
            self.cell_size,
            self.bubbles.len(),
            if self.checkerboard { "check" } else { "grid" }
        )
    }
}

/// Compute Vasarely bubbles intensity at a pixel.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;
    let min_dim = wf.min(hf);

    // Check each bubble for distortion
    let mut distorted_x = xf;
    let mut distorted_y = yf;
    let mut in_any_bubble = false;

    for bubble in &params.bubbles {
        let bx = bubble.x * wf;
        let by = bubble.y * hf;
        let br = bubble.radius * min_dim;

        if in_bulge(xf, yf, bx, by, br) {
            in_any_bubble = true;

            // Get the bulge displacement
            let (bulged_x, bulged_y) = bulge_spherical(xf, yf, bx, by, br, bubble.strength);
            let normalized_dist = dist(xf, yf, bx, by) / br;
            let z = (1.0 - normalized_dist * normalized_dist).sqrt();

            // Accumulate distortion (weighted by distance)
            distorted_x += (bulged_x - xf) * z;
            distorted_y += (bulged_y - yf) * z;
        }
    }

    let intensity = if params.checkerboard || params.line_thickness <= 0.0 {
        // Checkerboard pattern using shader primitive
        if checkerboard_xy(distorted_x, distorted_y, params.cell_size) {
            0.0
        } else {
            1.0
        }
    } else {
        // Grid lines pattern using shader primitives
        let dist_x = dist_to_grid(distorted_x, params.cell_size);
        let dist_y = dist_to_grid(distorted_y, params.cell_size);
        let dist_to_line = dist_x.min(dist_y);

        // Anti-aliased line rendering
        aa_edge(dist_to_line, params.line_thickness / 2.0, 1.0)
    };

    // Invert inside bubbles
    if in_any_bubble && params.invert_bubbles {
        1.0 - intensity
    } else {
        intensity
    }
}

/// Vasarely bubbles pattern.
#[derive(Debug, Clone)]
pub struct VasarelyBubbles {
    params: Params,
}

impl Default for VasarelyBubbles {
    fn default() -> Self {
        Self::golden()
    }
}

impl VasarelyBubbles {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for VasarelyBubbles {
    fn name(&self) -> &'static str {
        "vasarely_bubbles"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &self.params)
    }

    fn params_description(&self) -> String {
        self.params.to_string()
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        let parse_f32 = |v: &str| v.parse::<f32>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        let parse_bool = |v: &str| v.parse::<bool>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        let parse_usize = |v: &str| v.parse::<usize>().map_err(|e| format!("Invalid value '{}': {}", v, e));

        match name {
            "cell_size" => self.params.cell_size = parse_f32(value)?,
            "line_thickness" => self.params.line_thickness = parse_f32(value)?,
            "invert_bubbles" => self.params.invert_bubbles = parse_bool(value)?,
            "checkerboard" => self.params.checkerboard = parse_bool(value)?,
            "num_bubbles" => {
                let n = parse_usize(value)?;
                // Regenerate bubbles with random positions
                let mut rng = rand::rng();
                self.params.bubbles.clear();
                for _ in 0..n {
                    self.params.bubbles.push(Bubble {
                        x: rng.random_range(0.15..0.85),
                        y: rng.random_range(0.15..0.85),
                        radius: rng.random_range(0.1..0.3),
                        strength: rng.random_range(0.4..0.8),
                    });
                }
            }
            _ => return Err(format!("Unknown param '{}' for vasarely_bubbles", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("cell_size", format!("{:.1}", self.params.cell_size)),
            ("line_thickness", format!("{:.1}", self.params.line_thickness)),
            ("num_bubbles", self.params.bubbles.len().to_string()),
            ("invert_bubbles", self.params.invert_bubbles.to_string()),
            ("checkerboard", self.params.checkerboard.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("cell_size", "Cell Size", 10.0, 24.0, 1.0)
                .with_description("Grid cell size in pixels"),
            ParamSpec::slider("line_thickness", "Line Thickness", 0.0, 3.0, 0.5)
                .with_description("Line thickness (0 for solid checkerboard)"),
            ParamSpec::int("num_bubbles", "Num Bubbles", Some(1), Some(8))
                .with_description("Number of bubble distortions"),
            ParamSpec::bool("invert_bubbles", "Invert Bubbles")
                .with_description("Invert colors inside bubbles"),
            ParamSpec::bool("checkerboard", "Checkerboard")
                .with_description("Use checkerboard instead of grid lines"),
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
