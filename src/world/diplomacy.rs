//! Relations between the world's countries — grown, not written: what each thinks of each other
//! (their faiths, their forms of government, lands one holds of the other's, wars fought against
//! or beside each other, ruling houses married, trade, a common enemy, the fear of a much stronger
//! neighbour, an ally that did not come), and the pacts they make of it: alliances, pacts of
//! non-aggression, trade agreements, truces after wars, guarantees of a weak country by a strong
//! one, tribute of the beaten, marriages of ruling houses. A pact is made when both sides gain by
//! it and broken when they no longer do. Wars are fought by coalitions: the attacked calls its
//! allies and guarantors, the attacker its own allies who hate the attacked; who does not come when
//! called is remembered.
//!
//! The world's past (`history`) makes four hundred years of it; the game goes on by the same rules
//! (and the player's government makes and breaks pacts of its own, the others answering by them).

use crate::rng::Rng;
use crate::world::polity::Regime;

/// The kinds of pacts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A defensive alliance: each comes when the other is attacked (and may join its wars).
    Alliance,
    /// A pact of non-aggression for some years.
    Peace,
    /// A trade agreement: no duties on goods between them, fewer formalities.
    Trade,
    /// A truce after a war: no new war between them for some years.
    Truce,
    /// A strong country promises to defend a weak one (`a` the strong).
    Guarantee,
    /// A beaten country pays tribute to its victor (`a` the victor).
    Tribute,
    /// The ruling houses of two monarchies married.
    Marriage,
}

impl Kind {
    pub const ALL: [Kind; 7] = [Kind::Alliance, Kind::Peace, Kind::Trade, Kind::Truce, Kind::Guarantee, Kind::Tribute, Kind::Marriage];

    pub fn name(&self) -> &'static str {
        match self {
            Kind::Alliance => "Союз",
            Kind::Peace => "Договор о ненападении",
            Kind::Trade => "Торговый договор",
            Kind::Truce => "Перемирие",
            Kind::Guarantee => "Гарантия независимости",
            Kind::Tribute => "Дань",
            Kind::Marriage => "Брак правящих домов",
        }
    }

    pub fn about(&self) -> &'static str {
        match self {
            Kind::Alliance => "Оборонительный союз: напавший на одного воюет с обоими. Союзник шлёт деньги и закрывает врагу свои рынки; может позвать и в свою войну.",
            Kind::Peace => "Обе страны обещают не нападать друг на друга, пока договор в силе. Нарушивший прослывёт вероломным.",
            Kind::Trade => "Товары между странами идут без пошлин и с меньшими хлопотами — купцы везут больше; доход от пошлин на этом пути теряется.",
            Kind::Truce => "Мир после войны: несколько лет ни одна из сторон не может начать новую.",
            Kind::Guarantee => "Сильная страна обещает защищать слабую: на слабую нападут, только не боясь сильной.",
            Kind::Tribute => "Побеждённая платит победительнице часть доходов казны, пока не окрепнет настолько, чтобы отказаться.",
            Kind::Marriage => "Правящие дома породнились: дворы благосклонны друг к другу; пресечётся один дом — другой может потребовать престол.",
        }
    }

    /// Years such a pact runs before it is renewed or lapses.
    pub fn years(&self) -> u32 {
        match self {
            Kind::Alliance => 25,
            Kind::Peace => 10,
            Kind::Trade => 20,
            Kind::Truce => 12,
            Kind::Guarantee => 20,
            Kind::Tribute => 15,
            Kind::Marriage => 30,
        }
    }

    /// The pact binds both alike (else `a` is the strong side: the guarantor, the victor).
    pub fn mutual(&self) -> bool {
        !matches!(self, Kind::Guarantee | Kind::Tribute)
    }

    fn code(&self) -> u64 {
        Kind::ALL.iter().position(|k| k == self).unwrap_or(0) as u64
    }
}

/// A pact between `a` and `b`, from month `since` to month `until` (then renewed if both still
/// want it, else it lapses).
#[derive(Clone, Debug)]
pub struct Pact {
    pub kind: Kind,
    pub a: usize,
    pub b: usize,
    pub since: i32,
    pub until: i32,
}

