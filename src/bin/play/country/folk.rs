//! The world's people in the game: those the world's past left alive go on living month by month
//! (they age, die, new ones come of age each year in every country), and they are who the game's
//! posts are given to — the three who could run the player's economy and the three who could head
//! his laboratories are economists and scientists of his own country (if it has none of note, only
//! unknown officials are there to choose from), the rival's government is its own ruler and
//! ministers, the generals who lead the groups of armies are its soldiers with their records. What
//! they do in office goes into their lives (anyone may read them in the folder of people); a death
//! in office leaves the post empty.

use strategy::economy::advisor::{Economist, Mind};
use strategy::economy::science::Scientist;
use strategy::military::front::General;
use strategy::rng::{hash64, Rng};
use strategy::world::folk::{Calling, Post, Times};

use crate::game::Note;
use crate::Game;

/// The game's view of the world's people: the persons behind the player's choices of advisers.
#[derive(Clone, Debug, Default)]
pub struct Roster {
    /// The persons of `Game::pool` (economists) and `Game::scholars` (scientists), in their order.
    pub pool: Vec<u32>,
    pub scholars: Vec<u32>,
    /// The persons heading the rival's economy and laboratories.
    pub foe_econ: Option<u32>,
    pub foe_science: Option<u32>,
    /// What the folk's economists of the other countries had done to their productivity, and what
    /// was taken of their treasuries, when the game last looked (the year's change is laid on them).
    pub econ_seen: Vec<f32>,
    pub taken_seen: Vec<f32>,
    /// People the government dismissed in disgrace: no longer among the choices.
    pub out: Vec<u32>,
}

/// How many could run the economy, and head science.
const CHOICE: usize = 3;

impl Game {
    /// The map country of side `s`.
    pub fn country(&self, s: usize) -> usize {
        self.h.war.th.countries[s]
    }

    /// The side of map country `c`, if it is one of the two.
    pub fn side_of(&self, c: usize) -> Option<usize> {
        (0..2).find(|&s| self.country(s) == c)
    }

    /// How developed map country `c` is against the world now (0 … 1, the usual about 0.5): its
    /// output a head against the world's.
    pub fn dev_of(&self, c: usize) -> f32 {
        let heads: Vec<f32> = self.h.world.nations.iter().filter(|x| x.people() > 0.1).map(|x| (x.gdp_real / x.people()).max(1e-9).ln()).collect();
        let mean = heads.iter().sum::<f32>() / heads.len().max(1) as f32;
        let n = &self.h.world.nations[c];
        let mine = (n.gdp_real / n.people().max(1e-6)).max(1e-9).ln();
        (0.5 + 0.4 * (mine - mean)).clamp(0.02, 1.0)
    }

    /// What map country `c` lives through now, for its people.
    pub fn times_now(&self, c: usize) -> Times {
        let n = &self.h.world.nations[c];
        let since_start = self.month() as f32 / 12.0;
        let mut last: Option<(f32, Option<bool>)> = None;
        for w in &self.past {
            if let Some(i) = w.sides.iter().position(|&x| x == c) {
                let ago = w.year as f32 - w.months as f32 / 12.0 + since_start;
                if last.is_none_or(|l| ago < l.0) {
                    last = Some((ago.max(0.0), w.winner.map(|x| x == i)));
                }
            }
        }
        let side = self.side_of(c);
        if let Some(s) = side {
            for &(m, won) in &self.spirits.game_wars[s] {
                let ago = self.month().saturating_sub(m) as f32 / 12.0;
                if last.is_none_or(|l| ago < l.0) {
                    last = Some((ago, won));
                }
            }
        }
        let at_war = side.is_some() && self.h.at_war;
        let grudge = side.is_some_and(|s| self.state.grudge[s]);
        let glory = match side {
            Some(s) => self.h.war.sides[s].glory.clone(),
            None => self.staffs.get(c).map_or(Vec::new(), |x| x.glory.clone()),
        };
        let r = self.polity.of.get(c).map(|p| p.regime);
        Times {
            pop: n.people(),
            dev: self.dev_of(c),
            at_war,
            last_war: last,
            grudge,
            stage: self.know.get(c).map_or(0, |k| k.stage(&self.pack)),
            glory,
            lifelong: r.is_some_and(|r| r.lifelong()),
            term: r.and_then(|r| r.elected()).unwrap_or(0.0),
            soldiers: r.is_some_and(|r| r.soldiers()),
            crowned: r.is_some_and(|r| r.crowned()),
        }
    }

