//! What a country is like (as HOI4's national spirits, but nothing written beforehand): each
//! feature grows out of what the world made of the country — its ground and coasts, what lies
//! under it, how its people live and work, its treasury, its wars of the past century and of the
//! game, its staff's pride and doubt — and fades when what made it is gone. A feature comes in
//! slowly (a tenth of the way a month) and goes as slowly; while it is there it weighs on the
//! economy, the people, the laboratories, the factories and the divisions, by its strength. The
//! player sees why each is there (the facts, in words, the numbers on hover) and what it does; he
//! can change the facts (an education, an industrialisation, a war won), never the feature itself.
//! Both sides live by the same rules: the bot's country has its features too.

use strategy::pack::Pack;
use strategy::world::map::Terrain;

use crate::Game;

mod defs;
use defs::{ago_words, glory_def, DEFS};
mod facts;

/// A feature appears when its facts call for this much of it, and goes when they call for less
/// than `GONE`; its strength follows the facts by this share a month.
const COME: f32 = 0.35;
const GONE: f32 = 0.12;
const PACE: f32 = 0.1;

/// What features (and laws, programmes) do to a country, at their full strength. Multipliers are 1
/// and additions 0 when they do nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Eff {
    /// Output of the land, of the factories and mines, of the trades making military goods, of the
    /// trades the people buy from (cloth, goods, services).
    pub out_land: f32,
    pub out_industry: f32,
    pub out_military: f32,
    pub out_civil: f32,
    /// Discontent added a month to the people of the land, the workers, the learned, the owners.
    pub anger: [f32; 4],
    /// Stability a month.
    pub stability: f32,
    /// The laboratories' power.
    pub science: f32,
    /// How fast the factories learn a model, how far they can get, what a kit costs them.
    pub learn: f32,
    pub skill: f32,
    pub make_cost: f32,
    /// The divisions: fire attacking and defending, harm taken, organisation got back, digging in,
    /// experience, losses of the hungry and cut off.
    pub attack: f32,
    pub defend: f32,
    pub harm: f32,
    pub org_back: f32,
    pub dig: f32,
    pub exp: f32,
    pub attrition: f32,
    /// Share of the state's money that never reaches what it was meant for (a month).
    pub leak: f32,
    /// The most of the people the law of the draft can call, this many times.
    pub draft: f32,
    /// Factories learn models of this kind of unit faster (index, how much).
    pub kind: Option<(usize, f32)>,
}

impl Default for Eff {
    fn default() -> Eff {
        Eff {
            out_land: 1.0,
            out_industry: 1.0,
            out_military: 1.0,
            out_civil: 1.0,
            anger: [0.0; 4],
            stability: 0.0,
            science: 1.0,
            learn: 1.0,
            skill: 0.0,
            make_cost: 1.0,
            attack: 1.0,
            defend: 1.0,
            harm: 1.0,
            org_back: 1.0,
            dig: 1.0,
            exp: 1.0,
            attrition: 1.0,
            leak: 0.0,
            draft: 1.0,
            kind: None,
        }
    }
}

impl Eff {
    /// This one at strength `k` (0 to 1).
    pub fn at(&self, k: f32) -> Eff {
        let m = |x: f32| 1.0 + (x - 1.0) * k;
        Eff {
            out_land: m(self.out_land),
            out_industry: m(self.out_industry),
            out_military: m(self.out_military),
            out_civil: m(self.out_civil),
            anger: self.anger.map(|a| a * k),
            stability: self.stability * k,
            science: m(self.science),
            learn: m(self.learn),
            skill: self.skill * k,
            make_cost: m(self.make_cost),
            attack: m(self.attack),
            defend: m(self.defend),
            harm: m(self.harm),
            org_back: m(self.org_back),
            dig: m(self.dig),
            exp: m(self.exp),
            attrition: m(self.attrition),
            leak: self.leak * k,
            draft: m(self.draft),
            kind: self.kind.map(|(u, x)| (u, 1.0 + (x - 1.0) * k)),
        }
    }

    /// Two together.
    pub fn and(&self, o: &Eff) -> Eff {
        let mut a = [0.0; 4];
        for i in 0..4 {
            a[i] = self.anger[i] + o.anger[i];
        }
        Eff {
            out_land: self.out_land * o.out_land,
            out_industry: self.out_industry * o.out_industry,
            out_military: self.out_military * o.out_military,
            out_civil: self.out_civil * o.out_civil,
            anger: a,
            stability: self.stability + o.stability,
            science: self.science * o.science,
            learn: self.learn * o.learn,
            skill: self.skill + o.skill,
            make_cost: self.make_cost * o.make_cost,
            attack: self.attack * o.attack,
            defend: self.defend * o.defend,
            harm: self.harm * o.harm,
            org_back: self.org_back * o.org_back,
            dig: self.dig * o.dig,
            exp: self.exp * o.exp,
            attrition: self.attrition * o.attrition,
            leak: self.leak + o.leak,
            draft: self.draft * o.draft,
            kind: self.kind.or(o.kind),
        }
    }

