//! War on the map: two neighbouring countries fight province by province.
//!
//! Each side's staff (the doctrine's: what it believes, what it builds) also decides each
//! month where its army stands. Half of it holds the front, each stretch as strongly as the
//! enemy it sees across it; the rest, the striking force, goes where the staff's war games —
//! with the enemy as seen through fog — give at least even-and-a-half odds: one fist where
//! good odds are rare, many small blows along the front where the enemy is weak everywhere.
//! Nobody chooses "blitzkrieg" or "broad front": the situation does.
//!
//! Troops that fought rest a month where the battle left them: an offensive pauses after each
//! blow. (Without it a group that took a province struck again the next month, and the side
//! that attacked kept attacking until a capital fell.)
//!
//! Gathering a striking force takes time and can be seen: the groups a staff places next to
//! its targets this month attack next month, and meanwhile they stand in their provinces —
//! defending them if attacked, and seen by the enemy through fog, who strengthens the front
//! facing them. What of the striking force is not placed for an attack stays back as a
//! reserve and comes to the help of the provinces the enemy attacks, an equal part to each;
//! the enemy's war games do not see it. (Groups that gathered and attacked in one month, met
//! by a defence spread evenly along the front and no reserve, took nearly every province
//! they attacked, and a thin wedge reached a capital in three months.)
//!
//! A battle is fought on the ground of the province attacked. The winner of an attack takes
//! the province; the defenders who survive fall back to a neighbouring province of their own,
//! or, with none left, are lost. Supply runs from the capital through one's own provinces:
//! the farther an army is from it, the less it gets, so a deep advance runs out of breath; an
//! army cut off from it lives on what its own land gives, or on almost nothing in a pocket
//! on enemy ground.

use std::collections::VecDeque;
use crate::military::battle::{battle, Army, Kind};
use crate::military::doctrine::Country;
use crate::pack::Pack;
use crate::rng::{hash64, Rng};
use crate::world::countries::Politics;
use crate::world::map::{Terrain, World};

mod month;
mod sectors;
mod staff;
mod supply;

