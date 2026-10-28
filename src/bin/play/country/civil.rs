//! The state's civil budget: what a government pays for besides its army, its science and its
//! building — its officials, schools, doctors, police and courts, relief of the poor. Each is a line
//! the government sets (none … generous), a share of the country's output a month, paid out of
//! what the treasury used to hand back by head (`Nation::civil_by`) to the people who do the work:
//! officials, teachers and doctors are the learned, policemen and orderlies come from the plain
//! people, relief goes to the poorest. A budget bigger than what is left after the army eats the
//! reserve; then the officials go unpaid.
//!
//! What a line buys is not a modifier written beforehand but a stock that grows slowly with the
//! money and fades without it:
//! - officials → the state's reach (`reach`): how much of the tax it actually collects, how much
//!   leaks on the way; unpaid officials steal (`arrears`);
//! - schools → literacy (the order's own, through `Life::schooling`) and, from it, plain people who
//!   become learned (`demography`);
//! - doctors → deaths and epidemics (`demography`), deaths from want;
//! - police and courts → order: unrest that fades faster, owners who trust the courts;
//! - relief → money to the poorest: hunger is bought off with bread, and fewer die of want.

use strategy::pack::Pack;
use strategy::world::polity::{self, Regime};

use crate::country::policy::role;
use crate::game::Note;
use crate::Game;

pub const LINES: usize = 5;
pub const ADMIN: usize = 0;
pub const SCHOOLS: usize = 1;
pub const HEALTH: usize = 2;
pub const ORDER: usize = 3;
pub const RELIEF: usize = 4;

