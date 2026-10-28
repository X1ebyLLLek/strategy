//! Events: hard moments born of the state of the world, not written in advance — a group of the
//! people stirring, a famine, an empty treasury, an advisor taking from it, a general losing
//! battle after battle, the neighbour working up a claim, a war that wears the country out, a
//! government near its fall, an enemy asking for peace. Each comes with two or three choices and
//! what they cost (numbers on hover). The player's stop the clock till he chooses; the other
//! government chooses for itself, by what it has (money, how it stands), and the news tells it.

use strategy::rng::{hash64, Rng};

use crate::game::Note;
use crate::Game;
use crate::words::group_name;

mod write;
mod choose;

/// Months between two events of a side, at the least; months before the same kind comes again.
const PAUSE: u32 = 4;
const AGAIN: u32 = 12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Unrest(usize),
    Famine(usize),
    Treasury,
    Thief,
    General(usize),
    Claim,
    Weary,
    Coup,
    PeaceOffer,
    /// Lenders press for their money (a debt grown past bearing).
    Lenders,
    /// Prices run away.
    Dear,
    /// Reformers at court offer a programme for the country's worst want.
    Reform,
    /// A famous scientist (a person of the world) asks money for his work on a technique.
    Genius(u32, usize),
    /// A war hero (a person of the world) out of command speaks against the government.
    Hero(u32),
    /// Banks fail after cheap credit held too long.
    Panic,
    /// Owners take their money abroad.
    Flight,
    /// The order of society: a revolution near, officers plotting, a demand, a schism, an
    /// election, provinces that want to leave.
    Realm(crate::country::realm::Ev),
    /// An ally (a map country) at war calls us to its side.
    AllyCalls(usize),
    /// A country of the world offers a pact (its kind's number in `diplomacy::Kind::ALL`, the
    /// country).
    Offer(usize, usize),
}

impl Kind {
    fn code(&self) -> u32 {
        match self {
            Kind::Unrest(_) => 1,
            Kind::Famine(_) => 2,
            Kind::Treasury => 3,
            Kind::Thief => 4,
            Kind::General(_) => 5,
            Kind::Claim => 6,
            Kind::Weary => 7,
            Kind::Coup => 8,
            Kind::PeaceOffer => 9,
            Kind::Lenders => 10,
            Kind::Dear => 11,
            Kind::Reform => 12,
            Kind::Genius(..) => 13,
            Kind::Hero(_) => 14,
            Kind::Panic => 15,
            Kind::Flight => 16,
            Kind::Realm(e) => 100 + e.code(),
            Kind::AllyCalls(_) => 17,
            Kind::Offer(..) => 18,
        }
    }
}

/// An event waiting for the player's choice: its kind, words, choices (what, the numbers for the
/// note).
#[derive(Clone, Debug)]
pub struct Event {
    pub id: u64,
    pub side: usize,
    pub kind: Kind,
    pub title: String,
    pub text: String,
    pub choices: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default)]
pub struct Events {
    /// The player's events not yet answered (the first is shown).
    pub waiting: Vec<Event>,
    /// When each side last had an event, and each kind.
    pub last: [u32; 2],
    pub kinds: [Vec<(u32, u32)>; 2],
    pub next_id: u64,
    /// The economist's takings already noticed, of each side.
    pub took_seen: [f32; 2],
    /// What he had taken by last month, and how fast he takes (a share of what the state spends a
    /// month, smoothed over about half a year).
    pub took_prev: [f32; 2],
    pub theft: [f32; 2],
}

/// Takings below this share of the state's spending a month nobody's books show (the usual
/// leakage of any office); above it an audit may find them, the more likely the faster he takes and
/// the freer the press and the vote (a monthly chance of `FIND` per share above the floor).
const PETTY: f32 = 0.02;
const FIND: f32 = 0.6;

