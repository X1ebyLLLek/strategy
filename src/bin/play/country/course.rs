//! A country's course: what its government means to do for the next years — catch up in industry,
//! feed its peasants, build a great army of the arm it believes in, dig in behind a line of forts,
//! school its people, mend its finances, win back its lost lands, keep its people quiet, live by
//! trade. Not a script and not a trait written beforehand: the course grows out of who rules (his
//! beliefs, which the years of his youth gave him in the world's past), what the country lives
//! through (hunger, debts, unrest, a lost war, a neighbour holding its land, a neighbour far ahead
//! in technique or in arms) and what the times are (its stage of development against its rival's);
//! it holds for some years — the government's periods, each with its own preferences — and changes
//! when the ruler changes, when a war begins or ends, when a crisis comes, or when its time is up.
//!
//! The bot's government is led by its course everywhere at once: what its science works on (the
//! spheres, the kind of troops it favours), what programmes it begins and which economic order it
//! keeps, how much it gives the army and science, which kinds its factories make more of, its
//! finances (duties, taxes, subsidies). The player learns the rival's course from its deeds and
//! from intelligence (the folder of the country); his own country has none: he is its government.

use strategy::pack::{Line, Pack};
use strategy::rng::{hash64, Rng};
use strategy::world::progress::Leaning;

use crate::game::Note;
use crate::Game;

/// The kinds of course.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Industry,
    Agrarian,
    Army,
    Fortress,
    Science,
    Thrift,
    Revanche,
    Welfare,
    Trade,
}

impl Kind {
    pub fn name(&self) -> &'static str {
        match self {
            Kind::Industry => "Догнать и перегнать",
            Kind::Agrarian => "Хлеб и земля",
            Kind::Army => "Великая армия",
            Kind::Fortress => "Крепость",
            Kind::Science => "Просвещение",
            Kind::Thrift => "Казна прежде всего",
            Kind::Revanche => "Реванш",
            Kind::Welfare => "Порядок и хлеб",
            Kind::Trade => "Открытые рынки",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Kind::Industry => "anvil",
            Kind::Agrarian => "wheat",
            Kind::Army => "army",
            Kind::Fortress => "wall",
            Kind::Science => "graduate",
            Kind::Thrift => "coins",
            Kind::Revanche => "fist",
            Kind::Welfare => "bread",
            Kind::Trade => "shop",
        }
    }

    pub fn about(&self) -> &'static str {
        match self {
            Kind::Industry => "Заводы любой ценой: казна строит, пошлины защищают своё, учёные работают на хозяйство.",
            Kind::Agrarian => "Сначала накормить: реформа земли, урожай, крестьяне — опора власти.",
            Kind::Army => "Сильная армия: военная экономика, деньги генштабу, новые образцы любимого рода войск.",
            Kind::Fortress => "Оборона: линия укреплений, противотанковые средства и пушки, никаких авантюр.",
            Kind::Science => "Школы, университеты, лаборатории: страна учится, плоды — через годы.",
            Kind::Thrift => "Долги душат: экономия, погашение займов, никаких новых трат.",
            Kind::Revanche => "Вернуть отнятое: армия, патриотическое воспитание, притязания на соседа.",
            Kind::Welfare => "Народ ропщет: раздача бедным, твёрдые цены на хлеб, общественные работы.",
            Kind::Trade => "Жить торговлей: никаких пошлин, рынок решает, флот и пути.",
        }
    }

    /// How many years it means to hold, at least and at most.
    fn years(&self) -> (f32, f32) {
        match self {
            Kind::Thrift | Kind::Welfare => (3.0, 6.0),
            Kind::Science | Kind::Industry => (6.0, 12.0),
            _ => (4.0, 9.0),
        }
    }
}

