//! # StarPRNT Graphics Commands
//!
//! This module implements bit image and raster graphics commands for
//! Star Micronics thermal printers.
//!
//! ## Graphics Modes Overview
//!
//! StarPRNT supports several graphics modes:
//!
//! | Mode | Command | Description | Best For |
//! |------|---------|-------------|----------|
//! | Band | ESC k | 24-row strips | Streaming, simple patterns |
//! | Raster | ESC GS S | Arbitrary height | Full images, complex art |
//! | NV Graphics | ESC GS ( L | Stored in flash | Logos, repeated images |
//!
//! ## Coordinate System
//!
//! ```text
//! (0,0) ──────────────────────► X (horizontal, 576 dots max)
//!   │
//!   │   ████████  ← Each dot is ~0.125mm (203 DPI)
//!   │   ████████
//!   │   ████████
//!   ▼
//!   Y (vertical, paper feed direction)
//! ```
//!
//! ## Bit Packing
//!
//! Graphics data is packed as bytes where each bit represents one dot:
//! - Bit 7 (MSB) = leftmost dot
//! - Bit 0 (LSB) = rightmost dot
//! - 1 = black (print), 0 = white (no print)
//!
//! ```text
//! Byte value 0xF0 = 11110000 = ████░░░░
//! Byte value 0x0F = 00001111 = ░░░░████
//! Byte value 0xAA = 10101010 = █░█░█░█░
//! ```
//!
//! ## TSP650II Specifications
//!
//! | Property | Value |
//! |----------|-------|
//! | Max print width | 576 dots (72 bytes) |
//! | Resolution | 203 DPI (~8 dots/mm) |
//! | Band height | 24 dots |

use super::commands::{ESC, GS, u16_le};

// ============================================================================
// BAND MODE GRAPHICS (ESC k)
// ============================================================================

/// # Fine Density Bit Image - Band Mode (ESC k n1 n2 d1...dk)
///
/// Prints a 24-row band of graphics data. This is the simplest and most
/// efficient way to print graphics on Star printers.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC k n1 n2 d1...dk |
/// | Hex     | 1B 6B n1 n2 d1...dk |
/// | Decimal | 27 107 n1 n2 d1...dk |
///
/// ## Parameters
///
/// - `n1`: Width in bytes (1-72 for TSP650II)
/// - `n2`: Always 0 (reserved for future use)
/// - `d1...dk`: Image data, k = n1 × 24 bytes
///
/// ## Data Layout
///
/// Data is organized column-by-column, then row-by-row:
///
/// ```text
/// Columns:    0    1    2   ...  n1-1
///          ┌────┬────┬────┬───┬─────┐
/// Row 0    │ d1 │ d2 │ d3 │...│ dn1 │  (bytes 1 to n1)
/// Row 1    │    │    │    │   │     │  (bytes n1+1 to 2*n1)
/// ...      │    │    │    │   │     │
/// Row 23   │    │    │    │   │     │  (bytes 23*n1+1 to 24*n1)
///          └────┴────┴────┴───┴─────┘
///
/// Each byte: bit7=left, bit0=right (8 horizontal dots)
/// ```
///
/// ## Width Calculation
///
/// ```text
/// width_bytes = (width_dots + 7) / 8
///
/// For TSP650II (576 dots): 576 / 8 = 72 bytes
/// For 384 dots: 384 / 8 = 48 bytes
/// ```
///
/// ## Example
///
/// ```
/// use estrella::protocol::graphics;
///
/// // Create a 72-byte wide, 24-row black band (all dots on)
/// let data = vec![0xFF; 72 * 24];
/// let cmd = graphics::band(72, &data);
///
/// // Header is ESC k 72 0
/// assert_eq!(&cmd[0..4], &[0x1B, 0x6B, 72, 0]);
/// // Total length: 4 header + 72*24 data = 1732 bytes
/// assert_eq!(cmd.len(), 4 + 72 * 24);
/// ```
///
/// ## Performance Notes
///
/// - Band mode is efficient for streaming (24 rows at a time)
/// - No chunking needed - printer handles 24 rows naturally
/// - Feed between bands with ESC J if needed (3mm = ESC J 12)
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.12 (ESC k)
pub fn band(width_bytes: u8, data: &[u8]) -> Vec<u8> {
    debug_assert!(
        data.len() == width_bytes as usize * 24,
        "Band data must be exactly width_bytes * 24 bytes. Expected {}, got {}",
        width_bytes as usize * 24,
        data.len()
    );

    let mut cmd = Vec::with_capacity(4 + data.len());
    cmd.push(ESC);
    cmd.push(b'k');
    cmd.push(width_bytes);
    cmd.push(0); // n2 is always 0
    cmd.extend_from_slice(data);
    cmd
}

