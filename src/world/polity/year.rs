//! A year of a country's order: how far its economy has come, how many read, how pious it is, the
//! power and mood of each force of society, the movements, the unrest and the legitimacy — and what
//! breaks: a revolution, a coup, a reform won, a schism, a new faith, persecution, a house that dies
//! out.

use super::*;
use super::faith::{CHARITY, COMMERCE, HIERARCHY, LEARNING, THRIFT, TOLERANCE, WAR};

/// What a country lives through in a year, for its order.
#[derive(Clone, Debug, Default)]
pub struct Life {
    pub month: i32,
    /// Its development against the world's (0..1).
    pub dev: f32,
    /// How far through the era's stages it has come (0..1).
    pub stage: f32,
    /// Its economy's share of industry, if known (the game knows it; the past guesses it).
    pub industry: Option<f32>,
    pub at_war: bool,
    /// Years since its last war ended and whether it won (None: a draw).
    pub last_war: Option<(f32, Option<bool>)>,
    /// A neighbour holds land it lost.
    pub grudge: bool,
    /// Shares of its people in regions of another faith than the state's, and in regions it took
    /// from others.
    pub foreign: f32,
    pub taken: f32,
    /// How hungry its people are (0..1).
    pub hunger: f32,
    /// Its ruler's beliefs (reform, war, the state — each −1 … +1) and whether he is a soldier.
    pub ruler_reform: f32,
    pub ruler_hawk: f32,
    pub ruler_general: bool,
    /// Its staff's pride (the sum of its glory, about −1 … +1).
    pub glory: f32,
    /// The share of its neighbours where the people have a voice (ideas cross borders).
    pub free_near: f32,
    /// A new ruler came this year: his surname, and whether he is the old one's son.
    pub new_ruler: Option<(String, bool)>,
    /// What the game's levers do to each force's mood (the game's own reckoning), if it reckons it.
    pub moods: Option<[f32; BLOCS]>,
    /// The years this step covers (0: one; the game steps a month, 1/12).
    pub span: f32,
    /// What the game's own reckoning knows better than the past's guess (none: the guess): how
    /// well the schools are paid and run (times the law's custom — the game's school budget), and
    /// each force's share of the country's organised economic power (its incomes, its numbers in
    /// towns, how many of them read; summing to 1 over the forces of the economy, the clergy and
    /// the officers aside).
    pub schooling: Option<f32>,
    pub wealth: Option<[f32; BLOCS]>,
}

impl Life {
    fn years(&self) -> f32 {
        if self.span > 0.0 {
            self.span
        } else {
            1.0
        }
    }
}

/// What broke in a year.
#[derive(Clone, Debug, PartialEq)]
pub enum Upheaval {
    Revolution { from: Regime, to: Regime, by: usize },
    Coup { from: Regime },
    /// The form of government grew into another without a fall (chiefs crowned, generals back
    /// to their barracks, estates made a parliament).
    Grown { from: Regime, to: Regime },
    Reform { law: usize, from: u8, to: u8 },
    Schism,
    NewFaith,
    Persecution,
    HouseEnds,
}

fn toward(x: &mut f32, target: f32, pace: f32) {
    *x += (target - *x) * pace;
}

/// A pace a year, for a step of `years`.
fn per(pace: f32, years: f32) -> f32 {
    1.0 - (1.0 - pace.clamp(0.0, 1.0)).powf(years)
}

fn surname(name: &str) -> String {
    name.split_once(". ").map_or(name, |x| x.1).to_string()
}

impl Polities {
    /// A year of country `c` (named `name`): its order follows what it lives through; what broke.
    pub fn year(&mut self, c: usize, name: &str, life: &Life, rng: &mut Rng) -> Vec<Upheaval> {
        let mut out = self.drift(c, name, life, rng);
        out.extend(self.breaks(c, name, life, rng));
        out
    }

