//! # StarPRNT Barcode Commands
//!
//! This module implements barcode generation commands for Star Micronics printers.
//!
//! ## Supported Barcode Types
//!
//! | Type | Description | Density |
//! |------|-------------|---------|
//! | QR Code | 2D matrix barcode | High capacity |
//! | PDF417 | 2D stacked barcode | High capacity |
//!
//! ## QR Code Usage
//!
//! QR codes are generated in a multi-step process:
//!
//! 1. Configure QR settings (model, error correction, cell size)
//! 2. Send data to encode
//! 3. Print the barcode
//!
//! ```
//! use estrella::protocol::barcode::qr;
//!
//! let mut data = Vec::new();
//!
//! // Configure QR code
//! data.extend(qr::set_model(qr::QrModel::Model2));
//! data.extend(qr::set_error_correction(qr::QrErrorLevel::M));
//! data.extend(qr::set_cell_size(4));
//!
//! // Set data and print
//! data.extend(qr::set_data(b"https://example.com"));
//! data.extend(qr::print());
//! ```
//!
//! ## PDF417 Usage
//!
//! ```
//! use estrella::protocol::barcode::pdf417;
//!
//! let mut data = Vec::new();
//!
//! // Configure PDF417
//! data.extend(pdf417::set_ecc_level(2));
//! data.extend(pdf417::set_module_width(3));
//!
//! // Set data and print
//! data.extend(pdf417::set_data(b"Hello, PDF417!"));
//! data.extend(pdf417::print());
//! ```
//!
//! ## 1D Barcode Usage
//!
//! ```
//! use estrella::protocol::barcode::barcode1d;
//!
//! let mut data = Vec::new();
//!
//! // Print a Code39 barcode with human-readable text
//! data.extend(barcode1d::code39(b"HELLO123", 80));
//! ```
//!
//! ## Protocol Reference
//!
//! Based on "StarPRNT Command Specifications Rev. 4.10", Sections 2.3.14-2.3.16.

use super::commands::{ESC, GS, RS};

// ============================================================================
// 1D BARCODE COMMANDS (ESC b)
// ============================================================================

/// 1D Barcode command builders
///
/// Prints traditional linear barcodes like Code39, Code128, UPC, etc.
/// These barcodes encode data in varying width bars and spaces.
pub mod barcode1d {
    use super::{ESC, RS};

