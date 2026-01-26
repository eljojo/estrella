//! # Golden Tests
//!
//! These tests ensure that pattern and receipt rendering produces consistent output.
//!
//! ## Test Coverage
//!
//! - **Binary tests** (`.bin`): One raster mode + one band mode test (ripple pattern),
//!   plus all receipts. This validates the command generation pipeline.
//! - **Preview tests** (`.png`): All patterns, receipts, and weave blends get PNG previews.
//!   This validates visual rendering.
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

/// Patterns that use chaotic dynamics or heavy iterative floating-point math.
/// These produce platform-dependent output (ARM vs x86, different libm implementations)
/// due to the "butterfly effect" - tiny FP differences compound over many iterations.
/// We still generate golden files for visual inspection, but skip byte-exact comparison.
const PLATFORM_DEPENDENT_PATTERNS: &[&str] = &[
    "attractor", // Strange attractors: 100k+ iterations of chaotic systems
];

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Generate printer commands using raster mode via component system.
/// Uses build_raw() for pure StarPRNT protocol bytes (no drain markers).
fn generate_raster_commands(name: &str, height: usize) -> Vec<u8> {
    Receipt::new()
        .child(PatternComponent::new(name, height).with_title().raster_mode())
        .cut()
        .build_raw()
}

/// Generate printer commands using band mode via component system.
/// Uses build_raw() for pure StarPRNT protocol bytes (no drain markers).
fn generate_band_commands(name: &str, height: usize) -> Vec<u8> {
    Receipt::new()
        .child(PatternComponent::new(name, height).with_title().band_mode())
        .cut()
        .build_raw()
}

/// Generate preview PNG for a program
fn generate_preview_png(program: &Program) -> Vec<u8> {
    program.to_preview_png().expect("Preview rendering failed")
}

/// Generate weave PNG for golden tests
fn generate_weave_png(pattern_names: &[&str], height: usize, crossfade: usize) -> Vec<u8> {
    generate_weave_png_with_dither(pattern_names, height, crossfade, DitheringAlgorithm::Bayer)
}

/// Generate weave PNG with a specific dithering algorithm
fn generate_weave_png_with_dither(
    pattern_names: &[&str],
    height: usize,
    crossfade: usize,
    algorithm: DitheringAlgorithm,
) -> Vec<u8> {
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

    // Render with specified dithering algorithm
    let raster_data = dither::generate_raster(
        width,
        height,
        |x, y, w, h| weave.intensity(x, y, w, h),
        algorithm,
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

/// Patterns used for dithering algorithm comparison tests
const DITHER_TEST_PATTERNS: &[&str] = &["plasma", "rings", "ripple", "topography"];
const DITHER_TEST_HEIGHT: usize = 1200;
const DITHER_TEST_CROSSFADE: usize = 200;

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
    // Binary tests: Only ripple pattern (one for raster mode, one for band mode)
    // This validates the command generation pipeline without redundant files
    let ripple = patterns::by_name("ripple").unwrap();
    let (_width, ripple_height) = ripple.default_dimensions();

    let raster_cmd = generate_raster_commands("ripple", ripple_height);
    write_golden("ripple_raster", "bin", &raster_cmd);

    let band_cmd = generate_band_commands("ripple", ripple_height);
    write_golden("ripple_band", "bin", &band_cmd);

    // Preview PNGs for all patterns
    for &name in patterns::list_patterns() {
        let pattern = patterns::by_name(name).unwrap();
        let (_width, height) = pattern.default_dimensions();

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

    // Dithering algorithm comparison
    // Use 4 patterns (plasma, ring, ripple, topography) to show each algorithm's characteristics
    let dither_bayer = generate_weave_png_with_dither(
        DITHER_TEST_PATTERNS,
        DITHER_TEST_HEIGHT,
        DITHER_TEST_CROSSFADE,
        DitheringAlgorithm::Bayer,
    );
    write_golden("dither_bayer", "png", &dither_bayer);

    let dither_floyd_steinberg = generate_weave_png_with_dither(
        DITHER_TEST_PATTERNS,
        DITHER_TEST_HEIGHT,
        DITHER_TEST_CROSSFADE,
        DitheringAlgorithm::FloydSteinberg,
    );
    write_golden("dither_floyd_steinberg", "png", &dither_floyd_steinberg);

    let dither_atkinson = generate_weave_png_with_dither(
        DITHER_TEST_PATTERNS,
        DITHER_TEST_HEIGHT,
        DITHER_TEST_CROSSFADE,
        DitheringAlgorithm::Atkinson,
    );
    write_golden("dither_atkinson", "png", &dither_atkinson);

    let dither_jarvis = generate_weave_png_with_dither(
        DITHER_TEST_PATTERNS,
        DITHER_TEST_HEIGHT,
        DITHER_TEST_CROSSFADE,
        DitheringAlgorithm::Jarvis,
    );
    write_golden("dither_jarvis", "png", &dither_jarvis);

    println!("\nAll golden files written to {}/", GOLDEN_DIR);
}

// ============================================================================
// PATTERN BINARY TESTS
// ============================================================================

#[test]
fn test_binary_ripple_raster() {
    let pattern = patterns::Ripple::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_raster_commands("ripple", height);
    check_golden("ripple_raster", "bin", &cmd);
}

#[test]
fn test_binary_ripple_band() {
    let pattern = patterns::Ripple::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_band_commands("ripple", height);
    check_golden("ripple_band", "bin", &cmd);
}


// ============================================================================
// PATTERN PREVIEW TESTS
// ============================================================================

/// Test that all pattern previews match their golden PNGs
#[test]
fn test_preview_all_patterns() {
    for &name in patterns::list_patterns() {
        // Skip platform-dependent patterns (chaotic dynamics, heavy FP iteration)
        if PLATFORM_DEPENDENT_PATTERNS.contains(&name) {
            continue;
        }

        let pattern = patterns::by_name(name).expect("Pattern not found");
        let (_width, height) = pattern.default_dimensions();
        let program = Receipt::new()
            .child(PatternComponent::new(name, height).with_title().raster_mode())
            .cut()
            .compile();
        let png = generate_preview_png(&program);
        check_golden(name, "png", &png);
    }
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
// DITHERING ALGORITHM TESTS
// ============================================================================

#[test]
fn test_dither_bayer() {
    let png = generate_weave_png_with_dither(
        DITHER_TEST_PATTERNS,
        DITHER_TEST_HEIGHT,
        DITHER_TEST_CROSSFADE,
        DitheringAlgorithm::Bayer,
    );
    check_golden("dither_bayer", "png", &png);
}

#[test]
fn test_dither_floyd_steinberg() {
    let png = generate_weave_png_with_dither(
        DITHER_TEST_PATTERNS,
        DITHER_TEST_HEIGHT,
        DITHER_TEST_CROSSFADE,
        DitheringAlgorithm::FloydSteinberg,
    );
    check_golden("dither_floyd_steinberg", "png", &png);
}

#[test]
fn test_dither_atkinson() {
    let png = generate_weave_png_with_dither(
        DITHER_TEST_PATTERNS,
        DITHER_TEST_HEIGHT,
        DITHER_TEST_CROSSFADE,
        DitheringAlgorithm::Atkinson,
    );
    check_golden("dither_atkinson", "png", &png);
}

#[test]
fn test_dither_jarvis() {
    let png = generate_weave_png_with_dither(
        DITHER_TEST_PATTERNS,
        DITHER_TEST_HEIGHT,
        DITHER_TEST_CROSSFADE,
        DitheringAlgorithm::Jarvis,
    );
    check_golden("dither_jarvis", "png", &png);
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
