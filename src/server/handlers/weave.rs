//! Weave API handlers - blending multiple patterns with crossfade transitions.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use image::{GrayImage, Luma};
use serde::Deserialize;
use std::{collections::HashMap, io::Cursor, sync::Arc};

use crate::{
    printer::PrinterConfig,
    render::{
        context::RenderContext,
        dither,
        patterns::{self, Pattern},
        weave::{BlendCurve, Weave},
    },
    transport::BluetoothTransport,
};

use super::super::state::AppState;

// Available curves: "linear", "smooth", "ease-in", "ease-out"
// Hardcoded in frontend - see BlendCurve in src/render/weave.rs for reference

/// A pattern entry in the weave request.
#[derive(Debug, Deserialize)]
pub struct WeavePatternEntry {
    pub name: String,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

fn default_true() -> bool {
    true
}

/// Request body for weave preview/print.
#[derive(Debug, Deserialize)]
pub struct WeaveRequest {
    pub length_mm: f32,
    #[serde(default = "default_crossfade")]
    pub crossfade_mm: f32,
    #[serde(default = "default_curve")]
    pub curve: String,
    #[serde(default = "default_dither")]
    pub dither: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    pub patterns: Vec<WeavePatternEntry>,
    #[serde(default = "default_true")]
    pub cut: bool,
    #[serde(default = "default_true")]
    pub print_details: bool,
}

fn default_crossfade() -> f32 {
    30.0
}

fn default_curve() -> String {
    "smooth".to_string()
}

fn default_dither() -> String {
    "floyd-steinberg".to_string()
}

fn default_mode() -> String {
    "raster".to_string()
}

/// POST /api/weave/preview - Generate PNG preview of blended patterns.
pub async fn preview(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WeaveRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if req.patterns.len() < 2 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Weave requires at least 2 patterns".to_string(),
        ));
    }

    let ctx = RenderContext::new(
        reqwest::Client::builder()
            .user_agent("estrella/0.1")
            .build()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("HTTP client error: {}", e)))?,
        state.photo_sessions.clone(),
        state.intensity_cache.clone(),
    );

    // Calculate dimensions (needed for prepare)
    let config = PrinterConfig::TSP650II;
    let width = config.width_dots as usize;
    let height = config.mm_to_dots(req.length_mm) as usize;

    // Load, configure, and prepare patterns
    let mut pattern_impls: Vec<Box<dyn Pattern>> = Vec::new();
    for entry in &req.patterns {
        let mut pattern = patterns::by_name_golden(&entry.name).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Pattern '{}' not found", entry.name),
            )
        })?;

        for (param_name, param_value) in &entry.params {
            pattern.set_param(param_name, param_value).map_err(|e| {
                (StatusCode::BAD_REQUEST, format!("Invalid param: {}", e))
            })?;
        }

        pattern.prepare(width, height, &ctx).await.map_err(|e| {
            (StatusCode::BAD_REQUEST, format!("Prepare failed: {}", e))
        })?;

        pattern_impls.push(pattern);
    }

    let crossfade_pixels = config.mm_to_dots(req.crossfade_mm) as usize;

    // Parse curve
    let blend_curve = BlendCurve::from_str(&req.curve).unwrap_or(BlendCurve::Smooth);

    // Parse dithering algorithm
    let dither_algo = match req.dither.to_lowercase().as_str() {
        "none" | "threshold" => dither::DitheringAlgorithm::None,
        "floyd-steinberg" | "floyd_steinberg" | "fs" => dither::DitheringAlgorithm::FloydSteinberg,
        "atkinson" => dither::DitheringAlgorithm::Atkinson,
        "jarvis" | "jjn" => dither::DitheringAlgorithm::Jarvis,
        _ => dither::DitheringAlgorithm::Bayer,
    };

    // Create the weave
    let pattern_refs: Vec<&dyn Pattern> = pattern_impls.iter().map(|p| p.as_ref()).collect();
    let weave = Weave::new(pattern_refs)
        .crossfade_pixels(crossfade_pixels)
        .curve(blend_curve);

    // Render using dithering
    let raster_data = dither::generate_raster(
        width,
        height,
        |x, y, w, h| weave.intensity(x, y, w, h),
        dither_algo,
    );
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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("PNG encoding failed: {}", e)))?;

    Ok(([(header::CONTENT_TYPE, "image/png")], png_bytes))
}

