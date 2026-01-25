//! Line and stripe pattern functions.

use super::distance::dist_to_grid;
use super::transform::rotate;
use std::f32::consts::PI;

/// Parallel lines with anti-aliasing.
///
/// Returns intensity [0, 1] where 1 is on a line, 0 is between lines.
///
/// # Parameters
/// - `coord`: Position along the axis perpendicular to lines
/// - `spacing`: Distance between line centers
/// - `thickness`: Line thickness
pub fn lines(coord: f32, spacing: f32, thickness: f32) -> f32 {
    let dist = dist_to_grid(coord, spacing);
    aa_edge(dist, thickness / 2.0, 1.0)
}

/// Rotated parallel lines.
///
/// Lines at an arbitrary angle.
pub fn lines_rotated(x: f32, y: f32, angle: f32, spacing: f32, thickness: f32) -> f32 {
    // Rotate coordinates so lines become horizontal
    let (rx, _ry) = rotate(x, y, -angle);
    lines(rx, spacing, thickness)
}

/// Rotated parallel lines (angle in degrees).
pub fn lines_rotated_deg(x: f32, y: f32, angle_deg: f32, spacing: f32, thickness: f32) -> f32 {
    lines_rotated(x, y, angle_deg * PI / 180.0, spacing, thickness)
}

/// Anti-aliased edge function.
///
/// Creates a smooth transition from 1 to 0 as distance increases past the edge.
///
/// # Parameters
/// - `dist`: Distance from the edge (0 = on edge)
/// - `half_thickness`: Half the total thickness (distance from center to edge)
/// - `aa_width`: Width of the anti-aliasing transition (typically 1.0)
///
/// # Returns
/// - 1.0 if dist < half_thickness
/// - Smooth falloff from 1 to 0 over aa_width
/// - 0.0 if dist > half_thickness + aa_width
#[inline]
pub fn aa_edge(dist: f32, half_thickness: f32, aa_width: f32) -> f32 {
    if dist < half_thickness {
        1.0
    } else if dist < half_thickness + aa_width {
        1.0 - (dist - half_thickness) / aa_width
    } else {
        0.0
    }
}

/// Binary stripe pattern (no anti-aliasing).
///
/// Returns true for pixels on odd stripes, false for even stripes.
#[inline]
pub fn stripes(coord: f32, width: f32) -> bool {
    (coord / width).floor() as i32 % 2 != 0
}

/// Stripe pattern with displacement.
///
/// The coordinate is displaced before determining stripe membership.
#[inline]
pub fn stripes_displaced(coord: f32, displacement: f32, width: f32) -> bool {
    stripes(coord + displacement, width)
}

/// Grid lines (horizontal and vertical).
///
/// Returns intensity based on distance to nearest grid line.
pub fn grid_lines(x: f32, y: f32, spacing: f32, thickness: f32) -> f32 {
    let dx = dist_to_grid(x, spacing);
    let dy = dist_to_grid(y, spacing);
    let dist = dx.min(dy);
    aa_edge(dist, thickness / 2.0, 1.0)
}

/// Crosshatch pattern - two sets of rotated lines overlaid.
pub fn crosshatch(x: f32, y: f32, spacing: f32, thickness: f32, angle1: f32, angle2: f32) -> f32 {
    let l1 = lines_rotated(x, y, angle1, spacing, thickness);
    let l2 = lines_rotated(x, y, angle2, spacing, thickness);
    // Combine with max (either line visible)
    l1.max(l2)
}

/// Dashed line pattern.
///
/// Creates dashes along a line with the given dash and gap lengths.
#[inline]
pub fn dashed(coord: f32, dash_length: f32, gap_length: f32) -> bool {
    let period = dash_length + gap_length;
    let pos = coord.rem_euclid(period);
    pos < dash_length
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lines_on_line() {
        // At a line position
        let v = lines(0.0, 10.0, 2.0);
        assert!(v > 0.9, "should be on line: {}", v);
    }

    #[test]
    fn test_lines_between_lines() {
        // At center between lines
        let v = lines(5.0, 10.0, 2.0);
        assert!(v < 0.1, "should be between lines: {}", v);
    }

    #[test]
    fn test_aa_edge() {
        assert_eq!(aa_edge(0.0, 1.0, 1.0), 1.0); // On edge
        assert_eq!(aa_edge(0.5, 1.0, 1.0), 1.0); // Inside
        assert!((aa_edge(1.5, 1.0, 1.0) - 0.5).abs() < 1e-6); // Halfway through AA
        assert_eq!(aa_edge(2.5, 1.0, 1.0), 0.0); // Outside
    }

    #[test]
    fn test_stripes() {
        assert!(!stripes(0.5, 10.0));
        assert!(stripes(15.0, 10.0));
        assert!(!stripes(25.0, 10.0));
    }

    #[test]
    fn test_dashed() {
        assert!(dashed(0.0, 5.0, 3.0)); // In dash
        assert!(dashed(4.0, 5.0, 3.0)); // In dash
        assert!(!dashed(6.0, 5.0, 3.0)); // In gap
    }
}
