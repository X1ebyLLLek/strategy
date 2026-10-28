//! Division templates: what battalions a division is made of (a battalion is a tenth of a unit of
//! the pack — about a thousand men of infantry). The player who decides what the army builds
//! does it by templates: each kind's share of the army's money is the battalions of that kind in
//! the templates he builds, weighed by how much of each he wants and by what the kind costs his
//! country — the same `plan` the staff sets when it decides (the core is the same either way).
//! When the staff decides, its template is its plan put into battalions. In battle the kinds fight
//! as before; a template is how the army is ordered and how the map counts divisions.

use strategy::military::doctrine::Country;
use strategy::pack::{Line, Pack};

/// A battalion: this share of a unit of the pack.
pub const BATTALION: f32 = 0.1;
/// Battalions a division holds at most (five columns of five, as in HOI4).
pub const MOST: u32 = 25;
/// How much of a template is built: the words of the slider and the weights they stand for.
pub const WANT: [(&str, f32); 5] = [("не строить", 0.0), ("понемногу", 0.5), ("наравне", 1.0), ("больше", 2.0), ("главное", 4.0)];

#[derive(Clone, Debug)]
pub struct Template {
    pub name: String,
    /// Battalions of each kind of the pack.
    pub bat: Vec<u32>,
    /// How much of it is built (a weight, one of `WANT`).
    pub want: f32,
    /// Norm of equipment of each kind (`doctrine::norm::LEVELS`; empty: standard) — only the
    /// kinds with norms (`Unit::norms`) take one.
    pub norm: Vec<f32>,
    /// Its companies of support (indices into the pack's `support`).
    pub support: Vec<usize>,
}

impl Template {
    pub fn new(name: &str, bat: Vec<u32>, want: f32) -> Template {
        Template { name: name.into(), bat, want, norm: Vec::new(), support: Vec::new() }
    }

    pub fn battalions(&self) -> u32 {
        self.bat.iter().sum()
    }

    /// The norm of kind `u`.
    pub fn norm_of(&self, u: usize) -> f32 {
        self.norm.get(u).copied().unwrap_or(1.0)
    }

    /// The template as the war knows it.
    pub fn shape(&self) -> strategy::military::corps::Shape {
        let mut sh = strategy::military::corps::Shape::new(&self.name, self.units());
        sh.norm = self.norm.clone();
        sh.support = self.support.clone();
        sh
    }

    /// Units of each kind in one division.
    pub fn units(&self) -> Vec<f32> {
        self.bat.iter().map(|&b| b as f32 * BATTALION).collect()
    }
}

/// The templates a country starts with: a line division of the era's plainest front-line troops
/// (the cheapest place in the line) with the support that fires hardest at men for its cost, and
/// a heavy one led by the toughest front-line kind.
pub fn starters(pack: &Pack) -> Vec<Template> {
    let n = pack.units.len();
    let u = &pack.units;
    let front: Vec<usize> = (0..n).filter(|&k| u[k].line == Line::Front).collect();
    let plain = front.iter().copied().min_by(|&a, &b| (u[a].cost / u[a].width.max(0.1)).total_cmp(&(u[b].cost / u[b].width.max(0.1))));
    let heavy = front.iter().copied().filter(|&k| Some(k) != plain).max_by(|&a, &b| u[a].toughness.total_cmp(&u[b].toughness));
    let support = (0..n).filter(|&k| u[k].line == Line::Support).max_by(|&a, &b| (u[a].fire[0] / u[a].cost).total_cmp(&(u[b].fire[0] / u[b].cost)));
    let mut out = Vec::new();
    if let Some(p) = plain {
        let mut bat = vec![0u32; n];
        bat[p] = 9;
        if let Some(s) = support {
            bat[s] = 2;
        }
        out.push(Template::new("Линейная дивизия", bat, 1.0));
        if let Some(h) = heavy {
            let mut bat = vec![0u32; n];
            bat[h] = 5;
            bat[p] = 4;
            if let Some(s) = support {
                bat[s] = 1;
            }
            out.push(Template::new("Ударная дивизия", bat, 0.5));
        }
    }
    out
}

/// The army's plan from the templates: each kind's share of the money.
pub fn plan_of(templates: &[Template], c: &Country) -> Option<Vec<f32>> {
    let n = c.cost.len();
    let mut money = vec![0.0f32; n];
    for t in templates {
        // (each kind at what it costs with the kit the template gives it)
        let cost: Vec<f32> = (0..n).map(|u| c.cost_at(u, t.norm_of(u))).collect();
        let worth: f32 = t.units().iter().zip(&cost).map(|(u, k)| u * k).sum();
        if worth <= 0.0 || t.want <= 0.0 {
            continue;
        }
        // as many divisions of it as `want` says, the same money for each "наравне"
        for u in 0..n {
            money[u] += t.want * t.bat[u] as f32 * BATTALION * cost[u] / worth;
        }
    }
    let all: f32 = money.iter().sum();
    (all > 0.0).then(|| money.iter().map(|m| m / all).collect())
}

