//! Server state and configuration.

use image::DynamicImage;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;


/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Path to the printer device (e.g., "/dev/rfcomm0")
    pub device_path: String,
    /// Address to listen on (e.g., "0.0.0.0:8080")
    pub listen_addr: String,
}

/// Cache key for rendered layer intensity buffers.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct LayerCacheKey {
    /// Pattern name.
    pub pattern: String,
    /// Hash of pattern parameters (deterministic).
    pub params_hash: u64,
    /// Layer width in pixels.
    pub width: usize,
    /// Layer height in pixels.
    pub height: usize,
}

impl LayerCacheKey {
    /// Create a new cache key from layer parameters.
    pub fn new(pattern: &str, params: &HashMap<String, String>, width: usize, height: usize) -> Self {
        Self {
            pattern: pattern.to_string(),
            params_hash: hash_params(params),
            width,
            height,
        }
    }
}

/// Hash pattern parameters deterministically.
pub fn hash_params(params: &HashMap<String, String>) -> u64 {
    use std::collections::hash_map::DefaultHasher;

    // Sort keys for deterministic ordering
    let mut sorted: Vec<_> = params.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);

    let mut hasher = DefaultHasher::new();
    for (k, v) in sorted {
        k.hash(&mut hasher);
        v.hash(&mut hasher);
    }
    hasher.finish()
}

/// Cached layer intensity buffer (quantized to u8 + gzip compressed).
pub struct CachedLayer {
    /// Compressed intensity data (quantized u8 values, gzip compressed).
    compressed: Vec<u8>,
    /// Original uncompressed size (for allocation hint).
    uncompressed_size: usize,
    /// Last time this cache entry was accessed.
    pub last_accessed: Instant,
}

impl CachedLayer {
    /// Create a new cached layer from f32 intensities.
    /// Quantizes to u8 and compresses with gzip for significant memory savings.
    pub fn new(intensity: Vec<f32>) -> Self {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let uncompressed_size = intensity.len();

        // Quantize f32 -> u8
        let quantized: Vec<u8> = intensity
            .iter()
            .map(|&v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
            .collect();

        // Compress with gzip (fast compression level for speed)
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&quantized).expect("compression failed");
        let compressed = encoder.finish().expect("compression finish failed");

        Self {
            compressed,
            uncompressed_size,
            last_accessed: Instant::now(),
        }
    }

    /// Get intensity values, decompressing and dequantizing back to f32.
    pub fn intensity(&self) -> Vec<f32> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        // Decompress
        let mut decoder = GzDecoder::new(&self.compressed[..]);
        let mut quantized = Vec::with_capacity(self.uncompressed_size);
        decoder.read_to_end(&mut quantized).expect("decompression failed");

        // Dequantize u8 -> f32
        quantized
            .iter()
            .map(|&v| v as f32 / 255.0)
            .collect()
    }

    /// Get compressed size in bytes (for monitoring).
    pub fn compressed_size(&self) -> usize {
        self.compressed.len()
    }

    /// Get original uncompressed size in bytes.
    pub fn uncompressed_size(&self) -> usize {
        self.uncompressed_size
    }

    /// Update last_accessed time.
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
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
    /// Photo sessions for uploaded and downloaded images.
    pub photo_sessions: Arc<RwLock<HashMap<String, PhotoSession>>>,
    /// Cached layer intensity buffers for composer.
    pub layer_cache: Arc<RwLock<HashMap<LayerCacheKey, CachedLayer>>>,
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
            layer_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Session expiration time in seconds (30 minutes).
pub const SESSION_EXPIRATION_SECS: u64 = 30 * 60;
