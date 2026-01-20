//! # Golden Tests for Patterns
//!
//! These tests ensure that pattern rendering produces consistent output.
//!
//! ## PNG Golden Tests
//! Golden files are stored in `tests/golden/` and compared against generated output.
//! Patterns use their `default_dimensions()` for canonical sizing.
//!
//! To regenerate golden files (if intentional changes are made):
//! ```bash
//! cargo run -- print --png tests/golden/ripple_576x500.png ripple
//! cargo run -- print --png tests/golden/waves_576x500.png waves
//! cargo run -- print --png tests/golden/sick_576x1920.png sick
//! cargo run -- print --png tests/golden/calibration_576x500.png calibration
//! ```
//!
//! ## Binary Command Golden Tests
//! These test the actual printer command bytes (ESC sequences + raster data).
//! SHA256 hashes are stored in the test file. If a hash changes:
//! 1. Run with `-- --nocapture` to see the hex diff
//! 2. If the change is intentional, update the hash constant
//!
//! To see current hashes:
//! ```bash
//! cargo test binary_golden -- --nocapture
//! ```

use estrella::protocol::{commands, graphics};
use estrella::render::{dither, patterns};
use estrella::render::patterns::Pattern;
use sha2::{Sha256, Digest};

/// Generate raster data for a pattern using its default dimensions
fn generate_pattern_raster(pattern: &dyn Pattern) -> Vec<u8> {
    let (width, height) = pattern.default_dimensions();
    generate_pattern_raster_sized(pattern, width, height)
}

/// Generate raster data for a pattern with custom dimensions
fn generate_pattern_raster_sized(
    pattern: &dyn Pattern,
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
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);

    // Verify raster dimensions
    let expected_bytes = width.div_ceil(8) * height;
    assert_eq!(
        raster.len(),
        expected_bytes,
        "Ripple raster size mismatch"
    );

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
    assert_eq!(
        png, golden,
        "Waves PNG content differs from golden"
    );
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

// ============================================================================
// BINARY COMMAND GOLDEN TESTS
// ============================================================================

// Expected SHA256 hashes for binary command output
// To update: run `cargo test binary_golden -- --nocapture` and copy new hashes
mod binary_hashes {
    // Format: init + raster commands (256-row chunks) + cut
    pub const RIPPLE_RASTER: &str = "449d197789ed976fb2eaf7cfe6e24a3ab78d03b8ff83666a4941e7c0af4177ce";
    pub const WAVES_RASTER: &str = "63fa02dbe70882c79c078bd2f351de15889ad92e2ff682a638e07e52a80271a4";
    pub const CALIBRATION_RASTER: &str = "463b40dd194d16e1d384f98490d5f1a5887bf5c74e01fc91f687aa3eebe41064";
    pub const SICK_RASTER: &str = "647aea2544d224285ce3e99f41e6965c69282d7b4eea9092158cb800bff6de6c";

    // Format: init + band commands (24-row bands + feed) + cut
    pub const RIPPLE_BAND: &str = "7cc1cc0fa206421761f267cfdd9d5f8cd59a988001455f3cc51cf1eb10e6f887";
    pub const WAVES_BAND: &str = "4f837115c5ba04563d3d574302c59886e499b06bc972ec20dd4ebe572f3cc969";
    pub const CALIBRATION_BAND: &str = "826cdd180dccd22a0342f827db74281d44ff1412791445b885a6a5932c510855";
    pub const SICK_BAND: &str = "3e9c8a9e806c6c04e3f68c46262255ddcb5edf3ea82a6efe5acad44f6681c11e";
}

/// Generate printer commands using raster mode (ESC GS S)
fn generate_raster_commands(width: usize, height: usize, raster_data: &[u8]) -> Vec<u8> {
    let mut cmd = Vec::new();

    // Initialize printer
    cmd.extend(commands::init());

    // Send raster in 256-row chunks (matching main.rs behavior)
    let width_bytes = width.div_ceil(8);
    let chunk_rows = 256;

    let mut row_offset = 0;
    while row_offset < height {
        let chunk_height = (height - row_offset).min(chunk_rows);
        let byte_start = row_offset * width_bytes;
        let byte_end = (row_offset + chunk_height) * width_bytes;
        let chunk_data = &raster_data[byte_start..byte_end];

        cmd.extend(graphics::raster(width as u16, chunk_height as u16, chunk_data));

        row_offset += chunk_height;
    }

    // Cut paper
    cmd.extend(commands::cut_full_feed());

    cmd
}

/// Generate printer commands using band mode (ESC k)
fn generate_band_commands(width: usize, height: usize, raster_data: &[u8]) -> Vec<u8> {
    let mut cmd = Vec::new();

    // Initialize printer
    cmd.extend(commands::init());

    // Send in 24-row bands (matching Python sick.py behavior)
    let width_bytes = width.div_ceil(8);
    let band_height = 24;
    let full_band_size = width_bytes * band_height;

    let mut row_offset = 0;
    while row_offset < height {
        let band_rows = (height - row_offset).min(band_height);
        let byte_start = row_offset * width_bytes;
        let byte_end = (row_offset + band_rows) * width_bytes;
        let band_data = &raster_data[byte_start..byte_end];

        // Pad last band to 24 rows if needed (ESC k requires exactly 24 rows)
        if band_rows < band_height {
            let mut padded = band_data.to_vec();
            padded.resize(full_band_size, 0x00); // Pad with white
            cmd.extend(graphics::band(width_bytes as u8, &padded));
        } else {
            cmd.extend(graphics::band(width_bytes as u8, band_data));
        }
        cmd.extend(commands::feed_units(12)); // 3mm feed

        row_offset += band_rows;
    }

    // Cut paper
    cmd.extend(commands::cut_full_feed());

    cmd
}

