//! The land as people made it, for the game and its picture: the rivers as lines of water, the
//! villages, the ways — roads between the towns and from the villages to them, railways between
//! the larger towns in an age of steam. Each way is found over the land as a traveller would go —
//! round mountains and marshes, up gentle slopes, over a river only where it must (a bridge),
//! along a road already made where one leads the same way — then its pixel steps are straightened
//! and bent smooth. The railways are lines of supply in a war (`front::Campaign::grade`); the
//! window lays all of them on the land (`native::map::ways`).

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use strategy::military::front::{Campaign, Theatre, NO};
use strategy::rng::{hash64, Rng};
use strategy::world::countries::Politics;
use strategy::world::map::{Terrain, World, NONE};
use strategy::world::names;

/// The towns of the map: each country's capital and its most populous provinces (up to eight), a
/// name each; size 3 — the capital, 2 — the next two, 1 — the rest.
pub fn towns(world: &World, pol: &Politics, seed: u64) -> Vec<(usize, String, u8)> {
    let mut out = Vec::new();
    let mut rng = Rng::new(hash64(seed ^ 0x70E));
    let mut taken = std::collections::HashSet::new();
    for (c, country) in pol.countries.iter().enumerate() {
        let mut mine: Vec<usize> = (0..world.provinces.len()).filter(|&p| !world.provinces[p].sea && pol.country_of(p) == c as u32).collect();
        if mine.is_empty() {
            continue;
        }
        mine.sort_by(|&a, &b| pol.prov[b].pop.total_cmp(&pol.prov[a].pop));
        let cap = country.capital as usize;
        if let Some(k) = mine.iter().position(|&p| p == cap) {
            mine.remove(k);
            mine.insert(0, cap);
        }
        let n = (mine.len() / 10).clamp(1, 8);
        for (k, &p) in mine.iter().take(n).enumerate() {
            let size = if k == 0 { 3 } else if k < 3 { 2 } else { 1 };
            out.push((p, names::place(&mut rng, &mut taken), size));
        }
    }
    out
}

/// What a step costs along a railway and along a road laid for supply (a step off them — 1).
pub const RAIL_GRADE: f32 = 0.4;
pub const ROAD_GRADE: f32 = 0.65;

/// The land as people made it: the rivers (points, bounds, width in world pixels), the
/// villages (provinces), the ways (roads, railways), the pixels of the larger rivers (a way over
/// them is a bridge).
pub struct Land {
    pub rivers: Vec<(Vec<[f32; 2]>, [f32; 4], f32)>,
    pub villages: Vec<usize>,
    pub ways: Vec<Line>,
    pub river_px: Vec<bool>,
}

impl Land {
    /// The land of a world: its towns (province, name, size), each province's people, whether
    /// its age has railways.
    pub fn of(world: &World, towns: &[(usize, String, u8)], people: &[f32], rails: bool) -> Land {
        let rivers = rivers(world);
        let villages = villages_of(world, towns, people);
        let mut river_px = vec![false; world.w * world.h];
        for (pts, _, wd) in &rivers {
            if *wd >= 0.25 {
                // (every pixel the water covers: a way along the river's bed is no road)
                let r = (wd / 2.0).max(0.5);
                let reach = r.ceil() as i64;
                for p in pts {
                    for dy in -reach..=reach {
                        for dx in -reach..=reach {
                            if ((dx * dx + dy * dy) as f32).sqrt() > r {
                                continue;
                            }
                            let (x, y) = ((p[0].floor() as i64 + dx).rem_euclid(world.w as i64) as usize, ((p[1].max(0.0) as i64 + dy).clamp(0, world.h as i64 - 1)) as usize);
                            river_px[y * world.w + x] = true;
                        }
                    }
                }
            }
        }
        let town_list: Vec<(usize, u8)> = towns.iter().map(|t| (t.0, t.2)).collect();
        let ways = network(world, &town_list, &villages, &river_px, rails);
        Land { rivers, villages, ways, river_px }
    }
}

