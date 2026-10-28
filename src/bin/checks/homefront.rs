//! The home front: two neighbouring countries of a generated map, in a trading world, whose
//! economies raise, arm and feed their armies — some years of peace, then war on the map. A
//! chronicle of one war and self-checks on many.
//!
//! `checks homefront [--pack packs/x.toml] [--seed N] [--quick]`
//! (`--quick` — the chronicle only; `--twins` — the chronicle's two countries alike; `--army` —
//! only the checks of what armies grow from; `--econ` — only the checks of economists at war;
//! `--science` — only the checks of science, `--scientist` — of scientists, `--capture` — of
//! trophies (a pack with `[science]`); `--poor` — only the checks that a poor country lives;
//! `--draft` — only the checks of the law of the draft;
//! `DBG=1` — the army's market month by month)

use rayon::prelude::*;
use strategy::economy::advisor::{Economist, Mind};
use strategy::economy::nation::{Nation, Setup};
use strategy::economy::science::{Lab, Scientist};
use strategy::economy::trade::{self, World as Trading};
use strategy::military::doctrine::Country;
use strategy::military::front::{Campaign, Theatre};
use strategy::military::homefront::{lines, men_of, setup_of, shares, Homefront};
use strategy::pack::{Line, Pack};
use strategy::rng::{hash64, Rng};
use strategy::world::countries::{self, Politics};
use strategy::world::map::{self, Params, World};

mod army;
mod capture;
mod draft;
mod econ;
mod poor;
mod science;
mod suite;

use army::army_checks;
use capture::capture_checks;
use draft::draft_checks;
use econ::econ_checks;
use poor::poor_checks;
use science::{quality, science_checks, scientist_checks};
use suite::checks;
use crate::common::{pick, mean_se};

/// Months the world lives before the game, each country alone and then trading (as in the
/// trade checks), of peace with an army before the war, and of war at most.
const ALONE: u32 = 240;
const TOGETHER: u32 = 240;
const PEACE: u32 = 48;
const WAR: u32 = 36;
/// A country short of a good: what its makers of it give, of their output.
const SHORT: f32 = 0.25;
/// A poor country: its development against its twin's.
const POOR: f32 = 0.25;
/// Lucks of the years of peace whose armies the army checks compare.
const ARMY_LUCKS: [u64; 3] = [21, 22, 23];
/// Lucks each war of the checks is fought with.
const LUCKS: [u64; 4] = [11, 12, 13, 14];

/// Twins: what the two countries are on average.
fn average(a: &Setup, b: &Setup) -> Setup {
    Setup {
        name: "Близнец".into(),
        pop: 0.5 * (a.pop + b.pop),
        dev: 0.5 * (a.dev + b.dev),
        fertility: 0.5 * (a.fertility + b.fertility),
        land: 0.5 * (a.land + b.land),
        deposits: a.deposits.iter().zip(&b.deposits).map(|(x, y)| 0.5 * (x + y)).collect(),
    }
}

/// The countries of the map trading, settled before the game; with `twins`, the two of `pair`
/// are alike (what both are on average), and the first of them has `more` times the people
/// and deposits of the second and `dev` times its development.
fn trading(pack: &Pack, world: &World, pol: &Politics, pair: (usize, usize), twins: bool, more: f32, dev: f32) -> Trading {
    let mut setups: Vec<Setup> = (0..pol.countries.len()).map(|c| setup_of(pol, c)).collect();
    if twins {
        let avg = average(&setups[pair.0], &setups[pair.1]);
        for c in [pair.0, pair.1] {
            setups[c] = Setup { name: setups[c].name.clone(), ..avg.clone() };
        }
        setups[pair.0].pop *= more;
        setups[pair.0].land *= more;
        setups[pair.0].deposits.iter_mut().for_each(|d| *d *= more);
        setups[pair.0].dev *= dev;
    }
    let routes = trade::routes(world, pol, pack.trade.as_ref().expect("в наборе нет торговли"));
    let mut w = Trading::new(pack, setups.iter().map(|s| Nation::new(pack, s)).collect(), routes);
    w.settle(pack, ALONE, TOGETHER);
    w
}

