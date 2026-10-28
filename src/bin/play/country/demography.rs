//! The people's lives and deaths, group by group: the poor of the villages bear many children and
//! bury many; bread, doctors and towns make fewer die and — a generation later, as people learn to
//! read — fewer be born (the demographic transition, grown from how each group lives, not written per
//! country). Epidemics come now and then: war, hunger and crowded towns call them, doctors keep them
//! short. Plain people who learned to read become the learned, with their savings — as many as the
//! country's literacy can school.
//!
//! The engine counts it (`Nation::growth_by` — each group's births less deaths a month; deaths from
//! want stay the engine's, `famine_k`).

use strategy::rng::{hash64, Rng};

use crate::country::policy::role;
use crate::game::Note;
use crate::Game;

/// Births a year of a poor village people who cannot read, and how much literacy, towns and comfort
/// take off it; deaths a year without doctors, and what doctors and comfort take off them.
const BIRTH_TOP: f32 = 0.042;
const BIRTH_LIT: f32 = 0.3;
const BIRTH_TOWN: f32 = 0.15;
const BIRTH_RICH: f32 = 0.2;
/// (Calibrated so that a country of the custom — a fifth reads, village doctors — grows about 1% a
/// year, as the engine's one rate did: the village 1.2%, the towns 0.6%, the owners 0.2%.)
const DEATH_TOP: f32 = 0.046;
const DEATH_DOC: f32 = 0.012;
const DEATH_COMFORT: f32 = 0.01;

/// What the people live through, both sides.
#[derive(Clone, Debug, Default)]
pub struct Demo {
    /// Births and deaths a year (shares of the people), last month.
    pub births: [f32; 2],
    pub deaths: [f32; 2],
    /// Months of an epidemic left, and the share of the people it kills a month.
    pub epidemic: [u32; 2],
    pub plague: [f32; 2],
    /// The dead of epidemics in the game (millions); people who became learned last month.
    pub plague_dead: [f32; 2],
    pub schooled: [f32; 2],
    /// The share of the people the learned make for each point of literacy, at the start (they
    /// follow literacy by it).
    pub learned_k: [f32; 2],
    /// People a year ago, month by month (for the growth over a year).
    pub past: [Vec<f32>; 2],
}

impl Game {
    /// Each group's births and deaths a year for side `s` now: (births, deaths) by group.
    pub fn vital_rates(&self, s: usize) -> Vec<(f32, f32)> {
        let pack = &self.pack;
        let n = self.h.home(s);
        let lit = self.polity_of(s).literacy.clamp(0.0, 1.0);
        let doc = self.civil.health[s].clamp(0.0, 2.0);
        (0..pack.groups.len())
            .map(|g| {
                let r = role(pack, g);
                let town = if r == 0 { 0.0 } else { 1.0 };
                let comfort = (1.0 - n.hardship[g]).clamp(0.0, 1.0);
                let well = if r >= 2 { 1.0 } else { 0.0 };
                let born = BIRTH_TOP * (1.0 - BIRTH_LIT * lit - BIRTH_TOWN * town - BIRTH_RICH * well) * (0.7 + 0.3 * comfort);
                let died = DEATH_TOP - DEATH_DOC * doc - DEATH_COMFORT * comfort - 0.004 * well;
                (born.max(0.008), died.max(0.009))
            })
            .collect()
    }

    /// Before the world's month: each group's natural growth for the engine.
    pub fn demography_apply(&mut self) {
        if crate::country::econ_spoiled() {
            return;
        }
        for s in 0..2 {
            let rates = self.vital_rates(s);
            let n = self.h.home(s);
            let all = n.people().max(1e-9);
            let (b, d) = rates.iter().enumerate().fold((0.0, 0.0), |(b, d), (g, r)| (b + r.0 * n.pop[g] / all, d + r.1 * n.pop[g] / all));
            self.demo.births[s] = b;
            self.demo.deaths[s] = d;
            let by: Vec<f32> = rates.iter().map(|r| (r.0 - r.1) / 12.0).collect();
            self.h.home_mut(s).growth_by = by;
        }
    }

