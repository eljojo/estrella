//! Coordinate transformation functions.

use std::f32::consts::PI;

/// Rotate a point around the origin.
///
/// # Parameters
/// - `x`, `y`: Point coordinates
/// - `angle`: Rotation angle in radians (counter-clockwise)
///
/// # Returns
/// Rotated (x, y) coordinates
#[inline]
pub fn rotate(x: f32, y: f32, angle: f32) -> (f32, f32) {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    (x * cos_a - y * sin_a, x * sin_a + y * cos_a)
}

/// Rotate a point around the origin (angle in degrees).
#[inline]
pub fn rotate_deg(x: f32, y: f32, angle_deg: f32) -> (f32, f32) {
    rotate(x, y, angle_deg * PI / 180.0)
}

/// Convert pixel coordinates to center-relative coordinates.
///
/// Maps (0, 0) to (-width/2, -height/2) and (width, height) to (width/2, height/2).
#[inline]
pub fn center_coords(x: f32, y: f32, width: f32, height: f32) -> (f32, f32) {
    (x - width / 2.0, y - height / 2.0)
}

/// Normalize a coordinate to [-1, 1] range.
#[inline]
pub fn normalize(coord: f32, size: f32) -> f32 {
    (coord / size) * 2.0 - 1.0
}

/// Normalize a coordinate to [0, 1] range.
#[inline]
pub fn normalize01(coord: f32, size: f32) -> f32 {
    coord / size
}

/// Convert Cartesian coordinates to polar.
///
/// # Returns
/// (radius, angle) where angle is in radians [-PI, PI]
#[inline]
pub fn cart_to_polar(x: f32, y: f32) -> (f32, f32) {
    let r = (x * x + y * y).sqrt();
    let theta = y.atan2(x);
    (r, theta)
}

/// Convert polar coordinates to Cartesian.
#[inline]
pub fn polar_to_cart(r: f32, theta: f32) -> (f32, f32) {
    (r * theta.cos(), r * theta.sin())
}

/// Apply domain warping to coordinates.
///
/// Offsets the input coordinates by warp values scaled by amount.
/// Used to create organic distortions.
#[inline]
pub fn warp(x: f32, y: f32, warp_x: f32, warp_y: f32, amount: f32) -> (f32, f32) {
    (x + warp_x * amount, y + warp_y * amount)
}

/// Scale coordinates around a center point.
#[inline]
pub fn scale(x: f32, y: f32, cx: f32, cy: f32, scale_x: f32, scale_y: f32) -> (f32, f32) {
    ((x - cx) * scale_x + cx, (y - cy) * scale_y + cy)
}

/// Mirror coordinate across an axis.
///
/// `axis_pos` is the position of the mirror axis.
#[inline]
pub fn mirror(coord: f32, axis_pos: f32) -> f32 {
    let offset = coord - axis_pos;
    axis_pos - offset
}

/// Fold coordinate to create symmetry.
///
/// Values beyond `fold_pos` are reflected back.
#[inline]
pub fn fold(coord: f32, fold_pos: f32) -> f32 {
    if coord > fold_pos {
        2.0 * fold_pos - coord
    } else {
        coord
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotate_90() {
        let (x, y) = rotate(1.0, 0.0, PI / 2.0);
        assert!(x.abs() < 1e-6);
        assert!((y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_rotate_180() {
        let (x, y) = rotate(1.0, 0.0, PI);
        assert!((x + 1.0).abs() < 1e-6);
        assert!(y.abs() < 1e-6);
    }

    #[test]
    fn test_center_coords() {
        let (cx, cy) = center_coords(50.0, 50.0, 100.0, 100.0);
        assert!((cx).abs() < 1e-6);
        assert!((cy).abs() < 1e-6);
    }

    #[test]
    fn test_polar_roundtrip() {
        let (x, y) = (3.0, 4.0);
        let (r, theta) = cart_to_polar(x, y);
        let (x2, y2) = polar_to_cart(r, theta);
        assert!((x - x2).abs() < 1e-6);
        assert!((y - y2).abs() < 1e-6);
    }

    #[test]
    fn test_normalize() {
        assert!((normalize(50.0, 100.0)).abs() < 1e-6); // center -> 0
        assert!((normalize(0.0, 100.0) + 1.0).abs() < 1e-6); // left -> -1
        assert!((normalize(100.0, 100.0) - 1.0).abs() < 1e-6); // right -> 1
    }
}
