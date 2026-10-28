//! Economists: advisors who run a country's economy for its government — the tax, how much the
//! state builds, where, and whether by plan.
//!
//! Everything about an advisor is hidden. His work depends only on four things: education (does
//! he know how an economy works — can he foresee what a step will do), experience (it grows in
//! office; a green one slips more and misjudges more), personal beliefs (whether he takes from
//! the treasury; whether the state should build or leave it to owners) and talent (how many ways
//! he weighs). Nothing else is written into him: a strong one weighs ways on a model of the
//! country (the economy's own engine, as he understands it), an average one keeps to common
//! rules, a weak one slips and steals. The player sees only what he did and how the country
//! fared (`record`).

use rayon::prelude::*;
use crate::economy::nation::{Nation, Trade};
use crate::pack::Pack;
use crate::rng::{hash64, Rng};

/// He sits over the economy every this many months.
const SITTING: u32 = 3;
/// Months ahead a thinking advisor looks.
const HORIZON: u32 = 24;
/// How wrong an uneducated, green advisor's picture of his country's productivity is (σ, share).
const MODEL_ERR: f32 = 0.3;
/// Chance per sitting that an uneducated, green advisor does something else entirely.
const SLIP: f32 = 0.6;
/// Share of the state's monthly budget a wholly dishonest advisor takes.
const TAKE: f32 = 0.3;
/// Months for experience to grow by about two thirds of what is left.
const LEARN: f32 = 120.0;
/// A thinking advisor drops ways that give the army less than this share of what the custom would.
const ARMY_KEEP: f32 = 0.98;
/// How much worse a forecast a wholehearted believer takes for the way his belief favours most,
/// against the custom (share): beliefs weigh against the reckoning, as in life.
const DOGMA: f32 = 0.05;
/// How hard a believer in the state plans (`Nation::plan`) when he follows his rules.
const PLAN_MAX: f32 = 2.0;
/// Tax steps an advisor may choose: times the custom (the pack's, or the government's war tax).
const TAXES: [f32; 7] = [0.5, 0.7, 1.0, 1.4, 2.0, 3.0, 4.0];
const BUILDS: [f32; 3] = [0.0, 0.5, 1.0];
const PLANS: [f32; 3] = [0.0, 1.0, 2.0];

/// What an advisor believes in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beliefs {
    /// 1: never takes from the treasury; 0: takes what he can.
    pub honesty: f32,
    /// −1: the state should neither tax nor build, owners know best; 0: whatever works; +1: the
    /// state plans and builds (a technocrat) — what counts is what the country makes.
    pub state: f32,
}

/// The hidden qualities of an advisor, each 0..1 (beliefs aside).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mind {
    pub education: f32,
    pub experience: f32,
    pub talent: f32,
    pub beliefs: Beliefs,
}

impl Mind {
    pub fn new(education: f32, experience: f32, talent: f32, honesty: f32, state: f32) -> Mind {
        Mind { education, experience, talent, beliefs: Beliefs { honesty, state } }
    }

    /// Kinds the checks compare: educated, seasoned, gifted and honest, believing in whatever works.
    pub fn strong() -> Mind {
        Mind::new(0.9, 0.7, 0.9, 1.0, 0.0)
    }
    pub fn average() -> Mind {
        Mind::new(0.5, 0.5, 0.5, 0.9, 0.0)
    }
    /// Uneducated, green, dull and on the take.
    pub fn weak() -> Mind {
        Mind::new(0.15, 0.2, 0.3, 0.3, 0.0)
    }
    /// As strong, but believing that the state should plan and build.
    pub fn technocrat() -> Mind {
        Mind::new(0.9, 0.7, 0.9, 1.0, 1.0)
    }

    /// Someone the government might appoint: anyone at all.
    pub fn anyone(rng: &mut Rng) -> Mind {
        let honesty = 1.0 - rng.f32().powi(2);
        Mind::new(rng.f32(), 0.8 * rng.f32(), rng.f32(), honesty, 2.0 * rng.f32() - 1.0)
    }

    /// How many ways besides his rule an advisor weighs on his model: none for the uneducated or
    /// untalented, about six for the best.
    pub fn ways(&self) -> usize {
        (12.0 * self.education * self.talent - 4.0).round().max(0.0) as usize
    }
}

