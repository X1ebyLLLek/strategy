//! Science: a country's laboratories, fed by its economy.
//!
//! What science can do comes from the people working in the laboratories: the learned the pack
//! names (служащие, дворяне, механики…), drawn there by what the laboratories pay against what they
//! earn elsewhere in their own country (`Nation::scholars`) — not by how rich the country is. In a
//! war, trophies help: where the enemy's units of a kind are better, studying them speeds that
//! kind's designs and brings them towards the enemy's — as much as the country's scientists can
//! take in. The power goes to spheres: each kind of unit
//! of the pack, and the economy. Each new design of a kind changes what that kind really is (fire,
//! toughness) by an amount nobody knows: designers claim more than they made, the staff believes
//! them, and exercises and battles show the truth (`doctrine::Country`). A design may come out
//! worse than the one before (about a third are). A design for the economy makes every sector more
//! productive. Each design of a sphere costs more than the one before: an era's technique has its
//! limits. A head of the laboratories tries each design before the army gets it, and turns down
//! those that test worse than what it has — the better he judges, the fewer bad ones get through;
//! without a head, every design goes to the army.

use crate::economy::advisor::Mind;
use crate::economy::nation::Nation;
use crate::military::doctrine::Country;
use crate::pack::Pack;
use crate::rng::{hash64, Rng};
use crate::world::map::Terrain;

/// Designs a month a million people in the laboratories make.
const POWER: f32 = 0.4;
/// In a war, work towards a kind's next design from trophies, a month: this share of science's
/// power for each design's worth (`GAIN`) the enemy's units of the kind are better than ours; and a
/// design made then closes this share of the gap besides what it would give anyway.
const TROPHY: f32 = 0.05;
const COPY: f32 = 0.2;
/// What a new design of a kind of unit really adds (log, fire and toughness each), give or take.
const GAIN: f32 = 0.06;
const GAIN_SD: f32 = 0.12;
/// What a new design for the economy adds to every sector's productivity (log), give or take.
const PROD_GAIN: f32 = 0.02;
const PROD_SD: f32 = 0.02;
/// Designers claim this much more than they made, give or take (log).
const CLAIM: f32 = 0.1;
const CLAIM_SD: f32 = 0.15;
/// How much less sure the staff is of a kind after it gets a new design (variance, log).
pub const DOUBT: f32 = 0.05;
/// Each design of a sphere costs this much more than the one before (as a share of the first).
const HARDER: f32 = 0.25;
/// A scientist sits over the laboratories twice a year.
const SITTING: u32 = 6;
/// How much of the laboratories an uneducated scientist gives to the sphere he likes best.
const FAVOURITE: f32 = 0.6;
/// How wrong an uneducated, green scientist reads the staff's war games (share of games won).
const MISREAD: f32 = 0.05;
/// Chance per sitting that an uneducated, green scientist throws everything at one sphere.
const SLIP: f32 = 0.4;
/// Share of science's money a wholly dishonest scientist takes.
const TAKE: f32 = 0.3;
/// How wrong an uneducated, green scientist judges a design at its trials (log, sd).
const TRIAL: f32 = 0.15;
/// Months for experience to grow by about two thirds of what is left.
const LEARN: f32 = 120.0;
/// A scientist asks the war games about this many designs at once (one design changes too few of
/// the games to tell spheres apart).
const PROBE: f32 = 5.0;

/// One design that came out of the laboratories.
#[derive(Clone, Debug)]
pub struct Work {
    pub month: u32,
    /// The kind of unit, or None for the economy.
    pub kind: Option<usize>,
    /// What it really gave (hidden) and what its makers claim (log; for the economy, the claim
    /// is the truth: output shows it at once).
    pub gain: (f32, f32),
    pub claim: (f32, f32),
    /// Whether the army got it (a head of the laboratories may turn it down after trials).
    pub adopted: bool,
}

/// A country's laboratories.
#[derive(Clone)]
pub struct Lab {
    /// Share of science's power each sphere gets: each kind of unit, then the economy.
    pub focus: Vec<f32>,
    /// Work done towards each sphere's next design, and designs made.
    pub progress: Vec<f32>,
    pub made: Vec<u32>,
    pub works: Vec<Work>,
    /// Designs a month science can make at present.
    pub power: f32,
    /// Share of science's money that does not reach the laboratories (taken).
    pub leak: f32,
    /// How wrong the head of the laboratories judges a design at its trials (log, sd); None: no
    /// trials, every design goes to the army.
    pub trial: Option<f32>,
    /// Whether the laboratories study trophies in a war (off: a control).
    pub capture: bool,
    /// How much better the enemy's units of each kind are than ours, as trophies show it (log),
    /// this month of war.
    pub behind: Vec<f32>,
    /// Science's power this many times (what the country's schools and spirit make of its
    /// laboratories; 1: as the scholars give it).
    pub boost: f32,
    /// Whether a new design of a kind of unit waits for the government's word before the army
    /// gets it (the game: a model is put into production, or not); the designs waiting.
    pub hold: bool,
    pub offered: Vec<Work>,
    rng: Rng,
}

