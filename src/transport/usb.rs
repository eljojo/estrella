//! # USB Printer Transport
//!
//! This module provides communication with Star Micronics printers over
//! USB, using the kernel's USB printer class driver (e.g. `/dev/usb/lp0`).
//!
//! ## Differences from Bluetooth Transport
//!
//! - **No TTY configuration**: USB printer devices are not TTY devices, so
//!   `tcgetattr`/`tcsetattr` are skipped entirely.
//! - **No `tcdrain`**: USB printer devices don't support TTY ioctls. Instead,
//!   writes are paced with simple `flush()` calls between chunks.
//! - **Write-only**: The device file is opened in write-only mode.
//!
//! ## USB Setup (Linux)
//!
//! The Star printer should appear as a USB printer class device automatically:
//!
//! ```bash
//! # Check for USB printer device
//! $ ls /dev/usb/lp*
//! /dev/usb/lp0
//!
//! # Ensure permissions (may need root or lp group)
//! $ sudo chmod 666 /dev/usb/lp0
//! # Or add user to lp group:
//! $ sudo usermod -a -G lp $USER
//! ```

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration;

use crate::error::EstrellaError;

/// Default USB printer device path
pub const DEFAULT_DEVICE: &str = "/dev/usb/lp0";

/// Default chunk size for writes (bytes)
const CHUNK_SIZE: usize = 4096;

/// Delay between independent print jobs (milliseconds).
/// Gives the printer time to finish processing the current job before
/// receiving the next Init command.
const JOB_DELAY_MS: u64 = 2000;

/// # USB Printer Transport
///
/// Manages a connection to a Star printer over USB.
///
/// ## Example
///
/// ```no_run
/// use estrella::transport::usb::UsbTransport;
/// use estrella::protocol::commands;
///
/// let mut transport = UsbTransport::open("/dev/usb/lp0")?;
///
/// // Send initialization
/// transport.write_all(&commands::init())?;
///
/// // Send more data...
///
/// # Ok::<(), estrella::error::EstrellaError>(())
/// ```
pub struct UsbTransport {
    file: File,
    chunk_size: usize,
}

impl UsbTransport {
    /// Open a USB connection to the printer.
    ///
    /// ## Parameters
    ///
    /// - `device`: Path to the USB printer device (e.g., "/dev/usb/lp0")
    ///
    /// ## Notes
    ///
    /// Unlike `BluetoothTransport`, no TTY configuration is performed.
    /// The device is opened in write-only mode.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The device doesn't exist
    /// - Permission denied (may need root or lp group membership)
    pub fn open<P: AsRef<Path>>(device: P) -> Result<Self, EstrellaError> {
        let path = device.as_ref();

        let file = OpenOptions::new().write(true).open(path).map_err(|e| {
            EstrellaError::Transport(format!("Failed to open {}: {}", path.display(), e))
        })?;

        Ok(Self {
            file,
            chunk_size: CHUNK_SIZE,
        })
    }

    /// Open with default device path (/dev/usb/lp0)
    pub fn open_default() -> Result<Self, EstrellaError> {
        Self::open(DEFAULT_DEVICE)
    }

    /// Set the chunk size for large writes.
    ///
    /// Larger chunks are faster but may overflow the printer's buffer.
    /// Default is 4096 bytes.
    pub fn set_chunk_size(&mut self, size: usize) {
        self.chunk_size = size;
    }

    /// Write data to the printer.
    ///
    /// Small writes are sent directly. Large writes are automatically
    /// chunked to avoid buffer overflow.
    pub fn write_all(&mut self, data: &[u8]) -> Result<(), EstrellaError> {
        self.write_segment(data)?;

        self.file
            .flush()
            .map_err(|e| EstrellaError::Transport(format!("Flush failed: {}", e)))?;

        Ok(())
    }

    /// Send multiple independent print programs with pauses between them.
    ///
    /// Each program is sent completely, then the transport pauses to let
    /// the printer process it before sending the next. This prevents
    /// buffer overflow during long prints with large graphics.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// use estrella::transport::usb::UsbTransport;
    /// use estrella::ir::Program;
    ///
    /// let programs: Vec<Program> = program.split_for_long_print();
    /// let mut transport = UsbTransport::open("/dev/usb/lp0")?;
    /// transport.send_programs(&programs)?;
    /// ```
    pub fn send_programs(&mut self, programs: &[crate::ir::Program]) -> Result<(), EstrellaError> {
        let total = programs.len();
        println!("[send_programs] Sending {} program(s) to printer via USB", total);

        for (i, program) in programs.iter().enumerate() {
            let bytes = program.to_bytes();
            println!(
                "[send_programs] Job {}/{}: {} bytes",
                i + 1,
                total,
                bytes.len()
            );

            self.write_segment(&bytes)?;

            self.file
                .flush()
                .map_err(|e| EstrellaError::Transport(format!("Flush failed: {}", e)))?;

            // Pause between jobs (but not after the last one)
            if i < programs.len() - 1 {
                println!(
                    "[send_programs] Pausing {}ms for printer to process...",
                    JOB_DELAY_MS
                );
                thread::sleep(Duration::from_millis(JOB_DELAY_MS));
            }
        }

        println!("[send_programs] All jobs sent successfully via USB");
        Ok(())
    }

    /// Write a segment of data with chunking and flush pacing.
    ///
    /// Data is written in 4KB chunks. After each chunk, `flush()` ensures
    /// data has been handed to the kernel. Unlike the Bluetooth transport,
    /// no `tcdrain` is used since USB printer devices are not TTY devices.
    fn write_segment(&mut self, data: &[u8]) -> Result<(), EstrellaError> {
        if data.is_empty() {
            return Ok(());
        }

        if data.len() <= self.chunk_size {
            // Small write - send directly
            self.file
                .write_all(data)
                .map_err(|e| EstrellaError::Transport(format!("Write failed: {}", e)))?;
        } else {
            // Large write - chunk it with flush pacing
            for chunk in data.chunks(self.chunk_size) {
                self.file
                    .write_all(chunk)
                    .map_err(|e| EstrellaError::Transport(format!("Write failed: {}", e)))?;

                // Flush after each chunk to pace writes.
                // USB printer devices don't support tcdrain, so we use
                // flush() to push data from userspace to the kernel.
                self.file
                    .flush()
                    .map_err(|e| EstrellaError::Transport(format!("Flush failed: {}", e)))?;
            }
        }

        Ok(())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_device_path() {
        assert_eq!(DEFAULT_DEVICE, "/dev/usb/lp0");
    }

    // Note: Most transport tests require actual hardware.
    // Integration tests should be run manually with a connected printer.
}
