//! The war of divisions, as in HOI4: each side's army is divisions, each a whole of its own — made
//! by a template (its battalions of the pack's kinds), with its strength (the units it has of
//! what the template holds), its organisation (the share of it still fit to fight), its
//! experience, how dug in it is. A division stands in a province and marches province by
//! province, day by day: a plain takes a week and more for foot soldiers, a mountain twice as
//! long, a railway far less; the slowest of its kinds sets its pace.
//!
//! A division sent into a province of the enemy where the enemy stands attacks it from where it
//! stands: a fight begins there and goes on day by day, all the divisions of the enemy in the
//! province defending, all that attack it from around joining in. Only so many fit the width of
//! the ground (more when they come from more sides); the rest wait. Harm is done as in a battle
//! of the war's model (`war`): each kind fires at what it can harm, the defender keeps part of
//! it off by the ground and his trenches; it kills some men and wears out more — a division worn
//! out of its organisation leaves the fight: a defender falls back to a province of its own, or,
//! with none, is lost; an attacker stops. The province is taken when its defenders are gone and
//! the attackers walk in; a province of the enemy where nobody stands is taken by walking in.
//!
//! Supply: a division gets what the way of supply brings to its province (the front's `supply`),
//! less when more stand there than the province can feed; hungry divisions recover slowly, fire
//! less and melt away. Divisions are refilled from what the economy makes; whole new divisions
//! are formed by the templates each side builds.
//!
//! Who leads them: a division in a group of armies obeys its general — he holds his sector, each
//! stretch by the enemy he sees across it, and, told to attack, gathers blows where his war
//! games give him the odds he wants and strikes when they are ready (checking the odds again as
//! they are); the staff's reserve comes to the help of provinces under attack. A division the
//! player sent himself goes where he sent it and stays there till he gives it back. The bot's
//! divisions and the player's are led by the same rules — no side sees more than its eyes.

use crate::military::battle::{self, Army};
use crate::military::doctrine::norm;
use crate::military::front::{Campaign, Fought, General, Order, NO};
use crate::pack::{Line, Pack};
use crate::rng::{hash64, Rng};
use crate::world::map::Terrain;

mod day;
mod fight;
mod generals;
mod judge;
mod muster;
mod orders;
mod staff;
mod ways;

/// No group (the staff's reserve, or the player's own), no province.
pub const NONE: usize = usize::MAX;