    /// 1D Barcode type codes
    ///
    /// Each barcode type has different character sets and capacities.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    pub enum BarcodeType {
        /// UPC-E (6 digits, compressed UPC-A)
        UpcE = 48,
        /// UPC-A (12 digits)
        UpcA = 49,
        /// EAN-8 / JAN-8 (8 digits)
        Ean8 = 50,
        /// EAN-13 / JAN-13 (13 digits)
        Ean13 = 51,
        /// Code39 (A-Z, 0-9, space, -.$/%+)
        Code39 = 52,
        /// ITF (Interleaved 2 of 5, numeric pairs)
        Itf = 53,
        /// Code128 (full ASCII)
        Code128 = 54,
        /// Code93 (full ASCII, more compact than Code39)
        Code93 = 55,
        /// NW-7 / Codabar
        Nw7 = 56,
    }

    /// HRI (Human Readable Interpretation) position
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum HriPosition {
        /// No HRI text printed
        None = 0,
        /// HRI above barcode
        Above = 1,
        /// HRI below barcode (default)
        #[default]
        Below = 2,
        /// HRI both above and below
        Both = 3,
    }

    /// HRI font selection
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum HriFont {
        /// Font A (12×24 dots)
        #[default]
        FontA = 0,
        /// Font B (9×24 dots)
        FontB = 1,
    }

    /// Barcode module (bar) width
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum ModuleWidth {
        /// 2 dots minimum (narrowest)
        Dots2 = 1,
        /// 3 dots minimum (default)
        #[default]
        Dots3 = 2,
        /// 4 dots minimum (wider)
        Dots4 = 3,
    }

    /// Build n2 parameter from HRI options
    ///
    /// n2 encodes: font selection, HRI position, and line feed behavior.
    /// According to StarPRNT Command Spec Rev 4.10, Section 2.3.14 (page 80),
    /// n2 is a lookup table value, not a bitfield:
    ///
    /// | n2     | Font   | Position | Line feed    |
    /// |--------|--------|----------|--------------|
    /// | 1, 49  | ---    | None     | Execute      |
    /// | 2, 50  | Font A | Under    | Execute      |
    /// | 3, 51  | ---    | None     | Not execute  |
    /// | 4, 52  | Font A | Under    | Not execute  |
    ///
    /// Note: Values 1-4 and 49-52 ('1'-'4') are equivalent.
    fn build_n2(hri_pos: HriPosition, hri_font: HriFont, execute_linefeed: bool) -> u8 {
        // Use ASCII digit values (48+) for clarity
        match (hri_pos, hri_font, execute_linefeed) {
            (HriPosition::None, _, true) => 49,  // '1': No HRI, execute LF
            (HriPosition::None, _, false) => 51, // '3': No HRI, no LF
            (HriPosition::Below, HriFont::FontA, true) => 50,  // '2': Font A, under, execute LF
            (HriPosition::Below, HriFont::FontA, false) => 52, // '4': Font A, under, no LF
            (HriPosition::Above, HriFont::FontA, true) => 50,  // Fallback to under position
            (HriPosition::Above, HriFont::FontA, false) => 52,
            (HriPosition::Both, HriFont::FontA, true) => 50,
            (HriPosition::Both, HriFont::FontA, false) => 52,
            // Font B options - use Font A as fallback (spec doesn't list Font B for Code39)
            (HriPosition::Below, HriFont::FontB, true) => 50,
            (HriPosition::Below, HriFont::FontB, false) => 52,
            (HriPosition::Above, HriFont::FontB, true) => 50,
            (HriPosition::Above, HriFont::FontB, false) => 52,
            (HriPosition::Both, HriFont::FontB, true) => 50,
            (HriPosition::Both, HriFont::FontB, false) => 52,
        }
    }

    /// # Print 1D Barcode (ESC b n1 n2 n3 n4 data RS)
    ///
    /// Prints a 1D linear barcode.
    ///
    /// ## Protocol Details
    ///
    /// | Format  | Bytes |
    /// |---------|-------|
    /// | ASCII   | ESC b n1 n2 n3 n4 data RS |
    /// | Hex     | 1B 62 n1 n2 n3 n4 data 1E |
    /// | Decimal | 27 98 n1 n2 n3 n4 data 30 |
    ///
    /// ## Parameters
    ///
    /// - `n1`: Barcode type (see `BarcodeType`)
    /// - `n2`: HRI options (position, font, line feed)
    /// - `n3`: Mode (module width, etc.)
    /// - `n4`: Height in dots (1-255)
    /// - `data`: Barcode content
    /// - `RS`: Terminator (0x1E)
    ///
    /// ## Reference
    ///
    /// StarPRNT Command Spec Rev 4.10, Section 2.3.14
    pub fn barcode(
        barcode_type: BarcodeType,
        data: &[u8],
        height: u8,
        hri_pos: HriPosition,
        hri_font: HriFont,
        module_width: ModuleWidth,
    ) -> Vec<u8> {
        let n1 = barcode_type as u8;
        let n2 = build_n2(hri_pos, hri_font, true); // Execute line feed after printing
        let n3 = 48 + module_width as u8; // Mode byte: 48 + width
        let n4 = height.max(1);

        let mut cmd = Vec::with_capacity(6 + data.len() + 1);
        cmd.push(ESC);
        cmd.push(b'b');
        cmd.push(n1);
        cmd.push(n2);
        cmd.push(n3);
        cmd.push(n4);
        cmd.extend_from_slice(data);
        cmd.push(RS);
        cmd
    }

    /// # Print Code39 Barcode
    ///
    /// Code39 is a common alphanumeric barcode supporting:
    /// - Characters: A-Z, 0-9, space, - . $ / % +
    /// - Self-checking (no checksum required)
    ///
    /// ## Example
    ///
    /// ```
    /// use estrella::protocol::barcode::barcode1d;
    ///
    /// // Simple Code39 with default settings
    /// let cmd = barcode1d::code39(b"HELLO-123", 80);
    ///
    /// // With custom options
    /// let cmd = barcode1d::code39_with_options(
    ///     b"TEST",
    ///     80,
    ///     barcode1d::HriPosition::Below,
    ///     barcode1d::HriFont::FontA,
    ///     barcode1d::ModuleWidth::Dots3,
    /// );
    /// ```
    pub fn code39(data: &[u8], height: u8) -> Vec<u8> {
        barcode(
            BarcodeType::Code39,
            data,
            height,
            HriPosition::Below,
            HriFont::FontA,
            ModuleWidth::Dots2,
        )
    }

    /// Print Code39 barcode with custom options
    pub fn code39_with_options(
        data: &[u8],
        height: u8,
        hri_pos: HriPosition,
        hri_font: HriFont,
        module_width: ModuleWidth,
    ) -> Vec<u8> {
        barcode(BarcodeType::Code39, data, height, hri_pos, hri_font, module_width)
    }

    /// # Print Code128 Barcode
    ///
    /// Code128 is a high-density barcode supporting full ASCII.
    /// More compact than Code39 for the same data.
    ///
    /// ## Character Sets
    ///
    /// - Code A: ASCII 0-95 + control characters
    /// - Code B: ASCII 32-127 (default)
    /// - Code C: Numeric pairs (00-99)
    ///
    /// ## Example
    ///
    /// ```
    /// use estrella::protocol::barcode::barcode1d;
    ///
    /// let cmd = barcode1d::code128(b"Hello World 123", 80);
    /// ```
    pub fn code128(data: &[u8], height: u8) -> Vec<u8> {
        barcode(
            BarcodeType::Code128,
            data,
            height,
            HriPosition::Below,
            HriFont::FontA,
            ModuleWidth::Dots2,
        )
    }

    /// Print Code128 barcode with custom options
    pub fn code128_with_options(
        data: &[u8],
        height: u8,
        hri_pos: HriPosition,
        hri_font: HriFont,
        module_width: ModuleWidth,
    ) -> Vec<u8> {
        barcode(BarcodeType::Code128, data, height, hri_pos, hri_font, module_width)
    }

    /// # Print EAN-13 Barcode
    ///
    /// EAN-13 is the standard retail barcode (13 digits).
    /// Also known as JAN-13 in Japan.
    ///
    /// ## Example
    ///
    /// ```
    /// use estrella::protocol::barcode::barcode1d;
    ///
    /// // 12 digits (checksum auto-calculated) or 13 digits
    /// let cmd = barcode1d::ean13(b"5901234123457", 80);
    /// ```
    pub fn ean13(data: &[u8], height: u8) -> Vec<u8> {
        barcode(
            BarcodeType::Ean13,
            data,
            height,
            HriPosition::Below,
            HriFont::FontA,
            ModuleWidth::Dots3,
        )
    }

    /// # Print UPC-A Barcode
    ///
    /// UPC-A is the standard US retail barcode (12 digits).
    ///
    /// ## Example
    ///
    /// ```
    /// use estrella::protocol::barcode::barcode1d;
    ///
    /// let cmd = barcode1d::upca(b"012345678905", 80);
    /// ```
    pub fn upca(data: &[u8], height: u8) -> Vec<u8> {
        barcode(
            BarcodeType::UpcA,
            data,
            height,
            HriPosition::Below,
            HriFont::FontA,
            ModuleWidth::Dots3,
        )
    }

    /// # Print ITF (Interleaved 2 of 5) Barcode
    ///
    /// ITF is a numeric-only barcode that encodes digit pairs.
    /// Data length must be even.
    ///
    /// ## Example
    ///
    /// ```
    /// use estrella::protocol::barcode::barcode1d;
    ///
    /// let cmd = barcode1d::itf(b"12345678", 80);
    /// ```
    pub fn itf(data: &[u8], height: u8) -> Vec<u8> {
        barcode(
            BarcodeType::Itf,
            data,
            height,
            HriPosition::Below,
            HriFont::FontA,
            ModuleWidth::Dots3,
        )
    }
}

