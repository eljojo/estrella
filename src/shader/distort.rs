//! Spatial distortion functions.

use super::distance::dist;

/// Spherical bulge distortion.
///
/// Distorts coordinates as if a sphere is pushing through a flat surface.
/// Points inside the radius are displaced outward from the center.
///
/// # Parameters
/// - `x`, `y`: Input coordinates
/// - `cx`, `cy`: Center of the bulge
/// - `radius`: Radius of the affected area
/// - `strength`: How much the bulge displaces (0 = none, 1 = strong)
///
/// # Returns
/// Distorted (x, y) coordinates
pub fn bulge_spherical(
    x: f32,
    y: f32,
    cx: f32,
    cy: f32,
    radius: f32,
    strength: f32,
) -> (f32, f32) {
    let dx = x - cx;
    let dy = y - cy;
    let d = dist(x, y, cx, cy);
    let normalized = d / radius;

    if normalized >= 1.0 {
        // Outside the bulge radius - no distortion
        (x, y)
    } else {
        // Inside - apply spherical projection
        let z = (1.0 - normalized * normalized).sqrt();
        let bulge_factor = 1.0 + strength * z;
        (cx + dx * bulge_factor, cy + dy * bulge_factor)
    }
}

/// Check if a point is inside a spherical bulge region.
#[inline]
pub fn in_bulge(x: f32, y: f32, cx: f32, cy: f32, radius: f32) -> bool {
    dist(x, y, cx, cy) < radius
}

/// Exponential falloff.
///
/// Returns a value that falls off exponentially with distance.
/// Used for smooth, Gaussian-like transitions.
///
/// # Parameters
/// - `dist`: Distance from center
/// - `rate`: Falloff rate (higher = faster falloff)
///
/// # Returns
/// Value in [0, 1] where 1 is at dist=0
#[inline]
pub fn falloff_exp(dist: f32, rate: f32) -> f32 {
    (-dist * rate).exp()
}

/// Gaussian falloff.
///
/// Bell curve falloff based on distance.
#[inline]
pub fn falloff_gaussian(dist: f32, sigma: f32) -> f32 {
    (-(dist * dist) / (2.0 * sigma * sigma)).exp()
}

/// Linear falloff.
///
/// Linear decrease from 1 at dist=0 to 0 at dist=max_dist.
#[inline]
pub fn falloff_linear(dist: f32, max_dist: f32) -> f32 {
    (1.0 - dist / max_dist).max(0.0)
}

/// Wave displacement.
///
/// Calculates a displacement value from multiple sine waves.
///
/// # Parameters
/// - `coord`: The coordinate to base the wave on
/// - `amplitudes`: Amplitude for each wave
/// - `frequencies`: Frequency for each wave
/// - `phases`: Phase offset for each wave
///
/// # Returns
/// Total displacement (sum of all waves)
pub fn displace_wave(coord: f32, amplitudes: &[f32], frequencies: &[f32], phases: &[f32]) -> f32 {
    let mut total = 0.0;
    let n = amplitudes.len().min(frequencies.len()).min(phases.len());

    for i in 0..n {
        total += amplitudes[i] * (coord * frequencies[i] + phases[i]).sin();
    }

    total
}

/// Simple wave displacement with one wave.
#[inline]
pub fn displace_wave_simple(coord: f32, amplitude: f32, frequency: f32, phase: f32) -> f32 {
    amplitude * (coord * frequency + phase).sin()
}

/// Pinch/punch distortion.
///
/// Pulls coordinates toward (pinch) or pushes away from (punch) a center point.
///
/// # Parameters
/// - `strength`: Positive = pinch (toward center), negative = punch (away)
pub fn pinch(x: f32, y: f32, cx: f32, cy: f32, radius: f32, strength: f32) -> (f32, f32) {
    let dx = x - cx;
    let dy = y - cy;
    let d = dist(x, y, cx, cy);

    if d >= radius || d < 1e-10 {
        (x, y)
    } else {
        let normalized = d / radius;
        let factor = (1.0 - normalized).powf(strength.abs());
        let scale = if strength > 0.0 {
            // Pinch - move toward center
            factor
        } else {
            // Punch - move away from center
            1.0 / factor.max(0.01)
        };
        (cx + dx * scale, cy + dy * scale)
    }
}

/// Swirl distortion.
///
/// Rotates coordinates around a center, with rotation amount decreasing with distance.
pub fn swirl(x: f32, y: f32, cx: f32, cy: f32, radius: f32, angle: f32) -> (f32, f32) {
    let dx = x - cx;
    let dy = y - cy;
    let d = dist(x, y, cx, cy);

    if d >= radius {
        (x, y)
    } else {
        let normalized = d / radius;
        let twist = angle * (1.0 - normalized * normalized);
        let cos_t = twist.cos();
        let sin_t = twist.sin();
        (
            cx + dx * cos_t - dy * sin_t,
            cy + dx * sin_t + dy * cos_t,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bulge_outside() {
        // Point outside radius should not move
        let (x, y) = bulge_spherical(100.0, 100.0, 50.0, 50.0, 10.0, 1.0);
        assert!((x - 100.0).abs() < 1e-6);
        assert!((y - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_bulge_center() {
        // Point at center should not move (no direction to displace)
        let (x, y) = bulge_spherical(50.0, 50.0, 50.0, 50.0, 10.0, 1.0);
        assert!((x - 50.0).abs() < 1e-6);
        assert!((y - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_bulge_inside() {
        // Point inside should be displaced outward
        let (x, _y) = bulge_spherical(55.0, 50.0, 50.0, 50.0, 20.0, 1.0);
        assert!(x > 55.0, "should be displaced outward");
    }

    #[test]
    fn test_falloff_exp() {
        assert!((falloff_exp(0.0, 1.0) - 1.0).abs() < 1e-6);
        assert!(falloff_exp(1.0, 1.0) < 0.5);
        assert!(falloff_exp(10.0, 1.0) < 0.001);
    }

    #[test]
    fn test_displace_wave() {
        let d = displace_wave(0.0, &[10.0], &[1.0], &[0.0]);
        assert!(d.abs() < 1e-6, "sin(0) should be 0");

        let d = displace_wave(std::f32::consts::FRAC_PI_2, &[10.0], &[1.0], &[0.0]);
        assert!((d - 10.0).abs() < 1e-6, "sin(pi/2) should be 1");
    }
}
