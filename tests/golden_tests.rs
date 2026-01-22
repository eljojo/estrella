//! # Golden Tests
//!
//! These tests ensure that pattern and receipt rendering produces consistent output.
//!
//! ## Test Coverage
//!
//! All printable items get two types of golden tests:
//! - **Binary tests** (`.bin`): Verify printer command bytes (raster mode) are consistent
//! - **Preview tests** (`.png`): Verify visual preview rendering is consistent
//!
//! ## Regenerating Golden Files
//!
//! To regenerate all golden files:
//! ```bash
//! make golden
//! ```

use estrella::components::{ComponentExt, Pattern as PatternComponent, Receipt};
use estrella::ir::Program;
use estrella::receipt;
use estrella::render::dither::{self, DitheringAlgorithm};
use estrella::render::patterns::{self, Pattern};
use estrella::render::weave::{BlendCurve, Weave};
use std::fs;

/// Path to golden test directory
const GOLDEN_DIR: &str = "tests/golden";

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Generate printer commands using raster mode via component system
fn generate_raster_commands(name: &str, height: usize) -> Vec<u8> {
    Receipt::new()
        .child(PatternComponent::new(name, height).with_title().raster_mode())
        .cut()
        .build()
}

/// Generate preview PNG for a program
fn generate_preview_png(program: &Program) -> Vec<u8> {
    program.to_preview_png().expect("Preview rendering failed")
}

/// Generate weave PNG for golden tests
fn generate_weave_png(pattern_names: &[&str], height: usize, crossfade: usize) -> Vec<u8> {
    use image::{GrayImage, Luma};

    let width: usize = 576;
    let width_bytes = width.div_ceil(8);

    // Load patterns (golden/deterministic)
    let pattern_impls: Vec<Box<dyn Pattern>> = pattern_names
        .iter()
        .map(|name| patterns::by_name_golden(name).unwrap())
        .collect();

    let pattern_refs: Vec<&dyn Pattern> = pattern_impls.iter().map(|p| p.as_ref()).collect();
    let weave = Weave::new(pattern_refs)
        .crossfade_pixels(crossfade)
        .curve(BlendCurve::Smooth);

    // Render with Bayer dithering
    let raster_data = dither::generate_raster(
        width,
        height,
        |x, y, w, h| weave.intensity(x, y, w, h),
        DitheringAlgorithm::Bayer,
    );

    // Convert to PNG
    let mut img = GrayImage::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let byte_idx = y * width_bytes + x / 8;
            let bit_idx = 7 - (x % 8);
            let is_black = (raster_data[byte_idx] >> bit_idx) & 1 == 1;
            let color = if is_black { 0u8 } else { 255u8 };
            img.put_pixel(x as u32, y as u32, Luma([color]));
        }
    }

    let mut png_bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )
    .expect("Failed to encode PNG");
    png_bytes
}

/// Write binary data to a golden file
fn write_golden(name: &str, ext: &str, data: &[u8]) {
    let path = format!("{}/{}.{}", GOLDEN_DIR, name, ext);
    fs::write(&path, data).expect(&format!("Failed to write {}", path));
    println!("Wrote {} ({} bytes)", path, data.len());
}

