//! The home front: a country's economy raises, arms and feeds its army.
//!
//! The army is made of what the country makes or buys. Each month the state gives the army its
//! share of what it spends (more in war, and a war tax fills the treasury), and the army buys on
//! the country's own market: first the supply its troops use up, then the equipment of new
//! units, in the shares the staff wants. It pays the country's prices: where steel is scarce it
//! is dear, a tank costs more, and the staff, which weighs its plans by what units cost it,
//! builds fewer. What the market does not have the army cannot buy at all.
//!
//! Soldiers come from the people: who serves does not work. The dead are mourned, and grief
//! piles up in discontent with hunger and want; a war tax leaves people less to live on, and
//! what the army buys is not on sale for them.
//!
//! The troops take their supply from the army's stores; what is missing weakens the whole
//! front, as distance from the capital does.

use crate::economy::advisor::{Economist, Outside};
use crate::economy::nation::Nation;
use crate::economy::nation::Setup;
use crate::economy::science::{Lab, Scientist};
use crate::economy::trade::{self, World};
use crate::military::doctrine::Country;
use crate::military::front::Campaign;
use crate::military::front::Theatre;
use crate::pack::Pack;
use crate::rng::{hash64, Rng};
use crate::world::countries::Politics;
use crate::world::map::Terrain;
use crate::world::map;

/// Share of the state's spending that goes to the army in war, and how many times the pack's
/// tax the war tax is.
pub const WAR_MIL: f32 = 0.8;
pub const WAR_TAX: f32 = 2.0;
/// Months of a unit's supply and of its men's pay counted in what it costs (as in the pack's
/// worth of a unit: its equipment and six months of supply).
const SERVICE: f32 = 6.0;
/// Share of a month of fighting's supply troops use in a month of peace (exercises).
pub const PEACE_USE: f32 = 0.2;
/// Months of its troops' supply the army keeps in store besides this month's.
const STORE: f32 = 1.0;
/// Share of the army that leaves service each month of peace (they go back to work).
pub const RETIRE: f32 = 0.02;
/// Of the men of the units lost, the share who die; the rest (the wounded, prisoners) come home.
/// (Counting every man of a lost unit dead, a war of four years killed a sixth of the people.)
const KILLED: f32 = 0.3;
/// In peace the staff sees the neighbour's army once a year through fog this many times
/// thicker than in war (as in the doctrines' peace).
const PEACE_FOG: f32 = 2.0;
/// Passes of sharing the army's stores among the kinds the staff wants: goods left over when
/// one kind lacks a good go to the kinds that can still use them.
const PASSES: usize = 3;

/// What each kind of unit costs the country now, in the pack's worth: its goods at its own
/// prices against the usual ones (a good it lacks at what it would fetch: dearer and dearer
/// while the army asks for it in vain), its men at what they would earn at work for `SERVICE`
/// months.
pub fn costs(pack: &Pack, n: &Nation) -> Vec<f32> {
    let war = pack.war.as_ref().expect("в наборе нет раздела [war]");
    // a thousand men of a unit's figure, in the pack's worth (money is the pack's worth times the
    // scale of goods and the level of prices)
    let man = war.crew / 1000.0 * n.soldier_pay() * SERVICE / (war.scale * n.level).max(1e-9);
    let rel = |i: usize| n.price[i] / (pack.goods[i].price * n.level).max(1e-9);
    pack.units
        .iter()
        .enumerate()
        .map(|(k, u)| {
            let worth = |v: &[(usize, f32)]| v.iter().map(|&(g, a)| a * pack.goods[g].price * rel(g)).sum::<f32>();
            let make = n.make_k.get(k).copied().unwrap_or(1.0);
            let equip = if make == 1.0 { worth(&u.equip) } else { worth(&u.equip) * make };
            equip + SERVICE * worth(&u.supply) + u.men * man
        })
        .collect()
}