impl Pact {
    pub fn has(&self, c: usize) -> bool {
        self.a == c || self.b == c
    }

    pub fn other(&self, c: usize) -> usize {
        if self.a == c {
            self.b
        } else {
            self.a
        }
    }
}

/// A war of the world: the attacker's side and the defender's (the first of each is the one that
/// began it and the one it attacked), when it began, why, how it goes (−1 … +1, the attacker
/// winning above 0) and how weary each side is of it (0 … 1).
#[derive(Clone, Debug)]
pub struct War {
    pub sides: [Vec<usize>; 2],
    pub since: i32,
    pub cause: String,
    pub score: f32,
    pub weary: [f32; 2],
}

impl War {
    /// The side `c` fights on, if it fights.
    pub fn side_of(&self, c: usize) -> Option<usize> {
        (0..2).find(|&s| self.sides[s].contains(&c))
    }
}

/// What makes up an opinion: the reasons, in the order of `WHY`.
pub const WHY: usize = 11;
pub const FAITH: usize = 0;
pub const GOVERN: usize = 1;
pub const LANDS: usize = 2;
pub const ENEMIES: usize = 3;
pub const COMRADES: usize = 4;
pub const HOUSE: usize = 5;
pub const TRADE: usize = 6;
pub const FOE: usize = 7;
pub const FEAR: usize = 8;
pub const BETRAYAL: usize = 9;
pub const ENVOYS: usize = 10;
pub const WHY_NAME: [&str; WHY] = ["Вера", "Строй", "Земли и граница", "Войны друг с другом", "Воевали вместе", "Родство домов", "Торговля", "Общий враг", "Страх перед силой", "Вероломство", "Посольства и дары"];

/// What the world is like this year, for its relations: every country's form, power, neighbours,
/// faith, ruling house, ruler and people.
pub struct Scene<'a> {
    pub names: &'a [String],
    pub live: Vec<bool>,
    /// Each country's might (its economy's size).
    pub power: Vec<f32>,
    pub neighbours: Vec<Vec<usize>>,
    pub regime: Vec<Regime>,
    pub house: Vec<Option<String>>,
    /// How far apart two countries' faiths are (0 the same … 1 strangers).
    pub apart: &'a dyn Fn(usize, usize) -> f32,
    /// How much of country `a`'s lost land `b` holds, fading with the years (0 … 1).
    pub holds: &'a dyn Fn(usize, usize) -> f32,
    /// Each country's people of trade (the clout of merchants and industrialists, 0 … 1), its
    /// zealots, its hawks and its ruler's warlikeness (−1 … +1).
    pub trade: Vec<f32>,
    pub zeal: Vec<f32>,
    pub hawk: Vec<f32>,
}

/// Opinion moves this share of the way to what the year makes it.
const PACE: f32 = 0.3;
/// The memory of a war against each other, of a war fought side by side, of an ally that did not
/// come, fades over so many years.
const MEMORY: f32 = 40.0;
const BETRAYAL_YEARS: f32 = 50.0;
/// A country keeps at most so many alliances, pacts of non-aggression, trade agreements (more
/// with more people of trade), marriages of its house; guarantees so many weak countries, and is
/// guaranteed by so many strong ones.
const ALLIANCES: usize = 2;
const PEACES: usize = 3;
const TRADES: f32 = 2.0;
const TRADES_BY_TRADE: f32 = 8.0;
const MARRIAGES: usize = 2;
const GUARANTEES: usize = 2;
const GUARDED: usize = 1;

#[derive(Clone, Debug)]
pub struct Diplomacy {
    pub n: usize,
    /// What each country thinks of each other (−1 … +1), [a·n + b]: a's of b; and why (the parts
    /// it moves toward).
    pub opinion: Vec<f32>,
    pub why: Vec<[f32; WHY]>,
    pub pacts: Vec<Pact>,
    pub wars: Vec<War>,
    /// [a·n + b]: the last month they fought each other, the last month they fought side by side,
    /// the month b did not come when a called (i32::MIN: never).
    pub fought: Vec<i32>,
    pub beside: Vec<i32>,
    pub betrayed: Vec<i32>,
    /// What happened between countries: the month, the country it is told of, the words.
    pub chronicle: Vec<(i32, usize, String)>,
    /// The country a player governs (its pacts are made by him: what the others would make with
    /// it comes to him as an offer), and the offers waiting: the kind, the country offering.
    pub player: Option<usize>,
    pub offers: Vec<(Kind, usize)>,
    /// [a·n + b]: what envoys and gifts of b did to a's opinion of it (fading).
    pub warmth: Vec<f32>,
}