// ============================================================================
// QR CODE COMMANDS
// ============================================================================

/// QR Code command builders
///
/// QR codes encode data in a 2D matrix pattern that can be read by smartphones
/// and barcode scanners. They support error correction, allowing partial damage
/// recovery.
pub mod qr {
    use super::{ESC, GS};

    /// QR Code model selection
    ///
    /// ## Model Comparison
    ///
    /// | Model | Max Version | Max Data (L) | Features |
    /// |-------|-------------|--------------|----------|
    /// | Model 1 | 14 | ~1167 chars | Original QR |
    /// | Model 2 | 40 | ~7089 chars | Enhanced, with alignment patterns |
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum QrModel {
        /// Original QR Code (version 1-14)
        Model1 = 1,
        /// Enhanced QR Code (version 1-40, recommended)
        #[default]
        Model2 = 2,
    }

    /// QR Code error correction level
    ///
    /// Higher levels allow more damage recovery but reduce data capacity.
    ///
    /// ## Error Correction Levels
    ///
    /// | Level | Recovery | Best For |
    /// |-------|----------|----------|
    /// | L | ~7% | Clean environments |
    /// | M | ~15% | General use (default) |
    /// | Q | ~25% | Industrial use |
    /// | H | ~30% | Harsh environments |
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum QrErrorLevel {
        /// Level L: ~7% error recovery
        L = 0,
        /// Level M: ~15% error recovery (default, recommended)
        #[default]
        M = 1,
        /// Level Q: ~25% error recovery
        Q = 2,
        /// Level H: ~30% error recovery
        H = 3,
    }

    /// # Set QR Code Model (ESC GS y S 0 n)
    ///
    /// Selects QR Code model (Model 1 or Model 2).
    ///
    /// ## Protocol Details
    ///
    /// | Format  | Bytes |
    /// |---------|-------|
    /// | ASCII   | ESC GS y S 0 n |
    /// | Hex     | 1B 1D 79 53 30 n |
    /// | Decimal | 27 29 121 83 48 n |
    ///
    /// ## Parameters
    ///
    /// - `n = 1`: Model 1 (original, versions 1-14)
    /// - `n = 2`: Model 2 (enhanced, versions 1-40, with alignment patterns)
    ///
    /// ## Notes
    ///
    /// - Model 2 is recommended for most applications
    /// - Model 2 has alignment patterns that improve reading accuracy
    /// - Settings persist until ESC @ (init) or power off
    ///
    /// ## Reference
    ///
    /// StarPRNT Command Spec Rev 4.10, Section 2.3.15
    pub fn set_model(model: QrModel) -> Vec<u8> {
        vec![ESC, GS, b'y', b'S', b'0', model as u8]
    }

