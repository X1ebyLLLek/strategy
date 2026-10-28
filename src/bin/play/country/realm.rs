//! The order of society in the game (`strategy::world::polity` — what two hundred years made of each
//! country): month by month the forces of society judge the government by what it does — the tax
//! and who pays it, duties, the labour law, credit and the press, the army's and science's shares,
//! the draft, the war and how it goes, hunger, the programmes — and by its laws; the movements grow
//! or fade; unrest and legitimacy follow. The government's form, its laws and its faith weigh on
//! the economy, science, the draft, the divisions and stability (`realm_eff`).
//!
//! What breaks comes as events with choices for both sides (the bot chooses for itself): a
//! revolution near, officers plotting, a movement's demand, a schism, an election, provinces that
//! want to leave. The player may start a reform of any law (it takes months; those it suits are glad
//! while it goes, those it hurts are not), rule with an iron hand for a year, buy a force's favour,
//! or change the form of government from above — each a bet with a price. Everything a force feels
//! is shown with its reasons and numbers (`mood_parts`).

use strategy::rng::{hash64, Rng};
use strategy::world::polity::{self, Life, Regime, Upheaval, BLOCS, LAW, LAWS, MOVE, MOVES};

use crate::country::spirits::Eff;
use crate::game::Note;
use crate::Game;

mod interests;
mod moods;
mod upheaval;
// (what the window shows of the order of society)
#[cfg(feature = "native")]
pub use moods::{law_eff, MOBILITY, RENT, SERF_RENT};
#[cfg(feature = "native")]
pub use upheaval::{favour_cost, reform_months};
use upheaval::revolution_to;

/// A reform under way: the law, the level it goes to, months done and needed.
#[derive(Clone, Debug)]
pub struct Pending {
    pub law: usize,
    pub to: u8,
    pub done: u32,
    pub months: u32,
}

/// The game's side of the order of society, both sides.
#[derive(Clone, Debug, Default)]
pub struct Realm {
    pub pending: [Option<Pending>; 2],
    /// Months of the iron hand left; of the officers' purge.
    pub crackdown: [u32; 2],
    pub purge: [u32; 2],
    /// Favour bought of each force (a mood it fades from).
    pub favour: [[f32; BLOCS]; 2],
    /// The month of the next election (elected governments).
    pub election: [u32; 2],
    /// Each country's regions with their people (as the game began).
    pub regions: Vec<Vec<(usize, f32)>>,
    /// What each force's mood comes from now, both sides (for the panel and the charts).
    pub moods: [[f32; BLOCS]; 2],
    /// The month of the last change of government form (a coup from above waits).
    pub changed: [u32; 2],
    /// Each force's material lot month by month over the last year (`interests`), both sides.
    pub lots: [Vec<[f32; BLOCS]>; 2],
}

/// A force's mood from one thing: what, how much.
pub type Part = (String, f32);

/// The kinds of the order's events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ev {
    /// A revolution near, led by a movement.
    Revolution(usize),
    /// The officers plot.
    Officers,
    /// A movement demands a law.
    Demand(usize, usize, u8),
    /// A branch breaks away from the state faith.
    Schism,
    /// An election.
    Election,
    /// Provinces want to leave.
    Separatists,
}

impl Ev {
    pub fn code(&self) -> u32 {
        match self {
            Ev::Revolution(_) => 1,
            Ev::Officers => 2,
            Ev::Demand(m, ..) => 10 + *m as u32,
            Ev::Schism => 3,
            Ev::Election => 4,
            Ev::Separatists => 5,
        }
    }
}

/// The forms a government may give itself from above (the player's bet).
pub const FROM_ABOVE: [(Regime, &str); 5] = [
    (Regime::Constitutional, "Даровать конституцию"),
    (Regime::Republic, "Провозгласить республику"),
    (Regime::Junta, "Опереться на генералов"),
    (Regime::Party, "Власть одной партии"),
    (Regime::Theocracy, "Отдать власть церкви"),
];

impl Game {
    /// The order of side `s`'s country.
    pub fn polity_of(&self, s: usize) -> &polity::Polity {
        &self.polity.of[self.country(s)]
    }

