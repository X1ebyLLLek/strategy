//! The army's money as a ministry of war spends it, and the army as a government keeps it.
//!
//! A peace army is an establishment the government funds: so many divisions to a province of the
//! border (`PEACE_PER_FRONT` is the custom; the player chooses, a bot's course decides), and a war
//! reserve of equipment in the depots. Its money goes where an army's money goes in life: the pay of
//! the men called up (by the law of the draft) and of the career officers and sergeants (paid as
//! their trade pays, whatever the law); the mending of equipment (spare parts and the army's shops: a
//! share of the kit's worth a month, three times as much at war, when the field wears it out); the
//! ammunition and fuel troops burn at exercises and in battle; new equipment from the factories — in
//! peace only as much as the establishment and the depots need (what the government gave beyond it
//! stays in the treasury). Equipment not mended wears out; mended, it still ages (`WEAR`).
//!
//! When the men of a peace army have served their time they go home as reservists, and the kit of
//! what the establishment does not keep goes to the depots: a mobilisation (at war, or ordered in
//! peace) calls the reservists back to the depots' kit, and the army doubles in weeks with trained
//! men — then grows only as fast as the factories make new kit and recruits are trained. What a war
//! costs is counted: money beyond the peace's, the dead, the kit lost, the debt.

use strategy::military::corps::BATTALION;
use strategy::military::homefront::men_of;

use crate::game::Note;
use crate::Game;

/// The career men of an army: the share of those serving who are officers and sergeants by trade,
/// and their pay against what they would earn at work.
pub const CADRE_SHARE: f32 = 0.08;
pub const CADRE_PAY: f32 = 2.2;
/// Equipment that is mended still ages: this much of it goes out of service a month (about twenty
/// years of life).
pub const WEAR: f32 = 0.004;
/// Goods a month to mend equipment, of what went into it: in peace, and at war (the field wears it
/// out); equipment not mended wears out this much more a month; equipment in the depots ages this
/// much a month and costs a third of the mending.
pub const MEND: f32 = 0.006;
pub const MEND_WAR: f32 = 0.02;
pub const UNMENDED: f32 = 0.03;
pub const STORE_WEAR: f32 = 0.002;
/// The custom establishment of peace: divisions to a province of the border with the rival.
pub const PEACE_PER_FRONT: f32 = 1.5;
/// The establishments a government may choose: divisions to a province of the border, with names.
/// (the names on the tiles are short: about six letters to a line)
pub const ESTABLISHMENT_TILES: [&str; 5] = ["Кадровая", "Обычный", "Большой", "На взводе", "Военный"];
pub const ESTABLISHMENTS: [(f32, &str, &str); 5] = [
    (0.75, "Кадровая армия", "Малая армия мирного времени: дёшево, но к войне её придётся собирать из запаса."),
    (1.5, "Обычный штат", "Как водится: по полторы дивизии на провинцию границы."),
    (2.25, "Усиленный штат", "Армия больше обычного: дороже жалованье и обслуживание, к войне готова лучше."),
    (3.0, "Армия на взводе", "Почти военный штат в мирное время: казна платит за армию, которая пока не воюет."),
    (4.0, "Военный штат", "Столько дивизий, сколько держит фронт: так живут только перед самой войной."),
];
/// A war reserve in the depots: kit for this many times the peace army's.
pub const STOCK: f32 = 1.0;
/// Months in which a peace army buys the kit its establishment and depots still lack.
const FILL: f32 = 8.0;
/// Of what the depots hold, the share a mobilisation brings back to the army in a month (the
/// reservists come in weeks).
const CALL_UP: f32 = 0.6;
/// Units in the pool a peace army keeps for refilling its divisions, of what they hold; the rest goes
/// to the depots and its men home.
const POOL_KEEP: f32 = 0.04;
/// At war: the losses are many, the pool keeps more.
const POOL_KEEP_WAR: f32 = 0.15;
/// Reservists grow too old for service: this share a month.
const AGE_OUT: f32 = 0.008;
/// Months of books kept for the charts.
const BOOKS: usize = 240;

