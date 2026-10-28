//! The war of divisions (as in HOI4) between two neighbours of a generated map: a chronicle by
//! months and self-checks.
//!
//! `checks corps [--pack packs/x.toml] [--seed N] [--months M]` — a chronicle
//! of one war, then the checks; `--quick` — only the chronicle; `--checks` — only the checks.

use crate::common::pick;
use strategy::military::corps::{Corps, ORG_OUT};
use strategy::military::doctrine::Country;
use strategy::military::front::{Campaign, General, Theatre, NO};
use strategy::pack::Pack;
use strategy::rng::{hash64, Rng};
use strategy::world::countries::{self, Politics};
use strategy::world::map::{self, Params, World};

mod suite;
use suite::{checks, PACK_PATH};

/// Prices of goods each side pays against the pack's (None: the pack's), for the checks of what
/// kit the staffs choose.
static PRICES: std::sync::RwLock<[Option<Vec<f32>>; 2]> = std::sync::RwLock::new([None, None]);

/// The staff's template of side `s` in words: battalions, kit of each kind, companies.
fn staff_words(pack: &Pack, k: &Corps, s: usize) -> String {
    let Some(sh) = k.shapes[s].last() else { return String::new() };
    let bats: Vec<String> = sh.units.iter().enumerate().filter(|(_, x)| **x > 0.0).map(|(u, x)| format!("{} {} ({})", (x / 0.1).round(), pack.units[u].name, strategy::military::doctrine::norm::word(sh.norm_of(u)))).collect();
    let sup: Vec<&str> = sh.support.iter().map(|&i| pack.support[i].name.as_str()).collect();
    format!("«{}»: {} батальонов — {}; роты: {}", sh.name, sh.battalions().round(), bats.join(", "), if sup.is_empty() { "нет".into() } else { sup.join(", ") })
}

/// A war of divisions between the chosen pair of map `seed`: each side's army a year of its
/// budget (`money` shares it), its front in groups of armies under generals, its staff leading
/// them; with `twins`, both sides' equipment the same.
fn campaign_of(pack: &Pack, world: &World, pol: &Politics, pair: (usize, usize), money: [f32; 2], seed: u64, twins: bool) -> Campaign {
    let th = Theatre::new(world, pol, pair.0, pair.1);
    let mut ra = Rng::new(hash64(seed));
    let mut rb = Rng::new(hash64(seed ^ if twins { 0 } else { 0xB0B }));
    let ones = vec![1.0; pack.goods.len()];
    let prices = PRICES.read().unwrap().clone();
    let mut a = Country::new(pack, &pol.countries[pair.0].name, prices[0].as_deref().unwrap_or(&ones), &mut ra);
    let mut b = Country::new(pack, &pol.countries[pair.1].name, prices[1].as_deref().unwrap_or(&ones), &mut rb);
    // (the staffs choose the kit of their kinds by what it costs them)
    a.norm_free = true;
    b.norm_free = true;
    let mut c = Campaign::new(pack, th, [a, b], money, seed);
    // the divisions first, then as many armies as they make
    c.muster(pack, ["Кадровая дивизия", "Кадровая дивизия"], [Vec::new(), Vec::new()], [Vec::new(), Vec::new()]);
    let mut rg = Rng::new(hash64(seed ^ 0x6E4));
    for s in 0..2 {
        c.by_armies(s, true);
        if let Some(gs) = c.command[s].as_mut() {
            for (k, g) in gs.iter_mut().enumerate() {
                // (twins: their generals alike too)
                let mut r = if twins { Rng::new(hash64(seed ^ 0x6E4 ^ k as u64)) } else { Rng::new(rg.next_u64()) };
                g.general = Some(General::anyone(format!("генерал {k}"), &mut r));
            }
        }
    }
    c.deploy(pack);
    c
}

