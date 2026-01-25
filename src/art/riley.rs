//! # Riley Pattern
//!
//! Bridget Riley-inspired op art with wavy parallel lines that appear to move.
//!
//! ## Description
//!
//! Creates horizontal lines with sinusoidal displacement. Multiple wave
//! frequencies combine to create the characteristic optical illusion of
//! movement and depth found in Riley's work.

use crate::shader::*;
use rand::Rng;
use std::fmt;

/// Parameters for the Riley pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Base line spacing in pixels. Default: 8.0
    pub line_spacing: f32,
    /// Primary wave amplitude. Default: 15.0
    pub amplitude1: f32,
    /// Primary wave frequency. Default: 0.02
    pub freq1: f32,
    /// Secondary wave amplitude. Default: 8.0
    pub amplitude2: f32,
    /// Secondary wave frequency. Default: 0.05
    pub freq2: f32,
    /// Line thickness. Default: 3.0
    pub thickness: f32,
    /// Vertical wave component. Default: 0.01
    pub y_freq: f32,
    /// Phase offset. Default: 0.0
    pub phase: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            line_spacing: 8.0,
            amplitude1: 15.0,
            freq1: 0.02,
            amplitude2: 8.0,
            freq2: 0.05,
            thickness: 3.0,
            y_freq: 0.01,
            phase: 0.0,
        }
    }
}

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            line_spacing: rng.random_range(6.0..12.0),
            amplitude1: rng.random_range(10.0..25.0),
            freq1: rng.random_range(0.015..0.035),
            amplitude2: rng.random_range(4.0..12.0),
            freq2: rng.random_range(0.03..0.08),
            thickness: rng.random_range(2.0..5.0),
            y_freq: rng.random_range(0.005..0.02),
            phase: rng.random_range(0.0..std::f32::consts::TAU),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "spacing={:.1} amp=({:.1},{:.1}) freq=({:.3},{:.3}) thick={:.1}",
            self.line_spacing, self.amplitude1, self.amplitude2,
            self.freq1, self.freq2, self.thickness
        )
    }
}

/// Compute Riley pattern intensity at a pixel.
///
/// Returns intensity in [0.0, 1.0].
pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;

    // Calculate wave displacement
    let wave1 = params.amplitude1 * (xf * params.freq1 + params.phase).sin();
    let wave2 = params.amplitude2 * (xf * params.freq2 + yf * params.y_freq).sin();
    let displacement = wave1 + wave2;

    // Displaced y position
    let displaced_y = yf + displacement;

    // Distance from cell center (line at center of each cell)
    let dist_to_line = dist_from_cell_center(displaced_y, params.line_spacing);

    // Anti-aliased line
    aa_edge(dist_to_line, params.thickness / 2.0, 1.0)
}

/// Riley op-art pattern.
#[derive(Debug, Clone)]
pub struct Riley {
    params: Params,
}

impl Default for Riley {
    fn default() -> Self {
        Self::golden()
    }
}

impl Riley {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for Riley {
    fn name(&self) -> &'static str {
        "riley"
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
            "line_spacing" => self.params.line_spacing = parse_f32(value)?,
            "amplitude1" => self.params.amplitude1 = parse_f32(value)?,
            "freq1" => self.params.freq1 = parse_f32(value)?,
            "amplitude2" => self.params.amplitude2 = parse_f32(value)?,
            "freq2" => self.params.freq2 = parse_f32(value)?,
            "thickness" => self.params.thickness = parse_f32(value)?,
            "y_freq" => self.params.y_freq = parse_f32(value)?,
            "phase" => self.params.phase = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for riley", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("line_spacing", format!("{:.1}", self.params.line_spacing)),
            ("amplitude1", format!("{:.1}", self.params.amplitude1)),
            ("freq1", format!("{:.3}", self.params.freq1)),
            ("amplitude2", format!("{:.1}", self.params.amplitude2)),
            ("freq2", format!("{:.3}", self.params.freq2)),
            ("thickness", format!("{:.1}", self.params.thickness)),
            ("y_freq", format!("{:.3}", self.params.y_freq)),
            ("phase", format!("{:.2}", self.params.phase)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("line_spacing", "Line Spacing", 6.0, 12.0, 0.5)
                .with_description("Base line spacing in pixels"),
            ParamSpec::slider("amplitude1", "Amplitude 1", 10.0, 25.0, 0.5)
                .with_description("Primary wave amplitude"),
            ParamSpec::slider("freq1", "Frequency 1", 0.015, 0.035, 0.001)
                .with_description("Primary wave frequency"),
            ParamSpec::slider("amplitude2", "Amplitude 2", 4.0, 12.0, 0.5)
                .with_description("Secondary wave amplitude"),
            ParamSpec::slider("freq2", "Frequency 2", 0.03, 0.08, 0.005)
                .with_description("Secondary wave frequency"),
            ParamSpec::slider("thickness", "Thickness", 2.0, 5.0, 0.5)
                .with_description("Line thickness"),
            ParamSpec::slider("y_freq", "Y Frequency", 0.005, 0.02, 0.001)
                .with_description("Vertical wave component"),
            ParamSpec::slider("phase", "Phase", 0.0, 6.28, 0.1)
                .with_description("Phase offset"),
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
