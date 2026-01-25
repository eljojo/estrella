//! # Composer: Layer-Based Pattern Composition
//!
//! Compose multiple patterns into a single image with positioning, sizing,
//! blend modes, and opacity controls.
//!
//! ## Example
//!
//! ```
//! use estrella::render::composer::{ComposerSpec, LayerSpec, BlendMode, Composer};
//! use estrella::render::dither::DitheringAlgorithm;
//! use std::collections::HashMap;
//!
//! let spec = ComposerSpec {
//!     width: 576,
//!     height: 500,
//!     background: 0.0,
//!     layers: vec![
//!         LayerSpec {
//!             pattern: "ripple".to_string(),
//!             params: HashMap::new(),
//!             x: 0,
//!             y: 0,
//!             width: 576,
//!             height: 500,
//!             blend_mode: BlendMode::Normal,
//!             opacity: 1.0,
//!         },
//!     ],
//! };
//!
//! let composer = Composer::from_spec(&spec).unwrap();
//! let raster = composer.render(DitheringAlgorithm::FloydSteinberg);
//! ```

use crate::art::{self, Pattern};
use crate::render::dither::{generate_raster, DitheringAlgorithm};
use crate::shader::{blend_add, blend_difference, blend_multiply, blend_overlay, blend_screen, lerp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Blend modes for compositing layers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    /// Normal blending - top layer replaces bottom based on opacity.
    #[default]
    Normal,
    /// Multiply - darkens (useful for shadows and overlays).
    Multiply,
    /// Screen - lightens (useful for glows and highlights).
    Screen,
    /// Overlay - increases contrast, combines multiply and screen.
    Overlay,
    /// Add - additive blending (clamped to 1.0).
    Add,
    /// Difference - absolute difference between layers.
    Difference,
    /// Min - takes the darker of two values.
    Min,
    /// Max - takes the lighter of two values.
    Max,
}

impl BlendMode {
    /// Apply this blend mode to combine base and blend values.
    ///
    /// Both values should be in [0.0, 1.0] where 0.0 is white and 1.0 is black.
    #[inline]
    pub fn apply(self, base: f32, blend: f32) -> f32 {
        match self {
            BlendMode::Normal => blend,
            BlendMode::Multiply => blend_multiply(base, blend),
            BlendMode::Screen => blend_screen(base, blend),
            BlendMode::Overlay => blend_overlay(base, blend),
            BlendMode::Add => blend_add(base, blend),
            BlendMode::Difference => blend_difference(base, blend),
            BlendMode::Min => base.min(blend),
            BlendMode::Max => base.max(blend),
        }
    }

    /// List all available blend modes.
    pub fn all() -> &'static [BlendMode] {
        &[
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
            BlendMode::Add,
            BlendMode::Difference,
            BlendMode::Min,
            BlendMode::Max,
        ]
    }

    /// Get the name of this blend mode.
    pub fn name(&self) -> &'static str {
        match self {
            BlendMode::Normal => "normal",
            BlendMode::Multiply => "multiply",
            BlendMode::Screen => "screen",
            BlendMode::Overlay => "overlay",
            BlendMode::Add => "add",
            BlendMode::Difference => "difference",
            BlendMode::Min => "min",
            BlendMode::Max => "max",
        }
    }
}

/// Specification for a single layer in a composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerSpec {
    /// Pattern name (e.g., "ripple", "vasarely").
    pub pattern: String,

    /// Pattern-specific parameters.
    #[serde(default)]
    pub params: HashMap<String, String>,

    /// X position of the layer (can be negative for partial visibility).
    #[serde(default)]
    pub x: i32,

    /// Y position of the layer (can be negative for partial visibility).
    #[serde(default)]
    pub y: i32,

    /// Width of the layer in pixels.
    pub width: usize,

    /// Height of the layer in pixels.
    pub height: usize,

    /// Blend mode for compositing.
    #[serde(default)]
    pub blend_mode: BlendMode,

    /// Opacity (0.0 = transparent, 1.0 = fully opaque).
    #[serde(default = "default_opacity")]
    pub opacity: f32,
}

fn default_opacity() -> f32 {
    1.0
}

fn default_width() -> usize {
    576
}

fn default_height() -> usize {
    500
}

/// Specification for a complete composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposerSpec {
    /// Canvas width in pixels (default: 576 for thermal printer).
    #[serde(default = "default_width")]
    pub width: usize,

    /// Canvas height in pixels.
    #[serde(default = "default_height")]
    pub height: usize,

    /// Background intensity (0.0 = white, 1.0 = black).
    #[serde(default)]
    pub background: f32,

    /// Layers in bottom-to-top order.
    #[serde(default)]
    pub layers: Vec<LayerSpec>,
}

impl Default for ComposerSpec {
    fn default() -> Self {
        Self {
            width: 576,
            height: 500,
            background: 0.0,
            layers: Vec::new(),
        }
    }
}