/// The levers an economist moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Levers {
    /// Tax on incomes, times the custom.
    pub tax: f32,
    /// Share of the state's spending left after the army that may go into building.
    pub build: f32,
    /// How far the state builds by plan (`Nation::plan`).
    pub plan: f32,
    /// The sector the state builds, if he chooses it.
    pub focus: Option<usize>,
}

impl Levers {
    /// The country's custom: the pack's tax, building where people suffer most.
    pub fn custom() -> Levers {
        Levers { tax: 1.0, build: 1.0, plan: 0.0, focus: None }
    }

    /// Sets the levers on the country. `ask` is what the government sets for the army: its share of
    /// the state's spending (the advisor does not touch it), and the custom tax.
    pub fn apply(&self, n: &mut Nation, ask: (f32, f32)) {
        n.tax = ask.1 * self.tax;
        n.mil_share = ask.0;
        n.build_share = self.build;
        n.plan = self.plan;
        n.build_focus = self.focus;
    }

    /// How far these levers go the way a believer of `belief` (−1..+1) wants them, against the
    /// custom: a believer in the state wants it to plan and build (the tax is only a means), a
    /// believer in owners wants a low tax and a state that does not build.
    pub fn lean(&self, belief: f32) -> f32 {
        let builds = |l: &Levers| (l.plan / PLAN_MAX + 2.0 * l.build - 1.0) / 2.0;
        let frees = |l: &Levers| (-(l.tax.ln() / 4f32.ln()).clamp(-1.0, 1.0) + 1.0 - 2.0 * l.build) / 2.0;
        let c = Levers::custom();
        belief.max(0.0) * (builds(self) - builds(&c)) + (-belief).max(0.0) * (frees(self) - frees(&c))
    }

    /// In a few words, for the record.
    pub fn describe(&self, pack: &Pack, ask: (f32, f32)) -> String {
        let build = if self.build <= 0.0 {
            "казна не строит".to_string()
        } else {
            let what = self.focus.map_or("где людям хуже".to_string(), |s| pack.sectors[s].name.clone());
            let how = if self.plan > 0.0 { format!(", по плану ×{:.0}", 1.0 + self.plan) } else { String::new() };
            format!("стройка {:.0}%: {what}{how}", self.build * 100.0)
        };
        format!("налог {:.0}%, {build}", ask.1 * self.tax * 100.0)
    }
}

/// The world outside as an advisor's model of his country holds it: last month's trade, the
/// same every month, and the money for goods sold abroad that came last.
#[derive(Clone)]
pub struct Outside {
    pub trade: Trade,
    pub remit: Vec<f32>,
}

impl Outside {
    /// A country on its own.
    pub fn alone(pack: &Pack) -> Outside {
        Outside { trade: Trade::none(pack.goods.len()), remit: vec![0.0; pack.goods.len()] }
    }

    /// Country `n` as it traded last month; `remit`: money on its way home for its goods.
    pub fn of(n: &Nation, remit: &[f32]) -> Outside {
        Outside { trade: Trade { exports: n.shipped.clone(), imports: n.import.clone() }, remit: remit.to_vec() }
    }
}

/// How well a country lives: its people, each counted by how much of what they need they have.
pub fn welfare(n: &Nation) -> f32 {
    (0..n.pop.len()).map(|g| n.pop[g] * (1.0 - n.hardship[g])).sum()
}

/// One sitting of an advisor, as the government sees it.
#[derive(Clone, Debug)]
pub struct Deed {
    pub month: u32,
    /// What he did, in words.
    pub what: String,
}

/// A year of an advisor in office, as the government sees it.
#[derive(Clone, Debug)]
pub struct Year {
    pub year: u32,
    /// Change of how well the people live, and of real output, over the year (shares).
    pub welfare: f32,
    pub output: f32,
    pub unrest: usize,
}

/// An economist in office.
#[derive(Clone)]
pub struct Economist {
    pub mind: Mind,
    rng: Rng,
    /// How wrong his picture of each sector's productivity is, in units of his error.
    blind: Vec<f32>,
    pub levers: Levers,
    /// Months in office.
    pub months: u32,
    pub record: Vec<Deed>,
    pub years: Vec<Year>,
    /// Taken from the treasury, all told (hidden).
    pub took: f32,
    /// Where the year began: welfare, output (sums over its months), unrest events seen.
    year_start: (f32, f32, usize),
    year_sum: (f32, f32),
    last_year: Option<(f32, f32)>,
}

