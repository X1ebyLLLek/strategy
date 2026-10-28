//! The world's faiths: a few old ones at the beginning of the deep past, each over a stretch of the
//! map; then they spread region by region (a richer, more zealous neighbour converts; a state church
//! converts its own dissenters; tolerance lets them be), split (a learned, restless country whose
//! clergy is hated breaks away into a new branch — fewer priests, more learning, more thrift), and
//! now and then a new one is founded in a country in deep trouble. A faith's tenets (war, learning,
//! commerce, hierarchy, tolerance, thrift, charity) are drawn when it is founded and changed when it
//! splits — nothing about any faith is written beforehand.

use crate::rng::Rng;
use super::{Polities, CHURCH};

/// The tenets of a faith (−1 … +1 each).
pub const TENETS: usize = 7;
pub const WAR: usize = 0;
pub const LEARNING: usize = 1;
pub const COMMERCE: usize = 2;
pub const HIERARCHY: usize = 3;
pub const TOLERANCE: usize = 4;
pub const THRIFT: usize = 5;
pub const CHARITY: usize = 6;

/// Each tenet in words: its name, what it says at +1, at −1.
pub const TENET_WORDS: [(&str, &str, &str); TENETS] = [
    ("война", "священная война с неверными", "непротивление злу силой"),
    ("знание", "знание — богоугодно", "мудрость отцов выше книг"),
    ("торговля", "торговля благословенна", "лихва — грех"),
    ("иерархия", "строгая иерархия священства", "община верных без священства"),
    ("терпимость", "к иноверцам терпимы", "истинная вера одна"),
    ("бережливость", "труд и бережливость — добродетель", "радость дня, щедрые праздники"),
    ("милосердие", "милостыня — долг каждого", "каждый сам за себя"),
];

/// A faith of the world.
#[derive(Clone, Debug)]
pub struct Faith {
    pub name: String,
    /// The month it was founded (the oldest ones: the beginning of the deep past).
    pub founded: i32,
    /// The faith it split from.
    pub parent: Option<usize>,
    pub tenets: [f32; TENETS],
    /// Its colour on the map.
    pub hue: [u8; 3],
    /// Who founded it or led it away (a name).
    pub founder: String,
}

const KINDS: [&str; 7] = ["Церковь", "Учение", "Община", "Братство", "Вера", "Путь", "Завет"];
const ROOTS: [&str; 20] = [
    "Светоча", "Зари", "Неба", "Пламени", "Слова", "Солнца", "Звезды", "Истины", "Источника", "Камня", "Древа", "Чаши", "Ключа", "Знамения",
    "Пророка", "Предков", "Восхода", "Грома", "Луны", "Хранителей",
];
const BRANCHES: [&str; 10] = ["новый", "старый", "строгий", "вольный", "северный", "южный", "восточный", "западный", "очищенный", "братский"];

impl Faith {
    /// One of its tenets in words, if it holds it firmly.
    pub fn creed(&self) -> Vec<String> {
        let mut out = Vec::new();
        for t in 0..TENETS {
            let x = self.tenets[t];
            if x > 0.35 {
                out.push(TENET_WORDS[t].1.to_string());
            } else if x < -0.35 {
                out.push(TENET_WORDS[t].2.to_string());
            }
        }
        out
    }
}

fn hue(rng: &mut Rng) -> [u8; 3] {
    // muted colours, as old maps paint faiths
    let h = rng.f32() * 6.0;
    let (s, v) = (0.35 + 0.25 * rng.f32(), 0.55 + 0.3 * rng.f32());
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8]
}

impl Polities {
    /// A new faith: its name not taken yet, tenets drawn (a branch: its parent's, changed).
    pub fn found(&mut self, parent: Option<usize>, month: i32, founder: String, rng: &mut Rng) -> usize {
        let taken: Vec<String> = self.faiths.iter().map(|f| f.name.clone()).collect();
        let mut name = String::new();
        for _ in 0..40 {
            name = match parent {
                Some(p) => format!("{} — {} толк", self.faiths[p].name.split(" — ").next().unwrap_or(""), BRANCHES[rng.below(BRANCHES.len())]),
                None => format!("{} {}", KINDS[rng.below(KINDS.len())], ROOTS[rng.below(ROOTS.len())]),
            };
            if !taken.contains(&name) {
                break;
            }
        }
        let tenets = match parent {
            // a branch: fewer priests, more learning and thrift — or, against a lax church, stricter
            Some(p) => {
                let mut t = self.faiths[p].tenets;
                let strict = rng.f32() < 0.3;
                for x in t.iter_mut() {
                    *x = (*x + 0.25 * rng.normal()).clamp(-1.0, 1.0);
                }
                if strict {
                    t[TOLERANCE] = (t[TOLERANCE] - 0.6).max(-1.0);
                    t[WAR] = (t[WAR] + 0.4).min(1.0);
                    t[LEARNING] = (t[LEARNING] - 0.3).max(-1.0);
                } else {
                    t[HIERARCHY] = (t[HIERARCHY] - 0.7).max(-1.0);
                    t[LEARNING] = (t[LEARNING] + 0.4).min(1.0);
                    t[THRIFT] = (t[THRIFT] + 0.4).min(1.0);
                }
                t
            }
            None => {
                let mut t = [0.0f32; TENETS];
                for x in t.iter_mut() {
                    *x = (0.55 * rng.normal()).clamp(-1.0, 1.0);
                }
                t
            }
        };
        self.faiths.push(Faith { name, founded: month, parent, tenets, hue: hue(rng), founder });
        self.faiths.len() - 1
    }

