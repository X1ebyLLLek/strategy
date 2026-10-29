//! Who lives where and who owns it: population, development and resources per province,
//! provinces grouped into states, states into countries of very different size.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet, VecDeque};
use crate::noise::Noise;
use crate::pack::{Pack, Resource};
use crate::rng::{shuffle, Rng};
use crate::world::map::{cyl, Terrain, World, NONE};
use crate::world::names;


#[derive(Clone, Default)]
pub struct ProvInfo {
    pub state: u32,
    /// Millions of people.
    pub pop: f32,
    /// Output per person, relative (developed regions > 1).
    pub dev: f32,
    /// Deposit size per resource of the content pack.
    pub res: Vec<f32>,
    /// How much its land feeds people (ground, water, climate; see `fertile`).
    pub farm: f32,
}

pub struct State {
    pub provinces: Vec<u32>,
    pub neighbors: Vec<u32>,
    pub country: u32,
    pub pop: f32,
    /// Ice nobody lives on: belongs to no one.
    pub waste: bool,
}

pub struct Country {
    pub name: String,
    /// Share of land that feeds people, relative to the world average.
    pub fertility: f32,
    /// Its farmland, in the millions of people it would hold at the world's usual density.
    pub land: f32,
    pub states: Vec<u32>,
    pub capital: u32,
    pub color: [u8; 3],
}

pub struct Politics {
    pub prov: Vec<ProvInfo>,
    pub states: Vec<State>,
    pub countries: Vec<Country>,
    pub n_res: usize,
    /// The world's farmland per area and people per farmland, as the map was made (a country's
    /// fertility and farmland are reckoned against them).
    pub world_fert: f32,
    pub world_density: f32,
}

/// How many people the ground itself feeds, against open plains of the same area (before water,
/// climate and history): forests and hills a third or so, jungles and marshes less, mountains,
/// deserts and tundra next to nobody. (Before: forests 0.6, mountains 0.15, deserts 0.05 — the
/// people lay over the land nearly evenly, as they never do.)
fn habitability(t: Terrain) -> f32 {
    match t {
        Terrain::Plains => 1.0,
        Terrain::Forest => 0.35,
        Terrain::Hills => 0.45,
        Terrain::Jungle => 0.2,
        Terrain::Marsh => 0.15,
        Terrain::Mountains => 0.07,
        Terrain::Desert => 0.02,
        Terrain::Tundra => 0.015,
        _ => 0.0,
    }
}

/// Water gathered (land pixels upstream) at which a river feeds its valley fully; how many times
/// more a valley's land feeds than the land round it; what a valley feeds where the land round it
/// feeds nobody, against plains (the Nile: a river makes a desert a granary).
const BIG_RIVER: f32 = 40_000.0;
const VALLEY: f32 = 2.5;
const FLOODPLAIN: f32 = 0.8;
/// More people on a coast (fish, harbours, trade), and more again where a river meets the sea.
const COAST: f32 = 1.5;
const MOUTH: f32 = 1.3;
/// The old lands of a world: people gather where peoples have lived longest (whole regions dense,
/// others empty), this spread of the log over regions; and from province to province.
const CORES: f32 = 2.0;
const LOCAL: f32 = 0.35;

/// What a province's land feeds against open plains of the same area, all told (each pixel of it
/// one): its ground, the river through it (warm lands only — frozen rivers feed nobody), its
/// coast, its climate (the cold north feeds less). Its farmland, and the bed its people grow from.
/// `water`: the most water any of its pixels gathers.
fn fertile(world: &World, p: usize, water: f32) -> f32 {
    let pr = &world.provinces[p];
    if pr.sea {
        return 0.0;
    }
    let lat = ((pr.seed / world.w) as f32 / world.h as f32 - 0.5).abs() * 180.0;
    let warm = (1.0 - (lat - 45.0) / 25.0).clamp(0.0, 1.0);
    let cold = (1.0 - 0.7 * (lat - 50.0) / 15.0).clamp(0.3, 1.0);
    let river = ((1.0 + water / 450.0).ln() / (1.0 + BIG_RIVER / 450.0).ln()).clamp(0.0, 1.3);
    let coastal = pr.neighbors.iter().any(|&q| world.provinces[q as usize].sea);
    let shore = if coastal { COAST * if river > 0.3 { MOUTH } else { 1.0 } } else { 1.0 };
    let ground = habitability(pr.terrain) * cold;
    let valley = river * warm;
    pr.area as f32 * (ground * (1.0 + VALLEY * valley) + FLOODPLAIN * valley * (1.0 - ground).max(0.0)) * shore
}