impl Game {
    /// The month's events, for both sides; what should stop the clock (the player's).
    pub fn events_month(&mut self) -> Vec<String> {
        let mut alerts = Vec::new();
        let now = self.month();
        // how fast each economist takes from the treasury
        for s in 0..2 {
            let took = self.h.econ[s].as_ref().map_or(0.0, |e| e.took);
            let d = (took - self.ev.took_prev[s]).max(0.0);
            self.ev.took_prev[s] = took;
            let budget = self.h.home(s).state_budget().max(1e-6);
            self.ev.theft[s] = if self.h.econ[s].is_some() { 0.85 * self.ev.theft[s] + 0.15 * (d / budget).min(1.0) } else { 0.0 };
        }
        for s in 0..2 {
            if now < self.ev.last[s] + PAUSE && self.ev.last[s] > 0 {
                continue;
            }
            let Some(kind) = self.what_happens(s) else { continue };
            let again = self.ev.kinds[s].iter().any(|&(c, m)| c == kind.code() && now < m + AGAIN);
            if again {
                continue;
            }
            self.ev.last[s] = now.max(1);
            self.ev.kinds[s].retain(|&(c, _)| c != kind.code());
            self.ev.kinds[s].push((kind.code(), now));
            let ev = self.event_of(s, kind);
            if s == self.me && self.auto.is_none() {
                alerts.push(ev.title.clone());
                self.news.push(Note::new(now, &ev.title, true, "politics"));
                self.ev.waiting.push(ev);
            } else {
                let k = self.bot_choice(&ev);
                let what = ev.choices.get(k).map_or(String::new(), |c| c.0.clone());
                self.choose(&ev, k);
                let name = self.h.home(s).name.clone();
                self.news.push(Note::new(now, &format!("{name}: {} — {}", ev.title, lower_first(&what)), s == self.me, "politics"));
            }
        }
        alerts
    }

    /// The hardest thing that is the matter with side `s` now, if anything is.
    fn what_happens(&self, s: usize) -> Option<Kind> {
        let pack = &self.pack;
        let n = self.h.home(s);
        let st = self.state.stability[s];
        if st < 0.2 {
            return Some(Kind::Coup);
        }
        // the enemy losing asks for peace (of the player)
        if self.h.at_war && s == self.me {
            let held = self.h.war.people();
            if held[1 - s] < held[s] - 0.12 && self.h.war.month >= 6 {
                return Some(Kind::PeaceOffer);
            }
        }
        if self.h.at_war && self.h.war.month >= 12 && st < 0.45 {
            return Some(Kind::Weary);
        }
        // the order of society: a revolution near, a plot, a demand, an election
        if let Some(e) = self.realm_happens(s) {
            return Some(Kind::Realm(e));
        }
        // a group near rising
        let worst = (0..pack.groups.len()).max_by(|&a, &b| (n.discontent[a] / pack.groups[a].patience).total_cmp(&(n.discontent[b] / pack.groups[b].patience)))?;
        if n.discontent[worst] > 0.7 * pack.groups[worst].patience {
            return Some(Kind::Unrest(worst));
        }
        // hunger among many
        let all: f32 = n.pop.iter().sum::<f32>().max(1e-6);
        for (g, grp) in pack.groups.iter().enumerate() {
            let starving = grp.needs.iter().enumerate().filter(|(_, x)| x.vital).map(|(k, _)| n.unmet[g][k]).fold(0.0f32, f32::max);
            if starving > 0.25 && n.pop[g] / all > 0.1 {
                return Some(Kind::Famine(g));
            }
        }
        // an empty treasury
        if n.treasury < 0.02 * n.gdp.max(1e-6) {
            return Some(Kind::Treasury);
        }
        // an economist taking from the treasury: only one who takes more than the usual leakage,
        // and only when an audit finds it
        if self.h.econ[s].as_ref().is_some_and(|e| e.took - self.ev.took_seen[s] > 0.0) && self.ev.theft[s] > PETTY {
            let chance = FIND * (self.ev.theft[s] - PETTY) * self.audit(s);
            if (hash64(self.seed ^ 0x7F1E ^ (s as u64) << 40 ^ self.month() as u64) % 10_000) as f32 / 10_000.0 < chance {
                return Some(Kind::Thief);
            }
        }
        // a general losing battle after battle
        if let Some(gs) = self.h.war.command[s].as_ref() {
            for (k, g) in gs.iter().enumerate() {
                if let Some(x) = &g.general {
                    if x.lost >= 6 && x.won * 2 < x.lost {
                        return Some(Kind::General(k));
                    }
                }
            }
        }
        // what the country's features and its debts bring
        let f = &self.spirits.facts[s];
        if f.debt > 15.0 {
            return Some(Kind::Lenders);
        }
        if f.inflation > 0.12 {
            return Some(Kind::Dear);
        }
        // the owners: a bank panic, their money fleeing
        if self.fiscal.panic[s] {
            return Some(Kind::Panic);
        }
        if self.fiscal.confidence[s] < 0.65 && self.fiscal.fled[s] > 0.5 * self.income(s) {
            return Some(Kind::Flight);
        }
        // the people of the country: a famous scientist asks for money, a hero out of favour speaks
        let c = self.country(s);
        let now = self.month() as i32;
        if self.h.labs[s].is_some() {
            let gifted = self.folk.living(c, strategy::world::folk::Calling::Scientist).into_iter().find(|&id| {
                let p = self.folk.get(id);
                p.fame >= 0.9 && p.age(now) < 70.0 && (hash64(self.seed ^ id as u64 ^ (self.month() as u64) << 7) % 30) == 0
            });
            if let Some(id) = gifted {
                let love = self.folk.get(id).love;
                let open = self.know.get(c).map_or(Vec::new(), |k| k.open(pack));
                let mine: Vec<usize> = open.iter().copied().filter(|&t| strategy::world::folk::loves(pack, t, love)).collect();
                if let Some(&t) = mine.first().or(open.first()) {
                    return Some(Kind::Genius(id, t));
                }
            }
        }
        if !self.h.at_war && st < 0.65 {
            let hero = self.folk.living(c, strategy::world::folk::Calling::General).into_iter().find(|&id| {
                let p = self.folk.get(id);
                p.fame >= 1.6 && p.post().is_none() && p.age(now) < 68.0
            });
            if let Some(id) = hero {
                return Some(Kind::Hero(id));
            }
        }
        let has = |id: &str| self.spirits.list[s].iter().any(|sp| sp.def().id == id && sp.level > 0.5);
        if !self.h.at_war && st > 0.5 && self.policy.running[s].is_empty() && (has("illiterate") || has("agrarian") || has("hunger")) {
            return Some(Kind::Reform);
        }
        None
    }

