//! The month's first half: what each sector can make (land, room, return), where owners and
//! the state mean to build, what each group and sector will buy.

use super::*;

impl Nation {
    /// What a hand on the land makes against the usual density (1 for other sectors).
    pub(super) fn on_land(&self, pack: &Pack, s: usize) -> f32 {
        if pack.sectors[s].land && self.land_limited {
            roomy(self.land, self.tilled)
        } else {
            1.0
        }
    }

    /// How many new jobs a sector can take: it works near full (not short of inputs or of
    /// buyers), and the people it needs most are idle even after what is being built.
    fn room(&self, pack: &Pack, s: usize, free: &[f32], pipe: &[f32]) -> f32 {
        if pack.sectors[s].build.is_empty() || self.activity[s] < 0.9 {
            return 0.0;
        }
        let (g, w) = main_hands(pack, s);
        ((free[g] - 0.02 * self.pop[g]) / w).min(self.max_cap[s] - self.cap[s] - pipe[s]).max(0.0)
    }

    /// Monthly return of a new job: what its owner would keep at recent prices (output less
    /// inputs, less the workers' share) over what a job costs. Judged from prices, not from
    /// last month's books, so a closed sector comes back when its good grows scarce.
    fn yield_of(&self, pack: &Pack, s: usize) -> f32 {
        let sec = &pack.sectors[s];
        let p = &self.trend;
        let output: f32 = sec.outputs.iter().map(|&(j, a)| a * p[j]).sum::<f32>() * self.prod[s] * self.shock[s] * self.on_land(pack, s);
        let inputs: f32 = sec.inputs.iter().map(|&(i, a)| a * p[i]).sum();
        let kept = 1.0 - self.wage_share(pack, s);
        let mut y = (output - inputs) * kept / build_cost(pack, s, p).max(1e-6);
        // (a subsidy a job got last month is a return on it too)
        if !self.subsidy.is_empty() && self.cap[s] > 1e-6 {
            y += self.subsidised.get(s).copied().unwrap_or(0.0) / self.cap[s] / build_cost(pack, s, p).max(1e-6);
        }
        y
    }

    /// The state builds where people suffer most: the sector that makes the most painfully
    /// missing good, if there is room; otherwise where it pays best.
    fn state_target(&self, pack: &Pack, room: &[f32]) -> Option<usize> {
        let mut pain = vec![0.0f32; pack.goods.len()];
        for (g, grp) in pack.groups.iter().enumerate() {
            for (k, n) in grp.needs.iter().enumerate() {
                pain[n.good] += n.weight * self.unmet[g][k] * self.pop[g];
            }
        }
        let mut goods: Vec<usize> = (0..pain.len()).filter(|&i| pain[i] > 1e-3).collect();
        goods.sort_by(|&a, &b| pain[b].total_cmp(&pain[a]));
        for i in goods {
            let maker =
                (0..pack.sectors.len()).find(|&s| room[s] > 0.0 && pack.sectors[s].outputs.iter().any(|&(j, _)| j == i));
            if maker.is_some() {
                return maker;
            }
        }
        self.best(pack, room, None)
    }

