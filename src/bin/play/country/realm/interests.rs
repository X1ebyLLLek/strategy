//! What each force of society has, not what a table says it should feel: its real lot month by month
//! (what its people consumed a head, at the prices of the start — so a tax, a dole, a labour law or
//! a run of prices reaches it through the market, not through a modifier), the tax it actually paid,
//! its people out of work; the landowners by what the land earns, the industrialists by what the
//! factories earn, the merchants by the trade, the officers by the army's money. From the same
//! numbers: each force's weight in the economy (its power follows it, `Life::wealth`), its seats in
//! the chamber where the country has one, how the chamber votes on a reform and how the country
//! votes at an election — by its lot and by the economy: prices, work, growth.

use super::*;
use crate::country::policy::role;

/// The forces of the economy whose lot the market measures (the clergy's is its faith).
const MEASURED: [usize; 7] = [polity::NOBLES, polity::OFFICERS, polity::CAPITAL, polity::MERCHANTS, polity::LEARNED, polity::WORKERS, polity::PEASANTS];

/// Weight of each force's power in a chamber of estates and census.
const CENSUS: [f32; BLOCS] = [1.3, 1.0, 0.5, 1.2, 1.2, 0.8, 0.15, 0.25];

impl Game {
    /// The groups of the pack of role `r` (0 the people of the land … 3 the owners).
    fn groups_of(&self, r: usize) -> Vec<usize> {
        (0..self.pack.groups.len()).filter(|&g| role(&self.pack, g) == r).collect()
    }

    /// Each force's material lot of side `s` now (0 for those the market does not measure): the
    /// people's consumption a head at the prices of the start; the owners' earnings — the land's for
    /// the landowners, the rest for the industrialists — and the trade for the merchants, the army's
    /// money for the officers, all at the prices of the start.
    pub fn lots(&self, s: usize) -> [f32; BLOCS] {
        use polity::*;
        let pack = &self.pack;
        let n = self.h.home(s);
        let level = n.level.max(1e-6);
        let consumed = |gs: &[usize]| -> f32 {
            let pop: f32 = gs.iter().map(|&g| n.pop[g]).sum();
            if pop <= 1e-9 {
                return 0.0;
            }
            let real: f32 = gs
                .iter()
                .map(|&g| (0..pack.goods.len()).map(|j| (n.spent[g][j] / n.price[j].max(1e-9) + n.kind[g][j]) * pack.goods[j].price).sum::<f32>())
                .sum();
            real / pop
        };
        let mut lot = [0.0f32; BLOCS];
        lot[PEASANTS] = consumed(&self.groups_of(0));
        lot[WORKERS] = consumed(&self.groups_of(1));
        lot[LEARNED] = consumed(&self.groups_of(2));
        let (mut land, mut works) = (0.0f32, 0.0f32);
        for (k, sec) in pack.sectors.iter().enumerate() {
            let p = n.profit.get(k).copied().unwrap_or(0.0).max(0.0);
            if sec.land {
                land += p;
            } else {
                works += p;
            }
        }
        lot[NOBLES] = land / level;
        lot[CAPITAL] = works / level;
        lot[MERCHANTS] = (0..pack.goods.len()).map(|i| (n.shipped[i] + n.import[i]) * n.price[i]).sum::<f32>() / level + 0.05 * works / level;
        lot[OFFICERS] = (n.army_spent + n.soldiers_paid) / level;
        lot
    }

    /// The share of what each force of side `s` earned that the tax took last month (0 for those
    /// that pay none of their own).
    pub fn burdens(&self, s: usize) -> [f32; BLOCS] {
        use polity::*;
        let n = self.h.home(s);
        let burden = |gs: &[usize]| -> f32 {
            let inc: f32 = gs.iter().map(|&g| n.income[g].max(0.0)).sum();
            let tax: f32 = gs.iter().map(|&g| n.taxed.get(g).copied().unwrap_or(0.0)).sum();
            if inc > 1e-9 {
                tax / inc
            } else {
                0.0
            }
        };
        let owners = burden(&self.groups_of(3));
        let mut b = [0.0f32; BLOCS];
        b[PEASANTS] = burden(&self.groups_of(0));
        b[WORKERS] = burden(&self.groups_of(1));
        b[LEARNED] = burden(&self.groups_of(2));
        b[NOBLES] = owners;
        b[CAPITAL] = owners;
        b[MERCHANTS] = owners;
        b
    }

    /// The share of the people of force `b` of side `s` out of work (none for the forces of no trade).
    fn idle_of(&self, s: usize, b: usize) -> Option<f32> {
        let r = match b {
            polity::PEASANTS => 0,
            polity::WORKERS => 1,
            polity::LEARNED => 2,
            _ => return None,
        };
        let n = self.h.home(s);
        let gs: Vec<usize> = self.groups_of(r).into_iter().filter(|&g| n.unemployed[g].is_some()).collect();
        let pop: f32 = gs.iter().map(|&g| n.pop[g]).sum();
        (pop > 1e-9).then(|| gs.iter().map(|&g| n.unemployed[g].unwrap_or(0.0) * n.pop[g]).sum::<f32>() / pop)
    }