/// A course of a government.
#[derive(Clone, Debug)]
pub struct Course {
    pub kind: Kind,
    pub since: u32,
    pub until: u32,
    /// The kind of troops it favours, if its course is one of arms.
    pub favour: Option<usize>,
    /// Why it was taken, in words (the reasons that weighed most).
    pub why: Vec<String>,
    /// Who ruled when it was taken (a person of the world).
    pub ruler: Option<u32>,
}

/// The courses of both sides now, and those before.
#[derive(Clone, Debug, Default)]
pub struct Courses {
    pub now: [Option<Course>; 2],
    pub past: [Vec<Course>; 2],
}

impl Game {
    /// The worth of each course for side `s` now, with the reasons.
    pub fn course_scores(&self, s: usize) -> Vec<(Kind, f32, Vec<String>)> {
        let f = &self.spirits.facts[s];
        let c = self.country(s);
        let o = self.country(1 - s);
        let ruler = self.folk.offices[c].ruler.map(|r| self.folk.get(r).clone());
        let (hawk, reform, state) = ruler.as_ref().map_or((0.0, 0.0, 0.0), |r| (r.hawk, r.reform, r.state));
        let stage = self.know.get(c).map_or(0, |k| k.stage(&self.pack));
        let rival_stage = self.know.get(o).map_or(0, |k| k.stage(&self.pack));
        let behind = rival_stage as f32 - stage as f32;
        let danger = self.state.claim.iter().enumerate().any(|(x, cl)| x != s && cl.is_some()) || self.state.tension > 0.5;
        let worth = |side: usize| -> f32 {
            let st = &self.h.war.sides[side];
            st.army.count.iter().zip(&st.cost).map(|(a, b)| a * b).sum()
        };
        let (ours, theirs) = (worth(s), worth(1 - s));
        let outgunned = theirs > 1.25 * ours.max(1e-6);
        let richer = self.h.home(1 - s).gdp_real > 1.3 * self.h.home(s).gdp_real;
        let grudge = self.state.grudge[s];
        let lost_last = f.last_won == Some(false) && f.last_war.is_some_and(|y| y < 15.0);
        let mut v: Vec<(Kind, f32, Vec<String>)> = Vec::new();
        let mut add = |k: Kind, score: f32, why: Vec<(&str, bool)>| {
            let reasons: Vec<String> = why.into_iter().filter(|x| x.1).map(|x| x.0.to_string()).collect();
            v.push((k, score, reasons));
        };
        add(
            Kind::Industry,
            1.0 + 2.2 * (f.land_share - 0.35).max(0.0) + 0.8 * behind.max(0.0) + 0.7 * state.max(0.0) + 0.3 * reform.max(0.0) - if self.h.at_war { 1.0 } else { 0.0 },
            vec![("страна живёт землёй, заводов мало", f.land_share > 0.5), ("соперник ушёл вперёд в технике", behind > 0.0), ("правитель верит в государственную стройку", state > 0.4)],
        );
        add(
            Kind::Agrarian,
            0.6 + 6.0 * f.hunger + 2.0 * (1.0 - f.room).max(0.0) + if f.land_share > 0.55 && f.hunger > 0.03 { 0.8 } else { 0.0 } - 0.3 * reform.max(0.0),
            vec![("в стране голодают", f.hunger > 0.05), ("земли мало на всех", f.room < 0.85)],
        );
        add(
            Kind::Army,
            0.8 + 1.3 * hawk.max(0.0) + if danger { 1.2 } else { 0.0 } + if outgunned { 1.0 } else { 0.0 } + if self.h.at_war { 2.0 } else { 0.0 },
            vec![("правитель воинствен", hawk > 0.4), ("соседу грозят войной", danger), ("армия соседа сильнее", outgunned), ("идёт война", self.h.at_war)],
        );
        // (the forces of society pull the government their way: who is strong and heard)
        let pol = &self.polity.of[c];
        let heard = pol.heard();
        use strategy::world::polity as po;
        let pull = |b: usize| heard[b] * (1.0 + pol.mood[b].min(0.0).abs());
        let push: [(Kind, f32, &str); 8] = [
            (Kind::Industry, 2.0 * pull(po::CAPITAL) + 0.8 * pull(po::WORKERS), "промышленники в силе"),
            (Kind::Agrarian, 1.5 * pull(po::PEASANTS) + 1.0 * pull(po::NOBLES), "земля правит страной"),
            (Kind::Army, 2.0 * pull(po::OFFICERS) + 1.2 * pol.moves[po::HAWKS], "державники и офицеры"),
            (Kind::Science, 1.8 * pull(po::LEARNED) + 0.8 * pol.moves[po::LIBERALS], "интеллигенция слышна"),
            (Kind::Trade, 2.0 * pull(po::MERCHANTS), "купечество в силе"),
            (Kind::Welfare, 1.5 * pol.moves[po::LEVELLERS] + 1.2 * pol.unrest, "уравнители и смута"),
            (Kind::Revanche, 1.8 * pol.moves[po::REVANCHE], "реваншисты"),
            (Kind::Thrift, 0.6 * pull(po::MERCHANTS) + 0.4 * pull(po::CAPITAL), "собственники требуют порядка в казне"),
        ];
        add(
            Kind::Fortress,
            0.6 + if richer && hawk < 0.3 { 1.0 } else { 0.0 } + if f.capital_depth <= 3 { 0.8 } else { 0.0 } + if lost_last { 0.6 } else { 0.0 } + 0.5 * (-hawk).max(0.0),
            vec![("сосед богаче и сильнее", richer), ("столица близко к границе", f.capital_depth <= 3), ("последняя война проиграна", lost_last)],
        );
        add(
            Kind::Science,
            0.7 + 1.4 * reform.max(0.0) + 0.6 * behind.max(0.0) + if f.learned > 0.12 { 0.4 } else { 0.0 } + if lost_last { 0.4 } else { 0.0 } - if self.h.at_war { 1.2 } else { 0.0 },
            vec![("правитель — реформатор", reform > 0.4), ("отстаём в технике", behind > 0.0), ("образованных много", f.learned > 0.12)],
        );
        add(
            Kind::Thrift,
            0.3 + f.debt / 8.0 + if f.treasury < 0.0 { 1.2 } else { 0.0 } - 0.4 * hawk.max(0.0),
            vec![("долги велики", f.debt > 8.0), ("казна пуста", f.treasury < 0.0)],
        );
        if grudge || f.lost > 0 {
            add(
                Kind::Revanche,
                0.6 + if grudge { 1.6 } else { 0.4 } + 1.0 * hawk.max(0.0) + if lost_last { 0.5 } else { 0.0 } - if richer { 0.5 } else { 0.0 },
                vec![("сосед держит отнятые земли", grudge), ("правитель воинствен", hawk > 0.4), ("свежа обида поражения", lost_last)],
            );
        }
        add(
            Kind::Welfare,
            0.4 + 1.8 * f.anger + if f.unrest { 1.0 } else { 0.0 } + 3.0 * f.hunger,
            vec![("народ зол", f.anger > 0.8), ("волнения", f.unrest)],
        );
        add(
            Kind::Trade,
            0.5 + 2.5 * f.imports + 1.0 * f.coast + 0.6 * (-state).max(0.0) - if danger { 0.8 } else { 0.0 },
            vec![("страна живёт привозом", f.imports > 0.2), ("у моря", f.coast > 0.3), ("правитель — сторонник хозяев", state < -0.4)],
        );
        for (k, x, why) in push {
            if let Some(e) = v.iter_mut().find(|e| e.0 == k) {
                e.1 += x;
                if x > 0.4 {
                    e.2.push(why.to_string());
                }
            }
        }
        v
    }