/// The staff's plan as a template: ten battalions or so in the proportions of the units it buys.
pub fn of_plan(name: &str, plan: &[f32], c: &Country) -> Template {
    let counts: Vec<f32> = plan.iter().zip(&c.cost).map(|(p, k)| p / k.max(1e-9)).collect();
    let all: f32 = counts.iter().sum::<f32>().max(1e-9);
    let mut bat: Vec<u32> = counts.iter().map(|x| (x / all * 10.0).round() as u32).collect();
    if bat.iter().sum::<u32>() == 0 {
        if let Some(u) = (0..counts.len()).max_by(|&a, &b| counts[a].total_cmp(&counts[b])) {
            bat[u] = 10;
        }
    }
    Template::new(name, bat, 1.0)
}

/// The kit the factories make each kind with, by the templates built: each kind's norm weighed by
/// its battalions in them and by how much of each is built (standard where none builds it).
pub fn norm_of(templates: &[Template], nu: usize) -> Vec<f32> {
    (0..nu)
        .map(|u| {
            let (mut w, mut k) = (0.0f32, 0.0f32);
            for t in templates.iter().filter(|t| t.want > 0.0) {
                let b = t.bat.get(u).copied().unwrap_or(0) as f32 * t.want;
                w += b;
                k += b * t.norm_of(u);
            }
            if w > 0.0 {
                k / w
            } else {
                1.0
            }
        })
        .collect()
}

/// Divisions in so many units of each kind, counted by these templates (by battalions: the
/// templates built, weighed by how much of each).
pub fn division_size(templates: &[Template]) -> f32 {
    let (mut b, mut w) = (0.0f32, 0.0f32);
    for t in templates {
        if t.want > 0.0 && t.battalions() > 0 {
            b += t.want * t.battalions() as f32;
            w += t.want;
        }
    }
    if w > 0.0 {
        b / w * BATTALION
    } else {
        1.0
    }
}

/// A kind of troops as the designer counts it, per battalion: its name, icon, place in the line,
/// men (thousands), cost, width in the line, speed, fire at each class and toughness (as the staff
/// believes its own equipment to be), supply.
pub struct Kind {
    pub name: String,
    pub icon: String,
    pub line: Line,
    pub men: f32,
    pub cost: f32,
    pub width: f32,
    pub speed: f32,
    pub fire: Vec<f32>,
    pub tough: f32,
    pub supply: f32,
}

/// The kinds of the pack as the side's staff believes them to be.
pub fn kinds(pack: &Pack, c: &Country) -> Vec<Kind> {
    let war = pack.war.as_ref().unwrap();
    pack.units
        .iter()
        .enumerate()
        .map(|(u, k)| {
            let fire = c.own_fire[u].mean.exp();
            let tough = c.own_tough[u].mean.exp();
            Kind {
                name: k.name.clone(),
                icon: k.icon.clone(),
                line: k.line,
                men: k.men * war.crew * BATTALION,
                cost: c.cost[u] * BATTALION,
                width: if k.line == Line::Front { k.width * BATTALION } else { 0.0 },
                speed: k.speed,
                fire: k.fire.iter().map(|f| f * fire * BATTALION).collect(),
                tough: k.toughness * tough * BATTALION,
                supply: k.supply.iter().map(|&(g, a)| a * pack.goods[g].price).sum::<f32>() * BATTALION,
            }
        })
        .collect()
}

/// A division's qualities: battalions, men (thousands), worth, place in the line, speed (of the
/// slowest), fire at each class, toughness, supply.
#[derive(Clone, Debug, Default)]
pub struct Qualities {
    pub n: u32,
    pub men: f32,
    pub cost: f32,
    pub width: f32,
    pub speed: f32,
    pub fire: Vec<f32>,
    pub tough: f32,
    pub supply: f32,
}

pub fn qualities(bat: &[u32], kinds: &[Kind]) -> Qualities {
    let classes = kinds.first().map_or(0, |k| k.fire.len());
    let mut q = Qualities { fire: vec![0.0; classes], speed: f32::INFINITY, ..Default::default() };
    for (u, &b) in bat.iter().enumerate() {
        if b == 0 {
            continue;
        }
        let k = &kinds[u];
        let b32 = b as f32;
        q.n += b;
        q.men += b32 * k.men;
        q.cost += b32 * k.cost;
        q.width += b32 * k.width;
        q.tough += b32 * k.tough;
        q.supply += b32 * k.supply;
        q.speed = q.speed.min(k.speed);
        for (c, f) in k.fire.iter().enumerate() {
            q.fire[c] += b32 * f;
        }
    }
    if q.speed == f32::INFINITY {
        q.speed = 0.0;
    }
    q
}

