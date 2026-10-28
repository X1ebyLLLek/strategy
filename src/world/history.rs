//! The world's past (`History::of`): about a hundred years before the game, year by year. Now and
//! then a country goes to war with a neighbour — one that looks weaker than itself, or one that
//! holds land it lost; more often when a warlike man rules it — and the war is fought on the map by
//! the same front as in the game, by each country's own staff (what it believes grew out of its
//! earlier wars) and its generals, people of the world with lives of their own; the peace moves
//! borders: the winner takes the regions of the loser's it holds (never the one of its capital).
//!
//! Year by year the world's people live (`people::Folk`): they come of age, rise to posts, find
//! things out, reform or rob their economies, win and lose battles, die; and every country comes to
//! know more of the era's techniques (`progress::Know`) — by its own science (its development, its
//! scientists — a genius gives in a year what others take a generation over), by its neighbours
//! (what they know costs less to learn; what most of them know drifts in by itself), by its wars
//! (the beaten learn the victor's arms). A country fields no kind of troops it has no technique for.
//!
//! What the past leaves: the borders as they are, each country's lost lands, staffs proud of some
//! arms and doubting others, generals with a record, the people living and the great dead, what
//! each country knows and its stage of development, what its economists and rulers did to its
//! economy, and a chronicle — what happened, which anyone may read; why, nobody.

use crate::military::doctrine::Country;
use crate::military::front::{Campaign, General, Theatre};
use crate::pack::Pack;
use crate::rng::{hash64, Rng};
use crate::world::countries::Politics;
use crate::world::diplomacy::{Diplomacy, Kind, Scene};
use crate::world::folk::{Calling, Folk, Times};
use crate::world::map::World;
use crate::world::names::plural;
use crate::world::names;
use crate::world::polity::{self, Life, Polities, Upheaval};
use crate::world::progress::Know;

mod wars;
mod order;
use order::{diplomacy_year, faith_year, first_order, hunger_of, life_of, times_of, upheave};
use wars::{break_away, cause_of, conclude, fight, holds, move_region, pick_pair, small_war, war_pairs, war_power};

/// Years of the past whose great wars are fought by fronts on the map (the last century), and of
/// the deep past before it (faiths, governments, houses, pacts; its wars are fought by no front).
pub const YEARS: u32 = 100;
pub const DEEP: u32 = 300;
/// Wars begun in the last years of the past are not over when the game begins.
pub const OPEN: u32 = 2;
/// Chance that a war of the deep past begins somewhere in a year; that a small war (fought by no
/// front) begins in a year of the last century besides the great ones; the longest of them.
const DEEP_WAR_A_YEAR: f32 = 0.45;
const SMALL_WAR_A_YEAR: f32 = 0.3;
const SMALL_MONTHS: u32 = 48;
/// Share of an ally's economy its help adds to a side's money in a great war.
const ALLY_HELP: f32 = 0.3;
/// How a country's power in a war fought by no front grows with its economy (below 1: a great one
/// gains less than its size); the chance a victory short of the beaten giving up takes a region.
const WAR_SCALE: f32 = 0.75;
const TAKE_LAND: f32 = 0.35;
/// Years a lost land is remembered as one's own (a claim): after it, history.
const CLAIMS: f32 = 150.0;
/// Share of famines the chronicle tells (the rest are only lived through).
const FAMINE_TOLD: f32 = 0.2;
/// The pace of the past's science against the game's: the era's techniques are not all known by
/// its beginning — the leaders have most, the backward few.
const PACE: f32 = 0.55;
/// The cost of all techniques of an era the pace is reckoned for (the industrial era's): an era of
/// fewer techniques is learnt at a slower pace, so that it too is not all known by its beginning.
const ERA_COST: f32 = 214.0;

/// What of the drift of techniques from neighbours the past keeps (with all of it, by the game's
/// beginning nearly every country knew nearly all its era knows).
const SLOW_DRIFT: f32 = 0.4;