    /// # Set QR Error Correction Level (ESC GS y S 1 n)
    ///
    /// Sets the error correction level for QR code generation.
    ///
    /// ## Protocol Details
    ///
    /// | Format  | Bytes |
    /// |---------|-------|
    /// | ASCII   | ESC GS y S 1 n |
    /// | Hex     | 1B 1D 79 53 31 n |
    /// | Decimal | 27 29 121 83 49 n |
    ///
    /// ## Error Recovery Capacity
    ///
    /// | Level | n | Recovery | Overhead |
    /// |-------|---|----------|----------|
    /// | L | 0 | ~7% | Lowest |
    /// | M | 1 | ~15% | Default |
    /// | Q | 2 | ~25% | Higher |
    /// | H | 3 | ~30% | Highest |
    ///
    /// ## Trade-offs
    ///
    /// - Higher correction = larger QR code for same data
    /// - Higher correction = better readability when damaged/dirty
    /// - Level M is a good balance for receipts
    ///
    /// ## Reference
    ///
    /// StarPRNT Command Spec Rev 4.10, Section 2.3.15
    pub fn set_error_correction(level: QrErrorLevel) -> Vec<u8> {
        vec![ESC, GS, b'y', b'S', b'1', level as u8]
    }

    /// # Set QR Cell Size (ESC GS y S 2 n)
    ///
    /// Sets the size of each cell (module) in the QR code.
    ///
    /// ## Protocol Details
    ///
    /// | Format  | Bytes |
    /// |---------|-------|
    /// | ASCII   | ESC GS y S 2 n |
    /// | Hex     | 1B 1D 79 53 32 n |
    /// | Decimal | 27 29 121 83 50 n |
    ///
    /// ## Parameters
    ///
    /// - `n`: Cell size in dots (1-8)
    ///
    /// ## Size Guide
    ///
    /// | n | Cell Size | Typical Use |
    /// |---|-----------|-------------|
    /// | 1-2 | Very small | High-density, good scanners |
    /// | 3-4 | Small | General receipt use |
    /// | 5-6 | Medium | Easy scanning |
    /// | 7-8 | Large | Low-res cameras, far distance |
    ///
    /// ## Notes
    ///
    /// - Final QR size = (version * 4 + 17) * cell_size
    /// - Larger cells = easier scanning but larger print area
    /// - For receipts, 3-4 dots is typically good
    ///
    /// ## Reference
    ///
    /// StarPRNT Command Spec Rev 4.10, Section 2.3.15
    pub fn set_cell_size(size: u8) -> Vec<u8> {
        let size = size.clamp(1, 8);
        vec![ESC, GS, b'y', b'S', b'2', size]
    }

    /// # Set QR Code Data (ESC GS y D 1 m nL nH data)
    ///
    /// Sets the data to encode in the QR code (automatic mode analysis).
    ///
    /// ## Protocol Details
    ///
    /// | Format  | Bytes |
    /// |---------|-------|
    /// | ASCII   | ESC GS y D 1 m nL nH data... |
    /// | Hex     | 1B 1D 79 44 31 00 nL nH data... |
    /// | Decimal | 27 29 121 68 49 0 nL nH data... |
    ///
    /// ## Parameters
    ///
    /// - `m`: Data mode (0 = AUTO, 1 = MANUAL)
    ///   - AUTO (m=0): Printer auto-detects optimal encoding
    ///   - MANUAL (m=1): Force specific encoding mode
    /// - `nL, nH`: Data length as little-endian 16-bit (len = nL + nH * 256)
    /// - `data`: Bytes to encode
    ///
    /// This function uses AUTO mode (m=0) for automatic data type detection.
    ///
    /// ## Data Types (Auto-Detected)
    ///
    /// | Type | Characters | Efficiency |
    /// |------|------------|------------|
    /// | Numeric | 0-9 | 3.3 chars/codeword |
    /// | Alphanumeric | 0-9, A-Z, space, $%*+-./: | 2 chars/codeword |
    /// | Binary | Any byte | 1 char/codeword |
    /// | Kanji | Japanese characters | 1 char/codeword |
    ///
    /// ## Example
    ///
    /// ```
    /// use estrella::protocol::barcode::qr;
    ///
    /// // Encode a URL
    /// let data = qr::set_data(b"https://example.com");
    /// // Command: ESC GS y D 1 m nL nH data
    /// assert_eq!(data[0..6], [0x1B, 0x1D, 0x79, 0x44, 0x31, 0x00]);
    /// ```
    ///
    /// ## Reference
    ///
    /// StarPRNT Command Spec Rev 4.10, Section 2.3.15
    pub fn set_data(data: &[u8]) -> Vec<u8> {
        let len = data.len().min(u16::MAX as usize) as u16;
        let nl = (len & 0xFF) as u8;
        let nh = ((len >> 8) & 0xFF) as u8;

        // m = 0 for AUTO mode (automatic data type analysis)
        let mut cmd = vec![ESC, GS, b'y', b'D', b'1', 0, nl, nh];
        cmd.extend_from_slice(data);
        cmd
    }