/// A division's qualities with the kit its norms give each kind and its companies of support: as
/// `qualities`, each kind's price, fire, toughness and supply by its kit; the companies' men,
/// price (at the country's level of prices) and what they add to its fire.
pub fn qualities_of(pack: &Pack, c: &Country, bat: &[u32], norm: &[f32], support: &[usize], kinds: &[Kind]) -> Qualities {
    use strategy::military::doctrine::norm as kit;
    let mut q = qualities(bat, kinds);
    let classes = q.fire.len();
    q.cost = 0.0;
    q.tough = 0.0;
    q.supply = 0.0;
    q.fire = vec![0.0; classes];
    for (u, &b) in bat.iter().enumerate() {
        if b == 0 {
            continue;
        }
        let k = if pack.units[u].norms { norm.get(u).copied().unwrap_or(1.0) } else { 1.0 };
        let b32 = b as f32;
        let r = strategy::military::corps::ration_share(pack, u);
        q.cost += b32 * c.cost_at(u, k) * BATTALION;
        q.tough += b32 * kinds[u].tough * kit::tough(k);
        q.supply += b32 * kinds[u].supply * (r + (1.0 - r) * kit::eat(k));
        for (cl, f) in kinds[u].fire.iter().enumerate() {
            q.fire[cl] += b32 * f * kit::fire(k);
        }
    }
    let mut sh = strategy::military::corps::Shape::new("", bat.iter().map(|&b| b as f32 * BATTALION).collect());
    sh.support = support.to_vec();
    let gift = sh.gift(pack);
    q.fire.iter_mut().for_each(|f| *f *= gift.fire);
    let war = pack.war.as_ref().unwrap();
    let level = c.cost.iter().sum::<f32>() / pack.units.iter().map(|u| u.cost).sum::<f32>().max(1e-6);
    for x in support.iter().filter_map(|&k| pack.support.get(k)) {
        q.cost += x.cost * level;
        q.men += x.men * war.crew;
    }
    q
}

/// The plain division of the era: ten battalions of its plainest front-line kind (the cheapest
/// place in the line).
pub fn plain(kinds: &[Kind]) -> Qualities {
    let best = (0..kinds.len()).filter(|&u| kinds[u].line == Line::Front && kinds[u].width > 0.0).min_by(|&a, &b| (kinds[a].cost / kinds[a].width).total_cmp(&(kinds[b].cost / kinds[b].width)));
    let mut bat = vec![0u32; kinds.len()];
    if let Some(u) = best {
        bat[u] = 10;
    }
    qualities(&bat, kinds)
}

/// The battalions of the era's plain division (ten of its plainest front-line kind).
pub fn plain_bat(kinds: &[Kind]) -> Vec<u32> {
    let best = (0..kinds.len()).filter(|&u| kinds[u].line == Line::Front && kinds[u].width > 0.0).min_by(|&a, &b| (kinds[a].cost / kinds[a].width).total_cmp(&(kinds[b].cost / kinds[b].width)));
    let mut bat = vec![0u32; kinds.len()];
    if let Some(u) = best {
        bat[u] = 10;
    }
    bat
}

/// The grounds of the staff's trials of a template.
pub const TRIAL_GROUND: [(strategy::world::map::Terrain, &str); 3] = [(strategy::world::map::Terrain::Plains, "на равнине"), (strategy::world::map::Terrain::Forest, "в лесу"), (strategy::world::map::Terrain::Mountains, "в горах")];

/// The staff's trials of a template (as it believes its own equipment to be): its divisions
/// against the era's plain divisions of the same price, a line and a half of battle of them — the
/// share of battles it wins attacking and defending (not dug in: the ground's cover only), on each ground of
/// `TRIAL_GROUND`; and, for the measure, the same for the plain division against its like (an
/// attack on a defender in cover is hard for anyone: what tells a template is how much better or
/// worse it does than the plain one in its place). The same battle as the war's: its width, its
/// cover, each kind's fire on each ground; sixteen games each.
/// — with the kit its norms give its kinds and its companies (what
/// they add to its fire, and to its price).
pub fn trial_with(pack: &Pack, c: &Country, bat: &[u32], norm: &[f32], support: &[usize]) -> [[(f32, f32); 3]; 2] {
    let base = plain_bat(&kinds(pack, c));
    let mut sh = strategy::military::corps::Shape::new("", Vec::new());
    sh.support = support.to_vec();
    let a = trial_of(pack, c, bat, &base, norm, sh.gift(pack).fire, support);
    let b = trial_of(pack, c, &base, &base, &[], 1.0, &[]);
    std::array::from_fn(|s| std::array::from_fn(|g| (a[s][g], b[s][g])))
}

