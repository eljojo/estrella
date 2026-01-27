//! # Voronoi Shatter
//!
//! Cell-like structures creating a broken glass effect.
//!
//! ## Description
//!
//! Generates Voronoi diagrams where each cell is assigned to the nearest
//! seed point. The result looks like shattered glass, cellular structures,
//! or cracked earth depending on the rendering mode.

use crate::shader::*;
use rand::Rng;
use async_trait::async_trait;
use std::fmt;

/// Rendering mode for Voronoi cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderMode {
    /// Show cell edges only
    Edges,
    /// Fill cells with distance gradient
    Distance,
    /// Fill cells with unique value per cell
    Cells,
    /// Combination of cells and edges
    CellsAndEdges,
}

impl RenderMode {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "edges" => Some(Self::Edges),
            "distance" => Some(Self::Distance),
            "cells" => Some(Self::Cells),
            "cells_and_edges" | "cellsandedges" => Some(Self::CellsAndEdges),
            _ => None,
        }
    }
}

/// Parameters for Voronoi pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Number of seed points. Default: 50
    pub num_points: usize,
    /// Edge thickness. Default: 2.0
    pub edge_thickness: f32,
    /// Rendering mode. Default: CellsAndEdges
    pub mode: RenderMode,
    /// Distance metric power (2 = Euclidean, 1 = Manhattan-ish). Default: 2.0
    pub metric_power: f32,
    /// Seed for reproducibility. Default: 42
    pub seed: u32,
    /// Jitter amount for point positions. Default: 0.0
    pub jitter: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            num_points: 50,
            edge_thickness: 2.0,
            mode: RenderMode::CellsAndEdges,
            metric_power: 2.0,
            seed: 42,
            jitter: 0.0,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            num_points: rng.random_range(20..100),
            edge_thickness: rng.random_range(1.0..4.0),
            mode: match rng.random_range(0..4) {
                0 => RenderMode::Edges,
                1 => RenderMode::Distance,
                2 => RenderMode::Cells,
                _ => RenderMode::CellsAndEdges,
            },
            metric_power: rng.random_range(1.0..3.0),
            seed: rng.random(),
            jitter: rng.random_range(0.0..0.3),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode_str = match self.mode {
            RenderMode::Edges => "edges",
            RenderMode::Distance => "distance",
            RenderMode::Cells => "cells",
            RenderMode::CellsAndEdges => "cells+edges",
        };
        write!(
            f,
            "points={} edge={:.1} mode={} metric={:.1}",
            self.num_points, self.edge_thickness, mode_str, self.metric_power
        )
    }
}


/// Generate seed points.
fn generate_points(num: usize, width: usize, height: usize, seed: u32) -> Vec<(f32, f32)> {
    let mut points = Vec::with_capacity(num);
    for i in 0..num {
        let px = hash_f32(i as u32, seed) * width as f32;
        let py = hash_f32(i as u32, seed.wrapping_add(10000)) * height as f32;
        points.push((px, py));
    }
    points
}

/// Find the two nearest points and return distances.
fn find_nearest_two(
    x: f32,
    y: f32,
    points: &[(f32, f32)],
    metric_power: f32,
) -> (usize, f32, f32) {
    let mut min_dist = f32::MAX;
    let mut second_dist = f32::MAX;
    let mut min_idx = 0;

    for (i, &(px, py)) in points.iter().enumerate() {
        // Use shader's dist_minkowski for configurable metric (handles p=1, p=2, and other powers)
        let d = if metric_power == 2.0 {
            dist(x, y, px, py)
        } else if metric_power == 1.0 {
            dist_manhattan(x, y, px, py)
        } else {
            dist_minkowski(x, y, px, py, metric_power)
        };
        if d < min_dist {
            second_dist = min_dist;
            min_dist = d;
            min_idx = i;
        } else if d < second_dist {
            second_dist = d;
        }
    }

    (min_idx, min_dist, second_dist)
}

/// Cached Voronoi computation.
struct VoronoiCache {
    points: Vec<(f32, f32)>,
    params_hash: u64,
}

impl VoronoiCache {
    fn new(width: usize, height: usize, params: &Params) -> Self {
        let points = generate_points(params.num_points, width, height, params.seed);
        let params_hash = Self::hash_params(params, width, height);
        Self { points, params_hash }
    }

    fn hash_params(params: &Params, width: usize, height: usize) -> u64 {
        let mut h: u64 = params.num_points as u64;
        h = h.wrapping_mul(31).wrapping_add(params.seed as u64);
        h = h.wrapping_mul(31).wrapping_add(width as u64);
        h = h.wrapping_mul(31).wrapping_add(height as u64);
        h
    }
}

thread_local! {
    static CACHE: std::cell::RefCell<Option<VoronoiCache>> = const { std::cell::RefCell::new(None) };
}

pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let params_hash = VoronoiCache::hash_params(params, width, height);

        let need_recompute = match &*cache {
            Some(c) => c.params_hash != params_hash,
            None => true,
        };

        if need_recompute {
            *cache = Some(VoronoiCache::new(width, height, params));
        }

        let voronoi = cache.as_ref().unwrap();

        // Add jitter to query point
        let jx = if params.jitter > 0.0 {
            let jitter_val = hash_f32(
                (x as u32).wrapping_mul(997).wrapping_add(y as u32),
                params.seed.wrapping_add(5000),
            );
            (jitter_val - 0.5) * params.jitter * 10.0
        } else {
            0.0
        };
        let jy = if params.jitter > 0.0 {
            let jitter_val = hash_f32(
                (y as u32).wrapping_mul(991).wrapping_add(x as u32),
                params.seed.wrapping_add(6000),
            );
            (jitter_val - 0.5) * params.jitter * 10.0
        } else {
            0.0
        };

        let qx = x as f32 + jx;
        let qy = y as f32 + jy;

        let (nearest_idx, min_dist, second_dist) =
            find_nearest_two(qx, qy, &voronoi.points, params.metric_power);

        // Edge detection: difference between nearest and second nearest
        let edge_dist = second_dist - min_dist;
        let is_edge = edge_dist < params.edge_thickness;

        // Cell value based on point index
        let cell_value = hash_f32(nearest_idx as u32, params.seed.wrapping_add(20000));

        // Distance normalized by approximate average cell size
        let avg_cell_size = (width as f32 * height as f32 / params.num_points as f32).sqrt();
        let normalized_dist = (min_dist / avg_cell_size).min(1.0);

        match params.mode {
            RenderMode::Edges => {
                if is_edge { 1.0 } else { 0.0 }
            }
            RenderMode::Distance => {
                normalized_dist
            }
            RenderMode::Cells => {
                cell_value
            }
            RenderMode::CellsAndEdges => {
                if is_edge {
                    1.0
                } else {
                    cell_value * 0.7
                }
            }
        }
    })
}

/// Voronoi shatter pattern.
#[derive(Debug, Clone)]
pub struct Voronoi {
    params: Params,
}

impl Default for Voronoi {
    fn default() -> Self {
        Self::golden()
    }
}

impl Voronoi {
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

#[async_trait]
impl super::Pattern for Voronoi {
    fn name(&self) -> &'static str {
        "voronoi"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &self.params)
    }

    fn params_description(&self) -> String {
        self.params.to_string()
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        let parse_f32 = |v: &str| v.parse::<f32>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        let parse_usize = |v: &str| v.parse::<usize>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        let parse_u32 = |v: &str| v.parse::<u32>().map_err(|e| format!("Invalid value '{}': {}", v, e));

        match name {
            "num_points" => self.params.num_points = parse_usize(value)?,
            "edge_thickness" => self.params.edge_thickness = parse_f32(value)?,
            "mode" => {
                self.params.mode = RenderMode::from_str(value).ok_or_else(|| {
                    format!(
                        "Invalid mode '{}'. Use: edges, distance, cells, cells_and_edges",
                        value
                    )
                })?;
            }
            "metric_power" => self.params.metric_power = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            "jitter" => self.params.jitter = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for voronoi", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        let mode_str = match self.params.mode {
            RenderMode::Edges => "edges",
            RenderMode::Distance => "distance",
            RenderMode::Cells => "cells",
            RenderMode::CellsAndEdges => "cells_and_edges",
        };
        vec![
            ("num_points", self.params.num_points.to_string()),
            ("edge_thickness", format!("{:.1}", self.params.edge_thickness)),
            ("mode", mode_str.to_string()),
            ("metric_power", format!("{:.1}", self.params.metric_power)),
            ("seed", self.params.seed.to_string()),
            ("jitter", format!("{:.2}", self.params.jitter)),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::int("num_points", "Number of Points", Some(20), Some(100))
                .with_description("Number of seed points"),
            ParamSpec::slider("edge_thickness", "Edge Thickness", 1.0, 4.0, 0.5)
                .with_description("Edge thickness"),
            ParamSpec::select("mode", "Render Mode", vec!["edges", "distance", "cells", "cells_and_edges"])
                .with_description("Rendering mode"),
            ParamSpec::slider("metric_power", "Metric Power", 1.0, 3.0, 0.1)
                .with_description("Distance metric power (2=Euclidean)"),
            ParamSpec::int("seed", "Seed", Some(0), Some(999999))
                .with_description("Seed for reproducibility"),
            ParamSpec::slider("jitter", "Jitter", 0.0, 0.3, 0.01)
                .with_description("Jitter amount for point positions"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shade_range() {
        let params = Params::default();
        for y in (0..500).step_by(50) {
            for x in (0..576).step_by(50) {
                let v = shade(x, y, 576, 500, &params);
                assert!(v >= 0.0 && v <= 1.0, "value {} out of range at ({}, {})", v, x, y);
            }
        }
    }
}
