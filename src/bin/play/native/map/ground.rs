//! The world's layers for the video card, made once per world: the land's heights as they are
//! shown (lowlands, massifs carved by the relief of real mountains and hills, basins below the sea)
//! and the sea's depths, the normals that the sun lights (twice as fine as the world), the ground's
//! colour by its kind (soft between kinds, carried under the coast so that the shore takes no
//! blue), the provinces.

use rayon::prelude::*;
use strategy::world::map::{Terrain, World, NONE};

use strategy::world::map::smoothstep;
use super::scene::Layers;

mod bake;

pub use bake::layers;

/// World pixels of height of the highest peak (the land stands up as much when the camera tilts).
pub const RELIEF: f32 = 14.0;
/// The slopes as the sun sees them: steeper than they stand, so that ranges read from afar.
const SHADE: f32 = 2.0;
/// The fine crests and gullies laid over the ranges, as the sun sees them: only a little steeper
/// than they stand (three times as steep, every small fold went from glare to black — the ranges
/// looked like crumpled tin).
const SHADE_FINE: f32 = 1.5;
/// The heights as they cast shadows: a little higher than they stand (a lower sun for the
/// shadows alone, so that a range throws its valley into shade).
const SHADOW_LIFT: f32 = 1.35;
/// Where the sun stands (towards it; the same as the card's).
const SUN: [f32; 3] = [-0.45, -0.55, 0.70];

/// The ground's colour by kind (as from above: fields, woods, rock, sand, snow).
pub fn ground(t: Terrain) -> [f32; 3] {
    match t {
        Terrain::Plains => [124.0, 142.0, 78.0],
        Terrain::Forest => [56.0, 86.0, 46.0],
        Terrain::Hills => [128.0, 124.0, 84.0],
        Terrain::Mountains => [116.0, 108.0, 98.0],
        Terrain::Desert => [210.0, 186.0, 134.0],
        Terrain::Marsh => [84.0, 100.0, 74.0],
        Terrain::Jungle => [40.0, 78.0, 36.0],
        Terrain::Tundra => [138.0, 134.0, 110.0],
        Terrain::Arctic => [236.0, 239.0, 243.0],
        Terrain::Ocean | Terrain::Lake => [124.0, 142.0, 78.0],
    }
}

/// The kinds of ground that have a texture of their own — plains, forest, hills, mountains,
/// desert, marsh, tundra, snow (jungle is a darker forest): the layer of each kind of terrain.
fn kind(t: Terrain) -> Option<usize> {
    match t {
        Terrain::Plains => Some(0),
        Terrain::Forest | Terrain::Jungle => Some(1),
        Terrain::Hills => Some(2),
        Terrain::Mountains => Some(3),
        Terrain::Desert => Some(4),
        Terrain::Marsh => Some(5),
        Terrain::Tundra => Some(6),
        Terrain::Arctic => Some(7),
        Terrain::Ocean | Terrain::Lake => None,
    }
}

/// The ground's photographs (`assets/terrain`, CC0): colour then normals, each 512×512 RGBA, the
/// kinds in the order of `kind`.
pub fn textures() -> (Vec<u8>, usize) {
    const FILES: [&[u8]; 16] = [
        include_bytes!("../../../../../assets/terrain/plains_col.jpg"),
        include_bytes!("../../../../../assets/terrain/forest_col.jpg"),
        include_bytes!("../../../../../assets/terrain/hills_col.jpg"),
        include_bytes!("../../../../../assets/terrain/mountains_col.jpg"),
        include_bytes!("../../../../../assets/terrain/desert_col.jpg"),
        include_bytes!("../../../../../assets/terrain/marsh_col.jpg"),
        include_bytes!("../../../../../assets/terrain/tundra_col.jpg"),
        include_bytes!("../../../../../assets/terrain/snow_col.jpg"),
        include_bytes!("../../../../../assets/terrain/plains_nrm.jpg"),
        include_bytes!("../../../../../assets/terrain/forest_nrm.jpg"),
        include_bytes!("../../../../../assets/terrain/hills_nrm.jpg"),
        include_bytes!("../../../../../assets/terrain/mountains_nrm.jpg"),
        include_bytes!("../../../../../assets/terrain/desert_nrm.jpg"),
        include_bytes!("../../../../../assets/terrain/marsh_nrm.jpg"),
        include_bytes!("../../../../../assets/terrain/tundra_nrm.jpg"),
        include_bytes!("../../../../../assets/terrain/snow_nrm.jpg"),
    ];
    let size = 512;
    let mut out = Vec::with_capacity(size * size * 4 * FILES.len());
    for f in FILES {
        match image::load_from_memory(f) {
            Ok(im) => {
                let im = im.resize_exact(size as u32, size as u32, image::imageops::FilterType::Triangle).to_rgba8();
                out.extend_from_slice(im.as_raw());
            }
            Err(_) => out.extend(std::iter::repeat([128u8, 128, 255, 255]).take(size * size).flatten()),
        }
    }
    (out, size)
}

