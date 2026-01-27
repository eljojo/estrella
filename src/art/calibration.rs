//! # Calibration Pattern
//!
//! Diagnostic pattern with borders, diagonals, and bars for testing print quality.
//!
//! ## Description
//!
//! A binary (no grayscale) test pattern featuring:
//! - A 6-pixel border frame showing true print width
//! - X-shaped diagonals testing diagonal accuracy
//! - Vertical bars that increase in width, testing dot precision

use async_trait::async_trait;

/// Parameters for the calibration pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Border width in pixels. Default: 6
    pub border_width: usize,
    /// Diagonal line thickness. Default: 2
    pub diagonal_thickness: usize,
    /// Column width for bar groups. Default: 48
    pub bar_column_width: usize,
    /// Base bar width. Default: 2
    pub bar_base_width: usize,
    /// Vertical margin for bars. Default: 20
    pub bar_margin: usize,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            border_width: 6,
            diagonal_thickness: 2,
            bar_column_width: 48,
            bar_base_width: 2,
            bar_margin: 20,
        }
    }
}

/// Compute calibration pattern shade at a pixel.
///
/// Returns intensity in [0.0, 1.0] (binary: 0.0 or 1.0).
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;

    // Border check
    let border = x < params.border_width
        || x >= width - params.border_width
        || y < params.border_width
        || y >= height - params.border_width;

    // Diagonal from top-left to bottom-right
    let expected_x = (yf * (wf - 1.0) / (hf - 1.0)) as isize;
    let diag1 = (x as isize - expected_x).unsigned_abs() <= params.diagonal_thickness;

    // Diagonal from top-right to bottom-left
    let expected_x2 = ((wf - 1.0) - yf * (wf - 1.0) / (hf - 1.0)) as isize;
    let diag2 = (x as isize - expected_x2).unsigned_abs() <= params.diagonal_thickness;

    // Vertical bars that get thicker every bar_column_width pixels
    let bar_block = x / params.bar_column_width;
    let bar_width = params.bar_base_width + bar_block;
    let in_bar_region = y >= params.bar_margin && y < height - params.bar_margin;
    let bars = (x % params.bar_column_width) < bar_width && in_bar_region;

    if border || diag1 || diag2 || bars {
        1.0
    } else {
        0.0
    }
}

/// Calibration pattern.
#[derive(Debug, Clone)]
pub struct Calibration {
    params: Params,
}

impl Default for Calibration {
    fn default() -> Self {
        Self::golden()
    }
}

impl Calibration {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    // No randomization for calibration - it's a diagnostic pattern
    pub fn random() -> Self {
        Self::golden()
    }
}

#[async_trait]
impl super::Pattern for Calibration {
    fn name(&self) -> &'static str {
        "calibration"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &self.params)
    }

    fn default_dimensions(&self) -> (usize, usize) {
        (576, 240)
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        let parse_usize = |v: &str| v.parse::<usize>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        match name {
            "border_width" => self.params.border_width = parse_usize(value)?,
            "diagonal_thickness" => self.params.diagonal_thickness = parse_usize(value)?,
            "bar_column_width" => self.params.bar_column_width = parse_usize(value)?,
            "bar_base_width" => self.params.bar_base_width = parse_usize(value)?,
            "bar_margin" => self.params.bar_margin = parse_usize(value)?,
            _ => return Err(format!("Unknown param '{}' for calibration", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("border_width", self.params.border_width.to_string()),
            ("diagonal_thickness", self.params.diagonal_thickness.to_string()),
            ("bar_column_width", self.params.bar_column_width.to_string()),
            ("bar_base_width", self.params.bar_base_width.to_string()),
            ("bar_margin", self.params.bar_margin.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::int("border_width", "Border Width", Some(2), Some(10))
                .with_description("Border width in pixels"),
            ParamSpec::int("diagonal_thickness", "Diagonal Thickness", Some(1), Some(4))
                .with_description("Diagonal line thickness"),
            ParamSpec::int("bar_column_width", "Bar Column Width", Some(24), Some(72))
                .with_description("Column width for bar groups"),
            ParamSpec::int("bar_base_width", "Bar Base Width", Some(1), Some(4))
                .with_description("Base bar width"),
            ParamSpec::int("bar_margin", "Bar Margin", Some(10), Some(40))
                .with_description("Vertical margin for bars"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shade_range() {
        let params = Params::default();
        for y in (0..240).step_by(20) {
            for x in (0..576).step_by(50) {
                let v = shade(x, y, 576, 240, &params);
                assert!(v == 0.0 || v == 1.0, "Calibration should be binary");
            }
        }
    }

    #[test]
    fn test_border() {
        let params = Params::default();
        // Corners should be border
        assert_eq!(shade(0, 0, 576, 240, &params), 1.0);
        assert_eq!(shade(575, 0, 576, 240, &params), 1.0);
        assert_eq!(shade(0, 239, 576, 240, &params), 1.0);
    }
}
