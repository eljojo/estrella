//! Composer API handlers - layer-based pattern composition.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::{
    art::{self, PATTERNS},
    render::{
        self,
        composer::{
            render_layer_intensity, render_with_cached_layers, BlendMode, CachedLayerRef,
            Composer, ComposerSpec, LayerSpec,
        },
        dither::DitheringAlgorithm,
    },
    transport::BluetoothTransport,
};

use super::super::state::{AppState, CachedLayer, LayerCacheKey};

type LayerCache = Arc<RwLock<HashMap<LayerCacheKey, CachedLayer>>>;

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

/// Get or render a layer's intensity buffer, using cache when available.
fn get_or_render_layer(
    layer_spec: &LayerSpec,
    cache: &LayerCache,
) -> Result<Vec<f32>, String> {
    let cache_key = LayerCacheKey::new(
        &layer_spec.pattern,
        &layer_spec.params,
        layer_spec.width,
        layer_spec.height,
    );

    let mut cache_guard = cache.blocking_write();

    if let Some(cached) = cache_guard.get_mut(&cache_key) {
        cached.touch();
        return Ok(cached.intensity());
    }

    // Cache miss - render the layer
    let mut pattern = art::by_name(&layer_spec.pattern)
        .ok_or_else(|| format!("Unknown pattern: {}", layer_spec.pattern))?;

    for (k, v) in &layer_spec.params {
        pattern.set_param(k, v).map_err(|e| format!("Param error: {}", e))?;
    }

    let buffer = render_layer_intensity(pattern.as_ref(), layer_spec.width, layer_spec.height);

    // Cache the result
    let cached = CachedLayer::new(buffer.clone());
    println!(
        "[cache] Cached '{}' {}x{}: {} -> {} bytes ({:.1}%)",
        layer_spec.pattern,
        layer_spec.width,
        layer_spec.height,
        cached.uncompressed_size(),
        cached.compressed_size(),
        cached.compressed_size() as f32 / cached.uncompressed_size() as f32 * 100.0
    );
    cache_guard.insert(cache_key, cached);

    Ok(buffer)
}

/// Render a composition to PNG bytes using cached layers.
fn render_composition_to_png(
    spec: &ComposerSpec,
    layer_specs: &[LayerSpec],
    cache: &LayerCache,
    dither_algo: DitheringAlgorithm,
) -> Result<Vec<u8>, String> {
    // Get intensity buffers for all layers
    let intensity_buffers: Vec<Vec<f32>> = layer_specs
        .iter()
        .map(|spec| get_or_render_layer(spec, cache))
        .collect::<Result<_, _>>()?;

    // Build references for compositing
    let cached_refs: Vec<CachedLayerRef<'_>> = layer_specs
        .iter()
        .zip(intensity_buffers.iter())
        .map(|(spec, intensity)| CachedLayerRef {
            spec,
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
}

/// POST /api/composer/preview - Generate PNG preview of composition.
pub async fn preview(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ComposerPreviewRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if req.spec.width == 0 || req.spec.height == 0 {
        return Err((StatusCode::BAD_REQUEST, "Width and height must be positive".into()));
    }

    let dither_algo = parse_dither(&req.dither);
    let spec = req.spec.clone();
    let layer_specs = req.spec.layers;
    let cache = state.layer_cache.clone();

    let png_bytes = tokio::task::spawn_blocking(move || {
        render_composition_to_png(&spec, &layer_specs, &cache, dither_algo)
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
