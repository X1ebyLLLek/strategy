use crate::rng::{shuffle, Rng};

/// Improved Perlin noise (3D). The map samples it on a cylinder so the world wraps east-west.
pub struct Noise {
    p: Vec<u8>,
}

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

#[inline]
fn grad(h: u8, x: f32, y: f32, z: f32) -> f32 {
    let h = h & 15;
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

impl Noise {
    pub fn new(seed: u64) -> Self {
        let mut p: Vec<u8> = (0..=255u8).collect();
        shuffle(&mut p, &mut Rng::new(seed));
        p.extend_from_within(..);
        Noise { p }
    }

    /// Roughly in [-1, 1].
    pub fn get(&self, x: f32, y: f32, z: f32) -> f32 {
        let (fx, fy, fz) = (x.floor(), y.floor(), z.floor());
        let xi = (fx as i32 & 255) as usize;
        let yi = (fy as i32 & 255) as usize;
        let zi = (fz as i32 & 255) as usize;
        let (x, y, z) = (x - fx, y - fy, z - fz);
        let (u, v, w) = (fade(x), fade(y), fade(z));
        let p = &self.p;
        let a = p[xi] as usize + yi;
        let aa = p[a] as usize + zi;
        let ab = p[a + 1] as usize + zi;
        let b = p[xi + 1] as usize + yi;
        let ba = p[b] as usize + zi;
        let bb = p[b + 1] as usize + zi;
        lerp(
            w,
            lerp(
                v,
                lerp(u, grad(p[aa], x, y, z), grad(p[ba], x - 1.0, y, z)),
                lerp(u, grad(p[ab], x, y - 1.0, z), grad(p[bb], x - 1.0, y - 1.0, z)),
            ),
            lerp(
                v,
                lerp(u, grad(p[aa + 1], x, y, z - 1.0), grad(p[ba + 1], x - 1.0, y, z - 1.0)),
                lerp(
                    u,
                    grad(p[ab + 1], x, y - 1.0, z - 1.0),
                    grad(p[bb + 1], x - 1.0, y - 1.0, z - 1.0),
                ),
            ),
        )
    }

    /// Fractal sum: big shapes plus finer and finer detail.
    pub fn fbm(&self, x: f32, y: f32, z: f32, octaves: u32) -> f32 {
        let (mut amp, mut f, mut sum, mut norm) = (1.0, 1.0, 0.0, 0.0);
        for o in 0..octaves {
            let off = o as f32 * 17.31;
            sum += amp * self.get(x * f + off, y * f + off, z * f + off);
            norm += amp;
            amp *= 0.5;
            f *= 2.0;
        }
        sum / norm
    }

    /// Sharp crests: mountain ranges. In [0, 1].
    pub fn ridged(&self, x: f32, y: f32, z: f32, octaves: u32) -> f32 {
        let (mut amp, mut f, mut sum, mut norm) = (1.0, 1.0, 0.0, 0.0);
        for o in 0..octaves {
            let off = o as f32 * 23.17;
            let r = 1.0 - self.get(x * f + off, y * f + off, z * f + off).abs();
            sum += amp * r * r;
            norm += amp;
            amp *= 0.5;
            f *= 2.0;
        }
        sum / norm
    }
}