    /// The people of the game's posts as it begins: the player's choices of advisers among his
    /// country's economists and scientists, the rival's heads of economy and science.
    pub fn people_start(&mut self) {
        let n = self.h.world.nations.len();
        self.roster.econ_seen = self.folk.econ.clone();
        self.roster.taken_seen = self.folk.taken.clone();
        self.roster.econ_seen.resize(n, 0.0);
        self.roster.taken_seen.resize(n, 0.0);
        self.fill_choices();
        self.foe_people();
        // the debts the past left: what rulers and ministers took, and the wars, are owed now
        for s in 0..2 {
            let c = self.country(s);
            let owed = self.folk.taken.get(c).copied().unwrap_or(0.0).min(18.0);
            if owed > 1.0 {
                let income = self.income(s);
                let rate = self.loan_rate(s);
                self.policy.loans[s].push(crate::country::policy::Loan { owed: owed * income, rate, left: crate::country::policy::LOAN_TERM * 2 });
                self.policy.debt[s] = self.policy.loans[s].iter().map(|l| l.owed).sum();
            }
        }
    }

    /// The player's choices of advisers: living economists and scientists of his country, the best
    /// known first (unknown officials where there are not enough of them).
    pub fn fill_choices(&mut self) {
        let me = self.me;
        let c = self.country(me);
        let now = self.month() as i32;
        let t = self.times_now(c);
        let mut r = Rng::new(hash64(self.seed ^ 0xF01C ^ (now as u64) << 6 ^ c as u64));
        for (calling, sci) in [(Calling::Economist, false), (Calling::Scientist, true)] {
            let held = if sci { self.scholar } else { self.adviser };
            let mut ids: Vec<u32> = if sci { self.roster.scholars.clone() } else { self.roster.pool.clone() };
            let mut living: Vec<u32> = self.folk.living(c, calling).into_iter().filter(|id| !ids.contains(id) && !self.roster.out.contains(id) && self.folk.get(*id).age(now) < 72.0).collect();
            living.truncate(CHOICE * 2);
            let mut next = |g: &mut Game, r: &mut Rng| -> u32 {
                if living.is_empty() {
                    let age = 30.0 + 18.0 * r.f32();
                    g.folk.born(&g.pack, c, &t, now, Some(calling), age, r)
                } else {
                    living.remove(0)
                }
            };
            // (each keeps his place in the list — the player's choice is by place; the dead are
            // replaced where they stood, the one in office stays)
            for k in 0..ids.len() {
                if (!self.folk.get(ids[k]).alive() || self.roster.out.contains(&ids[k])) && held != Some(k) {
                    ids[k] = next(self, &mut r);
                }
            }
            while ids.len() < CHOICE {
                let id = next(self, &mut r);
                ids.push(id);
            }
            let names: Vec<(String, Mind)> = ids.iter().map(|&id| (self.folk.get(id).name.clone(), self.folk.get(id).mind())).collect();
            if sci {
                self.roster.scholars = ids;
                self.scholars = names;
            } else {
                self.roster.pool = ids;
                self.pool = names;
            }
        }
    }

