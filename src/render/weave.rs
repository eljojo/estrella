//! # Pattern Weaving
//!
//! Blend multiple patterns together with smooth crossfade transitions,
//! like a DJ mixing between tracks.

use crate::render::patterns::Pattern;

/// Blend curve types for crossfade transitions.
#[derive(Debug, Clone, Copy, Default)]
pub enum BlendCurve {
    /// Linear interpolation (constant rate)
    Linear,
    /// Smooth S-curve (slow start, fast middle, slow end)
    #[default]
    Smooth,
    /// Ease in (slow start, fast end)
    EaseIn,
    /// Ease out (fast start, slow end)
    EaseOut,
}

impl BlendCurve {
    /// Parse a blend curve from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "linear" => Some(BlendCurve::Linear),
            "smooth" => Some(BlendCurve::Smooth),
            "ease-in" | "easein" | "ease_in" => Some(BlendCurve::EaseIn),
            "ease-out" | "easeout" | "ease_out" => Some(BlendCurve::EaseOut),
            _ => None,
        }
    }

    /// Apply the blend curve to a linear t value [0, 1].
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            BlendCurve::Linear => t,
            BlendCurve::Smooth => {
                // Smoothstep: 3t² - 2t³
                t * t * (3.0 - 2.0 * t)
            }
            BlendCurve::EaseIn => {
                // Quadratic ease in: t²
                t * t
            }
            BlendCurve::EaseOut => {
                // Quadratic ease out: 1 - (1-t)²
                1.0 - (1.0 - t) * (1.0 - t)
            }
        }
    }
}

/// Configuration for pattern weaving.
pub struct WeaveConfig {
    /// Crossfade length in pixels (rows).
    pub crossfade_pixels: usize,
    /// Blend curve for transitions.
    pub curve: BlendCurve,
}

impl Default for WeaveConfig {
    fn default() -> Self {
        Self {
            crossfade_pixels: 240, // ~30mm at 203 DPI
            curve: BlendCurve::Smooth,
        }
    }
}

/// A weave of multiple patterns that blend into each other.
pub struct Weave<'a> {
    patterns: Vec<&'a dyn Pattern>,
    config: WeaveConfig,
}

impl<'a> Weave<'a> {
    /// Create a new weave from patterns.
    pub fn new(patterns: Vec<&'a dyn Pattern>) -> Self {
        Self {
            patterns,
            config: WeaveConfig::default(),
        }
    }

    /// Set the crossfade length in pixels.
    pub fn crossfade_pixels(mut self, pixels: usize) -> Self {
        self.config.crossfade_pixels = pixels;
        self
    }

    /// Set the blend curve.
    pub fn curve(mut self, curve: BlendCurve) -> Self {
        self.config.curve = curve;
        self
    }

    /// Compute intensity at a pixel position.
    ///
    /// This handles blending between patterns during crossfade zones.
    pub fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        let n = self.patterns.len();
        if n == 0 {
            return 0.0;
        }
        if n == 1 {
            return self.patterns[0].intensity(x, y, width, height);
        }

        let y_f = y as f32;
        let height_f = height as f32;
        let crossfade_f = self.config.crossfade_pixels as f32;
        let half_crossfade = crossfade_f / 2.0;

        // Transition points are at height * i / n for i in 1..n
        // Each transition has a crossfade zone centered on it
        //
        // Example with 3 patterns, height=900, crossfade=100:
        //   Transition 1 at y=300: crossfade from 250-350
        //   Transition 2 at y=600: crossfade from 550-650
        //
        // Pattern zones:
        //   Pattern 0: 0-250 solo, 250-350 fading out
        //   Pattern 1: 250-350 fading in, 350-550 solo, 550-650 fading out
        //   Pattern 2: 550-650 fading in, 650-900 solo

        // Check each transition point
        for i in 1..n {
            let transition_y = height_f * i as f32 / n as f32;
            let fade_start = transition_y - half_crossfade;
            let fade_end = transition_y + half_crossfade;

            if y_f >= fade_start && y_f < fade_end {
                // We're in a crossfade zone between pattern i-1 and pattern i
                let t = (y_f - fade_start) / crossfade_f;
                let t = self.config.curve.apply(t);

                let a = self.patterns[i - 1].intensity(x, y, width, height);
                let b = self.patterns[i].intensity(x, y, width, height);

                return a * (1.0 - t) + b * t;
            }
        }

        // Not in a crossfade zone - find which pattern's solo zone we're in
        for i in 0..n {
            let zone_start = if i == 0 {
                0.0
            } else {
                height_f * i as f32 / n as f32 + half_crossfade
            };
            let zone_end = if i == n - 1 {
                height_f
            } else {
                height_f * (i + 1) as f32 / n as f32 - half_crossfade
            };

            if y_f >= zone_start && y_f < zone_end {
                return self.patterns[i].intensity(x, y, width, height);
            }
        }

        // Fallback to last pattern
        self.patterns[n - 1].intensity(x, y, width, height)
    }

    /// Get the pattern names for display.
    pub fn pattern_names(&self) -> Vec<&'static str> {
        self.patterns.iter().map(|p| p.name()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_curves() {
        // Linear should be identity
        assert!((BlendCurve::Linear.apply(0.5) - 0.5).abs() < 0.001);

        // Smooth should equal 0.5 at t=0.5 (symmetric)
        assert!((BlendCurve::Smooth.apply(0.5) - 0.5).abs() < 0.001);

        // EaseIn should be slower at start (value < t for small t)
        assert!(BlendCurve::EaseIn.apply(0.3) < 0.3);

        // EaseOut should be faster at start (value > t for small t)
        assert!(BlendCurve::EaseOut.apply(0.3) > 0.3);

        // All should hit 0 and 1 at endpoints
        for curve in [BlendCurve::Linear, BlendCurve::Smooth, BlendCurve::EaseIn, BlendCurve::EaseOut] {
            assert!((curve.apply(0.0)).abs() < 0.001);
            assert!((curve.apply(1.0) - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_curve_from_str() {
        assert!(matches!(BlendCurve::from_str("linear"), Some(BlendCurve::Linear)));
        assert!(matches!(BlendCurve::from_str("smooth"), Some(BlendCurve::Smooth)));
        assert!(matches!(BlendCurve::from_str("ease-in"), Some(BlendCurve::EaseIn)));
        assert!(matches!(BlendCurve::from_str("ease-out"), Some(BlendCurve::EaseOut)));
        assert!(BlendCurve::from_str("invalid").is_none());
    }
}
