//! The world's people: those whose names are remembered — generals, scientists, economists,
//! statesmen — each born into a country and a class, schooled as his class and country allowed
//! (and by a teacher, if a great one took him), believing what the years of his youth taught him (a
//! lost war makes reformers and men of revenge, a victory makes men of the old ways, hunger makes
//! believers in the state), gifted or not by luck alone. They rise to posts (the ruler appoints,
//! by ability if he is able and honest, by loyalty if not), do what their gifts and beliefs let
//! them do — win battles, find things out, reform or rob the treasury, start wars — make friends,
//! pupils and rivals, and die. A country may have a genius in every sphere, or none at all.
//!
//! Everything that makes a person what he is stays hidden (talent, education, experience, honesty,
//! beliefs): anyone may know only his life — where he came from, his posts, his deeds, his ties,
//! his fame. The game's advisers, heads of science and generals are these people.

use crate::economy::advisor::Mind;
use crate::military::front::General;
use crate::pack::Pack;
use crate::rng::Rng;
use crate::world::names;
use crate::world::progress::Know;

mod life;
mod posts;

/// What a person is drawn to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Calling {
    General,
    Scientist,
    Economist,
    Statesman,
}

impl Calling {
    pub const ALL: [Calling; 4] = [Calling::General, Calling::Scientist, Calling::Economist, Calling::Statesman];

    pub fn word(&self) -> &'static str {
        match self {
            Calling::General => "военный",
            Calling::Scientist => "учёный",
            Calling::Economist => "экономист",
            Calling::Statesman => "политик",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Calling::General => "helmet",
            Calling::Scientist => "science",
            Calling::Economist => "coins",
            Calling::Statesman => "crown",
        }
    }
}

/// A post a person may hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Post {
    /// The head of the state (or of its government).
    Ruler,
    /// The one who runs the economy.
    Economy,
    /// The head of the laboratories.
    Science,
    /// The chief of the general staff.
    Staff,
    /// A general in command of troops.
    Command,
    /// A scientist with a chair and pupils.
    Chair,
}

impl Post {
    pub fn word(&self) -> &'static str {
        match self {
            Post::Ruler => "глава государства",
            Post::Economy => "глава хозяйства",
            Post::Science => "глава науки",
            Post::Staff => "начальник генштаба",
            Post::Command => "командующий",
            Post::Chair => "профессор",
        }
    }
}

/// How two people are tied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tie {
    Teacher,
    Pupil,
    Comrade,
    Rival,
    Patron,
    Protege,
    Parent,
    Child,
}

impl Tie {
    pub fn word(&self) -> &'static str {
        match self {
            Tie::Teacher => "учитель",
            Tie::Pupil => "ученик",
            Tie::Comrade => "сослуживец",
            Tie::Rival => "соперник",
            Tie::Patron => "покровитель",
            Tie::Protege => "выдвиженец",
            Tie::Parent => "отец",
            Tie::Child => "сын",
        }
    }

    fn back(&self) -> Tie {
        match self {
            Tie::Teacher => Tie::Pupil,
            Tie::Pupil => Tie::Teacher,
            Tie::Comrade => Tie::Comrade,
            Tie::Rival => Tie::Rival,
            Tie::Patron => Tie::Protege,
            Tie::Protege => Tie::Patron,
            Tie::Parent => Tie::Child,
            Tie::Child => Tie::Parent,
        }
    }
}

/// A post held, from a month to a month (None: still).
#[derive(Clone, Debug)]
pub struct Stint {
    pub post: Post,
    pub from: i32,
    pub to: Option<i32>,
}

/// Something a person did, as anyone may know it.
#[derive(Clone, Debug)]
pub struct Deed {
    pub month: i32,
    pub text: String,
}

/// A person of the world. Months are counted from the game's beginning (the past below 0).
#[derive(Clone, Debug)]
pub struct Person {
    pub id: u32,
    pub name: String,
    /// The map country he serves.
    pub country: usize,
    pub born: i32,
    pub died: Option<i32>,
    /// What his family was: 0 the people of the land, 1 workers, 2 the learned, 3 owners.
    pub origin: u8,
    pub calling: Calling,
    // hidden
    pub talent: f32,
    pub education: f32,
    pub experience: f32,
    pub honesty: f32,
    /// −1: owners know best … +1: the state should plan and build.
    pub state: f32,
    /// −1: peace at any price … +1: war is the way.
    pub hawk: f32,
    /// −1: the old ways … +1: reform.
    pub reform: f32,
    /// A general's daring (−1 cautious … +1 bold).
    pub bold: f32,
    /// The sphere he loves: a kind of units, or the economy (`pack.units.len()`).
    pub love: usize,
    // public
    pub fame: f32,
    pub posts: Vec<Stint>,
    pub deeds: Vec<Deed>,
    pub ties: Vec<(u32, Tie)>,
    /// A general's battles won and lost, provinces taken, wars fought; a scientist's discoveries.
    pub won: u32,
    pub lost: u32,
    pub took: u32,
    pub wars: u32,
    pub works: u32,
    /// How he died, in words (when he did).
    pub end: String,
}