/// A war ready to begin between countries `pair` of the map, trading in `trading`; with
/// `twins`, both countries' equipment and what their designers claim are the same.
fn sandbox(pack: &Pack, world: &World, pol: &Politics, pair: (usize, usize), trading: Trading, seed: u64, twins: bool) -> Homefront {
    let th = Theatre::new(world, pol, pair.0, pair.1);
    let mut ra = Rng::new(hash64(seed));
    let mut rb = Rng::new(hash64(seed ^ if twins { 0 } else { 0xB0B }));
    let ones = vec![1.0; pack.goods.len()];
    let a = Country::new(pack, &trading.nations[pair.0].name, &ones, &mut ra);
    let b = Country::new(pack, &trading.nations[pair.1].name, &ones, &mut rb);
    Homefront::new(pack, trading, [pair.0, pair.1], Campaign::raised(pack, th, [a, b], seed))
}

fn mix(pack: &Pack, army: &[f32]) -> String {
    let sh = shares(pack, army);
    let mut order: Vec<usize> = (0..sh.len()).filter(|&u| sh[u] >= 0.05).collect();
    order.sort_by(|&a, &b| sh[b].total_cmp(&sh[a]));
    order.iter().map(|&u| format!("{} {:.0}%", pack.units[u].name, sh[u] * 100.0)).collect::<Vec<_>>().join(", ")
}

fn line(pack: &Pack, h: &Homefront, s: usize, border: f32) -> String {
    let n = h.home(s);
    let c = &h.war.sides[s];
    format!(
        "армия {:.0} частей ({:.1} линии: {}), снабжение {:.0}%, служат {:.1}%, погибло {:.0} тыс., ВВП {:.0}, недовольство {:.2}, волнений {}",
        c.army.count.iter().sum::<f32>(),
        lines(pack, &c.army.count, border),
        mix(pack, &c.army.count),
        h.fed[s] * 100.0,
        n.drafted.iter().sum::<f32>() / n.people().max(1e-9) * 100.0,
        h.dead[s] * 1000.0,
        n.gdp_real,
        n.anger(),
        n.events.len()
    )
}

/// The army's market (`DBG=1`): each good's price against the usual, its share of the army's
/// order, what the army bought, what lies in its stores.
fn debug(pack: &Pack, h: &Homefront, what: &str) {
    let n = h.home(0);
    let rel: Vec<String> = (0..pack.goods.len())
        .map(|i| format!("{} {:.2}/{:.3}/{:.3}/{:.3}", pack.goods[i].id, n.price[i] / (pack.goods[i].price * n.level), n.army_wants[i], n.bought_by[i][3], n.arsenal[i]))
        .collect();
    println!("  {what}: деньги армии {:.2}, казна {:.1}, уровень {:.2}; цена к обычной / доля заказа / купила армия / в запасе:", n.army_money(), n.treasury, n.level);
    println!("    {}", rel.join(", "));
}

/// Who won: the side that made the other give up, or that holds more of its people at the end
/// (None when they hold nearly as much).
fn winner(h: &Homefront) -> Option<usize> {
    if let Some(s) = h.war.fallen {
        return Some(1 - s);
    }
    let p = h.war.people();
    if (p[0] - p[1]).abs() < 0.05 {
        None
    } else if p[0] > p[1] {
        Some(0)
    } else {
        Some(1)
    }
}