    /// The order's start: each country's regions, the first elections.
    pub fn realm_start(&mut self) {
        let n = self.countries.len();
        let mut regions: Vec<Vec<(usize, f32)>> = vec![Vec::new(); n];
        for (p, &c) in self.country_of.iter().enumerate() {
            if (c as usize) < n {
                let st = self.prov_state[p] as usize;
                let pop = self.prov_people[p];
                match regions[c as usize].iter_mut().find(|r| r.0 == st) {
                    Some(r) => r.1 += pop,
                    None => regions[c as usize].push((st, pop)),
                }
            }
        }
        self.realm.regions = regions;
        for s in 0..2 {
            if let Some(y) = self.polity_of(s).regime.elected() {
                self.realm.election[s] = (y * 12.0 * (0.3 + 0.7 * ((self.seed >> (s * 3)) % 7) as f32 / 7.0)) as u32 + 6;
            }
            self.realm.moods[s] = self.moods_now(s);
        }
    }

    /// The order's month, both sides (and once a year every country of the world): moods follow
    /// what the governments do; reforms go on; the events of the order. What should stop the clock.
    pub fn realm_month(&mut self) -> Vec<String> {
        let now = self.month();
        let me = self.me;
        let mut alerts = Vec::new();
        let mut r = Rng::new(hash64(self.seed ^ 0x9EA1 ^ (now as u64) << 6));
        for s in 0..2 {
            let c = self.country(s);
            let moods = self.moods_now(s);
            self.realm.moods[s] = moods;
            for x in self.realm.favour[s].iter_mut() {
                *x *= 0.96;
            }
            self.realm.crackdown[s] = self.realm.crackdown[s].saturating_sub(1);
            self.realm.purge[s] = self.realm.purge[s].saturating_sub(1);
            let life = self.life_now(s, moods);
            let name = self.h.home(s).name.clone();
            let ups = self.polity.drift(c, &name, &life, &mut r);
            let _ = ups;
            // the iron hand keeps the lid on
            if self.realm.crackdown[s] > 0 {
                let p = &mut self.polity.of[c];
                p.unrest *= 0.97;
                for m in [polity::LIBERALS, polity::LEVELLERS, polity::SEPARATISTS] {
                    p.moves[m] *= 0.985;
                }
            }
            // a reform goes on
            if let Some(mut p) = self.realm.pending[s].take() {
                p.done += 1;
                if p.done >= p.months {
                    let from = self.polity.of[c].laws[p.law];
                    self.polity.reform(c, &name, p.law, p.to, None, now as i32);
                    let line = format!("реформа завершена — {}: {} вместо «{}»", LAW[p.law].name.to_lowercase(), LAW[p.law].levels[p.to as usize], LAW[p.law].levels[from as usize]);
                    if s == me {
                        alerts.push(line.clone());
                    }
                    self.news.push(Note::new(now, &if s == me { line } else { format!("{name}: {line}") }, s == me, "society"));
                } else {
                    self.realm.pending[s] = Some(p);
                }
            }
            // at the worst, nobody asks: the government falls
            let p = &self.polity.of[c];
            if p.unrest > 0.85 && p.legit < 0.2 && r.f32() < 0.25 {
                let (by, _) = p.top_move();
                let to = revolution_to(p, by);
                alerts.extend(self.revolt(s, to, by));
            }
        }
        // the rest of the world: a year of its order now and then
        if now % 12 == 6 {
            self.world_order_year(&mut r);
        }
        alerts
    }

