//! The wars of the past: who goes to war with whom and why, the coalitions, small wars fought by
//! power and luck, great wars fought on the map by front, the terms of peace, lands that broke
//! away, how a past war is told.

use super::*;

/// The might of a side as a government weighing a war sees it: its own economy and half of its
/// allies'.
fn might(pol: &Politics, side: &[usize]) -> f32 {
    side.iter().enumerate().map(|(i, &c)| pol.gdp(c) * if i == 0 { 1.0 } else { 0.5 }).sum()
}

/// How much of country `a`'s lost land `b` holds (0 … 1), fading over `CLAIMS` years; `year` —
/// now (years before the game).
pub(super) fn holds(pol: &Politics, lost: &[Vec<(usize, u32)>], a: usize, b: usize, year: u32) -> f32 {
    lost[a]
        .iter()
        .filter(|&&(st, _)| pol.states[st].country as usize == b)
        .map(|&(_, y)| (1.0 - y.saturating_sub(year) as f32 / CLAIMS).max(0.0) * 0.5)
        .sum::<f32>()
        .min(1.0)
}

/// Who would go to war with whom this year, and how readily: a neighbour that looks weaker with
/// its allies than the attacker with its own, or one holding its land, or one of another faith its
/// zealots hate, or one it has come to hate; a warlike ruler dares more often; pacts hold most back.
#[allow(clippy::too_many_arguments)]
pub(super) fn war_pairs(pol: &Politics, polity: &Polities, diplo: &Diplomacy, lost: &[Vec<(usize, u32)>], last: &[Option<u32>], hawk: &dyn Fn(usize) -> f32, year: u32, rng: &mut Rng) -> Vec<(usize, usize, f32)> {
    let n = pol.countries.len();
    let rested = |c: usize| last[c].is_none_or(|y| y.saturating_sub(year) >= REST) && diplo.war_of(c).is_none();
    let mut pairs = Vec::new();
    for a in 0..n {
        if pol.countries[a].states.is_empty() || !rested(a) {
            continue;
        }
        let bold = (hawk(a).max(0.0) + polity.of[a].moves[polity::HAWKS]).min(1.0);
        for b in pol.neighbors(a) {
            if !rested(b) || diplo.restraint(a, b, bold).is_some() {
                continue;
            }
            let [att, def] = diplo.coalition(a, b);
            let power = might(pol, &att) / might(pol, &def).max(1e-6) * (MISJUDGE * rng.normal()).exp();
            let grudge = holds(pol, lost, a, b, year) > 0.0;
            let hate = (-diplo.of(a, b)).max(0.0);
            let lust = polity.war_lust(a, b) * (1.0 + 1.5 * hate);
            if power * lust.sqrt() < DARE && !grudge {
                continue;
            }
            let hk = (1.0 + HAWKS * hawk(a)).clamp(0.15, 2.0);
            pairs.push((a, b, power.min(4.0).powi(2) * if grudge { 4.0 } else { 1.0 } * hk * lust));
        }
    }
    pairs
}

/// One of `pairs`, by their weights.
pub(super) fn pick_pair(pairs: &[(usize, usize, f32)], rng: &mut Rng) -> Option<(usize, usize)> {
    let all: f32 = pairs.iter().map(|p| p.2).sum();
    if all <= 0.0 {
        return None;
    }
    let mut pick = rng.f32() * all;
    pairs.iter().find(|p| {
        pick -= p.2;
        pick <= 0.0
    }).or(pairs.last()).map(|p| (p.0, p.1))
}

/// Why `a` goes to war against `b`, in words.
pub(super) fn cause_of(pol: &Politics, polity: &Polities, diplo: &Diplomacy, lost: &[Vec<(usize, u32)>], a: usize, b: usize, year: u32, month: i32, rng: &mut Rng) -> &'static str {
    let apart = polity.distance(polity.of[a].state_faith, polity.of[b].state_faith) > 0.6;
    if apart && polity.of[a].moves[polity::ZEALOTS] > 0.3 {
        "священная война"
    } else if diplo.has(Kind::Marriage, a, b) && polity.of[b].house_since > month - 120 || polity.of[b].house.is_some() && polity.of[b].house_since > month - 60 {
        "война за наследство"
    } else if holds(pol, lost, a, b, year) > 0.0 {
        "война за утраченные земли"
    } else if crate::world::diplomacy::kinship(polity.of[a].regime, polity.of[b].regime) < -0.2 {
        "война идей"
    } else if rng.f32() < 0.5 {
        "пограничная война"
    } else {
        "война за дань"
    }
}

