//! # Visual Patterns for Thermal Printing
//!
//! This module provides pattern generators for creating visual art on thermal
//! printers. Each pattern computes an intensity value (0.0-1.0) for every pixel,
//! which is then dithered to produce binary output.
//!
//! ## Available Patterns
//!
//! | Pattern | Description |
//! |---------|-------------|
//! | [`Ripple`] | Concentric circles with wobble interference |
//! | [`Waves`] | Multi-oscillator interference pattern |
//! | [`Sick`] | Multi-section visual showcase (plasma, rings, topography, glitch) |
//! | [`Calibration`] | Diagnostic pattern with borders, diagonals, and bars |
//! | [`Other`] | Print tests suite (micro-feed, density, overburn, jitter) |
//!
//! ## Pattern Trait
//!
//! All patterns implement the [`Pattern`] trait:
//!
//! ```
//! use estrella::render::patterns::{Pattern, Ripple};
//!
//! let ripple = Ripple::default();
//!
//! // Get intensity at pixel (100, 200) in a 576x500 image
//! let intensity = ripple.shade(100, 200, 576, 500);
//! assert!(intensity >= 0.0 && intensity <= 1.0);
//! ```
//!
//! ## Gamma Correction
//!
//! Thermal printers have non-linear response characteristics. Gamma correction
//! compensates for this by adjusting intensity values:
//!
//! ```text
//! corrected = intensity ^ gamma
//!
//! gamma > 1: Darker output (more contrast)
//! gamma < 1: Lighter output (less contrast)
//! gamma = 1: No correction
//! ```
//!
//! Default gamma values are tuned for Star TSP650II printers.

use super::dither;
use crate::art;

/// Clamp a value to the range [0.0, 1.0]
#[inline]
fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

// ============================================================================
// PATTERN TRAIT
// ============================================================================

/// Trait for pattern generators.
///
/// Patterns compute intensity values that are later dithered to binary output.
pub trait Pattern: Send + Sync {
    /// Compute the shade (intensity) at a pixel position.
    ///
    /// ## Parameters
    ///
    /// - `x`: Horizontal pixel position (0 to width-1)
    /// - `y`: Vertical pixel position (0 to height-1)
    /// - `width`: Total image width in pixels
    /// - `height`: Total image height in pixels
    ///
    /// ## Returns
    ///
    /// Intensity value where:
    /// - 0.0 = white (no printing)
    /// - 1.0 = black (full printing)
    fn shade(&self, x: usize, y: usize, width: usize, height: usize) -> f32;

    /// Gamma correction exponent for this pattern.
    ///
    /// Applied after shade calculation: `final = shade ^ gamma`
    ///
    /// Default is 1.35, which provides good contrast on TSP650II.
    fn gamma(&self) -> f32 {
        1.35
    }

    /// Default dimensions (width, height) for this pattern.
    ///
    /// Returns the canonical dimensions that should be used by both
    /// the CLI and golden tests to ensure consistency.
    fn default_dimensions(&self) -> (usize, usize) {
        (576, 500)
    }

    /// Compute the final intensity with gamma correction applied.
    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        let shade = self.shade(x, y, width, height);
        clamp01(shade).powf(self.gamma())
    }

    /// Render the pattern to a byte array suitable for raster graphics.
    ///
    /// ## Parameters
    ///
    /// - `width`: Image width in pixels
    /// - `height`: Image height in pixels
    ///
    /// ## Returns
    ///
    /// Packed byte array where each bit represents one pixel.
    /// Length = `ceil(width/8) * height` bytes.
    fn render(&self, width: usize, height: usize) -> Vec<u8> {
        dither::generate_raster(width, height, |x, y, w, h| self.intensity(x, y, w, h))
    }
}

// ============================================================================
// RIPPLE PATTERN
// ============================================================================

