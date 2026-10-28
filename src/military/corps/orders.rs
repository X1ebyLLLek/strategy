//! The player's orders to divisions and groups: send, halt, give to a group, send on (a way
//! by points), a group's stretch of the front (drawn, or the whole border with a country —
//! shared evenly by several groups), a new group of chosen divisions.

use super::*;

impl Campaign {
    /// The player sends divisions `ids` of side `s` to province `to` (marching over his own land,
    /// or attacking into the enemy's): their general leaves them be till they are given back.
    /// Returns how many were sent.
    pub fn send(&mut self, s: usize, ids: &[u32], to: usize) -> usize {
        if to >= self.owner.len() {
            return 0;
        }
        let own_to = self.owner[to] == s;
        if !own_to && self.corps.as_ref().is_some_and(|c| c.peace) {
            return 0;
        }
        let d_own = self.days_to(s, to, true);
        let d_all = self.days_to(s, to, false);
        let Some(mut c) = self.corps.take() else { return 0 };
        let mut sent = 0;
        for &id in ids {
            let Some(i) = c.find(id) else { continue };
            let dv = &c.divs[i];
            if dv.side != s || dv.rest > 0.0 {
                continue;
            }
            // over one's own land where it leads there; else through the enemy's, step by step
            let way = if own_to && d_own[dv.at].is_finite() { self.way_by(s, dv.at, &d_own, true) } else { self.way_by(s, dv.at, &d_all, false) };
            if way.is_empty() && dv.at != to {
                continue;
            }
            let dv = &mut c.divs[i];
            dv.gone = if dv.way.first() == way.first() { dv.gone } else { 0.0 };
            dv.way = way;
            dv.manual = true;
            sent += 1;
        }
        for b in c.blows[s].iter_mut() {
            b.ids.retain(|id| !ids.contains(id));
        }
        c.blows[s].retain(|b| !b.ids.is_empty());
        self.corps = Some(c);
        sent
    }

    /// Divisions `ids` of side `s` stop where they are (the player's own, standing).
    pub fn halt(&mut self, s: usize, ids: &[u32]) {
        let Some(c) = self.corps.as_mut() else { return };
        for d in c.divs.iter_mut().filter(|d| d.side == s && ids.contains(&d.id)) {
            d.way.clear();
            d.gone = 0.0;
            d.manual = true;
        }
    }

    /// Divisions `ids` of side `s` go to group `g` (NONE: the reserve), back under its general.
    pub fn assign(&mut self, s: usize, ids: &[u32], g: usize) {
        let groups = self.command[s].as_ref().map_or(0, |v| v.len());
        let Some(c) = self.corps.as_mut() else { return };
        for d in c.divs.iter_mut().filter(|d| d.side == s && ids.contains(&d.id)) {
            d.group = if g < groups { g } else { NONE };
            d.manual = false;
        }
        for b in c.blows[s].iter_mut() {
            b.ids.retain(|id| !ids.contains(id));
        }
        c.blows[s].retain(|b| !b.ids.is_empty());
    }

    /// The player sends divisions `ids` of side `s` on from where their ways end now (where they
    /// stand, if they stand) to `to`: one stop more on their way. Returns how many were sent.
    pub fn send_on(&mut self, s: usize, ids: &[u32], to: usize) -> usize {
        if to >= self.owner.len() || (self.owner[to] != s && self.corps.as_ref().is_some_and(|c| c.peace)) {
            return 0;
        }
        let Some(c) = self.corps.as_ref() else { return 0 };
        // (the way on is laid once from each end)
        let mut ends: Vec<(usize, Vec<usize>)> = Vec::new();
        let mut todo: Vec<(usize, usize)> = Vec::new();
        for &id in ids {
            let Some(i) = c.find(id) else { continue };
            let d = &c.divs[i];
            let end = d.way.last().copied().unwrap_or(d.at);
            if d.side != s || d.rest > 0.0 || end == to {
                continue;
            }
            if !ends.iter().any(|e| e.0 == end) {
                ends.push((end, self.way_to(s, end, to)));
            }
            todo.push((i, end));
        }
        let mut c = self.corps.take().unwrap();
        let mut sent = 0;
        for (i, end) in todo {
            let more = &ends.iter().find(|e| e.0 == end).unwrap().1;
            if more.is_empty() {
                continue;
            }
            let d = &mut c.divs[i];
            d.way.extend(more.iter().copied());
            d.manual = true;
            sent += 1;
        }
        for b in c.blows[s].iter_mut() {
            b.ids.retain(|id| !ids.contains(id));
        }
        c.blows[s].retain(|b| !b.ids.is_empty());
        self.corps = Some(c);
        sent
    }

