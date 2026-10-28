//! The world's relations in the game: what the countries think of each other and the pacts they
//! keep (`strategy::world::diplomacy`, as four hundred years of the past left them) go on month by month.
//! The wars of the world elsewhere are fought by no front (their power and luck decide) and end in
//! truces and tribute; every year opinions follow their reasons, pacts lapse and are made; the
//! others' offers to the player come to him as events. Pacts do things: a trade agreement takes
//! the duty off the goods between the two; tribute goes from one treasury to another; allies and
//! guarantors called to a war send money every month and close their markets to the enemy (and
//! the war goes into their memory); a pact of non-aggression holds the other government back
//! from a claim, and breaking it makes the breaker faithless in the eyes of all. The player's
//! government proposes and breaks pacts, sends envoys, answers allies who call it, leaves a war.

use strategy::rng::{hash64, Rng};
use strategy::world::diplomacy::{Kind, Scene, WHY};
use strategy::world::polity;

use crate::game::Note;
use crate::Game;

/// What an ally or guarantor in a war sends the one it fights for, a month (a share of what its
/// state spends); the tribute of a tributary, a month.
pub const HELP: f32 = 0.06;
pub const TRIBUTE: f32 = 0.04;
/// What an envoy costs (months of the state's spending), how much warmer the other government grows
/// for it, the warmth at most, how it fades a month, months between two envoys to one country.
pub const ENVOY_COST: f32 = 0.25;
pub const ENVOY_WARMTH: f32 = 0.12;
pub const WARMTH_MOST: f32 = 0.4;
pub const WARMTH_FADE: f32 = 0.97;
pub const ENVOY_PAUSE: u32 = 6;
/// Chance a year that a war begins somewhere in the world (among countries other than the two of
/// the game).
const WORLD_WAR_A_YEAR: f32 = 0.35;
/// Months a refused proposal is not made again.
const REFUSED_PAUSE: u32 = 12;

/// The game's side of the world's relations.
#[derive(Clone, Debug, Default)]
pub struct Relations {
    /// The countries whose land touches each other's (map countries).
    pub adj: Vec<Vec<usize>>,
    /// Each region's provinces.
    pub region_provs: Vec<Vec<usize>>,
    /// Who helps each side of the game's war, and what it sent last month.
    pub helpers: [Vec<(usize, f32)>; 2],
    /// Ways between countries closed by a war (to open again at its peace).
    pub cuts: Vec<(usize, usize)>,
    /// When the player last sent an envoy to each country.
    pub envoys: Vec<(usize, u32)>,
    /// The player's proposals refused: country, kind, month.
    pub refused: Vec<(usize, Kind, u32)>,
    /// What each country paid and got by its pacts and wars last month (tribute, help).
    pub paid: Vec<f32>,
    pub got: Vec<f32>,
    /// Allies of the player that called him to their war and wait for his answer; his countries'
    /// share in the wars of the world (the war's principal on his side, what he sends a month).
    pub calls: Vec<usize>,
}

/// The world's relations do nothing (`NODIPLO=1`: a control — no duties of the world, no trade
/// agreements, no tribute, no allies in war, no wars of the world).
pub fn off() -> bool {
    std::env::var("NODIPLO").is_ok_and(|v| v == "1")
}

/// The kind of pact from its number in `Kind::ALL`, and back.
pub fn kind_of(k: usize) -> Option<Kind> {
    Kind::ALL.get(k).copied()
}

pub fn code_of(k: Kind) -> usize {
    Kind::ALL.iter().position(|x| *x == k).unwrap_or(0)
}

/// The duty a country of the world (not one of the game's two: theirs the player and the bot set)
/// lays on a good, by its government: crowns, generals and parties keep their markets, rich men's
/// republics open them.
fn duty_of(pack: &strategy::pack::Pack, r: polity::Regime, k: usize) -> f32 {
    let g = &pack.goods[k];
    if !g.tradable {
        return 0.0;
    }
    let base = match r {
        polity::Regime::Patricians => 0.02,
        polity::Regime::Republic | polity::Regime::Constitutional => 0.04,
        _ => 0.08,
    };
    if pack.ration(k) {
        base * 0.4
    } else {
        base
    }
}

impl Game {
    /// The country of map province region `st` now (the theatre's owner for the two of the game).
    pub fn region_owner(&self, st: usize) -> Option<usize> {
        let p = *self.rel.region_provs.get(st)?.first()?;
        let war = &self.h.war;
        match war.th.at.get(p) {
            Some(&q) if q != strategy::military::front::NO => Some(war.th.countries[war.owner[q as usize]]),
            _ => (self.country_of[p] != u32::MAX).then_some(self.country_of[p] as usize),
        }
    }