    /// # Print QR Code (ESC GS y P)
    ///
    /// Prints the QR code with the current settings and data.
    ///
    /// ## Protocol Details
    ///
    /// | Format  | Bytes |
    /// |---------|-------|
    /// | ASCII   | ESC GS y P |
    /// | Hex     | 1B 1D 79 50 |
    /// | Decimal | 27 29 121 80 |
    ///
    /// ## Notes
    ///
    /// - Must call set_data() before print()
    /// - If settings are invalid, this command is ignored
    /// - QR is printed at current alignment position
    ///
    /// ## Reference
    ///
    /// StarPRNT Command Spec Rev 4.10, Section 2.3.15
    pub fn print() -> Vec<u8> {
        vec![ESC, GS, b'y', b'P']
    }

    /// Generate a complete QR code command sequence
    ///
    /// This is a convenience function that generates all commands needed
    /// to print a QR code with the specified settings.
    ///
    /// ## Parameters
    ///
    /// - `data`: Data to encode
    /// - `cell_size`: Cell size in dots (1-8)
    /// - `error_level`: Error correction level
    ///
    /// ## Example
    ///
    /// ```
    /// use estrella::protocol::barcode::qr::{self, QrErrorLevel};
    ///
    /// let commands = qr::generate(b"Hello World", 4, QrErrorLevel::M);
    /// // Ready to send to printer
    /// ```
    pub fn generate(data: &[u8], cell_size: u8, error_level: QrErrorLevel) -> Vec<u8> {
        let mut cmd = Vec::new();
        cmd.extend(set_model(QrModel::Model2));
        cmd.extend(set_error_correction(error_level));
        cmd.extend(set_cell_size(cell_size));
        cmd.extend(set_data(data));
        cmd.extend(print());
        cmd
    }
}

// ============================================================================
// PDF417 COMMANDS
// ============================================================================

/// PDF417 barcode command builders
///
/// PDF417 is a stacked 2D barcode that can encode large amounts of data.
/// It's commonly used for shipping labels, ID cards, and documents.
pub mod pdf417 {
    use super::{ESC, GS};

    /// # Set PDF417 Size (ESC GS x S 0 n p1 p2)
    ///
    /// Sets the bar code size using ratio or fixed dimensions.
    ///
    /// ## Protocol Details
    ///
    /// | Format  | Bytes |
    /// |---------|-------|
    /// | ASCII   | ESC GS x S 0 n p1 p2 |
    /// | Hex     | 1B 1D 78 53 30 n p1 p2 |
    /// | Decimal | 27 29 120 83 48 n p1 p2 |
    ///
    /// ## Mode n=0 (USE_LIMITS): Aspect Ratio
    ///
    /// - `p1`: Vertical proportion (1-99)
    /// - `p2`: Horizontal proportion (1-99)
    /// - Ratio: p1/p2 must be between 0.01 and 10
    ///
    /// ## Mode n=1 (USE_FIXED): Fixed Dimensions
    ///
    /// - `p1`: Number of rows (0, or 3-90)
    /// - `p2`: Number of columns (0, or 1-30)
    /// - Constraint: p1 * p2 <= 928
    /// - Value 0 means "auto-calculate"
    ///
    /// ## Reference
    ///
    /// StarPRNT Command Spec Rev 4.10, Section 2.3.16
    pub fn set_size_ratio(vertical: u8, horizontal: u8) -> Vec<u8> {
        let v = vertical.clamp(1, 99);
        let h = horizontal.clamp(1, 99);
        vec![ESC, GS, b'x', b'S', b'0', 0, v, h]
    }

    /// Set PDF417 size using fixed rows and columns
    ///
    /// ## Parameters
    ///
    /// - `rows`: Number of rows (0 for auto, or 3-90)
    /// - `columns`: Number of columns (0 for auto, or 1-30)
    pub fn set_size_fixed(rows: u8, columns: u8) -> Vec<u8> {
        let r = if rows == 0 { 0 } else { rows.clamp(3, 90) };
        let c = if columns == 0 { 0 } else { columns.clamp(1, 30) };
        vec![ESC, GS, b'x', b'S', b'0', 1, r, c]
    }