/// What a war of divisions shows month by month.
#[derive(Default, Clone)]
struct Book {
    /// Front provinces of each side and those of them where at least one of its divisions stands
    /// (summed over the months).
    front: [f32; 2],
    covered: [f32; 2],
    /// Of them, those where its divisions are worth at least half the enemy's across (each enemy
    /// province's worth shared among the provinces of ours it touches).
    matched: [f32; 2],
    /// Fights (days each lasted), attacks won by each side.
    days: Vec<u32>,
    won: [u32; 2],
    fought: [u32; 2],
    /// Strength lost a day by divisions cut off from their capital and by the others (summed
    /// shares, division-days).
    cut_loss: f32,
    cut_days: f32,
    free_loss: f32,
    free_days: f32,
    /// Milliseconds a day of war took.
    ms: f32,
    daysn: f32,
}

/// The war lived `months` (thirty days of divisions, then the month's books), with `book` kept.
fn live(pack: &Pack, c: &mut Campaign, months: u32, book: &mut Book, mut each: impl FnMut(u32, &Campaign, &Book)) {
    let nu = pack.units.len();
    for m in 1..=months {
        for _ in 0..30 {
            let reach = [reach_of(c, 0), reach_of(c, 1)];
            let before: Vec<(u32, f32, bool)> = c.corps.as_ref().unwrap().divs.iter().map(|d| (d.id, d.has.iter().sum::<f32>(), reach[d.side][d.at] == NO && d.way.is_empty())).collect();
            let t = std::time::Instant::now();
            c.day(pack);
            book.ms += t.elapsed().as_secs_f32() * 1000.0;
            book.daysn += 1.0;
            let k = c.corps.as_ref().unwrap();
            for (id, had, cut) in before {
                // (losses on a day it did not fight: hunger; a division gone counts only if it
                // fought)
                let Some(d) = k.divs.iter().find(|d| d.id == id) else { continue };
                let now = d.has.iter().sum::<f32>();
                if d.busy || had <= 0.0 {
                    continue;
                }
                let lost = ((had - now) / had).max(0.0);
                if cut {
                    book.cut_loss += lost;
                    book.cut_days += 1.0;
                } else {
                    book.free_loss += lost;
                    book.free_days += 1.0;
                }
            }
            let _ = nu;
            if c.fallen.is_some() {
                break;
            }
        }
        let had = c.fought.len();
        c.step(pack);
        let _ = had;
        // the front its capital reaches: how much of it has a division standing on it, or a step
        // from it on its way there
        let k = c.corps.as_ref().unwrap();
        for s in 0..2 {
            let reach = reach_of(c, s);
            let front: Vec<usize> = c.front(s).into_iter().filter(|&p| reach[p] != NO).collect();
            book.front[s] += front.len() as f32;
            book.covered[s] += front.iter().filter(|&&p| k.divs.iter().any(|d| d.side == s && (d.at == p || d.way.first() == Some(&p)))).count() as f32;
            let n = c.owner.len();
            let mut worth = [vec![0.0f32; n], vec![0.0f32; n]];
            for d in &k.divs {
                worth[d.side][d.at] += Corps::worth(pack, &d.has) * d.org.max(0.2);
            }
            let o = 1 - s;
            for &p in &front {
                let threat: f32 = c.th.nbr[p].iter().filter(|&&q| c.owner[q] == o).map(|&q| worth[o][q] / c.th.nbr[q].iter().filter(|&&r| c.owner[r] == s).count().max(1) as f32).sum();
                if worth[s][p] >= 0.5 * threat {
                    book.matched[s] += 1.0;
                }
            }
        }
        each(m, c, book);
        if c.fallen.is_some() {
            break;
        }
    }
    for f in &c.fought {
        book.days.push(f.days);
        book.fought[f.attacker] += 1;
        if f.won {
            book.won[f.attacker] += 1;
        }
    }
}

fn reach_of(c: &Campaign, s: usize) -> Vec<u32> {
    // (the capital's reach over its own provinces, as the front counts it)
    let n = c.owner.len();
    let mut d = vec![NO; n];
    let cap = c.th.capital[s];
    if c.owner[cap] != s {
        return d;
    }
    let mut q = std::collections::VecDeque::from([cap]);
    d[cap] = 0;
    while let Some(p) = q.pop_front() {
        for &r in &c.th.nbr[p] {
            if c.owner[r] == s && d[r] == NO {
                d[r] = d[p] + 1;
                q.push_back(r);
            }
        }
    }
    d
}