    /// How much of country `a`'s lost land `b` holds now (0 … 1, fading over the years).
    pub fn holds_of(&self, a: usize, b: usize) -> f32 {
        let years = self.month() as f32 / 12.0;
        self.past_lost.get(a).map_or(0.0, |l| {
            l.iter().filter(|&&(st, _)| self.region_owner(st) == Some(b)).map(|&(_, y)| (1.0 - (y as f32 + years) / 150.0).max(0.0) * 0.5).sum::<f32>().min(1.0)
        })
    }

    /// The world's relations as the game begins: who neighbours whom; the player's country is his
    /// own in diplomacy; a war of the past going on between the game's two countries is ended on
    /// its eve by a truce (the game's war begins by its own claim).
    pub fn relations_start(&mut self) {
        let n = self.countries.len();
        let mut adj = vec![Vec::new(); n];
        for (p, prov) in self.world.provinces.iter().enumerate() {
            let a = self.country_of[p];
            if a == u32::MAX {
                continue;
            }
            for &q in &prov.neighbors {
                let b = self.country_of[q as usize];
                if b != u32::MAX && b != a && !adj[a as usize].contains(&(b as usize)) {
                    adj[a as usize].push(b as usize);
                }
            }
        }
        for a in &mut adj {
            a.sort();
        }
        self.rel.adj = adj;
        let regions = self.prov_state.iter().filter(|&&s| s != u32::MAX).map(|&s| s as usize + 1).max().unwrap_or(0);
        let mut rp = vec![Vec::new(); regions];
        for (p, &s) in self.prov_state.iter().enumerate() {
            if s != u32::MAX {
                rp[s as usize].push(p);
            }
        }
        self.rel.region_provs = rp;
        self.rel.paid = vec![0.0; n];
        self.rel.got = vec![0.0; n];
        let [a, b] = [self.country(0), self.country(1)];
        self.diplo.player = Some(self.country(self.me));
        self.diplo.offers.clear();
        let names: Vec<String> = self.countries.iter().map(|c| c.0.clone()).collect();
        while let Some(w) = self.diplo.wars.iter().position(|w| matches!((w.side_of(a), w.side_of(b)), (Some(x), Some(y)) if x != y)) {
            let war = self.diplo.end(w, None, false, 0, &names);
            self.diplo.chronicle.push((0, a, format!("Накануне: {} и {} заключили перемирие ({})", names[war.sides[0][0]], names[war.sides[1][0]], war.cause)));
        }
        self.relations_economy();
    }

    /// What the pacts do to the economy of the world: duty-free ways between partners; the duties
    /// of the world's governments (the two of the game set their own).
    pub fn relations_economy(&mut self) {
        if off() {
            return;
        }
        let n = self.countries.len().min(self.h.world.nations.len());
        let mut free = vec![vec![false; n]; n];
        for p in self.diplo.pacts.iter().filter(|p| p.kind == Kind::Trade) {
            if p.a < n && p.b < n {
                free[p.a][p.b] = true;
                free[p.b][p.a] = true;
            }
        }
        self.h.world.free = free;
        let pair = [self.country(0), self.country(1)];
        let nk = self.pack.goods.len();
        for c in 0..n {
            if pair.contains(&c) {
                continue;
            }
            let r = self.polity.of.get(c).map_or(polity::Regime::Autocracy, |p| p.regime);
            let duty: Vec<f32> = (0..nk).map(|k| duty_of(&self.pack, r, k)).collect();
            self.h.world.nations[c].duty = duty;
        }
    }

