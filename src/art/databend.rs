//! # Databend
//!
//! Raw byte visualization with digital artifacts and data corruption aesthetics.
//!
//! ## Description
//!
//! Simulates the look of corrupted binary data rendered visually. Creates
//! patterns that look like raw memory dumps, glitched images, or corrupted
//! file data with blocks, streaks, and digital noise.

use crate::shader::*;
use async_trait::async_trait;
use rand::Rng;
use std::fmt;

/// Parameters for databend pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Block size for data chunks. Default: 8
    pub block_size: usize,
    /// Corruption probability per block. Default: 0.15
    pub corruption_rate: f32,
    /// Streak length multiplier. Default: 3.0
    pub streak_length: f32,
    /// Bit depth simulation (1-8). Default: 4
    pub bit_depth: usize,
    /// Row shift corruption amount. Default: 0.3
    pub row_shift: f32,
    /// Seed for reproducibility. Default: 42
    pub seed: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            block_size: 8,
            corruption_rate: 0.15,
            streak_length: 3.0,
            bit_depth: 4,
            row_shift: 0.3,
            seed: 42,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            block_size: rng.random_range(4..16),
            corruption_rate: rng.random_range(0.05..0.3),
            streak_length: rng.random_range(1.0..6.0),
            bit_depth: rng.random_range(2..8),
            row_shift: rng.random_range(0.1..0.5),
            seed: rng.random(),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "block={} corrupt={:.2} streak={:.1} bits={}",
            self.block_size, self.corruption_rate, self.streak_length, self.bit_depth
        )
    }
}

/// Simulate reading a "byte" from corrupted data.
fn read_byte(block_x: usize, block_y: usize, seed: u32, bit_depth: usize) -> f32 {
    let raw = hash2_f32(block_x as u32, block_y as u32, seed);
    stairs(raw, 1 << bit_depth)
}

pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    // Calculate block coordinates
    let block_x = x / params.block_size;
    let block_y = y / params.block_size;

    // Check for row corruption (entire row gets shifted)
    let row_corrupt = hash_f32(block_y as u32, params.seed.wrapping_add(500)) < params.row_shift;
    let effective_block_x = if row_corrupt {
        let shift = (hash_f32(block_y as u32, params.seed.wrapping_add(600)) * 20.0) as usize;
        block_x.wrapping_add(shift)
    } else {
        block_x
    };

    // Check for block corruption
    let block_hash = hash2_f32(effective_block_x as u32, block_y as u32, params.seed);
    let is_corrupted = block_hash < params.corruption_rate;

    if is_corrupted {
        // Corrupted block: create streak or repeat pattern
        let streak_type = hash(effective_block_x as u32 + params.seed) % 4;

        match streak_type {
            0 => {
                // Horizontal streak: repeat a value across multiple blocks
                let streak_start = effective_block_x.saturating_sub(
                    (hash_f32(block_y as u32, params.seed.wrapping_add(100)) * params.streak_length)
                        as usize,
                );
                read_byte(streak_start, block_y, params.seed, params.bit_depth)
            }
            1 => {
                // Vertical streak: copy from above
                let src_y = block_y.saturating_sub(1);
                read_byte(effective_block_x, src_y, params.seed, params.bit_depth)
            }
            2 => {
                // Noise burst
                let local_x = x % params.block_size;
                let local_y = y % params.block_size;
                hash2_f32(
                    local_x as u32,
                    local_y as u32,
                    params.seed.wrapping_add(block_x as u32),
                )
            }
            _ => {
                // Solid block (stuck bit)
                if hash(effective_block_x as u32 + block_y as u32 + params.seed) % 2 == 0 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    } else {
        // Normal block: read "data"
        let base_value = read_byte(effective_block_x, block_y, params.seed, params.bit_depth);

        // Add subtle bit-level variation within block
        let local_x = x % params.block_size;
        let local_y = y % params.block_size;
        let sub_block = (local_y / 2) * (params.block_size / 2) + (local_x / 2);
        let variation = hash2_f32(
            sub_block as u32,
            effective_block_x as u32 * block_y as u32,
            params.seed.wrapping_add(200),
        ) * 0.1;

        clamp01(base_value + variation - 0.05)
    }
}

/// Databend pattern.
#[derive(Debug, Clone)]
pub struct Databend {
    params: Params,
}

impl Default for Databend {
    fn default() -> Self {
        Self::golden()
    }
}

impl Databend {
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
impl super::Pattern for Databend {
    fn name(&self) -> &'static str {
        "databend"
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
            "block_size" => self.params.block_size = parse_usize(value)?,
            "corruption_rate" => self.params.corruption_rate = parse_f32(value)?,
            "streak_length" => self.params.streak_length = parse_f32(value)?,
            "bit_depth" => self.params.bit_depth = parse_usize(value)?,
            "row_shift" => self.params.row_shift = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            _ => return Err(format!("Unknown param '{}' for databend", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("block_size", self.params.block_size.to_string()),
            (
                "corruption_rate",
                format!("{:.2}", self.params.corruption_rate),
            ),
            ("streak_length", format!("{:.1}", self.params.streak_length)),
            ("bit_depth", self.params.bit_depth.to_string()),
            ("row_shift", format!("{:.2}", self.params.row_shift)),
            ("seed", self.params.seed.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::int("block_size", "Block Size", Some(4), Some(16))
                .with_description("Block size for data chunks"),
            ParamSpec::slider("corruption_rate", "Corruption Rate", 0.05, 0.3, 0.01)
                .with_description("Corruption probability per block"),
            ParamSpec::slider("streak_length", "Streak Length", 1.0, 6.0, 0.5)
                .with_description("Streak length multiplier"),
            ParamSpec::int("bit_depth", "Bit Depth", Some(2), Some(8))
                .with_description("Bit depth simulation (1-8)"),
            ParamSpec::slider("row_shift", "Row Shift", 0.1, 0.5, 0.05)
                .with_description("Row shift corruption amount"),
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
