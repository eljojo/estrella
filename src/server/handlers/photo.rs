//! Photo upload and printing API handlers.

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use image::{imageops::FilterType, DynamicImage, RgbImage};
use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Instant};
use uuid::Uuid;

use crate::{
    printer::PrinterConfig,
    render::{
        self,
        dither::{self, DitheringAlgorithm},
    },
    transport::BluetoothTransport,
};

use super::super::state::{AppState, PhotoSession, SESSION_EXPIRATION_SECS};

/// Response from upload endpoint.
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub filename: String,
    pub width: u32,
    pub height: u32,
    /// True if the image is already binary (1-bit black/white)
    pub is_binary: bool,
}

/// Query parameters for preview endpoint.
#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    #[serde(default)]
    pub rotation: i32,
    #[serde(default = "default_dither")]
    pub dither: String,
    #[serde(default)]
    pub brightness: i32,
    #[serde(default)]
    pub contrast: i32,
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

/// Request body for print endpoint.
#[derive(Debug, Deserialize)]
pub struct PrintRequest {
    #[serde(default)]
    pub rotation: i32,
    #[serde(default = "default_dither")]
    pub dither: String,
    #[serde(default)]
    pub brightness: i32,
    #[serde(default)]
    pub contrast: i32,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_true")]
    pub cut: bool,
}

/// POST /api/photo/upload - Upload an image file.
pub async fn upload(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    // Clean up expired sessions first
    cleanup_expired_sessions(&state).await;

    // Extract the image field from multipart
    let mut image_data: Option<Vec<u8>> = None;
    let mut filename = String::from("unknown");

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "image" {
            filename = field
                .file_name()
                .unwrap_or("unknown")
                .to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read image: {}", e)))?;
            image_data = Some(bytes.to_vec());
            break;
        }
    }

    let image_bytes = image_data
        .ok_or((StatusCode::BAD_REQUEST, "No image field found".to_string()))?;

    // Decode the image (try HEIC first if it looks like HEIC, otherwise use image crate)
    let img = if is_heic(&image_bytes) || filename.to_lowercase().ends_with(".heic") || filename.to_lowercase().ends_with(".heif") {
        decode_heic(&image_bytes)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to decode HEIC: {}", e)))?
    } else {
        image::load_from_memory(&image_bytes)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to decode image: {}", e)))?
    };

    let width = img.width();
    let height = img.height();

    // Check if the image is already binary (1-bit) BEFORE resizing
    // (resize interpolation creates gray pixels that would skew the detection)
    let is_binary = is_binary_image(&img);

    // Pre-resize to a reasonable size for preview generation
    // Use 1152px (2x printer width) as max dimension to handle any rotation
    // while keeping preview generation fast
    let max_dim = 1152u32;
    let img = if width > max_dim || height > max_dim {
        let scale = max_dim as f32 / width.max(height) as f32;
        let new_width = (width as f32 * scale).round() as u32;
        let new_height = (height as f32 * scale).round() as u32;
        img.resize(new_width, new_height, FilterType::Triangle)
    } else {
        img
    };

    // Generate session ID and store
    let session_id = Uuid::new_v4();
    let session = PhotoSession::new(img);

    {
        let mut sessions = state.photo_sessions.write().await;
        sessions.insert(session_id, session);
    }

    Ok(Json(UploadResponse {
        id: session_id.to_string(),
        filename,
        width,
        height,
        is_binary,
    }))
}

/// GET /api/photo/:id/preview - Generate PNG preview of uploaded image.
pub async fn preview(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<PreviewQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let session_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session ID".to_string()))?;

    // Get the image from session (minimize lock time)
    let source_image = {
        let mut sessions = state.photo_sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or((StatusCode::NOT_FOUND, "Session not found or expired".to_string()))?;

        // Touch session to keep it alive
        session.touch();

        // Clone the image to release the lock quickly
        session.image.clone()
    };

    // Parse parameters
    let rotation = query.rotation;
    let brightness = query.brightness;
    let contrast = query.contrast;
    let dither_algo = parse_dither(&query.dither);

    // Move CPU-intensive work to blocking thread pool
    let png_bytes = tokio::task::spawn_blocking(move || {
        generate_preview_png(source_image, rotation, brightness, contrast, dither_algo)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Processing error: {}", e),
        )
    })?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(([(header::CONTENT_TYPE, "image/png")], png_bytes))
}