/// Days a division of speed 1 needs to cross a province of plains (a province is a few hundred
/// kilometres across; foot soldiers march some thirty a day).
pub const STEP_DAYS: f32 = 6.0;
/// A battle's width grows by this much for every side more it is attacked from, to at most
/// this many times its own.
const EXTRA_SIDE: f32 = 0.5;
const WIDEST: f32 = 2.0;
/// A division with less of its organisation than this leaves the fight.
pub const ORG_OUT: f32 = 0.25;
/// Organisation a resting, supplied division gets back in a day.
const ORG_BACK: f32 = 0.04;
/// A general puts on a province of his line no more than its battle's width holds this many times
/// over (a line and its relief): the rest stands in depth behind and comes up when the province is
/// attacked. (A whole army on the one province of a short stretch fought no wider for it.)
const LINE_FILL: f32 = 1.5;
/// A blow is struck when its divisions have this much of their organisation.
const READY: f32 = 0.6;
/// Days a division that fell back needs before it can be told anything.
const RETREAT_DAYS: f32 = 3.0;
/// Every so many days a general places his divisions anew.
const PLAN_EVERY: u32 = 5;
/// A province feeds this many full lines of a plain (more on a railway, near a depot).
const CAP_LINES: f32 = 3.0;
/// Below this share of its supply a division starves: it melts away, its organisation is capped.
/// (The way of supply brings at least a fifth to any province its capital reaches: a division
/// starves cut off, or crowded where the province cannot feed so many. At 0.3, an army fighting
/// far from its capital starved from the first day — twice the divisions lost the war.)
const STARVE: f32 = 0.15;
const ATTRITION: f32 = 0.006;
/// Share of itself a division cut off from its capital loses in a day, in its own land (on stores
/// and what the land gives) and in the enemy's (as the front's pockets melted: a fifth and two
/// fifths a month).
const CUT_LOSS: [f32; 2] = [0.007, 0.016];
/// Share of what it lacks a division gets in a day from what the country has made.
const REFILL: f32 = 0.1;
/// Share of a standing division's food a division passing through a province of its own takes
/// from it.
const MARCH_EAT: f32 = 0.5;
/// Organisation of a division just formed.
const FRESH_ORG: f32 = 0.3;
/// Experience a day of fighting gives (less the more it has), and what full experience adds to
/// its fire.
const EXP_GAIN: f32 = 0.01;
const EXP_FIRE: f32 = 0.25;
/// A new division is recruits: it drills this many days standing behind the line (its own land
/// off the front, where new divisions gather) before it is trained; till then its general leaves it be, and it fires at
/// `GREEN` of a trained one's fire and more as it learns (as in HOI4: a division trained, or
/// sent out early by the player, green). (A month: HOI4's 90 days are some 5% of its wars of
/// four to six years, ours last a year and a half to three.)
pub const TRAIN_DAYS: f32 = 30.0;
const GREEN: f32 = 0.6;
/// A blow not struck after so many days is given up.
const BLOW_DAYS: u32 = 20;
/// A battalion: this share of a unit of the pack.
pub const BATTALION: f32 = 0.1;
/// In a division built around the line (`line_first`), the least share of the era's soldier among
/// the battalions standing in the line.
const SOLDIERS: f32 = 0.4;
/// In a retreat a division leaves behind this many times as much of its equipment as of its men
/// (guns and lorries are heavier to save than men), less what its repairmen save; the pursuer
/// takes this share of what was left as trophies (more with repairmen of its own).
const ABANDON: f32 = 2.0;
const CAPTURE: f32 = 0.4;
/// A division formed short of equipment: at least this share of what its norm gives it.
const KIT_LEAST: f32 = 0.3;
/// Battalions a division of the staff holds at least and at most.
pub const FEWEST: f32 = 5.0;
pub const MOST: f32 = 18.0;
/// How many divisions the staff wants for each province of its front (a line, a relief, a
/// reserve), by the order of its groups: attacking it concentrates, in deep defence it spreads.
const DEPTH: [f32; 3] = [6.0, 8.0, 5.0];
/// The staff changes the size of its divisions only when its army and front call for a fifth more
/// or less (else every month would bring a new pattern).
const RESIZE: f32 = 0.2;
/// A staff gives its divisions a new pattern no oftener than every so many months (unless none
/// can be formed by the one it has).
const REORG: u32 = 3;
/// A month's tallies (`Corps::tally`): attacks launched, failed, stopped worn out; units lost;
/// division-days in a crowd, all division-days; equipment left behind, carried.
const T_ATT: usize = 0;
const T_FAIL: usize = 1;
const T_STOP: usize = 2;
const T_DEAD: usize = 3;
const T_CROWD: usize = 4;
const T_DAYS: usize = 5;
const T_LEFT: usize = 6;
/// A company is worth its price to the staff while it adds to a division's price no more than
/// this share, more by as much again for every need it answers fully.
const SUPPORT_BASE: f32 = 0.03;
const SUPPORT_NEED: f32 = 0.09;
/// How fast the staff's sense of what it lacks follows the months.
const LACK_PACE: f32 = 0.3;

/// Whether ground `t` is rough (where sappers and their bridges help a march).
pub fn rough(t: Terrain) -> bool {
    matches!(t, Terrain::Forest | Terrain::Hills | Terrain::Marsh | Terrain::Jungle | Terrain::Mountains)
}

