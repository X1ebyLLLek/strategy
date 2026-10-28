//! The neighbours and the rival: the countries whose land touches the player's as it is now, and
//! the one his government turns against. In peace (no claim begun, no war, no event waiting) the
//! player may turn on another neighbour: the theatre of a coming war is laid anew between his land
//! as he holds it and that country's; his army, staff, depots and lines of supply go with him; the
//! new rival brings its own staff and generals from the world's past, an economist and laboratories
//! of its own, and an army as large, for its economy, as the old rival kept.

use strategy::economy::advisor::{Economist, Mind};
use strategy::economy::science::{Lab, Scientist};
use strategy::military::front::{Campaign, General, Theatre, NO};
use strategy::rng::{hash64, Rng};
use strategy::world::{names, render};

use crate::game::Note;
use crate::Game;

impl Game {
    /// The countries whose land touches the player's (map countries), the rival among them.
    pub fn neighbours(&self) -> Vec<usize> {
        let me = self.me;
        let war = &self.h.war;
        let mut out: Vec<usize> = Vec::new();
        for (q, &p) in war.th.provs.iter().enumerate() {
            if war.owner[q] != me {
                continue;
            }
            for &r in &self.world.provinces[p].neighbors {
                let r = r as usize;
                let c = match war.th.at[r] {
                    t if t != NO => war.th.countries[war.owner[t as usize]],
                    _ => self.country_of[r] as usize,
                };
                if c != usize::MAX && c != war.th.countries[me] && c < self.countries.len() && !out.contains(&c) {
                    out.push(c);
                }
            }
        }
        out.sort();
        out
    }

