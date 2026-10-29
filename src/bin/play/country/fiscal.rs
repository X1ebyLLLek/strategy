//! The state's finances beyond the tax and the programmes: who pays the tax (a flat one, a
//! progressive one, a poll tax), to whom the state hands back what it does not spend (by head, to the
//! poor, to soldiers' families, to entrepreneurs), duties on foreign goods (free trade … autarky),
//! a cap on the prices of bread and what people cannot live without (price control — and queues),
//! the labour law (workers' share of what their sector adds), credit (the interest rate: cheap money
//! builds and inflates, dear money calms prices and slows building), the printing press (money for
//! the treasury now, dearer bread later), subsidies to chosen trades and state ownership of them.
//!
//! None of it is a modifier written beforehand: each lever moves the economy's core (`Nation`'s
//! levers of policy) — who has how much money, what pays to build, what comes from abroad — and
//! what follows (prices, investment, unemployment, discontent) comes out of the market itself. A
//! lever's only direct weight is on the mood of those it hits (owners do not like to be taxed,
//! nationalised or to pay unions) and, for some, on the government's standing. The bot's minister
//! decides by the same rules: by its course and what the country lives through.

use strategy::pack::Pack;

use crate::country::policy::role;
use crate::country::spirits::Eff;
use crate::game::Note;
use crate::Game;

/// A choice of a lever: its name, icon, what it means.
pub struct Choice {
    pub name: &'static str,
    pub icon: &'static str,
    pub about: &'static str,
}

pub static TAXES: [Choice; 4] = [
    Choice { name: "Плоский налог", icon: "scroll", about: "Все платят одну долю дохода." },
    Choice { name: "Прогрессивный налог", icon: "stamper", about: "Богатые платят больше, бедные меньше: крестьяне и рабочие довольнее, собственники ворчат и меньше строят." },
    Choice { name: "Налог на роскошь", icon: "gem", about: "Собственники отдают втрое больше обычного: деньги казне и бедным, собственники злы, стройка их стоит." },
    Choice { name: "Подушная подать", icon: "people", about: "С каждого поровну — бедные платят больше доли, богатые меньше: собственники строят, простой народ ропщет." },
];

/// The tax at the till: a share of what the buyer pays, on top (35th session).
pub static SALES: [Choice; 4] = [
    Choice { name: "Нет", icon: "stop", about: "Покупки ничем не облагаются." },
    Choice { name: "Лёгкий акциз", icon: "shop", about: "5% к цене товаров, кроме хлеба и того, без чего не живут. Платит тот, кто покупает: богатые тратят больше — платят больше." },
    Choice { name: "Тяжёлый акциз", icon: "gem", about: "12% к цене товаров, кроме хлеба. Казне много, но покупают меньше — и заводам, что их делают, хуже." },
    Choice { name: "Налог на всё", icon: "hazard", about: "10% и на хлеб: казне больше всего, а бедные, что тратят всё на еду, отдают долю побольше богатых." },
];

/// The tax on the profit of enterprises: taken from what sectors pay out to their owners.
pub static PROFIT: [Choice; 4] = [
    Choice { name: "Нет", icon: "stop", about: "Прибыль целиком достаётся владельцам: они строят и вкладывают." },
    Choice { name: "Лёгкий (10%)", icon: "scroll", about: "Десятая часть выплат владельцам идёт казне." },
    Choice { name: "Средний (20%)", icon: "stamper", about: "Пятая часть: казне заметно, владельцам меньше на новые заводы." },
    Choice { name: "Тяжёлый (35%)", icon: "fist", about: "Больше трети: казна богатеет, а стройка частных заводов стихает, уверенность владельцев падает." },
];

/// The tax on property: a share of the money a group holds, each month.
pub static ESTATE: [Choice; 4] = [
    Choice { name: "Нет", icon: "stop", about: "Накопленное не облагается." },
    Choice { name: "На капитал и земли", icon: "village", about: "0,6% запаса денег собственников в месяц: платят только владельцы, простой народ не затронут." },
    Choice { name: "Тяжёлый на капитал", icon: "gem", about: "1,5% запаса в месяц с собственников: казне много, они прячут и вывозят деньги." },
    Choice { name: "На любое имущество", icon: "people", about: "0,5% запаса со всех и 1,5% с собственников: у простых людей меньше сбережений на чёрный день." },
];