/// Compare data against a golden file
fn check_golden(name: &str, ext: &str, data: &[u8]) {
    let path = format!("{}/{}.{}", GOLDEN_DIR, name, ext);
    let golden = fs::read(&path).expect(&format!(
        "Golden file not found: {}. Run `make golden` to generate.",
        path
    ));

    if data.len() != golden.len() {
        panic!(
            "Golden file size mismatch for {}:\n\
             - Golden: {} bytes\n\
             - Actual: {} bytes\n\
             Run `make golden` to regenerate if this change is intentional.",
            path,
            golden.len(),
            data.len()
        );
    }

    if data != golden {
        // Find first difference for binary files
        let first_diff = data
            .iter()
            .zip(golden.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(data.len());

        panic!(
            "Golden file content mismatch for {}:\n\
             - First difference at byte {:#06x}\n\
             Run `make golden` to regenerate if this change is intentional.",
            path, first_diff
        );
    }
}

// ============================================================================
// GOLDEN FILE GENERATOR
// ============================================================================

/// Generate all golden files (binary commands + preview PNGs).
/// Run with: cargo test generate_golden_files -- --ignored --nocapture
#[test]
#[ignore]
fn generate_golden_files() {
    // Patterns - use list_patterns() to get all available patterns
    for &name in patterns::list_patterns() {
        let pattern = patterns::by_name(name).unwrap();
        let (_width, height) = pattern.default_dimensions();

        // Binary: raster mode
        let cmd = generate_raster_commands(name, height);
        write_golden(&format!("{}_raster", name), "bin", &cmd);

        // Preview PNG
        let program = Receipt::new()
            .child(PatternComponent::new(name, height).with_title().raster_mode())
            .cut()
            .compile();
        let png = generate_preview_png(&program);
        write_golden(name, "png", &png);
    }

    // Receipts
    // Use _golden variants with fixed dates for reproducible tests
    write_golden("demo_receipt", "bin", &receipt::demo_receipt_golden());
    write_golden("full_receipt", "bin", &receipt::full_receipt_golden());
    write_golden("markdown_demo", "bin", &receipt::markdown_demo_golden());

    // Use program_by_name_golden for preview PNGs with fixed dates
    let demo_program = receipt::program_by_name_golden("receipt").unwrap();
    write_golden("demo_receipt", "png", &generate_preview_png(&demo_program));

    let full_program = receipt::program_by_name_golden("receipt-full").unwrap();
    write_golden("full_receipt", "png", &generate_preview_png(&full_program));

    let markdown_program = receipt::program_by_name_golden("markdown").unwrap();
    write_golden("markdown_demo", "png", &generate_preview_png(&markdown_program));

    // Weave (crossfade between patterns)
    // Use 3 distinct patterns, 800px height (~100mm), 160px crossfade (~20mm)
    let weave_png = generate_weave_png(&["riley", "plasma", "waves"], 800, 160);
    write_golden("weave_crossfade", "png", &weave_png);

    println!("\nAll golden files written to {}/", GOLDEN_DIR);
}

// ============================================================================
// PATTERN BINARY TESTS (RASTER MODE)
// ============================================================================

#[test]
fn test_binary_ripple_raster() {
    let pattern = patterns::Ripple::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_raster_commands("ripple", height);
    check_golden("ripple_raster", "bin", &cmd);
}

#[test]
fn test_binary_waves_raster() {
    let pattern = patterns::Waves::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_raster_commands("waves", height);
    check_golden("waves_raster", "bin", &cmd);
}

#[test]
fn test_binary_calibration_raster() {
    let pattern = patterns::Calibration::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_raster_commands("calibration", height);
    check_golden("calibration_raster", "bin", &cmd);
}


// ============================================================================
// PATTERN PREVIEW TESTS
// ============================================================================

#[test]
fn test_preview_ripple() {
    let pattern = patterns::Ripple::default();
    let (_width, height) = pattern.default_dimensions();
    let program = Receipt::new()
        .child(PatternComponent::new("ripple", height).with_title().raster_mode())
        .cut()
        .compile();
    let png = generate_preview_png(&program);
    check_golden("ripple", "png", &png);
}

#[test]
fn test_preview_waves() {
    let pattern = patterns::Waves::default();
    let (_width, height) = pattern.default_dimensions();
    let program = Receipt::new()
        .child(PatternComponent::new("waves", height).with_title().raster_mode())
        .cut()
        .compile();
    let png = generate_preview_png(&program);
    check_golden("waves", "png", &png);
}

#[test]
fn test_preview_calibration() {
    let pattern = patterns::Calibration::default();
    let (_width, height) = pattern.default_dimensions();
    let program = Receipt::new()
        .child(PatternComponent::new("calibration", height).with_title().raster_mode())
        .cut()
        .compile();
    let png = generate_preview_png(&program);
    check_golden("calibration", "png", &png);
}


// ============================================================================
// RECEIPT BINARY TESTS
// ============================================================================

#[test]
fn test_binary_demo_receipt() {
    // Use _golden variant with fixed date for reproducible tests
    let cmd = receipt::demo_receipt_golden();
    check_golden("demo_receipt", "bin", &cmd);
}

#[test]
fn test_binary_full_receipt() {
    let cmd = receipt::full_receipt_golden();
    check_golden("full_receipt", "bin", &cmd);
}

#[test]
fn test_binary_markdown_demo() {
    let cmd = receipt::markdown_demo_golden();
    check_golden("markdown_demo", "bin", &cmd);
}

// ============================================================================
// RECEIPT PREVIEW TESTS
// ============================================================================

#[test]
fn test_preview_demo_receipt() {
    // Use _golden variant with fixed date for reproducible tests
    let program = receipt::program_by_name_golden("receipt").unwrap();
    let png = generate_preview_png(&program);
    check_golden("demo_receipt", "png", &png);
}

#[test]
fn test_preview_full_receipt() {
    let program = receipt::program_by_name_golden("receipt-full").unwrap();
    let png = generate_preview_png(&program);
    check_golden("full_receipt", "png", &png);
}

#[test]
fn test_preview_markdown_demo() {
    let program = receipt::program_by_name_golden("markdown").unwrap();
    let png = generate_preview_png(&program);
    check_golden("markdown_demo", "png", &png);
}

// ============================================================================
// WEAVE TESTS
// ============================================================================

#[test]
fn test_preview_weave_crossfade() {
    let weave_png = generate_weave_png(&["riley", "plasma", "waves"], 800, 160);
    check_golden("weave_crossfade", "png", &weave_png);
}

// ============================================================================
// MISCELLANEOUS TESTS
// ============================================================================

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
    let (_width, height) = pattern.default_dimensions();

    let program1 = Receipt::new()
        .child(PatternComponent::new("ripple", height).with_title().raster_mode())
        .cut()
        .compile();
    let program2 = Receipt::new()
        .child(PatternComponent::new("ripple", height).with_title().raster_mode())
        .cut()
        .compile();

    let png1 = generate_preview_png(&program1);
    let png2 = generate_preview_png(&program2);

    assert_eq!(png1, png2, "Pattern output should be deterministic");
}
