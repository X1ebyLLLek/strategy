//! The front cut into sectors and given to groups of armies: each group's stretch, its
//! general, the distances over the land, the threat the enemy makes to a province.

use super::*;

impl Campaign {
    /// The front of side `s` cut into sectors along the line, about `size` provinces each: each
    /// stretch of the front that hangs together is walked from one end (the province farthest
    /// along it); a sector grows from the first province not yet taken over its neighbours on the
    /// front, nearest the end first, until it has `size`; scraps of less than half a sector join a
    /// neighbouring sector. (Cut by bands of distance along the line, a ragged front at the end
    /// of a war fell into pieces that did not hang together.)
    pub fn sectors(&self, s: usize, size: usize) -> Vec<Vec<usize>> {
        let n = self.owner.len();
        let size = size.max(1);
        let front = self.front(s);
        let mut on = vec![false; n];
        for &p in &front {
            on[p] = true;
        }
        let along = |from: usize| -> Vec<u32> {
            let mut d = vec![NO; n];
            d[from] = 0;
            let mut q = VecDeque::from([from]);
            while let Some(p) = q.pop_front() {
                for &r in &self.th.nbr[p] {
                    if on[r] && d[r] == NO {
                        d[r] = d[p] + 1;
                        q.push_back(r);
                    }
                }
            }
            d
        };
        let mut sector = vec![usize::MAX; n];
        let mut out: Vec<Vec<usize>> = Vec::new();
        let mut seen = vec![false; n];
        for &p0 in &front {
            if seen[p0] {
                continue;
            }
            let d0 = along(p0);
            let stretch: Vec<usize> = front.iter().copied().filter(|&p| d0[p] != NO).collect();
            for &p in &stretch {
                seen[p] = true;
            }
            let end = *stretch.iter().max_by_key(|&&p| (d0[p], p)).unwrap();
            let d = along(end);
            let mut order = stretch.clone();
            order.sort_by_key(|&p| (d[p], p));
            for &start in &order {
                if sector[start] != usize::MAX {
                    continue;
                }
                let k = out.len();
                let mut part = vec![start];
                sector[start] = k;
                let mut edge: Vec<usize> = self.th.nbr[start].iter().copied().filter(|&r| on[r] && sector[r] == usize::MAX).collect();
                while part.len() < size {
                    edge.retain(|&r| sector[r] == usize::MAX);
                    let Some(i) = (0..edge.len()).min_by_key(|&i| (d[edge[i]], edge[i])) else { break };
                    let r = edge.swap_remove(i);
                    sector[r] = k;
                    part.push(r);
                    edge.extend(self.th.nbr[r].iter().copied().filter(|&x| on[x] && sector[x] == usize::MAX));
                }
                out.push(part);
            }
        }
        // scraps join the smallest neighbouring sector, if one touches them
        for k in 0..out.len() {
            if out[k].is_empty() || out[k].len() * 2 >= size {
                continue;
            }
            let near = out[k]
                .iter()
                .flat_map(|&p| self.th.nbr[p].iter().copied())
                .filter(|&r| on[r] && sector[r] != k && sector[r] != usize::MAX)
                .map(|r| sector[r])
                .min_by_key(|&j| (out[j].len(), j));
            if let Some(j) = near {
                let moved = std::mem::take(&mut out[k]);
                for &p in &moved {
                    sector[p] = j;
                }
                out[j].extend(moved);
            }
        }
        out.retain(|part| !part.is_empty());
        out
    }

    /// Side `s` leads its troops by groups of armies: one on each sector of its front as it is now,
    /// each told to hold, with an even share of the army; `staff`: its staff gives them their
    /// shares and orders from now on.
    pub fn by_groups(&mut self, s: usize, staff: bool) {
        let cut = self.sectors(s, ((SECTOR as f32 / self.scale).round() as usize).max(1));
        let k = cut.len().max(1) as f32;
        self.command[s] = Some(cut.into_iter().map(|anchor| Group { anchor, share: (1.0 - STAFF_RESERVE) / k, order: Order::Hold, strike: 1.0 - LINE_ATTACK, staff: true, general: None, facing: None }).collect());
        self.staff_led[s] = staff;
    }

    /// Side `s` leads its divisions by armies, as in HOI4: as many as it takes for no general to
    /// lead more than `ARMY` of them, each on a stretch of the front as it is now (the front cut
    /// into that many), told to hold, with an even share; `staff`: its staff gives them their
    /// shares and orders from now on. (Cut by the front's length, six provinces to a group, a long
    /// front gave a small army many generals and a short front a big army few, each with a hundred
    /// divisions and more.)
    pub fn by_armies(&mut self, s: usize, staff: bool) {
        let n = match &self.corps {
            Some(c) => c.divs.iter().filter(|d| d.side == s).count(),
            // (no divisions yet: about one to a unit of the army)
            None => self.sides[s].army.count.iter().sum::<f32>().round() as usize,
        };
        let k = n.div_ceil(ARMY).max(1).min(self.most_groups.max(1));
        let front = self.front(s).len().max(1);
        let mut cut = self.sectors(s, front.div_ceil(k).max(1));
        cut.truncate(self.most_groups.max(1));
        let k = cut.len().max(1) as f32;
        self.command[s] = Some(cut.into_iter().map(|anchor| Group { anchor, share: (1.0 - STAFF_RESERVE) / k, order: Order::Hold, strike: 1.0 - LINE_ATTACK, staff: true, general: None, facing: None }).collect());
        self.staff_led[s] = staff;
    }

