//! # StarPRNT Text Styling Commands
//!
//! This module implements text formatting commands for Star Micronics printers.
//!
//! ## Text Styling Overview
//!
//! StarPRNT supports various text effects that can be combined:
//!
//! | Style | Command | Effect |
//! |-------|---------|--------|
//! | Bold | ESC E / ESC F | **Emphasized** text |
//! | Underline | ESC - n | Underlined text |
//! | Invert | ESC 4 / ESC 5 | White on black |
//! | Double Width | ESC W n | 2x horizontal size |
//! | Double Height | ESC h n | 2x vertical size |
//! | Upside Down | SI / DC2 | 180° rotation |
//!
//! ## Text Alignment
//!
//! ```text
//! Left aligned (default)    |LEFT TEXT
//! Center aligned            |  CENTER TEXT
//! Right aligned             |      RIGHT TEXT
//! ```
//!
//! ## Font Selection
//!
//! | Font | Size | Columns (72mm) |
//! |------|------|----------------|
//! | Font A | 12×24 dots | 48 chars |
//! | Font B | 9×24 dots | 64 chars |
//! | Font C | 9×17 dots | 64 chars |

use super::commands::{ESC, GS, RS};

// ============================================================================
// TEXT ALIGNMENT
// ============================================================================

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Left = 0,
    Center = 1,
    Right = 2,
}

/// # Set Text Alignment (ESC GS a n)
///
/// Sets the alignment for subsequent text lines.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC GS a n |
/// | Hex     | 1B 1D 61 n |
/// | Decimal | 27 29 97 n |
///
/// ## Parameters
///
/// - `n = 0`: Left alignment (default)
/// - `n = 1`: Center alignment
/// - `n = 2`: Right alignment
///
/// ## Behavior
///
/// - Affects all subsequent text until changed
/// - Takes effect at start of next line
/// - Reset by ESC @ (initialize)
///
/// ## Example
///
/// ```
/// use estrella::protocol::text::{align, Alignment};
///
/// let center = align(Alignment::Center);
/// assert_eq!(center, vec![0x1B, 0x1D, 0x61, 0x01]);
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.4
pub fn align(alignment: Alignment) -> Vec<u8> {
    vec![ESC, GS, b'a', alignment as u8]
}

/// Convenience function for left alignment
#[inline]
pub fn align_left() -> Vec<u8> {
    align(Alignment::Left)
}

/// Convenience function for center alignment
#[inline]
pub fn align_center() -> Vec<u8> {
    align(Alignment::Center)
}

/// Convenience function for right alignment
#[inline]
pub fn align_right() -> Vec<u8> {
    align(Alignment::Right)
}

// ============================================================================
// FONT SELECTION
// ============================================================================

/// Available fonts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Font {
    /// Font A: 12×24 dots, 48 columns on 72mm paper
    #[default]
    A = 0,
    /// Font B: 9×24 dots, 64 columns on 72mm paper
    B = 1,
    /// Font C: 9×17 dots, 64 columns on 72mm paper (shorter height)
    C = 2,
}

/// # Select Font (ESC RS F n)
///
/// Selects the character font for subsequent text.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC RS F n |
/// | Hex     | 1B 1E 46 n |
/// | Decimal | 27 30 70 n |
///
/// ## Font Specifications
///
/// | Font | Char Size | Columns (72mm) | Best For |
/// |------|-----------|----------------|----------|
/// | A | 12×24 dots | 48 | Headers, emphasis |
/// | B | 9×24 dots | 64 | Normal text |
/// | C | 9×17 dots | 64 | Fine print, compact |
///
/// ## Example
///
/// ```
/// use estrella::protocol::text::{font, Font};
///
/// let font_b = font(Font::B);
/// assert_eq!(font_b, vec![0x1B, 0x1E, 0x46, 0x01]);
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.1
pub fn font(f: Font) -> Vec<u8> {
    vec![ESC, RS, b'F', f as u8]
}

// ============================================================================
// TEXT EMPHASIS (BOLD)
// ============================================================================