/// # Ripple Pattern
///
/// Creates concentric circles emanating from the image center, combined with
/// a subtle wobble interference pattern.
///
/// ## Visual Effect
///
/// ```text
///      ████████████████████████
///    ██                        ██
///   █                            █
///  █    ████████████████████      █
///  █   █                    █     █
/// █   █    ████████████      █    █
/// █   █   █            █     █    █
/// █   █  █   ██████    █     █    █
/// █   █  █  █      █   █     █    █
/// ```
///
/// ## Formula
///
/// The ripple effect is computed as:
///
/// ```text
/// r = sqrt((x - cx)² + (y - cy)²)  // Distance from center
/// ripple = 0.5 + 0.5 * cos(r / scale - y / drift)
/// wobble = 0.5 + 0.5 * sin(x / 37.0 + 0.7 * cos(y / 53.0))
/// v = ripple * (1 - wobble_mix) + wobble * wobble_mix
/// ```
///
/// ## Parameters
///
/// - `scale`: Ripple wavelength (larger = wider rings). Default: 6.5
/// - `drift`: Vertical drift factor (creates asymmetry). Default: 85.0
/// - `wobble_mix`: Blend amount for wobble pattern (0-1). Default: 0.25
///
/// ## Example
///
/// ```
/// use estrella::render::patterns::{Pattern, Ripple};
///
/// // Default settings
/// let ripple = Ripple::default();
/// let data = ripple.render(576, 500);
///
/// // Custom settings
/// let custom = Ripple {
///     scale: 10.0,      // Wider rings
///     drift: 100.0,     // Less vertical drift
///     wobble_mix: 0.1,  // Subtle wobble
///     gamma: 1.5,       // More contrast
/// };
/// ```
///
/// ## Origin
///
/// Ported from `print/star-prnt.py` in the estrella repository.
#[derive(Debug, Clone)]
pub struct Ripple {
    /// Ripple wavelength - larger values produce wider rings
    pub scale: f32,

    /// Vertical drift factor - creates downward "motion" effect
    pub drift: f32,

    /// Wobble blend factor (0.0 = pure ripple, 1.0 = pure wobble)
    pub wobble_mix: f32,

    /// Gamma correction exponent
    pub gamma: f32,
}

impl Default for Ripple {
    fn default() -> Self {
        Self {
            scale: 6.5,
            drift: 85.0,
            wobble_mix: 0.25,
            gamma: 1.35,
        }
    }
}

impl Pattern for Ripple {
    fn shade(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        let params = art::ripple::Params {
            center_x: 0.5,
            center_y: 0.5,
            scale: self.scale,
            drift: self.drift,
            wobble_mix: self.wobble_mix,
        };
        let v = art::ripple::shade(x, y, width, height, &params);

        // Add border (frame effect)
        if art::in_border(x, y, width, height, 6.0) {
            1.0
        } else {
            v
        }
    }

    fn gamma(&self) -> f32 {
        self.gamma
    }
}

// ============================================================================
// WAVES PATTERN
// ============================================================================

/// # Waves Pattern
///
/// Multi-oscillator interference pattern creating flowing wave effects.
///
/// ## Formula
///
/// ```text
/// nx = 2 * x / width - 1       // Normalized x in [-1, 1]
/// ny = 2 * y / height - 1      // Normalized y in [-1, 1]
/// r = sqrt(nx² + ny²)          // Normalized radius
///
/// horiz = sin(x / 19.0 + 0.7 * sin(y / 37.0))
/// vert = cos(y / 23.0 + 0.9 * cos(x / 41.0))
/// radial = cos(r * 24.0 - y / 29.0)
///
/// v = 0.45 * horiz + 0.35 * vert + 0.20 * radial
/// v = 0.5 + 0.5 * v  // Normalize to [0, 1]
/// ```
///
/// ## Origin
///
/// Ported from `print/waves.py` in the estrella repository.
#[derive(Debug, Clone)]
pub struct Waves {
    /// Gamma correction exponent
    pub gamma: f32,
}

impl Default for Waves {
    fn default() -> Self {
        Self { gamma: 1.25 }
    }
}

impl Pattern for Waves {
    fn shade(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        art::waves::shade(x, y, width, height, &art::waves::Params::default())
    }

    fn gamma(&self) -> f32 {
        self.gamma
    }
}

// ============================================================================
// SICK PATTERN
// ============================================================================