    /// The rival's heads of economy and science: those in office in its country (or the best known,
    /// or an unknown official), and the economist and scientist the game gives them.
    pub fn foe_people(&mut self) {
        let foe = 1 - self.me;
        let c = self.country(foe);
        let now = self.month() as i32;
        let t = self.times_now(c);
        let mut r = Rng::new(hash64(self.seed ^ 0xF0E1 ^ (c as u64) << 8 ^ now as u64));
        let head = |g: &mut Game, post: Post, calling: Calling, r: &mut Rng| -> u32 {
            if let Some(id) = g.folk.holder(c, post).filter(|&id| g.folk.get(id).alive()) {
                return id;
            }
            let id = match g.folk.living(c, calling).first() {
                Some(&id) => id,
                None => {
                    let age = 35.0 + 15.0 * r.f32();
                    g.folk.born(&g.pack, c, &t, now, Some(calling), age, r)
                }
            };
            g.folk.appoint(c, post, id, now);
            id
        };
        let e = head(self, Post::Economy, Calling::Economist, &mut r);
        self.roster.foe_econ = Some(e);
        self.foe_adviser = self.folk.get(e).name.clone();
        self.h.econ[foe] = Some(Economist::new(self.folk.get(e).mind(), hash64(self.seed ^ 0xB07 ^ c as u64 ^ (now as u64) << 20), &self.pack));
        if self.pack.science.is_some() {
            let sc = head(self, Post::Science, Calling::Scientist, &mut r);
            self.roster.foe_science = Some(sc);
            self.foe_scholar = self.folk.get(sc).name.clone();
            self.h.scientists[foe] = Some(Scientist::new(self.folk.get(sc).mind(), hash64(self.seed ^ 0x5C7 ^ c as u64 ^ (now as u64) << 20), &self.pack));
        }
    }

    /// The player gave a post (`Post::Economy`, `Post::Science`) to the person of his choice (None:
    /// he keeps it himself): the world's people know it.
    pub fn folk_appoint(&mut self, post: Post, who: Option<usize>) {
        let me = self.me;
        let c = self.country(me);
        let now = self.month() as i32;
        let list = if post == Post::Science { &self.roster.scholars } else { &self.roster.pool };
        match who.and_then(|k| list.get(k).copied()) {
            Some(id) => {
                self.folk.appoint(c, post, id, now);
                let p = &mut self.folk.people[id as usize];
                p.deeds.push(strategy::world::folk::Deed { month: now, text: format!("назначен правительством: {}", post.word()) });
                p.fame += 0.2;
            }
            None => {
                if let Some(old) = self.folk.holder(c, post) {
                    self.folk.vacate(c, post, old, now);
                }
            }
        }
    }

    /// A general for side `s`: one of its last wars' veterans, else one of its living soldiers not
    /// in command, else a new man (a colonel nobody knew).
    pub fn new_general(&mut self, s: usize, r: &mut Rng) -> General {
        let c = self.country(s);
        let now = self.month() as i32;
        let busy: Vec<String> = self.h.war.command[s].as_ref().map_or(Vec::new(), |gs| gs.iter().filter_map(|g| g.general.as_ref().map(|x| x.name.clone())).collect());
        let spare: Vec<String> = self.spare.iter().map(|g| g.name.clone()).collect();
        while !self.veterans[s].is_empty() {
            let v = General::of_country(&mut self.veterans[s], r);
            // (one who leads a group already, or waits among the spare, or was named in this very
            // muster, is not named twice: a man lay in the veterans' list twice once)
            let named = self.folk.find(c, &v.name).is_some_and(|id| self.folk.get(id).posts.iter().any(|st| st.post == Post::Command && st.to.is_none() && st.from == now));
            if busy.contains(&v.name) || spare.contains(&v.name) || named {
                continue;
            }
            // (a veteran who is a person of the world takes his command)
            if let Some(id) = self.folk.find(c, &v.name) {
                self.folk.appoint(c, Post::Command, id, now);
            }
            return v;
        }
        let free: Vec<u32> = self
            .folk
            .living(c, Calling::General)
            .into_iter()
            .filter(|&id| {
                let p = self.folk.get(id);
                // (not one given a command this very month: generals are named by the batch at a
                // muster, before any of them leads a group — one man led seven armies so)
                let just = p.posts.iter().any(|st| st.post == Post::Command && st.to.is_none() && st.from == now);
                p.age(now) < 66.0 && !busy.contains(&p.name) && !spare.contains(&p.name) && p.post() != Some(Post::Ruler) && !just
            })
            .collect();
        let id = if free.is_empty() {
            let t = self.times_now(c);
            let age = 34.0 + 14.0 * r.f32();
            self.folk.born(&self.pack, c, &t, now, Some(Calling::General), age, r)
        } else {
            // (the best known first, as a rule; now and then someone further down the list)
            let k = if r.f32() < 0.7 { 0 } else { r.below(free.len()) };
            free[k]
        };
        self.folk.appoint(c, Post::Command, id, now);
        self.folk.get(id).general()
    }

