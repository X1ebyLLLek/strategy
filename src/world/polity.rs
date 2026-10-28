//! How each country is governed and what its people believe — grown, not written: its form of
//! government (from a chiefs' free host to an autocracy, a republic, a junta, a party state, a
//! theocracy), the great forces of its society (the nobility, the clergy, the officers, the
//! industrialists, the merchants, the learned, the workers, the peasants) with their power and their
//! mood, its laws (the order of labour and of land, who has a voice, the church, the press, the
//! schools, the service in arms), its faiths, the movements that rise in it (revanche, freethinkers,
//! levellers, guardians of the old ways, zealots, the party of power, separatists), how legitimate
//! its government is and how restless its people.
//!
//! Two hundred years of the world's past make all of it: faiths are founded, spread region by
//! region, split, are persecuted; reformers win laws, revolutions and coups change governments,
//! restorations bring the old ones back, houses die out; a lost war makes revanchists and
//! reformers, a won one makes guardians of the old ways; learning makes freethinkers, factories make
//! levellers. Nothing is chosen from a list for a country: what it is like at the game's beginning
//! is what happened to it. In the game the same forces go on (the game's side lays them on the
//! economy, the government's stability, the war and the bots' courses).

use crate::rng::Rng;

pub mod faith;
mod year;

pub use faith::Faith;
pub use year::{Life, Upheaval};

/// The forces of society.
pub const BLOCS: usize = 8;
pub const NOBLES: usize = 0;
pub const CLERGY: usize = 1;
pub const OFFICERS: usize = 2;
pub const CAPITAL: usize = 3;
pub const MERCHANTS: usize = 4;
pub const LEARNED: usize = 5;
pub const WORKERS: usize = 6;
pub const PEASANTS: usize = 7;

/// A force of society: its name, icon, what it is.
pub struct BlocDef {
    pub name: &'static str,
    pub icon: &'static str,
    pub about: &'static str,
}

pub const BLOC: [BlocDef; BLOCS] = [
    BlocDef { name: "Знать", icon: "crown", about: "Владельцы земли и старые роды: держатся за поместья, зависимых крестьян и сословные права." },
    BlocDef { name: "Духовенство", icon: "faith", about: "Служители веры: хотят, чтобы вера была государственной, а школы — при храмах." },
    BlocDef { name: "Офицерство", icon: "helmet", about: "Командиры армии: деньги армии, всеобщая служба, победы; поражения им не прощают." },
    BlocDef { name: "Промышленники", icon: "anvil", about: "Владельцы заводов и шахт: свободный наём, защита своего рынка, голос по цензу." },
    BlocDef { name: "Купечество", icon: "shop", about: "Торговля и банки: мир, открытые рынки, терпимость, свобода слова." },
    BlocDef { name: "Интеллигенция", icon: "graduate", about: "Учёные, врачи, учителя, пресса: свобода печати, школы для всех, наука, голос." },
    BlocDef { name: "Рабочие", icon: "strike", about: "Люди заводов и шахт: голос для всех, защита труда, хлеб по средствам." },
    BlocDef { name: "Крестьяне", icon: "wheat", about: "Люди земли: земля тем, кто её пашет, воля, поменьше податей и рекрутчины." },
];

/// The laws (institutions) of a country: three levels each.
pub const LAWS: usize = 7;
pub const LABOUR: usize = 0;
pub const LAND: usize = 1;
pub const VOICE: usize = 2;
pub const CHURCH: usize = 3;
pub const PRESS: usize = 4;
pub const SCHOOLS: usize = 5;
pub const SERVICE: usize = 6;