    /// How country `c`'s order follows what it lives through (`life.span` years of it): its
    /// economy, readers and faithful, the forces' power and mood, the movements, the unrest, the
    /// legitimacy, its development's drift; a house that died out.
    pub fn drift(&mut self, c: usize, name: &str, life: &Life, rng: &mut Rng) -> Vec<Upheaval> {
        let yrs = life.years();
        let tenets = self.faiths.get(self.of[c].state_faith).map_or([0.0; super::faith::TENETS], |f| f.tenets);
        let p = &mut self.of[c];
        let mut out = Vec::new();
        let month = life.month;
        let recent = |within: f32, won: Option<bool>| life.last_war.is_some_and(|(y, w)| y < within && w == won);
        let lost = if recent(20.0, Some(false)) { 1.0 - life.last_war.map_or(20.0, |x| x.0) / 20.0 } else { 0.0 };
        let won = if recent(15.0, Some(true)) { 1.0 - life.last_war.map_or(15.0, |x| x.0) / 15.0 } else { 0.0 };
        let war_near = life.last_war.map_or(0.0, |(y, _)| (1.0 - y / 15.0).max(0.0)) + if life.at_war { 1.0 } else { 0.0 };
        // the economy, the readers, the faithful
        let industry = life.industry.unwrap_or((0.75 * life.stage + 0.25 * life.dev).clamp(0.0, 1.0));
        toward(&mut p.industry, industry, if life.industry.is_some() { 1.0 } else { per(0.1, yrs) });
        let schools = life.schooling.unwrap_or([0.7, 1.0, 1.5][p.laws[SCHOOLS].min(2) as usize]);
        let lit = ((0.05 + 0.35 * life.dev + 0.35 * p.industry) * schools * (1.0 + 0.25 * tenets[LEARNING] * p.piety)).clamp(0.02, 0.97);
        toward(&mut p.literacy, lit, per(0.04, yrs));
        let church = p.laws[CHURCH];
        let pious = (0.9 - 0.4 * p.literacy - 0.2 * p.industry + if church == 0 { 0.12 } else if church == 2 { -0.15 } else { 0.0 } + 0.12 * lost).clamp(0.05, 1.0);
        toward(&mut p.piety, pious, per(0.03, yrs));
        // the forces' power
        let l = |k: usize| p.laws[k] as f32 / 2.0;
        let mut raw = [0.0f32; BLOCS];
        raw[NOBLES] = 0.2 * (1.0 - p.industry) * (1.0 + 0.4 * (1.0 - l(LABOUR)) + 0.4 * (1.0 - l(LAND)));
        raw[CLERGY] = 0.15 * p.piety * (1.0 + if church == 0 { 0.6 } else { 0.0 }) * (0.7 + 0.3 * tenets[HIERARCHY]);
        raw[OFFICERS] = 0.06 + 0.1 * war_near.min(1.0) + 0.05 * life.glory.clamp(-1.0, 1.0) + 0.06 * l(SERVICE);
        raw[CAPITAL] = 0.03 + 0.3 * p.industry * if p.laws[LABOUR] == 2 { 1.0 } else { 0.6 };
        raw[MERCHANTS] = (0.05 + 0.08 * life.dev + 0.05 * tenets[COMMERCE]).max(0.02);
        raw[LEARNED] = 0.02 + 0.2 * p.literacy;
        raw[WORKERS] = 0.02 + 0.3 * p.industry * (0.5 + p.literacy);
        raw[PEASANTS] = 0.25 * (1.0 - p.industry) * (0.4 + 0.6 * l(LABOUR));
        let all: f32 = raw.iter().sum::<f32>().max(1e-6);
        let mut aim = raw.map(|x| x / all);
        // (where the game knows who holds the money and the towns, power follows it half way: the
        // forces of the economy keep their share between them, and split it by their weight)
        if let Some(w) = life.wealth {
            let econ = [NOBLES, CAPITAL, MERCHANTS, LEARNED, WORKERS, PEASANTS];
            let held: f32 = econ.iter().map(|&b| aim[b]).sum();
            let wsum: f32 = econ.iter().map(|&b| w[b].max(0.0)).sum::<f32>();
            if wsum > 1e-6 {
                for &b in &econ {
                    aim[b] = 0.5 * aim[b] + 0.5 * held * w[b].max(0.0) / wsum;
                }
            }
        }
        for b in 0..BLOCS {
            toward(&mut p.clout[b], aim[b], per(0.15, yrs));
        }
        // their mood: the laws, what happened, whether they are heard
        let heard = p.heard();
        let voice_all: f32 = (0..BLOCS).map(|b| p.clout[b]).sum::<f32>().max(1e-6);
        for b in 0..BLOCS {
            let w: f32 = WANTS[b].iter().map(|x| if x.0 == 9 { 0.0 } else { x.1 }).sum::<f32>().max(1e-6);
            let laws: f32 = (0..LAWS).map(|k| law_fit(b, k, p.laws[k])).sum::<f32>() / w;
            let shut = ((p.clout[b] / voice_all - heard[b]) / (p.clout[b] / voice_all).max(1e-6)).max(0.0);
            let mut m = 0.6 * laws - 0.35 * shut;
            match b {
                OFFICERS => m += 0.35 * won - 0.45 * lost,
                CLERGY => m += 0.2 * (p.piety - 0.5) - 0.4 * life.foreign + 0.2 * tenets[HIERARCHY],
                WORKERS | PEASANTS => m -= 0.6 * life.hunger + 0.1 * war_near.min(1.0) - 0.15 * tenets[CHARITY].max(0.0),
                MERCHANTS => m -= 0.25 * war_near.min(1.0) - 0.15 * tenets[COMMERCE],
                CAPITAL => m += 0.1 * tenets[THRIFT],
                _ => {}
            }
            if p.regime.repression() > 0.5 && (b == LEARNED || b == WORKERS) {
                m -= 0.2 * p.regime.repression();
            }
            let target = match life.moods {
                Some(g) => 0.5 * m + 0.5 * g[b],
                None => m,
            };
            toward(&mut p.mood[b], target.clamp(-1.0, 1.0), per(0.25, yrs));
        }
        // the movements
        let zeal = (0.5 + 0.5 * tenets[WAR] - 0.3 * tenets[TOLERANCE]).clamp(0.0, 1.5);
        let hurt = |b: usize| (-p.mood[b]).max(0.0);
        let mut target = [0.0f32; MOVES];
        target[REVANCHE] = 0.55 * if life.grudge { 1.0 } else { 0.0 } + 0.35 * lost + 0.1 * hurt(OFFICERS);
        target[LIBERALS] = 0.75 * p.literacy * (1.0 - 0.4 * l(PRESS) * l(VOICE)) + 0.25 * life.free_near + 0.5 * (p.clout[LEARNED] + p.clout[MERCHANTS]) * (hurt(LEARNED) + hurt(MERCHANTS)) - 0.25 * p.regime.repression();
        target[LEVELLERS] = 1.2 * (p.clout[WORKERS] * hurt(WORKERS) + 0.6 * p.clout[PEASANTS] * hurt(PEASANTS)) + 0.45 * life.hunger + 0.12 * (1.0 - l(LABOUR)) * p.literacy;
        target[GUARDIANS] = 0.8 * p.piety * (p.clout[NOBLES] + p.clout[CLERGY]) + 0.25 * won - 0.2 * p.literacy + 0.4 * (hurt(NOBLES) * p.clout[NOBLES] + hurt(CLERGY) * p.clout[CLERGY]);
        target[ZEALOTS] = 0.5 * p.piety * zeal * (0.25 + life.foreign + 0.3 * lost + if church == 2 { 0.4 } else { 0.0 });
        target[HAWKS] = 1.2 * p.clout[OFFICERS] + 0.3 * won + 0.2 * life.ruler_hawk.max(0.0) + 0.2 * hurt(OFFICERS);
        target[SEPARATISTS] = 0.9 * life.taken + 0.6 * life.foreign * if church == 0 { 1.0 } else { 0.5 };
        for m in 0..MOVES {
            let t = (target[m] + 0.03 * rng.normal() * yrs.sqrt()).clamp(0.0, 1.0);
            toward(&mut p.moves[m], t, per(0.15, yrs));
        }
        // unrest and legitimacy
        let anger: f32 = (0..BLOCS).map(|b| p.clout[b] * hurt(b)).sum();
        let rebels = p.moves[LEVELLERS] + p.moves[ZEALOTS] + 0.5 * p.moves[SEPARATISTS] + 0.5 * p.moves[LIBERALS] * (1.0 - l(VOICE));
        let unrest = 0.08 + 2.2 * anger + 0.3 * life.hunger + 0.25 * rebels - 0.25 * p.regime.repression() - 0.25 * p.legit;
        toward(&mut p.unrest, unrest.clamp(0.0, 1.0), per(0.3, yrs));
        let house_years = ((month - p.house_since) as f32 / 12.0).max(0.0);
        let legit = match p.regime {
            Regime::Autocracy | Regime::Estates | Regime::Constitutional => 0.45 + 0.3 * p.piety + 0.2 * (house_years / 100.0).min(1.0) - 0.2 * p.moves[LIBERALS],
            Regime::Republic | Regime::Patricians => 0.45 + 0.25 * l(VOICE) + 0.2 * p.literacy - 0.2 * p.moves[GUARDIANS],
            Regime::Junta | Regime::Chiefs => 0.35 + 0.3 * won - 0.3 * lost + 0.1 * life.glory.clamp(-1.0, 1.0),
            Regime::Party => 0.45 + 0.2 * p.clout[WORKERS] + 0.1 * p.industry - 0.2 * p.moves[LIBERALS],
            Regime::Theocracy => 0.3 + 0.6 * p.piety,
        } - 0.3 * life.hunger
            - 0.2 * lost
            - 0.3 * p.unrest
            + 0.3 * p.content();
        toward(&mut p.legit, legit.clamp(0.05, 1.0), per(0.15, yrs));
        // the development: free hands, an open press, schools, a faith of work and trade help;
        // unrest, bondage and a chiefs' host hold it back
        let d = 0.002 * (l(LABOUR) - 0.5) * 2.0 + 0.0015 * l(PRESS) + 0.001 * l(SCHOOLS) - 0.004 * p.unrest + 0.001 * tenets[COMMERCE] + 0.001 * tenets[THRIFT] - if p.regime == Regime::Chiefs { 0.0015 } else { 0.0 };
        // (what the order did long ago fades: the drift remembers about the last century)
        p.drift = (p.drift * (1.0 - 0.01 * yrs) + d * yrs).clamp(-0.35, 0.35);
        // a house that died out
        if let Some((sur, heir)) = &life.new_ruler {
            if p.regime.crowned() {
                let new_house = format!("дом {}", sur);
                if !*heir && p.house.as_ref() != Some(&new_house) {
                    if let Some(old) = p.house.clone() {
                        let line = format!("{name}: пресёкся {old}; на престоле — {new_house}");
                        p.story.push((month, line.clone()));
                        self.chronicle.push((month, c, line));
                        let p = &mut self.of[c];
                        p.legit = (p.legit - 0.15).max(0.05);
                        out.push(Upheaval::HouseEnds);
                    }
                    let p = &mut self.of[c];
                    p.house = Some(new_house);
                    p.house_since = month;
                }
            }
        }
        out
    }