impl Person {
    pub fn alive(&self) -> bool {
        self.died.is_none()
    }

    /// His age in years at month `now`.
    pub fn age(&self, now: i32) -> f32 {
        (self.died.unwrap_or(now).min(now) - self.born) as f32 / 12.0
    }

    /// The post he holds now, if any (the highest).
    pub fn post(&self) -> Option<Post> {
        self.posts.iter().filter(|s| s.to.is_none()).map(|s| s.post).min_by_key(|p| *p as u8)
    }

    /// What an adviser's work depends on (`advisor::Mind`): his education, experience, talent and
    /// beliefs.
    pub fn mind(&self) -> Mind {
        Mind::new(self.education, self.experience, self.talent, self.honesty, self.state)
    }

    /// He as a general of the war on the map.
    pub fn general(&self) -> General {
        General {
            name: self.name.clone(),
            education: self.education.clamp(0.05, 0.95),
            experience: self.experience.clamp(0.0, 0.95),
            talent: self.talent.clamp(0.05, 0.95),
            bold: self.bold.clamp(-1.0, 1.0),
            won: self.won,
            lost: self.lost,
            took: self.took,
            past: self.deeds.iter().rev().take(4).rev().map(|d| d.text.clone()).collect(),
        }
    }

    /// His fame in a word.
    pub fn fame_word(&self) -> &'static str {
        match self.fame {
            f if f < 0.25 => "безвестен",
            f if f < 0.7 => "известен немногим",
            f if f < 1.4 => "известен",
            f if f < 2.4 => "знаменит",
            _ => "легенда",
        }
    }

    /// Where he came from, in words.
    pub fn origin_word(&self) -> &'static str {
        match self.origin {
            0 => "из простой семьи",
            1 => "из семьи мастеровых",
            2 => "из образованной семьи",
            _ => "из состоятельной семьи",
        }
    }

    /// What his schooling looks like from outside (the truth of it is hidden: a self-taught man may
    /// know more than a graduate).
    pub fn schooling_word(&self) -> &'static str {
        match self.education {
            e if e < 0.25 => "почти без образования",
            e if e < 0.5 => "учился немного",
            e if e < 0.75 => "получил хорошее образование",
            _ => "блестяще образован",
        }
    }

    /// His life as anyone may read it: one line each.
    pub fn life(&self, folk: &Folk, now: i32) -> Vec<String> {
        let mut v = Vec::new();
        v.push(format!("{}, {}; {}", self.calling.word(), self.origin_word(), self.schooling_word()));
        for s in &self.posts {
            let when = match s.to {
                Some(to) => format!("{}–{}", year_word(s.from), year_word(to)),
                None => format!("с {}", year_word(s.from)),
            };
            v.push(format!("{} ({when})", s.post.word()));
        }
        for d in self.deeds.iter().rev().take(8).rev() {
            v.push(format!("{}: {}", year_word(d.month), d.text));
        }
        for &(o, t) in self.ties.iter().take(6) {
            if let Some(p) = folk.people.get(o as usize) {
                v.push(format!("{}: {}{}", t.word(), p.name, if p.alive() { "" } else { " (†)" }));
            }
        }
        match self.died {
            Some(d) => v.push(format!("умер {} в {} лет{}", year_word(d), self.age(d) as u32, if self.end.is_empty() { String::new() } else { format!(" — {}", self.end) })),
            None => v.push(format!("{} лет", self.age(now) as u32)),
        }
        v
    }

    fn deed(&mut self, month: i32, text: String, fame: f32) {
        self.deeds.push(Deed { month, text });
        self.fame += fame;
    }
}

/// A month in words: "в 1-м году игры", "за 23 года до игры".
pub fn year_word(m: i32) -> String {
    if m < 0 {
        let y = ((-m) as f32 / 12.0).round().max(1.0) as usize;
        format!("за {y} {} до начала", crate::world::names::plural(y, "год", "года", "лет"))
    } else {
        format!("{}-й год", m / 12 + 1)
    }
}

/// The posts of a country and who holds them.
#[derive(Clone, Debug, Default)]
pub struct Offices {
    pub ruler: Option<u32>,
    pub economy: Option<u32>,
    pub science: Option<u32>,
    pub staff: Option<u32>,
    pub command: Vec<u32>,
    pub chairs: Vec<u32>,
    /// When the ruler came and how long he means to stay (months).
    pub ruler_since: i32,
    pub ruler_term: i32,
}

