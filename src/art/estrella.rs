//! # Estrella - Cute Cartoon Star
//!
//! A kawaii-style 5-pointed star with a smiling face, eyes, and highlights.
//! Perfect for thermal receipt branding.
//!
//! ## Features
//! - Rounded/bulbous 5-pointed star shape
//! - Dark outline with lighter fill
//! - Cute oval eyes with highlights
//! - Smiling mouth
//! - Blush marks on cheeks
//! - Shine highlights

use super::{clamp01, ParamSpec};
use rand::Rng;
use std::f32::consts::PI;
use std::fmt;

/// Parameters for the estrella pattern.
#[derive(Debug, Clone)]
pub struct Params {
    /// Center X as fraction of width (0.0-1.0). Default: 0.5
    pub center_x: f32,
    /// Center Y as fraction of height (0.0-1.0). Default: 0.52
    pub center_y: f32,
    /// Star size as fraction of min dimension. Default: 0.42
    pub size: f32,
    /// Outline thickness. Default: 0.04
    pub outline: f32,
    /// Number of star points. Default: 5
    pub points: u32,
    /// Inner/outer radius ratio - controls valley depth (0.3 = deep, 0.7 = shallow). Default: 0.5
    pub inner_ratio: f32,
    /// Roundness (0.0 = pointy star, 1.0 = smooth blob). Default: 0.6
    pub roundness: f32,
    /// Show face features. Default: true
    pub show_face: bool,
    /// Eye size relative to star. Default: 0.12
    pub eye_size: f32,
    /// Gamma correction. Default: 1.0
    pub gamma: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.52,
            size: 0.42,
            outline: 0.04,
            points: 5,
            inner_ratio: 0.5,
            roundness: 0.75,
            show_face: true,
            eye_size: 0.12,
            gamma: 1.0,
        }
    }
}

impl Params {
    /// Generate randomized parameters for unique prints.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        Self {
            center_x: rng.random_range(0.4..0.6),
            center_y: rng.random_range(0.45..0.55),
            size: rng.random_range(0.35..0.48),
            outline: rng.random_range(0.03..0.06),
            points: rng.random_range(4..8),
            inner_ratio: rng.random_range(0.4..0.6),
            roundness: rng.random_range(0.4..0.8),
            show_face: true,
            eye_size: rng.random_range(0.10..0.15),
            gamma: rng.random_range(0.9..1.1),
        }
    }
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "points={} inner={:.2} round={:.2} face={}",
            self.points, self.inner_ratio, self.roundness, self.show_face
        )
    }
}

/// Compute radius for a straight line between two polar points at given angle.
fn line_radius(r1: f32, a1: f32, r2: f32, a2: f32, angle: f32) -> f32 {
    // Convert vertices to cartesian
    let x1 = r1 * a1.cos();
    let y1 = r1 * a1.sin();
    let x2 = r2 * a2.cos();
    let y2 = r2 * a2.sin();

    // Ray from origin at angle
    let dx = angle.cos();
    let dy = angle.sin();

    // Find intersection of ray with line segment
    let ex = x2 - x1;
    let ey = y2 - y1;

    let det = dx * (-ey) - dy * (-ex);
    if det.abs() < 1e-10 {
        return (r1 + r2) / 2.0;
    }

    let s = (x1 * (-ey) - y1 * (-ex)) / det;
    s.max(0.001)
}

