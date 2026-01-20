//! # Bayer 8x8 Ordered Dithering
//!
//! This module implements ordered dithering using a Bayer matrix to convert
//! continuous-tone (grayscale) images to binary (black/white) output suitable
//! for thermal printers.
//!
//! ## What is Dithering?
//!
//! Dithering simulates grayscale on a device that can only print black or white.
//! By varying the density of black dots, we create the illusion of different
//! gray levels.
//!
//! ```text
//! Grayscale:    White    Light    Medium    Dark    Black
//!               ░░░░░░   ░░▒░░░   ░▒░▒░▒   ▒▓▒▓▒▓   ██████
//! ```
//!
//! ## Ordered Dithering
//!
//! Ordered dithering uses a threshold matrix to decide which dots to print.
//! For each pixel position (x, y), we:
//!
//! 1. Look up a threshold value from the matrix using (x mod 8, y mod 8)
//! 2. Compare the pixel's intensity to the threshold
//! 3. If intensity > threshold, print black; otherwise leave white
//!
//! ## The Bayer Matrix
//!
//! The Bayer matrix is specifically designed to produce pleasing halftone
//! patterns. Its values are arranged to minimize visible artifacts:
//!
//! ```text
//!     0   1   2   3   4   5   6   7   (x mod 8)
//!   ┌───┬───┬───┬───┬───┬───┬───┬───┐
//! 0 │ 0 │32 │ 8 │40 │ 2 │34 │10 │42 │
//!   ├───┼───┼───┼───┼───┼───┼───┼───┤
//! 1 │48 │16 │56 │24 │50 │18 │58 │26 │
//!   ├───┼───┼───┼───┼───┼───┼───┼───┤
//! 2 │12 │44 │ 4 │36 │14 │46 │ 6 │38 │
//!   ├───┼───┼───┼───┼───┼───┼───┼───┤
//! 3 │60 │28 │52 │20 │62 │30 │54 │22 │
//!   ├───┼───┼───┼───┼───┼───┼───┼───┤
//! 4 │ 3 │35 │11 │43 │ 1 │33 │ 9 │41 │
//!   ├───┼───┼───┼───┼───┼───┼───┼───┤
//! 5 │51 │19 │59 │27 │49 │17 │57 │25 │
//!   ├───┼───┼───┼───┼───┼───┼───┼───┤
//! 6 │15 │47 │ 7 │39 │13 │45 │ 5 │37 │
//!   ├───┼───┼───┼───┼───┼───┼───┼───┤
//! 7 │63 │31 │55 │23 │61 │29 │53 │21 │
//!   └───┴───┴───┴───┴───┴───┴───┴───┘
//! (y mod 8)
//! ```
//!
//! Values range from 0-63. We normalize these to [0, 1) by computing:
//! `threshold = (value + 0.5) / 64.0`
//!
//! ## Why Bayer Dithering?
//!
//! - **Deterministic**: Same input always produces same output
//! - **No error accumulation**: Unlike Floyd-Steinberg, errors don't propagate
//! - **Fast**: O(1) lookup per pixel, easily parallelizable
//! - **Good patterns**: Produces visually pleasing halftone screens
//! - **Thermal-friendly**: Works well with thermal printer characteristics
//!
//! ## Comparison with Other Methods
//!
//! | Method | Speed | Quality | Artifacts |
//! |--------|-------|---------|-----------|
//! | Bayer | Fast | Good | Regular pattern |
//! | Floyd-Steinberg | Slow | Better | Noise, worms |
//! | Random | Fast | Poor | Noisy |
//! | Threshold | Fastest | Poor | Banding |
//!
//! ## Usage Example
//!
//! ```
//! use estrella::render::dither;
//!
//! // For a pixel at (10, 20) with 50% gray (0.5 intensity)
//! let should_print = dither::should_print(10, 20, 0.5);
//!
//! // Pack a row of boolean values into bytes
//! let row: Vec<bool> = vec![true, true, false, false, true, false, true, false];
//! let packed = dither::pack_row(&row);
//! assert_eq!(packed, vec![0b11001010]); // 0xCA
//! ```

