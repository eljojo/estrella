//! # Cellular Automata
//!
//! Rule 30/110/184 evolving downward, computational aesthetic.
//!
//! ## Description
//!
//! Renders elementary cellular automata (1D CA) evolving over time.
//! Each row represents a generation, with the pattern evolving downward.
//! Famous rules like Rule 30 (chaotic) and Rule 110 (Turing complete)
//! create fascinating emergent patterns.

use crate::shader::*;
use async_trait::async_trait;
use rand::Rng;
use std::fmt;

/// Parameters for cellular automata pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Wolfram rule number (0-255). Default: 30
    pub rule: u8,
    /// Cell size in pixels. Default: 2
    pub cell_size: usize,
    /// Initial state type: single, random, or pattern. Default: single
    pub init: InitType,
    /// Random initial density (0-1). Default: 0.5
    pub density: f32,
    /// Seed for random init. Default: 42
    pub seed: u32,
    /// Invert output. Default: false
    pub invert: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InitType {
    /// Single cell in center
    Single,
    /// Random initial row
    Random,
    /// Alternating pattern
    Alternating,
}

impl InitType {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "single" => Some(Self::Single),
            "random" => Some(Self::Random),
            "alternating" => Some(Self::Alternating),
            _ => None,
        }
    }
}

impl Default for Params {
    fn default() -> Self {
        Self {
            rule: 30,
            cell_size: 2,
            init: InitType::Single,
            density: 0.5,
            seed: 42,
            invert: false,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        // Interesting rules: 30, 45, 54, 60, 73, 90, 105, 110, 124, 135, 150, 169, 182
        let interesting_rules = [30, 45, 54, 60, 73, 90, 105, 110, 124, 135, 150, 169, 182];
        let rule = interesting_rules[rng.random_range(0..interesting_rules.len())];

        Self {
            rule,
            cell_size: rng.random_range(1..4),
            init: match rng.random_range(0..3) {
                0 => InitType::Single,
                1 => InitType::Random,
                _ => InitType::Alternating,
            },
            density: rng.random_range(0.3..0.7),
            seed: rng.random(),
            invert: rng.random_bool(0.3),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let init_str = match self.init {
            InitType::Single => "single",
            InitType::Random => "random",
            InitType::Alternating => "alt",
        };
        write!(
            f,
            "rule={} cell={} init={}",
            self.rule, self.cell_size, init_str
        )
    }
}

/// Compute cellular automaton.
struct CellularAutomaton {
    grid: Vec<Vec<bool>>,
    width: usize,
    height: usize,
}

impl CellularAutomaton {
    fn new(width: usize, height: usize, params: &Params) -> Self {
        let mut grid = vec![vec![false; width]; height];

        // Initialize first row
        match params.init {
            InitType::Single => {
                grid[0][width / 2] = true;
            }
            InitType::Random => {
                for (x, cell) in grid[0].iter_mut().enumerate().take(width) {
                    let h = hash((x as u32).wrapping_add(params.seed));
                    *cell = (h as f32 / u32::MAX as f32) < params.density;
                }
            }
            InitType::Alternating => {
                for (x, cell) in grid[0].iter_mut().enumerate().take(width) {
                    *cell = x % 2 == 0;
                }
            }
        }

        // Evolve
        for y in 1..height {
            for x in 0..width {
                let left = if x > 0 {
                    grid[y - 1][x - 1]
                } else {
                    grid[y - 1][width - 1]
                };
                let center = grid[y - 1][x];
                let right = if x < width - 1 {
                    grid[y - 1][x + 1]
                } else {
                    grid[y - 1][0]
                };

                // Convert neighborhood to index (0-7)
                let idx = (left as u8) << 2 | (center as u8) << 1 | (right as u8);

                // Apply rule
                grid[y][x] = (params.rule >> idx) & 1 == 1;
            }
        }

        Self {
            grid,
            width,
            height,
        }
    }

    fn get(&self, x: usize, y: usize) -> bool {
        if x < self.width && y < self.height {
            self.grid[y][x]
        } else {
            false
        }
    }
}

// Thread-local cache for automaton computation.
thread_local! {
    #[allow(clippy::type_complexity)]
    static CACHE: std::cell::RefCell<Option<(usize, usize, u8, u32, CellularAutomaton)>> =
        const { std::cell::RefCell::new(None) };
}

pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    // Calculate grid dimensions based on cell size
    let grid_width = width / params.cell_size.max(1);
    let grid_height = height / params.cell_size.max(1);

    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();

        // Check if we need to recompute
        let need_recompute = match &*cache {
            Some((w, h, r, s, _)) => {
                *w != grid_width || *h != grid_height || *r != params.rule || *s != params.seed
            }
            None => true,
        };

        if need_recompute {
            let ca = CellularAutomaton::new(grid_width, grid_height, params);
            *cache = Some((grid_width, grid_height, params.rule, params.seed, ca));
        }

        let (_, _, _, _, ca) = cache.as_ref().unwrap();

        // Map pixel to cell
        let cell_x = x / params.cell_size.max(1);
        let cell_y = y / params.cell_size.max(1);

        let is_alive = ca.get(cell_x, cell_y);

        let value = if is_alive { 1.0 } else { 0.0 };

        if params.invert { 1.0 - value } else { value }
    })
}