/// The newest tier of techniques anyone may work on `year` years before the game (of `top`): the
/// era's inventions come one tier after another over the last century — the newest only in its
/// last quarter, so that by the game's beginning few countries know them.
fn invented(top: usize, year: u32) -> usize {
    let k = 1.0 + (top.saturating_sub(1)) as f32 * (1.0 - year as f32 / YEARS as f32) / 0.85;
    (k.floor() as usize).clamp(1, top)
}

/// The pace of the past's science for `pack`'s era.
fn pace_of(pack: &Pack) -> f32 {
    let all: f32 = pack.techs.iter().filter(|t| t.tier > 0).map(|t| t.cost).sum();
    PACE * (all / ERA_COST).clamp(0.3, 1.5)
}
/// Chance that a war begins somewhere in a year.
const WAR_A_YEAR: f32 = 0.12;
/// Longest war of the past, months (then the peace is made by what each holds).
const WAR_MONTHS: u32 = 36;
/// A country does not go to war again within so many years of its last one.
const REST: u32 = 8;
/// Generals of wars longer ago than this are no longer in service.
pub const SERVICE: u32 = 25;
/// A country does not begin a war against a neighbour that does not look this much weaker than
/// itself (unless the neighbour holds its land); how far off it sees a neighbour's strength (the
/// log). (Seeing truly, the stronger began every war and won it.)
const DARE: f32 = 1.2;
const MISJUDGE: f32 = 0.45;
/// How much more often a country whose ruler is wholly warlike begins a war than one whose ruler
/// cares nothing either way (and how much less a wholly peaceful one).
const HAWKS: f32 = 0.9;
/// Research a year of a country of the least development, and what development adds (years of an
/// ordinary country's science).
const RESEARCH_LEAST: f32 = 0.3;
const RESEARCH_DEV: f32 = 1.3;
/// Chance the beaten learn each military technique of the victor they may work on; the victor,
/// each of the beaten.
const TROPHY_BEATEN: f32 = 0.5;
const TROPHY_VICTOR: f32 = 0.2;
/// People of every age a country has at the beginning of the past, per its development.
const FIRST_PEOPLE: f32 = 7.0;

/// A war of the past.
#[derive(Clone, Debug)]
pub struct PastWar {
    /// Years before the game it began.
    pub year: u32,
    /// The country that began it, and the one it attacked.
    pub sides: [usize; 2],
    pub names: [String; 2],
    pub months: u32,
    /// Index of the side that won (in `sides`), if one did; whether the loser capitulated.
    pub winner: Option<usize>,
    pub capitulated: bool,
    /// Regions that changed hands, each with its people (millions), and to which side.
    pub took: Vec<(usize, f32, usize)>,
    pub battles: usize,
    /// The generals of each side, as the war left them.
    pub generals: [Vec<General>; 2],
}

pub struct History {
    pub wars: Vec<PastWar>,
    /// Each country's staff, as its wars left it (its army sent home).
    pub staffs: Vec<Country>,
    /// Generals still in service, of each country.
    pub veterans: Vec<Vec<General>>,
    /// Regions each country lost, years before the game, and who holds them now.
    pub lost: Vec<Vec<(usize, u32)>>,
    /// The world's people as the past left them (the living go on into the game).
    pub folk: Folk,
    /// What each country knows of the era's techniques.
    pub know: Vec<Know>,
    /// How each country is governed and what its people believe; the world's faiths.
    pub polity: Polities,
    /// What the countries think of each other, their pacts, the wars still going on.
    pub diplo: Diplomacy,
}

/// Development of each country against the world's (0 the poorest … 1 the richest, about 0.5 the
/// usual).
pub fn dev_shares(pol: &Politics) -> Vec<f32> {
    let n = pol.countries.len();
    let devs: Vec<f32> = (0..n).map(|c| pol.dev(c).max(1e-6)).collect();
    let live: Vec<f32> = (0..n).filter(|&c| !pol.countries[c].states.is_empty()).map(|c| devs[c].ln()).collect();
    let mean = live.iter().sum::<f32>() / live.len().max(1) as f32;
    devs.iter().map(|d| (0.5 + 0.4 * (d.ln() - mean)).clamp(0.02, 1.0)).collect()
}

