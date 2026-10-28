//! A month of the war on the map: the staff's orders, the battles, provinces taken, pockets,
//! supply and losses; who governs taken land; the war's end.

use super::*;

impl Campaign {
    /// A month of war: the staffs rethink what to build; the attacks prepared last month are
    /// fought; new units join the reserves; each staff places its army for next month.
    pub fn step(&mut self, pack: &Pack) {
        if self.fallen.is_some() {
            return;
        }
        if self.corps.is_some() {
            self.month_divs(pack);
            return;
        }
        self.month += 1;
        let seed = hash64(self.seed ^ self.month as u64);
        let mut rng = Rng::new(seed);
        let n = self.owner.len();
        let nu = pack.units.len();

        // what to build, for the ground the front now runs over
        let mut ground: Vec<(Terrain, f32)> = Vec::new();
        for s in 0..2 {
            for p in self.front(s) {
                match ground.iter_mut().find(|g| g.0 == self.th.terrain[p]) {
                    Some(g) => g.1 += 1.0,
                    None => ground.push((self.th.terrain[p], 1.0)),
                }
            }
        }
        if !ground.is_empty() {
            let [a, b] = &mut self.sides;
            rayon::join(|| a.rethink(pack, &ground, seed ^ 1), || b.rethink(pack, &ground, seed ^ 2));
        }

        // 1. the attacks prepared last month: groups against the same province join into one
        let supply = [self.supply(0), self.supply(1)];
        let reach = [self.reach(0), self.reach(1)];
        let mut plans: Vec<(usize, usize, usize, Vec<f32>, f32)> = Vec::new();
        let mut plan_def: Vec<f32> = Vec::new();
        let defs = std::mem::take(&mut self.staged_def);
        for (i, (s, from, q, g, seen)) in std::mem::take(&mut self.staged).into_iter().enumerate() {
            match plans.iter_mut().find(|x| x.0 == s && x.2 == q) {
                Some(x) => add(&mut x.3, &g, 1.0),
                None => {
                    plans.push((s, from, q, g, seen));
                    plan_def.push(defs.get(i).copied().unwrap_or(0.0));
                }
            }
        }
        let mut attacked = [0usize; 2];
        for pl in &plans {
            attacked[1 - pl.0] += 1;
        }
        let help: [Vec<f32>; 2] = std::array::from_fn(|o| {
            if attacked[o] > 0 { self.reserve[o].iter().map(|x| x / attacked[o] as f32).collect() } else { vec![0.0; nu] }
        });
        for i in (1..plans.len()).rev() {
            let j = rng.below(i + 1);
            plans.swap(i, j);
            plan_def.swap(i, j);
        }
        for s in 0..2 {
            for p in self.front(s) {
                self.exposed[self.th.terrain[p] as usize] += 1.0;
            }
        }
        // troops that fought this month stay where the battle left them next month, to rest
        let mut tired: [Vec<Vec<f32>>; 2] = [vec![vec![0.0f32; nu]; n], vec![vec![0.0f32; nu]; n]];
        // a deep defence: each group's second line comes up to the provinces of its first line
        // attacked this month, an equal part to each
        let mut deep_help: [Vec<Vec<f32>>; 2] = [Vec::new(), Vec::new()];
        for o in 0..2 {
            if !self.deep[o].iter().any(|&x| x) {
                continue;
            }
            let lab = self.group_of(o);
            let k = self.command[o].as_ref().map_or(0, |g| g.len());
            let mut hit: Vec<Vec<usize>> = vec![Vec::new(); k];
            for pl in &plans {
                let q = pl.2;
                if pl.0 != o && self.owner[q] == o && self.deep[o][q] && lab[q] < k && !hit[lab[q]].contains(&q) {
                    hit[lab[q]].push(q);
                }
            }
            let mut up: Vec<Vec<f32>> = vec![vec![0.0; nu]; k];
            for r in 0..n {
                let g = lab[r];
                if self.deep[o][r] && self.owner[r] == o && g < k && !hit[g].is_empty() && !self.th.nbr[r].iter().any(|&x| self.owner[x] != o) {
                    let go: Vec<f32> = (0..nu).map(|u| self.troops[o][r][u].max(0.0) * DEEP_HELP).collect();
                    add(&mut self.troops[o][r], &go, -1.0);
                    add(&mut up[g], &go, 1.0);
                }
            }
            deep_help[o] = vec![vec![0.0; nu]; n];
            for g in 0..k {
                for &q in &hit[g] {
                    let share = 1.0 / hit[g].len() as f32;
                    add(&mut deep_help[o][q], &up[g], share);
                }
            }
        }
        for ((s, from, q, group, odds), def_then) in plans.into_iter().zip(plan_def) {
            let o = 1 - s;
            if self.owner[from] != s || self.owner[q] != o {
                continue;
            }
            // the group leaves its province for the attack
            let g: Vec<f32> = (0..nu).map(|u| group[u].min(self.troops[s][from][u])).collect();
            add(&mut self.troops[s][from], &g, -1.0);
            // a deep defence: its second line comes up
            if !deep_help[o].is_empty() {
                let h = std::mem::take(&mut deep_help[o][q]);
                if !h.is_empty() {
                    add(&mut self.troops[o][q], &h, 1.0);
                }
            }
            // the defender's reserve comes up, if the province can be reached from its capital
            if reach[o][q] != NO {
                add(&mut self.troops[o][q], &help[o], 1.0);
                let h = help[o].clone();
                add(&mut self.reserve[o], &h, -1.0);
                self.reserve[o].iter_mut().for_each(|x| *x = x.max(0.0));
            }
            let mut att = Army { supplied: supply[s][from], ..self.sides[s].real(g.clone()) };
            let mut def = Army { supplied: supply[o][q], dug: self.dug[o][q], ..self.sides[o].real(self.troops[o][q].clone()) };
            // the generals who lead them: the troops fight as well as they are led
            let mut lead = [1.0f32; 2];
            for (i, (side, at, army)) in [(s, from, &mut att), (o, q, &mut def)].into_iter().enumerate() {
                let k = self.general_at(side, at).map_or(1.0, |x| x.lead());
                lead[i] = k;
                army.fire.iter_mut().for_each(|f| *f *= k);
                army.tough.iter_mut().for_each(|t| *t *= k);
            }
            let kinds = [g.clone(), self.troops[o][q].clone()];
            let dug_then = self.dug[o][q];
            let ratio = Campaign::worth(pack, &g) / Campaign::worth(pack, &self.troops[o][q]).max(1e-6);
            let out = battle(pack, self.th.terrain[q], [&att, &def], Kind::Attack, &mut rng, false);
            let won = out.winner == Some(0);
            let lost = [
                Campaign::worth(pack, &g) - Campaign::worth(pack, &out.left[0].count),
                Campaign::worth(pack, &self.troops[o][q]) - Campaign::worth(pack, &out.left[1].count),
            ];
            // the generals of the groups that fought it: their record, their experience
            for (side, at, w) in [(s, from, won), (o, q, !won)] {
                if self.command[side].is_some() {
                    let lab = self.group_of(side);
                    if let Some(gen) = self.command[side].as_mut().and_then(|gs| gs.get_mut(lab[at])).and_then(|g| g.general.as_mut()) {
                        if w {
                            gen.won += 1;
                        } else {
                            gen.lost += 1;
                        }
                        if w && side == s {
                            gen.took += 1;
                        }
                        gen.experience += SEASON * (1.0 - gen.experience);
                    }
                }
            }
            self.fought.push(Fought { month: self.month, attacker: s, province: q, from, terrain: self.th.terrain[q], ratio, odds, won, supply: [att.supplied, def.supplied], lost, cut: reach[o][q] == NO, plan: [Campaign::worth(pack, &group), def_then, Campaign::worth(pack, &g), Campaign::worth(pack, &def.count)], kinds, lead, dug: dug_then, days: out.days, divs: [0; 2], kit: [1.0; 2], exp: [0.0; 2], companies: [0.0; 2], sides: 1, men: [0.0; 2], captured: 0.0, lost_kinds: [Vec::new(), Vec::new()], tactics: [[0; 8]; 2] });
            // the staff compares what its games promised with what happened
            let w = &mut self.sides[s].wariness;
            *w = (*w + WARY * (odds - if won { 1.0 } else { 0.0 })).clamp(-1.0, 2.0);
            for side in 0..2 {
                self.sides[side].battles += 1;
            }
            self.sides[if won { s } else { o }].battles_won += 1;
            for (me, ours, theirs, w) in [(s, &att, &def, won), (o, &def, &att, !won)] {
                let [a, b] = &mut self.sides;
                let (me, foe) = if me == 0 { (a, &*b) } else { (b, &*a) };
                me.learn(pack, ours, theirs, foe, w, &mut rng);
            }
            if won {
                self.owner[q] = s;
                self.dug[o][q] = 0.0;
                self.dug[s][q] = 0.0;
                self.taken[self.th.terrain[q] as usize] += 1.0;
                add(&mut self.troops[s][q], &out.left[0].count, 1.0);
                add(&mut tired[s][q], &out.left[0].count, 1.0);
                // the defenders fall back to their strongest neighbouring province, or are lost
                let back = self.th.nbr[q].iter().copied().filter(|&r| self.owner[r] == o).max_by(|&a, &b| {
                    Campaign::worth(pack, &self.troops[o][a]).total_cmp(&Campaign::worth(pack, &self.troops[o][b]))
                });
                self.troops[o][q].iter_mut().for_each(|x| *x = 0.0);
                if let Some(r) = back {
                    add(&mut self.troops[o][r], &out.left[1].count, 1.0);
                    add(&mut tired[o][r], &out.left[1].count, 1.0);
                }
            } else {
                add(&mut self.troops[s][from], &out.left[0].count, 1.0);
                add(&mut tired[s][from], &out.left[0].count, 1.0);
                self.troops[o][q] = out.left[1].count.clone();
                tired[o][q] = out.left[1].count.clone();
            }
        }
        self.govern();
        if self.fallen.is_some() {
            return;
        }

        // 2. new units join the reserve
        for s in 0..2 {
            let made = match self.made[s].take() {
                Some(m) => {
                    add(&mut self.sides[s].army.count, &m, 1.0);
                    m
                }
                None => {
                    let before = self.sides[s].army.count.clone();
                    self.sides[s].produce();
                    (0..nu).map(|u| self.sides[s].army.count[u] - before[u]).collect()
                }
            };
            add(&mut self.reserve[s], &made, 1.0);
        }

        // 3. each staff places its army for next month: all it can move (what is not cut off
        // from the capital) is gathered; half holds the front, each stretch by the enemy it sees
        // across it — gathered strikes included; groups are placed next to the provinces its
        // war games say it can take, the smallest group that gives the odds first; the rest
        // stays back as the reserve
        let supply = [self.supply(0), self.supply(1)];
        let reach = [self.reach(0), self.reach(1)];
        let seen: [Vec<f32>; 2] = std::array::from_fn(|s| {
            let o = 1 - s;
            (0..n).map(|q| if self.owner[q] == o { Campaign::worth(pack, &self.troops[o][q]) * (self.fog[s] * rng.normal()).exp() } else { 0.0 }).collect()
        });
        self.hidden = [vec![0.0; n], vec![0.0; n]];
        // what is truly there (the generals see it on the spot, each with his own error)
        let truth: [Vec<f32>; 2] = std::array::from_fn(|s| {
            let o = 1 - s;
            (0..n).map(|q| if self.owner[q] == o { Campaign::worth(pack, &self.troops[o][q]) } else { 0.0 }).collect()
        });
        for s in 0..2 {
            let o = 1 - s;
            let moving: Vec<usize> = self.front(s).into_iter().filter(|&p| reach[s][p] != NO).collect();
            if moving.is_empty() {
                continue;
            }
            let mut pool = std::mem::replace(&mut self.reserve[s], vec![0.0; nu]);
            for p in 0..n {
                if reach[s][p] != NO {
                    // the tired stay; the rest can be moved
                    let t = std::mem::replace(&mut self.troops[s][p], vec![0.0; nu]);
                    let stay: Vec<f32> = (0..nu).map(|u| tired[s][p][u].min(t[u])).collect();
                    let go: Vec<f32> = (0..nu).map(|u| t[u] - stay[u]).collect();
                    self.troops[s][p] = stay;
                    add(&mut pool, &go, 1.0);
                }
            }
            if self.command[s].is_some() {
                let planned = if self.staff_led[s] { self.staff_orders(pack, s, &pool, &seen[s], &supply, &reach, seed) } else { Vec::new() };
                self.place_groups(pack, s, pool, &seen[s], &truth[s], &supply, &reach, seed, &planned);
                continue;
            }
            let threat: Vec<f32> = moving
                .iter()
                .map(|&p| {
                    self.th.nbr[p]
                        .iter()
                        .filter(|&&q| self.owner[q] == o)
                        .map(|&q| seen[s][q] / self.th.nbr[q].iter().filter(|&&r| self.owner[r] == s).count().max(1) as f32)
                        .sum::<f32>()
                })
                .collect();
            let all: f32 = threat.iter().sum();
            for (i, &p) in moving.iter().enumerate() {
                let share = if all > 0.0 { 0.8 * threat[i] / all + 0.2 / moving.len() as f32 } else { 1.0 / moving.len() as f32 };
                add(&mut self.troops[s][p], &pool, HOLD * share);
            }
            let strike: Vec<f32> = pool.iter().map(|x| x * (1.0 - HOLD)).collect();
            let mut scored: Vec<(usize, usize, f32)> = Vec::new();
            for q in 0..n {
                if self.owner[q] != o {
                    continue;
                }
                let Some(from) = self.th.nbr[q].iter().copied().filter(|&r| self.owner[r] == s && reach[s][r] != NO).max_by(|&a, &b| supply[s][a].total_cmp(&supply[s][b])) else {
                    continue;
                };
                scored.push((q, from, self.odds(pack, s, q, &strike, supply[s][from], seen[s][q], supply[o][q], seed ^ q as u64)));
            }
            scored.sort_by(|a, b| b.2.total_cmp(&a.2));
            let mut left = 1.0f32;
            for &(q, from, best) in &scored {
                if best < ODDS || left <= 0.01 {
                    break;
                }
                let mut part = 0.0;
                let mut promised = 0.0;
                for &f in SPLITS.iter().rev() {
                    if f > left + 1e-6 {
                        continue;
                    }
                    let g: Vec<f32> = strike.iter().map(|x| x * f).collect();
                    let got = self.odds(pack, s, q, &g, supply[s][from], seen[s][q], supply[o][q], seed ^ q as u64 ^ 7);
                    if got >= ODDS {
                        part = f;
                        promised = got;
                        break;
                    }
                }
                if part == 0.0 {
                    continue;
                }
                left -= part;
                let g: Vec<f32> = strike.iter().map(|x| x * part).collect();
                add(&mut self.troops[s][from], &g, 1.0);
                self.staged.push((s, from, q, g, promised));
                self.staged_def.push(Campaign::worth(pack, &self.troops[o][q]));
            }
            self.reserve[s] = strike.iter().map(|x| x * left.max(0.0)).collect();
        }

        // each staff sees, through fog, the strikes the enemy has gathered across its front, and
        // sends part of its reserve to meet them: where the enemy it sees outweighs the troops
        // it has there (without it the gathering was never seen, attacks met no more than the
        // spread-out front, and provinces changed hands every half year)
        for s in 0..2 {
            let o = 1 - s;
            let mine: Vec<usize> = self.front(s).into_iter().filter(|&p| reach[s][p] != NO).collect();
            if mine.is_empty() {
                continue;
            }
            let gap: Vec<f32> = mine
                .iter()
                .map(|&p| {
                    let foe: f32 = self.th.nbr[p]
                        .iter()
                        .filter(|&&q| self.owner[q] == o)
                        .map(|&q| (Campaign::worth(pack, &self.troops[o][q]) - self.hidden[o][q]).max(0.0) * (self.fog[s] * rng.normal()).exp() / self.th.nbr[q].iter().filter(|&&r| self.owner[r] == s).count().max(1) as f32)
                        .sum();
                    (foe - Campaign::worth(pack, &self.troops[s][p])).max(0.0)
                })
                .collect();
            let all: f32 = gap.iter().sum();
            if all <= 0.0 {
                continue;
            }
            let sent: Vec<f32> = self.reserve[s].iter().map(|x| x * REACT).collect();
            for (i, &p) in mine.iter().enumerate() {
                add(&mut self.troops[s][p], &sent, gap[i] / all);
            }
            add(&mut self.reserve[s], &sent, -1.0);
        }

        // troops cut off from the capital melt away
        let reach_now = [self.reach(0), self.reach(1)];
        for s in 0..2 {
            for p in 0..n {
                if reach_now[s][p] == NO && self.owner[p] == s {
                    let k = 1.0 - if self.pockets { CUT_OFF_LOSS } else { OLD_CUT_OFF_LOSS }[if self.th.home[p] == s { 0 } else { 1 }];
                    self.troops[s][p].iter_mut().for_each(|x| *x *= k);
                }
            }
        }

        // pockets give up: a province cut off from its side's capital, its garrison weak against
        // the enemy around it (or none at all), goes over to the enemy
        for o in 0..2 {
            let s = 1 - o;
            for p in 0..n {
                if !self.pockets || self.owner[p] != o || reach_now[o][p] != NO {
                    continue;
                }
                let around: f32 = self.th.nbr[p].iter().filter(|&&r| self.owner[r] == s).map(|&r| Campaign::worth(pack, &self.troops[s][r])).sum();
                if around > 0.0 && Campaign::worth(pack, &self.troops[o][p]) < SURRENDER * around {
                    self.owner[p] = s;
                    self.troops[o][p].iter_mut().for_each(|x| *x = 0.0);
                    self.dug[o][p] = 0.0;
                    self.dug[s][p] = 0.0;
                    self.surrendered[o] += 1;
                }
            }
        }
        for s in 0..2 {
            self.cut_off[s] += (0..n).filter(|&p| self.owner[p] == s && reach_now[s][p] == NO).count() as f32;
        }

        // troops that stay where they are dig in, month by month
        for s in 0..2 {
            for p in 0..n {
                if self.owner[p] == s && self.troops[s][p].iter().sum::<f32>() > 0.05 {
                    self.dug[s][p] = (self.dug[s][p] + DIG_PACE * if self.deep[s][p] { DIG_DEEP } else { 1.0 }).min(1.0);
                } else {
                    self.dug[s][p] = 0.0;
                }
            }
        }

        // 4. the staffs' books: their armies are what stands in the provinces and in reserve
        for s in 0..2 {
            let mut total = self.reserve[s].clone();
            for p in 0..n {
                if self.owner[p] != s {
                    self.troops[s][p].iter_mut().for_each(|x| *x = 0.0);
                }
                let t = self.troops[s][p].clone();
                add(&mut total, &t, 1.0);
            }
            self.sides[s].army.count = total;
            self.sides[s].age();
        }
    }

