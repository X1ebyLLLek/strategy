//! The arms of a country as its factories make them (as HOI4's production lines and their
//! efficiency, but without clicking factories): each kind of unit is made by a model — a rifle, a
//! gun, a tank of such a design — and the factories learn to make it, month by month while they
//! make it (they forget a little when they make none): a model they know well costs them less of
//! every good than one they are still learning. Science brings new models: their makers praise
//! them, what they are really worth nobody knows until trials or battles show it. A new model is
//! the government's bet: put it into production and the factories start learning again (the
//! harder the kit, the more they lose), and the army is refitted with it only as fast as new units
//! come; keep the old one and the factories go on making it cheaply. Trials cost months but tell
//! the truth near enough. The government also says which kinds the factories should make more or
//! less of than the staff's shares (priorities); the staff's shares stay its own.

use strategy::economy::science::DOUBT;
use strategy::pack::Pack;
use strategy::rng::{hash64, Rng};

use crate::game::Note;
use crate::Game;

/// Priorities of production: how many times the staff's share, and their names.
pub const PRIO: [(f32, &str); 5] = [(0.0, "Не выпускать"), (0.5, "Меньше"), (1.0, "Как решит генштаб"), (1.6, "Больше"), (2.5, "Всё на это")];
pub const PRIO_USUAL: usize = 2;
/// A month of making a model takes the factories this share of the way to what they can learn
/// (slower for a harder kit); a month without it, they forget this much.
const LEARN: f32 = 0.05;
const FORGET: f32 = 0.004;
/// What the factories can learn at most, and at the least what they keep.
const SKILL_TOP: f32 = 0.9;
const SKILL_LEAST: f32 = 0.2;
/// The skill a kit's goods were reckoned at in the pack (at it, a kit costs what the pack says).
const SKILL_NORM: f32 = 0.8;
/// Skill of the factories at the start: in a kind the army keeps, and in one it does not.
const SKILL_KEPT: f32 = 0.78;
const SKILL_NEW: f32 = 0.4;
/// In peace, old equipment is rebuilt to the new model at the depots: this share a month.
const REBUILD: f32 = 0.012;
/// Months a model's trials take; how far off they judge it (log, sd).
pub const TRIAL_MONTHS: u32 = 4;
const TRIAL_SD: f32 = 0.03;
/// Months a model waits for the government's word before its makers give it up.
const WAIT: u32 = 24;
/// Names of hard kits, as their makers call them.
const NICKS: [&str; 16] = ["Гроза", "Сокол", "Медведь", "Вихрь", "Молот", "Ястреб", "Кречет", "Бурлак", "Тур", "Барс", "Ворон", "Гром", "Рысь", "Буран", "Витязь", "Шквал"];

/// A model of a kind's equipment.
#[derive(Clone, Debug)]
pub struct Model {
    pub gen: u32,
    pub name: String,
    /// The month it was put into production.
    pub month: u32,
    /// What it really is against the model before (log, fire and toughness; hidden), what its
    /// makers claimed, what trials showed if it had any.
    pub gain: (f32, f32),
    pub claim: (f32, f32),
    pub tried: Option<(f32, f32)>,
    /// What its kit costs against the first model's.
    pub cost: f32,
}

/// A line of production: a kind's models (the last is made now), how well the factories know
/// it, how much of the army has it, what was made last month and in all, the government's
/// priority.
#[derive(Clone, Debug)]
pub struct Line {
    pub models: Vec<Model>,
    pub skill: f32,
    pub refit: f32,
    pub made: f32,
    pub total: f32,
    pub prio: usize,
}

impl Line {
    pub fn model(&self) -> &Model {
        self.models.last().unwrap()
    }
}

/// A new model offered by the laboratories, waiting for the government's word.
#[derive(Clone, Debug)]
pub struct Offer {
    pub id: u64,
    pub kind: usize,
    pub gen: u32,
    pub name: String,
    pub month: u32,
    pub gain: (f32, f32),
    pub claim: (f32, f32),
    /// Months of trials left (0: none running), and what they showed.
    pub trial: u32,
    pub tried: Option<(f32, f32)>,
}