/// What each kind of unit costs the country now by its parts: its equipment (the standard kit),
/// `SERVICE` months of the supply its kit burns, its men and their rations (`Country::parts`).
pub fn parts(pack: &Pack, n: &Nation) -> Vec<[f32; 3]> {
    let war = pack.war.as_ref().expect("в наборе нет раздела [war]");
    let man = war.crew / 1000.0 * n.soldier_pay() * SERVICE / (war.scale * n.level).max(1e-9);
    let rel = |i: usize| n.price[i] / (pack.goods[i].price * n.level).max(1e-9);
    pack.units
        .iter()
        .enumerate()
        .map(|(k, u)| {
            let worth = |v: &[(usize, f32)], food: bool| v.iter().filter(|x| pack.ration(x.0) == food).map(|&(g, a)| a * pack.goods[g].price * rel(g)).sum::<f32>();
            let make = n.make_k.get(k).copied().unwrap_or(1.0);
            let equip = worth(&u.equip, false) + worth(&u.equip, true);
            let equip = if make == 1.0 { equip } else { equip * make };
            [equip, SERVICE * worth(&u.supply, false), u.men * man + SERVICE * worth(&u.supply, true)]
        })
        .collect()
}

/// What the troops of `army` use up in a month, `use_` of a month of fighting: goods (a heavier
/// kit, `norm`, burns more; empty: the standard).
fn needs(pack: &Pack, army: &[f32], norm: &[f32], use_: f32) -> Vec<f32> {
    let scale = pack.war.as_ref().unwrap().scale;
    let mut need = vec![0.0f32; pack.goods.len()];
    for (u, unit) in pack.units.iter().enumerate() {
        let eat = crate::military::doctrine::norm::eat(norm.get(u).copied().unwrap_or(1.0));
        for &(i, a) in &unit.supply {
            // (rations by the men, the rest by the kit)
            let k = if eat == 1.0 || pack.ration(i) { 1.0 } else { eat };
            need[i] += army[u] * k * a * scale * use_;
        }
    }
    need
}

/// What the army asks of the market this month, as shares of its money: its troops' supply
/// (this month's and `STORE` months more, less what is in store) first, then the equipment of
/// new units in the shares the staff wants (goods not on sale too: merchants see the buyers).
pub fn order(pack: &Pack, n: &Nation, c: &Country, use_: f32) -> Vec<f32> {
    order_parts(pack, n, c, use_, f32::INFINITY).0
}

/// As `order`, spending at most `cap` on the equipment of new units (a staff whose army is as big as
/// the government funds orders no more of it; the money stays in the treasury); with the money it
/// asks for its troops' supply and for new equipment.
pub fn order_parts(pack: &Pack, n: &Nation, c: &Country, use_: f32, cap: f32) -> (Vec<f32>, f32, f32) {
    let nk = pack.goods.len();
    let money = n.army_money();
    let p = &n.price;
    let need = needs(pack, &c.army.count, &c.norm, use_);
    // (a war reserve: the stores kept stocked for `reserve` months of the army at its fiercest too)
    let hold = if c.reserve > 0.0 { needs(pack, &c.army.count, &c.norm, 1.0) } else { vec![0.0; nk] };
    let goal = |i: usize| (1.0 + STORE) * need[i] + c.reserve * hold[i].max(need[i]);
    let mut want: Vec<f32> = (0..nk).map(|i| (goal(i) - n.arsenal[i]).max(0.0) * p[i]).collect();
    let supply: f32 = want.iter().sum();
    let mut equip = 0.0f32;
    if supply < money {
        let rest = (money - supply).min(cap);
        equip = rest;
        let makes: Vec<usize> = (0..pack.units.len()).filter(|&u| c.make_share(u) > 0.0).collect();
        let shares: f32 = makes.iter().map(|&u| c.make_share(u)).sum();
        let mut equip_money = vec![0.0f32; nk];
        for &u in &makes {
            let equip = &pack.units[u].equip;
            let worth: f32 = equip.iter().map(|&(i, a)| a * p[i]).sum();
            if worth <= 0.0 {
                continue;
            }
            for &(i, a) in equip {
                equip_money[i] += rest * c.make_share(u) / shares * a * p[i] / worth;
            }
        }
        // what lies in store beyond the troops' supply is not bought again (steel piled up
        // unused while arms, which every kind needs too, were short)
        for i in 0..nk {
            let spare = (n.arsenal[i] - goal(i)).max(0.0) * p[i];
            want[i] += (equip_money[i] - spare).max(0.0);
        }
    }
    let all: f32 = want.iter().sum();
    if all > 0.0 {
        want.iter_mut().for_each(|w| *w /= all);
    }
    (want, supply.min(money), equip)
}