/// Worth of a standard kit of kind `u`'s equipment at the pack's prices.
fn equip_worth(pack: &Pack, u: usize) -> f32 {
    pack.units[u].equip.iter().map(|&(i, a)| a * pack.goods[i].price).sum::<f32>().max(1e-6)
}

/// The need a company of support answers (`NEED`), as the staff weighs it: the most of those its
/// figures speak to.
fn need_of(x: &crate::pack::Support, lack: &[f32; 6]) -> f32 {
    let mut n = 0.0f32;
    if x.dig > 1.0 || x.rough < 1.0 {
        n = n.max(lack[0]);
    }
    if x.see > 0.0 || x.fire > 1.0 {
        n = n.max(lack[1]);
    }
    if x.org > 1.0 {
        n = n.max(lack[2]);
    }
    if x.saved > 0.0 {
        n = n.max(lack[3]);
    }
    if x.eat < 1.0 {
        n = n.max(lack[4]);
    }
    if x.salvage > 0.0 {
        n = n.max(lack[5]);
    }
    n
}

/// How much longer than a plain each ground is to cross.
pub fn ground_cost(t: Terrain) -> f32 {
    match t {
        Terrain::Plains | Terrain::Desert => 1.0,
        Terrain::Tundra => 1.2,
        Terrain::Forest | Terrain::Hills => 1.4,
        Terrain::Marsh | Terrain::Jungle => 1.8,
        Terrain::Arctic => 2.0,
        Terrain::Mountains => 2.2,
        Terrain::Ocean | Terrain::Lake => 3.0,
    }
}

/// How a whole side's divisions fight, beyond their templates: what its country brings to the war
/// (the game's national spirits, laws, the ways of its army). Every figure 1: as the divisions are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mods {
    /// Fire when attacking, and when defending.
    pub attack: f32,
    pub defend: f32,
    /// Harm taken (less: tougher men, better doctors).
    pub harm: f32,
    /// Organisation got back in a day, digging in, experience a day of fighting gives.
    pub org_back: f32,
    pub dig: f32,
    pub exp_gain: f32,
    /// Losses of the hungry and the cut off.
    pub attrition: f32,
}

impl Default for Mods {
    fn default() -> Mods {
        Mods::NONE
    }
}

impl Mods {
    pub const NONE: Mods = Mods { attack: 1.0, defend: 1.0, harm: 1.0, org_back: 1.0, dig: 1.0, exp_gain: 1.0, attrition: 1.0 };
}

/// Share of its harm fire does to a target whose armour it does not pierce (HOI4's armour: the
/// rest glances off).
pub const BOUNCE: f32 = 0.5;

/// A tactic of a day of battle (as HOI4's): who may use it (0 the attacker, 1 the defender), what
/// it does to the fire of the side's line, of its support (guns behind the line), of its armour,
/// and to the harm it takes; what it needs (`NEED_*`); the tactics of the other side it counters
/// (theirs then does nothing, and ours more).
pub struct Tactic {
    pub name: &'static str,
    pub side: u8,
    pub fire: f32,
    pub support: f32,
    pub armour: f32,
    pub harm: f32,
    pub needs: u8,
    pub counters: &'static [u8],
}

/// What a tactic needs: nothing; attacked from two sides or more; guns behind the line; armour;
/// rough ground.
pub const NEED_NONE: u8 = 0;
pub const NEED_SIDES: u8 = 1;
pub const NEED_GUNS: u8 = 2;
pub const NEED_ARMOUR: u8 = 3;
pub const NEED_ROUGH: u8 = 4;

