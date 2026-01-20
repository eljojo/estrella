//! # Golden Tests for Patterns
//!
//! These tests ensure that pattern rendering produces consistent output.
//! Golden files are stored in `tests/golden/` and compared against generated output.
//!
//! To regenerate golden files (if intentional changes are made):
//! ```bash
//! cargo run -- print --png tests/golden/ripple_576x500.png --height 500 --width 576 ripple
//! cargo run -- print --png tests/golden/waves_576x500.png --height 500 --width 576 waves
//! cargo run -- print --png tests/golden/sick_576x1920.png --height 1920 --width 576 sick
//! cargo run -- print --png tests/golden/calibration_576x500.png --height 500 --width 576 calibration
//! ```

use estrella::render::{dither, patterns};

const TEST_WIDTH: usize = 576;
const TEST_HEIGHT: usize = 500;

// Sick pattern has 4 sections of 480 rows each
const SICK_HEIGHT: usize = 480 * 4; // 1920 rows to see all sections

/// Generate raster data for a pattern (same logic as main.rs)
fn generate_pattern_raster(pattern: &dyn patterns::Pattern) -> Vec<u8> {
    generate_pattern_raster_sized(pattern, TEST_WIDTH, TEST_HEIGHT)
}

/// Generate raster data for a pattern with custom dimensions
fn generate_pattern_raster_sized(
    pattern: &dyn patterns::Pattern,
    width: usize,
    height: usize,
) -> Vec<u8> {
    let gamma = pattern.gamma();

    dither::generate_raster(width, height, |x, y, w, h| {
        let shade = pattern.shade(x, y, w, h);
        shade.powf(gamma).clamp(0.0, 1.0)
    })
}

/// Convert raster data to PNG bytes (for comparison)
fn raster_to_png(width: usize, height: usize, data: &[u8]) -> Vec<u8> {
    use image::{GrayImage, Luma, ImageEncoder};

    let mut img = GrayImage::new(width as u32, height as u32);
    let width_bytes = width.div_ceil(8);

    for y in 0..height {
        for x in 0..width {
            let byte_idx = y * width_bytes + x / 8;
            let bit_idx = 7 - (x % 8);
            let is_black = (data[byte_idx] >> bit_idx) & 1 == 1;
            let color = if is_black { 0u8 } else { 255u8 };
            img.put_pixel(x as u32, y as u32, Luma([color]));
        }
    }

    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(
            img.as_raw(),
            width as u32,
            height as u32,
            image::ExtendedColorType::L8,
        )
        .expect("PNG encoding failed");
    png_bytes
}

#[test]
fn test_ripple_golden() {
    let pattern = patterns::Ripple::default();
    let raster = generate_pattern_raster(&pattern);

    // Verify raster dimensions
    let expected_bytes = TEST_WIDTH.div_ceil(8) * TEST_HEIGHT;
    assert_eq!(
        raster.len(),
        expected_bytes,
        "Ripple raster size mismatch"
    );

    // Verify deterministic output by checking first few bytes
    // These are known values from the golden output
    let png = raster_to_png(TEST_WIDTH, TEST_HEIGHT, &raster);
    assert!(!png.is_empty(), "PNG generation failed");

    // Compare against stored golden file
    let golden = include_bytes!("golden/ripple_576x500.png");
    assert_eq!(
        png.len(),
        golden.len(),
        "Ripple PNG size differs from golden (regenerate if intentional)"
    );
    assert_eq!(
        png, golden,
        "Ripple PNG content differs from golden (regenerate if intentional)"
    );
}

#[test]
fn test_waves_golden() {
    let pattern = patterns::Waves::default();
    let raster = generate_pattern_raster(&pattern);

    let expected_bytes = TEST_WIDTH.div_ceil(8) * TEST_HEIGHT;
    assert_eq!(raster.len(), expected_bytes, "Waves raster size mismatch");

    let png = raster_to_png(TEST_WIDTH, TEST_HEIGHT, &raster);
    let golden = include_bytes!("golden/waves_576x500.png");
    assert_eq!(
        png.len(),
        golden.len(),
        "Waves PNG size differs from golden"
    );
    assert_eq!(
        png, golden,
        "Waves PNG content differs from golden"
    );
}

#[test]
fn test_sick_golden() {
    let pattern = patterns::Sick::default();
    let raster = generate_pattern_raster_sized(&pattern, TEST_WIDTH, SICK_HEIGHT);

    let expected_bytes = TEST_WIDTH.div_ceil(8) * SICK_HEIGHT;
    assert_eq!(raster.len(), expected_bytes, "Sick raster size mismatch");

    let png = raster_to_png(TEST_WIDTH, SICK_HEIGHT, &raster);
    let golden = include_bytes!("golden/sick_576x1920.png");
    assert_eq!(
        png.len(),
        golden.len(),
        "Sick PNG size differs from golden"
    );
    assert_eq!(
        png, golden,
        "Sick PNG content differs from golden"
    );
}

#[test]
fn test_calibration_golden() {
    let pattern = patterns::Calibration::default();
    let raster = generate_pattern_raster(&pattern);

    let expected_bytes = TEST_WIDTH.div_ceil(8) * TEST_HEIGHT;
    assert_eq!(
        raster.len(),
        expected_bytes,
        "Calibration raster size mismatch"
    );

    let png = raster_to_png(TEST_WIDTH, TEST_HEIGHT, &raster);
    let golden = include_bytes!("golden/calibration_576x500.png");
    assert_eq!(
        png.len(),
        golden.len(),
        "Calibration PNG size differs from golden"
    );
    assert_eq!(
        png, golden,
        "Calibration PNG content differs from golden"
    );
}

/// Test that all patterns in list_patterns() can be retrieved by name
#[test]
fn test_all_patterns_accessible() {
    for name in patterns::list_patterns() {
        let pattern = patterns::by_name(name);
        assert!(
            pattern.is_some(),
            "Pattern '{}' listed but not accessible via by_name()",
            name
        );
    }
}

/// Test that pattern output is deterministic (same input = same output)
#[test]
fn test_pattern_determinism() {
    let pattern = patterns::Ripple::default();

    let raster1 = generate_pattern_raster(&pattern);
    let raster2 = generate_pattern_raster(&pattern);

    assert_eq!(
        raster1, raster2,
        "Pattern output should be deterministic"
    );
}