    /// The kind of troops a course of arms favours: what the staff is proud of, what its chief and
    /// the ruler love, what is newly fielded, what answers the rival's arms (none it cannot field).
    fn favourite(&self, s: usize, kind: Kind) -> Option<usize> {
        let pack = &self.pack;
        let c = self.country(s);
        let st = &self.h.war.sides[s];
        let foe = &self.h.war.sides[1 - s];
        let foe_mix = foe.mix();
        let chief = self.folk.offices[c].staff.map(|x| self.folk.get(x).love);
        let ruler = self.folk.offices[c].ruler.map(|x| self.folk.get(x).love);
        let mut best: Option<(usize, f32)> = None;
        for (u, unit) in pack.units.iter().enumerate() {
            if st.is_locked(u) {
                continue;
            }
            let mut w = 2.0 * st.glory.get(u).copied().unwrap_or(0.0) + 0.5 * (st.own_fire[u].mean + st.own_tough[u].mean);
            if chief == Some(u) {
                w += 0.6;
            }
            if ruler == Some(u) {
                w += 0.3;
            }
            // an answer to what the rival fields most: against armour, what pierces it
            let foe_armour: f32 = (0..pack.units.len()).map(|v| foe_mix.get(v).copied().unwrap_or(0.0) * pack.units[v].armor).sum();
            if foe_armour > 0.0 && unit.pierce > 0.0 {
                w += 0.8 * foe_armour.min(1.0) * (unit.pierce / 10.0).min(1.0);
            }
            match kind {
                Kind::Fortress => w += 0.5 * (unit.defend - unit.attack) + if unit.line == Line::Support { 0.3 } else { 0.0 },
                Kind::Army | Kind::Revanche => w += 0.5 * (unit.attack - unit.defend) + 0.1 * unit.armor.min(5.0),
                _ => {}
            }
            if best.is_none_or(|b| w > b.1) {
                best = Some((u, w));
            }
        }
        best.map(|b| b.0)
    }