pub struct LawDef {
    pub name: &'static str,
    pub icon: &'static str,
    pub levels: [&'static str; 3],
    pub about: [&'static str; 3],
}

pub const LAW: [LawDef; LAWS] = [
    LawDef {
        name: "Труд",
        icon: "hand",
        levels: ["неволя", "зависимость", "вольный труд"],
        about: [
            "Работники — собственность хозяев: земля даёт дёшево, заводам некого нанять, люди бегут и бунтуют.",
            "Крестьяне привязаны к земле и господину: порядок на земле, но заводы ждут рук годами.",
            "Каждый нанимается, к кому хочет: заводы растут, знать теряет даровой труд.",
        ],
    },
    LawDef {
        name: "Земля",
        icon: "sprout",
        levels: ["поместья", "смешанное владение", "земля пахарю"],
        about: [
            "Почти вся земля — у знати и церкви: доход идёт владельцам, крестьяне злы.",
            "Часть земли выкуплена крестьянами: компромисс, которым никто не доволен до конца.",
            "Землёй владеют те, кто её пашет: крестьяне довольны и усердны, знать разорена.",
        ],
    },
    LawDef {
        name: "Голос",
        icon: "vote",
        levels: ["без представительства", "сословный и цензовый", "всеобщий"],
        about: [
            "Решает власть, страна молчит: недовольство копится, пока не прорвётся.",
            "Голос у сословий и собственников: богатые слышны, бедные — нет.",
            "Голос у всех: власть устойчивее, но должна угождать большинству.",
        ],
    },
    LawDef {
        name: "Вера и государство",
        icon: "faith",
        levels: ["государственная вера", "терпимость", "светское государство"],
        about: [
            "Одна вера — опора трона: духовенство сильно, иноверцы гонимы, наука под надзором.",
            "Своя вера главная, другие терпимы: мир между общинами.",
            "Вера — частное дело: духовенство слабеет, учёные свободны, ревнители злы.",
        ],
    },
    LawDef {
        name: "Печать",
        icon: "news",
        levels: ["цензура", "умеренная", "свобода печати"],
        about: [
            "Всё печатное проверяет власть: тихо, но учёные и вольнодумцы недовольны, ошибки власти не видны.",
            "Пишут почти обо всём, кроме власти и веры.",
            "Пишут обо всём: наука и торговля быстрее, власть под огнём критики.",
        ],
    },
    LawDef {
        name: "Школы",
        icon: "graduate",
        levels: ["при храмах", "для немногих", "для всех"],
        about: [
            "Учат грамоте при храмах, и не всех: страна темна, но послушна.",
            "Гимназии и университеты для тех, кто может платить: образованных мало, но они есть.",
            "Казна учит всех детей: дорого, плоды через поколение — грамотные рабочие, учёные, вольнодумцы.",
        ],
    },
    LawDef {
        name: "Служба",
        icon: "helmet",
        levels: ["наёмники и дружины", "кадровая армия", "всеобщая повинность"],
        about: [
            "Воюют те, кто за это получает: войско мало, но опытно; народ войны не чувствует.",
            "Постоянная армия и запас: служат по жребию.",
            "Служит каждый: огромный призыв, офицерство сильно, народ несёт тяготы.",
        ],
    },
];

/// What each force of society wants of each law: the level (9 — does not care) and how much.
pub const WANTS: [[(u8, f32); LAWS]; BLOCS] = [
    // labour, land, voice, church, press, schools, service
    [(0, 1.0), (0, 1.0), (1, 0.5), (0, 0.3), (0, 0.3), (9, 0.0), (0, 0.2)],
    [(9, 0.0), (0, 0.2), (1, 0.2), (0, 1.0), (0, 0.5), (0, 0.8), (9, 0.0)],
    [(9, 0.0), (9, 0.0), (0, 0.3), (9, 0.0), (0, 0.3), (9, 0.0), (2, 0.6)],
    [(2, 1.0), (1, 0.2), (1, 0.6), (9, 0.0), (1, 0.3), (1, 0.4), (1, 0.2)],
    [(2, 0.4), (9, 0.0), (1, 0.6), (1, 0.4), (2, 0.4), (9, 0.0), (0, 0.3)],
    [(2, 0.4), (9, 0.0), (2, 0.6), (2, 0.6), (2, 1.0), (2, 0.8), (9, 0.0)],
    [(2, 0.6), (9, 0.0), (2, 1.0), (9, 0.0), (2, 0.3), (2, 0.4), (0, 0.3)],
    [(2, 1.0), (2, 1.0), (2, 0.3), (0, 0.3), (9, 0.0), (9, 0.0), (0, 0.5)],
];

/// How a law at `level` sits with force `b`: −1 (hateful) … +1 (as it wants), 0 if it does not
/// care.
pub fn law_fit(b: usize, law: usize, level: u8) -> f32 {
    let (want, w) = WANTS[b][law];
    if want == 9 || w == 0.0 {
        return 0.0;
    }
    w * (1.0 - (level as f32 - want as f32).abs())
}

/// The forms of government.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Regime {
    Chiefs,
    Autocracy,
    Estates,
    Constitutional,
    Patricians,
    Republic,
    Junta,
    Party,
    Theocracy,
}

impl Regime {
    pub const ALL: [Regime; 9] = [
        Regime::Chiefs,
        Regime::Autocracy,
        Regime::Estates,
        Regime::Constitutional,
        Regime::Patricians,
        Regime::Republic,
        Regime::Junta,
        Regime::Party,
        Regime::Theocracy,
    ];

