//! Battles of one era: a chronicle of one battle, a table of which army beats which, and
//! self-checks at the end (exit code 1 on a FAIL).
//!
//! `checks battle [--pack packs/x.toml] [--battles N]`

use rayon::prelude::*;
use strategy::military::battle::{battle, Army, Kind};
use strategy::pack::{Line, Pack};
use strategy::rng::{hash64, Rng};
use strategy::world::map::Terrain;

const ENGINE: &str = include_str!("../../military/battle.rs");
/// Armies of the checks and tables are this many lines of the front deep (in the era's
/// cheapest soldiers' worth: what an army costs differs from era to era).
const LINES: f32 = 4.0;
const GROUND: [Terrain; 6] = [Terrain::Plains, Terrain::Desert, Terrain::Forest, Terrain::Hills, Terrain::Marsh, Terrain::Mountains];

fn place(t: Terrain) -> &'static str {
    match t {
        Terrain::Plains => "равнина",
        Terrain::Desert => "пустыня",
        Terrain::Forest => "лес",
        Terrain::Hills => "холмы",
        Terrain::Marsh => "болото",
        Terrain::Mountains => "горы",
        Terrain::Jungle => "джунгли",
        Terrain::Tundra => "тундра",
        _ => "вода",
    }
}

/// An army of `worth`, spent on unit kinds in the given shares of money.
fn army(pack: &Pack, shares: &[(usize, f32)], worth: f32) -> Army {
    let mut a = Army::new(pack);
    let all: f32 = shares.iter().map(|s| s.1).sum();
    for &(u, s) in shares {
        a.count[u] += worth * s / all / pack.units[u].cost;
    }
    a
}

/// Share of battles the first army wins (a battle neither side gives up counts as half).
fn wins(pack: &Pack, t: Terrain, a: &Army, b: &Army, kind: Kind, n: u32, seed: u64) -> f32 {
    let won: f32 = (0..n)
        .into_par_iter()
        .map(|i| {
            let mut rng = Rng::new(hash64(seed ^ (i as u64).wrapping_mul(0x9E37)));
            match battle(pack, t, [a, b], kind, &mut rng, false).winner {
                Some(0) => 1.0,
                Some(_) => 0.0,
                None => 0.5,
            }
        })
        .sum();
    won / n as f32
}

/// Armies the tables compare, all of the same worth: each kind that holds the front alone and
/// in pairs and all together, each of these alone and with a quarter of its money on one kind
/// of the second line or of the air.
fn candidates(pack: &Pack) -> Vec<(String, Vec<(usize, f32)>)> {
    let front: Vec<usize> = (0..pack.units.len()).filter(|&u| pack.units[u].line == Line::Front).collect();
    let extra: Vec<usize> = (0..pack.units.len()).filter(|&u| pack.units[u].line != Line::Front).collect();
    let mut bases: Vec<Vec<usize>> = front.iter().map(|&u| vec![u]).collect();
    for i in 0..front.len() {
        for j in i + 1..front.len() {
            bases.push(vec![front[i], front[j]]);
        }
    }
    if front.len() > 2 {
        bases.push(front.clone());
    }
    let mut out = Vec::new();
    for base in &bases {
        let core: Vec<(usize, f32)> = base.iter().map(|&u| (u, 1.0 / base.len() as f32)).collect();
        let name = base.iter().map(|&u| pack.units[u].name.clone()).collect::<Vec<_>>().join("+");
        out.push((name.clone(), core.clone()));
        for &e in &extra {
            let mut mix: Vec<(usize, f32)> = core.iter().map(|&(u, s)| (u, s * 0.75)).collect();
            mix.push((e, 0.25));
            out.push((format!("{name}+{}", pack.units[e].name), mix));
        }
    }
    out
}

