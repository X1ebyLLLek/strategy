//! Province outlines as smooth vector shapes, for the map seen close up.
//!
//! The map is pixels (a province per pixel); seen close, its borders are steps and blur. Here each
//! border between two provinces becomes a chain of points from one meeting point of three (or more)
//! provinces to the next — the steps cut into diagonals and the corners rounded (Chaikin) — each
//! chain once, so neighbours share it and no gap opens between them. The window strokes borders
//! along the chains.

use crate::world::map::{World, NONE};

/// A lake on a side of a chain (lake pixels have no province).
pub const LAKE: u32 = 1 << 20;
/// Beyond the map's top or bottom.
pub const FRAME: u32 = LAKE + 1;
/// The map's east-west seam (the map wraps): `SEAM + what lies across it`.
pub const SEAM: u32 = 1 << 21;

/// A border: what lies on its left and on its right (walking from its first point to its last;
/// the y axis goes down), and its points in world pixels.
pub struct Chain {
    pub left: u32,
    pub right: u32,
    pub pts: Vec<[f32; 2]>,
}

pub struct Shapes {
    pub chains: Vec<Chain>,
}

/// Rounds of corner-cutting after the steps are cut into diagonals.
const SMOOTH: usize = 2;
/// Points closer than this to the line through their neighbours are dropped (world pixels).
const KEEP: f32 = 0.1;

impl Shapes {
    pub fn of(world: &World) -> Shapes {
        let (w, h) = (world.w, world.h);
        let np = world.provinces.len() as u32;
        let label = |x: usize, y: usize| -> u32 {
            let o = world.owner[y * w + x];
            if o == NONE {
                LAKE
            } else {
                o
            }
        };
        let land = |l: u32| l < np && !world.provinces[l as usize].sea;
        // unit edges of the pixel grid: vertical V(X, y) from vertex (X, y) down to (X, y+1),
        // horizontal H(x, Y) from (x, Y) right to (x+1, Y); what lies left and right of each
        let nv = (w + 1) * h;
        let ne = nv + w * (h + 1);
        let sides = |e: usize| -> (u32, u32) {
            if e < nv {
                let (x, y) = (e % (w + 1), e / (w + 1));
                let l = if x == 0 { SEAM + label(w - 1, y) } else { label(x - 1, y) };
                let r = if x == w { SEAM + label(0, y) } else { label(x, y) };
                (l, r)
            } else {
                let e = e - nv;
                let (x, y) = (e % w, e / w);
                let l = if y == h { FRAME } else { label(x, y) };
                let r = if y == 0 { FRAME } else { label(x, y - 1) };
                (l, r)
            }
        };
        let keep: Vec<bool> = (0..ne)
            .map(|e| {
                let (l, r) = sides(e);
                l != r && (land(l) || land(r))
            })
            .collect();
        let vid = |x: usize, y: usize| y * (w + 1) + x;
        // the kept edges at a vertex, each with the vertex at its other end
        let around = |x: usize, y: usize| -> ([(usize, usize); 4], usize) {
            let mut out = [(0usize, 0usize); 4];
            let mut n = 0;
            if y > 0 && keep[vid(x, y - 1)] {
                out[n] = (vid(x, y - 1), vid(x, y - 1));
                n += 1;
            }
            if y < h && keep[vid(x, y)] {
                out[n] = (vid(x, y), vid(x, y + 1));
                n += 1;
            }
            if x > 0 && keep[nv + y * w + x - 1] {
                out[n] = (nv + y * w + x - 1, vid(x - 1, y));
                n += 1;
            }
            if x < w && keep[nv + y * w + x] {
                out[n] = (nv + y * w + x, vid(x + 1, y));
                n += 1;
            }
            (out, n)
        };
        let pair = |e: usize| {
            let (l, r) = sides(e);
            (l.min(r), l.max(r))
        };
        // a vertex where a chain ends: not two edges, or two edges between different provinces
        let node = |v: usize| {
            let (x, y) = (v % (w + 1), v / (w + 1));
            let (a, n) = around(x, y);
            n != 2 || pair(a[0].0) != pair(a[1].0)
        };
        let mut seen = vec![false; ne];
        // (left, right, vertices, whether it closes on itself away from any meeting point)
        let mut chains: Vec<(u32, u32, Vec<usize>, bool)> = Vec::new();
        let walk = |v0: usize, e0: usize, seen: &mut Vec<bool>| -> (u32, u32, Vec<usize>) {
            let mut pts = vec![v0];
            let (mut cur, mut e) = (v0, e0);
            loop {
                seen[e] = true;
                let (x, y) = (cur % (w + 1), cur / (w + 1));
                let (a, n) = around(x, y);
                let next = a[..n].iter().find(|t| t.0 == e).map(|t| t.1).unwrap();
                pts.push(next);
                if next == v0 || node(next) {
                    break;
                }
                let (nx, ny) = (next % (w + 1), next / (w + 1));
                let (b, m) = around(nx, ny);
                match b[..m].iter().find(|t| t.0 != e && !seen[t.0]) {
                    Some(t) => {
                        cur = next;
                        e = t.0;
                    }
                    None => break,
                }
            }
            // what lies left and right, walking from the first point to the second
            let (l, r) = sides(e0);
            // (a vertical edge walked down, a horizontal one walked right: the next vertex has the
            // larger number)
            if pts[1] > pts[0] {
                (l, r, pts)
            } else {
                (r, l, pts)
            }
        };
        for y in 0..=h {
            for x in 0..=w {
                let v = vid(x, y);
                if !node(v) {
                    continue;
                }
                let (a, n) = around(x, y);
                for &(e, _) in &a[..n] {
                    if !seen[e] {
                        let (l, r, pts) = walk(v, e, &mut seen);
                        chains.push((l, r, pts, false));
                    }
                }
            }
        }
        // borders that close on themselves (an island province, a lake inside one)
        for e in 0..ne {
            if keep[e] && !seen[e] {
                let v0 = if e < nv { e } else { let k = e - nv; vid(k % w, k / w) };
                let (l, r, pts) = walk(v0, e, &mut seen);
                chains.push((l, r, pts, true));
            }
        }
        // smooth: the steps cut into diagonals, corners rounded; the ends stay where they are (they
        // are shared with the other chains meeting there)
        let chains = chains
            .into_iter()
            .map(|(left, right, raw, closed)| {
                let pts: Vec<[f32; 2]> = raw.iter().map(|&v| [(v % (w + 1)) as f32, (v / (w + 1)) as f32]).collect();
                let edge = left >= FRAME || right >= FRAME;
                let pts = if edge {
                    straight(&pts)
                } else if closed {
                    smooth_loop(&pts)
                } else {
                    smooth_open(&pts)
                };
                Chain { left, right, pts }
            })
            .collect();
        Shapes { chains }
    }

}