impl Economist {
    pub fn new(mind: Mind, seed: u64, pack: &Pack) -> Economist {
        let mut rng = Rng::new(hash64(seed ^ 0xEC0));
        let blind = (0..pack.sectors.len()).map(|_| rng.normal()).collect();
        Economist {
            mind,
            rng,
            blind,
            levers: Levers::custom(),
            months: 0,
            record: Vec::new(),
            years: Vec::new(),
            took: 0.0,
            year_start: (0.0, 0.0, 0),
            year_sum: (0.0, 0.0),
            last_year: None,
        }
    }

    /// How wrong his model is now: less with education, and with experience.
    fn model_err(&self) -> f32 {
        MODEL_ERR * (1.0 - self.mind.education) * (1.0 - 0.5 * self.mind.experience)
    }

    /// What his rules say, coloured by what he believes: the custom, more tax and building by plan
    /// for a believer in the state, less of both for a believer in owners.
    pub fn rule(&self, pack: &Pack) -> Levers {
        let st = self.mind.beliefs.state;
        let tax = nearest(&TAXES, 1.0 + 0.5 * st);
        let build = nearest(&BUILDS, (1.0 + st).min(1.0));
        let plan = nearest(&PLANS, st.max(0.0) * PLAN_MAX);
        // a planner builds what makes the most per job at the usual prices
        let focus = if plan > 0.0 { favourite(pack) } else { None };
        Levers { tax, build, plan, focus }
    }

    /// A month in office, before the country's month: he sits each season, and takes his part.
    /// `ask`: what the government asks for the army (share of spending, tax), see `Levers::apply`.
    /// `outside`: the world as his model holds it.
    pub fn month(&mut self, pack: &Pack, n: &mut Nation, ask: (f32, f32), outside: &Outside) {
        if self.months % SITTING == 0 {
            self.sit(pack, n, ask, outside);
        }
        self.levers.apply(n, ask);
        // he takes his part of the state's money: it goes to the people he lives among (the group
        // with the most money a head), so no money leaves the country
        let take = TAKE * (1.0 - self.mind.beliefs.honesty).powi(2) * n.state_budget();
        if take > 0.0 {
            let g = (0..n.pop.len())
                .filter(|&g| n.pop[g] > 0.0)
                .max_by(|&a, &b| (n.money[a] / n.pop[a]).total_cmp(&(n.money[b] / n.pop[b])))
                .unwrap_or(0);
            n.treasury -= take;
            n.money[g] += take;
            self.took += take;
        }
        self.months += 1;
        self.mind.experience += (1.0 - self.mind.experience) / LEARN;
    }

    /// After the country's month: the books of his year.
    pub fn after(&mut self, n: &Nation) {
        if self.months % 12 == 1 {
            self.year_start.2 = n.events.len();
            self.year_sum = (0.0, 0.0);
        }
        self.year_sum.0 += welfare(n);
        self.year_sum.1 += n.gdp_real;
        if self.months % 12 == 0 {
            let now = self.year_sum;
            let (dw, dg) = match self.last_year {
                Some(prev) => (now.0 / prev.0.max(1e-9) - 1.0, now.1 / prev.1.max(1e-9) - 1.0),
                None => (0.0, 0.0),
            };
            self.years.push(Year { year: self.months / 12, welfare: dw, output: dg, unrest: n.events.len() - self.year_start.2 });
            self.last_year = Some(now);
        }
    }

    fn sit(&mut self, pack: &Pack, n: &Nation, ask: (f32, f32), outside: &Outside) {
        let rule = self.rule(pack);
        let now = self.levers;
        let mut choice = rule;
        let ways = self.mind.ways();
        if ways > 0 {
            // ways to weigh: the custom, what he does now, what his rules say, and some steps from
            // the last two
            let mut options = vec![Levers::custom()];
            for o in [now, rule] {
                if !options.contains(&o) {
                    options.push(o);
                }
            }
            let mut tries = 0;
            while options.len() < 3 + ways && tries < 50 {
                tries += 1;
                let base = if self.rng.f32() < 0.5 { now } else { rule };
                let o = self.nudge(pack, base);
                if !options.contains(&o) {
                    options.push(o);
                }
            }
            let model = self.model(n);
            let st = self.mind.beliefs.state.max(0.0);
            let outcomes: Vec<Forecast> = options.par_iter().map(|o| foresee(pack, &model, o, ask, outside)).collect();
            let base = outcomes[0];
            // the army must get no less than the custom would give it (what the government asked);
            // and what he believes in counts for something beside what the model says
            let belief = self.mind.beliefs.state;
            let score = |k: usize| {
                let f = &outcomes[k];
                if f.army < ARMY_KEEP * base.army {
                    f32::NEG_INFINITY
                } else {
                    (1.0 - st) * f.welfare / base.welfare.max(1e-9) + st * f.output / base.output.max(1e-9)
                        + DOGMA * options[k].lean(belief)
                }
            };
            let best = (0..options.len()).max_by(|&a, &b| score(a).total_cmp(&score(b))).unwrap();
            choice = options[best];
        }
        // an uneducated, green advisor sometimes does something else: a step or a few, any way
        // (not a pick from all the steps there are: with taxes from half to four times the custom,
        // a pick at random raised the tax on average, and the poor got more)
        let slip = SLIP * (1.0 - self.mind.education) * (1.0 - self.mind.experience);
        if self.rng.f32() < slip {
            for _ in 0..1 + self.rng.below(3) {
                choice = self.nudge(pack, choice);
            }
        }
        if choice != self.levers || self.record.is_empty() {
            self.record.push(Deed { month: self.months, what: choice.describe(pack, ask) });
        }
        self.levers = choice;
    }