/// A line of the civil budget: its name, icon, what it buys, the words of its levels and the share
/// of the country's output a month each level costs (level 1 is the custom, against which what a
/// line got is measured).
pub struct LineDef {
    pub name: &'static str,
    pub icon: &'static str,
    pub about: &'static str,
    pub levels: [&'static str; 4],
    pub cost: [f32; 4],
}

pub const LINE: [LineDef; LINES] = [
    LineDef {
        name: "Управление",
        icon: "scroll",
        about: "Чиновники, сборщики налогов, ревизоры. Чем лучше они оплачены и чем грамотнее страна, тем дальше достаёт государство: меньше уклоняются от налога, меньше растекается по дороге. Неоплаченный чиновник ворует.",
        levels: ["горстка", "обычное", "крепкое", "вездесущее"],
        cost: [0.003, 0.008, 0.014, 0.022],
    },
    LineDef {
        name: "Школы",
        icon: "graduate",
        about: "Учителя и школы. Грамотность растёт медленно, за поколение; грамотные становятся служащими, учёными, инженерами. Закон о школах говорит, кого учить, а деньги — как.",
        levels: ["ничего", "обычные", "щедрые", "всеобщие"],
        cost: [0.0, 0.006, 0.014, 0.026],
    },
    LineDef {
        name: "Медицина",
        icon: "meds",
        about: "Врачи, больницы, прививки. Меньше умирают — особенно дети и в мор; меньше гибнут от голода. Плоды видны через годы.",
        levels: ["знахари", "земские врачи", "больницы", "охрана здоровья"],
        cost: [0.0, 0.004, 0.01, 0.02],
    },
    LineDef {
        name: "Порядок",
        icon: "shield",
        about: "Полиция, суды, тюрьмы. Смута гаснет быстрее, собственники верят судам. Слишком много полиции злит интеллигенцию и рабочих.",
        levels: ["стража", "полиция", "крепкая полиция", "жандармерия"],
        cost: [0.002, 0.005, 0.009, 0.015],
    },
    LineDef {
        name: "Призрение",
        icon: "bread",
        about: "Пенсии, пособия, бесплатный хлеб беднейшим. Деньги идут тем, кто беднее всех: голод отступает хлебом, а не словами; меньше умирают от нужды. Дорого.",
        levels: ["ничего", "приюты", "пособия", "всеобщее"],
        cost: [0.0, 0.006, 0.015, 0.03],
    },
];

/// Share of the way a month each stock goes toward what its money buys now: health (about three
/// years), order (a year), the schools (four years: teachers are trained), the state's reach.
const HEALTH_PACE: f32 = 0.03;
const ORDER_PACE: f32 = 0.08;
const SCHOOL_PACE: f32 = 0.02;
const REACH_PACE: f32 = 0.015;

/// The civil budget of both sides and what it bought.
#[derive(Clone, Debug)]
pub struct Civil {
    /// Each side's level of each line (0 none … 3 generous).
    pub level: [[u8; LINES]; 2],
    /// What each line asked last month and what it got (money).
    pub asked: [[f32; LINES]; 2],
    pub paid: [[f32; LINES]; 2],
    /// What the money of recent years bought (1: as much as the custom keeps): the schools, the
    /// doctors, the police and courts.
    pub schools: [f32; 2],
    pub health: [f32; 2],
    pub order: [f32; 2],
    /// The state's reach (0.15 … 1): the share of what it is owed that it can collect and keep.
    pub reach: [f32; 2],
    /// How far behind the officials' pay is (a stock: 1 — a year of half pay); graft grows from it.
    pub arrears: [f32; 2],
    /// Whether the start was laid.
    pub ready: bool,
}

impl Default for Civil {
    fn default() -> Civil {
        Civil {
            level: [[1; LINES]; 2],
            asked: [[0.0; LINES]; 2],
            paid: [[0.0; LINES]; 2],
            schools: [1.0; 2],
            health: [1.0; 2],
            order: [1.0; 2],
            reach: [0.6; 2],
            arrears: [0.0; 2],
            ready: false,
        }
    }
}

/// Who a line's money goes to: the weight a head of a group of role `r` (0 the people of the land,
/// 1 the workers, 2 the learned, 3 the owners). Relief goes by want (`relief_weights`).
fn takers(line: usize, r: usize) -> f32 {
    const K: [[f32; 4]; 4] = [
        // officials: the learned, clerks from the workers
        [0.0, 0.15, 1.0, 0.0],
        // teachers
        [0.0, 0.05, 1.0, 0.0],
        // doctors, orderlies
        [0.0, 0.3, 1.0, 0.0],
        // policemen, warders, judges
        [0.25, 0.6, 0.3, 0.0],
    ];
    if line >= RELIEF {
        return 0.0;
    }
    K[line][r.min(3)]
}

/// Relief by want: a group gets the more, the further its income falls short of its needs.
fn relief_weights(pack: &Pack, n: &strategy::economy::nation::Nation) -> Vec<f32> {
    (0..pack.groups.len())
        .map(|g| {
            if role(pack, g) >= 2 || n.pop[g] <= 1e-6 {
                return 0.0;
            }
            let needs: f32 = pack.groups[g].needs.iter().map(|x| x.amount * n.price[x.good]).sum::<f32>() * n.pop[g];
            let has = (n.income[g] - n.taxed.get(g).copied().unwrap_or(0.0)).max(0.0) / 3.0 + n.money[g].max(0.0) / 12.0;
            let short = (1.0 - has / needs.max(1e-9)).clamp(0.05, 1.0);
            needs * short * (1.0 + 2.0 * n.hardship[g])
        })
        .collect()
}

impl Game {
    /// The custom levels of side `s`'s civil budget at the start: what its order, laws and faith
    /// would keep.
    fn civil_custom(&self, s: usize) -> [u8; LINES] {
        let p = self.polity_of(s);
        let charity = self.polity.faiths.get(p.state_faith).map_or(0.0, |f| f.tenets[polity::faith::CHARITY]);
        let mut l = [1u8; LINES];
        l[SCHOOLS] = p.laws[polity::SCHOOLS].min(2);
        l[HEALTH] = if p.literacy > 0.3 { 1 } else { 0 };
        l[ORDER] = if matches!(p.regime, Regime::Junta | Regime::Party | Regime::Autocracy | Regime::Theocracy) { 2 } else { 1 };
        l[RELIEF] = if charity > 0.3 || (p.laws[polity::VOICE] == 2 && p.regime == Regime::Republic) { 1 } else { 0 };
        l
    }

    /// The civil budget's start: the custom levels, the stocks as those levels keep them.
    pub fn civil_start(&mut self) {
        for s in 0..2 {
            let l = self.civil_custom(s);
            self.civil.level[s] = l;
            let cov = |k: usize| LINE[k].cost[l[k] as usize] / LINE[k].cost[1];
            self.civil.schools[s] = cov(SCHOOLS);
            self.civil.health[s] = cov(HEALTH);
            self.civil.order[s] = cov(ORDER);
            self.civil.reach[s] = self.reach_aim(s, cov(ADMIN));
        }
        self.civil.ready = true;
    }

    /// The reach the state's officials would give it, paid `cov` times the custom.
    fn reach_aim(&self, s: usize, cov: f32) -> f32 {
        let lit = self.polity_of(s).literacy;
        (0.35 + 0.3 * cov.clamp(0.0, 2.5).sqrt() + 0.25 * lit - 0.2 * self.civil.arrears[s].min(1.0)).clamp(0.15, 1.0)
    }

