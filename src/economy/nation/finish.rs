//! The month's second half, at the market's prices: goods shared out, wages and profits paid,
//! stores and capacity changed, people's discontent and unrest, births and deaths.

use super::*;

impl Nation {
    /// The rest of a month, once the prices are found (`p`, where the search stopped): who
    /// buys what, who earns what, discontent, people moving.
    pub fn finish(&mut self, pack: &Pack, mo: Month, p: Vec<f32>, trade: &Trade) {
        let (ng, ns, nk) = (pack.groups.len(), pack.sectors.len(), pack.goods.len());
        let (price, b) = self.settle_prices(pack, &mo, &p, trade);
        let absent: Vec<usize> = (0..nk).filter(|&j| mo.offered[j] + trade.imports[j] <= NONE).collect();
        self.wished = vec![0.0; nk];
        if !absent.is_empty() {
            let (mut p2, mut trial) = (price.clone(), trade.imports.clone());
            for &j in &absent {
                p2[j] = price[j] * CEIL;
                trial[j] = 1.0;
            }
            let w = self.bids(pack, &mo, &p2, &trial);
            for &j in &absent {
                self.wished[j] = w.total[j];
            }
        }
        self.price = price;
        let p = &self.price;
        let (invest, build, made, used, used_goods, jobs) = (&mo.invest, mo.build, &mo.made, &mo.used, &mo.used_goods, &mo.jobs);
        self.shipped = trade.exports.clone();
        self.import = trade.imports.clone();
        let supply: Vec<f32> = (0..nk).map(|j| self.supply(&mo, j, p[j], trade)).collect();
        self.unsold = (0..nk).map(|j| (supply[j] - b.total[j] / p[j]).max(0.0)).collect();
        // what was bought is shared between home and foreign goods as they were on sale
        let foreign: Vec<f32> = (0..nk).map(|j| if supply[j] > 0.0 { (trade.imports[j] / supply[j]).min(1.0) } else { 0.0 }).collect();
        self.import_sold = (0..nk).map(|j| b.total[j] / p[j] * foreign[j]).collect();
        let home_sold: Vec<f32> = (0..nk).map(|j| b.total[j] / p[j] - self.import_sold[j]).collect();
        for j in 0..nk {
            self.sold[j] = 0.7 * self.sold[j] + 0.3 * (home_sold[j] + self.shipped[j]);
        }
        self.offered = supply.clone();
        let got = |x: f32, j: usize| x / p[j];
        self.bought_by = (0..nk)
            .map(|j| {
                [
                    (0..ng).map(|g| b.group[g][j]).sum::<f32>() / p[j],
                    (0..ng).map(|g| b.invest[g].iter().map(|v| v[j]).sum::<f32>()).sum::<f32>() / p[j],
                    (0..ns).map(|s| b.sector[s][j]).sum::<f32>() / p[j],
                    b.mil[j] / p[j],
                    b.build[j] / p[j],
                ]
            })
            .collect();
        let jobs_of = |spend: &[f32], s: usize| {
            pack.sectors[s].build.iter().map(|&(i, a)| got(spend[i], i) / a).fold(f32::INFINITY, f32::min)
        };
        for (g, grp) in pack.groups.iter().enumerate() {
            for (k, n) in grp.needs.iter().enumerate() {
                let want = self.pop[g] * n.amount;
                let had = got(b.group[g][n.good], n.good) + self.kind[g][n.good];
                self.unmet[g][k] = if want > 0.0 { 1.0 - (had / want).min(1.0) } else { 0.0 };
            }
            self.money[g] -= b.group[g].iter().sum::<f32>() + b.invest[g].iter().flatten().sum::<f32>();
            self.spent[g] = b.group[g].clone();
            for &(s, _) in &invest[g] {
                let j = jobs_of(&b.invest[g][s], s);
                if j.is_finite() && j > 0.0 {
                    self.building.push_back((self.month + BUILD_MONTHS, s, j));
                }
            }
        }
        for s in 0..ns {
            for i in 0..nk {
                self.inv[s][i] += got(b.sector[s][i], i);
            }
            self.cash[s] -= b.sector[s].iter().sum::<f32>();
        }
        for i in 0..nk {
            self.arsenal[i] += got(b.mil[i], i);
        }
        self.army_spent = b.mil.iter().sum::<f32>();
        self.built = b.build.iter().sum::<f32>();
        self.treasury -= b.mil.iter().sum::<f32>() + b.build.iter().sum::<f32>();
        if let Some((s, _, _)) = build {
            let j = jobs_of(&b.build, s);
            if j.is_finite() && j > 0.0 {
                self.building.push_back((self.month + BUILD_MONTHS, s, j));
            }
        }
        for j in 0..nk {
            self.stock[j] = (mo.avail[j] - self.shipped[j] - home_sold[j]).max(0.0) * (1.0 - SPOIL);
            if self.stock[j] < NONE {
                self.stock[j] = 0.0;
            }
        }

        // 7. sellers earn what was paid, in proportion to what each made, and what their
        // goods fetched abroad; what foreign goods fetched here goes abroad, less the margin
        // of the merchants who sold them; workers get their share of what the sector added,
        // the sector keeps cash for inputs, the owner the rest
        let mut revenue = vec![0.0f32; ns];
        let mut income = vec![0.0f32; ng];
        let mut margins = 0.0f32;
        let mut duties = 0.0f32;
        let mut public = 0.0f32;
        for i in 0..nk {
            let abroad = b.total[i] * foreign[i];
            let margin = pack.trade.as_ref().map_or(0.0, |t| abroad * t.margin);
            if let Some(t) = &pack.trade {
                income[t.owner] += margin;
            }
            margins += margin;
            self.remit_out[i] = abroad - margin;
            // the duty on foreign goods stays at home, in the treasury
            if let Some(&d) = self.duty.get(i) {
                if d > 0.0 {
                    let take = (abroad - margin) * d.min(0.9) * (1.0 - self.exempt.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0));
                    self.remit_out[i] -= take;
                    self.treasury += take;
                    duties += take;
                }
            }
            let earned = b.total[i] - abroad + self.remit_in[i];
            let makers: f32 = (0..ns).map(|s| made[s][i]).sum();
            if makers > 0.0 {
                for s in 0..ns {
                    revenue[s] += earned * made[s][i] / makers;
                }
            } else {
                // sold from store, nobody made it this month: by capacity, or to the state
                let caps: f32 = (0..ns).filter(|&s| pack.sectors[s].outputs.iter().any(|o| o.0 == i)).map(|s| self.cap[s]).sum();
                for s in 0..ns {
                    if caps > 0.0 && pack.sectors[s].outputs.iter().any(|o| o.0 == i) {
                        revenue[s] += earned * self.cap[s] / caps;
                    }
                }
                if caps <= 0.0 {
                    self.treasury += earned;
                }
            }
            self.remit_in[i] = 0.0;
        }
        let mut paid = vec![0.0f32; ng];
        let mut profit_tax = vec![0.0f32; ng];
        let mut payout = 0.0f32;
        let mut employed = vec![0.0f32; ng];
        // what people took in kind was made and used too (what workers took from their sector
        // counts in its value added below; what the idle grew for themselves, here)
        let kind_all: f32 = (0..ng).map(|g| (0..nk).map(|j| self.kind[g][j] * p[j]).sum::<f32>()).sum();
        let kind_paid: Vec<f32> = (0..ns).map(|s| (0..nk).map(|j| mo.in_kind[s][j] * p[j]).sum()).collect();
        self.gdp = margins + kind_all - kind_paid.iter().sum::<f32>();
        self.gdp_real = (0..ng).map(|g| (0..nk).map(|j| self.kind[g][j] * pack.goods[j].price).sum::<f32>()).sum();
        for (s, sec) in pack.sectors.iter().enumerate() {
            // what the workers took in kind is part of what the sector made, and part of their
            // wages: counted in money only, a field whose bread fed its own peasants showed a
            // loss as soon as it bought imported tools, and closed while the tools paid
            let inputs: f32 = (0..nk).map(|i| used_goods[s][i] * p[i]).sum();
            let added = revenue[s] + kind_paid[s] - inputs;
            self.gdp += added;
            self.gdp_real += (0..nk).map(|j| (made[s][j] - used_goods[s][j]) * pack.goods[j].price).sum::<f32>();
            let share = self.wage_share(pack, s);
            let wages = (share * added.max(0.0)).max(kind_paid[s]);
            let in_money = wages - kind_paid[s];
            for &(g, w) in &sec.workers {
                income[g] += in_money * w;
                paid[g] += wages * w;
                employed[g] += used[s] * w;
            }
            self.profit[s] = added - wages;
            self.cash[s] += revenue[s] - in_money;
            let keep = WORKING_CAPITAL * (0..nk).map(|i| restock_full(pack, s, i) * self.cap[s] * p[i]).sum::<f32>();
            let dividend = (self.cash[s] - keep).max(0.0);
            self.cash[s] -= dividend;
            // (what the state owns of the sector, its share of the dividend goes to the treasury)
            // (the tax on profit comes off what the sector pays out, before its owners count it)
            payout += dividend;
            let levy = dividend * self.profit_tax.clamp(0.0, 0.9);
            let dividend = dividend - levy;
            if levy > 0.0 {
                self.treasury += levy;
                profit_tax[sec.owner] += levy;
            }
            match self.public.get(s) {
                Some(&k) if k > 0.0 => {
                    let ours = dividend * k.min(1.0);
                    self.treasury += ours;
                    public += ours;
                    income[sec.owner] += dividend - ours;
                }
                _ => income[sec.owner] += dividend,
            }
            if self.profit[s] < 0.0 {
                self.cap[s] *= 1.0 - CLOSURES;
            }
        }
        for g in 0..ng {
            if jobs[g] > 0.0 {
                // those serving in the army are not out of work
                self.unemployed[g] = Some((1.0 - employed[g] / (self.pop[g] - self.drafted[g] - self.scholars[g]).max(1e-6)).clamp(0.0, 1.0));
                self.pay[g] = paid[g] / employed[g].max(1e-6);
            }
            self.income[g] = income[g];
            let tax = match self.tax_by.get(g) {
                Some(&k) => income[g].max(0.0) * (self.tax * k).clamp(0.0, 0.95),
                None => income[g].max(0.0) * self.tax,
            };
            self.treasury += tax;
            self.money[g] += income[g] - tax;
            // the till takes its share of what the group spent, the estate tax a share of what it holds
            let till: f32 = match self.sales_tax.is_empty() {
                true => 0.0,
                false => (0..nk).map(|j| self.spent[g][j] * self.sales_tax.get(j).copied().unwrap_or(0.0)).sum(),
            };
            let till = till.min(self.money[g].max(0.0));
            self.money[g] -= till;
            self.treasury += till;
            let estate = (self.money[g].max(0.0) * self.estate_tax.get(g).copied().unwrap_or(0.0)).clamp(0.0, self.money[g].max(0.0));
            self.money[g] -= estate;
            self.treasury += estate;
            if let Some(t) = self.taxed.get_mut(g) {
                *t = tax + till + estate + profit_tax[g];
            }
            if let Some(t) = self.taxed_by.get_mut(g) {
                *t = [tax, till, profit_tax[g], estate];
            }
        }
        self.payout = payout;
        self.duties = duties;
        self.public_money = public;
        for j in 0..nk {
            if mo.offered[j] + trade.imports[j] > NONE {
                self.trend[j] += (1.0 - TREND_KEEP) * (p[j] - self.trend[j]);
            }
        }
        // people get used to the level of prices of what they buy
        let (now, then): (f32, f32) = (0..nk).fold((0.0, 0.0), |(a, b), j| (a + p[j] * self.sold[j], b + pack.goods[j].price * self.sold[j]));
        if then > 0.0 {
            self.level += GET_USED_PRICE * (now / then - self.level);
        }