fn chronicle(pack: &Pack, world: &World, pol: &Politics, pair: (usize, usize), seed: u64, twins: bool) {
    let started = std::time::Instant::now();
    let tw = trading(pack, world, pol, pair, twins, 1.0, 1.0);
    println!("══════ {} — тыл на карте {seed}: {} против {} ══════", pack.name, tw.nations[pair.0].name, tw.nations[pair.1].name);
    println!("  мир из {} стран с торговлей устоялся за {:.1} с", tw.nations.len(), started.elapsed().as_secs_f32());
    let mut h = sandbox(pack, world, pol, pair, tw, seed, twins);
    let border = (h.war.front(0).len() + h.war.front(1).len()) as f32 / 2.0;
    for s in 0..2 {
        let n = h.home(s);
        println!("  {}: людей {:.1} млн, ВВП {:.1} в месяц, деньги армии {:.2} в месяц", n.name, n.people(), n.gdp_real, n.army_money());
    }
    let dbg = std::env::var("DBG").is_ok();
    for m in 1..=PEACE {
        h.peace(pack);
        if dbg && m % 12 == 0 {
            debug(pack, &h, &format!("мир, мес {m}"));
        }
        if m % 12 == 0 {
            println!("  мир, мес {m}:");
            for s in 0..2 {
                println!("    {}: {}", h.home(s).name, line(pack, &h, s, border));
            }
        }
    }
    println!("  граница {border:.0} провинций; война: военный бюджет, военный налог, призыв");
    h.declare(pack);
    for m in 1..=WAR {
        h.step(pack);
        if dbg && m % 6 == 0 {
            debug(pack, &h, &format!("война, мес {m}"));
        }
        if m % 6 == 0 || h.war.fallen.is_some() {
            let held = h.war.held();
            let ppl = h.war.people();
            println!("  война, мес {m}: провинций {} / {} (своих людей {:.0}% / {:.0}%)", held[0], held[1], ppl[0] * 100.0, ppl[1] * 100.0);
            for s in 0..2 {
                println!("    {}: {}", h.home(s).name, line(pack, &h, s, border));
            }
        }
        if let Some(s) = h.war.fallen {
            println!("  мес {m}: {} капитулирует", h.home(s).name);
            break;
        }
    }
    for s in 0..2 {
        for e in h.home(s).events.iter().rev().take(3).collect::<Vec<_>>().into_iter().rev() {
            println!("  {}: {e}", h.home(s).name);
        }
    }
}

/// The ways the wars of the checks are fought.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Cond {
    /// Twins (the control).
    Twins,
    /// The same twins, but they stay at peace.
    Peace,
    /// Twins at war, but a war that asks nothing more of the people than peace did: no war
    /// footing, no grief (the control for discontent).
    Costless,
    /// The first side is cut off from all trade when the war begins.
    Blockade,
    /// One side (the first of the map's pair) has twice the people and deposits.
    Rich,
    /// Twins at war, but what the enemy holds stays in a country's economy (the control for
    /// occupation).
    Unheld,
}

/// What one war showed.
struct Run {
    map: usize,
    swapped: bool,
    luck: u64,
    cond: Cond,
    winner: Option<usize>,
    months: u32,
    /// Each side, per month of war: supply got, discontent, unrest so far.
    fed: [Vec<f32>; 2],
    anger: [Vec<f32>; 2],
    unrest: [Vec<usize>; 2],
    /// Share of the battles the first side won.
    won: f32,
    /// Men killed, against the people before the war; most of the people serving at once.
    dead: [f32; 2],
    drafted: [f32; 2],
    /// Share of the army's purchases (by worth) that came from abroad.
    imported: [f32; 2],
    /// People before the war (millions).
    people: f32,
    /// Each side, per month of war: share of its land (by people) the enemy holds, and its real
    /// output against what it was before the war.
    held: [Vec<f32>; 2],
    gdp: [Vec<f32>; 2],
    /// All the money of the world when the war began and at its end.
    money: (f64, f64),
    /// How much better the second side's equipment is than the first's at the end (log).
    gap: f32,
}

struct Map0 {
    world: World,
    pol: Politics,
    pair: (usize, usize),
    twins: Trading,
    /// With the first country of the pair twice as big, or `POOR` times as developed.
    other: Trading,
}

/// How the years of peace before a war go.
#[derive(Clone, Copy, PartialEq)]
struct Prep {
    /// The world with the first country of the pair changed (`Map0::other`).
    other: bool,
    /// The first side's makers of this good give `SHORT` of their output.
    short: Option<usize>,
    /// The first side does not trade at all.
    autarky: bool,
    /// Each side's economist (the first side's, the second's), from the first month of peace.
    econ: Option<(Mind, Mind)>,
    /// Each side's share of the state's spending on science (0: no laboratories).
    science: (f32, f32),
    /// Each side's scientist heading its laboratories.
    scientists: Option<(Mind, Mind)>,
    /// Months of peace before the war.
    peace: u32,
}

const PLAIN: Prep = Prep { other: false, short: None, autarky: false, econ: None, science: (0.0, 0.0), scientists: None, peace: PEACE };
/// Years of peace in which science has time to matter.
const SCIENCE_PEACE: u32 = 120;