/// # Enable Bold/Emphasis (ESC E)
///
/// Turns on emphasized (bold) printing for subsequent text.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC E |
/// | Hex     | 1B 45 |
/// | Decimal | 27 69 |
///
/// ## Effect
///
/// Text is printed with double-strike, appearing bolder/darker.
/// On thermal printers, this typically means more heat applied.
///
/// ## Example
///
/// ```
/// use estrella::protocol::text::{bold_on, bold_off};
///
/// let mut data = Vec::new();
/// data.extend(bold_on());
/// data.extend(b"IMPORTANT");
/// data.extend(bold_off());
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
#[inline]
pub fn bold_on() -> Vec<u8> {
    vec![ESC, b'E']
}

/// # Disable Bold/Emphasis (ESC F)
///
/// Turns off emphasized (bold) printing.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC F |
/// | Hex     | 1B 46 |
/// | Decimal | 27 70 |
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
#[inline]
pub fn bold_off() -> Vec<u8> {
    vec![ESC, b'F']
}

// ============================================================================
// UNDERLINE
// ============================================================================

/// # Set Underline Mode (ESC - n)
///
/// Enables or disables underline for subsequent text.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC - n |
/// | Hex     | 1B 2D n |
/// | Decimal | 27 45 n |
///
/// ## Parameters
///
/// - `n = 0`: Underline OFF
/// - `n = 1`: Underline ON (1 dot thick)
/// - `n = 2`: Underline ON (2 dots thick) - if supported
///
/// ## Note
///
/// Underline does not affect spaces or horizontal tabs.
///
/// ## Example
///
/// ```
/// use estrella::protocol::text::{underline_on, underline_off};
///
/// let mut data = Vec::new();
/// data.extend(underline_on());
/// data.extend(b"underlined text");
/// data.extend(underline_off());
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
#[inline]
pub fn underline_on() -> Vec<u8> {
    vec![ESC, b'-', 1]
}

/// Disable underline
#[inline]
pub fn underline_off() -> Vec<u8> {
    vec![ESC, b'-', 0]
}

// ============================================================================
// UPPERLINE (OVERLINE)
// ============================================================================

/// # Set Upperline/Overline Mode (ESC _ n)
///
/// Enables or disables upperline (overline) for subsequent text.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC _ n |
/// | Hex     | 1B 5F n |
/// | Decimal | 27 95 n |
///
/// ## Parameters
///
/// - `n = 0`: Upperline OFF
/// - `n = 1`: Upperline ON
///
/// ## Example
///
/// Combined with underline creates a "boxed" effect:
///
/// ```
/// use estrella::protocol::text::{underline_on, upperline_on, underline_off, upperline_off};
///
/// let mut data = Vec::new();
/// data.extend(underline_on());
/// data.extend(upperline_on());
/// data.extend(b"BOXED TEXT");
/// data.extend(underline_off());
/// data.extend(upperline_off());
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
#[inline]
pub fn upperline_on() -> Vec<u8> {
    vec![ESC, b'_', 1]
}

/// Disable upperline
#[inline]
pub fn upperline_off() -> Vec<u8> {
    vec![ESC, b'_', 0]
}

// ============================================================================
// INVERT (WHITE ON BLACK)
// ============================================================================

/// # Enable Inverted Printing (ESC 4)
///
/// Prints white text on a black background.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC 4 |
/// | Hex     | 1B 34 |
/// | Decimal | 27 52 |
///
/// ## Effect
///
/// ```text
/// Normal:   TEXT
/// Inverted: ████████
///           ░TEXT░░░
///           ████████
/// ```
///
/// ## Notes
///
/// - Uses more thermal paper (prints the background)
/// - Good for headers and emphasis
/// - May affect print speed
///
/// ## Example
///
/// ```
/// use estrella::protocol::text::{invert_on, invert_off};
///
/// let mut data = Vec::new();
/// data.extend(invert_on());
/// data.extend(b" SALE! ");
/// data.extend(invert_off());
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
#[inline]
pub fn invert_on() -> Vec<u8> {
    vec![ESC, b'4']
}

/// # Disable Inverted Printing (ESC 5)
///
/// Returns to normal black text on white background.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC 5 |
/// | Hex     | 1B 35 |
/// | Decimal | 27 53 |
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
#[inline]
pub fn invert_off() -> Vec<u8> {
    vec![ESC, b'5']
}

// ============================================================================
// CHARACTER SIZE
// ============================================================================

