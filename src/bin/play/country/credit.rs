//! Banks and the state's debt at home.
//!
//! Banks lend the owners money they did not have: a loan is new money, a repaid loan is money gone.
//! How much they mean to lend — the depth of credit, a share of the money the output needs — follows the bank
//! rate the government sets (cheap money lends more), the owners' confidence and how developed the
//! country is. Loans are rolled over as they fall due, so in a calm economy lending only follows its
//! growth; cheap money swells the stock of loans (owners build, prices rise), dear money makes them
//! repay (building slows, prices calm). A swollen stock of loans in a slump or a run of prices ends
//! in a bank crisis: part of the loans is lost, depositors lose savings with the banks that fail,
//! banks lend half as much for a year and a half, the owners lose heart.
//!
//! The state borrows at home by selling bonds: the owners (and the learned) pay money into the
//! treasury — money they would have built with — and get interest and their money back over ten
//! years out of the taxes. Lenders ask more of a shaky government, of one at war and of one deep in
//! debt. A government whose interest eats half its income may repudiate: the bondholders never
//! forget it.

use strategy::rng::{hash64, Rng};

use crate::country::policy::role;
use crate::game::Note;
use crate::Game;

/// The bank rate a month by the policy of credit (cheap, usual, dear, the gold standard).
pub const RATE: [f32; 4] = [0.0025, 0.004, 0.007, 0.0045];
/// The depth of credit the banks aim at, by the policy of credit: a share of the money the country
/// holds (what its output needs in coin, at the start — credit is measured against it, or a change
/// of the rate would double the money).
const DEPTH: [f32; 4] = [0.55, 0.4, 0.25, 0.3];
/// Share of the gap to the depth they aim at that banks close a month; months a loan runs.
const LEND_PACE: f32 = 0.03;
const LOAN_MONTHS: f32 = 96.0;
/// Months of a bond and of a credit crunch; bonds sold at once, in months of the state's income;
/// the most a state may owe at home and abroad, in months of its income.
const BOND_MONTHS: f32 = 120.0;
const CRUNCH: u32 = 18;
const BOND_ISSUE: f32 = 3.0;
const DEBT_MOST: f32 = 36.0;

/// Banks and the state's debt at home, both sides.
#[derive(Clone, Debug, Default)]
pub struct Credit {
    /// What the owners owe the banks; what was lent and repaid last month.
    pub loans: [f32; 2],
    pub lent: [f32; 2],
    pub repaid: [f32; 2],
    /// Months of a credit crunch left; bank crises in the game.
    pub crunch: [u32; 2],
    pub crises: [u32; 2],
    /// Bonds the people hold; their interest a month; what the treasury paid on them last month.
    pub bonds: [f32; 2],
    pub bond_rate: [f32; 2],
    pub bond_paid: [f32; 2],
    /// Whether the government repudiated its debt at home (the month), and the money in the country
    /// month by month over the last year.
    pub repudiated: [Option<u32>; 2],
    pub money: [Vec<f32>; 2],
    /// The money each country held for a month's output at the start (what its credit is measured
    /// against, times its output now).
    pub coin: [f32; 2],
    pub ready: bool,
}

impl Game {
    /// The depth of credit side `s`'s banks aim at now: a share of the money its output needs.
    pub fn credit_depth(&self, s: usize) -> f32 {
        let dev = self.dev_of(self.country(s)).clamp(0.0, 1.0);
        let crunch = if self.credit.crunch[s] > 0 { 0.5 } else { 1.0 };
        DEPTH[self.fiscal.rate[s].min(3)] * (0.75 + 0.5 * dev) * self.fiscal.confidence[s].clamp(0.3, 1.6) * crunch
    }

    /// The money side `s`'s output needs now, as its coin at the start says (what credit is
    /// measured against).
    pub fn coin_base(&self, s: usize) -> f32 {
        self.credit.coin[s] * self.h.home(s).gdp.max(0.0)
    }