    /// The world's relations this year, as the game's world stands.
    fn relations_year(&mut self) {
        let n = self.countries.len();
        let now = self.month() as i32;
        let names: Vec<String> = self.countries.iter().map(|c| c.0.clone()).collect();
        let holds: Vec<f32> = (0..n * n).map(|i| if i / n == i % n { 0.0 } else { self.holds_of(i / n, i % n) }).collect();
        let polity = &self.polity;
        let apart = |a: usize, b: usize| polity.distance(polity.of[a].state_faith, polity.of[b].state_faith);
        let hold = |a: usize, b: usize| holds[a * n + b];
        let sc = Scene {
            names: &names,
            live: (0..n).map(|c| c < self.h.world.nations.len() && !self.rel.adj[c].is_empty()).collect(),
            power: (0..n).map(|c| self.h.world.nations.get(c).map_or(1e-6, |x| x.gdp.max(1e-6))).collect(),
            neighbours: self.rel.adj.clone(),
            regime: (0..n).map(|c| polity.of[c].regime).collect(),
            house: (0..n).map(|c| polity.of[c].house.clone()).collect(),
            apart: &apart,
            holds: &hold,
            trade: (0..n).map(|c| polity.of[c].clout[polity::MERCHANTS] + polity.of[c].clout[polity::CAPITAL]).collect(),
            zeal: (0..n).map(|c| polity.of[c].moves[polity::ZEALOTS]).collect(),
            hawk: (0..n).map(|c| self.folk.hawk(c)).collect(),
        };
        let mut rng = Rng::new(hash64(self.seed ^ 0xD1A ^ (now as u64) << 12));
        let before = self.diplo.chronicle.len();
        self.diplo.year(&sc, now, 1.0, &mut rng);
        // a war somewhere in the world (the two of the game do not begin one there)
        let pair = [self.country(0), self.country(1)];
        if rng.f32() < WORLD_WAR_A_YEAR {
            let mut pairs: Vec<(usize, usize, f32)> = Vec::new();
            for a in 0..n {
                if pair.contains(&a) || !sc.live[a] || self.diplo.war_of(a).is_some() {
                    continue;
                }
                let bold = (sc.hawk[a].max(0.0) + polity.of[a].moves[polity::HAWKS]).min(1.0);
                for &b in &self.rel.adj[a] {
                    if pair.contains(&b) || self.diplo.restraint(a, b, bold).is_some() {
                        continue;
                    }
                    let [att, def] = self.diplo.coalition(a, b);
                    let might = |side: &[usize]| side.iter().enumerate().map(|(i, &c)| sc.power[c] * if i == 0 { 1.0 } else { 0.5 }).sum::<f32>().max(1e-6);
                    let power = might(&att) / might(&def) * (0.45 * rng.normal()).exp();
                    let grudge = hold(a, b) > 0.0;
                    let lust = polity.war_lust(a, b) * (1.0 + 1.5 * (-self.diplo.of(a, b)).max(0.0));
                    if power * lust.sqrt() < 1.2 && !grudge {
                        continue;
                    }
                    pairs.push((a, b, power.min(4.0).powi(2) * if grudge { 4.0 } else { 1.0 } * lust));
                }
            }
            let all: f32 = pairs.iter().map(|p| p.2).sum();
            if all > 0.0 {
                let mut pick = rng.f32() * all;
                let (a, b) = pairs.iter().find(|p| {
                    pick -= p.2;
                    pick <= 0.0
                }).map_or((pairs[0].0, pairs[0].1), |p| (p.0, p.1));
                let cause = if hold(a, b) > 0.0 { "война за утраченные земли" } else if apart(a, b) > 0.6 && polity.of[a].moves[polity::ZEALOTS] > 0.3 { "священная война" } else { "пограничная война" };
                let mut sides = self.diplo.coalition(a, b);
                // (the two of the game answer for themselves: the bot as the others do, the
                // player by his own choice)
                let me_c = self.country(self.me);
                let mine = sides.iter().position(|s| s.contains(&me_c));
                for s in &mut sides {
                    s.retain(|&c| c != me_c);
                }
                let w = self.diplo.begin(sides, now, cause, &names, &mut rng);
                if let Some(s) = mine {
                    let head = self.diplo.wars[w].sides[s][0];
                    if !self.rel.calls.contains(&head) {
                        self.rel.calls.push(head);
                    }
                }
                for s in 0..2 {
                    let enemy = self.diplo.wars[w].sides[1 - s][0];
                    for &x in self.diplo.wars[w].sides[s].clone().iter().filter(|x| pair.contains(x)) {
                        self.cut_way(x, enemy);
                    }
                }
            }
        }
        // the world's news of the year
        for (m, c, line) in self.diplo.chronicle[before..].to_vec() {
            let ours = c == self.country(self.me) || line.contains(&names[self.country(self.me)]) || line.contains(&names[self.country(1 - self.me)]);
            self.news.push(Note::new(m.max(0) as u32, &format!("мир: {line}"), ours, "politics"));
        }
        self.relations_economy();
    }

    /// Goods stop going between `a` and `b` (a war between them), till the peace.
    fn cut_way(&mut self, a: usize, b: usize) {
        if a < self.h.world.nations.len() && b < self.h.world.nations.len() && !self.rel.cuts.contains(&(a, b)) {
            self.h.world.cut(a, b);
            self.rel.cuts.push((a, b));
        }
    }

