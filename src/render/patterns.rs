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
// Classic patterns
pub use art::calibration::Calibration;
pub use art::crystal::Crystal;
pub use art::density::Density;
pub use art::erosion::Erosion;
pub use art::estrella::Estrella;
pub use art::flowfield::Flowfield;
pub use art::glitch::Glitch;
pub use art::jitter::Jitter;
pub use art::microfeed::Microfeed;
pub use art::mycelium::Mycelium;
pub use art::overburn::Overburn;
pub use art::plasma::Plasma;
pub use art::riley::Riley;
pub use art::riley_check::RileyCheck;
pub use art::riley_curve::RileyCurve;
pub use art::rings::Rings;
pub use art::ripple::Ripple;
pub use art::scintillate::Scintillate;
pub use art::topography::Topography;
pub use art::tunnel::Tunnel;
pub use art::vasarely::Vasarely;
pub use art::vasarely_bubbles::VasarelyBubbles;
pub use art::vasarely_hex::VasarelyHex;
pub use art::waves::Waves;
pub use art::zebra::Zebra;
// Glitch / Digital
pub use art::corrupt_barcode::CorruptBarcode;
pub use art::databend::Databend;
pub use art::scanline_tear::ScanlineTear;
// Algorithmic / Mathematical
pub use art::attractor::Attractor;
pub use art::automata::Automata;
pub use art::moire::Moire;
pub use art::reaction_diffusion::ReactionDiffusion;
pub use art::voronoi::Voronoi;
// Texture / Tactile
pub use art::crosshatch::Crosshatch;
pub use art::stipple::Stipple;
pub use art::weave::Weave;
pub use art::woodgrain::Woodgrain;

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
        // New patterns
        assert!(patterns.contains(&"corrupt_barcode"));
        assert!(patterns.contains(&"voronoi"));
        assert!(patterns.contains(&"weave"));
        assert_eq!(patterns.len(), 37);
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
