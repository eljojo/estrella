//! # Strange Attractor
//!
//! Lorenz or Rössler attractor rendered as density, chaotic but structured.
//!
//! ## Description
//!
//! Renders strange attractors (Lorenz, Rössler, or Clifford) as density maps.
//! The chaotic but deterministic trajectories create intricate, flowing
//! patterns that reveal the underlying mathematical structure.

use crate::shader::*;
use async_trait::async_trait;
use rand::RngExt;
use std::fmt;
use std::sync::Mutex;

/// Attractor type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttractorType {
    Lorenz,
    Rossler,
    Clifford,
}

impl AttractorType {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "lorenz" => Some(Self::Lorenz),
            "rossler" => Some(Self::Rossler),
            "clifford" => Some(Self::Clifford),
            _ => None,
        }
    }
}

/// Parameters for strange attractor pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Attractor type. Default: Lorenz
    pub attractor: AttractorType,
    /// Number of iterations. Default: 50000
    pub iterations: usize,
    /// Zoom level. Default: 12.0
    pub zoom: f32,
    /// X offset. Default: 0.0
    pub offset_x: f32,
    /// Y offset. Default: 0.0
    pub offset_y: f32,
    /// Brightness multiplier. Default: 3.0
    pub brightness: f32,
    /// Clifford a param. Default: -1.4
    pub a: f32,
    /// Clifford b param. Default: 1.6
    pub b: f32,
    /// Clifford c param. Default: 1.0
    pub c: f32,
    /// Clifford d param. Default: 0.7
    pub d: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            attractor: AttractorType::Clifford,
            iterations: 100000,
            zoom: 80.0,
            offset_x: 0.0,
            offset_y: 0.0,
            brightness: 2.0,
            a: -1.4,
            b: 1.6,
            c: 1.0,
            d: 0.7,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let attractor = match rng.random_range(0..3) {
            0 => AttractorType::Lorenz,
            1 => AttractorType::Rossler,
            _ => AttractorType::Clifford,
        };

        // Clifford attractor parameters that produce interesting patterns
        let a = rng.random_range(-2.0..-1.0);
        let b = rng.random_range(1.2..2.0);
        let c = rng.random_range(0.5..1.5);
        let d = rng.random_range(0.5..1.2);

        Self {
            attractor,
            iterations: rng.random_range(50000..150000),
            zoom: rng.random_range(60.0..120.0),
            offset_x: rng.random_range(-20.0..20.0),
            offset_y: rng.random_range(-20.0..20.0),
            brightness: rng.random_range(1.5..4.0),
            a,
            b,
            c,
            d,
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let type_str = match self.attractor {
            AttractorType::Lorenz => "lorenz",
            AttractorType::Rossler => "rossler",
            AttractorType::Clifford => "clifford",
        };
        write!(
            f,
            "type={} iter={} zoom={:.0} bright={:.1}",
            type_str, self.iterations, self.zoom, self.brightness
        )
    }
}

/// Accumulator for density rendering.
struct DensityMap {
    data: Vec<f32>,
    width: usize,
    height: usize,
    max_value: f32,
}

impl DensityMap {
    fn new(width: usize, height: usize) -> Self {
        Self {
            data: vec![0.0; width * height],
            width,
            height,
            max_value: 0.0,
        }
    }

    fn add_point(&mut self, x: f32, y: f32) {
        let px = ((x + 1.0) * 0.5 * self.width as f32) as isize;
        let py = ((y + 1.0) * 0.5 * self.height as f32) as isize;

        if px >= 0 && px < self.width as isize && py >= 0 && py < self.height as isize {
            let idx = py as usize * self.width + px as usize;
            self.data[idx] += 1.0;
            if self.data[idx] > self.max_value {
                self.max_value = self.data[idx];
            }
        }
    }

    fn get_normalized(&self, x: usize, y: usize) -> f32 {
        if self.max_value == 0.0 {
            return 0.0;
        }
        let idx = y * self.width + x;
        self.data[idx] / self.max_value
    }
}

/// Precomputed attractor density map.
struct AttractorCache {
    density: DensityMap,
    width: usize,
    height: usize,
    params_hash: u64,
}

impl AttractorCache {
    fn compute(width: usize, height: usize, params: &Params) -> Self {
        let mut density = DensityMap::new(width, height);

        match params.attractor {
            AttractorType::Lorenz => {
                Self::compute_lorenz(&mut density, params);
            }
            AttractorType::Rossler => {
                Self::compute_rossler(&mut density, params);
            }
            AttractorType::Clifford => {
                Self::compute_clifford(&mut density, params);
            }
        }

        let params_hash = Self::hash_params(params);
        Self {
            density,
            width,
            height,
            params_hash,
        }
    }