    /// The banks' start: the stock of loans the past left, at the custom depth (no money made by
    /// it: it is the money there is).
    pub fn credit_start(&mut self) {
        for s in 0..2 {
            let gdp = self.h.home(s).gdp.max(1e-6);
            self.credit.coin[s] = self.money_supply(s) / gdp;
            self.credit.loans[s] = self.credit_depth(s) * self.coin_base(s);
        }
        self.credit.ready = true;
    }

    /// All money in side `s`'s country: in the people's hands, in the treasury, in the trades' tills.
    pub fn money_supply(&self, s: usize) -> f32 {
        let n = self.h.home(s);
        n.money.iter().sum::<f32>() + n.treasury.max(0.0) + n.cash.iter().sum::<f32>()
    }

    /// The growth of side `s`'s money over the last year (a share).
    pub fn money_growth(&self, s: usize) -> f32 {
        let m = &self.credit.money[s];
        match (m.first(), m.last()) {
            (Some(a), Some(b)) if m.len() >= 2 && *a > 1e-6 => (b / a).powf(12.0 / (m.len() - 1) as f32) - 1.0,
            _ => 0.0,
        }
    }

    /// The interest side `s`'s lenders at home ask now (a month).
    pub fn bond_rate_now(&self, s: usize) -> f32 {
        let st = self.state.stability[s].clamp(0.0, 1.0);
        let income = self.income(s);
        let debt = (self.credit.bonds[s] + self.policy.debt[s]) / income;
        let burnt = if self.credit.repudiated[s].is_some() { 0.006 } else { 0.0 };
        RATE[self.fiscal.rate[s].min(3)] + 0.0015 + 0.006 * (1.0 - st) + 0.00025 * debt + if self.h.at_war { 0.002 } else { 0.0 } + burnt
    }

    /// The month of the banks and of the debt at home, both sides: lending and repaying, a crisis;
    /// the bonds' interest; the bot's borrowing. What should stop the clock.
    pub fn credit_month(&mut self) -> Vec<String> {
        let now = self.month();
        let me = self.me;
        let pack = self.pack.clone();
        let ng = pack.groups.len();
        let mut alerts = Vec::new();
        let mut r = Rng::new(hash64(self.seed ^ 0xC2ED ^ (now as u64) << 4));
        if crate::country::econ_spoiled() {
            return alerts;
        }
        if !self.credit.ready {
            self.credit_start();
        }
        let owners: Vec<usize> = (0..ng).filter(|&g| role(&pack, g) == 3).collect();
        let learned: Vec<usize> = (0..ng).filter(|&g| role(&pack, g) == 2).collect();
        for s in 0..2 {
            let base = self.coin_base(s);
            let aim = self.credit_depth(s) * base;
            let usual = DEPTH[1] * (0.75 + 0.5 * self.dev_of(self.country(s)).clamp(0.0, 1.0)) * base;
            let f = self.spirits.facts[s].clone();
            let loans = self.credit.loans[s];
            // loans fall due and are rolled over; the stock goes toward what the banks aim at
            let due = loans / LOAN_MONTHS;
            let gap = aim - loans;
            let (lent, repaid) = if gap >= 0.0 { (due + LEND_PACE * gap, due) } else { (due, due - LEND_PACE * gap) };
            // (what the owners cannot repay out of their money stays owed)
            let n = self.h.home_mut(s);
            let cash: f32 = owners.iter().map(|&g| n.money[g].max(0.0)).sum();
            let net = lent - repaid;
            let net = if net < 0.0 { net.max(-0.5 * cash) } else { net };
            for &g in &owners {
                let w = if cash > 1e-9 { n.money[g].max(0.0) / cash } else { 1.0 / owners.len() as f32 };
                n.money[g] += net * w;
            }
            self.credit.loans[s] = (loans + net).max(0.0);
            self.credit.lent[s] = lent;
            self.credit.repaid[s] = lent - net;
            self.credit.crunch[s] = self.credit.crunch[s].saturating_sub(1);
            // a crisis: a swollen stock of loans in a slump or in a run of prices
            let swollen = self.credit.loans[s] > 1.25 * usual;
            let shaken = f.growth < -0.02 || f.inflation > 0.15;
            if self.credit.crunch[s] == 0 && swollen && shaken && r.f32() < 0.08 {
                let lost = 0.15 * self.credit.loans[s];
                self.credit.loans[s] -= lost;
                let n = self.h.home_mut(s);
                // (depositors lose savings with the banks that fail: half of what was lost)
                let mut gone = 0.5 * lost;
                for &g in owners.iter().chain(learned.iter()) {
                    let x = (n.money[g].max(0.0) * 0.3).min(gone);
                    n.money[g] -= x;
                    gone -= x;
                }
                self.credit.crunch[s] = CRUNCH;
                self.credit.crises[s] += 1;
                self.fiscal.confidence[s] = (self.fiscal.confidence[s] - 0.25).max(0.3);
                self.fiscal.panic[s] = true;
                let name = self.h.home(s).name.clone();
                let line = if s == me { "банковский кризис: банки лопаются, кредит иссяк".to_string() } else { format!("{name}: банковский кризис") };
                let tip = format!("потеряно {:.0}% займов; полтора года банки дают вдвое меньше", 15.0);
                if s == me {
                    alerts.push(line.clone());
                }
                self.news.push(Note::new(now, &line, s == me, "econ").with_tip(tip));
            }
            // the bonds: interest and a tenth of the debt a year, out of the treasury to the holders
            let bonds = self.credit.bonds[s];
            if bonds > 1e-6 {
                let interest = bonds * self.credit.bond_rate[s];
                let back = bonds / BOND_MONTHS;
                let pay = interest + back;
                let n = self.h.home_mut(s);
                n.treasury -= pay;
                pay_holders(n, &owners, &learned, pay);
                self.credit.bonds[s] -= back;
                self.credit.bond_paid[s] = pay;
            } else {
                self.credit.bond_paid[s] = 0.0;
            }
            let m = self.money_supply(s);
            let ring = &mut self.credit.money[s];
            ring.push(m);
            if ring.len() > 13 {
                ring.remove(0);
            }
        }
        let bots: Vec<usize> = if self.auto.is_some() { vec![0, 1] } else { vec![1 - me] };
        for s in bots {
            self.bot_credit(s);
        }
        alerts
    }