    /// The ways a war closed open again, all but those between enemies still at war.
    fn reopen_ways(&mut self) {
        let pack = self.pack.clone();
        let war = self.h.at_war.then(|| [self.country(0), self.country(1)]);
        let mut keep = Vec::new();
        for (a, b) in std::mem::take(&mut self.rel.cuts) {
            let pair_war = war.is_some_and(|[x, y]| {
                let side = |c: usize| self.rel.helpers.iter().position(|h| h.iter().any(|z| z.0 == c)).or(if c == x { Some(0) } else if c == y { Some(1) } else { None });
                matches!((side(a), side(b)), (Some(p), Some(q)) if p != q)
            });
            if self.diplo.enemies(a, b) || pair_war {
                keep.push((a, b));
            } else {
                self.h.world.reopen(&pack, a, b);
            }
        }
        self.rel.cuts = keep;
    }

    /// The month of the world's relations: the wars elsewhere, tribute and help paid, envoys'
    /// warmth fading, the year's turn; the player asked to answer allies and offers. What should
    /// stop the clock.
    pub fn relations_month(&mut self) -> Vec<String> {
        let mut alerts = Vec::new();
        if off() {
            return alerts;
        }
        let now = self.month();
        let n = self.countries.len().min(self.h.world.nations.len());
        if self.rel.paid.len() != n {
            self.rel.paid = vec![0.0; n];
            self.rel.got = vec![0.0; n];
        }
        self.rel.paid.iter_mut().for_each(|x| *x = 0.0);
        self.rel.got.iter_mut().for_each(|x| *x = 0.0);
        let names: Vec<String> = self.countries.iter().map(|c| c.0.clone()).collect();
        let mut rng = Rng::new(hash64(self.seed ^ 0xD1B ^ (now as u64) << 12));
        // the wars of the world, fought by no front
        let mut w = 0;
        while w < self.diplo.wars.len() {
            let nations = &self.h.world.nations;
            let power = |c: usize| nations.get(c).map_or(1e-6, |x| x.gdp.max(1e-6).powf(0.75));
            match self.diplo.war_month(w, &power, &mut rng) {
                Some((win, harsh)) => {
                    let war = self.diplo.end(w, win, harsh, now as i32, &names);
                    let how = match win {
                        Some(s) => format!("победила {}", names[war.sides[s][0]]),
                        None => "без победителя".to_string(),
                    };
                    let mine = war.side_of(self.country(self.me)).is_some();
                    self.news.push(Note::new(now, &format!("мир: окончена война {} против {} — {how}", names[war.sides[0][0]], names[war.sides[1][0]]), mine, "politics"));
                    self.reopen_ways();
                }
                None => w += 1,
            }
        }
        // what the countries of the game send to the wars of the world they take part in
        let pair = [self.country(0), self.country(1)];
        for s in 0..2 {
            let c = pair[s];
            if let Some(w) = self.diplo.war_of(c) {
                let side = self.diplo.wars[w].side_of(c).unwrap_or(0);
                let head = self.diplo.wars[w].sides[side][0];
                if head != c && head < n {
                    let amt = HELP * self.h.world.nations[c].state_budget();
                    self.transfer(c, head, amt);
                }
            }
        }
        // tribute
        for p in self.diplo.pacts.clone().iter().filter(|p| p.kind == Kind::Tribute) {
            if p.a < n && p.b < n {
                let amt = TRIBUTE * self.h.world.nations[p.b].state_budget();
                self.transfer(p.b, p.a, amt);
            }
        }
        // help in the game's war
        if self.h.at_war {
            for s in 0..2 {
                let to = pair[s];
                let mut sent = Vec::new();
                for &(x, _) in &self.rel.helpers[s].clone() {
                    let amt = HELP * self.h.world.nations[x].state_budget();
                    self.transfer(x, to, amt);
                    sent.push((x, amt));
                }
                self.rel.helpers[s] = sent;
            }
        }
        // envoys' warmth fades
        for x in &mut self.diplo.warmth {
            *x *= WARMTH_FADE;
        }
        if now % 12 == 0 && now > 0 {
            self.relations_year();
        }
        // the player's allies call him; the others' offers
        if self.auto.is_none() {
            for head in std::mem::take(&mut self.rel.calls) {
                if self.diplo.war_of(head).is_some() {
                    let ev = self.event_of(self.me, crate::country::events::Kind::AllyCalls(head));
                    alerts.push(ev.title.clone());
                    self.news.push(Note::new(now, &ev.title, true, "politics"));
                    self.ev.waiting.push(ev);
                }
            }
            if self.ev.waiting.is_empty() {
                if let Some((k, from)) = self.diplo.offers.first().copied() {
                    self.diplo.offers.remove(0);
                    if !self.diplo.has(k, from, self.country(self.me)) {
                        let ev = self.event_of(self.me, crate::country::events::Kind::Offer(code_of(k), from));
                        alerts.push(ev.title.clone());
                        self.news.push(Note::new(now, &ev.title, true, "politics"));
                        self.ev.waiting.push(ev);
                    }
                }
            }
        } else {
            // (bots playing both sides: the offers to "the player" are taken as the bot would)
            self.rel.calls.clear();
            for (k, from) in std::mem::take(&mut self.diplo.offers) {
                let me_c = self.country(self.me);
                if self.answer_of(k, from).is_ok() {
                    self.diplo.sign(k, from, me_c, now as i32, k.years());
                }
            }
        }
        alerts
    }