/// Rates of the till tax a good bears at level `k` (share of its price): none, light on goods that
/// are not a ration, heavy on them, the same on all.
pub fn sales_of(pack: &Pack, k: usize, i: usize) -> f32 {
    let g = &pack.goods[i];
    if g.military {
        return 0.0;
    }
    let food = pack.ration(i);
    match k {
        1 if !food => 0.05,
        2 if !food => 0.12,
        3 => 0.10,
        _ => 0.0,
    }
}

/// The share of the profit tax at level `k`.
pub const PROFIT_RATE: [f32; 4] = [0.0, 0.10, 0.20, 0.35];

/// The monthly share of the money held that the estate tax takes from a group of role `r` at level `k`.
pub fn estate_of(k: usize, r: usize) -> f32 {
    const K: [[f32; 4]; 4] = [[0.0; 4], [0.0, 0.0, 0.0, 0.006], [0.0, 0.0, 0.0, 0.015], [0.005, 0.005, 0.005, 0.015]];
    K[k.min(3)][r.min(3)]
}

pub static DOLES: [Choice; 4] = [
    Choice { name: "Поровну на душу", icon: "people", about: "Что казна не тратит, раздаётся всем поровну." },
    Choice { name: "Бедным", icon: "bread", about: "Раздача тем, кто беднее: голодных меньше, бедные спокойнее." },
    Choice { name: "Семьям солдат", icon: "helmet", about: "Раздача семьям тех, кто служит: в войну народ терпеливее." },
    Choice { name: "Предпринимателям", icon: "coins", about: "Казна отдаёт остаток собственникам: у них больше денег на стройку, бедным меньше." },
];

pub static TARIFFS: [Choice; 4] = [
    Choice { name: "Свободная торговля", icon: "shop", about: "Без пошлин: везут что дешевле, своё не защищено." },
    Choice { name: "Умеренные пошлины", icon: "coins", about: "Десятая часть цены чужих товаров — в казну." },
    Choice { name: "Защита промышленности", icon: "anvil", about: "Высокие пошлины на чужие машины и товары: своим заводам выгодно строиться, всё дороже." },
    Choice { name: "Автаркия", icon: "wall", about: "Чужое почти не везут: всё своё, всё дороже, ничего не зависит от соседей." },
];

pub static LABOUR: [Choice; 4] = [
    Choice { name: "Без защиты труда", icon: "hand", about: "Хозяева платят сколько хотят: прибыль и стройка больше, рабочие злы." },
    Choice { name: "Как водится", icon: "scroll", about: "Плата по обычаю отрасли." },
    Choice { name: "Сильные профсоюзы", icon: "fist", about: "Рабочим доля больше: они спокойнее и покупают больше, хозяева строят меньше." },
    Choice { name: "Рабочий контроль", icon: "revolt", about: "Заводы почти отдают доход работникам: те довольны, хозяева в ярости, стройки почти нет." },
];

pub static RATES: [Choice; 4] = [
    Choice { name: "Дешёвый кредит", icon: "coins", about: "Банк даёт деньги легко: стройка растёт, а с ней цены." },
    Choice { name: "Обычная ставка", icon: "scroll", about: "Как водится." },
    Choice { name: "Дорогие деньги", icon: "stamper", about: "Банк берёт деньги из оборота: цены спокойнее, стройка медленнее." },
    Choice { name: "Золотой стандарт", icon: "gem", about: "Деньги обеспечены металлом: печатать нельзя, стройка скромнее, доверие к власти выше." },
];

pub static PRINTS: [Choice; 4] = [
    Choice { name: "Станок стоит", icon: "pause", about: "Казна живёт на налоги и займы." },
    Choice { name: "Понемногу", icon: "coins", about: "Каждый месяц казна печатает десятую часть своих доходов." },
    Choice { name: "Щедро", icon: "coins", about: "Треть доходов казны — новые деньги: цены пойдут вверх." },
    Choice { name: "Без остановки", icon: "hazard", about: "Казна печатает почти столько, сколько собирает: деньги дешевеют на глазах." },
];

