//! The game's state and how it goes: the world, the player's decisions, a day, a month.

use std::sync::Arc;
use strategy::economy::advisor::{Economist, Mind};
use strategy::economy::science::{Lab, Scientist};
use strategy::military::doctrine::Country;
use strategy::military::front::{self as front, Campaign, General, Order, Theatre};
use strategy::military::homefront::{pick, setup_of, trading_with, Homefront};
use strategy::pack::Pack;
use strategy::rng::{hash64, Rng};
use strategy::world::countries::{self, Politics};
use strategy::world::history::{History, PastWar};
use strategy::world::map::{self, World};
use strategy::world::{names, render};

use crate::army::templates::{self, Template, MOST, WANT};
use crate::words::group_name;
use crate::{ALONE, DAYS, MONTHS, PREWAR, TOGETHER};

/// How far a country's productivity may stand from the era's usual by what it knows and what its
/// past economists did (the log: ±0.25 — about −22% … +28%).
const PROD_GAP: f32 = 0.25;

/// The most groups of armies a side of the game forms (the keys 1–9 choose them): a great army has
/// bigger groups, not more.
pub const MOST_GROUPS: usize = 9;
/// The staffs of the game want a third as many divisions on a province of the front as the war's
/// suites reckon (two on a province held, about three in deep defence), divisions of up to forty
/// battalions, built around the line (`corps::line_first`): a great army is some hundred big
/// divisions, not three hundred small ones.
pub const DEPTH_K: f32 = 0.35;
pub const MOST_BATTALIONS: f32 = 40.0;
/// The most divisions a side of the game keeps for a province of its front (`Campaign::most_per_
/// front`): an army beyond that keeps its equipment in stores, for refilling.
pub const MOST_PER_FRONT: f32 = 4.0;

mod apply;
mod war;

/// One month of the chronicle: each side's output, army, discontent and people it holds.
#[derive(Clone)]
pub struct Row {
    pub month: u32,
    pub gdp: [f32; 2],
    pub army: [f32; 2],
    pub anger: [f32; 2],
    pub held: [f32; 2],
    /// The player's army by kind of troops.
    pub mix: Vec<f32>,
    /// Each side's treasury and what it took and paid this month; the government's standing, its
    /// legitimacy, the country's unrest; research (years of an ordinary country's science a month);
    /// the price level; the moods of the player's forces of society.
    pub book: [Book; 2],
    pub stability: [f32; 2],
    pub legit: [f32; 2],
    pub unrest: [f32; 2],
    pub science: [f32; 2],
    pub level: [f32; 2],
    pub moods: [f32; strategy::world::polity::BLOCS],
}

/// What a treasury took and paid in a month: taxes, duties, its own trades' dividends; handed
/// back, the army's goods, soldiers' pay, science, building, subsidies; the rest (programmes,
/// loans and their interest, the press, events: what the month changed beyond these); what was
/// left at its end.
#[derive(Clone, Copy, Debug, Default)]
pub struct Book {
    pub tax: f32,
    pub duty: f32,
    pub sectors: f32,
    pub dole: f32,
    pub army: f32,
    pub soldiers: f32,
    pub science: f32,
    pub build: f32,
    pub subsidy: f32,
    /// The civil budget (officials, schools, doctors, police, relief) and the bonds at home
    /// (interest and repayment).
    pub civil: f32,
    pub bonds: f32,
    pub other: f32,
    pub treasury: f32,
}

impl Book {
    /// All it took in.
    pub fn income(&self) -> f32 {
        self.tax + self.duty + self.sectors + self.other.max(0.0)
    }

    /// All it paid out.
    pub fn spent(&self) -> f32 {
        self.dole + self.army + self.soldiers + self.science + self.build + self.subsidy + self.civil + self.bonds + (-self.other).max(0.0)
    }
}

/// A line of the news: the month, what happened in words, the numbers for the note on hover,
/// whether it is yours and matters, the sphere it belongs to (the panel it opens).
#[derive(Clone)]
pub struct Note {
    pub month: u32,
    pub text: String,
    pub tip: String,
    pub hot: bool,
    pub tab: &'static str,
}

