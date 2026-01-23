//! Pattern API handlers.

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use image::{GrayImage, Luma};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Cursor, sync::Arc};

use crate::{
    art::ParamSpec,
    printer::PrinterConfig,
    render::{dither, patterns},
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
    Path(name): Path<String>,
    Query(query): Query<PreviewQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut pattern = patterns::by_name_golden(&name).ok_or(StatusCode::NOT_FOUND)?;

    // Apply custom params (skip the known query params)
    for (param_name, param_value) in &query.params {
        if param_name != "length_mm" && param_name != "dither" && param_name != "mode" {
            pattern
                .set_param(param_name, param_value)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
        }
    }

    // Calculate dimensions
    let config = PrinterConfig::TSP650II;
    let width = config.width_dots as usize;
    let height = config.mm_to_dots(query.length_mm) as usize;

    // Parse dithering algorithm
    let dither_algo = match query.dither.to_lowercase().as_str() {
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

    // Calculate dimensions
    let config = PrinterConfig::TSP650II;
    let width = config.width_dots as usize;
    let height = config.mm_to_dots(form.length_mm) as usize;

    // Parse dithering algorithm
    let dither_algo = match form.dither.to_lowercase().as_str() {
        "floyd-steinberg" | "floyd_steinberg" | "fs" => dither::DitheringAlgorithm::FloydSteinberg,
        "atkinson" => dither::DitheringAlgorithm::Atkinson,
        "jarvis" | "jjn" => dither::DitheringAlgorithm::Jarvis,
        _ => dither::DitheringAlgorithm::Bayer,
    };

    // Render pattern
    let raster_data = patterns::render(pattern.as_ref(), width, height, dither_algo);

    // Build print command based on mode
    use crate::components::{Component, Divider, Text};
    use crate::ir::{Op, Program};
    use crate::protocol::text::Font;

    let mut program = Program::new();
    program.push(Op::Init);

    // Print title if details enabled
    if form.print_details {
        // Title
        let title = Text::new(pattern.name()).center().bold().size(2, 1);
        let mut title_ops = Vec::new();
        title.emit(&mut title_ops);
        program.extend(title_ops);
        program.push(Op::Newline);

        // Divider
        let divider = Divider::dashed();
        let mut divider_ops = Vec::new();
        divider.emit(&mut divider_ops);
        program.extend(divider_ops);
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
        let divider = Divider::dashed();
        let mut divider_ops = Vec::new();
        divider.emit(&mut divider_ops);
        program.extend(divider_ops);

        // Parameters
        let params_list = pattern.list_params();
        if !params_list.is_empty() {
            let params_text = params_list
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(", ");
            let params = Text::new(&params_text).center().font(Font::B);
            let mut params_ops = Vec::new();
            params.emit(&mut params_ops);
            program.extend(params_ops);
            program.push(Op::Newline);
        }
    }

    program.push(Op::Feed { units: 24 }); // 6mm

    if form.cut {
        program.push(Op::Cut { partial: false });
    }

    let print_data = program.to_bytes();

    // Print to device
    let device_path = state.config.device_path.clone();
    let print_result = tokio::task::spawn_blocking(move || {
        let mut transport = BluetoothTransport::open(&device_path)?;
        transport.write_all(&print_data)?;
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