    /// What it does, line by line: (what, how much in words, good for the country).
    pub fn lines(&self, pack: &Pack) -> Vec<(String, String, bool)> {
        let mut v = Vec::new();
        let mut pct = |name: &str, x: f32, up_good: bool| {
            if (x - 1.0).abs() >= 0.005 {
                v.push((name.to_string(), format!("{:+.0}%", (x - 1.0) * 100.0), (x > 1.0) == up_good));
            }
        };
        pct("урожай и всё, что даёт земля", self.out_land, true);
        pct("выпуск заводов и рудников", self.out_industry, true);
        pct("выпуск военных заводов", self.out_military, true);
        pct("товары для людей", self.out_civil, true);
        pct("сила науки", self.science, true);
        pct("заводы осваивают образцы", self.learn, true);
        pct("цена снаряжения на заводах", self.make_cost, false);
        pct("огонь в наступлении", self.attack, true);
        pct("огонь в обороне", self.defend, true);
        pct("потери в бою", self.harm, false);
        pct("восстановление организации", self.org_back, true);
        pct("окапывание", self.dig, true);
        pct("опыт в боях", self.exp, true);
        pct("потери от голода и окружения", self.attrition, false);
        pct("сколько людей можно призвать", self.draft, true);
        if let Some((u, x)) = self.kind {
            pct(&format!("заводы осваивают: {}", pack.units.get(u).map_or("—", |x| x.name.as_str())), x, true);
        }
        if self.skill.abs() >= 0.005 {
            v.push(("предел навыка заводов".into(), format!("{:+.0}%", self.skill * 100.0), self.skill > 0.0));
        }
        let who = ["крестьяне", "рабочие", "образованные", "собственники"];
        for (i, a) in self.anger.iter().enumerate() {
            if a.abs() >= 0.001 {
                v.push((format!("недовольство: {}", who[i]), format!("{:+.1} за год", a * 12.0), *a < 0.0));
            }
        }
        // (a month's push to the government's standing fades by a twentieth a month: kept up, it
        // settles at twenty times itself)
        if self.stability.abs() >= 0.0005 {
            v.push(("устойчивость власти".into(), format!("{:+.0} пунктов", self.stability * 2000.0), self.stability > 0.0));
        }
        if self.leak.abs() >= 0.002 {
            v.push(("казна теряет".into(), format!("{:.0}% трат", self.leak * 100.0), false));
        }
        v
    }
}

/// What the world made of a country this month: the facts its features grow from.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    pub people: f32,
    /// Output a head against the world's mean; this year's growth of output, of prices.
    pub rich: f32,
    pub growth: f32,
    pub inflation: f32,
    /// Shares of the workers: on the land, in factories and mines, in arms.
    pub land_share: f32,
    pub industry_share: f32,
    pub arms_share: f32,
    /// Worst share of a vital need gone without, of any group; discontent; unrest anywhere.
    pub hunger: f32,
    pub anger: f32,
    pub unrest: bool,
    /// The land: hands it holds against those working it (below 1 — crowded).
    pub room: f32,
    /// Share of the learned among the people; the owners' share of incomes; goods from abroad.
    pub learned: f32,
    pub owners: f32,
    pub imports: f32,
    /// The treasury in months of the state's income; the debt in months of it.
    pub treasury: f32,
    pub debt: f32,
    /// Shares of the provinces held: mountains and hills, forests, marshes and jungles; by the sea.
    pub high: f32,
    pub wood: f32,
    pub coast: f32,
    pub provinces: usize,
    /// Provinces with each of the pack's resources; the least of them (a resource missing).
    pub deposits: Vec<usize>,
    /// Years since the country's last war, how it ended (Some(true) won); wars of the past century.
    pub last_war: Option<f32>,
    pub last_won: Option<bool>,
    pub wars: usize,
    /// Regions lost in the last 60 years (still held by others), regions taken in the last 40.
    pub lost: usize,
    pub taken: usize,
    /// Generals with a war behind them; the staff's pride in each kind; its doubt.
    pub veterans: usize,
    pub glory: Vec<f32>,
    pub doubt: f32,
    /// Men in arms against the people; the dead of the wars of the game against the people; the
    /// share of its people still in its hands; at war.
    pub drafted: f32,
    pub dead: f32,
    pub held: f32,
    pub at_war: bool,
    /// Share of the state's spending on the army.
    pub mil: f32,
    /// Provinces between the capital and the nearest province the rival holds.
    pub capital_depth: usize,
    /// Share of its provinces a railway runs through.
    pub rails: f32,
    /// Its people: the most famous living scientist's fame, whether a soldier rules it (a person
    /// of the world), what its rulers and ministers took of the treasury over the past (months of
    /// income); its stage of development against its rival's (above 0: ahead).
    pub genius: f32,
    pub soldier_rules: bool,
    pub graft: f32,
    pub stage_gap: f32,
    /// Its order of society: how pious and how lettered its people are, the share of them of the
    /// state's faith, how old its ruling house is (years; 0 — none), years since its government
    /// last fell (none: never in memory), whether generals rule it.
    pub piety: f32,
    pub literacy: f32,
    pub faith_share: f32,
    pub house_years: f32,
    pub fell: Option<f32>,
    pub junta: bool,
}