fn land_neighbors(world: &World, p: usize) -> impl Iterator<Item = usize> + '_ {
    world.provinces[p]
        .neighbors
        .iter()
        .map(|&q| q as usize)
        .filter(move |&q| !world.provinces[q].sea)
}

fn color(rng: &mut Rng) -> [u8; 3] {
    // pleasant muted colours: random hue, fixed-ish saturation and value
    let h = rng.f32() * 6.0;
    let (s, v) = (0.35 + 0.25 * rng.f32(), 0.65 + 0.25 * rng.f32());
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8]
}

/// The pack gives the era: what lies in the ground, how many people the world holds.
pub fn generate(world: &World, seed: u64, n_countries: usize, pack: &Pack) -> Politics {
    let resources = &pack.resources;
    let mut rng = Rng::new(seed ^ 0x9071_7105);
    // the most water each province's pixels gather (its biggest river)
    let mut water = vec![0.0f32; world.provinces.len()];
    for (i, _, f) in crate::world::render::rivers(world) {
        let o = world.owner[i];
        if o != NONE {
            water[o as usize] = water[o as usize].max(f);
        }
    }
    let fert: Vec<f32> = (0..world.provinces.len()).map(|p| fertile(world, p, water[p])).collect();
    let mut prov = economy(world, seed, resources, pack.people, &fert, &mut rng);
    let mut states = make_states(world, &mut prov, &mut rng);
    let mut countries = make_countries(world, &prov, &mut states, n_countries, &mut rng);

    // fertility: how much of the land feeds people, relative to the world average
    let farm = |p: usize| fert[p];
    let area = |p: usize| world.provinces[p].area as f32;
    let owned: Vec<usize> = countries.iter().flat_map(|c| c.states.iter()).flat_map(|&s| states[s as usize].provinces.iter().map(|&p| p as usize)).collect();
    let world_fert = owned.iter().map(|&p| farm(p)).sum::<f32>() / owned.iter().map(|&p| area(p)).sum::<f32>().max(1e-6);
    let world_density = owned.iter().map(|&p| prov[p].pop).sum::<f32>() / owned.iter().map(|&p| farm(p)).sum::<f32>().max(1e-6);
    for c in &mut countries {
        let ps: Vec<usize> = c.states.iter().flat_map(|&s| states[s as usize].provinces.iter().map(|&p| p as usize)).collect();
        let f = ps.iter().map(|&p| farm(p)).sum::<f32>() / ps.iter().map(|&p| area(p)).sum::<f32>().max(1e-6);
        c.fertility = f / world_fert.max(1e-6);
        c.land = ps.iter().map(|&p| farm(p)).sum::<f32>() * world_density;
    }
    Politics { prov, states, countries, n_res: resources.len(), world_fert, world_density }
}

impl Politics {
    /// Each country's fertility and farmland reckoned anew from the provinces it holds now (after
    /// the world's past moved borders: the land goes with its regions).
    pub fn refit_land(&mut self, world: &World) {
        for c in 0..self.countries.len() {
            let ps: Vec<usize> = self.provinces_of(c).collect();
            if ps.is_empty() {
                continue;
            }
            let farm: f32 = ps.iter().map(|&p| self.prov[p].farm).sum();
            let area: f32 = ps.iter().map(|&p| world.provinces[p].area as f32).sum();
            self.countries[c].fertility = farm / area.max(1e-6) / self.world_fert.max(1e-6);
            self.countries[c].land = farm * self.world_density;
        }
    }
}

