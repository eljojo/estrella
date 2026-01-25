//! # Rendering Module
//!
//! This module provides tools for generating visual content for thermal printers.
//!
//! ## Modules
//!
//! - [`dither`]: Bayer 8x8 ordered dithering for binary conversion
//! - [`patterns`]: Visual patterns (ripple, waves, calibration)
//! - [`weave`]: Pattern blending with crossfade transitions
//!
//! ## Usage Example
//!
//! ```
//! use estrella::render::patterns::{self, Pattern, Ripple};
//! use estrella::render::dither::DitheringAlgorithm;
//!
//! // Create a ripple pattern
//! let ripple = Ripple::default();
//!
//! // Render to raster data (576 dots wide, 500 rows tall) with Bayer dithering
//! let raster_data = patterns::render(&ripple, 576, 500, DitheringAlgorithm::Bayer);
//!
//! // raster_data is ready to send via graphics::raster() command
//! ```

use image::{GrayImage, Luma};
use std::io::Cursor;

pub mod composer;
pub mod dither;
pub mod patterns;
pub mod weave;

/// Convert packed 1-bit raster data to PNG bytes.
///
/// Takes raster data where each bit represents a pixel (1 = black, 0 = white),
/// packed 8 pixels per byte, MSB first.
///
/// # Arguments
/// * `width` - Width in pixels
/// * `height` - Height in pixels
/// * `raster_data` - Packed 1-bit raster data (width_bytes * height bytes)
///
/// # Returns
/// PNG-encoded image bytes, or an error message.
pub fn raster_to_png(width: usize, height: usize, raster_data: &[u8]) -> Result<Vec<u8>, String> {
    let width_bytes = width.div_ceil(8);

    let mut img = GrayImage::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let byte_idx = y * width_bytes + x / 8;
            let bit_idx = 7 - (x % 8);
            let is_black = (raster_data[byte_idx] >> bit_idx) & 1 == 1;
            let color = if is_black { 0u8 } else { 255u8 };
            img.put_pixel(x as u32, y as u32, Luma([color]));
        }
    }

    let mut png_bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    Ok(png_bytes)
}
