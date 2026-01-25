//! Quantization and banding functions.

/// Scanline pattern.
///
/// Returns true for pixels on a scanline, false for gaps.
///
/// # Parameters
/// - `y`: Vertical position
/// - `period`: Distance between scanlines
/// - `thickness`: Thickness of each scanline
#[inline]
pub fn scanline(y: usize, period: usize, thickness: usize) -> bool {
    if period == 0 {
        return false;
    }
    (y % period) < thickness
}

/// Scanline with float coordinates.
#[inline]
pub fn scanline_f(y: f32, period: f32, thickness: f32) -> bool {
    if period <= 0.0 {
        return false;
    }
    y.rem_euclid(period) < thickness
}

/// Band index for a given position.
///
/// Divides the space into bands and returns which band a position falls into.
#[inline]
pub fn band_index(pos: usize, band_size: usize) -> usize {
    if band_size == 0 {
        return 0;
    }
    pos / band_size
}

/// Band index with float coordinates.
#[inline]
pub fn band_index_f(pos: f32, band_size: f32) -> usize {
    if band_size <= 0.0 {
        return 0;
    }
    (pos / band_size).floor() as usize
}

/// Contour lines from a continuous value.
///
/// Creates bands/contours at regular intervals in the value space.
///
/// # Parameters
/// - `value`: Input value (typically 0.0 to 1.0)
/// - `num_contours`: Number of contour levels
///
/// # Returns
/// Distance to nearest contour line (0 = on line, 1 = farthest from line)
pub fn contour(value: f32, num_contours: f32) -> f32 {
    let scaled = value * num_contours;
    let frac = scaled - scaled.floor();
    // frac is in [0, 1), contour lines are at 0 and 1
    // Return normalized distance to nearest line
    let dist_to_low = frac;
    let dist_to_high = 1.0 - frac;
    dist_to_low.min(dist_to_high) * 2.0 // Scale to [0, 1]
}

/// Contour as binary (on a line or not).
#[inline]
pub fn contour_binary(value: f32, num_contours: f32, line_width: f32) -> bool {
    contour(value, num_contours) < line_width
}

/// Quantize a value to discrete levels.
///
/// Reduces the value to one of `levels` discrete values.
#[inline]
pub fn quantize(value: f32, levels: usize) -> f32 {
    if levels <= 1 {
        return 0.5;
    }
    let scaled = (value * levels as f32).floor();
    scaled / (levels - 1) as f32
}

/// Posterize - reduce to specific number of tones.
///
/// Like quantize but rounds to nearest level rather than floor.
#[inline]
pub fn posterize(value: f32, levels: usize) -> f32 {
    if levels <= 1 {
        return 0.5;
    }
    let scaled = (value * (levels - 1) as f32).round();
    scaled / (levels - 1) as f32
}

/// Threshold to binary.
#[inline]
pub fn threshold(value: f32, thresh: f32) -> f32 {
    if value >= thresh { 1.0 } else { 0.0 }
}

/// Multi-level threshold.
///
/// Returns which threshold level the value exceeds.
pub fn threshold_levels(value: f32, thresholds: &[f32]) -> usize {
    for (i, &t) in thresholds.iter().enumerate() {
        if value < t {
            return i;
        }
    }
    thresholds.len()
}

/// Bit crush / reduce bit depth.
///
/// Simulates lower bit depth by quantizing and rescaling.
#[inline]
pub fn bit_crush(value: f32, bits: u32) -> f32 {
    if bits >= 32 {
        return value;
    }
    let levels = 1u32 << bits;
    let scaled = (value * levels as f32).floor();
    scaled / (levels - 1) as f32
}

/// Step function with multiple steps.
///
/// Creates a staircase pattern.
#[inline]
pub fn stairs(value: f32, num_steps: usize) -> f32 {
    if num_steps == 0 {
        return 0.0;
    }
    (value * num_steps as f32).floor() / num_steps as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanline() {
        assert!(scanline(0, 4, 2));
        assert!(scanline(1, 4, 2));
        assert!(!scanline(2, 4, 2));
        assert!(!scanline(3, 4, 2));
        assert!(scanline(4, 4, 2)); // Repeats
    }

    #[test]
    fn test_band_index() {
        assert_eq!(band_index(0, 10), 0);
        assert_eq!(band_index(9, 10), 0);
        assert_eq!(band_index(10, 10), 1);
        assert_eq!(band_index(25, 10), 2);
    }

    #[test]
    fn test_contour() {
        // At contour line (value = 0.5 with 2 contours -> at line)
        let c = contour(0.5, 2.0);
        assert!(c < 0.1, "should be near contour line: {}", c);

        // Between contours
        let c = contour(0.25, 2.0);
        assert!(c > 0.8, "should be far from contour: {}", c);
    }

    #[test]
    fn test_quantize() {
        assert!((quantize(0.0, 4) - 0.0).abs() < 1e-6);
        assert!((quantize(0.3, 4) - 0.333).abs() < 0.01);
        assert!((quantize(0.9, 4) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_threshold() {
        assert_eq!(threshold(0.3, 0.5), 0.0);
        assert_eq!(threshold(0.7, 0.5), 1.0);
        assert_eq!(threshold(0.5, 0.5), 1.0);
    }

    #[test]
    fn test_bit_crush() {
        // 1 bit = 2 levels
        assert!(bit_crush(0.3, 1) == 0.0 || bit_crush(0.3, 1) == 1.0);
        assert!(bit_crush(0.7, 1) == 0.0 || bit_crush(0.7, 1) == 1.0);
    }
}
