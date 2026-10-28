//! What the countries come to know in the game (`progress::Know`): month by month each side's science
//! works on a technique — the one the player chose, or the one his head of science or the bot's
//! government leans to — as fast as its laboratories allow against how they began (the laboratories'
//! people, the country's development); a technique the world around knows costs less, and what most
//! of the world knows drifts in by itself. The other countries of the world go on learning too, year
//! by year (a new rival comes with what it knows). What a technique does is laid on at once: its
//! sectors work better, its kinds of troops fight better, a kind it opens can be fielded; its share
//! of science, factories, stability, the draft and supply goes into the country's effects.

use strategy::pack::Sphere;
use strategy::rng::{hash64, Rng};
use strategy::world::history::arm_staff;
use strategy::world::progress::{Know, Leaning};

use crate::country::spirits::Eff;
use crate::game::Note;
use crate::Game;

/// Research of a country of the least development a year, and what development adds (years of an
/// ordinary country's science; as in the world's past).
const LEAST: f32 = 0.3;
const DEV: f32 = 1.3;
/// The laboratories against how they began: no less than this share of the research, no more than
/// this many times.
const LAB_LOW: f32 = 0.3;
const LAB_HIGH: f32 = 3.0;

/// The game's side of research: how strong each side's laboratories were as the game began, what
/// each side researched last month, the techniques learnt (month, map country, technique).
#[derive(Clone, Debug, Default)]
pub struct Research {
    pub base: [f32; 2],
    pub points: [f32; 2],
    pub log: Vec<(u32, usize, usize)>,
}

impl Game {
    /// The share of the world that knows each technique, as country `c` sees it (its rival counts
    /// for more: it is watched).
    pub fn tech_seen(&self, c: usize) -> Vec<f32> {
        let nt = self.pack.techs.len();
        let others: Vec<usize> = (0..self.know.len()).filter(|&x| x != c && self.h.world.nations[x].people() > 0.05).collect();
        let rival = self.side_of(c).map(|s| self.country(1 - s));
        (0..nt)
            .map(|t| {
                let world = others.iter().filter(|&&o| self.know[o].known[t]).count() as f32 / others.len().max(1) as f32;
                match rival {
                    Some(r) if self.know[r].known[t] => 0.6 * world + 0.4,
                    Some(_) => 0.6 * world,
                    None => world,
                }
            })
            .collect()
    }

    /// The years of an ordinary country's science side `s` does in a month now.
    pub fn research_rate(&self, s: usize) -> f32 {
        let c = self.country(s);
        let dev = self.dev_of(c);
        let mut rate = (LEAST + DEV * dev.powf(1.2)) / 12.0;
        match self.h.labs[s].as_ref() {
            Some(lab) if self.research.base[s] > 1e-6 => rate *= (lab.power / self.research.base[s]).clamp(LAB_LOW, LAB_HIGH),
            Some(_) => {}
            None => {
                let sums = self.know[c].sums(&self.pack);
                rate *= 1.0 + sums.science + self.folk.schools.get(c).copied().unwrap_or(0.0);
            }
        }
        rate
    }

    /// What side `s` leans to when it chooses what to work on: the bot's government by its course;
    /// the player's head of science by his love (his ruler's — the player's — needs are what the
    /// laboratories work on: their focus).
    pub fn leaning(&self, s: usize) -> Leaning {
        let pack = &self.pack;
        let c = self.country(s);
        let mut l = if s != self.me || self.auto.is_some() { self.course_leaning(s) } else { Leaning::even(pack) };
        if s == self.me && self.auto.is_none() {
            // the laboratories' focus: the kinds of troops the player's science works on
            if let Some(lab) = self.h.labs[s].as_ref() {
                let k = lab.focus.len().max(1) as f32;
                for u in 0..pack.units.len() {
                    l.kinds[u] = (lab.focus.get(u).copied().unwrap_or(0.0) * k).max(0.2);
                }
                let eco = lab.focus.last().copied().unwrap_or(0.0) * k;
                l.sphere[0] = eco.max(0.3);
            }
            if let Some(id) = self.scholar.and_then(|k| self.roster.scholars.get(k).copied()) {
                let p = self.folk.get(id);
                if p.love < pack.units.len() {
                    l.kinds[p.love] *= 1.8;
                } else {
                    l.sphere[0] *= 1.5;
                }
            }
        }
        let _ = c;
        l
    }