    pub fn index(&self) -> usize {
        Regime::ALL.iter().position(|r| r == self).unwrap_or(0)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Regime::Chiefs => "Вольница вождей",
            Regime::Autocracy => "Самодержавие",
            Regime::Estates => "Сословная монархия",
            Regime::Constitutional => "Конституционная монархия",
            Regime::Patricians => "Республика богатых",
            Regime::Republic => "Республика",
            Regime::Junta => "Военная диктатура",
            Regime::Party => "Партийное государство",
            Regime::Theocracy => "Теократия",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Regime::Chiefs => "horn",
            Regime::Autocracy => "crown",
            Regime::Estates => "castle",
            Regime::Constitutional => "scroll",
            Regime::Patricians => "coins",
            Regime::Republic => "vote",
            Regime::Junta => "helmet",
            Regime::Party => "fist",
            Regime::Theocracy => "faith",
        }
    }

    pub fn about(&self) -> &'static str {
        match self {
            Regime::Chiefs => "Сильнейшие вожди правят с дружинами, общины платят им долю. Власть держится на силе и удаче.",
            Regime::Autocracy => "Вся власть у монарха и его двора; знать, церковь и армия — его опора, остальных не спрашивают.",
            Regime::Estates => "Монарх правит вместе с сословиями: знать и духовенство заседают, торгуются и мешают.",
            Regime::Constitutional => "Монарх царствует, правит кабинет, отвечающий перед выборным собранием.",
            Regime::Patricians => "Правят те, у кого деньги: заводчики, купцы и банкиры выбирают из своих.",
            Regime::Republic => "Глава государства выбран; власть меняется на выборах, и каждый голос нужно заслужить.",
            Regime::Junta => "Власть у генералов: порядок, армия прежде всего, несогласных — под надзор.",
            Regime::Party => "Одна партия от имени трудящихся правит всем: план, дисциплина, чистки.",
            Regime::Theocracy => "Правят служители веры: закон — от писания, иноверцы и вольнодумцы гонимы.",
        }
    }

    /// How much each force's power counts in the government (who is heard).
    pub fn voice(&self) -> [f32; BLOCS] {
        match self {
            Regime::Chiefs => [1.0, 0.3, 1.2, 0.3, 0.4, 0.2, 0.2, 0.6],
            Regime::Autocracy => [0.8, 0.7, 0.8, 0.3, 0.3, 0.2, 0.1, 0.1],
            Regime::Estates => [1.0, 0.9, 0.6, 0.5, 0.6, 0.3, 0.1, 0.2],
            Regime::Constitutional => [0.7, 0.5, 0.5, 0.9, 0.9, 0.7, 0.4, 0.4],
            Regime::Patricians => [0.6, 0.3, 0.4, 1.0, 1.0, 0.5, 0.15, 0.15],
            Regime::Republic => [0.4, 0.4, 0.4, 0.8, 0.7, 0.9, 0.8, 0.8],
            Regime::Junta => [0.6, 0.4, 1.5, 0.6, 0.3, 0.1, 0.1, 0.1],
            Regime::Party => [0.05, 0.05, 0.8, 0.1, 0.1, 0.6, 1.0, 0.4],
            Regime::Theocracy => [0.6, 1.6, 0.4, 0.2, 0.3, 0.1, 0.2, 0.5],
        }
    }

    /// The ruler rules for life (a monarch, a dictator, a high priest), unless overthrown.
    pub fn lifelong(&self) -> bool {
        matches!(self, Regime::Chiefs | Regime::Autocracy | Regime::Estates | Regime::Junta | Regime::Party | Regime::Theocracy)
    }

    /// Years between elections, if the government is elected.
    pub fn elected(&self) -> Option<f32> {
        match self {
            Regime::Constitutional => Some(5.0),
            Regime::Patricians => Some(6.0),
            Regime::Republic => Some(4.0),
            _ => None,
        }
    }

    /// How hard it keeps down those who are against it (0..1): less unrest shows, the learned
    /// and the workers hate it.
    pub fn repression(&self) -> f32 {
        match self {
            Regime::Chiefs => 0.2,
            Regime::Autocracy => 0.5,
            Regime::Estates => 0.3,
            Regime::Constitutional => 0.15,
            Regime::Patricians => 0.2,
            Regime::Republic => 0.05,
            Regime::Junta => 0.7,
            Regime::Party => 0.8,
            Regime::Theocracy => 0.6,
        }
    }

    /// How readily it goes to war (1 — as anyone).
    pub fn war_ease(&self) -> f32 {
        match self {
            Regime::Chiefs => 1.3,
            Regime::Autocracy => 1.1,
            Regime::Estates => 1.0,
            Regime::Constitutional => 0.8,
            Regime::Patricians => 0.7,
            Regime::Republic => 0.6,
            Regime::Junta => 1.5,
            Regime::Party => 1.2,
            Regime::Theocracy => 1.2,
        }
    }

    /// A ruler of this government comes from among the generals.
    pub fn soldiers(&self) -> bool {
        matches!(self, Regime::Junta | Regime::Chiefs)
    }

    /// A monarchy (a ruling house).
    pub fn crowned(&self) -> bool {
        matches!(self, Regime::Autocracy | Regime::Estates | Regime::Constitutional)
    }
}

