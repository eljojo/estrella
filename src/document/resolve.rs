//! Image resolution: downloads and processes images from URLs.
//!
//! `ImageResolver` handles all image fetching concerns so that `Document`
//! stays a pure data model with no HTTP or caching knowledge.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use image::{DynamicImage, imageops::FilterType};

use super::graphics::parse_dither_algorithm;
use super::types::ResolvedImage;
use super::{Component, Document};
use crate::EstrellaError;
use crate::render::context::RenderContext;
use crate::render::dither::{self, DitheringAlgorithm};
use crate::server::PhotoSession;

/// Resolves external resources (images) in a document.
///
/// Downloads images from URLs, caches them in the shared photo session store,
/// and processes them into raster data ready for printing.
pub struct ImageResolver {
    sessions: Arc<RwLock<HashMap<String, PhotoSession>>>,
}

impl ImageResolver {
    /// Create a resolver backed by a shared session cache.
    pub fn new(sessions: Arc<RwLock<HashMap<String, PhotoSession>>>) -> Self {
        Self { sessions }
    }

    /// Resolve all Image components in a document.
    ///
    /// Downloads images from URLs (using the cache when possible),
    /// resizes and dithers them, and populates `resolved_data`.
    /// Recurses into Canvas elements to resolve nested images.
    ///
    /// `print_width` is used as the default image width when no explicit
    /// width is set on an Image component.
    pub async fn resolve(
        &self,
        doc: &mut Document,
        print_width: usize,
    ) -> Result<(), EstrellaError> {
        for component in &mut doc.document {
            self.resolve_component(component, print_width).await?;
        }
        Ok(())
    }

    /// Recursively resolve images within a single component.
    fn resolve_component<'a>(
        &'a self,
        component: &'a mut Component,
        print_width: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EstrellaError>> + Send + 'a>>
    {
        Box::pin(async move {
            match component {
                Component::Image(img) => {
                    if !img.url.is_empty() && img.resolved_data.is_none() {
                        let source = fetch_image(&img.url, &self.sessions).await?;
                        let resolved = process_image(
                            source,
                            img.width.unwrap_or(print_width),
                            img.height,
                            img.dither.as_deref(),
                        );
                        img.resolved_data = Some(resolved);
                    }
                }
                Component::Canvas(canvas) => {
                    for element in &mut canvas.elements {
                        self.resolve_component(&mut element.component, print_width)
                            .await?;
                    }
                }
                _ => {}
            }
            Ok(())
        })
    }
}

/// Fetch an image from a URL using the render context's shared resources.
///
/// Uses the context's HTTP client and image cache. Downloads the image if
/// not cached, stores it in the cache, and returns the decoded image.
pub async fn fetch_image_with_ctx(
    url: &str,
    ctx: &RenderContext,
) -> Result<DynamicImage, EstrellaError> {
    // Check cache
    {
        let mut sessions = ctx.image_cache.write().await;
        if let Some(session) = sessions.get_mut(url) {
            session.touch();
            return Ok(session.image.clone());
        }
    }

    // Download using the context's HTTP client
    let response = ctx
        .http_client
        .get(url)
        .send()
        .await
        .map_err(|e| EstrellaError::Image(format!("Failed to download {}: {}", url, e)))?;
    if !response.status().is_success() {
        return Err(EstrellaError::Image(format!(
            "Failed to download {}: HTTP {}",
            url,
            response.status()
        )));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| EstrellaError::Image(format!("Failed to read image data: {}", e)))?;

    let image = image::load_from_memory(&bytes)
        .map_err(|e| EstrellaError::Image(format!("Failed to decode image: {}", e)))?;

    // Store in cache
    {
        let mut sessions = ctx.image_cache.write().await;
        sessions.insert(url.to_string(), PhotoSession::new(image.clone()));
    }

    Ok(image)
}

/// Fetch an image from a URL, using the session cache when available.
///
/// Convenience wrapper for callers that don't have a RenderContext.
pub async fn fetch_image(
    url: &str,
    sessions: &Arc<RwLock<HashMap<String, PhotoSession>>>,
) -> Result<DynamicImage, EstrellaError> {
    let ctx = RenderContext::new(
        reqwest::Client::builder()
            .user_agent("estrella/0.1")
            .build()
            .map_err(|e| EstrellaError::Image(format!("HTTP client error: {}", e)))?,
        sessions.clone(),
        Arc::new(RwLock::new(HashMap::new())),
    );
    fetch_image_with_ctx(url, &ctx).await
}

/// Process a downloaded image for printing.
///
/// Resizes to `target_width` preserving aspect ratio.
/// If `max_height` is set and the result is taller, resizes to fit within
/// that height constraint. Dithers with the specified algorithm (default:
/// Floyd-Steinberg).
fn process_image(
    source: DynamicImage,
    target_width: usize,
    max_height: Option<usize>,
    dither_str: Option<&str>,
) -> ResolvedImage {
    let dither_algo = dither_str
        .and_then(parse_dither_algorithm)
        .unwrap_or(DitheringAlgorithm::FloydSteinberg);

    // Resize to target width, preserving aspect ratio
    let aspect = source.height() as f32 / source.width() as f32;
    let scaled_height = (target_width as f32 * aspect).round() as u32;
    let mut resized = source.resize_exact(target_width as u32, scaled_height, FilterType::Lanczos3);

    // Apply max height constraint
    if let Some(max_h) = max_height
        && scaled_height > max_h as u32
    {
        resized = resized.resize(target_width as u32, max_h as u32, FilterType::Lanczos3);
    }

    let width = resized.width() as usize;
    let height = resized.height() as usize;
    let grayscale = resized.to_luma8();

    let raster_data = dither::generate_raster(
        width,
        height,
        |x, y, _w, _h| {
            let pixel = grayscale.get_pixel(x as u32, y as u32);
            1.0 - (pixel[0] as f32 / 255.0)
        },
        dither_algo,
    );

    ResolvedImage {
        raster_data,
        width: width as u16,
        height: height as u16,
    }
}
