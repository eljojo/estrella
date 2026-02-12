//! # Ripple Logo Generator
//!
//! Generates a ripple pattern logo matching the reference implementation
//! in `spec/nv_logo_store.py`.

use super::LogoRaster;
use crate::art;
use crate::render::dither;

/// Ripple logo generator.
///
/// Uses `art::ripple` with logo-specific parameters:
/// - Offset center at (52%, 35%)
/// - Wobble mix of 0.22
/// - 2-pixel border
/// - Gamma 1.35
pub struct RippleLogo;

impl RippleLogo {
    /// Default width for the ripple logo.
    pub const WIDTH: u16 = 576;
    /// Default height for the ripple logo.
    pub const HEIGHT: u16 = 120;

    /// Generate the ripple logo raster data.
    pub fn raster() -> LogoRaster {
        Self::raster_with_size(Self::WIDTH, Self::HEIGHT)
    }

    /// Generate ripple logo with custom dimensions.
    pub fn raster_with_size(width: u16, height: u16) -> LogoRaster {
        let w = width as usize;
        let h = height as usize;
        let params = art::ripple::Params::logo();

        let data = dither::generate_raster(
            w,
            h,
            |x, y, width, height| {
                // Add 2-pixel border
                if art::in_border(x, y, width, height, 2.0) {
                    1.0
                } else {
                    let shade = art::ripple::shade(x, y, width, height, &params);
                    art::gamma_correct(shade, 1.35)
                }
            },
            dither::DitheringAlgorithm::Bayer,
        );

        LogoRaster {
            width,
            height,
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ripple_dimensions() {
        let raster = RippleLogo::raster();
        assert_eq!(raster.width, 576);
        assert_eq!(raster.height, 120);
    }

    #[test]
    fn test_ripple_data_size() {
        let raster = RippleLogo::raster();
        let expected_size = 576usize.div_ceil(8) * 120; // 72 * 120 = 8640
        assert_eq!(raster.data.len(), expected_size);
    }

    #[test]
    fn test_ripple_has_pixels() {
        let raster = RippleLogo::raster();
        let black_pixels: usize = raster
            .data
            .iter()
            .map(|byte| byte.count_ones() as usize)
            .sum();
        // Should have a reasonable amount of black pixels
        assert!(black_pixels > 1000, "Ripple has too few pixels");
    }

    #[test]
    fn test_ripple_border() {
        let raster = RippleLogo::raster();
        // Top-left corner should be black (border)
        let top_left = (raster.data[0] >> 7) & 1;
        assert_eq!(top_left, 1, "Border should be filled");
    }

    #[test]
    fn test_custom_size() {
        let raster = RippleLogo::raster_with_size(288, 60);
        assert_eq!(raster.width, 288);
        assert_eq!(raster.height, 60);
        assert_eq!(raster.data.len(), 288usize.div_ceil(8) * 60);
    }
}