    /// # Set PDF417 ECC Level (ESC GS x S 1 n)
    ///
    /// Sets the error correction level (security level).
    ///
    /// ## Protocol Details
    ///
    /// | Format  | Bytes |
    /// |---------|-------|
    /// | ASCII   | ESC GS x S 1 n |
    /// | Hex     | 1B 1D 78 53 31 n |
    /// | Decimal | 27 29 120 83 49 n |
    ///
    /// ## Parameters
    ///
    /// - `n`: ECC level (0-8)
    ///
    /// ## ECC Level Details
    ///
    /// | Level | Error Codewords | Recovery |
    /// |-------|-----------------|----------|
    /// | 0 | 2 | Minimal |
    /// | 1 | 4 | Low |
    /// | 2 | 8 | Default |
    /// | 3 | 16 | Medium |
    /// | 4 | 32 | High |
    /// | 5 | 64 | Higher |
    /// | 6 | 128 | Very High |
    /// | 7 | 256 | Maximum |
    /// | 8 | 512 | Extreme |
    ///
    /// ## Notes
    ///
    /// - Higher levels increase barcode size
    /// - Level 2 is a good default for receipts
    /// - Default value: 1
    ///
    /// ## Reference
    ///
    /// StarPRNT Command Spec Rev 4.10, Section 2.3.16
    pub fn set_ecc_level(level: u8) -> Vec<u8> {
        let level = level.min(8);
        vec![ESC, GS, b'x', b'S', b'1', level]
    }

    /// # Set PDF417 Module Width (ESC GS x S 2 n)
    ///
    /// Sets the width of each module (bar) in the X direction.
    ///
    /// ## Protocol Details
    ///
    /// | Format  | Bytes |
    /// |---------|-------|
    /// | ASCII   | ESC GS x S 2 n |
    /// | Hex     | 1B 1D 78 53 32 n |
    /// | Decimal | 27 29 120 83 50 n |
    ///
    /// ## Parameters
    ///
    /// - `n`: Module width in dots (1-10 or 1-15 depending on printer)
    ///
    /// ## Notes
    ///
    /// - Recommended: n >= 2 for reliable scanning
    /// - Larger values = easier scanning but larger print
    /// - Default value: 2
    ///
    /// ## Reference
    ///
    /// StarPRNT Command Spec Rev 4.10, Section 2.3.16
    pub fn set_module_width(width: u8) -> Vec<u8> {
        let width = width.clamp(1, 15);
        vec![ESC, GS, b'x', b'S', b'2', width]
    }

    /// # Set PDF417 Module Aspect Ratio (ESC GS x S 3 n)
    ///
    /// Sets the height-to-width aspect ratio of each module.
    ///
    /// ## Protocol Details
    ///
    /// | Format  | Bytes |
    /// |---------|-------|
    /// | ASCII   | ESC GS x S 3 n |
    /// | Hex     | 1B 1D 78 53 33 n |
    /// | Decimal | 27 29 120 83 51 n |
    ///
    /// ## Parameters
    ///
    /// - `n`: Aspect ratio multiplier (1-10)
    ///   - Module height = module_width * n
    ///
    /// ## Notes
    ///
    /// - Default value: 3
    /// - Higher values make taller, narrower bars
    /// - Typical values: 2-4 for receipts
    ///
    /// ## Reference
    ///
    /// StarPRNT Command Spec Rev 4.10, Section 2.3.16
    pub fn set_module_aspect(aspect: u8) -> Vec<u8> {
        let aspect = aspect.clamp(1, 10);
        vec![ESC, GS, b'x', b'S', b'3', aspect]
    }

    /// # Set PDF417 Data (ESC GS x D nL nH data)
    ///
    /// Sets the data to encode in the PDF417 barcode.
    ///
    /// ## Protocol Details
    ///
    /// | Format  | Bytes |
    /// |---------|-------|
    /// | ASCII   | ESC GS x D nL nH data... |
    /// | Hex     | 1B 1D 78 44 nL nH data... |
    /// | Decimal | 27 29 120 68 nL nH data... |
    ///
    /// ## Parameters
    ///
    /// - `nL, nH`: Data length as little-endian 16-bit
    /// - `data`: Bytes to encode
    ///
    /// ## Reference
    ///
    /// StarPRNT Command Spec Rev 4.10, Section 2.3.16
    pub fn set_data(data: &[u8]) -> Vec<u8> {
        let len = data.len().min(u16::MAX as usize) as u16;
        let nl = (len & 0xFF) as u8;
        let nh = ((len >> 8) & 0xFF) as u8;

        let mut cmd = vec![ESC, GS, b'x', b'D', nl, nh];
        cmd.extend_from_slice(data);
        cmd
    }

