//! # Rendering Module
//!
//! This module provides tools for generating visual content for thermal printers.
//!
//! ## Modules
//!
//! - [`dither`]: Bayer 8x8 ordered dithering for binary conversion
//! - [`patterns`]: Visual patterns (ripple, waves, calibration)
//!
//! ## Usage Example
//!
//! ```
//! use estrella::render::patterns::{Pattern, Ripple};
//!
//! // Create a ripple pattern
//! let ripple = Ripple::default();
//!
//! // Render to raster data (576 dots wide, 500 rows tall)
//! let raster_data = ripple.render(576, 500);
//!
//! // raster_data is ready to send via graphics::raster() command
//! ```

pub mod dither;
pub mod patterns;
