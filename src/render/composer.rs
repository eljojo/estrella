//! Blend modes for layer compositing.
//!
//! Used by the Canvas component to composite elements with different blend modes.

use crate::shader::{blend_add, blend_difference, blend_multiply, blend_overlay, blend_screen};
use serde::{Deserialize, Serialize};

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
}