/// A month of the army's money: what the government gave it, the pay of the called-up and of the
/// career men, the mending, the supply burnt, the new kit, the navy, what it left unspent; and the
/// army it kept — divisions, units in the depots, reservists, men serving (millions).
#[derive(Clone, Copy, Default, Debug)]
pub struct MilBook {
    pub month: u32,
    pub allotted: f32,
    pub pay_men: f32,
    pub pay_cadre: f32,
    pub mend: f32,
    pub supply: f32,
    pub equip: f32,
    pub navy: f32,
    pub unspent: f32,
    pub divisions: u32,
    pub stored: f32,
    pub reservists: f32,
    pub serving: f32,
    /// Share of the mending the army got (1: all it asked).
    pub mended: f32,
}

impl MilBook {
    pub fn spent(&self) -> f32 {
        self.pay_men + self.pay_cadre + self.mend + self.supply + self.equip + self.navy
    }
}

/// What a war has cost a side so far: money on the army and the navy beyond what the peace spent,
/// men killed (millions), kit lost and left behind (battalions' worth), debt taken, months.
#[derive(Clone, Copy, Default, Debug)]
pub struct WarCost {
    pub money: f32,
    pub dead: f32,
    pub kit_lost: f32,
    pub debt: f32,
    pub months: u32,
    /// The debt when the war began (to count what it took).
    pub debt0: f32,
}

/// The game's side of the army's money: the depots, the reservists, the establishments, the
/// mobilisations, the books.
#[derive(Clone, Default)]
pub struct Mil {
    /// Equipment kept in the depots without men (units of each kind), and its kits.
    pub stored: [Vec<f32>; 2],
    pub stored_kit: [Vec<f32>; 2],
    /// Men who served and went home (millions): a mobilisation calls them back trained.
    pub reservists: [f32; 2],
    /// The peace establishment a government chose (`ESTABLISHMENTS` index; none: the custom, or the
    /// bot's course).
    pub chosen: [Option<usize>; 2],
    /// The month a mobilisation was ordered in peace (none: none ordered).
    pub mobilised: [Option<u32>; 2],
    /// Goods the mending asked for this month, and the money for them by good.
    pub mend_need: [Vec<f32>; 2],
    pub mend_ask: [Vec<f32>; 2],
    /// The books: this month's, and the months before (newest last).
    pub book: [MilBook; 2],
    pub books: [Vec<MilBook>; 2],
    /// What the army spent a month in peace (smoothed): the war's cost is counted against it.
    pub peace_spend: [f32; 2],
    pub war_cost: [WarCost; 2],
    /// The money the government gave the army this month, as it began.
    pub allotted: [f32; 2],
    /// Whether the war was on last month (a war begun is seen by this).
    pub at_war: bool,
    /// The share of each side's civil industry converted to arms (`army::industry`), the workplaces
    /// taken from each sector, the workplaces of arms works they became.
    pub convert: [f32; 2],
    pub converted_from: [Vec<f32>; 2],
    pub converted_to: [f32; 2],
}

impl Game {
    /// The army as the game keeps it from the first day: career men paid as their trade, mended
    /// equipment lasting for years, armies no bigger than a general leads well, each side's peace
    /// establishment — what the years before the game built beyond it is in the depots, its men at
    /// home as reservists.
    pub fn mil_start(&mut self) {
        let nu = self.pack.units.len();
        self.h.retire = WEAR;
        self.h.war.fixed_armies = true;
        for s in 0..2 {
            let n = self.h.home_mut(s);
            n.cadre_share = CADRE_SHARE;
            n.cadre_pay = CADRE_PAY;
            self.mil.stored[s] = vec![0.0; nu];
            self.mil.stored_kit[s] = vec![0.0; nu];
            // (the men of the years before: some served and went home)
            let serving: f32 = self.h.home(s).drafted.iter().sum();
            self.mil.reservists[s] = serving * 1.5;
            self.h.war.establish[s] = self.establishment(s);
            self.disband_over(s);
        }
        for s in 0..2 {
            self.mil.peace_spend[s] = self.h.home(s).soldiers_paid + self.h.home(s).army_spent;
        }
    }