#[derive(Clone, Debug, Default)]
pub struct Arms {
    pub lines: [Vec<Line>; 2],
    pub offers: [Vec<Offer>; 2],
    /// Models turned down or given up, for the record: (side, name, month).
    pub refused: Vec<(usize, String, u32)>,
    pub next: u64,
}

/// What the factories lose of their skill when they turn to a model this much better (log, as
/// claimed) of a kit this hard: they keep this share.
pub fn retain(pack: &Pack, u: usize, claim: (f32, f32)) -> f32 {
    let c = pack.units[u].complexity;
    (0.78 - 0.6 * (0.5 * (claim.0 + claim.1)).max(0.0) - 0.1 * (c - 1.0)).clamp(0.3, 0.85)
}

/// The name of model `gen` of kind `u` of a country (its letter, a nickname for a hard kit).
pub fn model_name(pack: &Pack, u: usize, country: &str, gen: u32, seed: u64) -> String {
    let unit = &pack.units[u];
    let mut noun = unit.model.clone();
    if let Some(f) = noun.chars().next() {
        noun = f.to_uppercase().collect::<String>() + &noun[f.len_utf8()..];
    }
    let letter: String = country.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
    let mut name = format!("{noun} {letter}-{}", roman(gen));
    if unit.complexity >= 1.5 {
        let k = hash64(seed ^ (u as u64) << 12 ^ gen as u64) as usize % NICKS.len();
        name.push_str(&format!(" «{}»", NICKS[k]));
    }
    name
}