// ============================================================================
// RASTER MODE GRAPHICS (ESC GS S)
// ============================================================================

/// # Print Raster Graphics Data (ESC GS S m xL xH yL yH n d1...dk)
///
/// Prints a raster image of arbitrary height. This is the most flexible
/// graphics command, suitable for complex images and visual art.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC GS S m xL xH yL yH n d1...dk |
/// | Hex     | 1B 1D 53 m xL xH yL yH n d1...dk |
/// | Decimal | 27 29 83 m xL xH yL yH n d1...dk |
///
/// ## Parameters
///
/// - `m`: Mode (1 = monochrome, 1 bit per dot)
/// - `xL, xH`: Width in bytes, little-endian (1-128)
/// - `yL, yH`: Height in dots, little-endian (1-65535)
/// - `n`: Color (0 = black)
/// - `d1...dk`: Image data, k = width_bytes × height bytes
///
/// ## Width and Height Encoding
///
/// Both dimensions use 16-bit little-endian encoding:
///
/// ```text
/// width_bytes = xL + (xH × 256)
/// height_dots = yL + (yH × 256)
///
/// Example: 72 bytes wide = [0x48, 0x00] (72 = 0x0048)
/// Example: 500 rows high = [0xF4, 0x01] (500 = 0x01F4)
/// ```
///
/// ## Data Layout
///
/// Same as band mode - row-by-row, each byte is 8 horizontal dots:
///
/// ```text
/// Row 0:    d[0]      d[1]      ... d[width-1]
/// Row 1:    d[width]  d[width+1] ... d[2*width-1]
/// ...
/// Row h-1:  d[(h-1)*width] ... d[h*width-1]
/// ```
///
/// ## Example
///
/// ```
/// use estrella::protocol::graphics;
///
/// // Create a 576-dot wide (72 bytes), 100-row tall image
/// let width_dots = 576;
/// let height = 100;
/// let data = vec![0xAA; 72 * 100]; // Vertical stripes pattern
///
/// let cmd = graphics::raster(width_dots, height, &data);
///
/// // Header: ESC GS S 1 72 0 100 0 0
/// assert_eq!(&cmd[0..3], &[0x1B, 0x1D, 0x53]);
/// assert_eq!(cmd[3], 1);  // m = monochrome
/// assert_eq!(cmd[4], 72); // xL = 72
/// assert_eq!(cmd[5], 0);  // xH = 0
/// assert_eq!(cmd[6], 100); // yL = 100
/// assert_eq!(cmd[7], 0);  // yH = 0
/// assert_eq!(cmd[8], 0);  // n = black
/// ```
///
/// ## Chunking for Bluetooth
///
/// When sending large images over Bluetooth, the data may need to be
/// chunked to avoid buffer overflow. Recommended chunk size: 256 rows.
///
/// ```text
/// // For a 1000-row image:
/// // Chunk 1: raster(576, 256, &data[0..72*256])
/// // Chunk 2: raster(576, 256, &data[72*256..72*512])
/// // Chunk 3: raster(576, 256, &data[72*512..72*768])
/// // Chunk 4: raster(576, 232, &data[72*768..72*1000])
/// ```
///
/// ## Comparison with Band Mode
///
/// | Aspect | Band (ESC k) | Raster (ESC GS S) |
/// |--------|--------------|-------------------|
/// | Height | Fixed 24 rows | Variable 1-65535 |
/// | Streaming | Natural | Requires chunking |
/// | Memory | Low | Higher for large images |
/// | Use case | Simple patterns | Complex images |
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.12 (ESC GS S)
pub fn raster(width_dots: u16, height: u16, data: &[u8]) -> Vec<u8> {
    let width_bytes = width_dots.div_ceil(8);
    let expected_len = width_bytes as usize * height as usize;

    debug_assert!(
        data.len() == expected_len,
        "Raster data length mismatch. Expected {} ({} bytes × {} rows), got {}",
        expected_len,
        width_bytes,
        height,
        data.len()
    );

    let [xl, xh] = u16_le(width_bytes);
    let [yl, yh] = u16_le(height);

    let mut cmd = Vec::with_capacity(9 + data.len());
    cmd.push(ESC);
    cmd.push(GS);
    cmd.push(b'S');
    cmd.push(1); // m = 1 (monochrome)
    cmd.push(xl);
    cmd.push(xh);
    cmd.push(yl);
    cmd.push(yh);
    cmd.push(0); // n = 0 (black)
    cmd.extend_from_slice(data);
    cmd
}