    /// The peace establishment side `s` keeps: divisions to a province of its border. A government's
    /// choice; else a bot's: its course's share of spending on the army against the custom one (a
    /// course of arms keeps more); the player who has not chosen keeps the custom one.
    pub fn establishment(&self, s: usize) -> f32 {
        // (so many to a usual province's width of the front: the game's provinces are narrower)
        let k = crate::prov_scale();
        if let Some(c) = self.mil.chosen[s] {
            return ESTABLISHMENTS[c.min(ESTABLISHMENTS.len() - 1)].0 * k;
        }
        let bot = s != self.me || self.auto.is_some();
        if !bot {
            return PEACE_PER_FRONT * k;
        }
        let scale = k;
        let n = self.h.home(s);
        let k = (n.mil_share / self.pack.military.max(0.05)).clamp(0.5, 2.0);
        // (and it answers the neighbour's army: an arms race, within bounds)
        let count = |x: usize| self.h.war.corps.as_ref().map_or(0, |c| c.divs.iter().filter(|d| d.side == x).count()) as f32;
        let fronts = |x: usize| self.h.war.front(x).len().max(1) as f32;
        let race = ((count(1 - s) / fronts(1 - s) + 0.5) / (count(s) / fronts(s) + 0.5)).sqrt().clamp(0.85, 1.35);
        PEACE_PER_FRONT * k * race * scale
    }

    /// Whether side `s` is mobilised: at war, or a mobilisation ordered in peace.
    pub fn mobilised(&self, s: usize) -> bool {
        self.h.at_war || self.mil.mobilised[s].is_some()
    }

    /// The divisions the establishment of side `s` keeps now (for the front it has).
    pub fn est_divisions(&self, s: usize) -> f32 {
        let per = if self.mobilised(s) { crate::game::MOST_PER_FRONT * crate::prov_scale() } else { self.establishment(s) };
        per * self.h.war.front(s).len().max(1) as f32
    }

    /// Units a division of side `s`'s pattern holds now.
    fn div_units(&self, s: usize) -> f32 {
        let Some(c) = self.h.war.corps.as_ref() else { return 3.0 };
        let n = c.divs.iter().filter(|d| d.side == s).count();
        if n > 0 {
            c.divs.iter().filter(|d| d.side == s).map(|d| d.has.iter().sum::<f32>()).sum::<f32>() / n as f32
        } else {
            c.shapes[s].last().map_or(3.0, |sh| sh.battalions() * BATTALION)
        }
    }

    /// Divisions of side `s` beyond its establishment are disbanded into the depots: their kit stored,
    /// their men home as reservists (the newest first). (At the game's start: the years before built
    /// the army by the old custom.)
    fn disband_over(&mut self, s: usize) {
        let most = self.est_divisions(s).ceil() as usize;
        let pack = self.pack.clone();
        let Some(c) = self.h.war.corps.as_mut() else { return };
        let mut ids: Vec<(u32, u32)> = c.divs.iter().filter(|d| d.side == s).map(|d| (d.no, d.id)).collect();
        if ids.len() <= most {
            return;
        }
        ids.sort();
        let gone: Vec<u32> = ids[most..].iter().map(|x| x.1).collect();
        let nu = pack.units.len();
        let mut units = vec![0.0f32; nu];
        for d in c.divs.iter().filter(|d| gone.contains(&d.id)) {
            for u in 0..nu {
                units[u] += d.has[u];
                self.mil.stored[s][u] += d.has[u];
                self.mil.stored_kit[s][u] += d.kit[u];
            }
        }
        c.divs.retain(|d| !gone.contains(&d.id));
        let men = men_of(&pack, &units);
        for u in 0..nu {
            self.h.war.sides[s].army.count[u] = (self.h.war.sides[s].army.count[u] - units[u]).max(0.0);
        }
        self.h.home_mut(s).release(men);
        self.mil.reservists[s] += men;
    }