/// Prepare image for printing: rotate, resize to printer width, adjust brightness/contrast.
fn prepare_for_print(
    source_image: DynamicImage,
    rotation: i32,
    brightness: i32,
    contrast: i32,
    filter: FilterType,
) -> DynamicImage {
    let target_width = PrinterConfig::TSP650II.width_dots as u32;

    // Rotate first to get correct orientation
    let rotated = match rotation % 360 {
        90 | -270 => source_image.rotate90(),
        180 | -180 => source_image.rotate180(),
        270 | -90 => source_image.rotate270(),
        _ => source_image,
    };

    // Resize to target width (576px)
    let aspect_ratio = rotated.height() as f32 / rotated.width() as f32;
    let target_height = (target_width as f32 * aspect_ratio).round() as u32;
    let resized = rotated.resize(target_width, target_height, filter);

    // Apply brightness/contrast
    apply_brightness_contrast_if_needed(&resized, brightness, contrast)
}

/// Generate dithered raster data from a grayscale image.
fn generate_dithered_raster(
    img: &DynamicImage,
    dither_algo: DitheringAlgorithm,
) -> (usize, usize, Vec<u8>) {
    let width = img.width() as usize;
    let height = img.height() as usize;
    let grayscale = img.to_luma8();

    let raster_data = dither::generate_raster(
        width,
        height,
        |x, y, _w, _h| {
            let pixel = grayscale.get_pixel(x as u32, y as u32);
            1.0 - (pixel[0] as f32 / 255.0)
        },
        dither_algo,
    );

    (width, height, raster_data)
}

/// Generate a dithered preview PNG (runs on blocking thread pool).
fn generate_preview_png(
    source_image: DynamicImage,
    rotation: i32,
    brightness: i32,
    contrast: i32,
    dither_algo: DitheringAlgorithm,
) -> Result<Vec<u8>, String> {
    // Use Triangle filter for speed in preview
    let processed = prepare_for_print(source_image, rotation, brightness, contrast, FilterType::Triangle);
    let (width, height, raster_data) = generate_dithered_raster(&processed, dither_algo);
    render::raster_to_png(width, height, &raster_data)
}

/// Generate raster data for printing (runs on blocking thread pool).
fn generate_print_raster(
    source_image: DynamicImage,
    rotation: i32,
    brightness: i32,
    contrast: i32,
    dither_algo: DitheringAlgorithm,
) -> (usize, usize, Vec<u8>) {
    // Use Lanczos3 for print quality
    let processed = prepare_for_print(source_image, rotation, brightness, contrast, FilterType::Lanczos3);
    generate_dithered_raster(&processed, dither_algo)
}

