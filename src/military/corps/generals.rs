//! The generals lead their divisions: each holds his sector stretch by stretch by the enemy he
//! sees across it and, told to attack, gathers blows where his war games give him the odds
//! he wants; where each division of a group stands.

use super::*;

impl Campaign {
    /// The generals and the staff of side `s` give their orders for today: blows gathered and
    /// ready are struck; every few days each general places his group anew; the reserve comes to
    /// the help of provinces under attack.
    pub(super) fn lead(&mut self, pack: &Pack, c: &mut Corps, s: usize, seen: &[f32], reach: &[Vec<u32>; 2], seed: u64) {
        let o = 1 - s;
        let groups = self.command[s].clone().unwrap_or_default();
        // (once a day, not for every general: the enemy's true worth in each province — what his
        // generals see with their own errors — and where the enemy's supply reaches)
        let mut truth = vec![0.0f32; self.owner.len()];
        for d in c.divs.iter().filter(|d| d.side != s) {
            truth[d.at] += Corps::power(pack, d) * d.org.max(0.2);
        }
        let foe_reach = self.reach(o);
        // 1. the blows: struck when gathered and ready, if the odds still hold; given up when stale
        let mut keep = Vec::new();
        for b in std::mem::take(&mut c.blows[s]) {
            let idx: Vec<usize> = b.ids.iter().filter_map(|&id| c.find(id)).filter(|&i| !c.divs[i].manual && c.divs[i].group == b.group).collect();
            if idx.is_empty() || self.owner[b.target] != o || b.froms.iter().any(|&f| self.owner[f] != s) || c.day > b.since + BLOW_DAYS {
                continue;
            }
            let ready = idx.iter().all(|&i| b.froms.contains(&c.divs[i].at) && c.divs[i].way.is_empty() && c.divs[i].org >= READY && c.divs[i].rest <= 0.0);
            if !ready {
                keep.push(Blow { ids: idx.iter().map(|&i| c.divs[i].id).collect(), ..b });
                continue;
            }
            let gr = groups.get(b.group);
            let wants = self.wants_of(gr);
            let judged = self.judged(gr, b.target, &truth, seen, seed);
            let mut units = vec![0.0f32; pack.units.len()];
            for &i in &idx {
                for (u, x) in c.divs[i].eff().iter().enumerate() {
                    units[u] += x * c.divs[i].org;
                }
            }
            let odds = self.odds(pack, s, b.target, &units, c.level[s][b.from], judged, c.level[o].get(b.target).copied().unwrap_or(0.0), seed ^ b.target as u64 ^ 0xB10);
            if odds + 0.1 >= wants {
                for &i in &idx {
                    c.divs[i].way = vec![b.target];
                    c.divs[i].gone = 0.0;
                }
                c.odds_for.push((b.target, s, odds));
            }
        }
        c.blows[s] = keep;
        // recruits standing on the line beside trained divisions go back to the camp to drill (one
        // alone there holds a hole of its general's line, green as it is)
        if c.day % PLAN_EVERY == 1 {
            let mut trained = vec![false; self.owner.len()];
            for d in c.divs.iter().filter(|d| d.side == s && d.drill >= 1.0) {
                trained[d.at] = true;
            }
            let back: Vec<usize> = (0..c.divs.len())
                .filter(|&i| {
                    let d = &c.divs[i];
                    d.side == s && d.drill < 1.0 && d.way.is_empty() && trained[d.at] && self.on_line(d.at, s) && self.free_but_green(c, i, s)
                })
                .collect();
            if !back.is_empty() {
                let camp = self.camp(s);
                let d = self.days_to(s, camp, true);
                for i in back {
                    let way = self.way_by(s, c.divs[i].at, &d, true);
                    if !way.is_empty() {
                        c.divs[i].way = way;
                        c.divs[i].gone = 0.0;
                    }
                }
            }
        }
        // 2. every few days each general places his group anew (not all on the same day)
        let parts = self.sectors_of(s);
        let staff = self.staff_led[s];
        for (g, gr) in groups.iter().enumerate() {
            if c.day > 1 && (c.day + g as u32) % PLAN_EVERY != 1 {
                continue;
            }
            let line: Vec<usize> = parts.get(g).map_or(Vec::new(), |p| p.iter().copied().filter(|&x| reach[s][x] != NO).collect());
            self.place_group(pack, c, s, g, gr, &line, seen, &truth, &foe_reach, seed);
        }
        // 3. the reserve (led by the staff): behind the front where the enemy is seen strongest;
        // it, and a group's divisions behind its line, come up to provinces under attack where the
        // attackers outweigh the defenders
        if staff {
            if c.day % PLAN_EVERY == 1 {
                let front = self.front(s);
                let behind = self.behind(s, &front);
                let free: Vec<usize> = (0..c.divs.len()).filter(|&i| self.free(c, i, s) && c.divs[i].group == NONE).collect();
                if !behind.is_empty() && !free.is_empty() {
                    let weight: Vec<f32> = behind.iter().map(|&p| 0.2 + self.th.nbr[p].iter().map(|&r| if self.owner[r] == s { self.threat(s, r, seen) } else { 0.0 }).sum::<f32>()).collect();
                    let mut want = Campaign::apportion(&weight, free.len(), false);
                    // (no more than the land feeds, with the others there: the rest in depth)
                    let per = free.iter().map(|&i| Campaign::eats(pack, &c.divs[i]) * c.gift(pack, &c.divs[i]).eat).sum::<f32>() / free.len() as f32;
                    let mut mine = vec![false; c.divs.len()];
                    for &i in &free {
                        mine[i] = true;
                    }
                    let mut others = vec![0usize; self.owner.len()];
                    for (i, d) in c.divs.iter().enumerate() {
                        if d.side == s && !mine[i] {
                            others[d.way.last().copied().unwrap_or(d.at)] += 1;
                        }
                    }
                    let room = |p: usize| -> usize { ((self.cap_at(pack, s, p) / per.max(1e-3)).floor() as usize).saturating_sub(others[p]) };
                    let mut over = 0;
                    for (k, &p) in behind.iter().enumerate() {
                        let r = room(p);
                        if want[k] > r {
                            over += want[k] - r;
                            want[k] = r;
                        }
                    }
                    let mut places: Vec<(usize, usize)> = behind.iter().copied().zip(want).filter(|x| x.1 > 0).collect();
                    if over > 0 {
                        let near: Vec<usize> = front.iter().chain(&behind).copied().collect();
                        places.extend(self.depth(s, &near, over, &room));
                    }
                    self.station(c, s, &free, &places);
                }
            }
            for f in c.fights.clone() {
                if f.side != o || f.balance < 0.5 {
                    continue;
                }
                let q = f.at;
                let d_q = self.days_to(s, q, true);
                let cand = (0..c.divs.len())
                    .filter(|&i| self.free(c, i, s) && c.divs[i].at != q && c.divs[i].way.last() != Some(&q) && d_q[c.divs[i].at].is_finite())
                    .filter(|&i| c.divs[i].group == NONE || !self.on_line(c.divs[i].at, s))
                    .min_by(|&a, &b| d_q[c.divs[a].at].total_cmp(&d_q[c.divs[b].at]).then(a.cmp(&b)));
                if let Some(i) = cand {
                    let way = self.way_by(s, c.divs[i].at, &d_q, true);
                    if !way.is_empty() {
                        c.divs[i].way = way;
                        c.divs[i].gone = 0.0;
                    }
                }
            }
        }
    }