pub static TACTICS: [Tactic; 8] = [
    Tactic { name: "Натиск", side: 0, fire: 1.12, support: 1.0, armour: 1.0, harm: 1.08, needs: NEED_NONE, counters: &[] },
    Tactic { name: "Охват", side: 0, fire: 1.2, support: 1.0, armour: 1.0, harm: 1.0, needs: NEED_SIDES, counters: &[5] },
    Tactic { name: "Артподготовка", side: 0, fire: 1.0, support: 1.4, armour: 1.0, harm: 0.92, needs: NEED_GUNS, counters: &[4] },
    Tactic { name: "Танковый клин", side: 0, fire: 1.0, support: 1.0, armour: 1.3, harm: 1.0, needs: NEED_ARMOUR, counters: &[] },
    Tactic { name: "Глухая оборона", side: 1, fire: 1.0, support: 1.0, armour: 1.0, harm: 0.9, needs: NEED_NONE, counters: &[0] },
    Tactic { name: "Контратака", side: 1, fire: 1.22, support: 1.0, armour: 1.0, harm: 1.05, needs: NEED_NONE, counters: &[0, 3] },
    Tactic { name: "Эластичная оборона", side: 1, fire: 0.92, support: 1.0, armour: 1.0, harm: 0.88, needs: NEED_NONE, counters: &[1, 3] },
    Tactic { name: "Засада", side: 1, fire: 1.25, support: 1.0, armour: 1.0, harm: 0.95, needs: NEED_ROUGH, counters: &[3] },
];
/// A side's general chooses its tactic anew every so many days of a fight.
pub const TACTIC_DAYS: u32 = 4;

/// A template as the war knows it: its name, the units of each kind a full division has, the
/// norm of equipment of each kind (`doctrine::norm`; empty: standard), its companies of support
/// (indices into the pack's `support`).
#[derive(Clone, Debug)]
pub struct Shape {
    pub name: String,
    pub units: Vec<f32>,
    pub norm: Vec<f32>,
    pub support: Vec<usize>,
}

/// What a division's companies of support give it, all together.
#[derive(Clone, Copy, Debug)]
pub struct Gift {
    pub dig: f32,
    pub rough: f32,
    pub see: f32,
    pub org: f32,
    pub saved: f32,
    pub eat: f32,
    pub fire: f32,
    pub salvage: f32,
}

impl Gift {
    pub const NONE: Gift = Gift { dig: 1.0, rough: 1.0, see: 0.0, org: 1.0, saved: 0.0, eat: 1.0, fire: 1.0, salvage: 0.0 };
}

impl Shape {
    /// A template of the units `units`, standard kit, no companies.
    pub fn new(name: &str, units: Vec<f32>) -> Shape {
        Shape { name: name.into(), units, norm: Vec::new(), support: Vec::new() }
    }

    /// A template of `size` battalions (rounded) in the proportions of `counts` (by largest
    /// remainders: as many as it says, no more).
    pub fn sized(name: &str, counts: &[f32], size: f32) -> Shape {
        let n = size.round().max(1.0) as usize;
        let w: Vec<f32> = counts.iter().map(|x| x.max(0.0)).collect();
        let bat = if w.iter().sum::<f32>() > 0.0 {
            Campaign::apportion(&w, n, false)
        } else {
            let mut b = vec![0usize; counts.len()];
            if let Some(u) = (0..counts.len()).max_by(|&a, &b| counts[a].total_cmp(&counts[b])) {
                b[u] = n;
            }
            b
        };
        Shape::new(name, bat.iter().map(|&b| b as f32 * BATTALION).collect())
    }

    /// The norm of kind `u`.
    pub fn norm_of(&self, u: usize) -> f32 {
        self.norm.get(u).copied().unwrap_or(1.0)
    }

    /// Battalions in it.
    pub fn battalions(&self) -> f32 {
        self.units.iter().sum::<f32>() / BATTALION
    }

    /// What its companies give.
    pub fn gift(&self, pack: &Pack) -> Gift {
        let mut g = Gift::NONE;
        let (mut stay, mut keep) = (1.0f32, 1.0f32);
        for &k in &self.support {
            let Some(x) = pack.support.get(k) else { continue };
            g.dig *= x.dig;
            g.rough *= x.rough;
            g.see = g.see.max(x.see);
            g.org *= x.org;
            g.eat *= x.eat;
            g.fire *= x.fire;
            stay *= 1.0 - x.saved;
            keep *= 1.0 - x.salvage;
        }
        g.saved = 1.0 - stay;
        g.salvage = 1.0 - keep;
        g
    }