        // 8. discontent piles up — from life getting worse than people are used to, and from
        // hunger, which nobody gets used to; when it overflows, the group rises
        for (g, grp) in pack.groups.iter().enumerate() {
            let wsum: f32 = grp.needs.iter().map(|n| n.weight).sum::<f32>().max(1e-6);
            self.hardship[g] = grp.needs.iter().enumerate().map(|(k, n)| n.weight * self.unmet[g][k]).sum::<f32>() / wsum;
            let starving = grp
                .needs
                .iter()
                .enumerate()
                .filter(|(_, n)| n.vital)
                .map(|(k, _)| self.unmet[g][k])
                .fold(0.0f32, f32::max);
            let worse = (self.hardship[g] - self.used_to[g]).max(0.0);
            self.used_to[g] += GET_USED * (self.hardship[g] - self.used_to[g]);
            if !self.live {
                self.used_to[g] = self.hardship[g];
            }
            self.discontent[g] = self.discontent[g] * KEEP_ANGER + if self.live { worse + starving } else { 0.0 };
            if self.unrest[g] > 0 {
                self.unrest[g] -= 1;
            } else if self.discontent[g] > grp.patience {
                self.unrest[g] = UNREST_MONTHS;
                self.discontent[g] *= 0.5;
                let cause = self.cause(pack, g);
                self.events.push(format!("мес {:>3}: волнения — {} ({})", self.month, grp.name, cause));
            }
            let born = self.growth_by.get(g).copied().unwrap_or(BIRTHS);
            self.pop[g] *= 1.0 + born - DEATHS_FROM_WANT * self.famine_k * starving;
        }

