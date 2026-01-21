//! # HTTP Server for Receipt Printing
//!
//! Provides a web interface for printing text receipts via HTTP.
//!
//! ## Usage
//!
//! ```bash
//! estrella serve --listen 0.0.0.0:8080 --device /dev/rfcomm0
//! ```
//!
//! Then open http://localhost:8080 in a browser to access the print form.
//!
//! ## TODO
//!
//! - Add tests for `build_receipt()` function
//! - Add handler tests using axum test utilities
//! - Add integration tests for full request/response cycle

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Form, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    components::{ComponentExt, Markdown, Receipt, Spacer, Text},
    error::EstrellaError,
    transport::BluetoothTransport,
};

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Path to the printer device (e.g., "/dev/rfcomm0")
    pub device_path: String,
    /// Address to listen on (e.g., "0.0.0.0:8080")
    pub listen_addr: String,
}

/// Form data submitted by the user.
#[derive(Debug, Deserialize)]
pub struct PrintForm {
    /// Optional title for the receipt
    pub title: Option<String>,
    /// Body text (required)
    pub body: String,
}

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
    let app_state = Arc::new(config.clone());

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/print", post(print_handler))
        .with_state(app_state);

    println!("🖨️  Estrella HTTP server starting...");
    println!("📡 Listening on: {}", config.listen_addr);
    println!("🔌 Printer device: {}", config.device_path);
    println!();
    println!("Open http://{}/ in your browser to print", config.listen_addr);
    println!();

    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .map_err(|e| EstrellaError::Transport(format!("Failed to bind to {}: {}", config.listen_addr, e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| EstrellaError::Transport(format!("Server error: {}", e)))?;

    Ok(())
}

/// Handle GET / - return the HTML form.
async fn index_handler() -> Html<&'static str> {
    Html(HTML_FORM)
}

/// Handle POST /print - print the receipt.
async fn print_handler(
    State(config): State<Arc<ServerConfig>>,
    Form(form): Form<PrintForm>,
) -> Response {
    // Validate input
    if form.body.trim().is_empty() {
        return error_response("Body cannot be empty");
    }

    // Build the receipt data
    let receipt_data = match build_receipt(&form) {
        Ok(data) => data,
        Err(e) => return error_response(&format!("Failed to build receipt: {}", e)),
    };

    // Print to device (blocking operation, run in separate thread)
    let device_path = config.device_path.clone();
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

/// Build receipt bytes from form data.
fn build_receipt(form: &PrintForm) -> Result<Vec<u8>, EstrellaError> {
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

    // Add cut command
    receipt = receipt.cut();

    Ok(receipt.build())
}

/// Print to the physical device.
fn print_to_device(device_path: &str, data: &[u8]) -> Result<(), EstrellaError> {
    let mut transport = BluetoothTransport::open(device_path)?;
    transport.write_all(data)?;
    Ok(())
}

/// Generate success response HTML.
fn success_response(form: &PrintForm) -> Response {
    let title_text = form
        .title
        .as_ref()
        .map(|t| format!("\"{}\"", t))
        .unwrap_or_else(|| "(no title)".to_string());

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Print Success</title>
    {}
</head>
<body>
    <div class="container">
        <div class="success">
            <h1>✓ Print Successful</h1>
            <p>Receipt {} printed successfully!</p>
            <a href="/" class="button">Print Another Receipt</a>
        </div>
    </div>
</body>
</html>"#,
        CSS_STYLES,
        title_text
    );

    Html(html).into_response()
}

/// Generate error response HTML.
fn error_response(error_msg: &str) -> Response {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Print Error</title>
    {}
</head>
<body>
    <div class="container">
        <div class="error">
            <h1>✗ Print Failed</h1>
            <p>{}</p>
            <a href="/" class="button">Try Again</a>
        </div>
    </div>
</body>
</html>"#,
        CSS_STYLES,
        error_msg
    );

    (StatusCode::INTERNAL_SERVER_ERROR, Html(html)).into_response()
}