/// The land seen from above (`assets/terrain/aerial_*.jpg`: USGS imagery, public domain): a
/// broad-leaved forest, a coniferous one, a marsh, two farmlands — 1024×1024 RGBA each, made to
/// tile.
pub fn aerials() -> (Vec<u8>, usize) {
    const FILES: [&[u8]; 5] = [
        include_bytes!("../../../../../assets/terrain/aerial_broad.jpg"),
        include_bytes!("../../../../../assets/terrain/aerial_conifer.jpg"),
        include_bytes!("../../../../../assets/terrain/aerial_marsh.jpg"),
        include_bytes!("../../../../../assets/terrain/aerial_farm_a.jpg"),
        include_bytes!("../../../../../assets/terrain/aerial_farm_b.jpg"),
    ];
    let size = 1024;
    let mut out = Vec::with_capacity(size * size * 4 * FILES.len());
    for f in FILES {
        match image::load_from_memory(f) {
            Ok(im) => out.extend_from_slice(im.resize_exact(size as u32, size as u32, image::imageops::FilterType::Triangle).to_rgba8().as_raw()),
            Err(_) => out.extend(std::iter::repeat([40u8, 60, 30, 255]).take(size * size).flatten()),
        }
    }
    (out, size)
}

/// Value noise on a lattice that goes round with the world (`period` cells across it).
fn vnoise(x: f32, y: f32, period: i64) -> f32 {
    let (ix, iy) = (x.floor() as i64, y.floor() as i64);
    let (fx, fy) = (x - ix as f32, y - iy as f32);
    let s = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let (u, v) = (s(fx), s(fy));
    let hash = |a: i64, b: i64| -> f32 {
        let mut h = (a.rem_euclid(period) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (b as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
        h ^= h >> 31;
        h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
        h ^= h >> 29;
        (h >> 40) as f32 / (1u64 << 24) as f32
    };
    let (a, b, c, d) = (hash(ix, iy), hash(ix + 1, iy), hash(ix, iy + 1), hash(ix + 1, iy + 1));
    (a + (b - a) * u) * (1.0 - v) + (c + (d - c) * u) * v
}

/// Fractal value noise of world pixels (0..1) at a base frequency (cells per world pixel), going
/// round with the world.
fn fbm(x: f32, y: f32, freq: f32, octaves: usize, w: usize) -> f32 {
    let (mut s, mut a, mut t, mut f) = (0.0, 0.5, 0.0, freq);
    for o in 0..octaves {
        s += a * vnoise(x * f + 31.0 * o as f32, y * f, (w as f32 * f).round().max(1.0) as i64);
        t += a;
        f *= 2.0;
        a *= 0.5;
    }
    s / t
}

/// How high the ranges stand as massifs in their middle, and the hill country (of the highest
/// peak); how deep the relief of the ranges and of the hills carves into them.
const MASSIF: f32 = 0.48;
const HILLS: f32 = 0.07;
const RANGE_RELIEF: f32 = 0.17;
const HILL_RELIEF: f32 = 0.045;

/// The relief of real land (`assets/terrain/relief_*.png`: USGS 3DEP heights, public domain — the
/// mountains of central Idaho, the Allegheny hills): the region's own slope taken off, made to
/// tile, −1..1.
struct Relief {
    n: usize,
    v: Vec<f32>,
}

impl Relief {
    fn load(bytes: &[u8]) -> Relief {
        match image::load_from_memory(bytes) {
            Ok(im) => {
                let g = im.to_luma16();
                Relief { n: g.width() as usize, v: g.as_raw().iter().map(|&x| x as f32 / 32767.5 - 1.0).collect() }
            }
            Err(_) => Relief { n: 1, v: vec![0.0] },
        }
    }

    /// At (u, v) in tiles (bilinear, going round).
    fn at(&self, u: f32, v: f32) -> f32 {
        let n = self.n;
        let (x, y) = (u * n as f32 - 0.5, v * n as f32 - 0.5);
        let (ix, iy) = (x.floor(), y.floor());
        let (fx, fy) = (x - ix, y - iy);
        let i0 = (ix as i64).rem_euclid(n as i64) as usize;
        let j0 = (iy as i64).rem_euclid(n as i64) as usize;
        let (i1, j1) = ((i0 + 1) % n, (j0 + 1) % n);
        let a = self.v[j0 * n + i0] * (1.0 - fx) + self.v[j0 * n + i1] * fx;
        let b = self.v[j1 * n + i0] * (1.0 - fx) + self.v[j1 * n + i1] * fx;
        a * (1.0 - fy) + b * fy
    }

    /// At a world point: the tile `per` times round the world, and over it the tile turned a
    /// quarter and a little larger, mixed by slow waves going round the world (so that no tile
    /// repeats to the eye); the spread kept.
    fn world(&self, x: f32, y: f32, w: usize, per: f32) -> f32 {
        let wf = w as f32;
        let per2 = (per * 0.77).round();
        let a = self.at(x * per / wf, y * per / wf);
        let b = self.at(y * per2 / wf + 0.37, -x * per2 / wf + 0.61);
        let k = std::f32::consts::TAU / wf;
        let m = 0.5 + 0.5 * (x * k * 7.0 + 1.7 * (y * k * 11.0).sin()).sin();
        (a * m + b * (1.0 - m)) / (m * m + (1.0 - m) * (1.0 - m)).sqrt()
    }
}

/// The heights twice as fine, smoothed by a cubic (x goes round, y stops at the edge).
fn twice(v: &[f32], w: usize, h: usize) -> Vec<f32> {
    let (w2, h2) = (2 * w, 2 * h);
    let mut fine = vec![0.0f32; w2 * h2];
    fine.par_chunks_mut(w2).enumerate().for_each(|(yy, row)| {
        let sy = (yy as f32 + 0.5) / 2.0 - 0.5;
        let iy = sy.floor();
        let wy = cubic(sy - iy);
        let ys: [usize; 4] = [-1i64, 0, 1, 2].map(|d| (iy as i64 + d).clamp(0, h as i64 - 1) as usize);
        for (xx, out) in row.iter_mut().enumerate() {
            let sx = (xx as f32 + 0.5) / 2.0 - 0.5;
            let ix = sx.floor();
            let wx = cubic(sx - ix);
            let xs: [usize; 4] = [-1i64, 0, 1, 2].map(|d| (ix as i64 + d).rem_euclid(w as i64) as usize);
            let mut s = 0.0;
            for j in 0..4 {
                let r = ys[j] * w;
                let mut a = 0.0;
                for i in 0..4 {
                    a += wx[i] * v[r + xs[i]];
                }
                s += wy[j] * a;
            }
            *out = s;
        }
    });
    fine
}

/// A layer of the world at a world point (bilinear; x goes round).
fn bil(v: &[f32], w: usize, h: usize, x: f32, y: f32) -> f32 {
    let (sx, sy) = (x - 0.5, (y - 0.5).clamp(0.0, h as f32 - 1.001));
    let (ix, iy) = (sx.floor(), sy.floor());
    let (fx, fy) = (sx - ix, sy - iy);
    let x0 = (ix as i64).rem_euclid(w as i64) as usize;
    let x1 = (x0 + 1) % w;
    let (y0, y1) = (iy as usize, (iy as usize + 1).min(h - 1));
    let a = v[y0 * w + x0] * (1.0 - fx) + v[y0 * w + x1] * fx;
    let b = v[y1 * w + x0] * (1.0 - fx) + v[y1 * w + x1] * fx;
    a * (1.0 - fy) + b * fy
}

/// World pixels from each pixel to the nearest one where `to` holds (a chamfer of 1 and √2 —
/// near enough the straight distance; x goes round, so the passes go twice).
pub fn reach(to: &[bool], w: usize, h: usize) -> Vec<f32> {
    let big = (w + h) as f32 * 2.0;
    let mut d: Vec<f32> = to.iter().map(|&t| if t { 0.0 } else { big }).collect();
    let s2 = std::f32::consts::SQRT_2;
    for _ in 0..2 {
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let l = y * w + (x + w - 1) % w;
                let mut v = d[i].min(d[l] + 1.0);
                if y > 0 {
                    let u = i - w;
                    v = v.min(d[u] + 1.0).min(d[(y - 1) * w + (x + w - 1) % w] + s2).min(d[(y - 1) * w + (x + 1) % w] + s2);
                }
                d[i] = v;
            }
        }
        for y in (0..h).rev() {
            for x in (0..w).rev() {
                let i = y * w + x;
                let r = y * w + (x + 1) % w;
                let mut v = d[i].min(d[r] + 1.0);
                if y + 1 < h {
                    let dn = i + w;
                    v = v.min(d[dn] + 1.0).min(d[(y + 1) * w + (x + w - 1) % w] + s2).min(d[(y + 1) * w + (x + 1) % w] + s2);
                }
                d[i] = v;
            }
        }
    }
    d
}