impl Note {
    /// A line as the engine words it: "мес 12: волнения — крестьяне (не хватает: хлеб 45%)" →
    /// the words, and what is in brackets for the note.
    pub fn new(month: u32, line: &str, hot: bool, tab: &'static str) -> Note {
        let line = strip_month(line);
        let (text, tip) = match (line.find(" ("), line.ends_with(')')) {
            (Some(k), true) => (line[..k].to_string(), line[k + 2..line.len() - 1].to_string()),
            _ => (line.to_string(), String::new()),
        };
        Note { month, text, tip, hot, tab }
    }

    pub fn with_tip(mut self, tip: String) -> Note {
        self.tip = tip;
        self
    }

}

/// "мес  12: волнения — …" → "волнения — …" (the engine's lines carry their month).
pub fn strip_month(s: &str) -> &str {
    match s.strip_prefix("мес") {
        Some(rest) => match rest.split_once(':') {
            Some((n, t)) if n.trim().chars().all(|c| c.is_ascii_digit()) => t.trim_start(),
            _ => s,
        },
        None => s,
    }
}

#[derive(Clone)]
pub struct Game {
    pub pack: Arc<Pack>,
    pub world: Arc<World>,
    pub h: Homefront,
    pub me: usize,
    pub frame: (usize, usize, usize, usize),
    pub border: f32,
    pub gdp0: [f32; 2],
    pub rows: Vec<Row>,
    pub news: Vec<Note>,
    pub seed: u64,
    /// People who could run your economy (their qualities hidden), and which of them does.
    pub pool: Vec<(String, Mind)>,
    pub adviser: Option<usize>,
    /// Who runs the other country's economy.
    pub foe_adviser: String,
    /// People who could head your laboratories, and which of them does; who heads the other's.
    pub scholars: Vec<(String, Mind)>,
    pub scholar: Option<usize>,
    pub foe_scholar: String,
    /// The clock: paused or running, at which speed (1–5), the day of the month (from 0); why it
    /// paused by itself, if it did; a number that changes whenever there is something new to show.
    pub paused: bool,
    pub speed: usize,
    pub day: u32,
    pub alert: Option<String>,
    pub version: u64,
    /// A number that changes whenever the world or the player's decisions change (not every day).
    pub stamp: u64,
    /// The months your economist and your scientist were appointed.
    pub since: [u32; 2],
    /// Generals who could take one of your groups instead of its own.
    pub spare: Vec<General>,
    /// The pack the world was built from (a new world is built from it again).
    pub path: String,
    /// The player's division templates (they give the army's plan when he decides what it builds).
    pub templates: Vec<Template>,
    /// Bots playing both sides, the game written down (`play --auto`).
    pub auto: Option<crate::tools::auto::Auto>,
    /// The world's past: its wars, newest last (anyone may read it); each side's generals of its
    /// last wars still in service; regions each side lost in them and the years.
    pub past: Vec<PastWar>,
    pub veterans: [Vec<General>; 2],
    pub lost: [Vec<(usize, u32)>; 2],
    /// The player's tax and army's share before the war (the peace gives them back).
    pub prewar: Option<(f32, f32)>,
    /// Each side's law of the draft, if chosen (`draft::LAWS`; none: the custom one — the bot's
    /// always).
    pub draft: [Option<usize>; 2],
    /// Claims, truces, tension, stability, depots, the state's building list.
    pub state: crate::world::statecraft::Statecraft,
    /// Events: the player's waiting for his choice; when each side had its last.
    pub ev: crate::country::events::Events,
    /// The portrait of each person met (a name — a file of `ui/portraits`), none twice while
    /// there are portraits left.
    pub faces: std::cell::RefCell<std::collections::HashMap<String, String>>,
    /// The countries of the map (name, colour) and the country of each map province (u32::MAX:
    /// none) — as the world's past left them (for the window of the game).
    pub countries: Vec<(String, [u8; 3])>,
    pub country_of: Vec<u32>,
    /// The state (region) each province of the world lies in, as the world was made (the map
    /// draws their borders, as HOI4's states; 28th session).
    pub state_of: Vec<u32>,
    /// The towns (province, name, size 1–3); each province's people (millions) and development.
    pub towns: Vec<(usize, String, u8)>,
    pub prov_people: Vec<f32>,
    pub prov_dev: Vec<f32>,
    /// Each province's deposits: the pack's resources found there.
    pub prov_res: Vec<Vec<usize>>,
    /// Every decision of the player, with the month and day it was taken (a save is these).
    pub journal: Vec<(u32, u32, crate::tools::saves::Act)>,
    /// Each province's region; the regions the world's past took from a country and another holds
    /// now: (region, the country that lost it, how many years before the game).
    pub prov_state: Vec<u32>,
    pub lost_lands: Vec<(u32, u32, u32)>,
    /// The land as people made it: rivers, villages, roads, railways (`land`).
    pub land: Arc<crate::world::land::Land>,
    /// Each country's capital (a map province; usize::MAX: none).
    pub capitals: Vec<usize>,
    /// The map country the player chose to play at the start (none: the pair the map made).
    pub chosen: Option<usize>,
    /// Every country's staff, generals and lost regions as the world's past left them (a new
    /// rival comes with its own).
    pub staffs: Vec<Country>,
    pub past_veterans: Vec<Vec<General>>,
    pub past_lost: Vec<Vec<(usize, u32)>>,
    /// What each country is like (its features, grown from what the world made of it).
    pub spirits: crate::country::spirits::Spirits,
    /// The state's economic policy: the order of the economy, programmes, loans.
    pub policy: crate::country::policy::Policy,
    /// Each side's lines of production: models, the factories' skill, new models offered.
    pub arms: crate::army::arms::Arms,
    /// What each side's features, law and programmes do together (this month).
    pub effs: [crate::country::spirits::Eff; 2],
    /// The world's people (all countries'), as the past left them and the game goes on; the persons
    /// behind the game's posts.
    pub folk: strategy::world::folk::Folk,
    pub roster: crate::country::folk::Roster,
    /// What each map country knows of the era's techniques; the game's side of research.
    pub know: Vec<strategy::world::progress::Know>,
    pub research: crate::country::research::Research,
    /// The state's finances beyond the tax (duties, taxes by group, credit, the press…).
    pub fiscal: crate::country::fiscal::Fiscal,
    /// The bots' courses: what each government means to do for the next years.
    pub courses: crate::country::course::Courses,
    /// Each treasury as the month began (the ledger counts what the month did to it).
    pub t0: [f32; 2],
    /// How every country is governed and what its people believe (as two hundred years made it,
    /// and as the game goes on); the game's side of it (reforms, the iron hand, favours,
    /// elections); the countries' great tasks.
    pub polity: strategy::world::polity::Polities,
    pub realm: crate::country::realm::Realm,
    pub agenda: crate::country::agenda::Agenda,
    /// What the countries think of each other, their pacts, the wars of the world (as four hundred
    /// years made them, and as the game goes on); the game's side of it.
    pub diplo: strategy::world::diplomacy::Diplomacy,
    pub rel: crate::world::relations::Relations,
    /// The army's money as a ministry of war spends it: the depots, the reservists, the
    /// establishments, the mobilisations, the books, the war's cost (`army::budget`).
    pub mil: crate::army::budget::Mil,
    /// Both navies, the seas and the trade through them (`navy`).
    pub navy: crate::navy::Navy,
    /// What each people thinks of the war; the war loans (`country::war_mood`).
    pub war_mood: crate::country::war_mood::WarMood,
    /// The civil budget and what it bought; the people's births, deaths and epidemics; the banks and
    /// the state's debt at home.
    pub civil: crate::country::civil::Civil,
    pub demo: crate::country::demography::Demo,
    pub credit: crate::country::credit::Credit,
}

