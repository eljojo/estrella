//! Pattern API handlers.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use image::{GrayImage, Luma};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Cursor, sync::Arc};

use crate::{
    art::ParamSpec,
    render::{context::RenderContext, dither, patterns},
    transport::BluetoothTransport,
};

use super::super::state::AppState;

/// Pattern information returned by the API.
#[derive(Debug, Serialize)]
pub struct PatternInfo {
    pub name: String,
    pub params: HashMap<String, String>,
    pub specs: Vec<ParamSpec>,
}

/// Query parameters for preview endpoint.
#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    pub length_mm: f32,
    #[serde(default = "default_dither")]
    pub dither: String,
    /// Mode is accepted but not used for preview (only affects printing).
    #[serde(default = "default_mode")]
    #[allow(dead_code)]
    pub mode: String,
    /// Override width in pixels (bypasses printer config).
    pub width: Option<usize>,
    /// Override height in pixels (bypasses length_mm calculation).
    pub height: Option<usize>,
    #[serde(flatten)]
    pub params: HashMap<String, String>,
}

fn default_dither() -> String {
    "floyd-steinberg".to_string()
}

fn default_mode() -> String {
    "raster".to_string()
}

fn default_true() -> bool {
    true
}

/// Form data for print endpoint.
#[derive(Debug, Deserialize)]
pub struct PatternPrintForm {
    pub length_mm: f32,
    #[serde(default = "default_dither")]
    pub dither: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub params: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub cut: bool,
    #[serde(default = "default_true")]
    pub print_details: bool,
}

/// GET /api/patterns - List all pattern names.
pub async fn list() -> Json<Vec<&'static str>> {
    Json(patterns::list_patterns().to_vec())
}

/// GET /api/patterns/:name/params - Get golden default params for a pattern.
pub async fn params(Path(name): Path<String>) -> Result<Json<PatternInfo>, StatusCode> {
    let pattern = patterns::by_name_golden(&name).ok_or(StatusCode::NOT_FOUND)?;

    let params: HashMap<String, String> = pattern
        .list_params()
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

    let specs = pattern.param_specs();

    Ok(Json(PatternInfo {
        name: pattern.name().to_string(),
        params,
        specs,
    }))
}

/// POST /api/patterns/:name/randomize - Get randomized params for a pattern.
pub async fn randomize(Path(name): Path<String>) -> Result<Json<PatternInfo>, StatusCode> {
    let pattern = patterns::by_name_random(&name).ok_or(StatusCode::NOT_FOUND)?;

    let params: HashMap<String, String> = pattern
        .list_params()
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

    let specs = pattern.param_specs();

    Ok(Json(PatternInfo {
        name: pattern.name().to_string(),
        params,
        specs,
    }))
}

/// GET /api/patterns/:name/preview - Generate PNG preview.
pub async fn preview(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(query): Query<PreviewQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut pattern = patterns::by_name_golden(&name).ok_or((
        StatusCode::NOT_FOUND,
        format!("Pattern '{}' not found", name),
    ))?;

    // Apply custom params (skip the known query params)
    for (param_name, param_value) in &query.params {
        if param_name != "length_mm" && param_name != "dither" && param_name != "mode" {
            pattern
                .set_param(param_name, param_value)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid param: {}", e)))?;
        }
    }

    // Calculate dimensions (use overrides if provided, otherwise active profile)
    let profile = state.active_profile.read().await;
    let width = query.width.unwrap_or(profile.width_dots());
    let height = query.height.unwrap_or_else(|| {
        profile
            .mm_to_dots(query.length_mm)
            .map(|d| d as usize)
            .unwrap_or(query.length_mm.round() as usize) // Canvas: treat as pixels directly
    });
    drop(profile);

    // Prepare pattern (async — handles I/O like image downloads)
    let ctx = RenderContext::new(
        reqwest::Client::builder()
            .user_agent("estrella/0.1")
            .build()
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("HTTP client error: {}", e),
                )
            })?,
        state.photo_sessions.clone(),
        state.intensity_cache.clone(),
    );
    pattern
        .prepare(width, height, &ctx)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Prepare failed: {}", e)))?;

    // Parse dithering algorithm
    let dither_algo = match query.dither.to_lowercase().as_str() {
        "none" | "threshold" => dither::DitheringAlgorithm::None,
        "floyd-steinberg" | "floyd_steinberg" | "fs" => dither::DitheringAlgorithm::FloydSteinberg,
        "atkinson" => dither::DitheringAlgorithm::Atkinson,
        "jarvis" | "jjn" => dither::DitheringAlgorithm::Jarvis,
        _ => dither::DitheringAlgorithm::Bayer,
    };

    // Render pattern
    let raster_data = patterns::render(pattern.as_ref(), width, height, dither_algo);
    let width_bytes = width.div_ceil(8);

    // Convert to PNG
    let mut img = GrayImage::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let byte_idx = y * width_bytes + x / 8;
            let bit_idx = 7 - (x % 8);
            let is_black = (raster_data[byte_idx] >> bit_idx) & 1 == 1;
            let color = if is_black { 0u8 } else { 255u8 };
            img.put_pixel(x as u32, y as u32, Luma([color]));
        }
    }

    let mut png_bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("PNG encoding failed: {}", e),
            )
        })?;

    Ok(([(header::CONTENT_TYPE, "image/png")], png_bytes))
}