    /// Where a new job pays best, among the sectors `owner` owns (all, for None).
    fn best(&self, pack: &Pack, room: &[f32], owner: Option<usize>) -> Option<usize> {
        (0..pack.sectors.len())
            .filter(|&s| room[s] > 0.0 && owner.is_none_or(|g| pack.sectors[s].owner == g))
            .map(|s| (s, self.yield_of(pack, s)))
            .filter(|&(_, r)| r > MIN_RETURN)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(s, _)| s)
    }

    /// The first part of a month: building, work, what is brought to market, plans to build
    /// and the state's budget.
    pub fn prepare(&mut self, pack: &Pack) -> Month {
        let (ng, ns, nk) = (pack.groups.len(), pack.sectors.len(), pack.goods.len());
        self.month += 1;
        while let Some(&(m, s, amt)) = self.building.front() {
            if m > self.month {
                break;
            }
            self.building.pop_front();
            self.cap[s] = (self.cap[s] + amt).min(self.max_cap[s]);
        }

        // 1. how much each sector can work with the people it gets
        let avail: Vec<f32> = (0..ng)
            .map(|g| (self.pop[g] - self.drafted[g] - self.scholars[g]).max(0.0) * if self.unrest[g] > 0 { UNREST_WORK } else { 1.0 })
            .collect();
        let mut jobs = vec![0.0; ng];
        for (s, sec) in pack.sectors.iter().enumerate() {
            for &(g, w) in &sec.workers {
                jobs[g] += self.cap[s] * w;
            }
        }
        let hands: Vec<f32> = (0..ng).map(|g| if jobs[g] > 0.0 { (avail[g] / jobs[g]).min(1.0) } else { 1.0 }).collect();
        let can: Vec<f32> =
            pack.sectors.iter().map(|sec| sec.workers.iter().map(|&(g, _)| hands[g]).fold(1.0, f32::min)).collect();

        // 2. work from the sector's own store: without inputs work stops, boosts are a bonus;
        // goods that lie unsold even at the lowest price are made less — cut step by step (cut
        // in one go, the price jumped back up next month, output came back in full, the glut
        // too, and coal, then steel, tools and bread went up and down every other month), but
        // idle capacity starts again at once when buyers come back
        for j in 0..nk {
            let surplus = (self.stock[j] - STORE_MONTHS * self.sold[j]) / (GLUT * self.sold[j]).max(1e-6);
            let lowest = LOWEST * pack.goods[j].price * self.level;
            let near = ((1.3 * lowest - self.price[j]) / (0.3 * lowest)).clamp(0.0, 1.0);
            let aim = 1.0 - surplus.clamp(0.0, 1.0) * near;
            self.wanted[j] = if aim > self.wanted[j] { aim } else { self.wanted[j] + OUTPUT_PACE * (aim - self.wanted[j]) };
        }
        let wanted = self.wanted.clone();
        let mut used = vec![0.0f32; ns];
        let mut mult = vec![1.0f32; ns];
        let mut used_goods = vec![vec![0.0f32; nk]; ns];
        for (s, sec) in pack.sectors.iter().enumerate() {
            // a glut on the market cuts only what goes to market, not what the makers eat
            let ks = self.kind_share[s];
            let mut act = can[s] * sec.outputs.iter().map(|&(j, _)| ks + (1.0 - ks) * wanted[j]).fold(0.0, f32::max);
            for &(i, a) in &sec.inputs {
                let need = act * self.cap[s] * a;
                if need > 0.0 {
                    act *= (self.inv[s][i] / need).min(1.0);
                }
            }
            self.activity[s] = if can[s] > 0.0 { act / can[s] } else { 1.0 };
            used[s] = act * self.cap[s];
            for &(i, a) in &sec.inputs {
                used_goods[s][i] += used[s] * a;
            }
            for &(i, a, gain) in &sec.boost {
                let got = if used[s] * a > 0.0 { (self.inv[s][i] / (used[s] * a)).min(1.0) } else { 0.0 };
                used_goods[s][i] += used[s] * a * got;
                mult[s] *= 1.0 + gain * got;
            }
            for i in 0..nk {
                self.inv[s][i] = (self.inv[s][i] - used_goods[s][i]).max(0.0);
            }
        }
        let mut made = vec![vec![0.0f32; nk]; ns];
        self.produced = vec![0.0; nk];
        for (s, sec) in pack.sectors.iter().enumerate() {
            let k = used[s] * self.prod[s] * self.shock[s] * mult[s] * self.on_land(pack, s);
            for &(j, a) in &sec.outputs {
                made[s][j] = k * a;
                self.produced[j] += k * a;
            }
        }
        // people who make what keeps them alive keep their part of it before anything goes to
        // market (peasants eat their own bread): their share of the wages, in kind, up to
        // their need. Without it a country whose money had all gone abroad for imports could
        // not buy its own bread, its fields stood and its people died out next to them.
        self.kind = vec![vec![0.0; nk]; ng];
        let mut in_kind = vec![vec![0.0f32; nk]; ns];
        for (s, sec) in pack.sectors.iter().enumerate() {
            let all: f32 = made[s].iter().sum();
            // the workers' share is of what the sector adds, not of all it makes: the inputs
            // are paid first (taking a share of the whole harvest, peasants left their owner
            // to pay for the tools out of the rest, and fields that bought tools closed)
            let worth: f32 = (0..nk).map(|j| made[s][j] * self.trend[j]).sum();
            let cost: f32 = (0..nk).map(|i| used_goods[s][i] * self.trend[i]).sum();
            let net = if worth > 0.0 { (1.0 - cost / worth).max(0.0) } else { 0.0 };
            // (estates leave the tiller less of his harvest, land of his own more)
            let by = self.wage_by.get(s).copied().unwrap_or(1.0);
            for &(g, w) in &sec.workers {
                for n in pack.groups[g].needs.iter().filter(|n| n.vital) {
                    let j = n.good;
                    let take = (made[s][j] * sec.wage_share * by * net * w).min(self.pop[g] * n.amount - self.kind[g][j]).max(0.0);
                    self.kind[g][j] += take;
                    in_kind[s][j] += take;
                    made[s][j] -= take;
                    self.produced[j] -= take;
                }
            }
            self.kind_share[s] = if all > 0.0 { 1.0 - made[s].iter().sum::<f32>() / all } else { 0.0 };
        }
        // people of a group that works the land and has no job there feed themselves from it,
        // at a part of the yield and only for themselves: when cheap imported bread closed the
        // farms, peasants without work and without money died beside the land
        let mut idle: Vec<f32> = avail.clone();
        for (s, sec) in pack.sectors.iter().enumerate() {
            for &(g, w) in &sec.workers {
                idle[g] -= used[s] * w;
            }
        }
        let mut feeding = 0.0f32;
        for (s, sec) in pack.sectors.iter().enumerate().filter(|(_, sec)| sec.land) {
            let yield_of = self.prod[s] * self.shock[s] * SUBSIST * self.on_land(pack, s);
            for &(g, _) in &sec.workers {
                for n in pack.groups[g].needs.iter().filter(|n| n.vital) {
                    let Some(&(_, a)) = sec.outputs.iter().find(|o| o.0 == n.good) else { continue };
                    let per_hand = yield_of * a;
                    if per_hand <= 0.0 || idle[g] <= 0.0 {
                        continue;
                    }
                    let take = (idle[g] * per_hand).min(self.pop[g] * n.amount - self.kind[g][n.good]).max(0.0);
                    self.kind[g][n.good] += take;
                    idle[g] -= take / per_hand;
                    feeding += take / per_hand;
                }
            }
        }
        // the land worked this month sets what a hand on it makes next month
        self.tilled = feeding + (0..ns).filter(|&s| pack.sectors[s].land).map(|s| used[s]).sum::<f32>();
        self.worked = used.clone();
        // capacity left idle for want of buyers or inputs (not of hands) wears out
        for s in 0..ns {
            self.cap[s] -= IDLE_WEAR * (self.cap[s] * can[s] - used[s]).max(0.0);
        }

        // 3. sellers bring this month's output, plus part of what their store holds above the
        // store they want (or minus part of what it lacks); merchants take some of it abroad
        // and bring foreign goods
        // (they never hold back this month's output to refill the store: buyers of inputs pay
        // nearly any price, so holding back 3% of the iron tripled its price; the store fills
        // up by itself from what does not sell)
        let home: Vec<f32> = (0..nk)
            .map(|j| self.produced[j] + (self.stock[j] - STORE_MONTHS * self.sold[j]).max(0.0) * STORE_RELEASE)
            .collect();
        let offered = home.clone();
        let on_hand: Vec<f32> = (0..nk).map(|j| self.produced[j] + self.stock[j]).collect();

        // 4. where building pays: free hands after what is already under construction
        let mut pipe = vec![0.0f32; ns];
        for &(_, s, amt) in &self.building {
            pipe[s] += amt;
        }
        let mut free: Vec<f32> = (0..ng).map(|g| avail[g] - jobs[g]).collect();
        for (s, sec) in pack.sectors.iter().enumerate() {
            for &(g, w) in &sec.workers {
                free[g] -= pipe[s] * w;
            }
        }
        let room: Vec<f32> = (0..ns).map(|s| self.room(pack, s, &free, &pipe)).collect();
        // every sector that pays grows, the faster the better it pays; idle hands are shared
        // among them all
        let people: f32 = self.pop.iter().sum::<f32>().max(1e-6);
        let pace = |s: usize| GROW * self.cap[s].max(SEED_JOBS * people);
        let mut want: Vec<f32> = (0..ns)
            .map(|s| {
                let eager = ((self.yield_of(pack, s) - MIN_RETURN) / MIN_RETURN).clamp(0.0, 1.0);
                (pace(s) * eager).min(room[s])
            })
            .collect();
        let mut asked = vec![0.0f32; ng];
        for (s, sec) in pack.sectors.iter().enumerate() {
            for &(g, w) in &sec.workers {
                asked[g] += want[s] * w;
            }
        }
        let fits: Vec<f32> =
            (0..ng).map(|g| if asked[g] > 0.0 { ((free[g] - 0.02 * self.pop[g]).max(0.0) / asked[g]).min(1.0) } else { 1.0 }).collect();
        for (s, sec) in pack.sectors.iter().enumerate() {
            want[s] *= sec.workers.iter().map(|&(g, _)| fits[g]).fold(1.0, f32::min);
        }
        let invest: Vec<Vec<(usize, f32)>> =
            (0..ng).map(|g| (0..ns).filter(|&s| want[s] > 0.0 && pack.sectors[s].owner == g).map(|s| (s, want[s])).collect()).collect();

        // 5. the treasury: a share of it each month, on the army, on building, and what is
        // left back to the people
        let budget = self.treasury.max(0.0) / STATE_RESERVE;
        let mil_on = (0..nk).any(|i| self.army_buys(pack, i) && offered[i] + self.import[i] > NONE);
        // those serving are paid out of the army's money first, what they would earn at work
        let wages = self.soldier_wages();
        let bill: f32 = wages.iter().sum();
        let paid = bill.min(budget * self.mil_share);
        let mil = if mil_on { budget * self.mil_share } else { paid };
        // science gets its share of what the army leaves
        let sci = match &pack.science {
            Some(_) => (budget * self.science_share).min((budget - mil).max(0.0)),
            None => 0.0,
        };
        // the state builds where owners leave room, at the pace of a sector that pays well
        let left: Vec<f32> = (0..ns).map(|s| (room[s] - want[s]).max(0.0)).collect();
        // by plan, the state builds where it chooses, whether hands are free or not
        let open: Vec<f32> = if self.plan > 0.0 {
            (0..ns)
                .map(|s| if pack.sectors[s].build.is_empty() { 0.0 } else { (self.max_cap[s] - self.cap[s] - pipe[s]).max(0.0) })
                .collect()
        } else {
            left.clone()
        };
        // and spends on it what it means to by degrees: switched on and off at once, state
        // building shook the steel market every few months
        let target = match self.build_focus {
            Some(s) if open[s] > 0.0 => Some(s),
            _ => self.state_target(pack, &left),
        };
        let aim = target.map_or(0.0, |s| {
            ((budget - mil - sci) * self.build_share).min((pace(s) * (1.0 + self.plan)).min(open[s]) * build_cost(pack, s, &self.price))
        });
        self.state_build += STATE_BUILD_PACE * (aim - self.state_build);
        let build = target.map(|s| (s, self.state_build.min(budget - mil - sci), self.state_build / build_cost(pack, s, &self.price).max(1e-9)));
        let mut dole = budget - mil - sci - build.map_or(0.0, |b| b.1);
        // subsidies out of what is left: to the sectors the state chose, as cash for their inputs
        // (what they do not need goes to their owners)
        if !self.subsidy.is_empty() {
            let asked: f32 = self.subsidy.iter().map(|x| x.max(0.0)).sum::<f32>() * budget;
            let k = if asked > 0.0 { (dole.max(0.0) / asked).min(1.0) } else { 0.0 };
            for s in 0..ns {
                let x = self.subsidy.get(s).copied().unwrap_or(0.0).max(0.0) * budget * k;
                self.subsidised[s] = x;
                self.cash[s] += x;
                self.treasury -= x;
                dole -= x;
            }
        }
        // the civil service is paid out of what is left (and, short of it, out of the reserve)
        if !self.civil_by.is_empty() {
            let bill: f32 = self.civil_by.iter().map(|x| x.max(0.0)).sum();
            let can = dole.max(0.0) + 0.5 * (self.treasury - budget).max(0.0);
            let k = if bill > can { can / bill.max(1e-9) } else { 1.0 };
            for g in 0..ng {
                self.money[g] += self.civil_by.get(g).copied().unwrap_or(0.0).max(0.0) * k;
            }
            self.treasury -= bill * k;
            self.civil_paid = bill * k;
            dole = (dole - bill * k).max(0.0);
        }
        self.treasury -= dole;
        self.doled = dole;
        if self.dole_by.is_empty() {
            for g in 0..ng {
                self.money[g] += dole * self.pop[g] / people;
            }
        } else {
            // (what the state hands back goes where it chose: to the poor, to the soldiers' families…)
            let w: Vec<f32> = (0..ng).map(|g| self.pop[g] * self.dole_by.get(g).copied().unwrap_or(1.0).max(0.0)).collect();
            let all: f32 = w.iter().sum::<f32>();
            for g in 0..ng {
                self.money[g] += dole * if all > 1e-9 { w[g] / all } else { self.pop[g] / people };
            }
        }
        // science hires the learned at the pay its money allows, against what they earn elsewhere
        // in the country (not against what money is worth abroad): w · Σ group · CALLING ·
        // (w / earning)^LURE = the money; what it does not spend stays in the treasury
        self.science_money = 0.0;
        if let Some(science) = &pack.science {
            if sci > 0.0 || science.learned.iter().any(|&g| self.scholars[g] > 0.0) {
                let earn: Vec<f32> = (0..ng).map(|g| self.earning(g).max(1e-6)).collect();
                let k: f32 = science.learned.iter().map(|&g| self.pop[g] * CALLING * earn[g].powf(-LURE)).sum();
                let w = if sci > 0.0 && k > 0.0 { (sci / k).powf(1.0 / (1.0 + LURE)) } else { 0.0 };
                for &g in &science.learned {
                    let want = (self.pop[g] * CALLING * (w / earn[g]).powf(LURE)).min(LAB_MAX * self.pop[g]);
                    self.scholars[g] += LAB_PACE * (want - self.scholars[g]);
                }
                let bill: f32 = science.learned.iter().map(|&g| self.scholars[g] * w).sum();
                let pay = bill.min(sci);
                if pay > 0.0 {
                    self.treasury -= pay;
                    for &g in &science.learned {
                        self.money[g] += self.scholars[g] * w * pay / bill;
                    }
                }
                self.science_money = pay;
                self.lab_pay = w;
            }
        }
        self.soldiers_paid = if bill > 0.0 { paid } else { 0.0 };
        if bill > 0.0 {
            self.treasury -= paid;
            for g in 0..ng {
                self.money[g] += wages[g] * paid / bill;
            }
        }
        let mil = mil - paid;

        // 6. the market
        let restock: Vec<Vec<f32>> = (0..ns)
            .map(|s| {
                let mut r = vec![0.0f32; nk];
                for (i, a) in store_needs(pack, s) {
                    r[i] += (self.cap[s] * can[s] * a * INPUT_STORE - self.inv[s][i]).max(0.0);
                }
                r
            })
            .collect();
        // how big a market each good could have here
        let mut potential = vec![0.0f32; nk];
        for (g, grp) in pack.groups.iter().enumerate() {
            for n in &grp.needs {
                potential[n.good] += self.pop[g] * n.amount;
            }
        }
        for s in 0..ns {
            for (i, a) in store_needs(pack, s) {
                potential[i] += self.cap[s] * a;
            }
        }
        if self.army_wants.is_empty() {
            let arms: Vec<usize> = (0..nk).filter(|&i| pack.goods[i].military).collect();
            for &i in &arms {
                potential[i] += budget * self.mil_share / arms.len() as f32 / (pack.goods[i].price * self.level).max(1e-9);
            }
        } else {
            for i in (0..nk).filter(|&i| self.army_wants[i] > 0.0) {
                potential[i] += budget * self.mil_share * self.army_wants[i] / (pack.goods[i].price * self.level).max(1e-9);
            }
        }
        Month {
            offered,
            home,
            potential,
            jobs,
            used,
            made,
            used_goods,
            in_kind,
            avail: on_hand,
            budget: self.money.iter().map(|m| m.max(0.0) / RESERVE).collect(),
            invest,
            restock,
            mil,
            build,
        }
    }
}