    /// What breaks in country `c` in `life.span` years: a revolution, a coup, a reform won, a
    /// schism, a prophet, persecution.
    pub fn breaks(&mut self, c: usize, name: &str, life: &Life, rng: &mut Rng) -> Vec<Upheaval> {
        let yrs = life.years();
        let tenets = self.faiths.get(self.of[c].state_faith).map_or([0.0; super::faith::TENETS], |f| f.tenets);
        let mut out = Vec::new();
        let month = life.month;
        let recent = |within: f32, won: Option<bool>| life.last_war.is_some_and(|(y, w)| y < within && w == won);
        let lost = if recent(20.0, Some(false)) { 1.0 - life.last_war.map_or(20.0, |x| x.0) / 20.0 } else { 0.0 };
        let p = &mut self.of[c];
        let church = p.laws[CHURCH];
        if (p.unrest > RESTLESS && p.legit < 0.55) || p.unrest > 0.8 {
            let chance = (p.unrest - 0.35) * 1.4 * (1.0 - p.legit) * yrs;
            if rng.f32() < chance {
                // the movement that leads it: of those that want a new order
                let cands = [(LIBERALS, p.moves[LIBERALS]), (LEVELLERS, p.moves[LEVELLERS]), (ZEALOTS, p.moves[ZEALOTS]), (GUARDIANS, p.moves[GUARDIANS] * 0.8), (HAWKS, p.moves[HAWKS] * 0.8)];
                let all: f32 = cands.iter().map(|x| x.1.max(0.01)).sum();
                let mut r = rng.f32() * all;
                let by = cands.iter().find(|x| {
                    r -= x.1.max(0.01);
                    r <= 0.0
                }).map_or(LIBERALS, |x| x.0);
                let to = match by {
                    LIBERALS if p.regime.crowned() && p.piety > 0.5 => Regime::Constitutional,
                    LIBERALS => Regime::Republic,
                    LEVELLERS if p.industry > 0.35 => Regime::Party,
                    LEVELLERS => Regime::Republic,
                    ZEALOTS => Regime::Theocracy,
                    GUARDIANS if p.regime.crowned() => Regime::Autocracy,
                    GUARDIANS => Regime::Autocracy,
                    _ => Regime::Junta,
                };
                if to != p.regime {
                    let from = p.regime;
                    self.change(c, name, to, by, month, rng);
                    out.push(if by == HAWKS { Upheaval::Coup { from } } else { Upheaval::Revolution { from, to, by } });
                    return out;
                }
            }
        }
        // forms of government age: chiefs become kings as the country grows; estates give way
        // to the crown or to a parliament; generals hand power back after some years; a
        // theocracy without the faithful falls
        let p = &mut self.of[c];
        let years_in = ((month - p.since) as f32 / 12.0).max(0.0);
        let grown = match p.regime {
            Regime::Chiefs if (life.dev > 0.3 || life.stage > 0.15) && rng.f32() < 0.04 * yrs => Some(if p.clout[NOBLES] > 0.25 { Regime::Estates } else { Regime::Autocracy }),
            Regime::Estates if p.clout[MERCHANTS] + p.clout[LEARNED] + p.clout[CAPITAL] > 0.3 && p.literacy > 0.25 && rng.f32() < 0.025 * yrs => Some(Regime::Constitutional),
            Regime::Estates if p.clout[NOBLES] < 0.15 && life.ruler_reform < 0.0 && rng.f32() < 0.012 * yrs => Some(Regime::Autocracy),
            Regime::Junta if years_in > 8.0 && rng.f32() < 0.07 * yrs => Some(match p.before {
                Some(r) if r.crowned() && p.literacy < 0.35 => Regime::Autocracy,
                Some(r) if r.crowned() => Regime::Constitutional,
                _ if p.literacy > 0.3 => Regime::Republic,
                _ if p.clout[CAPITAL] + p.clout[MERCHANTS] > 0.25 => Regime::Patricians,
                _ => Regime::Autocracy,
            }),
            Regime::Theocracy if p.piety < 0.55 && rng.f32() < 0.05 * yrs => Some(if p.literacy > 0.3 { Regime::Republic } else { Regime::Autocracy }),
            Regime::Party if years_in > 25.0 && p.moves[LIBERALS] > 0.35 && p.legit < 0.4 && rng.f32() < 0.04 * yrs => Some(Regime::Republic),
            _ => None,
        };
        if let Some(to) = grown {
            let from = p.regime;
            let words = match (from, to) {
                (Regime::Chiefs, _) => "сильнейший из вождей объявил себя государем",
                (Regime::Estates, Regime::Constitutional) => "сословное собрание стало парламентом",
                (Regime::Estates, _) => "государь распустил сословное собрание",
                (Regime::Junta, Regime::Autocracy) if p.before.is_some_and(|r| r.crowned()) => "генералы вернули престол",
                (Regime::Junta, Regime::Autocracy) => "диктатор провозгласил себя государем",
                (Regime::Junta, _) => "генералы ушли в казармы, власть отдана гражданским",
                (Regime::Theocracy, _) => "служители веры отстранены от власти",
                _ => "партия уступила власть выборному правительству",
            };
            p.before = Some(from);
            p.regime = to;
            p.since = month;
            p.legit = (p.legit + 0.15).min(0.8);
            let line = format!("{name}: {words} — теперь {}", to.name().to_lowercase());
            p.story.push((month, line.clone()));
            self.chronicle.push((month, c, line));
            out.push(Upheaval::Grown { from, to });
            return out;
        }
        // (officers seldom overthrow one of their own on the throne)
        let kin = if life.ruler_general { 0.4 } else { 1.0 };
        if p.regime != Regime::Junta && p.clout[OFFICERS] > 0.14 && p.mood[OFFICERS] < -0.3 && (lost > 0.3 || p.unrest > 0.4) && rng.f32() < 0.08 * kin * yrs {
            let from = p.regime;
            self.change(c, name, Regime::Junta, HAWKS, month, rng);
            out.push(Upheaval::Coup { from });
            return out;
        }
        let p = &mut self.of[c];
        // a reform: the strongest movement with aims, a government willing (or pressed) to grant it
        let closed = matches!(p.regime, Regime::Party | Regime::Junta | Regime::Theocracy);
        let mut best: Option<(usize, usize, u8, f32)> = None;
        for m in [LIBERALS, LEVELLERS, ZEALOTS, HAWKS] {
            for &(law, want) in MOVE[m].aims {
                let now = p.laws[law];
                if now == want {
                    continue;
                }
                // (a closed government opens neither the press nor the vote by reform)
                if closed && ((law == PRESS || law == VOICE) && want > now) {
                    continue;
                }
                let to = if want > now { now + 1 } else { now - 1 };
                let push = p.moves[m] * (0.5 + 0.5 * life.ruler_reform.max(0.0) + 0.5 * p.unrest);
                if best.is_none_or(|b| push > b.3) {
                    best = Some((m, law, to, push));
                }
            }
        }
        // the guardians may turn a law back under a ruler of the old ways
        if p.moves[GUARDIANS] > 0.45 && life.ruler_reform < -0.2 && !closed {
            for law in [PRESS, VOICE, CHURCH] {
                if p.laws[law] > 0 && rng.f32() < 0.05 * p.moves[GUARDIANS] * yrs {
                    let from = p.laws[law];
                    p.laws[law] -= 1;
                    let line = format!("{name}: охранители взяли верх — {}: {} вместо «{}»", LAW[law].name.to_lowercase(), LAW[law].levels[p.laws[law] as usize], LAW[law].levels[from as usize]);
                    p.story.push((month, line.clone()));
                    self.chronicle.push((month, c, line));
                    out.push(Upheaval::Reform { law, from, to: from - 1 });
                    return out;
                }
            }
        }
        if let Some((m, law, to, push)) = best {
            let chance = (0.08 + 0.4 * push) * if p.legit > 0.3 { 1.0 } else { 0.5 } * yrs;
            if push > 0.25 && rng.f32() < chance {
                let from = self.of[c].laws[law];
                self.reform(c, name, law, to, Some(m), month);
                out.push(Upheaval::Reform { law, from, to });
                return out;
            }
        }
        let p = &mut self.of[c];
        // the faith: a schism, a prophet, persecution
        if p.piety > 0.3 && p.piety < 0.85 && p.literacy > 0.22 && p.mood[CLERGY] < 0.1 && p.clout[LEARNED] + p.clout[MERCHANTS] > 0.18 && tenets[HIERARCHY] > -0.2 && rng.f32() < 0.06 * yrs {
            out.push(Upheaval::Schism);
        } else if (life.hunger > 0.4 || (lost > 0.5 && p.legit < 0.35)) && rng.f32() < 0.012 * yrs {
            out.push(Upheaval::NewFaith);
        } else if church == 0 && tenets[TOLERANCE] < -0.25 && life.foreign > 0.1 && rng.f32() < 0.12 * yrs {
            let line = format!("{name}: гонения на иноверцев");
            p.story.push((month, line.clone()));
            p.moves[SEPARATISTS] = (p.moves[SEPARATISTS] + 0.1).min(1.0);
            p.mood[MERCHANTS] -= 0.15;
            p.mood[LEARNED] -= 0.1;
            p.mood[CLERGY] = (p.mood[CLERGY] + 0.15).min(1.0);
            self.chronicle.push((month, c, line));
            out.push(Upheaval::Persecution);
        }
        out
    }

