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
    art,
    render::{
        self,
        composer::{
            render_layer_intensity, render_with_cached_layers, BlendMode, CachedLayerRef,
            ComposerSpec, LayerSpec,
        },
        context::RenderContext,
        dither::DitheringAlgorithm,
    },
    server::IntensityCacheKey,
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

/// Response info about blend modes.
#[derive(Debug, Serialize)]
pub struct BlendModeInfo {
    pub name: &'static str,
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

/// Create and prepare all patterns for a composition's layers.
///
/// Each layer becomes a pattern: create via `by_name()`, configure via `set_param()`,
/// and `prepare()` in the async context (handles I/O like image downloads).
/// Returns prepared patterns and their pre-rendered intensity buffers.
async fn prepare_layers(
    layers: &[LayerSpec],
    ctx: &RenderContext,
) -> Result<Vec<Vec<f32>>, String> {
    let mut buffers = Vec::with_capacity(layers.len());

    for (i, layer) in layers.iter().enumerate() {
        let mut pattern = art::by_name(&layer.pattern)
            .ok_or_else(|| format!("Layer {}: unknown pattern '{}'", i, layer.pattern))?;

        for (k, v) in &layer.params {
            pattern
                .set_param(k, v)
                .map_err(|e| format!("Layer {}: param error: {}", i, e))?;
        }

        // Async prepare — handles I/O (image downloads, etc.)
        pattern
            .prepare(layer.width, layer.height, ctx)
            .await
            .map_err(|e| format!("Layer {}: prepare failed: {}", i, e))?;

        // Pre-render intensity buffer (cached across requests)
        let key = IntensityCacheKey::new(&layer.pattern, &layer.params, layer.width, layer.height);
        let buffer = ctx
            .get_or_render_intensity(key, || {
                render_layer_intensity(pattern.as_ref(), layer.width, layer.height)
            })
            .await;
        buffers.push(buffer);
    }

    Ok(buffers)
}

/// POST /api/composer/preview - Generate PNG preview of composition.
pub async fn preview(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ComposerPreviewRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if req.spec.width == 0 || req.spec.height == 0 {
        return Err((StatusCode::BAD_REQUEST, "Width and height must be positive".into()));
    }

    let ctx = RenderContext::new(
        reqwest::Client::builder()
            .user_agent("estrella/0.1")
            .build()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("HTTP client error: {}", e)))?,
        state.photo_sessions.clone(),
        state.intensity_cache.clone(),
    );

    // Prepare all layers (async — handles image downloads, etc.)
    let intensity_buffers = prepare_layers(&req.spec.layers, &ctx)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let dither_algo = parse_dither(&req.dither);
    let spec = req.spec;

    let png_bytes = tokio::task::spawn_blocking(move || {
        // Build references for compositing
        let cached_refs: Vec<CachedLayerRef<'_>> = spec
            .layers
            .iter()
            .zip(intensity_buffers.iter())
            .map(|(layer_spec, intensity)| CachedLayerRef {
                spec: layer_spec,
                intensity: intensity.as_slice(),
            })
            .collect();

        // Composite and dither
        let raster_data = render_with_cached_layers(
            spec.width,
            spec.height,
            spec.background,
            &cached_refs,
            dither_algo,
        );

        render::raster_to_png(spec.width, spec.height, &raster_data)
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
    let err_json = |msg: String| (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"success": false, "error": msg})),
    );

    // Validate spec
    if req.spec.width == 0 || req.spec.height == 0 {
        return Err(err_json("Width and height must be positive".into()));
    }

    let ctx = RenderContext::new(
        reqwest::Client::builder()
            .user_agent("estrella/0.1")
            .build()
            .map_err(|e| err_json(format!("HTTP client error: {}", e)))?,
        state.photo_sessions.clone(),
        state.intensity_cache.clone(),
    );

    // Prepare all layers (async — handles image downloads, etc.)
    let intensity_buffers = prepare_layers(&req.spec.layers, &ctx)
        .await
        .map_err(|e| err_json(e))?;

    let dither_algo = parse_dither(&req.dither);
    let width = req.spec.width;
    let height = req.spec.height;
    let layer_count = req.spec.layers.len();
    let mode = req.mode.clone();
    let cut = req.cut;
    let device_path = state.config.device_path.clone();
    let spec = req.spec;

    println!(
        "[composer] Print request: {}x{} pixels, {} layers, mode={}",
        width, height, layer_count, mode
    );

    let print_result = tokio::task::spawn_blocking(move || {
        let cached_refs: Vec<CachedLayerRef<'_>> = spec
            .layers
            .iter()
            .zip(intensity_buffers.iter())
            .map(|(layer_spec, intensity)| CachedLayerRef {
                spec: layer_spec,
                intensity: intensity.as_slice(),
            })
            .collect();

        let raster_data = render_with_cached_layers(
            spec.width,
            spec.height,
            spec.background,
            &cached_refs,
            dither_algo,
        );

        // Build print command
        use crate::ir::{Op, Program};

        let mut program = Program::new();
        program.push(Op::Init);

        if mode == "band" {
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

        if cut {
            program.push(Op::Cut { partial: false });
        }

        let programs = program.split_for_long_print();
        println!(
            "[composer] Split into {} program(s)",
            programs.len()
        );
        let mut transport = BluetoothTransport::open(&device_path)?;
        transport.send_programs(&programs)?;
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
