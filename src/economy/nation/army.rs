//! The state and its people at war: land ceded and occupied, the state's budget and the
//! army's money, soldiers' pay, men drafted, released and buried.

use super::*;

/// A law of the draft (as in HOI4): its name (and a short one for its tile), its icon, the most of
/// each group that may serve at once, the share of his earnings at work one serving is paid. A wider draft gives more men and
/// cheaper ones (the army's money buys more kit); they are taken from work, and their families
/// live on what the law pays them.
pub struct Law {
    pub name: &'static str,
    pub short: &'static str,
    pub icon: &'static str,
    pub cap: f32,
    pub pay: f32,
}

pub const LAWS: [Law; 5] = [
    Law { name: "Добровольцы", short: "Наём", icon: "hand", cap: 0.015, pay: 1.0 },
    Law { name: "Ограниченный призыв", short: "Малый", icon: "shield", cap: 0.03, pay: 0.5 },
    Law { name: "Обширный призыв", short: "Набор", icon: "army", cap: 0.06, pay: 0.3 },
    Law { name: "Служба по требованию", short: "Общий", icon: "clash", cap: 0.10, pay: 0.2 },
    Law { name: "Все взрослые", short: "Все", icon: "fist", cap: 0.15, pay: 0.15 },
];

/// The custom laws: in peace, and at war (a state goes on a war footing with its draft too). (In
/// peace, volunteers: a standing army of about a hundredth of the people, as in life; with limited
/// conscription — three hundredths — a rich country of a hundred million kept 450 divisions in peace.)
pub const PEACE_LAW: usize = 0;
pub const WAR_LAW: usize = 3;

impl Nation {
    /// An enemy now holds `share` of the country (by its people): what it took more is set
    /// apart, what was freed comes back — people (not those serving), workplaces and deposits,
    /// stores, money.
    /// The part of the country the enemy holds is given up to `to` for good: its people,
    /// workplaces, land, money and stores join `to`'s (the world's money stays what it was).
    pub fn cede(&mut self, to: &mut Nation) {
        let Some(a) = self.away.take() else { return };
        for g in 0..a.pop.len() {
            to.pop[g] += a.pop[g];
            to.money[g] += a.money[g];
        }
        for k in 0..a.cap.len() {
            to.cap[k] += a.cap[k];
            to.cash[k] += a.cash[k];
            if to.max_cap[k].is_finite() && a.max_cap[k].is_finite() {
                to.max_cap[k] += a.max_cap[k];
            }
            for (x, y) in to.inv[k].iter_mut().zip(&a.inv[k]) {
                *x += y;
            }
        }
        for (x, y) in to.stock.iter_mut().zip(&a.stock) {
            *x += y;
        }
        to.land += a.land;
    }

    pub fn occupy(&mut self, share: f32) {
        let share = share.clamp(0.0, 0.99);
        let (ng, ns, nk) = (self.pop.len(), self.cap.len(), self.stock.len());
        let now = self.away.as_ref().map_or(0.0, |a| a.share);
        if (share - now).abs() < 1e-4 {
            return;
        }
        let a = self.away.get_or_insert_with(|| Away {
            share: 0.0,
            land: 0.0,
            pop: vec![0.0; ng],
            cap: vec![0.0; ns],
            max_cap: vec![0.0; ns],
            money: vec![0.0; ng],
            cash: vec![0.0; ns],
            stock: vec![0.0; nk],
            inv: vec![vec![0.0; nk]; ns],
        });
        let finite: Vec<bool> = self.max_cap.iter().map(|m| m.is_finite()).collect();
        if share > now {
            // of what is still at home
            let f = (share - now) / (1.0 - now);
            for g in 0..ng {
                let x = (self.pop[g] - self.drafted[g] - self.scholars[g]).max(0.0) * f;
                self.pop[g] -= x;
                a.pop[g] += x;
            }
            shift(&mut self.cap, &mut a.cap, f);
            let x = self.land * f;
            self.land -= x;
            a.land += x;
            shift(&mut self.money, &mut a.money, f);
            shift(&mut self.cash, &mut a.cash, f);
            shift(&mut self.stock, &mut a.stock, f);
            for k in 0..ns {
                shift(&mut self.inv[k], &mut a.inv[k], f);
                if finite[k] {
                    let x = self.max_cap[k] * f;
                    self.max_cap[k] -= x;
                    a.max_cap[k] += x;
                }
            }
        } else {
            // of what was held
            let f = (now - share) / now;
            shift(&mut a.pop, &mut self.pop, f);
            shift(&mut a.cap, &mut self.cap, f);
            let x = a.land * f;
            a.land -= x;
            self.land += x;
            shift(&mut a.money, &mut self.money, f);
            shift(&mut a.cash, &mut self.cash, f);
            shift(&mut a.stock, &mut self.stock, f);
            for k in 0..ns {
                shift(&mut a.inv[k], &mut self.inv[k], f);
                if finite[k] {
                    let x = a.max_cap[k] * f;
                    a.max_cap[k] -= x;
                    self.max_cap[k] += x;
                }
            }
        }
        a.share = share;
    }

