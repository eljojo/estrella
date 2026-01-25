//! Intensity and color adjustment functions.

/// Clamp a value to [0, 1] range.
#[inline]
pub fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

/// Clamp a value to an arbitrary range.
#[inline]
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.clamp(min, max)
}

/// Apply gamma correction.
///
/// - gamma < 1: Darkens midtones
/// - gamma > 1: Lightens midtones
/// - gamma = 1: No change
#[inline]
pub fn gamma(value: f32, gamma: f32) -> f32 {
    clamp01(value).powf(gamma)
}

/// Invert a value (1 - x).
#[inline]
pub fn invert(value: f32) -> f32 {
    1.0 - value
}

/// Adjust contrast around a center point.
///
/// # Parameters
/// - `value`: Input value [0, 1]
/// - `center`: The midpoint that stays fixed (typically 0.5)
/// - `amount`: Contrast multiplier (>1 increases, <1 decreases)
#[inline]
pub fn contrast(value: f32, center: f32, amount: f32) -> f32 {
    clamp01(center + (value - center) * amount)
}

/// Adjust brightness by adding an offset.
#[inline]
pub fn brightness(value: f32, offset: f32) -> f32 {
    clamp01(value + offset)
}

/// Remap a value from one range to another.
#[inline]
pub fn remap(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    let normalized = (value - from_min) / (from_max - from_min);
    to_min + normalized * (to_max - to_min)
}

/// Remap a value from one range to another, clamped.
#[inline]
pub fn remap_clamped(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    let normalized = ((value - from_min) / (from_max - from_min)).clamp(0.0, 1.0);
    to_min + normalized * (to_max - to_min)
}

/// Levels adjustment (like Photoshop levels).
///
/// # Parameters
/// - `value`: Input value
/// - `in_black`: Input black point (values below become 0)
/// - `in_white`: Input white point (values above become 1)
/// - `out_black`: Output black level
/// - `out_white`: Output white level
/// - `gamma`: Midtone gamma adjustment
pub fn levels(
    value: f32,
    in_black: f32,
    in_white: f32,
    out_black: f32,
    out_white: f32,
    mid_gamma: f32,
) -> f32 {
    // Map input range to [0, 1]
    let normalized = ((value - in_black) / (in_white - in_black)).clamp(0.0, 1.0);
    // Apply gamma
    let gamma_corrected = normalized.powf(mid_gamma);
    // Map to output range
    out_black + gamma_corrected * (out_white - out_black)
}

/// Simple threshold (binary).
#[inline]
pub fn threshold_binary(value: f32, thresh: f32) -> f32 {
    if value >= thresh { 1.0 } else { 0.0 }
}

/// Soft threshold using sigmoid.
#[inline]
pub fn threshold_soft(value: f32, thresh: f32, softness: f32) -> f32 {
    1.0 / (1.0 + (-(value - thresh) / softness).exp())
}

/// Expose (simulates camera exposure adjustment).
///
/// `stops` is in photographic stops (+1 = double brightness).
#[inline]
pub fn expose(value: f32, stops: f32) -> f32 {
    clamp01(value * 2.0_f32.powf(stops))
}

/// S-curve contrast enhancement.
///
/// Creates an S-shaped curve that increases contrast in midtones.
#[inline]
pub fn s_curve(value: f32, amount: f32) -> f32 {
    let x = value * 2.0 - 1.0; // Map to [-1, 1]
    let curved = x * (1.0 + amount * (1.0 - x.abs()));
    clamp01((curved + 1.0) * 0.5) // Map back to [0, 1]
}

/// Apply a lookup table (for arbitrary transfer functions).
///
/// The LUT should have 256 entries for full precision.
pub fn apply_lut(value: f32, lut: &[f32]) -> f32 {
    if lut.is_empty() {
        return value;
    }
    let index = (value.clamp(0.0, 1.0) * (lut.len() - 1) as f32).round() as usize;
    lut[index.min(lut.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp01() {
        assert_eq!(clamp01(-0.5), 0.0);
        assert_eq!(clamp01(0.5), 0.5);
        assert_eq!(clamp01(1.5), 1.0);
    }

    #[test]
    fn test_gamma() {
        // Gamma 1 = no change
        assert!((gamma(0.5, 1.0) - 0.5).abs() < 1e-6);
        // Gamma > 1 lightens midtones
        assert!(gamma(0.5, 2.0) < 0.5);
        // Gamma < 1 darkens midtones
        assert!(gamma(0.5, 0.5) > 0.5);
    }

    #[test]
    fn test_invert() {
        assert!((invert(0.0) - 1.0).abs() < 1e-6);
        assert!((invert(1.0) - 0.0).abs() < 1e-6);
        assert!((invert(0.3) - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_contrast() {
        // At center, no change
        assert!((contrast(0.5, 0.5, 2.0) - 0.5).abs() < 1e-6);
        // High contrast pushes values apart
        assert!(contrast(0.7, 0.5, 2.0) > 0.7);
        assert!(contrast(0.3, 0.5, 2.0) < 0.3);
    }

    #[test]
    fn test_remap() {
        assert!((remap(0.5, 0.0, 1.0, 0.0, 100.0) - 50.0).abs() < 1e-6);
        assert!((remap(0.0, 0.0, 1.0, 10.0, 20.0) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_expose() {
        // +1 stop = double
        assert!((expose(0.25, 1.0) - 0.5).abs() < 1e-6);
        // -1 stop = half
        assert!((expose(0.5, -1.0) - 0.25).abs() < 1e-6);
    }
}
