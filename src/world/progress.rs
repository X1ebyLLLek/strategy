//! What a country knows: the techniques of its era (`Pack::techs`) — not a tree to click through
//! but what the world made of it. At the beginning of the world's past every country knows the
//! era's first techniques; over a hundred years each comes to know more by its own science (how
//! many learned people it has, how rich it is, its scientists — a genius may give it in a year what
//! others take a generation over), by its neighbours (what they know costs less to learn: it is
//! seen, traded, copied) and by its wars (the beaten learn the victor's arms). What a country knows
//! weighs on its trades (productivity), its kinds of troops (fire, toughness — and kinds it cannot
//! field at all: no tanks without armour), its laboratories, factories, government and supply.
//! Its stage of development is how far it has come through the era's tiers.

use crate::pack::{Pack, Sphere};
use crate::rng::Rng;

/// A tier counts as mastered when a country knows this share of its techniques.
const MASTERED: f32 = 0.6;
/// What neighbours knowing a technique take off its cost, at most (all of them knowing it).
const SEEN: f32 = 0.6;
/// Chance a year that a technique most neighbours know comes into a country by itself (trade,
/// travellers, émigrés), whatever its own science does.
const DRIFT: f32 = 0.04;

/// A country's techniques.
#[derive(Clone, Debug)]
pub struct Know {
    /// Whether it knows each technique of the pack; the month it came to know it (from the game's
    /// beginning: the past is below 0); who gave it (a person of `people::Folk`), if anyone.
    pub known: Vec<bool>,
    pub when: Vec<i32>,
    pub by: Vec<Option<u32>>,
    /// Research done towards each (years of an ordinary country's science).
    pub progress: Vec<f32>,
    /// What its science works on now.
    pub target: Option<usize>,
    /// The newest tier anyone has invented yet (the world's past opens the era's tiers one after
    /// another; in the game, all are).
    pub newest: usize,
}

impl Know {
    /// What every country of the era knows at the beginning of the world's past: the first tier.
    pub fn start(pack: &Pack, month: i32) -> Know {
        let n = pack.techs.len();
        let known: Vec<bool> = pack.techs.iter().map(|t| t.tier == 0).collect();
        Know { when: vec![month; n], known, by: vec![None; n], progress: vec![0.0; n], target: None, newest: usize::MAX }
    }

    /// Whether it may work on technique `t`: not known yet, invented, everything it needs known.
    pub fn can(&self, pack: &Pack, t: usize) -> bool {
        !self.known[t] && pack.techs[t].tier <= self.newest && pack.techs[t].needs.iter().all(|&n| self.known[n])
    }

    /// The techniques it may work on now.
    pub fn open(&self, pack: &Pack) -> Vec<usize> {
        (0..pack.techs.len()).filter(|&t| self.can(pack, t)).collect()
    }

    /// How many techniques it knows.
    pub fn count(&self) -> usize {
        self.known.iter().filter(|&&k| k).count()
    }

    /// Kinds of units it cannot field: some technique opens them, and it knows none of those.
    pub fn locked(&self, pack: &Pack) -> Vec<bool> {
        (0..pack.units.len())
            .map(|u| {
                let openers: Vec<usize> = (0..pack.techs.len()).filter(|&t| pack.techs[t].unlocks.contains(&u)).collect();
                !openers.is_empty() && !openers.iter().any(|&t| self.known[t])
            })
            .collect()
    }

    /// What it knows adds to each sector's productivity (log).
    pub fn sector_gain(&self, pack: &Pack) -> Vec<f32> {
        let mut g = vec![0.0f32; pack.sectors.len()];
        for (t, tech) in pack.techs.iter().enumerate() {
            if self.known[t] {
                for &(k, x) in &tech.sectors {
                    g[k] += x;
                }
            }
        }
        g
    }

    /// What it knows adds to each kind of units' fire and toughness (log).
    pub fn unit_gain(&self, pack: &Pack) -> Vec<f32> {
        let mut g = vec![0.0f32; pack.units.len()];
        for (t, tech) in pack.techs.iter().enumerate() {
            if self.known[t] {
                for &(u, x) in &tech.units {
                    g[u] += x;
                }
            }
        }
        g
    }