    /// Side `s` sells bonds at home (three months of its income), if anyone will buy them; whether
    /// it sold.
    pub fn sell_bonds(&mut self, s: usize) -> bool {
        if crate::spoiled() || crate::country::econ_spoiled() {
            return false;
        }
        let pack = self.pack.clone();
        let ng = pack.groups.len();
        let income = self.income(s);
        if self.credit.bonds[s] + self.policy.debt[s] + BOND_ISSUE * income > DEBT_MOST * income {
            return false;
        }
        let holders: Vec<(usize, f32)> = (0..ng).filter_map(|g| match role(&pack, g) {
            3 => Some((g, 0.7)),
            2 => Some((g, 0.3)),
            _ => None,
        }).collect();
        let n = self.h.home(s);
        // (who buys: the owners and the learned, each at most a quarter of what they hold)
        let can: f32 = holders.iter().map(|&(g, _)| 0.25 * n.money[g].max(0.0)).sum();
        let amount = (BOND_ISSUE * income).min(can);
        if amount < 0.5 * income {
            return false;
        }
        let rate = self.bond_rate_now(s);
        let n = self.h.home_mut(s);
        let mut left = amount;
        let w: f32 = holders.iter().map(|&(g, k)| k * n.money[g].max(0.0)).sum::<f32>().max(1e-9);
        for &(g, k) in &holders {
            let x = (amount * k * n.money[g].max(0.0) / w).min(0.25 * n.money[g].max(0.0)).min(left);
            n.money[g] -= x;
            left -= x;
        }
        let sold = amount - left;
        n.treasury += sold;
        let old = self.credit.bonds[s];
        self.credit.bond_rate[s] = if old + sold > 1e-9 { (old * self.credit.bond_rate[s] + sold * rate) / (old + sold) } else { rate };
        self.credit.bonds[s] = old + sold;
        self.stamp += 1;
        if s == self.me {
            let now = self.month();
            self.news.push(Note::new(now, "казна продала облигации своим гражданам", false, "econ").with_tip(format!("{sold:.1} под {:.2}% в месяц на десять лет", rate * 100.0)));
        }
        true
    }

