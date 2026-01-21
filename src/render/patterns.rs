//! Pattern rendering for thermal printing.
//!
//! This module re-exports patterns from [`crate::art`] and provides
//! rendering utilities (dithering).
//!
//! See [`crate::art`] for pattern implementations and the [`Pattern`] trait.

use crate::art;

use super::dither;

// Re-export everything from art for backwards compatibility
pub use art::by_name;
pub use art::by_name_golden;
pub use art::by_name_random;
pub use art::Pattern;
pub use art::PATTERNS;
pub use art::calibration::Calibration;
pub use art::crystal::Crystal;
pub use art::density::Density;
pub use art::erosion::Erosion;
pub use art::flowfield::Flowfield;
pub use art::glitch::Glitch;
pub use art::jitter::Jitter;
pub use art::microfeed::Microfeed;
pub use art::mycelium::Mycelium;
pub use art::overburn::Overburn;
pub use art::plasma::Plasma;
pub use art::riley::Riley;
pub use art::rings::Rings;
pub use art::ripple::Ripple;
pub use art::scintillate::Scintillate;
pub use art::topography::Topography;
pub use art::vasarely::Vasarely;
pub use art::waves::Waves;

/// List all available pattern names.
pub fn list_patterns() -> &'static [&'static str] {
    PATTERNS
}

/// Render a pattern to a byte array suitable for raster graphics.
///
/// Uses the specified dithering algorithm to convert grayscale intensities
/// to binary output.
pub fn render(
    pattern: &dyn Pattern,
    width: usize,
    height: usize,
    algorithm: dither::DitheringAlgorithm,
) -> Vec<u8> {
    dither::generate_raster(width, height, |x, y, w, h| pattern.intensity(x, y, w, h), algorithm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_patterns() {
        let patterns = list_patterns();
        assert!(patterns.contains(&"ripple"));
        assert!(patterns.contains(&"calibration"));
        assert!(patterns.contains(&"riley"));
        assert!(patterns.contains(&"mycelium"));
        assert_eq!(patterns.len(), 18);
    }

    #[test]
    fn test_by_name() {
        assert!(by_name("ripple").is_some());
        assert!(by_name("RIPPLE").is_some()); // Case insensitive
        assert!(by_name("calibration").is_some());
        assert!(by_name("demo").is_some()); // Alias
        assert!(by_name("unknown").is_none());
    }

    #[test]
    fn test_render() {
        let ripple = Ripple::golden();
        let data = render(&ripple, 576, 100, dither::DitheringAlgorithm::Bayer);
        assert_eq!(data.len(), 72 * 100); // 576/8 = 72 bytes per row
    }
}
