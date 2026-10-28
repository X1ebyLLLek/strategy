//! The facts of a country a feature grows from (`Facts`), gathered each month from the world: its
//! land, economy, treasury, wars, people.

use super::*;

impl Game {
    /// The facts of side `s` now.
    pub fn facts(&self, s: usize) -> Facts {
        let pack = &self.pack;
        let n = self.h.home(s);
        let pop = n.people().max(1e-6);
        let mut f = Facts { people: pop, ..Default::default() };
        // output a head against the world's
        let heads: Vec<f32> = self.h.world.nations.iter().filter(|x| x.people() > 0.1).map(|x| x.gdp_real / x.people()).collect();
        let mean = heads.iter().sum::<f32>() / heads.len().max(1) as f32;
        f.rich = (n.gdp_real / pop) / mean.max(1e-9);
        let mem = &self.spirits.memory[s];
        if mem.len() >= 12 {
            let (g0, l0) = mem[mem.len() - 12];
            f.growth = n.gdp_real / g0.max(1e-9) - 1.0;
            f.inflation = n.level / l0.max(1e-9) - 1.0;
        }
        // who works where
        let worked: f32 = n.worked.iter().sum::<f32>().max(1e-9);
        for (k, sec) in pack.sectors.iter().enumerate() {
            let w = n.worked.get(k).copied().unwrap_or(0.0) / worked;
            if sec.land {
                f.land_share += w;
            } else if sec.outputs.iter().any(|o| pack.goods[o.0].military) {
                f.arms_share += w;
                f.industry_share += w;
            } else if sec.outputs.iter().any(|o| pack.goods[o.0].tradable) {
                f.industry_share += w;
            }
        }
        // hunger, anger, unrest
        for (g, grp) in pack.groups.iter().enumerate() {
            if n.pop[g] <= 1e-6 {
                continue;
            }
            for (i, need) in grp.needs.iter().enumerate() {
                if need.vital {
                    f.hunger = f.hunger.max(n.unmet.get(g).and_then(|v| v.get(i)).copied().unwrap_or(0.0));
                }
            }
        }
        f.anger = n.anger();
        f.unrest = n.unrest.iter().any(|&u| u > 0);
        f.room = if n.tilled > 1e-6 { n.land / n.tilled } else { 2.0 };
        // the learned, the owners, goods from abroad
        let learned: Vec<usize> = pack.science.as_ref().map_or(Vec::new(), |x| x.learned.clone());
        f.learned = learned.iter().map(|&g| n.pop[g]).sum::<f32>() / pop;
        let owners: Vec<usize> = {
            let mut v: Vec<usize> = pack.sectors.iter().map(|x| x.owner).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        let income: f32 = n.income.iter().sum::<f32>().max(1e-9);
        f.owners = owners.iter().map(|&g| n.income.get(g).copied().unwrap_or(0.0)).sum::<f32>() / income;
        let bought: f32 = (0..pack.goods.len()).map(|i| (n.produced[i] + n.import[i]) * n.price[i]).sum::<f32>().max(1e-9);
        f.imports = (0..pack.goods.len()).map(|i| n.import[i] * n.price[i]).sum::<f32>() / bought;
        // the treasury and the debt, in months of the state's income
        let income = (n.gdp * n.tax).max(1e-6);
        f.treasury = n.treasury / income;
        f.debt = (self.policy.debt[s] + self.credit.bonds[s]) / income;
        // the ground of the provinces it holds (the theatre's), its coasts, its deposits
        let th = &self.h.war.th;
        let mine: Vec<usize> = (0..th.provs.len()).filter(|&q| self.h.war.owner[q] == s).collect();
        f.provinces = mine.len();
        let nr = pack.resources.len();
        f.deposits = vec![0; nr];
        let mut coast = 0usize;
        let mut rails = 0usize;
        for &q in &mine {
            match th.terrain[q] {
                Terrain::Mountains | Terrain::Hills => f.high += 1.0,
                Terrain::Forest | Terrain::Marsh | Terrain::Jungle => f.wood += 1.0,
                _ => {}
            }
            let p = th.provs[q];
            if self.world.provinces[p].neighbors.iter().any(|&o| self.world.provinces.get(o as usize).is_some_and(|x| x.sea)) {
                coast += 1;
            }
            for &r in self.prov_res.get(p).map(|v| v.as_slice()).unwrap_or(&[]) {
                if r < nr {
                    f.deposits[r] += 1;
                }
            }
            // (the grade is what a step costs: a railway's is the cheapest)
            if self.h.war.grade.get(q).is_some_and(|&g| g <= crate::world::land::RAIL_GRADE + 1e-3) {
                rails += 1;
            }
        }
        let k = mine.len().max(1) as f32;
        f.high /= k;
        f.wood /= k;
        f.coast = coast as f32 / k;
        f.rails = rails as f32 / k;
        // its wars: of the past century and of the game
        let c = th.countries[s];
        let since_start = self.month() as f32 / 12.0;
        for w in &self.past {
            if let Some(i) = w.sides.iter().position(|&x| x == c) {
                f.wars += 1;
                let ago = w.year as f32 - w.months as f32 / 12.0 + since_start;
                if f.last_war.is_none_or(|y| ago < y) {
                    f.last_war = Some(ago.max(0.0));
                    f.last_won = w.winner.map(|x| x == i);
                }
            }
        }
        for &(m, won) in &self.spirits.game_wars[s] {
            let ago = (self.month().saturating_sub(m)) as f32 / 12.0;
            if f.last_war.is_none_or(|y| ago < y) {
                f.last_war = Some(ago);
                f.last_won = won;
            }
            f.wars += 1;
        }
        if self.h.at_war {
            f.last_war = Some(0.0);
            f.last_won = None;
        }
        f.lost = self.lost[s].iter().filter(|&&(_, y)| (y as f32 + since_start) < 60.0).count();
        f.taken = self
            .lost_lands
            .iter()
            .filter(|&&(st, loser, y)| loser as usize != c && (y as f32 + since_start) < 40.0 && (0..th.provs.len()).any(|q| self.h.war.owner[q] == s && self.prov_state.get(th.provs[q]).is_some_and(|&x| x == st)))
            .count();
        f.veterans = self.veterans[s].len();
        f.glory = self.h.war.sides[s].glory.clone();
        f.doubt = self.h.war.sides[s].doubt;
        f.drafted = n.drafted.iter().sum::<f32>() / pop;
        f.dead = self.spirits.war_dead[s].max(if self.h.at_war { self.h.dead[s] } else { 0.0 }) / pop;
        f.held = self.h.war.people()[s];
        f.at_war = self.h.at_war;
        f.mil = n.mil_share;
        // its people and what it knows
        let country = self.h.war.th.countries[s];
        f.genius = self.folk.living(country, strategy::world::folk::Calling::Scientist).first().map_or(0.0, |&id| self.folk.get(id).fame);
        f.soldier_rules = s != self.me && self.folk.offices.get(country).and_then(|o| o.ruler).is_some_and(|r| self.folk.get(r).calling == strategy::world::folk::Calling::General);
        f.graft = self.folk.taken.get(country).copied().unwrap_or(0.0);
        {
            let p = &self.polity.of[country];
            let now = self.month() as i32;
            f.piety = p.piety;
            f.literacy = p.literacy;
            let shares = self.polity.shares(self.realm.regions.get(country).map_or(&[][..], |v| &v[..]));
            f.faith_share = shares.iter().find(|x| x.0 == p.state_faith).map_or(1.0, |x| x.1);
            f.house_years = if p.house.is_some() { (now - p.house_since).max(0) as f32 / 12.0 } else { 0.0 };
            f.fell = p.fell.map(|m| (now - m).max(0) as f32 / 12.0);
            f.junta = matches!(p.regime, strategy::world::polity::Regime::Junta);
        }
        let other = self.h.war.th.countries[1 - s];
        if self.know.len() > country.max(other) {
            f.stage_gap = self.know[country].stage(pack) as f32 - self.know[other].stage(pack) as f32;
        }
        // how deep the capital lies: provinces of the theatre to the nearest one the other holds
        let cap = th.capital[s];
        let mut dist = vec![usize::MAX; th.provs.len()];
        let mut queue = std::collections::VecDeque::new();
        if cap < dist.len() {
            dist[cap] = 0;
            queue.push_back(cap);
        }
        f.capital_depth = 9;
        while let Some(q) = queue.pop_front() {
            if self.h.war.owner[q] != s {
                f.capital_depth = dist[q];
                break;
            }
            for &r in &th.nbr[q] {
                if dist[r] == usize::MAX {
                    dist[r] = dist[q] + 1;
                    queue.push_back(r);
                }
            }
        }
        f
    }
}