    /// What side `s` lives through now, for its order.
    fn life_now(&self, s: usize, moods: [f32; BLOCS]) -> Life {
        let c = self.country(s);
        let f = &self.spirits.facts[s];
        let p = &self.polity.of[c];
        let regions = self.realm.regions.get(c).cloned().unwrap_or_default();
        let all = regions.iter().map(|r| r.1).sum::<f32>().max(1e-6);
        let foreign = regions.iter().filter(|r| self.polity.region_faith.get(r.0).is_some_and(|&x| x != p.state_faith && x != usize::MAX)).map(|r| r.1).sum::<f32>() / all;
        let taken = regions.iter().filter(|r| self.lost_lands.iter().any(|l| l.0 as usize == r.0 && l.1 as usize != c)).map(|r| r.1).sum::<f32>() / all;
        let ruler = self.folk.offices[c].ruler.map(|id| self.folk.get(id));
        Life {
            month: self.month() as i32,
            dev: self.dev_of(c),
            stage: self.know.get(c).map_or(0.0, |k| k.stage(&self.pack) as f32 / self.pack.stages.len().saturating_sub(1).max(1) as f32),
            industry: Some((f.industry_share * 2.0).clamp(0.0, 1.0)),
            at_war: self.h.at_war,
            last_war: f.last_war.map(|y| (y, f.last_won)),
            grudge: self.state.grudge[s],
            foreign,
            taken,
            hunger: f.hunger.clamp(0.0, 1.0),
            ruler_reform: ruler.map_or(if self.realm.crackdown[s] > 0 { -0.4 } else { 0.0 }, |r| r.reform),
            ruler_hawk: ruler.map_or(0.0, |r| r.hawk),
            ruler_general: ruler.is_some_and(|r| r.calling == strategy::world::folk::Calling::General),
            glory: self.h.war.sides[s].glory.iter().sum::<f32>().clamp(-1.0, 1.0),
            free_near: 0.0,
            new_ruler: None,
            moods: Some(moods),
            span: 1.0 / 12.0,
            // (the game knows its schools and who holds the money)
            schooling: Some(self.schooling(s)),
            wealth: Some(self.wealth_of_blocs(s)),
        }
    }

    /// A year of the order of every country but the two: it lives as in the past (its upheavals
    /// come by themselves), its faiths spread.
    fn world_order_year(&mut self, r: &mut Rng) {
        let n = self.countries.len().min(self.polity.of.len());
        let pair = [self.country(0), self.country(1)];
        let month = self.month() as i32;
        for c in 0..n {
            if pair.contains(&c) || self.realm.regions.get(c).is_none_or(|v| v.is_empty()) {
                continue;
            }
            let ruler = self.folk.offices[c].ruler.map(|id| self.folk.get(id));
            let life = Life {
                month,
                dev: self.dev_of(c),
                stage: self.know.get(c).map_or(0.0, |k| k.stage(&self.pack) as f32 / self.pack.stages.len().saturating_sub(1).max(1) as f32),
                ruler_reform: ruler.map_or(0.0, |r| r.reform),
                ruler_hawk: ruler.map_or(0.0, |r| r.hawk),
                new_ruler: self.folk.changed.get_mut(c).and_then(|x| x.take()),
                span: 1.0,
                ..Default::default()
            };
            let name = self.countries[c].0.clone();
            let ups = self.polity.year(c, &name, &life, r);
            for u in ups {
                if let Upheaval::Revolution { .. } | Upheaval::Coup { .. } = u {
                    let t = self.times_now(c);
                    let soldiers = self.polity.of[c].regime.soldiers();
                    if let Some(id) = self.folk.overthrow(&self.pack.clone(), c, &t, month, "переворот", soldiers, r) {
                        let nm = self.folk.get(id).name.clone();
                        self.polity.crown(c, &nm, month);
                    }
                    let line = self.polity.chronicle.last().map(|x| x.2.clone()).unwrap_or_default();
                    self.news.push(Note::new(self.month(), &line, false, "news"));
                }
            }
        }
        // (the faiths spread: a region at a time)
        let owner: Vec<Option<usize>> = (0..self.polity.region_faith.len()).map(|st| self.realm.regions.iter().position(|v| v.iter().any(|x| x.0 == st))).collect();
        let mut next: Vec<Vec<usize>> = vec![Vec::new(); self.polity.region_faith.len()];
        for (p, nb) in self.world.provinces.iter().enumerate().map(|(p, x)| (p, &x.neighbors)) {
            let a = self.prov_state[p] as usize;
            for &q in nb {
                let b = self.prov_state[q as usize] as usize;
                if a != b && a < next.len() && !next[a].contains(&b) {
                    next[a].push(b);
                }
            }
        }
        let dev: Vec<f32> = (0..n).map(|c| self.dev_of(c)).collect();
        let pair_faith: Vec<usize> = pair.iter().map(|&c| self.polity.of[c].state_faith).collect();
        let got = self.polity.spread(&|st| owner.get(st).copied().flatten(), &|st| next.get(st).cloned().unwrap_or_default(), &dev, r);
        for (_, c, _, to) in got {
            if let Some(s) = (0..2).find(|&s| pair[s] == c) {
                if to != pair_faith[s] && s == self.me {
                    let line = format!("область обратилась в «{}»", self.polity.faiths[to].name);
                    self.news.push(Note::new(self.month(), &line, false, "society"));
                }
            }
        }
    }

}

