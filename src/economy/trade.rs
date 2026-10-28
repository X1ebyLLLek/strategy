//! Trade between countries. Every country keeps its own market and its own prices; goods go
//! from where they are cheap to where they are dear. Part of what is carried is lost on the
//! way (more by land than by sea, more in mountains than on plains, more for bulky goods).
//!
//! The prices of all trading countries are looked for together, month by month, the way one
//! country's market looks for its own: merchants send the more on a way, the more a good
//! fetches at its far end above what it costs at home after the way and their margin, until
//! the gap is no more than the way costs. The goods arrive the same month.
//!
//! (The first version had merchants who decided from last month's prices and sent goods that
//! arrived months later. Goods on the way and goods waiting unsold piled up between two
//! markets where buyers take about the same amount at any price, and every way to steer them
//! went round in circles: trade made most countries' prices swing. Months on the way will
//! come back for blockades and war, on top of prices found this way.)
//!
//! Foreign goods are sold like home goods; the money they fetch goes home to their makers next
//! month, less the margin of the merchants who sold them. Money never appears or vanishes.
//! Nothing here knows any era: the costs of the way come from the pack.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use rayon::prelude::*;
use crate::economy::nation::{Month, Nation, Seek, Trade as Exchange, SEARCH_STEPS, SHIP_MAX};
use crate::pack::{Pack, Trade};
use crate::world::countries::Politics;
use crate::world::map;

/// A way from one country to another.
#[derive(Clone, Copy, Debug)]
pub struct Route {
    /// A good of bulk 1 arrives as exp(-loss) of what left.
    pub loss: f32,
    pub months: u32,
}

/// Less than this (a thousand units a month) is no trade.
const NONE: f32 = 1e-3;
/// How much goes on a way per unit of the gap: at a gap of 1/KAPPA, as much as the far market
/// takes in a month. The larger, the nearer prices come to differing by just the way.
const KAPPA: f32 = 20.0;
/// Where a good is not on sale, merchants see a market of this share of what people and
/// sectors there would take at usual prices.
const SEEN: f32 = 0.25;
/// Share of the way to the new flows taken per step of the search (the flows move the prices
/// that move the flows).
const DAMP: f32 = 0.5;
/// Merchants keep to the ways they trade on: in a month the flow on a way changes by at most this
/// share of last month's, plus a new consignment of at most `FLOW_NEW` of the smaller of the two
/// markets (what sellers bring at home, what buyers at the far end would take at usual prices). (Free to change any flow at once, merchants split a big
/// buyer's imports among small sellers of nearly the same price anew every month: one seller sent
/// four times as much one month and nothing the next, and its bread jumped by a fifth. Measured by
/// the seller's market, a new consignment to a small buyer could be as big as all it takes.)
/// (`NOINERTIA=1` in the environment: merchants free to change any flow at once, as before — for
/// controls.)
const FLOW_PACE: f32 = 0.25;
/// The step merchants take narrows by this each step of the search, so the flows settle by its
/// end. (Where many small sellers send to one big buyer and his market caps them all, the flows
/// chased one another to the last step, and the month ended wherever they happened to be.)
const SETTLE: f32 = 0.95;
const FLOW_NEW: f32 = 0.05;

#[derive(Clone)]
pub struct World {
    pub nations: Vec<Nation>,
    /// [from][to]
    pub routes: Vec<Vec<Option<Route>>>,
    /// Share of each good that arrives on each way: [from][to][good].
    keep: Vec<Vec<Vec<f32>>>,
    /// Goods that left on each way last month: [from][to][good].
    pub flow: Vec<Vec<Vec<f32>>>,
    /// Money on its way home for goods sold abroad: [nation][good].
    pending: Vec<Vec<f32>>,
    /// What arrived per month, smoothed, at usual prices: [from][to].
    pub carried: Vec<Vec<f32>>,
    pub month: u32,
    /// Steps the search for prices took last month, and how far prices still moved.
    pub solver: (u32, f32),
    /// Where the search moved most at its last step: the country and good of the price, and the
    /// way (from, to, good) whose flow moved most against the seller's market.
    pub worst: (usize, usize, usize, usize, usize, f32),
    /// [from][to]: goods on this way pay no duty where they arrive (a trade agreement); empty:
    /// no agreements.
    pub free: Vec<Vec<bool>>,
}

