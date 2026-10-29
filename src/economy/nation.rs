//! A general economy of one country. Groups of people have needs; sectors turn work and
//! goods into other goods. Every month the market finds the prices at which what buyers
//! offer meets what sellers bring: people buy what matters most first and buy a little
//! less of what got dearer than they are used to; sectors refill their stores of inputs
//! while it pays. Workers get a share of what their sector adds, owners get the rest.
//! Everyone spends a part of the money they hold each month, the treasury too, so money
//! never piles up in one place and never vanishes.
//!
//! Prices give way first, jobs last: a price falls at most 30% a month and rises at most
//! 1.5 times (beyond that goods are shared out — a shortage shows as queues); only what
//! lies unsold even at the lowest price makes makers cut output. Owners and the state build
//! where prices say a new job would pay, or where people suffer; idle capacity wears out
//! and sectors that lose money close. Discontent piles up month after month and breaks out
//! as unrest.
//!
//! The engine knows nothing about any era: goods, groups, sectors and resources all come
//! from the content pack.

use std::collections::VecDeque;
use crate::pack::{Need, Pack};

mod finish;
mod market;
mod army;
mod prepare;

pub use army::{LAWS, PEACE_LAW, WAR_LAW};

/// Months of spending a group keeps in hand: each month it means to spend 1/RESERVE of its
/// money (more, if that does not cover its vital needs).
const RESERVE: f32 = 3.0;
/// The same for the treasury.
pub const STATE_RESERVE: f32 = 2.0;
/// Share of a group's money left after its needs that goes into new capacity of its own
/// sectors, when one of them pays.
const INVEST_SHARE: f32 = 0.3;
/// Lowest monthly return on a new job worth building for.
const MIN_RETURN: f32 = 0.005;
/// A sector that pays grows by at most this share of its jobs per month: in full at twice
/// the lowest return, less the nearer it is to it. (Owners used to pour every idle hand
/// into the one sector that paid best; six months later it all opened at once, the market
/// drowned, the capacity rotted and the next shortage started the cycle again.)
const GROW: f32 = 0.02;
/// A sector with no jobs at all grows from this share of the people.
const SEED_JOBS: f32 = 0.001;
/// Share of the way to what it means to spend on building that the state goes in a month.
const STATE_BUILD_PACE: f32 = 0.2;
/// Share of any good on the market that all building together may take in a month.
const BUILD_SHARE: f32 = 0.15;
/// Months of inputs a sector keeps in its store.
const INPUT_STORE: f32 = 1.0;
/// Months of input purchases a sector keeps in cash; the rest goes to its owner.
const WORKING_CAPITAL: f32 = 2.0;
/// Months of sales sellers like to keep in store, and the share of the gap to it they
/// close per month (selling off a surplus, or holding output back to refill).
const STORE_MONTHS: f32 = 0.5;
const STORE_RELEASE: f32 = 0.2;
/// Share of every store lost per month (spoilage, theft, wear).
const SPOIL: f32 = 0.02;
/// Months of sales above the wanted store at which makers stop making a good that sells
/// only at the lowest price (they cut in proportion before that, and the nearer the price
/// is to the lowest, the more).
const GLUT: f32 = 2.0;
/// Share of the way to the output they aim at that makers go in a month.
const OUTPUT_PACE: f32 = 0.3;
/// Sellers keep goods rather than sell below this share of last month's price
const FLOOR: f32 = 0.7;
/// and do not raise it above this many times last month's price (beyond that, goods are
/// shared out: shortages show as queues, not as wild prices),
const CEIL: f32 = 1.5;
/// nor sell below this share of their own recent price (and never below a hundredth of the
/// usual price): what is left over at that price waits in store, and only then makers cut
/// output (prices give way first, jobs last). Against the usual price, which follows prices
/// only by 5% a month, a country whose money flowed abroad could not get its prices down to
/// what was left: bread lay in store at a price nobody could pay.
const LOWEST: f32 = 0.5;
/// Below their recent price sellers put out (price / recent price)^KEEP_SLOPE of what they brought.
const KEEP_SLOPE: f32 = 2.0;
/// How much less people buy of a good that got dearer than they are used to:
/// demand ∝ (price / usual price)^-elasticity. The usual price of a good is its starting
/// price times the level of all prices people are used to: they get used to inflation,
/// but cheap bread stays cheap next to cloth.
const ELASTIC_VITAL: f32 = 0.3;
const ELASTIC_SOFT: f32 = 0.8;
const ELASTIC_INPUT: f32 = 0.5;
/// However cheap a need is, nobody buys more than this many times of it (one does not
/// eat twice as much); beyond it only what is not vital is bought, out of comfort.
const SATED: f32 = 1.1;
/// Share of the gap between today's level of prices and the one people are used to closed
/// per month.
const GET_USED_PRICE: f32 = 0.05;
/// Weight of the past in the smoothed prices makers judge by.
const TREND_KEEP: f32 = 0.8;
/// Less than this on the market (a thousand units) counts as nothing.
const NONE: f32 = 1e-3;
const BUILD_MONTHS: u32 = 6;
/// Discontent kept from one month to the next: anger fades slowly.
const KEEP_ANGER: f32 = 0.97;
const UNREST_MONTHS: u32 = 3;
/// Share of a group still working during unrest.
const UNREST_WORK: f32 = 0.8;
/// How fast people get used to a level of hardship (share of the gap closed per month).
const GET_USED: f32 = 0.02;
/// Share of idle capacity lost per month.
const IDLE_WEAR: f32 = 0.02;
/// Capacity lost per month while a sector loses money.
const CLOSURES: f32 = 0.02;
/// Share of a group's idle people who move each month into a group that pays better and
/// lacks hands (peasants into workers, workers into clerks...).
const MOBILITY: f32 = 0.02;
/// Unemployment nobody notices.
const NORMAL_IDLE: f32 = 0.05;
const BIRTHS: f32 = 0.0008;
const DEATHS_FROM_WANT: f32 = 0.02;
/// Share of a land sector's yield that people without a job there get by working the land
/// for themselves.
const SUBSIST: f32 = 0.5;
/// Land is limited: a hand on it makes (land / hands on it)^LAND_RETURN times what it makes at
/// the usual density — more where few work it, less where it is crowded — but never more than
/// LAND_MOST times as much (one man can work only so much of it). Without it a country whose
/// fields could not feed the peasant's family starved for ever: fewer mouths, and no more bread
/// for each of them.
const LAND_RETURN: f32 = 0.5;
const LAND_MOST: f32 = 3.0;
/// Hunger drives people to the land: of a group that goes hungrier than those who work the land,
/// this share a month (times how much hungrier) leaves for the land.
const TO_LAND: f32 = 0.05;
/// Merchants take out at most this share of what sellers bring to the home market: the
/// rest stays, so the home price still shows what the good is worth at home.
pub const SHIP_MAX: f32 = 0.9;
/// The dearest a good the army orders and nobody brings can get, against its usual price.
const UNFILLED: f32 = 20.0;
/// Discontent a group feels at losing, in a month, a share of its people to the war: losing a
/// thousandth of them weighs like going a twentieth short of bread for that month.
const GRIEF: f32 = 50.0;
/// The most of a group that can serve in the army at once (unless a law of the draft says
/// otherwise: `Nation::draft_cap`).
pub const DRAFT_MAX: f32 = 0.15;
/// Laboratories: at a pay as good as what he earns elsewhere in the country, this share of a
/// learned group would come (those with a calling); for more pay, more of them — as the pay against
/// their earnings to the power `LURE` — up to `LAB_MAX` of the group. People change work slowly:
/// `LAB_PACE` of the way a month.
const CALLING: f32 = 0.03;
const LURE: f32 = 2.0;
const LAB_MAX: f32 = 0.3;
const LAB_PACE: f32 = 0.1;
/// Steps of the search for a month's prices at most (alone and in a trading world alike, so
/// a country nobody can reach lives exactly as it would alone).
pub const SEARCH_STEPS: u32 = 80;

