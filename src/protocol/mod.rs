//! # StarPRNT Protocol Implementation
//!
//! This module provides low-level command builders for the StarPRNT protocol
//! used by Star Micronics thermal receipt printers.
//!
//! ## Module Structure
//!
//! - [`commands`]: Basic printer commands (init, cut, feed)
//! - [`graphics`]: Bit image and raster graphics commands
//! - [`text`]: Text styling (alignment, fonts, bold, underline, etc.)
//! - [`barcode`]: QR codes and PDF417 barcodes
//!
//! ## Usage Example
//!
//! ```
//! use estrella::protocol::{commands, graphics, text, barcode};
//!
//! // Build a simple print sequence
//! let mut data = Vec::new();
//!
//! // Initialize printer
//! data.extend(commands::init());
//!
//! // Set text style
//! data.extend(text::align_center());
//! data.extend(text::bold_on());
//! data.extend(b"RECEIPT\n");
//! data.extend(text::bold_off());
//! data.extend(text::align_left());
//!
//! // Print a 24-row graphics band
//! let band_data = vec![0xAA; 72 * 24]; // Vertical stripes
//! data.extend(graphics::band(72, &band_data));
//!
//! // Feed and cut
//! data.extend(commands::cut_full_feed());
//!
//! // Send `data` to printer via transport...
//! ```
//!
//! ## Protocol Reference
//!
//! This implementation is based on "StarPRNT Command Specifications Rev. 4.10"
//! by Star Micronics Co., Ltd.

pub mod barcode;
pub mod commands;
pub mod cp437;
pub mod graphics;
pub mod nv_graphics;
pub mod text;