/// Provinces from the capital within which an army is fully supplied, over how many more its
/// supply falls to the least, and what it is.
const SUPPLY_NEAR: f32 = 4.0;
const SUPPLY_FAR: f32 = 6.0;
const SUPPLY_LEAST: f32 = 0.2;
/// Cut off from the capital: on its own land (depots, people who feed it), and on enemy land.
const DEPOTS: f32 = 0.5;
const POCKET: f32 = 0.1;
/// Share of the army the staff keeps holding the front.
const HOLD: f32 = 0.5;
/// Odds the staff's war games must give an attack.
pub(crate) const ODDS: f32 = 0.6;
/// War games per target and size of the striking group. (At four, the smallest group that gave
/// the odds was often only the luckiest in its few games: the staff promised 95% and took 80% of
/// the living front; at eight, 92% and 82%, and a staff blind in fog loses again as it should.)
const GAMES: usize = 8;
/// A country gives up when it holds less than this share of the people it had before the war.
/// (Losing its capital it does not: the government moves to the most populous province it
/// still holds — ending a war with the capital, capitals two provinces from the border fell
/// in nine months.)
pub const CAPITULATE: f32 = 0.35;
/// How much deeper troops dig in each month they hold a province (fully in four months; a
/// province just taken starts from nothing). Without it nothing slowed a war between equals:
/// provinces of the front changed hands every half year, mountains hardly slower than plains.
pub(crate) const DIG_PACE: f32 = 0.25;
/// Share of troops cut off from the capital that melt away each month (desertion, hunger,
/// surrender): on their own land and on the enemy's. (Without it pockets dug in and held for
/// years, and a defeated country's land was a leopard's skin of islands; at a tenth and three
/// tenths, pockets of a whole army still held a year and more behind the front.)
const CUT_OFF_LOSS: [f32; 2] = [0.2, 0.4];
/// The old rule, for a control (`Campaign::pockets` false): pockets melt this much and give up
/// only to a battle.
const OLD_CUT_OFF_LOSS: [f32; 2] = [0.1, 0.3];
/// A province cut off from its side's capital gives up to the enemy around it when its garrison
/// is worth less than this share of the enemy's troops in the provinces next to it (an empty one
/// at once). (Without it the land behind a breakthrough stayed a sieve of islands for months,
/// each to be taken by a battle of its own.)
const SURRENDER: f32 = 0.25;
/// How far beyond the targets nearest to his arrow's head (or to the staff's targets) a general
/// looks for blows, in provinces.
pub(crate) const AROUND: u32 = 1;
/// What a battle teaches a general, of what he has still to learn.
pub(crate) const SEASON: f32 = 0.005;
/// Share of a blow's strength the most gifted general hides from the enemy's eyes while he
/// gathers it (the less gifted, as much less as his talent): an attack fails mostly where the
/// enemy saw the gathering and brought up his reserve (the defenders of a failed blow were twice
/// what they had been when it was planned, of one that succeeded a tenth more).
pub(crate) const HIDE: f32 = 0.6;
/// How much better troops fight under the most gifted general than under an average one (and
/// worse under the least gifted), in attack and in defence; and what experience adds, from the
/// usual. (Seeing the enemy truly did little: a blow fails mostly where the enemy's army, moved
/// anew each month, stands otherwise than when it was planned, which no general can see.)
const LEAD: f32 = 0.3;
const LEAD_SEASONED: f32 = 0.1;
/// How much more of his group a bold general throws into his blows (a cautious one, less).
pub(crate) const BOLD_PUSH: f32 = 0.3;
/// How fast a staff learns how far its war games flatter its attacks (per attack, of the gap
/// between the odds promised and what came of it).
pub(crate) const WARY: f32 = 0.05;
/// Share of its reserve a staff sends to the stretches of its front where it sees the enemy
/// gathering a strike (through fog): the rest stays back to help wherever an attack comes.
const REACT: f32 = 0.5;
/// Lines of the era's cheapest soldiers a year of both sides' budget buys, for every province
/// of the border between them. (At half a line, the troops of a province did not fill the
/// width even of a mountain front, and mountains held hardly better than plains.)
const LINES_A_YEAR: f32 = 2.0;
/// Spread of the log of the troops the staff sees in an enemy province next to its own.
pub const MAP_FOG: f32 = 0.5;
/// Sizes of the groups the striking force may split into, the smallest to take an empty
/// province. (In steps of two and more, the smallest group that gave the odds came out three
/// or four times the defenders anyway, and the cover of mountains decided nothing.)
const SPLITS: [f32; 11] = [1.0, 0.7, 0.5, 0.35, 0.25, 0.18, 0.12, 0.08, 0.05, 0.03, 0.02];

/// Provinces of the front a group of armies' sector holds, about (cut when groups are formed).
pub const SECTOR: usize = 6;
/// Divisions a general leads well (an army, as in HOI4); leading more, he leads each of them worse
/// by this share for every division over (to half at most).
pub const ARMY: usize = 24;
const OVER: f32 = 0.02;
/// Of a group told to hold: the share in the line (the rest just behind it); told to hold deep:
/// the first line (the rest in a second line behind it); told to attack: the share that keeps
/// the line while the rest strikes.
pub(crate) const LINE_HOLD: f32 = 0.85;
pub(crate) const LINE_DEEP: f32 = 0.55;
pub const LINE_ATTACK: f32 = 0.45;
/// Troops told to hold deep dig in this much faster.
pub(crate) const DIG_DEEP: f32 = 1.6;
/// Odds a general told to attack needs from his war games (a staff striking on its own wants
/// `ODDS`).
pub(crate) const ODDS_ORDERED: f32 = 0.5;
/// Share of the army a staff leading by groups keeps back as its own reserve.
const STAFF_RESERVE: f32 = 0.2;
/// A staff tells a group to hold deep where the enemy it sees across the sector outweighs what
/// the group will have in its line this many times.
const DEEP_AT: f32 = 1.3;
/// Of the troops of a group's second line (holding deep), the share that comes up to the
/// provinces of its first line attacked in a month, an equal part to each (a mobile second
/// echelon: a sector is a few days' march across).
const DEEP_HELP: f32 = 0.6;