/// A feature as it stands in a country now.
#[derive(Clone, Debug)]
pub struct Spirit {
    /// Which (`DEFS`), and the kind of unit it is about (glory), if any.
    pub def: usize,
    pub arg: usize,
    /// Its strength now (0 to 1) and what the facts call for.
    pub level: f32,
    pub want: f32,
    /// The month it came.
    pub since: u32,
}

/// The features of both countries, and what each remembers of the months before.
#[derive(Clone, Debug, Default)]
pub struct Spirits {
    pub list: [Vec<Spirit>; 2],
    pub facts: [Facts; 2],
    /// Output and prices of the last twelve months (for growth and inflation).
    pub memory: [Vec<(f32, f32)>; 2],
    /// The wars of the game: the month each ended and whether the side won it (None: a draw).
    pub game_wars: [Vec<(u32, Option<bool>)>; 2],
    /// The dead of the last war of the game, for its weariness (millions).
    pub war_dead: [f32; 2],
}

/// How good a feature is for the country, as the player will feel it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tone {
    Good,
    Bad,
    Mixed,
}

/// A kind of feature: its name, its icon, how good it is; how much the facts call for it (0 to
/// 1); why it is there, in words; what it does at its full strength.
pub struct Def {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub tone: Tone,
    pub want: fn(&Facts, &Pack, usize) -> f32,
    pub why: fn(&Facts, &Pack, usize) -> String,
    pub eff: fn(&Facts, &Pack, usize) -> Eff,
}

impl Spirit {
    pub fn def(&self) -> &'static Def {
        &DEFS[self.def]
    }

    /// Its name (glory: of which kind).
    pub fn name(&self, pack: &Pack) -> String {
        if self.def().id == "glory" {
            format!("Культ: {}", pack.units.get(self.arg).map_or("—", |u| u.name.as_str()))
        } else {
            self.def().name.to_string()
        }
    }

    /// Its icon (glory: the kind's).
    pub fn icon(&self, pack: &Pack) -> String {
        if self.def().id == "glory" {
            pack.units.get(self.arg).map_or("medal".into(), |u| u.icon.clone())
        } else {
            self.def().icon.to_string()
        }
    }

    pub fn why(&self, f: &Facts, pack: &Pack) -> String {
        (self.def().why)(f, pack, self.arg)
    }

    /// What it does now (at its strength).
    pub fn eff(&self, f: &Facts, pack: &Pack) -> Eff {
        (self.def().eff)(f, pack, self.arg).at(self.level)
    }
}

impl Game {