    /// Law `law` of country `c` becomes `to` (a reform won by movement `by`): those it suits are
    /// glad, those it hurts are not; a monarch granting the vote to all becomes a constitutional
    /// one, the rich men's republic one of all.
    pub fn reform(&mut self, c: usize, name: &str, law: usize, to: u8, by: Option<usize>, month: i32) {
        let p = &mut self.of[c];
        let from = p.laws[law];
        p.laws[law] = to;
        p.reforms += 1;
        for b in 0..BLOCS {
            let d = law_fit(b, law, to) - law_fit(b, law, from);
            p.mood[b] = (p.mood[b] + 0.5 * d).clamp(-1.0, 1.0);
        }
        if let Some(m) = by {
            p.moves[m] = (p.moves[m] - 0.2).max(0.0);
        }
        let what = match (law, to) {
            (LABOUR, 2) => "отменена зависимость — вольный труд".to_string(),
            (LABOUR, 1) => "неволя отменена, крестьяне остаются при земле".to_string(),
            (LAND, 2) => "земля отдана тем, кто её пашет".to_string(),
            (LAND, 1) => "крестьянам позволено выкупать землю".to_string(),
            (VOICE, 2) => "голос дан всем".to_string(),
            (VOICE, 1) => "созвано собрание сословий и собственников".to_string(),
            (CHURCH, 2) => "вера отделена от государства".to_string(),
            (CHURCH, 1) => "объявлена терпимость к иноверцам".to_string(),
            (CHURCH, 0) => "вера снова государственная".to_string(),
            (PRESS, 2) => "отменена цензура".to_string(),
            (PRESS, 1) => "цензура смягчена".to_string(),
            (PRESS, 0) => "введена цензура".to_string(),
            (SCHOOLS, 2) => "школы для всех детей".to_string(),
            (SCHOOLS, 1) => "открыты казённые гимназии".to_string(),
            (SCHOOLS, 0) => "школы отданы храмам".to_string(),
            (SERVICE, 2) => "введена всеобщая воинская повинность".to_string(),
            (SERVICE, 1) => "создана кадровая армия с запасом".to_string(),
            _ => format!("{}: {}", LAW[law].name.to_lowercase(), LAW[law].levels[to as usize]),
        };
        let line = format!("{name}: {what}{}", by.map_or(String::new(), |m| format!(" (добились {})", MOVE[m].name.to_lowercase())));
        p.story.push((month, line.clone()));
        // the form follows the vote
        let regime = p.regime;
        let new = match (regime, law, to) {
            (Regime::Autocracy | Regime::Estates, VOICE, 2) => Some(Regime::Constitutional),
            (Regime::Autocracy, VOICE, 1) => Some(Regime::Estates),
            (Regime::Patricians, VOICE, 2) => Some(Regime::Republic),
            _ => None,
        };
        self.chronicle.push((month, c, line));
        if let Some(r) = new {
            let p = &mut self.of[c];
            p.regime = r;
            p.since = month;
            let line = format!("{name}: теперь — {}", r.name().to_lowercase());
            p.story.push((month, line.clone()));
            self.chronicle.push((month, c, line));
        }
    }