    /// The beginning: `k` old faiths, each from a region of its own, flooding the map over the
    /// regions' neighbours (`next` of a region) until every region with people has one.
    pub fn first_faiths(&mut self, regions: usize, live: &dyn Fn(usize) -> bool, next: &dyn Fn(usize) -> Vec<usize>, month: i32, rng: &mut Rng) {
        self.region_faith = vec![usize::MAX; regions];
        let alive: Vec<usize> = (0..regions).filter(|&r| live(r)).collect();
        if alive.is_empty() {
            return;
        }
        let k = 3 + rng.below(3);
        // (each faith grows from its seed by a random cost per step: uneven, patchy borders)
        let mut cost = vec![f32::MAX; regions];
        let mut heap: std::collections::BinaryHeap<(std::cmp::Reverse<u32>, usize)> = Default::default();
        for _ in 0..k {
            let seed = alive[rng.below(alive.len())];
            if self.region_faith[seed] != usize::MAX {
                continue;
            }
            let f = self.found(None, month, crate::world::names::person(rng), rng);
            self.region_faith[seed] = f;
            cost[seed] = 0.0;
            heap.push((std::cmp::Reverse(0), seed));
        }
        while let Some((std::cmp::Reverse(c), r)) = heap.pop() {
            let c = c as f32 / 1000.0;
            if c > cost[r] {
                continue;
            }
            for q in next(r) {
                if !live(q) {
                    continue;
                }
                let step = 1.0 + 2.0 * rng.f32();
                if c + step < cost[q] {
                    cost[q] = c + step;
                    self.region_faith[q] = self.region_faith[r];
                    heap.push((std::cmp::Reverse(((c + step) * 1000.0) as u32), q));
                }
            }
        }
        // (regions no seed could reach: the nearest faith by number)
        for r in alive {
            if self.region_faith[r] == usize::MAX {
                self.region_faith[r] = 0;
            }
        }
    }

    /// A year of the faiths: regions converted by their neighbours and by their own state. `owner`
    /// — the country of a region; `next` — its neighbouring regions; `dev` — each country's
    /// development (0..1). The conversions, as (region, country, from, to).
    pub fn spread(&mut self, owner: &dyn Fn(usize) -> Option<usize>, next: &dyn Fn(usize) -> Vec<usize>, dev: &[f32], rng: &mut Rng) -> Vec<(usize, usize, usize, usize)> {
        let mut out = Vec::new();
        let n = self.region_faith.len();
        for r in 0..n {
            let mine = self.region_faith[r];
            if mine == usize::MAX {
                continue;
            }
            let Some(c) = owner(r) else { continue };
            let p = &self.of[c];
            let state = p.state_faith;
            let tol = self.faiths.get(state).map_or(0.0, |f| f.tenets[TOLERANCE]);
            // the own state converts its dissenters (a state church, an intolerant faith hardest)
            if mine != state && p.laws[CHURCH] < 2 {
                let push = if p.laws[CHURCH] == 0 { 0.035 } else { 0.012 } * (1.0 - 0.6 * tol) * (0.5 + p.piety);
                if rng.f32() < push {
                    self.region_faith[r] = state;
                    out.push((r, c, mine, state));
                    continue;
                }
            }
            // neighbours of another faith: missions, traders, marriages
            for q in next(r) {
                let theirs = self.region_faith.get(q).copied().unwrap_or(usize::MAX);
                if theirs == usize::MAX || theirs == mine {
                    continue;
                }
                let qc = owner(q);
                let zeal = self.faiths.get(theirs).map_or(0.0, |f| (f.tenets[WAR] + -f.tenets[TOLERANCE]) * 0.5 + 0.5);
                let pull = match qc {
                    Some(o) if o != c => (dev[o] / dev[c].max(0.05)).clamp(0.3, 3.0),
                    _ => 1.0,
                };
                // (a state church guards its flock; a secular state does not)
                let guard = if mine == state { [0.35, 0.7, 1.0][p.laws[CHURCH].min(2) as usize] } else { 1.2 };
                let chance = 0.004 * (0.4 + zeal) * pull * guard * (1.2 - p.piety * 0.5);
                if rng.f32() < chance {
                    self.region_faith[r] = theirs;
                    out.push((r, c, mine, theirs));
                    break;
                }
            }
        }
        out
    }

    /// Country `c` breaks away into a branch of its state faith (or a new faith, `new`): its
    /// capital's region and a share of its regions of that faith go over; its state takes it. The
    /// new faith.
    pub fn split(&mut self, c: usize, regions: &[usize], capital_region: usize, new: bool, month: i32, founder: String, rng: &mut Rng) -> usize {
        let old = self.of[c].state_faith;
        let f = self.found(if new { None } else { Some(old) }, month, founder, rng);
        let share = 0.5 + 0.4 * rng.f32();
        for &r in regions {
            if r == capital_region || (self.region_faith.get(r) == Some(&old) && rng.f32() < share) {
                if let Some(x) = self.region_faith.get_mut(r) {
                    *x = f;
                }
            }
        }
        self.of[c].state_faith = f;
        f
    }
}
