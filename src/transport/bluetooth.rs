//! # Bluetooth RFCOMM Transport
//!
//! This module provides communication with Star Micronics printers over
//! Bluetooth Serial Port Profile (SPP) via RFCOMM.
//!
//! ## Long Print Support
//!
//! For prints longer than ~100mm, the printer's internal buffer can overflow.
//! Use `send_programs()` to send multiple independent print jobs with pauses
//! between them, allowing the printer to process each job completely before
//! receiving the next.
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

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::error::EstrellaError;

/// Default RFCOMM device path
pub const DEFAULT_DEVICE: &str = "/dev/rfcomm0";

/// Default chunk size for writes (bytes)
const CHUNK_SIZE: usize = 4096;

/// Delay between chunks (milliseconds)
const CHUNK_DELAY_MS: u64 = 2;

/// Delay between independent print jobs (milliseconds)
/// This gives the printer time to process each job completely.
const JOB_DELAY_MS: u64 = 1000;

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
        self.write_segment(data)?;

        self.file
            .flush()
            .map_err(|e| EstrellaError::Transport(format!("Flush failed: {}", e)))?;

        Ok(())
    }

    /// Send multiple independent print programs with pauses between them.
    ///
    /// Each program is sent completely, then the transport pauses for 1 second
    /// to let the printer process it before sending the next. This prevents
    /// buffer overflow during long prints with large graphics.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// use estrella::transport::bluetooth::BluetoothTransport;
    /// use estrella::ir::Program;
    ///
    /// let programs: Vec<Program> = program.split_for_long_print();
    /// let mut transport = BluetoothTransport::open("/dev/rfcomm0")?;
    /// transport.send_programs(&programs)?;
    /// ```
    pub fn send_programs(&mut self, programs: &[crate::ir::Program]) -> Result<(), EstrellaError> {
        for (i, program) in programs.iter().enumerate() {
            let bytes = program.to_bytes();
            self.write_segment(&bytes)?;

            self.file
                .flush()
                .map_err(|e| EstrellaError::Transport(format!("Flush failed: {}", e)))?;

            // Pause between jobs (but not after the last one)
            if i < programs.len() - 1 {
                thread::sleep(Duration::from_millis(JOB_DELAY_MS));
            }
        }

        Ok(())
    }

    /// Write a segment of data with chunking.
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
// RFCOMM SETUP HELPERS
// ============================================================================

