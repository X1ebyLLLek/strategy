use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet, VecDeque};
use std::f32::consts::TAU;
use std::time::{Duration, Instant};
use rayon::prelude::*;
use crate::noise::Noise;
use crate::rng::{shuffle, Rng};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Terrain {
    Ocean,
    Lake,
    Plains,
    Forest,
    Hills,
    Mountains,
    Desert,
    Marsh,
    Jungle,
    Tundra,
    Arctic,
}

pub const TERRAINS: [Terrain; 11] = [
    Terrain::Ocean,
    Terrain::Lake,
    Terrain::Plains,
    Terrain::Forest,
    Terrain::Hills,
    Terrain::Mountains,
    Terrain::Desert,
    Terrain::Marsh,
    Terrain::Jungle,
    Terrain::Tundra,
    Terrain::Arctic,
];

impl Terrain {
    pub fn name(self) -> &'static str {
        match self {
            Terrain::Ocean => "ocean",
            Terrain::Lake => "lake",
            Terrain::Plains => "plains",
            Terrain::Forest => "forest",
            Terrain::Hills => "hills",
            Terrain::Mountains => "mountains",
            Terrain::Desert => "desert",
            Terrain::Marsh => "marsh",
            Terrain::Jungle => "jungle",
            Terrain::Tundra => "tundra",
            Terrain::Arctic => "arctic",
        }
    }

    /// Provinces per unit of area, relative to plains: empty land gets big provinces.
    fn density(self) -> f32 {
        match self {
            Terrain::Plains => 1.0,
            Terrain::Forest => 0.8,
            Terrain::Hills => 0.7,
            Terrain::Marsh => 0.5,
            Terrain::Jungle => 0.45,
            Terrain::Mountains => 0.4,
            Terrain::Tundra => 0.3,
            Terrain::Desert => 0.25,
            Terrain::Arctic => 0.12,
            Terrain::Ocean => 0.08,
            Terrain::Lake => 0.0,
        }
    }
}

pub struct Params {
    pub seed: u64,
    pub w: usize,
    pub h: usize,
    pub land_frac: f32,
    pub land_provinces: usize,
}

pub struct Province {
    /// Pixel the province grew from: its centre for labels and distances.
    pub seed: usize,
    pub area: u32,
    pub terrain: Terrain,
    pub sea: bool,
    pub neighbors: Vec<u32>,
}

pub struct World {
    pub w: usize,
    pub h: usize,
    /// 0 = sea level.
    pub height: Vec<f32>,
    pub terrain: Vec<Terrain>,
    /// Province id per pixel; NONE on lakes.
    pub owner: Vec<u32>,
    pub provinces: Vec<Province>,
}

pub const NONE: u32 = u32::MAX;

// pixel classes
const LAND: u8 = 0;
const SEA: u8 = 1;
const LAKE: u8 = 2;

/// 4-neighbours; the map wraps east-west, not north-south.
#[inline]
pub fn neighbors4(i: usize, w: usize, h: usize) -> ([usize; 4], usize) {
    let (x, y) = (i % w, i / w);
    let mut out = [0; 4];
    out[0] = y * w + (x + w - 1) % w;
    out[1] = y * w + (x + 1) % w;
    let mut n = 2;
    if y > 0 {
        out[n] = i - w;
        n += 1;
    }
    if y + 1 < h {
        out[n] = i + w;
        n += 1;
    }
    (out, n)
}

/// 0 below `a`, 1 above `b`, smooth between.
pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Point on a cylinder of circumference w: noise sampled here wraps east-west.
#[inline]
pub fn cyl(x: usize, y: usize, w: usize) -> (f32, f32, f32) {
    let r = w as f32 / TAU;
    let th = x as f32 / w as f32 * TAU;
    (r * th.cos(), r * th.sin(), y as f32)
}

/// Connected components of equal values (4-neighbour, wrapping). Returns (label per pixel, size per label).
pub fn label<T: PartialEq + Copy>(key: &[T], w: usize, h: usize) -> (Vec<u32>, Vec<u32>) {
    let n = w * h;
    let mut lab = vec![u32::MAX; n];
    let mut sizes = Vec::new();
    let mut stack = Vec::new();
    for s in 0..n {
        if lab[s] != u32::MAX {
            continue;
        }
        let id = sizes.len() as u32;
        lab[s] = id;
        stack.push(s);
        let mut count = 0;
        while let Some(i) = stack.pop() {
            count += 1;
            let (nb, k) = neighbors4(i, w, h);
            for &j in &nb[..k] {
                if lab[j] == u32::MAX && key[j] == key[i] {
                    lab[j] = id;
                    stack.push(j);
                }
            }
        }
        sizes.push(count);
    }
    (lab, sizes)
}

