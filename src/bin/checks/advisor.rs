//! Economist advisors: the same countries of the map run by different advisors — in the trading
//! world (the game's), the other countries keeping to their custom; one advisor's record as the
//! government sees it; checks.
//! `--alone` — each country on its own instead; `--levers` — how much fixed levers change a
//! country's life at all, in each situation; `--seeds 1,2`, `--countries N`, `--months M`.

use rayon::prelude::*;
use crate::common::mean_se;
use strategy::economy::advisor::{welfare, Economist, Levers, Mind, Outside};
use strategy::economy::nation::{Nation, Setup};
use strategy::economy::trade::World;
use strategy::military::homefront::{setup_of, trading};
use strategy::pack::Pack;
use strategy::world::countries;
use strategy::world::map::{self, Params};

const ALONE: u32 = 240;
const TOGETHER: u32 = 240;

fn mind(education: f32, experience: f32, talent: f32, honesty: f32, state: f32) -> Mind {
    Mind::new(education, experience, talent, honesty, state)
}
fn strong() -> Mind {
    Mind::strong()
}
fn average() -> Mind {
    Mind::average()
}
fn weak() -> Mind {
    Mind::weak()
}
fn technocrat() -> Mind {
    Mind::technocrat()
}

/// What befalls a country as the advisor takes office.
#[derive(Clone, Copy, Debug)]
enum Start {
    AsIs,
    /// Ruin: three fifths of its workplaces and stores destroyed, owners' savings gone.
    Ruin,
    /// Overbuilt: half as many workplaces again as it had.
    Overbuilt,
    /// Drought: the land gives half for the first two years.
    Drought,
}

fn befall(pack: &Pack, n: &mut Nation, how: Start) {
    match how {
        Start::AsIs => {}
        Start::Ruin => {
            n.cap.iter_mut().for_each(|c| *c *= 0.4);
            n.stock.iter_mut().for_each(|x| *x *= 0.3);
            n.inv.iter_mut().flatten().for_each(|x| *x *= 0.3);
            n.cash.iter_mut().for_each(|x| *x *= 0.3);
            for g in 0..n.pop.len() {
                if pack.sectors.iter().any(|s| s.owner == g) {
                    n.money[g] *= 0.3;
                }
            }
        }
        Start::Overbuilt => {
            for s in 0..n.cap.len() {
                n.cap[s] = (n.cap[s] * 1.5).min(n.max_cap[s]);
            }
        }
        Start::Drought => {}
    }
}

/// What the situation does month by month (a drought lasts its years).
fn weather(pack: &Pack, n: &mut Nation, how: Start, month: u32) {
    if let Start::Drought = how {
        let k = if month < 24 { 0.5 } else { 1.0 };
        for (s, sec) in pack.sectors.iter().enumerate() {
            if sec.land {
                n.shock[s] = k;
            }
        }
    }
}

/// A country: on its own, or one of a trading world.
#[derive(Clone)]
enum Home {
    Alone(Nation),
    In(World, usize),
}

impl Home {
    fn nation(&self) -> &Nation {
        match self {
            Home::Alone(n) => n,
            Home::In(w, c) => &w.nations[*c],
        }
    }
    fn nation_mut(&mut self) -> &mut Nation {
        match self {
            Home::Alone(n) => n,
            Home::In(w, c) => &mut w.nations[*c],
        }
    }
    fn outside(&self, pack: &Pack) -> Outside {
        match self {
            Home::Alone(_) => Outside::alone(pack),
            Home::In(w, c) => Outside::of(&w.nations[*c], w.pending(*c)),
        }
    }
    fn step(&mut self, pack: &Pack) {
        match self {
            Home::Alone(n) => n.step(pack),
            Home::In(w, _) => w.step(pack),
        }
    }
}

/// Who runs the economy.
#[derive(Clone, Copy)]
enum Who {
    Advisor(Mind, u64),
    Fixed(Levers),
}