/// A war of the world fought by no front to its end: its power and luck decide, month by month
/// (at most `SMALL_MONTHS`). The winning side, whether the beaten gave up, the months.
fn settle(diplo: &mut Diplomacy, w: usize, pol: &Politics, rng: &mut Rng) -> (Option<usize>, bool, u32) {
    let power = |c: usize| war_power(pol, c);
    for m in 1..=SMALL_MONTHS {
        if let Some((win, harsh)) = diplo.war_month(w, &power, rng) {
            return (win, harsh, m);
        }
    }
    let s = diplo.wars[w].score;
    (if s > 0.3 { Some(0) } else if s < -0.3 { Some(1) } else { None }, false, SMALL_MONTHS)
}

/// What a country brings to a war fought by no front: its economy, but a great one less than its
/// size (it guards every border, its far provinces send little).
pub(super) fn war_power(pol: &Politics, c: usize) -> f32 {
    pol.gdp(c).max(1e-6).powf(WAR_SCALE)
}

/// The regions the beaten `from` cedes to `to` at the peace: of those bordering the victor's
/// land (never its capital's), first those it once took from the victor, then the most peopled;
/// one, two if it gave up.
fn terms(pol: &Politics, lost: &[Vec<(usize, u32)>], from: usize, to: usize, harsh: bool) -> Vec<usize> {
    let cap = pol.countries[from].capital;
    let cap_region = if cap == crate::world::map::NONE { u32::MAX } else { pol.prov[cap as usize].state };
    let mut cands: Vec<(usize, f32)> = pol.countries[from]
        .states
        .iter()
        .filter(|&&s| s != cap_region && pol.states[s as usize].neighbors.iter().any(|&t| pol.states[t as usize].country as usize == to))
        .map(|&s| {
            let s = s as usize;
            let ours = lost[to].iter().any(|x| x.0 == s);
            (s, pol.states[s].pop + if ours { 100.0 } else { 0.0 })
        })
        .collect();
    cands.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let _ = harsh;
    cands.into_iter().take(1).map(|x| x.0).collect()
}

/// Region `st` passes from `from` to `to` in `year` (years before the game).
pub(super) fn move_region(pol: &mut Politics, lost: &mut [Vec<(usize, u32)>], st: usize, from: usize, to: usize, year: u32) {
    pol.countries[from].states.retain(|&x| x as usize != st);
    pol.countries[to].states.push(st as u32);
    pol.states[st].country = to as u32;
    lost[from].push((st, year));
    lost[to].retain(|&(x, _)| x != st);
}

/// The end of war `w` of the world (fought by no front): the peace, the regions ceded, the war
/// written into the past.
#[allow(clippy::too_many_arguments)]
pub(super) fn conclude(diplo: &mut Diplomacy, w: usize, win: Option<usize>, harsh: bool, months: u32, pol: &mut Politics, lost: &mut [Vec<(usize, u32)>], last: &mut [Option<u32>], year: u32, names: &[String], rng: &mut Rng) -> PastWar {
    let begun = diplo.wars[w].since;
    let began_year = (-begun / 12).max(0) as u32;
    let war = diplo.end(w, win, harsh, begun + months as i32, names);
    let (a, b) = (war.sides[0][0], war.sides[1][0]);
    let mut took = Vec::new();
    // (a victory fought by no front takes a region when the beaten gave up, or now and then;
    // else the peace is tribute and money)
    if let (Some(s), true) = (win, harsh || rng.f32() < TAKE_LAND) {
        let (to, from) = (war.sides[s][0], war.sides[1 - s][0]);
        for st in terms(pol, lost, from, to, harsh) {
            took.push((st, pol.states[st].pop, s));
            move_region(pol, lost, st, from, to, year);
        }
    }
    for s in 0..2 {
        for &c in &war.sides[s] {
            last[c] = Some(year);
        }
    }
    PastWar {
        year: began_year,
        sides: [a, b],
        names: [names[a].clone(), names[b].clone()],
        months,
        winner: win,
        capitulated: harsh,
        took,
        battles: 0,
        generals: [Vec::new(), Vec::new()],
    }
}