/// A straight border along the map's edge: its two ends.
fn straight(p: &[[f32; 2]]) -> Vec<[f32; 2]> {
    vec![p[0], p[p.len() - 1]]
}

fn mid(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// An open chain: its ends kept; the middles of its unit steps, then corners cut.
fn smooth_open(p: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let n = p.len();
    let mut q = Vec::with_capacity(n + 1);
    q.push(p[0]);
    for i in 1..n {
        q.push(mid(p[i - 1], p[i], 0.5));
    }
    q.push(p[n - 1]);
    for _ in 0..SMOOTH {
        let mut r = Vec::with_capacity(q.len() * 2);
        r.push(q[0]);
        for i in 0..q.len() - 1 {
            if i > 0 {
                r.push(mid(q[i], q[i + 1], 0.25));
            }
            if i + 1 < q.len() - 1 {
                r.push(mid(q[i], q[i + 1], 0.75));
            }
        }
        r.push(q[q.len() - 1]);
        q = r;
    }
    simplify(&q)
}

/// A closed chain (first point = last): the middles of its steps, then corners cut, all round.
fn smooth_loop(p: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let n = p.len() - 1;
    let mut q: Vec<[f32; 2]> = (0..n).map(|i| mid(p[i], p[i + 1], 0.5)).collect();
    for _ in 0..SMOOTH {
        let m = q.len();
        let mut r = Vec::with_capacity(m * 2);
        for i in 0..m {
            r.push(mid(q[i], q[(i + 1) % m], 0.25));
            r.push(mid(q[i], q[(i + 1) % m], 0.75));
        }
        q = r;
    }
    q.push(q[0]);
    simplify(&q)
}

/// Douglas–Peucker: points nearer than `KEEP` to the line through their neighbours go.
fn simplify(p: &[[f32; 2]]) -> Vec<[f32; 2]> {
    if p.len() <= 2 {
        return p.to_vec();
    }
    let mut keep = vec![false; p.len()];
    keep[0] = true;
    keep[p.len() - 1] = true;
    let mut stack = vec![(0usize, p.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (ax, ay, bx, by) = (p[a][0], p[a][1], p[b][0], p[b][1]);
        let (dx, dy) = (bx - ax, by - ay);
        let len = (dx * dx + dy * dy).sqrt();
        let mut far = (0.0f32, a);
        for (i, q) in p.iter().enumerate().take(b).skip(a + 1) {
            let d = if len < 1e-6 { ((q[0] - ax).powi(2) + (q[1] - ay).powi(2)).sqrt() } else { ((q[0] - ax) * dy - (q[1] - ay) * dx).abs() / len };
            if d > far.0 {
                far = (d, i);
            }
        }
        if far.0 > KEEP {
            keep[far.1] = true;
            stack.push((a, far.1));
            stack.push((far.1, b));
        }
    }
    p.iter().zip(&keep).filter(|(_, &k)| k).map(|(q, _)| *q).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::map::{generate, Params};

    /// Every border between two land provinces is drawn: some chain has one on each side (across
    /// the east-west seam a side is `SEAM + province`).
    #[test]
    fn every_border_has_a_chain() {
        let world = generate(&Params { seed: 3, w: 512, h: 256, land_frac: 0.32, land_provinces: 300 }).0;
        let s = Shapes::of(&world);
        let side = |x: u32| if x >= SEAM { x - SEAM } else { x };
        let pairs: std::collections::HashSet<(u32, u32)> = s
            .chains
            .iter()
            .map(|c| (side(c.left), side(c.right)))
            .map(|(l, r)| (l.min(r), l.max(r)))
            .collect();
        for (p, pr) in world.provinces.iter().enumerate() {
            if pr.sea {
                continue;
            }
            for &q in &pr.neighbors {
                if world.provinces[q as usize].sea {
                    continue;
                }
                let key = ((p as u32).min(q), (p as u32).max(q));
                assert!(pairs.contains(&key), "no chain between provinces {p} and {q}");
            }
        }
    }
}