    /// His model of the country: the country as it is, but each sector's productivity as he
    /// believes it.
    fn model(&self, n: &Nation) -> Nation {
        let mut m = n.clone();
        let err = self.model_err();
        for (p, b) in m.prod.iter_mut().zip(&self.blind) {
            *p *= (1.0 + err * b).max(0.1);
        }
        m.events.clear();
        m
    }

    /// One lever moved a step from `base`.
    fn nudge(&mut self, pack: &Pack, base: Levers) -> Levers {
        let mut o = base;
        match self.rng.below(4) {
            0 => {
                let direction = if self.rng.f32() < 0.5 { -1 } else { 1 };
                o.tax = step(&TAXES, o.tax, direction);
            },
            1 => o.build = step(&BUILDS, o.build, if self.rng.f32() < 0.5 { -1 } else { 1 }),
            2 => o.plan = step(&PLANS, o.plan, if self.rng.f32() < 0.5 { -1 } else { 1 }),
            _ => o.focus = self.any_sector(pack),
        }
        o
    }

    fn any_sector(&mut self, pack: &Pack) -> Option<usize> {
        let can: Vec<usize> = (0..pack.sectors.len()).filter(|&s| !pack.sectors[s].build.is_empty()).collect();
        let k = self.rng.below(can.len() + 1);
        can.get(k).copied()
    }
}

/// What a way would bring on the model over the horizon, a month on average.
#[derive(Clone, Copy)]
struct Forecast {
    welfare: f32,
    output: f32,
    /// The army's money.
    army: f32,
}

fn foresee(pack: &Pack, model: &Nation, o: &Levers, ask: (f32, f32), outside: &Outside) -> Forecast {
    let mut m = model.clone();
    o.apply(&mut m, ask);
    let (mut w, mut g, mut a) = (0.0, 0.0, 0.0);
    for _ in 0..HORIZON {
        m.remit_in.copy_from_slice(&outside.remit);
        a += m.state_budget() * m.mil_share;
        m.step_with(pack, &outside.trade);
        w += welfare(&m);
        g += m.gdp_real;
    }
    let h = HORIZON as f32;
    Forecast { welfare: w / h, output: g / h, army: a / h }
}

/// The sector that makes the most per job at the usual prices, among those the state can build.
fn favourite(pack: &Pack) -> Option<usize> {
    let worth = |s: usize| {
        let sec = &pack.sectors[s];
        let out: f32 = sec.outputs.iter().map(|&(j, a)| a * pack.goods[j].price).sum();
        let inp: f32 = sec.inputs.iter().map(|&(i, a)| a * pack.goods[i].price).sum();
        out - inp
    };
    (0..pack.sectors.len()).filter(|&s| !pack.sectors[s].build.is_empty()).max_by(|&a, &b| worth(a).total_cmp(&worth(b)))
}

fn nearest(steps: &[f32], x: f32) -> f32 {
    *steps.iter().min_by(|a, b| (*a - x).abs().total_cmp(&(*b - x).abs())).unwrap()
}

fn step(steps: &[f32], x: f32, d: i32) -> f32 {
    let i = steps.iter().position(|&s| s == x).unwrap_or_else(|| steps.iter().position(|&s| s == nearest(steps, x)).unwrap());
    steps[(i as i32 + d).clamp(0, steps.len() as i32 - 1) as usize]
}