/// POST /api/photo/:id/print - Print the uploaded image.
pub async fn print(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<PrintRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let session_id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "error": "Invalid session ID"})),
        )
    })?;

    // Get the image from session (minimize lock time)
    let source_image = {
        let mut sessions = state.photo_sessions.write().await;
        let session = sessions.get_mut(&session_id).ok_or((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"success": false, "error": "Session not found or expired"})),
        ))?;

        // Touch session to keep it alive
        session.touch();

        // Clone the image to release the lock quickly
        session.image.clone()
    };

    // Parse parameters
    let rotation = req.rotation;
    let brightness = req.brightness;
    let contrast = req.contrast;
    let dither_algo = parse_dither(&req.dither);
    let mode = req.mode.clone();
    let cut = req.cut;
    let device_path = state.config.device_path.clone();

    // Move all CPU-intensive work to blocking thread pool
    let print_result = tokio::task::spawn_blocking(move || {
        // Generate raster data
        let (width, height, raster_data) =
            generate_print_raster(source_image, rotation, brightness, contrast, dither_algo);

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

        // Split for long print and send to printer
        println!(
            "[photo] Print request: {}x{} pixels, mode={}",
            width, height, mode
        );
        let programs = program.split_for_long_print();
        println!("[photo] Split into {} program(s)", programs.len());
        let mut transport = BluetoothTransport::open(&device_path)?;
        transport.send_programs(&programs)?;
        Ok::<_, crate::EstrellaError>(())
    })
    .await;

    match print_result {
        Ok(Ok(())) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Photo printed successfully"
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

/// Apply brightness and contrast if needed, otherwise return a clone.
fn apply_brightness_contrast_if_needed(img: &DynamicImage, brightness: i32, contrast: i32) -> DynamicImage {
    if brightness != 0 || contrast != 0 {
        apply_brightness_contrast(img, brightness, contrast)
    } else {
        img.clone()
    }
}

/// Apply brightness and contrast adjustments to an image.
fn apply_brightness_contrast(img: &DynamicImage, brightness: i32, contrast: i32) -> DynamicImage {
    let mut rgba = img.to_rgba8();

    // Brightness: -100 to 100 maps to -255 to 255 offset
    let brightness_offset = (brightness as f32 / 100.0) * 255.0;

    // Contrast: -100 to 100 maps to factor
    // At -100: factor = 0 (all gray)
    // At 0: factor = 1 (no change)
    // At 100: factor = 2 (max contrast)
    let contrast_factor = if contrast >= 0 {
        1.0 + (contrast as f32 / 100.0)
    } else {
        1.0 + (contrast as f32 / 100.0)
    };

    for pixel in rgba.pixels_mut() {
        for c in 0..3 {
            // Skip alpha channel
            let val = pixel[c] as f32;
            // Apply contrast (around midpoint 128)
            let val = (val - 128.0) * contrast_factor + 128.0;
            // Apply brightness
            let val = val + brightness_offset;
            // Clamp to valid range
            pixel[c] = val.clamp(0.0, 255.0) as u8;
        }
    }

    DynamicImage::ImageRgba8(rgba)
}

/// Parse dithering algorithm from string.
fn parse_dither(dither: &str) -> DitheringAlgorithm {
    match dither.to_lowercase().as_str() {
        "none" | "threshold" => DitheringAlgorithm::None,
        "floyd-steinberg" | "floyd_steinberg" | "fs" => DitheringAlgorithm::FloydSteinberg,
        "atkinson" => DitheringAlgorithm::Atkinson,
        "jarvis" | "jjn" => DitheringAlgorithm::Jarvis,
        "bayer" => DitheringAlgorithm::Bayer,
        _ => DitheringAlgorithm::FloydSteinberg,
    }
}

/// Clean up expired photo sessions.
async fn cleanup_expired_sessions(state: &AppState) {
    let now = Instant::now();
    let mut sessions = state.photo_sessions.write().await;

    sessions.retain(|_, session| {
        let elapsed = now.duration_since(session.last_accessed);
        elapsed.as_secs() < SESSION_EXPIRATION_SECS
    });
}

/// Check if an image is already binary (1-bit black/white).
/// Returns true if 80%+ of pixels are near pure black or pure white.
/// Uses a threshold of 20 to handle anti-aliasing and compression artifacts.
fn is_binary_image(img: &DynamicImage) -> bool {
    const THRESHOLD: u8 = 20;
    const MIN_BINARY_RATIO: f32 = 0.80;

    let gray = img.to_luma8();
    let total = gray.pixels().count();
    if total == 0 {
        return false;
    }

    let binary_count = gray
        .pixels()
        .filter(|p| p.0[0] <= THRESHOLD || p.0[0] >= (255 - THRESHOLD))
        .count();

    (binary_count as f32 / total as f32) >= MIN_BINARY_RATIO
}

/// Check if the data looks like a HEIC/HEIF file by examining magic bytes.
/// HEIC files have an "ftyp" box near the start with HEIC-related brand codes.
fn is_heic(data: &[u8]) -> bool {
    if data.len() < 12 {
        return false;
    }

    // HEIC files start with a box size (4 bytes) followed by "ftyp" (bytes 4-7)
    if &data[4..8] != b"ftyp" {
        return false;
    }

    // Check the brand (bytes 8-11) for HEIC-related identifiers
    let brand = &data[8..12];
    matches!(
        brand,
        b"heic" | b"heix" | b"hevc" | b"hevx" | b"heim" | b"heis" | b"hevm" | b"hevs" | b"mif1" | b"msf1" | b"avif"
    )
}

/// Decode a HEIC/HEIF image using libheif.
fn decode_heic(data: &[u8]) -> Result<DynamicImage, String> {
    let lib_heif = LibHeif::new();
    let ctx = HeifContext::read_from_bytes(data).map_err(|e| format!("Failed to read HEIC: {}", e))?;

    let handle = ctx
        .primary_image_handle()
        .map_err(|e| format!("Failed to get primary image: {}", e))?;

    let image = lib_heif
        .decode(&handle, ColorSpace::Rgb(RgbChroma::Rgb), None)
        .map_err(|e| format!("Failed to decode HEIC image: {}", e))?;

    let planes = image.planes();
    let interleaved = planes
        .interleaved
        .ok_or("No interleaved RGB data in HEIC")?;

    let width = image.width();
    let height = image.height();
    let stride = interleaved.stride;
    let data = interleaved.data;

    // Create an RgbImage from the raw data
    let mut rgb_image = RgbImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let offset = (y as usize * stride) + (x as usize * 3);
            if offset + 2 < data.len() {
                let r = data[offset];
                let g = data[offset + 1];
                let b = data[offset + 2];
                rgb_image.put_pixel(x, y, image::Rgb([r, g, b]));
            }
        }
    }

    Ok(DynamicImage::ImageRgb8(rgb_image))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn create_test_image(width: u32, height: u32) -> DynamicImage {
        let mut img = RgbImage::new(width, height);
        // Create a simple gradient to verify transformations
        for y in 0..height {
            for x in 0..width {
                img.put_pixel(x, y, Rgb([x as u8, y as u8, 128]));
            }
        }
        DynamicImage::ImageRgb8(img)
    }

    #[test]
    fn test_rotation_90() {
        let img = create_test_image(100, 50);
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 50);

        let rotated = img.rotate90();
        assert_eq!(rotated.width(), 50);
        assert_eq!(rotated.height(), 100);
    }

    #[test]
    fn test_rotation_180() {
        let img = create_test_image(100, 50);
        let rotated = img.rotate180();
        assert_eq!(rotated.width(), 100);
        assert_eq!(rotated.height(), 50);
    }

    #[test]
    fn test_rotation_270() {
        let img = create_test_image(100, 50);
        let rotated = img.rotate270();
        assert_eq!(rotated.width(), 50);
        assert_eq!(rotated.height(), 100);
    }

    #[test]
    fn test_brightness_increase() {
        let img = create_test_image(10, 10);
        let original_pixel = img.to_rgba8().get_pixel(5, 5).0;

        let brightened = apply_brightness_contrast(&img, 50, 0);
        let bright_pixel = brightened.to_rgba8().get_pixel(5, 5).0;

        // Brightness increase should make pixels brighter (higher values)
        assert!(
            bright_pixel[0] > original_pixel[0] || bright_pixel[0] == 255,
            "Red channel should be brighter: {} vs {}",
            bright_pixel[0],
            original_pixel[0]
        );
    }

    #[test]
    fn test_brightness_decrease() {
        let img = create_test_image(10, 10);
        let original_pixel = img.to_rgba8().get_pixel(5, 5).0;

        let darkened = apply_brightness_contrast(&img, -50, 0);
        let dark_pixel = darkened.to_rgba8().get_pixel(5, 5).0;

        // Brightness decrease should make pixels darker (lower values)
        assert!(
            dark_pixel[0] < original_pixel[0] || dark_pixel[0] == 0,
            "Red channel should be darker: {} vs {}",
            dark_pixel[0],
            original_pixel[0]
        );
    }

    #[test]
    fn test_contrast_increase() {
        // Create a gray image
        let mut img = RgbImage::new(10, 10);
        for y in 0..10 {
            for x in 0..10 {
                img.put_pixel(x, y, Rgb([100, 100, 100])); // Mid-gray-ish
            }
        }
        let img = DynamicImage::ImageRgb8(img);

        let contrasted = apply_brightness_contrast(&img, 0, 50);
        let pixel = contrasted.to_rgba8().get_pixel(5, 5).0;

        // With contrast increase, values below 128 should go lower
        // Original: 100, after contrast (factor 1.5): (100-128)*1.5+128 = 86
        assert!(pixel[0] < 100, "Expected contrast to move 100 away from 128, got {}", pixel[0]);
    }

    #[test]
    fn test_no_change_when_zero() {
        let img = create_test_image(10, 10);
        let original_pixel = img.to_rgba8().get_pixel(5, 5).0;

        let unchanged = apply_brightness_contrast_if_needed(&img, 0, 0);
        let new_pixel = unchanged.to_rgba8().get_pixel(5, 5).0;

        assert_eq!(original_pixel[0], new_pixel[0]);
        assert_eq!(original_pixel[1], new_pixel[1]);
        assert_eq!(original_pixel[2], new_pixel[2]);
    }
}