/// The provinces of the theatre a line on the land passes through, in its order (each once).
pub fn crossed(world: &World, th: &Theatre, pts: &[[f32; 2]]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for k in 0..pts.len().saturating_sub(1) {
        let (a, b) = (pts[k], pts[k + 1]);
        let n = ((b[0] - a[0]).hypot(b[1] - a[1]) * 2.0).ceil().max(1.0) as usize;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let (x, y) = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
            let (xi, yi) = ((x.floor() as i64).rem_euclid(world.w as i64) as usize, (y.max(0.0) as usize).min(world.h - 1));
            let p = world.owner[yi * world.w + xi];
            if p == NONE {
                continue;
            }
            let q = th.at[p as usize];
            if q != NO && out.last() != Some(&(q as usize)) && !out.contains(&(q as usize)) {
                out.push(q as usize);
            }
        }
    }
    out
}

/// The grades of the theatre's provinces the land's railways pass through (`RAIL_GRADE`; others
/// 1); none if no railway passes through it.
pub fn rail_grades(world: &World, th: &Theatre, land: &Land) -> Vec<f32> {
    let mut g = vec![1.0f32; th.provs.len()];
    let mut any = false;
    for l in land.ways.iter().filter(|l| l.kind == RAIL) {
        for q in crossed(world, th, &l.pts) {
            g[q] = RAIL_GRADE;
            any = true;
        }
    }
    if any {
        g
    } else {
        Vec::new()
    }
}

/// The shortest way through side `s`'s own provinces of the theatre from `a` to `b` (both
/// included), if there is one.
pub fn plan(war: &Campaign, s: usize, a: usize, b: usize) -> Option<Vec<usize>> {
    let n = war.owner.len();
    if a >= n || b >= n || war.owner[a] != s || war.owner[b] != s {
        return None;
    }
    let mut from = vec![usize::MAX; n];
    from[a] = a;
    let mut q = VecDeque::from([a]);
    while let Some(p) = q.pop_front() {
        if p == b {
            break;
        }
        for &r in &war.th.nbr[p] {
            if war.owner[r] == s && from[r] == usize::MAX {
                from[r] = p;
                q.push_back(r);
            }
        }
    }
    if from[b] == usize::MAX {
        return None;
    }
    let mut path = vec![b];
    while *path.last().unwrap() != a {
        path.push(from[*path.last().unwrap()]);
    }
    path.reverse();
    Some(path)
}

/// A line on the land through the middles of map provinces, one after another (the way between
/// each two found over the land, as the roads are), smoothed.
pub fn line_through(world: &World, provs: &[usize], river: &[bool]) -> Vec<[f32; 2]> {
    let mut pts: Vec<[f32; 2]> = Vec::new();
    let none = vec![false; river.len()];
    for k in 0..provs.len().saturating_sub(1) {
        let (a, b) = (world.provinces[provs[k]].seed, world.provinces[provs[k + 1]].seed);
        let Some(mut p) = way(world, a, b, river, &none) else { continue };
        // (keep the line whole across the world's seam: each piece starts where the last ended)
        if let (Some(last), Some(first)) = (pts.last(), p.first()) {
            let dx = ((last[0] - first[0]) / world.w as f32).round() * world.w as f32;
            for q in p.iter_mut() {
                q[0] += dx;
            }
        }
        pts.extend(p);
    }
    smooth(&simplify(&pts, 0.9), 3)
}

/// A line on the land (world pixels; x may run past the world's edge — the map goes round) and
/// its kind (`MAIN`, `LOCAL`, `RAIL`).
#[derive(Clone, Debug)]
pub struct Line {
    pub pts: Vec<[f32; 2]>,
    pub kind: u8,
}

pub const MAIN: u8 = 0;
pub const LOCAL: u8 = 1;
pub const RAIL: u8 = 2;

