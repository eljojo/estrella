//! Receipt printing handlers.

use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    document::{Component, Divider, Document, Markdown, Spacer, Text},
    ir::Program,
    receipt::current_datetime,
    transport::BluetoothTransport,
};

use super::super::state::AppState;

fn default_true() -> bool {
    true
}

/// Form data for receipt operations.
#[derive(Debug, Deserialize)]
pub struct ReceiptForm {
    /// Optional title for the receipt
    pub title: Option<String>,
    /// Body text (required)
    pub body: String,
    /// Whether to cut the page after printing
    #[serde(default = "default_true")]
    pub cut: bool,
    /// Whether to print the date footer
    #[serde(default = "default_true")]
    pub print_details: bool,
}

/// Handle POST /api/receipt/print - print the receipt.
pub async fn print(State(state): State<Arc<AppState>>, Json(form): Json<ReceiptForm>) -> Response {
    // Validate input
    if form.body.trim().is_empty() {
        return error_response("Body cannot be empty");
    }

    // Check that the active profile can print
    let profile = state.active_profile.read().await;
    if !profile.can_print() {
        return error_response("Cannot print: active profile is a virtual canvas");
    }
    let width = profile.width_dots();
    drop(profile);

    // Build the receipt data
    let receipt_data = build_receipt(&form, Some(width)).to_bytes();

    // Print to device (blocking operation, run in separate thread)
    let device_path = state.config.device_path.clone();
    let print_result =
        tokio::task::spawn_blocking(move || print_to_device(&device_path, &receipt_data)).await;

    match print_result {
        Ok(Ok(())) => success_response(&form),
        Ok(Err(e)) => error_response(&format!("Print failed: {}", e)),
        Err(e) => error_response(&format!("Task error: {}", e)),
    }
}

/// Build receipt program from form data.
fn build_receipt(form: &ReceiptForm, width: Option<usize>) -> Program {
    let mut components = Vec::new();

    // Add title if provided
    if let Some(title) = &form.title
        && !title.trim().is_empty()
    {
        components.push(Component::Text(Text {
            content: title.trim().to_string(),
            center: true,
            bold: true,
            size: [3, 2],
            ..Default::default()
        }));
        components.push(Component::Spacer(Spacer::mm(2.0)));
    }

    // Parse body as Markdown
    components.push(Component::Markdown(Markdown::new(&form.body)));

    // Add date footer if print_details is enabled
    if form.print_details {
        components.push(Component::Spacer(Spacer::mm(3.0)));
        components.push(Component::Divider(Divider::default()));
        components.push(Component::Text(Text {
            content: format!("Printed: {}", current_datetime()),
            center: true,
            size: [0, 0],
            ..Default::default()
        }));
    }

    components.push(Component::Spacer(Spacer::mm(6.0)));

    Document {
        document: components,
        cut: form.cut,
        interpolate: false,
        width,
        ..Default::default()
    }
    .compile()
}

/// Print to the physical device.
fn print_to_device(device_path: &str, data: &[u8]) -> Result<(), crate::EstrellaError> {
    let mut transport = BluetoothTransport::open(device_path)?;
    transport.write_all(data)?;
    Ok(())
}

/// Generate success response JSON.
fn success_response(form: &ReceiptForm) -> Response {
    let title_text = form
        .title
        .as_ref()
        .map(|t| format!("\"{}\"", t))
        .unwrap_or_else(|| "(no title)".to_string());

    (
        StatusCode::OK,
        Html(format!(
            r#"{{"success": true, "message": "Receipt {} printed successfully"}}"#,
            title_text
        )),
    )
        .into_response()
}

/// Generate error response JSON.
fn error_response(error_msg: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Html(format!(r#"{{"success": false, "error": "{}"}}"#, error_msg)),
    )
        .into_response()
}

/// Handle POST /api/receipt/preview - generate PNG preview.
pub async fn preview(
    State(state): State<Arc<AppState>>,
    Json(form): Json<ReceiptForm>,
) -> impl IntoResponse {
    if form.body.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Body cannot be empty".to_string()));
    }

    let profile = state.active_profile.read().await;
    let print_width = profile.width_dots();
    drop(profile);

    // Build the receipt program and render to PNG
    let png_bytes = build_receipt(&form, Some(print_width))
        .to_preview_png_with_width(print_width)
        .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to render preview: {}", e),
        )
    })?;

    Ok(([(header::CONTENT_TYPE, "image/png")], png_bytes))
}