/// # Set Character Size (ESC i n1 n2)
///
/// Sets horizontal and vertical character expansion.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC i n1 n2 |
/// | Hex     | 1B 69 n1 n2 |
/// | Decimal | 27 105 n1 n2 |
///
/// ## Parameters
///
/// - `n1`: Vertical expansion (0-7 = 1x to 8x)
/// - `n2`: Horizontal expansion (0-7 = 1x to 8x)
///
/// ## Size Table
///
/// | Value | Multiplier |
/// |-------|------------|
/// | 0 | 1× (normal) |
/// | 1 | 2× |
/// | 2 | 3× |
/// | ... | ... |
/// | 7 | 8× |
///
/// ## Example
///
/// ```
/// use estrella::protocol::text::size;
///
/// // Double height and width (2x2)
/// let big = size(1, 1);
/// assert_eq!(big, vec![0x1B, 0x69, 0x01, 0x01]);
///
/// // Triple height, normal width
/// let tall = size(2, 0);
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
pub fn size(height_mult: u8, width_mult: u8) -> Vec<u8> {
    let h = height_mult.min(7);
    let w = width_mult.min(7);
    vec![ESC, b'i', h, w]
}

/// Reset to normal size (1x1)
#[inline]
pub fn size_normal() -> Vec<u8> {
    size(0, 0)
}

/// Double size (2x2)
#[inline]
pub fn size_double() -> Vec<u8> {
    size(1, 1)
}

/// # Double Width Mode (ESC W n)
///
/// Enables or disables double-width characters.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC W n |
/// | Hex     | 1B 57 n |
/// | Decimal | 27 87 n |
///
/// ## Parameters
///
/// - `n = 0`: Normal width
/// - `n = 1`: Double width
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
#[inline]
pub fn double_width_on() -> Vec<u8> {
    vec![ESC, b'W', 1]
}

#[inline]
pub fn double_width_off() -> Vec<u8> {
    vec![ESC, b'W', 0]
}

/// # Double Height Mode (ESC h n)
///
/// Enables or disables double-height characters.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC h n |
/// | Hex     | 1B 68 n |
/// | Decimal | 27 104 n |
///
/// ## Parameters
///
/// - `n = 0`: Normal height
/// - `n = 1`: Double height
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
#[inline]
pub fn double_height_on() -> Vec<u8> {
    vec![ESC, b'h', 1]
}

#[inline]
pub fn double_height_off() -> Vec<u8> {
    vec![ESC, b'h', 0]
}

// ============================================================================
// UPSIDE-DOWN MODE
// ============================================================================

/// SI (Shift In) control code for upside-down mode
const SI: u8 = 0x0F;

/// DC2 (Device Control 2) for canceling upside-down mode
const DC2: u8 = 0x12;

/// # Enable Upside-Down Mode (SI)
///
/// Prints subsequent text rotated 180 degrees.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | SI |
/// | Hex     | 0F |
/// | Decimal | 15 |
///
/// ## Effect
///
/// ```text
/// Normal:      HELLO
/// Upside-down: O˥˥ƎH (rotated 180°)
/// ```
///
/// ## Use Cases
///
/// - Customer copy that faces opposite direction
/// - Creative/artistic effects
/// - Tear-off stubs
///
/// ## Example
///
/// ```
/// use estrella::protocol::text::{upside_down_on, upside_down_off};
///
/// let mut data = Vec::new();
/// data.extend(upside_down_on());
/// data.extend(b"CUSTOMER COPY");
/// data.extend(upside_down_off());
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
#[inline]
pub fn upside_down_on() -> Vec<u8> {
    vec![SI]
}

/// # Disable Upside-Down Mode (DC2)
///
/// Returns to normal text orientation.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | DC2 |
/// | Hex     | 12 |
/// | Decimal | 18 |
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
#[inline]
pub fn upside_down_off() -> Vec<u8> {
    vec![DC2]
}

// ============================================================================
// REDUCED PRINTING
// ============================================================================