/// How many more months of fighting the stores of supply (`n.arsenal`) cover for `army` once this month's is taken
/// (`use_` of a month of fighting): by their worth at starting prices; unbounded when the army needs none.
pub fn cover(pack: &Pack, n: &Nation, army: &[f32], norm: &[f32], use_: f32) -> f32 {
    let need = needs(pack, army, norm, use_);
    let (mut have, mut all) = (0.0f32, 0.0f32);
    for (i, &q) in need.iter().enumerate() {
        if q > 0.0 {
            have += n.arsenal[i].min(10.0 * q) * pack.goods[i].price;
            all += q * pack.goods[i].price;
        }
    }
    if all > 0.0 {
        have / all
    } else {
        f32::INFINITY
    }
}

/// The troops of `army` take their month's supply (`use_` of a month of fighting) from the
/// army's stores: the share of it they got (by its worth at starting prices).
pub fn feed(pack: &Pack, n: &mut Nation, army: &[f32], norm: &[f32], use_: f32) -> f32 {
    let need = needs(pack, army, norm, use_);
    let (mut got, mut all) = (0.0f32, 0.0f32);
    for (i, &q) in need.iter().enumerate() {
        if q <= 0.0 {
            continue;
        }
        let take = q.min(n.arsenal[i]);
        n.arsenal[i] -= take;
        got += take * pack.goods[i].price;
        all += q * pack.goods[i].price;
    }
    if all > 0.0 {
        got / all
    } else {
        1.0
    }
}

/// New units from what the army's stores hold, in the shares the staff wants, as far as the
/// goods and the men who may still be called up go: units of each kind. The goods and the men
/// are taken.
pub fn equip(pack: &Pack, n: &mut Nation, making: &[f32], norm: &[f32]) -> Vec<f32> {
    let (nu, nk) = (pack.units.len(), pack.goods.len());
    let scale = pack.war.as_ref().unwrap().scale;
    // (a unit's kit by its norm: a light one takes less of each good; the standard, as before)
    let kit = |u: usize| norm.get(u).copied().unwrap_or(1.0);
    // (what the factories still learning a model, or making a dearer one, spend on a kit)
    let make: Vec<f32> = (0..nu).map(|u| n.make_k.get(u).copied().unwrap_or(1.0)).collect();
    let kit = |u: usize| if make[u] == 1.0 { kit(u) } else { kit(u) * make[u] };
    let mut made = vec![0.0f32; nu];
    for _ in 0..PASSES {
        let kinds: Vec<usize> =
            (0..nu).filter(|&u| making[u] > 0.0 && pack.units[u].equip.iter().all(|&(i, _)| n.arsenal[i] > 1e-9)).collect();
        if kinds.is_empty() {
            break;
        }
        // each good shared among the kinds by how much of their money goes to it
        let mut weight = vec![vec![0.0f32; nk]; nu];
        let mut total = vec![0.0f32; nk];
        for &u in &kinds {
            let equip = &pack.units[u].equip;
            let worth: f32 = equip.iter().map(|&(i, a)| a * n.price[i]).sum::<f32>().max(1e-9);
            for &(i, a) in equip {
                weight[u][i] = making[u] * a * n.price[i] / worth;
                total[i] += weight[u][i];
            }
        }
        let mut got = vec![0.0f32; nu];
        for &u in &kinds {
            got[u] = pack.units[u]
                .equip
                .iter()
                .map(|&(i, a)| n.arsenal[i] * weight[u][i] / total[i].max(1e-12) / (a * kit(u) * scale))
                .fold(f32::INFINITY, f32::min);
        }
        let men = men_of(pack, &got);
        let room = n.draftable();
        if men > room {
            got.iter_mut().for_each(|g| *g *= room / men);
        }
        let men = men_of(pack, &got);
        if men <= 0.0 {
            break;
        }
        n.draft(men);
        for &u in &kinds {
            for &(i, a) in &pack.units[u].equip {
                n.arsenal[i] = (n.arsenal[i] - got[u] * a * kit(u) * scale).max(0.0);
            }
            made[u] += got[u];
        }
    }
    made
}