    /// Country `c`'s government falls and takes form `to`, its laws changed by the movement `by`
    /// that won.
    pub fn change(&mut self, c: usize, name: &str, to: Regime, by: usize, month: i32, rng: &mut Rng) {
        let p = &mut self.of[c];
        let from = p.regime;
        p.regime = to;
        p.since = month;
        let laws = &mut p.laws;
        match to {
            Regime::Party => {
                laws[LABOUR] = 2;
                laws[LAND] = 2;
                laws[VOICE] = 0;
                laws[CHURCH] = 2;
                laws[PRESS] = 0;
                laws[SCHOOLS] = 2;
                laws[SERVICE] = 2;
            }
            Regime::Republic => {
                laws[VOICE] = 2;
                laws[PRESS] = laws[PRESS].max(1);
                laws[LABOUR] = laws[LABOUR].max(1);
                if by == LEVELLERS {
                    laws[LABOUR] = 2;
                    laws[LAND] = 2;
                }
                if by == LIBERALS {
                    laws[PRESS] = 2;
                    laws[CHURCH] = laws[CHURCH].max(1);
                }
            }
            Regime::Constitutional => {
                laws[VOICE] = laws[VOICE].max(1);
                laws[PRESS] = laws[PRESS].max(1);
            }
            Regime::Theocracy => {
                laws[CHURCH] = 0;
                laws[SCHOOLS] = 0;
                laws[PRESS] = 0;
            }
            Regime::Junta => {
                laws[PRESS] = 0;
                laws[SERVICE] = laws[SERVICE].max(1);
                laws[VOICE] = laws[VOICE].min(1);
            }
            Regime::Autocracy => {
                laws[VOICE] = 0;
                laws[PRESS] = laws[PRESS].min(1);
            }
            _ => {}
        }
        p.fell = Some(month);
        p.before = Some(from);
        if to == Regime::Junta || by == HAWKS {
            p.coups += 1;
        } else {
            p.revolutions += 1;
        }
        p.legit = 0.5;
        p.unrest *= 0.4;
        p.moves[by] *= 0.4;
        for m in 0..MOVES {
            if m != by && m != SEPARATISTS {
                p.moves[m] = (p.moves[m] + 0.08 * rng.f32()).min(1.0);
            }
        }
        // (those the old order kept down rejoice, its pillars do not)
        let old_v = from.voice();
        let new_v = to.voice();
        for b in 0..BLOCS {
            p.mood[b] = (p.mood[b] + 0.4 * (new_v[b] - old_v[b])).clamp(-1.0, 1.0);
        }
        p.drift -= 0.03;
        if !to.crowned() {
            p.house = None;
        }
        let how = if to == Regime::Junta {
            "военный переворот".to_string()
        } else if by == GUARDIANS {
            "реставрация".to_string()
        } else {
            format!("революция ({})", MOVE[by].name.to_lowercase())
        };
        let line = format!("{name}: {how} — {} {}, теперь {}", from.name().to_lowercase(), crate::world::names::agree(from.name(), "пала"), to.name().to_lowercase());
        p.story.push((month, line.clone()));
        self.chronicle.push((month, c, line));
    }

    /// A new monarch of a new house (after a revolution brought one back, or for a crowned form
    /// that has none).
    pub fn crown(&mut self, c: usize, ruler: &str, month: i32) {
        let p = &mut self.of[c];
        if p.regime.crowned() && p.house.is_none() {
            p.house = Some(format!("дом {}", surname(ruler)));
            p.house_since = month;
        }
    }
}