/// What a subsidy level gives a sector a month (a share of the state's budget), and a state
/// ownership level takes of its dividends.
pub const SUBSIDY: [f32; 4] = [0.0, 0.01, 0.025, 0.05];
pub const PUBLIC: [f32; 4] = [0.0, 0.25, 0.5, 1.0];
/// The cap on vital goods, times their usual price.
pub const CAP: f32 = 1.25;

/// The state's finances, both sides.
#[derive(Clone, Debug)]
pub struct Fiscal {
    pub tax: [usize; 2],
    /// The taxes besides the income tax: at the till, on profit, on property (levels 0 … 3).
    pub sales: [usize; 2],
    pub profit: [usize; 2],
    pub estate: [usize; 2],
    pub dole: [usize; 2],
    pub tariff: [usize; 2],
    pub caps: [bool; 2],
    pub labour: [usize; 2],
    pub rate: [usize; 2],
    pub print: [usize; 2],
    pub subsidy: [Vec<u8>; 2],
    pub public: [Vec<u8>; 2],
    /// Money printed, all told; what the treasury got from duties and its own sectors last month.
    pub printed: [f32; 2],
    /// Discontent of owners a nationalisation left (fades).
    pub seized: [f32; 2],
    /// The owners' confidence (1 the usual): how readily they build; it follows growth, prices,
    /// the government's standing, war, unrest, seizures and taxes. Money they took abroad when it
    /// was low (it comes back when it rises); months of cheap credit in a row; a bank panic waiting
    /// for the government's word; the share of taxes evaded last month.
    pub confidence: [f32; 2],
    pub fled: [f32; 2],
    pub cheap: [u32; 2],
    pub panic: [bool; 2],
    pub evaded: [f32; 2],
    /// Each side's embargo on the other's trade (in peace: a war cuts trade anyway), and whether
    /// the ways between them are cut by it now.
    pub embargo: [bool; 2],
    pub cut: bool,
}

impl Default for Fiscal {
    fn default() -> Fiscal {
        Fiscal {
            tax: [0; 2],
            sales: [0; 2],
            profit: [0; 2],
            estate: [0; 2],
            dole: [0; 2],
            tariff: [0; 2],
            caps: [false; 2],
            labour: [1; 2],
            rate: [1; 2],
            print: [0; 2],
            subsidy: [Vec::new(), Vec::new()],
            public: [Vec::new(), Vec::new()],
            printed: [0.0; 2],
            seized: [0.0; 2],
            confidence: [1.0; 2],
            fled: [0.0; 2],
            cheap: [0; 2],
            panic: [false; 2],
            evaded: [0.0; 2],
            embargo: [false; 2],
            cut: false,
        }
    }
}

/// The tax of a group of role `r` (0 the people of the land … 3 owners), times the custom, by scale.
pub fn tax_k(scale: usize, r: usize) -> f32 {
    const K: [[f32; 4]; 4] = [[1.0, 1.0, 1.0, 1.0], [0.7, 0.85, 1.15, 1.8], [0.5, 0.7, 1.4, 3.0], [1.25, 1.1, 0.8, 0.5]];
    K[scale.min(3)][r.min(3)]
}

/// Who gets what the state hands back: a weight a head for a group of role `r`.
fn dole_k(kind: usize, r: usize) -> f32 {
    const K: [[f32; 4]; 4] = [[1.0, 1.0, 1.0, 1.0], [2.2, 1.6, 0.5, 0.05], [1.0, 1.0, 1.0, 1.0], [0.3, 0.3, 0.6, 4.0]];
    K[kind.min(3)][r.min(3)]
}

/// The duty on good `i` by policy `k`.
pub fn duty_of(pack: &Pack, k: usize, i: usize) -> f32 {
    let g = &pack.goods[i];
    if !g.tradable {
        return 0.0;
    }
    let food = pack.ration(i);
    match k {
        1 => 0.1,
        2 => {
            if food {
                0.05
            } else if g.military {
                0.15
            } else {
                0.35
            }
        }
        3 => 0.6,
        _ => 0.0,
    }
}

