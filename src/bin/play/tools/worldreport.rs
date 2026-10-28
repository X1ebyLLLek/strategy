//! A report of the world the past makes (`play --world [--seeds N] [--seed S] [--pack …]`): only the
//! map and its two hundred years, no game — how the countries are governed, what they believe, how
//! much they know, how often governments fell and laws changed, a few countries' stories. To see
//! at once whether the worlds come out varied (and to tune the order of societies by it).

use strategy::pack::Pack;
use strategy::world::countries;
use strategy::world::diplomacy::Kind;
use strategy::world::history::History;
use strategy::world::map;
use strategy::world::polity::{Regime, MOVE, MOVES};

pub fn report(pack: &Pack, first: u64, seeds: u64) {
    let mut regimes = vec![0usize; Regime::ALL.len()];
    let (mut revolutions, mut coups, mut reforms, mut countries) = (0u32, 0u32, 0u32, 0usize);
    let mut faiths = Vec::new();
    let mut techs: Vec<usize> = Vec::new();
    let mut lit: Vec<f32> = Vec::new();
    let mut moves = vec![0.0f32; MOVES];
    let mut pacts = vec![0usize; Kind::ALL.len()];
    let (mut open_wars, mut in_war, mut past_wars, mut dead, mut moved) = (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut sizes: Vec<usize> = Vec::new();
    let t0 = std::time::Instant::now();
    for seed in first..first + seeds {
        let world = map::generate(&crate::map_params(seed)).0;
        let mut pol = countries::generate(&world, seed, 30, pack);
        let before: Vec<u32> = pol.states.iter().map(|s| s.country).collect();
        let mut was: Vec<usize> = pol.countries.iter().map(|c| c.states.len()).filter(|&k| k > 0).collect();
        was.sort();
        println!("мир {seed}: областей у страны до прошлого — меньше всех {}, середина {}, больше всех {} (всего {})", was[0], was[was.len() / 2], was[was.len() - 1], was.iter().sum::<usize>());
        let h = History::of(pack, &world, &mut pol, seed);
        moved += pol.states.iter().zip(&before).filter(|(s, &b)| s.country != b).count();
        if std::env::var("DBGWARS").is_ok() {
            let mut now: Vec<usize> = pol.countries.iter().map(|c| c.states.len()).collect();
            now.sort();
            println!("  мир {seed}: области сейчас {:?}; до {:?}", now, was);
        }
        dead += pol.countries.iter().filter(|c| c.states.is_empty()).count();
        for p in &h.diplo.pacts {
            pacts[Kind::ALL.iter().position(|k| *k == p.kind).unwrap_or(0)] += 1;
        }
        open_wars += h.diplo.wars.len();
        in_war += h.diplo.wars.iter().map(|w| w.sides[0].len() + w.sides[1].len()).sum::<usize>();
        past_wars += h.wars.len();
        {
            let n = pol.countries.len();
            let ops: Vec<f32> = (0..n * n).filter(|i| i / n != i % n).map(|i| h.diplo.opinion[i]).collect();
            let near: Vec<f32> = (0..n).flat_map(|a| pol.neighbors(a).into_iter().map(move |b| (a, b))).map(|(a, b)| h.diplo.opinion[a * n + b]).collect();
            let share = |v: &[f32], f: &dyn Fn(f32) -> bool| v.iter().filter(|&&x| f(x)).count() as f32 / v.len().max(1) as f32;
            println!("  мир {seed}: мнения — все: выше +30 {:.0}%, ниже −30 {:.0}%, среднее {:+.0}; соседи: выше +30 {:.0}%, ниже −30 {:.0}%", 100.0 * share(&ops, &|x| x > 0.3), 100.0 * share(&ops, &|x| x < -0.3), 100.0 * ops.iter().sum::<f32>() / ops.len().max(1) as f32, 100.0 * share(&near, &|x| x > 0.3), 100.0 * share(&near, &|x| x < -0.3));
        }
        if std::env::var("DBGWARS").is_ok() {
            let front: usize = h.wars.iter().filter(|w| w.battles > 0).map(|w| w.took.len()).sum();
            let small: usize = h.wars.iter().filter(|w| w.battles == 0 && w.year <= 100).map(|w| w.took.len()).sum();
            let deep: usize = h.wars.iter().filter(|w| w.battles == 0 && w.year > 100).map(|w| w.took.len()).sum();
            let nf = h.wars.iter().filter(|w| w.battles > 0).count();
            let capit = h.wars.iter().filter(|w| w.battles > 0 && w.capitulated).count();
            println!("  мир {seed}: краёв ушло — фронтом {front} (войн {nf}, капитуляций {capit}), малыми за век {small}, в глубоком прошлом {deep}");
        }
        for c in &pol.countries {
            if !c.states.is_empty() {
                sizes.push(c.states.len());
            }
        }
        let live: Vec<usize> = (0..pol.countries.len()).filter(|&c| !pol.countries[c].states.is_empty()).collect();
        for &c in &live {
            let p = &h.polity.of[c];
            regimes[p.regime.index()] += 1;
            revolutions += p.revolutions;
            coups += p.coups;
            reforms += p.reforms;
            countries += 1;
            techs.push(h.know[c].count());
            lit.push(p.literacy);
            for m in 0..MOVES {
                moves[m] += p.moves[m];
            }
        }
        let held: Vec<usize> = (0..h.polity.faiths.len()).filter(|&f| h.polity.region_faith.iter().any(|&x| x == f)).collect();
        faiths.push((h.polity.faiths.len(), held.len()));
        if seed == first {
            println!("мир {seed}: летопись обществ — {} записей; примеры:", h.polity.chronicle.len());
            for (m, _, line) in h.polity.chronicle.iter().filter(|x| !x.2.contains("голодные")).rev().take(25).collect::<Vec<_>>().iter().rev() {
                println!("  {} лет назад: {line}", (-m).max(0) / 12);
            }
            println!("  договоры и войны между странами (последние):");
            for (m, _, line) in h.diplo.chronicle.iter().rev().take(20).collect::<Vec<_>>().iter().rev() {
                println!("    {} лет назад: {line}", (-m).max(0) / 12);
            }
            println!("  войны, что идут к началу игры:");
            for w in &h.diplo.wars {
                let side = |s: usize| w.sides[s].iter().map(|&c| pol.countries[c].name.as_str()).collect::<Vec<_>>().join(" + ");
                println!("    {} против {} — {} ({} мес., счёт {:+.2})", side(0), side(1), w.cause, (-w.since).max(0), w.score);
            }
            for &c in live.iter().take(6) {
                let ps: Vec<String> = h.diplo.pacts_of(c).iter().map(|p| format!("{} с {}", p.kind.name().to_lowercase(), pol.countries[p.other(c)].name)).collect();
                println!("  {}: {}", pol.countries[c].name, if ps.is_empty() { "договоров нет".to_string() } else { ps.join("; ") });
            }
            for &c in live.iter().take(6) {
                let p = &h.polity.of[c];
                let fa = h.polity.faiths.get(p.state_faith).map_or(String::new(), |f| f.name.clone());
                println!("  {} — {}, вера «{fa}», законность {:.2}, смута {:.2}, грамотность {:.2}, набожность {:.2}, техник {}", pol.countries[c].name, p.regime.name().to_lowercase(), p.legit, p.unrest, p.literacy, p.piety, h.know[c].count());
            }
        }
    }
    println!("\nмиров {seeds}, стран {countries} ({:.1} с):", t0.elapsed().as_secs_f32());
    let words: Vec<String> = Regime::ALL.iter().zip(&regimes).filter(|x| *x.1 > 0).map(|(r, k)| format!("{} {k}", r.name().to_lowercase())).collect();
    println!("  строй: {}", words.join(", "));
    println!("  за четыреста лет на страну: революций {:.2}, переворотов {:.2}, реформ {:.2}", revolutions as f32 / countries.max(1) as f32, coups as f32 / countries.max(1) as f32, reforms as f32 / countries.max(1) as f32);
    let fw: Vec<String> = faiths.iter().map(|(a, b)| format!("{b} из {a}")).collect();
    println!("  вер в мире (живых из основанных): {}", fw.join(", "));
    techs.sort();
    lit.sort_by(|a, b| a.total_cmp(b));
    if !techs.is_empty() {
        println!("  техник (из {}): меньше всех {}, четверть {}, середина {}, три четверти {}, больше всех {}", pack.techs.len(), techs[0], techs[techs.len() / 4], techs[techs.len() / 2], techs[techs.len() * 3 / 4], techs[techs.len() - 1]);
        println!("  грамотность: {:.2} / {:.2} / {:.2}", lit[0], lit[lit.len() / 2], lit[lit.len() - 1]);
    }
    let pw: Vec<String> = Kind::ALL.iter().zip(&pacts).map(|(k, n)| format!("{} {:.1}", k.name().to_lowercase(), *n as f32 / seeds as f32)).collect();
    println!("  договоров на мир: {}", pw.join(", "));
    println!("  войн за 400 лет на мир {:.0}; идут к началу игры {:.1} (стран в них {:.1}); краёв сменило хозяина {:.0}; стран исчезло {:.1}", past_wars as f32 / seeds as f32, open_wars as f32 / seeds as f32, in_war as f32 / seeds as f32, moved as f32 / seeds as f32, dead as f32 / seeds as f32);
    sizes.sort();
    if !sizes.is_empty() {
        println!("  областей у страны: меньше всех {}, середина {}, больше всех {}", sizes[0], sizes[sizes.len() / 2], sizes[sizes.len() - 1]);
    }
    let mv: Vec<String> = (0..MOVES).map(|m| format!("{} {:.2}", MOVE[m].name.to_lowercase(), moves[m] / countries.max(1) as f32)).collect();
    println!("  движения (средняя поддержка): {}", mv.join(", "));
}
