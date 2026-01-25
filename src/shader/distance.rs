//! Distance functions for spatial calculations.

/// Euclidean distance between two points.
#[inline]
pub fn dist(x: f32, y: f32, cx: f32, cy: f32) -> f32 {
    let dx = x - cx;
    let dy = y - cy;
    (dx * dx + dy * dy).sqrt()
}

/// Squared Euclidean distance (avoids sqrt for comparisons).
#[inline]
pub fn dist_sq(x: f32, y: f32, cx: f32, cy: f32) -> f32 {
    let dx = x - cx;
    let dy = y - cy;
    dx * dx + dy * dy
}

/// Distance normalized by radius.
///
/// Returns 0.0 at center, 1.0 at radius distance.
#[inline]
pub fn dist_normalized(x: f32, y: f32, cx: f32, cy: f32, radius: f32) -> f32 {
    dist(x, y, cx, cy) / radius
}

/// Chebyshev distance (max of absolute differences).
///
/// Creates square-shaped distance fields. Used for rectangular frames.
#[inline]
pub fn dist_chebyshev(x: f32, y: f32, cx: f32, cy: f32) -> f32 {
    let dx = (x - cx).abs();
    let dy = (y - cy).abs();
    dx.max(dy)
}

/// Manhattan distance (sum of absolute differences).
///
/// Creates diamond-shaped distance fields.
#[inline]
pub fn dist_manhattan(x: f32, y: f32, cx: f32, cy: f32) -> f32 {
    let dx = (x - cx).abs();
    let dy = (y - cy).abs();
    dx + dy
}

/// Minkowski distance with configurable power.
///
/// - p=1: Manhattan distance
/// - p=2: Euclidean distance
/// - p=∞: Chebyshev distance (use dist_chebyshev instead)
#[inline]
pub fn dist_minkowski(x: f32, y: f32, cx: f32, cy: f32, p: f32) -> f32 {
    let dx = (x - cx).abs();
    let dy = (y - cy).abs();
    (dx.powf(p) + dy.powf(p)).powf(1.0 / p)
}

/// Distance to nearest grid line.
///
/// Returns the perpendicular distance from a coordinate to the nearest
/// grid line in a 1D grid with the given spacing.
/// Grid lines are at 0, spacing, 2*spacing, etc.
#[inline]
pub fn dist_to_grid(coord: f32, spacing: f32) -> f32 {
    let pos_in_cell = coord / spacing;
    let frac = pos_in_cell - pos_in_cell.floor();
    // Distance to nearest line (at 0.0 or 1.0 of cell)
    // frac is in [0, 1), nearest line is at 0 or 1
    let dist_to_low = frac;
    let dist_to_high = 1.0 - frac;
    dist_to_low.min(dist_to_high) * spacing
}

/// Distance from cell center in a 1D grid.
///
/// Returns 0 at the center of each cell, max (spacing/2) at cell boundaries.
/// Complement of dist_to_grid: `dist_from_cell_center + dist_to_grid = spacing/2`
///
/// Common usage: rendering lines centered within grid cells.
#[inline]
pub fn dist_from_cell_center(coord: f32, spacing: f32) -> f32 {
    let frac = (coord / spacing).fract();
    (frac - 0.5).abs() * spacing
}

/// Distance from a point to a line segment.
///
/// Returns the shortest distance from point (px, py) to the line segment
/// defined by endpoints (x1, y1) and (x2, y2).
pub fn dist_to_segment(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-10 {
        // Degenerate segment (point)
        return dist(px, py, x1, y1);
    }

    // Project point onto line, clamping to segment
    let t = ((px - x1) * dx + (py - y1) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    let closest_x = x1 + t * dx;
    let closest_y = y1 + t * dy;

    dist(px, py, closest_x, closest_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dist() {
        assert!((dist(0.0, 0.0, 3.0, 4.0) - 5.0).abs() < 1e-6);
        assert!((dist(1.0, 1.0, 1.0, 1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_dist_chebyshev() {
        assert!((dist_chebyshev(0.0, 0.0, 3.0, 4.0) - 4.0).abs() < 1e-6);
        assert!((dist_chebyshev(0.0, 0.0, 5.0, 2.0) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_dist_manhattan() {
        assert!((dist_manhattan(0.0, 0.0, 3.0, 4.0) - 7.0).abs() < 1e-6);
    }

    #[test]
    fn test_dist_to_grid() {
        // At grid line
        assert!(dist_to_grid(0.0, 10.0) < 1e-6);
        assert!(dist_to_grid(10.0, 10.0) < 1e-6);
        // At center of cell
        assert!((dist_to_grid(5.0, 10.0) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_dist_to_segment() {
        // Point on segment
        assert!(dist_to_segment(0.5, 0.0, 0.0, 0.0, 1.0, 0.0) < 1e-6);
        // Point perpendicular to segment
        assert!((dist_to_segment(0.5, 1.0, 0.0, 0.0, 1.0, 0.0) - 1.0).abs() < 1e-6);
        // Point beyond segment end
        assert!((dist_to_segment(2.0, 0.0, 0.0, 0.0, 1.0, 0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_dist_from_cell_center() {
        // At cell center
        assert!(dist_from_cell_center(5.0, 10.0) < 1e-6);
        assert!(dist_from_cell_center(15.0, 10.0) < 1e-6);
        // At grid line (edge of cell)
        assert!((dist_from_cell_center(0.0, 10.0) - 5.0).abs() < 1e-6);
        assert!((dist_from_cell_center(10.0, 10.0) - 5.0).abs() < 1e-6);
        // Complement relationship
        assert!((dist_from_cell_center(3.0, 10.0) + dist_to_grid(3.0, 10.0) - 5.0).abs() < 1e-6);
    }
}
