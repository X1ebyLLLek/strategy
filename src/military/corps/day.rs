//! A day of the war of divisions: what each side sees, the generals' orders, the march, the
//! fights, supply and hunger, refill.

use super::*;

impl Campaign {
    /// The worth of each side's divisions in each province, as side `s` sees the enemy's: on the
    /// spot where its own divisions stand next to them, else through fog (the same mist all month),
    /// less what the enemy's generals keep hidden of the blows they gather.
    pub(super) fn seen_divs(&self, pack: &Pack, c: &Corps, s: usize) -> Vec<f32> {
        let n = self.owner.len();
        let o = 1 - s;
        let mut truth = vec![0.0f32; n];
        let mut near = vec![false; n];
        for d in &c.divs {
            if d.side == o {
                truth[d.at] += Corps::power(pack, d) * d.org.max(0.2);
            } else {
                near[d.at] = true;
                let deep = c.gift(pack, d).see >= 1.0;
                for &r in &self.th.nbr[d.at] {
                    near[r] = true;
                    if deep {
                        for &x in &self.th.nbr[r] {
                            near[x] = true;
                        }
                    }
                }
            }
        }
        // the blows the enemy's generals gather, hidden as well as they can
        let mut hidden = vec![0.0f32; n];
        if let Some(gs) = &self.command[o] {
            for b in &c.blows[o] {
                let talent = gs.get(b.group).and_then(|g| g.general.as_ref()).map_or(0.0, |x| x.talent);
                for &id in &b.ids {
                    if let Some(i) = c.find(id) {
                        let d = &c.divs[i];
                        if b.froms.contains(&d.at) {
                            hidden[d.at] += Corps::power(pack, d) * crate::military::front::HIDE * talent;
                        }
                    }
                }
            }
        }
        (0..n)
            .map(|q| {
                if self.owner[q] != o || truth[q] <= 0.0 {
                    return 0.0;
                }
                let t = (truth[q] - hidden[q]).max(0.0);
                if near[q] {
                    t
                } else {
                    let mut r = Rng::new(hash64(self.seed ^ 0xF06 ^ ((self.month as u64) << 20) ^ ((q as u64) << 2) ^ s as u64));
                    t * (self.fog[s] * r.normal()).exp()
                }
            })
            .collect()
    }

    /// A day of the war of divisions.
    pub fn day(&mut self, pack: &Pack) {
        if self.fallen.is_some() {
            return;
        }
        let Some(mut c) = self.corps.take() else { return };
        c.day += 1;
        let seed = hash64(self.seed ^ 0xD1F ^ ((self.month as u64) << 16) ^ c.day as u64);
        let mut rng = Rng::new(seed);
        let n = self.owner.len();
        // the way of supply, and how many each province must feed
        c.level = [self.supply(0), self.supply(1)];
        let reach = [self.reach(0), self.reach(1)];
        let gifts: [Vec<Gift>; 2] = std::array::from_fn(|x| c.shapes[x].iter().map(|sh| sh.gift(pack)).collect());
        let gift_of = |d: &Division| gifts[d.side].get(d.shape).copied().unwrap_or(Gift::NONE);
        // (a division marching on over its own land carries rations for the road: it takes half of
        // what a standing one takes from the province — else a mobilisation's columns passing a
        // railway junction starved the province)
        let mut eating = [vec![0.0f32; n], vec![0.0f32; n]];
        for d in &c.divs {
            let passing = d.way.first().is_some_and(|&q| self.owner[q] == d.side);
            eating[d.side][d.at] += Campaign::eats(pack, d) * gift_of(d).eat * if passing { MARCH_EAT } else { 1.0 };
        }
        for d in c.divs.iter_mut() {
            let p = d.at;
            let cap = self.cap_at(pack, d.side, p);
            let crowd = (cap / eating[d.side][p].max(1e-6)).min(1.0);
            d.supplied = if c.fed || c.peace { 1.0 } else { c.level[d.side].get(p).copied().unwrap_or(0.0) * crowd };
            c.tally[d.side][T_DAYS] += 1.0;
            if crowd < 0.8 {
                c.tally[d.side][T_CROWD] += 1.0;
            }
        }
        // the generals and the staffs give their orders
        let seen = [self.seen_divs(pack, &c, 0), self.seen_divs(pack, &c, 1)];
        for s in 0..2 {
            if !c.idle[s] {
                self.lead(pack, &mut c, s, &seen[s], &reach, seed);
            }
        }
        // the marches, and the fights they begin
        self.march(pack, &mut c, &mut rng);
        // a day of each fight
        let mut fighting = vec![false; c.divs.len()];
        self.fight(pack, &mut c, &mut rng, &reach, &mut fighting);
        // rest, digging in, drill, hunger, refilling
        let line: [Vec<bool>; 2] = std::array::from_fn(|s| {
            let mut v = vec![false; n];
            for q in self.front(s) {
                v[q] = true;
            }
            v
        });
        for (i, d) in c.divs.iter_mut().enumerate() {
            d.busy = fighting.get(i).copied().unwrap_or(false);
            if d.busy {
                continue;
            }
            let gift = gifts[d.side].get(d.shape).copied().unwrap_or(Gift::NONE);
            let moving = !d.way.is_empty();
            if d.rest > 0.0 {
                d.rest = (d.rest - 1.0).max(0.0);
            }
            let m = c.mods[d.side];
            let back = ORG_BACK * gift.org * (0.3 + 0.7 * d.supplied) * if moving { 0.5 } else { 1.0 };
            let back = if m.org_back == 1.0 { back } else { back * m.org_back };
            let top = if d.supplied < STARVE { 0.5 } else { 1.0 };
            d.org = (d.org + back).min(top).max(d.org.min(top));
            if !moving && d.rest <= 0.0 {
                let deep = self.deep[d.side].get(d.at).copied().unwrap_or(false);
                let pace = crate::military::front::DIG_PACE / 30.0 * gift.dig * if deep { crate::military::front::DIG_DEEP } else { 1.0 };
                let pace = if m.dig == 1.0 { pace } else { pace * m.dig };
                d.dug = (d.dug + pace).min(1.0);
            } else {
                d.dug = 0.0;
            }
            // recruits drill standing behind the line (their own land off the front, out of the
            // fights): sent out early, a division stays green while it is at the front
            if d.drill < 1.0 && !moving && self.owner[d.at] == d.side && !line[d.side][d.at] {
                d.drill = (d.drill + 1.0 / TRAIN_DAYS).min(1.0);
            }
            // (the men lost take their equipment with them)
            if d.supplied < STARVE {
                let k = 1.0 - ATTRITION * (1.0 - d.supplied / STARVE) * m.attrition;
                d.has.iter_mut().chain(d.kit.iter_mut()).for_each(|x| *x *= k);
            }
            if reach[d.side][d.at] == NO && !c.fed && !c.peace {
                let k = 1.0 - CUT_LOSS[(self.th.home[d.at] != d.side) as usize] * m.attrition;
                d.has.iter_mut().chain(d.kit.iter_mut()).for_each(|x| *x *= k);
            }
            // refilled from what the country made, where the way of supply reaches: men, then the
            // equipment its norm gives them
            if d.supplied >= 0.5 && (reach[d.side][d.at] != NO || c.peace) {
                let sh = &c.shapes[d.side][d.shape];
                for u in 0..d.has.len() {
                    let need = (sh.units[u] - d.has[u]).max(0.0);
                    let take = (need * REFILL).min(c.pool[d.side][u]);
                    if take > 0.0 {
                        d.has[u] += take;
                        c.pool[d.side][u] -= take;
                    }
                    let need = (sh.norm_of(u) * d.has[u] - d.kit[u]).max(0.0);
                    let take = (need * REFILL).min(c.kit[d.side][u]);
                    if take > 0.0 {
                        d.kit[u] += take;
                        c.kit[d.side][u] -= take;
                    }
                }
            }
        }
        // divisions with nothing left are gone
        c.divs.retain(|d| d.has.iter().sum::<f32>() > 1e-3);
        self.corps = Some(c);
        self.sync_divs(pack);
    }

