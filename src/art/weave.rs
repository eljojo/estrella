//! # Fabric Weave
//!
//! Interlocking thread patterns creating woven textile textures.
//!
//! ## Description
//!
//! Simulates various fabric weave patterns including plain weave, twill,
//! satin, and basket weave. The pattern shows the over-under structure
//! of warp and weft threads.

use crate::shader::*;
use async_trait::async_trait;
use rand::RngExt;
use std::fmt;

/// Weave pattern type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeaveType {
    /// Simple over-under (checkerboard)
    Plain,
    /// Diagonal lines (like denim)
    Twill,
    /// Long floats creating smooth surface
    Satin,
    /// Groups of threads (like baskets)
    Basket,
    /// Herringbone zigzag
    Herringbone,
}

impl WeaveType {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "plain" => Some(Self::Plain),
            "twill" => Some(Self::Twill),
            "satin" => Some(Self::Satin),
            "basket" => Some(Self::Basket),
            "herringbone" => Some(Self::Herringbone),
            _ => None,
        }
    }
}

/// Parameters for fabric weave pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Thread width. Default: 6.0
    pub thread_width: f32,
    /// Gap between threads. Default: 1.0
    pub gap: f32,
    /// Weave type. Default: Twill
    pub weave_type: WeaveType,
    /// Twill/satin shift amount. Default: 2
    pub shift: usize,
    /// Thread texture amount. Default: 0.2
    pub texture: f32,
    /// Warp thread darkness (0-1). Default: 0.7
    pub warp_tone: f32,
    /// Weft thread darkness (0-1). Default: 0.5
    pub weft_tone: f32,
    /// Seed for texture randomness. Default: 42
    pub seed: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            thread_width: 6.0,
            gap: 1.0,
            weave_type: WeaveType::Twill,
            shift: 2,
            texture: 0.2,
            warp_tone: 0.7,
            weft_tone: 0.5,
            seed: 42,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            thread_width: rng.random_range(4.0..10.0),
            gap: rng.random_range(0.5..2.0),
            weave_type: match rng.random_range(0..5) {
                0 => WeaveType::Plain,
                1 => WeaveType::Twill,
                2 => WeaveType::Satin,
                3 => WeaveType::Basket,
                _ => WeaveType::Herringbone,
            },
            shift: rng.random_range(1..4),
            texture: rng.random_range(0.1..0.4),
            warp_tone: rng.random_range(0.5..0.9),
            weft_tone: rng.random_range(0.3..0.7),
            seed: rng.random(),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let type_str = match self.weave_type {
            WeaveType::Plain => "plain",
            WeaveType::Twill => "twill",
            WeaveType::Satin => "satin",
            WeaveType::Basket => "basket",
            WeaveType::Herringbone => "herringbone",
        };
        write!(
            f,
            "type={} thread={:.1} gap={:.1}",
            type_str, self.thread_width, self.gap
        )
    }
}

/// Determine if warp is on top at this cell.
fn warp_on_top(cell_x: usize, cell_y: usize, weave_type: WeaveType, shift: usize) -> bool {
    match weave_type {
        WeaveType::Plain => {
            // Simple checkerboard
            (cell_x + cell_y).is_multiple_of(2)
        }
        WeaveType::Twill => {
            // Diagonal pattern
            (cell_x + cell_y * shift).is_multiple_of(shift + 1)
        }
        WeaveType::Satin => {
            // Scattered pattern to hide diagonal
            let pattern_size = shift + 2;
            let offset = (cell_y * 2) % pattern_size;
            (cell_x + offset).is_multiple_of(pattern_size)
        }
        WeaveType::Basket => {
            // Groups of 2
            let group_x = cell_x / 2;
            let group_y = cell_y / 2;
            (group_x + group_y).is_multiple_of(2)
        }
        WeaveType::Herringbone => {
            // Zigzag pattern
            let period = (shift + 1) * 2;
            let phase = cell_y % period;
            let going_right = phase < period / 2;
            if going_right {
                (cell_x + phase).is_multiple_of(shift + 1)
            } else {
                (cell_x + period - phase).is_multiple_of(shift + 1)
            }
        }
    }
}

pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;

    let cell_size = params.thread_width + params.gap;

    // Determine which cell we're in (original formula for backward compatibility)
    let cell_x = (xf / cell_size).floor() as usize;
    let cell_y = (yf / cell_size).floor() as usize;

    // Position within cell
    let local_x = xf % cell_size;
    let local_y = yf % cell_size;

    // Check if we're in a thread or gap
    let in_warp = local_x < params.thread_width;
    let in_weft = local_y < params.thread_width;

    if !in_warp && !in_weft {
        // In gap - show background
        return 0.1;
    }

    // Determine which thread is on top
    let warp_top = warp_on_top(cell_x, cell_y, params.weave_type, params.shift);

    // Thread texture noise
    let texture_noise =
        hash2_f32((xf * 2.0) as u32, (yf * 2.0) as u32, params.seed) * params.texture;

    if in_warp && in_weft {
        // Intersection - show whichever is on top
        if warp_top {
            // Warp thread visible - runs vertically
            let thread_shade = 1.0 - (local_x / params.thread_width - 0.5).abs() * 0.3;
            clamp01(params.warp_tone * thread_shade + texture_noise)
        } else {
            // Weft thread visible - runs horizontally
            let thread_shade = 1.0 - (local_y / params.thread_width - 0.5).abs() * 0.3;
            clamp01(params.weft_tone * thread_shade + texture_noise)
        }
    } else if in_warp {
        // Only warp thread
        let thread_shade = 1.0 - (local_x / params.thread_width - 0.5).abs() * 0.3;
        clamp01(params.warp_tone * thread_shade + texture_noise)
    } else {
        // Only weft thread
        let thread_shade = 1.0 - (local_y / params.thread_width - 0.5).abs() * 0.3;
        clamp01(params.weft_tone * thread_shade + texture_noise)
    }
}

/// Fabric weave pattern.
#[derive(Debug, Clone)]
pub struct Weave {
    params: Params,
}

impl Default for Weave {
    fn default() -> Self {
        Self::golden()
    }
}

impl Weave {
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
impl super::Pattern for Weave {
    fn name(&self) -> &'static str {
        "weave"
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
            "thread_width" => self.params.thread_width = parse_f32(value)?,
            "gap" => self.params.gap = parse_f32(value)?,
            "weave_type" => {
                self.params.weave_type = WeaveType::from_str(value).ok_or_else(|| {
                    format!(
                        "Invalid weave type '{}'. Use: plain, twill, satin, basket, herringbone",
                        value
                    )
                })?;
            }
            "shift" => self.params.shift = parse_usize(value)?,
            "texture" => self.params.texture = parse_f32(value)?,
            "warp_tone" => self.params.warp_tone = parse_f32(value)?,
            "weft_tone" => self.params.weft_tone = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            _ => return Err(format!("Unknown param '{}' for weave", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        let type_str = match self.params.weave_type {
            WeaveType::Plain => "plain",
            WeaveType::Twill => "twill",
            WeaveType::Satin => "satin",
            WeaveType::Basket => "basket",
            WeaveType::Herringbone => "herringbone",
        };
        vec![
            ("thread_width", format!("{:.1}", self.params.thread_width)),
            ("gap", format!("{:.1}", self.params.gap)),
            ("weave_type", type_str.to_string()),
            ("shift", self.params.shift.to_string()),
            ("texture", format!("{:.2}", self.params.texture)),
            ("warp_tone", format!("{:.2}", self.params.warp_tone)),
            ("weft_tone", format!("{:.2}", self.params.weft_tone)),
            ("seed", self.params.seed.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("thread_width", "Thread Width", 4.0, 10.0, 0.5)
                .with_description("Thread width"),
            ParamSpec::slider("gap", "Gap", 0.5, 2.0, 0.1).with_description("Gap between threads"),
            ParamSpec::select(
                "weave_type",
                "Weave Type",
                vec!["plain", "twill", "satin", "basket", "herringbone"],
            )
            .with_description("Weave type"),
            ParamSpec::int("shift", "Shift", Some(1), Some(4))
                .with_description("Twill/satin shift amount"),
            ParamSpec::slider("texture", "Texture", 0.1, 0.4, 0.05)
                .with_description("Thread texture amount"),
            ParamSpec::slider("warp_tone", "Warp Tone", 0.5, 0.9, 0.05)
                .with_description("Warp thread darkness (0-1)"),
            ParamSpec::slider("weft_tone", "Weft Tone", 0.3, 0.7, 0.05)
                .with_description("Weft thread darkness (0-1)"),
            ParamSpec::int("seed", "Seed", Some(0), Some(999999))
                .with_description("Seed for texture randomness"),
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

    #[test]
    fn test_weave_types() {
        for weave_type in [
            WeaveType::Plain,
            WeaveType::Twill,
            WeaveType::Satin,
            WeaveType::Basket,
            WeaveType::Herringbone,
        ] {
            let params = Params {
                weave_type,
                ..Default::default()
            };
            let v = shade(100, 100, 576, 500, &params);
            assert!(v >= 0.0 && v <= 1.0);
        }
    }
}