    /// The bot's government reviews its course: when it has none, when its time is up, when the
    /// ruler changed, when a war began or ended, when a crisis came; the new one is the course worth
    /// most now (a course held is worth a little more: governments do not change their minds every
    /// year).
    pub fn course_month(&mut self) {
        let now = self.month();
        let bots: Vec<usize> = if self.auto.is_some() { vec![0, 1] } else { vec![1 - self.me] };
        for s in bots {
            let c = self.country(s);
            let ruler = self.folk.offices[c].ruler;
            let f = self.spirits.facts[s].clone();
            let review = match &self.courses.now[s] {
                None => true,
                Some(cur) => {
                    let is_peace_course = matches!(cur.kind, Kind::Thrift | Kind::Welfare | Kind::Trade | Kind::Agrarian);
                    now >= cur.until
                        || cur.ruler != ruler
                        || (self.h.at_war && !matches!(cur.kind, Kind::Army | Kind::Fortress | Kind::Revanche) && now.saturating_sub(cur.since) > 2)
                        || (is_peace_course && (self.state.tension > 0.6 || self.mil.mobilised[1 - s].is_some()) && now.saturating_sub(cur.since) > 2)
                        || (f.unrest && cur.kind != Kind::Welfare && now % 12 == 0)
                        || (f.treasury < -2.0 && cur.kind != Kind::Thrift && now % 12 == 0)
                }
            };
            if !review {
                continue;
            }
            let mut r = Rng::new(hash64(self.seed ^ 0xC0_75E ^ (now as u64) << 4 ^ s as u64));
            let mut scores = self.course_scores(s);
            if let Some(cur) = &self.courses.now[s] {
                for x in scores.iter_mut().filter(|x| x.0 == cur.kind) {
                    x.1 += 0.5;
                }
            }
            // (the same facts weigh differently on different men: a little luck)
            for x in scores.iter_mut() {
                x.1 *= (0.15 * r.normal()).exp();
            }
            scores.sort_by(|a, b| b.1.total_cmp(&a.1));
            let Some((kind, _, why)) = scores.into_iter().next() else { continue };
            let (lo, hi) = kind.years();
            let years = lo + (hi - lo) * r.f32();
            let favour = if matches!(kind, Kind::Army | Kind::Revanche | Kind::Fortress) { self.favourite(s, kind) } else { None };
            let same = self.courses.now[s].as_ref().is_some_and(|x| x.kind == kind && x.favour == favour);
            let course = Course { kind, since: now, until: now + (years * 12.0) as u32, favour, why, ruler };
            if let Some(old) = self.courses.now[s].replace(course) {
                self.courses.past[s].push(old);
            }
            if !same && now > 0 {
                let name = self.h.home(s).name.clone();
                let what = match favour {
                    Some(u) => format!("{} (ставка на род войск: {})", kind.name(), self.pack.units[u].name),
                    None => kind.name().to_string(),
                };
                let why = self.courses.now[s].as_ref().map_or(String::new(), |x| x.why.join("; "));
                let line = if s == self.me { format!("новый курс правительства: «{what}»") } else { format!("разведка: {name} берёт курс «{what}»") };
                self.news.push(Note::new(now, &line, s != self.me, "politics").with_tip(if why.is_empty() { kind.about().to_string() } else { format!("{} — {why}", kind.about()) }));
            }
        }
        // what the courses ask of the economy, science and factories, month by month
        let bots: Vec<usize> = if self.auto.is_some() { vec![0, 1] } else { vec![1 - self.me] };
        for s in bots {
            self.course_steer(s);
            self.bot_fiscal(s);
        }
    }