/// # Set Reduced Printing (ESC GS c h v)
///
/// Enables reduced (condensed) character printing.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC GS c h v |
/// | Hex     | 1B 1D 63 h v |
/// | Decimal | 27 29 99 h v |
///
/// ## Parameters
///
/// - `h`: Horizontal reduction
///   - 0 = Normal width (100%)
///   - 1 = Reduced width (~67%)
///
/// - `v`: Vertical reduction
///   - 0 = Normal height (100%)
///   - 1 = Reduced height (~50%)
///   - 2 = Reduced height (~75%)
///
/// ## Effect
///
/// ```text
/// Normal (0,0):     HELLO WORLD
/// Reduced (1,2):    HELLO WORLD  (smaller, condensed)
/// ```
///
/// ## Use Cases
///
/// - Fine print / legal text
/// - Fitting more text per line
/// - Receipts with dense information
///
/// ## Example
///
/// ```
/// use estrella::protocol::text::{reduced, reduced_off};
///
/// let mut data = Vec::new();
/// data.extend(reduced(1, 2)); // 67% width, 75% height
/// data.extend(b"Fine print text");
/// data.extend(reduced_off());
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
pub fn reduced(horizontal: u8, vertical: u8) -> Vec<u8> {
    let h = horizontal.min(1);
    let v = vertical.min(2);
    vec![ESC, GS, b'c', h, v]
}

/// Disable reduced printing (return to normal size)
#[inline]
pub fn reduced_off() -> Vec<u8> {
    reduced(0, 0)
}

// ============================================================================
// CODE PAGE SELECTION
// ============================================================================

/// Code page selection values
///
/// Common code pages for receipt printing. The TSP650II supports many more
/// code pages; see the StarPRNT Command Spec for the full list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CodePage {
    /// Code Page 437 (US English)
    Cp437 = 1,
    /// Katakana
    Katakana = 2,
    /// Code Page 858 (Multilingual + Euro)
    Cp858 = 3,
    /// Code Page 852 (Central European)
    Cp852 = 4,
    /// Code Page 860 (Portuguese)
    Cp860 = 5,
    /// Code Page 861 (Icelandic)
    Cp861 = 6,
    /// Code Page 863 (Canadian French)
    Cp863 = 7,
    /// Code Page 865 (Nordic)
    Cp865 = 8,
    /// Code Page 866 (Cyrillic)
    Cp866 = 9,
    /// Code Page 1252 (Windows Latin-1)
    Cp1252 = 32,
}

/// # Set Code Page (ESC GS t n)
///
/// Selects the international character code page.
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC GS t n |
/// | Hex     | 1B 1D 74 n |
/// | Decimal | 27 29 116 n |
///
/// ## Common Code Pages
///
/// | n | Code Page | Characters |
/// |---|-----------|------------|
/// | 1 | CP437 | US English (default) |
/// | 2 | Katakana | Japanese half-width |
/// | 3 | CP858 | Western European + Euro |
/// | 9 | CP866 | Cyrillic |
/// | 32 | CP1252 | Windows Latin-1 |
///
/// ## Example
///
/// ```
/// use estrella::protocol::text::{codepage, codepage_raw, CodePage};
///
/// // Set to Code Page 437 for standard ASCII + box drawing
/// let cmd = codepage(CodePage::Cp437);
/// assert_eq!(cmd, vec![0x1B, 0x1D, 0x74, 0x01]);
///
/// // Or use raw value for unsupported code pages
/// let cmd = codepage_raw(1);
/// assert_eq!(cmd, vec![0x1B, 0x1D, 0x74, 0x01]);
/// ```
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.2
pub fn codepage(cp: CodePage) -> Vec<u8> {
    vec![ESC, GS, b't', cp as u8]
}

/// Set code page using raw value
///
/// Use this for code pages not in the `CodePage` enum.
#[inline]
pub fn codepage_raw(n: u8) -> Vec<u8> {
    vec![ESC, GS, b't', n]
}

// ============================================================================
// SMOOTHING
// ============================================================================

/// # Set Smoothing Mode (ESC GS b n)
///
/// Enables or disables character smoothing (anti-aliasing).
///
/// ## Protocol Details
///
/// | Format  | Bytes |
/// |---------|-------|
/// | ASCII   | ESC GS b n |
/// | Hex     | 1B 1D 62 n |
/// | Decimal | 27 29 98 n |
///
/// ## Parameters
///
/// - `n = 0`: Smoothing OFF
/// - `n = 1`: Smoothing ON
///
/// ## Effect
///
/// When enabled, diagonal edges of enlarged characters are smoothed
/// to reduce jagged appearance. Most noticeable at 2x+ sizes.
///
/// ## Note
///
/// Smoothing is typically ON by default after ESC @.
///
/// ## Reference
///
/// StarPRNT Command Spec Rev 4.10, Section 2.3.3
#[inline]
pub fn smoothing_on() -> Vec<u8> {
    vec![ESC, GS, b'b', 1]
}

