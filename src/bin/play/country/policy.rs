//! The state's economic policy, beyond the levers of tax and spending: the order of the economy
//! (a law, as HOI4's economy laws: a free market … a total mobilisation — each a bet: more arms
//! for less of everything else, angrier people), the state's programmes (an industrialisation, a
//! land reform, schools for all, a rearmament, a line of forts, railways, austerity, public works,
//! a patriotic upbringing — each costs a share of the state's income every month for years, and
//! gives what it gives only while it runs or once it is done), loans from abroad (money now, the
//! interest every month, a lender asking more of a shaky government or one at war).
//!
//! Each month, before the world's month, what the country's features (`spirits`), its law and its
//! programmes do together is laid on its economy, laboratories and divisions (`apply_mods`); after
//! it the programmes go on, the debts are paid and the bot's government decides (the same rules as
//! the player's).

use strategy::military::corps::Mods;
use strategy::pack::Pack;
use strategy::rng::{hash64, Rng};

use crate::country::spirits::Eff;
use crate::game::Note;
use crate::Game;

/// The orders of the economy (`LAWS[k]`): name, icon, what it means in words.
pub struct Law {
    pub name: &'static str,
    pub icon: &'static str,
    pub about: &'static str,
}

pub static LAWS: [Law; 4] = [
    Law { name: "Вольный рынок", icon: "shop", about: "Хозяева сами решают, что делать: товаров для людей больше, военные заказы дороже и медленнее." },
    Law { name: "Смешанное хозяйство", icon: "scroll", about: "Как водится: рынок, а казна заказывает своё." },
    Law { name: "Военная экономика", icon: "anvil", about: "Заводы отдают армии лучшее: оружия больше, товаров меньше, люди ворчат." },
    Law { name: "Тотальная мобилизация", icon: "fist", about: "Всё для фронта: военные заводы работают за троих, люди живут впроголодь. В мирное время власть за это поплатится." },
];

/// The usual order (the custom of the age).
pub const USUAL_LAW: usize = 1;

/// How much of what the features, laws and programmes do to fire, harm, organisation, digging and
/// attrition reaches the divisions.
pub const COMBAT_WEIGHT: f32 = 0.5;

/// What a law does, at war or in peace.
pub fn law_eff(k: usize, at_war: bool) -> Eff {
    let e = Eff::default;
    match k {
        0 => Eff { out_civil: 1.04, out_industry: 1.02, out_military: 0.9, make_cost: 1.08, learn: 0.9, anger: [0.0, 0.0, 0.0, -0.004], stability: if at_war { -0.01 } else { 0.0 }, ..e() },
        2 => Eff { out_military: 1.2, out_civil: 0.93, learn: 1.25, make_cost: 0.95, anger: [0.006, 0.008, 0.004, 0.0], stability: if at_war { 0.0 } else { -0.002 }, ..e() },
        3 => Eff { out_military: 1.45, out_civil: 0.82, out_land: 0.95, learn: 1.5, skill: 0.1, make_cost: 0.9, draft: 1.25, anger: [0.02, 0.025, 0.012, 0.006], stability: if at_war { -0.004 } else { -0.015 }, ..e() },
        _ => e(),
    }
}

/// A programme of the state: its name, icon, months, the share of the state's income it takes a
/// month (less than 0: it brings money), what it is, what it does while it runs and once done.
pub struct Plan {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub months: u32,
    pub cost: f32,
    pub about: &'static str,
    pub running: fn() -> Eff,
    pub done: fn() -> Eff,
}

fn e() -> Eff {
    Eff::default()
}