struct Out {
    /// Mean welfare over the term, per head of the people at its start.
    w: f32,
    unrest: usize,
    took: f32,
    /// The army's money over the term, per month, and the state's budget.
    army: f32,
    budget: f32,
    econ: Option<Economist>,
}

fn run(pack: &Pack, home: &Home, how: Start, who: Who, months: u32) -> Out {
    let mut h = home.clone();
    befall(pack, h.nation_mut(), how);
    let ask = (pack.military, pack.tax);
    let people0: f32 = h.nation().pop.iter().sum();
    let events0 = h.nation().events.len();
    let mut econ = match who {
        Who::Advisor(m, seed) => Some(Economist::new(m, seed, pack)),
        Who::Fixed(_) => None,
    };
    let (mut w, mut army, mut budget) = (0.0, 0.0, 0.0);
    for month in 0..months {
        weather(pack, h.nation_mut(), how, month);
        let out = h.outside(pack);
        match (&mut econ, who) {
            (Some(e), _) => e.month(pack, h.nation_mut(), ask, &out),
            (None, Who::Fixed(l)) => l.apply(h.nation_mut(), ask),
            _ => {}
        }
        let n = h.nation();
        army += n.state_budget() * n.mil_share;
        budget += n.state_budget();
        h.step(pack);
        if let Some(e) = econ.as_mut() {
            e.after(h.nation());
        }
        w += welfare(h.nation());
    }
    let n = h.nation();
    let m = months as f32;
    Out { w: w / m / people0, unrest: n.events.len() - events0, took: econ.as_ref().map_or(0.0, |e| e.took) / m, army: army / m, budget: budget / m, econ }
}