    /// Shares it adds together: science's power, factories' learning, stability a year, the draft,
    /// supply.
    pub fn sums(&self, pack: &Pack) -> Sums {
        let mut s = Sums::default();
        for (t, tech) in pack.techs.iter().enumerate() {
            if self.known[t] {
                s.science += tech.science;
                s.learn += tech.learn;
                s.stability += tech.stability;
                s.draft += tech.draft;
                s.supply += tech.supply;
            }
        }
        s
    }

    /// Its stage of development: the tiers mastered, one after another.
    pub fn stage(&self, pack: &Pack) -> usize {
        let top = pack.techs.iter().map(|t| t.tier).max().unwrap_or(0);
        let mut stage = 0;
        for tier in 1..=top {
            let all = pack.techs.iter().filter(|t| t.tier == tier).count();
            if all == 0 {
                continue;
            }
            let k = (0..pack.techs.len()).filter(|&t| pack.techs[t].tier == tier && self.known[t]).count();
            if (k as f32) < MASTERED * all as f32 {
                break;
            }
            stage = tier;
        }
        stage.min(pack.stages.len().saturating_sub(1))
    }

    /// How far it is into the next tier (0..1), for a gauge.
    pub fn next_stage(&self, pack: &Pack) -> f32 {
        let tier = self.stage(pack) + 1;
        let all = pack.techs.iter().filter(|t| t.tier == tier).count();
        if all == 0 {
            return 1.0;
        }
        let k = (0..pack.techs.len()).filter(|&t| pack.techs[t].tier == tier && self.known[t]).count();
        (k as f32 / (MASTERED * all as f32)).min(1.0)
    }

    /// What technique `t` costs it, when `seen` of the countries it deals with know it already.
    pub fn cost(pack: &Pack, t: usize, seen: f32) -> f32 {
        pack.techs[t].cost * (1.0 - SEEN * seen.clamp(0.0, 1.0))
    }

    /// It comes to know `t` (month `when`, by person `by`).
    pub fn learn(&mut self, t: usize, when: i32, by: Option<u32>) {
        if !self.known[t] {
            self.known[t] = true;
            self.when[t] = when;
            self.by[t] = by;
            self.progress[t] = 0.0;
            if self.target == Some(t) {
                self.target = None;
            }
        }
    }

    /// `points` (years of an ordinary country's science) go to what it works on: what is done
    /// becomes known, what is left over goes to the next one `pick` chooses among those it may work
    /// on; `seen(t)` — the share of those it deals with that know `t`. The techniques learnt.
    pub fn research(&mut self, pack: &Pack, mut points: f32, seen: &dyn Fn(usize) -> f32, pick: &mut dyn FnMut(&Know, &[usize]) -> Option<usize>, when: i32, by: Option<u32>) -> Vec<usize> {
        let mut got = Vec::new();
        for _ in 0..8 {
            if points <= 1e-6 {
                break;
            }
            let target = match self.target.filter(|&t| self.can(pack, t)) {
                Some(t) => t,
                None => {
                    let open = self.open(pack);
                    match pick(self, &open) {
                        Some(t) => t,
                        None => break,
                    }
                }
            };
            self.target = Some(target);
            let cost = Know::cost(pack, target, seen(target)).max(0.05);
            let need = (cost - self.progress[target]).max(0.0);
            if points >= need {
                points -= need;
                self.learn(target, when, by);
                got.push(target);
            } else {
                self.progress[target] += points;
                points = 0.0;
            }
        }
        got
    }

    /// Techniques most of those it deals with know come in by themselves now and then (trade,
    /// travellers): the ones that came.
    pub fn drift(&mut self, pack: &Pack, seen: &dyn Fn(usize) -> f32, rng: &mut Rng, when: i32, chance: f32) -> Vec<usize> {
        let mut got = Vec::new();
        for t in self.open(pack) {
            let s = seen(t);
            if s >= 0.5 && rng.f32() < chance * s {
                self.learn(t, when, None);
                got.push(t);
            }
        }
        got
    }