    /// Goods a month the mending of side `s`'s equipment asks for: of its divisions', its pool's and
    /// (a third) its depots' kits.
    fn mend_goods(&self, s: usize) -> Vec<f32> {
        let pack = &self.pack;
        let nk = pack.goods.len();
        let scale = pack.war.as_ref().map_or(0.02, |w| w.scale);
        let mut kit = vec![0.0f32; pack.units.len()];
        if let Some(c) = self.h.war.corps.as_ref() {
            for d in c.divs.iter().filter(|d| d.side == s) {
                for (u, k) in kit.iter_mut().enumerate() {
                    *k += d.kit[u];
                }
            }
            for (u, k) in kit.iter_mut().enumerate() {
                *k += c.kit[s][u];
            }
        }
        let rate = if self.h.at_war { MEND_WAR } else { MEND };
        let mut need = vec![0.0f32; nk];
        for (u, unit) in pack.units.iter().enumerate() {
            let k = kit[u] + self.mil.stored_kit[s].get(u).copied().unwrap_or(0.0) / 3.0;
            for &(i, a) in &unit.equip {
                need[i] += k * a * scale * rate;
            }
        }
        need
    }

    /// The most a peace army of side `s` spends on new kit this month: what its establishment and
    /// its depots still lack (over `FILL` months) and what ageing takes, at its prices.
    fn procurement_cap(&self, s: usize) -> f32 {
        let pack = &self.pack;
        let Some(c) = self.h.war.corps.as_ref() else { return f32::INFINITY };
        let nu = pack.units.len();
        let scale = pack.war.as_ref().map_or(0.02, |w| w.scale);
        let n = self.h.home(s);
        let mut army = c.pool[s].clone();
        for d in c.divs.iter().filter(|d| d.side == s) {
            for u in 0..nu {
                army[u] += d.has[u];
            }
        }
        let have: f32 = army.iter().sum();
        let stored: f32 = self.mil.stored[s].iter().sum();
        let target = self.est_divisions(s) * self.div_units(s);
        let gap = (target - have).max(0.0) + (STOCK * target - stored).max(0.0);
        let units = gap / FILL + WEAR * have + STORE_WEAR * stored;
        // a unit's kit at the country's prices, by the army's make-up (or the staff's plan)
        let side = &self.h.war.sides[s];
        let weights: Vec<f32> = if have > 0.0 { army.clone() } else { (0..nu).map(|u| side.make_share(u)).collect() };
        let all: f32 = weights.iter().sum::<f32>().max(1e-9);
        let per: f32 = (0..nu).map(|u| weights[u] / all * side.norm_of(u) * pack.units[u].equip.iter().map(|&(i, a)| a * scale * n.price[i]).sum::<f32>()).sum();
        units * per
    }