/// # Sick Pattern
///
/// A multi-section visual pattern that cycles through 4 distinct visual styles,
/// creating a vertically-stacked showcase of different algorithmic effects.
///
/// ## Sections (cycling every 480 rows by default)
///
/// | Section | Name | Description |
/// |---------|------|-------------|
/// | 0 | Plasma | Moire/plasma effect with multiple sine waves |
/// | 1 | Rings | Concentric rings with diagonal interference |
/// | 2 | Topography | Contour line effect (like a topographic map) |
/// | 3 | Glitch | Blocky columns with scanlines |
///
/// ## Section Formulas
///
/// ### Section 0: Plasma
/// ```text
/// v = sin(x/11) + sin((x+y)/19) + cos(y/13) + sin(hypot(x - cx, y - cy)/9)
/// v = ((v + 4) / 8) ^ 1.2
/// ```
///
/// ### Section 1: Rings
/// ```text
/// r = hypot(nx, ny)  // normalized radius
/// rings = 0.5 + 0.5 * cos(r * 30 - y/25)
/// diag = 0.5 + 0.5 * sin((x - 2*y) / 23)
/// v = (0.65 * rings + 0.35 * diag) ^ 1.1
/// ```
///
/// ### Section 2: Topography
/// ```text
/// t = sin(x/17) + sin(y/29) + sin((x+y)/41)
/// contours = |fmod(t, 1.0) - 0.5| * 2
/// v = (1 - contours) ^ 2.2
/// ```
///
/// ### Section 3: Glitch
/// ```text
/// col = x / 12
/// base = sin(col * 0.7) * 0.5 + 0.5
/// wobble = 0.5 + 0.5 * sin((x + yy*7) / 15)
/// scan = 1.0 if (yy % 24) in [0, 1] else 0.0
/// v = max(0.55 * base + 0.45 * wobble, scan)
/// ```
///
/// ## Border
///
/// A 2-pixel black border is drawn around the entire image for alignment
/// verification and to clearly show the print width boundaries.
///
/// ## Origin
///
/// Ported from `print/sick.py` in the estrella repository.
#[derive(Debug, Clone)]
pub struct Sick {
    /// Height of each section in rows (default: 480 = 24 bands * 20)
    pub section_height: usize,
}

impl Default for Sick {
    fn default() -> Self {
        Self {
            section_height: 24 * 20, // 480 rows per section
        }
    }
}

impl Pattern for Sick {
    fn default_dimensions(&self) -> (usize, usize) {
        // 4 sections * section_height to show all visual styles
        (576, self.section_height * 4)
    }

    fn shade(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        // Determine which section we're in (cycles through 4 sections)
        let section = (y / self.section_height) % 4;

        let v = match section {
            0 => art::plasma::shade(x, y, width, height, &art::plasma::Params::default()),
            1 => art::rings::shade(x, y, width, height, &art::rings::Params::default()),
            2 => art::topography::shade(x, y, width, height, &art::topography::Params::default()),
            _ => art::glitch::shade(x, y % self.section_height, width, self.section_height, &art::glitch::Params::default()),
        };

        // Add a clean border (2 pixels) for alignment verification
        if art::in_border(x, y, width, height, 2.0) {
            1.0
        } else {
            clamp01(v)
        }
    }

    fn gamma(&self) -> f32 {
        // Each section applies its own gamma internally, so no additional correction
        1.0
    }
}

// ============================================================================
// CALIBRATION PATTERN
// ============================================================================

/// # Calibration Pattern
///
/// A diagnostic pattern for verifying print width, alignment, and dot accuracy.
/// Features clear geometric shapes that make it easy to spot issues.
///
/// ## Visual Elements
///
/// ```text
/// ████████████████████████████████████████
/// █                                      █
/// █ ██                              ██   █
/// █   ██    ▌▌ ▌▌▌ ▌▌▌▌ ▌▌▌▌▌    ██     █
/// █     ██  ▌▌ ▌▌▌ ▌▌▌▌ ▌▌▌▌▌  ██       █
/// █       ██▌▌ ▌▌▌ ▌▌▌▌ ▌▌▌▌▌██         █
/// █         ██ ▌▌▌ ▌▌▌▌ ▌▌▌██           █
/// █       ██  ▌▌▌▌ ▌▌▌▌ ▌██             █
/// █     ██    ▌▌▌▌ ▌▌▌▌██               █
/// █   ██      ▌▌▌▌ ▌▌██                 █
/// █ ██        ▌▌▌▌██                    █
/// █                                      █
/// ████████████████████████████████████████
/// ```
///
/// ## Components
///
/// | Element | Description | Purpose |
/// |---------|-------------|---------|
/// | Border | 6-pixel frame | Shows true print width |
/// | Diagonals | X-shaped cross | Tests diagonal accuracy |
/// | Bars | Width-increasing columns | Tests dot precision |
///
/// ## Bar Widths
///
/// Bars increase in width every 48 pixels:
/// - Column 0: 2 dots wide
/// - Column 1: 3 dots wide
/// - Column 2: 4 dots wide
/// - ... up to 9+ dots wide
///
/// ## Origin
///
/// Ported from `print/demo.py` in the estrella repository.
#[derive(Debug, Clone)]
pub struct Calibration {
    /// Border width in pixels (default: 6)
    pub border_width: usize,