    /// Whether province `p` of side `s` touches the enemy.
    fn on_line(&self, p: usize, s: usize) -> bool {
        self.th.nbr[p].iter().any(|&r| self.owner[r] != s)
    }

    /// Whether division `i` of side `s` is free for its general's orders: its own, trained, not
    /// sent by the player, not falling back, not attacking, not in a blow, not defending a fight.
    pub(super) fn free(&self, c: &Corps, i: usize, s: usize) -> bool {
        (c.divs[i].drill >= 1.0 || c.drill_free) && self.free_but_green(c, i, s)
    }

    /// `free`, whether trained or not.
    fn free_but_green(&self, c: &Corps, i: usize, s: usize) -> bool {
        let d = &c.divs[i];
        if d.side != s || d.manual || d.rest > 0.0 {
            return false;
        }
        if d.way.first().is_some_and(|&q| self.owner[q] != s) {
            return false;
        }
        if c.fights.iter().any(|f| f.at == d.at && f.side != s) {
            return false;
        }
        !c.blows[s].iter().any(|b| b.ids.contains(&d.id))
    }

    /// The odds a general (none: the staff) wants to strike at.
    fn wants_of(&self, gr: Option<&crate::military::front::Group>) -> f32 {
        let base = if gr.is_none_or(|g| g.staff) { crate::military::front::ODDS } else { crate::military::front::ODDS_ORDERED };
        gr.and_then(|g| g.general.as_ref()).map_or(base, |x| x.wants(base))
    }

