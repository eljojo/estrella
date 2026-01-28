//! Static file serving for the frontend.

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse},
};
use include_dir::{include_dir, Dir};
use std::sync::Arc;

use crate::document;

use super::state::AppState;

/// Embedded frontend distribution files.
static FRONTEND_DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/frontend/dist");

/// Serve the index.html file with cache-busting parameter and injected component types.
pub async fn index_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match FRONTEND_DIST.get_file("index.html") {
        Some(file) => {
            let contents = String::from_utf8_lossy(file.contents());
            // Inject cache-busting parameter into script/link tags
            let cache_bust = format!("?v={}", state.boot_time);
            let busted = contents
                .replace(".js\"", &format!(".js{}\"", cache_bust))
                .replace(".css\"", &format!(".css{}\"", cache_bust));

            // Inject component types as static data (avoids an API round-trip)
            let types_json = serde_json::to_string(&document::component_types()).unwrap();
            let script = format!(
                "<script>window.__COMPONENT_TYPES={}</script></head>",
                types_json
            );
            let busted = busted.replace("</head>", &script);

            Html(busted).into_response()
        }
        None => (StatusCode::NOT_FOUND, "Frontend not built").into_response(),
    }
}

/// Serve static assets from the assets directory.
pub async fn asset_handler(Path(path): Path<String>) -> impl IntoResponse {
    // Strip query params if present
    let clean_path = path.split('?').next().unwrap_or(&path);
    let file_path = format!("assets/{}", clean_path);

    match FRONTEND_DIST.get_file(&file_path) {
        Some(file) => {
            let mime = mime_guess::from_path(clean_path)
                .first_or_octet_stream()
                .to_string();
            // Set long cache headers since we use cache busting
            (
                [
                    (header::CONTENT_TYPE, mime),
                    (header::CACHE_CONTROL, "public, max-age=31536000".to_string()),
                ],
                file.contents().to_vec(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "Asset not found").into_response(),
    }
}
