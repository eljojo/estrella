//! # NV Graphics Commands
//!
//! Non-volatile graphics storage for Star Micronics printers.
//!
//! NV (Non-Volatile) graphics are stored in the printer's flash memory
//! and persist across power cycles. They're ideal for logos and other
//! frequently-used images.
//!
//! ## Key Codes
//!
//! Each NV graphic is identified by a 2-character key (e.g., "A0", "LG").
//! Key characters must be printable ASCII (32-126).
//!
//! ## Functions
//!
//! | Function | Purpose |
//! |----------|---------|
//! | 65 | Erase all NV graphics |
//! | 66 | Erase specified NV graphic |
//! | 67 | Define (store) NV graphic |
//! | 69 | Print NV graphic |
//!
//! ## Reference
//!
//! StarPRNT Command Spec Rev 4.10, Sections 2.3.13 (Functions 65-69)

use super::commands::{ESC, GS, u16_le};

/// Validate a 2-character NV graphics key.
///
/// Keys must be exactly 2 printable ASCII characters (32-126).
///
/// # Returns
///
/// `Some((kc1, kc2))` if valid, `None` otherwise.
pub fn validate_key(key: &str) -> Option<(u8, u8)> {
    let bytes: Vec<u8> = key.bytes().collect();
    if bytes.len() != 2 {
        return None;
    }
    let kc1 = bytes[0];
    let kc2 = bytes[1];
    if (32..=126).contains(&kc1) && (32..=126).contains(&kc2) {
        Some((kc1, kc2))
    } else {
        None
    }
}

// ============================================================================
// FUNCTION 65: ERASE ALL NV GRAPHICS
// ============================================================================

/// # Erase All NV Graphics (Function 65)
///
/// Erases all NV graphics stored in the printer's flash memory.
///
/// **WARNING:** This erases ALL stored graphics, including those used by
/// other applications. Use with caution.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC GS ( L pL pH m fn d1 d2 d3 |
/// | Hex     | 1B 1D 28 4C 05 00 30 41 43 4C 52 |
///
/// ## Parameters
///
/// - pL=5, pH=0 (p=5, length of m+fn+d1+d2+d3)
/// - m=48 (0x30)
/// - fn=65 (0x41)
/// - d1=67 ('C'), d2=76 ('L'), d3=82 ('R') - "CLR"
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.13, Function 65
#[inline]
pub fn erase_all() -> Vec<u8> {
    vec![
        ESC, GS, b'(', b'L', // Command prefix
        5, 0, // pL=5, pH=0
        48, 65, // m=48, fn=65
        67, 76, 82, // d1='C', d2='L', d3='R'
    ]
}

// ============================================================================
// FUNCTION 66: ERASE SPECIFIED NV GRAPHIC
// ============================================================================

/// # Erase Specified NV Graphic (Function 66)
///
/// Erases the NV graphic stored with the given key.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC GS ( L pL pH m fn kc1 kc2 |
/// | Hex     | 1B 1D 28 4C 04 00 30 42 kc1 kc2 |
///
/// ## Parameters
///
/// - pL=4, pH=0 (p=4, length of m+fn+kc1+kc2)
/// - m=48 (0x30)
/// - fn=66 (0x42)
/// - kc1, kc2: Key characters (32-126)
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.13, Function 66
pub fn erase(key: &str) -> Option<Vec<u8>> {
    let (kc1, kc2) = validate_key(key)?;
    Some(vec![
        ESC, GS, b'(', b'L', // Command prefix
        4, 0, // pL=4, pH=0
        48, 66, // m=48, fn=66
        kc1, kc2, // Key characters
    ])
}

// ============================================================================
// FUNCTION 67: DEFINE (STORE) NV GRAPHIC
// ============================================================================

/// # Define NV Graphic (Function 67)
///
/// Stores a raster image in the printer's non-volatile memory.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC GS ( L pL pH m fn a kc1 kc2 b xL xH yL yH c d1...dk |
///
/// ## Parameters
///
/// - pL, pH: Length of everything after (m through data)
/// - m=48 (0x30)
/// - fn=67 (0x43)
/// - a=48 (raster format)
/// - kc1, kc2: Key characters
/// - b=1 (number of colors, always 1 for monochrome)
/// - xL, xH: Width in dots (little-endian)
/// - yL, yH: Height in dots (little-endian)
/// - c=49 (black)
/// - d1...dk: Raster data (k = ceil(width/8) * height)
///
/// ## Data Format
///
/// Same as raster graphics: 1 bit per pixel, MSB is leftmost,
/// 1 = black (print), 0 = white.
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.13, Function 67
pub fn define(key: &str, width: u16, height: u16, data: &[u8]) -> Option<Vec<u8>> {
    let (kc1, kc2) = validate_key(key)?;

    let bytes_per_row = width.div_ceil(8);
    let expected_len = bytes_per_row as usize * height as usize;

    if data.len() != expected_len {
        return None;
    }

    // Build the payload: a kc1 kc2 b xL xH yL yH c data...
    let [xl, xh] = u16_le(width);
    let [yl, yh] = u16_le(height);

    let payload_len = 9 + data.len(); // a + kc1 + kc2 + b + xL + xH + yL + yH + c + data
    let body_len = 2 + payload_len; // m + fn + payload

    if body_len > 65535 {
        return None;
    }

    let [pl, ph] = u16_le(body_len as u16);

    let mut cmd = Vec::with_capacity(6 + body_len);

    // Header: ESC GS ( L pL pH
    cmd.extend([ESC, GS, b'(', b'L', pl, ph]);

    // Body: m fn
    cmd.extend([48, 67]); // m=48, fn=67

    // Payload: a kc1 kc2 b xL xH yL yH c
    cmd.extend([48, kc1, kc2, 1, xl, xh, yl, yh, 49]);

    // Raster data
    cmd.extend_from_slice(data);

    Some(cmd)
}