    /// After the world's month: each force's lot goes into the memory of the last year.
    pub fn lots_month(&mut self) {
        for s in 0..2 {
            let l = self.lots(s);
            let m = &mut self.realm.lots[s];
            m.push(l);
            if m.len() > 13 {
                m.remove(0);
            }
        }
    }

    /// How force `b`'s lot of side `s` changed over the last year (log), if it is known.
    pub fn lot_change(&self, s: usize, b: usize) -> Option<f32> {
        let m = &self.realm.lots[s];
        if m.len() < 4 {
            return None;
        }
        let (a, z) = (m[0][b], m[m.len() - 1][b]);
        (a > 1e-9 && z > 1e-9).then(|| (z / a).ln() * 12.0 / (m.len() - 1) as f32)
    }

    /// What force `b` of side `s` feels about its material lot, thing by thing: its income over the
    /// year, the tax it pays, work, prices (for the merchants, who lend), credit (for the
    /// industrialists, who borrow).
    pub fn interest_parts(&self, s: usize, b: usize) -> Vec<Part> {
        use polity::*;
        let mut v: Vec<Part> = Vec::new();
        if crate::country::econ_spoiled() {
            return v;
        }
        let mut add = |what: String, x: f32| {
            if x.abs() >= 0.01 {
                v.push((what, x));
            }
        };
        if MEASURED.contains(&b) {
            if let Some(x) = self.lot_change(s, b) {
                let word = match b {
                    NOBLES => "доход с земли за год",
                    CAPITAL => "прибыль заводов за год",
                    MERCHANTS => "торговля за год",
                    OFFICERS => "деньги армии за год",
                    _ => "достаток за год",
                };
                // (a people's consumption a head moves slowly, and every point of it is felt; the
                // land's rents, the factories' profits, the trade and the army's money swing by
                // tens of percent in a year — their owners feel the swing half as much)
                let k = if matches!(b, NOBLES | CAPITAL | MERCHANTS | OFFICERS) { 0.8 } else { 1.6 };
                add(format!("{word} {:+.0}%", (x.exp() - 1.0) * 100.0), (k * x).clamp(-0.3, 0.25));
            }
        }
        let burden = self.burdens(s)[b];
        if burden > 0.0 {
            add(format!("налог берёт {:.0}% дохода", burden * 100.0), (-1.6 * (burden - 0.15)).clamp(-0.4, 0.15));
        }
        if let Some(u) = self.idle_of(s, b) {
            if u > 0.05 {
                add(format!("без работы {:.0}%", u * 100.0), (-1.2 * (u - 0.05)).max(-0.35));
            }
        }
        let f = &self.spirits.facts[s];
        if b == MERCHANTS && f.inflation > 0.05 {
            add(format!("цены за год {:+.0}%", f.inflation * 100.0), (-1.2 * (f.inflation - 0.05)).max(-0.3));
        }
        if b == CAPITAL {
            let gdp = self.h.home(s).gdp.max(1e-6);
            let net = (self.credit.lent[s] - self.credit.repaid[s]) / gdp;
            add(if net >= 0.0 { "банки дают в долг".into() } else { "банки требуют долги".into() }, (3.0 * net).clamp(-0.15, 0.12));
            if self.credit.crunch[s] > 0 {
                add("кредит иссяк".into(), -0.12);
            }
        }
        if matches!(b, CAPITAL | MERCHANTS | NOBLES) && self.credit.repudiated[s].is_some_and(|m| self.month() < m + 120) {
            add("отказ платить по облигациям".into(), -0.2);
        }
        v
    }

