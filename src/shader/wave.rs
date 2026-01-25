//! Wave functions for periodic patterns.

/// Sine wave normalized to [0, 1].
///
/// # Parameters
/// - `coord`: Input coordinate
/// - `freq`: Frequency (radians per unit)
/// - `phase`: Phase offset in radians
#[inline]
pub fn wave_sin(coord: f32, freq: f32, phase: f32) -> f32 {
    (coord * freq + phase).sin() * 0.5 + 0.5
}

/// Cosine wave normalized to [0, 1].
#[inline]
pub fn wave_cos(coord: f32, freq: f32, phase: f32) -> f32 {
    (coord * freq + phase).cos() * 0.5 + 0.5
}

/// Raw sine wave in [-1, 1] range.
#[inline]
pub fn wave_sin_raw(coord: f32, freq: f32, phase: f32) -> f32 {
    (coord * freq + phase).sin()
}

/// Raw cosine wave in [-1, 1] range.
#[inline]
pub fn wave_cos_raw(coord: f32, freq: f32, phase: f32) -> f32 {
    (coord * freq + phase).cos()
}

/// Radial wave (concentric circles).
///
/// Creates circular wave patterns emanating from the origin.
/// Input `r` is typically a distance from center.
#[inline]
pub fn wave_radial(r: f32, freq: f32, phase: f32) -> f32 {
    wave_sin(r, freq, phase)
}

/// Triangle wave normalized to [0, 1].
///
/// Linear ramps up and down, creating sharp peaks.
/// Period is 2π/freq radians.
#[inline]
pub fn wave_triangle(coord: f32, freq: f32, phase: f32) -> f32 {
    use std::f32::consts::TAU;
    let t = ((coord * freq + phase) / TAU).rem_euclid(1.0);
    if t < 0.5 {
        t * 2.0
    } else {
        2.0 - t * 2.0
    }
}

/// Sawtooth wave normalized to [0, 1].
///
/// Linear ramp that resets at each period.
/// Period is 2π/freq radians.
#[inline]
pub fn wave_sawtooth(coord: f32, freq: f32, phase: f32) -> f32 {
    use std::f32::consts::TAU;
    ((coord * freq + phase) / TAU).rem_euclid(1.0)
}

/// Square wave (0 or 1).
///
/// Alternates between 0 and 1 with the given frequency.
/// `duty` controls the fraction of the period that is 1 (default 0.5).
/// Period is 2π/freq radians.
#[inline]
pub fn wave_square(coord: f32, freq: f32, phase: f32, duty: f32) -> f32 {
    use std::f32::consts::TAU;
    let t = ((coord * freq + phase) / TAU).rem_euclid(1.0);
    if t < duty { 1.0 } else { 0.0 }
}

/// Modulated wave - sine wave with frequency modulation.
///
/// The frequency is modulated by another wave, creating complex patterns.
#[inline]
pub fn wave_modulated(coord: f32, base_freq: f32, mod_freq: f32, mod_depth: f32, phase: f32) -> f32 {
    let modulation = (coord * mod_freq).sin() * mod_depth;
    let effective_freq = base_freq + modulation;
    (coord * effective_freq + phase).sin() * 0.5 + 0.5
}

/// Multi-frequency wave - sum of multiple sine waves.
///
/// Combines waves at different frequencies for complex periodic patterns.
pub fn wave_multi(coord: f32, frequencies: &[f32], amplitudes: &[f32], phase: f32) -> f32 {
    let mut sum = 0.0;
    let mut total_amp = 0.0;

    for (freq, amp) in frequencies.iter().zip(amplitudes.iter()) {
        sum += amp * wave_sin_raw(coord, *freq, phase);
        total_amp += amp.abs();
    }

    if total_amp > 0.0 {
        sum / total_amp * 0.5 + 0.5
    } else {
        0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    #[test]
    fn test_wave_sin_range() {
        for i in 0..100 {
            let v = wave_sin(i as f32 * 0.1, 1.0, 0.0);
            assert!(v >= 0.0 && v <= 1.0, "wave_sin out of range: {}", v);
        }
    }

    #[test]
    fn test_wave_sin_period() {
        // With freq=1.0 (radians per unit), period is 2π
        let v0 = wave_sin(0.0, 1.0, 0.0);
        let v1 = wave_sin(TAU, 1.0, 0.0);
        assert!((v0 - v1).abs() < 1e-5, "wave_sin should have period 2π");
    }

    #[test]
    fn test_wave_triangle_range() {
        for i in 0..100 {
            let v = wave_triangle(i as f32 * 0.1, 1.0, 0.0);
            assert!(v >= 0.0 && v <= 1.0, "wave_triangle out of range: {}", v);
        }
    }

    #[test]
    fn test_wave_square() {
        // At coord=0, phase=0, we're at start of cycle -> 1.0
        assert_eq!(wave_square(0.0, 1.0, 0.0, 0.5), 1.0);
        // At coord=π (half period), we're past duty cycle -> 0.0
        assert_eq!(wave_square(std::f32::consts::PI, 1.0, 0.0, 0.5), 0.0);
    }
}