/// A compiled layer ready for rendering.
struct CompiledLayer {
    pattern: Box<dyn Pattern>,
    spec: LayerSpec,
}

/// Composer for rendering layered pattern compositions.
pub struct Composer {
    width: usize,
    height: usize,
    background: f32,
    layers: Vec<CompiledLayer>,
}

impl std::fmt::Debug for Composer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Composer")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("background", &self.background)
            .field("layer_count", &self.layers.len())
            .finish()
    }
}

impl Composer {
    /// Create a Composer from a specification.
    ///
    /// Returns an error if any pattern name is unknown or parameters are invalid.
    pub fn from_spec(spec: &ComposerSpec) -> Result<Self, String> {
        let mut layers = Vec::with_capacity(spec.layers.len());

        for (i, layer_spec) in spec.layers.iter().enumerate() {
            // Get the pattern by name
            let mut pattern = art::by_name(&layer_spec.pattern).ok_or_else(|| {
                format!(
                    "Layer {}: unknown pattern '{}'",
                    i, layer_spec.pattern
                )
            })?;

            // Apply any custom parameters
            for (name, value) in &layer_spec.params {
                pattern.set_param(name, value).map_err(|e| {
                    format!("Layer {} ({}): {}", i, layer_spec.pattern, e)
                })?;
            }

            layers.push(CompiledLayer {
                pattern,
                spec: layer_spec.clone(),
            });
        }

        Ok(Self {
            width: spec.width,
            height: spec.height,
            background: spec.background.clamp(0.0, 1.0),
            layers,
        })
    }

    /// Get the canvas width.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get the canvas height.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Compute the intensity at a specific pixel position.
    ///
    /// Returns a value in [0.0, 1.0] where 0.0 is white and 1.0 is black.
    pub fn intensity(&self, x: usize, y: usize) -> f32 {
        let mut result = self.background;

        for layer in &self.layers {
            let spec = &layer.spec;

            // Check if pixel is within layer bounds
            let layer_x = x as i32 - spec.x;
            let layer_y = y as i32 - spec.y;

            if layer_x < 0
                || layer_y < 0
                || layer_x >= spec.width as i32
                || layer_y >= spec.height as i32
            {
                continue;
            }

            // Sample the pattern at local coordinates
            let local_x = layer_x as usize;
            let local_y = layer_y as usize;
            let intensity = layer.pattern.intensity(local_x, local_y, spec.width, spec.height);

            // Apply blend mode
            let blended = spec.blend_mode.apply(result, intensity);

            // Apply opacity
            result = lerp(result, blended, spec.opacity);
        }

        result.clamp(0.0, 1.0)
    }

