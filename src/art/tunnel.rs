//! # Tunnel Pattern
//!
//! Concentric rectangles creating a tunnel/vortex optical illusion.
//!
//! ## Description
//!
//! Creates nested rectangular frames that appear to recede into the distance,
//! creating a powerful sense of depth and movement. A classic op art effect
//! that appears to pull the viewer into the image.

use crate::shader::*;
use async_trait::async_trait;
use rand::Rng;
use std::fmt;

/// Parameters for the tunnel pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Frame thickness in pixels. Default: 15.0
    pub frame_thickness: f32,
    /// Gap between frames in pixels. Default: 15.0
    pub gap_thickness: f32,
    /// Perspective distortion (0=none, 1=strong). Default: 0.3
    pub perspective: f32,
    /// Center X offset as fraction (-0.5 to 0.5). Default: 0.0
    pub center_x: f32,
    /// Center Y offset as fraction (-0.5 to 0.5). Default: 0.0
    pub center_y: f32,
    /// Rotation in degrees. Default: 0.0
    pub rotation: f32,
    /// Whether to use rectangular (true) or circular (false) frames. Default: true
    pub rectangular: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            frame_thickness: 15.0,
            gap_thickness: 15.0,
            perspective: 0.3,
            center_x: 0.0,
            center_y: 0.0,
            rotation: 0.0,
            rectangular: true,
        }
    }
}

impl Params {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            frame_thickness: rng.random_range(8.0..25.0),
            gap_thickness: rng.random_range(8.0..25.0),
            perspective: rng.random_range(0.0..0.6),
            center_x: rng.random_range(-0.2..0.2),
            center_y: rng.random_range(-0.2..0.2),
            rotation: rng.random_range(0.0..45.0),
            rectangular: rng.random_bool(0.7),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "frame={:.0} gap={:.0} persp={:.2} shape={}",
            self.frame_thickness,
            self.gap_thickness,
            self.perspective,
            if self.rectangular { "rect" } else { "circle" }
        )
    }
}

/// Compute tunnel intensity at a pixel.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let wf = width as f32;
    let hf = height as f32;

    // Center point with offset
    let cx = wf * (0.5 + params.center_x);
    let cy = hf * (0.5 + params.center_y);

    // Translate to center-relative coordinates and rotate using shader primitive
    let dx = x as f32 - cx;
    let dy = y as f32 - cy;
    let (rx, ry) = rotate_deg(dx, dy, params.rotation);

    // Distance from center (using appropriate metric)
    let distance = if params.rectangular {
        // Chebyshev distance for rectangular frames
        dist_chebyshev(rx, ry, 0.0, 0.0)
    } else {
        // Euclidean distance for circular frames
        dist(rx, ry, 0.0, 0.0)
    };

    // Apply perspective distortion (frames get closer together toward center)
    let max_dist = (wf / 2.0).max(hf / 2.0);
    let norm_dist = distance / max_dist;
    let perspective_factor = 1.0 - params.perspective * (1.0 - norm_dist);

    // Effective frame + gap period
    let period = (params.frame_thickness + params.gap_thickness) * perspective_factor;

    if period < 1.0 {
        // Too compressed, just return solid
        return 0.5;
    }

    // Where are we in the frame cycle?
    let pos_in_cycle = distance % period;
    let effective_frame_thick = params.frame_thickness * perspective_factor;

    // Are we in a frame or a gap?
    if pos_in_cycle < effective_frame_thick {
        1.0 // Frame (black)
    } else {
        0.0 // Gap (white)
    }
}

/// Tunnel/vortex pattern.
#[derive(Debug, Clone)]
pub struct Tunnel {
    params: Params,
}

impl Default for Tunnel {
    fn default() -> Self {
        Self::golden()
    }
}

impl Tunnel {
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
impl super::Pattern for Tunnel {
    fn name(&self) -> &'static str {
        "tunnel"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &self.params)
    }

    fn params_description(&self) -> String {
        self.params.to_string()
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        let parse_f32 = |v: &str| {
            v.parse::<f32>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        let parse_bool = |v: &str| {
            v.parse::<bool>()
                .map_err(|e| format!("Invalid value '{}': {}", v, e))
        };
        match name {
            "frame_thickness" => self.params.frame_thickness = parse_f32(value)?,
            "gap_thickness" => self.params.gap_thickness = parse_f32(value)?,
            "perspective" => self.params.perspective = parse_f32(value)?,
            "center_x" => self.params.center_x = parse_f32(value)?,
            "center_y" => self.params.center_y = parse_f32(value)?,
            "rotation" => self.params.rotation = parse_f32(value)?,
            "rectangular" => self.params.rectangular = parse_bool(value)?,
            _ => return Err(format!("Unknown param '{}' for tunnel", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            (
                "frame_thickness",
                format!("{:.1}", self.params.frame_thickness),
            ),
            ("gap_thickness", format!("{:.1}", self.params.gap_thickness)),
            ("perspective", format!("{:.2}", self.params.perspective)),
            ("center_x", format!("{:.2}", self.params.center_x)),
            ("center_y", format!("{:.2}", self.params.center_y)),
            ("rotation", format!("{:.1}", self.params.rotation)),
            ("rectangular", self.params.rectangular.to_string()),
        ]
    }

    fn param_specs(&self) -> Vec<super::ParamSpec> {
        use super::ParamSpec;
        vec![
            ParamSpec::slider("frame_thickness", "Frame Thickness", 8.0, 25.0, 1.0)
                .with_description("Frame thickness in pixels"),
            ParamSpec::slider("gap_thickness", "Gap Thickness", 8.0, 25.0, 1.0)
                .with_description("Gap between frames in pixels"),
            ParamSpec::slider("perspective", "Perspective", 0.0, 0.6, 0.05)
                .with_description("Perspective distortion strength"),
            ParamSpec::slider("center_x", "Center X", -0.3, 0.3, 0.05)
                .with_description("Center X offset"),
            ParamSpec::slider("center_y", "Center Y", -0.3, 0.3, 0.05)
                .with_description("Center Y offset"),
            ParamSpec::slider("rotation", "Rotation", 0.0, 45.0, 5.0)
                .with_description("Rotation in degrees"),
            ParamSpec::bool("rectangular", "Rectangular")
                .with_description("Use rectangular (vs circular) frames"),
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
}