/// The figures that matter of an economy, as a minister reads them.
#[derive(Clone, Debug, Default)]
pub struct Figures {
    /// Growth of output over a year, prices' rise over a year, share of the working people out of
    /// work.
    pub growth: f32,
    pub inflation: f32,
    pub idle: f32,
    /// The treasury a month: what came in (taxes, duties, its sectors, printed) and what went out.
    pub taxes: f32,
    pub duties: f32,
    pub public: f32,
    pub printed: f32,
    /// Exports and imports a month, at home prices.
    pub exports: f32,
    pub imports: f32,
    /// Owners' share of all incomes; what a working man of each group earns against his needs.
    pub owners: f32,
    pub wages: Vec<(String, f32)>,
    /// The owners' confidence, their money abroad, the share of taxes evaded.
    pub confidence: f32,
    pub fled: f32,
    pub evaded: f32,
}

impl Game {
    /// The economy's figures of side `s` now.
    pub fn figures(&self, s: usize) -> Figures {
        let pack = &self.pack;
        let n = self.h.home(s);
        let f = &self.spirits.facts[s];
        let mut idle = (0.0f32, 0.0f32);
        for g in 0..pack.groups.len() {
            if let Some(u) = n.unemployed[g] {
                idle.0 += u * n.pop[g];
                idle.1 += n.pop[g];
            }
        }
        let exports: f32 = (0..pack.goods.len()).map(|i| n.shipped[i] * n.price[i]).sum();
        let imports: f32 = (0..pack.goods.len()).map(|i| n.import[i] * n.price[i]).sum();
        let wages = (0..pack.groups.len())
            .filter(|&g| n.unemployed[g].is_some() && n.pop[g] > 1e-6)
            .map(|g| {
                let needs: f32 = pack.groups[g].needs.iter().map(|x| x.amount * n.price[x.good]).sum::<f32>().max(1e-9);
                (pack.groups[g].name.clone(), n.pay[g] / needs)
            })
            .collect();
        Figures {
            growth: f.growth,
            inflation: f.inflation,
            idle: idle.0 / idle.1.max(1e-9),
            taxes: n.taxed.iter().sum(),
            duties: n.duties,
            public: n.public_money,
            printed: if self.fiscal.print[s] > 0 { self.print_amount(s) } else { 0.0 },
            exports,
            imports,
            owners: f.owners,
            wages,
            confidence: self.fiscal.confidence[s],
            fled: self.fiscal.fled[s],
            evaded: self.fiscal.evaded[s],
        }
    }

    /// What the press prints a month for side `s` now.
    fn print_amount(&self, s: usize) -> f32 {
        const K: [f32; 4] = [0.0, 0.1, 0.33, 0.8];
        if self.fiscal.rate[s] == 3 {
            return 0.0;
        }
        K[self.fiscal.print[s].min(3)] * self.income(s)
    }

    /// Before the world's month: each side's finances laid on its economy (the core's levers).
    pub fn fiscal_apply(&mut self) {
        let pack = self.pack.clone();
        let (ng, ns, nk) = (pack.groups.len(), pack.sectors.len(), pack.goods.len());
        for s in 0..2 {
            let fx = self.fiscal.clone();
            let print = self.print_amount(s);
            let at_war = self.h.at_war;
            let n = self.h.home_mut(s);
            n.tax_by = if fx.tax[s] == 0 { Vec::new() } else { (0..ng).map(|g| tax_k(fx.tax[s], role(&pack, g))).collect() };
            // (the self-checks' control: the three newer taxes are not laid on the economy)
            let off = crate::country::econ_spoiled();
            n.sales_tax = if fx.sales[s] == 0 || off { Vec::new() } else { (0..nk).map(|i| sales_of(&pack, fx.sales[s], i)).collect() };
            n.profit_tax = if off { 0.0 } else { PROFIT_RATE[fx.profit[s].min(3)] };
            n.estate_tax = if fx.estate[s] == 0 || off { Vec::new() } else { (0..ng).map(|g| estate_of(fx.estate[s], role(&pack, g))).collect() };
            n.dole_by = match fx.dole[s] {
                0 => Vec::new(),
                2 => (0..ng).map(|g| 0.3 + 6.0 * n.drafted.get(g).copied().unwrap_or(0.0) / n.pop[g].max(1e-6) + if at_war { 0.0 } else { 0.7 }).collect(),
                k => (0..ng).map(|g| dole_k(k, role(&pack, g))).collect(),
            };
            n.duty = if fx.tariff[s] == 0 { Vec::new() } else { (0..nk).map(|i| duty_of(&pack, fx.tariff[s], i)).collect() };
            n.price_cap = if fx.caps[s] { (0..nk).map(|i| if pack.ration(i) { CAP } else { 0.0 }).collect() } else { Vec::new() };
            n.wage_k = [0.85, 1.0, 1.15, 1.3][fx.labour[s].min(3)];
            n.invest_k = [1.6, 1.0, 0.7, 0.85][fx.rate[s].min(3)];
            n.subsidy = if fx.subsidy[s].iter().all(|&x| x == 0) { Vec::new() } else { (0..ns).map(|k| SUBSIDY[fx.subsidy[s].get(k).copied().unwrap_or(0).min(3) as usize]).collect() };
            n.public = if fx.public[s].iter().all(|&x| x == 0) { Vec::new() } else { (0..ns).map(|k| PUBLIC[fx.public[s].get(k).copied().unwrap_or(0).min(3) as usize]).collect() };
            // the press: new money for the treasury
            if print > 0.0 {
                n.treasury += print;
            }
            // (credit: the banks lend and are repaid — `credit`; the rate here is only what owners
            // think a new job must earn)
            self.fiscal.printed[s] += print;
            self.fiscal.seized[s] *= 0.97;
            self.business(s);
        }
        // an embargo: the ways between the two closed while either keeps it (in peace)
        let want = (self.fiscal.embargo[0] || self.fiscal.embargo[1]) && !self.h.at_war;
        if want != self.fiscal.cut && !self.h.at_war {
            let (a, b) = (self.h.at[0], self.h.at[1]);
            if want {
                self.h.world.cut(a, b);
            } else {
                self.h.world.reopen(&pack, a, b);
            }
            self.fiscal.cut = want;
        }
    }

