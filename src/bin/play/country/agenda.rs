//! The country's great task (as HOI4's national focus, but nothing written beforehand): the forces
//! of society and the movements put forward what the country should do next — catch up in
//! industry, feed the land, school the people, build the army, fill the treasury, master a
//! technique the neighbour has, win a reform, calm the country, outgrow the rival, unite the
//! faithful, take back a lost region — each from what the country is and lives through, each with
//! a measure, a deadline, a reward (legitimacy, those who asked for it glad, a lasting edge for
//! five years) and a price if it fails. The government takes one task at a time (the player
//! chooses among three offered; the bot by its course). Every game offers other tasks: they grow
//! out of the world the past made.

use strategy::rng::{hash64, Rng};
use strategy::world::polity;

use crate::country::spirits::Eff;
use crate::game::Note;
use crate::Game;

/// What a task asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Goal {
    Industry,
    Harvest,
    Literacy,
    Army,
    Treasury,
    Tech(usize),
    Reform(usize, u8),
    Calm,
    Outgrow,
    Faith,
    Revanche(usize),
}

impl Goal {
    pub fn code(&self) -> u32 {
        match self {
            Goal::Industry => 0,
            Goal::Harvest => 1,
            Goal::Literacy => 2,
            Goal::Army => 3,
            Goal::Treasury => 4,
            Goal::Tech(_) => 5,
            Goal::Reform(..) => 6,
            Goal::Calm => 7,
            Goal::Outgrow => 8,
            Goal::Faith => 9,
            Goal::Revanche(_) => 10,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Goal::Industry => "anvil",
            Goal::Harvest => "wheat",
            Goal::Literacy => "graduate",
            Goal::Army => "army",
            Goal::Treasury => "coins",
            Goal::Tech(_) => "science",
            Goal::Reform(..) => "scroll",
            Goal::Calm => "castle",
            Goal::Outgrow => "econ",
            Goal::Faith => "faith",
            Goal::Revanche(_) => "fist",
        }
    }

    /// Months it is given.
    pub fn months(&self) -> u32 {
        match self {
            Goal::Industry | Goal::Literacy | Goal::Tech(_) => 36,
            Goal::Outgrow => 60,
            Goal::Faith | Goal::Revanche(_) => 48,
            Goal::Calm => 18,
            _ => 24,
        }
    }

    /// The lasting edge of a task fulfilled (five years).
    pub fn reward(&self) -> Eff {
        let e = Eff::default;
        match self {
            Goal::Industry => Eff { out_industry: 1.05, learn: 1.05, ..e() },
            Goal::Harvest => Eff { out_land: 1.05, anger: [-0.002, 0.0, 0.0, 0.0], ..e() },
            Goal::Literacy => Eff { science: 1.08, ..e() },
            Goal::Army => Eff { attack: 1.03, org_back: 1.05, ..e() },
            Goal::Treasury => Eff { stability: 0.002, leak: -0.005, ..e() },
            Goal::Tech(_) => Eff { science: 1.04, learn: 1.05, ..e() },
            Goal::Reform(..) => Eff { stability: 0.003, ..e() },
            Goal::Calm => Eff { stability: 0.004, ..e() },
            Goal::Outgrow => Eff { out_civil: 1.03, out_industry: 1.02, ..e() },
            Goal::Faith => Eff { stability: 0.003, ..e() },
            Goal::Revanche(_) => Eff { attack: 1.05, stability: 0.004, ..e() },
        }
    }

    /// The reward in words.
    pub fn reward_words(&self) -> &'static str {
        match self {
            Goal::Industry => "5 лет: заводы +5%, навык заводов растёт быстрее",
            Goal::Harvest => "5 лет: урожай +5%, крестьяне спокойнее",
            Goal::Literacy => "5 лет: наука +8%",
            Goal::Army => "5 лет: атака +3%, порядок в частях восстанавливается быстрее",
            Goal::Treasury => "5 лет: власть устойчивее, утечки из казны меньше",
            Goal::Tech(_) => "5 лет: наука +4%, заводы учатся быстрее",
            Goal::Reform(..) => "5 лет: власть устойчивее",
            Goal::Calm => "5 лет: власть заметно устойчивее",
            Goal::Outgrow => "5 лет: товары +3%, заводы +2%",
            Goal::Faith => "5 лет: власть устойчивее",
            Goal::Revanche(_) => "5 лет: атака +5%, власть устойчивее",
        }
    }
}