impl Game {
    pub fn new(pack: Pack, seed: u64, me: usize) -> Game {
        Game::new_as(pack, seed, me, None)
    }

    /// A new game; `country` — the map country the player chose to play (its rival is the
    /// neighbour of comparable strength it shares the longest border with, or its longest border
    /// if none is comparable), else the pair the map makes and the player's side of it.
    pub fn new_as(pack: Pack, seed: u64, me: usize, country: Option<usize>) -> Game {
        let world = map::generate(&crate::map_params(seed)).0;
        let mut pol: Politics = countries::generate(&world, seed, 30, &pack);
        // the world's past: a hundred years of wars moved its borders (the pair that would have
        // played before it, if no other pair is left after it)
        let first = pick(&world, &pol).expect("на карте нет пары соседей сравнимой силы");
        let history = History::of(&pack, &world, &mut pol, seed);
        let setups: Vec<_> = (0..pol.countries.len()).map(|c| setup_of(&pol, c)).collect();
        // what each country knows makes its trades more or less productive than the era's usual
        // (the world's mean is the usual: the pack's figures were reckoned for it); its past
        // economists' work too
        let weights: Vec<f32> = (0..pol.countries.len()).map(|c| pol.pop(c)).collect();
        let mean = strategy::world::progress::mean_sector_gain(&pack, &history.know, &weights);
        let prod_k: Vec<Vec<f32>> = (0..pol.countries.len())
            .map(|c| {
                let g = history.know[c].sector_gain(&pack);
                let econ = history.folk.econ.get(c).copied().unwrap_or(0.0).clamp(-0.3, 0.3);
                // (within a quarter of the usual either way: a backward country makes less, but its
                // people do not starve for its science alone)
                (0..pack.sectors.len()).map(|k| (g[k] - mean[k] + econ).clamp(-PROD_GAP, PROD_GAP).exp()).collect()
            })
            .collect();
        let tw = trading_with(&pack, &world, &pol, &setups, ALONE, TOGETHER, &prod_k);
        let chosen = country.filter(|&c| c < pol.countries.len() && !pol.neighbors(c).is_empty());
        // the pair: neighbours whose economies (as the world's economy makes them — its land, its
        // techniques — not the map's people alone) differ less than twice
        let gdp = |c: usize| tw.nations.get(c).map_or(0.0, |n| n.gdp.max(1e-6));
        let (pair, me) = match chosen {
            Some(c) => {
                let len = |b: usize| Theatre::border(&world, &pol, c, b);
                let fair = |b: usize| {
                    let (ga, gb) = (gdp(c), gdp(b));
                    ga.max(gb) <= 2.0 * ga.min(gb)
                };
                let nb: Vec<usize> = pol.neighbors(c);
                let rival = nb.iter().copied().filter(|&b| fair(b)).max_by_key(|&b| len(b)).or_else(|| nb.iter().copied().max_by_key(|&b| len(b))).unwrap();
                ((c, rival), 0)
            }
            // (the map's own pair, as the map's people make them: what the clock's and the window's
            // self-checks are reckoned on)
            None => (pick(&world, &pol).unwrap_or(first), me),
        };
        // a grudge: the other holds land the past took from it
        let holds = |a: usize, b: usize| history.lost[a].iter().any(|&(st, _)| pol.states[st].country as usize == b);
        let grudge = [holds(pair.0, pair.1), holds(pair.1, pair.0)];
        let th = Theatre::new(&world, &pol, pair.0, pair.1);
        // the two staffs as their wars left them
        let a: Country = history.staffs[pair.0].clone();
        let b: Country = history.staffs[pair.1].clone();
        let mut h = Homefront::new(&pack, tw, [pair.0, pair.1], Campaign::raised(&pack, th, [a, b], seed));
        // three people who could run your economy, and the one who runs the other country's
        let mut rp = Rng::new(hash64(seed ^ 0xAD1));
        let pool: Vec<(String, Mind)> = (0..3).map(|_| (names::person(&mut rp), Mind::anyone(&mut rp))).collect();
        let foe_adviser = names::person(&mut rp);
        let foe_mind = Mind::anyone(&mut rp);
        // science: both states give it the custom share; the other's laboratories have a head
        let scholars: Vec<(String, Mind)> = (0..3).map(|_| (names::person(&mut rp), Mind::anyone(&mut rp))).collect();
        let foe_scholar = names::person(&mut rp);
        if let Some(science) = &pack.science {
            for s in 0..2 {
                h.home_mut(s).science_share = science.share;
                h.labs[s] = Some(Lab::new(&pack, hash64(seed ^ 0x1AB ^ s as u64)));
            }
        }
        // the years before the game: both countries keep an army in peace, which their own
        // economies build and their staffs shape by exercises; their laboratories work; the people
        // get used to the times. (Without them both came into the game with no army at all, and
        // the people's discontent rose from nothing over the first years.) The calendar of the
        // game begins after them. (Both live by the peace law of the draft.)
        for s in 0..2 {
            h.home_mut(s).set_law(strategy::economy::nation::PEACE_LAW);
        }
        for _ in 0..PREWAR {
            h.peace(&pack);
        }
        h.peace_months = 0;
        h.econ[1 - me] = Some(Economist::new(foe_mind, hash64(seed ^ 0xB07), &pack));
        if pack.science.is_some() {
            h.scientists[1 - me] = Some(Scientist::new(Mind::anyone(&mut rp), hash64(seed ^ 0x5C7), &pack));
        }
        let frame = render::frame(&world, &h.war.th);
        // (the border's length in the usual provinces: the game's are smaller — `prov_scale`)
        let border = (h.war.front(0).len() + h.war.front(1).len()) as f32 / 2.0 * crate::prov_scale();
        let gdp0 = [h.home(0).gdp_real, h.home(1).gdp_real];
        let towns = crate::world::land::towns(&world, &pol, seed);
        // the land's ways; its railways are lines of supply in a war (both sides use those they
        // hold), in the ages that have them
        let people: Vec<f32> = pol.prov.iter().map(|x| x.pop).collect();
        let land = Arc::new(crate::world::land::Land::of(&world, &towns, &people, pack.theme != "parchment"));
        h.war.grade = crate::world::land::rail_grades(&world, &h.war.th, &land);
        h.war.most_groups = MOST_GROUPS;
        // (the norms per province of the front, for the game's smaller provinces: so many to a
        // usual province's width — 28th session)
        h.war.scale = crate::prov_scale();
        h.war.depth_k = DEPTH_K * crate::prov_scale();
        h.war.most_battalions = MOST_BATTALIONS;
        h.war.line_first = true;
        h.war.most_per_front = MOST_PER_FRONT * crate::prov_scale();
        let mut g = Game {
            pack: Arc::new(pack),
            world: Arc::new(world),
            h,
            me,
            frame,
            border,
            gdp0,
            rows: Vec::new(),
            news: Vec::new(),
            seed,
            pool,
            adviser: None,
            foe_adviser,
            scholars,
            scholar: None,
            foe_scholar,
            paused: true,
            speed: 3,
            day: 0,
            alert: None,
            version: 0,
            stamp: 0,
            since: [0, 0],
            spare: Vec::new(),
            path: String::new(),
            templates: Vec::new(),
            auto: None,
            veterans: [history.veterans[pair.0].clone(), history.veterans[pair.1].clone()],
            lost: [history.lost[pair.0].clone(), history.lost[pair.1].clone()],
            past: history.wars,
            prewar: None,
            draft: [None, None],
            state: crate::world::statecraft::Statecraft { grudge, stability: [0.8, 0.8], ..Default::default() },
            ev: Default::default(),
            faces: Default::default(),
            countries: pol.countries.iter().map(|c| (c.name.clone(), c.color)).collect(),
            country_of: (0..pol.prov.len()).map(|p| pol.country_of(p)).collect(),
            state_of: pol.prov.iter().map(|x| x.state).collect(),
            towns,
            prov_people: pol.prov.iter().map(|x| x.pop).collect(),
            prov_dev: pol.prov.iter().map(|x| x.dev).collect(),
            prov_res: pol.prov.iter().map(|x| x.res.iter().enumerate().filter(|(_, &a)| a > 0.0).map(|(r, _)| r).collect()).collect(),
            journal: Vec::new(),
            prov_state: pol.prov.iter().map(|x| x.state).collect(),
            lost_lands: history
                .lost
                .iter()
                .enumerate()
                .flat_map(|(c, l)| l.iter().map(move |&(st, year)| (st as u32, c as u32, year)))
                .filter(|&(st, c, _)| pol.states[st as usize].country != c)
                .collect(),
            land,
            capitals: pol.countries.iter().map(|c| if c.capital == map::NONE { usize::MAX } else { c.capital as usize }).collect(),
            chosen,
            staffs: history.staffs,
            past_veterans: history.veterans,
            past_lost: history.lost,
            spirits: Default::default(),
            policy: Default::default(),
            arms: Default::default(),
            effs: Default::default(),
            folk: history.folk,
            roster: Default::default(),
            know: history.know,
            research: Default::default(),
            fiscal: Default::default(),
            courses: Default::default(),
            t0: [0.0; 2],
            polity: history.polity,
            realm: Default::default(),
            agenda: Default::default(),
            diplo: history.diplo,
            rel: Default::default(),
            mil: Default::default(),
            navy: Default::default(),
            war_mood: Default::default(),
            civil: Default::default(),
            demo: Default::default(),
            credit: Default::default(),
        };
        // the world's people at the game's posts: the player's choices of advisers, the rival's
        // ministers (the player's country's government is his)
        let mine = g.h.war.th.countries[me];
        g.folk.player = Some(mine);
        g.people_start();
        g.realm_start();
        g.civil_start();
        g.credit_start();
        // (the people's births and deaths are known from the first day)
        g.demography_apply();
        g.relations_start();
        for s in 0..2 {
            g.research.base[s] = g.h.labs[s].as_ref().map_or(0.0, |l| l.power);
        }
        g.templates = templates::starters(&g.pack);
        // the staffs choose the kit of their army by what it costs them; the armies stand in
        // divisions from the first day
        for s in 0..2 {
            g.h.war.sides[s].norm_free = true;
        }
        g.muster();
        // the army as a government keeps it: its establishment, the rest in the depots
        g.mil_start();
        g.peace_groups();
        // what each country is like as the game begins; its lines of production
        g.spirits_month();
        for s in 0..2 {
            g.effs[s] = g.total_eff(s);
        }
        g.arms_start();
        // the seas, the ports and the fleets the years before built
        g.navy_start();
        g.course_month();
        g.record();
        g
    }