/// What a group of armies is told to do on its sector of the front.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Order {
    /// Hold the line: most of the group along its sector, each stretch by the enemy seen across
    /// it, the rest just behind; it does not attack.
    Hold,
    /// Deep defence: a second line behind the first, nearly as strong; both dig in faster.
    Deep,
    /// Attack towards a province of the theatre (the arrow's head): blows are struck from the
    /// sector at the enemy provinces nearest the arrow's head, where the war games give the
    /// odds; the rest of the group keeps the line.
    Attack(usize),
}

/// A group of armies: the provinces of the front it was given (its anchor: each month its sector
/// is the part of the front nearest to them), its share of the side's army, its order, and — told
/// to attack — the share of its troops that strikes (the rest keeps the line); whether the staff
/// leads it (a side's staff leaves the groups the player took be); the general at its head; the
/// country of the world it faces, when not the enemy of this war (the player put it on the border
/// with another neighbour: its sector is then exactly its anchor, the provinces he gave it).
#[derive(Clone, Debug)]
pub struct Group {
    pub anchor: Vec<usize>,
    pub share: f32,
    pub order: Order,
    pub strike: f32,
    pub staff: bool,
    pub general: Option<General>,
    pub facing: Option<u32>,
}

/// A general: the same four hidden qualities as an advisor's (nobody sees them; only what he
/// does), and his record.
#[derive(Clone, Debug)]
pub struct General {
    pub name: String,
    /// How many ways of striking he weighs (with little of it, only rough ones).
    pub education: f32,
    /// Grows in battle, more slowly the more he has; it sharpens what he sees.
    pub experience: f32,
    /// How truly he judges the enemy across his sector.
    pub talent: f32,
    /// What he believes in: −1 the cautious (strikes only at good odds) … +1 the bold.
    pub bold: f32,
    /// Battles his group won and lost; provinces it took.
    pub won: u32,
    pub lost: u32,
    pub took: u32,
    /// His past, as anyone may know it: the wars he fought, how they ended, his record in each.
    pub past: Vec<String>,
}

impl General {
    /// Someone: his qualities drawn at random.
    pub fn anyone(name: String, rng: &mut Rng) -> General {
        let q = |r: &mut Rng| (0.5 + 0.22 * r.normal()).clamp(0.05, 0.95);
        let education = q(rng);
        let experience = q(rng) * 0.6;
        let talent = q(rng);
        let bold = (0.45 * rng.normal()).clamp(-1.0, 1.0);
        General { name, education, experience, talent, bold, won: 0, lost: 0, took: 0, past: Vec::new() }
    }

    /// The odds he wants from his war games where he is told to strike at `base`.
    pub(crate) fn wants(&self, base: f32) -> f32 {
        base - 0.12 * self.bold
    }

    /// How much better troops fight under him than under the average general.
    pub fn lead(&self) -> f32 {
        (1.0 + LEAD * (self.talent - 0.5) + LEAD_SEASONED * (self.experience - 0.3)).max(0.5)
    }

    /// How much better troops fight under him when he leads `n` divisions: over `ARMY`, worse for
    /// every one more (one man cannot see to them all).
    pub fn lead_of(&self, n: usize) -> f32 {
        self.lead() * (1.0 - OVER * n.saturating_sub(ARMY) as f32).max(0.5)
    }

    /// How far off (log spread) he sees the enemy across his sector: on the spot, truer than the
    /// staff far behind it, the more gifted and seasoned he is (the least gifted, hardly).
    pub(crate) fn blur(&self) -> f32 {
        MAP_FOG * (1.3 - self.talent) * (1.0 - 0.4 * self.experience)
    }
}

pub const NO: u32 = u32::MAX;