    /// The depots' kit of side `s` back to its army with its reservists (or recruits, when they run
    /// short): what a month of a mobilisation brings.
    fn call_up(&mut self, s: usize) {
        let pack = self.pack.clone();
        let nu = pack.units.len();
        let stored: f32 = self.mil.stored[s].iter().sum();
        if stored < 1e-4 {
            if let Some(c) = self.h.war.corps.as_mut() {
                c.fresh_drill[s] = 0.0;
            }
            return;
        }
        let (have, in_divs, pool) = self.h.war.corps.as_ref().map_or((0.0, 0.0, 0.0), |c| {
            let mine: Vec<&strategy::military::corps::Division> = c.divs.iter().filter(|d| d.side == s).collect();
            (mine.len() as f32, mine.iter().map(|d| d.has.iter().sum::<f32>()).sum::<f32>(), c.pool[s].iter().sum::<f32>())
        });
        // what the army lacks: divisions its establishment wants, and the pool that refills them; in
        // the first months of a mobilisation the depots come quickly
        let keep = if self.h.at_war { POOL_KEEP_WAR } else { POOL_KEEP };
        let lack = (self.est_divisions(s) - have).max(0.0) * self.div_units(s) + (keep * in_divs - pool).max(0.0);
        let take = lack.min(stored * CALL_UP.max(0.25)).min(stored);
        if take <= 1e-4 {
            if let Some(c) = self.h.war.corps.as_mut() {
                c.fresh_drill[s] = 0.0;
            }
            return;
        }
        let mut f = take / stored;
        let units: Vec<f32> = (0..nu).map(|u| self.mil.stored[s][u] * f).collect();
        let men = men_of(&pack, &units);
        let room = self.h.home(s).draftable();
        if men > room && men > 0.0 {
            f *= room / men;
        }
        let units: Vec<f32> = (0..nu).map(|u| self.mil.stored[s][u] * f).collect();
        let kits: Vec<f32> = (0..nu).map(|u| self.mil.stored_kit[s][u] * f).collect();
        let men = men_of(&pack, &units);
        if men <= 0.0 {
            return;
        }
        let came = self.h.home_mut(s).draft(men);
        let trained = (self.mil.reservists[s] / came.max(1e-9)).min(1.0);
        self.mil.reservists[s] = (self.mil.reservists[s] - came).max(0.0);
        for u in 0..nu {
            self.mil.stored[s][u] -= units[u];
            self.mil.stored_kit[s][u] -= kits[u];
            self.h.war.sides[s].army.count[u] += units[u];
        }
        if let Some(c) = self.h.war.corps.as_mut() {
            for u in 0..nu {
                c.pool[s][u] += units[u];
                c.kit[s][u] += kits[u];
            }
            c.fresh_drill[s] = trained;
        }
    }

    /// Before the world's month: each side's establishment, its mobilisation, the mending it asks for,
    /// the most it spends on new kit; the other orders paid out of the army's money (the mending and
    /// the navy's, `navy_before`) go to the market with the army's.
    pub fn mil_before(&mut self) {
        let nk = self.pack.goods.len();
        // (a war just begun: its cost is counted from now)
        if self.h.at_war && !self.mil.at_war {
            self.mil_war_begins();
            self.navy_war_begins();
        }
        self.mil.at_war = self.h.at_war;
        // the civil works turned to arms (or back)
        self.convert_month();
        for s in 0..2 {
            if self.mil.stored[s].is_empty() {
                continue;
            }
            self.h.war.establish[s] = if self.mobilised(s) { crate::game::MOST_PER_FRONT * crate::prov_scale() } else { self.establishment(s) };
            if self.mobilised(s) {
                self.call_up(s);
            } else if let Some(c) = self.h.war.corps.as_mut() {
                c.fresh_drill[s] = 0.0;
            }
            let need = self.mend_goods(s);
            let n = self.h.home(s);
            self.mil.mend_ask[s] = (0..nk).map(|i| need[i] * n.price[i]).collect();
            self.mil.mend_need[s] = need;
            self.mil.allotted[s] = n.state_budget() * n.mil_share;
            self.h.equip_cap[s] = if self.mobilised(s) {
                if self.h.at_war { f32::INFINITY } else { self.procurement_cap(s) * 4.0 }
            } else {
                self.procurement_cap(s)
            };
        }
        self.navy_before();
        for s in 0..2 {
            let navy = &self.navy.ask[s];
            let mend = &self.mil.mend_ask[s];
            self.h.side[s] = (0..nk).map(|i| mend.get(i).copied().unwrap_or(0.0) + navy.get(i).copied().unwrap_or(0.0)).collect();
            if self.h.side[s].iter().all(|&x| x <= 0.0) {
                self.h.side[s].clear();
            }
        }
    }

