//! How the towns and the villages are laid out on the land, close up: their streets — the roads
//! that pass through them and streets of their own going out from the middle (a ring street in
//! the large towns) — a square in the middle with its church, the houses along the streets, their
//! fronts to the street, close-packed town houses in the middle, cottages further out, barns at
//! the edge; the era's own buildings beyond (factories by the roads, windmills in the open).
//! Only the look: nothing here is the game's.

use crate::world::land::{Line, LOCAL, RAIL};
use strategy::rng::mix64;
use super::objects::{self, Towns};

fn unit(h: u64) -> f32 {
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// The distance from a point to a polyline.
fn to_line(p: [f32; 2], pts: &[[f32; 2]]) -> f32 {
    let mut best = f32::MAX;
    for s in pts.windows(2) {
        let (a, b) = (s[0], s[1]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l2 = (dx * dx + dy * dy).max(1e-9);
        let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0);
        best = best.min((p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy));
    }
    if pts.len() == 1 {
        best = (p[0] - pts[0][0]).hypot(p[1] - pts[0][1]);
    }
    best
}

/// A town laid out: its buildings, the streets of its own (drawn on the land as village roads),
/// and the built-up ground round its middle (x, y, radius — trodden and paved).
#[derive(Default)]
pub struct Layout {
    pub built: Towns,
    pub streets: Vec<Line>,
    pub cores: Vec<(f32, f32, f32)>,
}

/// How large the buildings stand against the land (world pixels a model unit).
const BIG: f32 = 1.3;

/// A building's footprint: its half-length along the street, half-depth, and the gap it keeps.
fn footprint(kind: usize) -> (f32, f32) {
    if let Some(f) = objects::baked_footprint(kind) {
        return f;
    }
    match kind {
        objects::COTTAGE => (0.24, 0.17),
        objects::TOWNHOUSE => (0.21, 0.19),
        objects::BARN => (0.36, 0.17),
        objects::HIPHOUSE => (0.26, 0.22),
        objects::CHURCH => (0.46, 0.18),
        objects::BLOCK => (0.34, 0.22),
        objects::FACTORY => (0.50, 0.44),
        _ => (0.22, 0.22),
    }
}

/// The towns' layout: `towns` (x, y, size 0 — a village .. 3), the ways of the land, the rivers
/// (points, width), whether a point is dry land, the era (0 — the staff's age, 1 — the wastes,
/// 2 — the age of muskets).
pub fn lay(towns: &[(f32, f32, u8)], ways: &[Line], rivers: &[(Vec<[f32; 2]>, f32)], dry: &dyn Fn(f32, f32) -> bool, era: f32) -> Layout {
    let mut out = Layout::default();
    // (the ways and rivers near a town: their pieces within reach of it)
    let near_lines = |cx: f32, cy: f32, reach: f32, pts: &[[f32; 2]]| -> Vec<Vec<[f32; 2]>> {
        let mut runs: Vec<Vec<[f32; 2]>> = Vec::new();
        let mut cur: Vec<[f32; 2]> = Vec::new();
        for s in pts.windows(2) {
            let near = to_line([cx, cy], s) < reach;
            if near {
                if cur.is_empty() {
                    cur.push(s[0]);
                }
                cur.push(s[1]);
            } else if !cur.is_empty() {
                runs.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            runs.push(cur);
        }
        runs
    };
    for (t, &(cx, cy, size)) in towns.iter().enumerate() {
        let mut h = mix64(t as u64 ^ 0x70E5);
        let mut rnd = || {
            h = mix64(h);
            unit(h)
        };
        let radius = match size {
            3 => 5.4,
            2 => 3.7,
            1 => 2.4,
            _ => 1.35,
        };
        let square = match size {
            3 => 1.05,
            2 => 0.80,
            1 => 0.55,
            _ => 0.0,
        };
        // the ways through the town, and the rivers it must keep off
        let mut roads: Vec<Vec<[f32; 2]>> = Vec::new();
        for l in ways {
            if l.kind == RAIL {
                continue;
            }
            roads.extend(near_lines(cx, cy, radius * 1.15, &l.pts));
        }
        let mut rails: Vec<Vec<[f32; 2]>> = Vec::new();
        for l in ways.iter().filter(|l| l.kind == RAIL) {
            rails.extend(near_lines(cx, cy, radius * 1.4, &l.pts));
        }
        let waters: Vec<(Vec<[f32; 2]>, f32)> = rivers.iter().flat_map(|(p, wd)| near_lines(cx, cy, radius * 1.5, p).into_iter().map(move |r| (r, *wd))).collect();
        let wet = |x: f32, y: f32| !dry(x, y) || waters.iter().any(|(r, wd)| to_line([x, y], r) < wd / 2.0 + 0.32);
        // the town's own streets: out from the square in the directions the roads leave free;
        // a ring street round the middle of a large town
        let dirs: Vec<f32> = roads
            .iter()
            .flat_map(|r| {
                let (a, b) = (r[0], r[r.len() - 1]);
                [(a[1] - cy).atan2(a[0] - cx), (b[1] - cy).atan2(b[0] - cx)]
            })
            .collect();
        let own = match size {
            3 => 6,
            2 => 4,
            1 => 3,
            _ => (roads.is_empty()) as usize,
        };
        let mut streets: Vec<Vec<[f32; 2]>> = Vec::new();
        let start = rnd() * std::f32::consts::TAU;
        let mut made = 0;
        for k in 0..own * 3 {
            if made >= own {
                break;
            }
            let a = start + k as f32 * std::f32::consts::TAU / (own as f32 * 2.0) + 0.35 * (rnd() - 0.5);
            let free = |a: f32| dirs.iter().all(|&d| {
                let dd = (a - d).rem_euclid(std::f32::consts::TAU);
                dd.min(std::f32::consts::TAU - dd) > 0.55
            });
            let taken = streets.iter().any(|s| {
                let e = s[s.len() - 1];
                let d = (e[1] - cy).atan2(e[0] - cx);
                let dd = (a - d).rem_euclid(std::f32::consts::TAU);
                dd.min(std::f32::consts::TAU - dd) < 0.6
            });
            if !free(a) || taken {
                continue;
            }
            let len = radius * (0.80 + 0.25 * rnd());
            let bend = 0.25 * (rnd() - 0.5);
            let mut pts = Vec::new();
            let steps = 4;
            for s in 0..=steps {
                let f = s as f32 / steps as f32;
                let r = square * 0.9 + (len - square * 0.9) * f;
                let aa = a + bend * f * f;
                let p = [cx + r * aa.cos(), cy + r * aa.sin()];
                if wet(p[0], p[1]) {
                    break;
                }
                pts.push(p);
            }
            if pts.len() >= 2 {
                streets.push(pts);
                made += 1;
            }
        }
        if size >= 2 {
            let rr = radius * if size == 3 { 0.52 } else { 0.58 };
            let mut ring: Vec<[f32; 2]> = Vec::new();
            let n = 36;
            let wob = rnd() * 6.0;
            for s in 0..=n {
                let a = s as f32 / n as f32 * std::f32::consts::TAU;
                let r = rr * (1.0 + 0.10 * (a * 3.0 + wob).sin());
                let p = [cx + r * a.cos(), cy + r * a.sin()];
                if wet(p[0], p[1]) {
                    if ring.len() >= 2 {
                        streets.push(std::mem::take(&mut ring));
                    }
                    ring.clear();
                    continue;
                }
                ring.push(p);
            }
            if ring.len() >= 2 {
                streets.push(ring);
            }
        }
        // (the square: a street round it, the houses about it fronting the square)
        if size >= 1 {
            let mut sq = Vec::new();
            for s in 0..=16 {
                let a = s as f32 / 16.0 * std::f32::consts::TAU;
                sq.push([cx + square * 0.75 * a.cos(), cy + square * 0.75 * a.sin()]);
            }
            streets.push(sq);
        }
        for s in &streets {
            out.streets.push(Line { pts: s.clone(), kind: LOCAL });
        }
        out.cores.push((cx, cy, radius * if size == 0 { 0.8 } else { 0.62 }));
        // the buildings: along every street and road, both sides, fronts to it
        let axes: Vec<&Vec<[f32; 2]>> = roads.iter().chain(streets.iter()).collect();
        let mut placed: Vec<([f32; 2], f32)> = Vec::new();
        let fits = |placed: &Vec<([f32; 2], f32)>, p: [f32; 2], r: f32| -> bool {
            if wet(p[0], p[1]) || (p[0] - cx).hypot(p[1] - cy) < square {
                return false;
            }
            if placed.iter().any(|(q, rq)| (q[0] - p[0]).hypot(q[1] - p[1]) < (r + rq) * 0.92) {
                return false;
            }
            // (off every street and road, rails too)
            axes.iter().all(|a| to_line(p, a) > r * 0.78 + 0.10) && rails.iter().all(|a| to_line(p, a) > r + 0.25)
        };
        // (the church on the square, its tower to the square; in the large towns a town hall
        // across it)
        if size >= 1 {
            for (k, kind) in [objects::CHURCH, objects::HIPHOUSE].into_iter().enumerate().take(if size >= 2 { 2 } else { 1 }) {
                for _ in 0..12 {
                    let a = rnd() * std::f32::consts::TAU + k as f32 * std::f32::consts::PI;
                    let vk = objects::baked_variant(kind, era, rnd());
                    let (hx, hy) = footprint(vk);
                    let d = square + hy + 0.08;
                    let p = [cx + d * a.cos(), cy + d * a.sin()];
                    let size_k = BIG * if kind == objects::CHURCH { 1.15 } else { 1.35 };
                    let r = hx.max(hy) * size_k;
                    if fits(&placed, p, r * 0.8) {
                        placed.push((p, r * 0.8));
                        // (a church along the radius, its tower — the +x end — to the square; a
                        // hall's front — its −y side — to the square)
                        let face = if kind == objects::CHURCH { a + std::f32::consts::PI } else { a - std::f32::consts::FRAC_PI_2 };
                        out.built.builds[vk].push([p[0], p[1], size_k, face / std::f32::consts::TAU]);
                        break;
                    }
                }
            }
        }
        for (ai, axis) in axes.iter().enumerate() {
            // (walk the axis: a building each step on either side while the town lasts)
            // (the way a piece of the axis runs; a house follows the street's own bend — between
            // the pieces round a corner of the line, as much of each as it is near)
            let piece = |i: usize| -> Option<[f32; 2]> {
                let (a, b) = (*axis.get(i)?, *axis.get(i + 1)?);
                let l = (b[0] - a[0]).hypot(b[1] - a[1]);
                (l >= 1e-4).then(|| [(b[0] - a[0]) / l, (b[1] - a[1]) / l])
            };
            for i in 0..axis.len().saturating_sub(1) {
                let (a, b) = (axis[i], axis[i + 1]);
                let len = (b[0] - a[0]).hypot(b[1] - a[1]);
                let Some(dir) = piece(i) else { continue };
                let prev = i.checked_sub(1).and_then(|j| piece(j)).unwrap_or(dir);
                let next = piece(i + 1).unwrap_or(dir);
                let mut t = 0.0f32;
                while t < len {
                    let p = [a[0] + dir[0] * t, a[1] + dir[1] * t];
                    let wgt = t / len;
                    let tan = {
                        let v = [(dir[0] + prev[0]) * (1.0 - wgt) + (dir[0] + next[0]) * wgt, (dir[1] + prev[1]) * (1.0 - wgt) + (dir[1] + next[1]) * wgt];
                        let l = v[0].hypot(v[1]).max(1e-6);
                        [v[0] / l, v[1] / l]
                    };
                    let nrm = [-tan[1], tan[0]];
                    let d = (p[0] - cx).hypot(p[1] - cy) / radius;
                    let core = d < 0.42 && size >= 2;
                    let step = BIG * if core { 0.46 } else if d < 0.75 { 0.54 } else { 0.72 };
                    t += step * (0.9 + 0.25 * rnd());
                    if d > 1.0 {
                        continue;
                    }
                    for side in [-1.0f32, 1.0] {
                        // (fewer to the edge; gardens between)
                        if rnd() > (1.0 - d.powf(2.2)) * if core { 1.0 } else { 0.95 } {
                            continue;
                        }
                        let kind = if era < 1.5 && core && size == 3 && rnd() < 0.35 {
                            objects::BLOCK
                        } else if core || (size >= 2 && d < 0.6 && rnd() < 0.45) {
                            objects::TOWNHOUSE
                        } else if d > 0.72 && rnd() < 0.35 {
                            objects::BARN
                        } else if rnd() < 0.3 {
                            objects::HIPHOUSE
                        } else {
                            objects::COTTAGE
                        };
                        let sz = BIG * match kind {
                            objects::BLOCK => 1.1 + 0.15 * rnd(),
                            objects::TOWNHOUSE => 1.05 + 0.2 * rnd(),
                            _ => 0.95 + 0.3 * rnd(),
                        };
                        // (32nd session: the baked houses stand where the made ones were meant)
                        let kind = objects::baked_variant(kind, era, rnd());
                        let (hx, hy) = footprint(kind);
                        let off = 0.13 + hy * sz + 0.04;
                        let rows = 1 + (core || (size >= 2 && d < 0.6)) as usize;
                        for row in 0..rows {
                            let o = off + row as f32 * (2.0 * hy * sz + 0.16);
                            let q = [p[0] + nrm[0] * side * o, p[1] + nrm[1] * side * o];
                            let r = hx.max(hy) * sz * 0.86;
                            if !fits(&placed, q, r) {
                                continue;
                            }
                            // (a corner plot belongs to the street it is nearer: a house fronts
                            // the street it is built along, not the one round the corner)
                            let own = to_line(q, axis);
                            if axes.iter().enumerate().any(|(j, a)| j != ai && to_line(q, a) < own + 0.05) {
                                continue;
                            }
                            placed.push((q, r));
                            // (the front — the model's −y side, with the door — to the street, the
                            // long side along it; the spoiled control: turned any way, as the old
                            // scattered houses were)
                            let mut ang = tan[1].atan2(tan[0]) + if side < 0.0 { std::f32::consts::PI } else { 0.0 };
                            if crate::native::spoiled() {
                                ang = rnd() * std::f32::consts::TAU;
                            }
                            out.built.builds[kind].push([q[0], q[1], sz, ang / std::f32::consts::TAU]);
                        }
                    }
                }
            }
        }
        // the era's buildings beyond the houses: factories by the roads and the railways (the
        // staff's age), windmills in the open (the age of muskets)
        let (want, kind) = if era < 0.5 {
            (match size {
                3 => 4,
                2 => 2,
                1 => (rnd() < 0.4) as usize,
                _ => 0,
            }, objects::FACTORY)
        } else if era > 1.5 {
            (match size {
                3 => 3,
                2 => 2,
                1 => 1,
                _ => (rnd() < 0.3) as usize,
            }, objects::MILL)
        } else {
            (0, objects::FACTORY)
        };
        let mut made = 0;
        for _ in 0..want * 30 {
            if made >= want {
                break;
            }
            let r = radius * (1.0 + 0.45 * rnd());
            let a = rnd() * std::f32::consts::TAU;
            let p = [cx + r * a.cos(), cy + r * a.sin()];
            let kind = objects::baked_variant(kind, era, rnd());
            let (hx, hy) = footprint(kind);
            let sz = 1.1 + 0.3 * rnd();
            let rr = hx.max(hy) * sz;
            if !fits(&placed, p, rr) {
                continue;
            }
            // (a factory by a way, along it)
            let mut ang = rnd() * std::f32::consts::TAU;
            if kind == objects::FACTORY {
                let by = roads.iter().chain(rails.iter()).map(|l| (to_line(p, l), l)).min_by(|x, y| x.0.total_cmp(&y.0));
                match by {
                    Some((d, l)) if d < 1.6 => {
                        let i = (0..l.len().saturating_sub(1)).min_by(|&i, &j| to_line(p, &l[i..i + 2]).total_cmp(&to_line(p, &l[j..j + 2]))).unwrap_or(0);
                        if l.len() >= 2 {
                            ang = (l[i + 1][1] - l[i][1]).atan2(l[i + 1][0] - l[i][0]);
                        }
                    }
                    _ => continue,
                }
            }
            placed.push((p, rr));
            out.built.builds[kind].push([p[0], p[1], sz, ang / std::f32::consts::TAU]);
            made += 1;
        }
    }
    out
}