    /// The course's word on the bot's spending, laboratories and factories this month.
    fn course_steer(&mut self, s: usize) {
        let Some(cur) = self.courses.now[s].clone() else { return };
        let pack = self.pack.clone();
        let nu = pack.units.len();
        let at_war = self.h.at_war;
        // the army's share of the state's spending, and science's
        let army = match cur.kind {
            Kind::Army | Kind::Revanche if !at_war => Some(pack.military * 1.3),
            Kind::Fortress if !at_war => Some(pack.military * 1.15),
            Kind::Thrift if !at_war => Some(pack.military * 0.75),
            _ => None,
        };
        self.h.army_share[s] = army;
        if let Some(x) = army {
            self.h.home_mut(s).mil_share = x;
        }
        if let Some(sc) = &pack.science {
            let k = match cur.kind {
                Kind::Science => 1.8,
                Kind::Industry => 1.2,
                Kind::Thrift => 0.6,
                Kind::Army | Kind::Revanche if at_war => 0.7,
                _ => 1.0,
            };
            self.h.home_mut(s).science_share = sc.share * k;
        }
        // the laboratories: the course's spheres, as far as the head of science lets it
        if let Some(lab) = self.h.labs[s].as_mut() {
            let k = nu + 1;
            let mut want = vec![1.0f32; k];
            match cur.kind {
                Kind::Industry | Kind::Agrarian | Kind::Trade | Kind::Welfare => want[nu] = 4.0,
                Kind::Science => want[nu] = 2.0,
                Kind::Army | Kind::Revanche | Kind::Fortress => {
                    if let Some(u) = cur.favour {
                        want[u] = 4.0;
                    }
                    want[nu] = 0.6;
                }
                Kind::Thrift => {}
            }
            for (u, w) in want.iter_mut().enumerate().take(nu) {
                if self.h.war.sides[s].is_locked(u) {
                    *w = 0.1;
                }
            }
            let all: f32 = want.iter().sum();
            if lab.focus.len() == k {
                for i in 0..k {
                    lab.focus[i] = 0.75 * lab.focus[i] + 0.25 * want[i] / all;
                }
            }
        }
        // the factories: more of the favoured kind, less of what the course has no use for
        if self.arms.lines[s].len() == nu {
            for u in 0..nu {
                let lv = match (cur.kind, cur.favour) {
                    (Kind::Army | Kind::Revanche | Kind::Fortress, Some(f)) if f == u => 3,
                    (Kind::Thrift, _) => 1,
                    _ => crate::army::arms::PRIO_USUAL,
                };
                self.arms.lines[s][u].prio = lv;
            }
        }
    }