/// Validate a Bluetooth MAC address format (XX:XX:XX:XX:XX:XX).
pub fn is_valid_mac(mac: &str) -> bool {
    let parts: Vec<&str> = mac.split(':').collect();
    if parts.len() != 6 {
        return false;
    }
    parts
        .iter()
        .all(|part| part.len() == 2 && part.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Find an existing RFCOMM device bound to the given MAC address.
///
/// Checks `/proc/net/rfcomm` and falls back to `rfcomm -a` command.
/// Returns the device path (e.g., "/dev/rfcomm0") if found.
#[cfg(unix)]
pub fn find_rfcomm_for_mac(mac: &str) -> Result<Option<String>, EstrellaError> {
    let mac_upper = mac.to_uppercase();

    // Try /proc/net/rfcomm first (format: "rfcomm0: XX:XX:XX:XX:XX:XX channel N ...")
    if let Ok(contents) = fs::read_to_string("/proc/net/rfcomm") {
        for line in contents.lines() {
            if line.to_uppercase().contains(&mac_upper) {
                if let Some(dev_name) = line.split(':').next() {
                    let dev_name = dev_name.trim();
                    let device_path = format!("/dev/{}", dev_name);
                    if Path::new(&device_path).exists() {
                        return Ok(Some(device_path));
                    }
                }
            }
        }
    }

    // Fallback: rfcomm -a command
    let output = Command::new("rfcomm")
        .arg("-a")
        .output()
        .map_err(|e| EstrellaError::Transport(format!("Failed to run 'rfcomm -a': {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.to_uppercase().contains(&mac_upper) {
            if let Some(dev_name) = line.split(':').next() {
                let dev_name = dev_name.trim();
                let device_path = format!("/dev/{}", dev_name);
                if Path::new(&device_path).exists() {
                    return Ok(Some(device_path));
                }
            }
        }
    }

    Ok(None)
}

#[cfg(not(unix))]
pub fn find_rfcomm_for_mac(_mac: &str) -> Result<Option<String>, EstrellaError> {
    Ok(None)
}

/// Set up an RFCOMM device for a Bluetooth MAC address.
///
/// Runs:
/// 1. `bluetoothctl connect <MAC>` - connect to device
/// 2. `l2ping -c 1 <MAC>` - verify connectivity
/// 3. `rfcomm bind <channel> <MAC> 1` - create /dev/rfcommN
///
/// Returns the device path on success (e.g., "/dev/rfcomm0").
///
/// **Requires root privileges** for `rfcomm bind`.
#[cfg(unix)]
pub fn setup_rfcomm(mac: &str, channel: u8) -> Result<String, EstrellaError> {
    let mac_upper = mac.to_uppercase();
    let device_path = format!("/dev/rfcomm{}", channel);

    // Step 1: Connect via bluetoothctl (may fail if already connected, that's ok)
    eprintln!("Connecting to {}...", mac_upper);
    let output = Command::new("bluetoothctl")
        .arg("connect")
        .arg(&mac_upper)
        .output()
        .map_err(|e| EstrellaError::Transport(format!("Failed to run bluetoothctl: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("Connection successful") || stdout.contains("already connected") {
        eprintln!("Connected.");
    } else {
        eprintln!("bluetoothctl returned: {}", stdout.trim());
        // Continue anyway - l2ping will verify
    }

    // Small delay for connection to stabilize
    thread::sleep(Duration::from_millis(500));

    // Step 2: Verify connectivity with l2ping
    eprintln!("Verifying connectivity...");
    let output = Command::new("l2ping")
        .arg("-c")
        .arg("1")
        .arg(&mac_upper)
        .output()
        .map_err(|e| EstrellaError::Transport(format!("Failed to run l2ping: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(EstrellaError::Transport(format!(
            "Device {} not reachable: {}",
            mac_upper,
            stderr.trim()
        )));
    }
    eprintln!("Device reachable.");

    // Step 3: Bind RFCOMM
    eprintln!("Binding rfcomm{}...", channel);
    let output = Command::new("rfcomm")
        .arg("bind")
        .arg(channel.to_string())
        .arg(&mac_upper)
        .arg("1") // RFCOMM channel 1 (standard for SPP)
        .output()
        .map_err(|e| EstrellaError::Transport(format!("Failed to run rfcomm bind: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(EstrellaError::Transport(format!(
            "rfcomm bind failed: {}",
            stderr.trim()
        )));
    }

    // Wait for device to appear
    thread::sleep(Duration::from_millis(500));

    if !Path::new(&device_path).exists() {
        return Err(EstrellaError::Transport(format!(
            "Device {} was not created",
            device_path
        )));
    }

    eprintln!("Created {}", device_path);
    Ok(device_path)
}

#[cfg(not(unix))]
pub fn setup_rfcomm(_mac: &str, _channel: u8) -> Result<String, EstrellaError> {
    Err(EstrellaError::Transport(
        "RFCOMM setup not supported on this platform".to_string(),
    ))
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

    #[test]
    fn test_valid_mac_addresses() {
        assert!(is_valid_mac("00:11:22:33:44:55"));
        assert!(is_valid_mac("AA:BB:CC:DD:EE:FF"));
        assert!(is_valid_mac("aa:bb:cc:dd:ee:ff"));
        assert!(is_valid_mac("00:00:00:00:00:00"));
    }

    #[test]
    fn test_invalid_mac_addresses() {
        assert!(!is_valid_mac("00:11:22:33:44")); // too short
        assert!(!is_valid_mac("00:11:22:33:44:55:66")); // too long
        assert!(!is_valid_mac("00-11-22-33-44-55")); // wrong separator
        assert!(!is_valid_mac("GG:HH:II:JJ:KK:LL")); // invalid hex
        assert!(!is_valid_mac("")); // empty
        assert!(!is_valid_mac("not-a-mac")); // garbage
    }

    // Note: Most transport tests require actual hardware.
    // Integration tests should be run manually with a connected printer.
}