pub static PLANS: &[Plan] = &[
    Plan {
        id: "industry",
        name: "Индустриализация",
        icon: "anvil",
        months: 36,
        cost: 0.1,
        about: "Казна строит заводы по плану, а не там, где выгодно хозяевам. Крестьяне платят за это своим хлебом.",
        running: || Eff { anger: [0.01, 0.0, 0.0, 0.0], ..e() },
        done: || Eff { out_industry: 1.06, learn: 1.1, ..e() },
    },
    Plan {
        id: "land",
        name: "Аграрная реформа",
        icon: "wheat",
        months: 24,
        cost: 0.06,
        about: "Земля — тем, кто её пашет: урожаи растут, крестьяне успокаиваются, собственники злы.",
        running: || Eff { anger: [0.0, 0.0, 0.0, 0.008], ..e() },
        done: || Eff { out_land: 1.08, anger: [-0.003, 0.0, 0.0, 0.0], ..e() },
    },
    Plan {
        id: "schools",
        name: "Всеобщая грамотность",
        icon: "graduate",
        months: 60,
        cost: 0.07,
        about: "Школы в каждой деревне: каждый месяц часть простых людей становится образованными. Наука и заводы получат людей через годы.",
        running: e,
        done: || Eff { science: 1.1, learn: 1.05, ..e() },
    },
    Plan {
        id: "rearm",
        name: "Перевооружение",
        icon: "gears",
        months: 24,
        cost: 0.08,
        about: "Станки и мастера для военных заводов: они быстрее осваивают новые образцы и делают их лучше.",
        running: || Eff { learn: 1.5, skill: 0.1, ..e() },
        done: || Eff { skill: 0.08, learn: 1.1, ..e() },
    },
    Plan {
        id: "forts",
        name: "Линия укреплений",
        icon: "wall",
        months: 18,
        cost: 0.07,
        about: "Бетон и колючая проволока по границе: обороняющиеся дивизии окапываются быстрее и держатся крепче.",
        running: e,
        done: || Eff { dig: 1.4, defend: 1.07, ..e() },
    },
    Plan {
        id: "rail",
        name: "Железные дороги",
        icon: "route",
        months: 36,
        cost: 0.06,
        about: "Казна кладёт пути к границе — к худшим по снабжению местам; когда сеть готова, войска и грузы ходят быстрее.",
        running: e,
        done: || Eff { org_back: 1.05, make_cost: 0.97, ..e() },
    },
    Plan {
        id: "austerity",
        name: "Жёсткая экономия",
        icon: "stamper",
        months: 12,
        cost: -0.08,
        about: "Урезать всё, что можно: казна наполняется, люди злятся, власть слабеет.",
        running: || Eff { anger: [0.012, 0.012, 0.012, 0.004], stability: -0.003, ..e() },
        done: e,
    },
    Plan {
        id: "works",
        name: "Общественные работы",
        icon: "spade",
        months: 12,
        cost: 0.08,
        about: "Дороги, плотины, мосты руками безработных: народ успокаивается, власть крепнет.",
        running: || Eff { anger: [-0.02, -0.02, 0.0, 0.0], stability: 0.004, ..e() },
        done: e,
    },
    Plan {
        id: "patriots",
        name: "Воспитание патриотов",
        icon: "flag",
        months: 24,
        cost: 0.04,
        about: "Газеты, школы и церковь учат любить отечество: людей легче призвать, солдаты стойче.",
        running: || Eff { stability: 0.003, ..e() },
        done: || Eff { draft: 1.1, org_back: 1.03, ..e() },
    },
];

/// At most this many programmes at once.
pub const PLANS_AT_ONCE: usize = 2;

/// A programme running: which (`PLANS`), when begun, months of work paid for so far.
#[derive(Clone, Debug)]
pub struct Running {
    pub plan: usize,
    pub since: u32,
    pub done: u32,
    /// Months it waited for money (the treasury could not pay).
    pub waited: u32,
}

/// A loan: what is still owed, the monthly interest, the months left.
#[derive(Clone, Debug)]
pub struct Loan {
    pub owed: f32,
    pub rate: f32,
    pub left: u32,
}

/// Months of the state's income a loan brings; its term; the most a country may owe.
pub const LOAN_MONTHS: f32 = 6.0;
pub const LOAN_TERM: u32 = 48;
pub const DEBT_MOST: f32 = 24.0;