#[inline]
pub fn smoothing_off() -> Vec<u8> {
    vec![ESC, GS, b'b', 0]
}

// ============================================================================
// TEXT STYLE BUILDER
// ============================================================================

/// Builder for combining multiple text styles
///
/// ## Example
///
/// ```
/// use estrella::protocol::text::{TextStyle, Alignment, Font};
///
/// let style = TextStyle::new()
///     .alignment(Alignment::Center)
///     .bold(true)
///     .size(1, 1);
///
/// let commands = style.to_commands();
/// ```
#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    pub alignment: Option<Alignment>,
    pub font: Option<Font>,
    pub bold: Option<bool>,
    pub underline: Option<bool>,
    pub upperline: Option<bool>,
    pub invert: Option<bool>,
    pub height_mult: Option<u8>,
    pub width_mult: Option<u8>,
    pub upside_down: Option<bool>,
    pub smoothing: Option<bool>,
}

impl TextStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alignment(mut self, a: Alignment) -> Self {
        self.alignment = Some(a);
        self
    }

    pub fn font(mut self, f: Font) -> Self {
        self.font = Some(f);
        self
    }

    pub fn bold(mut self, enabled: bool) -> Self {
        self.bold = Some(enabled);
        self
    }

    pub fn underline(mut self, enabled: bool) -> Self {
        self.underline = Some(enabled);
        self
    }

    pub fn upperline(mut self, enabled: bool) -> Self {
        self.upperline = Some(enabled);
        self
    }

    pub fn invert(mut self, enabled: bool) -> Self {
        self.invert = Some(enabled);
        self
    }

    pub fn size(mut self, height: u8, width: u8) -> Self {
        self.height_mult = Some(height);
        self.width_mult = Some(width);
        self
    }

    pub fn upside_down(mut self, enabled: bool) -> Self {
        self.upside_down = Some(enabled);
        self
    }

    pub fn smoothing(mut self, enabled: bool) -> Self {
        self.smoothing = Some(enabled);
        self
    }

    /// Generate command bytes for this style
    pub fn to_commands(&self) -> Vec<u8> {
        let mut cmds = Vec::new();

        if let Some(a) = self.alignment {
            cmds.extend(align(a));
        }
        if let Some(f) = self.font {
            cmds.extend(font(f));
        }
        if let Some(b) = self.bold {
            cmds.extend(if b { bold_on() } else { bold_off() });
        }
        if let Some(u) = self.underline {
            cmds.extend(if u { underline_on() } else { underline_off() });
        }
        if let Some(u) = self.upperline {
            cmds.extend(if u { upperline_on() } else { upperline_off() });
        }
        if let Some(i) = self.invert {
            cmds.extend(if i { invert_on() } else { invert_off() });
        }
        if self.height_mult.is_some() || self.width_mult.is_some() {
            let h = self.height_mult.unwrap_or(0);
            let w = self.width_mult.unwrap_or(0);
            cmds.extend(size(h, w));
        }
        if let Some(u) = self.upside_down {
            cmds.extend(if u {
                upside_down_on()
            } else {
                upside_down_off()
            });
        }
        if let Some(s) = self.smoothing {
            cmds.extend(if s { smoothing_on() } else { smoothing_off() });
        }

        cmds
    }

    /// Reset all styles to default
    pub fn reset() -> Vec<u8> {
        let mut cmds = Vec::new();
        cmds.extend(align(Alignment::Left));
        cmds.extend(font(Font::A));
        cmds.extend(bold_off());
        cmds.extend(underline_off());
        cmds.extend(upperline_off());
        cmds.extend(invert_off());
        cmds.extend(size_normal());
        cmds.extend(upside_down_off());
        cmds.extend(smoothing_on());
        cmds
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align() {
        assert_eq!(align(Alignment::Left), vec![0x1B, 0x1D, 0x61, 0x00]);
        assert_eq!(align(Alignment::Center), vec![0x1B, 0x1D, 0x61, 0x01]);
        assert_eq!(align(Alignment::Right), vec![0x1B, 0x1D, 0x61, 0x02]);
    }

    #[test]
    fn test_font() {
        assert_eq!(font(Font::A), vec![0x1B, 0x1E, 0x46, 0x00]);
        assert_eq!(font(Font::B), vec![0x1B, 0x1E, 0x46, 0x01]);
        assert_eq!(font(Font::C), vec![0x1B, 0x1E, 0x46, 0x02]);
    }

    #[test]
    fn test_bold() {
        assert_eq!(bold_on(), vec![0x1B, 0x45]);
        assert_eq!(bold_off(), vec![0x1B, 0x46]);
    }

    #[test]
    fn test_underline() {
        assert_eq!(underline_on(), vec![0x1B, 0x2D, 0x01]);
        assert_eq!(underline_off(), vec![0x1B, 0x2D, 0x00]);
    }

    #[test]
    fn test_upperline() {
        assert_eq!(upperline_on(), vec![0x1B, 0x5F, 0x01]);
        assert_eq!(upperline_off(), vec![0x1B, 0x5F, 0x00]);
    }

    #[test]
    fn test_invert() {
        assert_eq!(invert_on(), vec![0x1B, 0x34]);
        assert_eq!(invert_off(), vec![0x1B, 0x35]);
    }

    #[test]
    fn test_size() {
        assert_eq!(size(0, 0), vec![0x1B, 0x69, 0x00, 0x00]);
        assert_eq!(size(1, 1), vec![0x1B, 0x69, 0x01, 0x01]);
        // Should clamp to max 7
        assert_eq!(size(10, 10), vec![0x1B, 0x69, 0x07, 0x07]);
    }

    #[test]
    fn test_double_width_height() {
        assert_eq!(double_width_on(), vec![0x1B, 0x57, 0x01]);
        assert_eq!(double_width_off(), vec![0x1B, 0x57, 0x00]);
        assert_eq!(double_height_on(), vec![0x1B, 0x68, 0x01]);
        assert_eq!(double_height_off(), vec![0x1B, 0x68, 0x00]);
    }

    #[test]
    fn test_upside_down() {
        assert_eq!(upside_down_on(), vec![0x0F]);
        assert_eq!(upside_down_off(), vec![0x12]);
    }

    #[test]
    fn test_smoothing() {
        assert_eq!(smoothing_on(), vec![0x1B, 0x1D, 0x62, 0x01]);
        assert_eq!(smoothing_off(), vec![0x1B, 0x1D, 0x62, 0x00]);
    }

    #[test]
    fn test_text_style_builder() {
        let style = TextStyle::new().alignment(Alignment::Center).bold(true);

        let cmds = style.to_commands();
        assert!(cmds.len() > 0);

        // Should contain alignment command
        assert!(cmds.windows(4).any(|w| w == [0x1B, 0x1D, 0x61, 0x01]));
        // Should contain bold on command
        assert!(cmds.windows(2).any(|w| w == [0x1B, 0x45]));
    }

    #[test]
    fn test_text_style_reset() {
        let reset = TextStyle::reset();
        // Should contain multiple commands
        assert!(reset.len() > 10);
    }

    #[test]
    fn test_reduced() {
        // Normal (off)
        assert_eq!(reduced(0, 0), vec![0x1B, 0x1D, 0x63, 0x00, 0x00]);
        // Horizontal 67%, vertical 75%
        assert_eq!(reduced(1, 2), vec![0x1B, 0x1D, 0x63, 0x01, 0x02]);
        // Test clamping
        assert_eq!(reduced(10, 10), vec![0x1B, 0x1D, 0x63, 0x01, 0x02]);
    }

    #[test]
    fn test_codepage() {
        assert_eq!(codepage(CodePage::Cp437), vec![0x1B, 0x1D, 0x74, 0x01]);
        assert_eq!(codepage(CodePage::Cp866), vec![0x1B, 0x1D, 0x74, 0x09]);
        assert_eq!(codepage(CodePage::Cp1252), vec![0x1B, 0x1D, 0x74, 0x20]);
    }

    #[test]
    fn test_codepage_raw() {
        assert_eq!(codepage_raw(1), vec![0x1B, 0x1D, 0x74, 0x01]);
        assert_eq!(codepage_raw(255), vec![0x1B, 0x1D, 0x74, 0xFF]);
    }
}