/// The ground two countries fight over.
#[derive(Clone)]
pub struct Theatre {
    /// Map province of each province of the theatre.
    pub provs: Vec<usize>,
    /// Theatre province of each map province, or NO.
    pub at: Vec<u32>,
    pub nbr: Vec<Vec<usize>>,
    pub terrain: Vec<Terrain>,
    /// Millions of people.
    pub pop: Vec<f32>,
    /// Whose it was before the war.
    pub home: Vec<usize>,
    pub capital: [usize; 2],
    /// The map countries.
    pub countries: [usize; 2],
}

impl Theatre {
    pub fn new(world: &World, pol: &Politics, a: usize, b: usize) -> Theatre {
        let mut provs = Vec::new();
        let mut home = Vec::new();
        let mut at = vec![NO; world.provinces.len()];
        let mut list: Vec<(usize, usize)> = Vec::new();
        for (side, c) in [a, b].into_iter().enumerate() {
            for p in pol.provinces_of(c) {
                if !world.provinces[p].sea {
                    list.push((p, side));
                }
            }
        }
        // provinces in no order of their side (with one side's provinces first, wherever a tie
        // was broken by the first or the last of them, one side came out ahead)
        if std::env::var("NOSHUFFLE").is_err() {
            let mut rng = Rng::new(hash64((a.min(b) * 1000 + a.max(b)) as u64));
            for i in (1..list.len()).rev() {
                let j = rng.below(i + 1);
                list.swap(i, j);
            }
        }
        for (p, side) in list {
            at[p] = provs.len() as u32;
            provs.push(p);
            home.push(side);
        }
        let nbr: Vec<Vec<usize>> = provs
            .iter()
            .map(|&p| world.provinces[p].neighbors.iter().filter_map(|&q| (at[q as usize] != NO).then_some(at[q as usize] as usize)).collect())
            .collect();
        let terrain = provs.iter().map(|&p| world.provinces[p].terrain).collect();
        let pop = provs.iter().map(|&p| pol.prov[p].pop).collect();
        let capital = [a, b].map(|c| {
            let cap = pol.countries[c].capital as usize;
            if at[cap] != NO {
                at[cap] as usize
            } else {
                provs.iter().position(|&p| pol.country_of(p) as usize == c).unwrap()
            }
        });
        Theatre { provs, at, nbr, terrain, pop, home, capital, countries: [a, b] }
    }

    /// A theatre of two countries as they hold the land now (not as the map's politics drew it):
    /// each side's provinces (map provinces), each province's people, each side's capital (a map
    /// province of its own, else its first), the map countries. (A player's country turning on
    /// another neighbour after a war took or gave land.)
    pub fn of_lands(world: &World, lands: [&[usize]; 2], pop: &[f32], capitals: [usize; 2], countries: [usize; 2]) -> Theatre {
        let mut provs = Vec::new();
        let mut home = Vec::new();
        let mut at = vec![NO; world.provinces.len()];
        let mut list: Vec<(usize, usize)> = Vec::new();
        for (side, l) in lands.iter().enumerate() {
            for &p in l.iter() {
                if !world.provinces[p].sea && at[p] == NO {
                    at[p] = 0;
                    list.push((p, side));
                }
            }
        }
        let (a, b) = (countries[0], countries[1]);
        let mut rng = Rng::new(hash64((a.min(b) * 1000 + a.max(b)) as u64 ^ 0x1A4D));
        for i in (1..list.len()).rev() {
            let j = rng.below(i + 1);
            list.swap(i, j);
        }
        for (p, side) in list {
            at[p] = provs.len() as u32;
            provs.push(p);
            home.push(side);
        }
        let nbr: Vec<Vec<usize>> = provs
            .iter()
            .map(|&p| world.provinces[p].neighbors.iter().filter_map(|&q| (at[q as usize] != NO).then_some(at[q as usize] as usize)).collect())
            .collect();
        let terrain = provs.iter().map(|&p| world.provinces[p].terrain).collect();
        let pop = provs.iter().map(|&p| pop.get(p).copied().unwrap_or(0.0)).collect();
        let capital = [0, 1].map(|s| {
            let cap = capitals[s];
            if cap < at.len() && at[cap] != NO && home[at[cap] as usize] == s {
                at[cap] as usize
            } else {
                home.iter().position(|&h| h == s).unwrap_or(0)
            }
        });
        Theatre { provs, at, nbr, terrain, pop, home, capital, countries }
    }

