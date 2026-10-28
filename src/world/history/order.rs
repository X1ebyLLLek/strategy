//! How each country lives and is governed through the past: what it lives through in a year
//! (`Times`, `Life`), hunger, upheavals and their people, faiths spreading, the first order of the
//! world, a year of relations.

use super::*;

/// What country `c` lives through in year `year` (years before the game) of the past.
#[allow(clippy::too_many_arguments)]
pub(super) fn times_of(pol: &Politics, wars: &[PastWar], staffs: &[Country], lost: &[Vec<(usize, u32)>], know: &[Know], pack: &Pack, dev: &[f32], polity: &Polities, c: usize, year: u32) -> Times {
    let mut last: Option<(f32, Option<bool>)> = None;
    for w in wars {
        if let Some(i) = w.sides.iter().position(|&x| x == c) {
            let ended = w.year as f32 - w.months as f32 / 12.0;
            let ago = (ended - year as f32).max(0.0);
            if last.is_none_or(|l| ago < l.0) {
                last = Some((ago, w.winner.map(|x| x == i)));
            }
        }
    }
    let grudge = lost[c].iter().any(|&(s, _)| pol.states[s].country as usize != c);
    let r = polity.of.get(c).map(|p| p.regime);
    Times {
        pop: pol.pop(c),
        dev: dev[c],
        at_war: false,
        last_war: last,
        grudge,
        stage: know[c].stage(pack),
        glory: staffs[c].glory.clone(),
        lifelong: r.is_some_and(|r| r.lifelong()),
        term: r.and_then(|r| r.elected()).unwrap_or(0.0),
        soldiers: r.is_some_and(|r| r.soldiers()),
        crowned: r.is_some_and(|r| r.crowned()),
    }
}

/// What country `c` lives through in a year, for its order: `last` — its last war (years ago,
/// won); `ruler` — its ruler's beliefs (reform, war) and whether he is a soldier.
#[allow(clippy::too_many_arguments)]
pub(super) fn life_of(pol: &Politics, polity: &Polities, lost: &[Vec<(usize, u32)>], t: &Times, top: usize, c: usize, month: i32, last: Option<(f32, Option<bool>)>, hunger: f32, ruler: (f32, f32, bool)) -> Life {
    let regions = regions_of(pol, c);
    let all = regions.iter().map(|r| r.1).sum::<f32>().max(1e-6);
    let state = polity.of[c].state_faith;
    let foreign = regions.iter().filter(|r| polity.region_faith.get(r.0).is_some_and(|&f| f != state && f != usize::MAX)).map(|r| r.1).sum::<f32>() / all;
    // (regions it took from others: some other country lost them)
    let taken = regions.iter().filter(|r| lost.iter().enumerate().any(|(o, l)| o != c && l.iter().any(|x| x.0 == r.0))).map(|r| r.1).sum::<f32>() / all;
    let nb = pol.neighbors(c);
    let free = if nb.is_empty() { 0.0 } else { nb.iter().filter(|&&o| polity.of[o].laws[polity::VOICE] >= 1).count() as f32 / nb.len() as f32 };
    Life {
        month,
        dev: t.dev,
        stage: t.stage as f32 / top.max(1) as f32,
        industry: None,
        at_war: false,
        last_war: last,
        grudge: t.grudge,
        foreign,
        taken,
        hunger,
        ruler_reform: ruler.0,
        ruler_hawk: ruler.1,
        ruler_general: ruler.2,
        glory: t.glory.iter().sum::<f32>().clamp(-1.0, 1.0),
        free_near: free,
        new_ruler: None,
        moods: None,
        span: 1.0,
        schooling: None,
        wealth: None,
    }
}

/// A year's hunger in country `c` of development `dev` (0..1): the poor go hungry; now and then a
/// famine (bad harvests, a plague) — in words, if one came.
pub(super) fn hunger_of(dev: f32, rng: &mut Rng) -> (f32, bool) {
    if rng.f32() < 0.035 * (1.2 - dev) {
        (0.5 + 0.3 * rng.f32(), true)
    } else {
        ((0.35 - dev).max(0.0) * 0.6, false)
    }
}