/// The years of peace before a war, with `luck`: the twins (or the changed one and its twin).
fn prepare(pack: &Pack, m: &Map0, mi: usize, swapped: bool, prep: Prep, luck: u64) -> Homefront {
    let pair = if swapped { (m.pair.1, m.pair.0) } else { m.pair };
    let tw = if prep.other { m.other.clone() } else { m.twins.clone() };
    let mut h = sandbox(pack, &m.world, &m.pol, pair, tw, hash64(luck * 1000 + mi as u64 * 2 + swapped as u64), true);
    if let Some(good) = prep.short {
        for (sec, sector) in pack.sectors.iter().enumerate() {
            if sector.outputs.iter().any(|o| o.0 == good) {
                h.home_mut(0).shock[sec] = SHORT;
            }
        }
    }
    if prep.autarky {
        h.blockade(0);
    }
    if let Some((a, b)) = prep.econ {
        let seed = hash64(luck * 1000 + mi as u64 * 2 + swapped as u64) ^ 0xEC;
        h.econ = [Some(Economist::new(a, seed, pack)), Some(Economist::new(b, seed ^ 1, pack))];
    }
    for (s, share) in [prep.science.0, prep.science.1].into_iter().enumerate() {
        if share > 0.0 {
            let seed = hash64(luck * 1000 + mi as u64 * 2 + swapped as u64) ^ 0x5C ^ s as u64;
            h.home_mut(s).science_share = share;
            h.labs[s] = Some(Lab::new(pack, seed));
        }
    }
    if let Ok(k) = std::env::var("FOCUS1") {
        // a test of the instruments' power: the second side's laboratories work on one sphere only
        let k: usize = k.parse().unwrap();
        if let Some(lab) = h.labs[1].as_mut() {
            lab.focus = (0..lab.focus.len()).map(|i| if i == k { 1.0 } else { 0.0 }).collect();
        }
    }
    if let Some((a, b)) = prep.scientists {
        let seed = hash64(luck * 1000 + mi as u64 * 2 + swapped as u64) ^ 0x5C1;
        h.scientists = [Some(Scientist::new(a, seed, pack)), Some(Scientist::new(b, seed ^ 1, pack))];
    }
    for _ in 0..prep.peace {
        h.peace(pack);
    }
    h
}

/// The war (or the peace that follows instead) after years of peace `h`, fought with `luck`.
fn run(pack: &Pack, mut h: Homefront, mi: usize, swapped: bool, luck: u64, cond: Cond) -> Run {
    h.war.seed = hash64(luck * 1000 + mi as u64 * 2 + swapped as u64);
    let before = [h.home(0).people(), h.home(1).people()];
    let gdp0 = [h.home(0).gdp_real, h.home(1).gdp_real];
    let money0 = h.world.money_total();
    let (mut held, mut gdp) = ([Vec::new(), Vec::new()], [Vec::new(), Vec::new()]);
    let events = [h.home(0).events.len(), h.home(1).events.len()];
    let (mut fed, mut ang, mut unrest) = ([Vec::new(), Vec::new()], [Vec::new(), Vec::new()], [Vec::new(), Vec::new()]);
    let mut drafted = [0.0f32; 2];
    let (mut bought, mut came) = ([0.0f32; 2], [0.0f32; 2]);
    if cond == Cond::Costless {
        h.burden = false;
    }
    if cond == Cond::Unheld {
        h.occupation = false;
    }
    if cond != Cond::Peace {
        h.declare(pack);
    }
    if cond == Cond::Blockade {
        h.blockade(0);
    }
    let mut months = 0;
    for _ in 0..WAR {
        if cond == Cond::Peace {
            h.peace(pack);
        } else {
            h.step(pack);
            if h.war.fallen.is_some() {
                break;
            }
        }
        months += 1;
        let ppl = h.war.people();
        for s in 0..2 {
            let n = h.home(s);
            held[s].push(1.0 - ppl[s]);
            gdp[s].push(n.gdp_real / gdp0[s].max(1e-9));
            fed[s].push(h.fed[s]);
            ang[s].push(n.anger());
            unrest[s].push(n.events.len() - events[s]);
            drafted[s] = drafted[s].max(n.drafted.iter().sum::<f32>() / before[s]);
            for i in 0..pack.goods.len() {
                let worth = n.bought_by[i][3] * n.price[i];
                bought[s] += worth;
                if n.offered[i] > 0.0 {
                    came[s] += worth * (n.import[i] / n.offered[i]).min(1.0);
                }
            }
        }
    }
    let c = &h.war.sides[0];
    Run {
        map: mi,
        swapped,
        luck,
        cond,
        winner: if cond == Cond::Peace { None } else { winner(&h) },
        months,
        fed,
        anger: ang,
        unrest,
        won: c.battles_won as f32 / c.battles.max(1) as f32,
        dead: [h.dead[0] / before[0], h.dead[1] / before[1]],
        drafted,
        imported: [came[0] / bought[0].max(1e-9), came[1] / bought[1].max(1e-9)],
        people: before[0],
        held,
        gdp,
        money: (money0, h.world.money_total()),
        gap: quality(&h.war.sides[1]) - quality(&h.war.sides[0]),
    }
}