impl Lab {
    /// Laboratories with every sphere alike.
    pub fn new(pack: &Pack, seed: u64) -> Lab {
        let k = pack.units.len() + 1;
        Lab {
            focus: vec![1.0 / k as f32; k],
            progress: vec![0.0; k],
            made: vec![0; k],
            works: Vec::new(),
            power: 0.0,
            leak: 0.0,
            trial: None,
            capture: true,
            behind: vec![0.0; pack.units.len()],
            boost: 1.0,
            hold: false,
            offered: Vec::new(),
            rng: Rng::new(hash64(seed ^ 0x5C1)),
        }
    }

    /// What science can do this month: designs a month, from the people in the laboratories.
    pub fn power_of(n: &Nation) -> f32 {
        POWER * n.scholars.iter().sum::<f32>()
    }

    /// A month of the laboratories of country `n`, whose army's staff is `c`; `foe`: the enemy's
    /// staff, in a war.
    pub fn month(&mut self, pack: &Pack, n: &mut Nation, c: &mut Country, foe: Option<&Country>, month: u32) {
        self.power = Lab::power_of(n) * (1.0 - self.leak).max(0.0);
        if self.boost != 1.0 {
            self.power *= self.boost;
        }
        let all: f32 = self.focus.iter().sum::<f32>().max(1e-9);
        let nu = pack.units.len();
        // trophies: what the enemy fields of a kind better than ours
        for u in 0..nu {
            self.behind[u] = match foe {
                Some(f) if self.capture && f.army.count[u] > 0.0 => {
                    (0.5 * ((f.truth_fire[u] / c.truth_fire[u]).ln() + (f.truth_tough[u] / c.truth_tough[u]).ln())).max(0.0)
                }
                _ => 0.0,
            };
            self.progress[u] += self.power * TROPHY * self.behind[u] / GAIN;
        }
        for k in 0..self.focus.len() {
            self.progress[k] += self.power * self.focus[k] / all;
            loop {
                let cost = 1.0 + HARDER * self.made[k] as f32;
                if self.progress[k] < cost {
                    break;
                }
                self.progress[k] -= cost;
                self.made[k] += 1;
                let mut work = self.design(k, month);
                if let (Some(sd), Some(_)) = (self.trial, work.kind) {
                    let seen = 0.5 * (work.gain.0 + work.gain.1) + sd * self.rng.normal();
                    work.adopted = seen > 0.0;
                }
                if work.adopted && self.hold && work.kind.is_some() {
                    // (it waits for the government's word)
                    self.offered.push(work.clone());
                } else if work.adopted {
                    match work.kind {
                        Some(u) => c.redesign(u, work.gain, work.claim, DOUBT),
                        None => n.prod.iter_mut().for_each(|p| *p *= work.gain.0.exp()),
                    }
                }
                self.works.push(work);
            }
        }
    }

    fn design(&mut self, k: usize, month: u32) -> Work {
        let r = &mut self.rng;
        if k == self.focus.len() - 1 {
            let g = PROD_GAIN + PROD_SD * r.normal();
            return Work { month, kind: None, gain: (g, g), claim: (g, g), adopted: true };
        }
        let mean = GAIN + COPY * self.behind[k];
        let gain = (mean + GAIN_SD * r.normal(), mean + GAIN_SD * r.normal());
        let claim = (gain.0 + CLAIM + CLAIM_SD * r.normal(), gain.1 + CLAIM + CLAIM_SD * r.normal());
        Work { month, kind: Some(k), gain, claim, adopted: true }
    }
}

/// A scientist heading the laboratories: he chooses what they work on. Everything about him is
/// hidden, and his work depends only on his education, experience, beliefs and talent (as an
/// economist's): an able one asks the general staff's war games how much more often the army would
/// win if a kind of its units were a design better, or if it were as much bigger as a design for
/// the economy would make the country richer, and works on what wins most; an ordinary one works on
/// what the army builds; an uneducated one on the sphere he likes; a dishonest one takes a part of
/// science's money.
#[derive(Clone)]
pub struct Scientist {
    pub mind: Mind,
    rng: Rng,
    /// The sphere he likes best.
    favourite: usize,
    pub months: u32,
    /// Taken from science's money, all told (hidden).
    pub took: f32,
    /// What he set the laboratories to, sitting by sitting: (month, focus).
    pub record: Vec<(u32, Vec<f32>)>,
}

