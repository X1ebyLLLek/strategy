//! The seas of the map in zones (as the admiralties divide them): the zones, their coasts and the
//! countries whose ports stand on them; and the ways of trade from a country's capital to every
//! other — the same cheapest ways merchants take (`trade::routes`), with the zones of the sea they
//! cross, so that a fleet in a zone knows whose trade passes it.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet, VecDeque};

use strategy::pack::Trade;
use strategy::rng::{hash64, Rng};
use strategy::world::map;
use strategy::world::names;

/// Sea provinces to a zone, about.
const ZONE_SIZE: usize = 40;

/// A zone of the sea: its name, its sea provinces, the province at its middle (for its name on the
/// map), the land provinces on its coast, the zones next to it.
#[derive(Clone, Debug)]
pub struct Zone {
    pub name: String,
    pub provs: Vec<usize>,
    pub centre: usize,
    pub coast: Vec<usize>,
    pub near: Vec<usize>,
}

/// The seas of the map: each sea province's zone (`u32::MAX`: land), the zones.
#[derive(Clone, Debug, Default)]
pub struct Seas {
    pub zone_of: Vec<u32>,
    pub zones: Vec<Zone>,
}

impl Seas {
    /// The map's seas cut into zones of about `ZONE_SIZE` provinces: seeds as far from each other
    /// as the sea allows (each next one the farthest by steps from those already chosen), each sea
    /// province to its nearest seed.
    pub fn of(world: &map::World, seed: u64) -> Seas {
        let np = world.provinces.len();
        let sea: Vec<usize> = (0..np).filter(|&p| world.provinces[p].sea).collect();
        let mut zone_of = vec![u32::MAX; np];
        if sea.is_empty() {
            return Seas { zone_of, zones: Vec::new() };
        }
        let wet = |p: usize| world.provinces[p].sea;
        let k = (sea.len() / ZONE_SIZE).clamp(4, 120);
        // steps from the seeds chosen so far
        let mut dist = vec![u32::MAX; np];
        let mut seeds: Vec<usize> = Vec::new();
        let spread = |from: usize, dist: &mut Vec<u32>| {
            let mut q = VecDeque::new();
            dist[from] = 0;
            q.push_back(from);
            while let Some(p) = q.pop_front() {
                for &n in &world.provinces[p].neighbors {
                    let n = n as usize;
                    if wet(n) && dist[n] > dist[p] + 1 {
                        dist[n] = dist[p] + 1;
                        q.push_back(n);
                    }
                }
            }
        };
        while seeds.len() < k {
            // (an unreached sea — another ocean, an inland sea — gets a seed of its own first)
            let next = sea.iter().copied().max_by_key(|&p| (dist[p], Reverse(p))).unwrap();
            if dist[next] == 0 {
                break;
            }
            seeds.push(next);
            spread(next, &mut dist);
        }
        // each sea province to its nearest seed (by steps; ties to the first seed)
        let mut q = VecDeque::new();
        let mut d = vec![u32::MAX; np];
        for (z, &s) in seeds.iter().enumerate() {
            zone_of[s] = z as u32;
            d[s] = 0;
            q.push_back(s);
        }
        while let Some(p) = q.pop_front() {
            for &n in &world.provinces[p].neighbors {
                let n = n as usize;
                if wet(n) && d[n] == u32::MAX {
                    d[n] = d[p] + 1;
                    zone_of[n] = zone_of[p];
                    q.push_back(n);
                }
            }
        }
        let mut rng = Rng::new(hash64(seed ^ 0x5EA5));
        let mut taken = HashSet::new();
        let mut zones: Vec<Zone> = seeds
            .iter()
            .map(|&s| Zone { name: String::new(), provs: Vec::new(), centre: s, coast: Vec::new(), near: Vec::new() })
            .collect();
        for &p in &sea {
            if zone_of[p] != u32::MAX {
                zones[zone_of[p] as usize].provs.push(p);
            }
        }
        for p in 0..np {
            if wet(p) {
                continue;
            }
            for &n in &world.provinces[p].neighbors {
                let z = zone_of[n as usize];
                if z != u32::MAX && !zones[z as usize].coast.contains(&p) {
                    zones[z as usize].coast.push(p);
                }
            }
        }
        for &p in &sea {
            let a = zone_of[p];
            if a == u32::MAX {
                continue;
            }
            for &n in &world.provinces[p].neighbors {
                let b = zone_of[n as usize];
                if b != u32::MAX && b != a && !zones[a as usize].near.contains(&(b as usize)) {
                    zones[a as usize].near.push(b as usize);
                }
            }
        }
        for z in zones.iter_mut() {
            let place = names::place(&mut rng, &mut taken);
            z.name = if z.coast.is_empty() { format!("океан {place}") } else if z.coast.len() * 3 > z.provs.len() * 2 { format!("залив {place}") } else { format!("море {place}") };
        }
        Seas { zone_of, zones }
    }

