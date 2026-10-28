//! What a people thinks of its war, and how a state pays for it beyond the tax.
//!
//! Support for the war (0 … 1) is not written for a country: it grows from what the war is and what
//! it does. A people attacked rallies; one whose government started the war follows it as far as
//! its hawks and its grudge carry it. Then the war wears it down: its dead (against how many it is),
//! its land under the enemy (a little of it rallies, much of it breaks), its hunger and anger, the
//! prices, the war tax, the months; victories lift it. A government that holds the press keeps the
//! bad news quieter. Low support costs: strikes at the arms works, soldiers who desert (the divisions
//! get their organisation back slower), a government that loses its standing and its legitimacy, and
//! a people that demands peace — a bot's government takes it then, and a player's is pressed.
//!
//! War loans: a state at war sells bonds to those who save — the owners, the learned, the workers
//! with something put by: their money comes to the treasury now (they spend and build less), the
//! state owes it them with interest, and pays it back in peace. A patriotic loan is asked loudly; a
//! forced one is taken (and resented).

use strategy::rng::hash64;
use strategy::world::polity;

use crate::country::fiscal::Choice;
use crate::country::policy::role;
use crate::game::Note;
use crate::Game;

/// The war loans: none, voluntary, patriotic, forced.
pub static BONDS: [Choice; 4] = [
    Choice { name: "Без займов", icon: "stop", about: "Война живёт на налоги, займы за границей и печатный станок." },
    Choice { name: "Добровольный заём", icon: "scroll", about: "Казна продаёт облигации тем, у кого есть сбережения: малая часть их денег идёт на войну, под проценты." },
    Choice { name: "Патриотический заём", icon: "flag", about: "Газеты, плакаты, церковь: «всё для фронта». Сбережений в казну втрое больше, люди меньше тратят и строят." },
    Choice { name: "Принудительный заём", icon: "stamper", about: "Облигации раздают вместо денег: казна берёт десятую часть сбережений в месяц. Собственники злы, доверие падает." },
];
/// The share of their savings each loan takes a month from those who save.
const BOND_TAKE: [f32; 4] = [0.0, 0.01, 0.03, 0.08];
/// The interest a month on what the state owes for its loans; the share of it paid back a month in
/// peace (when the treasury can).
const BOND_RATE: f32 = 0.004;
const BOND_BACK: f32 = 1.0 / 60.0;
/// Support at which strikes begin, at which soldiers desert, at which the people demand peace.
pub const STRIKES: f32 = 0.4;
pub const DESERT: f32 = 0.3;
pub const DEMAND_PEACE: f32 = 0.22;

/// The game's side of the war's mood and loans.
#[derive(Clone, Default)]
pub struct WarMood {
    /// Each side's support for the war (0 … 1), what moves it (for the panel), its months of war.
    pub support: [f32; 2],
    pub parts: [Vec<(String, f32)>; 2],
    pub history: [Vec<(u32, f32)>; 2],
    /// The share of the arms works on strike last month.
    pub strike: [f32; 2],
    /// The war loan each side keeps (`BONDS`; none chosen: the custom — none in peace, a voluntary
    /// one at war), what the state owes for them to each group, what it took and paid last month.
    pub bonds: [Option<usize>; 2],
    pub owed: [Vec<f32>; 2],
    pub took: [f32; 2],
    pub paid: [f32; 2],
    /// The price level and tax when the war began (what it changed is counted against them), months
    /// the people have demanded peace.
    pub level0: [f32; 2],
    pub tax0: [f32; 2],
    pub demanding: [u32; 2],
}

impl Game {
    /// The war loan side `s` keeps now.
    pub fn bond_level(&self, s: usize) -> usize {
        if let Some(k) = self.war_mood.bonds[s] {
            return k.min(BONDS.len() - 1);
        }
        if !self.h.at_war {
            return 0;
        }
        // the custom: a voluntary loan; a bot's government in want of money asks a patriotic one
        let bot = s != self.me || self.auto.is_some();
        let n = self.h.home(s);
        let low = n.treasury < 6.0 * self.income(s).max(1e-6);
        if bot && low && self.war_mood.support[s] > 0.45 {
            2
        } else {
            1
        }
    }

