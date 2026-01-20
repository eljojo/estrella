//! # Printer Transport Layer
//!
//! This module provides communication backends for sending data to printers.
//!
//! ## Available Transports
//!
//! - [`bluetooth`]: Bluetooth RFCOMM for wireless printing (Linux)
//!
//! ## Future Transports
//!
//! - USB serial
//! - Network (TCP/IP)
//! - Mock transport for testing

pub mod bluetooth;

pub use bluetooth::BluetoothTransport;