/// POST /api/patterns/:name/print - Print the pattern.
pub async fn print(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(form): Json<PatternPrintForm>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut pattern = patterns::by_name_golden(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"success": false, "error": "Pattern not found"})),
        )
    })?;

    // Apply custom params
    for (param_name, param_value) in &form.params {
        pattern.set_param(param_name, param_value).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"success": false, "error": e})),
            )
        })?;
    }

    // Check that the active profile can print
    let profile = state.active_profile.read().await;
    if !profile.can_print() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"success": false, "error": "Cannot print: active profile is a virtual canvas"}),
            ),
        ));
    }
    let width = profile.width_dots();
    let height = profile
        .mm_to_dots(form.length_mm)
        .map(|d| d as usize)
        .unwrap_or(form.length_mm.round() as usize);
    drop(profile);

    // Prepare pattern (async — handles I/O like image downloads)
    let ctx = RenderContext::new(
        reqwest::Client::builder()
            .user_agent("estrella/0.1")
            .build()
            .map_err(|e| (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"success": false, "error": format!("HTTP client error: {}", e)})),
            ))?,
        state.photo_sessions.clone(),
        state.intensity_cache.clone(),
    );
    pattern.prepare(width, height, &ctx).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "error": format!("Prepare failed: {}", e)})),
        )
    })?;

    // Parse dithering algorithm
    let dither_algo = match form.dither.to_lowercase().as_str() {
        "none" | "threshold" => dither::DitheringAlgorithm::None,
        "floyd-steinberg" | "floyd_steinberg" | "fs" => dither::DitheringAlgorithm::FloydSteinberg,
        "atkinson" => dither::DitheringAlgorithm::Atkinson,
        "jarvis" | "jjn" => dither::DitheringAlgorithm::Jarvis,
        _ => dither::DitheringAlgorithm::Bayer,
    };

    // Render pattern
    let raster_data = patterns::render(pattern.as_ref(), width, height, dither_algo);

    // Build print command based on mode
    use crate::document::{Divider, EmitContext, Text};
    use crate::ir::{Op, Program};

    let mut program = Program::new();
    program.push(Op::Init);

    // Print title if details enabled
    if form.print_details {
        // Title
        let title = Text {
            content: pattern.name().to_string(),
            center: true,
            bold: true,
            size: [3, 2],
            ..Default::default()
        };
        let mut ctx = EmitContext::new(width);
        title.emit(&mut ctx);
        program.extend(ctx.ops);
        program.push(Op::Newline);

        // Divider
        let divider = Divider::default();
        let mut ctx = EmitContext::new(width);
        divider.emit(&mut ctx);
        program.extend(ctx.ops);
    }

    if form.mode == "band" {
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

    // Print parameters if details enabled
    if form.print_details {
        let divider = Divider::default();
        let mut ctx = EmitContext::new(width);
        divider.emit(&mut ctx);
        program.extend(ctx.ops);

        // Parameters
        let params_list = pattern.list_params();
        if !params_list.is_empty() {
            let params_text = params_list
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(", ");
            let params = Text {
                content: params_text,
                center: true,
                size: [0, 0],
                ..Default::default()
            };
            let mut ctx = EmitContext::new(width);
            params.emit(&mut ctx);
            program.extend(ctx.ops);
            program.push(Op::Newline);
        }
    }

    program.push(Op::Feed { units: 24 }); // 6mm

    if form.cut {
        program.push(Op::Cut { partial: false });
    }

    // Split for long print and send to printer
    let device_path = state.config.device_path.clone();
    let pattern_name = name.clone();

    println!(
        "[patterns] Print request: pattern={}, {}x{} pixels, mode={}",
        pattern_name, width, height, form.mode
    );

    let print_result = tokio::task::spawn_blocking(move || {
        let programs = program.split_for_long_print();
        println!("[patterns] Split into {} program(s)", programs.len());
        let mut transport = BluetoothTransport::open(&device_path)?;
        transport.send_programs(&programs)?;
        Ok::<_, crate::EstrellaError>(())
    })
    .await;

    match print_result {
        Ok(Ok(())) => Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("Pattern '{}' printed successfully", name)
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
