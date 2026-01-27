//! JSON API handlers for preview and printing.
//!
//! Accepts JSON documents using the unified Document model.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::document::{Document, ImageResolver};
use crate::transport::BluetoothTransport;

use super::super::state::AppState;

/// Handle POST /api/json/preview - render JSON document as PNG.
pub async fn preview(
    State(state): State<Arc<AppState>>,
    Json(mut doc): Json<Document>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Resolve images from URLs before compilation
    let resolver = ImageResolver::new(state.photo_sessions.clone());
    resolver
        .resolve(&mut doc)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Image resolution failed: {}", e)))?;

    let program = doc.compile();
    let png_bytes = program.to_preview_png().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Preview render failed: {}", e),
        )
    })?;

    Ok(([(header::CONTENT_TYPE, "image/png")], png_bytes))
}

/// Handle POST /api/json/print - print JSON document to device.
pub async fn print(
    State(state): State<Arc<AppState>>,
    Json(mut doc): Json<Document>,
) -> Response {
    // Resolve images from URLs before compilation
    let resolver = ImageResolver::new(state.photo_sessions.clone());
    if let Err(e) = resolver.resolve(&mut doc).await {
        return (
            StatusCode::BAD_REQUEST,
            Html(format!(
                r#"{{"success": false, "error": "Image resolution failed: {}"}}"#,
                e
            )),
        )
            .into_response();
    }

    match serde_json::to_string_pretty(&doc) {
        Ok(json) => eprintln!("=== JSON Print ===\n{}\n==================", json),
        Err(e) => eprintln!("(failed to serialize document for logging: {})", e),
    }

    let print_data = doc.build();
    let device_path = state.config.device_path.clone();

    let print_result = tokio::task::spawn_blocking(move || {
        let mut transport = BluetoothTransport::open(&device_path)?;
        transport.write_all(&print_data)?;
        Ok::<_, crate::EstrellaError>(())
    })
    .await;

    match print_result {
        Ok(Ok(())) => (
            StatusCode::OK,
            Html(r#"{"success": true, "message": "Document printed successfully"}"#.to_string()),
        )
            .into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                r#"{{"success": false, "error": "Print failed: {}"}}"#,
                e
            )),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                r#"{{"success": false, "error": "Task error: {}"}}"#,
                e
            )),
        )
            .into_response(),
    }
}