/// Men (millions) of the units `units`.
/// Shares of each kind in the worth of an army.
pub fn shares(pack: &Pack, army: &[f32]) -> Vec<f32> {
    let worth: Vec<f32> = army.iter().zip(&pack.units).map(|(n, u)| n * u.cost).collect();
    let all: f32 = worth.iter().sum::<f32>().max(1e-9);
    worth.iter().map(|w| w / all).collect()
}

/// Worth of an army in lines of the era's cheapest soldiers along a border of `border` provinces.
pub fn lines(pack: &Pack, army: &[f32], border: f32) -> f32 {
    let war = pack.war.as_ref().unwrap();
    let cheap = pack.units.iter().filter(|u| u.line == crate::pack::Line::Front).map(|u| u.cost / u.width).fold(f32::INFINITY, f32::min);
    let worth: f32 = army.iter().zip(&pack.units).map(|(n, u)| n * u.cost).sum();
    worth / (border * war.width * cheap).max(1e-9)
}

pub fn men_of(pack: &Pack, units: &[f32]) -> f32 {
    let crew = pack.war.as_ref().unwrap().crew;
    units.iter().zip(&pack.units).map(|(l, u)| l * u.men * crew / 1000.0).sum()
}

/// Two countries of a trading world whose economies raise their armies: at peace, then at war
/// on the map. The rest of the world trades with both (and sells them arms); between the two
/// the war closes the way.
#[derive(Clone)]
pub struct Homefront {
    pub world: World,
    /// The two countries among the world's.
    pub at: [usize; 2],
    pub war: Campaign,
    pub at_war: bool,
    pub peace_months: u32,
    /// Share of their supply each side's troops got last month.
    pub fed: [f32; 2],
    /// How many months of the troops' supply the stores still hold after this month's (`cover`).
    pub cover: [f32; 2],
    /// Men killed on each side (millions), and units each made, since the war began.
    pub dead: [f32; 2],
    pub made: [Vec<f32>; 2],
    /// Units each side's economy made last month (in peace or war).
    pub last_made: [Vec<f32>; 2],
    /// Supply shocks: from war month `.0` on, side `.1`'s sectors making good `.2` give `.3` of
    /// their output (a plant bombed, a mine lost). For experiments.
    pub shocks: Vec<(u32, usize, usize, f32)>,
    /// Whether what the enemy holds of a country leaves its economy (people, workplaces, stores,
    /// money). Without it (a control), a country produces as much with half its land lost.
    pub occupation: bool,
    /// Whether the war weighs on the people at home: a war footing (the army's share of the
    /// state's spending, a war tax) and grief for the dead. Without it (a control), a war that
    /// asks nothing more of them than peace did.
    pub burden: bool,
    /// Each side's economist, if it has one (otherwise the state keeps to its custom, or to what
    /// the player sets).
    pub econ: [Option<Economist>; 2],
    /// The share of the state's spending each side's government gives its army, if it sets one
    /// (otherwise the custom at peace, the war footing at war).
    pub army_share: [Option<f32>; 2],
    /// Each side's laboratories, if it has science (its state gives science its share of
    /// spending, `Nation::science_share`).
    pub labs: [Option<Lab>; 2],
    /// Each side's scientist heading its laboratories, if it has one (otherwise they work on
    /// every sphere alike, or on what the player sets).
    pub scientists: [Option<Scientist>; 2],
    /// Each side's war footing: the army's share of the state's spending at war and how many times
    /// the pack's tax the war tax is (`WAR_MIL`, `WAR_TAX` unless a game sets it — the game's
    /// countries go onto it month by month).
    pub footing: [(f32, f32); 2],
    /// Share of the army that leaves service each month of peace, its equipment with it (`RETIRE`
    /// unless a game sets it: equipment that is mended lasts for years).
    pub retire: f32,
    /// The most money each side's army spends on the equipment of new units this month (none: all
    /// it has left after its supply).
    pub equip_cap: [f32; 2],
    /// Other orders of each side's state paid out of its army's money (a navy, the mending of
    /// equipment): money by good, this month (empty: none); the goods they got, set apart from the
    /// army's stores after the market; the money each side's army asked for this month — supply,
    /// equipment, the other orders.
    pub side: [Vec<f32>; 2],
    pub side_got: [Vec<f32>; 2],
    pub spend: [[f32; 3]; 2],
    /// How much of a month of fighting's supply each side's troops burn this month of war (1: all of
    /// it, as always; a game sets it by how hard they fight — offensives burn more, a quiet front
    /// less).
    pub war_use: [f32; 2],
}