    /// The zones on whose coasts the provinces `land` stand (a country's ports).
    pub fn zones_by(&self, world: &map::World, land: &[usize]) -> Vec<usize> {
        let mut v: Vec<usize> = Vec::new();
        for &p in land {
            for &n in &world.provinces[p].neighbors {
                let z = self.zone_of[n as usize];
                if z != u32::MAX && !v.contains(&(z as usize)) {
                    v.push(z as usize);
                }
            }
        }
        v.sort();
        v
    }
}

/// A way of trade from a country's capital to country `to`'s: the zones of the sea it crosses (in
/// order), the share of its losses that is by sea (0: all by land), its provinces (for the map).
#[derive(Clone, Debug, Default)]
pub struct SeaWay {
    pub to: usize,
    pub zones: Vec<usize>,
    pub sea: f32,
    pub path: Vec<usize>,
}

/// The ways of trade from the capital `from` to every capital of `capitals` (`usize::MAX`: none), as
/// merchants go — the cheapest by loss, with the costs of `trade::routes`.
pub fn ways(world: &map::World, seas: &Seas, from: usize, capitals: &[usize], t: &Trade) -> Vec<SeaWay> {
    let np = world.provinces.len();
    let w = world.w as f32;
    let at = |p: usize| {
        let s = world.provinces[p].seed;
        ((s % world.w) as f32, (s / world.w) as f32)
    };
    let dist = |a: usize, b: usize| {
        let ((xa, ya), (xb, yb)) = (at(a), at(b));
        let dx = (xa - xb).abs();
        let dx = dx.min(w - dx);
        (dx * dx + (ya - yb) * (ya - yb)).sqrt()
    };
    let rough = |p: usize| t.terrain[world.provinces[p].terrain as usize];
    // (loss of one step between neighbouring provinces, and the part of it that is by sea)
    let edge = |p: usize, q: usize| {
        let d = dist(p, q);
        let (sp, sq) = (world.provinces[p].sea, world.provinces[q].sea);
        if sp && sq {
            (d * t.sea, d * t.sea)
        } else if !sp && !sq {
            (d * t.land * 0.5 * (rough(p) + rough(q)), 0.0)
        } else {
            let land = if sp { q } else { p };
            (0.5 * d * (t.sea + t.land * rough(land)) + t.port, 0.5 * d * t.sea + t.port)
        }
    };
    let mut loss = vec![f32::INFINITY; np];
    let mut wet = vec![0.0f32; np];
    let mut back = vec![usize::MAX; np];
    let mut heap = BinaryHeap::new();
    if from >= np {
        return Vec::new();
    }
    loss[from] = 0.0;
    heap.push(Reverse((0u32, from)));
    while let Some(Reverse((bits, p))) = heap.pop() {
        if f32::from_bits(bits) > loss[p] {
            continue;
        }
        for &q in &world.provinces[p].neighbors {
            let q = q as usize;
            let (dl, ds) = edge(p, q);
            let l = loss[p] + dl;
            if l < loss[q] {
                loss[q] = l;
                wet[q] = wet[p] + ds;
                back[q] = p;
                heap.push(Reverse((l.to_bits(), q)));
            }
        }
    }
    capitals
        .iter()
        .enumerate()
        .map(|(to, &dst)| {
            if dst >= np || dst == from || !loss[dst].is_finite() {
                return SeaWay { to, ..SeaWay::default() };
            }
            let mut path = vec![dst];
            let mut p = dst;
            while back[p] != usize::MAX {
                p = back[p];
                path.push(p);
            }
            path.reverse();
            let mut zones: Vec<usize> = Vec::new();
            for &p in &path {
                let z = seas.zone_of[p];
                if z != u32::MAX && zones.last() != Some(&(z as usize)) && !zones.contains(&(z as usize)) {
                    zones.push(z as usize);
                }
            }
            SeaWay { to, zones, sea: if loss[dst] > 0.0 { (wet[dst] / loss[dst]).clamp(0.0, 1.0) } else { 0.0 }, path }
        })
        .collect()
}
