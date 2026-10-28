//! What a choice of an event does — the player's, or the one a bot's government makes by what it
//! values.

use super::*;

impl Game {
    /// What a government that is a bot chooses: by what it has.
    pub(super) fn bot_choice(&self, ev: &Event) -> usize {
        let s = ev.side;
        let n = self.h.home(s);
        let rich = n.treasury > 0.1 * n.gdp.max(1e-6);
        let mut r = Rng::new(hash64(self.seed ^ ev.id ^ 0xEE));
        match ev.kind {
            Kind::Unrest(_) | Kind::Famine(_) => {
                if rich {
                    0
                } else {
                    1
                }
            }
            Kind::Treasury => 0,
            Kind::Thief => (r.f32() < 0.5) as usize,
            Kind::General(_) => 0,
            Kind::Claim => 0,
            Kind::Weary => {
                if self.takes_peace(1 - s) {
                    0
                } else {
                    1
                }
            }
            Kind::Coup => 0,
            Kind::PeaceOffer => 0,
            Kind::Lenders => {
                if self.state.stability[s] > 0.6 && self.h.at_war {
                    1
                } else {
                    0
                }
            }
            Kind::Dear => {
                if rich {
                    1
                } else {
                    0
                }
            }
            Kind::Reform => {
                if rich {
                    0
                } else {
                    2
                }
            }
            Kind::Genius(..) => {
                // (a government whose course is science pays even when money is short)
                let science = self.courses.now[s].as_ref().is_some_and(|c| c.kind == crate::country::course::Kind::Science);
                if rich || science {
                    0
                } else {
                    1
                }
            }
            Kind::Hero(_) => {
                let c = self.country(s);
                let hawk = self.folk.hawk(c);
                if hawk > 0.2 {
                    0
                } else if self.state.stability[s] > 0.5 {
                    2
                } else {
                    1
                }
            }
            Kind::Panic => {
                if rich {
                    0
                } else {
                    1
                }
            }
            Kind::Realm(e) => self.realm_bot(s, e),
            Kind::AllyCalls(_) | Kind::Offer(..) => 0,
            Kind::Flight => {
                let course = self.courses.now[s].as_ref().map(|c| c.kind);
                if matches!(course, Some(crate::country::course::Kind::Industry) | Some(crate::country::course::Kind::Trade)) {
                    1
                } else {
                    0
                }
            }
        }
    }