/// Each province's people, development and deposits. The people: as many as the world of the era
/// holds (`people`, millions, the least and the most: this world draws its own), laid where the land
/// feeds them (`fert`: ground, rivers, coasts, climate) and where peoples have lived longest (old
/// lands spread over whole regions), each province by its luck too.
fn economy(world: &World, seed: u64, resources: &[Resource], people: [f32; 2], fert: &[f32], rng: &mut Rng) -> Vec<ProvInfo> {
    let w = world.w;
    let k = w as f32 / 2048.0;
    let s = seed.wrapping_mul(7919);
    let devn = Noise::new(s + 1);
    let coren = Noise::new(s + 0xC0E);
    let resn: Vec<Noise> = (0..resources.len()).map(|r| Noise::new(s + 2 + r as u64)).collect();
    let mut out = vec![ProvInfo { state: NONE, ..Default::default() }; world.provinces.len()];
    for (p, pr) in world.provinces.iter().enumerate() {
        if pr.sea {
            continue;
        }
        let (cx, cy, cz) = cyl(pr.seed % w, pr.seed / w, w);
        let field = |n: &Noise, scale: f32| n.fbm(cx / (scale * k), cy / (scale * k), cz / (scale * k), 3);
        // developed and backward regions spread over whole continents
        let dev = (2.0 * field(&devn, 450.0)).exp();
        let old = (CORES * 2.0 * field(&coren, 350.0)).exp();
        let pop = fert[p] * old * (LOCAL * rng.normal()).exp() * dev.powf(0.2);
        let mut res = vec![0.0; resources.len()];
        for r in 0..resources.len() {
            // deposits cluster in basins
            let basin = (1.0 + 4.0 * field(&resn[r], 300.0)).max(0.0);
            if rng.f32() < resources[r].chance[pr.terrain as usize] * basin {
                res[r] = (0.7 * rng.normal()).exp() * (pr.area as f32 / 175.0).sqrt();
            }
        }
        out[p] = ProvInfo { state: NONE, pop, dev, res, farm: fert[p] };
    }
    // (how many this world holds: its own draw between the era's least and most)
    let want = people[0] + (people[1] - people[0]).max(0.0) * Rng::new(seed ^ 0x9E0B1E).f32();
    let total: f32 = out.iter().map(|i| i.pop).sum::<f32>().max(1e-6);
    for i in &mut out {
        i.pop *= want / total;
    }
    out
}

/// States: random seeds at least 3 steps apart, grown over the province graph.
/// Changing terrain and climbing into mountains cost more, so states follow natural regions.
fn make_states(world: &World, prov: &mut [ProvInfo], rng: &mut Rng) -> Vec<State> {
    let np = world.provinces.len();
    let mut land: Vec<usize> = (0..np).filter(|&p| !world.provinces[p].sea).collect();
    shuffle(&mut land, rng);
    let mut blocked = vec![false; np];
    let mut seeds = Vec::new();
    for &p in &land {
        if blocked[p] {
            continue;
        }
        seeds.push(p);
        blocked[p] = true;
        for q in land_neighbors(world, p) {
            blocked[q] = true;
            for r in land_neighbors(world, q) {
                blocked[r] = true;
            }
        }
    }

    let mut owner = vec![NONE; np];
    let mut dist = vec![f32::INFINITY; np];
    let mut heap = BinaryHeap::new();
    for (k, &s) in seeds.iter().enumerate() {
        owner[s] = k as u32;
        dist[s] = 0.0;
        heap.push(Reverse((0u32, s as u32)));
    }
    while let Some(Reverse((cb, p))) = heap.pop() {
        let (p, c) = (p as usize, f32::from_bits(cb));
        if c > dist[p] {
            continue;
        }
        for q in land_neighbors(world, p) {
            let (tp, tq) = (world.provinces[p].terrain, world.provinces[q].terrain);
            let step = 1.0
                + if tq != tp { 2.0 } else { 0.0 }
                + if tq == Terrain::Mountains { 2.0 } else { 0.0 };
            let nc = c + step;
            if nc < dist[q] {
                dist[q] = nc;
                owner[q] = owner[p];
                heap.push(Reverse((nc.to_bits(), q as u32)));
            }
        }
    }

    let mut states: Vec<State> = (0..seeds.len())
        .map(|_| State { provinces: Vec::new(), neighbors: Vec::new(), country: NONE, pop: 0.0, waste: false })
        .collect();
    for &p in &land {
        let s = owner[p];
        prov[p].state = s;
        states[s as usize].provinces.push(p as u32);
        states[s as usize].pop += prov[p].pop;
    }
    let mut pairs = HashSet::new();
    for &p in &land {
        for q in land_neighbors(world, p) {
            let (a, b) = (owner[p], owner[q]);
            if a != b {
                pairs.insert((a.min(b), a.max(b)));
            }
        }
    }
    for (a, b) in pairs {
        states[a as usize].neighbors.push(b);
        states[b as usize].neighbors.push(a);
    }
    for st in &mut states {
        st.provinces.sort_unstable();
        st.neighbors.sort_unstable();
        let ice: u32 = st
            .provinces
            .iter()
            .filter(|&&p| world.provinces[p as usize].terrain == Terrain::Arctic)
            .map(|&p| world.provinces[p as usize].area)
            .sum();
        let all: u32 = st.provinces.iter().map(|&p| world.provinces[p as usize].area).sum();
        st.waste = ice * 2 > all;
    }
    states
}