/// A layer of the world blurred: `passes` boxes of radius `r` across and down (near a bell).
fn soften(v: &mut [f32], w: usize, h: usize, r: usize, passes: usize) {
    let ri = r as i64;
    let n = (2 * r + 1) as f32;
    for _ in 0..passes {
        let src = v.to_vec();
        v.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            let mut a: f32 = (-ri..=ri).map(|d| src[y * w + d.rem_euclid(w as i64) as usize]).sum();
            for x in 0..w {
                row[x] = a / n;
                a += src[y * w + (x as i64 + ri + 1).rem_euclid(w as i64) as usize] - src[y * w + (x as i64 - ri).rem_euclid(w as i64) as usize];
            }
        });
        let src = v.to_vec();
        let col = |x: usize, y: i64| src[(y.clamp(0, h as i64 - 1) as usize) * w + x];
        let cols: Vec<Vec<f32>> = (0..w)
            .into_par_iter()
            .map(|x| {
                let mut c = vec![0.0f32; h];
                let mut a: f32 = (-ri..=ri).map(|d| col(x, d)).sum();
                for y in 0..h {
                    c[y] = a / n;
                    a += col(x, y as i64 + ri + 1) - col(x, y as i64 - ri);
                }
                c
            })
            .collect();
        for (x, c) in cols.iter().enumerate() {
            for y in 0..h {
                v[y * w + x] = c[y];
            }
        }
    }
}