    /// What a choice does.
    pub(super) fn choose(&mut self, ev: &Event, k: usize) {
        let s = ev.side;
        let me = self.me;
        match (ev.kind, k) {
            (Kind::Realm(e), k) => self.realm_choose(s, e, k),
            (Kind::Unrest(g), 0) => {
                let n = self.h.home_mut(s);
                let x = 0.3 * n.treasury.max(0.0);
                n.treasury -= x;
                n.money[g] += x;
                n.discontent[g] *= 0.6;
            }
            (Kind::Unrest(g), 1) => {
                self.h.home_mut(s).discontent[g] *= 0.75;
                self.state.mods[s] -= 0.06;
            }
            (Kind::Famine(g), 0) => {
                let n = self.h.home_mut(s);
                let x = 0.25 * n.treasury.max(0.0);
                n.treasury -= x;
                n.money[g] += x;
                n.discontent[g] *= 0.85;
            }
            (Kind::Famine(g), 1) => {
                self.h.home_mut(s).discontent[g] *= 0.9;
                self.state.mods[s] -= 0.02;
            }
            (Kind::Treasury, 0) => {
                let n = self.h.home_mut(s);
                if let Some(g) = (0..n.money.len()).max_by(|&a, &b| n.money[a].total_cmp(&n.money[b])) {
                    let x = 0.1 * n.money[g].max(0.0);
                    n.money[g] -= x;
                    n.treasury += x;
                    n.discontent[g] += 0.5;
                }
            }
            (Kind::Treasury, 1) => {
                let x = (self.h.home(s).mil_share * 0.67).max(0.0);
                self.h.army_share[s] = Some(x);
                self.h.home_mut(s).mil_share = x;
            }
            (Kind::Thief, 0) => {
                self.h.econ[s] = None;
                if s == me {
                    // (the thief is struck off the choices; his life remembers it; another takes
                    // his place among them)
                    if let Some(id) = self.adviser.and_then(|k| self.roster.pool.get(k).copied()) {
                        let now = self.month() as i32;
                        let p = &mut self.folk.people[id as usize];
                        p.deeds.push(strategy::world::folk::Deed { month: now, text: "уличён в казнокрадстве, отдан под суд и снят".into() });
                        p.fame = (p.fame - 0.2).max(0.0);
                        self.roster.out.push(id);
                    }
                    self.adviser = None;
                    self.folk_appoint(strategy::world::folk::Post::Economy, None);
                    self.fill_choices();
                }
                self.ev.took_seen[s] = 0.0;
            }
            (Kind::Thief, _) => {
                self.ev.took_seen[s] = self.h.econ[s].as_ref().map_or(0.0, |e| e.took);
                self.state.mods[s] -= 0.03;
            }
            (Kind::General(g), 0) => {
                let mut r = Rng::new(hash64(self.seed ^ ev.id ^ 0x6E1));
                let new = if s == me && !self.spare.is_empty() {
                    self.spare.remove(0)
                } else {
                    self.new_general(s, &mut r)
                };
                if let Some(gs) = self.h.war.command[s].as_mut() {
                    if let Some(gr) = gs.get_mut(g) {
                        if let Some(old) = gr.general.replace(new) {
                            self.veterans[s].push(old);
                        }
                    }
                }
            }
            (Kind::Claim, 0) => {
                let x = (self.h.home(s).mil_share + 0.15).min(0.9);
                self.h.army_share[s] = Some(x);
                self.h.home_mut(s).mil_share = x;
            }
            (Kind::Claim, 1) => {
                if let Some(c) = self.state.claim[1 - s].as_mut() {
                    c.ready += 3;
                }
                self.state.tension *= 0.8;
            }
            (Kind::Weary, 0) => {
                self.offer_peace(s);
            }
            (Kind::Weary, _) => {
                self.state.mods[s] -= 0.04;
            }
            (Kind::Coup, 0) => {
                let x = (self.h.home(s).mil_share + 0.1).min(0.9);
                self.h.army_share[s] = Some(x);
                self.h.home_mut(s).mil_share = x;
                self.state.mods[s] += 0.15;
            }
            (Kind::Coup, 1) => {
                let n = self.h.home_mut(s);
                n.discontent.iter_mut().for_each(|d| *d *= 0.7);
                n.tax *= 0.85;
                self.state.mods[s] += 0.08;
            }
            (Kind::Lenders, 0) => {
                for l in self.policy.loans[s].iter_mut() {
                    l.left += 24;
                    l.rate += 0.002;
                }
            }
            (Kind::Lenders, 1) => {
                self.policy.loans[s].clear();
                self.policy.debt[s] = 0.0;
                self.policy.no_loans_until[s] = self.month() + 36;
                self.state.mods[s] -= 0.25;
            }
            (Kind::Dear, 0) => {
                let n = self.h.home_mut(s);
                n.discontent.iter_mut().for_each(|d| *d *= 0.85);
                self.state.mods[s] -= 0.03;
            }
            (Kind::Dear, 1) => {
                let n = self.h.home_mut(s);
                let x = 0.15 * n.treasury.max(0.0);
                n.treasury -= x;
                let k = n.money.len().max(1) as f32;
                n.money.iter_mut().for_each(|m| *m += x / k);
                n.discontent.iter_mut().for_each(|d| *d *= 0.9);
            }
            (Kind::Reform, c) if c < 2 => {
                let hunger = self.spirits.list[s].iter().any(|sp| sp.def().id == "hunger");
                let schools = self.spirits.list[s].iter().any(|sp| sp.def().id == "illiterate");
                let (a, b) = if hunger { ("land", "industry") } else if schools { ("schools", "industry") } else { ("industry", "schools") };
                let id = if c == 0 { a } else { b };
                if let Some(p) = crate::country::policy::PLANS.iter().position(|x| x.id == id) {
                    self.begin_plan(s, p);
                }
            }
            (Kind::Genius(id, t), 0) => {
                let n = self.h.home_mut(s);
                let x = 0.12 * n.treasury.max(0.0);
                n.treasury -= x;
                let c = self.country(s);
                let pack = self.pack.clone();
                let now = self.month() as i32;
                let p = self.folk.get(id).clone();
                let gift = 0.6 + 2.0 * p.talent * (0.4 + p.education);
                if let Some(k) = self.know.get_mut(c) {
                    if k.can(&pack, t) {
                        k.progress[t] += gift;
                        let seen = 0.0;
                        if k.progress[t] >= strategy::world::progress::Know::cost(&pack, t, seen) {
                            k.learn(t, now, Some(id));
                            self.apply_tech(c, t);
                            self.folk.people[id as usize].works += 1;
                            self.folk.people[id as usize].deeds.push(strategy::world::folk::Deed { month: now, text: format!("на деньги казны завершил: {}", pack.techs[t].name) });
                            self.folk.people[id as usize].fame += 0.6;
                        } else {
                            self.folk.people[id as usize].deeds.push(strategy::world::folk::Deed { month: now, text: format!("получил деньги казны на работу: {}", pack.techs[t].name) });
                            self.folk.people[id as usize].fame += 0.15;
                        }
                    }
                }
            }
            (Kind::Genius(id, _), _) => {
                // he may leave for the rival, who will be glad of him
                let mut r = Rng::new(hash64(self.seed ^ ev.id ^ 0x9E1));
                let now = self.month() as i32;
                if r.f32() < 0.35 {
                    let to = self.country(1 - s);
                    let name = self.folk.get(id).name.clone();
                    let from = self.h.home(s).name.clone();
                    let rival = self.h.home(1 - s).name.clone();
                    let posts: Vec<strategy::world::folk::Post> = self.folk.get(id).posts.iter().filter(|x| x.to.is_none()).map(|x| x.post).collect();
                    let c = self.country(s);
                    for p in posts {
                        self.folk.vacate(c, p, id, now);
                    }
                    let p = &mut self.folk.people[id as usize];
                    p.country = to;
                    p.deeds.push(strategy::world::folk::Deed { month: now, text: format!("обиженный отказом, уехал из страны {from} в {rival}") });
                    self.folk.chronicle.push((now, to, format!("{name} уехал в {rival}")));
                    self.news.push(Note::new(self.month(), &format!("учёный {name} уехал к соседу ({rival})"), s == me, "people"));
                    if s == me {
                        self.fill_choices();
                    }
                }
            }
            (Kind::Hero(id), 0) => {
                self.state.mods[s] += 0.04;
                let g = self.folk.get(id).general();
                let c = self.country(s);
                let now = self.month() as i32;
                self.folk.appoint(c, strategy::world::folk::Post::Command, id, now);
                if s == me {
                    self.spare.insert(0, g);
                } else {
                    self.veterans[s].insert(0, g);
                }
            }
            (Kind::Hero(id), 1) => {
                self.state.mods[s] -= 0.04;
                let now = self.month() as i32;
                let p = &mut self.folk.people[id as usize];
                p.deeds.push(strategy::world::folk::Deed { month: now, text: "отправлен в отставку правительством".into() });
                p.fame *= 0.8;
            }
            (Kind::Hero(_), _) => {
                self.state.mods[s] -= 0.02;
            }
            (Kind::Panic, k) => {
                self.fiscal.panic[s] = false;
                self.fiscal.cheap[s] = 0;
                let pack = self.pack.clone();
                let owners: Vec<usize> = (0..pack.groups.len()).filter(|&g| crate::country::policy::role(&pack, g) == 3).collect();
                let n = self.h.home_mut(s);
                match k {
                    0 => {
                        let x = 0.2 * n.treasury.max(0.0);
                        n.treasury -= x;
                        if let Some(&g) = owners.first() {
                            n.money[g] += x;
                        }
                        self.fiscal.confidence[s] = (self.fiscal.confidence[s] + 0.2).min(1.2);
                    }
                    1 => {
                        for &g in &owners {
                            n.money[g] *= 0.85;
                            n.discontent[g] += 0.3;
                        }
                        self.fiscal.confidence[s] *= 0.85;
                    }
                    _ => {
                        let mut took = 0.0;
                        for &g in &owners {
                            let x = 0.1 * n.money[g].max(0.0);
                            n.money[g] -= x;
                            took += x;
                        }
                        n.treasury += took;
                        self.fiscal.seized[s] += 0.2;
                        self.state.mods[s] -= 0.03;
                    }
                }
                self.set_fiscal(s, "rate", 1);
            }
            (Kind::Flight, 0) => {
                self.fiscal.seized[s] += 0.05;
                self.fiscal.confidence[s] *= 0.95;
                // (what was still to go stays: half of what fled comes back by force)
                let back = 0.3 * self.fiscal.fled[s];
                self.fiscal.fled[s] -= back;
                let pack = self.pack.clone();
                if let Some(g) = (0..pack.groups.len()).find(|&g| crate::country::policy::role(&pack, g) == 3) {
                    self.h.home_mut(s).money[g] += back;
                }
            }
            (Kind::AllyCalls(head), k) => self.answer_call(head, k == 0),
            (Kind::Offer(k, from), choice) => {
                let me_c = self.country(s);
                if let Some(kind) = crate::world::relations::kind_of(k) {
                    let n = self.diplo.n;
                    if choice == 0 {
                        let now = self.month() as i32;
                        self.diplo.sign(kind, from, me_c, now, kind.years());
                        let line = format!("{}: {} и {}", kind.name(), self.countries[from].0, self.countries[me_c].0);
                        self.diplo.chronicle.push((now, me_c, line));
                        self.relations_economy();
                    } else if from < n && me_c < n {
                        self.diplo.warmth[from * n + me_c] -= 0.05;
                    }
                }
            }
            (Kind::Flight, 1) => {
                self.set_fiscal(s, "tax", 0);
                self.fiscal.confidence[s] = (self.fiscal.confidence[s] + 0.15).min(1.3);
                let pack = self.pack.clone();
                let n = self.h.home_mut(s);
                for g in 0..pack.groups.len() {
                    if crate::country::policy::role(&pack, g) < 2 {
                        n.discontent[g] += 0.1;
                    }
                }
            }
            (Kind::PeaceOffer, 0) => {
                // (the enemy asked: no need to ask it again)
                let line = format!("мир заключён: {} и {} остаются при том, что держат", self.h.home(0).name, self.h.home(1).name);
                self.end_war();
                self.news.push(Note::new(self.month(), &line, true, "politics"));
            }
            _ => {}
        }
    }
}