    /// A capital taken: the government moves to the most populous province the country still
    /// holds of its own. A country holding less than `CAPITULATE` of its people gives up.
    pub(crate) fn govern(&mut self) {
        for s in 0..2 {
            let mine: Vec<usize> = (0..self.owner.len()).filter(|&p| self.owner[p] == s && self.th.home[p] == s).collect();
            let had: f32 = (0..self.owner.len()).filter(|&p| self.th.home[p] == s).map(|p| self.th.pop[p]).sum();
            let has: f32 = mine.iter().map(|&p| self.th.pop[p]).sum();
            if has < CAPITULATE * had || mine.is_empty() {
                self.fallen = Some(s);
                return;
            }
            if self.owner[self.th.capital[s]] != s {
                let from = self.th.capital[s];
                self.th.capital[s] = *mine.iter().max_by(|&&a, &&b| self.th.pop[a].total_cmp(&self.th.pop[b])).unwrap();
                self.log.push(format!("мес {}: столица {} пала, правительство переехало", self.month, from));
            }
        }
    }

    /// The war is over: the troops go home (they stay in each side's army, off the map), the
    /// plans of attack and the groups of armies are dropped, the trenches filled in; the land each
    /// side holds is its own from now on.
    pub fn end(&mut self) {
        let n = self.owner.len();
        let nu = self.reserve[0].len();
        self.fallen = None;
        self.staged.clear();
        self.staged_def.clear();
        self.log.clear();
        for s in 0..2 {
            self.troops[s] = vec![vec![0.0; nu]; n];
            self.reserve[s] = vec![0.0; nu];
            self.dug[s] = vec![0.0; n];
            self.deep[s] = vec![false; n];
            self.hidden[s] = vec![0.0; n];
        }
        self.command = [None, None];
        self.staff_led = [false; 2];
        self.th.home = self.owner.clone();
        // (the divisions go home: each side's army is what they had and its pool; where the game
        // keeps its armies in divisions in peace, they stay, back under their staffs)
        if let Some(mut c) = self.corps.take() {
            if c.keep {
                c.peace = true;
                c.fights.clear();
                c.blows = [Vec::new(), Vec::new()];
                for d in c.divs.iter_mut() {
                    d.group = crate::military::corps::NONE;
                    d.manual = false;
                    d.way.clear();
                    d.gone = 0.0;
                    d.rest = 0.0;
                }
            }
            for s in 0..2 {
                let mut total = c.pool[s].clone();
                for d in c.divs.iter().filter(|d| d.side == s) {
                    for (t, x) in total.iter_mut().zip(&d.has) {
                        *t += x;
                    }
                }
                self.sides[s].army.count = total;
            }
            if c.keep {
                self.corps = Some(c);
            }
        }
    }
}
