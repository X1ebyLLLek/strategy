//! Trade between countries: toys first (two countries, one way between them), then the
//! countries of a map under each era. Self-checks at the end; exit code 1 on a FAIL.

use rayon::prelude::*;
use crate::common::swing;
use strategy::economy::nation::{Nation, Setup};
use strategy::economy::trade::{self as trade, Route, World};
use strategy::pack::Pack;
use strategy::world::countries;
use strategy::world::map::{self, Params};

mod maps;
use maps::{consumed, map_checks};

/// Pre-history: alone, then trading.
const ALONE: u32 = 240;
const TOGETHER: u32 = 240;
const MONTHS: u32 = 120;

type Checks = Vec<(String, bool, String)>;

fn setup(pack: &Pack, name: &str, fertility: f32, dev: f32) -> Setup {
    Setup { name: name.into(), pop: 100.0, dev, fertility, land: 100.0, deposits: vec![50.0; pack.resources.len()] }
}

/// Two countries and one way between them, both directions.
fn pair(pack: &Pack, a: &Setup, b: &Setup, route: Route) -> World {
    World::new(pack, vec![Nation::new(pack, a), Nation::new(pack, b)], vec![vec![None, Some(route)], vec![Some(route), None]])
}

/// Share of people's needs unmet, weighted by how much each hurts, over the whole country.
fn pain(x: &Nation) -> f32 {
    x.hardship.iter().zip(&x.pop).map(|(h, p)| h * p).sum::<f32>() / x.pop.iter().sum::<f32>().max(1e-6)
}

/// The same country with no one to trade with, lived as long.
fn alone(pack: &Pack, s: &Setup) -> Nation {
    alone_logged(pack, s).0
}

/// The same, with its prices month by month over the game.
fn alone_logged(pack: &Pack, s: &Setup) -> (Nation, Vec<Vec<f32>>) {
    // the same pre-history as in a trading world, with the money counted anew at what it buys
    // where the ways would open (nothing real changes, but the digits do)
    let mut x = Nation::new(pack, s);
    x.live = false;
    for _ in 0..ALONE {
        x.step(pack);
    }
    x.rebase(1.0 / x.level.max(1e-6));
    x.settle(pack, TOGETHER);
    let mut log = vec![Vec::new(); pack.goods.len()];
    for _ in 0..MONTHS {
        x.step(pack);
        for (k, l) in log.iter_mut().enumerate() {
            record(l, &x, k);
        }
    }
    (x, log)
}

/// Years of a country's game in which some price went back and forth by more than 10%.
fn restless_years(log: &[Vec<f32>]) -> usize {
    (0..MONTHS as usize / 12).filter(|&y| log.iter().map(|l| swing(&l[y * 12..y * 12 + 12])).fold(0.0f32, f32::max) > 0.1).count()
}

/// Adds this month's price to a log of log-prices; a month the good is not on the market
/// at all repeats the last real price (an absent good's price is only the usual one, and its
/// coming and going is not the price moving).
fn record(log: &mut Vec<f32>, x: &Nation, k: usize) {
    let p = if x.offered[k] > 1e-3 { x.price[k].ln() } else { log.last().copied().unwrap_or(x.price[k].ln()) };
    log.push(p);
}

