//! # Golden Tests
//!
//! These tests ensure that pattern and receipt rendering produces consistent output.
//!
//! ## Test Coverage
//!
//! All printable items get two types of golden tests:
//! - **Binary tests** (`.bin`): Verify printer command bytes are consistent
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
use estrella::render::patterns::{self, Pattern};
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

/// Generate printer commands using band mode via component system
fn generate_band_commands(name: &str, height: usize) -> Vec<u8> {
    Receipt::new()
        .child(PatternComponent::new(name, height).with_title().band_mode())
        .cut()
        .build()
}

/// Generate preview PNG for a program
fn generate_preview_png(program: &Program) -> Vec<u8> {
    program.to_preview_png().expect("Preview rendering failed")
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
    // Patterns
    for name in ["ripple", "waves", "calibration", "sick", "other"] {
        let pattern = patterns::by_name(name).unwrap();
        let (_width, height) = pattern.default_dimensions();

        // Binary: raster mode
        let cmd = generate_raster_commands(name, height);
        write_golden(&format!("{}_raster", name), "bin", &cmd);

        // Binary: band mode
        let cmd = generate_band_commands(name, height);
        write_golden(&format!("{}_band", name), "bin", &cmd);

        // Preview PNG
        let program = Receipt::new()
            .child(PatternComponent::new(name, height).with_title().raster_mode())
            .cut()
            .compile();
        let png = generate_preview_png(&program);
        write_golden(name, "png", &png);
    }

    // Receipts
    write_golden("demo_receipt", "bin", &receipt::demo_receipt());
    write_golden("full_receipt", "bin", &receipt::full_receipt());
    write_golden("markdown_demo", "bin", &receipt::markdown_demo());

    let demo_program = receipt::program_by_name("receipt").unwrap();
    write_golden("demo_receipt", "png", &generate_preview_png(&demo_program));

    let full_program = receipt::program_by_name("receipt-full").unwrap();
    write_golden("full_receipt", "png", &generate_preview_png(&full_program));

    let markdown_program = receipt::program_by_name("markdown").unwrap();
    write_golden("markdown_demo", "png", &generate_preview_png(&markdown_program));

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

#[test]
fn test_binary_sick_raster() {
    let pattern = patterns::Sick::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_raster_commands("sick", height);
    check_golden("sick_raster", "bin", &cmd);
}

#[test]
fn test_binary_other_raster() {
    let pattern = patterns::Other::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_raster_commands("other", height);
    check_golden("other_raster", "bin", &cmd);
}

// ============================================================================
// PATTERN BINARY TESTS (BAND MODE)
// ============================================================================

#[test]
fn test_binary_ripple_band() {
    let pattern = patterns::Ripple::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_band_commands("ripple", height);
    check_golden("ripple_band", "bin", &cmd);
}

#[test]
fn test_binary_waves_band() {
    let pattern = patterns::Waves::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_band_commands("waves", height);
    check_golden("waves_band", "bin", &cmd);
}

#[test]
fn test_binary_calibration_band() {
    let pattern = patterns::Calibration::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_band_commands("calibration", height);
    check_golden("calibration_band", "bin", &cmd);
}

#[test]
fn test_binary_sick_band() {
    let pattern = patterns::Sick::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_band_commands("sick", height);
    check_golden("sick_band", "bin", &cmd);
}

#[test]
fn test_binary_other_band() {
    let pattern = patterns::Other::default();
    let (_width, height) = pattern.default_dimensions();
    let cmd = generate_band_commands("other", height);
    check_golden("other_band", "bin", &cmd);
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

#[test]
fn test_preview_sick() {
    let pattern = patterns::Sick::default();
    let (_width, height) = pattern.default_dimensions();
    let program = Receipt::new()
        .child(PatternComponent::new("sick", height).with_title().raster_mode())
        .cut()
        .compile();
    let png = generate_preview_png(&program);
    check_golden("sick", "png", &png);
}

#[test]
fn test_preview_other() {
    let pattern = patterns::Other::default();
    let (_width, height) = pattern.default_dimensions();
    let program = Receipt::new()
        .child(PatternComponent::new("other", height).with_title().raster_mode())
        .cut()
        .compile();
    let png = generate_preview_png(&program);
    check_golden("other", "png", &png);
}

// ============================================================================
// RECEIPT BINARY TESTS
// ============================================================================

#[test]
fn test_binary_demo_receipt() {
    let cmd = receipt::demo_receipt();
    check_golden("demo_receipt", "bin", &cmd);
}

#[test]
fn test_binary_full_receipt() {
    let cmd = receipt::full_receipt();
    check_golden("full_receipt", "bin", &cmd);
}

#[test]
fn test_binary_markdown_demo() {
    let cmd = receipt::markdown_demo();
    check_golden("markdown_demo", "bin", &cmd);
}

// ============================================================================
// RECEIPT PREVIEW TESTS
// ============================================================================

#[test]
fn test_preview_demo_receipt() {
    let program = receipt::program_by_name("receipt").unwrap();
    let png = generate_preview_png(&program);
    check_golden("demo_receipt", "png", &png);
}

#[test]
fn test_preview_full_receipt() {
    let program = receipt::program_by_name("receipt-full").unwrap();
    let png = generate_preview_png(&program);
    check_golden("full_receipt", "png", &png);
}

#[test]
fn test_preview_markdown_demo() {
    let program = receipt::program_by_name("markdown").unwrap();
    let png = generate_preview_png(&program);
    check_golden("markdown_demo", "png", &png);
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