/// Who asks for a task: a force of society or a movement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Who {
    Bloc(usize),
    Move(usize),
}

impl Who {
    pub fn name(&self) -> &'static str {
        match self {
            Who::Bloc(b) => polity::BLOC[*b].name,
            Who::Move(m) => polity::MOVE[*m].name,
        }
    }
}

/// A task: what, who asked, from when to when, where the measure stood and where it must get.
#[derive(Clone, Debug)]
pub struct Task {
    pub goal: Goal,
    pub who: Who,
    pub since: u32,
    pub until: u32,
    pub base: f32,
    pub target: f32,
}

/// The tasks of both sides.
#[derive(Clone, Debug, Default)]
pub struct Agenda {
    pub now: [Option<Task>; 2],
    pub offers: [Vec<Task>; 2],
    /// Tasks done (true) and failed, with the month they ended.
    pub past: [Vec<(Task, bool, u32)>; 2],
    /// Rewards still running: the goal and the month they end.
    pub rewards: [Vec<(Goal, u32)>; 2],
    /// The month offers were last made.
    pub offered: [u32; 2],
}

impl Game {
    /// The task's title in words.
    pub fn task_title(&self, t: &Task) -> String {
        match t.goal {
            Goal::Industry => "Индустриальный рывок".into(),
            Goal::Harvest => "Хлеб для всех".into(),
            Goal::Literacy => "Грамотная страна".into(),
            Goal::Army => "Армия, которой боятся".into(),
            Goal::Treasury => "Полная казна".into(),
            Goal::Tech(k) => format!("Овладеть: {}", self.pack.techs.get(k).map_or("", |x| x.name.as_str())),
            Goal::Reform(l, v) => format!("Реформа: {}", polity::LAW[l].levels[v as usize]),
            Goal::Calm => "Успокоить страну".into(),
            Goal::Outgrow => format!("Обогнать {}", self.h.home(1 - self.side_of_task(t)).name),
            Goal::Faith => "Единая вера".into(),
            Goal::Revanche(_) => "Вернуть отнятое".into(),
        }
    }

    fn side_of_task(&self, t: &Task) -> usize {
        (0..2).find(|&s| self.agenda.now[s].as_ref().is_some_and(|x| x.since == t.since && x.goal == t.goal) || self.agenda.offers[s].iter().any(|x| x.since == t.since && x.goal == t.goal)).unwrap_or(self.me)
    }

    /// What the task asks, in words (the measure, the numbers).
    pub fn task_words(&self, s: usize, t: &Task) -> String {
        let now = self.task_measure(s, &t.goal);
        match t.goal {
            Goal::Industry => format!("выпуск заводов +15%: сейчас {:.0}, надо {:.0}", now, t.target),
            Goal::Harvest => format!("урожай +8% и без голода: сейчас {:.0}, надо {:.0}", now, t.target),
            Goal::Literacy => format!("образованных {:.1}% → {:.1}%", now * 100.0, t.target * 100.0),
            Goal::Army => format!("дивизий {:.0} → {:.0}", now, t.target),
            Goal::Treasury => format!("казна — {:.1} мес. доходов, надо {:.0}, без долгов", now, t.target),
            Goal::Tech(_) => "довести разработку до конца".into(),
            Goal::Reform(l, v) => format!("{}: «{}»", polity::LAW[l].name, polity::LAW[l].levels[v as usize]),
            Goal::Calm => format!("смута {:.0}% → ниже {:.0}%", now * 100.0, t.target * 100.0),
            Goal::Outgrow => format!("выпуск {:.0}% от соседа → 100%", now * 100.0),
            Goal::Faith => format!("своя вера у {:.0}% людей → {:.0}%", now * 100.0, t.target * 100.0),
            Goal::Revanche(st) => format!("держать отнятую область ({})", self.region_name(st)),
        }
    }

    fn region_name(&self, st: usize) -> String {
        self.towns
            .iter()
            .find(|t| self.prov_state.get(t.0).is_some_and(|&x| x as usize == st))
            .map_or_else(|| format!("область {st}"), |t| format!("область у {}", t.1))
    }

