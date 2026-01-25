//! Composer API handlers - layer-based pattern composition.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    art::{self, PATTERNS},
    render::{
        self,
        composer::{BlendMode, Composer, ComposerSpec},
        dither::DitheringAlgorithm,
    },
    transport::BluetoothTransport,
};

use super::super::state::AppState;

fn default_dither() -> String {
    "floyd-steinberg".to_string()
}

fn default_mode() -> String {
    "raster".to_string()
}

fn default_true() -> bool {
    true
}

/// Request body for composer preview.
#[derive(Debug, Deserialize)]
pub struct ComposerPreviewRequest {
    pub spec: ComposerSpec,
    #[serde(default = "default_dither")]
    pub dither: String,
}

/// Request body for composer print.
#[derive(Debug, Deserialize)]
pub struct ComposerPrintRequest {
    pub spec: ComposerSpec,
    #[serde(default = "default_dither")]
    pub dither: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_true")]
    pub cut: bool,
}

/// Response info about available patterns for composer.
#[derive(Debug, Serialize)]
pub struct ComposerPatternInfo {
    pub name: &'static str,
}

/// Response info about blend modes.
#[derive(Debug, Serialize)]
pub struct BlendModeInfo {
    pub name: &'static str,
}

/// GET /api/composer/patterns - List available patterns for layering.
pub async fn patterns() -> Json<Vec<ComposerPatternInfo>> {
    let patterns: Vec<ComposerPatternInfo> = PATTERNS
        .iter()
        .map(|&name| ComposerPatternInfo { name })
        .collect();
    Json(patterns)
}

/// GET /api/composer/blend-modes - List available blend modes.
pub async fn blend_modes() -> Json<Vec<BlendModeInfo>> {
    let modes: Vec<BlendModeInfo> = BlendMode::all()
        .iter()
        .map(|mode| BlendModeInfo { name: mode.name() })
        .collect();
    Json(modes)
}

/// Parse dithering algorithm from string.
fn parse_dither(s: &str) -> DitheringAlgorithm {
    match s.to_lowercase().as_str() {
        "none" | "threshold" => DitheringAlgorithm::None,
        "floyd-steinberg" | "floyd_steinberg" | "fs" => DitheringAlgorithm::FloydSteinberg,
        "atkinson" => DitheringAlgorithm::Atkinson,
        "jarvis" | "jjn" => DitheringAlgorithm::Jarvis,
        "bayer" => DitheringAlgorithm::Bayer,
        _ => DitheringAlgorithm::FloydSteinberg, // default
    }
}

/// POST /api/composer/preview - Generate PNG preview of composition.
pub async fn preview(
    Json(req): Json<ComposerPreviewRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Validate spec
    if req.spec.width == 0 || req.spec.height == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Width and height must be positive".to_string(),
        ));
    }

    // Parse dithering algorithm
    let dither_algo = parse_dither(&req.dither);
    let width = req.spec.width;
    let height = req.spec.height;

    // Run CPU-intensive rendering in blocking task to avoid starving the tokio runtime
    let png_bytes = tokio::task::spawn_blocking(move || {
        // Create composer from spec
        let composer = Composer::from_spec(&req.spec)?;

        // Render the composition
        let raster_data = composer.render(dither_algo);

        // Convert to PNG
        render::raster_to_png(width, height, &raster_data)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Task error: {}", e)))?
    .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok(([(header::CONTENT_TYPE, "image/png")], png_bytes))
}

/// POST /api/composer/print - Print the composition.
pub async fn print(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ComposerPrintRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Validate spec
    if req.spec.width == 0 || req.spec.height == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "error": "Width and height must be positive"})),
        ));
    }

    // Create composer from spec
    let composer = Composer::from_spec(&req.spec).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "error": e})),
        )
    })?;

    // Parse dithering algorithm
    let dither_algo = parse_dither(&req.dither);

    // Render the composition
    let raster_data = composer.render(dither_algo);

    // Build print command
    use crate::ir::{Op, Program};

    let mut program = Program::new();
    program.push(Op::Init);

    let width = req.spec.width;
    let height = req.spec.height;

    if req.mode == "band" {
        program.push(Op::Band {
            width_bytes: (width / 8) as u8,
            data: raster_data,
        });
    } else {
        program.push(Op::Raster {
            width: width as u16,
            height: height as u16,
            data: raster_data,
        });
    }

    program.push(Op::Feed { units: 24 }); // 6mm

    if req.cut {
        program.push(Op::Cut { partial: false });
    }

    let print_data = program.to_bytes();

    // Print to device
    let device_path = state.config.device_path.clone();
    let layer_count = req.spec.layers.len();

    let print_result = tokio::task::spawn_blocking(move || {
        let mut transport = BluetoothTransport::open(&device_path)?;
        transport.write_all(&print_data)?;
        Ok::<_, crate::EstrellaError>(())
    })
    .await;

    match print_result {
        Ok(Ok(())) => Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("Composition printed ({} layers)", layer_count)
        }))),
        Ok(Err(e)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": format!("Print failed: {}", e)})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": format!("Task error: {}", e)})),
        )),
    }
}

/// GET /api/composer/pattern/:name/params - Get params for a specific pattern.
pub async fn pattern_params(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pattern = art::by_name(&name).ok_or(StatusCode::NOT_FOUND)?;

    let params: std::collections::HashMap<String, String> = pattern
        .list_params()
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

    let specs = pattern.param_specs();

    Ok(Json(serde_json::json!({
        "name": pattern.name(),
        "params": params,
        "specs": specs
    })))
}
