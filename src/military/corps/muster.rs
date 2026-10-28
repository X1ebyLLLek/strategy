//! The army raised for the war: the divisions each side has when the war begins (by its
//! templates, the staff's cadre for the rest), where they stand, and the templates changed
//! during the war.

use super::*;

impl Campaign {
    /// The war begins with divisions: each side's army as its staff made it, formed into
    /// divisions of a template of its own make-up (`cadre`) — what does not fill a whole one stays
    /// in the pool; each side's templates to build from now on and how much of each (`shapes`,
    /// `wants`; empty: its staff's own, by its plan); each group of armies takes its share of the
    /// divisions on its sector of the front, the rest stands behind it.
    pub fn muster(&mut self, pack: &Pack, cadre: [&str; 2], shapes: [Vec<Shape>; 2], wants: [Vec<f32>; 2]) {
        let nu = pack.units.len();
        let mut c = Corps {
            pool: [vec![0.0; nu], vec![0.0; nu]],
            kit: [vec![0.0; nu], vec![0.0; nu]],
            trophies: [vec![0.0; nu], vec![0.0; nu]],
            abandoned: [vec![0.0; nu], vec![0.0; nu]],
            lost: [vec![0.0; nu], vec![0.0; nu]],
            lost_kit: [vec![0.0; nu], vec![0.0; nu]],
            lack: [[0.3; 6]; 2],
            level: [Vec::new(), Vec::new()],
            ..Default::default()
        };
        for s in 0..2 {
            let army = self.sides[s].army.count.clone();
            // (the army's equipment: what its factories made it with)
            c.kit[s] = (0..nu).map(|u| army[u] * self.sides[s].norm_of(u)).collect();
            c.pool[s] = army.clone();
            // the cadre: the army as it stands, in divisions as big as it and the front call for
            let mut first = self.staff_shape(pack, &c, s, cadre[s]);
            first.units = Shape::sized(cadre[s], &army, first.battalions()).units;
            let mut list = vec![first];
            let mut w = vec![0.0];
            c.by_plan[s] = shapes[s].is_empty();
            if shapes[s].is_empty() {
                // (the staff builds by its plan: a template of what it buys, as big as its army
                // and its front call for)
                let sh = self.staff_shape(pack, &c, s, "Дивизия генштаба");
                list.push(sh);
                w.push(1.0);
            } else {
                list.extend(shapes[s].iter().cloned());
                w.extend(wants[s].iter().copied());
            }
            c.shapes[s] = list;
            c.wants[s] = w;
            // as many whole divisions of the cadre as the army fills (its companies too)
            let extra = 1.0 + c.shapes[s][0].overhead(pack);
            let full = &c.shapes[s][0].units;
            let k = (0..nu).filter(|&u| full[u] > 0.0).map(|u| (army[u] / (full[u] * extra)).floor()).fold(f32::INFINITY, f32::min);
            let k = if k.is_finite() { k as usize } else { 0 };
            for _ in 0..k {
                let has = c.shapes[s][0].units.clone();
                let kit: Vec<f32> = (0..nu).map(|u| (has[u] * c.shapes[s][0].norm_of(u)).min(c.kit[s][u] / extra)).collect();
                for u in 0..nu {
                    c.pool[s][u] = (c.pool[s][u] - has[u] * extra).max(0.0);
                    c.kit[s][u] = (c.kit[s][u] - kit[u] * extra).max(0.0);
                }
                let cap = self.th.capital[s];
                Campaign::enlist(&mut c, s, 0, has, kit, cap, 1.0, 1.0);
            }
        }
        self.corps = Some(c);
        self.deploy(pack);
    }

    /// The groups take their shares of the divisions, standing along their sectors; the rest
    /// stands behind the front (a war's first day: they are there already).
    pub fn deploy(&mut self, pack: &Pack) {
        let Some(mut c) = self.corps.take() else { return };
        for s in 0..2 {
            let ids: Vec<usize> = (0..c.divs.len()).filter(|&i| c.divs[i].side == s).collect();
            let n = ids.len();
            let parts = self.sectors_of(s);
            let shares: Vec<f32> = self.command[s].as_ref().map_or(Vec::new(), |gs| gs.iter().map(|g| g.share).collect());
            let mut next = 0;
            for (g, part) in parts.iter().enumerate() {
                let k = ((shares.get(g).copied().unwrap_or(0.0) * n as f32).round() as usize).min(n - next);
                for j in 0..k {
                    let i = ids[next + j];
                    c.divs[i].group = g;
                    if !part.is_empty() {
                        c.divs[i].at = part[j % part.len()];
                    }
                }
                next += k;
            }
            let behind = self.behind(s, &self.front(s));
            for (j, &i) in ids[next..].iter().enumerate() {
                if !behind.is_empty() {
                    c.divs[i].at = behind[j % behind.len()];
                }
            }
        }
        self.corps = Some(c);
        self.sync_divs(pack);
    }

