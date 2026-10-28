//! The market: what every buyer bids at the prices, what is brought to it (made at home,
//! imported), and the prices at which the two meet.

use super::*;

impl Nation {
    /// What every buyer offers at prices `p`.
    pub(super) fn bids(&self, pack: &Pack, mo: &Month, p: &[f32], imports: &[f32]) -> Bids {
        let (ng, ns, nk) = (pack.groups.len(), pack.sectors.len(), pack.goods.len());
        let on = |j: usize| mo.offered[j] + imports[j] > NONE;
        let dear = |j: usize, e: f32| (p[j] / (pack.goods[j].price * self.level).max(1e-9)).powf(-e);
        let can_build = |s: usize| pack.sectors[s].build.iter().all(|&(i, _)| on(i));
        let mut b = Bids {
            total: vec![0.0; nk],
            group: vec![vec![0.0; nk]; ng],
            invest: vec![vec![vec![0.0; nk]; ns]; ng],
            sector: vec![vec![0.0; nk]; ns],
            mil: vec![0.0; nk],
            build: vec![0.0; nk],
        };
        // what each group has left after its needs, and what everyone means to build:
        // (group or None for the state, sector, money)
        let mut left = vec![0.0f32; ng];
        let mut projects: Vec<(Option<usize>, usize, f32)> = Vec::new();
        for (g, grp) in pack.groups.iter().enumerate() {
            let pop = self.pop[g];
            if pop <= 0.0 {
                continue;
            }
            let want = |n: &Need| {
                if on(n.good) {
                    (pop * n.amount * dear(n.good, if n.vital { ELASTIC_VITAL } else { ELASTIC_SOFT }).min(SATED) - self.kind[g][n.good]).max(0.0)
                } else {
                    0.0
                }
            };
            // the budget stretches to cover what keeps them alive, as far as the money goes
            let vital: f32 = grp.needs.iter().filter(|n| n.vital).map(|n| want(n) * p[n.good]).sum();
            let mut m = self.money[g].max(0.0).min(mo.budget[g].max(vital));
            // what keeps them alive first; then the money goes to the other needs in shares by
            // how much each weighs, and what a need does not take (it is cheap, or they have
            // enough of it) goes to the rest. Bought in the order of the pack, horses came after
            // luxury, the money ran out before them, nobody bought a horse even at a hundredth
            // of its price and breeders died out everywhere; bought in order of what eases want
            // most per coin, a price a hair's breadth higher swapped two needs and what was
            // spent on each jumped: prices leapt to the month's bound and back
            for n in grp.needs.iter().filter(|n| n.vital) {
                let q = want(n).min(m / p[n.good]);
                b.group[g][n.good] += q * p[n.good];
                m -= q * p[n.good];
            }
            let mut rest: Vec<&Need> = grp.needs.iter().filter(|n| !n.vital && want(n) > 0.0).collect();
            while !rest.is_empty() && m > 0.0 {
                let all: f32 = rest.iter().map(|n| n.weight).sum::<f32>().max(1e-12);
                let full: Vec<&Need> = rest.iter().copied().filter(|n| want(n) * p[n.good] <= m * n.weight / all).collect();
                if full.is_empty() {
                    for n in &rest {
                        b.group[g][n.good] += m * n.weight / all;
                    }
                    m = 0.0;
                    break;
                }
                for n in &full {
                    b.group[g][n.good] += want(n) * p[n.good];
                    m -= want(n) * p[n.good];
                }
                rest.retain(|n| !full.iter().any(|f| std::ptr::eq(*f, *n)));
            }
            m = m.max(0.0);
            // what is left: some into new capacity, the rest into living better
            let plans: Vec<(usize, f32)> = mo.invest[g]
                .iter()
                .filter(|&&(s, _)| can_build(s))
                .map(|&(s, jobs)| (s, jobs * build_cost(pack, s, p)))
                .collect();
            let full: f32 = plans.iter().map(|x| x.1).sum();
            let k = if full > 0.0 { (m * INVEST_SHARE * self.invest_k / full).min(1.0) } else { 0.0 };
            projects.extend(plans.iter().map(|&(s, money)| (Some(g), s, k * money)));
            left[g] = m;
        }
        if let Some((s, money, jobs)) = mo.build {
            if can_build(s) {
                projects.push((None, s, money.min(jobs * build_cost(pack, s, p))));
            }
        }
        // building takes at most a share of any good on the market in a month; the rest of
        // the project waits (or the state, building fields at 2% a month, bought a third of
        // all iron at once every few months and the gunsmiths stood idle)
        let mut asked = vec![0.0f32; nk];
        for &(_, s, money) in &projects {
            let cost = build_cost(pack, s, p);
            for &(i, a) in &pack.sectors[s].build {
                asked[i] += money * a / cost;
            }
        }
        let fit: Vec<f32> =
            (0..nk).map(|i| if asked[i] > 0.0 { (BUILD_SHARE * mo.offered[i] / asked[i]).min(1.0) } else { 1.0 }).collect();
        for &(who, s, money) in &projects {
            let cost = build_cost(pack, s, p);
            let k = pack.sectors[s].build.iter().map(|&(i, _)| fit[i]).fold(1.0, f32::min);
            for &(i, a) in &pack.sectors[s].build {
                let x = k * money * a * p[i] / cost;
                match who {
                    Some(g) => {
                        b.invest[g][s][i] += x;
                        left[g] -= x;
                    }
                    None => b.build[i] += x,
                }
            }
        }
        for (g, grp) in pack.groups.iter().enumerate() {
            let m = left[g].max(0.0);
            if self.pop[g] <= 0.0 {
                continue;
            }
            // living better: on what is not vital (one does not eat twice as much), in fixed
            // shares of the money, so a cheaper good is bought more
            let soft = grp.needs.iter().any(|n| !n.vital && on(n.good));
            let extra = |n: &&Need| on(n.good) && (!n.vital || !soft);
            let worth = |n: &Need| n.amount * pack.goods[n.good].price;
            let basket: f32 = grp.needs.iter().filter(extra).map(worth).sum();
            if basket > 0.0 {
                for n in grp.needs.iter().filter(extra) {
                    b.group[g][n.good] += m * worth(n) / basket;
                }
            }
        }
        for (s, sec) in pack.sectors.iter().enumerate() {
            // dear inputs are saved on (cheap ones are not hoarded beyond the store); inputs
            // dearer than the output is worth: buy only what the output can pay for. The
            // output is valued at its recent prices, not today's (or makers chase every swing)
            let value: f32 = sec.outputs.iter().map(|&(j, a)| a * self.trend[j]).sum::<f32>() * self.prod[s] * self.shock[s];
            let cost: f32 = sec.inputs.iter().map(|&(i, a)| a * p[i]).sum();
            let pays = if cost > 0.0 { (value / cost).min(1.0) } else { 1.0 };
            for &(i, _) in &sec.inputs {
                if on(i) {
                    b.sector[s][i] += mo.restock[s][i] * dear(i, ELASTIC_INPUT).min(1.0) * pays * p[i];
                }
            }
            // a boost is bought in full while it returns twice its cost
            for &(i, a, gain) in &sec.boost {
                if on(i) {
                    b.sector[s][i] += mo.restock[s][i] * (gain * value / (2.0 * a * p[i])).min(1.0) * p[i];
                }
            }
            let spend: f32 = b.sector[s].iter().sum();
            if spend > self.cash[s].max(0.0) {
                let k = self.cash[s].max(0.0) / spend;
                b.sector[s].iter_mut().for_each(|x| *x *= k);
            }
        }
        if self.army_wants.is_empty() {
            let mil: Vec<usize> = (0..nk).filter(|&i| pack.goods[i].military && on(i)).collect();
            for &i in &mil {
                // the army buys less of what got dear, like anybody (at a fixed sum, the price of
                // powder ran up 30 times when a trickle of it was all there was)
                b.mil[i] = mo.mil / mil.len() as f32 * dear(i, ELASTIC_SOFT).min(1.0);
            }
        } else {
            for i in (0..nk).filter(|&i| self.army_wants[i] > 0.0 && on(i)) {
                b.mil[i] = mo.mil * self.army_wants[i] * dear(i, ELASTIC_SOFT).min(1.0);
            }
        }
        for j in 0..nk {
            b.total[j] = b.mil[j]
                + b.build[j]
                + (0..ng).map(|g| b.group[g][j] + b.invest[g].iter().map(|v| v[j]).sum::<f32>()).sum::<f32>()
                + (0..ns).map(|s| b.sector[s][j]).sum::<f32>();
        }
        b
    }