/// The share of country `c`'s neighbours (and of the world, a little) that know each technique.
fn seen_by(pol: &Politics, know: &[Know], c: usize) -> Vec<f32> {
    let nt = know.first().map_or(0, |k| k.known.len());
    let nb = pol.neighbors(c);
    let world: Vec<usize> = (0..know.len()).filter(|&x| x != c && !pol.countries[x].states.is_empty()).collect();
    (0..nt)
        .map(|t| {
            let near = if nb.is_empty() { 0.0 } else { nb.iter().filter(|&&o| know[o].known[t]).count() as f32 / nb.len() as f32 };
            let far = world.iter().filter(|&&o| know[o].known[t]).count() as f32 / world.len().max(1) as f32;
            0.75 * near + 0.25 * far
        })
        .collect()
}

/// The staff of a country fields what its techniques let it: kinds it lacks are locked; what it
/// learnt since `before` (unit gains, logs) makes its kinds better (the staff knows it: its own
/// techniques).
pub fn arm_staff(pack: &Pack, staff: &mut Country, know: &Know, before: &[f32]) {
    let now = know.unit_gain(pack);
    for u in 0..now.len() {
        let d = now[u] - before.get(u).copied().unwrap_or(0.0);
        if d.abs() > 1e-6 {
            staff.redesign(u, (d, d), (d, d), 0.0);
        }
    }
    staff.lock(know.locked(pack));
}

