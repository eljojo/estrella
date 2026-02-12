//! # Image Pattern
//!
//! A pattern backed by an image URL. Downloads the image during `prepare()`,
//! resizes it to the target dimensions, and converts to a grayscale intensity
//! buffer. After preparation, `intensity()` is a simple buffer lookup.
//!
//! Works everywhere patterns work (composer, weave, patterns tab, documents)
//! with no special-casing — the caller just calls `prepare()` + `intensity()`.

use async_trait::async_trait;
use image::{DynamicImage, imageops::FilterType};

use super::{ParamSpec, ParamType, Pattern};
use crate::document::resolve::fetch_image_with_ctx;
use crate::render::context::RenderContext;

/// Default fallback image URL (used when no URL is specified).
const DEFAULT_IMAGE_URL: &str =
    "https://upload.wikimedia.org/wikipedia/en/6/61/Attempted_restoration_of_Ecce_Homo.jpg";

/// A pattern that renders an image from a URL.
///
/// Set the URL via `set_param("url", "https://...")`, then call `prepare()`
/// to download and process the image. After preparation, `intensity()` reads
/// from a pre-computed grayscale buffer.
///
/// If no URL is set, falls back to a default image so the pattern is always
/// renderable (useful in the composer where a layer is added before a URL is entered).
#[derive(Debug, Clone)]
pub struct ImagePattern {
    url: String,
    /// Pre-rendered intensity buffer (populated by prepare()).
    buffer: Option<Vec<f32>>,
    /// Dimensions the buffer was rendered at.
    prepared_dims: Option<(usize, usize)>,
}

impl Default for ImagePattern {
    fn default() -> Self {
        Self {
            url: DEFAULT_IMAGE_URL.to_string(),
            buffer: None,
            prepared_dims: None,
        }
    }
}

impl ImagePattern {
    pub fn golden() -> Self {
        Self::default()
    }

    pub fn random() -> Self {
        Self {
            url: "https://picsum.photos/800/576".to_string(),
            buffer: None,
            prepared_dims: None,
        }
    }

    /// Convert a DynamicImage to a grayscale intensity buffer at given dimensions.
    fn image_to_intensity(image: &DynamicImage, width: usize, height: usize) -> Vec<f32> {
        let resized = image.resize_exact(width as u32, height as u32, FilterType::Lanczos3);
        let gray = resized.to_luma8();
        let mut buffer = vec![0.0f32; width * height];
        for y in 0..height {
            for x in 0..width {
                let pixel = gray.get_pixel(x as u32, y as u32);
                buffer[y * width + x] = 1.0 - (pixel[0] as f32 / 255.0);
            }
        }
        buffer
    }
}

#[async_trait]
impl Pattern for ImagePattern {
    fn name(&self) -> &'static str {
        "image"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, _height: usize) -> f32 {
        if let Some(ref buffer) = self.buffer
            && let Some((bw, _bh)) = self.prepared_dims
            && bw == width
        {
            let idx = y * width + x;
            if idx < buffer.len() {
                return buffer[idx];
            }
        }
        // Not prepared or dimensions mismatch — return white
        0.0
    }

    async fn prepare(
        &mut self,
        width: usize,
        height: usize,
        ctx: &RenderContext,
    ) -> Result<(), String> {
        // Skip if already prepared at these dimensions
        if let Some((bw, bh)) = self.prepared_dims
            && bw == width
            && bh == height
        {
            return Ok(());
        }

        // Fetch image (uses context's HTTP client and cache)
        let image = fetch_image_with_ctx(&self.url, ctx)
            .await
            .map_err(|e| format!("Image fetch failed: {}", e))?;

        // Resize and convert to intensity buffer
        self.buffer = Some(Self::image_to_intensity(&image, width, height));
        self.prepared_dims = Some((width, height));

        Ok(())
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "url" => {
                if self.url != value {
                    self.url = value.to_string();
                    // Invalidate prepared buffer when URL changes
                    self.buffer = None;
                    self.prepared_dims = None;
                }
                Ok(())
            }
            _ => Err(format!("Unknown param '{}' for image pattern", name)),
        }
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![("url", self.url.clone())]
    }

    fn param_specs(&self) -> Vec<ParamSpec> {
        vec![ParamSpec {
            name: "url",
            label: "Image URL",
            param_type: ParamType::Text,
            description: Some("URL of the image to render"),
        }]
    }
}