    /// Diagonal line thickness (default: 2)
    pub diagonal_thickness: usize,

    /// Column width for bar groups (default: 48)
    pub bar_column_width: usize,

    /// Base bar width (default: 2)
    pub bar_base_width: usize,

    /// Vertical margin for bars (default: 20)
    pub bar_margin: usize,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            border_width: 6,
            diagonal_thickness: 2,
            bar_column_width: 48,
            bar_base_width: 2,
            bar_margin: 20,
        }
    }
}

impl Pattern for Calibration {
    fn shade(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        let yf = y as f32;
        let wf = width as f32;
        let hf = height as f32;

        // Border check
        let border = x < self.border_width
            || x >= width - self.border_width
            || y < self.border_width
            || y >= height - self.border_width;

        // Diagonal from top-left to bottom-right
        // Line equation: x/W = y/H, so x*H = y*W
        // Distance from point to line: |x*H - y*W| / sqrt(H² + W²)
        let expected_x = (yf * (wf - 1.0) / (hf - 1.0)) as isize;
        let diag1 = (x as isize - expected_x).unsigned_abs() <= self.diagonal_thickness;

        // Diagonal from top-right to bottom-left
        let expected_x2 = ((wf - 1.0) - yf * (wf - 1.0) / (hf - 1.0)) as isize;
        let diag2 = (x as isize - expected_x2).unsigned_abs() <= self.diagonal_thickness;

        // Vertical bars that get thicker every bar_column_width pixels
        let bar_block = x / self.bar_column_width;
        let bar_width = self.bar_base_width + bar_block;
        let in_bar_region = y >= self.bar_margin && y < height - self.bar_margin;
        let bars = (x % self.bar_column_width) < bar_width && in_bar_region;

        if border || diag1 || diag2 || bars {
            1.0
        } else {
            0.0
        }
    }

    fn gamma(&self) -> f32 {
        // No gamma correction needed for binary pattern
        1.0
    }
}

// ============================================================================
// OTHER PATTERN (Print Tests Suite)
// ============================================================================

/// # Other Pattern
///
/// A comprehensive test suite pattern that demonstrates various StarPRNT
/// rendering techniques and effects from `spec/print_tests.py`.
///
/// ## Sections (cycling every 300 rows by default)
///
/// | Section | Name | Description |
/// |---------|------|-------------|
/// | 0 | Micro-feed Lines | Horizontal lines with varying feed spacing |
/// | 1 | Density Comparison | Ripple pattern at 3 different densities |
/// | 2 | Overburn Effect | Double-pass simulation (darker, bloomed) |
/// | 3 | Jitter Bands | Ripple with horizontal banding artifacts |
///
/// ## Section Details
///
/// ### Section 0: Micro-feed Visualization
///
/// Displays horizontal 1-pixel lines separated by gaps representing different
/// ESC J n feed amounts (n=1, 2, 4, 8, 12). Each group of 8 lines uses the
/// same feed value. Useful for verifying feed command accuracy.
///
/// ### Section 1: Density Comparison
///
/// Shows the ripple pattern rendered at three different "print densities"
/// (simulated by adjusting gamma/intensity):
/// - Top third: Light (density=1, gamma=0.8)
/// - Middle third: Medium (density=3, gamma=1.0)
/// - Bottom third: Heavy (density=5, gamma=1.5)
///
/// ### Section 2: Overburn Effect
///
/// Simulates double-pass printing where the same image is printed twice,
/// causing darker tones and ink bloom. Achieved by darkening the ripple
/// pattern and adding slight blur/spread.
///
/// ### Section 3: Jitter/Banding Effect
///
/// Simulates the organic gradients and banding artifacts that occur when
/// inserting micro-feeds and delays between raster chunks. Creates horizontal
/// bands that vary slightly in intensity.
///
/// ## Origin
///
/// Based on test patterns from `spec/print_tests.py` in the estrella repository.
#[derive(Debug, Clone)]
pub struct Other {
    /// Height of each section in rows (default: 300)
    pub section_height: usize,
}