    /// Side `s` declares (or lifts) an embargo on the other's trade.
    pub fn set_embargo(&mut self, s: usize, on: bool) {
        if crate::spoiled() || self.fiscal.embargo[s] == on {
            return;
        }
        self.fiscal.embargo[s] = on;
        self.state.tension = (self.state.tension + if on { 0.15 } else { -0.05 }).clamp(0.0, 1.0);
        let now = self.month();
        let who = self.h.home(s).name.clone();
        let other = self.h.home(1 - s).name.clone();
        let line = if on { format!("{who} закрывает торговлю с {other}") } else { format!("{who} снимает запрет на торговлю с {other}") };
        self.news.push(Note::new(now, &line, true, if s == self.me { "econ" } else { "politics" }));
        self.stamp += 1;
    }

    /// The owners' month (side `s`): their confidence follows what they see; they build by it, take
    /// money abroad when it is low and bring it home when it is high; the more the tax and the
    /// weaker the government, the more of the tax they evade; cheap credit held too long with prices
    /// running away ends in a bank panic.
    fn business(&mut self, s: usize) {
        let pack = self.pack.clone();
        let f = self.spirits.facts[s].clone();
        let st = self.state.stability[s];
        let fx = self.fiscal.clone();
        let at_war = self.h.at_war;
        let mut target = 1.0 + 1.5 * f.growth.clamp(-0.2, 0.2) - 1.2 * (f.inflation - 0.05).max(0.0) - 0.8 * (0.6 - st).max(0.0) - 2.0 * fx.seized[s];
        if at_war {
            target -= 0.15;
        }
        if f.unrest {
            target -= 0.2;
        }
        // what they see in their own books: the tax they actually pay, what the factories earned over
        // the year, whether the courts protect them (the labour law and the tax reach them so)
        let burden = self.burdens(s)[strategy::world::polity::CAPITAL];
        target -= 1.2 * (burden - 0.15).max(-0.1);
        if let Some(x) = self.lot_change(s, strategy::world::polity::CAPITAL) {
            target += (0.6 * x).clamp(-0.2, 0.15);
        }
        target += (0.05 * (self.civil.order[s] - 1.0)).clamp(-0.05, 0.05);
        if fx.labour[s] == 3 {
            target -= 0.15;
        }
        if fx.rate[s] == 3 {
            target += 0.05;
        }
        let conf = (fx.confidence[s] + 0.1 * (target - fx.confidence[s])).clamp(0.3, 1.6);
        self.fiscal.confidence[s] = conf;
        let owners: Vec<usize> = (0..pack.groups.len()).filter(|&g| role(&pack, g) == 3).collect();
        // taxes evaded: the higher the tax and the weaker the government, the more — and the less
        // the state reaches (its officials, `civil`)
        // (all the taxes weigh: the others' share of the output, last month's, adds to the income tax's rate)
        let extra = {
            let n = self.h.home(s);
            n.taxed_by.iter().map(|t| t[1] + t[2] + t[3]).sum::<f32>() / n.gdp.max(1e-6)
        };
        let tax = self.h.home(s).tax + extra;
        let reach = (1.6 - self.civil.reach[s]).clamp(0.6, 1.45);
        let evasion = (((tax - 0.2) * 0.8 + (0.55 - st).max(0.0) * 0.4 + 0.04) * reach - 0.04).clamp(0.0, 0.4);
        self.fiscal.evaded[s] = evasion;
        // (months of cheap credit in a row, for the record; a bank crisis is the banks' — `credit`)
        self.fiscal.cheap[s] = if fx.rate[s] == 0 { fx.cheap[s] + 1 } else { 0 };
        let n = self.h.home_mut(s);
        n.invest_k *= conf;
        if evasion > 0.005 {
            let ng = pack.groups.len();
            if n.tax_by.len() != ng {
                n.tax_by = vec![1.0; ng];
            }
            for g in 0..ng {
                let k = match role(&pack, g) {
                    3 => 1.5,
                    2 => 1.0,
                    _ => 0.6,
                };
                n.tax_by[g] *= (1.0 - evasion * k).max(0.3);
            }
        }
        // (what is hidden is hidden from the newer taxes too: owners from the one on profit, the rich from the one
        // on property, everybody a little from the one at the till)
        if evasion > 0.005 {
            n.profit_tax *= (1.0 - 1.5 * evasion).max(0.3);
            for (g, t) in n.estate_tax.iter_mut().enumerate() {
                *t *= (1.0 - evasion * match role(&pack, g) { 3 => 1.5, 2 => 1.0, _ => 0.6 }).max(0.3);
            }
            for t in n.sales_tax.iter_mut() {
                *t *= 1.0 - 0.4 * evasion;
            }
        }
        // capital flight and its return
        let mut moved = 0.0f32;
        for &g in &owners {
            if let Some(m) = n.money.get_mut(g) {
                if conf < 0.8 {
                    let x = *m * 0.02 * (0.8 - conf) / 0.5;
                    *m -= x;
                    moved += x;
                }
            }
        }
        let mut back = 0.0f32;
        if conf > 1.0 && fx.fled[s] > 0.0 {
            back = fx.fled[s] * 0.05 * (conf - 1.0).min(0.5) / 0.5;
            if let Some(&g) = owners.first() {
                n.money[g] += back;
            }
        }
        self.fiscal.fled[s] += moved - back;
    }