/// What broke in country `c` laid on the world: a fallen government gets a new ruler (a person of
/// the world in the last century, a name in the deep past), a schism or a prophet a new faith, a
/// persecution converts dissenting regions.
#[allow(clippy::too_many_arguments)]
pub(super) fn upheave(pack: &Pack, pol: &Politics, polity: &mut Polities, folk: Option<&mut Folk>, t: &Times, c: usize, month: i32, ups: &[Upheaval], deep_ruler: &mut [(f32, f32)], rng: &mut Rng) {
    let name = pol.countries[c].name.clone();
    let mut folk = folk;
    for u in ups {
        match u {
            Upheaval::Revolution { to, .. } | Upheaval::Coup { from: to } => {
                let to = if matches!(u, Upheaval::Coup { .. }) { polity::Regime::Junta } else { *to };
                let how = if matches!(u, Upheaval::Coup { .. }) { "военный переворот" } else { "революция" };
                let ruler = match folk.as_deref_mut() {
                    Some(f) => f.overthrow(pack, c, t, month, how, to.soldiers(), rng).map(|id| f.get(id).name.clone()),
                    None => {
                        deep_ruler[c] = (0.4 * rng.normal(), 0.4 * rng.normal());
                        Some(names::person(rng))
                    }
                };
                if let Some(r) = ruler {
                    polity.crown(c, &r, month);
                    if let Some(f) = folk.as_deref_mut() {
                        f.changed[c] = None;
                    }
                }
            }
            Upheaval::Grown { from, to } => {
                // (generals, priests or a party leave: a civilian rules; a chief crowned rules on)
                let leaves = matches!(from, polity::Regime::Junta | polity::Regime::Theocracy | polity::Regime::Party);
                let ruler = match folk.as_deref_mut() {
                    Some(f) if leaves => f.overthrow(pack, c, t, month, "передача власти", to.soldiers(), rng).map(|id| f.get(id).name.clone()),
                    Some(f) => f.offices[c].ruler.map(|id| f.get(id).name.clone()),
                    None => {
                        if leaves {
                            deep_ruler[c] = (0.4 * rng.normal(), 0.4 * rng.normal());
                        }
                        Some(names::person(rng))
                    }
                };
                if let Some(r) = ruler {
                    polity.crown(c, &r, month);
                }
            }
            Upheaval::Schism | Upheaval::NewFaith => {
                let regions: Vec<usize> = regions_of(pol, c).into_iter().map(|r| r.0).collect();
                let cap = pol.countries[c].capital;
                let cap_region = if cap == crate::world::map::NONE { usize::MAX } else { pol.prov[cap as usize].state as usize };
                let new = matches!(u, Upheaval::NewFaith);
                // (the one who leads it: a person of the world if there is one to lead it)
                let founder = folk
                    .as_deref()
                    .and_then(|f| f.people.iter().filter(|p| p.country == c && p.alive() && matches!(p.calling, Calling::Scientist | Calling::Statesman)).max_by(|a, b| (a.reform * a.talent).total_cmp(&(b.reform * b.talent))).map(|p| p.name.clone()))
                    .unwrap_or_else(|| names::person(rng));
                let old = polity.of[c].state_faith;
                let f = polity.split(c, &regions, cap_region, new, month, founder.clone(), rng);
                let line = if new {
                    format!("{name}: явилось новое учение — «{}»; его пророк {founder}", polity.faiths[f].name)
                } else {
                    format!("{name}: раскол — от «{}» отделился «{}» (вёл {founder})", polity.faiths.get(old).map_or("", |x| x.name.as_str()), polity.faiths[f].name)
                };
                polity.of[c].story.push((month, line.clone()));
                polity.chronicle.push((month, c, line));
                polity.of[c].moves[polity::ZEALOTS] = (polity.of[c].moves[polity::ZEALOTS] + 0.2).min(1.0);
            }
            Upheaval::Persecution => {
                let state = polity.of[c].state_faith;
                for (r, _) in regions_of(pol, c) {
                    if polity.region_faith.get(r).is_some_and(|&f| f != state && f != usize::MAX) && rng.f32() < 0.3 {
                        polity.region_faith[r] = state;
                    }
                }
                polity.of[c].drift -= 0.01;
            }
            _ => {}
        }
    }
}