/// Fewer points where the line runs straight (Douglas–Peucker: none farther than `eps` from the
/// line kept).
fn simplify_keep(pts: &[[f32; 2]], eps: f32) -> Vec<bool> {
    if pts.len() < 3 {
        return vec![true; pts.len()];
    }
    let mut keep = vec![false; pts.len()];
    keep[0] = true;
    keep[pts.len() - 1] = true;
    let mut stack = vec![(0usize, pts.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        let (p, q) = (pts[a], pts[b]);
        let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let mut best = (0.0f32, 0usize);
        for (i, r) in pts.iter().enumerate().take(b).skip(a + 1) {
            let d = ((r[0] - p[0]) * dy - (r[1] - p[1]) * dx).abs() / len;
            if d > best.0 {
                best = (d, i);
            }
        }
        if best.0 > eps {
            keep[best.1] = true;
            stack.push((a, best.1));
            stack.push((best.1, b));
        }
    }
    keep
}

pub fn simplify(pts: &[[f32; 2]], eps: f32) -> Vec<[f32; 2]> {
    let keep = simplify_keep(pts, eps);
    pts.iter().zip(&keep).filter(|(_, &k)| k).map(|(p, _)| *p).collect()
}

/// Corners cut `rounds` times (the ends kept): a broken line becomes a smooth bend.
pub fn smooth(pts: &[[f32; 2]], rounds: usize) -> Vec<[f32; 2]> {
    let mut sm = pts.to_vec();
    if sm.len() < 3 {
        return sm;
    }
    for _ in 0..rounds {
        let mut next = vec![sm[0]];
        for k in 0..sm.len() - 1 {
            let (a, b) = (sm[k], sm[k + 1]);
            next.push([0.75 * a[0] + 0.25 * b[0], 0.75 * a[1] + 0.25 * b[1]]);
            next.push([0.25 * a[0] + 0.75 * b[0], 0.25 * a[1] + 0.75 * b[1]]);
        }
        next.push(*sm.last().unwrap());
        sm = next;
    }
    sm
}

/// What a step onto a pixel costs a traveller (None: water he cannot cross).
fn cost(world: &World, i: usize, j: usize, river: &[bool], road: &[bool]) -> Option<f32> {
    let t = world.terrain[j];
    if matches!(t, Terrain::Ocean | Terrain::Lake) {
        return None;
    }
    let ground = match t {
        Terrain::Plains => 1.0,
        Terrain::Forest => 1.7,
        Terrain::Hills => 2.2,
        Terrain::Mountains => 6.0,
        Terrain::Desert => 1.4,
        Terrain::Marsh => 3.5,
        Terrain::Jungle => 2.8,
        Terrain::Tundra => 1.5,
        Terrain::Arctic => 3.5,
        Terrain::Ocean | Terrain::Lake => 1.0,
    };
    let slope = (world.height[j] - world.height[i]).abs();
    let mut c = ground + 60.0 * slope;
    if river[j] {
        // (a bridge costs most where the water is first met, and something for every pixel of it)
        c += if river[i] { 5.0 } else { 14.0 };
    }
    if road[j] {
        c *= 0.3;
    }
    Some(c)
}

/// The cheapest way over the land from pixel `a` to pixel `b` (searched in a box round both;
/// the map goes round east to west), as pixel points (x unwrapped from `a`'s), or none.
pub fn way(world: &World, a: usize, b: usize, river: &[bool], road: &[bool]) -> Option<Vec<[f32; 2]>> {
    let (w, h) = (world.w as i64, world.h as i64);
    let (ax, ay) = (a as i64 % w, a as i64 / w);
    let (mut bx, by) = (b as i64 % w, b as i64 / w);
    if bx - ax > w / 2 {
        bx -= w;
    } else if ax - bx > w / 2 {
        bx += w;
    }
    let dist = (((bx - ax) * (bx - ax) + (by - ay) * (by - ay)) as f32).sqrt();
    let pad = (dist * 0.45) as i64 + 12;
    let (x0, x1) = (ax.min(bx) - pad, ax.max(bx) + pad);
    let (y0, y1) = ((ay.min(by) - pad).max(0), (ay.max(by) + pad).min(h - 1));
    let (bw, bh) = ((x1 - x0 + 1) as usize, (y1 - y0 + 1) as usize);
    let idx = |x: i64, y: i64| ((y - y0) as usize) * bw + (x - x0) as usize;
    let wpix = |x: i64, y: i64| (y * w + x.rem_euclid(w)) as usize;
    let mut g = vec![f32::INFINITY; bw * bh];
    let mut from = vec![u32::MAX; bw * bh];
    let mut heap: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();
    let key = |f: f32| f.to_bits();
    let est = |x: i64, y: i64| 0.3 * (((x - bx) * (x - bx) + (y - by) * (y - by)) as f32).sqrt();
    let s = idx(ax, ay);
    g[s] = 0.0;
    heap.push(Reverse((key(est(ax, ay)), s as u32)));
    let goal = idx(bx, by);
    while let Some(Reverse((_, c))) = heap.pop() {
        let c = c as usize;
        if c == goal {
            break;
        }
        let (cx, cy) = ((c % bw) as i64 + x0, (c / bw) as i64 + y0);
        let gc = g[c];
        for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let (nx, ny) = (cx + dx, cy + dy);
            if nx < x0 || nx > x1 || ny < y0 || ny > y1 {
                continue;
            }
            let Some(step) = cost(world, wpix(cx, cy), wpix(nx, ny), river, road) else {
                continue;
            };
            let n = idx(nx, ny);
            let ng = gc + step * if dx != 0 && dy != 0 { 1.414 } else { 1.0 };
            if ng < g[n] {
                g[n] = ng;
                from[n] = c as u32;
                heap.push(Reverse((key(ng + est(nx, ny)), n as u32)));
            }
        }
    }
    if from[goal] == u32::MAX {
        return None;
    }
    let mut pts = Vec::new();
    let mut c = goal;
    loop {
        pts.push([(c % bw) as f32 + x0 as f32 + 0.5, (c / bw) as f32 + y0 as f32 + 0.5]);
        if c == s {
            break;
        }
        c = from[c] as usize;
    }
    pts.reverse();
    Some(pts)
}