/// Regions that rise against the country that took them and go back to the one they were taken
/// from (a neighbour, at peace): where the separatists are strong.
#[allow(clippy::too_many_arguments)]
pub(super) fn break_away(pol: &mut Politics, polity: &mut Polities, diplo: &mut Diplomacy, lost: &mut [Vec<(usize, u32)>], year: u32, month: i32, names: &[String], rng: &mut Rng) {
    for c in 0..pol.countries.len() {
        if pol.countries[c].states.len() < 2 || polity.of[c].moves[polity::SEPARATISTS] < 0.3 || diplo.war_of(c).is_some() {
            continue;
        }
        if rng.f32() >= 0.25 * polity.of[c].moves[polity::SEPARATISTS] {
            continue;
        }
        let cap = pol.countries[c].capital;
        let cap_region = if cap == crate::world::map::NONE { u32::MAX } else { pol.prov[cap as usize].state };
        let back = pol.countries[c].states.iter().copied().filter(|&s| s != cap_region).find_map(|s| {
            let s = s as usize;
            (0..lost.len()).find(|&o| {
                o != c && !pol.countries[o].states.is_empty() && diplo.war_of(o).is_none() && lost[o].iter().any(|x| x.0 == s && x.1.saturating_sub(year) as f32 <= CLAIMS) && pol.states[s].neighbors.iter().any(|&t| pol.states[t as usize].country as usize == o)
            }).map(|o| (s, o))
        });
        if let Some((st, o)) = back {
            let line = format!("{}: отнятый край ({:.1} млн людей) восстал и вернулся к прежней хозяйке — {}", names[c], pol.states[st].pop, names[o]);
            move_region(pol, lost, st, c, o, year);
            // (it went back to its own: no claim is left of it)
            lost[o].retain(|x| x.0 != st);
            lost[c].retain(|x| x.0 != st);
            let p = &mut polity.of[c];
            p.moves[polity::SEPARATISTS] = (p.moves[polity::SEPARATISTS] - 0.3).max(0.0);
            p.legit = (p.legit - 0.08).max(0.05);
            p.story.push((month, line.clone()));
            polity.chronicle.push((month, c, line.clone()));
            diplo.chronicle.push((month, c, line));
        }
    }
}

/// A war of the world begun this year (fought by no front): who fights it; ended at once, unless
/// it begins in the last years of the past (then it goes on into the game).
#[allow(clippy::too_many_arguments)]
pub(super) fn small_war(pol: &mut Politics, polity: &mut Polities, diplo: &mut Diplomacy, lost: &mut [Vec<(usize, u32)>], last: &mut [Option<u32>], wars: &mut Vec<PastWar>, names: &[String], hawk: &dyn Fn(usize) -> f32, year: u32, month: i32, rng: &mut Rng) {
    let pairs = war_pairs(pol, polity, diplo, lost, last, hawk, year, rng);
    let Some((a, b)) = pick_pair(&pairs, rng) else { return };
    let cause = cause_of(pol, polity, diplo, lost, a, b, year, month, rng);
    let sides = diplo.coalition(a, b);
    let w = diplo.begin(sides, month, cause, names, rng);
    for line in polity_war_lines(diplo, w, names) {
        polity.of[a].story.push((month, line));
    }
    if year > OPEN {
        let (win, harsh, months) = settle(diplo, w, pol, rng);
        wars.push(conclude(diplo, w, win, harsh, months, pol, lost, last, year, names, rng));
    }
}

