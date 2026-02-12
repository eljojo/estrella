//! JSON API handlers for preview and printing.
//!
//! Accepts JSON documents using the unified Document model.

use axum::{
    Json,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::document::canvas::ElementLayout;
use crate::document::{self, Component, Document, ImageResolver};
use crate::ir::{Op, Program};
use crate::preview::{measure_cursor_y, measure_preview};
use crate::transport::BluetoothTransport;

use super::super::state::AppState;

/// Handle POST /api/json/preview - render JSON document as PNG.
pub async fn preview(
    State(state): State<Arc<AppState>>,
    Json(mut doc): Json<Document>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Resolve images from URLs before compilation
    let resolver = ImageResolver::new(state.photo_sessions.clone());
    resolver.resolve(&mut doc).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Image resolution failed: {}", e),
        )
    })?;

    let program = doc.compile();
    let png_bytes = program.to_preview_png().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Preview render failed: {}", e),
        )
    })?;

    Ok(([(header::CONTENT_TYPE, "image/png")], png_bytes))
}

/// Request body for canvas-layout endpoint.
#[derive(Deserialize)]
pub struct CanvasLayoutRequest {
    document: Vec<Component>,
    canvas_index: usize,
    #[serde(default)]
    cut: bool,
}

/// Response for canvas-layout: element bounding boxes + document positioning.
#[derive(Serialize)]
pub struct CanvasLayoutResponse {
    pub width: usize,
    pub height: usize,
    pub y_offset: usize,
    pub document_height: usize,
    pub elements: Vec<ElementLayout>,
}

/// Handle POST /api/json/canvas-layout - compute element bounding boxes for canvas overlay.
pub async fn canvas_layout(
    Json(req): Json<CanvasLayoutRequest>,
) -> Result<Json<CanvasLayoutResponse>, (StatusCode, String)> {
    let canvas_component = req.document.get(req.canvas_index).ok_or((
        StatusCode::BAD_REQUEST,
        format!("Invalid canvas_index: {}", req.canvas_index),
    ))?;

    let canvas = match canvas_component {
        Component::Canvas(c) => c,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Component at canvas_index is not a canvas".to_string(),
            ));
        }
    };

    let layout = canvas.compute_layout();

    // Compute Y offset using cursor position (where the canvas starts in the
    // preview image), and document height using trimmed buffer height (matching
    // the actual preview PNG dimensions).
    let mut prefix_ops = vec![Op::Init, Op::SetCodepage(1)];
    for comp in &req.document[..req.canvas_index] {
        comp.emit(&mut prefix_ops);
    }
    let y_offset = measure_cursor_y(&Program { ops: prefix_ops }).unwrap_or(0);

    let mut all_ops = vec![Op::Init, Op::SetCodepage(1)];
    for comp in &req.document {
        comp.emit(&mut all_ops);
    }
    if req.cut {
        all_ops.push(Op::Cut { partial: true });
    }
    let document_height = measure_preview(&Program { ops: all_ops }).unwrap_or(0);

    Ok(Json(CanvasLayoutResponse {
        width: layout.width,
        height: layout.height,
        y_offset,
        document_height,
        elements: layout.elements,
    }))
}

/// Handle POST /api/json/print - print JSON document to device.
pub async fn print(State(state): State<Arc<AppState>>, Json(mut doc): Json<Document>) -> Response {
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

/// Handle GET /api/json/component/:type/default - return a default component by type name.
pub async fn component_default(
    Path(type_name): Path<String>,
) -> Result<Json<Component>, StatusCode> {
    document::default_component(&type_name)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