#[derive(Clone, Debug)]
pub struct Policy {
    pub law: [usize; 2],
    pub running: [Vec<Running>; 2],
    /// Programmes each side finished, and the month.
    pub finished: [Vec<(usize, u32)>; 2],
    pub loans: [Vec<Loan>; 2],
    /// What each owes in all.
    pub debt: [f32; 2],
    /// What each paid last month for its loans and its programmes.
    pub paid: [(f32, f32); 2],
    /// No lender gives a side that defaulted anything before this month.
    pub no_loans_until: [u32; 2],
    /// A trade being modernised (its sector, months of work paid), and how many times each
    /// sector was.
    pub modern: [Option<(usize, u32)>; 2],
    pub modernised: [Vec<u32>; 2],
}

/// Months a modernisation of a trade takes; the share of the state's income it takes a month; what
/// it adds to the trade's productivity the first time (less each time after).
pub const MODERN_MONTHS: u32 = 18;
pub const MODERN_COST: f32 = 0.05;
pub const MODERN_GAIN: f32 = 0.12;

/// What the next modernisation of a trade modernised `times` times adds.
pub fn modern_gain(times: u32) -> f32 {
    MODERN_GAIN / (1.0 + times as f32)
}

impl Default for Policy {
    fn default() -> Policy {
        Policy { law: [USUAL_LAW; 2], running: [Vec::new(), Vec::new()], finished: [Vec::new(), Vec::new()], loans: [Vec::new(), Vec::new()], debt: [0.0; 2], paid: [(0.0, 0.0); 2], no_loans_until: [0; 2], modern: [None, None], modernised: [Vec::new(), Vec::new()] }
    }
}

/// The role a group of the pack plays: 0 the people of the land, 1 the workers, 2 the learned,
/// 3 the owners.
pub fn role(pack: &Pack, g: usize) -> usize {
    if pack.sectors.iter().any(|s| s.owner == g) {
        return 3;
    }
    if pack.science.as_ref().is_some_and(|x| x.learned.contains(&g)) {
        return 2;
    }
    let land: f32 = pack.sectors.iter().filter(|s| s.land).flat_map(|s| s.workers.iter()).filter(|w| w.0 == g).map(|w| w.1).sum();
    let other: f32 = pack.sectors.iter().filter(|s| !s.land).flat_map(|s| s.workers.iter()).filter(|w| w.0 == g).map(|w| w.1).sum();
    if land > other {
        0
    } else {
        1
    }
}

/// The state's money spent on a programme reaches the people who do its work — the workers six
/// parts, the learned four, by head (it used to vanish from the country); a saving (austerity) is
/// pay and pensions cut, from the plain people and the learned by what they hold.
pub fn pay_works(pack: &Pack, n: &mut strategy::economy::nation::Nation, money: f32) {
    let ng = pack.groups.len();
    if money > 0.0 {
        for (r, k) in [(1usize, 0.6f32), (2, 0.4)] {
            let gs: Vec<usize> = (0..ng).filter(|&g| role(pack, g) == r).collect();
            let all: f32 = gs.iter().map(|&g| n.pop[g]).sum();
            let (gs, k) = if all > 1e-9 { (gs, k) } else { ((0..ng).filter(|&g| role(pack, g) < 3).collect(), k) };
            let all: f32 = gs.iter().map(|&g| n.pop[g]).sum::<f32>().max(1e-9);
            for &g in &gs {
                n.money[g] += money * k * n.pop[g] / all;
            }
        }
    } else if money < 0.0 {
        let gs: Vec<usize> = (0..ng).filter(|&g| role(pack, g) < 3).collect();
        let held: f32 = gs.iter().map(|&g| n.money[g].max(0.0)).sum::<f32>().max(1e-9);
        let take = (-money).min(0.5 * held);
        for &g in &gs {
            n.money[g] -= take * n.money[g].max(0.0) / held;
        }
    }
}