impl Scientist {
    pub fn new(mind: Mind, seed: u64, pack: &Pack) -> Scientist {
        let mut rng = Rng::new(hash64(seed ^ 0x5C1E));
        let favourite = rng.below(pack.units.len() + 1);
        Scientist { mind, rng, favourite, months: 0, took: 0.0, record: Vec::new() }
    }

    /// A month in office, after the economy's month and before the laboratories': he sits twice a
    /// year over `lab` of country `n`, whose staff is `c` and whose front is `front`; and takes his
    /// part of the money science got.
    pub fn month(&mut self, pack: &Pack, lab: &mut Lab, n: &mut Nation, c: &Country, front: &[(Terrain, f32)], seed: u64) {
        if self.months % SITTING == 0 {
            self.sit(pack, lab, c, front, seed);
        }
        let share = TAKE * (1.0 - self.mind.beliefs.honesty).powi(2);
        lab.leak = share;
        lab.trial = Some(TRIAL * (1.0 - self.mind.education) * (1.0 - 0.5 * self.mind.experience));
        let take = share * n.science_money;
        if take > 0.0 {
            if let Some(science) = &pack.science {
                // out of what the learned were paid, to the people he lives among
                let learned: f32 = science.learned.iter().map(|&g| n.pop[g]).sum();
                let rich = (0..n.pop.len())
                    .filter(|&g| n.pop[g] > 0.0)
                    .max_by(|&a, &b| (n.money[a] / n.pop[a]).total_cmp(&(n.money[b] / n.pop[b])))
                    .unwrap_or(0);
                if learned > 0.0 {
                    for &g in &science.learned {
                        n.money[g] -= take * n.pop[g] / learned;
                    }
                    n.money[rich] += take;
                    self.took += take;
                }
            }
        }
        self.months += 1;
        self.mind.experience += (1.0 - self.mind.experience) / LEARN;
    }

    fn sit(&mut self, pack: &Pack, lab: &mut Lab, c: &Country, front: &[(Terrain, f32)], seed: u64) {
        let nu = pack.units.len();
        let k = nu + 1;
        let e = self.mind.education;
        let mut focus = vec![0.0f32; k];
        if self.mind.ways() > 0 && !front.is_empty() {
            // what each sphere would win, as the staff's war games see it (the same luck for all)
            let seed = hash64(seed ^ 0x5A7 ^ self.months as u64);
            // an even fight first: the size of our army at which the games are won half the time
            // (a staff sure of winning every game cannot tell what would help)
            let (mut lo, mut hi, mut size) = (0.125f32, 8.0f32, 1.0f32);
            for _ in 0..6 {
                if c.games_if(pack, front, seed, None, size) > 0.5 {
                    hi = size;
                } else {
                    lo = size;
                }
                size = (lo * hi).sqrt();
            }
            let base = c.games_if(pack, front, seed, None, size);
            let err = MISREAD * (1.0 - e) * (1.0 - 0.5 * self.mind.experience);
            let mut gain = vec![0.0f32; k];
            for u in 0..nu {
                gain[u] = c.games_if(pack, front, seed, Some((u, (PROBE * GAIN).exp())), size) - base + err * self.rng.normal();
            }
            gain[nu] = (c.games_if(pack, front, seed, None, size * (PROBE * PROD_GAIN).exp()) - base + err * self.rng.normal()) * (1.0 + self.mind.beliefs.state);
            let best = gain.iter().cloned().fold(0.0f32, f32::max);
            if best > 0.0 {
                for i in 0..k {
                    focus[i] = gain[i].max(0.0) + 0.05 * best;
                }
            }
        }
        if focus.iter().sum::<f32>() <= 0.0 {
            // the rule: work on what the army builds, and the economy as one sphere among them
            for u in 0..nu {
                focus[u] = c.plan[u] * nu as f32 / k as f32;
            }
            focus[nu] = (1.0 + self.mind.beliefs.state).max(0.0) / k as f32;
        }
        let all: f32 = focus.iter().sum::<f32>().max(1e-9);
        focus.iter_mut().for_each(|f| *f /= all);
        // the sphere he likes, the more the less he knows
        let fav = FAVOURITE * (1.0 - e);
        for (i, f) in focus.iter_mut().enumerate() {
            *f = *f * (1.0 - fav) + if i == self.favourite { fav } else { 0.0 };
        }
        // and sometimes everything on one sphere
        if self.rng.f32() < SLIP * (1.0 - e) * (1.0 - self.mind.experience) {
            let pick = self.rng.below(k);
            focus = (0..k).map(|i| if i == pick { 1.0 } else { 0.0 }).collect();
        }
        lab.focus = focus.clone();
        self.record.push((self.months, focus));
    }
}