    /// What its companies cost against its battalions (by the pack's worth, its kit by its norm).
    pub fn overhead(&self, pack: &Pack) -> f32 {
        let companies: f32 = self.support.iter().filter_map(|&k| pack.support.get(k)).map(|x| x.cost).sum();
        if companies <= 0.0 {
            return 0.0;
        }
        let battalions: f32 = (0..self.units.len()).map(|u| self.units[u] * worth_at(pack, u, self.norm_of(u))).sum();
        companies / battalions.max(1e-6)
    }
}

/// Worth of a unit of kind `u` with kit `k` at the pack's prices (its equipment by the kit, its
/// supply by what the kit burns, its men).
pub fn worth_at(pack: &Pack, u: usize, k: f32) -> f32 {
    let unit = &pack.units[u];
    let g = |v: &[(usize, f32)], food: bool| v.iter().filter(|x| pack.ration(x.0) == food).map(|&(i, a)| a * pack.goods[i].price).sum::<f32>();
    let war = pack.war.as_ref().unwrap();
    (g(&unit.equip, false) + g(&unit.equip, true)) * k + 6.0 * (g(&unit.supply, false) * norm::eat(k) + g(&unit.supply, true)) + unit.men * war.man_cost
}

/// Share of kind `u`'s supply that is rations (eaten by its men, whatever their kit).
pub fn ration_share(pack: &Pack, u: usize) -> f32 {
    let unit = &pack.units[u];
    let all: f32 = unit.supply.iter().map(|&(i, a)| a * pack.goods[i].price).sum();
    if all <= 0.0 {
        return 0.0;
    }
    unit.supply.iter().filter(|x| pack.ration(x.0)).map(|&(i, a)| a * pack.goods[i].price).sum::<f32>() / all
}

/// The era's soldier: the cheapest kind for its width among those standing in the line whose kit
/// the men carry (the infantry; a pack without such a kind: the cheapest of the line).
pub fn soldier_kind(pack: &Pack) -> Option<usize> {
    let cheap = |u: &usize| pack.units[*u].cost / pack.units[*u].width.max(1e-6);
    let line: Vec<usize> = (0..pack.units.len()).filter(|&u| pack.units[u].line == Line::Front).collect();
    let men: Vec<usize> = line.iter().copied().filter(|&u| pack.units[u].norms).collect();
    let pick = if men.is_empty() { &line } else { &men };
    pick.iter().copied().min_by(|a, b| cheap(a).total_cmp(&cheap(b)))
}

/// A division built around the line, from `plan` (battalions of each kind, as weights): no more
/// battalions firing from behind the line (`Line::Support`) than standing in it, and the era's
/// soldier at least `SOLDIERS` of those in it — guns and machines with nobody to hold the ground
/// are not a division (a staff whose games favour artillery and tanks built "divisions" of twenty
/// batteries and six tanks). What is taken from the others goes to the soldier.
pub fn line_first(pack: &Pack, plan: &mut [f32]) {
    let Some(soldier) = soldier_kind(pack) else { return };
    let nu = plan.len().min(pack.units.len());
    let in_line = |u: usize| pack.units[u].line == Line::Front;
    let behind = |u: usize| pack.units[u].line == Line::Support;
    let sum = |plan: &[f32], f: &dyn Fn(usize) -> bool| (0..nu).filter(|&u| f(u)).map(|u| plan[u]).sum::<f32>();
    // (x from behind the line to the soldier: behind − x = line + x)
    let (line, back) = (sum(plan, &in_line), sum(plan, &behind));
    if back > line {
        let x = (back - line) / 2.0;
        for u in (0..nu).filter(|&u| behind(u)) {
            plan[u] *= 1.0 - x / back;
        }
        plan[soldier] += x;
    }
    let line = sum(plan, &in_line);
    let others = line - plan[soldier];
    let short = (SOLDIERS * line - plan[soldier]).min(others);
    if short > 0.0 && others > 0.0 {
        for u in (0..nu).filter(|&u| in_line(u) && u != soldier) {
            plan[u] *= 1.0 - short / others;
        }
        plan[soldier] += short;
    }
}