/// The search for one good's price within a month: each step goes a share of the way (in
/// logs) towards the price at which the money offered buys what is on sale (what sellers put
/// out moves with the price too). The share halves whenever the price turns back and grows
/// again while it keeps going one way. With a fixed third of the way, a market where sellers
/// bring much more for a little more money (a store that opens above the recent price)
/// overshot both ways forever, and the month's price was wherever the search happened to stop.
#[derive(Clone, Copy)]
pub struct Seek {
    pace: f32,
    dir: f32,
}

impl Seek {
    const PACE: f32 = 1.0 / 3.0;

    pub fn new() -> Seek {
        Seek { pace: Seek::PACE, dir: 0.0 }
    }

    pub fn step(&mut self, p: f32, money: f32, on_sale: f32, floor: f32, ceil: f32) -> f32 {
        let t = (money / on_sale.max(1e-9)).clamp(floor, ceil).clamp(p * 0.5, p * 2.0);
        let dir = (t / p).ln();
        if dir * self.dir < 0.0 {
            self.pace *= 0.5;
        } else if self.dir != 0.0 {
            self.pace = (self.pace * 1.2).min(Seek::PACE);
        }
        if dir != 0.0 {
            self.dir = dir.signum();
        }
        p.powf(1.0 - self.pace) * t.powf(self.pace)
    }
}

impl Default for Seek {
    fn default() -> Seek {
        Seek::new()
    }
}