/// Bayer 8x8 dithering matrix
///
/// Values range from 0-63. The pattern creates a pleasing halftone screen
/// when used as thresholds for binary conversion.
///
/// The matrix is designed so that:
/// - Low values (0, 1, 2...) activate first at low intensities
/// - High values (61, 62, 63) activate last at high intensities
/// - The distribution minimizes visible patterns
pub const BAYER8: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// Get the dithering threshold for a pixel position.
///
/// Returns a value in the range (0, 1) that serves as the threshold for
/// determining whether to print a dot.
///
/// ## Parameters
///
/// - `x`: Horizontal pixel position
/// - `y`: Vertical pixel position
///
/// ## Returns
///
/// Threshold value in range (0.0078125, 0.9921875) - never exactly 0 or 1.
///
/// ## Algorithm
///
/// ```text
/// matrix_value = BAYER8[y mod 8][x mod 8]
/// threshold = (matrix_value + 0.5) / 64.0
/// ```
///
/// Adding 0.5 before dividing ensures:
/// - Black (intensity 1.0) always prints (threshold max = 63.5/64 < 1)
/// - White (intensity 0.0) never prints (threshold min = 0.5/64 > 0)
#[inline]
pub fn threshold(x: usize, y: usize) -> f32 {
    let matrix_value = BAYER8[y & 7][x & 7];
    (matrix_value as f32 + 0.5) / 64.0
}

/// Determine if a dot should be printed at the given position.
///
/// ## Parameters
///
/// - `x`: Horizontal pixel position
/// - `y`: Vertical pixel position
/// - `intensity`: Grayscale value where 0.0 = white, 1.0 = black
///
/// ## Returns
///
/// `true` if a black dot should be printed, `false` for white/no dot.
///
/// ## Example
///
/// ```
/// use estrella::render::dither::should_print;
///
/// // Full black always prints
/// assert!(should_print(0, 0, 1.0));
///
/// // Full white never prints
/// assert!(!should_print(0, 0, 0.0));
///
/// // 50% gray prints roughly half the dots
/// let mut count = 0;
/// for y in 0..8 {
///     for x in 0..8 {
///         if should_print(x, y, 0.5) {
///             count += 1;
///         }
///     }
/// }
/// assert!(count > 20 && count < 44); // Approximately 32 dots
/// ```
#[inline]
pub fn should_print(x: usize, y: usize, intensity: f32) -> bool {
    intensity > threshold(x, y)
}

/// Pack a row of boolean pixel values into bytes.
///
/// Converts a slice of bool values (true = black, false = white) into
/// a byte array suitable for printer graphics commands.
///
/// ## Bit Packing
///
/// - Bit 7 (MSB) = leftmost pixel
/// - Bit 0 (LSB) = rightmost pixel
/// - 1 = black (print dot), 0 = white (no dot)
///
/// ## Padding
///
/// If the row length is not a multiple of 8, the last byte is padded
/// with zeros (white) on the right.
///
/// ## Example
///
/// ```
/// use estrella::render::dither::pack_row;
///
/// // 8 pixels pack into 1 byte
/// let row = vec![true, true, true, true, false, false, false, false];
/// assert_eq!(pack_row(&row), vec![0xF0]); // 11110000
///
/// // 12 pixels pack into 2 bytes (4 bits padding)
/// let row = vec![true; 12];
/// assert_eq!(pack_row(&row), vec![0xFF, 0xF0]); // 11111111 11110000
/// ```
pub fn pack_row(pixels: &[bool]) -> Vec<u8> {
    let num_bytes = pixels.len().div_ceil(8);
    let mut bytes = vec![0u8; num_bytes];

    for (i, &pixel) in pixels.iter().enumerate() {
        if pixel {
            let byte_idx = i / 8;
            let bit_idx = 7 - (i % 8); // MSB first
            bytes[byte_idx] |= 1 << bit_idx;
        }
    }

    bytes
}

