//! # Bluetooth RFCOMM Transport
//!
//! This module provides communication with Star Micronics printers over
//! Bluetooth Serial Port Profile (SPP) via RFCOMM.
//!
//! ## Bluetooth Setup (Linux)
//!
//! Before using this transport, the printer must be paired and bound to an
//! RFCOMM device:
//!
//! ```bash
//! # 1. Find the printer's Bluetooth address
//! $ bluetoothctl
//! [bluetooth]# scan on
//! # Look for "Star Micronics" or "TSP650II"
//! # Note the address, e.g., 00:11:62:XX:XX:XX
//!
//! # 2. Pair with the printer
//! [bluetooth]# pair 00:11:62:XX:XX:XX
//!
//! # 3. Bind to RFCOMM device
//! $ sudo rfcomm bind 0 00:11:62:XX:XX:XX
//! # This creates /dev/rfcomm0
//! ```
//!
//! ## TTY Configuration
//!
//! The RFCOMM device is opened in raw mode to ensure binary data is
//! transmitted without modification:
//!
//! - **No input processing**: Disable IGNBRK, BRKINT, PARMRK, ISTRIP, etc.
//! - **No output processing**: Disable OPOST (no CR/LF translation)
//! - **8-bit characters**: CS8 (8 data bits, no parity)
//! - **No echo**: Disable ECHO, ECHONL
//! - **Non-canonical mode**: Disable ICANON (no line buffering)
//!
//! ## Chunked Writes
//!
//! Large data blocks are written in chunks to avoid overwhelming the
//! Bluetooth buffer. The default chunk size is 4096 bytes with a small
//! delay between chunks.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::thread;
use std::time::Duration;

use crate::error::EstrellaError;

/// Default RFCOMM device path
pub const DEFAULT_DEVICE: &str = "/dev/rfcomm0";

/// Default chunk size for writes (bytes)
const CHUNK_SIZE: usize = 4096;

/// Delay between chunks (milliseconds)
/// Throttles writes to ~200 KB/s to match Bluetooth SPP throughput.
/// Too fast = printer buffer starvation on large prints (>300mm).
const CHUNK_DELAY_MS: u64 = 20;

/// # Bluetooth Printer Transport
///
/// Manages a connection to a Star printer over Bluetooth RFCOMM.
///
/// ## Example
///
/// ```no_run
/// use estrella::transport::bluetooth::BluetoothTransport;
/// use estrella::protocol::commands;
///
/// let mut transport = BluetoothTransport::open("/dev/rfcomm0")?;
///
/// // Send initialization
/// transport.write_all(&commands::init())?;
///
/// // Send more data...
///
/// # Ok::<(), estrella::error::EstrellaError>(())
/// ```
pub struct BluetoothTransport {
    file: File,
    chunk_size: usize,
    chunk_delay: Duration,
}

impl BluetoothTransport {
    /// Open a Bluetooth connection to the printer.
    ///
    /// ## Parameters
    ///
    /// - `device`: Path to the RFCOMM device (e.g., "/dev/rfcomm0")
    ///
    /// ## TTY Configuration
    ///
    /// The device is configured for raw binary communication:
    /// - 8-bit characters, no parity
    /// - No input/output processing
    /// - No echo or canonical mode
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The device doesn't exist
    /// - Permission denied (may need root or dialout group)
    /// - TTY configuration fails
    pub fn open<P: AsRef<Path>>(device: P) -> Result<Self, EstrellaError> {
        let path = device.as_ref();

        let file = OpenOptions::new().write(true).open(path).map_err(|e| {
            EstrellaError::Transport(format!("Failed to open {}: {}", path.display(), e))
        })?;

        // Configure TTY for raw mode
        configure_tty_raw(file.as_raw_fd())?;

        Ok(Self {
            file,
            chunk_size: CHUNK_SIZE,
            chunk_delay: Duration::from_millis(CHUNK_DELAY_MS),
        })
    }

    /// Open with default device path (/dev/rfcomm0)
    pub fn open_default() -> Result<Self, EstrellaError> {
        Self::open(DEFAULT_DEVICE)
    }

    /// Set the chunk size for large writes.
    ///
    /// Larger chunks are faster but may overflow the Bluetooth buffer.
    /// Default is 4096 bytes.
    pub fn set_chunk_size(&mut self, size: usize) {
        self.chunk_size = size;
    }

