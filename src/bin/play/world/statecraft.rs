//! Statecraft: a war needs a cause — a claim the state works up for months, which the world sees
//! (the tension rises); a peace holds for years (a truce); each government stands as firmly as its
//! people let it (stability: their discontent, the war's weight, what the government chose in hard
//! moments); the other country's government decides on war by the same rules as the player — its
//! goal is to win at the least cost: it claims when its army looks clearly stronger than the
//! neighbour's (as its staff sees it, sure of itself or doubting after a lost war), or when the
//! neighbour holds land the past took from it. Peace can be offered: the other side takes it when
//! it is not winning. Depots of supply are built on the map; what the state builds at home goes by
//! the player's list of trades.

use strategy::rng::{hash64, Rng};

use crate::game::Note;
use crate::Game;

/// Months a claim takes to work up; months a peace holds before a new claim may be made.
pub const JUSTIFY: u32 = 6;
pub const TRUCE: u32 = 36;
/// Months a depot of supply takes to build; what it costs, in months of the army's money.
pub const HUB_MONTHS: u32 = 2;
pub const HUB_COST: f32 = 0.5;
/// Below this stability a government does not dare to make a claim.
pub const SHAKY: f32 = 0.3;
/// What a province of a railway and of a road costs, in months of the army's money; how many
/// provinces of each are laid in a month.
pub const RAIL_COST: f32 = 0.12;
pub const ROAD_COST: f32 = 0.05;
pub const RAIL_PACE: usize = 1;
pub const ROAD_PACE: usize = 2;
/// The longest line laid at once (provinces).
pub const LINE_MOST: usize = 24;

/// A line of supply being laid (or laid): whose, a railway or a road, the provinces of the
/// theatre it runs through, how many of them are laid; whether it waits for money.
#[derive(Clone, Debug)]
pub struct Works {
    pub side: usize,
    pub rail: bool,
    pub path: Vec<usize>,
    pub done: usize,
    pub short: bool,
}

impl Works {
    pub fn finished(&self) -> bool {
        self.done >= self.path.len()
    }
}

