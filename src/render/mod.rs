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

pub mod composer;
pub mod dither;
pub mod patterns;
pub mod weave;