/// The attacker's edge in the staff's trials: half as much again as the defender (at even
/// strength nobody's attack on a defender in cover succeeds, and nothing tells one template from
/// another).
const PUSH: f32 = 1.5;

#[allow(clippy::too_many_arguments)]
fn trial_of(pack: &Pack, c: &Country, bat: &[u32], base: &[u32], norm: &[f32], boost: f32, support: &[usize]) -> [[f32; 3]; 2] {
    use strategy::military::doctrine::norm as kit;
    use strategy::military::battle::{battle, Army, Kind as Fight};
    let war = pack.war.as_ref().unwrap();
    let units = |b: &[u32]| -> Vec<f32> { b.iter().map(|&x| x as f32 * BATTALION).collect() };
    let worth = |u: &[f32]| -> f32 { u.iter().zip(&c.cost).map(|(n, k)| n * k).sum() };
    // (ours at what its kit costs, its companies too)
    let k_of = |u: usize| if pack.units[u].norms { norm.get(u).copied().unwrap_or(1.0) } else { 1.0 };
    let level = c.cost.iter().sum::<f32>() / pack.units.iter().map(|u| u.cost).sum::<f32>().max(1e-6);
    let (ours1, theirs1) = (units(bat), units(base));
    let w_ours: f32 = ours1.iter().enumerate().map(|(u, n)| n * c.cost_at(u, k_of(u))).sum::<f32>() + support.iter().filter_map(|&k| pack.support.get(k)).map(|x| x.cost * level).sum::<f32>();
    let w_theirs = worth(&theirs1);
    let mut out = [[0.0f32; 3]; 2];
    if w_ours <= 0.0 || w_theirs <= 0.0 {
        return out;
    }
    // (a line and a half of plain divisions of the plain's width, and ours for the same money)
    let plain_w: f32 = theirs1.iter().zip(&pack.units).filter(|(_, u)| u.line == Line::Front).map(|(n, u)| n * u.width).sum::<f32>().max(1e-3);
    let n_plain = (war.width * 1.5 / plain_w).ceil();
    let n_ours = n_plain * w_theirs / w_ours;
    let fire: Vec<f32> = c.own_fire.iter().map(|g| g.mean.exp()).collect();
    let tough: Vec<f32> = c.own_tough.iter().map(|g| g.mean.exp()).collect();
    let fire_k: Vec<f32> = (0..fire.len()).map(|u| fire[u] * kit::fire(k_of(u)) * boost).collect();
    let tough_k: Vec<f32> = (0..tough.len()).map(|u| tough[u] * kit::tough(k_of(u))).collect();
    let army = |one: &[f32], k: f32, dug: f32| Army { count: one.iter().map(|x| x * k).collect(), supplied: 1.0, fire: fire.clone(), tough: tough.clone(), dug };
    let ours = |one: &[f32], k: f32, dug: f32| Army { count: one.iter().map(|x| x * k).collect(), supplied: 1.0, fire: fire_k.clone(), tough: tough_k.clone(), dug };
    for (gi, (t, _)) in TRIAL_GROUND.iter().enumerate() {
        for side in 0..2 {
            let mut won = 0;
            for game in 0..16u64 {
                let mut rng = strategy::rng::Rng::new(strategy::rng::hash64(0x7121 ^ game << 8 ^ (gi as u64) << 4 ^ side as u64));
                let (a, d, ours_attack) = if side == 0 { (ours(&ours1, n_ours * PUSH, 0.0), army(&theirs1, n_plain, 0.0), true) } else { (army(&theirs1, n_plain * PUSH, 0.0), ours(&ours1, n_ours, 0.0), false) };
                let out_b = battle(pack, *t, [&a, &d], Fight::Attack, &mut rng, false);
                if (out_b.winner == Some(0)) == ours_attack {
                    won += 1;
                }
            }
            out[side][gi] = won as f32 / 16.0;
        }
    }
    out
}

/// A division's size in words.
pub fn size_word(n: u32) -> &'static str {
    match n {
        0 => "пустая",
        1..=6 => "лёгкая",
        7..=12 => "обычного размера",
        13..=18 => "крупная",
        _ => "огромная",
    }
}

impl crate::Game {
}