/// Compute the radius of a star with straight edges and rounded corners.
fn star_radius(angle: f32, r_outer: f32, r_inner: f32, points: u32, roundness: f32) -> f32 {
    let n = points as f32;

    // Offset so top point is at top
    let a = angle + PI / 2.0;
    let a = ((a % (2.0 * PI)) + 2.0 * PI) % (2.0 * PI);

    // Segment size: each segment is PI/n (from tip to valley or valley to tip)
    let segment = PI / n;

    // Which segment and position within it
    let segment_idx = (a / segment) as usize % (2 * points as usize);
    let seg_start = segment_idx as f32 * segment;
    let seg_end = seg_start + segment;
    let pos = (a - seg_start) / segment; // 0 to 1 within segment

    // Vertices at segment boundaries
    let (r1, r2) = if segment_idx % 2 == 0 {
        (r_outer, r_inner) // tip to valley
    } else {
        (r_inner, r_outer) // valley to tip
    };

    // Corner rounding - valleys get a wider blend to avoid sharp inward tips
    let corner_size_tip = roundness * 0.45;
    let corner_size_valley = roundness * 0.95;

    let is_tip_start = segment_idx % 2 == 0;
    let corner_size_start = if is_tip_start { corner_size_tip } else { corner_size_valley };
    let is_tip_end = !is_tip_start;
    let corner_size_end = if is_tip_end { corner_size_tip } else { corner_size_valley };

    // Check if we're in a corner zone
    if pos < corner_size_start {
        // Near start of segment (at r1 vertex)
        let t = pos / corner_size_start;

        // Tips round inward slightly, valleys round outward more
        let is_tip = is_tip_start;

        let (corner_depth, r_base) = if is_tip {
            (roundness * 0.12, r1) // tips: gentle rounding
        } else {
            (roundness * 0.09, r1) // valleys: softer rounding
        };

        let r_corner = if is_tip {
            r_base * (1.0 - corner_depth)
        } else {
            r_base * (1.0 + corner_depth)
        };

        // Smooth blend using sine for circular feel
        let blend = (t * PI * 0.5).sin();
        let r_line = line_radius(r1, seg_start, r2, seg_end, a);
        return r_corner + (r_line - r_corner) * blend;
    } else if pos > 1.0 - corner_size_end {
        // Near end of segment (at r2 vertex)
        let t = (1.0 - pos) / corner_size_end;

        let is_tip = is_tip_end;

        let (corner_depth, r_base) = if is_tip {
            (roundness * 0.12, r2)
        } else {
            (roundness * 0.09, r2)
        };

        let r_corner = if is_tip {
            r_base * (1.0 - corner_depth)
        } else {
            r_base * (1.0 + corner_depth)
        };

        let blend = (t * PI * 0.5).sin();
        let r_line = line_radius(r1, seg_start, r2, seg_end, a);
        return r_corner + (r_line - r_corner) * blend;
    }

    // Middle of segment: straight line
    line_radius(r1, seg_start, r2, seg_end, a)
}

/// Signed distance to an n-pointed star.
/// Negative = inside, positive = outside.
fn star_sdf(x: f32, y: f32, r_outer: f32, r_inner: f32, points: u32, roundness: f32) -> f32 {
    let angle = y.atan2(x);
    let dist = (x * x + y * y).sqrt();
    let target_r = star_radius(angle, r_outer, r_inner, points, roundness);
    dist - target_r
}

/// Check if point is inside an ellipse.
fn in_ellipse(x: f32, y: f32, cx: f32, cy: f32, rx: f32, ry: f32) -> bool {
    let dx = (x - cx) / rx;
    let dy = (y - cy) / ry;
    dx * dx + dy * dy <= 1.0
}