    /// The player takes group `g` of side `s` from the staff and tells it `order` (told to attack,
    /// it strikes with the usual share of its troops).
    pub fn tell(&mut self, s: usize, g: usize, order: Order) {
        if let Some(gr) = self.command[s].as_mut().and_then(|v| v.get_mut(g)) {
            gr.staff = false;
            gr.order = order;
            gr.strike = 1.0 - LINE_ATTACK;
        }
    }

    /// The group of side `s` nearest to each province of the theatre (over the theatre's
    /// provinces, from the groups' anchors); usize::MAX where none reaches or the side has none.
    /// (A group facing another country than the enemy reaches out from nowhere: it holds just
    /// the provinces of its anchor.)
    pub fn group_of(&self, s: usize) -> Vec<usize> {
        let n = self.owner.len();
        let mut lab = vec![usize::MAX; n];
        let Some(groups) = &self.command[s] else { return lab };
        let mut q = VecDeque::new();
        for (k, g) in groups.iter().enumerate() {
            if g.facing.is_some() {
                continue;
            }
            for &p in &g.anchor {
                if lab[p] == usize::MAX {
                    lab[p] = k;
                    q.push_back(p);
                }
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
        for (k, g) in groups.iter().enumerate().filter(|(_, g)| g.facing.is_some()) {
            for &p in &g.anchor {
                if p < n && self.owner[p] == s {
                    lab[p] = k;
                }
            }
        }
        lab
    }

    /// The front of side `s` shared out among its groups (one list for each group): each province
    /// to the group whose anchor is nearest; a stretch no anchor reaches, to the first group. A
    /// group facing another country holds the provinces of its anchor; the stretch no group
    /// facing the enemy reaches goes to the first that does (none: nobody holds it).
    pub fn sectors_of(&self, s: usize) -> Vec<Vec<usize>> {
        let Some(groups) = &self.command[s] else { return Vec::new() };
        let lab = self.group_of(s);
        let mut out = vec![Vec::new(); groups.len()];
        let facing = groups.iter().any(|g| g.facing.is_some());
        let first = groups.iter().position(|g| g.facing.is_none());
        for p in self.front(s) {
            if !facing {
                out[if lab[p] == usize::MAX { 0 } else { lab[p] }].push(p);
            } else if lab[p] != usize::MAX {
                out[lab[p]].push(p);
            } else if let Some(k) = first {
                out[k].push(p);
            }
        }
        for (k, g) in groups.iter().enumerate().filter(|(_, g)| g.facing.is_some()) {
            out[k] = g.anchor.iter().copied().filter(|&p| self.owner[p] == s).collect();
        }
        out
    }

    /// The general of the group of side `s` that province `p` belongs to, if any.
    pub fn general_at(&self, s: usize, p: usize) -> Option<&General> {
        let gs = self.command[s].as_ref()?;
        let lab = self.group_of(s);
        gs.get(lab[p]).and_then(|g| g.general.as_ref())
    }

    /// Steps from province `t` to every province of the theatre, over all of them.
    pub fn distance_from(&self, t: usize) -> Vec<u32> {
        self.distance_from_all(&[t])
    }

    /// Steps from the nearest of provinces `from` to every province of the theatre.
    pub fn distance_from_all(&self, from: &[usize]) -> Vec<u32> {
        let mut d = vec![NO; self.owner.len()];
        let mut q = VecDeque::new();
        for &t in from {
            if d[t] == NO {
                d[t] = 0;
                q.push_back(t);
            }
        }
        while let Some(p) = q.pop_front() {
            for &r in &self.th.nbr[p] {
                if d[r] == NO {
                    d[r] = d[p] + 1;
                    q.push_back(r);
                }
            }
        }
        d
    }

    /// The enemy seen across province `p` of side `s`'s front (each enemy province's troops shared
    /// among the provinces of ours it touches).
    pub(crate) fn threat(&self, s: usize, p: usize, seen: &[f32]) -> f32 {
        let o = 1 - s;
        self.th.nbr[p].iter().filter(|&&q| self.owner[q] == o).map(|&q| seen[q] / self.th.nbr[q].iter().filter(|&&r| self.owner[r] == s).count().max(1) as f32).sum()
    }
}