pub fn main() {
    let args: Vec<String> = crate::args().into_iter().collect();
    let arg = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let flag = |k: &str| args.iter().any(|a| a == k);
    let pack = Pack::load(&arg("--pack").unwrap_or("packs/industrial.toml".into())).unwrap_or_else(|e| panic!("{e}"));
    let seeds: Vec<u64> = arg("--seeds").unwrap_or("1".into()).split(',').filter_map(|s| s.parse().ok()).collect();
    let months: u32 = arg("--months").and_then(|s| s.parse().ok()).unwrap_or(60);
    let per_map: usize = arg("--countries").and_then(|s| s.parse().ok()).unwrap_or(30);
    let alone = flag("--alone");
    let t0 = std::time::Instant::now();
    let mut homes: Vec<Home> = Vec::new();
    for &seed in &seeds {
        let world = map::generate(&Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 }).0;
        let pol = countries::generate(&world, seed, 30, &pack);
        let setups: Vec<Setup> = (0..pol.countries.len()).map(|c| setup_of(&pol, c)).collect();
        // the biggest countries first
        let mut order: Vec<usize> = (0..setups.len()).collect();
        order.sort_by(|&a, &b| setups[b].pop.total_cmp(&setups[a].pop));
        order.truncate(per_map);
        if alone {
            let settled: Vec<Nation> = order
                .par_iter()
                .map(|&c| {
                    let mut n = Nation::new(&pack, &setups[c]);
                    n.settle(&pack, ALONE);
                    n
                })
                .collect();
            homes.extend(settled.into_iter().map(Home::Alone));
        } else {
            let w = trading(&pack, &world, &pol, &setups, ALONE, TOGETHER);
            homes.extend(order.iter().map(|&c| Home::In(w.clone(), c)));
        }
    }
    println!(
        "{} — {} стран (карты {:?}, {}), подготовка {:.1} с",
        pack.name,
        homes.len(),
        seeds,
        if alone { "каждая сама по себе" } else { "в торгующем мире" },
        t0.elapsed().as_secs_f32()
    );
    if flag("--decomp") {
        // the weak one's harm, piece by piece: people's welfare and the army's money against the average
        let kinds = [
            ("средний", average()),
            ("слабый", weak()),
            ("слабый, но честный", mind(0.15, 0.2, 0.3, 1.0, 0.0)),
            ("средний, но вор", mind(0.5, 0.5, 0.5, 0.3, 0.0)),
        ];
        let jobs: Vec<(usize, usize)> = (0..homes.len()).flat_map(|c| (0..kinds.len()).map(move |k| (c, k))).collect();
        let outs: Vec<Out> =
            jobs.par_iter().map(|&(c, k)| run(&pack, &homes[c], Start::AsIs, Who::Advisor(kinds[k].1, 1 ^ (c as u64) << 8), months)).collect();
        let nk = kinds.len();
        for k in 0..nk {
            let w: Vec<f32> = (0..homes.len()).map(|c| (outs[c * nk + k].w / outs[c * nk].w - 1.0) * 100.0).collect();
            let a: Vec<f32> = (0..homes.len()).map(|c| (outs[c * nk + k].army / outs[c * nk].army - 1.0) * 100.0).collect();
            let took: f32 = (0..homes.len()).map(|c| outs[c * nk + k].took / outs[c * nk + k].budget.max(1e-9)).sum::<f32>() / homes.len() as f32;
            let (wm, ws) = mean_se(&w);
            let (am, as_) = mean_se(&a);
            println!("  {:<22} люди {:+.2}% ± {:.2}  армия {:+.1}% ± {:.1}  украдено {:.1}% бюджета", kinds[k].0, wm, ws, am, as_, took * 100.0);
        }
        return;
    }
    if flag("--techno") {
        // the technocrat against the average and against a pragmatist as able as he, by situation
        // the first is the one compared: a technocrat, or with `--liberal` a believer in owners as able
        let first = if flag("--liberal") { ("сторонник хозяев", mind(0.9, 0.7, 0.9, 1.0, -1.0)) } else { ("технократ", technocrat()) };
        let kinds = [first, ("средний", average()), ("сильный", strong())];
        let hows = [Start::AsIs, Start::Ruin, Start::Overbuilt, Start::Drought];
        let jobs: Vec<(usize, usize, usize)> =
            (0..homes.len()).flat_map(|c| (0..hows.len()).flat_map(move |h| (0..kinds.len()).map(move |k| (c, h, k)))).collect();
        let outs: Vec<Out> = jobs
            .par_iter()
            .map(|&(c, h, k)| run(&pack, &homes[c], hows[h], Who::Advisor(kinds[k].1, 1 ^ (c as u64) << 8), months))
            .collect();
        let at = |c: usize, h: usize, k: usize| &outs[(c * hows.len() + h) * kinds.len() + k];
        for (h, how) in hows.iter().enumerate() {
            for k in 1..kinds.len() {
                let w: Vec<f32> = (0..homes.len()).map(|c| (at(c, h, 0).w / at(c, h, k).w - 1.0) * 100.0).collect();
                let a: Vec<f32> = (0..homes.len()).map(|c| (at(c, h, 0).army / at(c, h, k).army.max(1e-9) - 1.0) * 100.0).collect();
                let (wm, ws) = mean_se(&w);
                let (am, as_) = mean_se(&a);
                println!(
                    "  {:<10} {} против «{}»: люди {:+.2}% ± {:.2} ({} из {}), армия {:+.1}% ± {:.1}",
                    format!("{how:?}"),
                    kinds[0].0,
                    kinds[k].0,
                    wm,
                    ws,
                    w.iter().filter(|&&d| d > 0.0).count(),
                    w.len(),
                    am,
                    as_
                );
            }
        }
        return;
    }
    if flag("--levers") {
        levers(&pack, &homes, months);
        return;
    }

    // every country with every advisor
    let t1 = std::time::Instant::now();
    let cases: Vec<(Start, Who)> = vec![
        (Start::AsIs, Who::Advisor(strong(), 1)),
        (Start::AsIs, Who::Advisor(strong(), 2)),
        (Start::AsIs, Who::Advisor(average(), 1)),
        (Start::AsIs, Who::Advisor(weak(), 1)),
        (Start::AsIs, Who::Fixed(Levers::custom())),
        (Start::Ruin, Who::Advisor(technocrat(), 1)),
        (Start::Ruin, Who::Advisor(average(), 1)),
        (Start::Drought, Who::Advisor(technocrat(), 1)),
        (Start::Drought, Who::Advisor(average(), 1)),
        (Start::AsIs, Who::Advisor(technocrat(), 1)),
    ];
    // CASES=0,2: only these cases (the checks that need others are left out)
    let only: Option<Vec<usize>> = std::env::var("CASES").ok().map(|v| v.split(',').filter_map(|x| x.parse().ok()).collect());
    let wanted = |k: usize| only.as_ref().is_none_or(|o| o.contains(&k));
    let jobs: Vec<(usize, usize)> = (0..homes.len()).flat_map(|c| (0..cases.len()).map(move |k| (c, k))).filter(|&(_, k)| wanted(k)).collect();
    let outs: Vec<Out> = jobs
        .par_iter()
        .map(|&(c, k)| {
            let who = match cases[k].1 {
                Who::Advisor(m, s) => Who::Advisor(m, s ^ (c as u64) << 8),
                w => w,
            };
            run(&pack, &homes[c], cases[k].0, who, months)
        })
        .collect();
    println!("{} стран × {} случаев × {} мес: {:.0} с", homes.len(), jobs.len() / homes.len().max(1), months, t1.elapsed().as_secs_f32());
    let mut res: Vec<Vec<Option<Out>>> = (0..homes.len()).map(|_| (0..cases.len()).map(|_| None).collect()).collect();
    for (&(c, k), o) in jobs.iter().zip(outs) {
        res[c][k] = Some(o);
    }
    if only.is_some() {
        // country by country
        for (c, r) in res.iter().enumerate() {
            let line: Vec<String> = r
                .iter()
                .enumerate()
                .filter_map(|(k, o)| o.as_ref().map(|o| {
                    let last = o.econ.as_ref().map_or(String::new(), |e| e.record.last().map_or(String::new(), |d| format!(" [{}]", d.what)));
                    format!("{k}: {:.4}{last}", o.w)
                }))
                .collect();
            println!("  {:<14} {}", homes[c].nation().name, line.join(" | "));
        }
        for (a, b) in [(0usize, 2usize), (2, 3), (0, 1), (5, 6), (7, 8)] {
            if wanted(a) && wanted(b) {
                let x: Vec<f32> = res.iter().map(|r| (r[a].as_ref().unwrap().w / r[b].as_ref().unwrap().w - 1.0) * 100.0).collect();
                let (m, se) = mean_se(&x);
                println!("  случай {a} против {b}: {m:+.2}% ± {se:.2} ({} из {})", x.iter().filter(|&&d| d > 0.0).count(), x.len());
            }
        }
        return;
    }
    let res: Vec<Vec<Out>> = res.into_iter().map(|r| r.into_iter().map(|o| o.unwrap()).collect()).collect();
    let diff = |a: usize, b: usize| -> Vec<f32> { res.iter().map(|r| (r[a].w / r[b].w - 1.0) * 100.0).collect() };
    let army = |a: usize, b: usize| -> Vec<f32> { res.iter().map(|r| (r[a].army / r[b].army.max(1e-9) - 1.0) * 100.0).collect() };
    let wins = |a: usize, b: usize| res.iter().filter(|r| r[a].w > r[b].w).count();

    // the record of one country's advisors, as the government sees it
    let show = 0;
    println!("\nпослужные списки в стране {} (что видит правительство):", homes[show].nation().name);
    for (k, label) in [(0, "советник А"), (3, "советник Б")] {
        let e = res[show][k].econ.as_ref().unwrap();
        println!("  {label}:");
        for d in e.record.iter().take(8) {
            println!("    год {} мес {:>2}: {}", d.month / 12 + 1, d.month % 12 + 1, d.what);
        }
        if e.record.len() > 8 {
            println!("    … ещё решений: {}", e.record.len() - 8);
        }
        let ys: Vec<String> = e
            .years
            .iter()
            .skip(1)
            .map(|y| format!("{:+.1}%/{:+.1}%{}", y.welfare * 100.0, y.output * 100.0, if y.unrest > 0 { format!(" ({} волн.)", y.unrest) } else { String::new() }))
            .collect();
        println!("    по годам (жизнь людей / выпуск): {}", ys.join(", "));
    }
    println!("  (А — сильный, Б — слабый; качества скрыты, игрок видит только это)");

    println!("\nза {} лет, разница по странам (средняя ± ошибка): как живут люди (в скольких странах лучше) | деньги армии:", months / 12);
    let line = |name: &str, a: usize, b: usize| {
        let (m, se) = mean_se(&diff(a, b));
        let (am, ase) = mean_se(&army(a, b));
        println!("  {:<46} люди {:+.2}% ± {:.2} ({} из {}) | армия {:+.1}% ± {:.1}", name, m, se, wins(a, b), res.len(), am, ase);
        (m, se, am, ase)
    };
    let sa = line("сильный против среднего", 0, 2);
    let aw = line("средний против слабого", 2, 3);
    let ctl = line("контроль: сильный против такого же сильного", 0, 1);
    line("средний против обычая (без советника)", 2, 4);
    let tp = line("технократ против прагматика того же ума", 9, 0);
    let ts = line("технократ против среднего: разруха", 5, 6);
    let td = line("технократ против среднего: засуха", 7, 8);
    // his worth in ruin against his worth in a drought, country by country
    let dd: Vec<f32> = res.iter().map(|r| ((r[5].w / r[6].w) - (r[7].w / r[8].w)) * 100.0).collect();
    let (ddm, dds) = mean_se(&dd);
    println!("  {:<46} {:+.2} ± {:.2} пункта", "польза технократа: разруха минус засуха", ddm, dds);
    let stolen: f32 = res.iter().map(|r| r[3].took).sum::<f32>() / res.iter().map(|r| r[2].took.max(1e-9)).sum::<f32>();
    let unrest: Vec<f32> = [0, 2, 3].iter().map(|&k| res.iter().map(|r| r[k].unrest as f32).sum::<f32>() / res.len() as f32).collect();
    println!(
        "  волнений за срок в среднем: сильный {:.1}, средний {:.1}, слабый {:.1}; слабый украл в {:.0} раз больше среднего",
        unrest[0], unrest[1], unrest[2], stolen
    );
    let again = run(&pack, &homes[show], Start::AsIs, Who::Advisor(strong(), 1 ^ (show as u64) << 8), months);
    let same = again.w == res[show][0].w;

    // better: one of the two (people, army) clearly better, the other not clearly worse
    let better = |x: (f32, f32, f32, f32)| (x.0 > 3.0 * x.1 && x.2 > -3.0 * x.3) || (x.2 > 3.0 * x.3 && x.0 > -3.0 * x.1);
    let checks: Vec<(&str, bool, String)> = vec![
        ("у сильного страна живёт лучше, чем у среднего", better(sa), format!("люди {:+.2}% ± {:.2}, армия {:+.1}% ± {:.1}", sa.0, sa.1, sa.2, sa.3)),
        ("у среднего лучше, чем у слабого", better(aw), format!("люди {:+.2}% ± {:.2}, армия {:+.1}% ± {:.1}", aw.0, aw.1, aw.2, aw.3)),
        (
            "контроль: равные качества — один итог",
            ctl.0.abs() < 3.0 * ctl.1.max(0.01) && ctl.2.abs() < 3.0 * ctl.3.max(0.1) && same,
            format!(
                "люди {:+.2}% ± {:.2}, армия {:+.1}% ± {:.1}; тот же советник заново — {}",
                ctl.0,
                ctl.1,
                ctl.2,
                ctl.3,
                if same { "то же до бита" } else { "ДРУГОЕ" }
            ),
        ),
        // he believes the state should plan and build, and his belief weighs against his own
        // reckoning: as in life, a technocrat need not be the best there is
        (
            "технократ не идеален: прагматик того же ума лучше",
            tp.0 < -3.0 * tp.1,
            format!("людям {:+.2}% ± {:.2}, армии {:+.1}% ± {:.1}", tp.0, tp.1, tp.2, tp.3),
        ),
    ];
    // not checks (at thirty countries below three standard errors in some eras): where he helps
    println!(
        "  сведения: технократ против среднего — в разрухе {:+.2}% ± {:.2}, в засуху {:+.2}% ± {:.2}; разница {:+.2} ± {:.2}",
        ts.0, ts.1, td.0, td.1, ddm, dds
    );
    println!("\nпроверки:");
    let mut ok_all = true;
    for (name, ok, detail) in &checks {
        ok_all &= ok;
        println!("  [{}] {:<44} {}", if *ok { "OK" } else { "FAIL" }, name, detail);
    }
    println!("{}: {}/{}", pack.name, checks.iter().filter(|c| c.1).count(), checks.len());
    if !ok_all {
        std::process::exit(1);
    }
}

