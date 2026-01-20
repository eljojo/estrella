//! # StarPRNT Protocol Commands
//!
//! This module implements the StarPRNT command protocol used by Star Micronics
//! thermal receipt printers (TSP650II, TSP700II, TSP800II, etc.).
//!
//! ## Protocol Overview
//!
//! StarPRNT is an ESC/POS-like protocol where commands are byte sequences
//! starting with escape characters. The protocol supports:
//!
//! - **Text printing**: Various fonts, sizes, alignments, and styles
//! - **Graphics**: Bit images, raster graphics, stored logos
//! - **Barcodes**: 1D (Code39, Code128, etc.) and 2D (QR, PDF417)
//! - **Paper control**: Feeding, cutting, page mode
//! - **Status**: Auto-Status-Back (ASB) for printer state monitoring
//!
//! ## Escape Sequence Structure
//!
//! Commands follow these patterns:
//! - Single byte: `LF`, `FF`, `HT`
//! - Two bytes: `ESC @`, `ESC E`, `ESC F`
//! - Multi-byte with parameters: `ESC d n`, `ESC k n1 n2 data...`
//!
//! ## Byte Order
//!
//! Multi-byte integers use **little-endian** encoding:
//! - `u16` value 0x1234 is sent as bytes `[0x34, 0x12]`
//!
//! ## Reference
//!
//! Based on "StarPRNT Command Specifications Rev. 4.10"
//! by Star Micronics Co., Ltd.

// ============================================================================
// ESCAPE SEQUENCE CONSTANTS
// ============================================================================

/// ESC (Escape) - Command prefix byte
///
/// Most StarPRNT commands begin with ESC (0x1B). This byte signals the start
/// of a control sequence rather than printable text.
pub const ESC: u8 = 0x1B;

/// GS (Group Separator) - Extended command prefix
///
/// Used in combination with ESC for extended commands:
/// - `ESC GS` prefix for graphics, status, and advanced features
/// - Hex: 0x1D, Decimal: 29
pub const GS: u8 = 0x1D;

/// RS (Record Separator) - Configuration command prefix
///
/// Used with ESC for printer configuration:
/// - `ESC RS` prefix for font selection, ASB settings
/// - Hex: 0x1E, Decimal: 30
pub const RS: u8 = 0x1E;

/// LF (Line Feed) - Print and advance one line
///
/// Prints any data in the line buffer and advances paper by the current
/// line spacing amount (default ~4mm for most fonts).
pub const LF: u8 = 0x0A;

/// FF (Form Feed) - Page eject in page mode
///
/// In standard mode: prints buffer and feeds to top of next page
/// In page mode: prints the composed page
pub const FF: u8 = 0x0C;

/// HT (Horizontal Tab) - Advance to next tab position
pub const HT: u8 = 0x09;

// ============================================================================
// INITIALIZATION COMMANDS
// ============================================================================

/// # Initialize Printer (ESC @)
///
/// Resets the printer to its power-on default state. This should be called
/// at the start of each print job to ensure consistent behavior.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC @ |
/// | Hex     | 1B 40 |
/// | Decimal | 27 64 |
///
/// ## What Gets Reset
///
/// - Print buffer is cleared
/// - Text formatting (bold, underline, invert) disabled
/// - Character size reset to 1x1
/// - Alignment reset to left
/// - Line spacing reset to default
/// - Tab positions cleared
///
/// ## What Does NOT Reset
///
/// - User-defined characters in RAM
/// - NV graphics stored in flash
/// - Macro definitions
/// - Configuration settings (density, ASB mode)
///
/// ## Example
///
/// ```
/// use estrella::protocol::commands;
///
/// let init = commands::init();
/// assert_eq!(init, vec![0x1B, 0x40]);
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.4.1
#[inline]
pub fn init() -> Vec<u8> {
    vec![ESC, b'@']
}

// ============================================================================
// CUTTER CONTROL COMMANDS
// ============================================================================

/// # Full Cut at Current Position (ESC d 0)
///
/// Performs a full cut at the current paper position without feeding.
/// If there is data in the line buffer, it is printed first.
///
/// ## Protocol Details
///
/// | Format  | Bytes    |
/// |---------|----------|
/// | ASCII   | ESC d 0  |
/// | Hex     | 1B 64 00 |
/// | Decimal | 27 100 0 |
///
/// ## Behavior
///
/// - Prints any pending data in line buffer
/// - Cuts paper at current position (may cut through printed content)
/// - Use `cut_full_feed()` to feed to cut position first
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.9
#[inline]
pub fn cut_full() -> Vec<u8> {
    vec![ESC, b'd', 0]
}

/// # Partial Cut at Current Position (ESC d 1)
///
/// Performs a partial cut (leaves small uncut portion) at current position.
///
/// ## Protocol Details
///
/// | Format  | Bytes    |
/// |---------|----------|
/// | ASCII   | ESC d 1  |
/// | Hex     | 1B 64 01 |
/// | Decimal | 27 100 1 |
///
/// ## Behavior
///
/// Partial cuts leave a small "hinge" connecting the receipt to the roll,
/// making it easy to tear off while preventing the receipt from falling.
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.9
#[inline]
pub fn cut_partial() -> Vec<u8> {
    vec![ESC, b'd', 1]
}

/// # Feed to Cut Position, Then Full Cut (ESC d 2)
///
/// Feeds paper forward to the cutter position, then performs a full cut.
/// This is the most commonly used cut command for receipts.
///
/// ## Protocol Details
///
/// | Format  | Bytes    |
/// |---------|----------|
/// | ASCII   | ESC d 2  |
/// | Hex     | 1B 64 02 |
/// | Decimal | 27 100 2 |
///
/// ## Behavior
///
/// 1. Prints any pending data in line buffer
/// 2. Feeds paper forward so last printed line is past the cutter
/// 3. Performs full cut
/// 4. Receipt falls into catch tray
///
/// ## Typical Use
///
/// ```
/// use estrella::protocol::commands;
///
/// // At end of receipt
/// let cut = commands::cut_full_feed();
/// // Send to printer...
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.9
#[inline]
pub fn cut_full_feed() -> Vec<u8> {
    vec![ESC, b'd', 2]
}