/// What a sector is, for the effects on output: (land, military, industry, civil).
pub fn kind_of(pack: &Pack, k: usize) -> (bool, bool, bool, bool) {
    let s = &pack.sectors[k];
    let military = s.outputs.iter().any(|o| pack.goods[o.0].military);
    let civil = s.outputs.iter().any(|o| pack.groups.iter().any(|g| g.needs.iter().any(|n| n.good == o.0 && !n.vital)));
    let industry = !s.land && !military && s.outputs.iter().any(|o| pack.goods[o.0].tradable);
    (s.land, military, industry, civil)
}

impl Game {
    /// The state's income a month (what its tax takes of the output), for the costs of
    /// programmes and the size of loans.
    pub fn income(&self, s: usize) -> f32 {
        let n = self.h.home(s);
        (n.gdp * n.tax).max(1e-6)
    }

    /// What side `s`'s features, law and programmes do together now.
    pub fn total_eff(&self, s: usize) -> Eff {
        let mut e = self.spirit_eff(s).and(&law_eff(self.policy.law[s], self.h.at_war));
        for r in &self.policy.running[s] {
            e = e.and(&(PLANS[r.plan].running)());
        }
        for &(p, _) in &self.policy.finished[s] {
            e = e.and(&(PLANS[p].done)());
        }
        // what the country knows (its techniques), and what its finances do to the people's mood
        // and how it is governed, what it believes, the great tasks it fulfilled
        e.and(&self.tech_eff(s)).and(&self.fiscal_eff(s)).and(&self.realm_eff(s)).and(&self.agenda_eff(s))
    }

    /// Everything that weighs on side `s`, thing by thing, with its name (the parts `total_eff`
    /// puts together): each feature, the order of the economy, each programme, what the country
    /// knows, its finances, its order of society, the great tasks fulfilled.
    pub fn eff_sources(&self, s: usize) -> Vec<(String, Eff)> {
        let f = &self.spirits.facts[s];
        let mut v: Vec<(String, Eff)> = Vec::new();
        for sp in &self.spirits.list[s] {
            v.push((format!("черта «{}»", sp.name(&self.pack)), sp.eff(f, &self.pack)));
        }
        let law = self.policy.law[s];
        v.push((format!("хозяйство: «{}»", LAWS[law].name), law_eff(law, self.h.at_war)));
        for r in &self.policy.running[s] {
            v.push((format!("программа «{}» идёт", PLANS[r.plan].name), (PLANS[r.plan].running)()));
        }
        for &(p, _) in &self.policy.finished[s] {
            v.push((format!("программа «{}» завершена", PLANS[p].name), (PLANS[p].done)()));
        }
        v.push(("техника страны".into(), self.tech_eff(s)));
        v.push(("финансы государства".into(), self.fiscal_eff(s)));
        v.extend(self.realm_eff_parts(s));
        for (g, _) in &self.agenda.rewards[s] {
            v.push((format!("выполненная задача ({})", g.reward_words()), g.reward()));
        }
        v
    }

