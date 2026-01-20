//! # Golden Tests for Patterns
//!
//! These tests ensure that pattern rendering produces consistent output.
//!
//! ## PNG Golden Tests
//! Golden PNG files are stored in `tests/golden/` and compared against generated output.
//! Patterns use their `default_dimensions()` for canonical sizing.
//!
//! ## Binary Command Golden Tests
//! Golden binary files (`.bin`) store the actual printer command bytes.
//! These are compared byte-for-byte against generated output.
//!
//! ## Regenerating Golden Files
//!
//! To regenerate all golden files (PNG + binary):
//! ```bash
//! make golden
//! ```
//!
//! This runs both the CLI for PNGs and the `write_golden_binaries` test.

use estrella::components::{ComponentExt, Pattern as PatternComponent, Receipt};
use estrella::receipt;
use estrella::render::patterns::Pattern;
use estrella::render::{dither, patterns};
use std::fs;

/// Generate raster data for a pattern using its default dimensions
fn generate_pattern_raster(pattern: &dyn Pattern) -> Vec<u8> {
    let (width, height) = pattern.default_dimensions();
    generate_pattern_raster_sized(pattern, width, height)
}

/// Generate raster data for a pattern with custom dimensions
fn generate_pattern_raster_sized(pattern: &dyn Pattern, width: usize, height: usize) -> Vec<u8> {
    let gamma = pattern.gamma();

    dither::generate_raster(width, height, |x, y, w, h| {
        let shade = pattern.shade(x, y, w, h);
        shade.powf(gamma).clamp(0.0, 1.0)
    })
}

/// Convert raster data to PNG bytes (for comparison)
fn raster_to_png(width: usize, height: usize, data: &[u8]) -> Vec<u8> {
    use image::{GrayImage, ImageEncoder, Luma};

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
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);

    // Verify raster dimensions
    let expected_bytes = width.div_ceil(8) * height;
    assert_eq!(raster.len(), expected_bytes, "Ripple raster size mismatch");

    let png = raster_to_png(width, height, &raster);
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
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);

    let expected_bytes = width.div_ceil(8) * height;
    assert_eq!(raster.len(), expected_bytes, "Waves raster size mismatch");

    let png = raster_to_png(width, height, &raster);
    let golden = include_bytes!("golden/waves_576x500.png");
    assert_eq!(
        png.len(),
        golden.len(),
        "Waves PNG size differs from golden"
    );
    assert_eq!(png, golden, "Waves PNG content differs from golden");
}

#[test]
fn test_sick_golden() {
    let pattern = patterns::Sick::default();
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);

    let expected_bytes = width.div_ceil(8) * height;
    assert_eq!(raster.len(), expected_bytes, "Sick raster size mismatch");

    let png = raster_to_png(width, height, &raster);
    let golden = include_bytes!("golden/sick_576x1920.png");
    assert_eq!(png.len(), golden.len(), "Sick PNG size differs from golden");
    assert_eq!(png, golden, "Sick PNG content differs from golden");
}

#[test]
fn test_calibration_golden() {
    let pattern = patterns::Calibration::default();
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);

    let expected_bytes = width.div_ceil(8) * height;
    assert_eq!(
        raster.len(),
        expected_bytes,
        "Calibration raster size mismatch"
    );

    let png = raster_to_png(width, height, &raster);
    let golden = include_bytes!("golden/calibration_576x500.png");
    assert_eq!(
        png.len(),
        golden.len(),
        "Calibration PNG size differs from golden"
    );
    assert_eq!(png, golden, "Calibration PNG content differs from golden");
}

#[test]
fn test_other_golden() {
    let pattern = patterns::Other::default();
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);

    let expected_bytes = width.div_ceil(8) * height;
    assert_eq!(raster.len(), expected_bytes, "Other raster size mismatch");

    let png = raster_to_png(width, height, &raster);
    let golden = include_bytes!("golden/other_576x1200.png");
    assert_eq!(
        png.len(),
        golden.len(),
        "Other PNG size differs from golden"
    );
    assert_eq!(png, golden, "Other PNG content differs from golden");
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

    assert_eq!(raster1, raster2, "Pattern output should be deterministic");
}

// ============================================================================
// BINARY COMMAND GOLDEN TESTS
// ============================================================================

/// Path to golden test directory
const GOLDEN_DIR: &str = "tests/golden";

/// Generate printer commands using raster mode via component system
fn generate_raster_commands(name: &str, height: usize) -> Vec<u8> {
    Receipt::new()
        .child(PatternComponent::new(name, height).raster_mode())
        .cut()
        .build()
}