/// What of a country an enemy holds: its people (but those serving in the army), workplaces,
/// stores and money, kept apart — they neither work nor buy here — until it is freed.
#[derive(Clone)]
pub struct Away {
    /// Share of the country held.
    pub share: f32,
    land: f32,
    pop: Vec<f32>,
    cap: Vec<f32>,
    max_cap: Vec<f32>,
    money: Vec<f32>,
    cash: Vec<f32>,
    stock: Vec<f32>,
    inv: Vec<Vec<f32>>,
}

/// Moves share `f` of each of `from` to `to`.
fn shift(from: &mut [f32], to: &mut [f32], f: f32) {
    for (a, b) in from.iter_mut().zip(to.iter_mut()) {
        let x = *a * f;
        *a -= x;
        *b += x;
    }
}

/// What the map says about a country.
#[derive(Clone)]
pub struct Setup {
    pub name: String,
    pub pop: f32,
    pub dev: f32,
    pub fertility: f32,
    /// Its farmland: the millions of people it would hold at the world's usual density.
    pub land: f32,
    pub deposits: Vec<f32>,
}

#[derive(Clone)]
pub struct Nation {
    pub name: String,
    pub month: u32,
    pub pop: Vec<f32>,
    /// Jobs (millions) per sector.
    pub cap: Vec<f32>,
    pub max_cap: Vec<f32>,
    pub prod: Vec<f32>,
    /// Multiplier on a sector's output, for what-if experiments (drought, blockade).
    pub shock: Vec<f32>,
    /// Hands the land holds at the usual density (land jobs at the start, for a country as dense
    /// as the world), and the hands that worked it last month (jobs, and those feeding themselves).
    pub land: f32,
    pub tilled: f32,
    /// False: land without limit and no flight to it from hunger — the engine as it was before
    /// (for controls: `FREELAND=1` in the environment).
    pub land_limited: bool,
    /// Goods sellers hold back in store.
    pub stock: Vec<f32>,
    /// Each sector's own store of inputs.
    pub inv: Vec<Vec<f32>>,
    /// Each sector's cash for buying inputs.
    pub cash: Vec<f32>,
    pub price: Vec<f32>,
    /// The level of prices people are used to (1 = starting prices).
    pub level: f32,
    /// Prices smoothed over the last months: what makers judge their output by.
    pub trend: Vec<f32>,
    pub money: Vec<f32>,
    pub treasury: f32,
    /// Money a month the state means to spend on building.
    pub state_build: f32,
    /// Military goods bought by the state.
    pub arsenal: Vec<f32>,
    /// Share of the state's spending that goes to the army, and the tax on incomes: the pack's,
    /// until a war changes them.
    pub mil_share: f32,
    pub tax: f32,
    /// Share of what is left of the state's spending after the army that may go into building
    /// (as much as there is room to build at 1), and the sector it builds, if the government
    /// chooses it (otherwise where people suffer most, or where it pays best).
    pub build_share: f32,
    pub build_focus: Option<usize>,
    /// How far the state builds by plan rather than by what pays: at 0 only where owners leave
    /// room (hands free, the sector working full), at the pace of a sector that pays well; above
    /// 0 also where there is no room (up to what its deposits allow), at up to 1 + `plan` times
    /// that pace.
    pub plan: f32,
    /// Share of the state's spending that goes to science (paid to the learned groups the pack
    /// names, by how many each is), and what went to it last month.
    pub science_share: f32,
    pub science_money: f32,
    /// What the army buys: the share of its money for each good. Empty: the pack's military
    /// goods alike.
    pub army_wants: Vec<f32>,
    /// How many times the goods of a standard kit a unit of each kind takes the country's
    /// factories to make (a model they are still learning takes more; a dearer new model more):
    /// empty — the kit's goods, as the pack gives them.
    pub make_k: Vec<f32>,
    /// People of each group serving in the army (millions): they do not work.
    pub drafted: Vec<f32>,
    /// The law of the draft: the most of each group that may serve at once, and the share of what
    /// a man earns at work that one serving is paid (1: as at work — an army of volunteers).
    pub draft_cap: f32,
    pub draft_pay: f32,
    /// The army's career men: the share of those serving who are officers and sergeants by trade,
    /// and their pay against what they would earn at work, whatever the law of the draft (0 and 0:
    /// none — every man serving is paid by the law).
    pub cadre_share: f32,
    pub cadre_pay: f32,
    /// Money of the army's this month already promised to other orders of the state's (a navy, the
    /// mending of equipment): the army's own order sees that much less (0: none).
    pub army_side: f32,
    /// People of each group working in the laboratories (millions): they do not work elsewhere;
    /// and the monthly pay the laboratories offer one of them.
    pub scholars: Vec<f32>,
    pub lab_pay: f32,
    /// The state's economic policy beyond the tax (the game's levers; each neutral by default —
    /// the engine counts exactly as before): times the tax each group pays (empty: a flat tax);
    /// weights of who gets what the state hands back (empty: by head); the share of the state's
    /// budget paid to each sector as a subsidy (empty: none) and what each got last month; the duty
    /// on foreign goods sold here, a share of what they fetch (empty: none) and what duties brought
    /// last month; the most each good may cost, times its usual price (empty or 0: no cap — price
    /// control: above it goods are shared out, as in a shortage); the share of each sector's
    /// dividends the state takes as its owner (empty: none) and what it got; the workers' share
    /// of value added, times (a labour law); how much of their spare money owners put into new
    /// capacity, times (credit: the interest rate); what the state taxed of each group.
    pub tax_by: Vec<f32>,
    pub dole_by: Vec<f32>,
    pub subsidy: Vec<f32>,
    pub subsidised: Vec<f32>,
    pub duty: Vec<f32>,
    pub duties: f32,
    /// The share of each good's imports that came duty-free (by a trade agreement; empty: none).
    pub exempt: Vec<f32>,
    pub price_cap: Vec<f32>,
    pub public: Vec<f32>,
    pub public_money: f32,
    pub wage_k: f32,
    pub invest_k: f32,
    pub taxed: Vec<f32>,
    /// The other taxes (32nd… 35th session): a share of what each good costs the buyer, paid on top
    /// at the till (empty: none); the share of the sectors' dividends the state takes before the
    /// owners see them; a monthly share of the money each group holds (empty: none). `taxed_by`
    /// says, group by group, what of `taxed` was each tax: [income, purchases, profit, estate].
    pub sales_tax: Vec<f32>,
    pub profit_tax: f32,
    pub estate_tax: Vec<f32>,
    pub taxed_by: Vec<[f32; 4]>,
    /// What the sectors paid out to their owners last month, before the profit tax.
    pub payout: f32,
    /// The order of the land and of labour, and the people's lives (the game's levers; neutral by
    /// default): the workers' share of what each sector adds, times (empty: as the pack and the
    /// labour law say — estates leave the tiller less, land of his own more); how readily idle
    /// people move to a group that pays better, times (serfs may not leave); each group's natural
    /// growth a month, births less deaths (empty: the engine's one rate for all); deaths from want,
    /// times (relief and doctors save some of the starving).
    pub wage_by: Vec<f32>,
    pub mobility_k: f32,
    pub growth_by: Vec<f32>,
    pub famine_k: f32,
    /// What the state pays this month for its civil service — officials, teachers, doctors,
    /// police, relief of the poor — to each group (empty: none, as before): out of what is left
    /// after the army, science and building, before anything is handed back; beyond that out of
    /// the treasury's reserve, at most half of it a month. What it could pay last month.
    pub civil_by: Vec<f32>,
    pub civil_paid: f32,
    /// What the treasury paid last month (for readouts only — nothing reads them back): handed
    /// back to the people, spent on the army's goods, on building, on soldiers' pay.
    pub doled: f32,
    pub army_spent: f32,
    pub built: f32,
    pub soldiers_paid: f32,
    /// What an enemy holds of the country, kept apart.
    pub away: Option<Away>,
    pub discontent: Vec<f32>,
    /// Hardship people have got used to.
    pub used_to: Vec<f32>,
    pub unrest: Vec<u32>,
    /// While false (pre-history), discontent does not pile up.
    pub live: bool,
    building: VecDeque<(u32, usize, f32)>,
    // last month, for readouts and decisions
    /// Share of the work its hands allowed that a sector did (less for want of inputs or buyers).
    pub activity: Vec<f32>,
    /// Share of full output makers mean to make of each good (less while it lies unsold).
    pub wanted: Vec<f32>,
    /// Jobs actually worked per sector.
    pub worked: Vec<f32>,
    /// What a sector earned above the inputs it used and its wages.
    pub profit: Vec<f32>,
    /// Monthly pay of a working member of a group.
    pub pay: Vec<f32>,
    /// Goods each group took in kind from what it made, last month, and the share of each
    /// sector's output taken so.
    pub kind: Vec<Vec<f32>>,
    pub kind_share: Vec<f32>,
    /// Money each group spent on each good last month, and what it earned (before tax).
    pub spent: Vec<Vec<f32>>,
    pub income: Vec<f32>,
    /// Share of a group without work; None for groups no sector employs.
    pub unemployed: Vec<Option<f32>>,
    pub unmet: Vec<Vec<f32>>,
    pub hardship: Vec<f32>,
    /// Value added at current and at starting prices.
    pub gdp: f32,
    pub gdp_real: f32,
    pub produced: Vec<f32>,
    /// Sales per month, smoothed.
    pub sold: Vec<f32>,
    /// Goods that found no buyer at the lowest price sellers accept.
    pub unsold: Vec<f32>,
    /// Who bought each good last month: people, owners building, sectors, army, state building.
    pub bought_by: Vec<[f32; 5]>,
    /// Steps the market took to find prices last month, and how far they still moved at the end.
    pub solver: (u32, f32),
    pub events: Vec<String>,
    /// Trade, set by the world: money coming home for goods sold abroad.
    pub remit_in: Vec<f32>,
    /// Trade, after a month: goods that went abroad, foreign goods that came, foreign goods
    /// sold here, and the money that leaves for them (what they fetched less the merchants'
    /// margin).
    pub shipped: Vec<f32>,
    pub import: Vec<f32>,
    pub import_sold: Vec<f32>,
    pub remit_out: Vec<f32>,
    /// Everything on the market last month, home and foreign.
    pub offered: Vec<f32>,
    /// For goods not on the market at all last month: the money buyers here would have offered
    /// if a little of each had been on sale, at half as much again as its usual price.
    pub wished: Vec<f32>,
}

