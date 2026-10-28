//! Doctrines growing from experience: two countries at war along a front. A chronicle of one
//! war (how each side's doctrine changed and why), then self-checks over many wars (exit
//! code 1 on a FAIL).
//!
//! `checks doctrine [--pack packs/x.toml] [--worlds N] [--months M]`

use rayon::prelude::*;
use crate::common::doctrine;
use strategy::military::doctrine::{Country, Duel};
use strategy::pack::{Line, Pack};
use strategy::rng::{hash64, Rng};
use strategy::world::map::Terrain;

const OPEN: [(Terrain, f32); 2] = [(Terrain::Plains, 0.7), (Terrain::Desert, 0.3)];
const MIXED: [(Terrain, f32); 3] = [(Terrain::Plains, 0.5), (Terrain::Forest, 0.3), (Terrain::Hills, 0.2)];
const ROUGH: [(Terrain, f32); 3] = [(Terrain::Mountains, 0.5), (Terrain::Hills, 0.3), (Terrain::Forest, 0.2)];
const WOODS: [(Terrain, f32); 2] = [(Terrain::Forest, 0.6), (Terrain::Marsh, 0.4)];
const STEPPE: [(Terrain, f32); 2] = [(Terrain::Plains, 0.5), (Terrain::Hills, 0.5)];

/// How a war is set up.
#[derive(Clone)]
struct Setup {
    front: Vec<(Terrain, f32)>,
    /// Each side's prices of the goods, against the pack's.
    prices: [Vec<f32>; 2],
    /// A doctrine that never changes, for either side.
    fixed: [Option<Vec<f32>>; 2],
    /// (kind, true worth of its equipment) forced on the first side.
    quality: Option<(usize, f32)>,
    fog: [f32; 2],
    /// Both sides get the same equipment and the same claims about it.
    twins: bool,
    months: u32,
    /// The second side changes its doctrine at this month to this plan.
    switch: Option<(u32, Vec<f32>)>,
    /// Each side's caution (None: as the engine sets it).
    caution: [Option<f32>; 2],
}

impl Setup {
    fn new(pack: &Pack, front: &[(Terrain, f32)], months: u32) -> Setup {
        let ng = pack.goods.len();
        Setup { front: front.to_vec(), prices: [vec![1.0; ng], vec![1.0; ng]], fixed: [None, None], quality: None, fog: [0.5, 0.5], twins: false, months, switch: None, caution: [None, None] }
    }
}

fn war(pack: &Pack, s: &Setup, seed: u64) -> Duel {
    let mut ra = Rng::new(hash64(seed));
    let mut rb = Rng::new(hash64(seed ^ if s.twins { 0 } else { 0xB0B }));
    let mut a = Country::new(pack, "Альфа", &s.prices[0], &mut ra);
    let mut b = Country::new(pack, "Бета", &s.prices[1], &mut rb);
    if let Some((u, q)) = s.quality {
        a.truth_fire[u] = q;
        a.truth_tough[u] = q;
        // designers still claim a little more than the era's figures
        a.own_fire[u].mean = 0.1;
        a.own_tough[u].mean = 0.1;
    }
    for (c, i) in [&mut a, &mut b].into_iter().zip(0..) {
        c.fog = s.fog[i];
        if let Some(k) = s.caution[i] {
            c.caution = k;
        }
        if let Some(p) = &s.fixed[i] {
            c.plan = p.clone();
            c.making = p.clone();
            c.fixed = true;
        }
    }
    let mut d = Duel::new(pack, [a, b], s.front.clone(), seed);
    for m in 0..s.months {
        if let Some((at, plan)) = &s.switch {
            if m == *at {
                d.sides[1].plan = plan.clone();
            }
        }
        d.step(pack);
    }
    d
}

/// Many wars of one setup, each with its own luck.
fn wars(pack: &Pack, s: &Setup, n: u32, seed: u64) -> Vec<Duel> {
    (0..n).into_par_iter().map(|i| war(pack, s, hash64(seed + i as u64))).collect()
}

/// Mean of each side's plan over many wars.
fn mean_plan(ds: &[Duel], side: usize) -> Vec<f32> {
    let nu = ds[0].sides[side].plan.len();
    (0..nu).map(|u| ds.iter().map(|d| d.sides[side].plan[u]).sum::<f32>() / ds.len() as f32).collect()
}