/// A generation in Roman letters (as the names of models are written: «Танк Р-II»).
pub fn roman(n: u32) -> String {
    let mut n = n.max(1);
    let mut out = String::new();
    for (v, s) in [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")] {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

/// Mean of a pair of logs as a share (e.g. +12%).
pub fn pct(g: (f32, f32)) -> f32 {
    (0.5 * (g.0 + g.1)).exp() - 1.0
}

impl Game {
    /// The lines of both sides as the game begins: each kind's first model; the factories know
    /// well what the army keeps.
    pub fn arms_start(&mut self) {
        let pack = self.pack.clone();
        for s in 0..2 {
            let name = self.h.home(s).name.clone();
            let army = self.h.war.sides[s].army.count.clone();
            let seed = hash64(self.seed ^ 0xA2A5 ^ s as u64);
            self.arms.lines[s] = (0..pack.units.len())
                .map(|u| Line {
                    models: vec![Model { gen: 1, name: model_name(&pack, u, &name, 1, seed), month: 0, gain: (0.0, 0.0), claim: (0.0, 0.0), tried: None, cost: 1.0 }],
                    skill: if army.get(u).copied().unwrap_or(0.0) > 0.01 { SKILL_KEPT } else { SKILL_NEW },
                    refit: 1.0,
                    made: 0.0,
                    total: 0.0,
                    prio: PRIO_USUAL,
                })
                .collect();
            if let Some(lab) = self.h.labs[s].as_mut() {
                lab.hold = true;
            }
        }
        self.arms_before();
    }

    /// Before the world's month: what a kit of each kind costs each side's factories now (the
    /// model's price, their skill, what the country's features and law make of it), the
    /// government's priorities.
    pub fn arms_before(&mut self) {
        let nu = self.pack.units.len();
        for s in 0..2 {
            if self.arms.lines[s].len() != nu {
                continue;
            }
            let e = self.effs[s].clone();
            let make: Vec<f32> = self.arms.lines[s].iter().map(|l| l.model().cost * e.make_cost * SKILL_NORM / l.skill.max(SKILL_LEAST)).collect();
            self.h.home_mut(s).make_k = make;
            let prio: Vec<f32> = self.arms.lines[s].iter().map(|l| PRIO[l.prio].0).collect();
            // (nothing at all is never asked: a kind the staff wants keeps a trickle)
            let all_zero = prio.iter().zip(&self.h.war.sides[s].making).all(|(p, m)| *p == 0.0 || *m == 0.0);
            self.h.war.sides[s].prio = if all_zero { Vec::new() } else { prio };
        }
    }

    /// After the world's month: the factories learn what they made; the army is refitted with the
    /// new models as new units come (and old kit is rebuilt at the depots in peace); the real worth
    /// of each kind follows the share refitted; new models offered by the laboratories wait for
    /// the government (the bot decides at once); trials go on.
    pub fn arms_month(&mut self) -> Vec<String> {
        let now = self.month();
        let me = self.me;
        let pack = self.pack.clone();
        let nu = pack.units.len();
        let mut alerts = Vec::new();
        for s in 0..2 {
            if self.arms.lines[s].len() != nu {
                continue;
            }
            let e = self.effs[s].clone();
            let made = self.h.last_made[s].clone();
            let top = (SKILL_TOP + e.skill).clamp(0.5, 1.2);
            for u in 0..nu {
                let m = made.get(u).copied().unwrap_or(0.0);
                let army = self.h.war.sides[s].army.count[u];
                let kind = e.kind.filter(|k| k.0 == u).map_or(1.0, |k| k.1);
                let l = &mut self.arms.lines[s][u];
                l.made = m;
                l.total += m;
                if m > 1e-4 {
                    let pace = LEARN * e.learn * kind / pack.units[u].complexity;
                    l.skill += pace.min(0.5) * (top - l.skill).max(0.0);
                } else {
                    l.skill = (l.skill - FORGET).max(SKILL_LEAST);
                }
                l.skill = l.skill.min(top.max(l.skill - FORGET));
                if army > 1e-6 {
                    let old = (army - m).max(0.0);
                    l.refit = ((l.refit * old + m) / army).clamp(0.0, 1.0);
                } else {
                    l.refit = 1.0;
                }
                if !self.h.at_war && l.refit < 1.0 {
                    l.refit = (l.refit + REBUILD * l.skill).min(1.0);
                }
                // (the real worth of the kind: the newest model's gain only as far as the army has it)
                if l.models.len() > 1 {
                    let g = l.model().gain;
                    let c = &mut self.h.war.sides[s];
                    let lag = 1.0 - l.refit;
                    c.truth_fire[u] = c.base_fire[u] * (-g.0 * lag).exp();
                    c.truth_tough[u] = c.base_tough[u] * (-g.1 * lag).exp();
                }
            }
            // new models from the laboratories
            let offered: Vec<strategy::economy::science::Work> = self.h.labs[s].as_mut().map_or(Vec::new(), |l| std::mem::take(&mut l.offered));
            for w in offered {
                let Some(u) = w.kind else { continue };
                let gen = self.arms.lines[s][u].model().gen + 1 + self.arms.offers[s].iter().filter(|o| o.kind == u).count() as u32;
                let name = model_name(&pack, u, &self.h.home(s).name.clone(), gen, hash64(self.seed ^ 0xA2A5 ^ s as u64));
                self.arms.next += 1;
                let offer = Offer { id: self.arms.next, kind: u, gen, name: name.clone(), month: now, gain: w.gain, claim: w.claim, trial: 0, tried: None };
                self.arms.offers[s].push(offer);
                if s == me && self.auto.is_none() {
                    let line = format!("конструкторы представили новый образец: {name}");
                    self.news.push(Note::new(now, &line, true, "arms").with_tip(format!("обещают огонь {:+.0}%, стойкость {:+.0}%", (w.claim.0.exp() - 1.0) * 100.0, (w.claim.1.exp() - 1.0) * 100.0)));
                    alerts.push(line);
                }
            }
            // trials; models waiting too long are given up
            let mut r = Rng::new(hash64(self.seed ^ 0x7121A1 ^ (now as u64) << 2 ^ s as u64));
            for o in self.arms.offers[s].iter_mut() {
                if o.trial > 0 {
                    o.trial -= 1;
                    if o.trial == 0 {
                        o.tried = Some((o.gain.0 + TRIAL_SD * r.normal(), o.gain.1 + TRIAL_SD * r.normal()));
                        if s == me {
                            let t = o.tried.unwrap();
                            self.news.push(Note::new(now, &format!("испытания образца {} закончены", o.name), true, "arms").with_tip(format!("огонь {:+.0}%, стойкость {:+.0}% к нынешнему", (t.0.exp() - 1.0) * 100.0, (t.1.exp() - 1.0) * 100.0)));
                        }
                    }
                }
            }
            let old: Vec<u64> = self.arms.offers[s].iter().filter(|o| now >= o.month + WAIT && o.trial == 0).map(|o| o.id).collect();
            for id in old {
                self.refuse(s, id);
            }
            // the bot's government decides at once
            if s != me || self.auto.is_some() {
                let ids: Vec<u64> = self.arms.offers[s].iter().map(|o| o.id).collect();
                for id in ids {
                    self.bot_model(s, id);
                }
            }
        }
        alerts
    }

    /// The bot's word on a new model: its staff believes what the makers claim; it changes models
    /// when the claim is worth what the factories would lose (in a war, only for a clearly better
    /// one).
    fn bot_model(&mut self, s: usize, id: u64) {
        let Some(o) = self.arms.offers[s].iter().find(|o| o.id == id).cloned() else { return };
        let seen = o.tried.unwrap_or(o.claim);
        let value = 0.5 * (seen.0 + seen.1);
        let lose = 1.0 - retain(&self.pack, o.kind, o.claim);
        let mut bar = 0.04 + lose * if self.h.at_war { 0.35 } else { 0.12 };
        // (a government whose course stakes on this kind takes a new model more readily)
        if self.courses.now[s].as_ref().is_some_and(|c| c.favour == Some(o.kind)) {
            bar *= 0.6;
        }
        if value > bar {
            self.adopt(s, id);
        } else {
            self.refuse(s, id);
        }
    }

    /// Side `s` puts model `id` into production: the factories start learning it, the army will
    /// be refitted with it as new units come; its staff believes what it was told (or saw at the
    /// trials).
    pub fn adopt(&mut self, s: usize, id: u64) -> bool {
        let Some(k) = self.arms.offers[s].iter().position(|o| o.id == id) else { return false };
        let o = self.arms.offers[s].remove(k);
        let u = o.kind;
        let now = self.month();
        let keep = retain(&self.pack, u, o.claim);
        let c = &mut self.h.war.sides[s];
        // (what the staff takes it for: the trials, when there were any, else the claim)
        let told = o.tried.unwrap_or(o.claim);
        c.redesign(u, o.gain, told, if o.tried.is_some() { DOUBT * 0.2 } else { DOUBT });
        let l = &mut self.arms.lines[s][u];
        let prev_cost = l.model().cost;
        let better = (0.5 * (o.gain.0 + o.gain.1)).max(0.0);
        l.models.push(Model { gen: o.gen, name: o.name.clone(), month: now, gain: o.gain, claim: o.claim, tried: o.tried, cost: prev_cost * (1.03 + 0.4 * better) });
        l.skill = (l.skill * keep).max(SKILL_LEAST);
        l.refit = 0.0;
        // (the old units keep their old kit: the kind's worth comes as the refit goes)
        let lag = 1.0;
        c.truth_fire[u] = c.base_fire[u] * (-o.gain.0 * lag).exp();
        c.truth_tough[u] = c.base_tough[u] * (-o.gain.1 * lag).exp();
        if s == self.me {
            self.news.push(Note::new(now, &format!("в производство запущен образец {}", o.name), false, "arms"));
        } else {
            let name = self.h.home(s).name.clone();
            self.news.push(Note::new(now, &format!("разведка: {name} перевооружается — {}", self.pack.units[u].name), false, "politics"));
        }
        self.stamp += 1;
        true
    }

    /// Side `s` turns down model `id`.
    pub fn refuse(&mut self, s: usize, id: u64) {
        if let Some(k) = self.arms.offers[s].iter().position(|o| o.id == id) {
            let o = self.arms.offers[s].remove(k);
            self.arms.refused.push((s, o.name, self.month()));
            self.stamp += 1;
        }
    }

    /// Side `s` sends model `id` to trials (if none ran yet).
    pub fn try_model(&mut self, s: usize, id: u64) {
        if let Some(o) = self.arms.offers[s].iter_mut().find(|o| o.id == id) {
            if o.trial == 0 && o.tried.is_none() {
                o.trial = TRIAL_MONTHS;
                self.stamp += 1;
            }
        }
    }

    /// What a battalion of kind `u` costs side `s` now against what the pack reckons it at (the
    /// factories' skill, the model's price, the country's ways).
    pub fn make_cost(&self, s: usize, u: usize) -> f32 {
        self.h.home(s).make_k.get(u).copied().unwrap_or(1.0)
    }
}