    /// Money from the treasury of `from` to that of `to` (both countries of the world).
    fn transfer(&mut self, from: usize, to: usize, amt: f32) {
        let n = &mut self.h.world.nations;
        if from >= n.len() || to >= n.len() || amt <= 0.0 {
            return;
        }
        let amt = amt.min(n[from].treasury.max(0.0));
        n[from].treasury -= amt;
        n[to].treasury += amt;
        self.rel.paid[from] += amt;
        self.rel.got[to] += amt;
    }

    /// The game's war begins: each side's allies and guarantors are called; those who come send
    /// money every month and close their markets to the enemy; who does not is remembered.
    pub fn pair_war_begins(&mut self) {
        if off() {
            return;
        }
        let now = self.month() as i32;
        let att = self.state.aggressor.unwrap_or(self.me);
        let (a, b) = (self.country(att), self.country(1 - att));
        let names: Vec<String> = self.countries.iter().map(|c| c.0.clone()).collect();
        let mut rng = Rng::new(hash64(self.seed ^ 0xA11 ^ (now as u64) << 8));
        // (a war of the world between the two goes on as the game's own)
        let w = match self.diplo.wars.iter().position(|w| matches!((w.side_of(a), w.side_of(b)), (Some(x), Some(y)) if x != y)) {
            Some(w) => w,
            None => {
                // (the two leave whatever wars of the world they fought)
                for c in [a, b] {
                    if let Some(w) = self.diplo.war_of(c) {
                        self.diplo.wars[w].sides.iter_mut().for_each(|s| s.retain(|&x| x != c));
                        if self.diplo.wars[w].sides.iter().any(|s| s.is_empty()) {
                            self.diplo.wars.remove(w);
                        }
                    }
                }
                let sides = self.diplo.coalition(a, b);
                let cause = if self.holds_of(a, b) > 0.0 { "война за утраченные земли" } else { "война" };
                self.diplo.begin(sides, now, cause, &names, &mut rng)
            }
        };
        let war = self.diplo.wars[w].clone();
        for s in 0..2 {
            // (each side's list as the war has it: the two may not head them, when the game's war
            // grew out of a war of the world)
            let me_c = self.country(s);
            let side = war.side_of(me_c).unwrap_or(if s == att { 0 } else { 1 });
            self.rel.helpers[s] = war.sides[side].iter().copied().filter(|&x| x != me_c && x != self.country(1 - s)).map(|x| (x, 0.0)).collect();
            let enemy = self.country(1 - s);
            for &(x, _) in &self.rel.helpers[s].clone() {
                self.cut_way(x, enemy);
                let line = format!("{} вступает в войну на стороне {}: шлёт деньги, закрывает рынки врагу", names[x], names[self.country(s)]);
                self.news.push(Note::new(now as u32, &line, true, "politics"));
            }
        }
    }