impl History {
    /// Four hundred years of the world of `pol`: three of the deep past (faiths, governments,
    /// houses, pacts; wars fought by no front), then a century whose great wars are fought by
    /// fronts on the map.
    pub fn of(pack: &Pack, world: &World, pol: &mut Politics, seed: u64) -> History {
        let n = pol.countries.len();
        let nu = pack.units.len();
        let mut rng = Rng::new(hash64(seed ^ 0x415_7027));
        // (the people and the techniques have their own luck, the order of societies its own, and
        // the relations between countries theirs)
        let mut prng = Rng::new(hash64(seed ^ 0x9E0_91E));
        let mut orng = Rng::new(hash64(seed ^ 0x9071_7E));
        let mut drng = Rng::new(hash64(seed ^ 0xD1_9A_C7));
        let names: Vec<String> = pol.countries.iter().map(|c| c.name.clone()).collect();
        let ones = vec![1.0; pack.goods.len()];
        let mut staffs: Vec<Country> = (0..n).map(|c| Country::new(pack, &pol.countries[c].name, &ones, &mut Rng::new(hash64(seed ^ 0x57A ^ (c as u64) << 8)))).collect();
        let mut last: Vec<Option<u32>> = vec![None; n];
        let mut lost: Vec<Vec<(usize, u32)>> = vec![Vec::new(); n];
        let mut wars: Vec<PastWar> = Vec::new();
        let start = -(YEARS as i32) * 12;
        let mut know: Vec<Know> = (0..n).map(|_| Know::start(pack, start)).collect();
        let mut folk = Folk::new(n);
        let mut diplo = Diplomacy::new(n);
        let pace = pace_of(pack);
        let top = pack.techs.iter().map(|t| t.tier).max().unwrap_or(1).min(pack.stages.len().saturating_sub(1)).max(1);
        // the deep past: faiths and governments, houses, pacts and wars fought by no front
        let deep_start = -((YEARS + DEEP) as i32) * 12;
        let dev0 = dev_shares(pol);
        let mut polity = first_order(pol, &dev0, deep_start, &mut orng);
        let mut deep_ruler: Vec<(f32, f32)> = (0..n).map(|_| (0.4 * orng.normal(), 0.4 * orng.normal())).collect();
        for year in (YEARS + 1..=YEARS + DEEP).rev() {
            let month = -(year as i32) * 12;
            let dev = dev_shares(pol);
            for c in 0..n {
                if pol.countries[c].states.is_empty() {
                    continue;
                }
                let mut t = times_of(pol, &wars, &staffs, &lost, &know, pack, &dev, &polity, c, year);
                t.stage = 0;
                t.at_war = diplo.war_of(c).is_some();
                let (hunger, famine) = hunger_of(dev[c], &mut orng);
                if famine && orng.f32() < FAMINE_TOLD {
                    let line = format!("{}: голодные годы", pol.countries[c].name);
                    polity.of[c].story.push((month, line.clone()));
                    polity.chronicle.push((month, c, line));
                }
                let mut life = life_of(pol, &polity, &lost, &t, top, c, month, t.last_war, hunger, (deep_ruler[c].0, deep_ruler[c].1, false));
                // (a monarch of the deep past reigns some twenty years; most crowns pass to a son)
                if orng.f32() < 1.0 / 22.0 {
                    deep_ruler[c] = (0.4 * orng.normal(), 0.4 * orng.normal());
                    let heir = orng.f32() < 0.82;
                    let sur = if heir {
                        polity.of[c].house.clone().map_or_else(|| names::person(&mut orng), |h| h.trim_start_matches("дом ").to_string())
                    } else {
                        names::person(&mut orng).split_once(". ").map_or(String::new(), |x| x.1.to_string())
                    };
                    life.new_ruler = Some((sur, heir));
                }
                let ups = polity.year(c, &pol.countries[c].name, &life, &mut orng);
                upheave(pack, pol, &mut polity, None, &t, c, month, &ups, &mut deep_ruler, &mut orng);
            }
            faith_year(pol, &mut polity, &dev, month, &mut orng);
            let hawk = |c: usize| deep_ruler[c].1;
            diplomacy_year(pol, &polity, &mut diplo, &lost, &names, &hawk, year, month, &mut drng);
            if drng.f32() < DEEP_WAR_A_YEAR {
                small_war(pol, &mut polity, &mut diplo, &mut lost, &mut last, &mut wars, &names, &hawk, year, month, &mut drng);
            }
            break_away(pol, &mut polity, &mut diplo, &mut lost, year, month, &names, &mut drng);
        }
        // the beginning of the last century: the people of every age, and a head start in
        // techniques by development
        let dev = dev_shares(pol);
        for c in 0..n {
            if pol.countries[c].states.is_empty() {
                continue;
            }
            let t = times_of(pol, &wars, &staffs, &lost, &know, pack, &dev, &polity, c, YEARS);
            let k = (FIRST_PEOPLE * (0.6 + dev[c]) + 0.3 * pol.pop(c).sqrt()).round() as usize;
            for _ in 0..k {
                let age = 24.0 + 40.0 * prng.f32();
                folk.born(pack, c, &t, start, None, age, &mut prng);
            }
            let lean = folk.leaning(pack, c);
            let head = 6.0 * dev[c] * dev[c] * pace;
            let seen = |_: usize| 0.0;
            let mut r = Rng::new(hash64(seed ^ 0x7EC ^ c as u64));
            let mut pick = |k: &Know, open: &[usize]| {
                let _ = k;
                lean.pick(pack, open, &seen, &mut r)
            };
            know[c].newest = 1;
            know[c].research(pack, head, &seen, &mut pick, start, None);
            arm_staff(pack, &mut staffs[c], &know[c], &vec![0.0; nu]);
        }
        for year in (1..=YEARS).rev() {
            let month = -(year as i32) * 12;
            let dev = dev_shares(pol);
            // a year of peace for every staff: doubt and glory fade; exercises
            for st in &mut staffs {
                for _ in 0..12 {
                    st.quiet();
                }
                st.exercise(&mut rng);
            }
            // a year of the people, of science and of the order of society, in every country
            for c in 0..n {
                if pol.countries[c].states.is_empty() {
                    continue;
                }
                let mut t = times_of(pol, &wars, &staffs, &lost, &know, pack, &dev, &polity, c, year);
                t.at_war = diplo.war_of(c).is_some();
                know[c].newest = invented(top, year);
                let seen_v = seen_by(pol, &know, c);
                let seen = |t: usize| seen_v.get(t).copied().unwrap_or(0.0);
                let before = know[c].unit_gain(pack);
                let extra = folk.year(pack, c, &t, month, &mut know[c], &seen, &mut prng);
                // (the first rulers of the century are of the houses the deep past left)
                if year == YEARS {
                    if let (Some(house), Some(id)) = (polity.of[c].house.clone(), folk.offices[c].ruler) {
                        let sur = house.trim_start_matches("дом ").to_string();
                        let initial = folk.get(id).name.split_once(' ').map_or("А.".to_string(), |x| x.0.to_string());
                        folk.people[id as usize].name = format!("{initial} {sur}");
                        folk.changed[c] = Some((sur, true));
                    }
                }
                let sums = know[c].sums(pack);
                let points = (RESEARCH_LEAST + RESEARCH_DEV * dev[c].powf(1.2)) * (1.0 + sums.science + folk.schools[c]).max(0.2) * polity.science_k(c) * pace + extra;
                let lean = folk.leaning(pack, c);
                let mut pick = |_: &Know, open: &[usize]| lean.pick(pack, open, &seen, &mut prng);
                let got = know[c].research(pack, points.max(0.0), &seen, &mut pick, month, None);
                let drift = know[c].drift(pack, &seen, &mut prng, month, Know::DRIFT * pace * SLOW_DRIFT);
                for tt in got.iter().chain(drift.iter()) {
                    if pack.techs[*tt].tier >= 3 {
                        folk.chronicle.push((month, c, format!("{} овладевает техникой: {}", pol.countries[c].name, pack.techs[*tt].name)));
                    }
                }
                arm_staff(pack, &mut staffs[c], &know[c], &before);
                // the order of society: what the year did to it, what broke
                let ruler = folk.offices[c].ruler.map_or((0.0, 0.0, false), |r| {
                    let r = folk.get(r);
                    (r.reform, r.hawk, r.calling == Calling::General)
                });
                let (hunger, famine) = hunger_of(dev[c], &mut orng);
                if famine && orng.f32() < FAMINE_TOLD {
                    let line = format!("{}: голодные годы", pol.countries[c].name);
                    polity.of[c].story.push((month, line.clone()));
                    polity.chronicle.push((month, c, line));
                }
                let mut life = life_of(pol, &polity, &lost, &t, top, c, month, t.last_war, hunger, ruler);
                life.new_ruler = folk.changed[c].take();
                let ups = polity.year(c, &pol.countries[c].name, &life, &mut orng);
                upheave(pack, pol, &mut polity, Some(&mut folk), &t, c, month, &ups, &mut deep_ruler, &mut orng);
            }
            faith_year(pol, &mut polity, &dev, month, &mut orng);
            let hawks: Vec<f32> = (0..n).map(|c| folk.hawk(c)).collect();
            let hawk = |c: usize| hawks[c];
            diplomacy_year(pol, &polity, &mut diplo, &lost, &names, &hawk, year, month, &mut drng);
            // the wars of the world still going on (begun in the last years) go another year
            let mut w = 0;
            while w < diplo.wars.len() {
                let power = |c: usize| war_power(pol, c);
                let mut end = None;
                for m in 0..12 {
                    if let Some(e) = diplo.war_month(w, &power, &mut drng) {
                        end = Some((e, m));
                        break;
                    }
                }
                match end {
                    Some(((win, harsh), m)) => {
                        let months = (month - diplo.wars[w].since).max(0) as u32 + m as u32 + 1;
                        let pw = conclude(&mut diplo, w, win, harsh, months, pol, &mut lost, &mut last, year, &names, &mut drng);
                        wars.push(pw);
                    }
                    None => w += 1,
                }
            }
            break_away(pol, &mut polity, &mut diplo, &mut lost, year, month, &names, &mut drng);
            // a small war, fought by no front
            if drng.f32() < SMALL_WAR_A_YEAR {
                small_war(pol, &mut polity, &mut diplo, &mut lost, &mut last, &mut wars, &names, &hawk, year, month, &mut drng);
            }
            if rng.f32() >= WAR_A_YEAR {
                continue;
            }
            // a great war, fought on the map by the two that begin it; their allies send money
            let pairs = war_pairs(pol, &polity, &diplo, &lost, &last, &hawk, year, &mut rng);
            let Some((a, b)) = pick_pair(&pairs, &mut rng) else { continue };
            let cause = cause_of(pol, &polity, &diplo, &lost, a, b, year, month, &mut drng);
            let sides = diplo.coalition(a, b);
            let w = diplo.begin(sides, month, cause, &names, &mut drng);
            if year <= OPEN {
                // (begun in the last years: it goes on when the game begins)
                continue;
            }
            let money = [0, 1].map(|s| diplo.wars[w].sides[s].iter().enumerate().map(|(i, &c)| pol.gdp(c) * if i == 0 { 1.0 } else { ALLY_HELP }).sum::<f32>());
            let times = [times_of(pol, &wars, &staffs, &lost, &know, pack, &dev, &polity, a, year), times_of(pol, &wars, &staffs, &lost, &know, pack, &dev, &polity, b, year)];
            let war = fight(pack, world, pol, &lost, &mut staffs, [a, b], money, year, hash64(seed ^ 0x3A2 ^ year as u64), &mut rng, nu, &mut folk, &times, &mut prng);
            let fought = diplo.end(w, war.winner, war.capitulated, month + war.months as i32, &names);
            for s in 0..2 {
                for &c in &fought.sides[s] {
                    last[c] = Some(year);
                }
            }
            // the peace: regions change hands
            for &(st, _, to) in &war.took {
                let (from, to) = ([a, b][1 - to], [a, b][to]);
                move_region(pol, &mut lost, st, from, to, year);
            }
            // the beaten learn the victor's arms from his trophies; the victor a little of theirs
            if let Some(w) = war.winner {
                let (win, lose) = ([a, b][w], [a, b][1 - w]);
                for (to, from, chance) in [(lose, win, TROPHY_BEATEN), (win, lose, TROPHY_VICTOR)] {
                    let before = know[to].unit_gain(pack);
                    let theirs: Vec<usize> = (0..pack.techs.len()).filter(|&t| know[from].known[t] && know[to].can(pack, t) && pack.techs[t].sphere == crate::pack::Sphere::Military).collect();
                    for t in theirs {
                        if prng.f32() < chance {
                            know[to].learn(t, month, None);
                            folk.chronicle.push((month, to, format!("{}: трофеи войны — {}", pol.countries[to].name, pack.techs[t].name)));
                        }
                    }
                    arm_staff(pack, &mut staffs[to], &know[to], &before);
                }
            }
            wars.push(war);
        }
        // generals still in service: of the wars of the last years, the living (the eldest left)
        let mut veterans: Vec<Vec<General>> = vec![Vec::new(); n];
        for w in &wars {
            if w.year > SERVICE {
                continue;
            }
            for s in 0..2 {
                for g in &w.generals[s] {
                    let c = w.sides[s];
                    if folk.find(c, &g.name).is_some_and(|id| !folk.get(id).alive()) {
                        continue;
                    }
                    match veterans[c].iter_mut().find(|v| v.name == g.name) {
                        Some(v) => *v = g.clone(),
                        None => veterans[c].push(g.clone()),
                    }
                }
            }
        }
        for st in &mut staffs {
            st.army.count = vec![0.0; nu];
        }
        // what four hundred years of their order did to the countries' development
        for c in 0..n {
            let k = polity.of[c].drift.exp();
            let provs: Vec<usize> = pol.provinces_of(c).collect();
            for p in provs {
                pol.prov[p].dev *= k;
            }
        }
        // (the farmland went with the regions that changed hands)
        pol.refit_land(world);
        // (the game may work on every tier)
        for k in &mut know {
            k.newest = usize::MAX;
        }
        // (lands lost too long ago are history, not a claim)
        for l in &mut lost {
            l.retain(|&(_, y)| (y as f32) <= CLAIMS);
        }
        wars.sort_by(|a, b| b.year.cmp(&a.year));
        History { wars, staffs, veterans, lost, folk, know, polity, diplo }
    }
}

impl General {
    /// One of `veterans` (the country's generals of its last wars), if one is left; else someone
    /// nobody has heard of yet.
    pub fn of_country(veterans: &mut Vec<General>, rng: &mut Rng) -> General {
        if veterans.is_empty() {
            General::anyone(names::person(rng), rng)
        } else {
            veterans.remove(rng.below(veterans.len()))
        }
    }
}