/// A claim on the neighbour: when it was begun, when it is ready.
#[derive(Clone, Copy, Debug)]
pub struct Claim {
    pub ready: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Statecraft {
    pub claim: [Option<Claim>; 2],
    /// No claim before this month (a peace was made).
    pub truce_until: u32,
    /// The world's unease, 0 to 1: claims and wars raise it, quiet years let it down.
    pub tension: f32,
    /// Who began the war now going on.
    pub aggressor: Option<usize>,
    /// How firmly each government stands, 0 to 1, and what its choices in hard moments added or
    /// took (fading).
    pub stability: [f32; 2],
    pub mods: [f32; 2],
    /// Whether each side holds a grudge: the other holds land the past took from it.
    pub grudge: [bool; 2],
    /// Depots being built: side, province of the theatre, the month it is ready.
    pub building: Vec<(usize, usize, u32)>,
    /// The trades the player's state builds in (taken in turn, month by month).
    pub queue: Vec<usize>,
    /// The lines of supply laid and being laid (railways, roads), in the order begun.
    pub works: Vec<Works>,
}

impl Game {
    /// The month's statecraft, after the world's month: stability, the claims, the bot's decisions,
    /// depots finished, the state's building. What should stop the clock.
    pub fn statecraft(&mut self) -> Vec<String> {
        let mut alerts = Vec::new();
        let now = self.month();
        let me = self.me;
        // stability: the people's discontent, the war's weight, the choices made
        for s in 0..2 {
            let n = self.h.home(s);
            let weary = if self.h.at_war {
                let held = self.h.war.people()[s];
                (self.h.dead[s] / n.people().max(1e-6) * 30.0).min(0.3) + (1.0 - held) * 0.4
            } else {
                0.0
            };
            let st = &mut self.state;
            st.mods[s] *= 0.95;
            st.stability[s] = (1.0 - n.anger() / 3.5 - weary + st.mods[s]).clamp(0.02, 1.0);
        }
        // the world calms in peace, and is uneasy in war
        let st = &mut self.state;
        st.tension = if self.h.at_war { (st.tension + 0.02).min(1.0) } else { st.tension * 0.97 };
        // depots finished
        let mut done = Vec::new();
        self.state.building.retain(|&(s, q, ready)| {
            if ready <= now {
                done.push((s, q));
                false
            } else {
                true
            }
        });
        for (s, q) in done {
            if self.h.war.owner[q] == s && !self.h.war.hubs[s].contains(&q) {
                self.h.war.hubs[s].push(q);
                if s == me {
                    self.news.push(Note::new(now, "построен склад снабжения", false, "army"));
                }
            }
        }
        // the lines of supply: a province or two a month, each paid as it is laid, on land the
        // side still holds
        self.lay_works();
        // the state builds by the player's list, a trade a month in turn
        if self.adviser.is_none() && !self.state.queue.is_empty() {
            let k = self.state.queue[now as usize % self.state.queue.len()];
            self.h.home_mut(me).build_focus = Some(k);
        }
        // claims ready
        for s in 0..2 {
            if let Some(c) = self.state.claim[s] {
                if c.ready == now {
                    if s == me {
                        let line = "цель войны обоснована — можно объявить войну".to_string();
                        self.news.push(Note::new(now, &line, true, "politics"));
                        alerts.push(line);
                    }
                }
            }
        }
        // the governments that are bots decide on war (the other country's always; both, when bots
        // play the whole game)
        let bots: Vec<usize> = if self.auto.is_some() { vec![0, 1] } else { vec![1 - me] };
        for s in bots {
            if let Some(line) = self.bot_war(s) {
                alerts.push(line);
            }
        }
        // the bots build depots for their fronts in a war, and lay lines of supply to them
        if self.h.at_war && now % 6 == 0 {
            for s in 0..2 {
                if s != me || self.auto.is_some() {
                    self.bot_depot(s);
                }
            }
        }
        if self.h.at_war && now % 3 == 1 {
            for s in 0..2 {
                if s != me || self.auto.is_some() {
                    self.bot_line(s);
                }
            }
        }
        alerts
    }

    /// What the army of side `s` is worth against the other's, as its staff sees it (through fog).
    fn strength_seen(&self, s: usize) -> f32 {
        let worth = |x: usize| -> f32 { self.h.war.sides[x].army.count.iter().zip(&self.pack.units).map(|(n, u)| n * u.cost).sum() };
        let fog = self.h.war.fog[s];
        let mut r = Rng::new(hash64(self.seed ^ 0x5EE ^ (self.month() as u64) << 4 ^ s as u64));
        worth(s) / (worth(1 - s) * (fog * r.normal()).exp()).max(1e-6)
    }

    /// A bot's government over a month: whether to work up a claim, whether to go to war with the
    /// claim it has. Its goal is to win at the least cost: it claims when its army looks clearly
    /// stronger (a staff sure of itself dares sooner, one that lost a war later), or when the
    /// neighbour holds land the past took from it. The news, if any.
    fn bot_war(&mut self, s: usize) -> Option<String> {
        let now = self.month();
        let me = self.me;
        let o = 1 - s;
        if self.h.at_war || self.h.war.fallen.is_some() {
            return None;
        }
        // (a pact holds it back; the neighbour's allies weigh against its army)
        let (held, allies) = self.restrained(s);
        let ratio = self.strength_seen(s) / allies;
        let sure = 1.0 - self.h.war.sides[s].doubt;
        let grudge = self.state.grudge[s];
        // (its order: a junta or a chiefs' host dares sooner, a republic later; zealots against a
        // neighbour of another faith, revanchists, the party of power push it on)
        let lust = self.polity.war_lust(self.country(s), self.country(o));
        let wants = ratio * (0.8 + 0.4 * sure) * lust.sqrt() >= 1.6 || (grudge && ratio >= 1.15);
        match self.state.claim[s] {
            None => {
                if now < self.state.truce_until || self.state.stability[s] < SHAKY || !wants || held {
                    return None;
                }
                let mut r = Rng::new(hash64(self.seed ^ 0xC1A ^ (now as u64) << 2 ^ s as u64));
                if r.f32() >= 0.15 {
                    return None;
                }
                self.state.claim[s] = Some(Claim { ready: now + JUSTIFY });
                self.state.tension = (self.state.tension + 0.2).min(1.0);
                let name = self.h.home(s).name.clone();
                let line = if s == me {
                    "правительство обосновывает цель войны".to_string()
                } else {
                    format!("разведка: {name} готовит войну против нас")
                };
                self.news.push(Note::new(now, &line, o == me, "politics"));
                if o == me {
                    self.raise_claim_event(s);
                }
                None
            }
            Some(c) if now >= c.ready => {
                // ready: to war, if it still looks worth it; else the claim is dropped
                if ratio >= 1.2 || (grudge && ratio >= 1.0) {
                    self.go_to_war(s);
                    Some(format!("{} объявляет войну", self.h.home(s).name))
                } else {
                    self.state.claim[s] = None;
                    self.news.push(Note::new(now, &format!("{} {} от войны", self.h.home(s).name, strategy::world::names::agree(&self.h.home(s).name, "отказалась")), false, "politics"));
                    None
                }
            }
            Some(_) => None,
        }
    }