    /// Render the composition to a dithered raster.
    ///
    /// Returns packed byte data suitable for printer graphics commands.
    pub fn render(&self, algorithm: DitheringAlgorithm) -> Vec<u8> {
        generate_raster(self.width, self.height, |x, y, _w, _h| {
            self.intensity(x, y)
        }, algorithm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_mode_normal() {
        assert!((BlendMode::Normal.apply(0.3, 0.7) - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_blend_mode_multiply() {
        assert!((BlendMode::Multiply.apply(0.5, 0.5) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn test_blend_mode_screen() {
        // Screen: 1 - (1 - 0.5) * (1 - 0.5) = 1 - 0.25 = 0.75
        assert!((BlendMode::Screen.apply(0.5, 0.5) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_blend_mode_add() {
        assert!((BlendMode::Add.apply(0.3, 0.4) - 0.7).abs() < 1e-6);
        // Clamped at 1.0
        assert!((BlendMode::Add.apply(0.8, 0.5) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_blend_mode_difference() {
        assert!((BlendMode::Difference.apply(0.7, 0.3) - 0.4).abs() < 1e-6);
        assert!((BlendMode::Difference.apply(0.3, 0.7) - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_blend_mode_min_max() {
        assert!((BlendMode::Min.apply(0.3, 0.7) - 0.3).abs() < 1e-6);
        assert!((BlendMode::Max.apply(0.3, 0.7) - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_composer_empty() {
        let spec = ComposerSpec {
            width: 100,
            height: 100,
            background: 0.5,
            layers: vec![],
        };
        let composer = Composer::from_spec(&spec).unwrap();

        // With no layers, intensity should be background everywhere
        assert!((composer.intensity(50, 50) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_composer_single_layer() {
        let spec = ComposerSpec {
            width: 100,
            height: 100,
            background: 0.0,
            layers: vec![LayerSpec {
                pattern: "calibration".to_string(),
                params: HashMap::new(),
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                blend_mode: BlendMode::Normal,
                opacity: 1.0,
            }],
        };
        let composer = Composer::from_spec(&spec).unwrap();

        // Intensity should come from the pattern
        let intensity = composer.intensity(50, 50);
        assert!(intensity >= 0.0 && intensity <= 1.0);
    }

    #[test]
    fn test_composer_offset_layer() {
        let spec = ComposerSpec {
            width: 100,
            height: 100,
            background: 0.0,
            layers: vec![LayerSpec {
                pattern: "calibration".to_string(),
                params: HashMap::new(),
                x: 50,
                y: 50,
                width: 50,
                height: 50,
                blend_mode: BlendMode::Normal,
                opacity: 1.0,
            }],
        };
        let composer = Composer::from_spec(&spec).unwrap();

        // Outside layer bounds should be background
        assert!((composer.intensity(10, 10) - 0.0).abs() < 1e-6);

        // Inside layer bounds should have pattern intensity
        let intensity = composer.intensity(60, 60);
        assert!(intensity >= 0.0 && intensity <= 1.0);
    }

    #[test]
    fn test_composer_opacity() {
        let spec = ComposerSpec {
            width: 100,
            height: 100,
            background: 0.0,
            layers: vec![LayerSpec {
                pattern: "calibration".to_string(),
                params: HashMap::new(),
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                blend_mode: BlendMode::Normal,
                opacity: 0.5,
            }],
        };
        let composer = Composer::from_spec(&spec).unwrap();

        // With 50% opacity on white background (0.0), result should be half the pattern intensity
        let full_spec = ComposerSpec {
            width: 100,
            height: 100,
            background: 0.0,
            layers: vec![LayerSpec {
                pattern: "calibration".to_string(),
                params: HashMap::new(),
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                blend_mode: BlendMode::Normal,
                opacity: 1.0,
            }],
        };
        let full_composer = Composer::from_spec(&full_spec).unwrap();

        let half_intensity = composer.intensity(50, 50);
        let full_intensity = full_composer.intensity(50, 50);

        // lerp(0.0, full_intensity, 0.5) = full_intensity * 0.5
        let expected = full_intensity * 0.5;
        assert!((half_intensity - expected).abs() < 1e-6);
    }

    #[test]
    fn test_composer_unknown_pattern() {
        let spec = ComposerSpec {
            width: 100,
            height: 100,
            background: 0.0,
            layers: vec![LayerSpec {
                pattern: "nonexistent_pattern".to_string(),
                params: HashMap::new(),
                x: 0,
                y: 0,
                width: 100,
                height: 100,
                blend_mode: BlendMode::Normal,
                opacity: 1.0,
            }],
        };

        let result = Composer::from_spec(&spec);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown pattern"));
    }

    #[test]
    fn test_composer_render_dimensions() {
        let spec = ComposerSpec {
            width: 64,
            height: 32,
            background: 0.5,
            layers: vec![],
        };
        let composer = Composer::from_spec(&spec).unwrap();
        let raster = composer.render(DitheringAlgorithm::Bayer);

        // 64 pixels wide = 8 bytes per row, 32 rows = 256 bytes
        assert_eq!(raster.len(), 8 * 32);
    }

    #[test]
    fn test_layer_spec_serialization() {
        let layer = LayerSpec {
            pattern: "ripple".to_string(),
            params: HashMap::from([("scale".to_string(), "8.0".to_string())]),
            x: 10,
            y: 20,
            width: 200,
            height: 150,
            blend_mode: BlendMode::Multiply,
            opacity: 0.7,
        };

        let json = serde_json::to_string(&layer).unwrap();
        let parsed: LayerSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.pattern, "ripple");
        assert_eq!(parsed.x, 10);
        assert_eq!(parsed.blend_mode, BlendMode::Multiply);
        assert!((parsed.opacity - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_composer_spec_serialization() {
        let spec = ComposerSpec {
            width: 576,
            height: 500,
            background: 0.1,
            layers: vec![
                LayerSpec {
                    pattern: "ripple".to_string(),
                    params: HashMap::new(),
                    x: 0,
                    y: 0,
                    width: 576,
                    height: 500,
                    blend_mode: BlendMode::Normal,
                    opacity: 1.0,
                },
            ],
        };

        let json = serde_json::to_string_pretty(&spec).unwrap();
        let parsed: ComposerSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.width, 576);
        assert_eq!(parsed.layers.len(), 1);
        assert_eq!(parsed.layers[0].pattern, "ripple");
    }

    #[test]
    fn test_composer_spec_defaults() {
        let json = r#"{
            "layers": []
        }"#;

        let spec: ComposerSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.width, 576);
        assert_eq!(spec.height, 500);
        assert!((spec.background - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_layer_spec_defaults() {
        let json = r#"{
            "pattern": "ripple",
            "width": 100,
            "height": 100
        }"#;

        let layer: LayerSpec = serde_json::from_str(json).unwrap();
        assert_eq!(layer.x, 0);
        assert_eq!(layer.y, 0);
        assert_eq!(layer.blend_mode, BlendMode::Normal);
        assert!((layer.opacity - 1.0).abs() < 1e-6);
    }
}