/// How much fixed levers change a country's life, in each situation, against the custom.
fn levers(pack: &Pack, homes: &[Home], months: u32) {
    let fav = (0..pack.sectors.len()).filter(|&s| !pack.sectors[s].build.is_empty()).max_by(|&a, &b| {
        let w = |s: usize| {
            let sec = &pack.sectors[s];
            sec.outputs.iter().map(|&(j, x)| x * pack.goods[j].price).sum::<f32>()
                - sec.inputs.iter().map(|&(i, x)| x * pack.goods[i].price).sum::<f32>()
        };
        w(a).total_cmp(&w(b))
    });
    let variants = [
        ("обычай", Levers::custom()),
        ("налог×0.5", Levers { tax: 0.5, ..Levers::custom() }),
        ("налог×2", Levers { tax: 2.0, ..Levers::custom() }),
        ("без стройки", Levers { build: 0.0, ..Levers::custom() }),
        ("налог×2 без стройки", Levers { tax: 2.0, build: 0.0, ..Levers::custom() }),
        ("план×3", Levers { plan: 2.0, ..Levers::custom() }),
        ("план×3+любимая", Levers { plan: 2.0, focus: fav, ..Levers::custom() }),
        ("налог×2+план×3+любимая", Levers { tax: 2.0, plan: 2.0, focus: fav, ..Levers::custom() }),
    ];
    for how in [Start::AsIs, Start::Ruin, Start::Overbuilt] {
        let t = std::time::Instant::now();
        let jobs: Vec<(usize, usize)> = (0..homes.len()).flat_map(|c| (0..variants.len()).map(move |v| (c, v))).collect();
        let outs: Vec<f32> = jobs.par_iter().map(|&(c, v)| run(pack, &homes[c], how, Who::Fixed(variants[v].1), months).w).collect();
        println!("{how:?} ({:.0} с):", t.elapsed().as_secs_f32());
        let nv = variants.len();
        for v in 0..nv {
            let x: Vec<f32> = (0..homes.len()).map(|c| (outs[c * nv + v] / outs[c * nv] - 1.0) * 100.0).collect();
            let (m, se) = mean_se(&x);
            let best = (0..homes.len()).filter(|&c| (0..nv).all(|u| outs[c * nv + v] >= outs[c * nv + u])).count();
            println!("  {:<26} {:+.1}% ± {:.1}   лучший в {} странах", variants[v].0, m, se, best);
        }
    }
}
