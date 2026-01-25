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

use serde::Serialize;

pub mod attractor;
pub mod automata;
pub mod calibration;
pub mod corrupt_barcode;
pub mod crosshatch;
pub mod crystal;
pub mod databend;
pub mod density;
pub mod erosion;
pub mod estrella;
pub mod flowfield;
pub mod glitch;
pub mod jitter;
pub mod microfeed;
pub mod moire;
pub mod mycelium;
pub mod overburn;
pub mod plasma;
pub mod reaction_diffusion;
pub mod riley;
pub mod riley_check;
pub mod riley_curve;
pub mod rings;
pub mod ripple;
pub mod scanline_tear;
pub mod scintillate;
pub mod stipple;
pub mod topography;
pub mod tunnel;
pub mod vasarely;
pub mod vasarely_bubbles;
pub mod vasarely_hex;
pub mod voronoi;
pub mod waves;
pub mod weave;
pub mod woodgrain;
pub mod zebra;

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
    "riley_check",
    "riley_curve",
    "vasarely",
    "vasarely_hex",
    "vasarely_bubbles",
    "scintillate",
    "tunnel",
    "zebra",
    // Generative/organic
    "flowfield",
    "erosion",
    "crystal",
    "mycelium",
    // Mascot
    "estrella",
    // Glitch / Digital
    "corrupt_barcode",
    "databend",
    "scanline_tear",
    // Algorithmic / Mathematical
    "moire",
    "reaction_diffusion",
    "attractor",
    "automata",
    "voronoi",
    // Texture / Tactile
    "crosshatch",
    "stipple",
    "woodgrain",
    "weave",
    // Diagnostic
    "microfeed",
    "density",
    "overburn",
    "jitter",
    "calibration",
];

/// Input type for a pattern parameter.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamType {
    /// Floating point number with optional range and step.
    Float {
        min: Option<f32>,
        max: Option<f32>,
        step: Option<f32>,
    },
    /// Integer with optional range.
    Int {
        min: Option<i32>,
        max: Option<i32>,
    },
    /// Slider (range input) with min, max, and step.
    Slider {
        min: f32,
        max: f32,
        step: f32,
    },
    /// Boolean toggle.
    Bool,
    /// Selection from a list of options.
    Select {
        options: Vec<&'static str>,
    },
}

/// Specification for a pattern parameter.
#[derive(Debug, Clone, Serialize)]
pub struct ParamSpec {
    /// Parameter name (matches the key in list_params).
    pub name: &'static str,
    /// Human-readable label for the UI.
    pub label: &'static str,
    /// Input type and constraints.
    pub param_type: ParamType,
    /// Optional description/tooltip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
}

impl ParamSpec {
    /// Create a float parameter with a range (renders as slider).
    pub fn slider(name: &'static str, label: &'static str, min: f32, max: f32, step: f32) -> Self {
        Self {
            name,
            label,
            param_type: ParamType::Slider { min, max, step },
            description: None,
        }
    }

    /// Create a float parameter.
    pub fn float(name: &'static str, label: &'static str) -> Self {
        Self {
            name,
            label,
            param_type: ParamType::Float { min: None, max: None, step: None },
            description: None,
        }
    }

    /// Create an integer parameter.
    pub fn int(name: &'static str, label: &'static str, min: Option<i32>, max: Option<i32>) -> Self {
        Self {
            name,
            label,
            param_type: ParamType::Int { min, max },
            description: None,
        }
    }

    /// Create a boolean parameter.
    pub fn bool(name: &'static str, label: &'static str) -> Self {
        Self {
            name,
            label,
            param_type: ParamType::Bool,
            description: None,
        }
    }

    /// Create a select parameter.
    pub fn select(name: &'static str, label: &'static str, options: Vec<&'static str>) -> Self {
        Self {
            name,
            label,
            param_type: ParamType::Select { options },
            description: None,
        }
    }