    /// The game's war is over: its memory, the truce (as long as the game's), the ways open again.
    pub fn pair_war_ends(&mut self, fallen: Option<usize>) {
        let now = self.month() as i32;
        let (a, b) = (self.country(0), self.country(1));
        let names: Vec<String> = self.countries.iter().map(|c| c.0.clone()).collect();
        if let Some(w) = self.diplo.wars.iter().position(|w| matches!((w.side_of(a), w.side_of(b)), (Some(x), Some(y)) if x != y)) {
            let held = self.h.war.people();
            let winner_side = match fallen {
                Some(s) => Some(1 - s),
                None if held[0] - held[1] > 0.1 => Some(0),
                None if held[1] - held[0] > 0.1 => Some(1),
                None => None,
            };
            let win = winner_side.and_then(|s| self.diplo.wars[w].side_of(self.country(s)));
            self.diplo.end(w, win, false, now, &names);
            let until = now + 12 * (crate::world::statecraft::TRUCE as i32 / 12).max(1);
            for p in self.diplo.pacts.iter_mut().filter(|p| p.kind == Kind::Truce && p.has(a) && p.has(b)) {
                p.until = until;
            }
        }
        self.rel.helpers = [Vec::new(), Vec::new()];
        self.reopen_ways();
        self.relations_economy();
    }

    /// Whether country `c` would take the player's proposal of a pact of `kind`; why not, if not.
    pub fn answer_of(&self, kind: Kind, c: usize) -> Result<String, String> {
        let me = self.country(self.me);
        let d = &self.diplo;
        let op = d.of(c, me);
        let pc = |x: f32| format!("{:+.0}", x * 100.0);
        if c == me {
            return Err("это мы".into());
        }
        if d.has(kind, me, c) || (kind.mutual() && d.has(kind, c, me)) {
            return Err("договор уже в силе".into());
        }
        if d.enemies(me, c) || (self.h.at_war && c == self.country(1 - self.me)) {
            return Err("мы воюем".into());
        }
        if let Some(&(_, _, m)) = self.rel.refused.iter().find(|r| r.0 == c && r.1 == kind) {
            if self.month() < m + REFUSED_PAUSE {
                return Err("недавно уже отказали — подождите год".into());
            }
        }
        let gdp = |x: usize| self.h.world.nations.get(x).map_or(1e-6, |n| n.gdp.max(1e-6));
        let near = self.rel.adj.get(c).is_some_and(|a| a.contains(&me) || a.iter().any(|x| self.rel.adj[*x].contains(&me)));
        match kind {
            Kind::Alliance => {
                let mine = d.allies(c).len();
                let shared = (0..self.countries.len()).find(|&x| x != c && x != me && d.of(x, c) < -0.2 && d.of(x, me) < -0.2);
                let threat = self.rel.adj.get(c).map_or(0.0, |a| a.iter().map(|&x| gdp(x) / gdp(c) * (-d.of(x, c)).max(0.0)).fold(0.0f32, f32::max));
                if !near {
                    Err("слишком далеко: союз нужен соседям и соседям соседей".into())
                } else if op < 0.3 {
                    Err(format!("нам недостаточно доверяют: мнение {} (нужно не меньше +30)", pc(op)))
                } else if mine >= 2 {
                    Err("у них уже два союза".into())
                } else if let Some(x) = shared {
                    Ok(format!("согласятся: общий враг — {}; мнение {}", self.countries[x].0, pc(op)))
                } else if threat > 0.4 {
                    Ok(format!("согласятся: им грозит сильный сосед; мнение {}", pc(op)))
                } else if op >= 0.6 {
                    Ok(format!("согласятся: дружба крепка, мнение {}", pc(op)))
                } else {
                    Err(format!("не видят нужды: общего врага нет, им ничто не грозит; мнение {}", pc(op)))
                }
            }
            Kind::Peace => {
                let neighbour = self.rel.adj.get(c).is_some_and(|a| a.contains(&me));
                let lust = self.polity.war_lust(c, me);
                if !neighbour {
                    Err("договор о ненападении — для соседей".into())
                } else if op < -0.3 {
                    Err(format!("слишком враждебны: мнение {}", pc(op)))
                } else if gdp(c) > 1.6 * gdp(me) && lust > 1.2 && op < 0.1 {
                    Err("им выгоднее держать руки свободными: они сильнее и воинственны".into())
                } else {
                    Ok(format!("согласятся: мнение {}", pc(op)))
                }
            }
            Kind::Trade => {
                let trade = self.polity.of.get(c).map_or(0.0, |p| p.clout[polity::MERCHANTS] + p.clout[polity::CAPITAL]);
                let room = (d.pacts_of(c).iter().filter(|p| p.kind == Kind::Trade).count() as f32) < 2.0 + 8.0 * trade;
                if op < 0.05 {
                    Err(format!("с нами не хотят торговать: мнение {}", pc(op)))
                } else if !room {
                    Err("у них и так много торговых договоров".into())
                } else {
                    Ok(format!("согласятся: купцы и промышленники у них за ({:.0}% силы общества); мнение {}", trade * 100.0, pc(op)))
                }
            }
            Kind::Guarantee => {
                if gdp(me) < 1.3 * gdp(c) {
                    Err("защищать можно только страну заметно слабее нас".into())
                } else if op < 0.0 {
                    Err(format!("нам не доверяют: мнение {}", pc(op)))
                } else {
                    Ok(format!("примут с благодарностью: мнение {}", pc(op)))
                }
            }
            Kind::Marriage => {
                let crowned = |x: usize| self.polity.of.get(x).is_some_and(|p| p.regime.crowned() && p.house.is_some());
                if !crowned(me) || !crowned(c) {
                    Err("брак возможен только между правящими домами монархий".into())
                } else if op < 0.05 {
                    Err(format!("двор не расположен: мнение {}", pc(op)))
                } else {
                    Ok(format!("двор согласен: мнение {}", pc(op)))
                }
            }
            Kind::Truce | Kind::Tribute => Err("такой договор не предлагают".into()),
        }
    }