/// What a kit `q` (against the standard) makes of a kind in a fight, fire and toughness together.
pub fn kit_eff(q: f32) -> f32 {
    if q == 1.0 {
        1.0
    } else {
        (norm::fire(q) * norm::tough(q)).sqrt()
    }
}

/// A division.
#[derive(Clone, Debug)]
pub struct Division {
    pub id: u32,
    pub side: usize,
    /// Its number in its side's army.
    pub no: u32,
    /// Its template (an index into its side's `shapes`).
    pub shape: usize,
    /// Units of each kind it has now (its men, by their places), and the equipment of each kind it
    /// carries (in standard kits: a unit at its norm carries `norm` of one).
    pub has: Vec<f32>,
    pub kit: Vec<f32>,
    /// Share of it fit to fight, 0 to 1.
    pub org: f32,
    /// The province it stands in; the provinces still ahead of it (the next first), and how far
    /// it has walked towards the next (0 to 1).
    pub at: usize,
    pub way: Vec<usize>,
    pub gone: f32,
    /// Its group of armies (NONE: the reserve).
    pub group: usize,
    /// Sent by the player himself: its general leaves it be.
    pub manual: bool,
    pub exp: f32,
    /// How far its training has gone: 0 recruits, 1 trained.
    pub drill: f32,
    pub dug: f32,
    /// What it went through in the war: units of men lost for good, equipment left behind
    /// (standard kits), days in fights.
    pub dead: f32,
    pub left: f32,
    pub days: u32,
    /// Days it still falls back after a lost fight.
    pub rest: f32,
    /// Share of its supply it gets today.
    pub supplied: f32,
    /// Whether it fought today (attacking, defending or falling back).
    pub busy: bool,
}

impl Division {
    /// Kit of kind `u` its men carry, against the standard (1).
    pub fn q(&self, u: usize) -> f32 {
        if self.has[u] > 1e-9 {
            self.kit.get(u).copied().unwrap_or(self.has[u]) / self.has[u]
        } else {
            1.0
        }
    }

    /// Its fire against a trained division's, by how far its training has gone.
    pub fn trained(&self) -> f32 {
        GREEN + (1.0 - GREEN) * self.drill
    }

    /// Its units of each kind as good as their kit makes them (a standard kit: as they are).
    pub fn eff(&self) -> Vec<f32> {
        (0..self.has.len()).map(|u| self.has[u] * kit_eff(self.q(u))).collect()
    }
}

/// A fight going on in a province.
#[derive(Clone, Debug)]
pub struct Fight {
    pub at: usize,
    /// The side that attacks; the province it first came from.
    pub side: usize,
    pub from: usize,
    pub days: u32,
    pub luck: [f32; 2],
    /// The line each side held at its widest (a side gives way when its fresh divisions hold less
    /// than `battle::HOLD` of it).
    pub held: [f32; 2],
    /// The odds the attacker's war games gave it (a half when nobody reckoned).
    pub odds: f32,
    /// Worth each side brought into it, at its first day; what each lost since; the kinds of each
    /// side then; how well each was led; how dug in the defenders were; supply of both.
    pub start: [f32; 2],
    pub lost: [f32; 2],
    pub kinds: [Vec<f32>; 2],
    pub lead: [f32; 2],
    pub dug: f32,
    pub supply: [f32; 2],
    /// Fresh strength of the attackers against both sides' today (0 to 1): the bar of the fight.
    pub balance: f32,
    /// For the record (`Fought`): divisions of each side at its widest, their equipment, their
    /// experience and their companies as it began; from how many sides it was attacked; men each
    /// side lost for good (thousands); equipment the attackers took.
    pub divs: [u32; 2],
    pub kit: [f32; 2],
    pub exp: [f32; 2],
    pub companies: [f32; 2],
    pub sides: u32,
    pub men: [f32; 2],
    pub captured: f32,
    /// Units of each kind each side lost for good in it (empty until it is fought).
    pub lost_kinds: [Vec<f32>; 2],
    /// Each side's tactic today (`TACTICS`), and the days each tactic was used.
    pub tactic: [u8; 2],
    pub tactics: [[u16; 8]; 2],
}