    /// The war's mood of side `s`: where support for it tends this month, thing by thing.
    fn war_mood_parts(&self, s: usize) -> Vec<(String, f32)> {
        let mut v: Vec<(String, f32)> = Vec::new();
        let c = self.country(s);
        let p = &self.polity.of[c];
        let aggressor = self.state.aggressor == Some(s);
        let hawks = p.moves.get(polity::HAWKS).copied().unwrap_or(0.0) + p.moves.get(polity::REVANCHE).copied().unwrap_or(0.0);
        if aggressor {
            v.push(("мы начали войну".into(), 0.42));
            v.push(("ястребы и реваншисты".into(), (0.3 * hawks).min(0.25)));
            if self.state.grudge[s] {
                v.push(("вернуть отнятое".into(), 0.12));
            }
        } else {
            v.push(("на нас напали".into(), 0.72));
        }
        let n = self.h.home(s);
        let people = n.people().max(1e-6);
        // (bad news weighs less where the government holds the press and the people)
        let quiet = 1.0 - 0.45 * p.regime.repression().clamp(0.0, 1.0);
        let dead = self.h.dead[s] / people;
        v.push(("погибшие".into(), -(dead * 25.0).min(0.5) * quiet));
        let held = self.h.war.people();
        let lost = (1.0 - held[s]).max(0.0);
        if lost > 0.0 {
            // (a little of the land under the enemy rallies a people; much of it breaks it)
            let x = if lost < 0.1 { 0.6 * lost } else { 0.06 - 0.9 * (lost - 0.1) };
            v.push(("враг на нашей земле".into(), x.max(-0.45) * quiet));
        }
        let taken = (1.0 - held[1 - s]).max(0.0);
        if taken > 0.0 {
            v.push(("наши победы".into(), (0.8 * taken).min(0.3)));
        }
        v.push(("недовольство и нужда".into(), -(0.6 * n.anger()).min(0.35) * quiet));
        let level0 = self.war_mood.level0[s].max(1e-6);
        let infl = (n.level / level0 - 1.0).max(0.0);
        if infl > 0.0 {
            v.push(("дороговизна".into(), -(0.6 * infl).min(0.25) * quiet));
        }
        let tax = (n.tax - self.war_mood.tax0[s]).max(0.0);
        if tax > 0.0 {
            v.push(("военный налог".into(), -(0.8 * tax).min(0.2)));
        }
        if self.bond_level(s) == 3 {
            v.push(("принудительный заём".into(), -0.06));
        }
        let months = self.h.war.month as f32;
        v.push(("долгая война".into(), -(0.004 * months).min(0.25) * quiet));
        if self.navy.sunk_trade[s] > 0.0 && self.navy.all_trade[s] > 0.0 {
            v.push(("враг топит наши суда".into(), -(0.3 * self.navy.sunk_trade[s] / self.navy.all_trade[s]).min(0.1)));
        }
        v.retain(|x| x.1.abs() >= 0.005);
        v
    }

    /// Before the world's month: what low support for the war costs — strikes at the arms works,
    /// desertion; the war loans taken and their interest paid (in peace, paid back).
    pub fn war_mood_apply(&mut self) {
        let pack = self.pack.clone();
        let ng = pack.groups.len();
        for s in 0..2 {
            // (the war loans)
            if self.war_mood.owed[s].len() != ng {
                self.war_mood.owed[s] = vec![0.0; ng];
            }
            let k = self.bond_level(s);
            let take = BOND_TAKE[k];
            let at_war = self.h.at_war;
            let n = self.h.home_mut(s);
            let mut took = 0.0f32;
            if take > 0.0 && at_war {
                for g in 0..ng {
                    // (those who save: the owners and the learned most, the workers some, the peasants
                    // little)
                    let k = match role(&pack, g) {
                        3 => 1.0,
                        2 => 0.8,
                        1 => 0.4,
                        _ => 0.15,
                    };
                    let x = n.money[g].max(0.0) * take * k;
                    n.money[g] -= x;
                    n.treasury += x;
                    self.war_mood.owed[s][g] += x;
                    took += x;
                }
            }
            // interest; in peace, the debt paid back as far as the treasury bears it
            let mut paid = 0.0f32;
            let owed: f32 = self.war_mood.owed[s].iter().sum();
            if owed > 0.0 {
                let interest = owed * BOND_RATE;
                let back = if at_war { 0.0 } else { owed * BOND_BACK };
                let pay = (interest + back).min(n.treasury.max(0.0) * 0.2);
                for g in 0..ng {
                    let share = self.war_mood.owed[s][g] / owed;
                    n.money[g] += pay * share;
                    let principal = (pay - interest).max(0.0) * share;
                    self.war_mood.owed[s][g] = (self.war_mood.owed[s][g] - principal).max(0.0);
                }
                n.treasury -= pay;
                paid = pay;
            }
            self.war_mood.took[s] = took;
            self.war_mood.paid[s] = paid;
            // strikes and desertion
            if !self.h.at_war {
                self.war_mood.strike[s] = 0.0;
                continue;
            }
            let sup = self.war_mood.support[s];
            let strike = ((STRIKES - sup) / STRIKES).clamp(0.0, 1.0) * 0.35;
            self.war_mood.strike[s] = strike;
            if strike > 0.0 {
                let n = self.h.home_mut(s);
                for (k, sec) in pack.sectors.iter().enumerate() {
                    if sec.outputs.iter().any(|o| pack.goods[o.0].military) {
                        if let Some(x) = n.shock.get_mut(k) {
                            *x *= 1.0 - strike;
                        }
                    }
                }
            }
            if sup < DESERT {
                let k = 1.0 - (DESERT - sup) / DESERT * 0.4;
                if let Some(c) = self.h.war.corps.as_mut() {
                    c.mods[s].org_back *= k;
                }
            }
        }
    }