    /// What the bot's course makes its science lean to (with the leanings of its ruler and its
    /// head of science).
    pub fn course_leaning(&self, s: usize) -> Leaning {
        let pack: &Pack = &self.pack;
        let c = self.country(s);
        let mut l = self.folk.leaning(pack, c);
        let Some(cur) = self.courses.now[s].as_ref() else { return l };
        match cur.kind {
            Kind::Industry => {
                l.sphere[0] *= 3.0;
                for k in 0..pack.sectors.len() {
                    let (_, military, industry, _) = crate::country::policy::kind_of(pack, k);
                    if industry || military {
                        l.sectors[k] *= 2.0;
                    }
                }
            }
            Kind::Agrarian => {
                l.sphere[0] *= 2.0;
                for k in 0..pack.sectors.len() {
                    if pack.sectors[k].land {
                        l.sectors[k] *= 3.0;
                    }
                }
            }
            Kind::Army | Kind::Revanche => {
                l.sphere[1] *= 3.0;
                if let Some(u) = cur.favour {
                    l.kinds[u] *= 3.0;
                }
            }
            Kind::Fortress => {
                l.sphere[1] *= 2.0;
                l.sphere[3] *= 1.5;
                if let Some(u) = cur.favour {
                    l.kinds[u] *= 2.5;
                }
            }
            Kind::Science => {
                l.sphere[2] *= 3.0;
                l.sphere[3] *= 1.5;
            }
            Kind::Thrift => l.sphere[0] *= 1.5,
            Kind::Welfare => {
                l.sphere[0] *= 1.5;
                l.sphere[3] *= 1.5;
                for k in 0..pack.sectors.len() {
                    if pack.sectors[k].land {
                        l.sectors[k] *= 1.5;
                    }
                }
            }
            Kind::Trade => {
                l.sphere[0] *= 2.0;
                l.sphere[3] *= 1.2;
            }
        }
        l
    }

    /// The programmes the bot's course wants first (ids of `policy::PLANS`).
    pub fn course_plans(&self, s: usize) -> Vec<&'static str> {
        match self.courses.now[s].as_ref().map(|c| c.kind) {
            Some(Kind::Industry) => vec!["industry", "rail"],
            Some(Kind::Agrarian) => vec!["land", "works"],
            Some(Kind::Army) => vec!["rearm", "patriots"],
            Some(Kind::Fortress) => vec!["forts", "rail"],
            Some(Kind::Science) => vec!["schools"],
            Some(Kind::Thrift) => vec!["austerity"],
            Some(Kind::Revanche) => vec!["rearm", "patriots"],
            Some(Kind::Welfare) => vec!["works", "land"],
            Some(Kind::Trade) => vec!["rail"],
            None => Vec::new(),
        }
    }

    /// The economic order the bot's course prefers in peace (None: as the danger says).
    pub fn course_law(&self, s: usize) -> Option<usize> {
        match self.courses.now[s].as_ref().map(|c| c.kind) {
            Some(Kind::Trade) => Some(0),
            Some(Kind::Army) | Some(Kind::Revanche) => Some(2),
            _ => None,
        }
    }

    /// What intelligence says of the rival's course, in words (and the reasons on its card).
    pub fn course_words(&self, s: usize) -> Option<(String, String, &'static str)> {
        let c = self.courses.now[s].as_ref()?;
        let what = match c.favour {
            Some(u) => format!("{} — ставка на «{}»", c.kind.name(), self.pack.units[u].name),
            None => c.kind.name().to_string(),
        };
        let why = if c.why.is_empty() { c.kind.about().to_string() } else { format!("{} Почему: {}.", c.kind.about(), c.why.join("; ")) };
        Some((what, why, c.kind.icon()))
    }
}