    /// The nearest of provinces `from` to every province of the theatre (over all of them);
    /// usize::MAX where none reaches.
    fn nearest_of(&self, from: &[usize]) -> Vec<usize> {
        let mut lab = vec![usize::MAX; self.owner.len()];
        let mut q = std::collections::VecDeque::new();
        for &p in from {
            if lab[p] == usize::MAX {
                lab[p] = p;
                q.push_back(p);
            }
        }
        while let Some(p) = q.pop_front() {
            for &r in &self.th.nbr[p] {
                if lab[r] == usize::MAX {
                    lab[r] = lab[p];
                    q.push_back(r);
                }
            }
        }
        lab
    }

    /// Group `g` of side `s` is given the provinces `line` to anchor its sector (each month its
    /// sector is the stretch of the front nearest them): the other groups give them up and keep
    /// the rest of their sectors (a group left with nothing keeps what it had — or, `strip`, is
    /// left without a front: it stands in reserve, as an army whose front line was drawn over).
    fn take_sector(&mut self, s: usize, g: usize, line: &[usize]) {
        self.take_sector_as(s, g, line, false);
    }

    fn take_sector_as(&mut self, s: usize, g: usize, line: &[usize], strip: bool) {
        let parts = self.sectors_of(s);
        let Some(gs) = self.command[s].as_mut() else { return };
        for (k, gr) in gs.iter_mut().enumerate() {
            if k == g {
                gr.anchor = line.to_vec();
                continue;
            }
            let kept: Vec<usize> = gr.anchor.iter().copied().filter(|p| !line.contains(p)).collect();
            if !kept.is_empty() {
                gr.anchor = kept;
                continue;
            }
            let rest: Vec<usize> = parts.get(k).map_or(Vec::new(), |p| p.iter().copied().filter(|p| !line.contains(p)).collect());
            if !rest.is_empty() || strip {
                gr.anchor = rest;
            }
        }
    }

    /// The player gives group `g` of side `s` its own stretch of the front, the provinces of his
    /// that he drew it through (the group is his from now: the staff leaves it be). Whether done.
    pub fn set_sector(&mut self, s: usize, g: usize, line: &[usize]) -> bool {
        let mut own: Vec<usize> = line.iter().copied().filter(|&p| p < self.owner.len() && self.owner[p] == s).collect();
        own.dedup();
        let groups = self.command[s].as_ref().map_or(0, |v| v.len());
        if own.is_empty() || g >= groups {
            return false;
        }
        self.take_sector(s, g, &own);
        let gr = &mut self.command[s].as_mut().unwrap()[g];
        gr.staff = false;
        gr.facing = None;
        true
    }

    /// Provinces `line` in their order along themselves: from one end of the line (the province
    /// farthest over them from the first), the nearest first; a piece apart from the rest after it,
    /// in its own order.
    pub fn along(&self, line: &[usize]) -> Vec<usize> {
        let on = |p: usize| line.contains(&p);
        let walk = |from: usize| -> Vec<(usize, u32)> {
            let mut d: Vec<(usize, u32)> = vec![(from, 0)];
            let mut q = std::collections::VecDeque::from([from]);
            while let Some(p) = q.pop_front() {
                let dp = d.iter().find(|x| x.0 == p).unwrap().1;
                for &r in &self.th.nbr[p] {
                    if on(r) && !d.iter().any(|x| x.0 == r) {
                        d.push((r, dp + 1));
                        q.push_back(r);
                    }
                }
            }
            d
        };
        let mut out: Vec<usize> = Vec::new();
        for &p0 in line {
            if out.contains(&p0) {
                continue;
            }
            let end = walk(p0).into_iter().max_by_key(|x| (x.1, x.0)).map_or(p0, |x| x.0);
            let mut piece = walk(end);
            piece.sort_by_key(|x| (x.1, x.0));
            out.extend(piece.into_iter().map(|x| x.0));
        }
        out
    }