    /// The month of research of both sides: points to what each works on (the player's choice is
    /// kept until it is learnt), what is learnt laid on; once a year the other countries learn too.
    pub fn research_month(&mut self) -> Vec<String> {
        let now = self.month();
        let pack = self.pack.clone();
        let mut alerts = Vec::new();
        if self.know.is_empty() {
            return alerts;
        }
        for s in 0..2 {
            let c = self.country(s);
            let points = self.research_rate(s);
            self.research.points[s] = points;
            let seen_v = self.tech_seen(c);
            let lean = self.leaning(s);
            let player = s == self.me && self.auto.is_none();
            let mut know = self.know[c].clone();
            let seen = |t: usize| seen_v.get(t).copied().unwrap_or(0.0);
            // (the player's own choice stays; his head of science, the bot's government choose by
            // reckoning, not by luck)
            let mut pick = |_: &Know, open: &[usize]| if player && self.scholar.is_none() { None } else { lean.best(&pack, open, &seen) };
            let got = know.research(&pack, points, &seen, &mut pick, now as i32, None);
            let mut r = Rng::new(hash64(self.seed ^ 0x7EC4 ^ (now as u64) << 3 ^ s as u64));
            let drift = know.drift(&pack, &seen, &mut r, now as i32, Know::DRIFT / 12.0);
            // (the player who leads science himself and chose nothing: the points wait in the
            // laboratories' general work — they go to the cheapest open technique)
            if player && self.scholar.is_none() && know.target.is_none() && got.is_empty() {
                if let Some(t) = know.open(&pack).into_iter().min_by(|&a, &b| Know::cost(&pack, a, seen(a)).total_cmp(&Know::cost(&pack, b, seen(b)))) {
                    know.progress[t] += points * 0.5;
                    if know.progress[t] >= Know::cost(&pack, t, seen(t)) {
                        know.learn(t, now as i32, None);
                    }
                }
            }
            let new: Vec<usize> = (0..pack.techs.len()).filter(|&t| know.known[t] && !self.know[c].known[t]).collect();
            self.know[c] = know;
            for t in new {
                self.apply_tech(c, t);
                if s == self.me {
                    let line = format!("страна овладела техникой: {}", pack.techs[t].name);
                    if !drift.contains(&t) {
                        alerts.push(line.clone());
                    }
                    let note = Note::new(now, &line, true, "science").with_tip(self.tech_tip(t));
                    self.news.push(note);
                } else {
                    let name = self.h.home(s).name.clone();
                    let visible = pack.techs[t].sphere == Sphere::Military || pack.techs[t].tier >= 3;
                    if visible {
                        self.news.push(Note::new(now, &format!("разведка: {name} {} техникой «{}»", strategy::world::names::agree(&name, "овладела"), pack.techs[t].name), false, "politics"));
                    }
                }
            }
        }
        if now % 12 == 5 {
            self.world_research_year();
        }
        alerts
    }

    /// A year of the other countries' science (as in the world's past, by their development and
    /// their people's leanings).
    fn world_research_year(&mut self) {
        let now = self.month() as i32;
        let pack = self.pack.clone();
        let sides = [self.country(0), self.country(1)];
        for c in 0..self.know.len() {
            if sides.contains(&c) || self.h.world.nations[c].people() < 0.05 {
                continue;
            }
            let dev = self.dev_of(c);
            let sums = self.know[c].sums(&pack);
            let points = (LEAST + DEV * dev.powf(1.2)) * (1.0 + sums.science + self.folk.schools.get(c).copied().unwrap_or(0.0)).max(0.2);
            let seen_v = self.tech_seen(c);
            let seen = |t: usize| seen_v.get(t).copied().unwrap_or(0.0);
            let lean = self.folk.leaning(&pack, c);
            let mut r = Rng::new(hash64(self.seed ^ 0x3EA2 ^ (now as u64) << 5 ^ c as u64));
            let mut know = self.know[c].clone();
            {
                let mut pick = |_: &Know, open: &[usize]| lean.pick(&pack, open, &seen, &mut r);
                know.research(&pack, points, &seen, &mut pick, now, None);
            }
            let mut r2 = Rng::new(hash64(self.seed ^ 0x3EA3 ^ (now as u64) << 5 ^ c as u64));
            know.drift(&pack, &seen, &mut r2, now, Know::DRIFT);
            let new: Vec<usize> = (0..pack.techs.len()).filter(|&t| know.known[t] && !self.know[c].known[t]).collect();
            self.know[c] = know;
            for t in new {
                self.apply_tech(c, t);
            }
        }
    }