/// The basins below the sea's level: one or two, far inland on low dry open land (deserts first;
/// where the world's own land is lowest), away from the ranges, the woods and the rivers — each a
/// long lowland lying its own way with a broken outline, gentle slopes and a broad floor below the
/// sea, salt flats in its lowest part; laid into the land's heights. How much each pixel is sunk (0..1:
/// the land there may stand below the sea), and the flat floor (0..1).
fn below_sea(world: &World, sea: &[f32], shore: &[f32], rough: &[f32], rivers: &[(Vec<[f32; 2]>, f32)], elev: &mut [f32]) -> (Vec<f32>, Vec<f32>) {
    let (w, h) = (world.w, world.h);
    let mut sink = vec![0.0f32; w * h];
    let mut floor = vec![0.0f32; w * h];
    let mut cand: Vec<(f32, usize)> = (0..w * h)
        .step_by(3)
        .filter(|&i| {
            // (in the dry middle latitudes, not by the poles)
            let lat = (((i / w) as f32 + 0.5) / h as f32 - 0.5).abs() * 2.0;
            sea[i] > 0.0 && shore[i] >= 40.0 && rough[i] < 0.03 && (0.12..0.62).contains(&lat) && matches!(world.terrain[i], Terrain::Desert | Terrain::Plains)
        })
        .map(|i| (sea[i] + 0.12 * vnoise((i % w) as f32 * 0.37, (i / w) as f32 * 0.37, i64::MAX / 4) - if world.terrain[i] == Terrain::Desert { 0.3 } else { 0.0 }, i))
        .collect();
    cand.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut taken: Vec<(f32, f32)> = Vec::new();
    for &(_, i) in &cand {
        if taken.len() >= 2 {
            break;
        }
        let (cx, cy) = ((i % w) as f32 + 0.5, (i / w) as f32 + 0.5);
        let apart = |&(x, y): &(f32, f32)| {
            let dx = (cx - x).abs();
            dx.min(w as f32 - dx).hypot(cy - y) > 300.0
        };
        if !taken.iter().all(apart) {
            continue;
        }
        // (a long lowland lying its own way, as a sunken stretch of land or a rift lies)
        let u = |k: f32| vnoise(cx * 0.11 + k, cy * 0.11 - k, i64::MAX / 4);
        let (len, wide) = (16.0 + 14.0 * u(0.0), 8.0 + 5.0 * u(3.7));
        let turn = u(7.1) * std::f32::consts::PI;
        let (ca, sa) = (turn.cos(), turn.sin());
        // (the whole of it on open dry land, away from the sea, the ranges, the woods, the rivers)
        let ok = (0..24).all(|k| {
            let a = k as f32 / 24.0 * std::f32::consts::TAU;
            let (lx, ly) = (a.cos() * len, a.sin() * wide);
            let (x, y) = (cx + lx * ca - ly * sa, cy + lx * sa + ly * ca);
            if y < 0.0 || y >= h as f32 {
                return false;
            }
            let j = (y as usize) * w + (x.rem_euclid(w as f32) as usize).min(w - 1);
            sea[j] > 0.0 && shore[j] >= 10.0 && rough[j] < 0.25 && matches!(world.terrain[j], Terrain::Desert | Terrain::Plains | Terrain::Hills)
        });
        let dry = rivers.iter().all(|(pts, _)| {
            pts.iter().all(|q| {
                let dx = (q[0] - cx).abs();
                dx.min(w as f32 - dx).hypot(q[1] - cy) > len * 0.9
            })
        });
        if !ok || !dry {
            continue;
        }
        taken.push((cx, cy));
        let deep = -(0.02 + 0.03 * u(9.3));
        let reach_px = (len * 2.1).ceil() as i64;
        for dy in -reach_px..=reach_px {
            let y = cy as i64 + dy;
            if y < 0 || y >= h as i64 {
                continue;
            }
            for dx in -reach_px..=reach_px {
                let x = (cx as i64 + dx).rem_euclid(w as i64) as usize;
                let j = y as usize * w + x;
                if sea[j] <= 0.0 {
                    continue;
                }
                let (px, py) = (cx + dx as f32, y as f32 + 0.5);
                // (its outline broken: bays and spurs, the land bent by a slow noise)
                let (wx, wy) = (px + 6.0 * (fbm(px, py, 1.0 / 14.0, 2, w) - 0.5), py + 6.0 * (fbm(px + 41.0, py, 1.0 / 14.0, 2, w) - 0.5));
                let (ox, oy) = (wx - cx, wy - cy);
                let (lx, ly) = (ox * ca + oy * sa, -ox * sa + oy * ca);
                let d = (lx / len).hypot(ly / wide) + 0.15 * (fbm(px, py, 1.0 / 5.0, 2, w) - 0.5);
                // (a gentle bowl: the slopes long — the land falls into it hardly to be seen —, the
                // floor broad and flat)
                let bowl = 1.0 - smoothstep(0.10, 1.9, d);
                if bowl <= 0.0 {
                    continue;
                }
                elev[j] = elev[j] * (1.0 - bowl) + deep * bowl;
                sink[j] = sink[j].max(bowl);
                // (the lowest of it a flat of salt, in patches)
                let patch = smoothstep(0.40, 0.60, fbm(px + 13.0, py, 1.0 / 3.0, 3, w) + 0.35 * (1.0 - d));
                floor[j] = floor[j].max((1.0 - smoothstep(0.10, 0.50, d)) * patch);
            }
        }
    }
    if std::env::var("MAPDUMP").is_ok() {
        eprintln!("впадины ниже моря: {:?}", taken);
    }
    (sink, floor)
}

/// Catmull–Rom weights for a fraction.
fn cubic(t: f32) -> [f32; 4] {
    let (t2, t3) = (t * t, t * t * t);
    [(-t3 + 2.0 * t2 - t) / 2.0, (3.0 * t3 - 5.0 * t2 + 2.0) / 2.0, (-3.0 * t3 + 4.0 * t2 + t) / 2.0, (t3 - t2) / 2.0]
}

/// What the land carries besides its own shape: the rivers (points, width in world pixels), the
/// ways (`ways::network`), the places that till the land round them (x, y, radius).
#[derive(Default)]
pub struct Extra {
    pub rivers: Vec<(Vec<[f32; 2]>, f32)>,
    pub ways: Vec<crate::world::land::Line>,
    pub farms: Vec<(f32, f32, f32)>,
    /// The built-up ground of the towns (x, y, radius): trodden and paved, not tilled.
    pub cores: Vec<(f32, f32, f32)>,
}

/// World pixels the distances to the rivers and to the ways are kept to (`Layers::detail`,
/// `Layers::ways`); the widest river's half-width.
pub const RIVER_RANGE: f32 = 3.0;
pub const WAY_RANGE: f32 = 2.0;
pub const RIVER_HW: f32 = 1.28;