    /// What home sellers put out at price `p`: all they brought at the recent price or above,
    /// less below it (the rest waits in store), and above it also from their store. Without
    /// the first, where buyers take the same amount at any price — fed and clothed people
    /// buying what they need — the price was not determined at all, and fuel and services
    /// swapped places every month.
    fn home_out(&self, mo: &Month, j: usize, p: f32) -> f32 {
        mo.home[j] * (p / self.trend[j].max(1e-9)).powf(KEEP_SLOPE).min(1.0) + self.drawn(mo, j, p)
    }

    /// Everything on sale at price `p`: what home sellers put out, less what merchants take
    /// abroad, and foreign goods.
    pub(super) fn supply(&self, mo: &Month, j: usize, p: f32, trade: &Trade) -> f32 {
        (self.home_out(mo, j, p) - trade.exports[j]).max(0.0) + trade.imports[j]
    }

    /// Above the recent price sellers also sell from their store, all of it at half as much
    /// again (buyers of inputs pay nearly any price, so a shortage of a few percent tripled
    /// the price of tools while seven months of them lay in store).
    fn drawn(&self, mo: &Month, j: usize, p: f32) -> f32 {
        let spare = (mo.avail[j] - mo.home[j]).max(0.0);
        spare * ((p / self.trend[j].max(1e-9) - 1.0) / (CEIL - 1.0)).clamp(0.0, 1.0)
    }