        // 9. idle people move to groups that pay better and lack hands
        for (g, grp) in pack.groups.iter().enumerate() {
            let idle = self.unemployed[g].unwrap_or(0.0);
            if idle <= NORMAL_IDLE {
                continue;
            }
            let dest = grp
                .becomes
                .iter()
                .copied()
                .filter(|&h| self.unemployed[h].is_some_and(|u| u < NORMAL_IDLE) && self.pay[h] > self.pay[g])
                .max_by(|&a, &b| self.pay[a].total_cmp(&self.pay[b]));
            if let Some(h) = dest {
                let movers = self.pop[g] * (idle - NORMAL_IDLE) * MOBILITY * self.mobility_k;
                let share = movers / self.pop[g].max(1e-6);
                let money = self.money[g] * share;
                self.pop[g] -= movers;
                self.pop[h] += movers;
                self.money[g] -= money;
                self.money[h] += money;
            }
        }
        // 10. hunger drives people to the land: those who go hungrier than the people who work it
        // leave for it (town-dwellers to the villages in a famine)
        if self.land_limited {
            let hunger: Vec<f32> = pack
                .groups
                .iter()
                .enumerate()
                .map(|(g, grp)| grp.needs.iter().enumerate().filter(|(_, n)| n.vital).map(|(k, _)| self.unmet[g][k]).fold(0.0f32, f32::max))
                .collect();
            let mut tillers = vec![0.0f32; ng];
            for s in (0..ns).filter(|&s| pack.sectors[s].land) {
                let (g, w) = main_hands(pack, s);
                tillers[g] += self.cap[s] * w;
            }
            if let Some(t) = (0..ng).filter(|&g| tillers[g] > 0.0).max_by(|&a, &b| tillers[a].total_cmp(&tillers[b])) {
                for g in (0..ng).filter(|&g| g != t && hunger[g] > hunger[t]) {
                    let free = (self.pop[g] - self.drafted[g] - self.scholars[g]).max(0.0);
                    let movers = free * TO_LAND * (hunger[g] - hunger[t]);
                    if movers <= 0.0 || self.pop[g] <= 0.0 {
                        continue;
                    }
                    let money = self.money[g] * movers / self.pop[g];
                    self.pop[g] -= movers;
                    self.pop[t] += movers;
                    self.money[g] -= money;
                    self.money[t] += money;
                }
            }
        }
    }
}