    /// What side `s`'s finances do to its people's mood and its government's standing (a month).
    pub fn fiscal_eff(&self, s: usize) -> Eff {
        let fx = &self.fiscal;
        // (who pays the tax, who gets the dole, the labour law and the caps reach the people through
        // their purses — the market counts them; what is left here is the government's standing and
        // the owners' memory of a seizure)
        let mut a = [0.0f32; 4];
        let mut st = 0.0f32;
        if fx.tax[s] == 2 {
            st -= 0.0008;
        }
        // (a tax on bread is remembered by everyone who buys it)
        if fx.sales[s] == 3 {
            st -= 0.0010;
        }
        if fx.estate[s] == 3 {
            st -= 0.0004;
        }
        if fx.dole[s] == 2 && self.h.at_war {
            st += 0.0015;
        }
        if fx.labour[s] == 3 {
            st -= 0.0008;
        }
        if fx.rate[s] == 3 {
            st += 0.0006;
        }
        if fx.print[s] >= 2 {
            st -= 0.0005 * fx.print[s] as f32;
        }
        a[3] += self.fiscal.seized[s];
        Eff { anger: a, stability: st, ..Eff::default() }
    }

    /// Side `s` changes a lever of its finances: `what` — "tax", "dole", "tariff", "caps", "labour",
    /// "rate", "print"; `k` the choice.
    pub fn set_fiscal(&mut self, s: usize, what: &str, k: usize) {
        if crate::spoiled() {
            return;
        }
        let k3 = k.min(3);
        let fx = &mut self.fiscal;
        let (old, name) = match what {
            "tax" => (std::mem::replace(&mut fx.tax[s], k3), TAXES[k3].name),
            "sales" => (std::mem::replace(&mut fx.sales[s], k3), SALES[k3].name),
            "profit" => (std::mem::replace(&mut fx.profit[s], k3), PROFIT[k3].name),
            "estate" => (std::mem::replace(&mut fx.estate[s], k3), ESTATE[k3].name),
            "dole" => (std::mem::replace(&mut fx.dole[s], k3), DOLES[k3].name),
            "tariff" => (std::mem::replace(&mut fx.tariff[s], k3), TARIFFS[k3].name),
            "caps" => (std::mem::replace(&mut fx.caps[s], k > 0) as usize, if k > 0 { "Твёрдые цены на хлеб" } else { "Цены свободны" }),
            "labour" => (std::mem::replace(&mut fx.labour[s], k3), LABOUR[k3].name),
            "rate" => (std::mem::replace(&mut fx.rate[s], k3), RATES[k3].name),
            "print" => (std::mem::replace(&mut fx.print[s], k3), PRINTS[k3].name),
            _ => return,
        };
        let new = match what {
            "caps" => (k > 0) as usize,
            _ => k3,
        };
        if old == new {
            return;
        }
        self.stamp += 1;
        let now = self.month();
        if s == self.me {
            let what_tax = match what {
                "sales" => "налог с покупок",
                "profit" => "налог на прибыль",
                "estate" => "налог на имущество",
                _ => "финансы",
            };
            self.news.push(Note::new(now, &format!("{what_tax}: {}", name.to_lowercase()), false, "econ"));
        } else {
            let who = self.h.home(s).name.clone();
            if matches!(what, "tariff" | "print" | "rate") {
                self.news.push(Note::new(now, &format!("{who}: {}", name.to_lowercase()), false, "politics"));
            }
        }
    }

