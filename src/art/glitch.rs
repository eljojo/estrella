//! # Glitch Effect
//!
//! Blocky columns with horizontal scanlines creating a digital glitch aesthetic.
//!
//! ## Formula
//!
//! ```text
//! col = x / 12
//! base = sin(col * 0.7) * 0.5 + 0.5
//! wobble = 0.5 + 0.5 * sin((x + y * 7) / 15)
//! scan = 1.0 if (y % 24) < 2 else 0.0
//! v = max(0.55 * base + 0.45 * wobble, scan)
//! ```

use crate::shader::*;
use rand::Rng;
use std::fmt;

/// Parameters for the glitch effect.
#[derive(Debug, Clone)]
pub struct Params {
    /// Column width in pixels. Default: 12
    pub column_width: usize,
    /// Column frequency multiplier. Default: 0.7
    pub column_freq: f32,
    /// Wobble frequency divisor. Default: 15.0
    pub wobble_freq: f32,
    /// Wobble vertical multiplier. Default: 7.0
    pub wobble_vert: f32,
    /// Scanline period in rows. Default: 24
    pub scanline_period: usize,
    /// Scanline thickness in rows. Default: 2
    pub scanline_thickness: usize,
    /// Base weight in blend. Default: 0.55
    pub base_weight: f32,
    /// Wobble weight in blend. Default: 0.45
    pub wobble_weight: f32,
    /// Gamma correction. Default: 1.0
    pub gamma: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            column_width: 12,
            column_freq: 0.7,
            wobble_freq: 15.0,
            wobble_vert: 7.0,
            scanline_period: 24,
            scanline_thickness: 2,
            base_weight: 0.55,
            wobble_weight: 0.45,
            gamma: 1.0,
        }
    }
}

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            column_width: rng.random_range(8..20),
            column_freq: rng.random_range(0.4..1.2),
            wobble_freq: rng.random_range(10.0..25.0),
            wobble_vert: rng.random_range(4.0..12.0),
            scanline_period: rng.random_range(16..36),
            scanline_thickness: rng.random_range(1..4),
            base_weight: rng.random_range(0.4..0.7),
            wobble_weight: rng.random_range(0.3..0.6),
            gamma: rng.random_range(0.9..1.3),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "col={} freq={:.2} wobble={:.1} scan={}x{} gamma={:.2}",
            self.column_width, self.column_freq, self.wobble_freq,
            self.scanline_period, self.scanline_thickness, self.gamma
        )
    }
}

/// Compute glitch intensity at a pixel.
///
/// Returns intensity in [0.0, 1.0] with gamma applied.
pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let col = (x / params.column_width) as f32;

    // Base intensity varies by column
    let base = wave_sin(col, params.column_freq, 0.0);

    // Wobble adds horizontal variation
    let wobble = wave_sin(xf + yf * params.wobble_vert, 1.0 / params.wobble_freq, 0.0);

    // Scanlines: dark lines at regular intervals
    let scan = if scanline(y, params.scanline_period, params.scanline_thickness) {
        1.0
    } else {
        0.0
    };

    // Blend base and wobble, then overlay scanlines
    let blended = params.base_weight * base + params.wobble_weight * wobble;
    gamma(clamp01(blended.max(scan)), params.gamma)
}

/// Glitch pattern.
#[derive(Debug, Clone)]
pub struct Glitch {
    params: Params,
}

impl Default for Glitch {
    fn default() -> Self {
        Self::golden()
    }
}

impl Glitch {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for Glitch {
    fn name(&self) -> &'static str {
        "glitch"
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
        match name {
            "column_width" => self.params.column_width = parse_usize(value)?,
            "column_freq" => self.params.column_freq = parse_f32(value)?,
            "wobble_freq" => self.params.wobble_freq = parse_f32(value)?,
            "wobble_vert" => self.params.wobble_vert = parse_f32(value)?,
            "scanline_period" => self.params.scanline_period = parse_usize(value)?,
            "scanline_thickness" => self.params.scanline_thickness = parse_usize(value)?,
            "base_weight" => self.params.base_weight = parse_f32(value)?,
            "wobble_weight" => self.params.wobble_weight = parse_f32(value)?,
            "gamma" => self.params.gamma = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for glitch", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("column_width", self.params.column_width.to_string()),
            ("column_freq", format!("{:.2}", self.params.column_freq)),
            ("wobble_freq", format!("{:.1}", self.params.wobble_freq)),
            ("wobble_vert", format!("{:.1}", self.params.wobble_vert)),
            ("scanline_period", self.params.scanline_period.to_string()),
            ("scanline_thickness", self.params.scanline_thickness.to_string()),
            ("base_weight", format!("{:.2}", self.params.base_weight)),
            ("wobble_weight", format!("{:.2}", self.params.wobble_weight)),
            ("gamma", format!("{:.2}", self.params.gamma)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::int("column_width", "Column Width", Some(8), Some(20))
                .with_description("Column width in pixels"),
            ParamSpec::slider("column_freq", "Column Frequency", 0.4, 1.2, 0.05)
                .with_description("Column frequency multiplier"),
            ParamSpec::slider("wobble_freq", "Wobble Frequency", 10.0, 25.0, 0.5)
                .with_description("Wobble frequency divisor"),
            ParamSpec::slider("wobble_vert", "Wobble Vertical", 4.0, 12.0, 0.5)
                .with_description("Wobble vertical multiplier"),
            ParamSpec::int("scanline_period", "Scanline Period", Some(16), Some(36))
                .with_description("Scanline period in rows"),
            ParamSpec::int("scanline_thickness", "Scanline Thickness", Some(1), Some(4))
                .with_description("Scanline thickness in rows"),
            ParamSpec::slider("base_weight", "Base Weight", 0.4, 0.7, 0.01)
                .with_description("Base weight in blend"),
            ParamSpec::slider("wobble_weight", "Wobble Weight", 0.3, 0.6, 0.01)
                .with_description("Wobble weight in blend"),
            ParamSpec::slider("gamma", "Gamma", 0.9, 1.3, 0.05)
                .with_description("Gamma correction"),
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

    #[test]
    fn test_scanlines() {
        let params = Params::default();
        // Row 0 should have scanline
        let v0 = shade(100, 0, 576, 500, &params);
        // Row 12 should not have scanline
        let v12 = shade(100, 12, 576, 500, &params);
        assert!(v0 >= v12, "Scanline should be darker");
    }
}
