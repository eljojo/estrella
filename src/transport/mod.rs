//! # Printer Transport Layer
//!
//! This module provides communication backends for sending data to printers.
//!
//! ## Available Transports
//!
//! - [`bluetooth`]: Bluetooth RFCOMM for wireless printing (Linux)
//! - [`usb`]: USB printer class devices (e.g. `/dev/usb/lp0`)
//!
//! ## Transport Auto-Detection
//!
//! Use [`open_transport()`] to automatically select the right transport
//! based on the device path:
//! - Paths containing "rfcomm" → Bluetooth (with TTY configuration)
//! - Everything else → USB (no TTY ioctls)

pub mod bluetooth;
pub mod usb;

pub use bluetooth::BluetoothTransport;
pub use usb::UsbTransport;

use crate::error::EstrellaError;

/// A transport that can write data and send programs to a printer.
///
/// This trait abstracts over Bluetooth and USB transports so callers
/// don't need to know which one is in use.
pub trait Transport {
    /// Write raw bytes to the printer.
    fn write_all(&mut self, data: &[u8]) -> Result<(), EstrellaError>;

    /// Send multiple independent print programs with pauses between them.
    fn send_programs(&mut self, programs: &[crate::ir::Program]) -> Result<(), EstrellaError>;
}

impl Transport for BluetoothTransport {
    fn write_all(&mut self, data: &[u8]) -> Result<(), EstrellaError> {
        BluetoothTransport::write_all(self, data)
    }

    fn send_programs(&mut self, programs: &[crate::ir::Program]) -> Result<(), EstrellaError> {
        BluetoothTransport::send_programs(self, programs)
    }
}

impl Transport for UsbTransport {
    fn write_all(&mut self, data: &[u8]) -> Result<(), EstrellaError> {
        UsbTransport::write_all(self, data)
    }

    fn send_programs(&mut self, programs: &[crate::ir::Program]) -> Result<(), EstrellaError> {
        UsbTransport::send_programs(self, programs)
    }
}

/// Auto-detect and open the appropriate transport for a device path.
///
/// - If the path contains "rfcomm", opens a `BluetoothTransport` (with TTY config)
/// - Otherwise, opens a `UsbTransport` (no TTY ioctls)
///
/// ## Example
///
/// ```no_run
/// use estrella::transport::open_transport;
///
/// // Bluetooth device → BluetoothTransport
/// let mut bt = open_transport("/dev/rfcomm0")?;
/// bt.write_all(&[0x1B, 0x40])?;
///
/// // USB device → UsbTransport
/// let mut usb = open_transport("/dev/usb/lp0")?;
/// usb.write_all(&[0x1B, 0x40])?;
///
/// # Ok::<(), estrella::error::EstrellaError>(())
/// ```
pub fn open_transport(device: &str) -> Result<Box<dyn Transport>, EstrellaError> {
    if device.contains("rfcomm") {
        println!("[transport] Detected Bluetooth device: {}", device);
        Ok(Box::new(BluetoothTransport::open(device)?))
    } else {
        println!("[transport] Detected USB/other device: {}", device);
        Ok(Box::new(UsbTransport::open(device)?))
    }
}