/// Generate printer commands using band mode via component system
fn generate_band_commands(name: &str, height: usize) -> Vec<u8> {
    Receipt::new()
        .child(PatternComponent::new(name, height).band_mode())
        .cut()
        .build()
}

/// Write binary data to a golden file
fn write_golden_binary(name: &str, data: &[u8]) {
    let path = format!("{}/{}.bin", GOLDEN_DIR, name);
    fs::write(&path, data).expect(&format!("Failed to write {}", path));
    println!("Wrote {} ({} bytes)", path, data.len());
}

/// Compare binary data against a golden file
fn check_binary_golden(name: &str, data: &[u8]) {
    let path = format!("{}/{}.bin", GOLDEN_DIR, name);
    let golden = fs::read(&path).expect(&format!(
        "Golden file not found: {}. Run `make golden` to generate.",
        path
    ));

    if data != golden {
        // Find first difference
        let first_diff = data
            .iter()
            .zip(golden.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(data.len().min(golden.len()));

        panic!(
            "Binary mismatch for {}:\n\
             - Golden: {} bytes\n\
             - Actual: {} bytes\n\
             - First difference at byte {:#06x}\n\
             Run `make golden` to regenerate if this change is intentional.",
            name,
            golden.len(),
            data.len(),
            first_diff
        );
    }
}

// ============================================================================
// GOLDEN BINARY FILE GENERATOR
// ============================================================================

/// Generate all golden binary files.
/// Run with: cargo test write_golden_binaries --ignored -- --nocapture
#[test]
#[ignore]
fn write_golden_binaries() {
    // Pattern raster commands (using component system -> codegen)
    for name in ["ripple", "waves", "calibration", "sick", "other"] {
        let pattern = patterns::by_name(name).unwrap();
        let (_width, height) = pattern.default_dimensions();

        // Raster mode
        let cmd = generate_raster_commands(name, height);
        write_golden_binary(&format!("{}_raster", name), &cmd);

        // Band mode
        let cmd = generate_band_commands(name, height);
        write_golden_binary(&format!("{}_band", name), &cmd);
    }

    // Receipts
    write_golden_binary("demo_receipt", &receipt::demo_receipt());
    write_golden_binary("full_receipt", &receipt::full_receipt());

    println!("\nAll golden binary files written to {}/", GOLDEN_DIR);
}

// ============================================================================
// RASTER MODE TESTS
// ============================================================================

#[test]
fn test_binary_golden_ripple_raster() {
    let pattern = patterns::Ripple::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_raster_commands("ripple", height);
    check_binary_golden("ripple_raster", &cmd);
}

#[test]
fn test_binary_golden_waves_raster() {
    let pattern = patterns::Waves::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_raster_commands("waves", height);
    check_binary_golden("waves_raster", &cmd);
}

#[test]
fn test_binary_golden_calibration_raster() {
    let pattern = patterns::Calibration::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_raster_commands("calibration", height);
    check_binary_golden("calibration_raster", &cmd);
}

#[test]
fn test_binary_golden_sick_raster() {
    let pattern = patterns::Sick::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_raster_commands("sick", height);
    check_binary_golden("sick_raster", &cmd);
}

#[test]
fn test_binary_golden_other_raster() {
    let pattern = patterns::Other::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_raster_commands("other", height);
    check_binary_golden("other_raster", &cmd);
}

// ============================================================================
// BAND MODE TESTS
// ============================================================================

#[test]
fn test_binary_golden_ripple_band() {
    let pattern = patterns::Ripple::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_band_commands("ripple", height);
    check_binary_golden("ripple_band", &cmd);
}

#[test]
fn test_binary_golden_waves_band() {
    let pattern = patterns::Waves::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_band_commands("waves", height);
    check_binary_golden("waves_band", &cmd);
}

#[test]
fn test_binary_golden_calibration_band() {
    let pattern = patterns::Calibration::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_band_commands("calibration", height);
    check_binary_golden("calibration_band", &cmd);
}

#[test]
fn test_binary_golden_sick_band() {
    let pattern = patterns::Sick::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_band_commands("sick", height);
    check_binary_golden("sick_band", &cmd);
}

#[test]
fn test_binary_golden_other_band() {
    let pattern = patterns::Other::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_band_commands("other", height);
    check_binary_golden("other_band", &cmd);
}

// ============================================================================
// RECEIPT TESTS
// ============================================================================

#[test]
fn test_binary_golden_demo_receipt() {
    let cmd = receipt::demo_receipt();
    check_binary_golden("demo_receipt", &cmd);
}

#[test]
fn test_binary_golden_full_receipt() {
    let cmd = receipt::full_receipt();
    check_binary_golden("full_receipt", &cmd);
}