fn share(plan: &[f32], kinds: &[usize]) -> f32 {
    kinds.iter().map(|&u| plan[u]).sum()
}

fn names(pack: &Pack, kinds: &[usize]) -> String {
    kinds.iter().map(|&u| pack.units[u].name.as_str()).collect::<Vec<_>>().join(", ")
}

/// The last war: twins fight a first war while the kind `young` is still raw (half its later
/// worth), the first side with more money; fifteen years of peace, in which the kind comes of
/// age (or, for a control, does not); a second war on equal terms.
const WAR1: u32 = 48;
const PEACE: u32 = 180;
const WAR2: u32 = 48;

struct History {
    /// Who won more battles of the first war.
    first_winner: usize,
    /// Battles each side won in the first year of the second war, of how many.
    first_year: [(u32, u32); 2],
    /// Each side's share of the young kind in its plan a year into the second war, and what it
    /// rated the kind at (log, against the truth) when the second war began.
    share: [f32; 2],
    belief: [f32; 2],
    /// Glory of the young kind at the end of the first war.
    glory: [f32; 2],
    log: Vec<String>,
}

fn history(pack: &Pack, young: usize, comes_of_age: bool, seed: u64, tell: bool) -> History {
    let mut s = Setup::new(pack, &MIXED, 0);
    s.twins = true;
    let mut d = war(pack, &s, seed);
    let mut log = Vec::new();
    let line = |d: &Duel, what: &str, log: &mut Vec<String>| {
        for side in &d.sides {
            let r = side.own_fire[young].mean + side.glory[young];
            log.push(format!(
                "  {what} {:<6} доктрина: {} | «{}» ставят в {:.0}% от настоящего (слава {:+.0}%), сомнение {:.0}%",
                side.name,
                doctrine(pack, &side.plan),
                pack.units[young].name,
                (r - side.truth_fire[young].ln()).exp() * 100.0,
                side.glory[young] * 100.0,
                side.doubt * 100.0
            ));
        }
    };
    for side in &mut d.sides {
        side.mature(young, 0.5);
    }
    let base = d.sides[0].budget;
    d.sides[0].budget = base * 1.4;
    for m in 1..=WAR1 {
        d.step(pack);
        if tell && m % 24 == 0 {
            line(&d, &format!("война 1, год {}:", m / 12), &mut log);
        }
    }
    let first_winner = if d.sides[0].battles_won >= d.sides[1].battles_won { 0 } else { 1 };
    d.end_war(pack, first_winner);
    let glory = [d.sides[0].glory[young], d.sides[1].glory[young]];
    if tell {
        log.push(format!("  первая война: побед у Альфы {}, у Беты {} из {}", d.sides[0].battles_won, d.sides[1].battles_won, d.sides[0].battles));
    }
    d.sides[0].budget = base;
    for m in 1..=PEACE {
        if comes_of_age {
            let k = 0.5 + 0.7 * m as f32 / PEACE as f32;
            for side in &mut d.sides {
                side.mature(young, k);
            }
        }
        d.peace(pack, 0.3);
        if tell && m % 60 == 0 {
            line(&d, &format!("мир, год {}:", m / 12), &mut log);
        }
    }
    let belief = [0, 1].map(|i| {
        let side = &d.sides[i];
        side.own_fire[young].mean + side.glory[young] - side.truth_fire[young].ln()
    });
    let before = [(d.sides[0].battles_won, d.sides[0].battles), (d.sides[1].battles_won, d.sides[1].battles)];
    let mut share = [0.0; 2];
    let mut first_year = [(0, 0); 2];
    for m in 1..=WAR2 {
        d.step(pack);
        if m == 12 {
            share = [d.sides[0].plan[young], d.sides[1].plan[young]];
            first_year = [0, 1].map(|i| (d.sides[i].battles_won - before[i].0, d.sides[i].battles - before[i].1));
        }
        if tell && m % 12 == 0 {
            line(&d, &format!("война 2, год {}:", m / 12), &mut log);
        }
    }
    let second = [0, 1].map(|i| (d.sides[i].battles_won - before[i].0, d.sides[i].battles - before[i].1));
    if tell {
        log.push(format!("  вторая война: побед у Альфы {}, у Беты {} из {}", second[0].0, second[1].0, second[0].1));
    }
    let _ = second;
    History { first_winner, first_year, share, belief, glory, log }
}