    /// Before the world's month: what each country's features, law and programmes do, laid on
    /// its economy (the output of its trades, its people's discontent, the money that leaks), its
    /// laboratories, its law of the draft, its divisions and its government's stability.
    pub fn apply_mods(&mut self) {
        let pack = self.pack.clone();
        // human capital: factories work better where more of the people are learned (against the
        // world's share of them)
        let learned: Vec<usize> = pack.science.as_ref().map_or(Vec::new(), |x| x.learned.clone());
        let share = |n: &strategy::economy::nation::Nation| learned.iter().map(|&g| n.pop[g]).sum::<f32>() / n.people().max(1e-6);
        let live: Vec<f32> = self.h.world.nations.iter().filter(|n| n.people() > 0.05).map(share).collect();
        let mean = live.iter().sum::<f32>() / live.len().max(1) as f32;
        // (graft: the less the state reaches and the worse its officials are paid, the more leaks)
        let graft = [0, 1].map(|s| (1.6 - self.civil.reach[s]).clamp(0.6, 1.45) * (1.0 + 1.5 * self.civil.arrears[s].min(1.0)));
        for s in 0..2 {
            let e = self.total_eff(s);
            self.effs[s] = e.clone();
            let skill = if learned.is_empty() || mean <= 1e-6 { 1.0 } else { ((share(self.h.home(s)) / mean).max(1e-3).ln() * 0.08).exp().clamp(0.92, 1.08) };
            let n = self.h.home_mut(s);
            for k in 0..pack.sectors.len() {
                let (land, military, industry, civil) = kind_of(&pack, k);
                let mut m = 1.0f32;
                if industry || military {
                    m *= skill;
                }
                if land {
                    m *= e.out_land;
                }
                if military {
                    m *= e.out_military;
                }
                if industry {
                    m *= e.out_industry;
                }
                if civil {
                    m *= e.out_civil;
                }
                if let Some(x) = n.shock.get_mut(k) {
                    *x = m;
                }
            }
            if n.live {
                for g in 0..pack.groups.len() {
                    let a = e.anger[role(&pack, g)];
                    if a != 0.0 {
                        n.discontent[g] = (n.discontent[g] + a).max(0.0);
                    }
                }
            }
            n.draft_cap = (n.draft_cap * e.draft).min(0.5);
            // (the money that leaks goes to those who take it)
            if e.leak > 0.0 && n.treasury > 0.0 {
                let take = n.state_budget() * e.leak * graft[s];
                n.treasury -= take;
                let owner = pack.sectors.first().map_or(0, |x| x.owner);
                if let Some(m) = n.money.get_mut(owner) {
                    *m += take;
                }
            }
            if let Some(lab) = self.h.labs[s].as_mut() {
                lab.boost = e.science;
            }
            if let Some(c) = self.h.war.corps.as_mut() {
                // (the battle feels a country's ways, but they do not win its wars alone: several
                // features together made a quarter's edge, and a country lost its war in three months)
                let k = |x: f32| 1.0 + (x - 1.0) * COMBAT_WEIGHT;
                c.mods[s] = Mods { attack: k(e.attack), defend: k(e.defend), harm: k(e.harm), org_back: k(e.org_back), dig: k(e.dig), exp_gain: e.exp, attrition: k(e.attrition) };
            }
            self.state.mods[s] += e.stability;
        }
    }