/// A blow a general gathers: his divisions march to `from`, and when they are there and ready
/// they attack `target`.
#[derive(Clone, Debug)]
pub struct Blow {
    pub group: usize,
    /// The province its supply is reckoned from, and all the provinces of ours round the target it
    /// gathers in (it strikes from all sides it can: the fight is wider so).
    pub from: usize,
    pub froms: Vec<usize>,
    pub target: usize,
    pub ids: Vec<u32>,
    pub since: u32,
}

/// The divisions of both sides of a war.
#[derive(Clone, Debug, Default)]
pub struct Corps {
    pub divs: Vec<Division>,
    pub next: u32,
    pub numbers: [u32; 2],
    /// Each side's templates; how much of each it builds (weights; new divisions are formed by
    /// them).
    pub shapes: [Vec<Shape>; 2],
    pub wants: [Vec<f32>; 2],
    /// What each side's economy made that no division holds yet: units of each kind (their men),
    /// and equipment of each kind (standard kits).
    pub pool: [Vec<f32>; 2],
    pub kit: [Vec<f32>; 2],
    /// Equipment each side took from the enemy (standard kits of each kind, all the war), and
    /// left behind.
    pub trophies: [Vec<f32>; 2],
    pub abandoned: [Vec<f32>; 2],
    /// What each side's staff saw it lacked, for its companies of support (a share 0 to 1 each, by
    /// `NEED`): rough ground on its front, attacks that failed, attacks stopped worn out, men
    /// lost, hunger in crowded provinces, equipment left behind.
    pub lack: [[f32; 6]; 2],
    /// This month's tallies for it (`TALLY`).
    pub tally: [[f32; 8]; 2],
    /// Units each side's economy made last month.
    pub made: [f32; 2],
    /// The month (of the divisions' days) each side's staff last gave its divisions a new pattern.
    pub reorg: [u32; 2],
    /// The game keeps its armies in divisions in peace too: `peace` — no war is fought (nobody
    /// starves or is cut off, nobody walks into the other's land: the staffs keep them in
    /// garrisons behind the border); `keep` — when a war ends they stay, back under their staffs.
    pub peace: bool,
    pub keep: bool,
    pub fights: Vec<Fight>,
    pub blows: [Vec<Blow>; 2],
    /// Days of the war gone.
    pub day: u32,
    /// What happened, for the news: (day of the war, side it concerns, words).
    pub news: Vec<(u32, usize, String)>,
    /// Each side's supply in each province today (what the way brings, before crowding).
    pub level: [Vec<f32>; 2],
    /// Whether each side's staff builds by its plan (else by the templates given it).
    pub by_plan: [bool; 2],
    /// Where each side's new divisions gather, and the group they join, if its player chose (none:
    /// the capital or the depot nearest the front; the group with the fewest for its share).
    pub muster: [Option<usize>; 2],
    pub muster_group: [Option<usize>; 2],
    /// Controls for the checks: a side's generals and staff give no orders (`idle`); nobody goes
    /// hungry (`fed`); recruits are free for their generals' orders at once (`drill_free`).
    pub idle: [bool; 2],
    pub fed: bool,
    pub drill_free: bool,
    /// How far the men of the divisions each side forms now have been trained (0: recruits, as
    /// always; a mobilisation of reservists who served before brings trained men).
    pub fresh_drill: [f32; 2],
    /// What each side's country brings to its divisions (`Mods`; neutral: as they are).
    pub mods: [Mods; 2],
    /// Units of each kind each side lost for good in the war's fights, and equipment of each kind
    /// it lost with them (standard kits).
    pub lost: [Vec<f32>; 2],
    pub lost_kit: [Vec<f32>; 2],
    /// Odds of the attacks launched today, by province attacked and side (taken by the fights they
    /// open).
    odds_for: Vec<(usize, usize, f32)>,
}