/// Compute estrella intensity at a pixel.
///
/// Returns intensity in [0.0, 1.0] where 0.0 = white, 1.0 = black.
pub fn shade(x: usize, y: usize, width: usize, height: usize, params: &Params) -> f32 {
    let w = width as f32;
    let h = height as f32;
    let min_dim = w.min(h);

    // Normalize coordinates to [-1, 1] centered
    let cx = w * params.center_x;
    let cy = h * params.center_y;
    let px = (x as f32 - cx) / (min_dim * 0.5);
    let py = (y as f32 - cy) / (min_dim * 0.5);

    let star_size = params.size;
    let r_outer = star_size;
    let r_inner = star_size * params.inner_ratio;

    // Star signed distance
    let star_d = star_sdf(px, py, r_outer, r_inner, params.points, params.roundness);

    // Outline thickness in normalized coords
    let outline_w = params.outline;

    // Outside the star entirely (including outline)
    if star_d > outline_w {
        return 0.0; // white background
    }

    // In the outline region
    if star_d > 0.0 {
        return 1.0; // black outline
    }

    // Inside the star - base fill (light gray for thermal print)
    let mut intensity = 0.15;

    // Add subtle gradient/shading for 3D effect
    // Lighter toward top-left, darker toward bottom-right
    let shade_factor = (px * 0.3 + py * 0.3 + 0.1).clamp(-0.1, 0.1);
    intensity += shade_factor;

    // Face features (only if enabled and inside star body)
    if params.show_face {
        let eye_r = params.eye_size * star_size;
        let eye_rx = eye_r * 0.65; // Horizontal radius (oval)
        let eye_ry = eye_r * 1.0; // Vertical radius

        // Eye positions (relative to star center, slightly above center)
        let eye_y = -star_size * 0.08;
        let eye_spacing = star_size * 0.28;
        let left_eye_x = -eye_spacing;
        let right_eye_x = eye_spacing;

        // Left eye
        if in_ellipse(px, py, left_eye_x, eye_y, eye_rx, eye_ry) {
            intensity = 0.95; // Dark eye
            // Eye highlight (small white dot in upper right of eye)
            let hl_x = left_eye_x + eye_rx * 0.3;
            let hl_y = eye_y - eye_ry * 0.3;
            let hl_r = eye_r * 0.25;
            if in_ellipse(px, py, hl_x, hl_y, hl_r, hl_r) {
                intensity = 0.0; // White highlight
            }
        }

        // Right eye
        if in_ellipse(px, py, right_eye_x, eye_y, eye_rx, eye_ry) {
            intensity = 0.95; // Dark eye
            // Eye highlight
            let hl_x = right_eye_x + eye_rx * 0.3;
            let hl_y = eye_y - eye_ry * 0.3;
            let hl_r = eye_r * 0.25;
            if in_ellipse(px, py, hl_x, hl_y, hl_r, hl_r) {
                intensity = 0.0; // White highlight
            }
        }

        // Mouth - simple arc/smile
        let mouth_y = star_size * 0.18;
        let mouth_w = star_size * 0.22;
        let mouth_h = star_size * 0.12;

        // Mouth as a half-ellipse (open smile)
        let mouth_dx = px / mouth_w;
        let mouth_dy = (py - mouth_y) / mouth_h;
        if mouth_dx * mouth_dx + mouth_dy * mouth_dy < 1.0 && py > mouth_y {
            intensity = 0.85; // Mouth interior

            // Tongue (small ellipse at bottom of mouth)
            let tongue_y = mouth_y + mouth_h * 0.5;
            let tongue_w = mouth_w * 0.5;
            let tongue_h = mouth_h * 0.4;
            if in_ellipse(px, py, 0.0, tongue_y, tongue_w, tongue_h) {
                intensity = 0.5; // Tongue (medium gray)
            }
        }

        // Upper lip line
        let lip_y = mouth_y;
        let lip_thickness = star_size * 0.015;
        if (py - lip_y).abs() < lip_thickness && px.abs() < mouth_w {
            // Curved lip line
            let lip_curve = mouth_h * 0.3 * (1.0 - (px / mouth_w).powi(2));
            if (py - lip_y + lip_curve).abs() < lip_thickness {
                intensity = 0.9;
            }
        }

        // Blush marks (small ovals on cheeks)
        let blush_y = star_size * 0.12;
        let blush_x = star_size * 0.38;
        let blush_rx = star_size * 0.08;
        let blush_ry = star_size * 0.04;

        // Left blush
        if in_ellipse(px, py, -blush_x, blush_y, blush_rx, blush_ry) {
            intensity = 0.35; // Subtle blush
        }
        // Right blush
        if in_ellipse(px, py, blush_x, blush_y, blush_rx, blush_ry) {
            intensity = 0.35;
        }

        // Shine highlights on upper left of star
        let shine1_x = -star_size * 0.22;
        let shine1_y = -star_size * 0.38;
        let shine1_rx = star_size * 0.04;
        let shine1_ry = star_size * 0.12;

        // Rotated ellipse for shine (roughly 30 degrees)
        let rot_angle: f32 = -0.5; // radians
        let cos_a = rot_angle.cos();
        let sin_a = rot_angle.sin();
        let rpx = (px - shine1_x) * cos_a - (py - shine1_y) * sin_a;
        let rpy = (px - shine1_x) * sin_a + (py - shine1_y) * cos_a;

        if (rpx / shine1_rx).powi(2) + (rpy / shine1_ry).powi(2) < 1.0 {
            intensity = 0.0; // White shine
        }

        // Second smaller shine
        let shine2_x = -star_size * 0.16;
        let shine2_y = -star_size * 0.22;
        let shine2_r = star_size * 0.025;
        if in_ellipse(px, py, shine2_x, shine2_y, shine2_r, shine2_r) {
            intensity = 0.0;
        }
    }

    // Apply anti-aliasing near outline edge
    if star_d > -outline_w * 0.5 {
        let edge_blend = (-star_d / (outline_w * 0.5)).clamp(0.0, 1.0);
        intensity = intensity * (1.0 - edge_blend) + 1.0 * edge_blend;
    }

    clamp01(intensity).powf(params.gamma)
}

/// Estrella pattern - cute cartoon star.
#[derive(Debug, Clone)]
pub struct Estrella {
    params: Params,
}

impl Default for Estrella {
    fn default() -> Self {
        Self::golden()
    }
}

impl Estrella {
    /// Create with golden (deterministic) params for reproducible output.
    pub fn golden() -> Self {
        Self { params: Params::default() }
    }

