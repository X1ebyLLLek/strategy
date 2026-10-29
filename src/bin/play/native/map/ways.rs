//! The ways and the rivers of the land on the video card: for each texel the step to the nearest
//! line (`near`), so that the card draws thin lines sharp at any zoom. The lines themselves are
//! the land's (`crate::world::land`).

use rayon::prelude::*;

pub use crate::world::land::{MAIN, LOCAL, RAIL};

/// The way to the nearest line from every texel (`scale` texels per world pixel), written into
/// channels `ch`, `ch + 1` of an RGBA picture (`out`, `w·scale` × `h·scale`): the step east and
/// south to the line's nearest point, each 128 ± 127 for ± `range` world pixels; (255, 255) —
/// no line within `range`. Steps, not distances: the card's filter mixes the steps of
/// neighbouring texels exactly (they change evenly along a straight line), so a line thinner
/// than a texel stays whole at any zoom. A shorter way wins over what is there already. Each
/// line: its points and a byte of its own written into channel `attr` (if any) where it is the
/// nearest.
pub fn near(out: &mut [u8], w: usize, h: usize, scale: usize, ch: usize, attr: Option<usize>, lines: &[(&[[f32; 2]], u8)], range: f32) {
    near_by(out, w, h, scale, ch, attr, lines, range, 0.0)
}

/// `near`, where among lines the winner at a texel is the one whose far side is nearest: the
/// distance less `bias` times the line's byte (rivers: a brook's mouth beside a great river must
/// not hide the great river's water — the byte is the half-width, `bias` its unit).
pub fn near_by(out: &mut [u8], w: usize, h: usize, scale: usize, ch: usize, attr: Option<usize>, lines: &[(&[[f32; 2]], u8)], range: f32, bias: f32) {
    const T: usize = 64;
    let (tw, th) = (w * scale, h * scale);
    let (nx, ny) = (tw.div_ceil(T), th.div_ceil(T));
    let s = scale as f32;
    // the segments that come near each tile
    let mut tiles: Vec<Vec<([f32; 2], [f32; 2], u8)>> = vec![Vec::new(); nx * ny];
    // (each line a little longer at both ends: at a segment's end the way to it turns round the
    // end, which the card's mixing cannot follow — the end is laid past the line's true end)
    const EXT: f32 = 0.35;
    let stretch = |a: [f32; 2], b: [f32; 2]| -> [f32; 2] {
        let (dx, dy) = (a[0] - b[0], a[1] - b[1]);
        let l = (dx * dx + dy * dy).sqrt().max(1e-6);
        [a[0] + dx / l * EXT, a[1] + dy / l * EXT]
    };
    for (pts, at) in lines {
        let last = pts.len().saturating_sub(1);
        for k in 0..last {
            let (mut a, mut b) = (pts[k], pts[k + 1]);
            if k == 0 {
                a = stretch(pts[0], pts[1]);
            }
            if k + 1 == last {
                b = stretch(pts[last], pts[last - 1]);
            }
            let (x0, x1) = (a[0].min(b[0]) - range, a[0].max(b[0]) + range);
            let (y0, y1) = ((a[1].min(b[1]) - range).max(0.0), (a[1].max(b[1]) + range).min(h as f32 - 0.01));
            if y1 < y0 {
                continue;
            }
            let (ty0, ty1) = ((y0 * s) as usize / T, (y1 * s) as usize / T);
            let tx0 = ((x0 * s).floor() as i64).div_euclid(T as i64);
            let tx1 = ((x1 * s).floor() as i64).div_euclid(T as i64);
            for ty in ty0..=ty1.min(ny - 1) {
                for tx in tx0..=tx1 {
                    // (a tile past the world's edge is the tile on the other side; the segment
                    // moved there with it)
                    let wrap = tx.div_euclid(nx as i64);
                    let shift = -(wrap as f32) * w as f32;
                    let t = ty * nx + tx.rem_euclid(nx as i64) as usize;
                    tiles[t].push(([a[0] + shift, a[1]], [b[0] + shift, b[1]], *at));
                }
            }
        }
    }
    let rows: Vec<(usize, Vec<(f32, f32, f32, f32, u8)>)> = (0..ny)
        .into_par_iter()
        .map(|ty| {
            let rh = T.min(th - ty * T);
            if (0..nx).all(|tx| tiles[ty * nx + tx].is_empty()) {
                return (ty, Vec::new());
            }
            let mut band = vec![(f32::MAX, f32::MAX, 0.0f32, 0.0f32, 0u8); tw * rh];
            for tx in 0..nx {
                let segs = &tiles[ty * nx + tx];
                if segs.is_empty() {
                    continue;
                }
                for yy in 0..rh {
                    let py = ((ty * T + yy) as f32 + 0.5) / s;
                    for xx in 0..T.min(tw - tx * T) {
                        let px = ((tx * T + xx) as f32 + 0.5) / s;
                        let c = &mut band[yy * tw + tx * T + xx];
                        for (a, b, at) in segs {
                            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                            let l2 = dx * dx + dy * dy;
                            let t = if l2 > 0.0 { (((px - a[0]) * dx + (py - a[1]) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
                            let (ex, ey) = (a[0] + t * dx - px, a[1] + t * dy - py);
                            let d2 = ex * ex + ey * ey;
                            let score = if bias > 0.0 { d2.sqrt() - bias * *at as f32 } else { d2 };
                            if score < c.0 {
                                *c = (score, d2, ex, ey, *at);
                            }
                        }
                    }
                }
            }
            (ty, band)
        })
        .collect();
    for (ty, band) in rows {
        for (k, &(_, d2, ex, ey, what)) in band.iter().enumerate() {
            if d2 >= range * range {
                continue;
            }
            let (yy, x) = (k / tw, k % tw);
            let i = ((ty * T + yy) * tw + x) * 4;
            let old = step_of(out[i + ch], out[i + ch + 1], range);
            if d2 < old.0 * old.0 + old.1 * old.1 {
                out[i + ch] = (128.0 + ex / range * 127.0).round().clamp(1.0, 254.0) as u8;
                out[i + ch + 1] = (128.0 + ey / range * 127.0).round().clamp(1.0, 254.0) as u8;
                if let Some(a) = attr {
                    out[i + a] = what;
                }
            }
        }
    }
}

/// The step to the nearest line kept in two bytes by `near` (no line: a step longer than `range`).
pub fn step_of(a: u8, b: u8, range: f32) -> (f32, f32) {
    if a == 255 && b == 255 {
        return (range * 2.0, range * 2.0);
    }
    ((a as f32 - 128.0) / 127.0 * range, (b as f32 - 128.0) / 127.0 * range)
}

/// The distance to the nearest line kept by `near` at a world point (texel nearest to it).
pub fn dist_at(v: &[u8], w2: usize, h2: usize, ch: usize, x: f32, y: f32, range: f32) -> f32 {
    let (xx, yy) = (((x * 2.0).floor() as i64).rem_euclid(w2 as i64) as usize, ((y * 2.0).max(0.0) as usize).min(h2 - 1));
    let i = (yy * w2 + xx) * 4;
    let (a, b) = step_of(v[i + ch], v[i + ch + 1], range);
    a.hypot(b)
}