    /// Side `s` subsidises sector `k` at level `lv` (0 none … 3), or owns it at level `lv` (`public`).
    pub fn set_sector(&mut self, s: usize, k: usize, lv: u8, public: bool) {
        let ns = self.pack.sectors.len();
        if k >= ns || crate::spoiled() {
            return;
        }
        let lv = lv.min(3);
        let v = if public { &mut self.fiscal.public[s] } else { &mut self.fiscal.subsidy[s] };
        if v.len() < ns {
            v.resize(ns, 0);
        }
        let old = v[k];
        v[k] = lv;
        if public && lv > old {
            // a nationalisation: the owners never forget it soon; the government is shaken
            let d = (PUBLIC[lv as usize] - PUBLIC[old as usize]) * self.h.home(s).worked.get(k).copied().unwrap_or(0.0) / self.h.home(s).worked.iter().sum::<f32>().max(1e-6);
            self.fiscal.seized[s] += 0.02 + 0.3 * d;
            self.state.mods[s] -= 0.01 + 0.1 * d;
            if s == self.me {
                let now = self.month();
                self.news.push(Note::new(now, &format!("в казну отходит отрасль: {}", self.pack.sectors[k].name), true, "econ"));
            }
        }
        self.stamp += 1;
    }

    /// The bot's minister of finance (every season): by its course and what the country lives
    /// through — dear money against inflation and cheap against a slump, the press in a war with an
    /// empty treasury, the poor's taxes and dole against unrest, duties to protect what its course
    /// builds, price caps in hunger, subsidies to the arsenals in danger.
    pub fn bot_fiscal(&mut self, s: usize) {
        let now = self.month();
        if now % 3 != (s as u32 + 1) % 3 {
            return;
        }
        let f = self.spirits.facts[s].clone();
        let pack = self.pack.clone();
        let course = self.courses.now[s].as_ref().map(|c| c.kind);
        use crate::country::course::Kind;
        // the rate: by prices and growth (a rule any central bank keeps)
        let rate = if f.inflation > 0.12 {
            2
        } else if f.inflation < 0.01 && f.growth < 0.0 {
            0
        } else if course == Some(Kind::Thrift) {
            3
        } else {
            1
        };
        self.set_fiscal(s, "rate", rate);
        // the press: only a government in a war with an empty treasury, or one whose course cares
        // little for prices
        let print = if self.h.at_war && f.treasury < 0.5 { if f.held < 0.8 { 2 } else { 1 } } else { 0 };
        self.set_fiscal(s, "print", if self.fiscal.rate[s] == 3 { 0 } else { print });
        // taxes and the dole: against the anger of the poor, or for the owners
        let poor_angry = f.anger > 0.9 || f.hunger > 0.08;
        // (owners losing heart: a minister who wants them to build does not frighten them more)
        let timid = self.fiscal.confidence[s] < 0.7;
        let tax = match course {
            Some(Kind::Welfare) if !timid => 1,
            Some(Kind::Industry) | Some(Kind::Trade) if !poor_angry => 0,
            _ if poor_angry && !timid => 1,
            _ if timid => 0,
            _ => self.fiscal.tax[s],
        };
        self.set_fiscal(s, "tax", tax);
        // the other taxes: a government short of money in a war reaches for the till; one that cares
        // for the poor leans on the rich; one building industry spares its owners
        let broke = f.treasury < 0.5 && self.h.at_war;
        let sales = if broke && !poor_angry { 1 } else { 0 };
        self.set_fiscal(s, "sales", sales);
        let profit = match course {
            Some(Kind::Welfare) if !timid => 1,
            _ if broke && !timid => 1,
            _ => 0,
        };
        self.set_fiscal(s, "profit", profit);
        let estate = match course {
            Some(Kind::Welfare) if !timid => 1,
            _ => 0,
        };
        self.set_fiscal(s, "estate", estate);
        let dole = if self.h.at_war {
            2
        } else if poor_angry || course == Some(Kind::Welfare) {
            1
        } else if course == Some(Kind::Industry) && f.anger < 0.5 {
            3
        } else {
            0
        };
        self.set_fiscal(s, "dole", dole);
        // duties: protection for a country building its industry, free trade for one that lives by
        // it, autarky for one at war or cut off
        let tariff = match course {
            Some(Kind::Industry) => 2,
            Some(Kind::Trade) => 0,
            Some(Kind::Army) | Some(Kind::Revanche) | Some(Kind::Fortress) => 1,
            _ if f.imports > 0.25 => 1,
            _ => 0,
        };
        self.set_fiscal(s, "tariff", if self.h.at_war && tariff < 1 { 1 } else { tariff });
        // an embargo on a rival it means to fight or fears; lifted when the danger passes
        let danger = self.state.tension > 0.6 || self.state.claim[s].is_some();
        let hostile = matches!(course, Some(Kind::Revanche) | Some(Kind::Army)) && danger;
        if !self.h.at_war {
            self.set_embargo(s, hostile && f.imports < 0.3);
        }
        // price caps in hunger (the queues are the price of it)
        self.set_fiscal(s, "caps", (f.hunger > 0.1 || (self.h.at_war && f.inflation > 0.2)) as usize);
        // labour: as the course and the workers' mood say
        let labour = match course {
            Some(Kind::Welfare) => 2,
            Some(Kind::Industry) if f.anger < 0.6 => 0,
            _ => 1,
        };
        self.set_fiscal(s, "labour", labour);
        // subsidies: to the arsenals in danger and war; to the trades the course builds
        let ns = pack.sectors.len();
        for k in 0..ns {
            let (land, military, industry, _civil) = crate::country::policy::kind_of(&pack, k);
            let lv = match course {
                Some(Kind::Army) | Some(Kind::Revanche) | Some(Kind::Fortress) if military => 2,
                Some(Kind::Industry) if industry => 1,
                Some(Kind::Agrarian) if land => 1,
                _ if military && self.h.at_war => 2,
                _ => 0,
            };
            self.set_sector(s, k, lv, false);
        }
        // state ownership: only a government that believes in the state, building its industry
        if course == Some(Kind::Industry) && !timid {
            let c = self.country(s);
            let state = self.folk.offices[c].ruler.map_or(0.0, |r| self.folk.get(r).state);
            if state > 0.5 {
                for k in 0..ns {
                    let (_, military, industry, _) = crate::country::policy::kind_of(&pack, k);
                    if military || industry {
                        let have = self.fiscal.public[s].get(k).copied().unwrap_or(0);
                        if have < 2 {
                            self.set_sector(s, k, 2, true);
                        }
                    }
                }
            }
        }
    }
}