    /// A war of the game is over: what each side's generals did goes into their lives.
    pub fn folk_after_war(&mut self, loser: Option<usize>) {
        let now = self.month() as i32;
        let held = self.h.war.people();
        let mut r = Rng::new(hash64(self.seed ^ 0xAF7E ^ now as u64));
        for s in 0..2 {
            let c = self.country(s);
            let enemy = self.h.home(1 - s).name.clone();
            let gens: Vec<General> = self.h.war.command[s].as_ref().map_or(Vec::new(), |gs| gs.iter().filter_map(|g| g.general.clone()).collect());
            let pairs: Vec<(u32, General)> = gens.into_iter().filter_map(|g| self.folk.find(c, &g.name).map(|id| (id, g))).collect();
            let won = match loser {
                Some(l) => Some(l != s),
                None if held[s] - held[1 - s] > 0.03 => Some(true),
                None if held[1 - s] - held[s] > 0.03 => Some(false),
                None => None,
            };
            self.folk.after_war(c, &pairs, won, &enemy, now, &mut r);
        }
    }

    /// The people's month: in the two countries, the old die (a death in office leaves the post
    /// empty: the sphere goes back to the player, the bot finds another man); once a year, in every
    /// country, new people come of age, posts are filled, scientists find things out, the other
    /// countries' economists better or rob their economies. What should stop the clock.
    pub fn folk_month(&mut self) -> Vec<String> {
        let now = self.month() as i32;
        let me = self.me;
        let mut alerts = Vec::new();
        let mut r = Rng::new(hash64(self.seed ^ 0xF017 ^ (now as u64) << 4));
        // deaths, month by month, in the two countries
        for s in 0..2 {
            let c = self.country(s);
            let living: Vec<u32> = self.folk.people.iter().filter(|p| p.country == c && p.alive()).map(|p| p.id).collect();
            for id in living {
                let age = self.folk.get(id).age(now);
                let chance = 1.0 - (1.0 - mortality(age)).powf(1.0 / 12.0);
                if r.f32() < chance {
                    let p = self.folk.get(id).clone();
                    self.folk.die(id, now, if age > 70.0 { "от старости" } else { "от болезни" });
                    if p.fame >= 0.7 || p.post().is_some() {
                        let line = format!("умер {} — {}", p.name, p.post().map_or(p.calling.word().to_string(), |x| x.word().to_string()));
                        if s == me {
                            self.news.push(Note::new(now as u32, &line, p.post().is_some(), "people"));
                        } else {
                            self.news.push(Note::new(now as u32, &format!("{}: {line}", self.h.home(s).name), false, "people"));
                        }
                    }
                    // a death in office
                    if s == me {
                        if self.adviser.is_some_and(|k| self.roster.pool.get(k) == Some(&id)) {
                            self.adviser = None;
                            self.h.econ[me] = None;
                            let line = format!("умер глава хозяйства {} — экономикой вы правите сами", p.name);
                            alerts.push(line.clone());
                            self.news.push(Note::new(now as u32, &line, true, "econ"));
                        }
                        if self.scholar.is_some_and(|k| self.roster.scholars.get(k) == Some(&id)) {
                            self.scholar = None;
                            self.h.scientists[me] = None;
                            let line = format!("умер глава науки {} — науку вы ведёте сами", p.name);
                            alerts.push(line.clone());
                            self.news.push(Note::new(now as u32, &line, true, "science"));
                        }
                    } else if self.roster.foe_econ == Some(id) || self.roster.foe_science == Some(id) {
                        self.foe_people();
                    }
                    // (a candidate who died is replaced at once, not at the year's end)
                    if s == me && (self.roster.pool.contains(&id) || self.roster.scholars.contains(&id)) {
                        self.fill_choices();
                    }
                }
            }
        }
        if now % 12 == 11 {
            self.folk_year(&mut r);
            self.fill_choices();
        }
        alerts
    }