/// The movements that rise in a country.
pub const MOVES: usize = 7;
pub const REVANCHE: usize = 0;
pub const LIBERALS: usize = 1;
pub const LEVELLERS: usize = 2;
pub const GUARDIANS: usize = 3;
pub const ZEALOTS: usize = 4;
pub const HAWKS: usize = 5;
pub const SEPARATISTS: usize = 6;

pub struct MoveDef {
    pub name: &'static str,
    pub icon: &'static str,
    pub about: &'static str,
    /// The laws it fights for (law, level).
    pub aims: &'static [(usize, u8)],
}

pub const MOVE: [MoveDef; MOVES] = [
    MoveDef { name: "Реваншисты", icon: "fist", about: "Вернуть отнятые земли — войной, и поскорее.", aims: &[(SERVICE, 2)] },
    MoveDef { name: "Вольнодумцы", icon: "quill", about: "Свобода печати, голос, светское государство, власть по закону.", aims: &[(PRESS, 2), (VOICE, 2), (CHURCH, 2)] },
    MoveDef { name: "Уравнители", icon: "revolt", about: "Землю пахарю, волю работнику, голос каждому — и долой богачей.", aims: &[(LABOUR, 2), (LAND, 2), (VOICE, 2)] },
    MoveDef { name: "Охранители", icon: "castle", about: "Ничего не менять: старые законы, старая вера, твёрдая власть.", aims: &[] },
    MoveDef { name: "Ревнители веры", icon: "faith", about: "Вера превыше всего: государственная церковь, школы при храмах, война с иноверцами.", aims: &[(CHURCH, 0), (SCHOOLS, 0)] },
    MoveDef { name: "Державники", icon: "medal", about: "Сильная армия и сильная власть: служба для всех, порядок, слава.", aims: &[(SERVICE, 2), (PRESS, 0)] },
    MoveDef { name: "Отделенцы", icon: "flag", about: "Земли, взятые у соседей, и иноверные области хотят уйти.", aims: &[] },
];