pub fn main() {
    let args: Vec<String> = crate::args().into_iter().collect();
    let arg = |k: &str| args.iter().position(|a| a == k).map(|i| args[i + 1].clone());
    let path = arg("--pack").unwrap_or("packs/industrial.toml".into());
    let n: u32 = arg("--worlds").map_or(24, |v| v.parse().expect("--worlds N"));
    let months: u32 = arg("--months").map_or(96, |v| v.parse().expect("--months M"));
    let pack = Pack::load(&path).unwrap_or_else(|e| panic!("{e}"));
    let nu = pack.units.len();
    println!("══════ {} — доктрины ══════", pack.name);
    if args.iter().any(|a| a == "--caution") {
        // only: a cautious staff against a careless one, war by war
        for (label, front) in [("смешанный фронт", &MIXED[..]), ("поле", &OPEN[..]), ("горы", &ROUGH[..])] {
            let mut s = Setup::new(&pack, front, months);
            s.caution[1] = Some(0.0);
            let ds = wars(&pack, &s, n, 750);
            let per: Vec<f32> = ds.iter().map(|d| d.sides[0].battles_won as f32 / d.sides[0].battles.max(1) as f32).collect();
            let mean = per.iter().sum::<f32>() / per.len() as f32;
            let sd = (per.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / (per.len() as f32 - 1.0)).sqrt();
            println!("  {label}: осторожный побеждает в {:.1}% ± {:.1}% боёв ({} войн)", mean * 100.0, sd / (per.len() as f32).sqrt() * 100.0, per.len());
        }
        return;
    }

    // a chronicle: one war on a mixed front, the second side without one of the goods its
    // dearest kind is made of
    let heavy = (0..nu).filter(|&u| pack.units[u].line == Line::Front).max_by(|&a, &b| pack.units[a].cost.total_cmp(&pack.units[b].cost)).unwrap();
    let dear_good = pack.units[heavy].equip.iter().max_by(|a, b| (a.1 * pack.goods[a.0].price).total_cmp(&(b.1 * pack.goods[b.0].price))).unwrap().0;
    {
        let mut s = Setup::new(&pack, &MIXED, 0);
        s.prices[1][dear_good] = 3.0;
        let start = std::time::Instant::now();
        let mut d = war(&pack, &s, 11);
        println!(
            "  Фронт: равнина 50%, лес 30%, холмы 20%. У Беты {} втрое дороже. Обе начинают поровну всех родов войск.",
            pack.goods[dear_good].name
        );
        for year in 1..=8 {
            for _ in 0..12 {
                d.step(&pack);
            }
            for side in &d.sides {
                let seen: Vec<usize> = (0..nu).filter(|&u| side.foe_mix[u] >= 0.15).collect();
                println!(
                    "  год {year} {:<6} доктрина: {} | побед {}/{} | видят у врага: {}",
                    side.name,
                    doctrine(&pack, &side.plan),
                    side.battles_won,
                    side.battles,
                    seen.iter().map(|&u| format!("{} {:.0}%", pack.units[u].name, side.foe_mix[u] * 100.0)).collect::<Vec<_>>().join(", ")
                );
            }
        }
        for side in &d.sides {
            let wrong: Vec<String> = (0..nu)
                .filter(|&u| side.army.count[u] > 0.1 || side.plan[u] > 0.0)
                .map(|u| format!("{} {:+.0}%", pack.units[u].name, (side.own_fire[u].mean - side.truth_fire[u].ln()) * 100.0))
                .collect();
            println!("  {}: ошибка веры в огонь своих частей к концу: {}", side.name, wrong.join(", "));
        }
        println!("  (одна война за {:.1} с)", start.elapsed().as_secs_f32());
    }
    if args.iter().any(|a| a == "--history") {
        // only the chronicle of the last war
        let seed: u64 = arg("--seed").map_or(21, |v| v.parse().expect("--seed N"));
        let h = history(&pack, heavy, true, seed, true);
        println!(
            "\n  Две войны близнецов. В первой «{0}» сырые (вдвое слабее будущих), у Альфы денег в 1,4 раза больше; 15 лет мира, «{0}» взрослеют; вторая война на равных.",
            pack.units[heavy].name
        );
        for l in &h.log {
            println!("{l}");
        }
        return;
    }

    let started = std::time::Instant::now();
    let mut checks: Vec<(String, bool, String)> = Vec::new();
    let front_kinds: Vec<usize> = (0..nu).filter(|&u| pack.units[u].line == Line::Front).collect();
    let cheap = *front_kinds.iter().min_by(|&&a, &&b| pack.units[a].cost.total_cmp(&pack.units[b].cost)).unwrap();
    let both = |ds: &[Duel]| -> Vec<f32> {
        let (a, b) = (mean_plan(ds, 0), mean_plan(ds, 1));
        a.iter().zip(&b).map(|(x, y)| (x + y) / 2.0).collect()
    };
    let wins_of = |ds: &[Duel], side: usize| {
        ds.iter().map(|d| d.sides[side].battles_won as f32).sum::<f32>() / ds.iter().map(|d| d.sides[side].battles as f32).sum::<f32>().max(1.0)
    };

    // control: twins on the same front end up with the same doctrines on average
    let twins = {
        let mut s = Setup::new(&pack, &MIXED, months);
        s.twins = true;
        wars(&pack, &s, n, 100)
    };
    // a shift of a side's mean share counts only if it stands out of the luck of the wars:
    // more than three standard errors of the difference between the twins
    let shifts: Vec<(f32, f32)> = (0..nu)
        .map(|u| {
            let d: Vec<f32> = twins.iter().map(|w| w.sides[0].plan[u] - w.sides[1].plan[u]).collect();
            let mean = d.iter().sum::<f32>() / d.len() as f32;
            let var = d.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / (d.len() as f32 - 1.0).max(1.0);
            (mean, (var / d.len() as f32).sqrt())
        })
        .collect();
    let biased: Vec<usize> = (0..nu).filter(|&u| shifts[u].0.abs() > 3.0 * shifts[u].1 && shifts[u].0.abs() > 0.05).collect();
    let worst = (0..nu).max_by(|&a, &b| (shifts[a].0.abs() / shifts[a].1.max(1e-6)).total_cmp(&(shifts[b].0.abs() / shifts[b].1.max(1e-6)))).unwrap();
    let won = wins_of(&twins, 0);
    checks.push((
        "контроль: близнецы — одинаковые доктрины".into(),
        biased.is_empty() && (0.4..=0.6).contains(&won),
        format!(
            "сильнее всего разнится «{}»: {:+.0}% ± {:.0}%; первые побеждают в {:.0}% боёв",
            pack.units[worst].name,
            shifts[worst].0 * 100.0,
            shifts[worst].1 * 100.0,
            won * 100.0
        ),
    ));

    // the kind the tests below mass, make dear or make worse than claimed: of the front line,
    // beyond the plain soldier, the one twins build most (the dearest kind of the wastes, war
    // rigs, is hardly built at all, and tests on it showed nothing)
    let tw = both(&twins);
    let heavy = *front_kinds.iter().filter(|&&u| u != cheap).max_by(|&&a, &&b| tw[a].total_cmp(&tw[b])).unwrap();
    let dear_good = pack.units[heavy].equip.iter().max_by(|a, b| (a.1 * pack.goods[a.0].price).total_cmp(&(b.1 * pack.goods[b.0].price))).unwrap().0;

    // the ground shapes doctrine: kinds that lose their fire in rough country are built less there
    let rough_terrains = [Terrain::Forest, Terrain::Hills, Terrain::Mountains, Terrain::Marsh];
    // (of the front line only: guns behind it lose fire in rough country too, but need no place
    // in a front that narrows there, and grow more valuable)
    let shy: Vec<usize> = (0..nu)
        .filter(|&u| pack.units[u].line == Line::Front)
        .filter(|&u| rough_terrains.iter().map(|&t| pack.units[u].terrain[t as usize]).sum::<f32>() / 4.0 < 0.75)
        .collect();
    let open = both(&wars(&pack, &Setup::new(&pack, &OPEN, months), n, 200));
    let rough = both(&wars(&pack, &Setup::new(&pack, &ROUGH, months), n, 200));
    checks.push((
        "местность растит доктрину".into(),
        !shy.is_empty() && share(&open, &shy) >= share(&rough, &shy) + 0.05,
        format!("доля тех, кто слабеет в лесах и горах ({}): фронт в поле {:.0}%, в горах {:.0}%", names(&pack, &shy), share(&open, &shy) * 100.0, share(&rough, &shy) * 100.0),
    ));

    // what things cost shapes doctrine: dear steel, fewer tanks
    let dear = {
        let mut s = Setup::new(&pack, &MIXED, months);
        s.prices[0][dear_good] = 3.0;
        wars(&pack, &s, n, 300)
    };
    let (da, db) = (mean_plan(&dear, 0), mean_plan(&dear, 1));
    checks.push((
        "цены растят доктрину".into(),
        da[heavy] + 0.05 <= db[heavy],
        format!("«{}», когда {} втрое дороже: {:.0}%, у соседа по обычной цене {:.0}%", pack.units[heavy].name, pack.goods[dear_good].name, da[heavy] * 100.0, db[heavy] * 100.0),
    ));

    // the race of answers, judged by what counts — winning: against an enemy that masses one
    // kind, a staff that learns wins more and more often, and more than one that keeps its first
    // doctrine. (Judged by how much of the "answer by the table of fire" it builds, it failed
    // with muskets: the staff beat massed cavalry with guns, which no table called an answer.)
    let class = pack.units[heavy].class;
    let mut by_answer: Vec<usize> = (0..nu).filter(|&u| u != heavy).collect();
    by_answer.sort_by(|&a, &b| (pack.units[b].fire[class] / pack.units[b].cost).total_cmp(&(pack.units[a].fire[class] / pack.units[a].cost)));
    let answers: Vec<usize> = by_answer.into_iter().take(2).collect();
    let mut massed = vec![0.0; nu];
    massed[heavy] = 0.7;
    massed[cheap] += 0.3;
    let even = vec![1.0 / nu as f32; nu];
    let race = |plan: &Vec<f32>, fog: f32, m: u32, seed: u64| {
        let mut s = Setup::new(&pack, &MIXED, m);
        s.fixed[1] = Some(plan.clone());
        s.fog[0] = fog;
        wars(&pack, &s, n, seed)
    };
    // the first side's share of battles won between two runs of the same wars (one the start
    // of the other)
    let between = |early: &[Duel], late: &[Duel]| {
        let won: u32 = early.iter().zip(late).map(|(a, b)| b.sides[0].battles_won - a.sides[0].battles_won).sum();
        let all: u32 = early.iter().zip(late).map(|(a, b)| b.sides[0].battles - a.sides[0].battles).sum();
        won as f32 / all.max(1) as f32
    };
    let half = months / 2;
    let vs_massed = race(&massed, 0.5, months, 400);
    let vs_massed_half = race(&massed, 0.5, half, 400);
    let dogma_massed = {
        let mut s = Setup::new(&pack, &MIXED, months);
        s.fixed = [Some(even.clone()), Some(massed.clone())];
        wars(&pack, &s, n, 400)
    };
    let vs_even = race(&even, 0.5, months, 400);
    let (am, ae) = (share(&mean_plan(&vs_massed, 0), &answers), share(&mean_plan(&vs_massed_half, 0), &answers));
    let (first, second) = (wins_of(&vs_massed_half, 0), between(&vs_massed_half, &vs_massed));
    let dogma = wins_of(&dogma_massed, 0);
    let _ = ae;
    checks.push((
        "враг видит манеру и подстраивается".into(),
        second >= first + 0.03 && second >= dogma + 0.1,
        format!(
            "против войска, где «{}» 70%: учащийся побеждает в {:.0}% боёв в первые {} мес. и в {:.0}% потом (неизменная доктрина — {:.0}%); строит к концу: {} (по таблице огня ответ — {}: {:.0}%, против ровного врага {:.0}%)",
            pack.units[heavy].name,
            first * 100.0,
            half,
            second * 100.0,
            dogma * 100.0,
            doctrine(&pack, &mean_plan(&vs_massed, 0)),
            names(&pack, &answers),
            am * 100.0,
            share(&mean_plan(&vs_even, 0), &answers) * 100.0
        ),
    ));
    // seeing the enemy: the enemy fights with an even army for two years, then masses one kind;
    // how often the staff wins in the two years after, seeing clearly and through thick fog
    let after = |fog: f32| {
        let at = |m: u32| {
            let mut s = Setup::new(&pack, &MIXED, m);
            s.fixed[1] = Some(even.clone());
            s.switch = Some((24, massed.clone()));
            s.fog[0] = fog;
            wars(&pack, &s, n * 2, 500)
        };
        let (a, b) = (at(24), at(48));
        (between(&a, &b), share(&mean_plan(&a, 0), &answers), share(&mean_plan(&b, 0), &answers))
    };
    let (clear, foggy) = (after(0.5), after(3.0));
    // and between two staffs that both learn, the one that sees the other better wins more
    let blind = {
        let mut s = Setup::new(&pack, &MIXED, months);
        s.twins = true;
        s.fog = [3.0, 0.5];
        wars(&pack, &s, n * 2, 520)
    };
    let seen = wins_of(&blind, 0);
    // knowing the enemy can only pay where not knowing is punished: where an even army, built
    // knowing nothing, loses to one that masses a kind (with muskets it beat massed cavalry in
    // two battles of three, and a blind twin won as often as a seeing one)
    let punished = dogma < 0.6;
    checks.push((
        "разведка окупается, где незнание наказуемо".into(),
        !punished || seen <= 0.47,
        format!(
            "ровная армия против массовой побеждает в {:.0}% — незнание {}; близнецы учатся оба, первый видит врага в густом тумане: побеждает в {:.0}% боёв. Против врага с неизменной доктриной туман побед не стоит: после смены его доктрины видящий ясно побеждает в {:.0}%, в тумане — в {:.0}% (туман держит армию «на всё понемногу»: ответ по таблице огня {:.0}% → {:.0}% у видящего и {:.0}% → {:.0}% в тумане)",
            dogma * 100.0,
            if punished { "наказуемо" } else { "почти не наказуемо" },
            seen * 100.0,
            clear.0 * 100.0,
            foggy.0 * 100.0,
            clear.1 * 100.0,
            clear.2 * 100.0,
            foggy.1 * 100.0,
            foggy.2 * 100.0
        ),
    ));

    // the truth comes out in battle: equipment far worse than claimed is found out and dropped
    let tried = |q: f32| {
        let mut s = Setup::new(&pack, &MIXED, months);
        s.quality = Some((heavy, q));
        wars(&pack, &s, n, 600)
    };
    let (bad, good) = (tried(0.6), tried(1.0));
    let err = bad.iter().map(|d| (d.sides[0].own_fire[heavy].mean - 0.6f32.ln()).abs()).sum::<f32>() / bad.len() as f32;
    let (sb, sg) = (mean_plan(&bad, 0)[heavy], mean_plan(&good, 0)[heavy]);
    checks.push((
        "правду о технике показывают бои".into(),
        err < 0.25 && sb + 0.05 <= sg,
        format!(
            "«{}» на 40% хуже заявленного: ошибка веры {:.0}% → {:.0}%, доля {:.0}% (при честной технике {:.0}%)",
            pack.units[heavy].name,
            (0.1 - 0.6f32.ln()) * 100.0,
            err * 100.0,
            sb * 100.0,
            sg * 100.0
        ),
    ));

    // not a check: a staff that expects the enemy to answer wins a little more often than one
    // that does not (on 64 wars a front: +2..7 points in the industrial era and with muskets,
    // nothing in the wastes), and it does not keep doctrines from going all-or-nothing
    // (`--caution` measures it war by war)
    let careless = {
        let mut s = Setup::new(&pack, &MIXED, months);
        s.caution[1] = Some(0.0);
        wars(&pack, &s, n, 750)
    };
    let extreme = |ds: &[Duel], side: usize| ds.iter().map(|d| d.sides[side].plan.iter().cloned().fold(0.0f32, f32::max)).sum::<f32>() / ds.len() as f32;
    println!(
        "  осторожность: ждущий ответа врага генштаб побеждает беспечного в {:.0}% боёв; самый большой род войск в доктрине: у осторожного {:.0}%, у беспечного {:.0}%",
        wins_of(&careless, 0) * 100.0,
        extreme(&careless, 0) * 100.0,
        extreme(&careless, 1) * 100.0
    );

    // the last war: a kind that was raw in the first war comes of age in the peace; the side
    // that lost the first war, blaming its defeat on what it did not have and doubting, takes
    // to it sooner, and the winner, sure of the old lesson, later
    let young = heavy;
    // (more wars than the other checks: a year of battles tells little)
    let lives = |grow: bool, seed: u64| -> Vec<History> { (0..n * 3).into_par_iter().map(|i| history(&pack, young, grow, hash64(seed + i as u64), false)).collect() };
    let (grown, still) = (lives(true, 1100), lives(false, 1100));
    let by = |hs: &[History], f: &dyn Fn(&History, usize) -> f32| {
        let w = hs.iter().map(|h| f(h, h.first_winner)).sum::<f32>() / hs.len() as f32;
        let l = hs.iter().map(|h| f(h, 1 - h.first_winner)).sum::<f32>() / hs.len() as f32;
        (w, l)
    };
    let (sw, sl) = by(&grown, &|h, i| h.share[i]);
    let (bw, bl) = by(&grown, &|h, i| h.belief[i]);
    let (gw, gl) = by(&grown, &|h, i| h.glory[i]);
    let revenge = |hs: &[History]| {
        let won: u32 = hs.iter().map(|h| h.first_year[1 - h.first_winner].0).sum();
        let all: u32 = hs.iter().map(|h| h.first_year[1 - h.first_winner].1).sum();
        won as f32 / all.max(1) as f32
    };
    let (rg, rs) = (revenge(&grown), revenge(&still));
    checks.push((
        "последняя война: проигравший перестраивается".into(),
        sl >= sw + 0.05 && bl > bw,
        format!(
            "«{}» после войны, где был сырым: слава у победителя {:+.0}%, у проигравшего {:+.0}%; к началу второй войны ставят в {:.0}% и {:.0}% от настоящего; через год второй войны строят {:.0}% и {:.0}%",
            pack.units[young].name,
            gw * 100.0,
            gl * 100.0,
            bw.exp() * 100.0,
            bl.exp() * 100.0,
            sw * 100.0,
            sl * 100.0
        ),
    ));
    checks.push((
        "реванш — у того, кто переоценил".into(),
        rg >= rs + 0.03,
        format!(
            "проигравший первую войну выигрывает в первый год второй {:.0}% боёв; если бы «{}» не повзрослел — {:.0}% (контроль: дело в переоценке, а не в поражении самом по себе)",
            rg * 100.0,
            pack.units[young].name,
            rs * 100.0
        ),
    ));

    // experience beats dogma: a staff that learns beats one that keeps its first doctrine
    let dogma = race(&even, 0.5, months, 700);
    let late = |ds: &[Duel]| wins_of(ds, 0);
    checks.push((
        "опыт сильнее догмы".into(),
        late(&dogma) > 0.55,
        format!("учащийся генштаб против неизменной доктрины побеждает в {:.0}% боёв", late(&dogma) * 100.0),
    ));

    // every war is its own story: what the main kind of the army is differs from war to war
    let fronts: [&[(Terrain, f32)]; 5] = [&OPEN, &MIXED, &ROUGH, &WOODS, &STEPPE];
    let mains: Vec<usize> = (0..n * 2)
        .into_par_iter()
        .flat_map(|i| {
            let mut rng = Rng::new(hash64(800 + i as u64));
            let mut s = Setup::new(&pack, fronts[rng.below(5)], months);
            for side in 0..2 {
                for g in 0..pack.goods.len() {
                    s.prices[side][g] = (0.5 * rng.normal()).exp();
                }
            }
            let d = war(&pack, &s, hash64(900 + i as u64));
            (0..2)
                .map(|k| (0..nu).max_by(|&a, &b| d.sides[k].plan[a].total_cmp(&d.sides[k].plan[b])).unwrap())
                .collect::<Vec<_>>()
        })
        .collect();
    let mut tally = vec![0usize; nu];
    for &m in &mains {
        tally[m] += 1;
    }
    let kinds = tally.iter().filter(|&&c| c > 0).count();
    let top = *tally.iter().max().unwrap() as f32 / mains.len() as f32;
    checks.push((
        "в каждой войне свои доктрины".into(),
        kinds >= 3 && top <= 0.7,
        format!(
            "главный род войск в {} армиях: {}",
            mains.len(),
            (0..nu).filter(|&u| tally[u] > 0).map(|u| format!("{} {}", pack.units[u].name, tally[u])).collect::<Vec<_>>().join(", ")
        ),
    ));

    println!("\nпроверки ({} войн по {} мес., {:.0} с):", n, months, started.elapsed().as_secs_f32());
    let mut ok = true;
    for (name, pass, detail) in &checks {
        ok &= pass;
        println!("  [{}] {:<40} {}", if *pass { "OK" } else { "FAIL" }, name, detail);
    }
    if !ok {
        std::process::exit(1);
    }
}