/// Cellular automata pattern.
#[derive(Debug, Clone)]
pub struct Automata {
    params: Params,
}

impl Default for Automata {
    fn default() -> Self {
        Self::golden()
    }
}

impl Automata {
    pub fn golden() -> Self {
        Self {
            params: Params::default(),
        }
    }

    pub fn random() -> Self {
        Self {
            params: Params::random(),
        }
    }
}

#[async_trait]
impl super::Pattern for Automata {
    fn name(&self) -> &'static str {
        "automata"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &self.params)
    }

    fn params_description(&self) -> String {
        self.params.to_string()
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        let parse_u8 = |v: &str| {
            v.parse::<u8>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        let parse_usize = |v: &str| {
            v.parse::<usize>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        let parse_f32 = |v: &str| {
            v.parse::<f32>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        let parse_u32 = |v: &str| {
            v.parse::<u32>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        let parse_bool = |v: &str| {
            v.parse::<bool>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };

        match name {
            "rule" => self.params.rule = parse_u8(value)?,
            "cell_size" => self.params.cell_size = parse_usize(value)?,
            "init" => {
                self.params.init = InitType::from_str(value).ok_or_else(|| {
                    format!(
                        "Invalid init type '{}'. Use: single, random, alternating",
                        value
                    )
                })?;
            }
            "density" => self.params.density = parse_f32(value)?,
            "seed" => self.params.seed = parse_u32(value)?,
            "invert" => self.params.invert = parse_bool(value)?,
            _ => return Err(format!("Unknown param '{}' for automata", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        let init_str = match self.params.init {
            InitType::Single => "single",
            InitType::Random => "random",
            InitType::Alternating => "alternating",
        };
        vec![
            ("rule", self.params.rule.to_string()),
            ("cell_size", self.params.cell_size.to_string()),
            ("init", init_str.to_string()),
            ("density", format!("{:.2}", self.params.density)),
            ("seed", self.params.seed.to_string()),
            ("invert", self.params.invert.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::int("rule", "Rule", Some(0), Some(255))
                .with_description("Wolfram rule number (0-255)"),
            ParamSpec::int("cell_size", "Cell Size", Some(1), Some(4))
                .with_description("Cell size in pixels"),
            ParamSpec::select("init", "Init Type", vec!["single", "random", "alternating"])
                .with_description("Initial state type"),
            ParamSpec::slider("density", "Density", 0.3, 0.7, 0.05)
                .with_description("Random initial density"),
            ParamSpec::int("seed", "Seed", Some(0), Some(999999))
                .with_description("Seed for random init"),
            ParamSpec::bool("invert", "Invert").with_description("Invert output"),
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
    fn test_rule_30() {
        // Rule 30 starting from single cell should produce known pattern
        let params = Params {
            rule: 30,
            cell_size: 1,
            init: InitType::Single,
            ..Default::default()
        };
        let ca = CellularAutomaton::new(11, 5, &params);

        // Row 0: single cell in center
        assert!(ca.get(5, 0));
        assert!(!ca.get(4, 0));
        assert!(!ca.get(6, 0));

        // Row 1: should expand
        assert!(ca.get(4, 1));
        assert!(ca.get(5, 1));
        assert!(ca.get(6, 1));
    }
}