impl Homefront {
    pub fn new(pack: &Pack, world: World, at: [usize; 2], war: Campaign) -> Homefront {
        let nu = pack.units.len();
        let mut h = Homefront {
            world,
            at,
            war,
            at_war: false,
            peace_months: 0,
            fed: [1.0; 2],
            cover: [f32::INFINITY; 2],
            dead: [0.0; 2],
            made: [vec![0.0; nu], vec![0.0; nu]],
            last_made: [vec![0.0; nu], vec![0.0; nu]],
            shocks: Vec::new(),
            burden: true,
            occupation: true,
            econ: [None, None],
            army_share: [None, None],
            labs: [None, None],
            scientists: [None, None],
            footing: [(WAR_MIL, WAR_TAX); 2],
            retire: RETIRE,
            equip_cap: [f32::INFINITY; 2],
            side: [Vec::new(), Vec::new()],
            side_got: [Vec::new(), Vec::new()],
            spend: [[0.0; 3]; 2],
            war_use: [1.0; 2],
        };
        for s in 0..2 {
            let n = &mut h.world.nations[at[s]];
            n.arsenal.iter_mut().for_each(|x| *x = 0.0);
            h.war.sides[s].army.count.iter_mut().for_each(|x| *x = 0.0);
            h.war.sides[s].cost = costs(pack, n);
            h.war.sides[s].parts = parts(pack, n);
        }
        h
    }

    pub fn home(&self, s: usize) -> &Nation {
        &self.world.nations[self.at[s]]
    }

    pub fn home_mut(&mut self, s: usize) -> &mut Nation {
        &mut self.world.nations[self.at[s]]
    }

    /// The ground of the front, by terrain.
    pub fn ground(&self) -> Vec<(Terrain, f32)> {
        let mut ground: Vec<(Terrain, f32)> = Vec::new();
        for s in 0..2 {
            for p in self.war.front(s) {
                let t = self.war.th.terrain[p];
                match ground.iter_mut().find(|g| g.0 == t) {
                    Some(g) => g.1 += 1.0,
                    None => ground.push((t, 1.0)),
                }
            }
        }
        ground
    }

    /// What side `s`'s government asks of the economy for the army: its share of the state's
    /// spending (what it sets, or the custom at peace and the war footing at war) and the custom
    /// tax (at war, the war tax).
    pub fn ask(&self, pack: &Pack, s: usize) -> (f32, f32) {
        let (mil, tax) = if self.at_war && self.burden { (self.footing[s].0, pack.tax * self.footing[s].1) } else { (pack.military, pack.tax) };
        (self.army_share[s].unwrap_or(mil), tax)
    }