    /// After the world's month: what the other orders got split between the mending and the navy;
    /// equipment not mended wears out, the depots age; in peace what the establishment does not keep
    /// goes to the depots and its men home; the reservists age; the books and the war's cost.
    pub fn mil_after(&mut self) {
        let pack = self.pack.clone();
        let nk = pack.goods.len();
        let nu = pack.units.len();
        let now = self.month();
        for s in 0..2 {
            if self.mil.stored[s].is_empty() {
                continue;
            }
            // what the mending and the navy got, each by its money
            let got = self.h.side_got[s].clone();
            let mut mend_got = vec![0.0f32; nk];
            let mut navy_got = vec![0.0f32; nk];
            for i in 0..got.len().min(nk) {
                let m = self.mil.mend_ask[s].get(i).copied().unwrap_or(0.0);
                let v = self.navy.ask[s].get(i).copied().unwrap_or(0.0);
                if m + v > 0.0 {
                    mend_got[i] = got[i] * m / (m + v);
                    navy_got[i] = got[i] * v / (m + v);
                }
            }
            self.navy.got[s] = navy_got;
            let need = &self.mil.mend_need[s];
            let want: f32 = (0..nk).map(|i| need.get(i).copied().unwrap_or(0.0) * pack.goods[i].price).sum();
            let have: f32 = (0..nk).map(|i| mend_got[i].min(need.get(i).copied().unwrap_or(0.0)) * pack.goods[i].price).sum();
            let mended = if want > 0.0 { (have / want).clamp(0.0, 1.0) } else { 1.0 };
            let worn = UNMENDED * (1.0 - mended);
            if let Some(c) = self.h.war.corps.as_mut() {
                for d in c.divs.iter_mut().filter(|d| d.side == s) {
                    for k in d.kit.iter_mut() {
                        *k *= 1.0 - worn;
                    }
                }
                for k in c.kit[s].iter_mut() {
                    *k *= 1.0 - worn;
                }
            }
            for u in 0..nu {
                self.mil.stored[s][u] *= 1.0 - STORE_WEAR;
                self.mil.stored_kit[s][u] *= 1.0 - STORE_WEAR - worn / 3.0;
            }
            // the pool beyond what refills the divisions goes to the depots, its men home (at war
            // too: kit the front has no room for waits without men)
            {
                let keep_share = if self.mobilised(s) { POOL_KEEP_WAR } else { POOL_KEEP };
                let mut moved = vec![0.0f32; nu];
                if let Some(c) = self.h.war.corps.as_mut() {
                    let mut in_divs = vec![0.0f32; nu];
                    for d in c.divs.iter().filter(|d| d.side == s) {
                        for u in 0..nu {
                            in_divs[u] += d.has[u];
                        }
                    }
                    for u in 0..nu {
                        let keep = keep_share * in_divs[u];
                        let x = (c.pool[s][u] - keep).max(0.0);
                        if x <= 0.0 {
                            continue;
                        }
                        let k = c.kit[s][u] * x / c.pool[s][u].max(1e-9);
                        c.pool[s][u] -= x;
                        c.kit[s][u] -= k;
                        moved[u] = x;
                        self.mil.stored[s][u] += x;
                        self.mil.stored_kit[s][u] += k;
                    }
                }
                let men = men_of(&pack, &moved);
                for u in 0..nu {
                    self.h.war.sides[s].army.count[u] = (self.h.war.sides[s].army.count[u] - moved[u]).max(0.0);
                }
                if men > 0.0 {
                    self.h.home_mut(s).release(men);
                    self.mil.reservists[s] += men;
                }
                // (the establishment shrank: divisions over it are disbanded)
                if !self.mobilised(s) {
                    self.disband_over(s);
                }
            }
            self.mil.reservists[s] *= 1.0 - AGE_OUT;
            // the books
            let n = self.h.home(s);
            let pay = n.soldiers_paid;
            let (cs, cp, dp) = (n.cadre_share, n.cadre_pay, n.draft_pay);
            let cadre_part = if cs > 0.0 { cs * cp / ((1.0 - cs) * dp + cs * cp).max(1e-9) } else { 0.0 };
            let [sup, eq, side] = self.h.spend[s];
            let asked = (sup + eq + side).max(1e-9);
            let goods = n.army_spent;
            let mend_money: f32 = self.mil.mend_ask[s].iter().sum();
            let navy_money: f32 = self.navy.ask[s].iter().sum();
            let side_part = |x: f32| if side > 0.0 { goods * side / asked * x / (mend_money + navy_money).max(1e-9) } else { 0.0 };
            let crew = self.navy_crew_pay(s).min(pay * (1.0 - cadre_part));
            let divisions = self.h.war.corps.as_ref().map_or(0, |c| c.divs.iter().filter(|d| d.side == s).count() as u32);
            let book = MilBook {
                month: now,
                allotted: self.mil.allotted[s],
                pay_men: (pay * (1.0 - cadre_part) - crew).max(0.0),
                pay_cadre: pay * cadre_part,
                mend: side_part(mend_money),
                supply: goods * sup / asked,
                equip: goods * eq / asked,
                navy: side_part(navy_money) + crew,
                unspent: 0.0,
                divisions,
                stored: self.mil.stored[s].iter().sum(),
                reservists: self.mil.reservists[s],
                serving: n.drafted.iter().sum(),
                mended,
            };
            let mut book = book;
            book.unspent = (book.allotted - book.spent()).max(0.0);
            self.mil.book[s] = book;
            self.mil.books[s].push(book);
            if self.mil.books[s].len() > BOOKS {
                self.mil.books[s].remove(0);
            }
            if self.h.at_war {
                let w = &mut self.mil.war_cost[s];
                w.months += 1;
                w.money += (book.spent() - self.mil.peace_spend[s]).max(0.0);
                w.dead = self.h.dead[s];
                w.kit_lost = self.h.war.corps.as_ref().map_or(0.0, |c| (c.lost_kit[s].iter().sum::<f32>() + c.abandoned[s].iter().sum::<f32>()) / BATTALION);
                w.debt = (self.policy.debt[s] - w.debt0).max(0.0);
            } else {
                self.mil.peace_spend[s] += 0.1 * (book.spent() - self.mil.peace_spend[s]);
            }
        }
    }

