//! Server state and configuration.

use image::DynamicImage;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Path to the printer device (e.g., "/dev/rfcomm0")
    pub device_path: String,
    /// Address to listen on (e.g., "0.0.0.0:8080")
    pub listen_addr: String,
}

/// A photo session storing an uploaded image.
pub struct PhotoSession {
    /// The decoded image
    pub image: DynamicImage,
    /// Last accessed time (for expiration)
    pub last_accessed: Instant,
}

impl PhotoSession {
    pub fn new(image: DynamicImage) -> Self {
        Self {
            image,
            last_accessed: Instant::now(),
        }
    }

    /// Touch the session to update last_accessed time.
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
}

/// Application state shared across handlers.
pub struct AppState {
    pub config: ServerConfig,
    /// Unix timestamp of server boot for cache busting.
    pub boot_time: u64,
    /// Photo sessions for uploaded images.
    pub photo_sessions: Arc<RwLock<HashMap<Uuid, PhotoSession>>>,
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        let boot_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            config,
            boot_time,
            photo_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Session expiration time in seconds (30 minutes).
pub const SESSION_EXPIRATION_SECS: u64 = 30 * 60;