/// A country's order: how it is governed and what its people believe.
#[derive(Clone, Debug)]
pub struct Polity {
    pub regime: Regime,
    /// The month the government took its form.
    pub since: i32,
    /// How rightful the people hold the government (0..1).
    pub legit: f32,
    /// Each force's power (shares, summing to 1) and mood (−1 hostile … +1 content).
    pub clout: [f32; BLOCS],
    pub mood: [f32; BLOCS],
    /// The level of each law (0..2).
    pub laws: [u8; LAWS],
    /// The faith the state holds its own (none: it holds none — a secular state keeps the faith of
    /// most).
    pub state_faith: usize,
    /// How pious the people are, how many can read (0..1).
    pub piety: f32,
    pub literacy: f32,
    /// Each movement's support (0..1).
    pub moves: [f32; MOVES],
    /// How restless the country is (0..1); above `RESTLESS` its government may fall.
    pub unrest: f32,
    /// The ruling house of a monarchy and when it came.
    pub house: Option<String>,
    pub house_since: i32,
    /// How far its economy has come from the land to the factories (0..1).
    pub industry: f32,
    /// What its government has lived through: reforms, revolutions, coups.
    pub reforms: u32,
    pub revolutions: u32,
    pub coups: u32,
    /// The month its government last fell (a revolution, a coup), if it ever did; the form it had
    /// before.
    pub fell: Option<i32>,
    pub before: Option<Regime>,
    /// Its story in lines (month, line), oldest first.
    pub story: Vec<(i32, String)>,
    /// Its development's drift over the past (log), for the world's economy.
    pub drift: f32,
}

/// The order of every country, the world's faiths and each region's faith.
#[derive(Clone, Debug, Default)]
pub struct Polities {
    pub of: Vec<Polity>,
    pub faiths: Vec<Faith>,
    /// The faith of each region (a state of the map; usize::MAX for the ice nobody lives on).
    pub region_faith: Vec<usize>,
    /// What the chronicle says of governments and faiths: (month, country, line).
    pub chronicle: Vec<(i32, usize, String)>,
}

/// Unrest above which a government may fall.
pub const RESTLESS: f32 = 0.55;

impl Polity {
    /// A country at the beginning of the deep past: a form of government by its development and
    /// luck, the old laws.
    pub fn first(dev: f32, faith: usize, month: i32, rng: &mut Rng) -> Polity {
        let r = rng.f32();
        let regime = if dev < 0.3 && r < 0.6 {
            Regime::Chiefs
        } else if r < 0.55 {
            Regime::Autocracy
        } else if r < 0.85 {
            Regime::Estates
        } else if r < 0.93 {
            Regime::Patricians
        } else {
            Regime::Theocracy
        };
        let mut laws = [0u8, 0, 0, 0, 0, 0, 0];
        laws[LABOUR] = if rng.f32() < 0.35 { 0 } else { 1 };
        laws[LAND] = if rng.f32() < 0.75 { 0 } else { 1 };
        laws[VOICE] = if regime == Regime::Estates || regime == Regime::Patricians { 1 } else { 0 };
        laws[CHURCH] = if rng.f32() < 0.8 { 0 } else { 1 };
        laws[PRESS] = 0;
        laws[SCHOOLS] = if dev > 0.6 && rng.f32() < 0.5 { 1 } else { 0 };
        laws[SERVICE] = if rng.f32() < 0.6 { 0 } else { 1 };
        if regime == Regime::Patricians {
            laws[LABOUR] = 2;
        }
        Polity {
            regime,
            since: month - (rng.f32() * 600.0) as i32,
            legit: 0.6 + 0.2 * rng.f32(),
            clout: [1.0 / BLOCS as f32; BLOCS],
            mood: [0.0; BLOCS],
            laws,
            state_faith: faith,
            piety: 0.75 + 0.2 * rng.f32(),
            literacy: (0.03 + 0.12 * dev).min(0.3),
            moves: [0.0; MOVES],
            unrest: 0.1,
            house: None,
            house_since: month,
            industry: 0.0,
            reforms: 0,
            revolutions: 0,
            coups: 0,
            fell: None,
            before: None,
            story: Vec::new(),
            drift: 0.0,
        }
    }

    /// The force with the most power in the government (its power weighted by its voice).
    pub fn heard(&self) -> [f32; BLOCS] {
        let v = self.regime.voice();
        let mut h = [0.0f32; BLOCS];
        for b in 0..BLOCS {
            h[b] = self.clout[b] * v[b];
        }
        let all: f32 = h.iter().sum::<f32>().max(1e-6);
        h.map(|x| x / all)
    }