/// Pairs of places worth a way between them: each place to those near it that no third place
/// stands between (a relative neighbourhood graph), no longer than `most` world pixels.
fn pairs(xy: &[(f32, f32)], w: f32, most: f32) -> Vec<(usize, usize, f32)> {
    let d = |a: usize, b: usize| {
        let dx = (xy[a].0 - xy[b].0).abs();
        let dx = dx.min(w - dx);
        dx.hypot(xy[a].1 - xy[b].1)
    };
    let n = xy.len();
    let mut out = Vec::new();
    for a in 0..n {
        for b in a + 1..n {
            let ab = d(a, b);
            if ab > most {
                continue;
            }
            if (0..n).any(|c| c != a && c != b && d(a, c).max(d(b, c)) < ab) {
                continue;
            }
            out.push((a, b, ab));
        }
    }
    out.sort_by(|x, y| x.2.total_cmp(&y.2));
    out
}

/// The ways of a world: main roads between the towns, roads from the villages to the nearest
/// way, and (`rails`) railways between the larger towns. `river` marks the pixels of rivers
/// (a crossing there is a bridge).
pub fn network(world: &World, towns: &[(usize, u8)], villages: &[usize], river: &[bool], rails: bool) -> Vec<Line> {
    let w = world.w;
    let at = |p: usize| world.provinces[p].seed;
    let xy = |i: usize| ((i % w) as f32 + 0.5, (i / w) as f32 + 0.5);
    let mut road = vec![false; w * world.h];
    let mut out: Vec<Line> = Vec::new();
    // a way laid: where it runs along a road already made it is that road (only the new
    // stretches are laid, each reaching a pixel into the old road, so that they join it)
    let lay = |pts: Vec<[f32; 2]>, kind: u8, road: &mut Vec<bool>, out: &mut Vec<Line>| {
        let pix = |p: &[f32; 2]| ((p[1] as usize).min(world.h - 1)) * w + (p[0].floor() as i64).rem_euclid(w as i64) as usize;
        let runs: Vec<Vec<[f32; 2]>> = if kind == RAIL {
            vec![pts.clone()]
        } else {
            // (a pixel next to a road counts as on it: two roads side by side are one road)
            let near_road = |p: &[f32; 2]| -> bool {
                (-1i64..=1).any(|dy| (-1i64..=1).any(|dx| {
                    let (x, y) = ((p[0].floor() as i64 + dx).rem_euclid(w as i64) as usize, (p[1] as i64 + dy).clamp(0, world.h as i64 - 1) as usize);
                    road[y * w + x]
                }))
            };
            let old: Vec<bool> = pts.iter().map(|p| near_road(p)).collect();
            let mut runs = Vec::new();
            let mut i = 0;
            while i < pts.len() {
                if old[i] {
                    i += 1;
                    continue;
                }
                let mut j = i;
                while j < pts.len() && !old[j] {
                    j += 1;
                }
                let (a, b) = (i.saturating_sub(1), (j + 1).min(pts.len()));
                if b - a >= 2 {
                    runs.push(pts[a..b].to_vec());
                }
                i = j;
            }
            for p in &pts {
                road[pix(p)] = true;
            }
            runs
        };
        for r in runs {
            out.push(Line { pts: smooth(&simplify(&r, 0.9), 3), kind });
        }
    };
    // the main roads
    let tp: Vec<(f32, f32)> = towns.iter().map(|t| xy(at(t.0))).collect();
    for (a, b, _) in pairs(&tp, w as f32, 220.0) {
        if let Some(p) = way(world, at(towns[a].0), at(towns[b].0), river, &road) {
            lay(p, MAIN, &mut road, &mut out);
        }
    }
    // the villages: a road to the nearest town or way, if it is near
    for &v in villages {
        let s = at(v);
        // (the nearest pixel of a road or a town, looked for in a widening square)
        let mut best: Option<(f32, usize)> = None;
        for r in 1..40i64 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (x, y) = ((s % w) as i64 + dx, (s / w) as i64 + dy);
                    if y < 0 || y >= world.h as i64 {
                        continue;
                    }
                    let i = (y as usize) * w + x.rem_euclid(w as i64) as usize;
                    if road[i] {
                        let d = ((dx * dx + dy * dy) as f32).sqrt();
                        if best.map_or(true, |b| d < b.0) {
                            best = Some((d, i));
                        }
                    }
                }
            }
            if best.is_some_and(|b| b.0 <= r as f32) {
                break;
            }
        }
        if let Some((d, i)) = best {
            if d > 2.0 {
                if let Some(p) = way(world, s, i, river, &road) {
                    lay(p, LOCAL, &mut road, &mut out);
                }
            }
        }
    }
    // the railways: between the larger towns (they keep to the gentle ground more than roads)
    if rails {
        let big: Vec<usize> = (0..towns.len()).filter(|&t| towns[t].1 >= 2).collect();
        let bp: Vec<(f32, f32)> = big.iter().map(|&t| tp[t]).collect();
        let no_road = vec![false; road.len()];
        for (a, b, _) in pairs(&bp, w as f32, 300.0) {
            if let Some(p) = way(world, at(towns[big[a]].0), at(towns[big[b]].0), river, &no_road) {
                lay(p, RAIL, &mut road, &mut out);
            }
        }
    }
    out
}