    /// The economies' month: the economists sit, both armies order, the world's markets work,
    /// the troops take their supply (`use_` of a month of fighting), new units are made. Returns
    /// the units made.
    fn economy(&mut self, pack: &Pack, uses: [f32; 2]) -> [Vec<f32>; 2] {
        let mut frac: [Vec<f32>; 2] = [Vec::new(), Vec::new()];
        for s in 0..2 {
            let ask = self.ask(pack, s);
            let c = self.at[s];
            if let Some(e) = self.econ[s].as_mut() {
                let outside = Outside::of(&self.world.nations[c], self.world.pending(c));
                e.month(pack, &mut self.world.nations[c], ask, &outside);
            }
        }
        for s in 0..2 {
            let n = &mut self.world.nations[self.at[s]];
            let c = &mut self.war.sides[s];
            c.cost = costs(pack, n);
            c.parts = parts(pack, n);
            if !c.norm.is_empty() {
                c.price_norms();
            }
            c.retool();
            // (what other orders of the state take of the army's money, its own order does not see)
            let side: f32 = self.side[s].iter().sum();
            n.army_side = side;
            let (want, supply, equip) = order_parts(pack, n, c, uses[s], self.equip_cap[s]);
            n.army_side = 0.0;
            self.spend[s] = [supply, equip, side];
            if side > 0.0 {
                // the army's shares of what is left to it, and the other orders' money by good
                let own = (n.army_money() - side).max(0.0);
                let money: Vec<f32> = (0..want.len()).map(|i| want[i] * own + self.side[s].get(i).copied().unwrap_or(0.0)).collect();
                let all: f32 = money.iter().sum::<f32>().max(1e-12);
                frac[s] = (0..money.len()).map(|i| if money[i] > 0.0 { self.side[s].get(i).copied().unwrap_or(0.0) / money[i] } else { 0.0 }).collect();
                n.army_wants = money.iter().map(|m| m / all).collect();
            } else {
                n.army_wants = want;
            }
        }
        let before: [Vec<f32>; 2] = [self.world.nations[self.at[0]].arsenal.clone(), self.world.nations[self.at[1]].arsenal.clone()];
        self.world.step(pack);
        // what the other orders bought goes to them, not to the army's stores
        for s in 0..2 {
            if frac[s].is_empty() {
                self.side_got[s].clear();
                continue;
            }
            let n = &mut self.world.nations[self.at[s]];
            let got: Vec<f32> = (0..n.arsenal.len()).map(|i| (n.arsenal[i] - before[s][i]).max(0.0) * frac[s][i]).collect();
            for i in 0..got.len() {
                n.arsenal[i] -= got[i];
            }
            self.side_got[s] = got;
        }
        for s in 0..2 {
            if let Some(e) = self.econ[s].as_mut() {
                e.after(&self.world.nations[self.at[s]]);
            }
        }
        let month = self.peace_months + self.war.month;
        let ground = if self.scientists.iter().any(|x| x.is_some()) { self.ground() } else { Vec::new() };
        for s in 0..2 {
            if let Some(lab) = self.labs[s].as_mut() {
                if let Some(sc) = self.scientists[s].as_mut() {
                    let seed = hash64(self.war.seed ^ 0x5C ^ s as u64);
                    sc.month(pack, lab, &mut self.world.nations[self.at[s]], &self.war.sides[s], &ground, seed);
                }
                // in a war, the enemy's units are there to be studied
                let [a, b] = &mut self.war.sides;
                let (ours, theirs) = if s == 0 { (a, &*b) } else { (b, &*a) };
                let foe = if self.at_war { Some(theirs) } else { None };
                lab.month(pack, &mut self.world.nations[self.at[s]], ours, foe, month);
            }
        }
        let mut made: [Vec<f32>; 2] = [Vec::new(), Vec::new()];
        for s in 0..2 {
            let n = &mut self.world.nations[self.at[s]];
            let c = &self.war.sides[s];
            if std::env::var("SUPDBG").is_ok() {
                let need = needs(pack, &c.army.count, &c.norm, uses[s]);
                let line: Vec<String> = (0..need.len()).filter(|&i| need[i] > 0.0).map(|i| format!("{} нужно {:.2} склад {:.2}", pack.goods[i].name, need[i], n.arsenal[i])).collect();
                eprintln!("СНАБ сторона {s} месяц {} армия {:.0} использование {:.2} деньги армии {:.2}: {}", self.war.month, c.army.count.iter().sum::<f32>(), uses[s], n.army_money(), line.join("; "));
            }
            self.fed[s] = feed(pack, n, &c.army.count, &c.norm, uses[s]);
            self.cover[s] = cover(pack, n, &c.army.count, &c.norm, uses[s]);
            made[s] = equip(pack, n, &c.shares_made(), &c.norm);
            self.last_made[s] = made[s].clone();
        }
        made
    }