    fn is_valid_for(&self, width: usize, height: usize, params_hash: u64) -> bool {
        self.width == width && self.height == height && self.params_hash == params_hash
    }

    fn hash_params(params: &Params) -> u64 {
        let mut h: u64 = params.attractor as u64;
        h = h.wrapping_mul(31).wrapping_add(params.iterations as u64);
        h = h
            .wrapping_mul(31)
            .wrapping_add((params.zoom * 100.0) as u64);
        h = h.wrapping_mul(31).wrapping_add((params.a * 1000.0) as u64);
        h = h.wrapping_mul(31).wrapping_add((params.b * 1000.0) as u64);
        h
    }

    fn compute_lorenz(density: &mut DensityMap, params: &Params) {
        let sigma = 10.0;
        let rho = 28.0;
        let beta = 8.0 / 3.0;
        let dt = 0.005;

        let mut x = 0.1;
        let mut y = 0.0;
        let mut z = 0.0;

        // Skip transient
        for _ in 0..1000 {
            let dx = sigma * (y - x);
            let dy = x * (rho - z) - y;
            let dz = x * y - beta * z;
            x += dx * dt;
            y += dy * dt;
            z += dz * dt;
        }

        // Collect points
        for _ in 0..params.iterations {
            let dx = sigma * (y - x);
            let dy = x * (rho - z) - y;
            let dz = x * y - beta * z;
            x += dx * dt;
            y += dy * dt;
            z += dz * dt;

            // Project to 2D (x-z plane works well for Lorenz)
            let px = (x + params.offset_x) / params.zoom;
            let py = (z - 25.0 + params.offset_y) / params.zoom;
            density.add_point(px, py);
        }
    }

    fn compute_rossler(density: &mut DensityMap, params: &Params) {
        let a = 0.2;
        let b = 0.2;
        let c = 5.7;
        let dt = 0.02;

        let mut x = 0.1;
        let mut y = 0.0;
        let mut z = 0.0;

        // Skip transient
        for _ in 0..1000 {
            let dx = -y - z;
            let dy = x + a * y;
            let dz = b + z * (x - c);
            x += dx * dt;
            y += dy * dt;
            z += dz * dt;
        }

        // Collect points
        for _ in 0..params.iterations {
            let dx = -y - z;
            let dy = x + a * y;
            let dz = b + z * (x - c);
            x += dx * dt;
            y += dy * dt;
            z += dz * dt;

            let px = (x + params.offset_x) / params.zoom;
            let py = (y + params.offset_y) / params.zoom;
            density.add_point(px, py);
        }
    }

    fn compute_clifford(density: &mut DensityMap, params: &Params) {
        let mut x = 0.1;
        let mut y = 0.1;

        for _ in 0..params.iterations {
            let nx = (params.a * y).sin() + params.c * (params.a * x).cos();
            let ny = (params.b * x).sin() + params.d * (params.b * y).cos();
            x = nx;
            y = ny;

            let px = (x + params.offset_x / params.zoom) / 2.5;
            let py = (y + params.offset_y / params.zoom) / 2.5;
            density.add_point(px, py);
        }
    }
}

/// Strange attractor pattern with per-instance caching.
pub struct Attractor {
    params: Params,
    /// Per-instance cache to avoid thrashing when multiple attractors render at different sizes
    cache: Mutex<Option<AttractorCache>>,
}

impl std::fmt::Debug for Attractor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attractor")
            .field("params", &self.params)
            .finish()
    }
}

impl Clone for Attractor {
    fn clone(&self) -> Self {
        Self {
            params: self.params.clone(),
            cache: Mutex::new(None), // Don't clone cache, let it recompute
        }
    }
}

impl Default for Attractor {
    fn default() -> Self {
        Self::golden()
    }
}

impl Attractor {
    pub fn golden() -> Self {
        Self {
            params: Params::default(),
            cache: Mutex::new(None),
        }
    }

    pub fn random() -> Self {
        Self {
            params: Params::random(),
            cache: Mutex::new(None),
        }
    }

    /// Get cached density or compute it
    fn get_density(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        let params_hash = AttractorCache::hash_params(&self.params);

        let mut cache = self.cache.lock().unwrap();

        // Check if cache is valid
        let need_recompute = match &*cache {
            Some(c) => !c.is_valid_for(width, height, params_hash),
            None => true,
        };

        if need_recompute {
            *cache = Some(AttractorCache::compute(width, height, &self.params));
        }

        let attractor_cache = cache.as_ref().unwrap();
        attractor_cache.density.get_normalized(x, y)
    }
}