    /// Where the measure of `goal` stands now for side `s`.
    pub fn task_measure(&self, s: usize, goal: &Goal) -> f32 {
        let n = self.h.home(s);
        let f = &self.spirits.facts[s];
        let c = self.country(s);
        match *goal {
            Goal::Industry => n.gdp_real * f.industry_share,
            Goal::Harvest => n.gdp_real * f.land_share * if f.hunger > 0.03 { 0.9 } else { 1.0 },
            Goal::Literacy => f.learned,
            Goal::Army => self.h.war.sides[s].army.count.iter().sum(),
            Goal::Treasury => if f.debt > 0.5 { f.treasury.min(0.0) } else { f.treasury },
            Goal::Tech(t) => self.know.get(c).map_or(0.0, |k| k.known.get(t).copied().unwrap_or(false) as u8 as f32),
            Goal::Reform(l, v) => (self.polity.of[c].laws[l] == v) as u8 as f32,
            Goal::Calm => self.polity.of[c].unrest,
            Goal::Outgrow => n.gdp_real / self.h.home(1 - s).gdp_real.max(1e-6),
            Goal::Faith => {
                let p = &self.polity.of[c];
                let shares = self.polity.shares(self.realm.regions.get(c).map_or(&[][..], |v| &v[..]));
                shares.iter().find(|x| x.0 == p.state_faith).map_or(0.0, |x| x.1)
            }
            Goal::Revanche(st) => {
                let w = &self.h.war;
                let provs: Vec<usize> = (0..self.prov_state.len()).filter(|&p| self.prov_state[p] as usize == st).collect();
                let mut held = 0;
                let mut all = 0;
                for p in provs {
                    if let Some(&q) = w.th.at.get(p) {
                        if q != strategy::military::front::NO {
                            all += 1;
                            if w.owner[q as usize] == s {
                                held += 1;
                            }
                        }
                    }
                }
                if all > 0 && held * 2 > all {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    /// How far the task has come (0..1).
    pub fn task_progress(&self, s: usize, t: &Task) -> f32 {
        let now = self.task_measure(s, &t.goal);
        match t.goal {
            Goal::Calm => ((t.base - now) / (t.base - t.target).max(1e-6)).clamp(0.0, 1.0),
            Goal::Tech(_) | Goal::Reform(..) | Goal::Revanche(_) => now.clamp(0.0, 1.0),
            _ => ((now - t.base) / (t.target - t.base).abs().max(1e-6)).clamp(0.0, 1.0),
        }
    }

    /// Tasks side `s` could take now, with who asks for each and how strongly.
    fn task_candidates(&self, s: usize) -> Vec<(Goal, Who, f32)> {
        use polity::*;
        let c = self.country(s);
        let p = &self.polity.of[c];
        let f = &self.spirits.facts[s];
        let cl = |b: usize| p.clout[b];
        let mv = |m: usize| p.moves[m];
        let mut v: Vec<(Goal, Who, f32)> = Vec::new();
        if f.industry_share < 0.45 {
            v.push((Goal::Industry, Who::Bloc(CAPITAL), 0.3 + cl(CAPITAL) + cl(WORKERS)));
        }
        if f.hunger > 0.03 || f.land_share > 0.4 {
            v.push((Goal::Harvest, Who::Bloc(PEASANTS), 0.2 + cl(PEASANTS) + cl(NOBLES) + 2.0 * f.hunger));
        }
        if p.literacy < 0.85 {
            v.push((Goal::Literacy, Who::Bloc(LEARNED), 0.2 + cl(LEARNED) + mv(LIBERALS)));
        }
        v.push((Goal::Army, Who::Bloc(OFFICERS), 0.2 + cl(OFFICERS) + mv(HAWKS)));
        if f.debt > 0.5 || f.treasury < 3.0 {
            v.push((Goal::Treasury, Who::Bloc(MERCHANTS), 0.2 + cl(MERCHANTS) + cl(CAPITAL) + 0.05 * f.debt.min(10.0)));
        }
        // a technique the rival has and we have not (or the cheapest open one)
        if let (Some(k), Some(o)) = (self.know.get(c), self.know.get(self.country(1 - s))) {
            let open = k.open(&self.pack);
            let theirs: Vec<usize> = open.iter().copied().filter(|&t| o.known[t]).collect();
            let pick = theirs.first().or(open.first()).copied();
            if let Some(t) = pick {
                v.push((Goal::Tech(t), Who::Bloc(LEARNED), 0.15 + cl(LEARNED) + if theirs.is_empty() { 0.0 } else { 0.3 }));
            }
        }
        // the strongest movement's demand
        for m in [LIBERALS, LEVELLERS, ZEALOTS, HAWKS] {
            if let Some(&(law, want)) = MOVE[m].aims.iter().find(|&&(l, w)| p.laws[l] != w) {
                if self.reform_allowed(s, law, if want > p.laws[law] { p.laws[law] + 1 } else { p.laws[law].saturating_sub(1) }).is_ok() || self.realm.pending[s].is_some() {
                    let to = if want > p.laws[law] { p.laws[law] + 1 } else { p.laws[law] - 1 };
                    v.push((Goal::Reform(law, to), Who::Move(m), mv(m) * 1.2));
                }
            }
        }
        if p.unrest > 0.3 {
            v.push((Goal::Calm, Who::Move(GUARDIANS), 0.2 + mv(GUARDIANS) + p.unrest));
        }
        if self.h.home(1 - s).gdp_real > self.h.home(s).gdp_real {
            v.push((Goal::Outgrow, Who::Move(HAWKS), 0.15 + mv(HAWKS) + cl(CAPITAL)));
        }
        let shares = self.polity.shares(self.realm.regions.get(c).map_or(&[][..], |x| &x[..]));
        let ours = shares.iter().find(|x| x.0 == p.state_faith).map_or(0.0, |x| x.1);
        if ours < 0.85 && p.laws[CHURCH] < 2 {
            v.push((Goal::Faith, Who::Move(ZEALOTS), 0.1 + mv(ZEALOTS) + cl(CLERGY)));
        }
        // a lost region the rival holds
        let rival = self.country(1 - s);
        if let Some(&(st, _, _)) = self.lost_lands.iter().find(|l| l.1 as usize == c && self.country_of.iter().enumerate().any(|(pv, &o)| o as usize == rival && self.prov_state[pv] == l.0)) {
            v.push((Goal::Revanche(st as usize), Who::Move(REVANCHE), 0.2 + 1.5 * mv(REVANCHE)));
        }
        v
    }

    /// Three offers for side `s` (drawn by how strongly each is asked for).
    fn make_offers(&mut self, s: usize) {
        let now = self.month();
        let mut cands = self.task_candidates(s);
        let mut r = Rng::new(hash64(self.seed ^ 0xA6E ^ (now as u64) << 3 ^ s as u64));
        let mut out = Vec::new();
        while out.len() < 3 && !cands.is_empty() {
            let all: f32 = cands.iter().map(|x| x.2.max(0.01)).sum();
            let mut pick = r.f32() * all;
            let i = cands.iter().position(|x| {
                pick -= x.2.max(0.01);
                pick <= 0.0
            }).unwrap_or(0);
            let (goal, who, _) = cands.remove(i);
            // (one of each kind)
            cands.retain(|x| x.0.code() != goal.code());
            let base = self.task_measure(s, &goal);
            let target = match goal {
                Goal::Industry => base * 1.15,
                Goal::Harvest => base * 1.08,
                Goal::Literacy => (base * 1.2).max(base + 0.01),
                Goal::Army => (base * 1.25).max(base + 6.0),
                Goal::Treasury => 6.0,
                Goal::Calm => 0.25,
                Goal::Outgrow => 1.0,
                Goal::Faith => (base + 0.1).min(0.95).max(0.8),
                _ => 1.0,
            };
            out.push(Task { goal, who, since: now, until: now + goal.months(), base, target });
        }
        self.agenda.offers[s] = out;
        self.agenda.offered[s] = now;
    }

    /// Side `s` takes offer `k`.
    pub fn take_task(&mut self, s: usize, k: usize) {
        if self.agenda.now[s].is_some() || k >= self.agenda.offers[s].len() {
            return;
        }
        let now = self.month();
        let mut t = self.agenda.offers[s].remove(k);
        t.since = now;
        t.until = now + t.goal.months();
        let title = self.task_title(&t);
        self.agenda.offers[s].clear();
        self.agenda.now[s] = Some(t);
        let name = self.h.home(s).name.clone();
        self.news.push(Note::new(now, &if s == self.me { format!("великая задача страны: «{title}»") } else { format!("{name}: великая задача — «{title}»") }, false, "society"));
    }

    /// The tasks' month, both sides: done or failed, rewards end, new offers; the bot chooses. What
    /// should stop the clock.
    pub fn agenda_month(&mut self) -> Vec<String> {
        let now = self.month();
        let me = self.me;
        let mut alerts = Vec::new();
        for s in 0..2 {
            self.agenda.rewards[s].retain(|r| r.1 > now);
            if let Some(t) = self.agenda.now[s].clone() {
                let done = self.task_progress(s, &t) >= 1.0;
                if done || now >= t.until {
                    let c = self.country(s);
                    let title = self.task_title(&t);
                    let name = self.h.home(s).name.clone();
                    let p = &mut self.polity.of[c];
                    if done {
                        p.legit = (p.legit + 0.1).min(1.0);
                        if let Who::Bloc(b) = t.who {
                            self.realm.favour[s][b] += 0.4;
                        }
                        if let Who::Move(m) = t.who {
                            p.moves[m] = (p.moves[m] - 0.15).max(0.0);
                        }
                        self.agenda.rewards[s].push((t.goal, now + 60));
                        let line = format!("задача выполнена: «{title}» — {}", t.goal.reward_words());
                        if s == me {
                            alerts.push(line.clone());
                        }
                        self.news.push(Note::new(now, &if s == me { line } else { format!("{name}: {line}") }, s == me, "society"));
                    } else {
                        p.legit = (p.legit - 0.08).max(0.05);
                        match t.who {
                            Who::Bloc(b) => self.realm.favour[s][b] -= 0.3,
                            Who::Move(m) => p.moves[m] = (p.moves[m] + 0.1).min(1.0),
                        }
                        let line = format!("задача провалена: «{title}» — {} разочарованы", t.who.name().to_lowercase());
                        if s == me {
                            alerts.push(line.clone());
                        }
                        self.news.push(Note::new(now, &if s == me { line } else { format!("{name}: {line}") }, s == me, "society"));
                    }
                    self.agenda.past[s].push((t, done, now));
                    self.agenda.now[s] = None;
                }
            }
            if self.agenda.now[s].is_none() && (self.agenda.offers[s].is_empty() || now >= self.agenda.offered[s] + 12) {
                self.make_offers(s);
            }
            // the bot takes the task its course is about
            if s != me || self.auto.is_some() {
                if self.agenda.now[s].is_none() && !self.agenda.offers[s].is_empty() {
                    let course = self.courses.now[s].as_ref().map(|c| c.kind);
                    let want = |g: &Goal| -> f32 {
                        use crate::country::course::Kind as K;
                        match (course, g) {
                            (Some(K::Industry), Goal::Industry) | (Some(K::Agrarian), Goal::Harvest) | (Some(K::Army), Goal::Army) | (Some(K::Fortress), Goal::Army) => 3.0,
                            (Some(K::Science), Goal::Literacy | Goal::Tech(_)) | (Some(K::Thrift), Goal::Treasury) | (Some(K::Welfare), Goal::Calm | Goal::Harvest) => 3.0,
                            (Some(K::Revanche), Goal::Revanche(_) | Goal::Army) | (Some(K::Trade), Goal::Outgrow | Goal::Treasury) => 3.0,
                            _ => 1.0,
                        }
                    };
                    let k = (0..self.agenda.offers[s].len()).max_by(|&a, &b| want(&self.agenda.offers[s][a].goal).total_cmp(&want(&self.agenda.offers[s][b].goal))).unwrap_or(0);
                    self.take_task(s, k);
                }
            }
        }
        alerts
    }

    /// What side `s`'s fulfilled tasks give, together.
    pub fn agenda_eff(&self, s: usize) -> Eff {
        let mut e = Eff::default();
        for (g, _) in &self.agenda.rewards[s] {
            e = e.and(&g.reward());
        }
        e
    }

    /// The player's decisions: take an offer (`task=k`), drop the task (`task_drop`).
    pub fn agenda_form(&mut self, s: usize, form: &[(String, String)]) {
        let get = |k: &str| form.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
        if let Some(k) = get("task").and_then(|v| v.trim().parse::<usize>().ok()) {
            self.take_task(s, k);
        }
        if get("task_drop").is_some() {
            if let Some(t) = self.agenda.now[s].take() {
                let c = self.country(s);
                let p = &mut self.polity.of[c];
                p.legit = (p.legit - 0.04).max(0.05);
                let now = self.month();
                self.agenda.past[s].push((t, false, now));
                self.make_offers(s);
            }
        }
    }
}
