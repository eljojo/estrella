//! # Vasarely Hexagonal Cubes Pattern
//!
//! Victor Vasarely-inspired isometric cube tessellation.
//!
//! ## Description
//!
//! Creates a hexagonal grid filled with isometric cubes that appear to be
//! 3D boxes viewed from above. This creates the classic Vasarely optical
//! effect of ambiguous depth - cubes can appear to pop out or recede.
//! Very Windows 98 / early 3D graphics aesthetic.

use crate::shader::*;
use rand::Rng;
use std::fmt;

/// Parameters for the Vasarely hex pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Hexagon size in pixels. Default: 40.0
    pub hex_size: f32,
    /// Top face intensity (0=white, 1=black). Default: 0.0
    pub top_intensity: f32,
    /// Left face intensity. Default: 0.5
    pub left_intensity: f32,
    /// Right face intensity. Default: 1.0
    pub right_intensity: f32,
    /// Gradient strength on faces. Default: 0.2
    pub gradient_strength: f32,
    /// Rotation offset in degrees. Default: 0.0
    pub rotation: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            hex_size: 40.0,
            top_intensity: 0.0,
            left_intensity: 0.5,
            right_intensity: 1.0,
            gradient_strength: 0.2,
            rotation: 0.0,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        // Randomize which face is which shade
        let mut intensities = [
            rng.random_range(0.0..0.3),
            rng.random_range(0.4..0.7),
            rng.random_range(0.8..1.0),
        ];
        // Shuffle
        for i in (1..3).rev() {
            let j = rng.random_range(0..=i);
            intensities.swap(i, j);
        }

        Self {
            hex_size: rng.random_range(25.0..60.0),
            top_intensity: intensities[0],
            left_intensity: intensities[1],
            right_intensity: intensities[2],
            gradient_strength: rng.random_range(0.0..0.4),
            rotation: rng.random_range(0.0..60.0),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "hex={:.0} faces=({:.1},{:.1},{:.1})",
            self.hex_size, self.top_intensity, self.left_intensity, self.right_intensity
        )
    }
}

/// Get the local distance from hexagon center for gradient effects.
fn hex_local_dist(x: f32, y: f32, hex_size: f32) -> f32 {
    let (q, r) = hex_cell(x, y, hex_size);
    let (hx, hy) = hex_center(q, r, hex_size);
    let lx = x - hx;
    let ly = y - hy;
    dist(lx, ly, 0.0, 0.0) / hex_size
}

/// Compute Vasarely hex intensity at a pixel.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    // Apply rotation around center
    let xf = x as f32 - cx;
    let yf = y as f32 - cy;
    let (rx_offset, ry_offset) = rotate_deg(xf, yf, params.rotation);
    let rx = rx_offset + cx;
    let ry = ry_offset + cy;

    // Use shader primitives for hex grid
    let face = hex_cube_face(rx, ry, params.hex_size);
    let local_dist = hex_local_dist(rx, ry, params.hex_size);

    let base_intensity = match face {
        0 => params.top_intensity,
        1 => params.left_intensity,
        _ => params.right_intensity,
    };

    // Add subtle gradient based on distance from center
    let gradient = local_dist * params.gradient_strength;
    clamp01(base_intensity + gradient * 0.3)
}

/// Vasarely hexagonal cubes pattern.
#[derive(Debug, Clone)]
pub struct VasarelyHex {
    params: Params,
}

impl Default for VasarelyHex {
    fn default() -> Self {
        Self::golden()
    }
}

impl VasarelyHex {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for VasarelyHex {
    fn name(&self) -> &'static str {
        "vasarely_hex"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &self.params)
    }

    fn params_description(&self) -> String {
        self.params.to_string()
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        let parse_f32 = |v: &str| v.parse::<f32>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        match name {
            "hex_size" => self.params.hex_size = parse_f32(value)?,
            "top_intensity" => self.params.top_intensity = parse_f32(value)?,
            "left_intensity" => self.params.left_intensity = parse_f32(value)?,
            "right_intensity" => self.params.right_intensity = parse_f32(value)?,
            "gradient_strength" => self.params.gradient_strength = parse_f32(value)?,
            "rotation" => self.params.rotation = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for vasarely_hex", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("hex_size", format!("{:.1}", self.params.hex_size)),
            ("top_intensity", format!("{:.2}", self.params.top_intensity)),
            ("left_intensity", format!("{:.2}", self.params.left_intensity)),
            ("right_intensity", format!("{:.2}", self.params.right_intensity)),
            ("gradient_strength", format!("{:.2}", self.params.gradient_strength)),
            ("rotation", format!("{:.1}", self.params.rotation)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("hex_size", "Hex Size", 25.0, 60.0, 5.0)
                .with_description("Hexagon size in pixels"),
            ParamSpec::slider("top_intensity", "Top Face", 0.0, 1.0, 0.1)
                .with_description("Top face intensity"),
            ParamSpec::slider("left_intensity", "Left Face", 0.0, 1.0, 0.1)
                .with_description("Left face intensity"),
            ParamSpec::slider("right_intensity", "Right Face", 0.0, 1.0, 0.1)
                .with_description("Right face intensity"),
            ParamSpec::slider("gradient_strength", "Gradient", 0.0, 0.4, 0.05)
                .with_description("Gradient strength on faces"),
            ParamSpec::slider("rotation", "Rotation", 0.0, 60.0, 5.0)
                .with_description("Rotation offset in degrees"),
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
