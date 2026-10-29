//! The staff's month: new divisions by the templates and the purse, the staff's own pattern
//! of division (its size and kit), the army in peace and its mobilisation, the groups
//! regrouped, the army's reckoning.

use super::*;

impl Campaign {
    /// A month of the war of divisions: the staffs rethink what to build; what the economy made
    /// goes to the pool, and whole divisions are formed from it by the templates each side builds;
    /// each staff gives its groups their shares and orders and moves divisions between them to
    /// match; a capital taken moves, a country holding too little of its people gives up; the
    /// books: each side's army is its divisions and its pool.
    pub(crate) fn month_divs(&mut self, pack: &Pack) {
        self.month += 1;
        let seed = hash64(self.seed ^ self.month as u64);
        let nu = pack.units.len();
        let mut ground: Vec<(Terrain, f32)> = Vec::new();
        for s in 0..2 {
            for p in self.front(s) {
                self.exposed[self.th.terrain[p] as usize] += 1.0;
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
        let Some(mut c) = self.corps.take() else { return };
        // what the economy made (or, without one, what the staff's budget makes)
        for s in 0..2 {
            let made = match self.made[s].take() {
                Some(m) => m,
                None => {
                    let before = self.sides[s].army.count.clone();
                    self.sides[s].produce();
                    (0..nu).map(|u| self.sides[s].army.count[u] - before[u]).collect()
                }
            };
            for u in 0..nu {
                c.pool[s][u] += made[u].max(0.0);
                c.kit[s][u] += made[u].max(0.0) * self.sides[s].norm_of(u);
            }
            c.made[s] = made.iter().map(|x| x.max(0.0)).sum();
            // what the staff saw it lacked this month; its pattern; new divisions
            Campaign::reckon(&mut c, s);
            c.lack[s][0] = self.rough_share(s);
            self.staff_pattern(pack, &mut c, s, &ground, seed);
            self.form(pack, &mut c, s);
        }
        // the staffs' shares and orders, and divisions moved between groups to match
        let reach = [self.reach(0), self.reach(1)];
        let supply = [self.supply(0), self.supply(1)];
        for s in 0..2 {
            if !self.staff_led[s] || self.command[s].is_none() {
                continue;
            }
            let seen = self.seen_divs(pack, &c, s);
            let mut units = vec![0.0f32; nu];
            for d in c.divs.iter().filter(|d| d.side == s) {
                for u in 0..nu {
                    units[u] += d.has[u];
                }
            }
            self.staff_orders(pack, s, &units, &seen, &supply, &reach, seed);
            self.regroup(&mut c, s);
            // (as many new armies as the reserve gives)
            for k in 0..12 {
                if !self.raise_new(&mut c, s, &seen, seed ^ k) {
                    break;
                }
            }
        }
        self.corps = Some(c);
        self.govern();
        // the books
        self.sync_divs(pack);
        let c = self.corps.as_ref().unwrap();
        for s in 0..2 {
            let mut total = c.pool[s].clone();
            for d in c.divs.iter().filter(|d| d.side == s) {
                for u in 0..nu {
                    total[u] += d.has[u];
                }
            }
            self.sides[s].army.count = total;
            self.sides[s].age();
            self.cut_off[s] += (0..self.owner.len()).filter(|&p| self.owner[p] == s && reach[s][p] == NO).count() as f32;
        }
    }

    /// A staff that builds by its plan forms divisions of its plan's make-up, as big as its army
    /// and front call for, with the kit it chose and the companies worth their price: every few
    /// months it weighs them anew (the kit too), and a new pattern comes when any of it changed —
    /// or at once, when the stores no longer fill the one it has.
    fn staff_pattern(&mut self, pack: &Pack, c: &mut Corps, s: usize, ground: &[(Terrain, f32)], seed: u64) {
        if !c.by_plan[s] {
            return;
        }
        let now = c.day / 30;
        let due = now >= c.reorg[s] + REORG || !self.can_form(pack, c, s, c.shapes[s].len().saturating_sub(1));
        if !due {
            return;
        }
        if !ground.is_empty() {
            self.sides[s].renorm(pack, ground, seed ^ 0xA0A ^ s as u64);
        }
        // (each new make-up of the staff's its own name: the second pattern, the third…)
        const NTH: [&str; 9] = ["второй", "третий", "четвёртый", "пятый", "шестой", "седьмой", "восьмой", "девятый", "десятый"];
        let made = c.shapes[s].iter().filter(|x| x.name.starts_with("Дивизия генштаба")).count();
        let name = if made == 0 { "Дивизия генштаба".to_string() } else { format!("Дивизия генштаба, {} образец", NTH.get(made - 1).copied().unwrap_or("новый")) };
        let sh = self.staff_shape(pack, c, s, &name);
        // (a new pattern when the kit or the companies changed, or the battalions by a sixth of
        // them — not every turn of the plan)
        let differs = |x: &Shape| {
            let moved: f32 = x.units.iter().zip(&sh.units).map(|(a, b)| (a - b).abs()).sum::<f32>() / BATTALION;
            x.norm != sh.norm || x.support != sh.support || moved >= (x.battalions() / 6.0).max(2.0)
        };
        if c.shapes[s].last().is_none_or(differs) {
            c.reorg[s] = now;
            c.shapes[s].push(sh);
            c.wants[s] = vec![0.0; c.shapes[s].len()];
            *c.wants[s].last_mut().unwrap() = 1.0;
        }
    }

    /// A month of peace for the divisions (the game keeps its armies in divisions in peace too):
    /// what each economy made (`made`, units) goes to its stores, with the kit its factories give
    /// it; `retire` of the army goes home (the peace's books send their men back to work); each
    /// staff weighs its pattern and forms new divisions; the books.
    pub fn peace_divs(&mut self, pack: &Pack, made: &[Vec<f32>; 2], retire: f32, ground: &[(Terrain, f32)]) {
        let Some(mut c) = self.corps.take() else { return };
        let nu = pack.units.len();
        let seed = hash64(self.seed ^ 0x9EAC ^ c.day as u64);
        for s in 0..2 {
            for u in 0..nu {
                let m = made[s].get(u).copied().unwrap_or(0.0).max(0.0);
                c.pool[s][u] += m;
                c.kit[s][u] += m * self.sides[s].norm_of(u);
            }
            c.made[s] = made[s].iter().map(|x| x.max(0.0)).sum();
            let k = 1.0 - retire;
            for x in c.pool[s].iter_mut().chain(c.kit[s].iter_mut()) {
                *x *= k;
            }
            for d in c.divs.iter_mut().filter(|d| d.side == s) {
                for x in d.has.iter_mut().chain(d.kit.iter_mut()) {
                    *x *= k;
                }
            }
            Campaign::reckon(&mut c, s);
            c.lack[s][0] = self.rough_share(s);
            self.staff_pattern(pack, &mut c, s, ground, seed);
            self.form(pack, &mut c, s);
        }
        c.divs.retain(|d| d.has.iter().sum::<f32>() > 1e-3);
        self.corps = Some(c);
        self.sync_divs(pack);
        let c = self.corps.as_ref().unwrap();
        for s in 0..2 {
            let mut total = c.pool[s].clone();
            for d in c.divs.iter().filter(|d| d.side == s) {
                for u in 0..nu {
                    total[u] += d.has[u];
                }
            }
            self.sides[s].army.count = total;
        }
    }

    /// The war begins for armies that stood in divisions in peace: the groups of armies take their
    /// shares of them, the nearest first (they march to their sectors); the rest stays the staff's
    /// reserve.
    pub fn mobilise(&mut self) {
        let Some(mut c) = self.corps.take() else { return };
        c.peace = false;
        for d in c.divs.iter_mut() {
            d.manual = false;
        }
        for s in 0..2 {
            self.regroup(&mut c, s);
        }
        self.corps = Some(c);
    }

    /// Whether side `s`'s stores could fill a division of its template `k` (as `form` asks).
    fn can_form(&self, pack: &Pack, c: &Corps, s: usize, k: usize) -> bool {
        let Some(sh) = c.shapes[s].get(k) else { return false };
        let extra = 1.0 + sh.overhead(pack);
        let full = &sh.units;
        let has: Vec<f32> = (0..full.len()).map(|u| full[u].min(c.pool[s][u] / extra)).collect();
        (0..full.len()).all(|u| full[u] <= 0.0 || has[u] >= 0.4 * full[u]) && Corps::worth(pack, &has) >= 0.8 * Corps::worth(pack, full)
    }

    /// Side `s` forms whole divisions from its pool, by its templates (the one most behind what
    /// it wants of it first), while the pool fills at least most of one; they gather at its camp
    /// behind the line (`camp`) and join the group that has the fewest for its share.
    fn form(&mut self, pack: &Pack, c: &mut Corps, s: usize) {
        let nu = pack.units.len();
        let wants = c.wants[s].clone();
        if wants.iter().sum::<f32>() <= 0.0 {
            return;
        }
        // (a side whose stores did not feed the troops it has last month raises no more: the army grows to what can be supplied)
        if self.stores[s] < self.feed_least {
            return;
        }
        // (behind the line: recruits gathered on the front cannot drill)
        let at = self.camp(s);
        // (or where the player told them to gather, while it is his)
        let at = c.muster[s].filter(|&p| p < self.owner.len() && self.owner[p] == s).unwrap_or(at);
        let to = c.muster_group[s].filter(|&g| self.command[s].as_ref().is_some_and(|gs| g < gs.len()));
        // (no more divisions than the front has room for: the rest stays in the stores)
        let most = (self.most_per_front.min(self.establish[s]) * self.front(s).len().max(1) as f32).ceil() as usize;
        let n0 = c.divs.iter().filter(|d| d.side == s).count() as f32;
        for _ in 0..500 {
            if c.divs.iter().filter(|d| d.side == s).count() >= most {
                break;
            }
            let built: Vec<f32> = (0..c.shapes[s].len()).map(|k| c.divs.iter().filter(|d| d.side == s && d.shape == k).count() as f32).collect();
            let Some(k) = (0..wants.len()).filter(|&k| wants[k] > 0.0).min_by(|&a, &b| (built[a] / wants[a]).total_cmp(&(built[b] / wants[b])).then(a.cmp(&b))) else { break };
            // (nor one more than the stores of supply carry: each new division thins the cover of the whole army)
            if self.cover_least > 0.0 && n0 > 0.0 {
                let n = c.divs.iter().filter(|d| d.side == s).count() as f32;
                if self.cover[s] * n0 / (n + 1.0) < self.cover_least {
                    break;
                }
            }
            let sh = &c.shapes[s][k];
            let full = sh.units.clone();
            // (its companies are paid for from the same stores: a share more of each)
            let extra = 1.0 + sh.overhead(pack);
            let worth = Corps::worth(pack, &full);
            let has: Vec<f32> = (0..nu).map(|u| full[u].min(c.pool[s][u] / extra)).collect();
            let enough = (0..nu).all(|u| full[u] <= 0.0 || has[u] >= 0.4 * full[u]);
            if worth <= 0.0 || Corps::worth(pack, &has) < 0.8 * worth || !enough {
                break;
            }
            // its equipment: what its norm gives its men, as far as the stores go (a division short
            // of it is formed, but not one with next to none)
            let kit: Vec<f32> = (0..nu).map(|u| (has[u] * sh.norm_of(u)).min(c.kit[s][u] / extra)).collect();
            let want: f32 = (0..nu).map(|u| has[u] * sh.norm_of(u) * equip_worth(pack, u)).sum();
            let got: f32 = (0..nu).map(|u| kit[u] * equip_worth(pack, u)).sum();
            if want > 0.0 && got < KIT_LEAST * want {
                break;
            }
            for u in 0..nu {
                c.pool[s][u] = (c.pool[s][u] - has[u] * extra).max(0.0);
                c.kit[s][u] = (c.kit[s][u] - kit[u] * extra).max(0.0);
            }
            let id = Campaign::enlist(c, s, k, has, kit, at, FRESH_ORG, c.fresh_drill[s]);
            // the group the player sends them to; else the group with the fewest divisions for its
            // share takes it
            if let Some(g) = to {
                let i = c.find(id).unwrap();
                c.divs[i].group = g;
            } else if let Some(gs) = &self.command[s] {
                let count = |g: usize| c.divs.iter().filter(|d| d.side == s && d.group == g).count() as f32;
                let size = self.army_size(c, s) as f32;
                if let Some(g) = (0..gs.len()).filter(|&g| gs[g].share > 0.0 && count(g) < size).min_by(|&a, &b| (count(a) / gs[a].share).total_cmp(&(count(b) / gs[b].share)).then(a.cmp(&b))) {
                    let i = c.find(id).unwrap();
                    c.divs[i].group = g;
                }
            }
        }
    }

    /// The divisions an army of side `s` takes: as many as a general leads well (`ARMY`), more
    /// when the side forms no more than `most_groups` armies and has divisions for more.
    pub fn army_size(&self, c: &Corps, s: usize) -> usize {
        if self.most_groups == usize::MAX || self.fixed_armies {
            return crate::military::front::ARMY;
        }
        let n = c.divs.iter().filter(|d| d.side == s).count();
        crate::military::front::ARMY.max(n.div_ceil(self.most_groups.max(1)))
    }

    /// The staff of side `s` moves divisions between its groups and its reserve to match their
    /// shares now (a side that has just formed its groups: the reserve fills them).
    pub fn regroup_side(&mut self, s: usize) {
        let Some(mut c) = self.corps.take() else { return };
        self.regroup(&mut c, s);
        self.corps = Some(c);
    }

    /// The staff of side `s` fills its armies from its reserve (and from armies over what a general
    /// leads well), the nearest divisions first, each army to `ARMY`. The player's groups keep theirs.
    fn regroup(&self, c: &mut Corps, s: usize) {
        let Some(gs) = &self.command[s] else { return };
        let k = gs.len();
        let staff: Vec<bool> = gs.iter().map(|g| g.staff).collect();
        // (each army as many as its general leads well, the rest is the reserve: the staff gathers
        // force where the enemy presses by cutting stretches shorter there — more armies to a
        // province — not by thinning armies elsewhere. Shared out by the enemy seen, a quiet stretch
        // of many provinces got two or three and its line was full of holes, while hundreds stood
        // in the reserve)
        let parts = self.sectors_of(s);
        // (in the game, an army too small for all its groups is shared out evenly among them, not
        // poured into the first till it is full)
        let size = if self.most_groups == usize::MAX {
            self.army_size(c, s)
        } else {
            let staffed = staff.iter().filter(|&&x| x).count().max(1);
            let all = c.divs.iter().filter(|d| d.side == s && !d.manual).count();
            self.army_size(c, s).min(all.div_ceil(staffed)).max(1)
        };
        let want: Vec<usize> = vec![size; k];
        let count = |c: &Corps, g: usize| c.divs.iter().filter(|d| d.side == s && d.group == g && !d.manual).count();
        // the spare: the reserve, and the staff's groups over their share
        let mut spare: Vec<usize> = (0..c.divs.len()).filter(|&i| c.divs[i].side == s && c.divs[i].group == NONE && !c.divs[i].manual).collect();
        for g in 0..k {
            if !staff[g] {
                continue;
            }
            let over = count(c, g).saturating_sub(want[g]);
            let mine: Vec<usize> = (0..c.divs.len()).filter(|&i| c.divs[i].side == s && c.divs[i].group == g && self.free(c, i, s)).collect();
            spare.extend(mine.iter().rev().take(over).copied());
        }
        for g in 0..k {
            if !staff[g] {
                continue;
            }
            let lack = want[g].saturating_sub(count(c, g));
            if lack == 0 || spare.is_empty() {
                continue;
            }
            let d = self.distance_from_all(parts.get(g).map_or(&[][..], |p| &p[..]));
            for _ in 0..lack {
                let Some(j) = (0..spare.len()).min_by_key(|&j| (d[c.divs[spare[j]].at], spare[j])) else { break };
                let i = spare.swap_remove(j);
                c.divs[i].group = g;
            }
        }
        // what is still spare is the staff's reserve
        for i in spare {
            c.divs[i].group = NONE;
        }
    }

    /// The staff of side `s` raises a new army when its reserve holds trained divisions enough for
    /// half of one (`ARMY`) beyond what its armies have room for, and an army of its own holds two
    /// provinces of the front or more: the army
    /// pressed hardest for its divisions (the enemy it sees across its stretch against them) gives up
    /// the far half of its stretch to the new one, which takes the reserve's divisions nearest it, as
    /// many as a general leads well; a new man at its head. (With no army leading more than a
    /// general leads well, a growing army's new divisions waited in the reserve for good.) Not while
    /// the player sends the new divisions to a group of his.
    fn raise_new(&mut self, c: &mut Corps, s: usize, seen: &[f32], seed: u64) -> bool {
        let army = self.army_size(c, s);
        let Some(gs) = &self.command[s] else { return false };
        // (as many armies as a side forms at most: then they grow instead)
        if c.muster_group[s].is_some() || gs.len() >= self.most_groups {
            return false;
        }
        let spare: Vec<usize> = (0..c.divs.len()).filter(|&i| c.divs[i].side == s && c.divs[i].group == NONE && !c.divs[i].manual && c.divs[i].drill >= 1.0).collect();
        let parts = self.sectors_of(s);
        let count = |g: usize| c.divs.iter().filter(|d| d.side == s && d.group == g).count() as f32;
        // (only when the reserve holds half an army more than the staff's armies have room for: else
        // the armies multiplied — seventy of eight divisions each — while the old ones stood half empty)
        let room: usize = (0..gs.len()).filter(|&g| gs[g].staff && gs[g].facing.is_none()).map(|g| army.saturating_sub(count(g) as usize)).sum();
        if spare.len() < room + army / 2 {
            return false;
        }
        let pressed = |g: usize| parts[g].iter().map(|&p| self.threat(s, p, seen)).sum::<f32>() / count(g).max(1.0);
        let Some(g) = (0..gs.len())
            .filter(|&g| gs[g].staff && gs[g].facing.is_none() && parts.get(g).is_some_and(|p| p.len() >= 2))
            .max_by(|&a, &b| pressed(a).total_cmp(&pressed(b)).then(b.cmp(&a)))
        else {
            return false;
        };
        let line = self.along(&parts[g]);
        let half = line.len() / 2;
        let (keep, give) = (line[..half].to_vec(), line[half..].to_vec());
        let mut rng = Rng::new(hash64(seed ^ 0xA2A1 ^ ((s as u64) << 8) ^ gs.len() as u64));
        let name = crate::world::names::person(&mut rng);
        let general = General::anyone(name.clone(), &mut rng);
        let gs = self.command[s].as_mut().unwrap();
        gs[g].anchor = keep;
        let share = gs[g].share / 2.0;
        gs[g].share = share;
        gs.push(crate::military::front::Group { anchor: give.clone(), share, order: Order::Hold, strike: 1.0 - crate::military::front::LINE_ATTACK, staff: true, general: Some(general), facing: None });
        let k = gs.len() - 1;
        // (the reserve's divisions nearest its stretch)
        let d = self.distance_from_all(&give);
        let mut spare = spare;
        spare.sort_by_key(|&i| (d[c.divs[i].at], i));
        let n = spare.len().min(army);
        for &i in &spare[..n] {
            c.divs[i].group = k;
        }
        c.news.push((c.day, s, format!("генштаб сформировал новую армию: {n} {}, во главе {name}", crate::world::names::plural(n, "дивизия", "дивизии", "дивизий"))));
        true
    }

    /// What a division eats (units that need supply, by the kit they burn).
    pub fn eats(pack: &Pack, d: &Division) -> f32 {
        (0..d.has.len())
            .filter(|&u| !pack.units[u].supply.is_empty())
            .map(|u| {
                let r = ration_share(pack, u);
                d.has[u] * (r + (1.0 - r) * norm::eat(d.q(u)))
            })
            .sum()
    }

    /// Share of side `s`'s front on rough ground.
    fn rough_share(&self, s: usize) -> f32 {
        let f = self.front(s);
        if f.is_empty() {
            return 0.0;
        }
        f.iter().filter(|&&p| rough(self.th.terrain[p])).count() as f32 / f.len() as f32
    }

    /// The month's tallies of side `s` become what its staff sees it lacks; the tallies start
    /// again.
    fn reckon(c: &mut Corps, s: usize) {
        let t = c.tally[s];
        let units: f32 = c.divs.iter().filter(|d| d.side == s).map(|d| d.has.iter().sum::<f32>()).sum::<f32>() + c.pool[s].iter().sum::<f32>();
        let kit: f32 = c.divs.iter().filter(|d| d.side == s).map(|d| d.kit.iter().sum::<f32>()).sum::<f32>() + c.kit[s].iter().sum::<f32>();
        let mut seen = c.lack[s];
        if t[T_ATT] > 0.0 {
            seen[1] = t[T_FAIL] / t[T_ATT];
            seen[2] = (t[T_STOP] / t[T_ATT]).min(1.0);
        }
        // (a twentieth of the army lost or left behind in a month is as bad as it gets)
        seen[3] = (20.0 * t[T_DEAD] / units.max(1e-6)).min(1.0);
        if t[T_DAYS] > 0.0 {
            seen[4] = (3.0 * t[T_CROWD] / t[T_DAYS]).min(1.0);
        }
        seen[5] = (20.0 * t[T_LEFT] / (kit + t[T_LEFT]).max(1e-6)).min(1.0);
        for k in 1..6 {
            c.lack[s][k] += LACK_PACE * (seen[k] - c.lack[s][k]);
        }
        c.tally[s] = [0.0; 8];
    }

    /// The staff's template for side `s`: its plan put into battalions — as many to a division as
    /// its army can give each province of its front a few divisions (a line, a relief, a reserve:
    /// more spread out in deep defence, more concentrated when attacking) — with the kit the staff
    /// chose for each kind and the companies of support worth their price to it. A poor army on a
    /// long front gets small, light divisions; a rich one on a short front big, heavy ones.
    pub(crate) fn staff_shape(&self, pack: &Pack, c: &Corps, s: usize, name: &str) -> Shape {
        let nu = pack.units.len();
        let side = &self.sides[s];
        let mut plan: Vec<f32> = (0..nu).map(|u| side.plan[u] / side.cost[u].max(1e-9)).collect();
        if self.line_first {
            line_first(pack, &mut plan);
        }
        // the army: its divisions, its stores, a month of what it makes
        let units: f32 = c.pool[s].iter().sum::<f32>() + c.divs.iter().filter(|d| d.side == s).map(|d| d.has.iter().sum::<f32>()).sum::<f32>() + c.made[s];
        let front = self.front(s).len().max(1) as f32;
        let depth = match &self.command[s] {
            Some(gs) if !gs.is_empty() => {
                let all: f32 = gs.iter().map(|g| g.share).sum::<f32>().max(1e-6);
                gs.iter()
                    .map(|g| {
                        g.share / all
                            * match g.order {
                                Order::Hold => DEPTH[0],
                                Order::Deep => DEPTH[1],
                                Order::Attack(_) => DEPTH[2],
                            }
                    })
                    .sum::<f32>()
            }
            _ => DEPTH[0],
        };
        let mut size = (units / BATTALION / (front * depth * self.depth_k)).clamp(FEWEST, self.most_battalions.max(FEWEST));
        // (the size it has stays while the call is within a fifth of it)
        if let Some(old) = c.shapes[s].iter().rev().find(|x| x.name.starts_with(name.split(',').next().unwrap_or(name))) {
            let b = old.battalions();
            if b > 0.0 && (size - b).abs() <= RESIZE * b {
                size = b;
            }
        }
        let mut sh = Shape::sized(name, &plan, size);
        if !side.norm.is_empty() {
            sh.norm = side.norm.clone();
        }
        // companies: each as what it answers weighs against what it adds to a division's price
        let base: f32 = (0..nu).map(|u| sh.units[u] * worth_at(pack, u, sh.norm_of(u))).sum::<f32>().max(1e-6);
        sh.support = (0..pack.support.len()).filter(|&k| pack.support[k].cost / base <= SUPPORT_BASE + SUPPORT_NEED * need_of(&pack.support[k], &c.lack[s])).collect();
        sh
    }

    /// The theatre's books from the divisions: each side's troops in each province, how dug in.
    pub fn sync_divs(&mut self, pack: &Pack) {
        let Some(c) = self.corps.as_ref() else { return };
        let n = self.owner.len();
        let nu = pack.units.len();
        for s in 0..2 {
            self.troops[s] = vec![vec![0.0; nu]; n];
            self.reserve[s] = vec![0.0; nu];
            self.dug[s] = vec![0.0; n];
        }
        let mut weight = [vec![0.0f32; n], vec![0.0f32; n]];
        for d in &c.divs {
            let eff = d.eff();
            for u in 0..nu {
                self.troops[d.side][d.at][u] += eff[u];
            }
            let w = d.has.iter().sum::<f32>();
            self.dug[d.side][d.at] += d.dug * w;
            weight[d.side][d.at] += w;
        }
        for s in 0..2 {
            for p in 0..n {
                if weight[s][p] > 0.0 {
                    self.dug[s][p] /= weight[s][p];
                }
            }
        }
    }
}