    /// Pairs of province neighbours across the border of two countries: how long their front is.
    pub fn border(world: &World, pol: &Politics, a: usize, b: usize) -> usize {
        pol.provinces_of(a)
            .map(|p| world.provinces[p].neighbors.iter().filter(|&&q| pol.country_of(q as usize) as usize == b).count())
            .sum()
    }
}

/// One battle, for the record.
#[derive(Clone, Debug)]
pub struct Fought {
    pub month: u32,
    pub attacker: usize,
    /// The province attacked, and the one the attack came from.
    pub province: usize,
    pub from: usize,
    pub terrain: Terrain,
    /// Worth of the attackers against the defenders, as it was; and the odds the attacker's
    /// war games gave the attack.
    pub ratio: f32,
    pub odds: f32,
    pub won: bool,
    /// Supply of the attackers and of the defenders.
    pub supply: [f32; 2],
    /// Worth each side lost in it: the attackers, the defenders.
    pub lost: [f32; 2],
    /// Whether the defenders were cut off from their capital (a pocket).
    pub cut: bool,
    /// Worth of the attackers as the blow was planned and of the defenders then (truly); of the
    /// attackers and the defenders as the battle began.
    pub plan: [f32; 4],
    /// Units of each kind on each side as the battle began; how well each was led (1: an average
    /// general or none); how dug in the defenders were.
    pub kinds: [Vec<f32>; 2],
    pub lead: [f32; 2],
    pub dug: f32,
    /// Days it lasted.
    pub days: u32,
    /// A fight of divisions (`corps`; the old battles leave these empty): divisions on each side at
    /// its widest; their equipment against what their norms give them; their experience; their
    /// companies of support (a division's mean); from how many sides it was attacked; men each side
    /// lost for good (thousands); equipment the attackers took (standard kits).
    pub divs: [u32; 2],
    pub kit: [f32; 2],
    pub exp: [f32; 2],
    pub companies: [f32; 2],
    pub sides: u32,
    pub men: [f32; 2],
    pub captured: f32,
    /// Units of each kind each side lost for good (the war of divisions; empty in the front's
    /// battles); the days each side used each tactic (`corps::TACTICS`).
    pub lost_kinds: [Vec<f32>; 2],
    pub tactics: [[u16; 8]; 2],
}