/// What is fixed for a month while the market looks for prices (alone, or with all the
/// countries it trades with at once).
pub struct Month {
    /// What sellers bring to the home market (foreign goods come with the market).
    offered: Vec<f32>,
    home: Vec<f32>,
    /// What buyers here would take of each good at usual prices, at most: how big a market
    /// merchants see where a good is not on sale at all.
    potential: Vec<f32>,
    /// Work done this month: jobs per group, jobs worked and goods made and used per sector.
    jobs: Vec<f32>,
    used: Vec<f32>,
    made: Vec<Vec<f32>>,
    used_goods: Vec<Vec<f32>>,
    /// What each sector's workers took of its output in kind: [sector][good].
    in_kind: Vec<Vec<f32>>,
    /// All they have: this month's output and the store.
    avail: Vec<f32>,
    /// Money each group means to spend before it looks at prices.
    budget: Vec<f32>,
    /// Where each group would build, and how many jobs at most.
    invest: Vec<Vec<(usize, f32)>>,
    /// Inputs and boosts each sector would buy at usual prices.
    restock: Vec<Vec<f32>>,
    mil: f32,
    /// (sector, money, jobs at most) the state builds.
    build: Option<(usize, f32, f32)>,
}

impl Month {
    /// What buyers here would take of each good at usual prices, at most.
    pub fn potential(&self) -> &[f32] {
        &self.potential
    }
    /// What home sellers bring of each good.
    pub fn brought(&self) -> &[f32] {
        &self.offered
    }
}