/// Create a raster command for a chunk of a larger image.
///
/// This is useful when transmitting large images over Bluetooth,
/// where buffer limitations require breaking the image into pieces.
///
/// ## Parameters
///
/// - `width_dots`: Image width in dots (e.g., 576 for TSP650II)
/// - `chunk_height`: Height of this chunk in rows
/// - `data`: Pixel data for this chunk only
///
/// ## Example
///
/// ```
/// use estrella::protocol::graphics;
///
/// let width = 576;
/// let full_height = 1000;
/// let chunk_size = 256;
/// let width_bytes = (width + 7) / 8;
///
/// // Full image data
/// let full_data: Vec<u8> = vec![0xAA; width_bytes as usize * full_height];
///
/// // Send in chunks
/// for chunk_start in (0..full_height).step_by(chunk_size) {
///     let chunk_end = (chunk_start + chunk_size).min(full_height);
///     let chunk_h = chunk_end - chunk_start;
///     let byte_start = chunk_start * width_bytes as usize;
///     let byte_end = chunk_end * width_bytes as usize;
///     let chunk_data = &full_data[byte_start..byte_end];
///
///     let cmd = graphics::raster(width, chunk_h as u16, chunk_data);
///     // Send cmd to printer...
/// }
/// ```
#[inline]
pub fn raster_chunk(width_dots: u16, chunk_height: u16, data: &[u8]) -> Vec<u8> {
    raster(width_dots, chunk_height, data)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_band_header() {
        let data = vec![0xFF; 72 * 24];
        let cmd = band(72, &data);

        // Check header bytes
        assert_eq!(cmd[0], 0x1B); // ESC
        assert_eq!(cmd[1], 0x6B); // 'k'
        assert_eq!(cmd[2], 72); // width_bytes
        assert_eq!(cmd[3], 0); // n2 = 0

        // Check total length
        assert_eq!(cmd.len(), 4 + 72 * 24);
    }

    #[test]
    fn test_band_small() {
        let data = vec![0xAA; 8 * 24]; // 64 dots wide
        let cmd = band(8, &data);

        assert_eq!(&cmd[0..4], &[0x1B, 0x6B, 8, 0]);
        assert_eq!(cmd.len(), 4 + 8 * 24);
    }

    #[test]
    fn test_band_preserves_data() {
        let data: Vec<u8> = (0..72 * 24).map(|i| (i % 256) as u8).collect();
        let cmd = band(72, &data);

        // Data should be preserved after header
        assert_eq!(&cmd[4..], &data[..]);
    }

    #[test]
    fn test_raster_header() {
        let data = vec![0xFF; 72 * 100];
        let cmd = raster(576, 100, &data);

        assert_eq!(cmd[0], 0x1B); // ESC
        assert_eq!(cmd[1], 0x1D); // GS
        assert_eq!(cmd[2], 0x53); // 'S'
        assert_eq!(cmd[3], 1); // m = monochrome
        assert_eq!(cmd[4], 72); // xL (576/8 = 72)
        assert_eq!(cmd[5], 0); // xH
        assert_eq!(cmd[6], 100); // yL
        assert_eq!(cmd[7], 0); // yH
        assert_eq!(cmd[8], 0); // n = black
    }

    #[test]
    fn test_raster_large_height() {
        // Test with height > 255 to verify little-endian encoding
        let height: u16 = 500;
        let data = vec![0xFF; 72 * height as usize];
        let cmd = raster(576, height, &data);

        // 500 = 0x01F4 -> [0xF4, 0x01] in little-endian
        assert_eq!(cmd[6], 0xF4); // yL
        assert_eq!(cmd[7], 0x01); // yH
    }

    #[test]
    fn test_raster_width_rounding() {
        // 577 dots should round up to 73 bytes
        // But we need data that matches
        let width_dots = 577;
        let width_bytes = (width_dots + 7) / 8; // 73
        let data = vec![0xFF; width_bytes as usize * 10];
        let cmd = raster(width_dots, 10, &data);

        assert_eq!(cmd[4], 73); // xL
        assert_eq!(cmd[5], 0); // xH
    }

    #[test]
    fn test_raster_total_length() {
        let width = 576;
        let height = 100;
        let width_bytes = (width + 7) / 8;
        let data = vec![0x00; width_bytes as usize * height as usize];
        let cmd = raster(width, height, &data);

        // 9 header bytes + data
        assert_eq!(cmd.len(), 9 + 72 * 100);
    }

    #[test]
    fn test_raster_preserves_data() {
        let data: Vec<u8> = (0..72 * 50).map(|i| (i % 256) as u8).collect();
        let cmd = raster(576, 50, &data);

        // Data should be preserved after 9-byte header
        assert_eq!(&cmd[9..], &data[..]);
    }
}