    /// Side `s` buys back what it can of its bonds at once (out of its treasury); whether it did.
    pub fn redeem_bonds(&mut self, s: usize) -> bool {
        let pack = self.pack.clone();
        let ng = pack.groups.len();
        let owners: Vec<usize> = (0..ng).filter(|&g| role(&pack, g) == 3).collect();
        let learned: Vec<usize> = (0..ng).filter(|&g| role(&pack, g) == 2).collect();
        let pay = self.credit.bonds[s].min(self.h.home(s).treasury.max(0.0) * 0.6);
        if pay <= 1e-6 || crate::spoiled() {
            return false;
        }
        let n = self.h.home_mut(s);
        n.treasury -= pay;
        pay_holders(n, &owners, &learned, pay);
        self.credit.bonds[s] -= pay;
        self.stamp += 1;
        true
    }

    /// Side `s` repudiates half its debt at home: the treasury breathes, the bondholders never
    /// forget it.
    pub fn repudiate(&mut self, s: usize) {
        if self.credit.bonds[s] <= 1e-6 || crate::spoiled() {
            return;
        }
        let now = self.month();
        self.credit.bonds[s] *= 0.5;
        self.credit.repudiated[s] = Some(now);
        self.fiscal.confidence[s] = (self.fiscal.confidence[s] - 0.3).max(0.3);
        self.fiscal.seized[s] += 0.05;
        self.state.mods[s] -= 0.08;
        let c = self.country(s);
        let p = &mut self.polity.of[c];
        for b in [strategy::world::polity::CAPITAL, strategy::world::polity::MERCHANTS, strategy::world::polity::NOBLES] {
            p.mood[b] = (p.mood[b] - 0.25).max(-1.0);
        }
        let name = self.h.home(s).name.clone();
        let line = if s == self.me { "правительство отказалось платить половину внутреннего долга".to_string() } else { format!("{name} отказывается платить по своим облигациям") };
        self.news.push(Note::new(now, &line, true, if s == self.me { "econ" } else { "politics" }));
        self.stamp += 1;
    }

    /// The bot's borrowing at home (every season): bonds when the treasury is empty and the owners
    /// have money (a loan abroad is the policy's); a buy-back when the treasury is fat; repudiation
    /// when the interest eats half the income in a war or a crisis.
    fn bot_credit(&mut self, s: usize) {
        let now = self.month();
        if now % 3 != (2 + s as u32) % 3 {
            return;
        }
        let f = self.spirits.facts[s].clone();
        let income = self.income(s);
        if self.credit.bond_paid[s] > 0.5 * income && f.treasury < 0.0 && (self.h.at_war || self.credit.crunch[s] > 0) {
            self.repudiate(s);
            return;
        }
        if f.treasury < 0.5 {
            self.sell_bonds(s);
        } else if f.treasury > 6.0 && self.credit.bonds[s] > 0.0 {
            self.redeem_bonds(s);
        }
    }
}

/// Pays `pay` to the bondholders: the owners seven parts, the learned three, each by what they hold.
fn pay_holders(n: &mut strategy::economy::nation::Nation, owners: &[usize], learned: &[usize], pay: f32) {
    let parts: [(&[usize], f32); 2] = [(owners, if learned.is_empty() { 1.0 } else { 0.7 }), (learned, 0.3)];
    for (gs, k) in parts {
        if gs.is_empty() {
            continue;
        }
        let w: f32 = gs.iter().map(|&g| n.pop[g]).sum::<f32>().max(1e-9);
        for &g in gs {
            n.money[g] += pay * k * n.pop[g] / w;
        }
    }
}