    /// Side `s` declares the war.
    pub fn go_to_war(&mut self, s: usize) {
        self.state.aggressor = Some(s);
        self.state.claim = [None, None];
        self.state.tension = (self.state.tension + 0.3).min(1.0);
        self.apply(&[("declare".to_string(), "1".to_string())]);
    }

    /// The player works up a claim on the neighbour (if a truce and his stability let him).
    pub fn claim(&mut self) -> bool {
        let me = self.me;
        let now = self.month();
        if self.h.at_war || self.state.claim[me].is_some() || now < self.state.truce_until || self.state.stability[me] < SHAKY {
            return false;
        }
        self.state.claim[me] = Some(Claim { ready: now + JUSTIFY });
        self.state.tension = (self.state.tension + 0.2).min(1.0);
        self.news.push(Note::new(now, "начато обоснование цели войны: мир это видит", true, "politics"));
        true
    }

    /// Whether side `s` (a bot) would take a peace as things stand: when it is not winning.
    pub fn takes_peace(&self, s: usize) -> bool {
        let held = self.h.war.people();
        // (or when its people demand peace)
        held[s] <= held[1 - s] + 0.02 || self.h.at_war && self.war_mood.support[s] < crate::country::war_mood::DEMAND_PEACE
    }

    /// A peace as things stand (each keeps what it holds), offered by side `from`; whether it was
    /// taken. The news is told.
    pub fn offer_peace(&mut self, from: usize) -> bool {
        let now = self.month();
        if !self.h.at_war {
            return false;
        }
        let other = 1 - from;
        // the player is asked, as by an enemy losing (the clock waits for his answer)
        if other == self.me && self.auto.is_none() {
            if !self.ev.waiting.iter().any(|e| e.kind == crate::country::events::Kind::PeaceOffer) {
                let ev = self.event_of(other, crate::country::events::Kind::PeaceOffer);
                self.news.push(Note::new(now, &ev.title, true, "politics"));
                self.ev.waiting.push(ev);
            }
            return false;
        }
        let taken = self.takes_peace(other);
        if !taken {
            self.news.push(Note::new(now, &format!("{} {} мир", self.h.home(other).name, strategy::world::names::agree(&self.h.home(other).name, "отвергла")), true, "politics"));
            return false;
        }
        let line = format!("мир заключён: {} и {} остаются при том, что держат", self.h.home(0).name, self.h.home(1).name);
        self.end_war();
        self.news.push(Note::new(now, &line, true, "politics"));
        true
    }

    /// The war is over (by a capitulation or a peace): the truce begins, the claims are dropped.
    pub fn after_war(&mut self) {
        self.state.truce_until = self.month() + TRUCE;
        self.state.claim = [None, None];
        self.state.aggressor = None;
    }