    /// The player's decisions in diplomacy (from the form): a pact proposed (`pact=k:c`), broken
    /// (`unpact=k:c`), an envoy sent (`envoy=c`), a war of the world left (`leave=1`).
    pub fn relations_form(&mut self, form: &[(String, String)]) {
        let get = |k: &str| form.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
        let pair = |v: &str| -> Option<(Kind, usize)> {
            let (k, c) = v.split_once(':')?;
            Some((kind_of(k.trim().parse().ok()?)?, c.trim().parse().ok()?))
        };
        let now = self.month();
        let me = self.country(self.me);
        let n = self.countries.len();
        if let Some((k, c)) = get("pact").as_deref().and_then(pair).filter(|x| x.1 < n) {
            let name = self.countries[c].0.clone();
            match self.answer_of(k, c) {
                Ok(why) => {
                    let (a, b) = if k == Kind::Guarantee { (me, c) } else { (me, c) };
                    self.diplo.sign(k, a, b, now as i32, k.years());
                    self.diplo.chronicle.push((now as i32, me, format!("{}: {} и {}", k.name(), self.countries[me].0, name)));
                    self.news.push(Note::new(now, &format!("{name} принимает: {}", k.name().to_lowercase()), true, "politics").with_tip(why));
                    self.relations_economy();
                }
                Err(why) => {
                    self.rel.refused.retain(|r| !(r.0 == c && r.1 == k));
                    self.rel.refused.push((c, k, now));
                    self.news.push(Note::new(now, &format!("{name} отказывает: {} — {why}", k.name().to_lowercase()), true, "politics"));
                }
            }
        }
        if let Some((k, c)) = get("unpact").as_deref().and_then(pair).filter(|x| x.1 < n) {
            if self.diplo.cancel(k, me, c) {
                let i = c * n + me;
                // (who breaks an alliance or a pact is remembered for it; the others less)
                match k {
                    Kind::Alliance | Kind::Peace | Kind::Guarantee => self.diplo.betrayed[i] = now as i32 - 12 * 20,
                    Kind::Tribute => self.diplo.betrayed[i] = now as i32,
                    _ => {}
                }
                self.news.push(Note::new(now, &format!("расторгнут: {} с {}", k.name().to_lowercase(), self.countries[c].0), true, "politics"));
                self.relations_economy();
            }
        }
        if let Some(c) = get("envoy").and_then(|v| v.trim().parse::<usize>().ok()).filter(|&c| c < n && c != me) {
            let last = self.rel.envoys.iter().find(|e| e.0 == c).map(|e| e.1);
            let cost = ENVOY_COST * self.h.home(self.me).state_budget();
            if last.is_none_or(|m| now >= m + ENVOY_PAUSE) && self.h.home(self.me).treasury >= cost {
                self.h.home_mut(self.me).treasury -= cost;
                let i = c * n + me;
                self.diplo.warmth[i] = (self.diplo.warmth[i] + ENVOY_WARMTH).min(WARMTH_MOST);
                self.rel.envoys.retain(|e| e.0 != c);
                self.rel.envoys.push((c, now));
                self.news.push(Note::new(now, &format!("посольство в {} с дарами", self.countries[c].0), false, "politics").with_tip(format!("стоило {cost:.2}; мнение о нас растёт")));
            }
        }
        if get("leave").is_some() {
            if let Some(w) = self.diplo.war_of(me) {
                let side = self.diplo.wars[w].side_of(me).unwrap_or(0);
                let head = self.diplo.wars[w].sides[side][0];
                let enemies = self.diplo.wars[w].sides[1 - side].clone();
                if head != me {
                    self.diplo.wars[w].sides[side].retain(|&x| x != me);
                    for e in enemies {
                        self.diplo.sign(Kind::Truce, me, e, now as i32, 5);
                    }
                    self.diplo.betrayed[head * n + me] = now as i32;
                    self.diplo.cancel(Kind::Alliance, me, head);
                    self.news.push(Note::new(now, &format!("мы вышли из войны на стороне {}: союз расторгнут", self.countries[head].0), true, "politics"));
                    self.reopen_ways();
                }
            }
        }
    }