fn fade(month: i32, then: i32, years: f32) -> f32 {
    if then == i32::MIN {
        return 0.0;
    }
    (1.0 - (month - then) as f32 / 12.0 / years).max(0.0)
}

/// How near two forms of government stand (−1 … +1): kings are kin to kings, elected governments
/// to elected ones; the party state is the enemy of crowns and of the rich.
pub fn kinship(a: Regime, b: Regime) -> f32 {
    use Regime::*;
    let elected = |r: Regime| r.elected().is_some();
    let mut k = if a == b { 0.08 } else { 0.0 };
    if a.crowned() && b.crowned() {
        k += 0.05;
    }
    if elected(a) && elected(b) {
        k += 0.1;
    }
    let red = |x: Regime, y: Regime| x == Party && (y.crowned() || y == Patricians || y == Theocracy);
    if red(a, b) || red(b, a) {
        k -= 0.35;
    }
    if (a == Republic && b == Autocracy) || (a == Autocracy && b == Republic) {
        k -= 0.12;
    }
    if (a == Junta) != (b == Junta) && (a == Republic || b == Republic) {
        k -= 0.08;
    }
    k
}

impl Diplomacy {
    pub fn new(n: usize) -> Diplomacy {
        Diplomacy {
            n,
            opinion: vec![0.0; n * n],
            why: vec![[0.0; WHY]; n * n],
            pacts: Vec::new(),
            wars: Vec::new(),
            fought: vec![i32::MIN; n * n],
            beside: vec![i32::MIN; n * n],
            betrayed: vec![i32::MIN; n * n],
            chronicle: Vec::new(),
            player: None,
            offers: Vec::new(),
            warmth: vec![0.0; n * n],
        }
    }

    pub fn of(&self, a: usize, b: usize) -> f32 {
        if a == b || a >= self.n || b >= self.n {
            return 0.0;
        }
        self.opinion[a * self.n + b]
    }

    /// The pact of kind `kind` between `a` and `b` (for a guarantee or tribute, `a` the strong
    /// side, or either way when `either`).
    pub fn pact(&self, kind: Kind, a: usize, b: usize) -> Option<&Pact> {
        self.pacts.iter().find(|p| p.kind == kind && ((p.a == a && p.b == b) || ((kind.mutual()) && p.a == b && p.b == a)))
    }

    pub fn has(&self, kind: Kind, a: usize, b: usize) -> bool {
        self.pact(kind, a, b).is_some()
    }

    /// Any pact of `kind` binding `a` and `b`, whichever is the strong side.
    pub fn either(&self, kind: Kind, a: usize, b: usize) -> bool {
        self.has(kind, a, b) || self.has(kind, b, a)
    }

    /// Country `c`'s allies.
    pub fn allies(&self, c: usize) -> Vec<usize> {
        self.pacts.iter().filter(|p| p.kind == Kind::Alliance && p.has(c)).map(|p| p.other(c)).collect()
    }

    /// The strong countries that guarantee `c` (and its overlord, which defends its tributary).
    pub fn guardians(&self, c: usize) -> Vec<usize> {
        self.pacts.iter().filter(|p| matches!(p.kind, Kind::Guarantee | Kind::Tribute) && p.b == c).map(|p| p.a).collect()
    }

    /// Every pact `c` is party to.
    pub fn pacts_of(&self, c: usize) -> Vec<&Pact> {
        self.pacts.iter().filter(|p| p.has(c)).collect()
    }

    /// The war `c` fights, if it fights one.
    pub fn war_of(&self, c: usize) -> Option<usize> {
        self.wars.iter().position(|w| w.side_of(c).is_some())
    }

    /// Whether `a` and `b` are at war with each other.
    pub fn enemies(&self, a: usize, b: usize) -> bool {
        self.wars.iter().any(|w| matches!((w.side_of(a), w.side_of(b)), (Some(x), Some(y)) if x != y))
    }