/// The faiths' year in the world: regions converted; a country whose people turned to another
/// faith takes it in time (a secular state at once, as most believe).
pub(super) fn faith_year(pol: &Politics, polity: &mut Polities, dev: &[f32], month: i32, rng: &mut Rng) {
    let owner = |r: usize| -> Option<usize> {
        let c = pol.states.get(r)?.country;
        (c != crate::world::map::NONE).then_some(c as usize)
    };
    let next = |r: usize| -> Vec<usize> { pol.states[r].neighbors.iter().map(|&x| x as usize).collect() };
    polity.spread(&owner, &next, dev, rng);
    for c in 0..pol.countries.len() {
        if pol.countries[c].states.is_empty() {
            continue;
        }
        let shares = polity.shares(&regions_of(pol, c));
        let Some(&(top, share)) = shares.first() else { continue };
        let p = &polity.of[c];
        if top != p.state_faith && share > 0.5 && (p.laws[polity::CHURCH] == 2 || rng.f32() < 0.08) {
            let name = &pol.countries[c].name;
            let line = format!("{name} {} «{}»", crate::world::names::agree(name, "приняла"), polity.faiths[top].name);
            let p = &mut polity.of[c];
            p.state_faith = top;
            p.story.push((month, line.clone()));
            polity.chronicle.push((month, c, line));
        }
    }
}

/// The order of the world at the beginning of the deep past: the old faiths over the map, a form
/// of government for every country.
pub(super) fn first_order(pol: &Politics, dev: &[f32], month: i32, rng: &mut Rng) -> Polities {
    let mut polity = Polities::default();
    let live = |r: usize| !pol.states[r].waste && pol.states[r].country != crate::world::map::NONE && pol.states[r].pop > 0.0;
    let next = |r: usize| -> Vec<usize> { pol.states[r].neighbors.iter().map(|&x| x as usize).collect() };
    polity.first_faiths(pol.states.len(), &live, &next, month, rng);
    for c in 0..pol.countries.len() {
        let shares = polity.shares(&regions_of(pol, c));
        let faith = shares.first().map_or(0, |x| x.0);
        let mut p = polity::Polity::first(dev[c], faith, month, rng);
        if p.regime.crowned() {
            p.house = Some(format!("дом {}", names::person(rng).split_once(". ").map_or("", |x| x.1)));
            p.house_since = month - (rng.f32() * 900.0) as i32;
        }
        polity.of.push(p);
    }
    polity
}

/// The world's relations this year, as its diplomacy sees them.
#[allow(clippy::too_many_arguments)]
pub(super) fn diplomacy_year(pol: &Politics, polity: &Polities, diplo: &mut Diplomacy, lost: &[Vec<(usize, u32)>], names: &[String], hawk: &dyn Fn(usize) -> f32, year: u32, month: i32, rng: &mut Rng) {
    let n = pol.countries.len();
    let apart = |a: usize, b: usize| polity.distance(polity.of[a].state_faith, polity.of[b].state_faith);
    let hold = |a: usize, b: usize| holds(pol, lost, a, b, year);
    let sc = Scene {
        names,
        live: (0..n).map(|c| !pol.countries[c].states.is_empty()).collect(),
        power: (0..n).map(|c| pol.gdp(c).max(1e-6)).collect(),
        neighbours: (0..n).map(|c| pol.neighbors(c)).collect(),
        regime: (0..n).map(|c| polity.of[c].regime).collect(),
        house: (0..n).map(|c| polity.of[c].house.clone()).collect(),
        apart: &apart,
        holds: &hold,
        trade: (0..n).map(|c| polity.of[c].clout[polity::MERCHANTS] + polity.of[c].clout[polity::CAPITAL]).collect(),
        zeal: (0..n).map(|c| polity.of[c].moves[polity::ZEALOTS]).collect(),
        hawk: (0..n).map(hawk).collect(),
    };
    diplo.year(&sc, month, 1.0, rng);
}

/// The regions of country `c`, each with its people.
fn regions_of(pol: &Politics, c: usize) -> Vec<(usize, f32)> {
    pol.countries[c].states.iter().map(|&s| (s as usize, pol.states[s as usize].pop)).collect()
}