fn quantile(v: &[f32], q: f32) -> f32 {
    let mut s = v.to_vec();
    let k = ((s.len() - 1) as f32 * q) as usize;
    s.select_nth_unstable_by(k, |a, b| a.total_cmp(b));
    s[k]
}

struct Timer {
    t: Instant,
    log: Vec<(&'static str, Duration)>,
}

impl Timer {
    fn lap(&mut self, name: &'static str) {
        let now = Instant::now();
        self.log.push((name, now - self.t));
        self.t = now;
    }
}

pub fn generate(p: &Params) -> (World, Vec<(&'static str, Duration)>) {
    let mut tm = Timer { t: Instant::now(), log: Vec::new() };

    let mut height = gen_height(p);
    tm.lap("height");

    let sea = quantile(&height, 1.0 - p.land_frac);
    height.par_iter_mut().for_each(|v| *v -= sea);
    let cls = classify_water(&mut height, p.w, p.h);
    tm.lap("coast/lakes");

    let terrain = gen_terrain(p, &height, &cls);
    tm.lap("climate");

    let (owner, provinces) = make_provinces(p, &height, &terrain, &cls);
    tm.lap("provinces");

    let world = World {
        w: p.w,
        h: p.h,
        height,
        terrain,
        owner,
        provinces,
    };
    (world, tm.log)
}

fn gen_height(p: &Params) -> Vec<f32> {
    let w = p.w;
    let s = p.seed.wrapping_mul(1000);
    let base = Noise::new(s + 1);
    let warp = [Noise::new(s + 2), Noise::new(s + 3), Noise::new(s + 4)];
    let ridge = Noise::new(s + 5);
    let belt = Noise::new(s + 6);
    // wavelengths are fractions of the map width, so a bigger map is the same world in finer detail
    let k = w as f32 / 2048.0;
    let mut out = vec![0.0f32; w * p.h];
    out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, v) in row.iter_mut().enumerate() {
            let (cx, cy, cz) = cyl(x, y, w);
            let ws = 1.0 / (230.0 * k);
            let amp = 80.0 * k;
            let qx = cx + amp * warp[0].fbm(cx * ws, cy * ws, cz * ws, 4);
            let qy = cy + amp * warp[1].fbm(cx * ws, cy * ws, cz * ws, 4);
            let qz = cz + amp * warp[2].fbm(cx * ws, cy * ws, cz * ws, 4);
            let bs = 1.0 / (450.0 * k);
            let b = base.fbm(qx * bs, qy * bs, qz * bs, 8);
            let rs = 1.0 / (150.0 * k);
            let rg = ridge.ridged(qx * rs, qy * rs, qz * rs, 6);
            let ms = 1.0 / (520.0 * k);
            let m = smoothstep(0.05, 0.3, belt.fbm(cx * ms, cy * ms, cz * ms, 3));
            *v = b + 0.35 * m * rg * rg;
        }
    });
    out
}

/// Sinks land specks, fills puddles, tells lakes from the ocean.
fn classify_water(height: &mut [f32], w: usize, h: usize) -> Vec<u8> {
    let n = w * h;
    let mut cls: Vec<u8> = height.iter().map(|&v| if v > 0.0 { LAND } else { SEA }).collect();
    let (lab, sizes) = label(&cls, w, h);
    for i in 0..n {
        let sz = sizes[lab[i] as usize];
        if cls[i] == LAND && sz < 16 {
            height[i] = -1e-4;
            cls[i] = SEA;
        } else if cls[i] == SEA && sz < 12 {
            height[i] = 1e-4;
            cls[i] = LAND;
        }
    }
    let (lab, sizes) = label(&cls, w, h);
    let lake_max = (n / 600).max(100) as u32;
    for i in 0..n {
        if cls[i] == SEA && sizes[lab[i] as usize] < lake_max {
            cls[i] = LAKE;
        }
    }
    cls
}

fn water_distance(cls: &[u8], w: usize, h: usize) -> Vec<u32> {
    let mut d = vec![u32::MAX; w * h];
    let mut q = VecDeque::new();
    for (i, &c) in cls.iter().enumerate() {
        if c != LAND {
            d[i] = 0;
            q.push_back(i);
        }
    }
    while let Some(i) = q.pop_front() {
        let (nb, k) = neighbors4(i, w, h);
        for &j in &nb[..k] {
            if d[j] == u32::MAX {
                d[j] = d[i] + 1;
                q.push_back(j);
            }
        }
    }
    d
}