    /// A month of peace: the staffs sit each season over the ground of their border, see their
    /// equipment at exercises and the neighbour's army through thick fog once a year; the army
    /// is bought, used a little, and an old part of it goes back to work.
    pub fn peace(&mut self, pack: &Pack) {
        self.peace_months += 1;
        let seed = hash64(self.war.seed ^ 0x9EACE ^ self.peace_months as u64);
        let mut rng = Rng::new(seed);
        if self.peace_months % 3 == 0 {
            let ground = self.ground();
            let [a, b] = &mut self.war.sides;
            rayon::join(|| a.rethink(pack, &ground, seed ^ 1), || b.rethink(pack, &ground, seed ^ 2));
        }
        let made = self.economy(pack, [PEACE_USE; 2]);
        for s in 0..2 {
            let c = &mut self.war.sides[s];
            let mut gone = vec![0.0f32; made[s].len()];
            for u in 0..gone.len() {
                c.army.count[u] += made[s][u];
                gone[u] = c.army.count[u] * self.retire;
                c.army.count[u] -= gone[u];
            }
            c.age();
            if self.peace_months % 12 == 0 {
                c.exercise(&mut rng);
            }
            let men = men_of(pack, &gone);
            self.world.nations[self.at[s]].release(men);
        }
        if self.peace_months % 12 == 0 {
            let (x, y) = (self.war.sides[1].army.clone(), self.war.sides[0].army.clone());
            let fogs = [self.war.sides[0].fog * PEACE_FOG, self.war.sides[1].fog * PEACE_FOG];
            self.war.sides[0].spy(&x, fogs[0], &mut rng);
            self.war.sides[1].spy(&y, fogs[1], &mut rng);
        }
    }

    /// The war begins: the way between the two closes, the armies stand along their borders,
    /// the states go on a war footing.
    pub fn declare(&mut self, pack: &Pack) {
        self.at_war = true;
        self.world.cut(self.at[0], self.at[1]);
        self.war.place();
        if self.burden {
            for s in 0..2 {
                let n = &mut self.world.nations[self.at[s]];
                n.mil_share = self.footing[s].0;
                n.tax = pack.tax * self.footing[s].1;
            }
        }
    }

    /// Peace: what each side holds of the other's land becomes its own — the people, workplaces,
    /// land, money and stores of it go over; the troops go home, and each state asks of its
    /// people what it asks in peace.
    pub fn make_peace(&mut self, pack: &Pack) {
        let held = self.war.people();
        for s in 0..2 {
            if self.occupation {
                self.world.nations[self.at[s]].occupy(1.0 - held[s]);
            }
        }
        if self.occupation {
            for s in 0..2 {
                let (i, j) = (self.at[s], self.at[1 - s]);
                let (lo, hi) = self.world.nations.split_at_mut(i.max(j));
                let (from, to) = if i < j { (&mut lo[i], &mut hi[0]) } else { (&mut hi[0], &mut lo[j]) };
                from.cede(to);
            }
        }
        self.war.end();
        self.at_war = false;
        for s in 0..2 {
            let (mil, tax) = self.ask(pack, s);
            let n = &mut self.world.nations[self.at[s]];
            n.mil_share = mil;
            if self.econ[s].is_none() {
                n.tax = tax;
            }
        }
    }

    /// Side `s` is cut off from all trade (a blockade).
    pub fn blockade(&mut self, s: usize) {
        for j in 0..self.world.nations.len() {
            if j != self.at[s] {
                self.world.cut(self.at[s], j);
            }
        }
    }