#[derive(Clone)]
pub struct Campaign {
    pub th: Theatre,
    pub owner: Vec<usize>,
    /// Troops of each side in each province: [side][province][kind].
    pub troops: [Vec<Vec<f32>>; 2],
    pub sides: [Country; 2],
    pub fog: [f32; 2],
    pub month: u32,
    pub seed: u64,
    pub fought: Vec<Fought>,
    /// The side whose capital fell, if one did.
    pub fallen: Option<usize>,
    pub log: Vec<String>,
    /// Attacks prepared for next month: (side, from, target, group, the odds its war games
    /// gave).
    pub staged: Vec<(usize, usize, usize, Vec<f32>, f32)>,
    /// Worth of the defenders of each staged blow's target when it was planned (for the record).
    pub staged_def: Vec<f32>,
    /// Worth of each side's gathered blows in each province that its generals keep hidden from
    /// the enemy's eyes (this month).
    pub hidden: [Vec<f32>; 2],
    /// Each side's reserve behind the front.
    pub reserve: [Vec<f32>; 2],
    /// How dug in each side is in each province, 0 to 1.
    pub dug: [Vec<f32>; 2],
    /// Province-months on the front, and provinces taken, by terrain.
    pub exposed: [f32; 11],
    pub taken: [f32; 11],
    /// Units a side's economy made this month (set from outside before a step; without them
    /// its staff's budget makes them), and the share of its troops' supply its stores give.
    pub made: [Option<Vec<f32>>; 2],
    pub stores: [f32; 2],
    /// Months of the troops' supply the stores still hold after this month's (unbounded without an economy), and the
    /// least a side must be left with by raising a division (`cover_least`; 0 unless set).
    pub cover: [f32; 2],
    pub cover_least: f32,
    /// How each side leads its troops: None — its staff over the whole front, as one; Some — by
    /// groups of armies on sectors of the front.
    pub command: [Option<Vec<Group>>; 2],
    /// Whether the side's staff gives its groups their shares and orders each month (else they
    /// stay as given).
    pub staff_led: [bool; 2],
    /// The most groups of armies a side forms (`by_armies`): a great army has bigger groups, not
    /// more of them (unbounded unless set).
    pub most_groups: usize,
    /// How many divisions the staffs want on a province of the front, against the usual (`DEPTH`),
    /// and the most battalions in a division of theirs: fewer and bigger divisions where set
    /// (1 and `corps::MOST` unless set).
    pub depth_k: f32,
    pub most_battalions: f32,
    /// Whether the staffs build their divisions around the line (`corps::line_first`): no more
    /// guns behind it than men and machines in it, the era's soldier the core of those (off
    /// unless set: a division is then the plan's make-up as it is).
    pub line_first: bool,
    /// The most divisions a side keeps for each province of its front: what its factories make
    /// beyond them waits in its stores and refills the divisions it has (unbounded unless set).
    pub most_per_front: f32,
    /// The share of its troops' supply the stores must have given last month for a side to raise new divisions (a staff
    /// does not raise what it cannot feed: the army grows to what the economy can supply, no further; 0 unless set).
    pub feed_least: f32,
    /// Whether an army takes no more divisions than a general leads well (`ARMY`), however many the
    /// side has (the rest stay in the staff's reserve): false — a side forming no more than
    /// `most_groups` armies swells them to hold all its divisions (the old rule).
    pub fixed_armies: bool,
    /// The most divisions each side's government keeps for a province of its front now (its
    /// establishment: smaller in peace than `most_per_front`, filled at a mobilisation; unbounded
    /// unless set).
    pub establish: [f32; 2],
    /// How wide a province of this map is against the usual one (28th session: the game's world
    /// has three times as many, each 0.58 as wide): the march between neighbours, the supply's
    /// reach from the capital, what a province feeds and the stretches of the old front are
    /// reckoned in true distance by it (1 unless set: in provinces, as before).
    pub scale: f32,
    /// Provinces where a side's troops hold deep (they dig in faster).
    pub deep: [Vec<bool>; 2],
    /// Provinces each side lost to its pockets giving up, since the war began; province-months
    /// each side held cut off from its capital.
    pub surrendered: [u32; 2],
    pub cut_off: [f32; 2],
    /// Supply depots each side has built (provinces of the theatre): the way of supply starts
    /// there too, as long as the depot is its own and can be reached from its capital.
    pub hubs: [Vec<usize>; 2],
    /// The lines of supply laid through the theatre (railways, roads): each province's grade —
    /// what a step costs between two provinces that both lie on lines (the dearer of the two;
    /// a step off a line costs 1). Empty: no lines (every step costs 1, as before them).
    pub grade: Vec<f32>,
    /// Whether pockets give up to the enemy around them (false: the old rule, a control;
    /// `OLDPOCKETS=1` for every campaign).
    pub pockets: bool,
    /// The war of divisions (as in HOI4), when the war is fought by divisions day by day (the game
    /// does); None: the month's war of the front's model.
    pub corps: Option<crate::military::corps::Corps>,
}

fn add(a: &mut [f32], b: &[f32], k: f32) {
    for (x, y) in a.iter_mut().zip(b) {
        *x += y * k;
    }
}