/// The words of the war the countries of war `w` keep in their stories.
fn polity_war_lines(diplo: &Diplomacy, w: usize, names: &[String]) -> Vec<String> {
    let war = &diplo.wars[w];
    vec![format!("{} против {} — {}", names[war.sides[0][0]], names[war.sides[1][0]], war.cause)]
}

/// A war of the past between `pair` (the first began it), fought on the map with the countries'
/// own staffs, which it gives back changed, and their generals — people of the world, whose lives
/// it changes too.
#[allow(clippy::too_many_arguments)]
pub(super) fn fight(pack: &Pack, world: &World, pol: &Politics, lost: &[Vec<(usize, u32)>], staffs: &mut [Country], pair: [usize; 2], money: [f32; 2], year: u32, seed: u64, _rng: &mut Rng, nu: usize, folk: &mut Folk, times: &[Times; 2], prng: &mut Rng) -> PastWar {
    let [a, b] = pair;
    let month = -(year as i32) * 12;
    let th = Theatre::new(world, pol, a, b);
    let mut sides = [staffs[a].clone(), staffs[b].clone()];
    for s in &mut sides {
        // the army of the last war went home long ago
        s.army.count = vec![0.0; nu];
    }
    let mut c = Campaign::new(pack, th, sides, money, seed);
    let mut ids: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
    for s in 0..2 {
        c.by_groups(s, true);
        if let Some(gs) = c.command[s].as_mut() {
            let gens = folk.war_generals(pack, pair[s], &times[s], gs.len(), month, prng);
            for (g, (id, mut gen)) in gs.iter_mut().zip(gens) {
                gen.past.push(format!("{} лет назад воевал против {}", year, pol.countries[pair[1 - s]].name));
                g.general = Some(gen);
                ids[s].push(id);
            }
        }
    }
    for _ in 0..WAR_MONTHS {
        c.step(pack);
        if c.fallen.is_some() {
            break;
        }
    }
    // who won: the one that did not give up; without a capitulation, the one holding clearly more
    // of the other's people than it lost of its own
    let held = c.people();
    let winner = match c.fallen {
        Some(s) => Some(1 - s),
        None if held[0] - held[1] > 0.1 => Some(0),
        None if held[1] - held[0] > 0.1 => Some(1),
        None => None,
    };
    // the regions each side holds the most of, of the other's (not the other's capital's); from a
    // country that gave up, those it holds a third of
    // (the peace leaves the beaten most of its land: the world would not suffer a victor taking
    // everything — at most a third of its regions if it gave up, a fifth if not; first those the
    // victor once lost, then those it holds the most of)
    let mut took = Vec::new();
    for s in 0..2 {
        let o = 1 - s;
        let cap = pol.prov[pol.countries[pair[o]].capital as usize].state;
        let given = c.fallen == Some(o);
        let mut cands: Vec<(usize, f32)> = Vec::new();
        for &st in &pol.countries[pair[o]].states {
            if st == cap {
                continue;
            }
            let provs: Vec<usize> = pol.states[st as usize].provinces.iter().map(|&p| p as usize).filter(|&p| c.th.at[p] != crate::military::front::NO).collect();
            if provs.is_empty() {
                continue;
            }
            let ours = provs.iter().filter(|&&p| c.owner[c.th.at[p] as usize] == s).count();
            if (given && ours * 3 >= provs.len() && ours > 0) || ours * 2 > provs.len() {
                let back = if lost[pair[s]].iter().any(|x| x.0 == st as usize) { 1.0 } else { 0.0 };
                cands.push((st as usize, back + ours as f32 / provs.len() as f32));
            }
        }
        cands.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let most = ((pol.countries[pair[o]].states.len() as f32 * if given { 0.34 } else { 0.2 }).ceil() as usize).max(1);
        for (st, _) in cands.into_iter().take(most) {
            took.push((st, pol.states[st].pop, s));
        }
    }
    let mut sides = c.sides.clone();
    for (s, st) in sides.iter_mut().enumerate() {
        match winner {
            Some(w) => st.conclude(pack, w == s),
            None => st.doubt = st.doubt.max(0.5),
        }
    }
    let [sa, sb] = sides;
    staffs[a] = sa;
    staffs[b] = sb;
    let generals = [0, 1].map(|s| {
        let mut gs: Vec<General> = c.command[s].as_ref().map_or(Vec::new(), |gs| gs.iter().filter_map(|g| g.general.clone()).collect());
        let end = match winner {
            Some(w) if w == s => "победа",
            Some(_) => "поражение",
            None => "мир без победителя",
        };
        for g in &mut gs {
            if let Some(line) = g.past.last_mut() {
                *line = format!("{line} ({end}): побед {}, поражений {}, взял провинций {}", g.won, g.lost, g.took);
            }
        }
        gs
    });
    // the generals' lives: their records, fame, comrades, the fallen
    for s in 0..2 {
        let pairs: Vec<(u32, General)> = ids[s].iter().copied().zip(generals[s].iter().cloned()).collect();
        let won = winner.map(|w| w == s);
        folk.after_war(pair[s], &pairs, won, &pol.countries[pair[1 - s]].name, month + c.month as i32, prng);
    }
    PastWar {
        year,
        sides: pair,
        names: [pol.countries[a].name.clone(), pol.countries[b].name.clone()],
        months: c.month,
        winner,
        capitulated: c.fallen.is_some(),
        took,
        battles: c.fought.len(),
        generals,
    }
}