/// The side that made the other give up, or that holds more of its own people at the end (None
/// when nearly the same).
fn winner(c: &Campaign) -> Option<usize> {
    if let Some(s) = c.fallen {
        return Some(1 - s);
    }
    let p = c.people();
    if (p[0] - p[1]).abs() < 0.03 {
        None
    } else if p[0] > p[1] {
        Some(0)
    } else {
        Some(1)
    }
}

fn chronicle(pack: &Pack, seed: u64, months: u32) {
    let world = map::generate(&Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 }).0;
    let pol = countries::generate(&world, seed, 30, &pack);
    let Some(pair) = pick(&world, &pol) else {
        println!("на карте {seed} нет пары соседей сравнимой силы");
        return;
    };
    let money = match std::env::var("RICH").ok().and_then(|v| v.parse::<f32>().ok()) {
        Some(k) => [k, 1.0],
        None => [pol.gdp(pair.0), pol.gdp(pair.1)],
    };
    let mut c = campaign_of(pack, &world, &pol, pair, money, seed, std::env::var("RICH").is_ok());
    let names = [c.sides[0].name.clone(), c.sides[1].name.clone()];
    let k = c.corps.as_ref().unwrap();
    println!("══════ {} — война дивизиями на карте {seed}: {} против {} ══════", pack.name, names[0], names[1]);
    for s in 0..2 {
        let n = k.divs.iter().filter(|d| d.side == s).count();
        let sh = &k.shapes[s][0];
        let bats: Vec<String> = sh.units.iter().zip(&pack.units).filter(|(x, _)| **x > 0.0).map(|(x, u)| format!("{} {}", (x / 0.1).round(), u.name)).collect();
        println!("  {}: {} дивизий ({}), фронт {} провинций, групп {}", names[s], n, bats.join(", "), c.front(s).len(), c.command[s].as_ref().map_or(0, |g| g.len()));
    }
    let mut book = Book::default();
    let mut last = 0;
    let t0 = std::time::Instant::now();
    live(pack, &mut c, months, &mut book, |m, c, b| {
        let k = c.corps.as_ref().unwrap();
        let held = c.held();
        let people = c.people();
        let side = |s: usize| {
            let ds: Vec<_> = k.divs.iter().filter(|d| d.side == s).collect();
            let n = ds.len().max(1) as f32;
            let org = ds.iter().map(|d| d.org).sum::<f32>() / n;
            let str_ = ds.iter().map(|d| k.strength(pack, d)).sum::<f32>() / n;
            let tired = ds.iter().filter(|d| d.org < ORG_OUT).count();
            let pool: f32 = Corps::worth(pack, &k.pool[s]);
            (ds.len(), org, str_, tired, pool)
        };
        let period = &c.fought[last.min(c.fought.len())..];
        let att: Vec<(usize, usize)> = (0..2).map(|s| (period.iter().filter(|f| f.attacker == s).count(), period.iter().filter(|f| f.attacker == s && f.won).count())).collect();
        let (a, b_) = (side(0), side(1));
        println!(
            "  мес {m:>2}: провинций {}/{} (своих людей {:.0}%/{:.0}%) | дивизий {}/{} (орг. {:.2}/{:.2}, численность {:.2}/{:.2}, выдохлись {}/{}) | атак {} ({} успешных) / {} ({}) | фронт прикрыт {:.0}%/{:.0}% | {:.1} мс/день",
            held[0], held[1], people[0] * 100.0, people[1] * 100.0, a.0, b_.0, a.1, b_.1, a.2, b_.2, a.3, b_.3, att[0].0, att[0].1, att[1].0, att[1].1,
            100.0 * b.covered[0] / b.front[0].max(1.0), 100.0 * b.covered[1] / b.front[1].max(1.0), b.ms / b.daysn.max(1.0)
        );
        last = c.fought.len();
        for s in 0..2 {
            println!("        {}", c.stand_words(pack, s));
        }
        if std::env::var("GAPS").is_ok() {
            let reach = [reach_of(c, 0), reach_of(c, 1)];
            let parts = [c.sectors_of(0), c.sectors_of(1)];
            for s in 0..2 {
                for p in c.front(s) {
                    if k.divs.iter().any(|d| d.side == s && d.at == p) {
                        continue;
                    }
                    let g = parts[s].iter().position(|x| x.contains(&p));
                    let going = k.divs.iter().filter(|d| d.side == s && d.way.last() == Some(&p)).count();
                    let mine = g.map_or(0, |g| k.divs.iter().filter(|d| d.side == s && d.group == g).count());
                    let free = g.map_or(0, |g| k.divs.iter().filter(|d| d.side == s && d.group == g && d.way.is_empty()).count());
                    println!("      дыра {s}: пров {p} reach {} группа {:?} (дивизий {mine}, стоят {free}), идут туда {going}, враг рядом {}", if reach[s][p] == NO { "нет".to_string() } else { reach[s][p].to_string() }, g, c.th.nbr[p].iter().filter(|&&r| c.owner[r] != s).count());
                    for d in k.divs.iter().filter(|d| d.side == s && d.way.last() == Some(&p)).take(3) {
                        let fight = k.fights.iter().find(|f| f.at == d.at).map(|f| f.side);
                        println!("         дивизия {} в {} путь {:?} пройдено {:.2} отход {:.0} орг {:.2} бой у неё {:?} скорость {:.2}", d.no, d.at, d.way, d.gone, d.rest, d.org, fight, Corps::speed(pack, &d.has));
                    }
                }
            }
        }
    });
    let k = c.corps.as_ref().unwrap();
    for (day, s, line) in k.news.iter().take(8) {
        println!("    день {day}: {}: {line}", names[*s]);
    }
    for s in 0..2 {
        let eq: Vec<f32> = k.divs.iter().filter(|d| d.side == s).map(|d| k.equipped(pack, d)).collect();
        println!(
            "  {}: {}; оснащены в среднем на {:.0}%, трофеев {:.1} ед., брошено {:.1}; нехватки {:?}",
            names[s],
            staff_words(pack, k, s),
            100.0 * eq.iter().sum::<f32>() / eq.len().max(1) as f32,
            k.trophies[s].iter().sum::<f32>(),
            k.abandoned[s].iter().sum::<f32>(),
            k.lack[s].map(|x| (x * 100.0).round() / 100.0)
        );
    }
    let mut days: Vec<u32> = Vec::new();
    let _ = &mut days;
    println!(
        "  итог: {}; боёв {}, побед наступавших {}/{} и {}/{}; голод: окружённые теряют {:.2}% в день, прочие {:.3}%; {:.0} с",
        match c.fallen {
            Some(s) => format!("{} капитулирует на {}-м месяце", names[s], c.month),
            None => "война идёт".to_string(),
        },
        c.fought.len(),
        book.won[0],
        book.fought[0],
        book.won[1],
        book.fought[1],
        100.0 * book.cut_loss / book.cut_days.max(1.0),
        100.0 * book.free_loss / book.free_days.max(1.0),
        t0.elapsed().as_secs_f32()
    );
}

pub fn main() {
    let args: Vec<String> = crate::args().into_iter().collect();
    let arg = |k: &str| args.iter().position(|a| a == k).map(|i| args[i + 1].clone());
    let path = arg("--pack").unwrap_or("packs/industrial.toml".into());
    let seed: u64 = arg("--seed").map_or(1, |v| v.parse().expect("--seed N"));
    let months: u32 = arg("--months").map_or(12, |v| v.parse().expect("--months M"));
    let pack = Pack::load(&path).unwrap_or_else(|e| panic!("{e}"));
    *PACK_PATH.write().unwrap() = path.clone();
    if !args.iter().any(|a| a == "--checks") {
        chronicle(&pack, seed, months);
    }
    if !args.iter().any(|a| a == "--quick") {
        let m: u32 = arg("--check-months").map_or(18, |v| v.parse().expect("--check-months M"));
        println!("══════ проверки войны дивизиями ({}) ══════", pack.name);
        if !checks(&pack, m) {
            std::process::exit(1);
        }
    }
}