/// Runs the game; returns the years in which some price in some country went back and forth
/// by more than 10% (a single event is allowed, a swing every year is not), and the worst
/// such swing.
fn run(world: &mut World, pack: &Pack, months: u32, trace: bool) -> (usize, f32) {
    let mut log: Vec<Vec<Vec<f32>>> = vec![vec![Vec::new(); pack.goods.len()]; world.nations.len()];
    for m in 1..=months {
        world.step(pack);
        for (c, x) in world.nations.iter().enumerate() {
            for k in 0..pack.goods.len() {
                record(&mut log[c][k], x, k);
            }
        }
        if let Some(k) = std::env::var("EVERY").ok().and_then(|id| pack.goods.iter().position(|g| g.id == id)) {
            if trace {
                let x = &world.nations[1];
                println!(
                    "  B мес {m:>3}: {} ×{:.2} (сглаж ×{:.2}) в A ×{:.2} | везут {:.3} на рынке {:.3} продано {:.3} ждёт {:.3} | купили люди {:.3} отрасли {:.3} стройка {:.3}",
                    pack.goods[k].name, x.price[k] / pack.goods[k].price, x.trend[k] / pack.goods[k].price, world.nations[0].price[k] / pack.goods[k].price,
                    world.flow[0][1][k], x.import[k], x.import_sold[k], x.import[k] - x.import_sold[k], x.bought_by[k][0], x.bought_by[k][2], x.bought_by[k][1] + x.bought_by[k][4]
                );
            }
        }
        if trace && (m <= 3 || m % 24 == 0) {
            let line: Vec<String> = (0..pack.goods.len())
                .map(|k| {
                    format!(
                        "{}: A→B {:.2} B→A {:.2}, цена A {:.2} B {:.2}",
                        pack.goods[k].name,
                        world.flow[0][1][k],
                        world.flow[1][0][k],
                        world.nations[0].price[k],
                        world.nations[1].price[k]
                    )
                })
                .collect();
            println!(
                "  мес {m:>3}: {} | тяжесть A {:.0}% B {:.0}%",
                line.join(" | "),
                pain(&world.nations[0]) * 100.0,
                pain(&world.nations[1]) * 100.0
            );
        }
    }
    let mut restless = 0;
    let mut worst = 0.0f32;
    for y in 0..months as usize / 12 {
        let w = log.iter().flatten().map(|l| swing(&l[y * 12..y * 12 + 12])).fold(0.0f32, f32::max);
        worst = worst.max(w);
        if w > 0.1 {
            restless += 1;
        }
    }
    (restless, worst)
}

/// Every route and good with prices at both ends, from a country that makes it: no big gain
/// may be left on the table (more than 25% after the way, while the exporter could still
/// send more), and no trade may go on at a big loss (more than 20%). Prices keep moving as
/// countries grow, so a gap of a few percent either way is just the world changing.
fn arbitrage(world: &World, pack: &Pack) -> (usize, usize, f32, usize) {
    let (mut active, mut off, mut worst, mut priced) = (0, 0, 1.0f32, 0);
    for i in 0..world.nations.len() {
        let a = &world.nations[i];
        for j in 0..world.nations.len() {
            for k in 0..pack.goods.len() {
                let Some(g) = world.gap(pack, i, j, k) else { continue };
                if !pack.goods[k].tradable || a.produced[k] <= 1e-3 {
                    continue;
                }
                priced += 1;
                let home = a.offered[k] - a.import[k] + a.shipped[k];
                let at_limit = a.shipped[k] >= 0.8 * 0.9 * home;
                let flows = world.flow[i][j][k] > 0.0;
                if flows {
                    active += 1;
                }
                if (g > 1.25 && !at_limit) || (flows && g < 0.8) {
                    off += 1;
                    if std::env::var("OFF").is_ok() {
                        let b = &world.nations[j];
                        println!(
                            "    неравновесие {}→{} {}: везут {:.3}, разрыв {:.3}, цены {:.3}/{:.3}, на рынке {:.3}/{:.3}",
                            a.name, b.name, pack.goods[k].name, world.flow[i][j][k], g, a.price[k], b.price[k], a.offered[k], b.offered[k]
                        );
                    }
                    if (g - 1.0).abs() > (worst - 1.0).abs() {
                        worst = g;
                    }
                }
            }
        }
    }
    (active, off, worst, priced)
}