    /// Why `a` will not attack `b` now, if it will not: they are allies, at a truce, bound by a pact
    /// of non-aggression (which a warlike government may break: `bold` — how warlike, 0 … 1).
    pub fn restraint(&self, a: usize, b: usize, bold: f32) -> Option<&'static str> {
        if self.either(Kind::Alliance, a, b) {
            Some("союзники")
        } else if self.has(Kind::Truce, a, b) {
            Some("перемирие")
        } else if self.has(Kind::Peace, a, b) && bold < 0.8 {
            Some("договор о ненападении")
        } else if self.has(Kind::Tribute, b, a) || self.has(Kind::Guarantee, a, b) {
            Some("сама обещала ей защиту")
        } else if self.war_of(a).is_some() || self.war_of(b).is_some() && !self.enemies(a, b) {
            Some("уже воюет")
        } else {
            None
        }
    }

    /// Who would fight if `a` attacked `b`: the attacker's side (`a` and its allies who hate `b`
    /// and are bound to it by nothing), the defender's (`b`, its allies, its guarantors, its
    /// overlord — those bound to `a` by nothing). Those at war already stay out.
    pub fn coalition(&self, a: usize, b: usize) -> [Vec<usize>; 2] {
        let free = |x: usize| self.war_of(x).is_none();
        let bound = |x: usize, y: usize| self.either(Kind::Alliance, x, y) || self.has(Kind::Truce, x, y) || self.has(Kind::Peace, x, y);
        let mut def = vec![b];
        for x in self.allies(b).into_iter().chain(self.guardians(b)) {
            if x != a && !def.contains(&x) && free(x) && !bound(x, a) {
                def.push(x);
            }
        }
        // (neighbours of a much stronger attacker who fear and hate it stand with its victim: the
        // balance of power)
        for x in 0..self.n {
            if x != a && x != b && !def.contains(&x) && free(x) && !bound(x, a) && self.why[x * self.n + a][FEAR] < -0.15 && self.of(x, a) < -0.35 && self.of(x, b) > -0.1 {
                def.push(x);
            }
        }
        let mut att = vec![a];
        for x in self.allies(a) {
            if x != b && !def.contains(&x) && !att.contains(&x) && free(x) && !bound(x, b) && self.of(x, b) < -0.25 {
                att.push(x);
            }
        }
        [att, def]
    }

    /// Whether `x`, called by `to`, comes to its war (a friend comes more surely; luck decides).
    pub fn comes(&self, x: usize, to: usize, rng: &mut Rng) -> bool {
        rng.f32() < 0.5 + 0.55 * self.of(x, to)
    }

    /// Begin a war of `sides` (each side's principal first) in `month` for `cause`; those called
    /// who do not come are remembered and their pacts with the caller broken. Its index and who
    /// fights.
    pub fn begin(&mut self, sides: [Vec<usize>; 2], month: i32, cause: &str, names: &[String], rng: &mut Rng) -> usize {
        let mut fight: [Vec<usize>; 2] = [vec![sides[0][0]], vec![sides[1][0]]];
        for s in 0..2 {
            let head = sides[s][0];
            for &x in sides[s].iter().skip(1) {
                if self.comes(x, head, rng) {
                    fight[s].push(x);
                } else {
                    self.betrayed[head * self.n + x] = month;
                    let line = format!("{} {} на помощь союзнику — {}", names[x], crate::world::names::agree(&names[x], "не пришла"), names[head]);
                    self.chronicle.push((month, head, line));
                    self.pacts.retain(|p| !(p.kind == Kind::Alliance && p.has(x) && p.has(head)));
                }
            }
        }
        // a pact of non-aggression broken: the attacker is faithless in the eyes of all
        let (a, b) = (fight[0][0], fight[1][0]);
        if self.has(Kind::Peace, a, b) {
            self.pacts.retain(|p| !(p.kind == Kind::Peace && p.has(a) && p.has(b)));
            for o in 0..self.n {
                if o != a {
                    self.betrayed[o * self.n + a] = self.betrayed[o * self.n + a].max(month - 12 * 20);
                }
            }
            self.betrayed[b * self.n + a] = month;
            self.chronicle.push((month, a, format!("{} {} договор о ненападении с {}", names[a], crate::world::names::agree(&names[a], "нарушила"), names[b])));
        }
        for s in 0..2 {
            for &x in &fight[s] {
                for &y in &fight[1 - s] {
                    self.pacts.retain(|p| !(p.has(x) && p.has(y) && matches!(p.kind, Kind::Trade | Kind::Marriage | Kind::Guarantee | Kind::Tribute)));
                }
            }
        }
        let allies: Vec<String> = (0..2)
            .filter(|&s| fight[s].len() > 1)
            .map(|s| format!("на стороне {} — {}", names[fight[s][0]], fight[s][1..].iter().map(|&x| names[x].as_str()).collect::<Vec<_>>().join(", ")))
            .collect();
        let line = format!("Война: {} против {} — {}{}", names[a], names[b], cause, if allies.is_empty() { String::new() } else { format!("; {}", allies.join("; ")) });
        self.chronicle.push((month, a, line));
        self.wars.push(War { sides: fight, since: month, cause: cause.to_string(), score: 0.0, weary: [0.0; 2] });
        self.wars.len() - 1
    }

    /// A month of war `w` of the world, fought by no front (the power of each side and its luck
    /// decide); its end, if it ended: the winning side (None: a draw) and whether the loser gave up.
    pub fn war_month(&mut self, w: usize, power: &dyn Fn(usize) -> f32, rng: &mut Rng) -> Option<(Option<usize>, bool)> {
        let war = &mut self.wars[w];
        let might: [f32; 2] = [0, 1].map(|s| war.sides[s].iter().enumerate().map(|(i, &c)| power(c) * if i == 0 { 1.0 } else { 0.5 }).sum::<f32>().max(1e-6));
        let edge = (might[0] - might[1]) / (might[0] + might[1]);
        war.score = (war.score + 0.05 * edge + 0.06 * rng.normal()).clamp(-1.0, 1.0);
        for s in 0..2 {
            let losing = if s == 0 { (-war.score).max(0.0) } else { war.score.max(0.0) };
            war.weary[s] = (war.weary[s] + 1.0 / 48.0 + 0.03 * losing).min(1.0);
        }
        if war.score >= 0.95 {
            Some((Some(0), true))
        } else if war.score <= -0.95 {
            Some((Some(1), true))
        } else if war.weary[0] >= 1.0 || war.weary[1] >= 1.0 {
            let tired = if war.weary[0] >= war.weary[1] { 0 } else { 1 };
            let lead = if war.score > 0.25 { Some(0) } else if war.score < -0.25 { Some(1) } else { None };
            Some((lead.or(if war.score.abs() > 0.1 { Some(1 - tired) } else { None }), false))
        } else {
            None
        }
    }

    /// War `w` ends in `month`: won by side `winner` (None: a draw), the loser giving up if `harsh`.
    /// Truces between all who fought; each remembers whom it fought and beside whom; a beaten
    /// country that gave up pays tribute. The war as it was.
    pub fn end(&mut self, w: usize, winner: Option<usize>, harsh: bool, month: i32, names: &[String]) -> War {
        let war = self.wars.remove(w);
        let n = self.n;
        for s in 0..2 {
            for &x in &war.sides[s] {
                for &y in &war.sides[1 - s] {
                    self.fought[x * n + y] = month;
                }
                for &y in &war.sides[s] {
                    if x != y {
                        self.beside[x * n + y] = month;
                    }
                }
            }
        }
        let (a, b) = (war.sides[0][0], war.sides[1][0]);
        for &x in &war.sides[0] {
            for &y in &war.sides[1] {
                let years = if (x == a && y == b) || (x == b && y == a) { Kind::Truce.years() } else { 8 };
                self.sign(Kind::Truce, x, y, month, years);
            }
        }
        if let (Some(win), true) = (winner, harsh) {
            let (v, l) = (war.sides[win][0], war.sides[1 - win][0]);
            self.pacts.retain(|p| !(p.has(l) && p.kind == Kind::Tribute));
            self.sign(Kind::Tribute, v, l, month, Kind::Tribute.years());
            self.chronicle.push((month, l, format!("{} платит дань; получает её {}", names[l], names[v])));
        }
        war
    }

    /// A pact of `kind` the year makes between `a` and `b` (told in `line`): signed, or, when one
    /// of them is the player's country, offered to him (once while he has not answered).
    fn make(&mut self, kind: Kind, a: usize, b: usize, month: i32, line: String) {
        if let Some(p) = self.player.filter(|&p| p == a || p == b) {
            let from = if p == a { b } else { a };
            if !self.offers.iter().any(|o| o.0 == kind && o.1 == from) {
                self.offers.push((kind, from));
            }
            return;
        }
        self.sign(kind, a, b, month, kind.years());
        self.chronicle.push((month, a, line));
    }

    /// Sign a pact of `kind` between `a` and `b` for `years` (a pact already in force is extended).
    pub fn sign(&mut self, kind: Kind, a: usize, b: usize, month: i32, years: u32) {
        let until = month + 12 * years as i32;
        if let Some(p) = self.pacts.iter_mut().find(|p| p.kind == kind && ((p.a == a && p.b == b) || (kind.mutual() && p.a == b && p.b == a))) {
            p.until = p.until.max(until);
            return;
        }
        self.pacts.push(Pact { kind, a, b, since: month, until });
    }

    /// Break the pact of `kind` between `a` and `b` (either way).
    pub fn cancel(&mut self, kind: Kind, a: usize, b: usize) -> bool {
        let before = self.pacts.len();
        self.pacts.retain(|p| !(p.kind == kind && p.has(a) && p.has(b)));
        self.pacts.len() < before
    }

    /// What `a`'s opinion of `b` comes to this year, by its reasons.
    pub fn reasons(&self, sc: &Scene, a: usize, b: usize, month: i32) -> [f32; WHY] {
        let n = self.n;
        let mut w = [0.0f32; WHY];
        // (a far country matters less, for good or ill; a neighbour's border is always a rub)
        let near = sc.neighbours[a].contains(&b);
        let close = near || sc.neighbours[a].iter().any(|x| sc.neighbours[*x].contains(&b));
        let reach = if near { 1.0 } else if close { 0.7 } else { 0.4 };
        let apart = (sc.apart)(a, b);
        w[FAITH] = reach * if apart <= 0.0 { 0.15 } else if apart < 0.75 { 0.03 } else { -0.15 * (1.0 + sc.zeal[a]) };
        w[GOVERN] = reach * kinship(sc.regime[a], sc.regime[b]);
        w[LANDS] = -0.6 * (sc.holds)(a, b) - 0.1 * (sc.holds)(b, a) - if near { 0.07 } else { 0.0 };
        w[ENEMIES] = -0.5 * fade(month, self.fought[a * n + b], MEMORY);
        w[COMRADES] = 0.35 * fade(month, self.beside[a * n + b], MEMORY);
        w[HOUSE] = if self.has(Kind::Marriage, a, b) {
            0.3
        } else if sc.house[a].is_some() && sc.house[a] == sc.house[b] {
            0.25
        } else {
            0.0
        };
        w[TRADE] = if self.has(Kind::Trade, a, b) { 0.15 } else { 0.0 };
        let foe = (0..n).any(|x| x != a && x != b && sc.live[x] && self.of(a, x) < -0.3 && self.of(b, x) < -0.3);
        w[FOE] = if foe { 0.25 } else { 0.0 };
        if near && sc.power[b] > 1.5 * sc.power[a] {
            let hostile = (-self.of(b, a)).max(0.0) + 0.3 * sc.hawk[b].max(0.0);
            w[FEAR] = -0.12 * (sc.power[b] / sc.power[a] / 3.0).min(1.0) * (1.0 + 2.0 * hostile);
        }
        if self.has(Kind::Tribute, b, a) {
            w[FEAR] -= 0.25;
        }
        w[BETRAYAL] = -0.6 * fade(month, self.betrayed[a * n + b], BETRAYAL_YEARS);
        w[ENVOYS] = self.warmth.get(a * n + b).copied().unwrap_or(0.0);
        w
    }

    /// A year of the world's relations (or `span` of one): opinions follow their reasons; pacts
    /// lapse, are renewed or broken; new ones are made by those who gain by them.
    pub fn year(&mut self, sc: &Scene, month: i32, span: f32, rng: &mut Rng) {
        let n = self.n;
        let pace = 1.0 - (1.0 - PACE).powf(span.max(1e-3));
        let mut next = self.opinion.clone();
        for a in 0..n {
            for b in 0..n {
                if a == b || !sc.live[a] || !sc.live[b] {
                    continue;
                }
                let w = self.reasons(sc, a, b, month);
                let target: f32 = w.iter().sum();
                let i = a * n + b;
                next[i] = (self.opinion[i] + (target - self.opinion[i]) * pace + 0.04 * span.sqrt() * rng.normal()).clamp(-1.0, 1.0);
                self.why[i] = w;
            }
        }
        self.opinion = next;
        // pacts of the dead go with them
        self.pacts.retain(|p| sc.live[p.a] && sc.live[p.b]);
        // pacts lapse at their term, unless both still want them; some are broken before it
        let mut kept = Vec::new();
        for p in std::mem::take(&mut self.pacts) {
            // (the player keeps his pacts till he breaks them: only the other side may turn away)
            let mine = |c: usize| self.player == Some(c);
            let (oa, ob) = (if mine(p.a) { 1.0 } else { self.of(p.a, p.b) }, if mine(p.b) { 1.0 } else { self.of(p.b, p.a) });
            let lapse = p.until <= month;
            let (keep, why) = match p.kind {
                Kind::Alliance => (oa > -0.05 && ob > -0.05 && !(lapse && (oa < 0.2 || ob < 0.2)), "охладели друг к другу"),
                Kind::Peace => (!lapse || (oa > -0.2 && ob > -0.2 && rng.f32() < 0.5), "срок вышел"),
                Kind::Trade => (oa > -0.3 && ob > -0.3 && !(lapse && (oa < 0.0 || ob < 0.0)), "рассорились"),
                Kind::Truce => (!lapse, ""),
                Kind::Guarantee => (oa > 0.0 && !(lapse && oa < 0.2), "гарант охладел"),
                // the tributary refuses when it grows strong enough (or the term is over)
                Kind::Tribute => (!lapse && sc.power[p.b] < 0.8 * sc.power[p.a], "данница окрепла и отказалась платить"),
                Kind::Marriage => (sc.regime[p.a].crowned() && sc.regime[p.b].crowned() && !lapse, "монархии больше нет"),
            };
            if keep {
                let mut p = p;
                if lapse {
                    p.until = month + 12 * p.kind.years() as i32;
                }
                kept.push(p);
            } else if p.kind != Kind::Truce && !(matches!(p.kind, Kind::Peace | Kind::Marriage) && lapse) {
                let line = format!("Конец договора ({}): {} и {} — {why}", p.kind.name().to_lowercase(), sc.names[p.a], sc.names[p.b]);
                self.chronicle.push((month, p.a, line));
            }
        }
        self.pacts = kept;
        // new pacts
        let threat = |d: &Diplomacy, c: usize| -> f32 {
            sc.neighbours[c].iter().filter(|&&x| sc.live[x]).map(|&x| (sc.power[x] / sc.power[c].max(1e-6)) * (-d.of(x, c)).max(0.0)).fold(0.0f32, f32::max)
        };
        let count = |d: &Diplomacy, c: usize, k: Kind, strong: bool| d.pacts.iter().filter(|p| p.kind == k && if k.mutual() { p.has(c) } else if strong { p.a == c } else { p.b == c }).count();
        let chance = |x: f32| rng_chance(x * span);
        let mut order: Vec<usize> = (0..n).filter(|&c| sc.live[c]).collect();
        for i in (1..order.len()).rev() {
            let j = rng.below(i + 1);
            order.swap(i, j);
        }
        for &a in &order {
            for &b in &order {
                if a >= b || self.enemies(a, b) {
                    continue;
                }
                let (oa, ob) = (self.of(a, b), self.of(b, a));
                let near = sc.neighbours[a].contains(&b) || sc.neighbours[a].iter().any(|x| sc.neighbours[*x].contains(&b));
                let roll = rng.f32();
                // an alliance: friends who share an enemy, or one threatened and a strong friend
                if !self.either(Kind::Alliance, a, b) && oa > 0.35 && ob > 0.35 && count(self, a, Kind::Alliance, true) < ALLIANCES && count(self, b, Kind::Alliance, true) < ALLIANCES && near {
                    let shared = (0..n).any(|x| x != a && x != b && sc.live[x] && self.of(x, a) < -0.2 && self.of(x, b) < -0.2);
                    let need = threat(self, a).max(threat(self, b));
                    if (shared || need > 0.4) && roll < chance(0.2) {
                        self.make(Kind::Alliance, a, b, month, format!("Союз: {} и {}", sc.names[a], sc.names[b]));
                        continue;
                    }
                }
                // a pact of non-aggression: neighbours not friends but not enemies, one busy or afraid
                if sc.neighbours[a].contains(&b) && !self.has(Kind::Peace, a, b) && !self.either(Kind::Alliance, a, b) && oa > -0.3 && ob > -0.3 && oa < 0.35 && count(self, a, Kind::Peace, true) < PEACES && count(self, b, Kind::Peace, true) < PEACES {
                    let weak = sc.power[a].min(sc.power[b]) / sc.power[a].max(sc.power[b]).max(1e-6);
                    let busy = threat(self, a) > 0.3 || threat(self, b) > 0.3;
                    if (busy || weak < 0.6) && roll < chance(0.06) {
                        self.make(Kind::Peace, a, b, month, format!("Договор о ненападении: {} и {}", sc.names[a], sc.names[b]));
                        continue;
                    }
                }
                // a trade agreement: on speaking terms, people of trade on both sides
                let room = |c: usize| (count(self, c, Kind::Trade, true) as f32) < TRADES + TRADES_BY_TRADE * sc.trade[c];
                if near && !self.has(Kind::Trade, a, b) && oa > 0.15 && ob > 0.15 && room(a) && room(b) && sc.trade[a] + sc.trade[b] > 0.18 && roll < chance(0.05 * (sc.trade[a] + sc.trade[b]) / 0.3) {
                    self.make(Kind::Trade, a, b, month, format!("Торговый договор: {} и {}", sc.names[a], sc.names[b]));
                    continue;
                }
                // a marriage of ruling houses
                if sc.regime[a].crowned() && sc.regime[b].crowned() && sc.house[a].is_some() && sc.house[b].is_some() && sc.house[a] != sc.house[b] && !self.has(Kind::Marriage, a, b) && count(self, a, Kind::Marriage, true) < MARRIAGES && count(self, b, Kind::Marriage, true) < MARRIAGES && near && oa > 0.05 && ob > 0.05 && roll < chance(0.03) {
                    let line = format!("Брак правящих домов: {} и {} — {} и {}", sc.names[a], sc.names[b], sc.house[a].clone().unwrap_or_default(), sc.house[b].clone().unwrap_or_default());
                    self.make(Kind::Marriage, a, b, month, line);
                    continue;
                }
            }
        }
        // guarantees: a strong country shields a weak friend from a neighbour it dislikes
        for &g in &order {
            if count(self, g, Kind::Guarantee, true) >= GUARANTEES {
                continue;
            }
            for &w in &order {
                let near = sc.neighbours[g].contains(&w) || sc.neighbours[g].iter().any(|x| sc.neighbours[*x].contains(&w));
                if g == w || !near || sc.power[g] < 1.5 * sc.power[w] || self.of(g, w) < 0.25 || self.has(Kind::Guarantee, g, w) || self.enemies(g, w) || count(self, w, Kind::Guarantee, false) >= GUARDED {
                    continue;
                }
                let menace = sc.neighbours[w].iter().any(|&x| x != g && sc.live[x] && sc.power[x] > sc.power[w] && self.of(x, w) < -0.15 && self.of(g, x) < 0.0);
                if menace && rng.f32() < chance(0.08) {
                    if self.player == Some(g) {
                        continue;
                    }
                    self.make(Kind::Guarantee, g, w, month, format!("{} берёт под защиту: {}", sc.names[g], sc.names[w]));
                    break;
                }
            }
        }
        // (stable order of the pacts)
        self.pacts.sort_by_key(|p| (p.kind.code(), p.a, p.b));
    }
}

/// A chance for a step: kept within 0 … 1.
fn rng_chance(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}