    /// The chance a year of `drift` in the world's past.
    pub const DRIFT: f32 = DRIFT;
}

/// The shares a country's techniques add together.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sums {
    pub science: f32,
    pub learn: f32,
    pub stability: f32,
    pub draft: f32,
    pub supply: f32,
}

/// The name of stage `k` of the era.
pub fn stage_name(pack: &Pack, k: usize) -> &str {
    pack.stages.get(k.min(pack.stages.len().saturating_sub(1))).map_or("—", |s| s.as_str())
}

/// What a country leans to when it chooses what to work on: a weight for each sphere, and for each
/// kind of units (techniques for that kind), and for the economy's sectors (techniques for them).
#[derive(Clone, Debug)]
pub struct Leaning {
    pub sphere: [f32; 4],
    pub kinds: Vec<f32>,
    pub sectors: Vec<f32>,
}

impl Leaning {
    /// Every sphere, kind and sector alike.
    pub fn even(pack: &Pack) -> Leaning {
        Leaning { sphere: [1.0; 4], kinds: vec![1.0; pack.units.len()], sectors: vec![1.0; pack.sectors.len()] }
    }

    fn sphere_index(s: Sphere) -> usize {
        match s {
            Sphere::Economy => 0,
            Sphere::Military => 1,
            Sphere::Science => 2,
            Sphere::State => 3,
        }
    }

    /// How much it wants technique `t`: its sphere's weight, more for what opens or betters the
    /// kinds and sectors it cares for; cheaper ones first.
    pub fn want(&self, pack: &Pack, t: usize, seen: f32) -> f32 {
        let tech = &pack.techs[t];
        let mut w = self.sphere[Leaning::sphere_index(tech.sphere)].max(0.01);
        let mut boost = 0.0f32;
        for &(u, x) in &tech.units {
            boost += self.kinds.get(u).copied().unwrap_or(1.0) * x * 6.0;
        }
        for &u in &tech.unlocks {
            boost += self.kinds.get(u).copied().unwrap_or(1.0) * 0.8;
        }
        for &(k, x) in &tech.sectors {
            boost += self.sectors.get(k).copied().unwrap_or(1.0) * x * 5.0;
        }
        w *= 1.0 + boost;
        w / Know::cost(pack, t, seen).max(0.3)
    }

    /// Picks among `open` by weight (the luck of who happened to work on what).
    pub fn pick(&self, pack: &Pack, open: &[usize], seen: &dyn Fn(usize) -> f32, rng: &mut Rng) -> Option<usize> {
        if open.is_empty() {
            return None;
        }
        let w: Vec<f32> = open.iter().map(|&t| self.want(pack, t, seen(t)).powi(2)).collect();
        let all: f32 = w.iter().sum();
        if all <= 0.0 {
            return Some(open[rng.below(open.len())]);
        }
        let mut r = rng.f32() * all;
        for (i, &x) in w.iter().enumerate() {
            r -= x;
            if r <= 0.0 {
                return Some(open[i]);
            }
        }
        open.last().copied()
    }

    /// The best of `open` by weight (no luck: the player's adviser and the bot's government choose
    /// by reckoning).
    pub fn best(&self, pack: &Pack, open: &[usize], seen: &dyn Fn(usize) -> f32) -> Option<usize> {
        open.iter().copied().max_by(|&a, &b| self.want(pack, a, seen(a)).total_cmp(&self.want(pack, b, seen(b))))
    }
}

/// The world's mean of each sector's productivity gain (log) over `knows`: a country is as
/// productive as the era expects when it knows what the world knows on average (the pack's
/// figures were reckoned for that), more when it knows more.
pub fn mean_sector_gain(pack: &Pack, knows: &[Know], weights: &[f32]) -> Vec<f32> {
    let mut m = vec![0.0f32; pack.sectors.len()];
    let all: f32 = weights.iter().sum::<f32>().max(1e-9);
    for (k, w) in knows.iter().zip(weights) {
        for (s, g) in k.sector_gain(pack).into_iter().enumerate() {
            m[s] += g * w / all;
        }
    }
    m
}