    /// How sharp side `s`'s audit is (about 0.5 … 1.5): a free press and a vote find a thief
    /// sooner; a custom of graft hides him.
    pub fn audit(&self, s: usize) -> f32 {
        let c = self.country(s);
        let l = |k: usize| self.polity.of.get(c).map_or(0.0, |p| p.laws[k] as f32 / 2.0);
        let graft = self.spirits.list[s].iter().any(|sp| sp.def().id == "graft" && sp.level > 0.5);
        (0.55 + 0.45 * l(strategy::world::polity::PRESS) + 0.35 * l(strategy::world::polity::VOICE)) * if graft { 0.6 } else { 1.0 }
    }

    /// The player answers his first waiting event (by its id) with choice `k`.
    pub fn answer(&mut self, id: u64, k: usize) {
        let Some(i) = self.ev.waiting.iter().position(|e| e.id == id) else { return };
        let ev = self.ev.waiting.remove(i);
        let k = k.min(ev.choices.len().saturating_sub(1));
        let what = ev.choices.get(k).map_or(String::new(), |c| c.0.clone());
        self.choose(&ev, k);
        self.news.push(Note::new(self.month(), &format!("{} — {}", ev.title, lower_first(&what)), false, "politics"));
        self.version += 1;
        self.stamp += 1;
    }

    /// The neighbour `s` begins its claim on us: the player hears of it.
    pub fn raise_claim_event(&mut self, s: usize) {
        let me = self.me;
        if s == me || self.auto.is_some() {
            return;
        }
        let now = self.month();
        self.ev.kinds[me].retain(|&(c, _)| c != Kind::Claim.code());
        self.ev.kinds[me].push((Kind::Claim.code(), now));
        let ev = self.event_of(me, Kind::Claim);
        self.ev.waiting.push(ev);
    }

}

/// "Принять мир" → "принять мир" (the first letter only: names stay as they are).
fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}
