//! # Page Mode Commands
//!
//! Page mode allows composing a print area with absolute X,Y positioning
//! before printing. Unlike standard mode where each line prints immediately,
//! page mode accumulates content and prints when explicitly commanded.
//!
//! ## Workflow
//!
//! 1. Enter page mode (`page_mode_enter`)
//! 2. Define print region (`page_mode_set_region`)
//! 3. Set print direction (`page_mode_set_direction`)
//! 4. Position and add content:
//!    - `page_mode_set_position_x` / `page_mode_set_position_y`
//!    - Print text or graphics
//! 5. Print and exit (`page_mode_print_and_exit`)
//!
//! ## Reference
//!
//! StarPRNT Command Spec Rev 4.10, Section "ESC GS P" (Page Mode)

use super::commands::{ESC, GS};

/// Page mode direction and origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PageDirection {
    /// Left to right, starting top-left (normal reading direction)
    LeftToRightTopLeft = 0,
    /// Bottom to top, starting bottom-left (90° counter-clockwise)
    BottomToTopBottomLeft = 1,
    /// Right to left, starting bottom-right (180° rotation)
    RightToLeftBottomRight = 2,
    /// Top to bottom, starting top-right (90° clockwise)
    TopToBottomTopRight = 3,
}

/// Enter page mode.
///
/// **Command:** ESC GS P 0
///
/// Must be called at beginning of line with no print data in buffer.
/// After entering page mode, set the print region and direction before adding content.
pub fn page_mode_enter() -> Vec<u8> {
    vec![ESC, GS, b'P', 0]
}

/// Set page mode print region.
///
/// **Command:** ESC GS P 3 xL xH yL yH dxL dxH dyL dyH
///
/// ## Parameters (all in 1/8 mm units)
///
/// - `x`, `y`: Origin position (typically 0, 0)
/// - `width`, `height`: Canvas dimensions
///
/// For TSP650II (72mm printable width):
/// - width = 72mm × 8 = 576 units
/// - height = depends on content (e.g., 100mm × 8 = 800 units)
pub fn page_mode_set_region(x: u16, y: u16, width: u16, height: u16) -> Vec<u8> {
    let [xl, xh] = x.to_le_bytes();
    let [yl, yh] = y.to_le_bytes();
    let [dxl, dxh] = width.to_le_bytes();
    let [dyl, dyh] = height.to_le_bytes();

    vec![ESC, GS, b'P', 3, xl, xh, yl, yh, dxl, dxh, dyl, dyh]
}

/// Set page mode print direction and origin.
///
/// **Command:** ESC GS P 2 n
///
/// Determines text flow direction and starting point for content positioning.
pub fn page_mode_set_direction(dir: PageDirection) -> Vec<u8> {
    vec![ESC, GS, b'P', 2, dir as u8]
}

/// Set absolute Y position in page mode.
///
/// **Command:** ESC GS P 4 nL nH
///
/// ## Parameters
///
/// - `pos_eighth_mm`: Y position in 1/8 mm units
///
/// Use this to position content vertically within the page region.
pub fn page_mode_set_position_y(pos_eighth_mm: u16) -> Vec<u8> {
    let [nl, nh] = pos_eighth_mm.to_le_bytes();
    vec![ESC, GS, b'P', 4, nl, nh]
}

/// Print page mode content and return to standard mode.
///
/// **Command:** ESC GS P 7
///
/// This prints all accumulated content at once and exits page mode.
/// Printer returns to standard mode after printing.
pub fn page_mode_print_and_exit() -> Vec<u8> {
    vec![ESC, GS, b'P', 7]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_mode_enter() {
        let cmd = page_mode_enter();
        assert_eq!(cmd, vec![0x1B, 0x1D, b'P', 0]);
    }

    #[test]
    fn test_page_mode_set_region() {
        // 72mm × 100mm region at origin
        let cmd = page_mode_set_region(0, 0, 576, 800);
        assert_eq!(
            cmd,
            vec![
                0x1B, 0x1D, b'P', 3,
                0, 0,      // x = 0
                0, 0,      // y = 0
                64, 2,     // width = 576 (0x0240)
                32, 3,     // height = 800 (0x0320)
            ]
        );
    }

    #[test]
    fn test_page_mode_direction() {
        let cmd = page_mode_set_direction(PageDirection::LeftToRightTopLeft);
        assert_eq!(cmd, vec![0x1B, 0x1D, b'P', 2, 0]);
    }

    #[test]
    fn test_page_mode_set_position_y() {
        let cmd = page_mode_set_position_y(160); // 20mm
        assert_eq!(cmd, vec![0x1B, 0x1D, b'P', 4, 160, 0]);
    }

    #[test]
    fn test_page_mode_print() {
        let cmd = page_mode_print_and_exit();
        assert_eq!(cmd, vec![0x1B, 0x1D, b'P', 7]);
    }
}
