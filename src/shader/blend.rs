//! Blending and interpolation functions.

/// Linear interpolation between two values.
///
/// Returns `a` when `t=0`, `b` when `t=1`, and linear blend in between.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Smoothstep interpolation.
///
/// Hermite interpolation that is smooth at the endpoints.
/// Returns 0 when x <= edge0, 1 when x >= edge1, and smooth curve in between.
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smoother step (Ken Perlin's version).
///
/// Has zero first AND second derivatives at endpoints.
#[inline]
pub fn smootherstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Sigmoid function for soft thresholding.
///
/// Creates an S-curve centered at `threshold` with given `sharpness`.
/// Higher sharpness = steeper transition.
#[inline]
pub fn sigmoid(x: f32, threshold: f32, sharpness: f32) -> f32 {
    1.0 / (1.0 + (-(x - threshold) * sharpness).exp())
}

/// Weighted blend of multiple values.
///
/// Returns the weighted average of values.
pub fn blend_weighted(values: &[f32], weights: &[f32]) -> f32 {
    let mut sum = 0.0;
    let mut weight_sum = 0.0;

    for (v, w) in values.iter().zip(weights.iter()) {
        sum += v * w;
        weight_sum += w;
    }

    if weight_sum > 0.0 {
        sum / weight_sum
    } else {
        0.0
    }
}

/// Maximum blend (union).
///
/// Returns the maximum of all values.
#[inline]
pub fn blend_max(values: &[f32]) -> f32 {
    values.iter().cloned().fold(f32::MIN, f32::max)
}

/// Minimum blend (intersection).
///
/// Returns the minimum of all values.
#[inline]
pub fn blend_min(values: &[f32]) -> f32 {
    values.iter().cloned().fold(f32::MAX, f32::min)
}

/// Soft maximum (smooth union).
///
/// Like max but with smooth blending at transitions.
#[inline]
pub fn soft_max(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    lerp(a, b, h) + k * h * (1.0 - h)
}

/// Soft minimum (smooth intersection).
#[inline]
pub fn soft_min(a: f32, b: f32, k: f32) -> f32 {
    -soft_max(-a, -b, k)
}

/// Screen blend mode (like Photoshop screen).
///
/// Lightens by inverting, multiplying, and inverting again.
#[inline]
pub fn blend_screen(a: f32, b: f32) -> f32 {
    1.0 - (1.0 - a) * (1.0 - b)
}

/// Multiply blend mode.
#[inline]
pub fn blend_multiply(a: f32, b: f32) -> f32 {
    a * b
}

/// Overlay blend mode.
///
/// Combines multiply and screen based on base value.
#[inline]
pub fn blend_overlay(base: f32, blend: f32) -> f32 {
    if base < 0.5 {
        2.0 * base * blend
    } else {
        1.0 - 2.0 * (1.0 - base) * (1.0 - blend)
    }
}

/// Additive blend (clamped).
#[inline]
pub fn blend_add(a: f32, b: f32) -> f32 {
    (a + b).min(1.0)
}

/// Difference blend.
#[inline]
pub fn blend_difference(a: f32, b: f32) -> f32 {
    (a - b).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 10.0, 0.0) - 0.0).abs() < 1e-6);
        assert!((lerp(0.0, 10.0, 1.0) - 10.0).abs() < 1e-6);
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_smoothstep() {
        assert!((smoothstep(0.0, 1.0, -0.5) - 0.0).abs() < 1e-6);
        assert!((smoothstep(0.0, 1.0, 1.5) - 1.0).abs() < 1e-6);
        assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_sigmoid() {
        // At threshold, should be 0.5
        assert!((sigmoid(0.5, 0.5, 10.0) - 0.5).abs() < 1e-6);
        // Far below threshold, should be near 0
        assert!(sigmoid(0.0, 0.5, 10.0) < 0.01);
        // Far above threshold, should be near 1
        assert!(sigmoid(1.0, 0.5, 10.0) > 0.99);
    }

    #[test]
    fn test_blend_weighted() {
        let values = [0.0, 1.0];
        let weights = [1.0, 1.0];
        assert!((blend_weighted(&values, &weights) - 0.5).abs() < 1e-6);

        let weights = [3.0, 1.0];
        assert!((blend_weighted(&values, &weights) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn test_blend_screen() {
        assert!((blend_screen(0.0, 0.0) - 0.0).abs() < 1e-6);
        assert!((blend_screen(1.0, 1.0) - 1.0).abs() < 1e-6);
        // Screen always lightens
        assert!(blend_screen(0.5, 0.5) > 0.5);
    }
}