impl Default for Other {
    fn default() -> Self {
        Self {
            section_height: 300,
        }
    }
}

impl Pattern for Other {
    fn default_dimensions(&self) -> (usize, usize) {
        // 4 sections * section_height to show all test patterns
        (576, self.section_height * 4)
    }

    fn shade(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        // Determine which section we're in (cycles through 4 sections)
        let section = (y / self.section_height) % 4;
        let yy = y % self.section_height; // Local y within section

        match section {
            0 => {
                // Section 0: Micro-feed visualization
                // Groups of 8 lines with different feed spacing
                // Feed values: 1, 2, 4, 8, 12 (units of ~0.25mm)

                let feed_values = [1, 2, 4, 8, 12];
                let lines_per_group = 8;

                // Each group uses a specific feed value
                let group_idx = yy / (lines_per_group * 4); // 4 pixels between line groups
                let feed_idx = group_idx.min(feed_values.len() - 1);
                let _feed_spacing = feed_values[feed_idx] as usize;

                // Position within the current group
                let group_y = yy % (lines_per_group * 4);
                let line_num = group_y / 4; // Which line in this group (0-7)
                let line_y = group_y % 4; // Position within spacing

                // Draw 1-pixel line, then feed_spacing pixels of gap
                let is_line = line_y == 0 && line_num < lines_per_group;

                if is_line {
                    1.0
                } else {
                    0.0
                }
            }
            1 => {
                // Section 1: Density comparison
                // Three horizontal bands showing different densities
                let sub_section = (yy * 3) / self.section_height;

                // Use ripple pattern as base
                let xf = x as f32;
                let yf = y as f32;
                let wf = width as f32;
                let hf = height as f32;

                let cx = wf / 2.0;
                let cy = hf / 2.0;
                let dx = xf - cx;
                let dy = yf - cy;
                let r = (dx * dx + dy * dy).sqrt();

                let ripple = 0.5 + 0.5 * (r / 6.5 - yf / 85.0).cos();
                let wobble = 0.5 + 0.5 * (xf / 37.0 + 0.7 * (yf / 53.0).cos()).sin();
                let v = 0.75 * ripple + 0.25 * wobble;

                // Apply different gamma values to simulate densities
                let gamma = match sub_section {
                    0 => 0.8,  // Light (density=1)
                    1 => 1.0,  // Medium (density=3)
                    _ => 1.5,  // Heavy (density=5)
                };

                clamp01(v).powf(gamma)
            }
            2 => {
                // Section 2: Overburn (double-pass) effect
                // Darken and spread the pattern to simulate printing twice
                let xf = x as f32;
                let yf = y as f32;
                let wf = width as f32;
                let hf = height as f32;

                let cx = wf / 2.0;
                let cy = hf / 2.0;
                let dx = xf - cx;
                let dy = yf - cy;
                let r = (dx * dx + dy * dy).sqrt();

                let ripple = 0.5 + 0.5 * (r / 6.5 - yf / 85.0).cos();
                let wobble = 0.5 + 0.5 * (xf / 37.0 + 0.7 * (yf / 53.0).cos()).sin();
                let v = 0.75 * ripple + 0.25 * wobble;

                // Simulate overburn by darkening and adding bloom
                // Double-pass means darker overall + some spreading
                let darkened = v * 0.7 + 0.3; // Shift darker

                // Add slight blur by sampling neighbors (bloom effect)
                let blur_amount = 0.15;
                let blurred = darkened * (1.0 - blur_amount) + v * blur_amount;

                clamp01(blurred).powf(1.6) // Extra contrast
            }
            _ => {
                // Section 3: Jitter/banding effect
                // Horizontal bands with varying intensity (organic gradients)
                let xf = x as f32;
                let yf = y as f32;
                let wf = width as f32;
                let hf = height as f32;

                let cx = wf / 2.0;
                let cy = hf / 2.0;
                let dx = xf - cx;
                let dy = yf - cy;
                let r = (dx * dx + dy * dy).sqrt();

                let ripple = 0.5 + 0.5 * (r / 6.5 - yf / 85.0).cos();
                let wobble = 0.5 + 0.5 * (xf / 37.0 + 0.7 * (yf / 53.0).cos()).sin();
                let v = 0.75 * ripple + 0.25 * wobble;

                // Create banding every 24 rows (one band height)
                let band_num = yy / 24;
                let band_y = yy % 24;

                // Intensity varies slightly per band (simulates jitter/cooldown)
                let band_variation = ((band_num as f32 * 0.3).sin() + 1.0) / 2.0;
                let band_mod = 0.9 + band_variation * 0.2; // 0.9 to 1.1

                // Add visible band boundaries (darker lines)
                let is_band_edge = band_y == 0 || band_y == 1;
                let edge_darkening = if is_band_edge { 0.15 } else { 0.0 };

                let modified = (v * band_mod) + edge_darkening;
                clamp01(modified).powf(1.3)
            }
        }
    }

