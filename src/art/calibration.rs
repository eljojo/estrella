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

/// Calibration pattern with default parameters.
#[derive(Debug, Clone, Default)]
pub struct Calibration;

impl super::Pattern for Calibration {
    fn name(&self) -> &'static str {
        "calibration"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &Params::default())
    }

    fn default_dimensions(&self) -> (usize, usize) {
        (576, 240)
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