/// A country's trade in one month: goods that leave and goods that arrive (after the way).
#[derive(Clone)]
pub struct Trade {
    pub exports: Vec<f32>,
    pub imports: Vec<f32>,
}

impl Trade {
    pub fn none(nk: usize) -> Trade {
        Trade { exports: vec![0.0; nk], imports: vec![0.0; nk] }
    }
}

/// Money each buyer offers for each good.
struct Bids {
    total: Vec<f32>,
    group: Vec<Vec<f32>>,
    /// [group][sector][good]
    invest: Vec<Vec<Vec<f32>>>,
    sector: Vec<Vec<f32>>,
    mil: Vec<f32>,
    build: Vec<f32>,
}

fn needs_cost(pack: &Pack, g: usize, pop: f32, price: &[f32]) -> f32 {
    pack.groups[g].needs.iter().map(|n| pop * n.amount * price[n.good]).sum()
}

/// Goods a sector keeps in store: inputs, and boosts at their full rate.
fn store_needs(pack: &Pack, s: usize) -> impl Iterator<Item = (usize, f32)> + '_ {
    let sec = &pack.sectors[s];
    sec.inputs.iter().copied().chain(sec.boost.iter().map(|&(i, a, _)| (i, a)))
}

fn build_cost(pack: &Pack, s: usize, price: &[f32]) -> f32 {
    pack.sectors[s].build.iter().map(|&(i, a)| a * price[i]).sum()
}

/// The group a sector needs most of.
fn main_hands(pack: &Pack, s: usize) -> (usize, f32) {
    pack.sectors[s].workers.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap_or((0, 1.0))
}

