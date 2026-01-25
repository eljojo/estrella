//! # Estrella - Thermal Receipt Printer Library
//!
//! Estrella is a Rust library for printing on Star Micronics thermal printers
//! via Bluetooth. It provides:
//!
//! - **Protocol implementation**: StarPRNT command builders
//! - **Visual patterns**: Ripple, waves, and calibration patterns
//! - **Dithering**: Bayer 8x8 ordered dithering for grayscale conversion
//! - **Transport**: Bluetooth RFCOMM communication
//!
//! ## Quick Start
//!
//! ```no_run
//! use estrella::{
//!     protocol::{commands, graphics},
//!     render::patterns::{self, Ripple},
//!     transport::BluetoothTransport,
//!     printer::PrinterConfig,
//! };
//!
//! // Open connection to printer
//! let mut transport = BluetoothTransport::open("/dev/rfcomm0")?;
//!
//! // Get printer configuration
//! let config = PrinterConfig::TSP650II;
//!
//! // Create a ripple pattern
//! let ripple = Ripple::default();
//! let height = 500;
//!
//! // Render pattern to raster data with Bayer dithering
//! use estrella::render::dither::DitheringAlgorithm;
//! let raster_data = patterns::render(
//!     &ripple,
//!     config.width_dots as usize,
//!     height,
//!     DitheringAlgorithm::Bayer
//! );
//!
//! // Build print sequence
//! let mut data = Vec::new();
//! data.extend(commands::init());
//! data.extend(graphics::raster(config.width_dots, height as u16, &raster_data));
//! data.extend(commands::cut_full_feed());
//!
//! // Send to printer
//! transport.write_all(&data)?;
//!
//! # Ok::<(), estrella::error::EstrellaError>(())
//! ```
//!
//! ## Module Overview
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`protocol`] | StarPRNT command builders |
//! | [`render`] | Dithering and pattern generation |
//! | [`transport`] | Communication backends |
//! | [`printer`] | Printer configurations |
//! | [`error`] | Error types |
//!
//! ## Supported Printers
//!
//! Currently tested with:
//! - Star TSP650II (80mm paper, 203 DPI, Bluetooth)
//!
//! Other Star printers using StarPRNT protocol should work with
//! appropriate configuration adjustments.

pub mod art;
pub mod components;
pub mod error;
pub mod ir;
pub mod logos;
pub mod preview;
pub mod printer;
pub mod protocol;
pub mod receipt;
pub mod render;
pub mod server;
pub mod shader;
pub mod transport;

// Re-exports for convenience
pub use error::EstrellaError;
pub use printer::PrinterConfig;
pub use transport::BluetoothTransport;
