//! # Scanline Tear
//!
//! Horizontal displacement glitches creating torn, shifted scanline effects.
//!
//! ## Description
//!
//! Creates the appearance of a video signal with horizontal tearing,
//! where portions of scanlines are displaced, creating a broken,
//! glitched aesthetic reminiscent of corrupted video feeds.

use crate::shader::*;
use async_trait::async_trait;
use rand::Rng;
use std::fmt;

/// Parameters for scanline tear pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Base pattern frequency. Default: 0.02
    pub pattern_freq: f32,
    /// Tear intensity (0-1). Default: 0.7
    pub tear_intensity: f32,
    /// Number of tear zones. Default: 12
    pub tear_zones: usize,
    /// Maximum horizontal displacement. Default: 80.0
    pub max_displacement: f32,
    /// Tear thickness in scanlines. Default: 8
    pub tear_thickness: usize,
    /// Add static noise. Default: 0.15
    pub static_noise: f32,
    /// Seed for reproducibility. Default: 42
    pub seed: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            pattern_freq: 0.02,
            tear_intensity: 0.7,
            tear_zones: 12,
            max_displacement: 80.0,
            tear_thickness: 8,
            static_noise: 0.15,
            seed: 42,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            pattern_freq: rng.random_range(0.01..0.04),
            tear_intensity: rng.random_range(0.4..0.9),
            tear_zones: rng.random_range(6..20),
            max_displacement: rng.random_range(40.0..120.0),
            tear_thickness: rng.random_range(4..16),
            static_noise: rng.random_range(0.05..0.3),
            seed: rng.random(),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "freq={:.3} tear={:.2} zones={} disp={:.0}",
            self.pattern_freq, self.tear_intensity, self.tear_zones, self.max_displacement
        )
    }
}

/// Determine if a scanline is in a tear zone and get displacement.
fn get_tear_displacement(y: usize, height: usize, params: &Params) -> f32 {
    let zone_height = height / params.tear_zones.max(1);
    let zone_index = y / zone_height.max(1);
    let pos_in_zone = y % zone_height.max(1);

    // Check if this zone has a tear
    let zone_hash = hash_f32(zone_index as u32, params.seed);
    if zone_hash > params.tear_intensity {
        return 0.0;
    }

    // Check if we're in the tear portion of the zone
    if pos_in_zone >= params.tear_thickness {
        return 0.0;
    }

    // Calculate displacement amount
    let displacement_hash = hash_f32(zone_index as u32, params.seed.wrapping_add(100));
    let direction = if hash(zone_index as u32 + params.seed).is_multiple_of(2) {
        1.0
    } else {
        -1.0
    };

    displacement_hash * params.max_displacement * direction
}

/// Create base pattern (gradient bands).
fn base_pattern(x: f32, y: f32, params: &Params) -> f32 {
    // Diagonal gradient bands
    let band1 = wave_sin(x + y, params.pattern_freq, 0.0);
    // Horizontal bands
    let band2 = wave_sin(y, params.pattern_freq * 2.0, 0.0);
    // Vertical variation
    let band3 = wave_sin(x, params.pattern_freq * 0.5, 0.0);

    band1 * 0.5 + band2 * 0.3 + band3 * 0.2
}

pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;

    // Get tear displacement for this scanline
    let displacement = get_tear_displacement(y, height, params);
    let displaced_x = xf + displacement;

    // Wrap around or clamp
    let effective_x = if displaced_x < 0.0 {
        displaced_x + width as f32
    } else if displaced_x >= width as f32 {
        displaced_x - width as f32
    } else {
        displaced_x
    };

    // Base pattern
    let base = base_pattern(effective_x, yf, params);

    // Add edge artifacts at tear boundaries
    let zone_height = (height / params.tear_zones.max(1)).max(1);
    let pos_in_zone = y % zone_height;
    let edge_artifact = if pos_in_zone == 0 || pos_in_zone == params.tear_thickness {
        0.3
    } else {
        0.0
    };

    // Add static noise
    let noise = hash2_f32(x as u32, y as u32, params.seed.wrapping_add(200)) * params.static_noise;

    // Add horizontal sync artifacts (faint lines)
    let hsync = if scanline(y, 2, 1) { 0.02 } else { 0.0 };

    // Color fringing at tear edges (simulate RGB offset)
    let fringe = if displacement.abs() > 1.0 {
        let fringe_offset = (x as i32 - 2).max(0) as usize;
        let fringe_val = base_pattern(fringe_offset as f32, yf, params);
        (fringe_val - base).abs() * 0.2
    } else {
        0.0
    };

    clamp01(base + edge_artifact + noise + hsync + fringe)
}

/// Scanline tear pattern.
#[derive(Debug, Clone)]
pub struct ScanlineTear {
    params: Params,
}

impl Default for ScanlineTear {
    fn default() -> Self {
        Self::golden()
    }
}

impl ScanlineTear {
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
impl super::Pattern for ScanlineTear {
    fn name(&self) -> &'static str {
        "scanline_tear"
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
        let parse_usize = |v: &str| {
            v.parse::<usize>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        let parse_u32 = |v: &str| {
            v.parse::<u32>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        match name {
            "pattern_freq" => self.params.pattern_freq = parse_f32(value)?,
            "tear_intensity" => self.params.tear_intensity = parse_f32(value)?,
            "tear_zones" => self.params.tear_zones = parse_usize(value)?,
            "max_displacement" => self.params.max_displacement = parse_f32(value)?,
            "tear_thickness" => self.params.tear_thickness = parse_usize(value)?,
            "static_noise" => self.params.static_noise = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            _ => return Err(format!("Unknown param '{}' for scanline_tear", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("pattern_freq", format!("{:.3}", self.params.pattern_freq)),
            (
                "tear_intensity",
                format!("{:.2}", self.params.tear_intensity),
            ),
            ("tear_zones", self.params.tear_zones.to_string()),
            (
                "max_displacement",
                format!("{:.0}", self.params.max_displacement),
            ),
            ("tear_thickness", self.params.tear_thickness.to_string()),
            ("static_noise", format!("{:.2}", self.params.static_noise)),
            ("seed", self.params.seed.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("pattern_freq", "Pattern Frequency", 0.01, 0.04, 0.005)
                .with_description("Base pattern frequency"),
            ParamSpec::slider("tear_intensity", "Tear Intensity", 0.4, 0.9, 0.05)
                .with_description("Tear intensity (0-1)"),
            ParamSpec::int("tear_zones", "Tear Zones", Some(6), Some(20))
                .with_description("Number of tear zones"),
            ParamSpec::slider("max_displacement", "Max Displacement", 40.0, 120.0, 5.0)
                .with_description("Maximum horizontal displacement"),
            ParamSpec::int("tear_thickness", "Tear Thickness", Some(4), Some(16))
                .with_description("Tear thickness in scanlines"),
            ParamSpec::slider("static_noise", "Static Noise", 0.05, 0.3, 0.01)
                .with_description("Add static noise"),
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