    /// Add a description to this param spec.
    pub fn with_description(mut self, desc: &'static str) -> Self {
        self.description = Some(desc);
        self
    }
}

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

    /// Get parameter specifications with type info for UI rendering.
    fn param_specs(&self) -> Vec<ParamSpec> {
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
        "riley_check" => Some(Box::new(riley_check::RileyCheck::golden())),
        "riley_curve" => Some(Box::new(riley_curve::RileyCurve::golden())),
        "vasarely" => Some(Box::new(vasarely::Vasarely::golden())),
        "vasarely_hex" => Some(Box::new(vasarely_hex::VasarelyHex::golden())),
        "vasarely_bubbles" => Some(Box::new(vasarely_bubbles::VasarelyBubbles::golden())),
        "scintillate" => Some(Box::new(scintillate::Scintillate::golden())),
        "tunnel" => Some(Box::new(tunnel::Tunnel::golden())),
        "zebra" => Some(Box::new(zebra::Zebra::golden())),
        "flowfield" => Some(Box::new(flowfield::Flowfield::golden())),
        "erosion" => Some(Box::new(erosion::Erosion::golden())),
        "crystal" => Some(Box::new(crystal::Crystal::golden())),
        "mycelium" => Some(Box::new(mycelium::Mycelium::golden())),
        // Mascot
        "estrella" => Some(Box::new(estrella::Estrella::golden())),
        // Glitch / Digital
        "corrupt_barcode" => Some(Box::new(corrupt_barcode::CorruptBarcode::golden())),
        "databend" => Some(Box::new(databend::Databend::golden())),
        "scanline_tear" => Some(Box::new(scanline_tear::ScanlineTear::golden())),
        // Algorithmic / Mathematical
        "moire" => Some(Box::new(moire::Moire::golden())),
        "reaction_diffusion" => Some(Box::new(reaction_diffusion::ReactionDiffusion::golden())),
        "attractor" => Some(Box::new(attractor::Attractor::golden())),
        "automata" => Some(Box::new(automata::Automata::golden())),
        "voronoi" => Some(Box::new(voronoi::Voronoi::golden())),
        // Texture / Tactile
        "crosshatch" => Some(Box::new(crosshatch::Crosshatch::golden())),
        "stipple" => Some(Box::new(stipple::Stipple::golden())),
        "woodgrain" => Some(Box::new(woodgrain::Woodgrain::golden())),
        "weave" => Some(Box::new(weave::Weave::golden())),
        // Diagnostic
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
        "riley_check" => Some(Box::new(riley_check::RileyCheck::random())),
        "riley_curve" => Some(Box::new(riley_curve::RileyCurve::random())),
        "vasarely" => Some(Box::new(vasarely::Vasarely::random())),
        "vasarely_hex" => Some(Box::new(vasarely_hex::VasarelyHex::random())),
        "vasarely_bubbles" => Some(Box::new(vasarely_bubbles::VasarelyBubbles::random())),
        "scintillate" => Some(Box::new(scintillate::Scintillate::random())),
        "tunnel" => Some(Box::new(tunnel::Tunnel::random())),
        "zebra" => Some(Box::new(zebra::Zebra::random())),
        "flowfield" => Some(Box::new(flowfield::Flowfield::random())),
        "erosion" => Some(Box::new(erosion::Erosion::random())),
        "crystal" => Some(Box::new(crystal::Crystal::random())),
        "mycelium" => Some(Box::new(mycelium::Mycelium::random())),
        // Mascot
        "estrella" => Some(Box::new(estrella::Estrella::random())),
        // Glitch / Digital
        "corrupt_barcode" => Some(Box::new(corrupt_barcode::CorruptBarcode::random())),
        "databend" => Some(Box::new(databend::Databend::random())),
        "scanline_tear" => Some(Box::new(scanline_tear::ScanlineTear::random())),
        // Algorithmic / Mathematical
        "moire" => Some(Box::new(moire::Moire::random())),
        "reaction_diffusion" => Some(Box::new(reaction_diffusion::ReactionDiffusion::random())),
        "attractor" => Some(Box::new(attractor::Attractor::random())),
        "automata" => Some(Box::new(automata::Automata::random())),
        "voronoi" => Some(Box::new(voronoi::Voronoi::random())),
        // Texture / Tactile
        "crosshatch" => Some(Box::new(crosshatch::Crosshatch::random())),
        "stipple" => Some(Box::new(stipple::Stipple::random())),
        "woodgrain" => Some(Box::new(woodgrain::Woodgrain::random())),
        "weave" => Some(Box::new(weave::Weave::random())),
        // Diagnostic
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