    /// Why the player may not turn on another neighbour now (None: he may).
    pub fn rival_blocked(&self) -> Option<&'static str> {
        if self.h.at_war {
            Some("идёт война")
        } else if self.state.claim[self.me].is_some() {
            Some("цель войны уже обосновывается")
        } else if self.state.claim[1 - self.me].is_some() {
            Some("соперник готовит войну против нас")
        } else if !self.ev.waiting.is_empty() {
            Some("ждёт ответа событие")
        } else {
            None
        }
    }

    /// The player's government turns on country `c` (a neighbour): the theatre is laid anew;
    /// whether it was done.
    pub fn set_rival(&mut self, c: usize) -> bool {
        let me = self.me;
        let foe = 1 - me;
        let now = self.month();
        if self.rival_blocked().is_some() || c == self.h.war.th.countries[foe] || !self.neighbours().contains(&c) {
            return false;
        }
        let old = &self.h.war;
        let mine = old.th.countries[me];
        // what each holds of the old theatre is its own on the map now
        for (q, &p) in old.th.provs.iter().enumerate() {
            self.country_of[p] = old.th.countries[old.owner[q]] as u32;
        }
        let my_land: Vec<usize> = (0..old.th.provs.len()).filter(|&q| old.owner[q] == me).map(|q| old.th.provs[q]).collect();
        let their_land: Vec<usize> = (0..self.world.provinces.len()).filter(|&p| !self.world.provinces[p].sea && self.country_of[p] as usize == c).collect();
        if their_land.is_empty() || my_land.is_empty() {
            return false;
        }
        let my_cap = old.th.provs[old.th.capital[me]];
        let mut lands: [&[usize]; 2] = [&my_land, &their_land];
        let mut caps = [my_cap, self.capitals.get(c).copied().unwrap_or(usize::MAX)];
        let mut countries = [mine, c];
        if me == 1 {
            lands.swap(0, 1);
            caps.swap(0, 1);
            countries.swap(0, 1);
        }
        let th = Theatre::of_lands(&self.world, lands, &self.prov_people, caps, countries);
        // the sides: the player's staff and army as they are; the rival's staff as its past left
        // it, its army as large, for its economy, as the old rival's was for its own
        let old_foe = self.h.at[foe];
        let scale = self.h.world.nations[c].gdp_real / self.h.world.nations[old_foe].gdp_real.max(1e-9);
        let mut sides = [old.sides[0].clone(), old.sides[1].clone()];
        let mut theirs = self.staffs.get(c).cloned().unwrap_or_else(|| old.sides[foe].clone());
        theirs.army.count = old.sides[foe].army.count.iter().map(|x| x * scale.clamp(0.1, 10.0)).collect();
        sides[foe] = theirs;
        let mut camp = Campaign::raised(&self.pack, th, sides, hash64(self.seed ^ 0x41A1 ^ (c as u64) << 8 ^ now as u64));
        // the player's depots and lines go with him (by the map's provinces)
        let at_new = camp.th.at.clone();
        let to_new = |q: usize| -> Option<usize> {
            let p = old.th.provs[q];
            let t = at_new[p];
            (t != NO).then_some(t as usize)
        };
        camp.hubs[me] = old.hubs[me].iter().filter_map(|&q| to_new(q)).collect();
        let mut works: Vec<crate::world::statecraft::Works> = Vec::new();
        for w in self.state.works.iter().filter(|w| w.side == me) {
            let path: Vec<usize> = w.path.iter().filter_map(|&q| to_new(q)).collect();
            if path.len() >= 2 {
                works.push(crate::world::statecraft::Works { side: me, rail: w.rail, done: w.done.min(path.len()), path, short: false });
            }
        }
        camp.grade = crate::world::land::rail_grades(&self.world, &camp.th, &self.land);
        for w in &works {
            let g = if w.rail { crate::world::land::RAIL_GRADE } else { crate::world::land::ROAD_GRADE };
            if camp.grade.len() != camp.owner.len() {
                camp.grade = vec![1.0; camp.owner.len()];
            }
            for &q in &w.path[..w.done] {
                camp.grade[q] = camp.grade[q].min(g);
            }
        }
        camp.staff_led = old.staff_led;
        // the rival's people at the head of its economy and laboratories
        let mut rp = Rng::new(hash64(self.seed ^ 0xF0E ^ (c as u64) << 12 ^ now as u64));
        self.foe_adviser = names::person(&mut rp);
        let mind = Mind::anyone(&mut rp);
        self.h.econ[foe] = Some(Economist::new(mind, hash64(self.seed ^ 0xB07 ^ c as u64), &self.pack));
        self.foe_scholar = names::person(&mut rp);
        if self.pack.science.is_some() {
            let mut lab = Lab::new(&self.pack, hash64(self.seed ^ 0x1AB ^ (c as u64) << 4));
            lab.made.iter_mut().for_each(|x| *x = 0);
            self.h.labs[foe] = Some(lab);
            self.h.scientists[foe] = Some(Scientist::new(Mind::anyone(&mut rp), hash64(self.seed ^ 0x5C7 ^ c as u64), &self.pack));
            if let Some(sc) = &self.pack.science {
                self.h.world.nations[c].science_share = sc.share;
            }
        }
        self.h.at[foe] = c;
        self.h.army_share[foe] = None;

        // (an embargo was on the old rival: its ways open again)
        if self.fiscal.cut {
            let pack = self.pack.clone();
            self.h.world.reopen(&pack, self.h.at[me], old_foe);
            self.fiscal.cut = false;
        }
        self.fiscal.embargo = [false; 2];
        let old_corps = self.h.war.corps.clone();
        let old_provs = self.h.war.th.provs.clone();
        camp.most_groups = crate::game::MOST_GROUPS;
        camp.scale = crate::prov_scale();
        camp.depth_k = crate::game::DEPTH_K * crate::prov_scale();
        camp.most_battalions = crate::game::MOST_BATTALIONS;
        camp.line_first = true;
        camp.most_per_front = crate::game::MOST_PER_FRONT * crate::prov_scale();
        self.h.war = camp;
        // its own people at the head of its economy and laboratories; its government's course is
        // its own (reviewed at once)
        self.foe_people();
        if let Some(old) = self.courses.now[foe].take() {
            self.courses.past[foe].push(old);
        }
        // the armies in divisions: the rival's formed anew; ours as they stood (by the map's
        // provinces; where the new theatre has none, at the capital)
        self.muster();
        if let (Some(oc), Some(nc)) = (old_corps, self.h.war.corps.as_mut()) {
            let shift = oc.next;
            for d in nc.divs.iter_mut() {
                d.id += shift;
            }
            nc.next += shift;
            nc.divs.retain(|d| d.side != me);
            let cap = self.h.war.th.capital[me];
            for mut d in oc.divs.into_iter().filter(|d| d.side == me) {
                let t = old_provs.get(d.at).and_then(|&p| self.h.war.th.at.get(p).copied()).unwrap_or(NO);
                d.at = if t != NO && self.h.war.owner[t as usize] == me { t as usize } else { cap };
                d.way.clear();
                d.gone = 0.0;
                d.group = strategy::military::corps::NONE;
                nc.divs.push(d);
            }
            nc.shapes[me] = oc.shapes[me].clone();
            nc.wants[me] = oc.wants[me].clone();
            nc.pool[me] = oc.pool[me].clone();
            nc.kit[me] = oc.kit[me].clone();
            nc.numbers[me] = oc.numbers[me];
            nc.by_plan[me] = oc.by_plan[me];
            nc.lack[me] = oc.lack[me];
            nc.reorg[me] = oc.reorg[me];
        }
        let pack = self.pack.clone();
        self.h.war.sync_divs(&pack);
        self.h.fed = [1.0; 2];
        self.h.dead = [0.0; 2];
        let nu = self.pack.units.len();
        self.h.made = [vec![0.0; nu], vec![0.0; nu]];
        self.h.last_made[foe] = vec![0.0; nu];
        self.frame = render::frame(&self.world, &self.h.war.th);
        self.border = (self.h.war.front(0).len() + self.h.war.front(1).len()) as f32 / 2.0 * crate::prov_scale();
        self.gdp0[foe] = self.h.home(foe).gdp_real;
        self.veterans[foe] = self.past_veterans.get(c).cloned().unwrap_or_default();
        self.lost[foe] = self.past_lost.get(c).cloned().unwrap_or_default();
        // a grudge: the other holds land the past took from it (by the regions' holders now)
        let holds = |a: usize, b: usize, g: &Game| g.past_lost.get(a).is_some_and(|l| l.iter().any(|&(st, _)| (0..g.prov_state.len()).any(|p| g.prov_state[p] as usize == st && g.country_of[p] as usize == b)));
        let (a, b) = (self.h.war.th.countries[0], self.h.war.th.countries[1]);
        self.state.grudge = [holds(a, b, self), holds(b, a, self)];
        self.state.claim = [None, None];
        self.state.building.clear();
        self.state.works = works;
        self.state.stability[foe] = 0.8;
        self.state.mods[foe] = 0.0;
        // (the army in groups of armies along the border with the new rival: the old groups' generals
        // lead them)
        let generals: Vec<General> = self.h.war.command[me].take().map_or(Vec::new(), |gs| gs.into_iter().filter_map(|g| g.general).collect());
        self.peace_groups();
        if let Some(gs) = self.h.war.command[me].as_mut() {
            for (g, x) in gs.iter_mut().zip(generals) {
                g.general = Some(x);
            }
        }
        let name = self.countries[c].0.clone();
        self.news.push(Note::new(now, &format!("правительство видит теперь главного соперника в стране {name}"), true, "politics"));
        self.stamp += 1;
        self.version += 1;
        true
    }
}
