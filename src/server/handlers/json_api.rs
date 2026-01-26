//! JSON API handlers for preview and printing.
//!
//! Accepts JSON documents that map to the full component library.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::components::ComponentExt;
use crate::json_api::JsonDocument;
use crate::transport::BluetoothTransport;

use super::super::state::AppState;

/// Handle POST /api/json/preview - render JSON document as PNG.
pub async fn preview(Json(doc): Json<JsonDocument>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let receipt = doc.to_receipt().map_err(|e| {
        (StatusCode::BAD_REQUEST, e.to_string())
    })?;

    let program = receipt.compile();
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
    Json(doc): Json<JsonDocument>,
) -> Response {
    let receipt_data = match doc.to_receipt() {
        Ok(r) => r.build(),
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Html(format!(
                    r#"{{"success": false, "error": "{}"}}"#,
                    e.to_string().replace('"', "\\\"")
                )),
            )
                .into_response();
        }
    };
    let device_path = state.config.device_path.clone();

    let print_result = tokio::task::spawn_blocking(move || {
        let mut transport = BluetoothTransport::open(&device_path)?;
        transport.write_all(&receipt_data)?;
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