fn toy(checks: &mut Checks, path: &str, a: Setup, b: Setup, about: &str, needs_trade: bool) {
    let pack = Pack::load(path).unwrap_or_else(|e| panic!("{e}"));
    let way = Route { loss: 0.1, months: 1 };
    let tag = pack.name.to_lowercase();
    println!("══════ {}: {} ══════", pack.name, about);
    let solo = [alone(&pack, &a), alone(&pack, &b)];
    let mut w = pair(&pack, &a, &b, way);
    w.settle(&pack, ALONE, TOGETHER);
    let start = w.money_total();
    let (restless, swung) = run(&mut w, &pack, MONTHS, true);
    let drift = ((w.money_total() - start) / start).abs();
    checks.push((format!("{tag}: деньги в мире сохраняются"), drift < 1e-4, format!("за 10 лет {:+.5}%", drift * 100.0)));
    let (active, off, worst, _) = arbitrage(&w, &pack);
    checks.push((
        format!("{tag}: выгоды от перевозки не осталось"),
        off == 0,
        format!("путей с торговлей {active}, с неравновесием {off}; худший разрыв {worst:.3} (1 = цена пути)"),
    ));
    let line: Vec<String> = (0..2)
        .map(|c| format!("{} {:.0}% → {:.0}%", w.nations[c].name, pain(&solo[c]) * 100.0, pain(&w.nations[c]) * 100.0))
        .collect();
    if std::env::var("WHO").is_ok() {
        for c in 0..2 {
            for (label, x) in [("одна", &solo[c]), ("с торговлей", &w.nations[c])] {
                let gs: Vec<String> = (0..pack.groups.len())
                    .map(|g| format!("{} {:.1}чел {:.0}% ({}){}", pack.groups[g].name, x.pop[g], x.hardship[g] * 100.0, x.cause(&pack, g), x.unemployed[g].map_or(String::new(), |u| format!(" без работы {:.0}%", u * 100.0))))
                    .collect();
                println!("  {} {label}: {}", x.name, gs.join("; "));
                println!("      цены {:?}", x.price.iter().zip(&pack.goods).map(|(p, g)| format!("{} ×{:.2}", g.name, p / g.price)).collect::<Vec<_>>());
            }
        }
    }
    let gain: Vec<f32> = (0..2).map(|c| pain(&solo[c]) - pain(&w.nations[c])).collect();
    if needs_trade {
        // trade has losers too: cheap bread from B hurts A's peasants, whose harvest fetches
        // less (the corn laws); and B's peasants, paid in kind, have no money for imported
        // tools however near they are. What must hold: the good B lacks comes and is used, B
        // is not worse off, and the world has no fewer people and consumes no less
        let lacking: Vec<usize> = (0..pack.goods.len()).filter(|&k| solo[1].produced[k] <= 1e-3 && pack.goods[k].tradable && !pack.goods[k].military).collect();
        let came: f32 = lacking.iter().map(|&k| w.nations[1].import_sold[k]).sum();
        let people = |v: &[&Nation]| v.iter().map(|x| x.pop.iter().sum::<f32>()).sum::<f32>();
        let eaten = |v: &[&Nation]| v.iter().map(|x| consumed(x, &pack) * x.pop.iter().sum::<f32>()).sum::<f32>();
        let (alone_w, trade_w) = ([&solo[0], &solo[1]], [&w.nations[0], &w.nations[1]]);
        checks.push((
            format!("{tag}: чего у B нет — привезли, B не хуже"),
            came > 1e-2 && gain[1] > -0.02 && people(&trade_w) >= 0.99 * people(&alone_w) && eaten(&trade_w) >= 0.99 * eaten(&alone_w),
            format!(
                "ввезено недостающего {came:.2} в мес; тяжесть одна → с торговлей: {}; люди {:.0} → {:.0}, потребление {:.0} → {:.0}",
                line.join(", "),
                people(&alone_w),
                people(&trade_w),
                eaten(&alone_w),
                eaten(&trade_w)
            ),
        ));
    } else {
        checks.push((
            format!("{tag}: торговля никому не повредила"),
            gain.iter().all(|&g| g > -0.02),
            format!("тяжесть одна → с торговлей: {}", line.join(", ")),
        ));
    }
    checks.push((
        format!("{tag}: цены не качаются"),
        restless <= 2,
        format!("беспокойных лет {restless} из {} (худший год: туда-обратно {:.0}%)", MONTHS / 12, swung * 100.0),
    ));

    if needs_trade {
        // control: twins have nothing to trade
        let mut twins = pair(&pack, &a, &a, way);
        twins.settle(&pack, ALONE, TOGETHER);
        run(&mut twins, &pack, MONTHS, false);
        let f: f32 = (0..pack.goods.len()).map(|k| twins.flow[0][1][k] + twins.flow[1][0][k]).sum();
        checks.push(("контроль: близнецам нечем торговать".into(), f < 1e-3, format!("везут {f:.4} в месяц")));

        // control: a way that eats everything is the same as no trade at all
        let mut walled = pair(&pack, &a, &b, Route { loss: 20.0, months: 1 });
        walled.settle(&pack, ALONE, TOGETHER);
        run(&mut walled, &pack, MONTHS, false);
        let same = walled.nations.iter().zip(&solo).all(|(x, y)| x.price == y.price && x.pop == y.pop);
        checks.push(("контроль: непроходимый путь = без торговли".into(), same, format!("тяжесть B {:.4} и {:.4}", pain(&walled.nations[1]), pain(&solo[1]))));
    }
}

