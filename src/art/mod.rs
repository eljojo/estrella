//! # Art Generation
//!
//! Reusable visual pattern generators for thermal printing.
//!
//! This module provides the core algorithms for generating visual effects.
//! Each effect is a pure function that computes intensity values, which can
//! be used by both the pattern rendering system and logo generation.
//!
//! ## Available Effects
//!
//! | Effect | Description |
//! |--------|-------------|
//! | [`ripple`] | Concentric circles with wobble interference |
//! | [`waves`] | Multi-oscillator interference pattern |
//! | [`plasma`] | Overlapping sine waves creating moire patterns |
//! | [`rings`] | Concentric rings with diagonal interference |
//! | [`topography`] | Contour lines like elevation maps |
//! | [`glitch`] | Blocky columns with scanlines |
//!
//! ## Usage
//!
//! ```ignore
//! use estrella::art;
//!
//! // Get intensity at a pixel
//! let intensity = art::ripple::shade(x, y, width, height, &art::ripple::Params::default());
//! ```

pub mod glitch;
pub mod plasma;
pub mod rings;
pub mod ripple;
pub mod topography;
pub mod waves;

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