impl PastWar {
    /// The war in a few words, numbers left for the note (only how long ago).
    pub fn brief(&self) -> String {
        let [a, b] = &self.names;
        let how = match (self.winner, self.capitulated) {
            (Some(w), true) if self.battles == 0 => format!("{} {} поражение", self.names[1 - w], crate::world::names::agree(&self.names[1 - w], "признала")),
            (Some(w), true) => format!("{} {}", self.names[1 - w], crate::world::names::agree(&self.names[1 - w], "капитулировала")),
            (Some(w), false) => format!("мир на условиях {}", self.names[w]),
            (None, _) => "мир без победителя".to_string(),
        };
        let land: Vec<String> = (0..2).filter(|&s| self.took.iter().any(|t| t.2 == s)).map(|s| format!("{} {} земли", self.names[s], crate::world::names::agree(&self.names[s], "взяла"))).collect();
        let long = match self.months {
            0..=6 => "короткая война",
            7..=18 => "война",
            _ => "долгая война",
        };
        format!(
            "{} {} назад {} {} на {} — {}, {}{}",
            self.year,
            plural(self.year as usize, "год", "года", "лет"),
            a,
            crate::world::names::agree(a, "напала"),
            b,
            long,
            how,
            if land.is_empty() { String::new() } else { format!("; {}", land.join(", ")) }
        )
    }

    /// The war in words, as a chronicle tells it.
    pub fn words(&self) -> String {
        let [a, b] = &self.names;
        let how = match (self.winner, self.capitulated) {
            (Some(w), true) if self.battles == 0 => format!("{} {} поражение и платит дань", self.names[1 - w], crate::world::names::agree(&self.names[1 - w], "признала")),
            (Some(w), true) => format!("{} {}", self.names[1 - w], crate::world::names::agree(&self.names[1 - w], "капитулировала")),
            (Some(w), false) => format!("мир на условиях {}", self.names[w]),
            (None, _) => "мир без победителя".to_string(),
        };
        let mut land = String::new();
        for s in 0..2 {
            let (k, people) = self.took.iter().filter(|t| t.2 == s).fold((0, 0.0f32), |(k, p), t| (k + 1, p + t.1));
            if k > 0 {
                land += &format!("; {} {} {} {} ({:.1} млн людей)", self.names[s], crate::world::names::agree(&self.names[s], "взяла"), k, plural(k, "область", "области", "областей"), people);
            }
        }
        format!("{} {} назад {} {} на {}: {} мес., {}{}", self.year, plural(self.year as usize, "год", "года", "лет"), a, crate::world::names::agree(a, "напала"), b, self.months, how, land)
    }
}
