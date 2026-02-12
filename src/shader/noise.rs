//! Hash and noise functions for procedural generation.

/// Integer hash function using bit manipulation.
///
/// Produces a pseudo-random u32 from an input u32. Good for seeding
/// and deriving other random values.
#[inline]
pub fn hash(mut x: u32) -> u32 {
    x = x.wrapping_mul(0x45d9f3b);
    x ^= x >> 16;
    x = x.wrapping_mul(0x45d9f3b);
    x ^= x >> 16;
    x
}

/// Hash two integers to produce a pseudo-random u32.
#[inline]
pub fn hash2(x: u32, y: u32) -> u32 {
    hash(x ^ hash(y))
}

/// Hash three integers to produce a pseudo-random u32.
#[inline]
pub fn hash3(x: u32, y: u32, z: u32) -> u32 {
    hash(x ^ hash(y) ^ hash(z.wrapping_mul(0x9e3779b9)))
}

/// Convert a hash to a float in [0, 1].
#[inline]
pub fn hash_f32(x: u32, seed: u32) -> f32 {
    (hash(x.wrapping_add(seed)) as f32) / (u32::MAX as f32)
}

/// Convert a 2D hash to a float in [0, 1].
///
/// Uses the same mixing constants as the original pattern implementations.
#[inline]
pub fn hash2_f32(x: u32, y: u32, seed: u32) -> f32 {
    let n = hash(
        seed.wrapping_add((x).wrapping_mul(374761393))
            .wrapping_add((y).wrapping_mul(668265263)),
    );
    (n as f32) / (u32::MAX as f32)
}

/// 2D value noise with smooth interpolation.
///
/// Returns a value in [0, 1] that varies smoothly across the plane.
/// Uses bilinear interpolation with smoothstep for continuity.
pub fn noise2d(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - x.floor();
    let yf = y - y.floor();

    // Smoothstep interpolation
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);

    // Hash corners using the standard mixing constants
    let h = |ix: i32, iy: i32| -> f32 { hash2_f32(ix as u32, iy as u32, seed) };

    let n00 = h(xi, yi);
    let n10 = h(xi + 1, yi);
    let n01 = h(xi, yi + 1);
    let n11 = h(xi + 1, yi + 1);

    // Bilinear interpolation
    let nx0 = n00 * (1.0 - u) + n10 * u;
    let nx1 = n01 * (1.0 - u) + n11 * u;
    nx0 * (1.0 - v) + nx1 * v
}

/// Fractal Brownian Motion - layered noise with decreasing amplitude.
///
/// Combines multiple octaves of noise at different frequencies to create
/// natural-looking patterns with detail at multiple scales.
///
/// # Parameters
/// - `x`, `y`: Coordinates
/// - `octaves`: Number of noise layers (typically 3-8)
/// - `seed`: Random seed
/// - `lacunarity`: Frequency multiplier per octave (default: 2.0)
/// - `persistence`: Amplitude multiplier per octave (default: 0.5)
pub fn fbm(x: f32, y: f32, octaves: usize, seed: u32) -> f32 {
    fbm_params(x, y, octaves, seed, 2.0, 0.5)
}

/// Fractal Brownian Motion with configurable parameters.
pub fn fbm_params(
    x: f32,
    y: f32,
    octaves: usize,
    seed: u32,
    lacunarity: f32,
    persistence: f32,
) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut max_value = 0.0;

    for i in 0..octaves {
        // Use * 1000 offset to match original pattern implementations
        value += amplitude
            * noise2d(
                x * frequency,
                y * frequency,
                seed.wrapping_add(i as u32 * 1000),
            );
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }

    value / max_value
}

/// Ridge noise - creates sharp ridges/valleys by folding the noise.
///
/// Takes the absolute value and inverts, creating sharp peaks where
/// the noise crosses zero.
pub fn ridge(x: f32, y: f32, scale: f32, seed: u32) -> f32 {
    let n = noise2d(x * scale, y * scale, seed);
    // Map [0,1] to [-1,1], take abs, invert
    let centered = n * 2.0 - 1.0;
    1.0 - centered.abs()
}

/// Multi-octave ridge noise for terrain-like patterns.
pub fn ridge_fbm(x: f32, y: f32, octaves: usize, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_value = 0.0;

    for i in 0..octaves {
        let r = ridge(
            x * frequency,
            y * frequency,
            1.0,
            seed.wrapping_add(i as u32),
        );
        value += amplitude * r * r; // Square for sharper ridges
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / max_value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        assert_eq!(hash(42), hash(42));
        assert_ne!(hash(42), hash(43));
    }

    #[test]
    fn test_hash_f32_range() {
        for i in 0..1000 {
            let v = hash_f32(i, 0);
            assert!(v >= 0.0 && v <= 1.0);
        }
    }

    #[test]
    fn test_noise2d_range() {
        for y in 0..100 {
            for x in 0..100 {
                let v = noise2d(x as f32 * 0.1, y as f32 * 0.1, 0);
                assert!(v >= 0.0 && v <= 1.0, "noise2d out of range: {}", v);
            }
        }
    }

    #[test]
    fn test_noise2d_continuity() {
        // Values at nearby points should be similar
        let v1 = noise2d(5.0, 5.0, 0);
        let v2 = noise2d(5.01, 5.0, 0);
        assert!((v1 - v2).abs() < 0.1, "noise should be continuous");
    }

    #[test]
    fn test_fbm_range() {
        for y in 0..50 {
            for x in 0..50 {
                let v = fbm(x as f32 * 0.1, y as f32 * 0.1, 4, 0);
                assert!(v >= 0.0 && v <= 1.0, "fbm out of range: {}", v);
            }
        }
    }
}