/// CSS styles for the HTML pages.
const CSS_STYLES: &str = r#"<style>
    * {
        margin: 0;
        padding: 0;
        box-sizing: border-box;
    }

    body {
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 20px;
    }

    .container {
        background: white;
        border-radius: 16px;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
        max-width: 600px;
        width: 100%;
        padding: 40px;
    }

    h1 {
        color: #2d3748;
        font-size: 32px;
        margin-bottom: 8px;
        font-weight: 700;
    }

    .subtitle {
        color: #718096;
        font-size: 16px;
        margin-bottom: 32px;
    }

    .form-group {
        margin-bottom: 24px;
    }

    label {
        display: block;
        color: #4a5568;
        font-weight: 600;
        margin-bottom: 8px;
        font-size: 14px;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }

    input[type="text"],
    textarea {
        width: 100%;
        padding: 12px 16px;
        border: 2px solid #e2e8f0;
        border-radius: 8px;
        font-size: 16px;
        font-family: inherit;
        transition: border-color 0.3s ease;
    }

    input[type="text"]:focus,
    textarea:focus {
        outline: none;
        border-color: #667eea;
    }

    textarea {
        min-height: 200px;
        resize: vertical;
        font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, 'Courier New', monospace;
    }

    .hint {
        color: #a0aec0;
        font-size: 13px;
        margin-top: 6px;
    }

    button {
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
        border: none;
        padding: 14px 32px;
        font-size: 16px;
        font-weight: 600;
        border-radius: 8px;
        cursor: pointer;
        width: 100%;
        transition: transform 0.2s ease, box-shadow 0.2s ease;
        box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
    }

    button:hover {
        transform: translateY(-2px);
        box-shadow: 0 6px 20px rgba(102, 126, 234, 0.6);
    }

    button:active {
        transform: translateY(0);
    }

    .success {
        text-align: center;
    }

    .success h1 {
        color: #48bb78;
        font-size: 48px;
        margin-bottom: 16px;
    }

    .success p {
        color: #4a5568;
        font-size: 18px;
        margin-bottom: 32px;
    }

    .error {
        text-align: center;
    }

    .error h1 {
        color: #f56565;
        font-size: 48px;
        margin-bottom: 16px;
    }

    .error p {
        color: #4a5568;
        font-size: 18px;
        margin-bottom: 32px;
        word-break: break-word;
    }

    .button {
        display: inline-block;
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
        padding: 14px 32px;
        text-decoration: none;
        border-radius: 8px;
        font-weight: 600;
        transition: transform 0.2s ease, box-shadow 0.2s ease;
        box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
    }

    .button:hover {
        transform: translateY(-2px);
        box-shadow: 0 6px 20px rgba(102, 126, 234, 0.6);
    }

    @media (max-width: 640px) {
        .container {
            padding: 24px;
        }

        h1 {
            font-size: 24px;
        }

        .subtitle {
            font-size: 14px;
        }
    }
</style>"#;

/// HTML form for printing receipts.
const HTML_FORM: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Estrella Printer</title>
    <style>
    * {
        margin: 0;
        padding: 0;
        box-sizing: border-box;
    }

    body {
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 20px;
    }

    .container {
        background: white;
        border-radius: 16px;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
        max-width: 600px;
        width: 100%;
        padding: 40px;
    }

    h1 {
        color: #2d3748;
        font-size: 32px;
        margin-bottom: 8px;
        font-weight: 700;
    }

    .subtitle {
        color: #718096;
        font-size: 16px;
        margin-bottom: 32px;
    }

    .form-group {
        margin-bottom: 24px;
    }

    label {
        display: block;
        color: #4a5568;
        font-weight: 600;
        margin-bottom: 8px;
        font-size: 14px;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }

    input[type="text"],
    textarea {
        width: 100%;
        padding: 12px 16px;
        border: 2px solid #e2e8f0;
        border-radius: 8px;
        font-size: 16px;
        font-family: inherit;
        transition: border-color 0.3s ease;
    }

    input[type="text"]:focus,
    textarea:focus {
        outline: none;
        border-color: #667eea;
    }

    textarea {
        min-height: 200px;
        resize: vertical;
        font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, 'Courier New', monospace;
    }

    .hint {
        color: #a0aec0;
        font-size: 13px;
        margin-top: 6px;
    }

    button {
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
        border: none;
        padding: 14px 32px;
        font-size: 16px;
        font-weight: 600;
        border-radius: 8px;
        cursor: pointer;
        width: 100%;
        transition: transform 0.2s ease, box-shadow 0.2s ease;
        box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
    }

    button:hover {
        transform: translateY(-2px);
        box-shadow: 0 6px 20px rgba(102, 126, 234, 0.6);
    }

    button:active {
        transform: translateY(0);
    }

    @media (max-width: 640px) {
        .container {
            padding: 24px;
        }

        h1 {
            font-size: 24px;
        }

        .subtitle {
            font-size: 14px;
        }
    }
    </style>
</head>
<body>
    <div class="container">
        <h1>🖨️ Estrella Printer</h1>
        <p class="subtitle">Print a text receipt to your thermal printer</p>

        <form method="POST" action="/print">
            <div class="form-group">
                <label for="title">Title (optional)</label>
                <input type="text" id="title" name="title" placeholder="Receipt Title">
                <p class="hint">Optional header text for your receipt</p>
            </div>

            <div class="form-group">
                <label for="body">Body *</label>
                <textarea id="body" name="body" required placeholder="Enter your receipt text here...
Line 1
Line 2
Line 3

Add as many lines as you need!"></textarea>
                <p class="hint">Required. Each line will be printed as entered.</p>
            </div>

            <button type="submit">🚀 Print Receipt</button>
        </form>
    </div>
</body>
</html>"#;
