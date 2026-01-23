//! Receipt printing handlers.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    components::{ComponentExt, Divider, Markdown, Receipt, Spacer, Text},
    ir::Program,
    protocol::text::Font,
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
pub async fn print(
    State(state): State<Arc<AppState>>,
    Json(form): Json<ReceiptForm>,
) -> Response {
    // Validate input
    if form.body.trim().is_empty() {
        return error_response("Body cannot be empty");
    }

    // Build the receipt data
    let receipt_data = build_receipt(&form).to_bytes();

    // Print to device (blocking operation, run in separate thread)
    let device_path = state.config.device_path.clone();
    let print_result = tokio::task::spawn_blocking(move || {
        print_to_device(&device_path, &receipt_data)
    })
    .await;

    match print_result {
        Ok(Ok(())) => success_response(&form),
        Ok(Err(e)) => error_response(&format!("Print failed: {}", e)),
        Err(e) => error_response(&format!("Task error: {}", e)),
    }
}

/// Build receipt program from form data.
fn build_receipt(form: &ReceiptForm) -> Program {
    let mut receipt = Receipt::new();

    // Add title if provided
    if let Some(title) = &form.title {
        if !title.trim().is_empty() {
            receipt = receipt
                .child(Text::new(title.trim()).center().bold().size(2, 1))
                .child(Spacer::mm(2.0));
        }
    }

    // Parse body as Markdown
    receipt = receipt.child(Markdown::new(&form.body));

    // Add date footer if print_details is enabled
    if form.print_details {
        receipt = receipt
            .child(Spacer::mm(3.0))
            .child(Divider::dashed())
            .child(
                Text::new(&format!("Printed: {}", current_datetime()))
                    .center()
                    .font(Font::B),
            );
    }

    receipt = receipt.child(Spacer::mm(6.0));

    if form.cut {
        receipt = receipt.cut();
    }

    receipt.compile()
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
pub async fn preview(Json(form): Json<ReceiptForm>) -> impl IntoResponse {
    if form.body.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Body cannot be empty".to_string(),
        ));
    }

    // Build the receipt program and render to PNG
    let png_bytes = build_receipt(&form).to_preview_png().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to render preview: {}", e),
        )
    })?;

    Ok(([(header::CONTENT_TYPE, "image/png")], png_bytes))
}