fn mean(v: &[f32]) -> f32 {
    v.iter().sum::<f32>() / v.len().max(1) as f32
}

/// The eight maps of the checks, each with its twins' world and the world where the first of the
/// pair is twice as big (`more`) or less developed (`dev`).
fn maps(pack: &Pack, more: f32, dev: f32) -> Vec<Map0> {
    maps_n(pack, more, dev, 8)
}

/// The same, on `count` maps (`MAPS` overrides).
fn maps_n(pack: &Pack, more: f32, dev: f32, count: u64) -> Vec<Map0> {
    let count: u64 = std::env::var("MAPS").ok().and_then(|v| v.parse().ok()).unwrap_or(count);
    (1..=count)
        .into_par_iter()
        .filter_map(|seed| {
            let world = map::generate(&Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 }).0;
            let pol = countries::generate(&world, seed, 30, &pack);
            let pair = pick(&world, &pol)?;
            let twins = trading(pack, &world, &pol, pair, true, 1.0, 1.0);
            // (the same world twice need not be settled twice)
            let other = if more == 1.0 && dev == 1.0 { twins.clone() } else { trading(pack, &world, &pol, pair, true, more, dev) };
            Some(Map0 { world, pol, pair, twins, other })
        })
        .collect()
}

fn report(checks: &[(String, bool, String)], started: std::time::Instant) -> bool {
    println!("\nпроверки ({:.0} с):", started.elapsed().as_secs_f32());
    let mut ok = true;
    for (name, pass, detail) in checks {
        ok &= pass;
        println!("  [{}] {:<44} {}", if *pass { "OK" } else { "FAIL" }, name, detail);
    }
    ok
}