    /// Set the delay between chunks.
    ///
    /// Longer delays give the printer more time to process data.
    /// Default is 2ms.
    pub fn set_chunk_delay(&mut self, delay: Duration) {
        self.chunk_delay = delay;
    }

    /// Write data to the printer.
    ///
    /// Small writes are sent directly. Large writes are automatically
    /// chunked to avoid buffer overflow.
    pub fn write_all(&mut self, data: &[u8]) -> Result<(), EstrellaError> {
        if data.len() <= self.chunk_size {
            // Small write - send directly
            self.file
                .write_all(data)
                .map_err(|e| EstrellaError::Transport(format!("Write failed: {}", e)))?;
        } else {
            // Large write - chunk it
            for chunk in data.chunks(self.chunk_size) {
                self.file
                    .write_all(chunk)
                    .map_err(|e| EstrellaError::Transport(format!("Write failed: {}", e)))?;

                if !self.chunk_delay.is_zero() {
                    thread::sleep(self.chunk_delay);
                }
            }
        }

        self.file
            .flush()
            .map_err(|e| EstrellaError::Transport(format!("Flush failed: {}", e)))?;

        Ok(())
    }
}

/// Configure a file descriptor for raw TTY mode.
///
/// This disables all input/output processing so binary data passes through
/// unmodified. Essential for printer communication.
///
/// ## What Gets Disabled
///
/// - **Input flags**: IGNBRK, BRKINT, PARMRK, ISTRIP, INLCR, IGNCR, ICRNL, IXON, IXOFF, IXANY
/// - **Output flags**: OPOST
/// - **Local flags**: ECHO, ECHONL, ICANON, ISIG, IEXTEN
/// - **Control flags**: CSIZE, PARENB (then CS8 is set)
///
/// Note: IXON/IXOFF/IXANY disable XON/XOFF software flow control. This is critical
/// because 0x11 (XON/DC1) and 0x13 (XOFF/DC3) can appear in binary raster data.
#[cfg(unix)]
fn configure_tty_raw(fd: i32) -> Result<(), EstrellaError> {
    use std::mem::MaybeUninit;

    // Get current terminal attributes
    let mut termios = MaybeUninit::uninit();
    let result = unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) };
    if result != 0 {
        return Err(EstrellaError::Transport(format!(
            "tcgetattr failed: {}",
            io::Error::last_os_error()
        )));
    }
    let mut termios = unsafe { termios.assume_init() };

    // Input flags: disable all processing
    // IXON/IXOFF/IXANY: disable XON/XOFF flow control (0x11/0x13 could appear in binary data)
    // Matches Python: attrs[0] &= ~(... | IXON | IXOFF | IXANY)
    termios.c_iflag &= !(libc::IGNBRK
        | libc::BRKINT
        | libc::PARMRK
        | libc::ISTRIP
        | libc::INLCR
        | libc::IGNCR
        | libc::ICRNL
        | libc::IXON
        | libc::IXOFF
        | libc::IXANY);

    // Output flags: disable post-processing
    termios.c_oflag &= !libc::OPOST;

    // Local flags: disable echo, canonical mode, signals
    termios.c_lflag &= !(libc::ECHO | libc::ECHONL | libc::ICANON | libc::ISIG | libc::IEXTEN);

    // Control flags: 8-bit characters, no parity
    termios.c_cflag &= !(libc::CSIZE | libc::PARENB);
    termios.c_cflag |= libc::CS8;

    // Apply settings immediately
    let result = unsafe { libc::tcsetattr(fd, libc::TCSANOW, &termios) };
    if result != 0 {
        return Err(EstrellaError::Transport(format!(
            "tcsetattr failed: {}",
            io::Error::last_os_error()
        )));
    }

    Ok(())
}

#[cfg(not(unix))]
fn configure_tty_raw(_fd: i32) -> Result<(), EstrellaError> {
    // On non-Unix platforms, skip TTY configuration
    // The device may work differently
    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_device_path() {
        assert_eq!(DEFAULT_DEVICE, "/dev/rfcomm0");
    }

    // Note: Most transport tests require actual hardware.
    // Integration tests should be run manually with a connected printer.
}