    /// What line `k` of side `s` got last month, against the custom (1: as the custom pays).
    pub fn civil_cover(&self, s: usize, k: usize) -> f32 {
        let gdp = self.h.home(s).gdp.max(1e-6);
        self.civil.paid[s][k] / (LINE[k].cost[1] * gdp)
    }

    /// How well side `s`'s schools work, times the custom of its law of schools (for its literacy):
    /// the law says whom to teach, the money how well.
    pub fn schooling(&self, s: usize) -> f32 {
        let law = [0.7, 1.0, 1.5][self.polity_of(s).laws[polity::SCHOOLS].min(2) as usize];
        law * (0.35 + 0.65 * self.civil.schools[s].clamp(0.0, 2.5).sqrt())
    }

    /// Before the world's month: each side's civil budget is laid on its economy — what each group
    /// gets for its service — and deaths from want, which relief and doctors make fewer.
    pub fn civil_apply(&mut self) {
        if !self.civil.ready {
            self.civil_start();
        }
        let pack = self.pack.clone();
        let ng = pack.groups.len();
        for s in 0..2 {
            let level = self.civil.level[s];
            let n = self.h.home(s);
            let gdp = n.gdp.max(0.0);
            let roles: Vec<usize> = (0..ng).map(|g| role(&pack, g)).collect();
            let relief = relief_weights(&pack, n);
            let mut by = vec![0.0f32; ng];
            let mut asked = [0.0f32; LINES];
            for k in 0..LINES {
                let money = LINE[k].cost[level[k].min(3) as usize] * gdp;
                asked[k] = money;
                if money <= 0.0 {
                    continue;
                }
                let w: Vec<f32> = if k == RELIEF { relief.clone() } else { (0..ng).map(|g| n.pop[g] * takers(k, roles[g])).collect() };
                let all: f32 = w.iter().sum();
                // (a pack without such people: the line pays the plain people by head)
                let w: Vec<f32> = if all > 1e-9 { w } else { (0..ng).map(|g| if roles[g] < 3 { n.pop[g] } else { 0.0 }).collect() };
                let all: f32 = w.iter().sum::<f32>().max(1e-9);
                for g in 0..ng {
                    by[g] += money * w[g] / all;
                }
            }
            self.civil.asked[s] = asked;
            if crate::country::econ_spoiled() {
                let n = self.h.home_mut(s);
                n.civil_by = Vec::new();
                n.famine_k = 1.0;
                continue;
            }
            let famine = (1.0 - 0.3 * (self.civil.paid[s][RELIEF] / (LINE[RELIEF].cost[1] * gdp.max(1e-6))).min(2.0) / 2.0 - 0.25 * self.civil.health[s].min(2.0) / 2.0).max(0.4);
            let n = self.h.home_mut(s);
            n.civil_by = by;
            n.famine_k = famine;
        }
    }

    /// After the world's month: what each line got (the treasury may not have paid it all), and the
    /// stocks it buys; unpaid officials; the bot's budget.
    pub fn civil_month(&mut self) -> Vec<String> {
        let now = self.month();
        let me = self.me;
        let mut alerts = Vec::new();
        for s in 0..2 {
            if crate::country::econ_spoiled() {
                continue;
            }
            let n = self.h.home(s);
            let asked: f32 = self.civil.asked[s].iter().sum();
            let k = if asked > 1e-9 { (n.civil_paid / asked).min(1.0) } else { 1.0 };
            for l in 0..LINES {
                self.civil.paid[s][l] = self.civil.asked[s][l] * k;
            }
            let was = self.civil.arrears[s];
            self.civil.arrears[s] = (self.civil.arrears[s] * 0.95 + (1.0 - k) / 6.0).min(2.0);
            if s == me && was < 0.1 && self.civil.arrears[s] >= 0.1 {
                let line = "казна не платит чиновникам и учителям сполна".to_string();
                alerts.push(line.clone());
                self.news.push(Note::new(now, &line, true, "econ").with_tip(format!("выплачено {:.0}% гражданских статей", k * 100.0)));
            }
            let cov = |l: usize| self.civil_cover(s, l);
            let (sc, he, or, ad) = (cov(SCHOOLS), cov(HEALTH), cov(ORDER), cov(ADMIN));
            self.civil.schools[s] += SCHOOL_PACE * (sc - self.civil.schools[s]);
            self.civil.health[s] += HEALTH_PACE * (he - self.civil.health[s]);
            self.civil.order[s] += ORDER_PACE * (or - self.civil.order[s]);
            let aim = self.reach_aim(s, ad);
            self.civil.reach[s] += REACH_PACE * (aim - self.civil.reach[s]);
            // police and courts: the unrest fades faster where they are many
            let c = self.country(s);
            let order = self.civil.order[s];
            let p = &mut self.polity.of[c];
            p.unrest = (p.unrest - 0.003 * (order - 0.5).max(0.0)).max(0.0);
        }
        let bots: Vec<usize> = if self.auto.is_some() { vec![0, 1] } else { vec![1 - me] };
        for s in bots {
            self.bot_civil(s);
        }
        alerts
    }