    /// The enemy in province `q` as the general of group `gr` judges it: on the spot, with his own
    /// error, from what it truly is (`truth`: worth by province); none: as the staff sees it.
    fn judged(&self, gr: Option<&crate::military::front::Group>, q: usize, truth: &[f32], seen: &[f32], seed: u64) -> f32 {
        match gr.and_then(|g| g.general.as_ref()) {
            Some(x) => truth[q] * (x.blur() * Rng::new(hash64(seed ^ 0x6E7 ^ q as u64)).normal()).exp(),
            None => seen[q],
        }
    }

    /// `n` places shared out by `weight`, by largest remainders; with `each`, every place gets one
    /// first while there are enough.
    pub(crate) fn apportion(weight: &[f32], n: usize, each: bool) -> Vec<usize> {
        let k = weight.len();
        let mut out = vec![0usize; k];
        if k == 0 || n == 0 {
            return out;
        }
        let mut left = n;
        if each && n >= k {
            out.iter_mut().for_each(|x| *x = 1);
            left -= k;
        } else if each {
            // fewer than places: the most weighed first
            let mut order: Vec<usize> = (0..k).collect();
            order.sort_by(|&a, &b| weight[b].total_cmp(&weight[a]).then(a.cmp(&b)));
            for &i in order.iter().take(n) {
                out[i] = 1;
            }
            return out;
        }
        let all: f32 = weight.iter().sum::<f32>().max(1e-9);
        let exact: Vec<f32> = weight.iter().map(|w| w / all * left as f32).collect();
        let mut given = 0;
        for i in 0..k {
            let f = exact[i].floor() as usize;
            out[i] += f;
            given += f;
        }
        let mut rest: Vec<usize> = (0..k).collect();
        rest.sort_by(|&a, &b| (exact[b] - exact[b].floor()).total_cmp(&(exact[a] - exact[a].floor())).then(a.cmp(&b)));
        for &i in rest.iter().take(left - given) {
            out[i] += 1;
        }
        out
    }