#[async_trait]
impl super::Pattern for Attractor {
    fn name(&self) -> &'static str {
        "attractor"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        let density = self.get_density(x, y, width, height);

        // Apply brightness with log scale for better contrast
        let log_density = if density > 0.0 {
            (1.0 + density * 100.0).ln() / (101.0_f32).ln()
        } else {
            0.0
        };

        clamp01(log_density * self.params.brightness)
    }

    fn params_description(&self) -> String {
        self.params.to_string()
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        let parse_f32 = |v: &str| {
            v.parse::<f32>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        let parse_usize = |v: &str| {
            v.parse::<usize>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        match name {
            "attractor" => {
                self.params.attractor = AttractorType::from_str(value).ok_or_else(|| {
                    format!(
                        "Invalid attractor type '{}'. Use: lorenz, rossler, clifford",
                        value
                    )
                })?;
            }
            "iterations" => self.params.iterations = parse_usize(value)?,
            "zoom" => self.params.zoom = parse_f32(value)?,
            "offset_x" => self.params.offset_x = parse_f32(value)?,
            "offset_y" => self.params.offset_y = parse_f32(value)?,
            "brightness" => self.params.brightness = parse_f32(value)?,
            "a" => self.params.a = parse_f32(value)?,
            "b" => self.params.b = parse_f32(value)?,
            "c" => self.params.c = parse_f32(value)?,
            "d" => self.params.d = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for attractor", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        let type_str = match self.params.attractor {
            AttractorType::Lorenz => "lorenz",
            AttractorType::Rossler => "rossler",
            AttractorType::Clifford => "clifford",
        };
        vec![
            ("attractor", type_str.to_string()),
            ("iterations", self.params.iterations.to_string()),
            ("zoom", format!("{:.0}", self.params.zoom)),
            ("offset_x", format!("{:.1}", self.params.offset_x)),
            ("offset_y", format!("{:.1}", self.params.offset_y)),
            ("brightness", format!("{:.1}", self.params.brightness)),
            ("a", format!("{:.2}", self.params.a)),
            ("b", format!("{:.2}", self.params.b)),
            ("c", format!("{:.2}", self.params.c)),
            ("d", format!("{:.2}", self.params.d)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::select(
                "attractor",
                "Attractor Type",
                vec!["lorenz", "rossler", "clifford"],
            )
            .with_description("Attractor type"),
            ParamSpec::int("iterations", "Iterations", Some(50000), Some(150000))
                .with_description("Number of iterations"),
            ParamSpec::slider("zoom", "Zoom", 60.0, 120.0, 5.0).with_description("Zoom level"),
            ParamSpec::slider("offset_x", "Offset X", -20.0, 20.0, 1.0)
                .with_description("X offset"),
            ParamSpec::slider("offset_y", "Offset Y", -20.0, 20.0, 1.0)
                .with_description("Y offset"),
            ParamSpec::slider("brightness", "Brightness", 1.5, 4.0, 0.1)
                .with_description("Brightness multiplier"),
            ParamSpec::slider("a", "A Parameter", -2.0, -1.0, 0.1)
                .with_description("Clifford a parameter"),
            ParamSpec::slider("b", "B Parameter", 1.2, 2.0, 0.1)
                .with_description("Clifford b parameter"),
            ParamSpec::slider("c", "C Parameter", 0.5, 1.5, 0.1)
                .with_description("Clifford c parameter"),
            ParamSpec::slider("d", "D Parameter", 0.5, 1.2, 0.1)
                .with_description("Clifford d parameter"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art::Pattern;

    #[test]
    fn test_intensity_range() {
        let attractor = Attractor::golden();
        for y in (0..500).step_by(50) {
            for x in (0..576).step_by(50) {
                let v = attractor.intensity(x, y, 576, 500);
                assert!(
                    v >= 0.0 && v <= 1.0,
                    "value {} out of range at ({}, {})",
                    v,
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn test_multiple_instances_different_sizes() {
        // This tests the fix for cache thrashing - two attractors at different sizes
        let a1 = Attractor::golden();
        let a2 = Attractor::golden();

        // Render first pixel of each at different sizes
        let v1 = a1.intensity(0, 0, 300, 300);
        let v2 = a2.intensity(0, 0, 576, 500);

        // Both should return valid values
        assert!(v1 >= 0.0 && v1 <= 1.0);
        assert!(v2 >= 0.0 && v2 <= 1.0);

        // Render again - should use cached values, not thrash
        let v1b = a1.intensity(1, 1, 300, 300);
        let v2b = a2.intensity(1, 1, 576, 500);

        assert!(v1b >= 0.0 && v1b <= 1.0);
        assert!(v2b >= 0.0 && v2b <= 1.0);
    }
}