    /// Each force's weight in side `s`'s economy (for its power): the owners by what they earn,
    /// the merchants by the trade, the learned by their incomes and letters, the workers and the
    /// peasants by their numbers and incomes — organised as far as they read and are free.
    pub fn wealth_of_blocs(&self, s: usize) -> [f32; BLOCS] {
        use polity::*;
        let n = self.h.home(s);
        let p = self.polity_of(s);
        let all_inc: f32 = n.income.iter().map(|x| x.max(0.0)).sum::<f32>().max(1e-9);
        let all_pop = n.people().max(1e-9);
        let inc = |r: usize| self.groups_of(r).iter().map(|&g| n.income[g].max(0.0)).sum::<f32>() / all_inc;
        let pop = |r: usize| self.groups_of(r).iter().map(|&g| n.pop[g]).sum::<f32>() / all_pop;
        let (mut land, mut works) = (0.0f32, 0.0f32);
        for (k, sec) in self.pack.sectors.iter().enumerate() {
            let x = n.profit.get(k).copied().unwrap_or(0.0).max(0.0);
            if sec.land {
                land += x;
            } else {
                works += x;
            }
        }
        let owners = inc(3);
        let profits = (land + works).max(1e-9);
        let trade: f32 = (0..self.pack.goods.len()).map(|i| (n.shipped[i] + n.import[i]) * n.price[i]).sum::<f32>() / n.gdp.max(1e-9);
        let lit = p.literacy;
        let free = |k: usize| 0.4 + 0.3 * p.laws[k] as f32;
        let mut w = [0.0f32; BLOCS];
        w[NOBLES] = owners * land / profits;
        w[CAPITAL] = owners * works / profits;
        w[MERCHANTS] = 0.3 * owners * trade.min(1.0) + 0.02;
        w[LEARNED] = inc(2) * (0.6 + lit);
        w[WORKERS] = (0.5 * pop(1) + 0.5 * inc(1)) * (0.3 + 0.7 * lit) * free(polity::LABOUR);
        w[PEASANTS] = (0.4 * pop(0) + 0.6 * inc(0)) * 0.5 * free(polity::LABOUR);
        w
    }

    /// Side `s`'s chamber: each force's share of the seats, if the country has one (a voice of the
    /// estates and the census: power and money vote; a voice for all: the people vote, as far as they
    /// read; none where nobody is asked).
    pub fn seats(&self, s: usize) -> Option<[f32; BLOCS]> {
        use polity::*;
        let p = self.polity_of(s);
        let mut w = [0.0f32; BLOCS];
        match p.laws[VOICE] {
            0 => return None,
            1 => {
                for b in 0..BLOCS {
                    w[b] = p.clout[b] * CENSUS[b];
                }
            }
            _ => {
                let n = self.h.home(s);
                let all = n.people().max(1e-9);
                let pop = |r: usize| self.groups_of(r).iter().map(|&g| n.pop[g]).sum::<f32>() / all;
                let turnout = 0.4 + 0.6 * p.literacy;
                let owners = pop(3);
                let rich = (p.clout[NOBLES] + p.clout[CAPITAL] + p.clout[MERCHANTS]).max(1e-6);
                w[PEASANTS] = pop(0) * turnout;
                w[WORKERS] = pop(1) * turnout;
                w[LEARNED] = pop(2);
                for b in [NOBLES, CAPITAL, MERCHANTS] {
                    w[b] = owners * p.clout[b] / rich + 0.3 * p.clout[b];
                }
                // (the officers and the clergy have no seats of their own: they sway voters)
                w[OFFICERS] = 0.3 * p.clout[OFFICERS];
                w[CLERGY] = 0.5 * p.clout[CLERGY] * p.piety;
            }
        }
        let all: f32 = w.iter().sum::<f32>().max(1e-9);
        Some(w.map(|x| x / all))
    }

    /// How the economy sways the voters of side `s`: prices and want of work against the
    /// government, growth for it (−0.4 … +0.3).
    pub fn economy_vote(&self, s: usize) -> f32 {
        let f = &self.spirits.facts[s];
        let figures = self.figures(s);
        let misery = f.inflation.max(0.0) + figures.idle;
        (-2.0 * (misery - 0.10) + 1.5 * f.growth.clamp(-0.15, 0.15)).clamp(-0.4, 0.3)
    }

    /// Each force's support for side `s`'s government (0 … 1): its mood and the economy.
    pub fn support(&self, s: usize) -> [f32; BLOCS] {
        let p = self.polity_of(s);
        let e = self.economy_vote(s);
        let mut v = [0.0f32; BLOCS];
        for b in 0..BLOCS {
            v[b] = 0.5 + 0.5 * (2.0 * (p.mood[b] + e)).tanh();
        }
        v
    }

    /// The government's share of the chamber's seats if side `s` voted now.
    pub fn vote_share(&self, s: usize) -> Option<f32> {
        let seats = self.seats(s)?;
        let sup = self.support(s);
        Some((0..BLOCS).map(|b| seats[b] * sup[b]).sum())
    }

    /// How side `s`'s chamber would vote on reforming `law` to `to`: the share of the seats for it.
    /// A force votes by what the reform does to it; one that does not care follows its support for
    /// the government.
    pub fn reform_vote(&self, s: usize, law: usize, to: u8) -> Option<f32> {
        let seats = self.seats(s)?;
        let p = self.polity_of(s);
        let sup = self.support(s);
        let from = p.laws[law];
        let mut yes = 0.0;
        for b in 0..BLOCS {
            let d = polity::law_fit(b, law, to) - polity::law_fit(b, law, from);
            let x = if d > 0.02 {
                0.85 + 0.15 * sup[b]
            } else if d < -0.02 {
                0.15 * sup[b]
            } else {
                sup[b]
            };
            yes += seats[b] * x;
        }
        Some(yes)
    }
}