pub fn main() {
    let mut checks = Vec::new();
    let args: Vec<String> = crate::args().into_iter().collect();
    if args.get(1).map(|a| a.as_str()) == Some("--map") {
        let seed = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
        map_checks(&mut checks, &args[2], seed);
        for (name, pass, detail) in &checks {
            println!("  [{}] {:<44} {}", if *pass { "OK" } else { "FAIL" }, name, detail);
        }
        return;
    }
    if crate::args().into_iter().nth(1).as_deref() == Some("--open1") {
        // how trade opens in toy 1: a fertile and a barren country, one good
        let pack = Pack::load("packs/toy/t1_one.toml").unwrap();
        let (a, b) = (setup(&pack, "A", 1.4, 1.0), setup(&pack, "B", 0.6, 1.0));
        let mut w = pair(&pack, &a, &b, Route { loss: 0.1, months: 1 });
        w.settle(&pack, ALONE, 0);
        for m in 0..=240 {
            if m > 0 {
                w.step(&pack);
            }
            if m % 12 == 0 || m < 6 {
                let x = &w.nations;
                println!(
                    "{m:>3} A→B {:>6.2} B→A {:>5.2} | цена A {:.2} B {:.2} | деньги A {:>4.0} B {:>4.0} | люди A {:>5.1} B {:>5.1} | поля B {:.1}/{:.1} | голод B {:.0}% | B купили {:.1}, сделали {:.1}, ввезли {:.1}",
                    w.flow[0][1][0], w.flow[1][0][0], x[0].price[0], x[1].price[0], x[0].money_total(), x[1].money_total(),
                    x[0].pop.iter().sum::<f32>(), x[1].pop.iter().sum::<f32>(), x[1].worked[0], x[1].cap[0], x[1].unmet[0][0] * 100.0,
                    x[1].bought_by[0][0], x[1].produced[0], x[1].import_sold[0]
                );
            }
        }
        return;
    }
    if crate::args().into_iter().nth(1).as_deref() == Some("--open") {
        // how trade opens: two countries of toy 2 after living alone
        let pack = Pack::load("packs/toy/t2_two.toml").unwrap();
        let (a, b) = (setup(&pack, "A", 1.6, 0.6), setup(&pack, "B", 0.6, 1.6));
        let mut w = pair(&pack, &a, &b, Route { loss: 0.1, months: 1 });
        w.settle(&pack, ALONE, 0);
        for c in 0..2 {
            let x = &w.nations[c];
            println!("{}: деньги {:.0}, цены {:?}, ВВП {:.1}, тяжесть {:.0}%", x.name, x.money_total(), x.price, x.gdp, pain(x) * 100.0);
        }
        let months: u32 = crate::args().into_iter().nth(2).and_then(|m| m.parse().ok()).unwrap_or(120);
        for m in 1..=months {
            w.step(&pack);
            if m <= 24 || m % 12 == 0 {
                let x = &w.nations;
                println!(
                    "{m:>3} еда A→B {:>5.2} B→A {:>5.2} ц {:.2}/{:.2} | одежда A→B {:>5.2} B→A {:>5.2} ц {:.2}/{:.2} | деньги {:.0}/{:.0} | тяж {:.0}%/{:.0}% | поля {:.1}/{:.1} ткачи {:.1}/{:.1} | люди {:.1}/{:.1} голод {:.0}%/{:.0}% {}",
                    w.flow[0][1][0], w.flow[1][0][0], x[0].price[0], x[1].price[0],
                    w.flow[0][1][1], w.flow[1][0][1], x[0].price[1], x[1].price[1],
                    x[0].money_total(), x[1].money_total(), pain(&x[0]) * 100.0, pain(&x[1]) * 100.0,
                    x[0].cap[0], x[1].cap[0], x[0].cap[1], x[1].cap[1],
                    x[0].pop.iter().sum::<f32>(), x[1].pop.iter().sum::<f32>(),
                    x[0].unmet[0][0] * 100.0, x[1].unmet[0][0] * 100.0,
                    x[0].events.last().map_or("", |e| e.as_str())
                );
            }
        }
        let solo = alone(&pack, &a);
        for (label, x) in [("A с торговлей", &w.nations[0]), ("A одна", &solo), ("B с торговлей", &w.nations[1])] {
            println!("{label}: {}", x.debug_line(&pack));
            for g in 0..pack.groups.len() {
                println!(
                    "   {} {:.1} чел, деньги {:.0}, тяжесть {:.0}%, плата {:.2}, {}",
                    pack.groups[g].name, x.pop[g], x.money[g], x.hardship[g] * 100.0, x.pay[g], x.cause(&pack, g)
                );
            }
            println!("   казна {:.0}, поля {:.1}/{:.1} ткачи {:.1}/{:.1}, цены {:?}, сделано {:?}", x.treasury, x.worked[0], x.cap[0], x.worked[1], x.cap[1], x.price, x.produced);
        }
        return;
    }
    let t1 = Pack::load("packs/toy/t1_one.toml").unwrap();
    toy(
        &mut checks,
        "packs/toy/t1_one.toml",
        setup(&t1, "A", 1.4, 1.0),
        setup(&t1, "B", 0.6, 1.0),
        "один товар, A плодородная, B бедная — платить за ввоз B нечем, торговля должна затихнуть сама",
        false,
    );
    let t2 = Pack::load("packs/toy/t2_two.toml").unwrap();
    toy(
        &mut checks,
        "packs/toy/t2_two.toml",
        setup(&t2, "A", 1.6, 0.6),
        setup(&t2, "B", 0.6, 1.6),
        "два товара, A — поля, B — ткачи",
        false,
    );
    let t3 = Pack::load("packs/toy/t3_chain.toml").unwrap();
    let mut barren = setup(&t3, "B", 1.0, 1.0);
    barren.deposits = vec![0.0; t3.resources.len()];
    toy(
        &mut checks,
        "packs/toy/t3_chain.toml",
        setup(&t3, "A", 1.0, 1.0),
        barren,
        "руда → инструменты; у A железо есть, у B его нет совсем — торговля нужна B",
        true,
    );
    println!("проверки:");
    let mut ok = true;
    for (name, pass, detail) in &checks {
        ok &= pass;
        println!("  [{}] {:<44} {}", if *pass { "OK" } else { "FAIL" }, name, detail);
    }
    if !ok {
        std::process::exit(1);
    }
}