    /// The forces in government: the heard ones, strongest first, until they hold over half.
    pub fn coalition(&self) -> Vec<usize> {
        let h = self.heard();
        let mut order: Vec<usize> = (0..BLOCS).collect();
        order.sort_by(|&a, &b| h[b].total_cmp(&h[a]));
        let mut out = Vec::new();
        let mut held = 0.0;
        for b in order {
            out.push(b);
            held += h[b];
            if held > 0.5 {
                break;
            }
        }
        out
    }

    /// The strongest movement and its support.
    pub fn top_move(&self) -> (usize, f32) {
        (0..MOVES).map(|m| (m, self.moves[m])).max_by(|a, b| a.1.total_cmp(&b.1)).unwrap_or((0, 0.0))
    }

    /// How content the society is with its government, weighted by power (−1 … +1).
    pub fn content(&self) -> f32 {
        (0..BLOCS).map(|b| self.clout[b] * self.mood[b]).sum()
    }

    /// The level of law `law` in words.
    pub fn law_word(&self, law: usize) -> &'static str {
        LAW[law].levels[self.laws[law].min(2) as usize]
    }
}

impl Polities {
    /// The faith followed by most of country `c`'s people (its regions'): the faith and its share.
    pub fn shares(&self, regions: &[(usize, f32)]) -> Vec<(usize, f32)> {
        let mut out: Vec<(usize, f32)> = Vec::new();
        let all: f32 = regions.iter().map(|r| r.1).sum::<f32>().max(1e-6);
        for &(st, pop) in regions {
            let f = self.region_faith.get(st).copied().unwrap_or(usize::MAX);
            if f == usize::MAX {
                continue;
            }
            match out.iter_mut().find(|x| x.0 == f) {
                Some(x) => x.1 += pop / all,
                None => out.push((f, pop / all)),
            }
        }
        out.sort_by(|a, b| b.1.total_cmp(&a.1));
        out
    }

    /// How far apart two faiths are (0 the same … 1 strangers): a branch of the same root is
    /// nearer.
    pub fn distance(&self, a: usize, b: usize) -> f32 {
        if a == b {
            return 0.0;
        }
        if a >= self.faiths.len() || b >= self.faiths.len() {
            return 1.0;
        }
        if self.root(a) == self.root(b) {
            0.5
        } else {
            1.0
        }
    }

    /// The first faith of the line faith `f` came from.
    pub fn root(&self, mut f: usize) -> usize {
        for _ in 0..64 {
            match self.faiths.get(f).and_then(|x| x.parent) {
                Some(p) => f = p,
                None => break,
            }
        }
        f
    }

    /// How readily country `a` would begin a war against `b`: its government's form, its
    /// zealots against a neighbour of another faith, its revanchists and hawks.
    pub fn war_lust(&self, a: usize, b: usize) -> f32 {
        let (pa, pb) = (&self.of[a], &self.of[b]);
        let apart = self.distance(pa.state_faith, pb.state_faith);
        let zeal = self.faiths.get(pa.state_faith).map_or(0.0, |f| f.tenets[faith::WAR].max(0.0));
        let holy = 1.0 + apart * (0.6 * pa.moves[ZEALOTS] + 0.8 * zeal * pa.piety);
        let party = 1.0 + 0.6 * pa.moves[HAWKS] + 0.4 * pa.moves[REVANCHE];
        // (a free people with a voice is slow to war; a restless one may be sent to one)
        let calm = 1.0 - 0.3 * (pa.laws[VOICE] as f32 / 2.0) * (1.0 - pa.unrest);
        (pa.regime.war_ease() * holy * party * calm).clamp(0.2, 4.0)
    }

    /// What a country's order does to its science (a multiplier): readers, the press, the schools,
    /// a faith that loves or fears learning.
    pub fn science_k(&self, c: usize) -> f32 {
        let p = &self.of[c];
        let learning = self.faiths.get(p.state_faith).map_or(0.0, |f| f.tenets[faith::LEARNING]);
        let press = [0.88, 1.0, 1.1][p.laws[PRESS].min(2) as usize];
        let church = if p.laws[CHURCH] == 0 && learning < 0.0 { 0.92 } else { 1.0 };
        ((0.55 + 0.9 * p.literacy) * press * church * (1.0 + 0.15 * learning * p.piety)).clamp(0.4, 1.8)
    }
}