    /// After the world's month: programmes go on (paid, or waiting for money), finished ones are
    /// written down; loans are paid; the bot's government decides.
    pub fn policy_month(&mut self) -> Vec<String> {
        let now = self.month();
        let me = self.me;
        let pack = self.pack.clone();
        let mut alerts = Vec::new();
        for s in 0..2 {
            let income = self.income(s);
            // the programmes
            let mut spent = 0.0f32;
            let mut ended = Vec::new();
            for i in 0..self.policy.running[s].len() {
                let p = &PLANS[self.policy.running[s][i].plan];
                let cost = p.cost * income;
                if cost > 0.0 && self.h.home(s).treasury < cost {
                    self.policy.running[s][i].waited += 1;
                    continue;
                }
                let n = self.h.home_mut(s);
                n.treasury -= cost;
                pay_works(&pack, n, cost);
                spent += cost;
                self.policy.running[s][i].done += 1;
                // (what some do month by month)
                match p.id {
                    "schools" => {
                        // a share of the plain people become learned
                        let learned: Vec<usize> = pack.science.as_ref().map_or(Vec::new(), |x| x.learned.clone());
                        let n = self.h.home_mut(s);
                        if let Some(&to) = learned.first() {
                            for g in 0..pack.groups.len() {
                                if learned.contains(&g) || role(&pack, g) == 3 {
                                    continue;
                                }
                                let move_ = n.pop[g] * 0.0012;
                                n.pop[g] -= move_;
                                n.pop[to] += move_;
                            }
                        }
                    }
                    "industry" => {
                        let n = self.h.home_mut(s);
                        n.plan = n.plan.max(0.6);
                    }
                    "rail" => {
                        if self.policy.running[s][i].done % 3 == 0 {
                            self.bot_line(s);
                        }
                    }
                    _ => {}
                }
                let r = &self.policy.running[s][i];
                if r.done >= p.months {
                    ended.push(i);
                }
            }
            for &i in ended.iter().rev() {
                let r = self.policy.running[s].remove(i);
                let p = &PLANS[r.plan];
                self.policy.finished[s].push((r.plan, now));
                if p.id == "land" {
                    let n = self.h.home_mut(s);
                    for g in 0..pack.groups.len() {
                        if role(&pack, g) == 0 {
                            n.discontent[g] *= 0.6;
                        }
                    }
                }
                if p.id == "industry" {
                    let adviser = s == me && self.adviser.is_some();
                    if s != me || !adviser {
                        self.h.home_mut(s).plan = 0.0;
                    }
                }
                if s == me {
                    let line = format!("программа «{}» завершена", p.name);
                    self.news.push(Note::new(now, &line, true, "nation"));
                    alerts.push(line);
                }
            }
            // the modernisation of a trade
            if let Some((k, done)) = self.policy.modern[s] {
                let cost = MODERN_COST * income;
                if self.h.home(s).treasury >= cost {
                    let n = self.h.home_mut(s);
                    n.treasury -= cost;
                    pay_works(&pack, n, cost);
                    spent += cost;
                    let done = done + 1;
                    if done >= MODERN_MONTHS {
                        let times = self.policy.modernised[s].get(k).copied().unwrap_or(0);
                        let gain = modern_gain(times);
                        if let Some(p) = self.h.home_mut(s).prod.get_mut(k) {
                            *p *= 1.0 + gain;
                        }
                        if self.policy.modernised[s].len() < pack.sectors.len() {
                            self.policy.modernised[s].resize(pack.sectors.len(), 0);
                        }
                        self.policy.modernised[s][k] += 1;
                        self.policy.modern[s] = None;
                        if s == me {
                            let line = format!("модернизация завершена: {} (+{:.0}% производительности)", pack.sectors[k].name, gain * 100.0);
                            self.news.push(Note::new(now, &line, true, "econ"));
                            alerts.push(line);
                        }
                    } else {
                        self.policy.modern[s] = Some((k, done));
                    }
                }
            }
            // the loans
            let mut paid = 0.0f32;
            let n = &mut self.h.world.nations[self.h.at[s]];
            for l in self.policy.loans[s].iter_mut() {
                let interest = l.owed * l.rate;
                let back = l.owed / l.left.max(1) as f32;
                let pay = interest + back;
                n.treasury -= pay;
                paid += pay;
                l.owed -= back;
                l.left = l.left.saturating_sub(1);
            }
            self.policy.loans[s].retain(|l| l.left > 0 && l.owed > 1e-6);
            self.policy.debt[s] = self.policy.loans[s].iter().map(|l| l.owed).sum();
            self.policy.paid[s] = (paid, spent);
        }
        // the bot's government
        let bots: Vec<usize> = if self.auto.is_some() { vec![0, 1] } else { vec![1 - me] };
        for s in bots {
            self.bot_policy(s);
        }
        alerts
    }

    /// The interest a lender asks of side `s` now (a month): more of a shaky government, more of
    /// one at war, more of one deep in debt.
    pub fn loan_rate(&self, s: usize) -> f32 {
        let st = self.state.stability[s].clamp(0.0, 1.0);
        let debt = self.policy.debt[s] / self.income(s);
        0.005 + 0.008 * (1.0 - st) + if self.h.at_war { 0.004 } else { 0.0 } + 0.0004 * debt
    }

