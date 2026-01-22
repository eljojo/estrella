//! # Crystal Growth Pattern
//!
//! Dendritic crystal growth creating snowflake and frost-like branching structures.
//!
//! ## Description
//!
//! Creates branching crystalline patterns using recursive fractal structures.
//! The pattern mimics ice crystals, snowflakes, frost on windows, or mineral
//! dendrites with configurable symmetry and branching characteristics.

use super::clamp01;
use rand::Rng;
use std::fmt;

/// Parameters for the crystal growth pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Symmetry fold (1 = none, 6 = hexagonal snowflake). Default: 6
    pub symmetry: usize,
    /// Number of recursive branch levels. Default: 4
    pub levels: usize,
    /// Branch length decay per level. Default: 0.6
    pub decay: f32,
    /// Branch angle spread in radians. Default: 0.5
    pub spread: f32,
    /// Line thickness. Default: 2.0
    pub thickness: f32,
    /// Seed for reproducibility. Default: 42
    pub seed: u32,
    /// Base branch length as fraction of size. Default: 0.3
    pub length: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            symmetry: 6,
            levels: 4,
            decay: 0.6,
            spread: 0.5,
            thickness: 2.0,
            seed: 42,
            length: 0.3,
        }
    }
}

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            symmetry: *[1, 4, 5, 6, 8].get(rng.random_range(0..5)).unwrap_or(&6),
            levels: rng.random_range(3..6),
            decay: rng.random_range(0.5..0.75),
            spread: rng.random_range(0.3..0.8),
            thickness: rng.random_range(1.5..3.5),
            seed: rng.random(),
            length: rng.random_range(0.2..0.4),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "sym={} levels={} decay={:.2} spread={:.2} seed={}",
            self.symmetry, self.levels, self.decay, self.spread, self.seed
        )
    }
}

/// Simple hash for deterministic randomness.
fn hash(mut x: u32) -> u32 {
    x = x.wrapping_mul(0x45d9f3b);
    x ^= x >> 16;
    x = x.wrapping_mul(0x45d9f3b);
    x ^= x >> 16;
    x
}

fn hash_float(x: u32) -> f32 {
    (hash(x) as f32) / (u32::MAX as f32)
}

/// Distance from point to line segment.
fn point_to_segment_dist(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 0.0001 {
        return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
    }

    let t = ((px - x1) * dx + (py - y1) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    let proj_x = x1 + t * dx;
    let proj_y = y1 + t * dy;
    ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt()
}

/// Recursively check distance to crystal branches.
fn crystal_distance(
    px: f32, py: f32,
    cx: f32, cy: f32,
    angle: f32,
    length: f32,
    level: usize,
    params: &Params,
    rng_state: &mut u32,
) -> f32 {
    if level == 0 || length < 2.0 {
        return f32::MAX;
    }

    // End point of this branch
    let ex = cx + angle.cos() * length;
    let ey = cy + angle.sin() * length;

    // Distance to this branch segment
    let mut min_dist = point_to_segment_dist(px, py, cx, cy, ex, ey);

    // Early exit if we're far away
    let branch_dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
    if branch_dist > length * 3.0 {
        return min_dist;
    }

    // Recurse into sub-branches
    let next_length = length * params.decay;
    let num_branches = if level > 2 { 2 } else { 3 };

    for i in 0..num_branches {
        *rng_state = hash(*rng_state);
        let angle_offset = (i as f32 - (num_branches - 1) as f32 / 2.0) * params.spread;
        let variation = (hash_float(*rng_state) - 0.5) * 0.2;
        let branch_angle = angle + angle_offset + variation;

        // Branch from a point along this segment
        *rng_state = hash(*rng_state);
        let t = 0.3 + hash_float(*rng_state) * 0.5;
        let bx = cx + angle.cos() * length * t;
        let by = cy + angle.sin() * length * t;

        let d = crystal_distance(px, py, bx, by, branch_angle, next_length, level - 1, params, rng_state);
        min_dist = min_dist.min(d);
    }

    // Also continue the main branch
    let d = crystal_distance(px, py, ex, ey, angle, next_length * 0.8, level - 1, params, rng_state);
    min_dist.min(d)
}

/// Compute crystal pattern intensity at a pixel.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let size = (width.min(height) as f32) / 2.0;
    let base_length = size * params.length;

    let mut min_dist = f32::MAX;

    // For each symmetry fold
    for s in 0..params.symmetry {
        let base_angle = s as f32 * std::f32::consts::TAU / params.symmetry as f32;
        let mut rng_state = params.seed.wrapping_add(s as u32 * 12345);

        let d = crystal_distance(xf, yf, cx, cy, base_angle, base_length, params.levels, params, &mut rng_state);
        min_dist = min_dist.min(d);
    }

    // Convert distance to intensity
    let half_thick = params.thickness / 2.0;
    if min_dist < half_thick {
        1.0
    } else if min_dist < half_thick + 1.5 {
        clamp01(1.0 - (min_dist - half_thick) / 1.5)
    } else {
        0.0
    }
}

/// Crystal growth pattern.
#[derive(Debug, Clone)]
pub struct Crystal {
    params: Params,
}

impl Default for Crystal {
    fn default() -> Self {
        Self::golden()
    }
}

impl Crystal {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for Crystal {
    fn name(&self) -> &'static str {
        "crystal"
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
            "symmetry" => self.params.symmetry = parse_usize(value)?,
            "levels" => self.params.levels = parse_usize(value)?,
            "decay" => self.params.decay = parse_f32(value)?,
            "spread" => self.params.spread = parse_f32(value)?,
            "thickness" => self.params.thickness = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            "length" => self.params.length = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for crystal", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("symmetry", self.params.symmetry.to_string()),
            ("levels", self.params.levels.to_string()),
            ("decay", format!("{:.2}", self.params.decay)),
            ("spread", format!("{:.2}", self.params.spread)),
            ("thickness", format!("{:.1}", self.params.thickness)),
            ("seed", self.params.seed.to_string()),
            ("length", format!("{:.2}", self.params.length)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::select("symmetry", "Symmetry", vec!["1", "4", "5", "6", "8"])
                .with_description("Symmetry fold (1=none, 6=hexagonal snowflake)"),
            ParamSpec::int("levels", "Levels", Some(3), Some(6))
                .with_description("Number of recursive branch levels"),
            ParamSpec::slider("decay", "Decay", 0.5, 0.75, 0.05)
                .with_description("Branch length decay per level"),
            ParamSpec::slider("spread", "Spread", 0.3, 0.8, 0.05)
                .with_description("Branch angle spread in radians"),
            ParamSpec::slider("thickness", "Thickness", 1.5, 3.5, 0.5)
                .with_description("Line thickness"),
            ParamSpec::int("seed", "Seed", Some(0), Some(999999))
                .with_description("Seed for reproducibility"),
            ParamSpec::slider("length", "Length", 0.2, 0.4, 0.02)
                .with_description("Base branch length as fraction of size"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shade_range() {
        let params = Params::default();
        for y in (0..200).step_by(50) {
            for x in (0..200).step_by(50) {
                let v = shade(x, y, 200, 200, &params);
                assert!(v >= 0.0 && v <= 1.0);
            }
        }
    }
}