    /// # Print PDF417 (ESC GS x P)
    ///
    /// Prints the PDF417 barcode with current settings and data.
    ///
    /// ## Protocol Details
    ///
    /// | Format  | Bytes |
    /// |---------|-------|
    /// | ASCII   | ESC GS x P |
    /// | Hex     | 1B 1D 78 50 |
    /// | Decimal | 27 29 120 80 |
    ///
    /// ## Notes
    ///
    /// - Must call set_data() before print()
    /// - If settings are invalid, this command is ignored
    ///
    /// ## Reference
    ///
    /// StarPRNT Command Spec Rev 4.10, Section 2.3.16
    pub fn print() -> Vec<u8> {
        vec![ESC, GS, b'x', b'P']
    }

    /// Generate a complete PDF417 command sequence
    ///
    /// Convenience function to generate all commands needed for a PDF417 barcode.
    ///
    /// ## Parameters
    ///
    /// - `data`: Data to encode
    /// - `module_width`: Module width in dots (1-15)
    /// - `ecc_level`: Error correction level (0-8)
    pub fn generate(data: &[u8], module_width: u8, ecc_level: u8) -> Vec<u8> {
        let mut cmd = Vec::new();
        cmd.extend(set_ecc_level(ecc_level));
        cmd.extend(set_module_width(module_width));
        cmd.extend(set_module_aspect(3)); // Default aspect ratio per spec
        cmd.extend(set_data(data));
        cmd.extend(print());
        cmd
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod barcode1d_tests {
        use super::barcode1d::*;

        #[test]
        fn test_code39_header() {
            let cmd = code39(b"TEST", 80);
            assert_eq!(cmd[0], 0x1B); // ESC
            assert_eq!(cmd[1], b'b'); // b
            assert_eq!(cmd[2], 52); // n1 = Code39
            // n2=50 ('2'): Font A, Under position, Execute line feed (spec page 80)
            assert_eq!(cmd[3], 50);
            assert_eq!(cmd[4], 49); // n3 = 48 + 1 (Dots2)
            assert_eq!(cmd[5], 80); // n4 = height
            assert_eq!(&cmd[6..10], b"TEST"); // data
            assert_eq!(cmd[10], 0x1E); // RS terminator
        }

        #[test]
        fn test_code39_total_length() {
            let data = b"HELLO-123";
            let cmd = code39(data, 100);
            // 6 header bytes + data length + 1 RS terminator
            assert_eq!(cmd.len(), 6 + data.len() + 1);
        }

        #[test]
        fn test_code128() {
            let cmd = code128(b"Hello", 80);
            assert_eq!(cmd[0], 0x1B);
            assert_eq!(cmd[1], b'b');
            assert_eq!(cmd[2], 54); // Code128
            assert_eq!(cmd[5], 80); // height
            assert_eq!(*cmd.last().unwrap(), 0x1E); // RS
        }

        #[test]
        fn test_ean13() {
            let cmd = ean13(b"5901234123457", 80);
            assert_eq!(cmd[2], 51); // EAN13
            assert_eq!(cmd.len(), 6 + 13 + 1);
        }

        #[test]
        fn test_upca() {
            let cmd = upca(b"012345678905", 80);
            assert_eq!(cmd[2], 49); // UPC-A
        }

        #[test]
        fn test_itf() {
            let cmd = itf(b"12345678", 80);
            assert_eq!(cmd[2], 53); // ITF
        }

        #[test]
        fn test_hri_options() {
            // Test HRI below with Font A (spec-compliant combination)
            let cmd = code39_with_options(
                b"TEST",
                80,
                HriPosition::Below,
                HriFont::FontA,
                ModuleWidth::Dots3,
            );
            // n2=50 ('2'): Font A, Under position, Execute line feed (spec page 80)
            assert_eq!(cmd[3], 50);
            // n3 = 48 + 2 (Dots3) = 50
            assert_eq!(cmd[4], 50);
        }

        #[test]
        fn test_hri_none() {
            let cmd = code39_with_options(
                b"TEST",
                80,
                HriPosition::None,
                HriFont::FontA,
                ModuleWidth::Dots2,
            );
            // n2=49 ('1'): No HRI, Execute line feed (spec page 80)
            assert_eq!(cmd[3], 49);
        }

        #[test]
        fn test_height_minimum() {
            let cmd = code39(b"TEST", 0);
            // Height should be clamped to minimum 1
            assert_eq!(cmd[5], 1);
        }
    }

    mod qr_tests {
        use super::qr::*;

        #[test]
        fn test_set_model() {
            assert_eq!(
                set_model(QrModel::Model1),
                vec![0x1B, 0x1D, 0x79, 0x53, 0x30, 0x01]
            );
            assert_eq!(
                set_model(QrModel::Model2),
                vec![0x1B, 0x1D, 0x79, 0x53, 0x30, 0x02]
            );
        }

        #[test]
        fn test_set_error_correction() {
            assert_eq!(
                set_error_correction(QrErrorLevel::L),
                vec![0x1B, 0x1D, 0x79, 0x53, 0x31, 0x00]
            );
            assert_eq!(
                set_error_correction(QrErrorLevel::M),
                vec![0x1B, 0x1D, 0x79, 0x53, 0x31, 0x01]
            );
            assert_eq!(
                set_error_correction(QrErrorLevel::Q),
                vec![0x1B, 0x1D, 0x79, 0x53, 0x31, 0x02]
            );
            assert_eq!(
                set_error_correction(QrErrorLevel::H),
                vec![0x1B, 0x1D, 0x79, 0x53, 0x31, 0x03]
            );
        }

        #[test]
        fn test_set_cell_size() {
            assert_eq!(
                set_cell_size(4),
                vec![0x1B, 0x1D, 0x79, 0x53, 0x32, 0x04]
            );
            // Test clamping
            assert_eq!(
                set_cell_size(0),
                vec![0x1B, 0x1D, 0x79, 0x53, 0x32, 0x01]
            ); // Min is 1
            assert_eq!(
                set_cell_size(20),
                vec![0x1B, 0x1D, 0x79, 0x53, 0x32, 0x08]
            ); // Max is 8
        }

        #[test]
        fn test_set_data() {
            let data = set_data(b"Hello");
            // ESC GS y D 1 m nL nH data
            assert_eq!(data[0..6], [0x1B, 0x1D, 0x79, 0x44, 0x31, 0x00]); // m=0 (AUTO)
            assert_eq!(data[6], 5); // Length low byte
            assert_eq!(data[7], 0); // Length high byte
            assert_eq!(&data[8..], b"Hello");
        }

        #[test]
        fn test_print() {
            assert_eq!(print(), vec![0x1B, 0x1D, 0x79, 0x50]);
        }

        #[test]
        fn test_generate() {
            let cmd = generate(b"Test", 4, QrErrorLevel::M);
            // Should contain model, error correction, cell size, data, print
            assert!(cmd.len() > 20);
            // Should end with print command
            assert!(cmd.ends_with(&[0x1B, 0x1D, 0x79, 0x50]));
        }
    }

    mod pdf417_tests {
        use super::pdf417::*;

        #[test]
        fn test_set_size_ratio() {
            let cmd = set_size_ratio(1, 2);
            assert_eq!(cmd, vec![0x1B, 0x1D, 0x78, 0x53, 0x30, 0x00, 0x01, 0x02]);
        }

        #[test]
        fn test_set_size_fixed() {
            let cmd = set_size_fixed(10, 5);
            assert_eq!(cmd, vec![0x1B, 0x1D, 0x78, 0x53, 0x30, 0x01, 0x0A, 0x05]);

            // Test auto values (0)
            let auto_cmd = set_size_fixed(0, 0);
            assert_eq!(
                auto_cmd,
                vec![0x1B, 0x1D, 0x78, 0x53, 0x30, 0x01, 0x00, 0x00]
            );
        }

        #[test]
        fn test_set_ecc_level() {
            assert_eq!(
                set_ecc_level(2),
                vec![0x1B, 0x1D, 0x78, 0x53, 0x31, 0x02]
            );
            // Test clamping
            assert_eq!(
                set_ecc_level(100),
                vec![0x1B, 0x1D, 0x78, 0x53, 0x31, 0x08]
            ); // Max is 8
        }

        #[test]
        fn test_set_module_width() {
            assert_eq!(
                set_module_width(3),
                vec![0x1B, 0x1D, 0x78, 0x53, 0x32, 0x03]
            );
            // Test clamping
            assert_eq!(
                set_module_width(0),
                vec![0x1B, 0x1D, 0x78, 0x53, 0x32, 0x01]
            ); // Min is 1
        }

        #[test]
        fn test_set_module_aspect() {
            assert_eq!(
                set_module_aspect(3),
                vec![0x1B, 0x1D, 0x78, 0x53, 0x33, 0x03]
            );
            // Test clamping
            assert_eq!(
                set_module_aspect(0),
                vec![0x1B, 0x1D, 0x78, 0x53, 0x33, 0x01]
            ); // Min is 1
            assert_eq!(
                set_module_aspect(20),
                vec![0x1B, 0x1D, 0x78, 0x53, 0x33, 0x0A]
            ); // Max is 10
        }

        #[test]
        fn test_set_data() {
            let data = set_data(b"Test123");
            assert_eq!(data[0..4], [0x1B, 0x1D, 0x78, 0x44]);
            assert_eq!(data[4], 7); // Length low byte
            assert_eq!(data[5], 0); // Length high byte
            assert_eq!(&data[6..], b"Test123");
        }

        #[test]
        fn test_print() {
            assert_eq!(print(), vec![0x1B, 0x1D, 0x78, 0x50]);
        }

        #[test]
        fn test_generate() {
            let cmd = generate(b"Test", 3, 2);
            // Should contain ecc, module width, aspect ratio, data, print
            assert!(cmd.len() > 20);
            // Should end with print command
            assert!(cmd.ends_with(&[0x1B, 0x1D, 0x78, 0x50]));
        }
    }
}