    /// Side `s` takes a loan, if lenders still give it one; whether it got it.
    pub fn take_loan(&mut self, s: usize) -> bool {
        let income = self.income(s);
        let amount = LOAN_MONTHS * income;
        if self.policy.debt[s] + amount > DEBT_MOST * income || self.month() < self.policy.no_loans_until[s] {
            return false;
        }
        let rate = self.loan_rate(s);
        self.policy.loans[s].push(Loan { owed: amount, rate, left: LOAN_TERM });
        self.policy.debt[s] += amount;
        self.h.home_mut(s).treasury += amount;
        if s == self.me {
            let now = self.month();
            self.news.push(Note::new(now, "казна взяла заём за границей", false, "nation").with_tip(format!("{amount:.1} под {:.1}% в месяц на {LOAN_TERM} месяцев", rate * 100.0)));
        }
        true
    }

    /// Side `s` pays off what it can of its oldest loan at once; whether it paid anything.
    pub fn repay(&mut self, s: usize) -> bool {
        let n = self.h.home(s);
        let can = n.treasury.max(0.0) * 0.8;
        let Some(l) = self.policy.loans[s].first_mut() else { return false };
        let pay = l.owed.min(can);
        if pay <= 1e-6 {
            return false;
        }
        l.owed -= pay;
        self.h.home_mut(s).treasury -= pay;
        self.policy.loans[s].retain(|l| l.owed > 1e-6);
        self.policy.debt[s] = self.policy.loans[s].iter().map(|l| l.owed).sum();
        true
    }

    /// Side `s` begins programme `p`, if it may (not already running or done, room for it);
    /// whether it began.
    pub fn begin_plan(&mut self, s: usize, p: usize) -> bool {
        if p >= PLANS.len() || self.policy.running[s].len() >= PLANS_AT_ONCE || self.policy.running[s].iter().any(|r| r.plan == p) {
            return false;
        }
        // (a programme with lasting fruits is done once; austerity and works may come again)
        if self.policy.finished[s].iter().any(|f| f.0 == p) && !matches!(PLANS[p].id, "austerity" | "works") {
            return false;
        }
        let now = self.month();
        self.policy.running[s].push(Running { plan: p, since: now, done: 0, waited: 0 });
        if s == self.me {
            self.news.push(Note::new(now, &format!("начата программа «{}»", PLANS[p].name), false, "nation"));
        } else {
            let name = self.h.home(s).name.clone();
            self.news.push(Note::new(now, &format!("{name}: начата программа «{}»", PLANS[p].name), false, "politics"));
        }
        true
    }

    /// Side `s` stops programme `p` (what was paid is lost).
    pub fn stop_plan(&mut self, s: usize, p: usize) {
        self.policy.running[s].retain(|r| r.plan != p);
    }

    /// Side `s` begins to modernise trade `k` (one at a time); whether it began.
    pub fn modernise(&mut self, s: usize, k: usize) -> bool {
        if k >= self.pack.sectors.len() || self.policy.modern[s].is_some() {
            return false;
        }
        self.policy.modern[s] = Some((k, 0));
        if s == self.me {
            let now = self.month();
            self.news.push(Note::new(now, &format!("начата модернизация: {}", self.pack.sectors[k].name), false, "econ"));
        }
        true
    }

    /// Side `s` lives by economic law `k`.
    pub fn set_eco_law(&mut self, s: usize, k: usize) {
        // (the spoiled game of the window's self-checks takes the word and does nothing)
        if crate::spoiled() {
            return;
        }
        if k < LAWS.len() && self.policy.law[s] != k {
            self.policy.law[s] = k;
            // (a change of order shakes the government a little)
            self.state.mods[s] -= 0.03;
            let now = self.month();
            if s == self.me {
                self.news.push(Note::new(now, &format!("хозяйство живёт по закону «{}»", LAWS[k].name), false, "nation"));
            } else {
                let name = self.h.home(s).name.clone();
                self.news.push(Note::new(now, &format!("{name} переходит к закону «{}»", LAWS[k].name), true, "politics"));
            }
        }
    }