    /// The portrait of `name` (a general, or someone in civil dress), the same for him all the
    /// game, nobody else's while there are portraits left; empty when there are none (the window
    /// draws a face then).
    pub fn face_of(&self, name: &str, post: &str) -> String {
        let mut map = self.faces.borrow_mut();
        if let Some(f) = map.get(name) {
            return f.clone();
        }
        let pools = [("c", crate::portraits::count("c")), ("g", crate::portraits::count("g"))];
        let face = |prefix: &str, k: usize, _n: usize| format!("{prefix}{k:03}");
        let order: Vec<(&str, usize)> = if post == "general" { vec![pools[1]] } else { vec![pools[0], pools[1]] };
        let h = hash64(name.bytes().fold(0xF00Du64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64)));
        let used: std::collections::HashSet<String> = map.values().cloned().collect();
        let mut pick = String::new();
        for (prefix, n) in &order {
            if *n == 0 {
                continue;
            }
            let start = (h % *n as u64) as usize;
            if let Some(k) = (0..*n).map(|k| (start + k) % n).find(|&k| !used.contains(&face(prefix, k, *n))) {
                pick = face(prefix, k, *n);
                break;
            }
        }
        if pick.is_empty() {
            if let Some((prefix, n)) = order.iter().find(|x| x.1 > 0) {
                pick = face(prefix, h as usize % n, *n);
            }
        }
        map.insert(name.to_string(), pick.clone());
        pick
    }

    /// Days in the month now running.
    pub fn days(&self) -> u32 {
        DAYS[self.month() as usize % 12]
    }

    /// Today, as a date: day, month, year of the game.
    pub fn date(&self) -> String {
        let m = self.month();
        format!("{} {}, {}-й год", self.day + 1, MONTHS[m as usize % 12], m / 12 + 1)
    }

    /// A day goes by: a day of the war of divisions; at the end of the month the world lives the
    /// month, and what matters stops the clock.
    pub fn tick(&mut self) {
        // (an event waits for the player's choice: the clock waits with it)
        if self.paused || self.h.war.fallen.is_some() || !self.ev.waiting.is_empty() {
            return;
        }
        self.war_day();
        self.day += 1;
        if self.day >= self.days() {
            self.day = 0;
            let alerts = self.month_end();
            if self.auto.is_some() {
                // (bots play: the clock stops only where a look at the screen is due)
                if self.after_month() {
                    self.paused = true;
                }
            } else if !alerts.is_empty() {
                self.paused = true;
                self.alert = Some(alerts.join("; "));
            }
        }
        self.version += 1;
    }

    pub fn month(&self) -> u32 {
        self.h.peace_months + self.h.war.month
    }

    pub fn record(&mut self) {
        let held = self.h.war.people();
        let book = [0, 1].map(|s| {
            let n = self.h.home(s);
            let mut b = Book {
                tax: n.taxed.iter().sum(),
                duty: n.duties,
                sectors: n.public_money,
                dole: n.doled,
                army: n.army_spent,
                soldiers: n.soldiers_paid,
                science: n.science_money,
                build: n.built,
                subsidy: n.subsidised.iter().sum(),
                civil: n.civil_paid,
                bonds: self.credit.bond_paid[s],
                other: 0.0,
                treasury: n.treasury,
            };
            if !self.rows.is_empty() {
                let known = b.tax + b.duty + b.sectors - b.dole - b.army - b.soldiers - b.science - b.build - b.subsidy - b.civil - b.bonds;
                b.other = n.treasury - self.t0[s] - known;
            }
            b
        });
        let me = self.me;
        self.rows.push(Row {
            month: self.month(),
            gdp: [self.h.home(0).gdp_real, self.h.home(1).gdp_real],
            army: [self.h.war.sides[0].army.count.iter().sum(), self.h.war.sides[1].army.count.iter().sum()],
            anger: [self.h.home(0).anger(), self.h.home(1).anger()],
            held,
            mix: self.h.war.sides[me].army.count.clone(),
            book,
            stability: self.state.stability,
            legit: [self.polity_of(0).legit, self.polity_of(1).legit],
            unrest: [self.polity_of(0).unrest, self.polity_of(1).unrest],
            science: self.research.points,
            level: [self.h.home(0).level, self.h.home(1).level],
            moods: self.polity_of(me).mood,
        });
    }

    /// The rest of the month: its days of war, then the month of the world (the same world as by
    /// the clock, day by day); what should stop the clock.
    pub fn advance(&mut self) -> Vec<String> {
        while self.day < self.days() {
            self.war_day();
            self.day += 1;
        }
        self.day = 0;
        self.month_end()
    }

    /// A month of the world; what should stop the clock: unrest at home, the front's news (a
    /// capital lost), a capitulation, a design of your laboratories.
    pub fn month_end(&mut self) -> Vec<String> {
        let me = self.me;
        let now = self.month();
        let mut alerts = Vec::new();
        self.t0 = [self.h.home(0).treasury, self.h.home(1).treasury];
        {
            let capitals = self.h.war.th.capital;
            let events = [self.h.home(0).events.len(), self.h.home(1).events.len()];
            let works = self.h.labs[me].as_ref().map_or(0, |l| l.works.len());
            if std::env::var("DIVDBG").is_ok() {
                self.dump_divisions();
            }
            self.keep_laws();
            // what the countries' features, laws and programmes do this month; their finances; what
            // a kit costs
            self.apply_mods();
            self.fiscal_apply();
            // the civil budget, the people's births and deaths, the order of land and labour
            self.civil_apply();
            self.demography_apply();
            self.laws_apply();
            // what the war's mood costs (strikes, desertion); the war loans
            self.war_mood_apply();
            self.arms_before();
            // the army's money: its establishment, mobilisation, mending, new kit; the navy's orders
            self.mil_before();
            if self.h.at_war {
                self.war_footing();
                self.war_use_month();
                for s in 0..2 {
                    self.reanchor(s);
                }
                self.h.step(&self.pack);
            } else {
                self.h.peace(&self.pack);
                // (the army in divisions: what the economy made, who went home, new divisions)
                let made = self.h.last_made.clone();
                let ground = self.h.ground();
                let pack = self.pack.clone();
                let retire = self.h.retire;
                self.h.war.peace_divs(&pack, &made, retire, &ground);
            }
            // what the army's money did: the mending, the depots, the books; the navy's month
            self.mil_after();
            alerts.extend(self.navy_month());
            self.mobilisation_bots();
            for s in 0..2 {
                let n = self.h.home(s);
                for e in &n.events[events[s].min(n.events.len())..] {
                    let note = Note::new(now, &format!("{}: {}", n.name, strip_month(e)), s == me, "econ");
                    if s == me {
                        alerts.push(note.text.clone());
                    }
                    self.news.push(note);
                }
            }
            for l in std::mem::take(&mut self.h.war.log) {
                if l.contains("столица") {
                    continue;
                }
                let note = Note::new(now, &l, true, "army");
                alerts.push(note.text.clone());
                self.news.push(note);
            }
            // the divisions lost in pockets: ours by name, the enemy's counted
            if let Some(c) = self.h.war.corps.as_mut() {
                let mut theirs = 0;
                for (_, s, line) in std::mem::take(&mut c.news) {
                    if s == me {
                        self.news.push(Note::new(now, &line, true, "army"));
                    } else {
                        theirs += 1;
                    }
                }
                if theirs > 0 {
                    self.news.push(Note::new(now, &format!("окружены и уничтожены дивизии врага: {theirs}"), true, "army"));
                }
            }
            for s in 0..2 {
                if self.h.war.th.capital[s] != capitals[s] {
                    let line = format!("пала столица {} — правительство переехало", self.h.home(s).name);
                    alerts.push(line.clone());
                    self.news.push(Note::new(now, &line, true, "army"));
                }
            }
            // what your laboratories made, as their makers present it
            // (what the laboratories made is news, but it does not stop the clock: at top speed
            // designs come often, and the stops were a nuisance)
            if let Some(lab) = self.h.labs[me].as_ref() {
                for w in lab.works[works..].iter().filter(|w| !(lab.hold && w.kind.is_some() && w.adopted)) {
                    self.news.push(match w.kind {
                        Some(u) if !w.adopted => Note::new(w.month, &format!("новую разработку — {} — после испытаний не приняли", self.pack.units[u].name), true, "science"),
                        Some(u) => Note::new(w.month, &format!("новая разработка — {}: конструкторы хвалят", self.pack.units[u].name), true, "science").with_tip(format!(
                            "обещают огонь {:+.0}%, стойкость {:+.0}%",
                            (w.claim.0.exp() - 1.0) * 100.0,
                            (w.claim.1.exp() - 1.0) * 100.0
                        )),
                        None => Note::new(w.month, if w.gain.0 >= 0.0 { "учёные подняли производительность хозяйства" } else { "новшество учёных в хозяйстве вышло неудачным" }, true, "science").with_tip(format!("{:+.1}%", (w.gain.0.exp() - 1.0) * 100.0)),
                    });
                }
            }
            if let Some(s) = self.h.war.fallen {
                let line = format!("{} капитулирует", self.h.home(s).name);
                alerts.push(line.clone());
                self.news.push(Note::new(now, &line, true, "army"));
                // the peace (bots playing for the record stop here)
                if self.auto.is_none() {
                    let line = self.peace(s);
                    alerts.push(line);
                }
            }
            // the factories, the countries' features, the state's programmes and loans
            alerts.extend(self.arms_month());
            self.spirits_month();
            // what the countries came to know, their people's month, the bots' courses
            alerts.extend(self.research_month());
            alerts.extend(self.folk_month());
            // the civil budget, the people, the banks and the debt at home; each force's lot
            alerts.extend(self.civil_month());
            alerts.extend(self.demography_month());
            alerts.extend(self.credit_month());
            self.lots_month();
            alerts.extend(self.realm_month());
            alerts.extend(self.war_mood_month());
            self.course_month();
            alerts.extend(self.agenda_month());
            alerts.extend(self.policy_month());
            alerts.extend(self.relations_month());
            // statecraft and the events of the month
            if self.h.war.fallen.is_none() {
                alerts.extend(self.statecraft());
                alerts.extend(self.events_month());
            }
            self.record();
        }
        self.stamp += 1;
        alerts
    }
}
