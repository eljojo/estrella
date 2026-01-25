//! Grid and cell-based pattern functions.

use std::f32::consts::PI;

/// Get the grid cell indices for a coordinate.
///
/// Returns (cell_x, cell_y) as integers.
#[inline]
pub fn grid_cell(x: f32, y: f32, cell_size: f32) -> (i32, i32) {
    ((x / cell_size).floor() as i32, (y / cell_size).floor() as i32)
}

/// Get the position within a cell as [0, 1].
///
/// Returns (u, v) where both are in [0, 1) range.
#[inline]
pub fn cell_pos(x: f32, y: f32, cell_size: f32) -> (f32, f32) {
    let u = (x / cell_size).rem_euclid(1.0);
    let v = (y / cell_size).rem_euclid(1.0);
    (u, v)
}

/// Get the position within a cell centered at [-.5, .5].
#[inline]
pub fn cell_pos_centered(x: f32, y: f32, cell_size: f32) -> (f32, f32) {
    let (u, v) = cell_pos(x, y, cell_size);
    (u - 0.5, v - 0.5)
}

/// Checkerboard pattern.
///
/// Returns true for "black" squares, false for "white".
#[inline]
pub fn checkerboard(cell_x: i32, cell_y: i32) -> bool {
    (cell_x + cell_y) % 2 != 0
}

/// Checkerboard pattern from coordinates.
#[inline]
pub fn checkerboard_xy(x: f32, y: f32, cell_size: f32) -> bool {
    let (cx, cy) = grid_cell(x, y, cell_size);
    checkerboard(cx, cy)
}

/// Hexagonal grid cell in axial coordinates.
///
/// Returns (q, r) axial coordinates for the hexagon containing (x, y).
/// Uses pointy-top hexagons.
pub fn hex_cell(x: f32, y: f32, hex_size: f32) -> (i32, i32) {
    // Convert to axial coordinates
    let q = (x * 3.0_f32.sqrt() / 3.0 - y / 3.0) / hex_size;
    let r = y * 2.0 / 3.0 / hex_size;

    // Convert to cube coordinates for rounding
    let cube_x = q;
    let cube_z = r;
    let cube_y = -cube_x - cube_z;

    // Round cube coordinates
    let mut rx = cube_x.round();
    let ry = cube_y.round();
    let mut rz = cube_z.round();

    // Fix rounding errors to maintain x + y + z = 0
    let x_diff = (rx - cube_x).abs();
    let y_diff = (ry - cube_y).abs();
    let z_diff = (rz - cube_z).abs();

    if x_diff > y_diff && x_diff > z_diff {
        rx = -ry - rz;
    } else if y_diff <= z_diff {
        rz = -rx - ry;
    }

    (rx as i32, rz as i32)
}

/// Get the center of a hexagonal cell.
pub fn hex_center(q: i32, r: i32, hex_size: f32) -> (f32, f32) {
    let x = hex_size * 3.0_f32.sqrt() * (q as f32 + r as f32 / 2.0);
    let y = hex_size * 1.5 * r as f32;
    (x, y)
}

/// Determine which face of an isometric cube a point is on.
///
/// Returns 0 for top face, 1 for left face, 2 for right face.
/// Used for Vasarely-style hex cube patterns.
pub fn hex_cube_face(x: f32, y: f32, hex_size: f32) -> usize {
    let (q, r) = hex_cell(x, y, hex_size);
    let (hx, hy) = hex_center(q, r, hex_size);

    // Local coordinates within hexagon
    let lx = x - hx;
    let ly = y - hy;

    // Determine face by angle
    let angle = ly.atan2(lx);
    let normalized = if angle < 0.0 { angle + 2.0 * PI } else { angle };

    // Divide into thirds
    if normalized < 2.0 * PI / 3.0 {
        2 // Right face
    } else if normalized < 4.0 * PI / 3.0 {
        0 // Top face
    } else {
        1 // Left face
    }
}

/// Brick pattern offset.
///
/// Every other row is offset by half a cell, creating a brick layout.
#[inline]
pub fn brick_offset(y: f32, cell_height: f32, offset: f32) -> f32 {
    let row = (y / cell_height).floor() as i32;
    if row % 2 != 0 { offset } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_cell() {
        assert_eq!(grid_cell(5.0, 5.0, 10.0), (0, 0));
        assert_eq!(grid_cell(15.0, 25.0, 10.0), (1, 2));
        assert_eq!(grid_cell(-5.0, -5.0, 10.0), (-1, -1));
    }

    #[test]
    fn test_cell_pos() {
        let (u, v) = cell_pos(15.0, 25.0, 10.0);
        assert!((u - 0.5).abs() < 1e-6);
        assert!((v - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_checkerboard() {
        assert!(!checkerboard(0, 0));
        assert!(checkerboard(0, 1));
        assert!(checkerboard(1, 0));
        assert!(!checkerboard(1, 1));
    }

    #[test]
    fn test_hex_cell_center() {
        // Cell at origin should contain origin
        let (q, r) = hex_cell(0.0, 0.0, 10.0);
        assert_eq!((q, r), (0, 0));
    }
}