/// The villages: the more peopled half of the provinces without a town (their middles).
fn villages_of(world: &World, towns: &[(usize, String, u8)], people: &[f32]) -> Vec<usize> {
    let town: std::collections::HashSet<usize> = towns.iter().map(|t| t.0).collect();
    let land: Vec<usize> = (0..world.provinces.len()).filter(|&p| !world.provinces[p].sea && !town.contains(&p)).collect();
    let mut pop: Vec<f32> = land.iter().map(|&p| people.get(p).copied().unwrap_or(0.0)).collect();
    pop.sort_by(|a, b| a.total_cmp(b));
    let cut = pop.get(pop.len() / 2).copied().unwrap_or(0.0);
    land.into_iter().filter(|&p| people.get(p).copied().unwrap_or(0.0) > cut && world.provinces[p].terrain != Terrain::Mountains).collect()
}

/// A river's width in world pixels from the water that gathers in it, as a multiple of the
/// least river's: brooks stay thin, the great rivers run many pixels wide.
fn river_width(ratio: f32) -> f32 {
    let l = ratio.max(1.0).ln();
    if l <= 1.8 {
        0.16 + 0.24 * l
    } else {
        0.592 + 4.0 * ((l - 1.8) / 2.3).min(1.25).powf(1.4)
    }
}