    /// The player answers an ally that calls him to its war (`join`), or the bot does.
    pub fn answer_call(&mut self, head: usize, join: bool) {
        let now = self.month();
        let me = self.country(self.me);
        let n = self.countries.len();
        let Some(w) = self.diplo.war_of(head) else { return };
        let side = self.diplo.wars[w].side_of(head).unwrap_or(0);
        if join {
            self.diplo.wars[w].sides[side].push(me);
            let enemies = self.diplo.wars[w].sides[1 - side].clone();
            for &e in &enemies {
                self.cut_way(me, e);
            }
            // an enemy at our border: the war is ours too — it becomes the rival, the war is
            // declared and fought on the map; a far one: we send money and close our markets to it
            let near = self.neighbours();
            let target = enemies.iter().copied().find(|e| near.contains(e));
            if let (Some(e), false) = (target, self.h.at_war) {
                let rival = self.country(1 - self.me);
                let ok = e == rival || {
                    let waiting = std::mem::take(&mut self.ev.waiting);
                    let claims = self.state.claim;
                    self.state.claim = [None, None];
                    let ok = self.set_rival(e);
                    if !ok {
                        self.state.claim = claims;
                    }
                    self.ev.waiting = waiting;
                    ok
                };
                if ok {
                    self.news.push(Note::new(now, &format!("мы вступили в войну на стороне {}: война с {} начинается на нашей границе", self.countries[head].0, self.countries[e].0), true, "army"));
                    self.go_to_war(self.me);
                    return;
                }
            }
            let cost = HELP * self.h.home(self.me).state_budget();
            self.news.push(Note::new(now, &format!("мы вступили в войну на стороне {}: враг далеко — шлём деньги ({cost:.2} в месяц) и закрыли ему рынки", self.countries[head].0), true, "politics"));
        } else {
            self.diplo.betrayed[head * n + me] = now as i32;
            self.diplo.cancel(Kind::Alliance, me, head);
            self.diplo.cancel(Kind::Guarantee, me, head);
            self.news.push(Note::new(now, &format!("мы отказали {}: союз расторгнут, нас считают вероломными", self.countries[head].0), true, "politics"));
        }
    }

    /// The reasons of country `a`'s opinion of `b`, in words with numbers (the largest first).
    pub fn reasons_words(&self, a: usize, b: usize) -> Vec<(&'static str, f32)> {
        let n = self.diplo.n;
        let w = self.diplo.why.get(a * n + b).copied().unwrap_or([0.0; WHY]);
        let mut out: Vec<(&'static str, f32)> = (0..WHY).filter(|&k| w[k].abs() > 0.005).map(|k| (strategy::world::diplomacy::WHY_NAME[k], w[k])).collect();
        out.sort_by(|x, y| y.1.abs().total_cmp(&x.1.abs()));
        out
    }

    /// Whether the other government of the game is held back from a claim on the player by a pact
    /// (a truce or non-aggression — a warlike one may break the latter), and how much the
    /// player's allies weigh against it (1: none).
    pub fn restrained(&self, s: usize) -> (bool, f32) {
        let (a, b) = (self.country(s), self.country(1 - s));
        let bold = (self.folk.hawk(a).max(0.0) + self.polity.of[a].moves[polity::HAWKS]).min(1.0);
        let held = match self.diplo.restraint(a, b, bold) {
            Some("уже воюет") => false,
            Some(_) => true,
            None => false,
        };
        let gdp = |x: usize| self.h.world.nations.get(x).map_or(1e-6, |n| n.gdp.max(1e-6));
        let [_, def] = self.diplo.coalition(a, b);
        let help: f32 = def.iter().skip(1).map(|&x| gdp(x) * (0.5 + 0.55 * self.diplo.of(x, b)).clamp(0.0, 1.0)).sum::<f32>();
        (held, 1.0 + 0.5 * help / gdp(b))
    }
}