/// What a country lives through in a year, for its people.
#[derive(Clone, Debug, Default)]
pub struct Times {
    /// People (millions), development (0..1).
    pub pop: f32,
    pub dev: f32,
    pub at_war: bool,
    /// Years since its last war ended, and whether it won (None: a draw); None — no war remembered.
    pub last_war: Option<(f32, Option<bool>)>,
    /// A neighbour holds land it lost.
    pub grudge: bool,
    /// Its stage of development.
    pub stage: usize,
    /// Its staff's pride in each kind of units.
    pub glory: Vec<f32>,
    /// How its government is made: the ruler rules for life (a monarch, a dictator); years an
    /// elected one serves (0: as it happens); the ruler comes from the generals; a crown passes from
    /// father to son.
    pub lifelong: bool,
    pub term: f32,
    pub soldiers: bool,
    pub crowned: bool,
}

/// Everybody the world remembers, and the posts of every country.
#[derive(Clone, Debug, Default)]
pub struct Folk {
    pub people: Vec<Person>,
    pub offices: Vec<Offices>,
    /// What the chronicle says of people: (month, country, line).
    pub chronicle: Vec<(i32, usize, String)>,
    /// Each country: what its economists did to its productivity (log), what they and the rulers
    /// took from the treasury (months of income, all told), what its rulers did for schools (log
    /// of the power of its science).
    pub econ: Vec<f32>,
    pub taken: Vec<f32>,
    pub schools: Vec<f32>,
    /// The player's country (in the game): its government is his — the world fills neither its
    /// ruler's seat nor its ministries of economy and science.
    pub player: Option<usize>,
    /// Each country's last change of ruler not yet seen by its order (`polity`): the new ruler's
    /// surname and whether he is the old one's son.
    pub changed: Vec<Option<(String, bool)>>,
}

impl Folk {
    pub fn new(countries: usize) -> Folk {
        Folk { offices: vec![Offices::default(); countries], econ: vec![0.0; countries], taken: vec![0.0; countries], schools: vec![0.0; countries], changed: vec![None; countries], ..Default::default() }
    }

    pub fn get(&self, id: u32) -> &Person {
        &self.people[id as usize]
    }

    /// The living of country `c` drawn to `calling`, the most famous first.
    pub fn living(&self, c: usize, calling: Calling) -> Vec<u32> {
        let mut v: Vec<u32> = self.people.iter().filter(|p| p.country == c && p.alive() && p.calling == calling).map(|p| p.id).collect();
        v.sort_by(|&a, &b| self.get(b).fame.total_cmp(&self.get(a).fame));
        v
    }

    /// All the living of country `c`, the most famous first.
    pub fn all_living(&self, c: usize) -> Vec<u32> {
        let mut v: Vec<u32> = self.people.iter().filter(|p| p.country == c && p.alive()).map(|p| p.id).collect();
        v.sort_by(|&a, &b| self.get(b).fame.total_cmp(&self.get(a).fame));
        v
    }

    /// The dead of country `c` whose fame outlived them, the most famous first.
    pub fn great_dead(&self, c: usize, least: f32) -> Vec<u32> {
        let mut v: Vec<u32> = self.people.iter().filter(|p| p.country == c && !p.alive() && p.fame >= least).map(|p| p.id).collect();
        v.sort_by(|&a, &b| self.get(b).fame.total_cmp(&self.get(a).fame));
        v
    }

    /// A person of country `c` by name.
    pub fn find(&self, c: usize, name: &str) -> Option<u32> {
        self.people.iter().find(|p| p.country == c && p.name == name).map(|p| p.id)
    }

    /// The holder of `post` in country `c`.
    pub fn holder(&self, c: usize, post: Post) -> Option<u32> {
        let o = &self.offices[c];
        match post {
            Post::Ruler => o.ruler,
            Post::Economy => o.economy,
            Post::Science => o.science,
            Post::Staff => o.staff,
            Post::Command => o.command.first().copied(),
            Post::Chair => o.chairs.first().copied(),
        }
    }

    fn tie(&mut self, a: u32, b: u32, t: Tie) {
        if a == b {
            return;
        }
        if !self.people[a as usize].ties.iter().any(|x| x.0 == b && x.1 == t) {
            self.people[a as usize].ties.push((b, t));
            self.people[b as usize].ties.push((a, t.back()));
        }
    }

}

/// Whether technique `t` belongs to the sphere a person loves (a kind of units, or the economy).
pub fn loves(pack: &Pack, t: usize, love: usize) -> bool {
    let tech = &pack.techs[t];
    if love >= pack.units.len() {
        tech.sphere != crate::pack::Sphere::Military
    } else {
        tech.units.iter().any(|x| x.0 == love) || tech.unlocks.contains(&love)
    }
}

