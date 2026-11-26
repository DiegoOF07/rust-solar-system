// noise.rs
// Enhanced procedural noise functions with Simplex noise

use raylib::math::Vector3;

/// Simple hash function for pseudo-random values
#[inline]
fn hash(p: Vector3) -> f32 {
    let mut h = p.x * 127.1 + p.y * 311.7 + p.z * 74.7;
    h = (h.sin() * 43_758.547).fract();
    h
}

/// Simplex noise gradient function
#[inline]
fn grad(hash: i32, x: f32, y: f32, z: f32) -> f32 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

/// Simplex noise - more efficient than Perlin, less directional artifacts
pub fn simplex_noise(p: Vector3) -> f32 {
    // Skewing factors
    const F3: f32 = 1.0 / 3.0;
    const G3: f32 = 1.0 / 6.0;

    // Skew input space
    let s = (p.x + p.y + p.z) * F3;
    let i = (p.x + s).floor();
    let j = (p.y + s).floor();
    let k = (p.z + s).floor();

    let t = (i + j + k) * G3;
    let x0 = p.x - (i - t);
    let y0 = p.y - (j - t);
    let z0 = p.z - (k - t);

    // Determine simplex
    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0 {
            (1.0, 0.0, 0.0, 1.0, 1.0, 0.0)
        } else if x0 >= z0 {
            (1.0, 0.0, 0.0, 1.0, 0.0, 1.0)
        } else {
            (0.0, 0.0, 1.0, 1.0, 0.0, 1.0)
        }
    } else {
        if y0 < z0 {
            (0.0, 0.0, 1.0, 0.0, 1.0, 1.0)
        } else if x0 < z0 {
            (0.0, 1.0, 0.0, 0.0, 1.0, 1.0)
        } else {
            (0.0, 1.0, 0.0, 1.0, 1.0, 0.0)
        }
    };

    // Offsets for corners
    let x1 = x0 - i1 + G3;
    let y1 = y0 - j1 + G3;
    let z1 = z0 - k1 + G3;
    let x2 = x0 - i2 + 2.0 * G3;
    let y2 = y0 - j2 + 2.0 * G3;
    let z2 = z0 - k2 + 2.0 * G3;
    let x3 = x0 - 1.0 + 3.0 * G3;
    let y3 = y0 - 1.0 + 3.0 * G3;
    let z3 = z0 - 1.0 + 3.0 * G3;

    // Hash coordinates
    let gi0 = hash(Vector3::new(i, j, k)) as i32 & 255;
    let gi1 = hash(Vector3::new(i + i1, j + j1, k + k1)) as i32 & 255;
    let gi2 = hash(Vector3::new(i + i2, j + j2, k + k2)) as i32 & 255;
    let gi3 = hash(Vector3::new(i + 1.0, j + 1.0, k + 1.0)) as i32 & 255;

    // Calculate contributions
    let mut n = 0.0;
    let t0 = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
    if t0 > 0.0 {
        let t0 = t0 * t0;
        n += t0 * t0 * grad(gi0, x0, y0, z0);
    }

    let t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
    if t1 > 0.0 {
        let t1 = t1 * t1;
        n += t1 * t1 * grad(gi1, x1, y1, z1);
    }

    let t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
    if t2 > 0.0 {
        let t2 = t2 * t2;
        n += t2 * t2 * grad(gi2, x2, y2, z2);
    }

    let t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
    if t3 > 0.0 {
        let t3 = t3 * t3;
        n += t3 * t3 * grad(gi3, x3, y3, z3);
    }

    32.0 * n
}

/// Fractal Brownian Motion (fBm) - layered noise for detail
#[inline]
pub fn fbm(p: Vector3, octaves: i32, lacunarity: f32, gain: f32) -> f32 {
    // force a small number of octaves (max 4)
    let oct = octaves.clamp(1, 4);
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;

    // Use simplex_noise but with fewer octaves
    for _ in 0..oct {
        value += amplitude
            * simplex_noise(Vector3::new(
                p.x * frequency,
                p.y * frequency,
                p.z * frequency,
            ));
        frequency *= lacunarity;
        amplitude *= gain;
    }

    // small normalization (keeps values in expected range)
    value * 0.5
}