    /// The features of both countries after the month: the facts gathered, each feature's
    /// strength following them, new ones coming and old ones going (with the news).
    pub fn spirits_month(&mut self) {
        let now = self.month();
        let pack = self.pack.clone();
        for s in 0..2 {
            // (the dead of the last war weigh less year by year: half in about four years)
            if !self.h.at_war {
                self.spirits.war_dead[s] *= 0.985;
            }
            let n = self.h.home(s);
            let mem = &mut self.spirits.memory[s];
            mem.push((n.gdp_real, n.level));
            if mem.len() > 13 {
                mem.remove(0);
            }
        }
        for s in 0..2 {
            let f = self.facts(s);
            // what the facts call for: every feature, and glory for each kind
            let mut wants: Vec<(usize, usize, f32)> = Vec::new();
            for (d, def) in DEFS.iter().enumerate() {
                if def.id == "glory" {
                    for u in 0..pack.units.len() {
                        wants.push((d, u, (def.want)(&f, &pack, u)));
                    }
                } else {
                    wants.push((d, 0, (def.want)(&f, &pack, 0)));
                }
            }
            // (opposites never stand together: the stronger call wins)
            let pairs = [("agrarian", "industrial"), ("learned", "illiterate"), ("victors", "defeat"), ("debt", "treasure"), ("boom", "slump"), ("poor", "wealthy"), ("sea", "autarky"), ("crowded", "vast"), ("veterans", "untested"), ("advanced", "backward")];
            for (a, b) in pairs {
                let ia = DEFS.iter().position(|d| d.id == a).unwrap();
                let ib = DEFS.iter().position(|d| d.id == b).unwrap();
                let wa = wants.iter().find(|w| w.0 == ia).map_or(0.0, |w| w.2);
                let wb = wants.iter().find(|w| w.0 == ib).map_or(0.0, |w| w.2);
                let lose = if wa >= wb { ib } else { ia };
                for w in wants.iter_mut().filter(|w| w.0 == lose) {
                    w.2 = 0.0;
                }
            }
            // (glory: only the proudest kind)
            let gd = glory_def();
            if let Some(best) = wants.iter().filter(|w| w.0 == gd).max_by(|a, b| a.2.total_cmp(&b.2)).map(|w| w.1) {
                for w in wants.iter_mut().filter(|w| w.0 == gd && w.1 != best) {
                    w.2 = 0.0;
                }
            }
            let me = self.me;
            let name = self.h.home(s).name.clone();
            let mut news: Vec<(String, bool)> = Vec::new();
            let list = &mut self.spirits.list[s];
            for &(d, u, want) in &wants {
                match list.iter_mut().position(|x| x.def == d && x.arg == u) {
                    Some(i) => {
                        let sp = &mut list[i];
                        sp.want = want;
                        sp.level += PACE * (want - sp.level);
                        if want < GONE && sp.level < GONE {
                            let nm = sp.name(&pack);
                            list.remove(i);
                            news.push((format!("ушло в прошлое: «{nm}»"), false));
                        }
                    }
                    None if want >= COME => {
                        // (at the start of a game the country is what it is: its features at full)
                        let first = now <= 1;
                        let sp = Spirit { def: d, arg: u, level: if first { want } else { PACE * want }, want, since: now };
                        if !first {
                            news.push((format!("новая черта страны: «{}»", sp.name(&pack)), true));
                        }
                        list.push(sp);
                    }
                    None => {}
                }
            }
            list.sort_by(|a, b| b.level.total_cmp(&a.level));
            self.spirits.facts[s] = f;
            for (line, hot) in news {
                if s == me {
                    self.news.push(crate::game::Note::new(now, &line, hot, "nation"));
                } else if hot {
                    self.news.push(crate::game::Note::new(now, &format!("{name}: {line}"), false, "politics"));
                }
            }
        }
    }

    /// What side `s`'s features do together now.
    pub fn spirit_eff(&self, s: usize) -> Eff {
        let f = &self.spirits.facts[s];
        self.spirits.list[s].iter().fold(Eff::default(), |a, sp| a.and(&sp.eff(f, &self.pack)))
    }

    /// The country's portrait in a few lines: its strongest features as a story.
    pub fn portrait_words(&self, s: usize) -> String {
        let f = &self.spirits.facts[s];
        let n = self.h.home(s);
        let size = match f.people {
            x if x < 5.0 => "малая",
            x if x < 40.0 => "средняя",
            x if x < 150.0 => "многолюдная",
            _ => "огромная",
        };
        let ground = if f.high > 0.3 {
            "горная"
        } else if f.wood > 0.35 {
            "лесная"
        } else if f.coast > 0.35 {
            "приморская"
        } else {
            "равнинная"
        };
        let way = if f.land_share > 0.55 {
            "живёт землёй"
        } else if f.industry_share > 0.3 {
            "живёт заводами"
        } else {
            "живёт и землёй, и ремеслом"
        };
        let mut text = format!("{} — {size} {ground} страна; {way}", n.name);
        match (f.last_war, f.last_won) {
            (Some(y), Some(true)) if y > 0.5 => text.push_str(&format!("; последнюю войну выиграла {}", ago_words(y))),
            (Some(y), Some(false)) if y > 0.5 => text.push_str(&format!("; последнюю войну проиграла {}", ago_words(y))),
            (Some(y), None) if y > 0.5 => text.push_str(&format!("; последняя война {} кончилась ничем", ago_words(y))),
            (Some(_), _) => text.push_str("; она воюет"),
            (None, _) => text.push_str("; войн на её памяти не было"),
        }
        if f.lost > 0 {
            text.push_str("; соседи держат её отнятые земли");
        }
        text.push('.');
        text
    }

    /// A war of the game is over: who won it, for the features that remember wars.
    pub fn war_ended(&mut self, loser: Option<usize>) {
        let now = self.month();
        let held = self.h.war.people();
        for s in 0..2 {
            let won = match loser {
                Some(l) => Some(l != s),
                None => {
                    let d = held[s] - held[1 - s];
                    if d > 0.03 {
                        Some(true)
                    } else if d < -0.03 {
                        Some(false)
                    } else {
                        None
                    }
                }
            };
            self.spirits.game_wars[s].push((now, won));
            self.spirits.war_dead[s] = self.h.dead[s];
        }
    }
}
