//! # HTTP Server for Receipt and Pattern Printing
//!
//! Provides a web interface for printing text receipts and visual patterns via HTTP.
//!
//! ## Usage
//!
//! ```bash
//! estrella serve --listen 0.0.0.0:8080 --device /dev/rfcomm0
//! ```
//!
//! Then open http://localhost:8080 in a browser to access the UI.

mod handlers;
mod state;
mod static_files;

pub use state::ServerConfig;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::error::EstrellaError;
use state::{AppState, SESSION_EXPIRATION_SECS};

/// Start the HTTP server.
///
/// ## Example
///
/// ```no_run
/// use estrella::server::{serve, ServerConfig};
///
/// # async fn example() -> Result<(), estrella::error::EstrellaError> {
/// let config = ServerConfig {
///     device_path: "/dev/rfcomm0".to_string(),
///     listen_addr: "0.0.0.0:8080".to_string(),
/// };
///
/// serve(config).await?;
/// # Ok(())
/// # }
/// ```
pub async fn serve(config: ServerConfig) -> Result<(), EstrellaError> {
    let app_state = Arc::new(AppState::new(config.clone()));

    // Spawn background cache cleanup task
    tokio::spawn(cleanup_caches(app_state.clone()));

    let app = Router::new()
        // Frontend
        .route("/", get(static_files::index_handler))
        .route("/assets/*path", get(static_files::asset_handler))
        // JSON API
        .route("/api/json/preview", post(handlers::json_api::preview))
        .route("/api/json/print", post(handlers::json_api::print))
        // Receipt API
        .route("/api/receipt/print", post(handlers::receipt::print))
        .route("/api/receipt/preview", post(handlers::receipt::preview))
        // Pattern API
        .route("/api/patterns", get(handlers::patterns::list))
        .route("/api/patterns/:name/params", get(handlers::patterns::params))
        .route(
            "/api/patterns/:name/preview",
            get(handlers::patterns::preview),
        )
        .route(
            "/api/patterns/:name/randomize",
            post(handlers::patterns::randomize),
        )
        .route(
            "/api/patterns/:name/print",
            post(handlers::patterns::print),
        )
        // Weave API
        .route("/api/weave/preview", post(handlers::weave::preview))
        .route("/api/weave/print", post(handlers::weave::print))
        // Composer API
        .route("/api/composer/patterns", get(handlers::composer::patterns))
        .route(
            "/api/composer/blend-modes",
            get(handlers::composer::blend_modes),
        )
        .route(
            "/api/composer/pattern/:name/params",
            get(handlers::composer::pattern_params),
        )
        .route("/api/composer/preview", post(handlers::composer::preview))
        .route("/api/composer/print", post(handlers::composer::print))
        // Photo API (50MB limit for uploads)
        .route(
            "/api/photo/upload",
            post(handlers::photo::upload).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/api/photo/:id/preview", get(handlers::photo::preview))
        .route("/api/photo/:id/print", post(handlers::photo::print))
        .with_state(app_state);

    println!("Estrella HTTP server starting...");
    println!("Listening on: {}", config.listen_addr);
    println!("Printer device: {}", config.device_path);
    println!();
    println!(
        "Open http://{}/ in your browser to print",
        config.listen_addr
    );
    println!();

    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .map_err(|e| {
            EstrellaError::Transport(format!("Failed to bind to {}: {}", config.listen_addr, e))
        })?;

    axum::serve(listener, app)
        .await
        .map_err(|e| EstrellaError::Transport(format!("Server error: {}", e)))?;

    Ok(())
}

/// Background task to clean up expired cache entries.
async fn cleanup_caches(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    let expiration = Duration::from_secs(SESSION_EXPIRATION_SECS);

    loop {
        interval.tick().await;
        let now = Instant::now();

        // Clean up layer cache
        {
            let mut cache = state.layer_cache.write().await;
            let before = cache.len();
            cache.retain(|_, v| now.duration_since(v.last_accessed) < expiration);
            let after = cache.len();
            if before != after {
                println!(
                    "[cache] Cleaned up {} expired layer cache entries ({} remaining)",
                    before - after,
                    after
                );
            }
        }

        // Clean up photo sessions
        {
            let mut sessions = state.photo_sessions.write().await;
            let before = sessions.len();
            sessions.retain(|_, v| now.duration_since(v.last_accessed) < expiration);
            let after = sessions.len();
            if before != after {
                println!(
                    "[cache] Cleaned up {} expired photo sessions ({} remaining)",
                    before - after,
                    after
                );
            }
        }
    }
}