fn gen_terrain(p: &Params, height: &[f32], cls: &[u8]) -> Vec<Terrain> {
    let (w, h) = (p.w, p.h);
    let n = w * h;
    let dist = water_distance(cls, w, h);
    let hmax = height.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let s = p.seed.wrapping_mul(1000);
    let tn = Noise::new(s + 7);
    let mn = Noise::new(s + 8);
    let k = w as f32 / 2048.0;

    let mut temp = vec![0.0f32; n];
    let mut moist = vec![0.0f32; n];
    temp.par_chunks_mut(w)
        .zip(moist.par_chunks_mut(w))
        .enumerate()
        .for_each(|(y, (tr, mr))| {
            let lat = ((y as f32 + 0.5) / h as f32 * 2.0 - 1.0).abs();
            // wet equator, dry ~30°, wet ~60°, dry poles
            let band = (lat * 90.0 / 60.0 * TAU).cos();
            for x in 0..w {
                let i = y * w + x;
                let (cx, cy, cz) = cyl(x, y, w);
                let e = height[i].max(0.0) / hmax;
                let ts = 1.0 / (300.0 * k);
                tr[x] = 1.0 - lat.powf(1.3) - 0.6 * e + 0.12 * tn.fbm(cx * ts, cy * ts, cz * ts, 4);
                let coast = (-(dist[i] as f32) / (90.0 * k)).exp();
                let ms = 1.0 / (260.0 * k);
                mr[x] = 0.45 * coast + 0.35 * mn.fbm(cx * ms, cy * ms, cz * ms, 5) + 0.2 * band;
            }
        });

    // elevation and moisture as ranks among land pixels: shares stay stable from seed to seed
    let land: Vec<usize> = (0..n).filter(|&i| cls[i] == LAND).collect();
    let rank = |vals: &[f32]| -> Vec<f32> {
        let mut idx = land.clone();
        idx.par_sort_unstable_by(|&a, &b| vals[a].total_cmp(&vals[b]));
        let mut r = vec![0.0f32; n];
        let m = idx.len().max(1) as f32;
        for (k, &i) in idx.iter().enumerate() {
            r[i] = k as f32 / m;
        }
        r
    };
    let er = rank(height);
    let mr = rank(&moist);
    // ruggedness = height + steepness: ranges, not plateaus, become mountains
    let mut slope = vec![0.0f32; n];
    slope.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let at = |xx: usize, yy: usize| height[yy * w + xx];
            let dx = at((x + 3) % w, y) - at((x + w - 3) % w, y);
            let dy = at(x, (y + 3).min(h - 1)) - at(x, y.saturating_sub(3));
            row[x] = (dx * dx + dy * dy).sqrt();
        }
    });
    let sr = rank(&slope);
    let rugged: Vec<f32> = (0..n).map(|i| er[i] + sr[i]).collect();
    let rr = rank(&rugged);

    (0..n)
        .into_par_iter()
        .map(|i| match cls[i] {
            SEA => Terrain::Ocean,
            LAKE => Terrain::Lake,
            _ => {
                let (t, e, m, r) = (temp[i], er[i], mr[i], rr[i]);
                if t < 0.10 {
                    Terrain::Arctic
                } else if r > 0.93 {
                    Terrain::Mountains
                } else if r > 0.80 {
                    Terrain::Hills
                } else if t < 0.25 {
                    Terrain::Tundra
                } else if m < 0.22 && t > 0.45 {
                    Terrain::Desert
                } else if m > 0.80 && t > 0.68 {
                    Terrain::Jungle
                } else if m > 0.82 && e < 0.25 {
                    Terrain::Marsh
                } else if m > 0.5 {
                    Terrain::Forest
                } else {
                    Terrain::Plains
                }
            }
        })
        .collect()
}