    /// The player builds a depot of supply in province `q` of the theatre (his own, reached from
    /// his capital), if the treasury pays for it.
    pub fn build_depot(&mut self, q: usize) -> bool {
        let me = self.me;
        let now = self.month();
        if q >= self.h.war.owner.len() || self.h.war.owner[q] != me || self.h.war.hubs[me].contains(&q) || self.state.building.iter().any(|b| b.0 == me && b.1 == q) {
            return false;
        }
        let n = self.h.home_mut(me);
        let cost = HUB_COST * n.army_money().max(0.01);
        if n.treasury < cost {
            self.news.push(Note::new(now, "на склад снабжения не хватает денег в казне", true, "army"));
            return false;
        }
        n.treasury -= cost;
        self.state.building.push((me, q, now + HUB_MONTHS));
        self.news.push(Note::new(now, "заложен склад снабжения", false, "army").with_tip(format!("стоит {cost:.2}, готов через {HUB_MONTHS} мес.")));
        true
    }

    /// Whether the age has railways (else only roads are laid).
    pub fn has_rails(&self) -> bool {
        self.pack.theme != "parchment"
    }

    /// The name of a line of supply in the age: a railway or a road.
    pub fn line_name(&self, rail: bool) -> &'static str {
        match (rail, self.pack.theme.as_str()) {
            (true, _) => "железная дорога",
            (false, "parchment") => "мощёный тракт",
            (false, "rust") => "расчищенная дорога",
            (false, _) => "шоссе",
        }
    }

    /// What a line through `n` provinces costs side `s`, and the months it takes.
    pub fn line_cost(&self, s: usize, rail: bool, n: usize) -> (f32, u32) {
        let per = if rail { RAIL_COST } else { ROAD_COST } * self.h.home(s).army_money().max(0.01);
        let pace = if rail { RAIL_PACE } else { ROAD_PACE };
        (per * n as f32, n.div_ceil(pace) as u32)
    }

    /// A line of supply begun by side `s` through `path` (provinces of the theatre, each next to
    /// the one before, all its own); whether it was begun.
    pub fn begin_line(&mut self, s: usize, rail: bool, path: Vec<usize>) -> bool {
        let w = &self.h.war;
        let n = w.owner.len();
        if (rail && !self.has_rails()) || path.len() < 2 || path.len() > LINE_MOST || path.iter().any(|&q| q >= n || w.owner[q] != s) || path.windows(2).any(|p| !w.th.nbr[p[0]].contains(&p[1])) {
            return false;
        }
        // (the provinces already on such a line or better need no laying)
        let g = |q: usize| w.grade.get(q).copied().unwrap_or(1.0);
        let want = if rail { crate::world::land::RAIL_GRADE } else { crate::world::land::ROAD_GRADE };
        if path.iter().all(|&q| g(q) <= want) {
            return false;
        }
        if s == self.me {
            let (cost, months) = self.line_cost(s, rail, path.len());
            let now = self.month();
            let name = self.line_name(rail);
            self.news.push(Note::new(now, &format!("начали прокладывать: {name}"), false, "army").with_tip(format!("провинций {}, стоит {cost:.2}, месяцев {months}", path.len())));
        }
        self.state.works.push(Works { side: s, rail, path, done: 0, short: false });
        true
    }

    /// A month of laying alone (for the self-checks).
    pub fn lay_month(&mut self) {
        self.lay_works();
    }