    /// The war begins: its cost is counted from now.
    pub fn mil_war_begins(&mut self) {
        for s in 0..2 {
            self.mil.war_cost[s] = WarCost { debt0: self.policy.debt[s], ..WarCost::default() };
            self.mil.mobilised[s] = None;
        }
    }

    /// Side `s` orders a mobilisation in peace (or calls it off): the reservists and the depots' kit
    /// come to the army; the neighbour sees it.
    pub fn order_mobilisation(&mut self, s: usize, on: bool) {
        let now = self.month();
        if self.h.at_war || on == self.mil.mobilised[s].is_some() {
            return;
        }
        self.mil.mobilised[s] = if on { Some(now) } else { None };
        let name = self.h.home(s).name.clone();
        let line = if on { format!("{name} объявляет мобилизацию: резервисты идут в полки") } else { format!("{name} отменяет мобилизацию: резервисты расходятся по домам") };
        if on {
            self.state.tension = (self.state.tension + 0.15).min(1.0);
        }
        self.news.push(Note::new(now, &line, true, "army"));
    }

    /// The bots' mobilisations: a bot answers a neighbour's mobilisation with its own (and calls its
    /// own off when the neighbour's ends and its course is not one of arms).
    pub fn mobilisation_bots(&mut self) {
        if self.h.at_war {
            return;
        }
        for s in 0..2 {
            let bot = s != self.me || self.auto.is_some();
            if !bot {
                continue;
            }
            let other = self.mil.mobilised[1 - s].is_some();
            if other && self.mil.mobilised[s].is_none() {
                self.order_mobilisation(s, true);
            } else if !other && self.mil.mobilised[s].is_some_and(|m| self.month() >= m + 6) {
                self.order_mobilisation(s, false);
            }
        }
    }
}