/// Turbulence - absolute values create sharp features
#[inline]
pub fn turbulence(p: Vector3, octaves: i32) -> f32 {
    fbm(p, octaves.clamp(1, 4), 2.0, 0.5).abs()
}

/// Voronoi/cellular noise - creates cell-like patterns
#[inline]
pub fn voronoi(p: Vector3, scale: f32) -> f32 {
    // Sample a few pseudo-random points using hash at cell and three neighbors
    let scaled = Vector3::new(p.x * scale, p.y * scale, p.z * scale);
    let cell = Vector3::new(scaled.x.floor(), scaled.y.floor(), scaled.z.floor());

    let center = Vector3::new(
        cell.x + hash(cell),
        cell.y + hash(Vector3::new(cell.y, cell.x, cell.z)),
        cell.z + hash(Vector3::new(cell.z, cell.y, cell.x)),
    );

    let sample1 = Vector3::new(
        cell.x + 0.6 + hash(Vector3::new(cell.x + 1.0, cell.y, cell.z)),
        cell.y + 0.2 + hash(Vector3::new(cell.y + 1.0, cell.x, cell.z)),
        cell.z + 0.8 + hash(Vector3::new(cell.z + 1.0, cell.y, cell.x)),
    );
    let sample2 = Vector3::new(
        cell.x - 0.4 + hash(Vector3::new(cell.x - 1.0, cell.y, cell.z)),
        cell.y + 0.3 + hash(Vector3::new(cell.y - 1.0, cell.x, cell.z)),
        cell.z - 0.6 + hash(Vector3::new(cell.z - 1.0, cell.y, cell.x)),
    );

    let dx = scaled.x - center.x;
    let dy = scaled.y - center.y;
    let dz = scaled.z - center.z;
    let d0 = (dx * dx + dy * dy + dz * dz).sqrt();

    let dx1 = scaled.x - sample1.x;
    let dy1 = scaled.y - sample1.y;
    let dz1 = scaled.z - sample1.z;
    let d1 = (dx1 * dx1 + dy1 * dy1 + dz1 * dz1).sqrt();

    let dx2 = scaled.x - sample2.x;
    let dy2 = scaled.y - sample2.y;
    let dz2 = scaled.z - sample2.z;
    let d2 = (dx2 * dx2 + dy2 * dy2 + dz2 * dz2).sqrt();

    // Return min dist like a voronoi distance but cheaper
    d0.min(d1.min(d2))
}

/// Warp/domain distortion - creates swirling patterns
#[inline]
pub fn warp_noise(p: Vector3, amount: f32) -> f32 {
    let offset = Vector3::new(
        fbm(p, 3, 2.0, 0.5),
        fbm(Vector3::new(p.y, p.z, p.x), 3, 2.0, 0.5),
        fbm(Vector3::new(p.z, p.x, p.y), 3, 2.0, 0.5),
    );

    let warped = Vector3::new(
        p.x + offset.x * amount,
        p.y + offset.y * amount,
        p.z + offset.z * amount,
    );

    // use simplex_noise once on warped point (not multiple octaves)
    simplex_noise(warped) * 0.7
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples() -> impl Iterator<Item = Vector3> {
        (0..500).map(|i| {
            let t = i as f32 * 0.37;
            Vector3::new(t.sin() * 3.0, t * 0.11, (t * 1.3).cos() * 2.0)
        })
    }

    #[test]
    fn the_noise_is_deterministic() {
        for p in samples() {
            assert_eq!(fbm(p, 4, 2.0, 0.5), fbm(p, 4, 2.0, 0.5));
            assert_eq!(voronoi(p, 3.0), voronoi(p, 3.0));
        }
    }

    #[test]
    fn the_noise_stays_in_a_range_the_shaders_can_mix() {
        for p in samples() {
            let f = fbm(p, 4, 2.0, 0.5);
            assert!(f.is_finite() && f.abs() <= 1.0, "fbm({p:?}) = {f}");
            let t = turbulence(p, 3);
            assert!((0.0..=1.0).contains(&t), "turbulence({p:?}) = {t}");
            let v = voronoi(p, 3.0);
            assert!(v.is_finite() && v >= 0.0, "voronoi({p:?}) = {v}");
        }
    }
}