/// Generate a dithered raster image from an intensity function.
///
/// ## Parameters
///
/// - `width`: Image width in pixels
/// - `height`: Image height in pixels
/// - `intensity_fn`: Function that returns intensity (0.0-1.0) for each (x, y)
///
/// ## Returns
///
/// Packed byte array suitable for raster graphics commands.
/// Length = `ceil(width/8) * height` bytes.
///
/// ## Example
///
/// ```
/// use estrella::render::dither::generate_raster;
///
/// // Generate a gradient from white to black
/// let data = generate_raster(64, 100, |x, _y, w, _h| {
///     x as f32 / w as f32
/// });
///
/// assert_eq!(data.len(), 8 * 100); // 64 pixels / 8 = 8 bytes per row
/// ```
pub fn generate_raster<F>(width: usize, height: usize, intensity_fn: F) -> Vec<u8>
where
    F: Fn(usize, usize, usize, usize) -> f32,
{
    let width_bytes = width.div_ceil(8);
    let mut data = Vec::with_capacity(width_bytes * height);

    for y in 0..height {
        let mut row_pixels = Vec::with_capacity(width);
        for x in 0..width {
            let intensity = intensity_fn(x, y, width, height);
            row_pixels.push(should_print(x, y, intensity));
        }
        data.extend(pack_row(&row_pixels));
    }

    data
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bayer_matrix_values() {
        // Check matrix contains all values 0-63 exactly once
        let mut seen = [false; 64];
        for row in &BAYER8 {
            for &val in row {
                assert!(val < 64, "Matrix value {} out of range", val);
                assert!(!seen[val as usize], "Duplicate value {}", val);
                seen[val as usize] = true;
            }
        }
        assert!(seen.iter().all(|&s| s), "Not all values 0-63 present");
    }

    #[test]
    fn test_threshold_range() {
        for y in 0..8 {
            for x in 0..8 {
                let t = threshold(x, y);
                assert!(t > 0.0, "Threshold at ({},{}) should be > 0", x, y);
                assert!(t < 1.0, "Threshold at ({},{}) should be < 1", x, y);
            }
        }
    }

    #[test]
    fn test_threshold_periodicity() {
        // Matrix should repeat every 8 pixels
        for y in 0..8 {
            for x in 0..8 {
                let t1 = threshold(x, y);
                let t2 = threshold(x + 8, y);
                let t3 = threshold(x, y + 8);
                let t4 = threshold(x + 8, y + 8);
                assert_eq!(t1, t2);
                assert_eq!(t1, t3);
                assert_eq!(t1, t4);
            }
        }
    }

    #[test]
    fn test_black_always_prints() {
        for y in 0..100 {
            for x in 0..100 {
                assert!(
                    should_print(x, y, 1.0),
                    "Black (1.0) should always print at ({},{})",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn test_white_never_prints() {
        for y in 0..100 {
            for x in 0..100 {
                assert!(
                    !should_print(x, y, 0.0),
                    "White (0.0) should never print at ({},{})",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn test_gray_distribution() {
        // 50% gray should print roughly half the dots in an 8x8 block
        let mut count = 0;
        for y in 0..8 {
            for x in 0..8 {
                if should_print(x, y, 0.5) {
                    count += 1;
                }
            }
        }
        // Exact count is 32 for 0.5 intensity with properly calibrated threshold
        assert!(
            count >= 28 && count <= 36,
            "50% gray should print ~32 dots, got {}",
            count
        );
    }

    #[test]
    fn test_pack_row_8_pixels() {
        // All black
        assert_eq!(pack_row(&[true; 8]), vec![0xFF]);
        // All white
        assert_eq!(pack_row(&[false; 8]), vec![0x00]);
        // Alternating
        assert_eq!(
            pack_row(&[true, false, true, false, true, false, true, false]),
            vec![0xAA]
        );
        // High nibble
        assert_eq!(
            pack_row(&[true, true, true, true, false, false, false, false]),
            vec![0xF0]
        );
    }

    #[test]
    fn test_pack_row_padding() {
        // 4 pixels should pad to 1 byte
        assert_eq!(pack_row(&[true, true, true, true]), vec![0xF0]);

        // 9 pixels should pad to 2 bytes
        let nine_black = vec![true; 9];
        let packed = pack_row(&nine_black);
        assert_eq!(packed.len(), 2);
        assert_eq!(packed[0], 0xFF);
        assert_eq!(packed[1], 0x80); // 10000000
    }

    #[test]
    fn test_pack_row_empty() {
        assert_eq!(pack_row(&[]), vec![]);
    }

    #[test]
    fn test_generate_raster_dimensions() {
        let data = generate_raster(576, 100, |_, _, _, _| 0.5);
        assert_eq!(data.len(), 72 * 100); // 576/8 = 72 bytes per row
    }

    #[test]
    fn test_generate_raster_all_black() {
        let data = generate_raster(16, 2, |_, _, _, _| 1.0);
        assert_eq!(data.len(), 4); // 16/8 = 2 bytes per row, 2 rows
        assert!(data.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_generate_raster_all_white() {
        let data = generate_raster(16, 2, |_, _, _, _| 0.0);
        assert_eq!(data.len(), 4);
        assert!(data.iter().all(|&b| b == 0x00));
    }
}