impl Nation {
    pub fn new(pack: &Pack, s: &Setup) -> Nation {
        let (ng, ns, nk) = (pack.groups.len(), pack.sectors.len(), pack.goods.len());
        let pop: Vec<f32> = pack.groups.iter().map(|g| g.share * s.pop).collect();
        let total: f32 = pop.iter().sum();
        let prod: Vec<f32> = pack
            .sectors
            .iter()
            .map(|sec| s.dev.max(0.05).sqrt() * if sec.land { s.fertility.max(0.05).sqrt() } else { 1.0 })
            .collect();
        let max_cap: Vec<f32> = pack
            .sectors
            .iter()
            .map(|sec| match sec.deposit {
                Some(r) => s.deposits.get(r).copied().unwrap_or(0.0) * sec.per_deposit,
                None => f32::INFINITY,
            })
            .collect();
        let cap: Vec<f32> = (0..ns).map(|k| (pack.sectors[k].start * total).min(max_cap[k])).collect();
        let base: Vec<f32> = pack.goods.iter().map(|g| g.price).collect();
        let on_land = |k: usize| pack.sectors[k].land;
        let land = s.land * (0..ns).filter(|&k| on_land(k)).map(|k| pack.sectors[k].start).sum::<f32>();
        let tilled: f32 = (0..ns).filter(|&k| on_land(k)).map(|k| cap[k]).sum();

        let mut stock = vec![0.0; nk];
        let mut inv = vec![vec![0.0; nk]; ns];
        let mut cash = vec![0.0; ns];
        for (k, sec) in pack.sectors.iter().enumerate() {
            let boost: f32 = sec.boost.iter().map(|b| 1.0 + b.2).product();
            for &(j, a) in &sec.outputs {
                stock[j] += cap[k] * prod[k] * boost * a * STORE_MONTHS;
            }
            for (i, a) in store_needs(pack, k) {
                inv[k][i] = cap[k] * a * INPUT_STORE;
                cash[k] += cap[k] * a * base[i] * WORKING_CAPITAL;
            }
        }
        let money = (0..ng).map(|g| RESERVE * needs_cost(pack, g, pop[g], &base)).collect();
        let sold = stock.iter().map(|x| x / STORE_MONTHS).collect();
        Nation {
            name: s.name.clone(),
            month: 0,
            pop,
            cap,
            max_cap,
            prod,
            shock: vec![1.0; ns],
            land,
            tilled,
            land_limited: std::env::var_os("FREELAND").is_none(),
            stock,
            inv,
            cash,
            price: base.clone(),
            level: 1.0,
            trend: pack.goods.iter().map(|g| g.price).collect(),
            money,
            treasury: 0.0,
            state_build: 0.0,
            arsenal: vec![0.0; nk],
            mil_share: pack.military,
            tax: pack.tax,
            build_share: 1.0,
            build_focus: None,
            plan: 0.0,
            science_share: 0.0,
            science_money: 0.0,
            army_wants: Vec::new(),
            make_k: Vec::new(),
            drafted: vec![0.0; ng],
            draft_cap: DRAFT_MAX,
            draft_pay: 1.0,
            cadre_share: 0.0,
            cadre_pay: 0.0,
            army_side: 0.0,
            scholars: vec![0.0; ng],
            lab_pay: 0.0,
            tax_by: Vec::new(),
            dole_by: Vec::new(),
            subsidy: Vec::new(),
            subsidised: vec![0.0; ns],
            doled: 0.0,
            army_spent: 0.0,
            built: 0.0,
            soldiers_paid: 0.0,
            duty: Vec::new(),
            exempt: Vec::new(),
            duties: 0.0,
            price_cap: Vec::new(),
            public: Vec::new(),
            public_money: 0.0,
            wage_k: 1.0,
            invest_k: 1.0,
            taxed: vec![0.0; ng],
            sales_tax: Vec::new(),
            profit_tax: 0.0,
            estate_tax: Vec::new(),
            taxed_by: vec![[0.0; 4]; ng],
            payout: 0.0,
            wage_by: Vec::new(),
            mobility_k: 1.0,
            growth_by: Vec::new(),
            famine_k: 1.0,
            civil_by: Vec::new(),
            civil_paid: 0.0,
            away: None,
            discontent: vec![0.0; ng],
            used_to: vec![0.0; ng],
            unrest: vec![0; ng],
            live: true,
            building: VecDeque::new(),
            activity: vec![1.0; ns],
            wanted: vec![1.0; nk],
            worked: vec![0.0; ns],
            profit: vec![0.0; ns],
            pay: vec![0.0; ng],
            spent: vec![vec![0.0; nk]; ng],
            kind: vec![vec![0.0; nk]; ng],
            kind_share: vec![0.0; ns],
            income: vec![0.0; ng],
            unemployed: vec![None; ng],
            unmet: pack.groups.iter().map(|g| vec![0.0; g.needs.len()]).collect(),
            hardship: vec![0.0; ng],
            gdp: 0.0,
            gdp_real: 0.0,
            produced: vec![0.0; nk],
            sold,
            unsold: vec![0.0; nk],
            bought_by: vec![[0.0; 5]; nk],
            solver: (0, 0.0),
            events: Vec::new(),
            import: vec![0.0; nk],
            remit_in: vec![0.0; nk],
            shipped: vec![0.0; nk],
            import_sold: vec![0.0; nk],
            remit_out: vec![0.0; nk],
            offered: vec![0.0; nk],
            wished: vec![0.0; nk],
        }
    }