/// Random-order dart throwing: a candidate becomes a seed if no seed is closer than
/// the mean of both exclusion radii. Sparse terrain has a bigger radius.
fn poisson(order: &[usize], terrain: &[Terrain], base_r: f32, w: usize, h: usize) -> Vec<usize> {
    let cs = base_r.max(1.0);
    let gw = (w as f32 / cs).ceil() as usize;
    let gh = (h as f32 / cs).ceil() as usize;
    let mut grid: Vec<Vec<u32>> = vec![Vec::new(); gw * gh];
    let mut seeds: Vec<usize> = Vec::new();
    let mut radii: Vec<f32> = Vec::new();
    let min_density = order
        .iter()
        .map(|&i| terrain[i].density())
        .fold(1.0f32, f32::min)
        .max(0.01);
    let rmax = base_r / min_density.sqrt();
    let half_w = w as f32 * 0.5;
    for &i in order {
        let r = base_r / terrain[i].density().sqrt();
        let (x, y) = ((i % w) as f32, (i / w) as f32);
        let (gx, gy) = ((x / cs) as isize, (y / cs) as isize);
        let reach = ((r + rmax) * 0.5 / cs).ceil() as isize;
        let mut ok = true;
        'search: for dy in -reach..=reach {
            let yy = gy + dy;
            if yy < 0 || yy >= gh as isize {
                continue;
            }
            for dx in -reach..=reach {
                let xx = (gx + dx).rem_euclid(gw as isize) as usize;
                for &s in &grid[yy as usize * gw + xx] {
                    let s = s as usize;
                    let (sx, sy) = ((seeds[s] % w) as f32, (seeds[s] / w) as f32);
                    let mut ddx = (sx - x).abs();
                    if ddx > half_w {
                        ddx = w as f32 - ddx;
                    }
                    let ddy = sy - y;
                    let lim = (r + radii[s]) * 0.5;
                    if ddx * ddx + ddy * ddy < lim * lim {
                        ok = false;
                        break 'search;
                    }
                }
            }
        }
        if ok {
            grid[gy as usize * gw + gx as usize].push(seeds.len() as u32);
            seeds.push(i);
            radii.push(r);
        }
    }
    seeds
}

/// Grows all provinces at once (multi-source Dijkstra). Climbing and changing terrain cost
/// extra, so borders settle on ridges and terrain edges. Land never grows into sea.
fn grow(seeds: &[usize], height: &[f32], terrain: &[Terrain], cls: &[u8], owner: &mut [u32], w: usize, h: usize) {
    let hmax = height.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let hscale = 150.0 / hmax;
    let mut dist = vec![f32::INFINITY; w * h];
    let mut heap = BinaryHeap::new();
    for (k, &s) in seeds.iter().enumerate() {
        dist[s] = 0.0;
        owner[s] = k as u32;
        heap.push(Reverse((0u32, s as u32)));
    }
    while let Some(Reverse((cb, i))) = heap.pop() {
        let (i, c) = (i as usize, f32::from_bits(cb));
        if c > dist[i] {
            continue;
        }
        let (nb, k) = neighbors4(i, w, h);
        for &j in &nb[..k] {
            if cls[j] != cls[i] {
                continue;
            }
            let step = 1.0
                + hscale * (height[j] - height[i]).abs()
                + if terrain[j] != terrain[i] { 2.0 } else { 0.0 };
            let nc = c + step;
            if nc < dist[j] {
                dist[j] = nc;
                owner[j] = owner[i];
                heap.push(Reverse((nc.to_bits(), j as u32)));
            }
        }
    }
}

