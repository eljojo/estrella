//! # Printer Configuration
//!
//! This module defines hardware specifications for supported thermal printers.
//!
//! ## Supported Printers
//!
//! | Model | Width (dots) | Resolution | Band Height |
//! |-------|--------------|------------|-------------|
//! | TSP650II | 576 | 203 DPI | 24 rows |
//!
//! ## Usage
//!
//! ```
//! use estrella::printer::PrinterConfig;
//!
//! let config = PrinterConfig::TSP650II;
//! println!("Print width: {} dots ({} bytes)",
//!          config.width_dots,
//!          config.width_bytes);
//! ```

/// # Printer Configuration
///
/// Defines the hardware characteristics of a thermal printer.
///
/// ## Physical Properties
///
/// - **width_dots**: Maximum printable width in dots (pixels)
/// - **width_bytes**: Width in bytes (width_dots / 8)
/// - **dpi**: Resolution in dots per inch
/// - **band_height**: Height of one graphics band (ESC k command)
///
/// ## Bluetooth Tuning
///
/// - **max_chunk_rows**: Maximum rows per raster command over Bluetooth
///
/// ## Calculations
///
/// ```text
/// dots_per_mm = dpi / 25.4
/// width_mm = width_dots / dots_per_mm
///
/// For TSP650II:
///   dots_per_mm = 203 / 25.4 ≈ 8
///   width_mm = 576 / 8 = 72mm
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PrinterConfig {
    /// Printer model name
    pub name: &'static str,

    /// Maximum print width in dots (pixels)
    pub width_dots: u16,

    /// Print width in bytes (width_dots / 8)
    pub width_bytes: u16,

    /// Resolution in dots per inch
    pub dpi: u16,

    /// Band height for ESC k command (always 24 for StarPRNT)
    pub band_height: u16,

    /// Maximum rows per raster chunk (for Bluetooth buffer limits)
    pub max_chunk_rows: u16,
}

impl PrinterConfig {
    /// # Star TSP650II Configuration
    ///
    /// 80mm paper width thermal receipt printer.
    ///
    /// ## Specifications
    ///
    /// | Property | Value |
    /// |----------|-------|
    /// | Paper width | 80mm |
    /// | Print width | 72mm (576 dots) |
    /// | Resolution | 203 DPI |
    /// | Interface | Bluetooth/USB/Serial |
    /// | Cutter | Auto-cutter (full/partial) |
    ///
    /// ## Print Area
    ///
    /// ```text
    /// ├── 4mm ──┼────── 72mm printable ──────┼── 4mm ──┤
    /// │ margin  │         576 dots           │ margin  │
    /// ```
    pub const TSP650II: Self = Self {
        name: "Star TSP650II",
        width_dots: 576,
        width_bytes: 72,
        dpi: 203,
        band_height: 24,
        max_chunk_rows: 256,
    };

    /// Calculate dots per millimeter
    ///
    /// ## Example
    ///
    /// ```
    /// use estrella::printer::PrinterConfig;
    ///
    /// let config = PrinterConfig::TSP650II;
    /// assert!((config.dots_per_mm() - 8.0).abs() < 0.1);
    /// ```
    #[inline]
    pub fn dots_per_mm(&self) -> f32 {
        self.dpi as f32 / 25.4
    }

    /// Calculate print width in millimeters
    #[inline]
    pub fn width_mm(&self) -> f32 {
        self.width_dots as f32 / self.dots_per_mm()
    }

    /// Convert millimeters to dots
    #[inline]
    pub fn mm_to_dots(&self, mm: f32) -> u16 {
        (mm * self.dots_per_mm()).round() as u16
    }

    /// Convert dots to millimeters
    #[inline]
    pub fn dots_to_mm(&self, dots: u16) -> f32 {
        dots as f32 / self.dots_per_mm()
    }
}

impl Default for PrinterConfig {
    fn default() -> Self {
        Self::TSP650II
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tsp650ii_dimensions() {
        let config = PrinterConfig::TSP650II;
        assert_eq!(config.width_dots, 576);
        assert_eq!(config.width_bytes, 72);
        assert_eq!(config.width_dots, config.width_bytes * 8);
    }

    #[test]
    fn test_dots_per_mm() {
        let config = PrinterConfig::TSP650II;
        let dpmm = config.dots_per_mm();
        // 203 DPI ≈ 8 dots/mm
        assert!((dpmm - 8.0).abs() < 0.1);
    }

    #[test]
    fn test_width_mm() {
        let config = PrinterConfig::TSP650II;
        let width = config.width_mm();
        // 576 dots / 8 dpmm = 72mm
        assert!((width - 72.0).abs() < 1.0);
    }

    #[test]
    fn test_mm_to_dots() {
        let config = PrinterConfig::TSP650II;
        // 10mm ≈ 80 dots
        let dots = config.mm_to_dots(10.0);
        assert!((dots as i32 - 80).abs() < 2);
    }

    #[test]
    fn test_dots_to_mm() {
        let config = PrinterConfig::TSP650II;
        // 80 dots ≈ 10mm
        let mm = config.dots_to_mm(80);
        assert!((mm - 10.0).abs() < 0.5);
    }

    #[test]
    fn test_default_is_tsp650ii() {
        let default = PrinterConfig::default();
        assert_eq!(default.name, PrinterConfig::TSP650II.name);
    }
}