impl World {
    pub fn new(pack: &Pack, nations: Vec<Nation>, routes: Vec<Vec<Option<Route>>>) -> World {
        let (n, nk) = (nations.len(), pack.goods.len());
        let keep = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        (0..nk)
                            .map(|k| match (routes[i][j], &pack.trade) {
                                (Some(r), Some(_)) if pack.goods[k].tradable => (-r.loss * pack.goods[k].bulk).exp(),
                                _ => 0.0,
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();
        World {
            nations,
            routes,
            keep,
            flow: vec![vec![vec![0.0; nk]; n]; n],
            pending: vec![vec![0.0; nk]; n],
            carried: vec![vec![0.0; n]; n],
            month: 0,
            solver: (0, 0.0),
            worst: (0, 0, 0, 0, 0, 0.0),
            free: Vec::new(),
        }
    }

    /// Goods go between countries `i` and `j` again, by the ways between them (an embargo lifted).
    pub fn reopen(&mut self, pack: &Pack, i: usize, j: usize) {
        for (a, b) in [(i, j), (j, i)] {
            for k in 0..pack.goods.len() {
                self.keep[a][b][k] = match (self.routes[a][b], &pack.trade) {
                    (Some(r), Some(_)) if pack.goods[k].tradable => (-r.loss * pack.goods[k].bulk).exp(),
                    _ => 0.0,
                };
            }
        }
    }

    /// The way between countries `i` and `j` carries only `k` of what it would by its own losses, both
    /// ways (ships sunk on it, a blockade of a port: merchants see the losses and carry less); 1 —
    /// as the way is. A way cut stays cut.
    pub fn hamper(&mut self, pack: &Pack, i: usize, j: usize, k: f32) {
        let k = k.clamp(0.01, 1.0);
        for (a, b) in [(i, j), (j, i)] {
            if self.keep[a][b].iter().all(|&x| x <= 0.0) {
                continue;
            }
            for g in 0..pack.goods.len() {
                self.keep[a][b][g] = match (self.routes[a][b], &pack.trade) {
                    (Some(r), Some(_)) if pack.goods[g].tradable => (-r.loss * pack.goods[g].bulk).exp() * k,
                    _ => 0.0,
                };
            }
        }
    }

    /// Share of good `g` sent from `i` that arrives at `j` now (0: none goes).
    pub fn kept(&self, i: usize, j: usize, g: usize) -> f32 {
        self.keep[i][j][g]
    }

    /// No goods go between countries `i` and `j` any more (a war between them, a blockade).
    pub fn cut(&mut self, i: usize, j: usize) {
        for (a, b) in [(i, j), (j, i)] {
            self.keep[a][b].iter_mut().for_each(|k| *k = 0.0);
            self.flow[a][b].iter_mut().for_each(|f| *f = 0.0);
        }
    }

    /// Money on its way home to country `c` for its goods sold abroad, by good: it arrives next
    /// month.
    pub fn pending(&self, c: usize) -> &[f32] {
        &self.pending[c]
    }

    /// All money in the world: in the countries and on its way home. Must never change.
    pub fn money_total(&self) -> f64 {
        self.nations.iter().map(|x| x.money_total() as f64).sum::<f64>()
            + self.pending.iter().flatten().map(|&x| x as f64).sum::<f64>()
    }

    /// Pre-history: each country lives `alone` months on its own, then `together` months
    /// trading, before the game starts.
    pub fn settle(&mut self, pack: &Pack, alone: u32, together: u32) {
        self.nations.par_iter_mut().for_each(|x| {
            x.live = false;
            for _ in 0..alone {
                x.step(pack);
            }
            // a country alone can have any level of prices: only what goods cost against each
            // other matters there. Before the ways open its money is counted at what it buys
            // (or a poor country whose prices stood ten times above its neighbour's, having
            // as much money and fewer goods, sent all of it abroad within a year, its prices
            // fell tenfold, and its fields were ruined)
            if alone > 0 {
                x.rebase(1.0 / x.level.max(1e-6));
            }
        });
        // (ways that opened by degrees did worse: nothing moved for years, then trade started
        // all at once when the gap outgrew what the way still ate)
        for _ in 0..together {
            self.step(pack);
        }
        self.nations.iter_mut().for_each(|x| x.begin());
        self.month = 0;
    }

    pub fn step(&mut self, pack: &Pack) {
        let (n, nk) = (self.nations.len(), pack.goods.len());
        self.month += 1;
        let margin = pack.trade.as_ref().map_or(0.0, |t| t.margin);
        for (c, x) in self.nations.iter_mut().enumerate() {
            for k in 0..nk {
                x.remit_in[k] += self.pending[c][k];
                self.pending[c][k] = 0.0;
            }
        }
        let mos: Vec<Month> = self.nations.par_iter_mut().map(|x| x.prepare(pack)).collect();
        let bounds: Vec<(Vec<f32>, Vec<f32>)> = self.nations.iter().map(|x| x.bounds(pack)).collect();
        let mut p: Vec<Vec<f32>> = self.nations.iter().map(|x| x.price.clone()).collect();
        // last month's trade is where the search starts
        let mut f = self.flow.clone();
        let keep = &self.keep;
        let arriving = |f: &Vec<Vec<Vec<f32>>>| -> Vec<Vec<f32>> {
            (0..n).map(|c| (0..nk).map(|k| (0..n).map(|i| f[i][c][k] * keep[i][c][k]).sum()).collect()).collect()
        };
        // where a good is not on sale at all, buyers would pay for the first of it as much as a
        // price may rise in a month: merchants judge the far end by that (at the usual price an
        // absent good showed no gain at all, and a country without iron never got any tools)
        let first = arriving(&f);
        for c in 0..n {
            for k in 0..nk {
                if mos[c].brought()[k] + first[c][k] <= NONE {
                    p[c][k] = bounds[c].1[k];
                }
            }
        }
        self.solver = (0, 0.0);
        let mut seek = vec![vec![Seek::new(); nk]; n];
        // what merchants keep of a sale at each end after its duty
        let free_of_duty: Vec<Vec<f32>> = self.nations.iter().map(|x| (0..nk).map(|k| 1.0 - x.duty.get(k).copied().unwrap_or(0.0).clamp(0.0, 0.9)).collect()).collect();
        // the same for every flow: a step towards what merchants want, halved each time the flow
        // turns back (more goods lower the price at the far end, which lowers what they want to
        // send twenty times as much: half of the way to it overshot forever)
        let mut pace = vec![vec![vec![DAMP; nk]; n]; n];
        let mut turn = vec![vec![vec![0.0f32; nk]; n]; n];
        // a country whose prices have settled and whose trade did not move is left alone (so a
        // country nobody can reach finds the same prices as alone, to the last digit)
        let mut done = vec![false; n];
        let free = std::env::var_os("NOINERTIA").is_some();
        let pacts = &self.free;
        let duty_free = |i: usize, j: usize| pacts.get(i).and_then(|r| r.get(j)).copied().unwrap_or(false);
        for step in 0..SEARCH_STEPS {
            let imports = arriving(&f);
            let nations = &self.nations;
            let mk: Vec<(Vec<f32>, Vec<f32>)> =
                (0..n).into_par_iter().map(|c| nations[c].market(pack, &mos[c], &p[c], &imports[c])).collect();
            // what a good is worth to merchants at each end: abroad no more than the price at
            // which all that is on sale there sells (at a price held up by the month's floor the
            // rest finds no buyer), at home no less than the price at which what is left sells
            // (a good rationed at home is worth more than its capped price)
            let on_sale = |c: usize, k: usize| (mk[c].1[k] - (0..n).map(|j| f[c][j][k]).sum::<f32>()).max(0.0) + imports[c][k];
            let clears: Vec<Vec<f32>> = (0..n)
                .map(|c| {
                    (0..nk)
                        .map(|k| {
                            let q = on_sale(c, k);
                            if q > NONE { mk[c].0[k] / q } else { p[c][k] }
                        })
                        .collect()
                })
                .collect();
            // what merchants would send on each way at these prices: the more, the wider the
            // gap after the way and their margin; at most SHIP_MAX of what sellers put out, and
            // into a market no more than sells there at the lowest price it may take this month
            let mut want = vec![vec![vec![0.0f32; nk]; n]; n];
            for i in 0..n {
                for k in 0..nk {
                    let home = mk[i].1[k];
                    if home <= NONE {
                        continue;
                    }
                    let near = p[i][k].max(clears[i][k]).max(1e-9);
                    for j in 0..n {
                        if keep[i][j][k] <= 0.0 {
                            continue;
                        }
                        let gap = p[j][k].min(clears[j][k]) * keep[i][j][k] * (1.0 - margin) * if duty_free(i, j) { 1.0 } else { free_of_duty[j][k] } / near - 1.0;
                        if gap > 0.0 {
                            let market = (mk[j].0[k] / p[j][k].max(1e-9)).max(SEEN * mos[j].potential()[k]);
                            want[i][j][k] = KAPPA * market * gap;
                        }
                    }
                    let all: f32 = (0..n).map(|j| want[i][j][k]).sum();
                    if all > SHIP_MAX * home {
                        (0..n).for_each(|j| want[i][j][k] *= SHIP_MAX * home / all);
                    }
                }
            }
            for j in 0..n {
                for k in 0..nk {
                    let coming: f32 = (0..n).map(|i| want[i][j][k] * keep[i][j][k]).sum();
                    if coming <= 0.0 {
                        continue;
                    }
                    let home_left = (mk[j].1[k] - (0..n).map(|d| f[j][d][k]).sum::<f32>()).max(0.0);
                    // where it is not on sale at all, nobody bids for it yet: a first consignment
                    // of the market merchants see there
                    let room = if home_left + imports[j][k] <= NONE {
                        SEEN * mos[j].potential()[k]
                    } else {
                        (mk[j].0[k] / bounds[j].0[k].max(1e-9) - home_left).max(0.0)
                    };
                    if coming > room {
                        (0..n).for_each(|i| want[i][j][k] *= room / coming);
                    }
                }
            }
            let narrow = DAMP * SETTLE.powi(step as i32);
            let mut moved_f = vec![0.0f32; n];
            let mut worst_f = (0usize, 0usize, 0usize, 0.0f32);
            for i in 0..n {
                let home: Vec<f32> = (0..nk).map(|k| mk[i].1[k].max(NONE)).collect();
                for j in 0..n {
                    for k in 0..nk {
                        let dir = want[i][j][k] - f[i][j][k];
                        if dir * turn[i][j][k] < 0.0 {
                            pace[i][j][k] *= 0.5;
                        } else if turn[i][j][k] != 0.0 {
                            pace[i][j][k] = (pace[i][j][k] * 1.2).min(DAMP);
                        }
                        if dir != 0.0 {
                            turn[i][j][k] = dir.signum();
                        }
                        let last = self.flow[i][j][k];
                        let next = if free {
                            f[i][j][k] + pace[i][j][k] * dir
                        } else {
                            (f[i][j][k] + pace[i][j][k].min(narrow) * dir).clamp(last * (1.0 - FLOW_PACE), last * (1.0 + FLOW_PACE) + FLOW_NEW * home[k])
                        };
                        let d = (next - f[i][j][k]).abs() / home[k];
                        if d > worst_f.3 {
                            worst_f = (i, j, k, d);
                        }
                        moved_f[i] = moved_f[i].max(d);
                        moved_f[j] = moved_f[j].max(d);
                        f[i][j][k] = if next < NONE * 1e-2 { 0.0 } else { next };
                    }
                }
            }
            // the prices at which what is offered meets what is on sale, home and foreign
            let mut moved = 0.0f32;
            let mut worst_p = (0usize, 0usize, 0.0f32);
            for c in 0..n {
                if done[c] && moved_f[c] < 1e-3 {
                    continue;
                }
                let mut moved_c = 0.0f32;
                for k in 0..nk {
                    if mos[c].brought()[k] + imports[c][k] <= NONE {
                        continue;
                    }
                    let out: f32 = (0..n).map(|j| f[c][j][k]).sum();
                    let on_sale = (mk[c].1[k] - out).max(0.0) + imports[c][k];
                    let next = seek[c][k].step(p[c][k], mk[c].0[k], on_sale, bounds[c].0[k], bounds[c].1[k]);
                    let d = (next / p[c][k]).ln().abs();
                    if d > worst_p.2 {
                        worst_p = (c, k, d);
                    }
                    moved_c = moved_c.max(d);
                    p[c][k] = next;
                }
                done[c] = moved_c < 1e-4 && moved_f[c] < 1e-3;
                moved = moved.max(moved_c);
            }
            self.solver = (step + 1, moved);
            self.worst = (worst_p.0, worst_p.1, worst_f.0, worst_f.1, worst_f.2, worst_f.3);
            if done.iter().all(|&d| d) {
                break;
            }
        }

        let imports = arriving(&f);
        // the share of each good's imports that came by duty-free ways
        if !self.free.is_empty() {
            for c in 0..n {
                let ex: Vec<f32> = (0..nk)
                    .map(|k| {
                        let all = imports[c][k];
                        if all <= 0.0 {
                            return 0.0;
                        }
                        (0..n).filter(|&i| self.free.get(i).and_then(|r| r.get(c)).copied().unwrap_or(false)).map(|i| f[i][c][k] * self.keep[i][c][k]).sum::<f32>() / all
                    })
                    .collect();
                self.nations[c].exempt = ex;
            }
        }
        let trades: Vec<Exchange> = (0..n)
            .map(|c| Exchange { exports: (0..nk).map(|k| (0..n).map(|j| f[c][j][k]).sum()).collect(), imports: imports[c].clone() })
            .collect();
        self.nations
            .par_iter_mut()
            .zip(mos.into_par_iter())
            .zip(p.into_par_iter())
            .zip(trades.par_iter())
            .for_each(|(((x, mo), p), t)| x.finish(pack, mo, p, t));

        // the money foreign goods fetched goes home next month, to each exporter as much as
        // its goods were of what arrived
        let mut arrived = vec![vec![0.0f32; n]; n];
        for c in 0..n {
            for k in 0..nk {
                let all = imports[c][k];
                if all <= 0.0 {
                    continue;
                }
                let paid = self.nations[c].remit_out[k];
                for i in 0..n {
                    let q = f[i][c][k] * self.keep[i][c][k];
                    if q > 0.0 {
                        self.pending[i][k] += paid * q / all;
                        arrived[i][c] += q * pack.goods[k].price;
                    }
                }
            }
        }
        for i in 0..n {
            for j in 0..n {
                self.carried[i][j] = 0.9 * self.carried[i][j] + 0.1 * arrived[i][j];
            }
        }
        self.flow = f;
    }

    /// For a good not on sale in country `j`: what it would cost there from the cheapest
    /// country that makes it (after the way and the margin), against the most buyers there would
    /// pay for a first consignment. Below 1 merchants should have brought it.
    pub fn reach(&self, pack: &Pack, j: usize, k: usize) -> Option<f32> {
        let t = pack.trade.as_ref()?;
        let pay = self.nations[j].bounds(pack).1[k];
        (0..self.nations.len())
            .filter(|&i| self.keep[i][j][k] > 0.0 && self.nations[i].produced[k] > NONE)
            .map(|i| self.nations[i].price[k] / (self.keep[i][j][k] * (1.0 - t.margin)) / pay)
            .min_by(|a, b| a.total_cmp(b))
    }

    /// Price at the far end against the near end, after the way and the margin: 1 means
    /// nothing left to gain by carrying more. For a route and a good.
    pub fn gap(&self, pack: &Pack, i: usize, j: usize, k: usize) -> Option<f32> {
        let t = pack.trade.as_ref()?;
        if self.keep[i][j][k] <= 0.0 {
            return None;
        }
        let (a, b) = (&self.nations[i], &self.nations[j]);
        if a.offered[k] <= NONE || b.offered[k] <= NONE {
            return None;
        }
        Some(b.price[k] * self.keep[i][j][k] * (1.0 - t.margin) / a.price[k])
    }
}

/// Ways between capitals over the map's provinces: the cheapest by loss of goods, with the
/// months it takes.
pub fn routes(world: &map::World, pol: &Politics, t: &Trade) -> Vec<Vec<Option<Route>>> {
    let np = world.provinces.len();
    let w = world.w as f32;
    let at = |p: usize| {
        let s = world.provinces[p].seed;
        ((s % world.w) as f32, (s / world.w) as f32)
    };
    let dist = |a: usize, b: usize| {
        let ((xa, ya), (xb, yb)) = (at(a), at(b));
        let dx = (xa - xb).abs();
        let dx = dx.min(w - dx);
        (dx * dx + (ya - yb) * (ya - yb)).sqrt()
    };
    let rough = |p: usize| t.terrain[world.provinces[p].terrain as usize];
    // (loss, months) of one step between neighbouring provinces
    let edge = |p: usize, q: usize| {
        let d = dist(p, q);
        let (sp, sq) = (world.provinces[p].sea, world.provinces[q].sea);
        if sp && sq {
            (d * t.sea, d / t.sea_speed)
        } else if !sp && !sq {
            let r = 0.5 * (rough(p) + rough(q));
            (d * t.land * r, d * r / t.land_speed)
        } else {
            let land = if sp { q } else { p };
            (0.5 * d * (t.sea + t.land * rough(land)) + t.port, 0.5 * d * (1.0 / t.sea_speed + rough(land) / t.land_speed))
        }
    };
    let caps: Vec<usize> = pol.countries.iter().map(|c| c.capital as usize).collect();
    caps.par_iter()
        .map(|&src| {
            let mut loss = vec![f32::INFINITY; np];
            let mut time = vec![0.0f32; np];
            let mut heap = BinaryHeap::new();
            loss[src] = 0.0;
            heap.push(Reverse((0u32, src)));
            while let Some(Reverse((bits, p))) = heap.pop() {
                if f32::from_bits(bits) > loss[p] {
                    continue;
                }
                for &q in &world.provinces[p].neighbors {
                    let q = q as usize;
                    let (dl, dt) = edge(p, q);
                    let l = loss[p] + dl;
                    if l < loss[q] {
                        loss[q] = l;
                        time[q] = time[p] + dt;
                        heap.push(Reverse((l.to_bits(), q)));
                    }
                }
            }
            caps.iter()
                .map(|&dst| {
                    (dst != src && loss[dst].is_finite())
                        .then(|| Route { loss: loss[dst], months: (time[dst].ceil() as u32).max(1) })
                })
                .collect()
        })
        .collect()
}