pub fn main() {
    let args: Vec<String> = crate::args().into_iter().collect();
    let path = args.iter().position(|a| a == "--pack").map_or("packs/industrial.toml".to_string(), |i| args[i + 1].clone());
    let n: u32 = args.iter().position(|a| a == "--battles").map_or(200, |i| args[i + 1].parse().expect("--battles N"));
    let pack = Pack::load(&path).unwrap_or_else(|e| panic!("{e}"));
    let Some(war) = &pack.war else {
        println!("{}: в наборе нет раздела [war]", pack.name);
        return;
    };
    println!("══════ {} — бой ══════", pack.name);
    for u in &pack.units {
        let fire: Vec<String> = war.classes.iter().zip(&u.fire).filter(|(_, f)| **f > 0.0).map(|(c, f)| format!("{c} {f}")).collect();
        println!(
            "  {:<24} {:<7} {:<8} цена {:>5.1} | огонь: {} | стойкость {} укрытие {} скорость {}",
            u.name,
            war.classes[u.class],
            match u.line {
                Line::Front => "фронт",
                Line::Support => "за линией",
                Line::Air => "воздух",
            },
            u.cost,
            fire.join(", "),
            u.toughness,
            u.cover,
            u.speed
        );
    }
    let mut checks: Vec<(String, bool, String)> = Vec::new();

    // the engine must not know this era
    let ids: Vec<&String> = pack.units.iter().map(|u| &u.id).chain(war.classes.iter()).collect();
    let leaked: Vec<&&String> = ids.iter().filter(|id| ENGINE.contains(&format!("\"{id}\""))).collect();
    checks.push(("движок не знает этой эпохи".into(), leaked.is_empty(), format!("упоминаний в коде: {}", leaked.len())));

    // the plain soldier of the era: the kind of the front line that costs least
    let line = (0..pack.units.len()).filter(|&u| pack.units[u].line == Line::Front).min_by(|&a, &b| pack.units[a].cost.total_cmp(&pack.units[b].cost)).unwrap();
    let soldiers = |worth: f32| army(&pack, &[(line, 1.0)], worth);
    let name = &pack.units[line].name;
    #[allow(non_snake_case)]
    let BUDGET = LINES * war.width * pack.units[line].cost / pack.units[line].width;

    // a chronicle: the mix with most kinds attacks the plain soldiers in hills
    {
        let all: Vec<(usize, f32)> = (0..pack.units.len()).map(|u| (u, 1.0)).collect();
        let a = army(&pack, &all, BUDGET);
        let b = soldiers(BUDGET);
        let mut rng = Rng::new(7);
        let out = battle(&pack, Terrain::Hills, [&a, &b], Kind::Attack, &mut rng, true);
        println!("\n  Бой в холмах. Наступают: {}. Обороняются: {}.", a.describe(&pack), b.describe(&pack));
        for l in &out.log {
            println!("{l}");
        }
        println!("  Осталось: первые — {}; вторые — {}.", out.left[0].describe(&pack), out.left[1].describe(&pack));
    }

    // control: twins meeting in the open
    let even = wins(&pack, Terrain::Plains, &soldiers(BUDGET), &soldiers(BUDGET), Kind::Meeting, n * 2, 1);
    checks.push(("контроль: равные встречаются — орёл или решка".into(), (0.42..=0.58).contains(&even), format!("первые побеждают в {:.0}% ({name})", even * 100.0)));
    let two = wins(&pack, Terrain::Plains, &soldiers(2.0 * BUDGET), &soldiers(BUDGET), Kind::Meeting, n, 2);
    checks.push(("вдвое больше — почти всегда победа".into(), two >= 0.9, format!("на равнине вдвое больший побеждает в {:.0}%", two * 100.0)));
    let held_plain = 1.0 - wins(&pack, Terrain::Plains, &soldiers(BUDGET), &soldiers(BUDGET), Kind::Attack, n, 3);
    let held_hill = 1.0 - wins(&pack, Terrain::Mountains, &soldiers(BUDGET), &soldiers(BUDGET), Kind::Attack, n, 4);
    checks.push((
        "обороняться легче, в горах — намного".into(),
        (0.5..=0.85).contains(&held_plain) && held_hill >= 0.9,
        format!("равные силы: оборона держится на равнине в {:.0}%, в горах в {:.0}%", held_plain * 100.0, held_hill * 100.0),
    ));
    // how many times stronger an attacker must be to win half the time, against defenders who
    // fill the line once (a defence many lines deep is not worn out in a battle's days at all)
    let needed = |t: Terrain, lines: f32, seed: u64| {
        let unit = &pack.units[line];
        let base = lines * unit.cost * war.width * war.frontage[t as usize] / unit.width;
        let (mut lo, mut hi) = (0.5f32, 16.0f32);
        for _ in 0..12 {
            let mid = (lo * hi).sqrt();
            if wins(&pack, t, &soldiers(mid * base), &soldiers(base), Kind::Attack, n, seed) >= 0.5 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        (lo * hi).sqrt()
    };
    let (plain_x, hill_x, deep_x) = (needed(Terrain::Plains, 1.0, 5), needed(Terrain::Mountains, 1.0, 6), needed(Terrain::Mountains, 2.0, 7));
    checks.push((
        "в горах нужен перевес в разы больше".into(),
        hill_x >= 1.5 * plain_x && deep_x > hill_x,
        format!("перевес для половины побед: равнина ×{plain_x:.1}, горы ×{hill_x:.1}, горы против обороны в две линии ×{deep_x:.1}"),
    ));
    let mut hungry = soldiers(BUDGET);
    hungry.supplied = 0.0;
    let starved = wins(&pack, Terrain::Plains, &hungry, &soldiers(BUDGET), Kind::Meeting, n, 8);
    checks.push(("без снабжения не воюют".into(), starved <= 0.15, format!("равные, у первых нет припасов: побеждают в {:.0}%", starved * 100.0)));

    // which army beats which, at equal worth, meeting on each kind of ground
    let cands = candidates(&pack);
    let armies: Vec<Army> = cands.iter().map(|(_, s)| army(&pack, s, BUDGET)).collect();
    let nc = cands.len();
    let m = n.min(100).max(20);
    let jobs: Vec<(usize, usize, usize)> = (0..GROUND.len()).flat_map(|t| (0..nc).flat_map(move |i| (0..nc).map(move |j| (t, i, j)))).collect();
    let table: Vec<f32> = jobs
        .par_iter()
        .map(|&(t, i, j)| if i == j { 0.5 } else { wins(&pack, GROUND[t], &armies[i], &armies[j], Kind::Meeting, m, 1000 + (t * nc * nc + i * nc + j) as u64) })
        .collect();
    let rate = |t: usize, i: usize, j: usize| table[t * nc * nc + i * nc + j];
    // pure armies against each other, on plains and in mountains
    let pure: Vec<usize> = (0..nc).filter(|&i| cands[i].1.len() == 1).collect();
    for t in [0usize, 5] {
        println!("\n  {}: сколько боёв выигрывает строка у столбца (равная цена, встречный бой)", place(GROUND[t]));
        println!("    {:<24}{}", "", pure.iter().map(|&j| format!("{:>12}", cands[j].0.chars().take(11).collect::<String>())).collect::<String>());
        for &i in &pure {
            println!("    {:<24}{}", cands[i].0, pure.iter().map(|&j| format!("{:>11.0}%", rate(t, i, j) * 100.0)).collect::<String>());
        }
    }
    // the best answer to each army on each ground
    let mut best_of = vec![vec![0usize; nc]; GROUND.len()];
    let mut answers: Vec<usize> = Vec::new();
    for t in 0..GROUND.len() {
        for j in 0..nc {
            let b = (0..nc).max_by(|&a, &b| rate(t, a, j).total_cmp(&rate(t, b, j))).unwrap();
            best_of[t][j] = b;
            if !answers.contains(&b) {
                answers.push(b);
            }
        }
    }
    println!("\n  лучший ответ (на равнине / в горах):");
    for &j in &pure {
        println!(
            "    против «{}»: {} ({:.0}%) / {} ({:.0}%)",
            cands[j].0,
            cands[best_of[0][j]].0,
            rate(0, best_of[0][j], j) * 100.0,
            cands[best_of[5][j]].0,
            rate(5, best_of[5][j], j) * 100.0
        );
    }
    let dominant: Vec<usize> = (0..nc).filter(|&i| (0..GROUND.len()).all(|t| (0..nc).all(|j| j == i || rate(t, i, j) >= 0.55))).collect();
    checks.push((
        "нет войска, что бьёт всех везде".into(),
        dominant.is_empty(),
        if dominant.is_empty() { format!("из {nc} войск равной цены") } else { format!("бьёт всех: {}", cands[dominant[0]].0) },
    ));
    checks.push((
        "лучший ответ зависит от врага и местности".into(),
        answers.len() >= 4,
        format!("разных лучших ответов {} из {nc} войск", answers.len()),
    ));
    let unused: Vec<&str> = (0..pack.units.len()).filter(|&u| !answers.iter().any(|&a| cands[a].1.iter().any(|s| s.0 == u))).map(|u| pack.units[u].name.as_str()).collect();
    checks.push((
        "у каждого рода войск есть своё дело".into(),
        unused.is_empty(),
        if unused.is_empty() { "каждый входит в чей-то лучший ответ".into() } else { format!("ни в одном лучшем ответе: {}", unused.join(", ")) },
    ));
    // the army whose worst result is best, against any of these on any ground
    let worst = |i: usize| (0..GROUND.len()).flat_map(|t| (0..nc).map(move |j| (t, j))).map(|(t, j)| rate(t, i, j)).fold(1.0f32, f32::min);
    let safest = (0..nc).max_by(|&a, &b| worst(a).total_cmp(&worst(b))).unwrap();
    let best_pure = pure.iter().copied().max_by(|&a, &b| worst(a).total_cmp(&worst(b))).unwrap();
    checks.push((
        "смешанное войско надёжнее чистого".into(),
        cands[safest].1.len() > 1,
        format!(
            "лучший худший случай: {} ({:.0}%); у лучшего из чистых, «{}», — {:.0}%",
            cands[safest].0,
            worst(safest) * 100.0,
            cands[best_pure].0,
            worst(best_pure) * 100.0
        ),
    ));

    println!("\nпроверки:");
    let mut ok = true;
    for (name, pass, detail) in &checks {
        ok &= pass;
        println!("  [{}] {:<44} {}", if *pass { "OK" } else { "FAIL" }, name, detail);
    }
    if !ok {
        std::process::exit(1);
    }
}