impl Campaign {
    /// The war begins. Each side spends a month's `budget` (shares of `money` between them),
    /// has a year of it in its army already, and stands along its border, evenly.
    pub fn new(pack: &Pack, th: Theatre, sides: [Country; 2], money: [f32; 2], seed: u64) -> Campaign {
        let mut c = Campaign::blank(pack, th, sides, seed);
        // a month's budget: `LINES_A_YEAR` of the era's cheapest soldiers for every province of
        // the border between them in a year, shared by what each can pay
        let war = pack.war.as_ref().expect("в наборе нет раздела [war]");
        let cheap = pack.units.iter().filter(|u| u.line == crate::pack::Line::Front).map(|u| u.cost / u.width).fold(f32::INFINITY, f32::min);
        let border = (c.front(0).len() + c.front(1).len()) as f32 / 2.0;
        let year = LINES_A_YEAR * border * war.width * cheap;
        let all = money[0] + money[1];
        for s in 0..2 {
            c.sides[s].budget = 2.0 * year / 12.0 * money[s] / all.max(1e-9);
            for _ in 0..12 {
                c.sides[s].produce();
            }
        }
        c.place();
        c
    }

    /// The war begins between armies already raised: each stands along its border, evenly.
    pub fn raised(pack: &Pack, th: Theatre, sides: [Country; 2], seed: u64) -> Campaign {
        let mut c = Campaign::blank(pack, th, sides, seed);
        c.place();
        c
    }

    /// Each side's army stands along its border, evenly.
    pub fn place(&mut self) {
        for s in 0..2 {
            let front = self.front(s);
            let army = self.sides[s].army.count.clone();
            for &p in &front {
                add(&mut self.troops[s][p], &army, 1.0 / front.len() as f32);
            }
        }
    }

    fn blank(pack: &Pack, th: Theatre, sides: [Country; 2], seed: u64) -> Campaign {
        let n = th.provs.len();
        let nu = pack.units.len();
        let owner = th.home.clone();
        Campaign {
            th,
            owner,
            troops: [vec![vec![0.0f32; nu]; n], vec![vec![0.0f32; nu]; n]],
            sides,
            fog: [MAP_FOG; 2],
            month: 0,
            seed,
            fought: Vec::new(),
            fallen: None,
            log: Vec::new(),
            staged: Vec::new(),
            staged_def: Vec::new(),
            hidden: [vec![0.0; n], vec![0.0; n]],
            reserve: [vec![0.0f32; nu], vec![0.0f32; nu]],
            exposed: [0.0; 11],
            taken: [0.0; 11],
            dug: [vec![0.0; n], vec![0.0; n]],
            made: [None, None],
            stores: [1.0; 2],
            cover: [f32::INFINITY; 2],
            cover_least: 0.0,
            command: [None, None],
            staff_led: [false; 2],
            most_groups: usize::MAX,
            depth_k: 1.0,
            most_battalions: crate::military::corps::MOST,
            line_first: false,
            most_per_front: f32::INFINITY,
            fixed_armies: false,
            feed_least: 0.0,
            establish: [f32::INFINITY; 2],
            scale: 1.0,
            deep: [vec![false; n], vec![false; n]],
            surrendered: [0; 2],
            cut_off: [0.0; 2],
            hubs: [Vec::new(), Vec::new()],
            grade: Vec::new(),
            pockets: std::env::var("OLDPOCKETS").is_err(),
            corps: None,
        }
    }

    /// Provinces of side `s` that touch the enemy's.
    pub fn front(&self, s: usize) -> Vec<usize> {
        (0..self.owner.len()).filter(|&p| self.owner[p] == s && self.th.nbr[p].iter().any(|&q| self.owner[q] != s)).collect()
    }

    /// Share of its people before the war each side still holds.
    pub fn people(&self) -> [f32; 2] {
        [0, 1].map(|s| {
            let had: f32 = (0..self.owner.len()).filter(|&p| self.th.home[p] == s).map(|p| self.th.pop[p]).sum();
            let has: f32 = (0..self.owner.len()).filter(|&p| self.owner[p] == s && self.th.home[p] == s).map(|p| self.th.pop[p]).sum();
            has / had.max(1e-9)
        })
    }

    /// Provinces each side holds.
    pub fn held(&self) -> [usize; 2] {
        [self.owner.iter().filter(|&&o| o == 0).count(), self.owner.iter().filter(|&&o| o == 1).count()]
    }
}