    /// What technique `t`, just learnt by map country `c`, does: its sectors work better, its kinds
    /// of troops fight better and those it opens can be fielded (the side's staff, or the country's
    /// staff of the world's past for a rival to come).
    pub fn apply_tech(&mut self, c: usize, t: usize) {
        let pack = self.pack.clone();
        let tech = &pack.techs[t];
        if let Some(n) = self.h.world.nations.get_mut(c) {
            for &(k, x) in &tech.sectors {
                if let Some(p) = n.prod.get_mut(k) {
                    *p *= x.exp();
                }
            }
        }
        // (the staff learns what the technique gives its kinds: before, without it)
        let mut before = self.know[c].unit_gain(&pack);
        for &(u, x) in &tech.units {
            before[u] -= x;
        }
        match self.side_of(c) {
            Some(s) => arm_staff(&pack, &mut self.h.war.sides[s], &self.know[c], &before),
            None => {
                if let Some(st) = self.staffs.get_mut(c) {
                    arm_staff(&pack, st, &self.know[c], &before);
                }
            }
        }
        self.research.log.push((self.month(), c, t));
        self.stamp += 1;
    }

    /// The player's science works on technique `t` from now (if it may).
    pub fn set_tech(&mut self, s: usize, t: usize) {
        let c = self.country(s);
        if t < self.pack.techs.len() && self.know[c].can(&self.pack, t) {
            self.know[c].target = Some(t);
            self.stamp += 1;
        }
    }

    /// What the techniques side `s` knows do together, as features do (science, factories,
    /// stability, the draft, supply).
    pub fn tech_eff(&self, s: usize) -> Eff {
        let c = self.country(s);
        let Some(k) = self.know.get(c) else { return Eff::default() };
        let sums = k.sums(&self.pack);
        let schools = self.folk.schools.get(c).copied().unwrap_or(0.0);
        Eff {
            science: 1.0 + sums.science + schools,
            learn: 1.0 + sums.learn,
            stability: sums.stability / 12.0,
            draft: 1.0 + sums.draft,
            attrition: 1.0 / (1.0 + sums.supply),
            org_back: 1.0 + 0.5 * sums.supply,
            ..Eff::default()
        }
    }

    /// A technique in words, for a note: what it does.
    pub fn tech_tip(&self, t: usize) -> String {
        let pack = &self.pack;
        let tech = &pack.techs[t];
        let mut parts: Vec<String> = Vec::new();
        for &(k, x) in &tech.sectors {
            parts.push(format!("{} {:+.0}%", pack.sectors[k].name, (x.exp() - 1.0) * 100.0));
        }
        for &(u, x) in &tech.units {
            parts.push(format!("{} {:+.0}%", pack.units[u].name, (x.exp() - 1.0) * 100.0));
        }
        for &u in &tech.unlocks {
            parts.push(format!("можно выставить: {}", pack.units[u].name));
        }
        let pcts = [("наука", tech.science), ("заводы осваивают", tech.learn), ("призыв", tech.draft), ("снабжение", tech.supply)];
        for (name, x) in pcts {
            if x.abs() > 1e-4 {
                parts.push(format!("{name} {:+.0}%", x * 100.0));
            }
        }
        if tech.stability.abs() > 1e-4 {
            parts.push(if tech.stability > 0.0 { "власть крепче".into() } else { "власть слабее".into() });
        }
        if parts.is_empty() {
            tech.about.clone()
        } else {
            parts.join(", ")
        }
    }

    /// What side `s` knows that the other does not, and the other way round (the rival's as far
    /// as intelligence sees: arms and what shows — the higher tiers).
    pub fn tech_gap(&self, s: usize) -> (Vec<usize>, Vec<usize>) {
        let (a, b) = (self.country(s), self.country(1 - s));
        let pack = &self.pack;
        let seen = |t: usize| pack.techs[t].sphere == Sphere::Military || pack.techs[t].tier >= 2;
        let ours: Vec<usize> = (0..pack.techs.len()).filter(|&t| self.know[a].known[t] && !self.know[b].known[t] && seen(t)).collect();
        let theirs: Vec<usize> = (0..pack.techs.len()).filter(|&t| !self.know[a].known[t] && self.know[b].known[t] && seen(t)).collect();
        (ours, theirs)
    }
}