    /// Pre-history: the country lives `months` before the game starts, without unrest,
    /// so prices, capacities and habits settle instead of coming raw from the pack.
    pub fn settle(&mut self, pack: &Pack, months: u32) {
        self.live = false;
        for _ in 0..months {
            self.step(pack);
        }
        self.begin();
    }

    /// The game starts: from now on discontent piles up.
    pub fn begin(&mut self) {
        self.live = true;
        // what is being built keeps its months left, not the pre-history's calendar (or the
        // queue waits for month 245 and nothing gets built for twenty years)
        for b in &mut self.building {
            b.0 = b.0.saturating_sub(self.month);
        }
        self.month = 0;
        self.discontent.iter_mut().for_each(|d| *d = 0.0);
        self.events.clear();
    }

    /// Every sum of money in the country times `f`, and every price with it: nothing real
    /// changes (what anybody can buy, makes or owns), only what a coin is called.
    pub fn rebase(&mut self, f: f32) {
        let scale = |v: &mut Vec<f32>| v.iter_mut().for_each(|x| *x *= f);
        scale(&mut self.money);
        scale(&mut self.cash);
        scale(&mut self.price);
        scale(&mut self.trend);
        scale(&mut self.remit_in);
        scale(&mut self.remit_out);
        scale(&mut self.income);
        scale(&mut self.pay);
        scale(&mut self.profit);
        self.lab_pay *= f;
        scale(&mut self.subsidised);
        scale(&mut self.taxed);
        self.taxed_by.iter_mut().for_each(|t| t.iter_mut().for_each(|x| *x *= f));
        self.payout *= f;
        scale(&mut self.civil_by);
        self.civil_paid *= f;
        self.duties *= f;
        self.public_money *= f;
        self.doled *= f;
        self.army_spent *= f;
        self.built *= f;
        self.soldiers_paid *= f;
        self.spent.iter_mut().for_each(scale);
        self.treasury *= f;
        self.state_build *= f;
        self.level *= f;
        self.gdp *= f;
    }

    /// The workers' share of what sector `s` adds: the pack's, times the labour law (`wage_k`) and
    /// the sector's own order (`wage_by`), at most 0.95.
    pub fn wage_share(&self, pack: &Pack, s: usize) -> f32 {
        let base = pack.sectors[s].wage_share;
        let by = self.wage_by.get(s).copied().unwrap_or(1.0);
        if self.wage_k == 1.0 && by == 1.0 {
            base
        } else {
            (base * self.wage_k * by).min(0.95)
        }
    }

    /// Its people (millions).
    pub fn people(&self) -> f32 {
        self.pop.iter().sum()
    }

    /// Discontent of the people, weighed by how many each group is.
    pub fn anger(&self) -> f32 {
        let all = self.people().max(1e-9);
        self.discontent.iter().zip(&self.pop).map(|(d, p)| d * p).sum::<f32>() / all
    }

    /// All money in the country: must never change.
    pub fn money_total(&self) -> f32 {
        let home = self.money.iter().sum::<f32>() + self.treasury + self.cash.iter().sum::<f32>();
        match &self.away {
            Some(a) => home + a.money.iter().sum::<f32>() + a.cash.iter().sum::<f32>(),
            None => home,
        }
    }

    /// A month alone.
    pub fn step(&mut self, pack: &Pack) {
        let none = Trade::none(pack.goods.len());
        let mo = self.prepare(pack);
        let p = self.clear(pack, &mo, &none);
        self.finish(pack, mo, p, &none);
    }

    /// A month with the world outside held still: `trade` leaves and arrives as given, and the
    /// money for goods sold abroad is what `remit_in` holds (what a country's model of itself
    /// assumes). With no trade, the same month as alone.
    pub fn step_with(&mut self, pack: &Pack, trade: &Trade) {
        let mo = self.prepare(pack);
        let p = self.clear(pack, &mo, trade);
        self.finish(pack, mo, p, trade);
    }

    /// One line about why money moves the way it does (for tracing).
    pub fn debug_line(&self, pack: &Pack) -> String {
        let idle: Vec<String> = (0..pack.groups.len())
            .filter_map(|g| self.unemployed[g].map(|u| format!("{} {:.0}%/{:.2}", &pack.groups[g].name, u * 100.0, self.pay[g])))
            .collect();
        let short: Vec<String> = pack
            .sectors
            .iter()
            .enumerate()
            .filter(|(s, _)| self.activity[*s] < 0.95)
            .map(|(s, sec)| format!("{} {:.0}%", sec.name, self.activity[s] * 100.0))
            .collect();
        let losing: Vec<&str> =
            pack.sectors.iter().enumerate().filter(|(s, _)| self.profit[*s] < 0.0).map(|(_, sec)| sec.name.as_str()).collect();
        format!(
            "без работы/плата: {}; нет сырья: {}; в убыток: {}",
            idle.join(", "),
            if short.is_empty() { "—".into() } else { short.join(", ") },
            if losing.is_empty() { "—".into() } else { losing.join(", ") }
        )
    }