// ============================================================================
// FUNCTION 69: PRINT NV GRAPHIC
// ============================================================================

/// # Print NV Graphic (Function 69)
///
/// Prints a previously stored NV graphic.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC GS ( L pL pH m fn kc1 kc2 x y |
/// | Hex     | 1B 1D 28 4C 06 00 30 45 kc1 kc2 x y |
///
/// ## Parameters
///
/// - pL=6, pH=0 (p=6, length of m+fn+kc1+kc2+x+y)
/// - m=48 (0x30)
/// - fn=69 (0x45)
/// - kc1, kc2: Key characters (32-126)
/// - x: Horizontal scale (1 = 1x, 2 = 2x)
/// - y: Vertical scale (1 = 1x, 2 = 2x)
///
/// ## Notes
///
/// - Must be called at the beginning of a line (empty line buffer)
/// - The graphic prints immediately and advances paper
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.13, Function 69
pub fn print(key: &str, scale_x: u8, scale_y: u8) -> Option<Vec<u8>> {
    let (kc1, kc2) = validate_key(key)?;

    // Validate scale (must be 1 or 2)
    if !(1..=2).contains(&scale_x) || !(1..=2).contains(&scale_y) {
        return None;
    }

    Some(vec![
        ESC, GS, b'(', b'L', // Command prefix
        6, 0, // pL=6, pH=0
        48, 69, // m=48, fn=69
        kc1, kc2, // Key characters
        scale_x, scale_y,
    ])
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_key_valid() {
        assert_eq!(validate_key("A0"), Some((b'A', b'0')));
        assert_eq!(validate_key("LG"), Some((b'L', b'G')));
        assert_eq!(validate_key("  "), Some((b' ', b' '))); // Space is valid (32)
        assert_eq!(validate_key("~~"), Some((b'~', b'~'))); // Tilde is valid (126)
    }

    #[test]
    fn test_validate_key_invalid() {
        assert_eq!(validate_key("A"), None); // Too short
        assert_eq!(validate_key("ABC"), None); // Too long
        assert_eq!(validate_key(""), None); // Empty
    }

    #[test]
    fn test_erase_all() {
        let cmd = erase_all();
        assert_eq!(cmd, vec![0x1B, 0x1D, 0x28, 0x4C, 5, 0, 48, 65, 67, 76, 82]);
    }

    #[test]
    fn test_erase() {
        let cmd = erase("A0").unwrap();
        assert_eq!(cmd, vec![0x1B, 0x1D, 0x28, 0x4C, 4, 0, 48, 66, b'A', b'0']);
    }

    #[test]
    fn test_erase_invalid_key() {
        assert!(erase("A").is_none());
        assert!(erase("ABC").is_none());
    }

    #[test]
    fn test_define() {
        // 8x2 image (1 byte wide, 2 rows)
        let data = vec![0xFF, 0xAA];
        let cmd = define("A0", 8, 2, &data).unwrap();

        // Header: ESC GS ( L pL pH
        assert_eq!(&cmd[0..4], &[0x1B, 0x1D, 0x28, 0x4C]);

        // pL pH = 13 (2 + 9 + 2 bytes of data)
        assert_eq!(&cmd[4..6], &[13, 0]);

        // m=48, fn=67
        assert_eq!(&cmd[6..8], &[48, 67]);

        // a=48, kc1='A', kc2='0', b=1
        assert_eq!(&cmd[8..12], &[48, b'A', b'0', 1]);

        // xL xH = 8, yL yH = 2
        assert_eq!(&cmd[12..16], &[8, 0, 2, 0]);

        // c=49
        assert_eq!(cmd[16], 49);

        // data
        assert_eq!(&cmd[17..], &[0xFF, 0xAA]);
    }

    #[test]
    fn test_define_wrong_data_length() {
        let data = vec![0xFF]; // Wrong length for 8x2
        assert!(define("A0", 8, 2, &data).is_none());
    }

    #[test]
    fn test_print() {
        let cmd = print("A0", 1, 1).unwrap();
        assert_eq!(
            cmd,
            vec![0x1B, 0x1D, 0x28, 0x4C, 6, 0, 48, 69, b'A', b'0', 1, 1]
        );
    }

    #[test]
    fn test_print_scaled() {
        let cmd = print("LG", 2, 2).unwrap();
        assert_eq!(
            cmd,
            vec![0x1B, 0x1D, 0x28, 0x4C, 6, 0, 48, 69, b'L', b'G', 2, 2]
        );
    }

    #[test]
    fn test_print_invalid_scale() {
        assert!(print("A0", 0, 1).is_none());
        assert!(print("A0", 3, 1).is_none());
        assert!(print("A0", 1, 0).is_none());
        assert!(print("A0", 1, 3).is_none());
    }

    #[test]
    fn test_print_invalid_key() {
        assert!(print("A", 1, 1).is_none());
    }
}
