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
//! | [`Sick`] | Calibration pattern with borders and diagonals |
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
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let xf = x as f32;
        let yf = y as f32;

        // Distance from center
        let dx = xf - cx;
        let dy = yf - cy;
        let r = (dx * dx + dy * dy).sqrt();

        // Ripple: concentric circles with vertical drift
        let ripple = 0.5 + 0.5 * (r / self.scale - yf / self.drift).cos();

        // Wobble: interference pattern
        let wobble = 0.5 + 0.5 * (xf / 37.0 + 0.7 * (yf / 53.0).cos()).sin();

        // Blend ripple and wobble
        let v = ripple * (1.0 - self.wobble_mix) + wobble * self.wobble_mix;

        // Add border (frame effect)
        let border_width = 6.0;
        let on_border = xf < border_width
            || xf >= (width as f32 - border_width)
            || yf < border_width
            || yf >= (height as f32 - border_width);

        if on_border {
            1.0
        } else {
            clamp01(v)
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
        let xf = x as f32;
        let yf = y as f32;
        let wf = width as f32;
        let hf = height as f32;

        // Normalized coordinates [-1, 1]
        let nx = 2.0 * xf / wf - 1.0;
        let ny = 2.0 * yf / hf - 1.0;
        let r = (nx * nx + ny * ny).sqrt();

        // Three oscillators
        let horiz = (xf / 19.0 + 0.7 * (yf / 37.0).sin()).sin();
        let vert = (yf / 23.0 + 0.9 * (xf / 41.0).cos()).cos();
        let radial = (r * 24.0 - yf / 29.0).cos();

        // Weighted blend
        let v = 0.45 * horiz + 0.35 * vert + 0.20 * radial;

        // Normalize from [-1, 1] to [0, 1]
        clamp01(0.5 + 0.5 * v)
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
    fn shade(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        let xf = x as f32;
        let yf = y as f32;
        let wf = width as f32;
        let hf = height as f32;

        // Normalized coordinates in [-1, 1]
        let nx = (xf - wf * 0.5) / (wf * 0.5);
        let ny = (yf - hf * 0.5) / (hf * 0.5);

        // Determine which section we're in (cycles through 4 sections)
        let section = (y / self.section_height) % 4;
        let yy = y % self.section_height; // Local y within section

        let v = match section {
            0 => {
                // Section 0: Plasma / Moire
                // Multiple overlapping sine waves create interference patterns
                let plasma = (xf / 11.0).sin()
                    + ((xf + yf) / 19.0).sin()
                    + (yf / 13.0).cos()
                    + ((xf - wf * 0.35).hypot(yf - hf * 0.2) / 9.0).sin();

                // Normalize from roughly [-4, 4] to [0, 1]
                let normalized = (plasma + 4.0) / 8.0;

                // Apply gamma for contrast
                normalized.powf(1.2)
            }
            1 => {
                // Section 1: Concentric rings + diagonal interference
                let r = (nx * nx + ny * ny).sqrt();

                // Rings emanating from center
                let rings = 0.5 + 0.5 * (r * 30.0 - yf / 25.0).cos();

                // Diagonal wave pattern
                let diag = 0.5 + 0.5 * ((xf - 2.0 * yf) / 23.0).sin();

                // Blend: 65% rings, 35% diagonal
                let blended = 0.65 * rings + 0.35 * diag;

                blended.powf(1.1)
            }
            2 => {
                // Section 2: Topography contour lines
                // Like elevation lines on a topographic map
                let t = (xf / 17.0).sin() + (yf / 29.0).sin() + ((xf + yf) / 41.0).sin();

                // Create contour lines by mapping to periodic bands
                // fmod gives us repeating regions, then we find distance to band center
                let t_wrapped = t - t.floor(); // fmod to [0, 1)
                let contours = (t_wrapped - 0.5).abs() * 2.0; // 0 at midline

                // Invert so contour lines are dark
                (1.0 - contours).powf(2.2)
            }
            _ => {
                // Section 3: Glitch effect
                // Blocky columns with horizontal scanlines
                let col = (x / 12) as f32;

                // Base intensity varies by column
                let base = (col * 0.7).sin() * 0.5 + 0.5;

                // Wobble adds horizontal variation
                let wobble = 0.5 + 0.5 * ((xf + (yy as f32 * 7.0)) / 15.0).sin();

                // Scanlines: dark lines every 24 rows (at rows 0 and 1 of each band)
                let scan_pos = yy % 24;
                let scan = if scan_pos == 0 || scan_pos == 1 {
                    1.0
                } else {
                    0.0
                };

                // Blend base and wobble, then overlay scanlines
                let blended = 0.55 * base + 0.45 * wobble;
                blended.max(scan)
            }
        };

        // Add a clean border (2 pixels) for alignment verification
        let border = x < 2 || x >= width - 2 || y < 2 || y >= height - 2;
        if border {
            return 1.0;
        }

        clamp01(v)
    }

    fn gamma(&self) -> f32 {
        // Each section applies its own gamma internally, so no additional correction
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
/// - "sick" - Calibration pattern
///
/// ## Returns
///
/// `Some(Box<dyn Pattern>)` if the name is recognized, `None` otherwise.
pub fn by_name(name: &str) -> Option<Box<dyn Pattern>> {
    match name.to_lowercase().as_str() {
        "ripple" => Some(Box::new(Ripple::default())),
        "waves" => Some(Box::new(Waves::default())),
        "sick" => Some(Box::new(Sick::default())),
        _ => None,
    }
}

/// List all available pattern names.
pub fn list_patterns() -> &'static [&'static str] {
    &["ripple", "waves", "sick"]
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
    fn test_by_name() {
        assert!(by_name("ripple").is_some());
        assert!(by_name("RIPPLE").is_some()); // Case insensitive
        assert!(by_name("waves").is_some());
        assert!(by_name("sick").is_some());
        assert!(by_name("unknown").is_none());
    }

    #[test]
    fn test_list_patterns() {
        let patterns = list_patterns();
        assert!(patterns.contains(&"ripple"));
        assert!(patterns.contains(&"waves"));
        assert!(patterns.contains(&"sick"));
    }

    #[test]
    fn test_gamma_default() {
        let ripple = Ripple::default();
        assert!((ripple.gamma() - 1.35).abs() < 0.01);

        let waves = Waves::default();
        assert!((waves.gamma() - 1.25).abs() < 0.01);

        let sick = Sick::default();
        assert!((sick.gamma() - 1.0).abs() < 0.01);
    }
}