fn make_provinces(p: &Params, height: &[f32], terrain: &[Terrain], cls: &[u8]) -> (Vec<u32>, Vec<Province>) {
    let (w, h) = (p.w, p.h);
    let n = w * h;
    let mut rng = Rng::new(p.seed ^ 0xA5A5_5A5A);

    // exclusion radius such that the land gets about land_provinces seeds
    // (random sequential packing leaves ~1.44 r^2 of area per seed)
    let wsum: f64 = (0..n)
        .filter(|&i| cls[i] == LAND)
        .map(|i| terrain[i].density() as f64)
        .sum();
    let base_r = (wsum / (1.436 * p.land_provinces.max(1) as f64)).sqrt() as f32;

    let mut land: Vec<usize> = (0..n).filter(|&i| cls[i] == LAND).collect();
    let mut sea: Vec<usize> = (0..n).filter(|&i| cls[i] == SEA).collect();
    shuffle(&mut land, &mut rng);
    shuffle(&mut sea, &mut rng);
    let mut seeds = poisson(&land, terrain, base_r, w, h);
    seeds.extend(poisson(&sea, terrain, base_r, w, h));

    let mut owner = vec![NONE; n];
    grow(&seeds, height, terrain, cls, &mut owner, w, h);

    // a component with no seed (a tiny island) becomes one province
    let mut stack = Vec::new();
    for s in 0..n {
        if owner[s] != NONE || cls[s] == LAKE {
            continue;
        }
        let id = seeds.len() as u32;
        seeds.push(s);
        owner[s] = id;
        stack.push(s);
        while let Some(i) = stack.pop() {
            let (nb, k) = neighbors4(i, w, h);
            for &j in &nb[..k] {
                if owner[j] == NONE && cls[j] == cls[i] {
                    owner[j] = id;
                    stack.push(j);
                }
            }
        }
    }

    let np = seeds.len();
    let mut hist = vec![[0u32; 11]; np];
    for i in 0..n {
        if owner[i] != NONE {
            hist[owner[i] as usize][terrain[i] as usize] += 1;
        }
    }
    let mut provinces: Vec<Province> = (0..np)
        .map(|k| {
            let hh = &hist[k];
            let best = (0..11).max_by_key(|&t| (hh[t], std::cmp::Reverse(t))).unwrap();
            Province {
                seed: seeds[k],
                area: hh.iter().sum(),
                terrain: TERRAINS[best],
                sea: cls[seeds[k]] == SEA,
                neighbors: Vec::new(),
            }
        })
        .collect();

    let mut pairs = HashSet::new();
    for i in 0..n {
        let a = owner[i];
        if a == NONE {
            continue;
        }
        let right = if i % w + 1 == w { i + 1 - w } else { i + 1 };
        for j in [right, i + w] {
            if j >= n {
                continue;
            }
            let b = owner[j];
            if b != NONE && b != a {
                pairs.insert((a.min(b), a.max(b)));
            }
        }
    }
    for (a, b) in pairs {
        provinces[a as usize].neighbors.push(b);
        provinces[b as usize].neighbors.push(a);
    }
    for pr in &mut provinces {
        pr.neighbors.sort_unstable();
    }
    (owner, provinces)
}

#[cfg(test)]
impl World {
    /// Self-checks: (name, passed, detail); `target` — the land provinces asked for.
    pub fn checks(&self, target: usize) -> Vec<(&'static str, bool, String)> {
        let n = self.w * self.h;
        let unassigned = (0..n)
            .filter(|&i| self.owner[i] == NONE && self.terrain[i] != Terrain::Lake)
            .count();

        let (lab, sizes) = label(&self.owner, self.w, self.h);
        let mut parts = vec![0u32; self.provinces.len()];
        let mut seen = vec![false; sizes.len()];
        for i in 0..n {
            let l = lab[i] as usize;
            if !seen[l] {
                seen[l] = true;
                if self.owner[i] != NONE {
                    parts[self.owner[i] as usize] += 1;
                }
            }
        }
        let broken = parts.iter().filter(|&&c| c != 1).count();

        let land = self.provinces.iter().filter(|p| !p.sea).count();
        let lo = target as f32 * 0.75;
        let hi = target as f32 * 1.25;
        let lonely = self
            .provinces
            .iter()
            .filter(|p| !p.sea && p.neighbors.is_empty())
            .count();

        vec![
            ("every pixel has a province", unassigned == 0, format!("{unassigned} without")),
            ("every province is one piece", broken == 0, format!("{broken} broken")),
            (
                "land province count near target",
                (land as f32) >= lo && (land as f32) <= hi,
                format!("{land} vs target {target}"),
            ),
            ("no land province without neighbours", lonely == 0, format!("{lonely} alone")),
        ]
    }

    pub fn fingerprint(&self) -> u64 {
        let mut f: u64 = 0xCBF2_9CE4_8422_2325;
        for (&o, &t) in self.owner.iter().zip(&self.terrain) {
            for b in o.to_le_bytes().into_iter().chain([t as u8]) {
                f ^= b as u64;
                f = f.wrapping_mul(0x100_0000_01B3);
            }
        }
        f
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::Pack;
    use crate::world::countries;

    /// The map and its countries hold together (what the old `map` program checked): every pixel
    /// in a province, every province and state one piece, land provinces near the asked number,
    /// great powers and small states, resources unevenly spread — and the same seed, the same map.
    #[test]
    fn map_and_countries_hold_together() {
        let pack = Pack::load("packs/industrial.toml").unwrap();
        for seed in [1, 2] {
            let p = Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 };
            let world = generate(&p).0;
            for (name, ok, detail) in world.checks(p.land_provinces) {
                assert!(ok, "map {seed}: {name} ({detail})");
            }
            assert_eq!(world.fingerprint(), generate(&p).0.fingerprint(), "map {seed}: not the same twice");
            let pol = countries::generate(&world, seed, 30, &pack);
            for (name, ok, detail) in pol.checks(&world) {
                assert!(ok, "map {seed}: {name} ({detail})");
            }
        }
    }
}