/// # Feed to Cut Position, Then Partial Cut (ESC d 3)
///
/// Feeds paper forward to the cutter position, then performs a partial cut.
///
/// ## Protocol Details
///
/// | Format  | Bytes    |
/// |---------|----------|
/// | ASCII   | ESC d 3  |
/// | Hex     | 1B 64 03 |
/// | Decimal | 27 100 3 |
///
/// ## Behavior
///
/// Same as `cut_full_feed()` but leaves a small uncut portion.
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.9
#[inline]
pub fn cut_partial_feed() -> Vec<u8> {
    vec![ESC, b'd', 3]
}

// ============================================================================
// PAPER FEED COMMANDS
// ============================================================================

/// # Micro Feed (ESC J n)
///
/// Feeds paper forward by n/4 millimeters (n dots at 203 DPI).
///
/// ## Protocol Details
///
/// | Format  | Bytes     |
/// |---------|-----------|
/// | ASCII   | ESC J n   |
/// | Hex     | 1B 4A n   |
/// | Decimal | 27 74 n   |
///
/// ## Parameters
///
/// - `n`: Feed amount in units of 1/4 mm (0-255)
///   - n=4 feeds 1mm
///   - n=12 feeds 3mm
///   - n=255 feeds ~63.75mm (maximum)
///
/// ## Resolution Note
///
/// At 203 DPI, one dot ≈ 0.125mm. The ESC J command uses 1/4mm units,
/// which equals approximately 2 dots per unit.
///
/// ## Example
///
/// ```
/// use estrella::protocol::commands;
///
/// // Feed 3mm
/// let feed = commands::feed_units(12);
/// assert_eq!(feed, vec![0x1B, 0x4A, 12]);
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.5
#[inline]
pub fn feed_units(n: u8) -> Vec<u8> {
    vec![ESC, b'J', n]
}

/// Feed paper by millimeters (convenience wrapper for `feed_units`)
///
/// Converts millimeters to the ESC J command's 1/4mm units.
///
/// ## Parameters
///
/// - `mm`: Feed amount in millimeters (0.0 - 63.75)
///
/// ## Example
///
/// ```
/// use estrella::protocol::commands;
///
/// // Feed 5mm
/// let feed = commands::feed_mm(5.0);
/// assert_eq!(feed, vec![0x1B, 0x4A, 20]); // 5mm * 4 = 20 units
/// ```
#[inline]
pub fn feed_mm(mm: f32) -> Vec<u8> {
    let units = (mm * 4.0).round().clamp(0.0, 255.0) as u8;
    feed_units(units)
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Encode a u16 value as little-endian bytes [low, high]
///
/// StarPRNT uses little-endian encoding for all multi-byte integers.
///
/// ## Example
///
/// ```
/// use estrella::protocol::commands::u16_le;
///
/// assert_eq!(u16_le(0x1234), [0x34, 0x12]);
/// assert_eq!(u16_le(576), [0x40, 0x02]); // 576 = 0x0240
/// ```
#[inline]
pub const fn u16_le(value: u16) -> [u8; 2] {
    [value as u8, (value >> 8) as u8]
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        assert_eq!(init(), vec![0x1B, 0x40]);
    }

    #[test]
    fn test_cut_full() {
        assert_eq!(cut_full(), vec![0x1B, 0x64, 0x00]);
    }

    #[test]
    fn test_cut_partial() {
        assert_eq!(cut_partial(), vec![0x1B, 0x64, 0x01]);
    }

    #[test]
    fn test_cut_full_feed() {
        assert_eq!(cut_full_feed(), vec![0x1B, 0x64, 0x02]);
    }

    #[test]
    fn test_cut_partial_feed() {
        assert_eq!(cut_partial_feed(), vec![0x1B, 0x64, 0x03]);
    }

    #[test]
    fn test_feed_units() {
        assert_eq!(feed_units(0), vec![0x1B, 0x4A, 0x00]);
        assert_eq!(feed_units(12), vec![0x1B, 0x4A, 0x0C]);
        assert_eq!(feed_units(255), vec![0x1B, 0x4A, 0xFF]);
    }

    #[test]
    fn test_feed_mm() {
        // 1mm = 4 units
        assert_eq!(feed_mm(1.0), vec![0x1B, 0x4A, 4]);
        // 3mm = 12 units
        assert_eq!(feed_mm(3.0), vec![0x1B, 0x4A, 12]);
        // 0.5mm = 2 units
        assert_eq!(feed_mm(0.5), vec![0x1B, 0x4A, 2]);
    }

    #[test]
    fn test_feed_mm_clamps() {
        // Should clamp to 255 max
        assert_eq!(feed_mm(100.0), vec![0x1B, 0x4A, 255]);
        // Should clamp to 0 min
        assert_eq!(feed_mm(-5.0), vec![0x1B, 0x4A, 0]);
    }

    #[test]
    fn test_u16_le() {
        assert_eq!(u16_le(0x0000), [0x00, 0x00]);
        assert_eq!(u16_le(0x00FF), [0xFF, 0x00]);
        assert_eq!(u16_le(0xFF00), [0x00, 0xFF]);
        assert_eq!(u16_le(0x1234), [0x34, 0x12]);
        assert_eq!(u16_le(576), [0x40, 0x02]); // Common width: 576 dots
    }
}
