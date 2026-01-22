//! # Stipple
//!
//! Dot density shading like pen & ink illustration.
//!
//! ## Description
//!
//! Creates stippling patterns where dot density varies to represent
//! different tonal values, similar to traditional pen & ink illustration
//! and pointillism techniques.

use rand::Rng;
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

/// Hash function.
fn hash(mut x: u32) -> u32 {
    x = x.wrapping_mul(0x45d9f3b);
    x ^= x >> 16;
    x = x.wrapping_mul(0x45d9f3b);
    x ^= x >> 16;
    x
}

fn hash_f32(x: u32, seed: u32) -> f32 {
    (hash(x.wrapping_add(seed)) as f32) / (u32::MAX as f32)
}

/// Value noise for tonal variation.
fn noise2d(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - x.floor();
    let yf = y - y.floor();

    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);

    let h = |ix: i32, iy: i32| -> f32 {
        let n = hash(
            seed.wrapping_add((ix as u32).wrapping_mul(374761393))
                .wrapping_add((iy as u32).wrapping_mul(668265263)),
        );
        (n as f32) / (u32::MAX as f32)
    };

    let n00 = h(xi, yi);
    let n10 = h(xi + 1, yi);
    let n01 = h(xi, yi + 1);
    let n11 = h(xi + 1, yi + 1);

    let nx0 = n00 * (1.0 - u) + n10 * u;
    let nx1 = n01 * (1.0 - u) + n11 * u;
    nx0 * (1.0 - v) + nx1 * v
}

/// Fractal noise.
fn fbm(x: f32, y: f32, octaves: usize, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut max_value = 0.0;

    for i in 0..octaves {
        value += amplitude * noise2d(x * frequency, y * frequency, seed.wrapping_add(i as u32 * 1000));
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / max_value
}

pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;

    // Get grid cell
    let cell_x = (xf / params.spacing).floor() as i32;
    let cell_y = (yf / params.spacing).floor() as i32;

    // Check nearby cells for dots
    let mut min_dist = f32::MAX;

    for dy in -1..=1 {
        for dx in -1..=1 {
            let cx = cell_x + dx;
            let cy = cell_y + dy;

            // Dot center with jitter
            let cell_hash = (cx as u32).wrapping_mul(374761393).wrapping_add((cy as u32).wrapping_mul(668265263));
            let jitter_x = (hash_f32(cell_hash, params.seed) - 0.5) * params.jitter * params.spacing;
            let jitter_y = (hash_f32(cell_hash, params.seed.wrapping_add(1000)) - 0.5) * params.jitter * params.spacing;

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
            let adjusted_tone = ((tone - 0.5) * params.contrast + 0.5).clamp(0.0, 1.0);

            // Dot radius based on tone (darker = larger dot)
            let radius = params.min_radius + adjusted_tone * (params.max_radius - params.min_radius);

            // Distance to dot center
            let dx_pos = xf - dot_x;
            let dy_pos = yf - dot_y;
            let dist = (dx_pos * dx_pos + dy_pos * dy_pos).sqrt();

            // Normalized distance (0 at center, 1 at edge)
            let norm_dist = dist / radius;
            if norm_dist < min_dist {
                min_dist = norm_dist;
            }
        }
    }

    // Anti-aliased dot
    if min_dist < 1.0 {
        1.0
    } else if min_dist < 1.5 {
        1.0 - (min_dist - 1.0) * 2.0
    } else {
        0.0
    }
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
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

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
        let parse_f32 = |v: &str| v.parse::<f32>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        let parse_u32 = |v: &str| v.parse::<u32>().map_err(|e| format!("Invalid value '{}': {}", v, e));

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
