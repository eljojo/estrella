//! # Riley Checkerboard Pattern
//!
//! Bridget Riley-inspired distorted checkerboard creating movement illusion.
//!
//! ## Description
//!
//! Creates a checkerboard pattern with wave distortions that make the grid
//! appear to undulate and breathe. Inspired by Riley's "Movement in Squares"
//! and similar pieces where rigid geometry seems to gain organic life.

use crate::shader::*;
use async_trait::async_trait;
use rand::RngExt;
use std::f32::consts::PI;
use std::fmt;

/// Parameters for the Riley checkerboard pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Base cell size in pixels. Default: 20.0
    pub cell_size: f32,
    /// Horizontal wave amplitude. Default: 0.4
    pub wave_amplitude: f32,
    /// Horizontal wave frequency. Default: 0.015
    pub wave_freq: f32,
    /// Vertical compression wave amplitude. Default: 0.3
    pub compress_amplitude: f32,
    /// Vertical compression wave frequency. Default: 0.008
    pub compress_freq: f32,
    /// Phase offset for wave. Default: 0.0
    pub phase: f32,
    /// Whether to add secondary diagonal wave. Default: true
    pub diagonal_wave: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            cell_size: 20.0,
            wave_amplitude: 0.4,
            wave_freq: 0.015,
            compress_amplitude: 0.3,
            compress_freq: 0.008,
            phase: 0.0,
            diagonal_wave: true,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            cell_size: rng.random_range(12.0..28.0),
            wave_amplitude: rng.random_range(0.2..0.6),
            wave_freq: rng.random_range(0.008..0.025),
            compress_amplitude: rng.random_range(0.1..0.5),
            compress_freq: rng.random_range(0.005..0.015),
            phase: rng.random_range(0.0..PI * 2.0),
            diagonal_wave: rng.random_bool(0.7),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cell={:.0} wave=({:.2},{:.3}) compress=({:.2},{:.3})",
            self.cell_size,
            self.wave_amplitude,
            self.wave_freq,
            self.compress_amplitude,
            self.compress_freq
        )
    }
}

/// Compute Riley checkerboard intensity at a pixel.
pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    // Horizontal wave distortion (makes columns wiggle)
    let h_wave = params.wave_amplitude * (yf * params.wave_freq + params.phase).sin();

    // Vertical compression wave (makes rows compress/expand)
    let v_wave = 1.0 + params.compress_amplitude * (xf * params.compress_freq).sin();

    // Optional diagonal wave for extra complexity
    let diag_wave = if params.diagonal_wave {
        0.15 * ((xf + yf) * 0.02 + params.phase * 0.5).sin()
    } else {
        0.0
    };

    // Apply distortions to grid coordinates
    let distorted_x = xf + h_wave * params.cell_size;
    let distorted_y = yf * v_wave + diag_wave * params.cell_size;

    // Checkerboard pattern
    let (gx, gy) = grid_cell(distorted_x, distorted_y, params.cell_size);

    // Alternating black/white
    if checkerboard(gx, gy) { 0.0 } else { 1.0 }
}

/// Riley checkerboard op-art pattern.
#[derive(Debug, Clone)]
pub struct RileyCheck {
    params: Params,
}

impl Default for RileyCheck {
    fn default() -> Self {
        Self::golden()
    }
}

impl RileyCheck {
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
impl super::Pattern for RileyCheck {
    fn name(&self) -> &'static str {
        "riley_check"
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
            "wave_amplitude" => self.params.wave_amplitude = parse_f32(value)?,
            "wave_freq" => self.params.wave_freq = parse_f32(value)?,
            "compress_amplitude" => self.params.compress_amplitude = parse_f32(value)?,
            "compress_freq" => self.params.compress_freq = parse_f32(value)?,
            "phase" => self.params.phase = parse_f32(value)?,
            "diagonal_wave" => self.params.diagonal_wave = parse_bool(value)?,
            _ => return Err(format!("Unknown param '{}' for riley_check", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("cell_size", format!("{:.1}", self.params.cell_size)),
            (
                "wave_amplitude",
                format!("{:.2}", self.params.wave_amplitude),
            ),
            ("wave_freq", format!("{:.3}", self.params.wave_freq)),
            (
                "compress_amplitude",
                format!("{:.2}", self.params.compress_amplitude),
            ),
            ("compress_freq", format!("{:.3}", self.params.compress_freq)),
            ("phase", format!("{:.2}", self.params.phase)),
            ("diagonal_wave", self.params.diagonal_wave.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("cell_size", "Cell Size", 12.0, 48.0, 1.0)
                .with_description("Base cell size in pixels"),
            ParamSpec::slider("wave_amplitude", "Wave Amplitude", 0.2, 0.6, 0.05)
                .with_description("Horizontal wave distortion strength"),
            ParamSpec::slider("wave_freq", "Wave Frequency", 0.008, 0.025, 0.001)
                .with_description("Horizontal wave frequency"),
            ParamSpec::slider("compress_amplitude", "Compress Amplitude", 0.1, 0.5, 0.05)
                .with_description("Vertical compression wave strength"),
            ParamSpec::slider("compress_freq", "Compress Frequency", 0.005, 0.015, 0.001)
                .with_description("Vertical compression wave frequency"),
            ParamSpec::slider("phase", "Phase", 0.0, std::f32::consts::TAU, 0.1)
                .with_description("Phase offset for wave"),
            ParamSpec::bool("diagonal_wave", "Diagonal Wave")
                .with_description("Add secondary diagonal wave"),
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