    /// After the world's month: each side's support for the war moves towards where its war pulls it;
    /// what low support does to the government; a people demanding peace.
    pub fn war_mood_month(&mut self) -> Vec<String> {
        let mut alerts = Vec::new();
        let now = self.month();
        for s in 0..2 {
            if !self.h.at_war {
                self.war_mood.support[s] = 0.0;
                self.war_mood.demanding[s] = 0;
                self.war_mood.level0[s] = self.h.home(s).level;
                self.war_mood.tax0[s] = self.h.home(s).tax;
                self.war_mood.parts[s].clear();
                continue;
            }
            let parts = self.war_mood_parts(s);
            let target: f32 = parts.iter().map(|x| x.1).sum::<f32>().clamp(0.0, 1.0);
            let was = self.war_mood.support[s];
            let first = self.h.war.month <= 1 && was == 0.0;
            let now_s = if first { target } else { was + 0.15 * (target - was) };
            self.war_mood.support[s] = now_s;
            self.war_mood.parts[s] = parts;
            self.war_mood.history[s].push((now, now_s));
            if self.war_mood.history[s].len() > 240 {
                self.war_mood.history[s].remove(0);
            }
            // a war the people do not want weighs on the government
            let c = self.country(s);
            if now_s < STRIKES {
                let k = STRIKES - now_s;
                self.state.stability[s] = (self.state.stability[s] - 0.03 * k).max(0.0);
                let p = &mut self.polity.of[c];
                p.legit = (p.legit - 0.02 * k).max(0.0);
                p.unrest = (p.unrest + 0.03 * k).min(1.0);
            }
            let name = self.h.home(s).name.clone();
            // the news of it crossing a line
            let crossed = |line: f32| was >= line && now_s < line;
            if crossed(STRIKES) {
                let text = format!("{name}: на военных заводах забастовки — люди устали от войны");
                if s == self.me {
                    alerts.push(text.clone());
                }
                self.news.push(Note::new(now, &text, s == self.me, "politics"));
            }
            if crossed(DESERT) {
                let text = format!("{name}: солдаты бегут из окопов");
                self.news.push(Note::new(now, &text, s == self.me, "politics"));
            }
            if now_s < DEMAND_PEACE {
                self.war_mood.demanding[s] += 1;
            } else {
                self.war_mood.demanding[s] = 0;
            }
            if self.war_mood.demanding[s] == 1 {
                let text = format!("{name}: народ требует мира");
                if s == self.me {
                    alerts.push(text.clone());
                }
                self.news.push(Note::new(now, &text, true, "politics"));
            }
        }
        // a bot's government whose people demand peace offers it (every few months)
        if self.h.at_war {
            for s in 0..2 {
                let bot = s != self.me || self.auto.is_some();
                if bot && self.war_mood.demanding[s] >= 3 && (now + hash64(self.seed ^ s as u64) as u32) % 4 == 0 && self.offer_peace(s) {
                    break;
                }
            }
        }
        alerts
    }

    /// The war's mood of side `s` as a mood of its forces of society (for `mood_parts`): what each
    /// thinks of the war going as it goes.
    pub fn war_mood_of_bloc(&self, s: usize, b: usize) -> f32 {
        if !self.h.at_war {
            return 0.0;
        }
        let x = self.war_mood.support[s] - 0.5;
        match b {
            polity::OFFICERS => 0.3 * x + 0.05,
            polity::MERCHANTS | polity::CAPITAL => 0.2 * x,
            polity::WORKERS | polity::PEASANTS => 0.35 * x,
            _ => 0.25 * x,
        }
    }
}