    /// Whether the army buys good `i`.
    pub(super) fn army_buys(&self, pack: &Pack, i: usize) -> bool {
        if self.army_wants.is_empty() {
            pack.goods[i].military
        } else {
            self.army_wants[i] > 0.0
        }
    }

    /// What the state means to spend this month (a share of the treasury).
    pub fn state_budget(&self) -> f32 {
        self.treasury.max(0.0) / STATE_RESERVE
    }

    /// Money the army has to spend on the market this month, as the state will count it: its
    /// share of what the state spends, less the pay of those serving.
    pub fn army_money(&self) -> f32 {
        let army = self.treasury.max(0.0) / STATE_RESERVE * self.mil_share;
        let left = army - self.soldier_wages().iter().sum::<f32>().min(army);
        if self.army_side > 0.0 {
            (left - self.army_side).max(0.0)
        } else {
            left
        }
    }

    /// What a member of group `g` earns in a month at his work: his pay where sectors employ the
    /// group, otherwise its income a head.
    pub fn earning(&self, g: usize) -> f32 {
        if self.unemployed[g].is_some() {
            self.pay[g]
        } else {
            self.income[g] / self.pop[g].max(1e-9)
        }
    }

    /// What a man would earn at work in a month (money per million people): what the groups that
    /// work earn, by how many each is.
    pub fn soldier_pay(&self) -> f32 {
        let (mut paid, mut all) = (0.0f32, 0.0f32);
        for g in 0..self.pop.len() {
            if self.unemployed[g].is_some() {
                paid += self.pop[g] * self.pay[g];
                all += self.pop[g];
            }
        }
        if all > 0.0 {
            paid / all
        } else {
            0.0
        }
    }

    /// The pay of those serving, by their group: what each would earn at work (a group that
    /// does not work for wages, what a working man earns), times the share the law of the draft
    /// pays them.
    pub fn soldier_wages(&self) -> Vec<f32> {
        let avg = self.soldier_pay();
        // (the career men among them are paid as their trade pays, whatever the law)
        let pay = if self.cadre_share > 0.0 { (1.0 - self.cadre_share) * self.draft_pay + self.cadre_share * self.cadre_pay } else { self.draft_pay };
        (0..self.pop.len()).map(|g| self.drafted[g] * if self.unemployed[g].is_some() { self.pay[g] } else { avg } * pay).collect()
    }

    /// `men` millions called up, from each group by how many of it may still be called (at most
    /// `draft_cap` of a group serves at once): how many came.
    pub fn draft(&mut self, men: f32) -> f32 {
        let free: Vec<f32> = (0..self.pop.len()).map(|g| (self.draft_cap * self.pop[g] - self.drafted[g]).max(0.0)).collect();
        let all: f32 = free.iter().sum();
        if all <= 0.0 || men <= 0.0 {
            return 0.0;
        }
        let k = (men / all).min(1.0);
        for (d, f) in self.drafted.iter_mut().zip(&free) {
            *d += f * k;
        }
        k * all
    }

    /// `men` millions leave the army alive and go back to work, from each group by how many of it serve.
    pub fn release(&mut self, men: f32) {
        let all: f32 = self.drafted.iter().sum();
        if all > 0.0 && men > 0.0 {
            let k = (men / all).min(1.0);
            self.drafted.iter_mut().for_each(|d| *d -= *d * k);
        }
    }

    /// The country lives by law `k` of the draft (`LAWS`).
    pub fn set_law(&mut self, k: usize) {
        self.draft_cap = LAWS[k].cap;
        self.draft_pay = LAWS[k].pay;
    }

    /// Who can still be called up: millions.
    pub fn draftable(&self) -> f32 {
        (0..self.pop.len()).map(|g| (self.draft_cap * self.pop[g] - self.drafted[g]).max(0.0)).sum()
    }

    /// `men` millions of those serving die (or are lost as prisoners): each group loses its part
    /// by how many of it serve, and (with `grieve`) mourns them.
    pub fn bury(&mut self, men: f32, grieve: bool) {
        let all: f32 = self.drafted.iter().sum();
        if all <= 0.0 || men <= 0.0 {
            return;
        }
        let k = (men / all).min(1.0);
        for g in 0..self.pop.len() {
            let dead = self.drafted[g] * k;
            self.drafted[g] -= dead;
            if grieve && self.live && self.pop[g] > 0.0 {
                self.discontent[g] += GRIEF * dead / self.pop[g];
            }
            self.pop[g] -= dead;
        }
    }
}
