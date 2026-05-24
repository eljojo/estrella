//! Receipt printing handlers.

use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
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

fn default_title_size() -> [u8; 2] {
    [3, 2]
}

fn default_body_size() -> [u8; 2] {
    [0, 0]
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
    /// Title character size [height, width]. 0 = 1×, 1 = 2×, etc. Defaults to [3, 2].
    #[serde(default = "default_title_size")]
    pub title_size: [u8; 2],
    /// Body character size [height, width]. 0 = 1×, 1 = 2×, etc. Defaults to [0, 0] (normal).
    #[serde(default = "default_body_size")]
    pub body_size: [u8; 2],
}

/// Handle POST /api/receipt/print - print the receipt.
pub async fn print(State(state): State<Arc<AppState>>, Json(form): Json<ReceiptForm>) -> Response {
    // Validate input
    if form.body.trim().is_empty() {
        return error_response("Body cannot be empty");
    }

    let printer_width = state.config.printer_width;
    let receipt_data = build_receipt(&form, printer_width).to_bytes();

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
fn build_receipt(form: &ReceiptForm, printer_width: u16) -> Program {
    let chars_per_line = printer_width as usize / 12;
    let mut components = Vec::new();

    // Add title if provided
    if let Some(title) = &form.title
        && !title.trim().is_empty()
    {
        components.push(Component::Text(Text {
            content: title.trim().to_string(),
            center: true,
            bold: true,
            size: form.title_size,
            ..Default::default()
        }));
        components.push(Component::Spacer(Spacer::mm(2.0)));
    }

    // Parse body as Markdown, word-wrapped to fit the printer
    components.push(Component::Markdown(Markdown {
        size: form.body_size,
        chars_per_line: Some(chars_per_line),
        ..Markdown::new(&form.body)
    }));

    // Add date footer if print_details is enabled
    if form.print_details {
        components.push(Component::Spacer(Spacer::mm(3.0)));
        components.push(Component::Divider(Divider { width: Some(chars_per_line), ..Default::default() }));
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
    #[derive(Serialize)]
    struct SuccessBody<'a> {
        success: bool,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<&'a str>,
    }

    let title = form.title.as_deref().filter(|t| !t.trim().is_empty());
    let message = match title {
        Some(t) => format!("Receipt \"{}\" printed successfully", t),
        None => "Receipt printed successfully".to_string(),
    };

    (StatusCode::OK, Json(SuccessBody { success: true, message, title })).into_response()
}

/// Generate error response JSON.
fn error_response(error_msg: &str) -> Response {
    #[derive(Serialize)]
    struct ErrorBody<'a> {
        success: bool,
        error: &'a str,
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorBody { success: false, error: error_msg }),
    )
        .into_response()
}

/// Handle POST /api/receipt/preview - generate PNG preview.
pub async fn preview(State(state): State<Arc<AppState>>, Json(form): Json<ReceiptForm>) -> impl IntoResponse {
    if form.body.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Body cannot be empty".to_string()));
    }

    let printer_width = state.config.printer_width;
    let png_bytes = build_receipt(&form, printer_width).to_preview_png().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to render preview: {}", e),
        )
    })?;

    Ok(([(header::CONTENT_TYPE, "image/png")], png_bytes))
}