    /// Create with randomized params for unique prints.
    pub fn random() -> Self {
        Self { params: Params::random() }
    }
}

impl super::Pattern for Estrella {
    fn name(&self) -> &'static str {
        "estrella"
    }

    fn intensity(&self, x: usize, y: usize, width: usize, height: usize) -> f32 {
        shade(x, y, width, height, &self.params)
    }

    fn default_dimensions(&self) -> (usize, usize) {
        (576, 576) // Square for the star
    }

    fn params_description(&self) -> String {
        self.params.to_string()
    }

    fn set_param(&mut self, name: &str, value: &str) -> Result<(), String> {
        let parse_f32 = |v: &str| v.parse::<f32>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        let parse_u32 = |v: &str| v.parse::<u32>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        let parse_bool = |v: &str| v.parse::<bool>().map_err(|e| format!("Invalid value '{}': {}", v, e));
        match name {
            "center_x" => self.params.center_x = parse_f32(value)?,
            "center_y" => self.params.center_y = parse_f32(value)?,
            "size" => self.params.size = parse_f32(value)?,
            "outline" => self.params.outline = parse_f32(value)?,
            "points" => self.params.points = parse_u32(value)?.max(3),
            "inner_ratio" => self.params.inner_ratio = parse_f32(value)?,
            "roundness" => self.params.roundness = parse_f32(value)?,
            "show_face" => self.params.show_face = parse_bool(value)?,
            "eye_size" => self.params.eye_size = parse_f32(value)?,
            "gamma" => self.params.gamma = parse_f32(value)?,
            _ => return Err(format!("Unknown param '{}' for estrella", name)),
        }
        Ok(())
    }

    fn list_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("points", format!("{}", self.params.points)),
            ("inner_ratio", format!("{:.2}", self.params.inner_ratio)),
            ("roundness", format!("{:.2}", self.params.roundness)),
            ("size", format!("{:.2}", self.params.size)),
            ("outline", format!("{:.2}", self.params.outline)),
            ("show_face", format!("{}", self.params.show_face)),
            ("eye_size", format!("{:.2}", self.params.eye_size)),
            ("center_x", format!("{:.2}", self.params.center_x)),
            ("center_y", format!("{:.2}", self.params.center_y)),
            ("gamma", format!("{:.2}", self.params.gamma)),
        ]
    }

    fn param_specs(&self) -> Vec<ParamSpec> {
        vec![
            ParamSpec::int("points", "Points", Some(3), Some(12))
                .with_description("Number of star points"),
            ParamSpec::slider("inner_ratio", "Valley Depth", 0.2, 0.8, 0.02)
                .with_description("Inner/outer ratio (lower = deeper valleys)"),
            ParamSpec::slider("roundness", "Roundness", 0.0, 1.0, 0.05)
                .with_description("Shape smoothness (0 = pointy, 1 = blobby)"),
            ParamSpec::slider("size", "Size", 0.2, 0.8, 0.01)
                .with_description("Star size relative to image"),
            ParamSpec::slider("outline", "Outline", 0.01, 0.1, 0.005)
                .with_description("Outline thickness"),
            ParamSpec::bool("show_face", "Show Face")
                .with_description("Display face features"),
            ParamSpec::slider("eye_size", "Eye Size", 0.05, 0.2, 0.01)
                .with_description("Eye size relative to star"),
            ParamSpec::slider("center_x", "Center X", 0.0, 1.0, 0.01)
                .with_description("Horizontal center position"),
            ParamSpec::slider("center_y", "Center Y", 0.0, 1.0, 0.01)
                .with_description("Vertical center position"),
            ParamSpec::slider("gamma", "Gamma", 0.5, 2.0, 0.05)
                .with_description("Contrast adjustment"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shade_range() {
        let params = Params::default();
        for y in (0..576).step_by(50) {
            for x in (0..576).step_by(50) {
                let v = shade(x, y, 576, 576, &params);
                assert!((0.0..=1.0).contains(&v), "shade out of range: {}", v);
            }
        }
    }

    #[test]
    fn test_star_center_inside() {
        let params = Params::default();
        // Center should be inside the star (low-ish intensity, not outline)
        let v = shade(288, 299, 576, 576, &params);
        assert!(v < 0.5, "Center should be star fill, not outline: {}", v);
    }

    #[test]
    fn test_outside_is_white() {
        let params = Params::default();
        // Corners should be white (outside star)
        let v = shade(0, 0, 576, 576, &params);
        assert!(v < 0.01, "Corner should be white: {}", v);
    }
}