    /// A month of war: each economy feeds and arms its army, the front fights, the dead are
    /// mourned.
    pub fn step(&mut self, pack: &Pack) {
        if self.war.fallen.is_some() {
            return;
        }
        let month = self.war.month + 1;
        for &(from, s, good, k) in &self.shocks {
            if month >= from {
                let n = &mut self.world.nations[self.at[s]];
                for (sec, sector) in pack.sectors.iter().enumerate() {
                    if sector.outputs.iter().any(|o| o.0 == good) {
                        n.shock[sec] = k;
                    }
                }
            }
        }
        let made = self.economy(pack, self.war_use);
        for s in 0..2 {
            self.war.stores[s] = self.fed[s];
            self.war.cover[s] = self.cover[s];
        }
        let before = [self.war.sides[0].army.count.clone(), self.war.sides[1].army.count.clone()];
        for s in 0..2 {
            self.war.made[s] = Some(made[s].clone());
            for (t, m) in self.made[s].iter_mut().zip(&made[s]) {
                *t += m;
            }
        }
        self.war.step(pack);
        if self.war.fallen.is_some() {
            // the war is over before the books were made up
            return;
        }
        let held = self.war.people();
        for s in 0..2 {
            if self.occupation {
                self.world.nations[self.at[s]].occupy(1.0 - held[s]);
            }
            let after = &self.war.sides[s].army.count;
            let lost: Vec<f32> = (0..after.len()).map(|u| (before[s][u] + made[s][u] - after[u]).max(0.0)).collect();
            let men = men_of(pack, &lost);
            self.dead[s] += men * KILLED;
            let n = &mut self.world.nations[self.at[s]];
            n.bury(men * KILLED, self.burden);
            n.release(men * (1.0 - KILLED));
        }
    }
}

/// The pair of neighbours with the longest border whose economies differ less than twice (as
/// on the front's map).
pub fn pick(world: &map::World, pol: &Politics) -> Option<(usize, usize)> {
    pick_by(world, pol, &|c| pol.gdp(c), 2.0)
}

/// The pair of neighbours with the longest border whose economies, as `gdp` measures them,
/// differ less than `most` times.
pub fn pick_by(world: &map::World, pol: &Politics, gdp: &dyn Fn(usize) -> f32, most: f32) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, usize)> = None;
    for a in 0..pol.countries.len() {
        if pol.countries[a].states.is_empty() {
            continue;
        }
        for b in pol.neighbors(a) {
            if b <= a {
                continue;
            }
            let (ga, gb) = (gdp(a), gdp(b));
            // (a country measured at nothing is left out)
            if ga <= 0.0 || gb <= 0.0 {
                continue;
            }
            if ga.max(gb) > most * ga.min(gb) {
                continue;
            }
            let len = Theatre::border(world, pol, a, b);
            if best.is_none_or(|x| len > x.2) {
                best = Some((a, b, len));
            }
        }
    }
    best.map(|x| (x.0, x.1))
}

/// What the map says about country `c`.
pub fn setup_of(pol: &Politics, c: usize) -> Setup {
    Setup { name: pol.countries[c].name.clone(), pop: pol.pop(c), dev: pol.dev(c), fertility: pol.countries[c].fertility, land: pol.countries[c].land, deposits: pol.res(c) }
}

/// The map's countries trading, each from `setups`, settled before the game: `alone` months each
/// on its own, then `together` trading (as in the trade checks).
pub fn trading(pack: &Pack, world: &map::World, pol: &Politics, setups: &[Setup], alone: u32, together: u32) -> World {
    let routes = trade::routes(world, pol, pack.trade.as_ref().expect("в наборе нет торговли"));
    let mut w = World::new(pack, setups.iter().map(|s| Nation::new(pack, s)).collect(), routes);
    w.settle(pack, alone, together);
    w
}


/// As `trading`, but each country's sectors are as productive as `prod_k[country][sector]` times
/// what the map makes them (what it knows of the era's techniques, what its economists did in the
/// world's past); an empty `prod_k` — exactly `trading`.
pub fn trading_with(pack: &Pack, world: &map::World, pol: &Politics, setups: &[Setup], alone: u32, together: u32, prod_k: &[Vec<f32>]) -> World {
    let routes = trade::routes(world, pol, pack.trade.as_ref().expect("в наборе нет торговли"));
    let nations = setups
        .iter()
        .enumerate()
        .map(|(c, s)| {
            let mut n = Nation::new(pack, s);
            if let Some(k) = prod_k.get(c) {
                for (p, &x) in n.prod.iter_mut().zip(k) {
                    *p *= x;
                }
            }
            n
        })
        .collect();
    let mut w = World::new(pack, nations, routes);
    w.settle(pack, alone, together);
    w
}