    /// The player gives his groups `gs` of side `s` a line through provinces `line` of his, cut
    /// among them into stretches one after another along it, each as long as its divisions call for; `facing` — the country of the world
    /// the line faces when not the enemy of this war (each group then holds exactly its stretch;
    /// else each holds the part of the front nearest its stretch, as the front moves). Other groups
    /// give the line up; the groups are his. Whether done.
    pub fn set_front(&mut self, s: usize, gs: &[usize], line: &[usize], facing: Option<u32>) -> bool {
        let groups = self.command[s].as_ref().map_or(0, |v| v.len());
        let mut own: Vec<usize> = Vec::new();
        for &p in line {
            if p < self.owner.len() && self.owner[p] == s && !own.contains(&p) {
                own.push(p);
            }
        }
        let mut who: Vec<usize> = Vec::new();
        for &g in gs {
            if g < groups && !who.contains(&g) {
                who.push(g);
            }
        }
        if own.is_empty() || who.is_empty() {
            return false;
        }
        let ordered = self.along(&own);
        let n = ordered.len();
        // (each army a stretch as long as its divisions call for, at least one province: an army of
        // twenty-four does not hold as much as one of six)
        let count: Vec<f32> = who.iter().map(|&g| self.corps.as_ref().map_or(1.0, |c| c.divs.iter().filter(|d| d.side == s && d.group == g).count().max(1) as f32)).collect();
        let all: f32 = count.iter().sum();
        let mut ends = Vec::with_capacity(who.len());
        let mut acc = 0.0;
        for x in &count {
            acc += x;
            ends.push(((acc / all * n as f32).round() as usize).min(n));
        }
        for (i, &g) in who.iter().enumerate() {
            // (stretch i: from the end of the one before; more armies than provinces share the last)
            let a = if i == 0 { 0 } else { ends[i - 1] }.min(n - 1);
            let b = ends[i].max(a + 1).min(n);
            let piece: Vec<usize> = ordered[a..b].to_vec();
            self.take_sector_as(s, g, &piece, true);
            let gr = &mut self.command[s].as_mut().unwrap()[g];
            gr.staff = false;
            gr.facing = facing;
            // (put on the border with another neighbour, it holds there: its old attack was
            // against the enemy)
            if facing.is_some() && matches!(gr.order, Order::Attack(_)) {
                gr.order = Order::Hold;
            }
        }
        true
    }

    /// The player disbands group `g` of side `s` (28th session): its divisions go to the staff's
    /// reserve, its stretch to the group before it (the one after, if it was the first), its blows
    /// are called off; the groups after it take its place in the list. Its general, if it had one
    /// (None, and nothing done, if it is the only group).
    pub fn disband(&mut self, s: usize, g: usize) -> Option<Option<General>> {
        let n = self.command[s].as_ref().map_or(0, |v| v.len());
        if g >= n || n <= 1 {
            return None;
        }
        let gs = self.command[s].as_mut().unwrap();
        let gone = gs.remove(g);
        let heir = g.saturating_sub(1).min(gs.len() - 1);
        for p in gone.anchor {
            if !gs[heir].anchor.contains(&p) {
                gs[heir].anchor.push(p);
            }
        }
        if let Some(c) = self.corps.as_mut() {
            for d in c.divs.iter_mut().filter(|d| d.side == s && d.group != NONE) {
                if d.group == g {
                    d.group = NONE;
                    d.manual = false;
                } else if d.group > g {
                    d.group -= 1;
                }
            }
            c.blows[s].retain(|b| b.group != g);
            for b in c.blows[s].iter_mut() {
                if b.group > g {
                    b.group -= 1;
                }
            }
            c.muster_group[s] = match c.muster_group[s] {
                Some(m) if m == g => None,
                Some(m) if m > g => Some(m - 1),
                x => x,
            };
        }
        Some(gone.general)
    }

    /// The player forms a new group of armies of side `s` of its divisions `ids`, `general` at
    /// its head: its sector is the stretch of the front nearest where they stand (taken from the
    /// groups that had it); it holds there and is his (the staff leaves it be, and sends it no new
    /// divisions). Its number, if formed.
    pub fn raise(&mut self, s: usize, ids: &[u32], general: Option<General>) -> Option<usize> {
        let c = self.corps.as_ref()?;
        self.command[s].as_ref()?;
        let at: Vec<usize> = ids.iter().filter_map(|&id| c.find(id)).filter(|&i| c.divs[i].side == s).map(|i| c.divs[i].at).collect();
        if at.is_empty() {
            return None;
        }
        // (each division's nearest province of the front)
        let front = self.front(s);
        let lab = self.nearest_of(&front);
        let mut anchor: Vec<usize> = at.iter().map(|&p| if lab[p] == usize::MAX { p } else { lab[p] }).collect();
        anchor.sort_unstable();
        anchor.dedup();
        let gs = self.command[s].as_mut().unwrap();
        gs.push(crate::military::front::Group { anchor: Vec::new(), share: 0.0, order: Order::Hold, strike: 1.0 - crate::military::front::LINE_ATTACK, staff: false, general, facing: None });
        let k = gs.len() - 1;
        self.take_sector(s, k, &anchor);
        self.assign(s, ids, k);
        Some(k)
    }
}
