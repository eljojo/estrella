//! Server state and configuration.

use std::time::{SystemTime, UNIX_EPOCH};

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Path to the printer device (e.g., "/dev/rfcomm0")
    pub device_path: String,
    /// Address to listen on (e.g., "0.0.0.0:8080")
    pub listen_addr: String,
}

/// Application state shared across handlers.
pub struct AppState {
    pub config: ServerConfig,
    /// Unix timestamp of server boot for cache busting.
    pub boot_time: u64,
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        let boot_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self { config, boot_time }
    }
}