    /// A new division of template `shape` of side `s` with `has` and `kit`, standing in `at`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn enlist(c: &mut Corps, s: usize, shape: usize, has: Vec<f32>, kit: Vec<f32>, at: usize, org: f32, drill: f32) -> u32 {
        c.numbers[s] += 1;
        let id = c.next;
        c.next += 1;
        c.divs.push(Division { id, side: s, no: c.numbers[s], shape, has, kit, org, at, way: Vec::new(), gone: 0.0, group: NONE, manual: false, exp: 0.0, drill, dug: 0.0, dead: 0.0, left: 0.0, days: 0, rest: 0.0, supplied: 1.0, busy: false });
        id
    }

    /// Where side `s`'s recruits gather and drill: the capital or the depot nearest the front that
    /// stands two steps behind the line (one, if none does); with all of them on the line or next to
    /// it, the province of its own nearest the capital that is far enough. (Gathered at a capital
    /// the front had reached, recruits never drilled — they cannot under fire — and a quarter of an
    /// army stood there for months, green, in one province.)
    pub(super) fn camp(&self, s: usize) -> usize {
        let front = self.front(s);
        let df = self.distance_from_all(&front);
        let reach = self.reach(s);
        let own = |p: usize| p < self.owner.len() && self.owner[p] == s && reach[p] != NO;
        let cap = self.th.capital[s];
        for far in [2, 1] {
            let best = std::iter::once(cap)
                .chain(self.hubs[s].iter().copied())
                .filter(|&p| own(p) && df[p] >= far && df[p] != NO)
                .min_by_key(|&p| (df[p], p));
            if let Some(p) = best {
                return p;
            }
        }
        // (none of them: the nearest province to the capital far enough from the line)
        let dc = self.distance_from(cap);
        for far in [2, 1] {
            if let Some(p) = (0..self.owner.len()).filter(|&p| own(p) && df[p] >= far && df[p] != NO).min_by_key(|&p| (dc[p], p)) {
                return p;
            }
        }
        cap
    }

    /// Provinces of side `s` next to its front that touch no enemy (just behind the line).
    pub(super) fn behind(&self, s: usize, line: &[usize]) -> Vec<usize> {
        let mut b: Vec<usize> = line
            .iter()
            .flat_map(|&p| self.th.nbr[p].iter().copied())
            .filter(|&r| self.owner[r] == s && !self.th.nbr[r].iter().any(|&x| self.owner[x] != s))
            .collect();
        b.sort_unstable();
        b.dedup();
        if b.is_empty() {
            b = line.to_vec();
        }
        b
    }

    /// The templates side `s` builds from now on, and how much of each (the first, its cadre,
    /// stays: its divisions are refilled by it).
    pub fn set_shapes(&mut self, s: usize, shapes: Vec<Shape>, wants: Vec<f32>) {
        let Some(c) = self.corps.as_mut() else { return };
        c.by_plan[s] = shapes.is_empty();
        if shapes.is_empty() {
            return;
        }
        let mut list = vec![c.shapes[s][0].clone()];
        let mut w = vec![0.0];
        // a template the divisions already have keeps its place (they are refilled by it)
        for old in c.shapes[s].iter().skip(1) {
            list.push(old.clone());
            w.push(0.0);
        }
        for (sh, wt) in shapes.into_iter().zip(wants) {
            match list.iter().position(|x| x.name == sh.name && x.units == sh.units && x.norm == sh.norm && x.support == sh.support) {
                Some(k) => w[k] = wt,
                None => {
                    list.push(sh);
                    w.push(wt);
                }
            }
        }
        c.shapes[s] = list;
        c.wants[s] = w;
    }
}