    /// The bot's government over its policy: the law by the war and the danger; a programme for
    /// its worst trouble when it can pay; a loan in a war with an empty treasury. (Its goal is to
    /// win at the least cost: it mobilises only when it must.)
    /// The law a bot's government wants for side `s` now: the war economy in a war or under a
    /// threat, a total mobilisation in a war going badly, the custom in peace.
    pub fn bot_law(&self, s: usize) -> usize {
        let f = &self.spirits.facts[s];
        let danger = self.state.claim.iter().enumerate().any(|(x, c)| x != s && c.is_some()) || self.state.tension > 0.5;
        if self.h.at_war {
            if f.held < 0.85 {
                3
            } else {
                2
            }
        } else if danger {
            2
        } else {
            // in peace, as its course prefers
            self.course_law(s).unwrap_or(USUAL_LAW)
        }
    }

    fn bot_policy(&mut self, s: usize) {
        let now = self.month();
        let f = self.spirits.facts[s].clone();
        let danger = self.state.claim.iter().enumerate().any(|(x, c)| x != s && c.is_some()) || self.state.tension > 0.5;
        let law = self.bot_law(s);
        // (a law is changed no oftener than every six months)
        if law != self.policy.law[s] && now % 6 == (s as u32 * 3) % 6 {
            self.set_eco_law(s, law);
        }
        // a programme, twice a year, when there is money for it
        if now % 6 == 2 && self.policy.running[s].len() < PLANS_AT_ONCE {
            let income = self.income(s);
            let n = self.h.home(s);
            let id = |x: &str| PLANS.iter().position(|p| p.id == x).unwrap();
            let mut wish: Vec<usize> = Vec::new();
            // what its course wants first
            for p in self.course_plans(s) {
                wish.push(id(p));
            }
            if f.debt > 8.0 || f.treasury < -1.0 {
                wish.push(id("austerity"));
            }
            if f.anger > 1.0 {
                wish.push(id("works"));
            }
            if f.hunger > 0.05 {
                wish.push(id("land"));
            }
            if self.h.at_war || danger {
                wish.push(id("rearm"));
                if f.capital_depth <= 3 {
                    wish.push(id("forts"));
                }
            }
            if f.learned < 0.12 {
                wish.push(id("schools"));
            }
            if f.land_share > 0.5 {
                wish.push(id("industry"));
            }
            wish.push(id("rail"));
            wish.push(id("patriots"));
            let mut r = Rng::new(hash64(self.seed ^ 0xB0_11C ^ (now as u64) << 3 ^ s as u64));
            if n.treasury > 4.0 * income && r.f32() < 0.6 {
                for p in wish {
                    if self.begin_plan(s, p) {
                        break;
                    }
                }
            } else if let Some(&p) = wish.first().filter(|&&p| PLANS[p].cost < 0.0) {
                self.begin_plan(s, p);
            }
        }
        // a trade to modernise: the one whose goods are dearest against the usual (the course's
        // trades count for more)
        if now % 12 == 7 && self.policy.modern[s].is_none() {
            let n = self.h.home(s);
            let pack = &self.pack;
            let lean = self.course_leaning(s);
            let dear = (0..pack.sectors.len()).filter(|&k| n.worked.get(k).copied().unwrap_or(0.0) > 0.01).max_by(|&a, &b| {
                let r = |k: usize| pack.sectors[k].outputs.first().map_or(1.0, |o| n.price[o.0] / (pack.goods[o.0].price * n.level).max(1e-9)) * lean.sectors.get(k).copied().unwrap_or(1.0).sqrt();
                r(a).total_cmp(&r(b))
            });
            if let Some(k) = dear {
                if n.treasury > 3.0 * self.income(s) {
                    self.modernise(s, k);
                }
            }
        }
        // a loan, in a war with an empty treasury
        if self.h.at_war && self.h.home(s).treasury < 0.0 && f.debt < 12.0 {
            self.take_loan(s);
        }
    }
}
