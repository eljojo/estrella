//! # Bluetooth RFCOMM Transport
//!
//! This module provides communication with Star Micronics printers over
//! Bluetooth Serial Port Profile (SPP) via RFCOMM.
//!
//! ## Long Print Support
//!
//! For prints longer than ~100mm, the printer's internal buffer can overflow.
//! This transport automatically handles "drain markers" inserted by the IR
//! chunking pass. When a drain marker is encountered, the transport pauses
//! for 1 second to let the printer catch up.
//!
//! The drain marker is a 9-byte sequence: `ESC NUL "DRAIN" NUL ESC`
//! It's stripped from the output and replaced with a pause.
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

/// Delay when drain marker is encountered (milliseconds)
/// This gives the printer time to process buffered data.
const DRAIN_DELAY_MS: u64 = 1000;

/// Import drain marker from IR module
use crate::ir::DRAIN_MARKER;

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
    ///
    /// ## Drain Markers
    ///
    /// If the data contains drain markers (from `insert_drain_points()`),
    /// the transport will:
    /// 1. Send all data before the marker
    /// 2. Flush and wait 1 second
    /// 3. Continue with remaining data
    ///
    /// This prevents buffer overflow during long prints.
    pub fn write_all(&mut self, data: &[u8]) -> Result<(), EstrellaError> {
        // Split data on drain markers and process each segment
        let segments = split_on_drain_markers(data);

        for (i, segment) in segments.iter().enumerate() {
            // Write segment with chunking
            self.write_segment(segment)?;

            // If not the last segment, this means we hit a drain marker
            // Flush and wait for printer to catch up
            if i < segments.len() - 1 {
                self.file
                    .flush()
                    .map_err(|e| EstrellaError::Transport(format!("Flush failed: {}", e)))?;
                thread::sleep(Duration::from_millis(DRAIN_DELAY_MS));
            }
        }

        self.file
            .flush()
            .map_err(|e| EstrellaError::Transport(format!("Flush failed: {}", e)))?;

        Ok(())
    }

    /// Write a segment of data with chunking (no drain marker handling).
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

/// Split data on drain markers, returning segments without the markers.
///
/// Example: `[A, A, DRAIN, B, B, DRAIN, C]` → `[[A, A], [B, B], [C]]`
fn split_on_drain_markers(data: &[u8]) -> Vec<&[u8]> {
    let mut segments = Vec::new();
    let mut start = 0;

    while start < data.len() {
        // Search for drain marker starting at current position
        if let Some(pos) = find_drain_marker(&data[start..]) {
            // Add segment before marker (may be empty)
            segments.push(&data[start..start + pos]);
            // Skip past the marker
            start = start + pos + DRAIN_MARKER.len();
        } else {
            // No more markers, add remaining data
            segments.push(&data[start..]);
            break;
        }
    }

    // If data ended with a marker, we might have no trailing segment
    if segments.is_empty() {
        segments.push(&[] as &[u8]);
    }

    segments
}

/// Find the position of a drain marker in the data.
fn find_drain_marker(data: &[u8]) -> Option<usize> {
    if data.len() < DRAIN_MARKER.len() {
        return None;
    }
    data.windows(DRAIN_MARKER.len())
        .position(|window| window == DRAIN_MARKER)
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

    // ========== Drain Marker Tests ==========

    #[test]
    fn test_find_drain_marker_present() {
        let mut data = vec![0x01, 0x02, 0x03];
        data.extend(DRAIN_MARKER);
        data.extend(&[0x04, 0x05]);

        let pos = find_drain_marker(&data);
        assert_eq!(pos, Some(3));
    }

    #[test]
    fn test_find_drain_marker_absent() {
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let pos = find_drain_marker(&data);
        assert_eq!(pos, None);
    }

    #[test]
    fn test_find_drain_marker_at_start() {
        let mut data = DRAIN_MARKER.to_vec();
        data.extend(&[0x01, 0x02]);

        let pos = find_drain_marker(&data);
        assert_eq!(pos, Some(0));
    }

    #[test]
    fn test_find_drain_marker_at_end() {
        let mut data = vec![0x01, 0x02];
        data.extend(DRAIN_MARKER);

        let pos = find_drain_marker(&data);
        assert_eq!(pos, Some(2));
    }

    #[test]
    fn test_split_no_markers() {
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let segments = split_on_drain_markers(&data);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0], &[0x01, 0x02, 0x03, 0x04, 0x05]);
    }

    #[test]
    fn test_split_one_marker() {
        let mut data = vec![0x01, 0x02];
        data.extend(DRAIN_MARKER);
        data.extend(&[0x03, 0x04]);

        let segments = split_on_drain_markers(&data);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], &[0x01, 0x02]);
        assert_eq!(segments[1], &[0x03, 0x04]);
    }

    #[test]
    fn test_split_multiple_markers() {
        let mut data = vec![0x01];
        data.extend(DRAIN_MARKER);
        data.extend(&[0x02, 0x03]);
        data.extend(DRAIN_MARKER);
        data.extend(&[0x04]);

        let segments = split_on_drain_markers(&data);

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], &[0x01]);
        assert_eq!(segments[1], &[0x02, 0x03]);
        assert_eq!(segments[2], &[0x04]);
    }

    #[test]
    fn test_split_marker_at_start() {
        let mut data = DRAIN_MARKER.to_vec();
        data.extend(&[0x01, 0x02]);

        let segments = split_on_drain_markers(&data);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], &[] as &[u8]); // Empty segment before marker
        assert_eq!(segments[1], &[0x01, 0x02]);
    }

    #[test]
    fn test_split_marker_at_end() {
        let mut data = vec![0x01, 0x02];
        data.extend(DRAIN_MARKER);

        let segments = split_on_drain_markers(&data);

        // Should have the segment before marker, then nothing after
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0], &[0x01, 0x02]);
    }

    #[test]
    fn test_split_consecutive_markers() {
        let mut data = vec![0x01];
        data.extend(DRAIN_MARKER);
        data.extend(DRAIN_MARKER); // Two markers in a row
        data.extend(&[0x02]);

        let segments = split_on_drain_markers(&data);

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], &[0x01]);
        assert_eq!(segments[1], &[] as &[u8]); // Empty between markers
        assert_eq!(segments[2], &[0x02]);
    }

    #[test]
    fn test_split_empty_data() {
        let data: Vec<u8> = vec![];
        let segments = split_on_drain_markers(&data);

        assert_eq!(segments.len(), 1);
        assert!(segments[0].is_empty());
    }

    // Note: Most transport tests require actual hardware.
    // Integration tests should be run manually with a connected printer.
}