    /// A year of every country's people: new ones come of age, posts filled, discoveries made; the
    /// other countries' economists' work and theft laid on their economies.
    fn folk_year(&mut self, r: &mut Rng) {
        let now = self.month() as i32;
        let pack = self.pack.clone();
        let n = self.h.world.nations.len();
        let mine = self.country(self.me);
        self.folk.player = Some(mine);
        for c in 0..n.min(self.know.len()) {
            if self.h.world.nations[c].people() < 0.05 {
                continue;
            }
            let t = self.times_now(c);
            let seen_v = self.tech_seen(c);
            let seen = |t: usize| seen_v.get(t).copied().unwrap_or(0.0);
            let before: Vec<bool> = self.know[c].known.clone();
            let mut know = self.know[c].clone();
            let _ = self.folk.year(&pack, c, &t, now, &mut know, &seen, r);
            // discoveries of the year: what they do
            let new: Vec<usize> = (0..pack.techs.len()).filter(|&t| know.known[t] && !before[t]).collect();
            self.know[c] = know;
            for t in new {
                self.apply_tech(c, t);
            }
            // the other countries' economists: their year's work on their economies
            if self.side_of(c).is_none() {
                let de = self.folk.econ.get(c).copied().unwrap_or(0.0) - self.roster.econ_seen.get(c).copied().unwrap_or(0.0);
                let dt = self.folk.taken.get(c).copied().unwrap_or(0.0) - self.roster.taken_seen.get(c).copied().unwrap_or(0.0);
                let nation = &mut self.h.world.nations[c];
                if de.abs() > 1e-6 {
                    nation.prod.iter_mut().for_each(|p| *p *= de.exp());
                }
                if dt > 0.0 {
                    let income = (nation.gdp * nation.tax).max(0.0);
                    let take = (dt * income).min(nation.treasury.max(0.0));
                    nation.treasury -= take;
                    let owner = pack.sectors.first().map_or(0, |x| x.owner);
                    if let Some(m) = nation.money.get_mut(owner) {
                        *m += take;
                    }
                }
            }
            if c < self.roster.econ_seen.len() {
                self.roster.econ_seen[c] = self.folk.econ.get(c).copied().unwrap_or(0.0);
                self.roster.taken_seen[c] = self.folk.taken.get(c).copied().unwrap_or(0.0);
            }
        }
        // the rival's heads may have changed (a new ruler brings his own men)
        let foe = 1 - self.me;
        let c = self.country(foe);
        if self.folk.holder(c, Post::Economy) != self.roster.foe_econ || (self.pack.science.is_some() && self.folk.holder(c, Post::Science) != self.roster.foe_science) {
            self.foe_people();
            let name = self.h.home(foe).name.clone();
            self.news.push(Note::new(now as u32, &format!("{name}: новые люди во главе хозяйства и науки"), false, "politics"));
        }
        // the world's chronicle of people: news of the two countries
        let since = now - 12;
        let lines: Vec<(i32, usize, String)> = self.folk.chronicle.iter().filter(|x| x.0 > since && x.0 <= now).cloned().collect();
        for (m, c, line) in lines {
            if let Some(s) = self.side_of(c) {
                let text = if s == self.me { line } else { format!("{}: {line}", self.h.home(s).name) };
                self.news.push(Note::new(m.max(0) as u32, &text, false, "people"));
            }
        }
    }
}

/// Chance of dying within a year at `age` (as the world's past has it).
fn mortality(age: f32) -> f32 {
    (0.0025 * ((age - 30.0) / 10.5).exp()).min(0.6)
}