    /// After the world's month: epidemics come and go; the lettered become the learned; the memory
    /// of the people for the growth over a year. What should stop the clock.
    pub fn demography_month(&mut self) -> Vec<String> {
        let now = self.month();
        let me = self.me;
        let pack = self.pack.clone();
        let mut alerts = Vec::new();
        let mut r = Rng::new(hash64(self.seed ^ 0xDE30 ^ (now as u64) << 5));
        if crate::country::econ_spoiled() {
            return alerts;
        }
        for s in 0..2 {
            if self.demo.learned_k[s] <= 0.0 {
                let lit = self.polity_of(s).literacy.max(0.02);
                self.demo.learned_k[s] = self.learned_share(s) / lit;
            }
            // an epidemic: war, hunger and towns call it, doctors keep it short and mild
            let f = self.spirits.facts[s].clone();
            let doc = self.civil.health[s].clamp(0.0, 2.0);
            if self.demo.epidemic[s] > 0 {
                self.demo.epidemic[s] -= 1;
                let k = self.demo.plague[s];
                let n = self.h.home_mut(s);
                let dead = n.people() * k;
                for p in n.pop.iter_mut() {
                    *p *= 1.0 - k;
                }
                // (the survivors keep the money; the sick and the bereaved grieve)
                if n.live {
                    for d in n.discontent.iter_mut() {
                        *d += 0.04;
                    }
                }
                self.demo.plague_dead[s] += dead;
            } else {
                let towns = 1.0 - f.land_share;
                let chance = 0.0018 * (1.0 + 3.0 * f.hunger + if self.h.at_war { 1.0 } else { 0.0 }) * (0.6 + 0.8 * towns) * (1.4 - 0.45 * doc);
                if r.f32() < chance {
                    let months = 3 + (r.f32() * 6.0 * (1.3 - 0.4 * doc).max(0.3)) as u32;
                    let k = (0.0015 + 0.004 * r.f32()) * (1.3 - 0.45 * doc).max(0.25);
                    self.demo.epidemic[s] = months;
                    self.demo.plague[s] = k;
                    let name = self.h.home(s).name.clone();
                    let line = if s == me { "в стране мор".to_string() } else { format!("{name}: мор") };
                    let tip = format!("около {:.1}% людей в месяц, {months} мес.; врачи сокращают мор", k * 100.0);
                    if s == me {
                        alerts.push(line.clone());
                    }
                    self.news.push(Note::new(now, &line, s == me, "econ").with_tip(tip));
                }
            }
            // the lettered become the learned: as many as the literacy holds against the start, a
            // little each month, with their savings, while the learned are paid better
            let learned: Vec<usize> = (0..pack.groups.len()).filter(|&g| role(&pack, g) == 2).collect();
            let lit = self.polity_of(s).literacy;
            let aim = self.demo.learned_k[s] * lit;
            let have = self.learned_share(s);
            self.demo.schooled[s] = 0.0;
            if let Some(&to) = learned.iter().max_by(|&&a, &&b| self.h.home(s).pop[a].total_cmp(&self.h.home(s).pop[b])) {
                if aim > have {
                    let n = self.h.home_mut(s);
                    let all = n.people();
                    let want = 0.01 * (aim - have) * all;
                    let from: Vec<usize> = (0..pack.groups.len()).filter(|&g| role(&pack, g) < 2 && n.pop[g] > 1e-6 && n.pay[g] < n.pay[to].max(1e-9)).collect();
                    let plain: f32 = from.iter().map(|&g| n.pop[g]).sum();
                    if plain > 1e-6 {
                        // (a serf may not leave to study: the land's people go as freely as the law
                        // of labour lets them)
                        let free = n.mobility_k;
                        for &g in &from {
                            let movers = want * n.pop[g] / plain * if role(&pack, g) == 0 { free } else { 1.0 };
                            let money = n.money[g] * movers / n.pop[g];
                            n.pop[g] -= movers;
                            n.pop[to] += movers;
                            n.money[g] -= money;
                            n.money[to] += money;
                        }
                        self.demo.schooled[s] = want;
                    }
                }
            }
            let people = self.h.home(s).people();
            let past = &mut self.demo.past[s];
            past.push(people);
            if past.len() > 13 {
                past.remove(0);
            }
        }
        alerts
    }

    /// The share of side `s`'s people who are the learned (the officials, teachers, engineers).
    pub fn learned_share(&self, s: usize) -> f32 {
        let pack = &self.pack;
        let n = self.h.home(s);
        (0..pack.groups.len()).filter(|&g| role(pack, g) == 2).map(|g| n.pop[g]).sum::<f32>() / n.people().max(1e-9)
    }

    /// Side `s`'s people's growth over the last year (a share).
    pub fn people_growth(&self, s: usize) -> f32 {
        let past = &self.demo.past[s];
        match (past.first(), past.last()) {
            (Some(a), Some(b)) if past.len() >= 2 && *a > 1e-6 => (b / a).powf(12.0 / (past.len() - 1) as f32) - 1.0,
            _ => self.demo.births[s] - self.demo.deaths[s],
        }
    }
}
