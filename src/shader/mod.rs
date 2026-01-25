//! # Shader Primitives Library
//!
//! Reusable building blocks for pattern generation. These functions mirror
//! common operations found in fragment shaders and can be composed to create
//! complex visual effects.
//!
//! ## Categories
//!
//! - [`noise`]: Hash functions, value noise, fractal brownian motion
//! - [`distance`]: Euclidean, Chebyshev, grid distance functions
//! - [`wave`]: Sine, cosine, radial waves
//! - [`transform`]: Rotation, coordinate normalization, polar conversion
//! - [`grid`]: Cell indexing, checkerboard, hexagonal grids
//! - [`line`]: Parallel lines, anti-aliased edges, stripes
//! - [`distort`]: Spherical bulge, exponential falloff
//! - [`blend`]: Linear interpolation, smoothstep, sigmoid
//! - [`quantize`]: Scanlines, contours, band indexing
//! - [`adjust`]: Gamma, contrast, clamping
//!
//! ## Example
//!
//! ```rust
//! use estrella::shader::*;
//!
//! fn my_pattern(x: usize, y: usize, width: usize, height: usize) -> f32 {
//!     let (cx, cy) = center_coords(x as f32, y as f32, width as f32, height as f32);
//!     let r = dist(cx, cy, 0.0, 0.0);
//!     let wave = wave_sin(r, 0.05, 0.0);
//!     gamma(wave, 1.2)
//! }
//! ```

pub mod adjust;
pub mod blend;
pub mod distance;
pub mod distort;
pub mod grid;
pub mod line;
pub mod noise;
pub mod quantize;
pub mod transform;
pub mod wave;

// Re-export all primitives at the top level for convenience
pub use adjust::*;
pub use blend::*;
pub use distance::*;
pub use distort::*;
pub use grid::*;
pub use line::*;
pub use noise::*;
pub use quantize::*;
pub use transform::*;
pub use wave::*;