    /// How far prices may move this month: not below 70% of last month's (nor below half the
    /// sellers' recent price), not above one and a half times it.
    pub fn bounds(&self, pack: &Pack) -> (Vec<f32>, Vec<f32>) {
        let floor: Vec<f32> = (0..pack.goods.len())
            .map(|j| (self.price[j] * FLOOR).max(self.price[j].min((LOWEST * self.trend[j]).max(0.01 * pack.goods[j].price * self.level))))
            .collect();
        let ceil: Vec<f32> = self.price.iter().map(|p| p * CEIL).collect();
        if self.price_cap.iter().all(|&c| c <= 0.0) {
            return (floor, ceil);
        }
        // price control: never above the cap (what is short is shared out, as in any shortage)
        let mut floor = floor;
        let mut ceil = ceil;
        for j in 0..pack.goods.len() {
            let cap = self.price_cap.get(j).copied().unwrap_or(0.0);
            if cap > 0.0 {
                ceil[j] = ceil[j].min(cap * pack.goods[j].price * self.level);
                floor[j] = floor[j].min(ceil[j]);
            }
        }
        (floor, ceil)
    }

    /// For a world looking for the prices of all trading countries at once: what buyers here
    /// offer for each good at prices `p` (with `imports` on sale), and what home sellers put out.
    pub fn market(&self, pack: &Pack, mo: &Month, p: &[f32], imports: &[f32]) -> (Vec<f32>, Vec<f32>) {
        let b = self.bids(pack, mo, p, imports);
        (b.total, (0..pack.goods.len()).map(|j| self.home_out(mo, j, p[j])).collect())
    }

    /// The prices at which the money offered for each good buys exactly what is on sale (or what
    /// sells at the lowest price sellers accept), with `trade` leaving and arriving as given.
    pub(super) fn clear(&mut self, pack: &Pack, mo: &Month, trade: &Trade) -> Vec<f32> {
        let nk = pack.goods.len();
        let (floor, ceil) = self.bounds(pack);
        let mut p = self.price.clone();
        let mut seek = vec![Seek::new(); nk];
        self.solver = (0, 0.0);
        for step in 0..SEARCH_STEPS {
            let b = self.bids(pack, mo, &p, &trade.imports);
            let mut moved = 0.0f32;
            for j in 0..nk {
                if mo.offered[j] + trade.imports[j] > NONE {
                    let next = seek[j].step(p[j], b.total[j], self.supply(mo, j, p[j], trade), floor[j], ceil[j]);
                    moved = moved.max((next / p[j]).ln().abs());
                    p[j] = next;
                }
            }
            self.solver = (step + 1, moved);
            if moved < 1e-4 {
                break;
            }
        }
        p
    }

    /// The month's prices from where the search for them stopped, and what everyone gets: at
    /// the price sellers ask there may be not enough for all — then everyone gets the same
    /// share of what they asked for, and keeps the rest of the money.
    pub(super) fn settle_prices(&self, pack: &Pack, mo: &Month, p: &[f32], trade: &Trade) -> (Vec<f32>, Bids) {
        let nk = pack.goods.len();
        let (floor, ceil) = self.bounds(pack);
        let mut b = self.bids(pack, mo, p, &trade.imports);
        let price: Vec<f32> = (0..nk)
            .map(|j| {
                // nothing on the market: no price of its own, just the usual one — but what the
                // army keeps asking for and nobody brings gets dearer as a shortage would, so
                // merchants abroad see what it would fetch here (at half as much again as the
                // usual price, a poor country never got a single rifle: rifles cost twice the
                // usual everywhere)
                if mo.offered[j] + trade.imports[j] > NONE {
                    (b.total[j] / self.supply(mo, j, p[j], trade).max(1e-9)).clamp(floor[j], ceil[j])
                } else if self.army_wants.get(j).is_some_and(|&w| w > 0.0) {
                    (self.price[j] * CEIL).min(UNFILLED * pack.goods[j].price * self.level)
                } else {
                    pack.goods[j].price * self.level
                }
            })
            .collect();
        for j in 0..nk {
            let paid = self.supply(mo, j, price[j], trade) * price[j];
            if b.total[j] > paid && b.total[j] > 0.0 {
                let r = paid / b.total[j];
                for row in b.group.iter_mut().chain(b.invest.iter_mut().flatten()).chain(b.sector.iter_mut()) {
                    row[j] *= r;
                }
                b.mil[j] *= r;
                b.build[j] *= r;
                b.total[j] = paid;
            }
        }
        (price, b)
    }
}
