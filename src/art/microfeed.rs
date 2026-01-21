//! # Micro-feed Test Pattern
//!
//! Horizontal lines with progressively increasing spacing.
//!
//! ## Description
//!
//! Displays horizontal 1-pixel lines where each subsequent line has slightly
//! more spacing than the previous one. Creates a gradient effect from dense
//! to sparse lines. Useful for testing printer feed accuracy at different spacings.

/// Parameters for the micro-feed test pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Starting gap in pixels for the first line. Default: 1
    pub start_gap: usize,
    /// How much the gap increases per line. Default: 1
    pub gap_increment: usize,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            start_gap: 1,
            gap_increment: 1,
        }
    }
}

/// Compute micro-feed pattern shade at a pixel.
///
/// Returns intensity in [0.0, 1.0] (binary: 0.0 or 1.0).
pub fn shade(x: usize, y: usize, _width: usize, _height: usize, params: &Params) -> f32 {
    let _ = x; // Unused - pattern is purely horizontal

    // Find which line we're on by summing gaps: gap(n) = start_gap + n * gap_increment
    // Position of line n = sum(i=0 to n-1) of gap(i) = n * start_gap + n*(n-1)/2 * gap_increment
    // We need to find if y matches any line position

    let mut line_pos = 0usize;
    let mut line_num = 0usize;

    while line_pos <= y {
        if line_pos == y {
            return 1.0;
        }
        let gap = params.start_gap + line_num * params.gap_increment;
        line_pos += gap;
        line_num += 1;
    }

    0.0
}

/// Microfeed test pattern with default parameters.
#[derive(Debug, Clone, Default)]
pub struct Microfeed;

impl super::Pattern for Microfeed {
    fn name(&self) -> &'static str {
        "microfeed"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &Params::default())
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
                assert!(v == 0.0 || v == 1.0, "Microfeed should be binary");
            }
        }
    }

    #[test]
    fn test_progressive_spacing() {
        let params = Params::default(); // start_gap=1, gap_increment=1
        // Line 0 at y=0
        assert_eq!(shade(100, 0, 576, 500, &params), 1.0);
        // Gap of 1, so line 1 at y=1
        assert_eq!(shade(100, 1, 576, 500, &params), 1.0);
        // Gap of 2, so line 2 at y=3
        assert_eq!(shade(100, 3, 576, 500, &params), 1.0);
        // Gap of 3, so line 3 at y=6
        assert_eq!(shade(100, 6, 576, 500, &params), 1.0);
        // Gap of 4, so line 4 at y=10
        assert_eq!(shade(100, 10, 576, 500, &params), 1.0);
        // In-between should be 0
        assert_eq!(shade(100, 2, 576, 500, &params), 0.0);
        assert_eq!(shade(100, 5, 576, 500, &params), 0.0);
    }
}
