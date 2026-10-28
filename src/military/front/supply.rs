//! Supply from the capital through one's own land (and by railways), what a province gets;
//! the odds of an attack by the staff's war games.

use super::*;

impl Campaign {
    /// Steps from side `s`'s capital through its own provinces (NO where cut off).
    pub(crate) fn reach(&self, s: usize) -> Vec<u32> {
        let mut d = vec![NO; self.owner.len()];
        let cap = self.th.capital[s];
        if self.owner[cap] != s {
            return d;
        }
        let mut q = VecDeque::from([cap]);
        d[cap] = 0;
        while let Some(p) = q.pop_front() {
            for &r in &self.th.nbr[p] {
                if self.owner[r] == s && d[r] == NO {
                    d[r] = d[p] + 1;
                    q.push_back(r);
                }
            }
        }
        d
    }

    /// Steps of the way of supply of side `s` to each of its provinces: from its capital, or from
    /// one of its depots that the capital reaches (a depot counts as a step from the capital).
    fn supply_way(&self, s: usize) -> Vec<u32> {
        let d = self.reach(s);
        let hubs: Vec<usize> = self.hubs[s].iter().copied().filter(|&h| h < self.owner.len() && self.owner[h] == s && d[h] != NO).collect();
        if hubs.is_empty() {
            return d;
        }
        let mut way = d.clone();
        let mut q = VecDeque::new();
        for &h in &hubs {
            if way[h] > 1 {
                way[h] = 1;
            }
        }
        // (a way of unit steps from sources at 0 and 1: the capital's distances first, then the
        // depots' — relaxed until nothing is shorter)
        for p in 0..self.owner.len() {
            if way[p] != NO {
                q.push_back(p);
            }
        }
        while let Some(p) = q.pop_front() {
            for &r in &self.th.nbr[p] {
                if self.owner[r] == s && way[r] != NO && way[p] + 1 < way[r] {
                    way[r] = way[p] + 1;
                    q.push_back(r);
                }
            }
        }
        way
    }

    /// The way of supply of side `s` along the lines laid (`grade`): as `supply_way`, but a step
    /// between two provinces on lines costs their grade — the cheapest way from the capital and
    /// the depots (NO where none).
    fn supply_way_lines(&self, s: usize) -> Vec<f32> {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;
        let n = self.owner.len();
        let reach = self.reach(s);
        let mut way = vec![f32::INFINITY; n];
        let mut heap: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();
        let cap = self.th.capital[s];
        if reach[cap] != NO {
            way[cap] = 0.0;
            heap.push(Reverse((0.0f32.to_bits(), cap)));
        }
        for &h in &self.hubs[s] {
            if h < n && self.owner[h] == s && reach[h] != NO && way[h] > 1.0 {
                way[h] = 1.0;
                heap.push(Reverse((1.0f32.to_bits(), h)));
            }
        }
        let g = |p: usize| self.grade.get(p).copied().unwrap_or(1.0);
        while let Some(Reverse((k, p))) = heap.pop() {
            if f32::from_bits(k) > way[p] {
                continue;
            }
            for &r in &self.th.nbr[p] {
                if self.owner[r] != s {
                    continue;
                }
                let step = if g(p) < 1.0 && g(r) < 1.0 { g(p).max(g(r)) } else { 1.0 };
                let nw = way[p] + step;
                if nw < way[r] {
                    way[r] = nw;
                    heap.push(Reverse((nw.to_bits(), r)));
                }
            }
        }
        way.iter().map(|&x| if x.is_finite() { x } else { NO as f32 }).collect()
    }

    /// How well side `s`'s troops in each province are supplied.
    pub fn supply(&self, s: usize) -> Vec<f32> {
        // (with lines laid the way is counted in their grades; without — in steps, as always)
        let d: Vec<f32> = if self.grade.is_empty() { self.supply_way(s).iter().map(|&x| if x == NO { NO as f32 } else { x as f32 }).collect() } else { self.supply_way_lines(s) };
        (0..self.owner.len())
            .map(|p| {
                // what the way from the capital brings, of what the country's stores give
                self.stores[s]
                    * if d[p] != NO as f32 {
                        let far = (d[p] * self.scale - SUPPLY_NEAR).max(0.0) / SUPPLY_FAR;
                        (1.0 - (1.0 - SUPPLY_LEAST) * far).max(SUPPLY_LEAST)
                    } else if self.th.home[p] == s {
                        DEPOTS
                    } else {
                        POCKET
                    }
            })
            .collect()
    }

    pub(super) fn worth(pack: &Pack, t: &[f32]) -> f32 {
        t.iter().zip(&pack.units).map(|(n, u)| n * u.cost).sum()
    }

    /// The staff of side `s` reckons its odds of taking province `q` with `group` (supplied
    /// `fed`) against the defenders it believes are there: `seen` worth of the enemy's army
    /// as it sees its make-up.
    pub(crate) fn odds(&self, pack: &Pack, s: usize, q: usize, group: &[f32], fed: f32, seen: f32, foe_fed: f32, seed: u64) -> f32 {
        let foe_dug = self.dug[1 - s][q];
        let staff = &self.sides[s];
        let nu = pack.units.len();
        let per: f32 = (0..nu).map(|u| staff.foe_mix[u] * pack.units[u].cost).sum::<f32>().max(1e-9);
        let ours = Army { count: group.to_vec(), supplied: fed, fire: staff.rated(&staff.own_fire), tough: staff.rated(&staff.own_tough), dug: 0.0 };
        let theirs = Army {
            count: (0..nu).map(|u| seen * staff.wariness.exp() * staff.foe_mix[u] / per).collect(),
            supplied: foe_fed,
            fire: staff.rated(&staff.foe_fire),
            tough: staff.rated(&staff.foe_tough),
            dug: foe_dug,
        };
        let mut won = 0;
        for g in 0..GAMES {
            let mut rng = Rng::new(hash64(seed ^ (g as u64 * 0x9E37_79B9)));
            if battle(pack, self.th.terrain[q], [&ours, &theirs], Kind::Attack, &mut rng, false).winner == Some(0) {
                won += 1;
            }
        }
        won as f32 / GAMES as f32
    }
}