/// How many countries a world of the game's size holds (the checks of the core keep their own, on a
/// smaller map). The real world has a couple of hundred, and the world before the machines hundreds more:
/// a country of a few provinces is the usual, a country of a hundred the rare.
pub const COUNTRIES: usize = 72;

/// Countries: seeds spread over landmasses by their people, each with its own vigour; they grow out
/// over the province graph by what the land asks for (33rd session; before, 30 countries grew as even
/// patches, ten of them held three quarters of the land, and the median country a fiftieth):
/// - **settled land resists, empty land does not**: a dense valley is taken slowly, a steppe, a desert
///   or a tundra at once — so the core lands break into many small countries, and the great ones by land
///   are the thin ones (Russia, Canada, the Sahara's states), as in the world;
/// - **barriers**: mountains hold a country back (Switzerland, the Caucasus, the Balkans: a cradle
///   there is likelier too), open plains let it run on (Rus, the Chinese plain);
/// - **a country that has grown tires**: its growth costs more the more it holds against what its vigour
///   would carry, so no one swallows a continent for being lucky with its seed;
/// - **islands** with some people of their own are countries of their own, small islands go to the
///   nearest.
/// (Seeded by land alone, a third of the countries were deserts and tundras of a few hundred
/// thousand people.)
fn make_countries(world: &World, prov: &[ProvInfo], states: &mut [State], n_countries: usize, rng: &mut Rng) -> Vec<Country> {
    let ns = states.len();
    let live: Vec<usize> = (0..ns).filter(|&s| !states[s].waste).collect();
    if live.is_empty() {
        return Vec::new();
    }

    // landmasses over inhabited states
    let mut mass = vec![NONE; ns];
    let mut mass_size = Vec::new();
    for &s0 in &live {
        if mass[s0] != NONE {
            continue;
        }
        let id = mass_size.len() as u32;
        let mut q = VecDeque::from([s0]);
        mass[s0] = id;
        let mut n = 0;
        while let Some(s) = q.pop_front() {
            n += 1;
            for &t in &states[s].neighbors {
                let t = t as usize;
                if mass[t] == NONE && !states[t].waste {
                    mass[t] = id;
                    q.push_back(t);
                }
            }
        }
        mass_size.push(n);
    }
    // (each landmass as many countries as its share of the people: one at least where people live)
    let mut mass_pop = vec![0.0f32; mass_size.len()];
    for &s in &live {
        mass_pop[mass[s] as usize] += states[s].pop;
    }
    let all_pop: f32 = mass_pop.iter().sum::<f32>().max(1e-6);
    let per_country = all_pop / n_countries.max(1) as f32;
    let mut quota: Vec<usize> = (0..mass_size.len())
        .map(|m| if mass_pop[m] >= 0.4 * per_country { ((mass_pop[m] / per_country).round() as usize).max(1).min(mass_size[m]) } else { 0 })
        .collect();

    // what each state is like: the share of its land in mountains, in open plains, on the sea; its weight (people, and
    // a little for the land itself: a country of empty land still has something to be tired of)
    let share = |s: usize, f: &dyn Fn(Terrain) -> bool| {
        let st = &states[s];
        let all: f32 = st.provinces.iter().map(|&p| world.provinces[p as usize].area as f32).sum::<f32>().max(1.0);
        st.provinces.iter().filter(|&&p| f(world.provinces[p as usize].terrain)).map(|&p| world.provinces[p as usize].area as f32).sum::<f32>() / all
    };
    let mountains: Vec<f32> = (0..ns).map(|s| share(s, &|t| t == Terrain::Mountains)).collect();
    let rugged: Vec<f32> = (0..ns).map(|s| mountains[s] + 0.4 * share(s, &|t| t == Terrain::Hills)).collect();
    let open: Vec<f32> = (0..ns).map(|s| share(s, &|t| t == Terrain::Plains)).collect();
    let coast: Vec<bool> = (0..ns).map(|s| states[s].provinces.iter().any(|&p| world.provinces[p as usize].neighbors.iter().any(|&q| world.provinces[q as usize].sea))).collect();
    let mean_pop = live.iter().map(|&s| states[s].pop).sum::<f32>() / live.len() as f32;
    let area_of = |s: usize| states[s].provinces.iter().map(|&p| world.provinces[p as usize].area as f32).sum::<f32>();
    let mean_area = live.iter().map(|&s| area_of(s)).sum::<f32>() / live.len() as f32;
    let popw: Vec<f32> = (0..ns).map(|s| states[s].pop / mean_pop.max(1e-6)).collect();
    let weight: Vec<f32> = (0..ns).map(|s| states[s].pop + 0.12 * mean_pop * area_of(s) / mean_area.max(1.0)).collect();
    let total_weight: f32 = live.iter().map(|&s| weight[s]).sum();

    // (a state's chance to be a country's cradle by its people, likelier in the highlands and on the coast: the
    // order of a weighted draw)
    let mut keyed: Vec<(f32, usize)> = live
        .iter()
        .map(|&s| {
            let w = (states[s].pop.max(0.0) + 1e-3 * per_country).powf(0.8) * (1.0 + 1.2 * rugged[s]) * if coast[s] { 1.3 } else { 1.0 };
            (rng.f32().max(1e-12).ln() / w, s)
        })
        .collect();
    keyed.sort_by(|a, b| b.0.total_cmp(&a.0));
    let order: Vec<usize> = keyed.into_iter().map(|x| x.1).collect();
    let mut blocked = vec![false; ns];
    let mut seeds = Vec::new();
    for &s in &order {
        let m = mass[s] as usize;
        if quota[m] == 0 || blocked[s] {
            continue;
        }
        quota[m] -= 1;
        seeds.push(s);
        blocked[s] = true;
        for &t in &states[s].neighbors {
            blocked[t as usize] = true;
        }
    }
    // (the vigour: a heavy tail — a few great, many small; a cradle in a populous place has more of it)
    let vigour: Vec<f32> = seeds.iter().map(|&s| ((0.85 * rng.normal()).exp() * (0.7 + 0.3 * popw[s].clamp(0.3, 3.0).sqrt())).clamp(0.35, 3.0)).collect();
    // (what a country can hold before it tires: by its vigour, against the world's weight spread over the countries)
    let cap: Vec<f32> = vigour.iter().map(|&v| 2.0 * total_weight / seeds.len().max(1) as f32 * v.powf(1.5)).collect();

    let mut owner = vec![NONE; ns];
    let mut dist = vec![f32::INFINITY; ns];
    let mut held = vec![0.0f32; seeds.len()];
    let mut done = vec![false; ns];
    let mut heap = BinaryHeap::new();
    for (c, &s) in seeds.iter().enumerate() {
        owner[s] = c as u32;
        dist[s] = 0.0;
        heap.push(Reverse((0u32, s as u32)));
    }
    while let Some(Reverse((cb, s))) = heap.pop() {
        let (s, d) = (s as usize, f32::from_bits(cb));
        if d > dist[s] || done[s] {
            continue;
        }
        done[s] = true;
        let c = owner[s] as usize;
        held[c] += weight[s];
        let tired = 1.0 + (held[c] / cap[c]).powi(2);
        for &t in &states[s].neighbors {
            let t = t as usize;
            if states[t].waste || done[t] {
                continue;
            }
            let resist = 0.45 + 0.9 * popw[t].min(6.0).sqrt();
            let barrier = 1.0 + 2.5 * mountains[t] + 0.8 * rugged[t];
            let run = 1.0 - 0.25 * open[t];
            let nd = d + resist * barrier * run * tired / vigour[c];
            if nd < dist[t] {
                dist[t] = nd;
                owner[t] = c as u32;
                heap.push(Reverse((nd.to_bits(), t as u32)));
            }
        }
    }

    // small islands go to the nearest country: overseas possessions
    let w = world.w;
    let px = |s: usize| {
        let p = world.provinces[states[s].provinces[0] as usize].seed;
        ((p % w) as f32, (p / w) as f32)
    };
    for &s in &live {
        if owner[s] != NONE {
            continue;
        }
        let (x, y) = px(s);
        let best = (0..seeds.len()).min_by(|&a, &b| {
            let d = |c: usize| {
                let (cx, cy) = px(seeds[c]);
                let dx = (cx - x).abs().min(w as f32 - (cx - x).abs());
                dx * dx + (cy - y) * (cy - y)
            };
            d(a).total_cmp(&d(b))
        });
        if let Some(c) = best {
            owner[s] = c as u32;
        }
    }

    // neighbours get clearly different colours: best of a few random candidates
    let nc = seeds.len();
    let mut adj = vec![HashSet::new(); nc];
    for &s in &live {
        for &t in &states[s].neighbors {
            let (a, b) = (owner[s], owner[t as usize]);
            if a != NONE && b != NONE && a != b {
                adj[a as usize].insert(b as usize);
            }
        }
    }
    let mut colors: Vec<Option<[u8; 3]>> = vec![None; nc];
    for c in 0..nc {
        let mut best = ([0u8; 3], -1.0f32);
        for _ in 0..16 {
            let cand = color(rng);
            let gap = adj[c]
                .iter()
                .filter_map(|&o| colors[o])
                .map(|o| (0..3).map(|k| (cand[k] as f32 - o[k] as f32).powi(2)).sum::<f32>())
                .fold(f32::INFINITY, f32::min);
            if gap > best.1 {
                best = (cand, gap);
            }
        }
        colors[c] = Some(best.0);
    }
    let names = names::countries(rng, nc);
    let mut countries: Vec<Country> = names
        .into_iter()
        .zip(colors)
        .map(|(name, color)| Country { name, fertility: 1.0, land: 0.0, states: Vec::new(), capital: NONE, color: color.unwrap() })
        .collect();
    for &s in &live {
        if owner[s] != NONE {
            states[s].country = owner[s];
            countries[owner[s] as usize].states.push(s as u32);
        }
    }
    for c in &mut countries {
        let big = c.states.iter().max_by(|&&a, &&b| states[a as usize].pop.total_cmp(&states[b as usize].pop));
        if let Some(&s) = big {
            let cap = states[s as usize]
                .provinces
                .iter()
                .max_by(|&&a, &&b| prov[a as usize].pop.total_cmp(&prov[b as usize].pop));
            c.capital = *cap.unwrap();
        }
    }
    countries
}