/// Compute SHA256 hash of data and return as hex string
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Show hex diff between expected and actual data (first difference)
fn show_hex_diff(name: &str, expected_hash: &str, actual_hash: &str, data: &[u8]) {
    println!("\n=== BINARY GOLDEN TEST FAILED: {} ===", name);
    println!("Expected hash: {}", expected_hash);
    println!("Actual hash:   {}", actual_hash);
    println!("\nCommand length: {} bytes", data.len());

    // Show first 256 bytes as hex dump
    println!("\nFirst 256 bytes (hex):");
    for (i, chunk) in data.iter().take(256).collect::<Vec<_>>().chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        for byte in chunk {
            print!("{:02x} ", byte);
        }
        println!();
    }

    // Show command structure summary
    println!("\nCommand structure:");
    let mut pos = 0;
    let mut cmd_count = 0;
    while pos < data.len() && cmd_count < 20 {
        if data[pos] == 0x1B {
            // ESC sequence
            if pos + 1 < data.len() {
                let next = data[pos + 1];
                match next {
                    0x40 => println!("  {:04x}: ESC @ (init)", pos),
                    0x64 => {
                        if pos + 2 < data.len() {
                            println!("  {:04x}: ESC d {:02x} (cut)", pos, data[pos + 2]);
                        }
                    }
                    0x6B => {
                        if pos + 3 < data.len() {
                            println!("  {:04x}: ESC k {:02x} {:02x} (band, {} bytes wide)",
                                pos, data[pos + 2], data[pos + 3], data[pos + 2]);
                        }
                    }
                    0x4A => {
                        if pos + 2 < data.len() {
                            println!("  {:04x}: ESC J {:02x} (feed {} units)", pos, data[pos + 2], data[pos + 2]);
                        }
                    }
                    0x1D => {
                        if pos + 2 < data.len() && data[pos + 2] == 0x53 {
                            // ESC GS S raster command
                            if pos + 8 < data.len() {
                                let xl = data[pos + 4] as u16;
                                let xh = data[pos + 5] as u16;
                                let yl = data[pos + 6] as u16;
                                let yh = data[pos + 7] as u16;
                                let w = xl + xh * 256;
                                let h = yl + yh * 256;
                                println!("  {:04x}: ESC GS S (raster {}x{} = {} bytes)",
                                    pos, w, h, w as usize * h as usize);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        pos += 1;
        cmd_count += 1;
    }

    println!("\nTo update hash, copy the actual hash above to binary_hashes::{}", name);
}

/// Check binary output against expected hash
fn check_binary_golden(name: &str, expected_hash: &str, data: &[u8]) {
    let actual_hash = sha256_hex(data);

    println!("{}: {} ({} bytes)", name, actual_hash, data.len());

    if expected_hash != "PENDING" && actual_hash != expected_hash {
        show_hex_diff(name, expected_hash, &actual_hash, data);
        panic!(
            "Binary golden test failed for {}: hash mismatch",
            name
        );
    }
}

// Raster mode tests (default mode)
#[test]
fn test_binary_golden_ripple_raster() {
    let pattern = patterns::Ripple::default();
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);
    let cmd = generate_raster_commands(width, height, &raster);

    check_binary_golden("RIPPLE_RASTER", binary_hashes::RIPPLE_RASTER, &cmd);
}

#[test]
fn test_binary_golden_waves_raster() {
    let pattern = patterns::Waves::default();
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);
    let cmd = generate_raster_commands(width, height, &raster);

    check_binary_golden("WAVES_RASTER", binary_hashes::WAVES_RASTER, &cmd);
}

#[test]
fn test_binary_golden_calibration_raster() {
    let pattern = patterns::Calibration::default();
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);
    let cmd = generate_raster_commands(width, height, &raster);

    check_binary_golden("CALIBRATION_RASTER", binary_hashes::CALIBRATION_RASTER, &cmd);
}

#[test]
fn test_binary_golden_sick_raster() {
    let pattern = patterns::Sick::default();
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);
    let cmd = generate_raster_commands(width, height, &raster);

    check_binary_golden("SICK_RASTER", binary_hashes::SICK_RASTER, &cmd);
}

// Band mode tests (--band flag)
#[test]
fn test_binary_golden_ripple_band() {
    let pattern = patterns::Ripple::default();
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);
    let cmd = generate_band_commands(width, height, &raster);

    check_binary_golden("RIPPLE_BAND", binary_hashes::RIPPLE_BAND, &cmd);
}

#[test]
fn test_binary_golden_waves_band() {
    let pattern = patterns::Waves::default();
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);
    let cmd = generate_band_commands(width, height, &raster);

    check_binary_golden("WAVES_BAND", binary_hashes::WAVES_BAND, &cmd);
}

#[test]
fn test_binary_golden_calibration_band() {
    let pattern = patterns::Calibration::default();
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);
    let cmd = generate_band_commands(width, height, &raster);

    check_binary_golden("CALIBRATION_BAND", binary_hashes::CALIBRATION_BAND, &cmd);
}

#[test]
fn test_binary_golden_sick_band() {
    let pattern = patterns::Sick::default();
    let (width, height) = pattern.default_dimensions();
    let raster = generate_pattern_raster(&pattern);
    let cmd = generate_band_commands(width, height, &raster);

    check_binary_golden("SICK_BAND", binary_hashes::SICK_BAND, &cmd);
}