    /// The marches: each division walks towards the next province of its way; into a province of
    /// the enemy where the enemy stands it does not walk but attacks; into one where nobody stands
    /// it walks, and takes it.
    fn march(&mut self, pack: &Pack, c: &mut Corps, rng: &mut Rng) {
        let n = self.owner.len();
        let mut count = [vec![0u32; n], vec![0u32; n]];
        for d in &c.divs {
            count[d.side][d.at] += 1;
        }
        for i in 0..c.divs.len() {
            let (s, at) = (c.divs[i].side, c.divs[i].at);
            if c.divs[i].rest > 0.0 {
                continue;
            }
            let Some(&next) = c.divs[i].way.first() else { continue };
            if next == at {
                c.divs[i].way.remove(0);
                continue;
            }
            if !self.th.nbr[at].contains(&next) {
                c.divs[i].way.clear();
                continue;
            }
            let o = 1 - s;
            if c.peace && self.owner[next] != s {
                c.divs[i].way.clear();
                continue;
            }
            if self.owner[next] != s && count[o][next] > 0 {
                // an attack: a fight there, if none yet
                if c.fight_at(next).is_none() {
                    let odds = c.odds_for.iter().find(|x| x.0 == next && x.1 == s).map_or(0.5, |x| x.2);
                    let luck = [(battle::BATTLE_LUCK * rng.normal()).exp(), (battle::BATTLE_LUCK * rng.normal()).exp()];
                    c.fights.push(Fight {
                        at: next,
                        side: s,
                        from: at,
                        days: 0,
                        luck,
                        held: [0.0; 2],
                        odds,
                        start: [0.0; 2],
                        lost: [0.0; 2],
                        kinds: [Vec::new(), Vec::new()],
                        lead: [1.0; 2],
                        dug: 0.0,
                        supply: [0.0; 2],
                        balance: 0.5,
                        divs: [0; 2],
                        kit: [1.0; 2],
                        exp: [0.0; 2],
                        companies: [0.0; 2],
                        sides: 1,
                        men: [0.0; 2],
                        captured: 0.0,
                        lost_kinds: [Vec::new(), Vec::new()],
                        tactic: [0, 4],
                        tactics: [[0; 8]; 2],
                    });
                }
                continue;
            }
            // (an enemy's attack on the province it stands in holds it there)
            if c.fights.iter().any(|f| f.at == at && f.side == o) {
                continue;
            }
            let mut days = self.step_days(pack, &c.divs[i].has, at, next);
            if rough(self.th.terrain[next]) {
                days *= c.gift(pack, &c.divs[i]).rough;
            }
            let d = &mut c.divs[i];
            d.gone += 1.0 / days.max(0.5);
            d.dug = 0.0;
            if d.gone >= 1.0 {
                d.gone = 0.0;
                d.at = next;
                d.way.remove(0);
                count[s][at] -= 1;
                count[s][next] += 1;
                if self.owner[next] != s {
                    self.take(s, next);
                }
            }
        }
        c.odds_for.clear();
    }

    /// Side `s` takes province `q`.
    pub(super) fn take(&mut self, s: usize, q: usize) {
        self.owner[q] = s;
        self.dug[0][q] = 0.0;
        self.dug[1][q] = 0.0;
        self.taken[self.th.terrain[q] as usize] += 1.0;
    }
}
