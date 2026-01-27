//! Render context — shared resources available to patterns during preparation.
//!
//! Patterns receive a `RenderContext` in their `prepare()` method. Most patterns
//! ignore it entirely. Patterns that need external resources (e.g., downloading
//! an image) use the context to access shared infrastructure like HTTP clients
//! and caches, keeping callers unaware of what happens behind the scenes.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::server::{CachedIntensity, IntensityCacheKey, PhotoSession};

/// Shared resources available to patterns during `prepare()`.
///
/// Constructed once per request (or per server lifetime) and passed through
/// to all patterns. Patterns reach into it for what they need; most ignore
/// it entirely.
pub struct RenderContext {
    /// HTTP client for downloading external resources.
    pub http_client: reqwest::Client,
    /// Shared image cache (downloaded images, photo uploads).
    pub image_cache: Arc<RwLock<HashMap<String, PhotoSession>>>,
    /// Cached rendered intensity buffers (compressed, cross-request).
    pub intensity_cache: Arc<RwLock<HashMap<IntensityCacheKey, CachedIntensity>>>,
}

impl RenderContext {
    /// Create a context from shared state.
    pub fn new(
        http_client: reqwest::Client,
        image_cache: Arc<RwLock<HashMap<String, PhotoSession>>>,
        intensity_cache: Arc<RwLock<HashMap<IntensityCacheKey, CachedIntensity>>>,
    ) -> Self {
        Self {
            http_client,
            image_cache,
            intensity_cache,
        }
    }

    /// Create a minimal context for non-server use (CLI, tests).
    /// Has an HTTP client but empty caches.
    pub fn empty() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .user_agent("estrella/0.1")
                .build()
                .expect("failed to build HTTP client"),
            image_cache: Arc::new(RwLock::new(HashMap::new())),
            intensity_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Look up a cached intensity buffer, or compute and cache it.
    pub async fn get_or_render_intensity(
        &self,
        key: IntensityCacheKey,
        compute: impl FnOnce() -> Vec<f32>,
    ) -> Vec<f32> {
        // Check cache
        {
            let mut cache = self.intensity_cache.write().await;
            if let Some(entry) = cache.get_mut(&key) {
                entry.touch();
                return entry.intensity();
            }
        }

        // Cache miss — compute
        let buffer = compute();

        // Store compressed
        let cached = CachedIntensity::new(&buffer);
        {
            let mut cache = self.intensity_cache.write().await;
            cache.insert(key, cached);
        }

        buffer
    }
}
