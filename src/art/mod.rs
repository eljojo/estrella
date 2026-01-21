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

pub mod calibration;
pub mod crystal;
pub mod density;
pub mod erosion;
pub mod flowfield;
pub mod glitch;
pub mod jitter;
pub mod microfeed;
pub mod mycelium;
pub mod overburn;
pub mod plasma;
pub mod riley;
pub mod rings;
pub mod ripple;
pub mod scintillate;
pub mod topography;
pub mod vasarely;
pub mod waves;

/// All available patterns, in display order.
pub const PATTERNS: &[&str] = &[
    // Classic patterns
    "ripple",
    "waves",
    "plasma",
    "rings",
    "topography",
    "glitch",
    // Op art
    "riley",
    "vasarely",
    "scintillate",
    // Generative/organic
    "flowfield",
    "erosion",
    "crystal",
    "mycelium",
    // Diagnostic
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

    /// Human-readable description of the current parameters.
    fn params_description(&self) -> String {
        String::new()
    }

    /// Set a parameter by name. Returns error if param name is unknown or value is invalid.
    fn set_param(&mut self, name: &str, _value: &str) -> Result<(), String> {
        Err(format!("Pattern '{}' has no configurable params or unknown param '{}'", self.name(), name))
    }

    /// List available parameters as (name, current_value) pairs.
    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![]
    }
}

/// Get a pattern by name with golden (deterministic) parameters.
/// Used for golden tests and reproducible output.
pub fn by_name(name: &str) -> Option<Box<dyn Pattern>> {
    by_name_golden(name)
}

/// Get a pattern by name with golden (deterministic) parameters.
pub fn by_name_golden(name: &str) -> Option<Box<dyn Pattern>> {
    match name.to_lowercase().as_str() {
        "ripple" => Some(Box::new(ripple::Ripple::golden())),
        "waves" => Some(Box::new(waves::Waves::golden())),
        "plasma" => Some(Box::new(plasma::Plasma::golden())),
        "rings" => Some(Box::new(rings::Rings::golden())),
        "topography" => Some(Box::new(topography::Topography::golden())),
        "glitch" => Some(Box::new(glitch::Glitch::golden())),
        "riley" => Some(Box::new(riley::Riley::golden())),
        "vasarely" => Some(Box::new(vasarely::Vasarely::golden())),
        "scintillate" => Some(Box::new(scintillate::Scintillate::golden())),
        "flowfield" => Some(Box::new(flowfield::Flowfield::golden())),
        "erosion" => Some(Box::new(erosion::Erosion::golden())),
        "crystal" => Some(Box::new(crystal::Crystal::golden())),
        "mycelium" => Some(Box::new(mycelium::Mycelium::golden())),
        "microfeed" => Some(Box::new(microfeed::Microfeed::golden())),
        "density" => Some(Box::new(density::Density::golden())),
        "overburn" => Some(Box::new(overburn::Overburn::golden())),
        "jitter" => Some(Box::new(jitter::Jitter::golden())),
        "calibration" | "demo" => Some(Box::new(calibration::Calibration::golden())),
        _ => None,
    }
}

/// Get a pattern by name with randomized parameters for unique prints.
pub fn by_name_random(name: &str) -> Option<Box<dyn Pattern>> {
    match name.to_lowercase().as_str() {
        "ripple" => Some(Box::new(ripple::Ripple::random())),
        "waves" => Some(Box::new(waves::Waves::random())),
        "plasma" => Some(Box::new(plasma::Plasma::random())),
        "rings" => Some(Box::new(rings::Rings::random())),
        "topography" => Some(Box::new(topography::Topography::random())),
        "glitch" => Some(Box::new(glitch::Glitch::random())),
        "riley" => Some(Box::new(riley::Riley::random())),
        "vasarely" => Some(Box::new(vasarely::Vasarely::random())),
        "scintillate" => Some(Box::new(scintillate::Scintillate::random())),
        "flowfield" => Some(Box::new(flowfield::Flowfield::random())),
        "erosion" => Some(Box::new(erosion::Erosion::random())),
        "crystal" => Some(Box::new(crystal::Crystal::random())),
        "mycelium" => Some(Box::new(mycelium::Mycelium::random())),
        "microfeed" => Some(Box::new(microfeed::Microfeed::random())),
        "density" => Some(Box::new(density::Density::random())),
        "overburn" => Some(Box::new(overburn::Overburn::random())),
        "jitter" => Some(Box::new(jitter::Jitter::random())),
        "calibration" | "demo" => Some(Box::new(calibration::Calibration::random())),
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
