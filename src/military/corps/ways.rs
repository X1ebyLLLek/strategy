//! The ways on the ground: how many divisions a province feeds, the days a step takes, the
//! days to every province and the way there.

use super::*;

impl Campaign {
    /// What province `p` can feed of side `s`'s troops (units that eat): so many full lines of a
    /// plain, more on a railway, near the capital or a depot.
    pub fn cap_at(&self, pack: &Pack, s: usize, p: usize) -> f32 {
        let war = pack.war.as_ref().expect("в наборе нет раздела [war]");
        let mut cap = CAP_LINES * war.width * self.scale;
        if self.grade.get(p).is_some_and(|&g| g < 1.0) {
            cap *= 1.5;
        }
        if p == self.th.capital[s] || self.hubs[s].contains(&p) {
            cap *= 1.5;
        }
        cap
    }

    /// Days a division with `has` needs to go from province `a` to its neighbour `b`.
    pub fn step_days(&self, pack: &Pack, has: &[f32], a: usize, b: usize) -> f32 {
        STEP_DAYS * self.scale * self.way_cost(a, b) / Corps::speed(pack, has)
    }

    /// The cost of a step between neighbours: the ground entered, less along railways.
    fn way_cost(&self, a: usize, b: usize) -> f32 {
        let g = |p: usize| self.grade.get(p).copied().unwrap_or(1.0);
        let rail = if g(a) < 1.0 && g(b) < 1.0 { g(a).max(g(b)).max(0.25) } else { 1.0 };
        ground_cost(self.th.terrain[b]) * rail
    }

    /// Days of march (at speed 1) from every province to `to`, over the provinces side `s` holds
    /// (`own`), or over all of them, the enemy's dearer (INFINITY where none leads).
    pub(super) fn days_to(&self, s: usize, to: usize, own: bool) -> Vec<f32> {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;
        let n = self.owner.len();
        let mut d = vec![f32::INFINITY; n];
        let mut heap: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();
        d[to] = 0.0;
        heap.push(Reverse((0, to)));
        while let Some(Reverse((k, p))) = heap.pop() {
            if f32::from_bits(k) > d[p] {
                continue;
            }
            for &r in &self.th.nbr[p] {
                if own && self.owner[r] != s {
                    continue;
                }
                // (from r into p: the ground of p)
                let enemy = if self.owner[p] != s { 1.5 } else { 1.0 };
                let nd = d[p] + STEP_DAYS * self.scale * self.way_cost(r, p) * enemy;
                if nd < d[r] {
                    d[r] = nd;
                    heap.push(Reverse((nd.to_bits(), r)));
                }
            }
        }
        d
    }

    /// The way from `from` to the province the map `d` leads to (each step to the neighbour
    /// nearest it), without `from` itself; empty when none.
    pub(super) fn way_by(&self, s: usize, from: usize, d: &[f32], own: bool) -> Vec<usize> {
        let mut way = Vec::new();
        let mut p = from;
        if !d[p].is_finite() {
            return way;
        }
        while d[p] > 0.0 && way.len() < 200 {
            let Some(next) = self.th.nbr[p].iter().copied().filter(|&r| (!own || self.owner[r] == s) && d[r] < d[p]).min_by(|&a, &b| d[a].total_cmp(&d[b]).then(a.cmp(&b))) else { break };
            way.push(next);
            p = next;
        }
        way
    }

    /// The way divisions of side `s` standing in `from` would take to `to`, as `send` lays it
    /// (over one's own land where it leads there, else through the enemy's), without `from`;
    /// empty when none leads there. For the map's preview of an order.
    pub fn way_to(&self, s: usize, from: usize, to: usize) -> Vec<usize> {
        let n = self.owner.len();
        if to >= n || from >= n || from == to {
            return Vec::new();
        }
        if self.owner[to] == s {
            let d = self.days_to(s, to, true);
            if d[from].is_finite() {
                return self.way_by(s, from, &d, true);
            }
        }
        let d = self.days_to(s, to, false);
        self.way_by(s, from, &d, false)
    }
}