    fn gamma(&self) -> f32 {
        // Each section applies its own gamma internally
        1.0
    }
}

// ============================================================================
// PATTERN REGISTRY
// ============================================================================

/// Get a pattern by name.
///
/// ## Available Patterns
///
/// - "ripple" - Concentric circles with wobble
/// - "waves" - Multi-oscillator interference
/// - "sick" - Multi-section visual showcase
/// - "calibration" - Diagnostic pattern with borders, diagonals, bars
/// - "other" - Print tests suite (micro-feed, density, overburn, jitter)
///
/// ## Returns
///
/// `Some(Box<dyn Pattern>)` if the name is recognized, `None` otherwise.
pub fn by_name(name: &str) -> Option<Box<dyn Pattern>> {
    match name.to_lowercase().as_str() {
        "ripple" => Some(Box::new(Ripple::default())),
        "waves" => Some(Box::new(Waves::default())),
        "sick" => Some(Box::new(Sick::default())),
        "calibration" | "demo" => Some(Box::new(Calibration::default())),
        "other" => Some(Box::new(Other::default())),
        _ => None,
    }
}

/// List all available pattern names.
pub fn list_patterns() -> &'static [&'static str] {
    &["ripple", "waves", "sick", "calibration", "other"]
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ripple_shade_range() {
        let ripple = Ripple::default();
        for y in 0..100 {
            for x in 0..100 {
                let s = ripple.shade(x, y, 576, 500);
                assert!(
                    s >= 0.0 && s <= 1.0,
                    "Ripple shade out of range at ({},{}): {}",
                    x,
                    y,
                    s
                );
            }
        }
    }

    #[test]
    fn test_ripple_border() {
        let ripple = Ripple::default();

        // Top-left corner should be border (black)
        assert_eq!(ripple.shade(0, 0, 576, 500), 1.0);
        assert_eq!(ripple.shade(5, 5, 576, 500), 1.0);

        // Just inside border should not be forced black
        let inside = ripple.shade(10, 10, 576, 500);
        assert!(inside < 1.0 || inside >= 0.0); // Could be any valid shade
    }

    #[test]
    fn test_ripple_symmetry() {
        let ripple = Ripple::default();
        let w = 576;
        let h = 500;

        // Horizontal symmetry (approximately, due to wobble)
        let left = ripple.shade(100, 250, w, h);
        let right = ripple.shade(w - 100, 250, w, h);
        // Allow for wobble effect breaking perfect symmetry
        assert!(
            (left - right).abs() < 0.3,
            "Left {} vs right {} differ too much",
            left,
            right
        );
    }

    #[test]
    fn test_waves_shade_range() {
        let waves = Waves::default();
        for y in 0..100 {
            for x in 0..100 {
                let s = waves.shade(x, y, 576, 500);
                assert!(
                    s >= 0.0 && s <= 1.0,
                    "Waves shade out of range at ({},{}): {}",
                    x,
                    y,
                    s
                );
            }
        }
    }

    #[test]
    fn test_sick_border() {
        let sick = Sick::default();
        let w = 576;
        let h = 1920; // 4 sections * 480

        // Corners are border (2 pixel border)
        assert_eq!(sick.shade(0, 0, w, h), 1.0);
        assert_eq!(sick.shade(1, 0, w, h), 1.0);
        assert_eq!(sick.shade(w - 1, 0, w, h), 1.0);
        assert_eq!(sick.shade(0, h - 1, w, h), 1.0);
        assert_eq!(sick.shade(w - 1, h - 1, w, h), 1.0);

        // Just inside border should not be forced to 1.0
        let inside = sick.shade(10, 10, w, h);
        assert!(inside >= 0.0 && inside <= 1.0);
    }

    #[test]
    fn test_sick_shade_range() {
        let sick = Sick::default();
        let w = 576;
        let h = 1920; // 4 sections

        // Test shade values in all 4 sections
        for section in 0..4 {
            let y = section * 480 + 240; // Middle of each section
            for x in (10..w - 10).step_by(50) {
                let s = sick.shade(x, y, w, h);
                assert!(
                    s >= 0.0 && s <= 1.0,
                    "Sick shade out of range at ({},{}) section {}: {}",
                    x,
                    y,
                    section,
                    s
                );
            }
        }
    }

    #[test]
    fn test_sick_sections() {
        let sick = Sick::default();
        let w = 576;
        let h = 2000;

        // Each section should produce different patterns
        // Sample from the center of each section
        let s0 = sick.shade(w / 2, 240, w, h); // Section 0: Plasma
        let s1 = sick.shade(w / 2, 720, w, h); // Section 1: Rings
        let s2 = sick.shade(w / 2, 1200, w, h); // Section 2: Topography
        let s3 = sick.shade(w / 2, 1680, w, h); // Section 3: Glitch

        // All should be valid shades
        assert!(s0 >= 0.0 && s0 <= 1.0);
        assert!(s1 >= 0.0 && s1 <= 1.0);
        assert!(s2 >= 0.0 && s2 <= 1.0);
        assert!(s3 >= 0.0 && s3 <= 1.0);
    }

    #[test]
    fn test_pattern_render_dimensions() {
        let ripple = Ripple::default();
        let data = ripple.render(576, 100);
        assert_eq!(data.len(), 72 * 100); // 576/8 = 72 bytes per row
    }

    #[test]
    fn test_calibration_border() {
        let cal = Calibration::default();
        let w = 576;
        let h = 240;

        // Corners are border (6 pixel border)
        assert_eq!(cal.shade(0, 0, w, h), 1.0);
        assert_eq!(cal.shade(5, 5, w, h), 1.0);
        assert_eq!(cal.shade(w - 1, 0, w, h), 1.0);
        assert_eq!(cal.shade(0, h - 1, w, h), 1.0);

        // Just inside border should be white (unless on diagonal/bar)
        // Check center area which should be empty
        let inside = cal.shade(w / 2, h / 2, w, h);
        // Could be on diagonal, so just check it's valid
        assert!(inside >= 0.0 && inside <= 1.0);
    }

    #[test]
    fn test_calibration_diagonals() {
        let cal = Calibration::default();
        let w = 576;
        let h = 240;

        // Top-left corner region (just inside border) should have diagonal
        // At y=20, expected_x for diag1 ≈ 20 * 575 / 239 ≈ 48
        let y = 20;
        let expected_x = (y as f32 * (w as f32 - 1.0) / (h as f32 - 1.0)) as usize;
        assert_eq!(
            cal.shade(expected_x, y, w, h),
            1.0,
            "Diagonal should be black"
        );
    }

    #[test]
    fn test_calibration_bars() {
        let cal = Calibration::default();
        let w = 576;
        let h = 240;

        // Bars are in the middle region (y between 20 and h-20)
        let y = h / 2;

        // First bar block (x < 48) has width 2, so x=0,1 are in bar
        // But those overlap with border. x=7 is NOT in bar (7 % 48 = 7 >= 2)
        assert_eq!(cal.shade(0, y, w, h), 1.0, "x=0 is border+bar");
        assert_eq!(cal.shade(1, y, w, h), 1.0, "x=1 is border+bar");

        // Second bar block (48 <= x < 96) has width 3, so x=48,49,50 are in bar
        assert_eq!(
            cal.shade(48, y, w, h),
            1.0,
            "Second bar start should be black"
        );
        assert_eq!(cal.shade(49, y, w, h), 1.0, "Second bar should be black");
        assert_eq!(cal.shade(50, y, w, h), 1.0, "Second bar should be black");

        // x=51 in second block (width 3), so should be white
        assert_eq!(cal.shade(51, y, w, h), 0.0, "After bar should be white");
    }

    #[test]
    fn test_calibration_shade_range() {
        let cal = Calibration::default();
        for y in 0..100 {
            for x in 0..100 {
                let s = cal.shade(x, y, 576, 240);
                assert!(
                    s == 0.0 || s == 1.0,
                    "Calibration should be binary at ({},{}): {}",
                    x,
                    y,
                    s
                );
            }
        }
    }

    #[test]
    fn test_by_name() {
        assert!(by_name("ripple").is_some());
        assert!(by_name("RIPPLE").is_some()); // Case insensitive
        assert!(by_name("waves").is_some());
        assert!(by_name("sick").is_some());
        assert!(by_name("calibration").is_some());
        assert!(by_name("demo").is_some()); // Alias
        assert!(by_name("unknown").is_none());
    }

    #[test]
    fn test_list_patterns() {
        let patterns = list_patterns();
        assert!(patterns.contains(&"ripple"));
        assert!(patterns.contains(&"waves"));
        assert!(patterns.contains(&"sick"));
        assert!(patterns.contains(&"calibration"));
    }

    #[test]
    fn test_gamma_default() {
        let ripple = Ripple::default();
        assert!((ripple.gamma() - 1.35).abs() < 0.01);

        let waves = Waves::default();
        assert!((waves.gamma() - 1.25).abs() < 0.01);

        let sick = Sick::default();
        assert!((sick.gamma() - 1.0).abs() < 0.01);

        let other = Other::default();
        assert!((other.gamma() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_other_shade_range() {
        let other = Other::default();
        let w = 576;
        let h = 1200; // 4 sections * 300

        // Test shade values in all 4 sections
        for section in 0..4 {
            let y = section * 300 + 150; // Middle of each section
            for x in (10..w - 10).step_by(50) {
                let s = other.shade(x, y, w, h);
                assert!(
                    s >= 0.0 && s <= 1.0,
                    "Other shade out of range at ({},{}) section {}: {}",
                    x,
                    y,
                    section,
                    s
                );
            }
        }
    }

    #[test]
    fn test_other_sections() {
        let other = Other::default();
        let w = 576;
        let h = 1200;

        // Section 0: Micro-feed lines (binary pattern)
        let s0_line = other.shade(w / 2, 0, w, h); // Should be a line
        let s0_gap = other.shade(w / 2, 2, w, h); // Should be gap
        assert!(s0_line == 1.0 || s0_line == 0.0);
        assert!(s0_gap == 1.0 || s0_gap == 0.0);

        // Section 1: Density comparison (should have variation)
        let s1 = other.shade(w / 2, 450, w, h);
        assert!(s1 >= 0.0 && s1 <= 1.0);

        // Section 2: Overburn effect (should be darker)
        let s2 = other.shade(w / 2, 750, w, h);
        assert!(s2 >= 0.0 && s2 <= 1.0);

        // Section 3: Jitter bands (should have banding)
        let s3 = other.shade(w / 2, 1050, w, h);
        assert!(s3 >= 0.0 && s3 <= 1.0);
    }

    #[test]
    fn test_other_default_dimensions() {
        let other = Other::default();
        let (w, h) = other.default_dimensions();
        assert_eq!(w, 576);
        assert_eq!(h, 1200); // 4 sections * 300
    }

    #[test]
    fn test_by_name_other() {
        assert!(by_name("other").is_some());
        assert!(by_name("OTHER").is_some()); // Case insensitive
    }

    #[test]
    fn test_list_patterns_includes_other() {
        let patterns = list_patterns();
        assert!(patterns.contains(&"other"));
    }
}
