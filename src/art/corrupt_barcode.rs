//! # Corrupt Barcode
//!
//! Stretched, torn, glitched barcode aesthetics with digital decay.
//!
//! ## Description
//!
//! Creates a barcode-like pattern that appears corrupted and degraded.
//! Vertical bars stretch, tear, and glitch horizontally with noise
//! interference and data corruption artifacts.

use super::clamp01;
use rand::Rng;
use std::fmt;

/// Parameters for corrupt barcode pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Base bar width. Default: 4
    pub bar_width: usize,
    /// Corruption intensity (0-1). Default: 0.6
    pub corruption: f32,
    /// Horizontal tear frequency. Default: 0.03
    pub tear_freq: f32,
    /// Vertical stretch zones. Default: 5
    pub stretch_zones: usize,
    /// Noise interference amount. Default: 0.2
    pub noise: f32,
    /// Seed for reproducibility. Default: 42
    pub seed: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            bar_width: 4,
            corruption: 0.6,
            tear_freq: 0.03,
            stretch_zones: 5,
            noise: 0.2,
            seed: 42,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            bar_width: rng.random_range(2..8),
            corruption: rng.random_range(0.3..0.9),
            tear_freq: rng.random_range(0.01..0.06),
            stretch_zones: rng.random_range(3..8),
            noise: rng.random_range(0.1..0.4),
            seed: rng.random(),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "bar={} corrupt={:.2} tear={:.3} noise={:.2}",
            self.bar_width, self.corruption, self.tear_freq, self.noise
        )
    }
}

/// Simple hash for pseudo-random values.
fn hash(x: u32) -> u32 {
    let mut h = x;
    h = h.wrapping_mul(0x45d9f3b);
    h ^= h >> 16;
    h = h.wrapping_mul(0x45d9f3b);
    h ^= h >> 16;
    h
}

fn hash_f32(x: u32, seed: u32) -> f32 {
    (hash(x.wrapping_add(seed)) as f32) / (u32::MAX as f32)
}

/// Generate a pseudo-random barcode pattern.
fn barcode_value(bar_index: usize, seed: u32) -> bool {
    // Generate seemingly random but deterministic bar pattern
    let h = hash((bar_index as u32).wrapping_add(seed));
    // Use multiple bits to create varied patterns
    (h % 3) != 0 || ((h >> 8) % 5) < 2
}

pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;

    // Calculate horizontal tear/displacement based on y position
    let tear_noise = (yf * params.tear_freq).sin() * 0.5 + 0.5;
    let tear_amount = tear_noise * params.corruption * 30.0;

    // Add additional tear based on y zones
    let zone_tear = if y % 47 < 3 {
        hash_f32(y as u32 / 47, params.seed) * 20.0 * params.corruption
    } else {
        0.0
    };

    let displaced_x = xf + tear_amount + zone_tear;

    // Calculate stretch zones (vertical bands that stretch/compress)
    let stretch_zone = (xf / wf * params.stretch_zones as f32).floor() as usize;
    let stretch_factor = 0.5 + hash_f32(stretch_zone as u32, params.seed.wrapping_add(100)) * 1.5;

    let effective_x = displaced_x * stretch_factor;
    let bar_index = (effective_x / params.bar_width as f32) as usize;

    // Get base barcode value
    let is_bar = barcode_value(bar_index, params.seed);
    let base = if is_bar { 1.0 } else { 0.0 };

    // Add vertical corruption zones (data decay)
    let decay_zone = (yf / hf * 8.0).floor() as u32;
    let decay = hash_f32(decay_zone.wrapping_add(bar_index as u32), params.seed.wrapping_add(200));
    let corrupted = if decay < params.corruption * 0.3 {
        1.0 - base // Invert some areas
    } else {
        base
    };

    // Add noise interference
    let noise_val = hash_f32(
        (x as u32).wrapping_mul(373).wrapping_add((y as u32).wrapping_mul(677)),
        params.seed.wrapping_add(300),
    );
    let with_noise = corrupted * (1.0 - params.noise) + noise_val * params.noise;

    // Add scanline glitches
    let scanline = if y % 24 < 1 { 0.3 } else { 0.0 };

    clamp01(with_noise + scanline)
}

/// Corrupt barcode pattern.
#[derive(Debug, Clone)]
pub struct CorruptBarcode {
    params: Params,
}

impl Default for CorruptBarcode {
    fn default() -> Self {
        Self::golden()
    }
}

impl CorruptBarcode {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for CorruptBarcode {
    fn name(&self) -> &'static str {
        "corrupt_barcode"
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
            "bar_width" => self.params.bar_width = parse_usize(value)?,
            "corruption" => self.params.corruption = parse_f32(value)?,
            "tear_freq" => self.params.tear_freq = parse_f32(value)?,
            "stretch_zones" => self.params.stretch_zones = parse_usize(value)?,
            "noise" => self.params.noise = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            _ => return Err(format!("Unknown param '{}' for corrupt_barcode", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("bar_width", self.params.bar_width.to_string()),
            ("corruption", format!("{:.2}", self.params.corruption)),
            ("tear_freq", format!("{:.3}", self.params.tear_freq)),
            ("stretch_zones", self.params.stretch_zones.to_string()),
            ("noise", format!("{:.2}", self.params.noise)),
            ("seed", self.params.seed.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::int("bar_width", "Bar Width", Some(2), Some(8))
                .with_description("Base bar width"),
            ParamSpec::slider("corruption", "Corruption", 0.3, 0.9, 0.05)
                .with_description("Corruption intensity (0-1)"),
            ParamSpec::slider("tear_freq", "Tear Frequency", 0.01, 0.06, 0.005)
                .with_description("Horizontal tear frequency"),
            ParamSpec::int("stretch_zones", "Stretch Zones", Some(3), Some(8))
                .with_description("Vertical stretch zones"),
            ParamSpec::slider("noise", "Noise", 0.1, 0.4, 0.05)
                .with_description("Noise interference amount"),
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
                assert!(v >= 0.0 && v <= 1.0, "value {} out of range at ({}, {})", v, x, y);
            }
        }
    }
}