/// The rivers as lines: each way of water from its source down to the sea, a lake or the river
/// it joins. Every point carries the river's width there, which grows downstream (a brook, a
/// stream, a river, a great river) and flares into an estuary at the open sea; on the flat land
/// the way wanders in bends that grow with the width; then the line is straightened and bent
/// smooth, and cut into short pieces, each with the width of its middle.
pub fn rivers(world: &World) -> Vec<(Vec<[f32; 2]>, [f32; 4], f32)> {
    let w = world.w;
    let wf = w as f32;
    // (brooks too: the network of a map that draws the water as lines is denser than the one the front's picture needs)
    let segs = strategy::world::render::rivers_min(world, true, 260.0);
    let mut down: std::collections::HashMap<usize, (usize, f32)> = std::collections::HashMap::new();
    let mut has_up: std::collections::HashSet<usize> = std::collections::HashSet::new();
    // (the width scale is fixed at the old least river: a brook is thin, the great rivers as wide as before)
    let least = 450.0f32;
    if std::env::var("RIVERDBG").is_ok() {
        let mut fl: Vec<f32> = segs.iter().map(|s| s.2).collect();
        fl.sort_by(|a, b| a.total_cmp(b));
        let q = |f: f32| fl[((fl.len() - 1) as f32 * f) as usize];
        eprintln!("реки: клеток {}, наименьший сток {least}; 50% {:.0}, 90% {:.0}, 99% {:.0}, наибольший {:.0}", fl.len(), q(0.5), q(0.9), q(0.99), fl[fl.len() - 1]);
    }
    for &(i, j, f) in &segs {
        down.insert(i, (j, f));
        has_up.insert(j);
    }
    // cells from each one down to the mouth (the cells come from the sea upstream), and whether
    // the mouth is in the open sea
    let mut to_sea: std::collections::HashMap<usize, (u32, bool)> = std::collections::HashMap::new();
    for &(i, j, _) in &segs {
        let d = to_sea.get(&j).map_or((0, world.terrain[j] == Terrain::Ocean), |&(d, open)| (d + 1, open));
        to_sea.insert(i, d);
    }
    let noise = strategy::noise::Noise::new(0x71E5);
    // a point of the way: where the river passes the cell and how wide it is there. A cell is
    // always given the same point, so a brook ends exactly on the river it joins.
    let width_at = |i: usize, f: f32| {
        let mut wd = river_width(f / least);
        if let Some(&(d, true)) = to_sea.get(&i) {
            if d < 10 && wd > 0.8 {
                let t = 1.0 - d as f32 / 10.0;
                wd *= 1.0 + 0.6 * t * t;
            }
        }
        wd
    };
    let place = |i: usize| ((i % w) as f32 + 0.5, (i / w) as f32 + 0.5);
    let node = |i: usize, f: f32| -> [f32; 3] {
        // the cell's middle, evened out with the next six downstream (a staircase of cells would
        // show on the bank of a wide river); the same for a cell whichever river asks, and the
        // seam of the map kept whole
        let (x0, y0) = place(i);
        let (mut sx, mut sy, mut n) = (x0, y0, 1.0f32);
        let mut at = i;
        for _ in 0..6 {
            let Some(&(j, _)) = down.get(&at) else { break };
            at = j;
            let (mut px, py) = place(at);
            if px - x0 > wf / 2.0 {
                px -= wf;
            } else if x0 - px > wf / 2.0 {
                px += wf;
            }
            sx += px;
            sy += py;
            n += 1.0;
        }
        let (x, y) = (sx / n, sy / n);
        let wd = width_at(i, f);
        let d = to_sea.get(&i).map_or(0, |d| d.0);
        let land = match world.terrain[i] {
            Terrain::Hills => 0.5,
            Terrain::Mountains => 0.15,
            _ => 1.0,
        };
        let taper = (d as f32 / 6.0).min(1.0);
        // the way's direction a few cells on downstream (the seam of the map kept whole)
        let mut at = i;
        for _ in 0..5 {
            match down.get(&at) {
                Some(&(j, _)) => at = j,
                None => break,
            }
        }
        let (ax, ay) = place(at);
        let mut tx = ax - x;
        if tx > wf / 2.0 {
            tx -= wf;
        } else if tx < -wf / 2.0 {
            tx += wf;
        }
        let ty = ay - y;
        let tl = tx.hypot(ty).max(1e-3);
        let (nx, ny) = (-ty / tl, tx / tl);
        // meanders on the flat land: a swing to either side, by the cells from the mouth (so a
        // brook's end falls on the river it joins), its length and reach growing with the width
        let phase = 2.5 * noise.get(x / 37.0, y / 37.0, 0.5);
        let len = 7.0 * wd + 14.0;
        let tau = std::f32::consts::TAU;
        let swing = (tau * d as f32 / len + phase).sin() + 0.45 * (tau * d as f32 / (0.41 * len) + 1.7 + 2.0 * phase).sin();
        let reach = (0.9 * wd + 0.15).min(4.2) * land * taper * 0.8;
        // (and a small drift off the straight, whatever the way)
        let lam = 8.0 + 5.0 * wd;
        let amp = (0.2 * wd + 0.1).min(1.0) * land * taper * 1.6;
        let dx = noise.get(x / lam, y / lam, 0.31) * amp;
        let dy = noise.get(x / lam + 9.7, y / lam + 3.1, 0.77) * amp;
        [x + nx * reach * swing + dx, y + ny * reach * swing + dy, wd]
    };
    let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mut heads: Vec<usize> = segs.iter().map(|s| s.0).filter(|i| !has_up.contains(i)).collect();
    heads.sort_unstable();
    for h in heads {
        let mut i = h;
        let mut pts = vec![node(i, down[&i].1)];
        while let Some(&(j, f)) = down.get(&i) {
            let last = *pts.last().unwrap();
            let mut q = match down.get(&j) {
                Some(&(_, fj)) => node(j, fj),
                // (the mouth: a cell of the sea or a lake, as wide as the river's last cell)
                None => [(j % w) as f32 + 0.5, (j / w) as f32 + 0.5, width_at(i, f)],
            };
            // (the map goes round: keep the line whole across the seam)
            if q[0] - last[0] > wf / 2.0 {
                q[0] -= wf;
            } else if last[0] - q[0] > wf / 2.0 {
                q[0] += wf;
            }
            pts.push(q);
            if !seen.insert(j) || !down.contains_key(&j) {
                break;
            }
            i = j;
        }
        if pts.len() >= 2 {
            out.push(pts);
        }
    }
    let mut lines = Vec::new();
    for pts in out {
        // the pixels' steps straightened (none farther than a quarter of a pixel from the way of the
        // water), then bent smooth by two rounds of corner cutting (ends kept)
        let flat: Vec<[f32; 2]> = pts.iter().map(|p| [p[0], p[1]]).collect();
        let keep = simplify_keep(&flat, 0.25);
        let kept: Vec<[f32; 3]> = pts.iter().zip(&keep).filter(|(_, &k)| k).map(|(p, _)| *p).collect();
        let sm = smooth3(&kept, 2);
        // (pieces of up to four steps, a point shared between neighbours)
        let mut at = 0;
        while at + 1 < sm.len() {
            let to = (at + 4).min(sm.len() - 1);
            let run = &sm[at..=to];
            let wd = run.iter().map(|p| p[2]).sum::<f32>() / run.len() as f32;
            let xy: Vec<[f32; 2]> = run.iter().map(|p| [p[0], p[1]]).collect();
            let mut bb = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
            for q in &xy {
                bb = [bb[0].min(q[0]), bb[1].min(q[1]), bb[2].max(q[0]), bb[3].max(q[1])];
            }
            lines.push((xy, bb, wd));
            at = to;
        }
    }
    lines
}

/// The rivers' pieces gathered into whole rivers: runs of pieces each starting where the last one ended
/// (`rivers` gives them so, a river's pieces one after another, from its source down).
pub fn river_chains(rivers: &[(Vec<[f32; 2]>, [f32; 4], f32)]) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = Vec::new();
    for (i, r) in rivers.iter().enumerate() {
        let joins = i > 0 && rivers[i - 1].0.last() == r.0.first();
        match (joins, out.last_mut()) {
            (true, Some(c)) => c.push(i),
            _ => out.push(vec![i]),
        }
    }
    out
}

/// Corners cut `rounds` times like `smooth`, carrying a third number (the width) along.
fn smooth3(pts: &[[f32; 3]], rounds: usize) -> Vec<[f32; 3]> {
    let mut sm = pts.to_vec();
    if sm.len() < 3 {
        return sm;
    }
    let mix = |a: [f32; 3], b: [f32; 3], t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
    for _ in 0..rounds {
        let mut next = vec![sm[0]];
        for k in 0..sm.len() - 1 {
            next.push(mix(sm[k], sm[k + 1], 0.25));
            next.push(mix(sm[k], sm[k + 1], 0.75));
        }
        next.push(*sm.last().unwrap());
        sm = next;
    }
    sm
}