    /// Side `s` sets line `k` of its civil budget to `lv`.
    pub fn set_civil(&mut self, s: usize, k: usize, lv: u8) {
        if k >= LINES || crate::spoiled() {
            return;
        }
        let lv = lv.min(3);
        if self.civil.level[s][k] == lv {
            return;
        }
        self.civil.level[s][k] = lv;
        self.stamp += 1;
        if s == self.me {
            let now = self.month();
            self.news.push(Note::new(now, &format!("бюджет: {} — {}", LINE[k].name.to_lowercase(), LINE[k].levels[lv as usize]), false, "econ"));
        }
    }

    /// The bot's civil budget (twice a year): what its course cares for, raised where the country
    /// hurts (relief in hunger, police or relief in unrest — as the government's form prefers,
    /// officials where the tax is evaded), cut where the treasury cannot pay; never below what the
    /// law of schools obliges.
    fn bot_civil(&mut self, s: usize) {
        let now = self.month();
        if now % 6 != (3 + s as u32) % 6 {
            return;
        }
        use crate::country::course::Kind;
        let course = self.courses.now[s].as_ref().map(|c| c.kind);
        let mut l = match course {
            Some(Kind::Welfare) => [1, 2, 2, 2, 2],
            Some(Kind::Science) => [1, 3, 1, 1, 0],
            Some(Kind::Industry) => [2, 2, 1, 1, 0],
            Some(Kind::Agrarian) => [1, 1, 1, 1, 1],
            Some(Kind::Army) | Some(Kind::Revanche) | Some(Kind::Fortress) => [1, 1, 1, 2, 0],
            Some(Kind::Thrift) => [2, 1, 0, 1, 0],
            Some(Kind::Trade) => [1, 1, 1, 1, 0],
            None => self.civil_custom(s),
        };
        let f = self.spirits.facts[s].clone();
        let p = self.polity_of(s);
        if f.hunger > 0.05 {
            l[RELIEF] += 1;
        }
        if f.anger > 0.9 || p.unrest > 0.4 {
            if p.regime.repression() >= 0.5 {
                l[ORDER] += 1;
            } else {
                l[RELIEF] += 1;
            }
        }
        if self.fiscal.evaded[s] > 0.12 {
            l[ADMIN] += 1;
        }
        // (a treasury in debt cuts what it can: relief and doctors first)
        if f.treasury < 0.0 || f.debt > 10.0 {
            l[RELIEF] = l[RELIEF].saturating_sub(1);
            l[HEALTH] = l[HEALTH].saturating_sub(1);
        }
        let law = p.laws[polity::SCHOOLS].min(2);
        l[SCHOOLS] = l[SCHOOLS].max(law);
        for (k, &x) in l.iter().enumerate() {
            self.set_civil(s, k, (x as u8).min(3));
        }
    }

    /// What the civil budget does to each force's mood (beyond the money it pays them): a police
    /// state angers the learned and the workers; schools for all please the learned and, where the
    /// church held the schools, anger the clergy.
    pub fn civil_moods(&self, s: usize, b: usize) -> f32 {
        use polity::{CHURCH, CLERGY, LEARNED, NOBLES, OFFICERS, WORKERS};
        let l = self.civil.level[s];
        let mut x = 0.0;
        if l[ORDER] >= 3 {
            x += match b {
                LEARNED => -0.12,
                WORKERS => -0.08,
                NOBLES | OFFICERS => 0.05,
                _ => 0.0,
            };
        }
        if l[SCHOOLS] >= 2 {
            x += match b {
                LEARNED => 0.08,
                CLERGY if self.polity_of(s).laws[CHURCH] == 0 => -0.06,
                _ => 0.0,
            };
        }
        x
    }
}
