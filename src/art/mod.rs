//! # Art Generation
//!
//! Visual pattern generators for thermal printing. Each pattern is self-contained
//! in its own module with a struct implementing the [`Pattern`] trait.
//!
//! ## Adding a New Pattern
//!
//! 1. Create `src/art/mypattern.rs` with a struct implementing [`Pattern`]
//! 2. Add `pub mod mypattern;` below
//! 3. Add to [`PATTERNS`] array
//! 4. Run `make golden` to generate test files

pub mod density;
pub mod glitch;
pub mod jitter;
pub mod microfeed;
pub mod overburn;
pub mod plasma;
pub mod rings;
pub mod ripple;
pub mod topography;
pub mod waves;
pub mod calibration;

/// All available patterns, in display order.
pub const PATTERNS: &[&str] = &[
    "ripple",
    "waves",
    "plasma",
    "rings",
    "topography",
    "glitch",
    "microfeed",
    "density",
    "overburn",
    "jitter",
    "calibration",
];

/// Trait for pattern generators.
pub trait Pattern: Send + Sync {
    /// Pattern name (lowercase, e.g., "ripple").
    fn name(&self) -> &'static str;

    /// Compute intensity at a pixel position. Returns 0.0 (white) to 1.0 (black).
    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32;

    /// Default dimensions (width, height) for this pattern.
    fn default_dimensions(&self) -> (usize, usize) {
        (576, 500)
    }
}

/// Get a pattern by name.
pub fn by_name(name: &str) -> Option<Box<dyn Pattern>> {
    match name.to_lowercase().as_str() {
        "ripple" => Some(Box::new(ripple::Ripple::default())),
        "waves" => Some(Box::new(waves::Waves::default())),
        "plasma" => Some(Box::new(plasma::Plasma::default())),
        "rings" => Some(Box::new(rings::Rings::default())),
        "topography" => Some(Box::new(topography::Topography::default())),
        "glitch" => Some(Box::new(glitch::Glitch::default())),
        "microfeed" => Some(Box::new(microfeed::Microfeed::default())),
        "density" => Some(Box::new(density::Density::default())),
        "overburn" => Some(Box::new(overburn::Overburn::default())),
        "jitter" => Some(Box::new(jitter::Jitter::default())),
        "calibration" | "demo" => Some(Box::new(calibration::Calibration::default())),
        _ => None,
    }
}

/// Clamp a value to [0.0, 1.0].
#[inline]
pub fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

/// Apply gamma correction to an intensity value.
#[inline]
pub fn gamma_correct(intensity: f32, gamma: f32) -> f32 {
    clamp01(intensity).powf(gamma)
}

/// Check if a pixel is within a border region.
#[inline]
pub fn in_border(x: usize, y: usize, width: usize, height: usize, border_width: f32) -> bool {
    let xf = x as f32;
    let yf = y as f32;
    let wf = width as f32;
    let hf = height as f32;
    xf < border_width || xf >= (wf - border_width) || yf < border_width || yf >= (hf - border_width)
}