    /// The main reason a group suffers, in words.
    pub fn cause(&self, pack: &Pack, g: usize) -> String {
        let grp = &pack.groups[g];
        let worst = grp
            .needs
            .iter()
            .enumerate()
            .max_by(|(a, na), (b, nb)| (na.weight * self.unmet[g][*a]).total_cmp(&(nb.weight * self.unmet[g][*b])));
        let mut parts = Vec::new();
        if let Some((k, n)) = worst {
            if self.unmet[g][k] > 0.05 {
                parts.push(format!("не хватает: {} {:.0}%", pack.goods[n.good].name, self.unmet[g][k] * 100.0));
            }
        }
        if let Some(u) = self.unemployed[g] {
            if u > 0.15 {
                parts.push(format!("без работы {:.0}%", u * 100.0));
            }
        }
        if parts.is_empty() {
            "мелкие нехватки".into()
        } else {
            parts.join(", ")
        }
    }
}

/// Amount of good `i` a sector keeps in store per job.
/// What a hand on the land makes against the usual density: `land` hands it holds so, `tilled`
/// working it.
fn roomy(land: f32, tilled: f32) -> f32 {
    if tilled <= 0.0 {
        return LAND_MOST;
    }
    (land / tilled).powf(LAND_RETURN).min(LAND_MOST)
}

fn restock_full(pack: &Pack, s: usize, i: usize) -> f32 {
    store_needs(pack, s).filter(|&(k, _)| k == i).map(|(_, a)| a * INPUT_STORE).sum()
}

/// A pack at a glance, for modders: at starting prices and capacities of an average
/// country, how much of each good is made against how much is wanted, and how much
/// each group earns against what its needs cost.
pub struct Balance {
    /// (good, made, wanted)
    pub goods: Vec<(String, f32, f32)>,
    /// (group, income after tax, cost of needs)
    pub groups: Vec<(String, f32, f32)>,
    /// (group, jobs, people) for groups some sector employs
    pub jobs: Vec<(String, f32, f32)>,
}

pub fn balance(pack: &Pack) -> Balance {
    let (ng, nk) = (pack.groups.len(), pack.goods.len());
    let pop: Vec<f32> = pack.groups.iter().map(|g| g.share * 100.0).collect();
    let base: Vec<f32> = pack.goods.iter().map(|x| x.price).collect();
    let jobs = |s: usize| pack.sectors[s].start * 100.0;
    let mut made = vec![0.0f32; nk];
    let mut wanted = vec![0.0f32; nk];
    let mut by = vec![vec![0.0f32; nk]; pack.sectors.len()];
    for (s, sec) in pack.sectors.iter().enumerate() {
        let boost: f32 = sec.boost.iter().map(|b| 1.0 + b.2).product();
        for &(j, a) in &sec.outputs {
            by[s][j] = jobs(s) * a * boost;
            made[j] += by[s][j];
        }
        for (i, a) in store_needs(pack, s) {
            wanted[i] += jobs(s) * a;
        }
    }
    for (g, grp) in pack.groups.iter().enumerate() {
        for n in &grp.needs {
            wanted[n.good] += pop[g] * n.amount;
        }
    }
    let mut income = vec![0.0f32; ng];
    for (s, sec) in pack.sectors.iter().enumerate() {
        let revenue: f32 = (0..nk)
            .map(|j| if made[j] > 0.0 { made[j].min(wanted[j]) * by[s][j] / made[j] * base[j] } else { 0.0 })
            .sum();
        let cost: f32 = store_needs(pack, s).map(|(i, a)| jobs(s) * a * base[i]).sum();
        let va = revenue - cost;
        let wages = va.max(0.0) * sec.wage_share;
        for &(g, w) in &sec.workers {
            income[g] += wages * w;
        }
        income[sec.owner] += va - wages;
    }
    let mut places = vec![0.0f32; ng];
    for (s, sec) in pack.sectors.iter().enumerate() {
        for &(g, w) in &sec.workers {
            places[g] += jobs(s) * w;
        }
    }
    Balance {
        jobs: (0..ng).filter(|&g| places[g] > 0.0).map(|g| (pack.groups[g].name.clone(), places[g], pop[g])).collect(),
        goods: (0..nk)
            .filter(|&i| !pack.goods[i].military)
            .map(|i| (pack.goods[i].name.clone(), made[i], wanted[i]))
            .collect(),
        groups: (0..ng)
            .map(|g| (pack.groups[g].name.clone(), income[g] * (1.0 - pack.tax), needs_cost(pack, g, pop[g], &base)))
            .collect(),
    }
}