    /// Places for `n` divisions of side `s` behind its line `line`, ring after ring of its own land
    /// off the front (just behind the line first, then a step further…, four at most), each province
    /// as many as `room` leaves there, evenly within a ring. (All put just behind the line, the rest
    /// of an army too big for its stretch stood where it was — at the capital, forty and more in one
    /// province.)
    pub(super) fn depth(&self, s: usize, line: &[usize], n: usize, room: &dyn Fn(usize) -> usize) -> Vec<(usize, usize)> {
        let mut seen = vec![false; self.owner.len()];
        for &p in line {
            seen[p] = true;
        }
        let mut ring: Vec<usize> = line.to_vec();
        let mut out: Vec<(usize, usize)> = Vec::new();
        let mut left = n;
        for _ in 0..4 {
            if left == 0 {
                break;
            }
            let mut next = Vec::new();
            for &p in &ring {
                for &r in &self.th.nbr[p] {
                    if !seen[r] && self.owner[r] == s && !self.on_line(r, s) {
                        seen[r] = true;
                        next.push(r);
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            next.sort_unstable();
            let mut got = vec![0usize; next.len()];
            let rooms: Vec<usize> = next.iter().map(|&p| room(p)).collect();
            // (one at a time round the ring while there is room)
            loop {
                let mut gave = false;
                for k in 0..next.len() {
                    if left > 0 && got[k] < rooms[k] {
                        got[k] += 1;
                        left -= 1;
                        gave = true;
                    }
                }
                if !gave || left == 0 {
                    break;
                }
            }
            out.extend(next.iter().copied().zip(got).filter(|x| x.1 > 0));
            ring = next;
        }
        out
    }

    /// Divisions `ids` (indices) of side `s` go to their places (province, how many). First every
    /// place gets one: one standing there stays (stopping if it was going on); a place with none
    /// takes the nearest division that is not the only one of another place — so a hole is
    /// closed from beside it, not by a division marching from the far rear. Then the rest of each
    /// place's count: those standing there stay, those going there go on, the rest go to the places
    /// still wanted, the nearest first; any left over stay where they are. (Sent off to new places
    /// every few days while their relief was still on its way, a line left a tenth of its
    /// provinces empty; filled by whoever was left over, its holes waited for new divisions
    /// marching from the capital.)
    fn station(&self, c: &mut Corps, s: usize, ids: &[usize], places: &[(usize, usize)]) {
        let mut need: Vec<(usize, usize)> = places.to_vec();
        let mut placed = vec![false; ids.len()];
        let stop = |c: &mut Corps, i: usize| {
            c.divs[i].way.clear();
            c.divs[i].gone = 0.0;
        };
        let go = |me: &Campaign, c: &mut Corps, i: usize, p: usize, d: &[f32]| {
            let way = me.way_by(s, c.divs[i].at, d, true);
            let dv = &mut c.divs[i];
            if dv.way.first() != way.first() {
                dv.gone = 0.0;
            }
            dv.way = way;
            let _ = p;
        };
        // 1. one for every place: who stands there (the ones staying put first)
        let mut order: Vec<usize> = (0..ids.len()).collect();
        order.sort_by_key(|&j| (!c.divs[ids[j]].way.is_empty(), j));
        let mut holes = Vec::new();
        for x in need.iter_mut() {
            if x.1 == 0 {
                continue;
            }
            match order.iter().copied().find(|&j| !placed[j] && c.divs[ids[j]].at == x.0) {
                Some(j) => {
                    placed[j] = true;
                    stop(c, ids[j]);
                    x.1 -= 1;
                }
                None => holes.push(x.0),
            }
        }
        // a hole takes the nearest division that is not holding a place of its own
        for p in holes {
            let d = self.days_to(s, p, true);
            let Some(j) = (0..ids.len()).filter(|&j| !placed[j] && d[c.divs[ids[j]].at].is_finite()).min_by(|&a, &b| d[c.divs[ids[a]].at].total_cmp(&d[c.divs[ids[b]].at]).then(a.cmp(&b))) else { continue };
            placed[j] = true;
            go(self, c, ids[j], p, &d);
            if let Some(x) = need.iter_mut().find(|x| x.0 == p) {
                x.1 -= 1;
            }
        }
        // 2. the rest of each place's count: who stands there, who goes there
        for &j in &order {
            if placed[j] {
                continue;
            }
            let at = c.divs[ids[j]].at;
            if let Some(x) = need.iter_mut().find(|x| x.0 == at && x.1 > 0) {
                x.1 -= 1;
                placed[j] = true;
                stop(c, ids[j]);
            }
        }
        for j in 0..ids.len() {
            if placed[j] {
                continue;
            }
            if let Some(&dest) = c.divs[ids[j]].way.last() {
                if let Some(x) = need.iter_mut().find(|x| x.0 == dest && x.1 > 0) {
                    x.1 -= 1;
                    placed[j] = true;
                }
            }
        }
        // the places still wanted take the nearest of the rest
        let mut left: Vec<usize> = (0..ids.len()).filter(|&j| !placed[j]).map(|j| ids[j]).collect();
        for (p, k) in need {
            if k == 0 || left.is_empty() {
                continue;
            }
            let d = self.days_to(s, p, true);
            for _ in 0..k {
                let Some(j) = (0..left.len()).filter(|&j| d[c.divs[left[j]].at].is_finite()).min_by(|&a, &b| d[c.divs[left[a]].at].total_cmp(&d[c.divs[left[b]].at]).then(a.cmp(&b))) else { break };
                let i = left.swap_remove(j);
                go(self, c, i, p, &d);
            }
        }
    }

    /// General of group `g` of side `s` places its divisions on its sector `line`: told to hold,
    /// most along the line, each stretch by the enemy seen across it, the rest just behind it (in
    /// deep defence, nearly half behind); told to attack, the share that strikes gathers blows at
    /// the enemy provinces nearest the arrow's head — for each, the fewest divisions, nearest it
    /// first, that give him the odds he wants — the rest keeps the line. Whatever he is told, he
    /// walks into the enemy's provinces next to his line where nobody stands, and strikes at the
    /// enemy's pockets there (cut off from their capital) with what stands behind his line. (With
    /// no such rule, empty provinces of the enemy stayed in the middle of one's own land, and the
    /// land of both sides grew into each other.)
    #[allow(clippy::too_many_arguments)]
    fn place_group(&mut self, pack: &Pack, c: &mut Corps, s: usize, g: usize, gr: &crate::military::front::Group, line: &[usize], seen: &[f32], truth: &[f32], foe_reach: &[u32], seed: u64) {
        let o = 1 - s;
        let mut avail: Vec<usize> = (0..c.divs.len()).filter(|&i| c.divs[i].group == g && self.free(c, i, s)).collect();
        // (recruits stay at their banner while the trained can hold the line: only holes — provinces
        // of the line where none of ours stands or is going — that his free trained cannot fill take
        // the best trained of them, green as they are; counted against the line's length, an army of
        // twenty-four busy in its fights took recruits to stand beside them)
        let mut held = vec![false; self.owner.len()];
        for d in c.divs.iter().filter(|d| d.side == s) {
            held[d.way.last().copied().unwrap_or(d.at)] = true;
            held[d.at] = true;
        }
        let holes = line.iter().filter(|&&p| !held[p]).count();
        if avail.len() < holes {
            let mut green: Vec<usize> = (0..c.divs.len()).filter(|&i| c.divs[i].group == g && c.divs[i].drill < 1.0 && self.free_but_green(c, i, s)).collect();
            green.sort_by(|&a, &b| c.divs[b].drill.total_cmp(&c.divs[a].drill).then(a.cmp(&b)));
            avail.extend(green.into_iter().take(holes - avail.len()));
        }
        let in_blows: usize = c.blows[s].iter().filter(|b| b.group == g).map(|b| b.ids.len()).sum();
        if line.is_empty() || avail.is_empty() {
            return;
        }
        let behind = self.behind(s, line);
        for &p in line.iter().chain(&behind) {
            self.deep[s][p] = gr.order == Order::Deep;
        }
        // the targets: told to attack, the enemy's provinces next to the sector nearest the arrow's
        // head; whatever he is told, the empty ones and the pockets
        let mut targets: Vec<(usize, usize, f32, u8)> = Vec::new();
        let dh = match gr.order {
            Order::Attack(head) => self.distance_from(head),
            _ => vec![NO; self.owner.len()],
        };
        // (in peace he only places them: no blows, nobody walks over the border)
        let line_of_targets: &[usize] = if c.peace { &[] } else { line };
        for &p in line_of_targets {
            for &q in &self.th.nbr[p] {
                // Do not keep assigning troops to a province that another division is already
                // walking into.  Empty provinces have no fight to reserve them for us, so without
                // this check every new plan could pile a fresh stack onto the same capture.
                if self.owner[q] != o
                    || targets.iter().any(|t| t.0 == q)
                    || c.blows[s].iter().any(|b| b.target == q)
                    || c.divs.iter().any(|d| d.side == s && d.way.first() == Some(&q))
                {
                    continue;
                }
                let from = self.th.nbr[q].iter().copied().filter(|r| line.contains(r)).max_by(|&a, &b| c.level[s][a].total_cmp(&c.level[s][b]).then(b.cmp(&a))).unwrap_or(p);
                let judged = self.judged(Some(gr), q, truth, seen, seed);
                targets.push((q, from, judged, 0u8));
            }
        }
        let nearest = targets.iter().map(|t| dh[t.0]).min().unwrap_or(NO);
        for t in targets.iter_mut() {
            t.3 = if t.2 <= 0.0 {
                1
            } else if dh[t.0] <= nearest.saturating_add(crate::military::front::AROUND) {
                2
            } else if foe_reach[t.0] == NO {
                3
            } else {
                0
            };
        }
        targets.retain(|t| t.3 > 0);
        targets.sort_by_key(|t| (t.3, dh[t.0], t.0));

        // An undefended adjacent province is movement, not an offensive.  It must not wait for
        // the attack allocator (which may have kept every available division on the line while it
        // looks for favourable battles).  Send exactly one division already on the neighbouring
        // friendly province; subsequent plans see its order above and leave the capture alone.
        for &(q, _from, _, kind) in &targets {
            if kind != 1 {
                continue;
            }
            let Some(pos) = avail
                .iter()
                .position(|&i| {
                    c.divs[i].way.is_empty()
                        && line.contains(&c.divs[i].at)
                        && self.th.nbr[c.divs[i].at].contains(&q)
                })
            else {
                continue;
            };
            let i = avail.swap_remove(pos);
            c.divs[i].way = vec![q];
            c.divs[i].gone = 0.0;
        }
        let all = avail.len() + in_blows;
        let striking = match gr.order {
            Order::Attack(_) => {
                let bold = gr.general.as_ref().map_or(0.0, |x| x.bold);
                (gr.strike * (1.0 + crate::military::front::BOLD_PUSH * bold)).clamp(0.0, 0.9)
            }
            _ => 0.25,
        };
        let mut strike_n = ((striking * all as f32).round() as usize).max(targets.iter().any(|t| t.3 == 1) as usize).saturating_sub(in_blows).min(avail.len());
        // how many divisions a province can feed (by what one of them eats)
        let per: f32 = {
            let eat = |i: usize| Campaign::eats(pack, &c.divs[i]) * c.gift(pack, &c.divs[i]).eat;
            avail.iter().map(|&i| eat(i)).sum::<f32>() / avail.len().max(1) as f32
        };
        // (others' divisions there or going there take room too: another group's, the reserve's,
        // recruits; each general counting only his own, provinces held 40–60% more than they fed)
        let mut mine = vec![false; c.divs.len()];
        for &i in &avail {
            mine[i] = true;
        }
        let mut others = vec![0usize; self.owner.len()];
        for (i, d) in c.divs.iter().enumerate() {
            if d.side == s && !mine[i] {
                others[d.way.last().copied().unwrap_or(d.at)] += 1;
            }
        }
        let room = |me: &Campaign, p: usize| -> usize {
            let fed = (me.cap_at(pack, s, p) / per.max(1e-3)).floor() as usize;
            fed.saturating_sub(others[p]).max((others[p] == 0) as usize)
        };
        // (how many a province of the line takes: its battle's width, a line and its relief)
        let war = pack.war.as_ref().unwrap();
        let wide: f32 = avail
            .iter()
            .map(|&i| {
                let d = &c.divs[i];
                let front = (0..pack.units.len()).any(|u| pack.units[u].line == Line::Front && d.has[u] > 0.0);
                let holds = if front { Line::Front } else { Line::Support };
                (0..pack.units.len()).filter(|&u| pack.units[u].line == holds).map(|u| d.has[u] * pack.units[u].width).sum::<f32>()
            })
            .sum::<f32>()
            / avail.len().max(1) as f32;
        let line_room = |p: usize| -> usize { ((LINE_FILL * war.width * war.frontage[self.th.terrain[p] as usize] / wide.max(1e-3)).ceil() as usize).max(1) };
        if !targets.is_empty() && strike_n > 0 {
            // the strongest strike (worth of what is fit to fight); an empty province takes the
            // nearest one
            avail.sort_by(|&a, &b| {
                let w = |i: usize| Corps::power(pack, &c.divs[i]) * c.divs[i].org;
                w(b).total_cmp(&w(a)).then(a.cmp(&b))
            });
            let mut pool: Vec<usize> = avail.drain(..strike_n).collect();
            let wants = self.wants_of(Some(gr));
            let rough = gr.general.as_ref().is_some_and(|x| x.education < 0.4);
            for &(q, from, judged, _) in &targets {
                if pool.is_empty() || strike_n == 0 {
                    break;
                }
                // (gathered in every province of ours round the target, no more than they feed)
                let mut froms: Vec<usize> = self.th.nbr[q].iter().copied().filter(|&r| self.owner[r] == s && line.contains(&r)).collect();
                if !froms.contains(&from) {
                    froms.push(from);
                }
                let rooms: Vec<usize> = froms.iter().map(|&f| room(self, f)).collect();
                let most = rooms.iter().sum::<usize>().max(1);
                let d = self.days_to(s, from, true);
                pool.sort_by(|&a, &b| d[c.divs[a].at].total_cmp(&d[c.divs[b].at]).then(a.cmp(&b)));
                let mut sizes: Vec<usize> = vec![1, 2, 3, 4, 6, 8, 11, 15, 20, 27, 36];
                if rough {
                    sizes.retain(|k| [1, 3, 8, 20].contains(k));
                }
                let top = pool.len().min(most);
                sizes.retain(|&k| k <= top);
                if sizes.last() != Some(&top) {
                    sizes.push(top);
                }
                for k in sizes {
                    let mut units = vec![0.0f32; pack.units.len()];
                    for &i in &pool[..k] {
                        for (u, x) in c.divs[i].eff().iter().enumerate() {
                            units[u] += x;
                        }
                    }
                    let odds = self.odds(pack, s, q, &units, c.level[s][from], judged, c.level[o].get(q).copied().unwrap_or(0.0), seed ^ q as u64 ^ 0xA77);
                    if odds >= wants {
                        let chosen: Vec<usize> = pool.drain(..k).collect();
                        strike_n = strike_n.saturating_sub(k);
                        let w: Vec<f32> = rooms.iter().map(|&r| r as f32).collect();
                        let split = Campaign::apportion(&w, chosen.len(), true);
                        let places: Vec<(usize, usize)> = froms.iter().copied().zip(split).filter(|x| x.1 > 0).collect();
                        let used: Vec<usize> = places.iter().map(|x| x.0).collect();
                        self.station(c, s, &chosen, &places);
                        c.blows[s].push(Blow { group: g, from, froms: used, target: q, ids: chosen.iter().map(|&i| c.divs[i].id).collect(), since: c.day });
                        break;
                    }
                }
            }
            // what found no blow keeps the line
            avail.extend(pool);
        }
        // the line, each stretch by the enemy seen across it; the rest behind it
        let first = match gr.order {
            Order::Hold => crate::military::front::LINE_HOLD,
            Order::Deep => crate::military::front::LINE_DEEP,
            Order::Attack(_) => 1.0,
        };
        let on_line = ((first * avail.len() as f32).round() as usize).min(avail.len());
        let threat: Vec<f32> = line.iter().map(|&p| self.threat(s, p, seen)).collect();
        let all: f32 = threat.iter().sum();
        let weight: Vec<f32> = (0..line.len()).map(|i| if all > 0.0 { 0.8 * threat[i] / all + 0.2 / line.len() as f32 } else { 1.0 / line.len() as f32 }).collect();
        // (no province gets more than it can feed: the rest goes where there is room, then behind)
        let mut want = Campaign::apportion(&weight, on_line, true);
        let caps: Vec<usize> = line.iter().map(|&p| room(self, p).min(line_room(p))).collect();
        let mut over = 0;
        for i in 0..line.len() {
            if want[i] > caps[i] {
                over += want[i] - caps[i];
                want[i] = caps[i];
            }
        }
        let mut order: Vec<usize> = (0..line.len()).collect();
        order.sort_by(|&a, &b| weight[b].total_cmp(&weight[a]).then(a.cmp(&b)));
        for &i in &order {
            let put = over.min(caps[i] - want[i]);
            want[i] += put;
            over -= put;
        }
        let mut places: Vec<(usize, usize)> = line.iter().copied().zip(want).filter(|x| x.1 > 0).collect();
        let back = avail.len() - on_line + over;
        if back > 0 {
            places.extend(self.depth(s, line, back, &|p| room(self, p)));
        }
        self.station(c, s, &avail, &places);
    }
}