impl Corps {
    pub fn worth(pack: &Pack, has: &[f32]) -> f32 {
        has.iter().zip(&pack.units).map(|(n, u)| n * u.cost).sum()
    }

    /// A division's worth as its kit makes it (a standard kit: its worth).
    pub fn power(pack: &Pack, d: &Division) -> f32 {
        Corps::worth(pack, &d.eff())
    }

    /// Share of the equipment its norm gives its men that a division carries (1: all of it).
    pub fn equipped(&self, pack: &Pack, d: &Division) -> f32 {
        let sh = &self.shapes[d.side][d.shape];
        let e = |u: usize| equip_worth(pack, u);
        let want: f32 = (0..d.has.len()).map(|u| d.has[u] * sh.norm_of(u) * e(u)).sum();
        if want <= 1e-9 {
            return 1.0;
        }
        (0..d.has.len()).map(|u| d.kit.get(u).copied().unwrap_or(0.0) * e(u)).sum::<f32>() / want
    }

    /// What a division's companies give it.
    pub fn gift(&self, pack: &Pack, d: &Division) -> Gift {
        self.shapes[d.side].get(d.shape).map_or(Gift::NONE, |sh| sh.gift(pack))
    }

    /// The division of id `id` (its index).
    pub fn find(&self, id: u32) -> Option<usize> {
        self.divs.iter().position(|d| d.id == id)
    }

    /// The pace of a division: the slowest of its kinds on the ground (planes fly ahead).
    pub fn speed(pack: &Pack, has: &[f32]) -> f32 {
        let s = has.iter().zip(&pack.units).filter(|(n, u)| **n > 0.01 && u.line != Line::Air).map(|(_, u)| u.speed).fold(f32::INFINITY, f32::min);
        if s.is_finite() {
            s.max(0.1)
        } else {
            1.0
        }
    }

    /// Full strength of a division: the units its template holds.
    pub fn full<'a>(&'a self, d: &Division) -> &'a [f32] {
        &self.shapes[d.side][d.shape].units
    }

    /// Share of its template's worth a division has (0 to 1).
    pub fn strength(&self, pack: &Pack, d: &Division) -> f32 {
        let full = Corps::worth(pack, self.full(d));
        if full <= 0.0 {
            return 0.0;
        }
        (Corps::worth(pack, &d.has) / full).clamp(0.0, 1.0)
    }

    fn fight_at(&self, q: usize) -> Option<usize> {
        self.fights.iter().position(|f| f.at == q)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apportion_gives_all() {
        let w = [1.0, 2.0, 3.0];
        for n in 0..20 {
            assert_eq!(Campaign::apportion(&w, n, false).iter().sum::<usize>(), n);
            assert_eq!(Campaign::apportion(&w, n, true).iter().sum::<usize>(), n);
        }
        assert_eq!(Campaign::apportion(&w, 2, true), vec![0, 1, 1]);
    }

    #[test]
    fn shape_sized_is_ten() {
        let s = Shape::sized("x", &[9.0, 0.0, 2.0], 10.0);
        let b: f32 = s.units.iter().sum::<f32>() / BATTALION;
        assert!((b - 10.0).abs() < 1.5, "{b}");
    }
}