    /// The month's laying: each line under way lays its next province or two, if the treasury
    /// pays and the side still holds the land (else it waits).
    fn lay_works(&mut self) {
        let now = self.month();
        let me = self.me;
        let n = self.h.war.owner.len();
        for k in 0..self.state.works.len() {
            let wk = self.state.works[k].clone();
            if wk.finished() {
                continue;
            }
            let pace = if wk.rail { RAIL_PACE } else { ROAD_PACE };
            let per = if wk.rail { RAIL_COST } else { ROAD_COST } * self.h.home(wk.side).army_money().max(0.01);
            let grade = if wk.rail { crate::world::land::RAIL_GRADE } else { crate::world::land::ROAD_GRADE };
            let mut done = wk.done;
            let mut short = false;
            for _ in 0..pace {
                let Some(&q) = wk.path.get(done) else { break };
                if self.h.war.owner[q] != wk.side {
                    break;
                }
                let nat = self.h.home_mut(wk.side);
                if nat.treasury < per {
                    short = true;
                    break;
                }
                nat.treasury -= per;
                let war = &mut self.h.war;
                if war.grade.len() != n {
                    war.grade = vec![1.0; n];
                }
                // (the spoiled control: a line laid that the supply never hears of)
                if !crate::spoiled() {
                    war.grade[q] = war.grade[q].min(grade);
                }
                done += 1;
            }
            let name = self.line_name(wk.rail);
            let w = &mut self.state.works[k];
            let warn = short && !w.short && w.side == me;
            w.short = short;
            w.done = done;
            let laid = w.finished() && w.side == me;
            let len = w.path.len();
            if warn {
                self.news.push(Note::new(now, "на прокладку линии не хватает денег в казне", true, "army"));
            }
            if laid {
                self.news.push(Note::new(now, &format!("проложена {name}"), false, "army").with_tip(format!("провинций {len}")));
            }
        }
    }

    /// A bot lays a line of supply towards its front province worst supplied: from the nearest
    /// province on a line, a depot or the capital, through its own land (one line at a time).
    pub fn bot_line(&mut self, s: usize) {
        if self.state.works.iter().any(|w| w.side == s && !w.finished()) {
            return;
        }
        let w = &self.h.war;
        let sup = w.supply(s);
        let Some(&q) = w.front(s).iter().filter(|&&p| sup[p] < 0.7).min_by(|&&a, &&b| sup[a].total_cmp(&sup[b])) else { return };
        let d = w.distance_from(q);
        let g = |p: usize| w.grade.get(p).copied().unwrap_or(1.0);
        let starts: Vec<usize> = (0..w.owner.len()).filter(|&p| w.owner[p] == s && p != q && (g(p) < 1.0 || w.hubs[s].contains(&p) || p == w.th.capital[s])).collect();
        let Some(&from) = starts.iter().min_by_key(|&&p| d[p]) else { return };
        let Some(path) = crate::world::land::plan(w, s, from, q) else { return };
        if path.len() < 3 || path.len() > 16 {
            return;
        }
        let rail = self.has_rails();
        let (cost, _) = self.line_cost(s, rail, path.len());
        if self.h.home(s).treasury >= cost * 0.5 {
            self.begin_line(s, rail, path);
        }
    }

    /// A bot's depot: at its front province worst supplied, if none is near it.
    fn bot_depot(&mut self, s: usize) {
        let w = &self.h.war;
        let sup = w.supply(s);
        let front = w.front(s);
        let Some(&q) = front.iter().filter(|&&p| sup[p] < 0.7).min_by(|&&a, &&b| sup[a].total_cmp(&sup[b])) else { return };
        let d = w.distance_from(q);
        if w.hubs[s].iter().any(|&h| d[h] <= 2) || self.state.building.iter().any(|b| b.0 == s) {
            return;
        }
        let n = self.h.home_mut(s);
        let cost = HUB_COST * n.army_money().max(0.01);
        if n.treasury >= cost {
            n.treasury -= cost;
            let now = self.month();
            self.state.building.push((s, q, now + HUB_MONTHS));
        }
    }

    /// Stability in a word.
    pub fn stability_word(&self, s: usize) -> &'static str {
        match self.state.stability[s] {
            x if x >= 0.75 => "прочна",
            x if x >= 0.5 => "устойчива",
            x if x >= SHAKY => "шатается",
            _ => "на грани",
        }
    }

    /// Tension in a word.
    pub fn tension_word(&self) -> &'static str {
        match self.state.tension {
            x if x < 0.15 => "спокойно",
            x if x < 0.4 => "неспокойно",
            x if x < 0.7 => "тревожно",
            _ => "на грани войны",
        }
    }
}

impl Game {

}