pub fn main() {
    let args: Vec<String> = crate::args().into_iter().collect();
    let arg = |k: &str| args.iter().position(|a| a == k).map(|i| args[i + 1].clone());
    let path = arg("--pack").unwrap_or("packs/industrial.toml".into());
    let seed: u64 = arg("--seed").map_or(1, |v| v.parse().expect("--seed N"));
    let pack = Pack::load(&path).unwrap_or_else(|e| panic!("{e}"));
    let world = map::generate(&Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 }).0;
    let pol = countries::generate(&world, seed, 30, &pack);
    let Some(pair) = pick(&world, &pol) else {
        println!("на карте {seed} нет пары соседей сравнимой силы");
        return;
    };
    if args.iter().any(|a| a == "--army") {
        if !army_checks(&pack) {
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a| a == "--econ") {
        if !econ_checks(&pack) {
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a| a == "--science") {
        if !science_checks(&pack) {
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a| a == "--draft") {
        if !draft_checks(&pack) {
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a| a == "--poor") {
        if !poor_checks(&pack) {
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a| a == "--poorphase") {
        // (information) the less developed twin of map `--seed` through the world's pre-history:
        // alone, then trading — its people, what it lacks, and how it lives, every few years
        let dev: f32 = std::env::var("POORDEV").ok().and_then(|v| v.parse().ok()).unwrap_or(0.5);
        let mut setups: Vec<Setup> = (0..pol.countries.len()).map(|c| setup_of(&pol, c)).collect();
        let avg = average(&setups[pair.0], &setups[pair.1]);
        for c in [pair.0, pair.1] {
            setups[c] = Setup { name: setups[c].name.clone(), ..avg.clone() };
        }
        setups[pair.0].dev *= dev;
        let routes = trade::routes(&world, &pol, pack.trade.as_ref().unwrap());
        let mut w = Trading::new(&pack, setups.iter().map(|s| Nation::new(&pack, s)).collect(), routes);
        let show = |w: &Trading, when: &str| {
            for (label, c) in [("бедный", pair.0), ("богатый", pair.1)] {
                let n = &w.nations[c];
                let pain = |g: usize| n.hardship[g] * n.pop[g];
                let g = (0..pack.groups.len()).max_by(|&a, &b| pain(a).total_cmp(&pain(b))).unwrap();
                let groups: Vec<String> = (0..pack.groups.len())
                    .map(|g| format!("{} {:.1} (нужда {:.0}%, без работы {})", &pack.groups[g].name, n.pop[g], n.hardship[g] * 100.0, n.unemployed[g].map_or("—".into(), |u| format!("{:.0}%", u * 100.0))))
                    .collect();
                println!("  {when:<22} {label:<8} людей {:>6.1}, выпуск {:>6.1}, беда: {} — {} | {}", n.pop.iter().sum::<f32>(), n.gdp_real, pack.groups[g].name, n.cause(&pack, g), groups.join(", "));
            }
        };
        show(&w, "начало");
        for m in 1..=ALONE {
            for c in [pair.0, pair.1] {
                w.nations[c].live = false;
                w.nations[c].step(&pack);
            }
            if m % 60 == 0 {
                show(&w, &format!("одни, {} лет", m / 12));
            }
        }
        // the rest of the world alone too, then the ways open (as `World::settle`)
        let others: Vec<usize> = (0..w.nations.len()).filter(|&c| c != pair.0 && c != pair.1).collect();
        for &c in &others {
            w.nations[c].live = false;
            for _ in 0..ALONE {
                w.nations[c].step(&pack);
            }
        }
        for x in w.nations.iter_mut() {
            x.rebase(1.0 / x.level.max(1e-6));
        }
        for m in 1..=TOGETHER {
            w.step(&pack);
            if m % 60 == 0 || m == 12 {
                show(&w, &format!("с торговлей, {} мес", m));
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--poorlife") {
        // (information) where a less developed twin (`POORDEV`, half by default) loses its people: after the world's pre-history, after ten more
        // years of the trading world alone, and after ten years of peace with armies
        let dev: f32 = std::env::var("POORDEV").ok().and_then(|v| v.parse().ok()).unwrap_or(0.5);
        let maps = maps_n(&pack, 1.0, dev, 4);
        for (mi, m) in maps.iter().enumerate() {
            let ppl = |w: &Trading, c: usize| w.nations[c].pop.iter().sum::<f32>();
            let (p, r) = m.pair;
            let mut w = m.other.clone();
            let start = (ppl(&w, p), ppl(&w, r));
            for _ in 0..120 {
                w.step(&pack);
            }
            let world = (ppl(&w, p), ppl(&w, r));
            let mut h = prepare(&pack, m, mi, false, Prep { other: true, peace: SCIENCE_PEACE, ..PLAIN }, 0);
            let army = (h.home(0).people(), h.home(1).people());
            let _ = &mut h;
            println!(
                "  карта {}: бедный/богатый, млн людей — после предыстории {:.1}/{:.1}, ещё 10 лет торговли {:.1}/{:.1}, 10 лет мира с армиями {:.1}/{:.1}",
                mi + 1, start.0, start.1, world.0, world.1, army.0, army.1
            );
        }
        return;
    }
    if args.iter().any(|a| a == "--capture") {
        if !capture_checks(&pack) {
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a| a == "--scientist") {
        if !scientist_checks(&pack) {
            std::process::exit(1);
        }
        return;
    }
    chronicle(&pack, &world, &pol, pair, seed, args.iter().any(|a| a == "--twins"));
    if !args.iter().any(|a| a == "--quick") && !checks(&pack) {
        std::process::exit(1);
    }
}