/// POST /api/weave/print - Print the blended patterns.
pub async fn print(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WeaveRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if req.patterns.len() < 2 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "error": "Weave requires at least 2 patterns"})),
        ));
    }

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

    // Calculate dimensions (needed for prepare)
    let config = PrinterConfig::TSP650II;
    let width = config.width_dots as usize;
    let height = config.mm_to_dots(req.length_mm) as usize;
    let crossfade_pixels = config.mm_to_dots(req.crossfade_mm) as usize;

    // Load, configure, and prepare patterns
    let mut pattern_impls: Vec<Box<dyn Pattern>> = Vec::new();
    for entry in &req.patterns {
        let mut pattern = patterns::by_name_golden(&entry.name).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"success": false, "error": format!("Pattern '{}' not found", entry.name)})),
            )
        })?;

        for (param_name, param_value) in &entry.params {
            pattern.set_param(param_name, param_value).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"success": false, "error": format!("Invalid param: {}", e)})),
                )
            })?;
        }

        pattern.prepare(width, height, &ctx).await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"success": false, "error": format!("Prepare failed: {}", e)})),
            )
        })?;

        pattern_impls.push(pattern);
    }

    // Parse curve
    let blend_curve = BlendCurve::from_str(&req.curve).unwrap_or(BlendCurve::Smooth);

    // Parse dithering algorithm
    let dither_algo = match req.dither.to_lowercase().as_str() {
        "none" | "threshold" => dither::DitheringAlgorithm::None,
        "floyd-steinberg" | "floyd_steinberg" | "fs" => dither::DitheringAlgorithm::FloydSteinberg,
        "atkinson" => dither::DitheringAlgorithm::Atkinson,
        "jarvis" | "jjn" => dither::DitheringAlgorithm::Jarvis,
        _ => dither::DitheringAlgorithm::Bayer,
    };

    // Create the weave
    let pattern_refs: Vec<&dyn Pattern> = pattern_impls.iter().map(|p| p.as_ref()).collect();
    let weave = Weave::new(pattern_refs)
        .crossfade_pixels(crossfade_pixels)
        .curve(blend_curve);

    // Render using dithering
    let raster_data = dither::generate_raster(
        width,
        height,
        |x, y, w, h| weave.intensity(x, y, w, h),
        dither_algo,
    );

    // Build print command based on mode
    use crate::document::{Divider, Text};
    use crate::ir::{Op, Program};

    let mut program = Program::new();
    program.push(Op::Init);

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

    // For success message
    let pattern_names: Vec<&str> = req.patterns.iter().map(|p| p.name.as_str()).collect();
    let pattern_list = pattern_names.join(" -> ");

    // Print details at bottom if enabled
    if req.print_details {
        let divider = Divider::default();
        let mut divider_ops = Vec::new();
        divider.emit(&mut divider_ops);
        program.extend(divider_ops);

        // Each pattern with its params
        for (i, entry) in req.patterns.iter().enumerate() {
            let params_str = if entry.params.is_empty() {
                String::new()
            } else {
                let params: Vec<String> = entry
                    .params
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                format!(" ({})", params.join(", "))
            };
            let line = format!("{}. {}{}", i + 1, entry.name, params_str);
            let text = Text {
                content: line,
                size: [0, 0],
                ..Default::default()
            };
            let mut text_ops = Vec::new();
            text.emit(&mut text_ops);
            program.extend(text_ops);
            program.push(Op::Newline);
        }
    }

    program.push(Op::Feed { units: 24 }); // 6mm

    if req.cut {
        program.push(Op::Cut { partial: false });
    }

    // Split for long print and send to printer
    let device_path = state.config.device_path.clone();

    println!(
        "[weave] Print request: {} patterns, {}x{} pixels, mode={}",
        pattern_names.len(), width, height, req.mode
    );

    let print_result = tokio::task::spawn_blocking(move || {
        let programs = program.split_for_long_print();
        println!("[weave] Split into {} program(s)", programs.len());
        let mut transport = BluetoothTransport::open(&device_path)?;
        transport.send_programs(&programs)?;
        Ok::<_, crate::EstrellaError>(())
    })
    .await;

    match print_result {
        Ok(Ok(())) => Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("Weave printed: {}", pattern_list)
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