impl Politics {
    pub fn pop(&self, c: usize) -> f32 {
        self.countries[c].states.iter().map(|&s| self.states[s as usize].pop).sum()
    }

    /// Economic output: people times productivity.
    pub fn gdp(&self, c: usize) -> f32 {
        self.provinces_of(c).map(|p| self.prov[p].pop * self.prov[p].dev).sum()
    }

    pub fn res(&self, c: usize) -> Vec<f32> {
        let mut r = vec![0.0; self.n_res];
        for p in self.provinces_of(c) {
            for (k, v) in r.iter_mut().enumerate() {
                *v += self.prov[p].res[k];
            }
        }
        r
    }

    /// The number of provinces of each state of a country (its size by land).
    pub fn provinces_of_n<'a>(&'a self, c: &'a Country) -> impl Iterator<Item = usize> + 'a {
        c.states.iter().map(move |&s| self.states[s as usize].provinces.len())
    }

    pub fn provinces_of(&self, c: usize) -> impl Iterator<Item = usize> + '_ {
        self.countries[c]
            .states
            .iter()
            .flat_map(move |&s| self.states[s as usize].provinces.iter().map(|&p| p as usize))
    }

    /// Countries sharing a land border.
    pub fn neighbors(&self, c: usize) -> Vec<usize> {
        let mut out: Vec<usize> = self.countries[c]
            .states
            .iter()
            .flat_map(|&s| self.states[s as usize].neighbors.iter())
            .map(|&t| self.states[t as usize].country)
            .filter(|&o| o != NONE && o as usize != c)
            .map(|o| o as usize)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Population-weighted development.
    pub fn dev(&self, c: usize) -> f32 {
        self.gdp(c) / self.pop(c).max(1e-6)
    }

    /// Province -> country, NONE for sea, ice and unowned land.
    pub fn country_of(&self, p: usize) -> u32 {
        let s = self.prov[p].state;
        if s == NONE {
            NONE
        } else {
            self.states[s as usize].country
        }
    }

    #[cfg(test)]
    pub fn checks(&self, world: &World) -> Vec<(&'static str, bool, String)> {
        let land: Vec<usize> = (0..world.provinces.len()).filter(|&p| !world.provinces[p].sea).collect();
        let stateless = land.iter().filter(|&&p| self.prov[p].state == NONE).count();

        // every state is one piece of land
        let mut broken = 0;
        for st in &self.states {
            let set: HashSet<usize> = st.provinces.iter().map(|&p| p as usize).collect();
            let mut seen = HashSet::from([st.provinces[0] as usize]);
            let mut q = vec![st.provinces[0] as usize];
            while let Some(p) = q.pop() {
                for r in land_neighbors(world, p) {
                    if set.contains(&r) && seen.insert(r) {
                        q.push(r);
                    }
                }
            }
            if seen.len() != set.len() {
                broken += 1;
            }
        }
        let ownerless = self.states.iter().filter(|s| !s.waste && s.country == NONE).count();

        let mut pops: Vec<f32> = (0..self.countries.len()).map(|c| self.pop(c)).collect();
        pops.sort_by(|a, b| b.total_cmp(a));
        let ratio = pops.first().unwrap_or(&0.0) / pops.get(pops.len() / 2).unwrap_or(&1.0).max(1e-6);
        let nc = self.countries.len();
        let without: Vec<usize> = (0..self.n_res)
            .map(|r| (0..nc).filter(|&c| self.res(c)[r] == 0.0).count())
            .collect();

        vec![
            ("every land province is in a state", stateless == 0, format!("{stateless} without")),
            ("every state is one piece", broken == 0, format!("{broken} broken")),
            ("every inhabited state has an owner", ownerless == 0, format!("{ownerless} ownerless")),
            ("great powers and small states", ratio >= 4.0, format!("largest / median population = {ratio:.1}")),
            (
                "every resource: some have it, some not",
                without.iter().all(|&k| k > 0 && k < nc),
                format!("countries without each resource: {without:?} of {nc}"),
            ),
        ]
    }
}
