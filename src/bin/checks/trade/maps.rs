//! The thirty countries of a map with trade against the same countries alone (`checks trade --map
//! packs/x.toml [seed]`): calm, missing goods, people and consumption, near trade, what changed.

use super::*;

/// What people consumed last month, at starting prices, per head: bought plus taken in kind.
pub(super) fn consumed(x: &Nation, pack: &Pack) -> f32 {
    let all: f32 = (0..pack.groups.len())
        .map(|g| (0..pack.goods.len()).map(|j| (x.spent[g][j] / x.price[j].max(1e-9) + x.kind[g][j]) * pack.goods[j].price).sum::<f32>())
        .sum();
    all / x.pop.iter().sum::<f32>().max(1e-6)
}

/// Goods somebody here needs that can be carried and were not on the market at all (a need
/// that goes unmet because a group cannot pay for what is on sale is not trade's to mend,
/// nor one for services, which nobody carries).
fn missing(x: &Nation, pack: &Pack) -> usize {
    (0..pack.goods.len()).filter(|&k| missing_good(x, pack, k)).count()
}

fn missing_good(x: &Nation, pack: &Pack, k: usize) -> bool {
    // buyers here would take at least four thousand units of a first consignment at half as
    // much again as the usual price (less is below what the engine tells from nothing)
    let need = pack.groups.iter().any(|g| g.needs.iter().any(|n| n.good == k));
    pack.goods[k].tradable && need && x.offered[k] <= 1e-3 && x.wished[k] / (1.5 * pack.goods[k].price * x.level) > 4e-3
}

/// The countries of a map under one era, trading, against the same countries alone.
pub(super) fn map_checks(checks: &mut Checks, path: &str, seed: u64) {
    let pack = Pack::load(path).unwrap_or_else(|e| panic!("{e}"));
    let Some(rules) = &pack.trade else { return };
    let tag = pack.name.to_lowercase();
    println!("\n══════ {} — карта {seed} ══════", pack.name);
    let world = map::generate(&Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 }).0;
    let pol = countries::generate(&world, seed, 30, &pack);
    let routes = trade::routes(&world, &pol, rules);
    let setups: Vec<Setup> = (0..pol.countries.len())
        .map(|c| Setup {
            name: pol.countries[c].name.clone(),
            pop: pol.pop(c),
            dev: pol.dev(c),
            fertility: pol.countries[c].fertility,
            land: pol.countries[c].land,
            deposits: pol.res(c),
        })
        .collect();
    let n = setups.len();
    let (solo, solo_log): (Vec<Nation>, Vec<Vec<Vec<f32>>>) = setups.par_iter().map(|s| alone_logged(&pack, s)).unzip();
    let mut w = World::new(&pack, setups.iter().map(|s| Nation::new(&pack, s)).collect(), routes.clone());
    w.settle(&pack, ALONE, TOGETHER);
    let start = w.money_total();
    let mut log: Vec<Vec<Vec<f32>>> = vec![vec![Vec::new(); pack.goods.len()]; n];
    let traced = std::env::var("TRACE").ok().and_then(|name| (0..n).find(|&c| w.nations[c].name == name));
    // per country and good over the game: what came in and went out, against what was used
    let (mut came, mut went, mut used) = (vec![vec![0.0f32; pack.goods.len()]; n], vec![vec![0.0f32; pack.goods.len()]; n], vec![vec![0.0f32; pack.goods.len()]; n]);
    for m in 0..MONTHS {
        w.step(&pack);
        if std::env::var("SOLVER").is_ok() {
            let (c, k, i, j, kf, df) = w.worst;
            println!(
                "  мес {m:>3}: поиск цен {} шагов, цены ещё двигались на {:.4} ({} {}, рынок {:.3}); поток {}→{} {} на {:.4} рынка",
                w.solver.0, w.solver.1, w.nations[c].name, pack.goods[k].name, w.nations[c].offered[k], w.nations[i].name, w.nations[j].name, pack.goods[kf].name, df
            );
        }
        for (c, x) in w.nations.iter().enumerate() {
            for k in 0..pack.goods.len() {
                came[c][k] += x.import_sold[k];
                went[c][k] += x.shipped[k];
                used[c][k] += x.sold[k] + x.import_sold[k];
            }
        }
        if let (Some(c), Ok(id)) = (traced, std::env::var("GOOD")) {
            // one good of the traced country, with who sends it and who buys it
            let k = pack.goods.iter().position(|g| g.id == id).expect("no such good");
            let x = &w.nations[c];
            let mut from: Vec<String> = (0..n).filter(|&i| w.flow[i][c][k] > 0.0).map(|i| format!("{} {:.3}", w.nations[i].name, w.flow[i][c][k])).collect();
            from.extend((0..n).filter(|&i| w.flow[c][i][k] > 0.0).map(|i| format!("→ {} {:.3}", w.nations[i].name, w.flow[c][i][k])));
            let b = x.bought_by[k];
            println!(
                "  {m:>3} ×{:.2} сгл ×{:.2} | сделано {:.3} своё на рынке {:.3} чужое {:.3} продано чужого {:.3} | купили люди {:.3} отр {:.3} стройка {:.3} гос {:.3} | вывоз {:.3} | везут: {}",
                x.price[k] / pack.goods[k].price, x.trend[k] / pack.goods[k].price, x.produced[k], x.offered[k] - x.import[k], x.import[k], x.import_sold[k],
                b[0], b[2], b[1], b[4], x.shipped[k], from.join(", ")
            );
        } else if let Some(c) = traced {
            let x = &w.nations[c];
            let goods: Vec<String> = (0..pack.goods.len())
                .filter(|&k| pack.goods[k].tradable)
                .map(|k| format!("{} ×{:.2} {:+.1}", &pack.goods[k].name.chars().take(4).collect::<String>(), x.price[k] / pack.goods[k].price, x.import_sold[k] - x.shipped[k]))
                .collect();
            println!("  {m:>3} деньги {:.0} ввоз-вывоз по товарам: {}", x.money_total(), goods.join(" | "));
        }
        for (c, x) in w.nations.iter().enumerate() {
            for k in 0..pack.goods.len() {
                record(&mut log[c][k], x, k);
            }
        }
    }
    if let Ok(id) = std::env::var("ROUTE") {
        // the biggest trade in this good that is out of balance, month by month
        let k = pack.goods.iter().position(|g| g.id == id).expect("no such good");
        let mut best = None;
        for i in 0..n {
            for j in 0..n {
                if let Some(g) = w.gap(&pack, i, j, k) {
                    if w.flow[i][j][k] > 0.0 && (g - 1.0).abs() > 0.1 && best.is_none_or(|(_, _, f)| w.flow[i][j][k] > f) {
                        best = Some((i, j, w.flow[i][j][k]));
                    }
                }
            }
        }
        if let Some((i, j, _)) = best {
            let r = routes[i][j].unwrap();
            println!("  {} → {}: путь теряет {:.0}% за {} мес", w.nations[i].name, w.nations[j].name, (1.0 - (-r.loss * pack.goods[k].bulk).exp()) * 100.0, r.months);
            for m in 0..24 {
                w.step(&pack);
                let (a, b) = (&w.nations[i], &w.nations[j]);
                let planned: f32 = (0..n).map(|d| w.flow[i][d][k]).sum();
                println!(
                    "   {m:>2} поток {:.3} (всего из A {:.3}, ушло {:.3}, дома на рынке {:.3}) | A цена {:.2} сгл {:.2} | B цена {:.2} сгл {:.2} ввезено {:.3} продано {:.3} | разрыв {:.2}",
                    w.flow[i][j][k], planned, a.shipped[k], a.offered[k], a.price[k], a.trend[k], b.price[k], b.trend[k], b.import[k], b.import_sold[k], w.gap(&pack, i, j, k).unwrap_or(0.0)
                );
            }
        }
        return;
    }
    let drift = ((w.money_total() - start) / start).abs();
    checks.push((format!("{tag}: деньги в мире сохраняются"), drift < 1e-4, format!("за 10 лет {:+.5}%", drift * 100.0)));

    let (active, off, worst, priced) = arbitrage(&w, &pack);
    if std::env::var("WHY").is_ok() {
        for k in 0..pack.goods.len() {
            let (mut act_off, mut idle_off, mut act, mut gaps) = (0, 0, 0, Vec::new());
            for i in 0..n {
                for j in 0..n {
                    let Some(g) = w.gap(&pack, i, j, k) else { continue };
                    if w.flow[i][j][k] > 0.0 {
                        act += 1;
                        if (g - 1.0).abs() > 0.1 {
                            act_off += 1;
                            gaps.push(g);
                        }
                    } else if g > 1.15 {
                        idle_off += 1;
                        gaps.push(g);
                    }
                }
            }
            gaps.sort_by(|a, b| a.total_cmp(b));
            let med = gaps.get(gaps.len() / 2).copied().unwrap_or(0.0);
            let absent = w.nations.iter().filter(|x| x.offered[k] <= 1e-3).count();
            let makers = w.nations.iter().filter(|x| x.produced[k] > 1e-3).count();
            println!(
                "  {:<10} торгуют {act:>4}, из них в неравновесии {act_off:>4}; не торгуют при выгоде {idle_off:>4}; медианный разрыв {med:.2}; нет на рынке в {absent} странах, делают в {makers}",
                pack.goods[k].name
            );
        }
        let lack: Vec<String> = (0..pack.goods.len())
            .map(|k| {
                let c = w.nations.iter().filter(|x| (0..pack.groups.len()).any(|g| pack.groups[g].needs.iter().zip(&x.unmet[g]).any(|(nd, &u)| nd.good == k && u > 0.99))).count();
                format!("{} {c}", pack.goods[k].name)
            })
            .collect();
        println!("  стран, где товара нет вовсе (с торговлей): {}", lack.join(", "));
    }
    checks.push((
        format!("{tag}: выгоды от перевозки почти не осталось"),
        (off as f32) <= 0.05 * priced as f32,
        format!("пар страна-страна-товар с ценами {priced}, торгуют {active}, в неравновесии {off}; худший разрыв {worst:.2}"),
    ));

    if std::env::var("LACK").is_ok() {
        for c in 0..n {
            for k in 0..pack.goods.len() {
                let (a, t) = (missing_good(&solo[c], &pack, k), missing_good(&w.nations[c], &pack, k));
                if a != t {
                    let x = &w.nations[c];
                    println!(
                        "  {} {}: {} | с торговлей сделано {:.4} (одна {:.4}), ввоз {:.4}, цена ×{:.2} (одна ×{:.2}), досягаемо {:?}",
                        x.name, pack.goods[k].name, if t { "пропал с торговлей" } else { "появился с торговлей" }, x.produced[k], solo[c].produced[k], x.import[k],
                        x.price[k] / pack.goods[k].price, solo[c].price[k] / pack.goods[k].price, w.reach(&pack, c, k)
                    );
                }
            }
        }
    }
    let lack_solo: usize = solo.iter().map(|x| missing(x, &pack)).sum();
    let lack_trade: usize = w.nations.iter().map(|x| missing(x, &pack)).sum();
    // what is still missing must be out of reach: no maker could bring it for what buyers there
    // would pay for a first consignment (in the wastes a caravan loses half of what it carries)
    let mut reachable = Vec::new();
    for j in 0..n {
        for k in 0..pack.goods.len() {
            if missing_good(&w.nations[j], &pack, k) {
                if let Some(r) = w.reach(&pack, j, k).filter(|&r| r < 0.95) {
                    reachable.push(format!("{} {} {r:.2}", w.nations[j].name, pack.goods[k].name));
                }
            }
        }
    }
    checks.push((
        format!("{tag}: чего не было совсем — привезли"),
        reachable.is_empty(),
        format!(
            "нужных товаров, которых нет на рынке вовсе: одни {lack_solo}, с торговлей {lack_trade}; из них можно было привезти {}{}",
            reachable.len(),
            if reachable.is_empty() { String::new() } else { format!(" ({})", reachable.iter().take(3).cloned().collect::<Vec<_>>().join(", ")) }
        ),
    ));

    let people = |v: &[Nation]| v.iter().map(|x| x.pop.iter().sum::<f32>()).sum::<f32>();
    let pains = |v: &[Nation]| v.iter().map(|x| pain(x) * x.pop.iter().sum::<f32>()).sum::<f32>() / people(v);
    // gains from trade: not fewer people, and not less consumed in all (trade feeds more
    // people, who share it — per head is shown); trade has losers too, and the pain of the
    // unmet needs is shown, not required to fall
    let eaten = |v: &[Nation]| v.iter().map(|x| consumed(x, &pack) * x.pop.iter().sum::<f32>()).sum::<f32>() / people(v);
    if std::env::var("BYGOOD").is_ok() {
        // what the world's people consumed of each good (at usual prices), alone and trading
        let of = |v: &[Nation], j: usize| -> f32 {
            v.iter().map(|x| (0..pack.groups.len()).map(|g| (x.spent[g][j] / x.price[j].max(1e-9) + x.kind[g][j]) * pack.goods[j].price).sum::<f32>()).sum()
        };
        for (j, good) in pack.goods.iter().enumerate() {
            println!("  {:<14} одни {:>8.1} с торговлей {:>8.1} ({:+.1}%)", good.name, of(&solo, j), of(&w.nations, j), (of(&w.nations, j) / of(&solo, j).max(1e-9) - 1.0) * 100.0);
        }
        let idle = |v: &[Nation], g: usize| -> (f32, f32) { (v.iter().map(|x| x.unemployed[g].unwrap_or(0.0) * x.pop[g]).sum(), v.iter().map(|x| x.pop[g]).sum()) };
        for (g, grp) in pack.groups.iter().enumerate() {
            let (a, b) = (idle(&solo, g), idle(&w.nations, g));
            println!("  {:<14} без работы одни {:>6.1} из {:>6.0}, с торговлей {:>6.1} из {:>6.0}", grp.name, a.0, a.1, b.0, b.1);
        }
        let land = |v: &[Nation]| -> (f32, f32) { (v.iter().map(|x| x.tilled).sum(), v.iter().map(|x| x.land).sum()) };
        println!("  земля: пашут одни {:.0} из {:.0}, с торговлей {:.0} из {:.0}", land(&solo).0, land(&solo).1, land(&w.nations).0, land(&w.nations).1);
    }
    checks.push((
        format!("{tag}: людей и потребления не меньше"),
        people(&w.nations) >= 0.99 * people(&solo) && eaten(&w.nations) * people(&w.nations) >= 0.99 * eaten(&solo) * people(&solo),
        format!(
            "люди {:.0} → {:.0}; потребление всего {:.0} → {:.0}, на душу {:.2} → {:.2}; тяжесть нужд в мире {:.1}% → {:.1}%",
            people(&solo),
            people(&w.nations),
            eaten(&solo) * people(&solo),
            eaten(&w.nations) * people(&w.nations),
            eaten(&solo),
            eaten(&w.nations),
            pains(&solo) * 100.0,
            pains(&w.nations) * 100.0
        ),
    ));

    // calm is compared on goods the country had without trade: a market that did not exist did
    // not swing, and any market looks more restless than none
    let had = |c: usize| -> Vec<usize> { (0..pack.goods.len()).filter(|&k| solo[c].produced[k] > 1e-3 || solo[c].offered[k] > 1e-3).collect() };
    let pick = |l: &[Vec<f32>], ks: &[usize]| -> Vec<Vec<f32>> { ks.iter().map(|&k| l[k].clone()).collect() };
    let restless: Vec<usize> = (0..n).map(|c| restless_years(&pick(&log[c], &had(c)))).collect();
    let calm = restless.iter().filter(|&&r| r <= 2).count();
    let calm_solo = (0..n).filter(|&c| restless_years(&pick(&solo_log[c], &had(c))) <= 2).count();
    if std::env::var("LOSERS").is_ok() {
        // who is worse off trading, and why: pop-weighted pain and people, alone vs trading
        let mut d: Vec<(usize, f32)> = (0..n)
            .map(|c| (c, pain(&w.nations[c]) * w.nations[c].pop.iter().sum::<f32>() - pain(&solo[c]) * solo[c].pop.iter().sum::<f32>()))
            .collect();
        d.sort_by(|a, b| b.1.total_cmp(&a.1));
        for &(c, dp) in d.iter().take(6) {
            for (label, x) in [("одна", &solo[c]), ("торгует", &w.nations[c])] {
                let gs: Vec<String> = (0..pack.groups.len())
                    .filter(|&g| x.hardship[g] > 0.03)
                    .map(|g| format!("{} {:.0}чел {:.0}% ({})", pack.groups[g].name, x.pop[g], x.hardship[g] * 100.0, x.cause(&pack, g)))
                    .collect();
                println!("  {:<12} {label:<8} люди {:.0} | {}", x.name, x.pop.iter().sum::<f32>(), gs.join("; "));
            }
            println!("     прирост беды (тяжесть×люди) {dp:+.1}");
        }
    }
    if std::env::var("KIND").is_ok() {
        // where restlessness that trade added comes from: goods a country mostly imports,
        // mostly exports, or barely trades
        let bad = |l: &Vec<f32>| (0..MONTHS as usize / 12).filter(|&y| swing(&l[y * 12..y * 12 + 12]) > 0.1).count() > 2;
        let (mut imp, mut exp, mut none, mut absent) = (0, 0, 0, 0);
        for c in 0..n {
            for k in 0..pack.goods.len() {
                if !(bad(&log[c][k]) && !bad(&solo_log[c][k])) {
                    continue;
                }
                let u = used[c][k].max(1e-6);
                if solo[c].produced[k] <= 1e-3 && solo[c].offered[k] <= 1e-3 {
                    absent += 1;
                } else if came[c][k] > 0.3 * u {
                    imp += 1;
                } else if went[c][k] > 0.3 * u {
                    exp += 1;
                } else {
                    none += 1;
                }
            }
        }
        println!("  добавленные торговлей качели: товара не было без торговли {absent}, в основном ввоз {imp}, в основном вывоз {exp}, почти без торговли {none}");
    }
    if std::env::var("CMP").is_ok() {
        // per good: in how many countries it is restless (more than 2 years of 10), alone and trading
        let bad = |l: &Vec<f32>| (0..MONTHS as usize / 12).filter(|&y| swing(&l[y * 12..y * 12 + 12]) > 0.1).count() > 2;
        for k in 0..pack.goods.len() {
            let a = (0..n).filter(|&c| bad(&solo_log[c][k])).count();
            let t = (0..n).filter(|&c| bad(&log[c][k])).count();
            let worse: Vec<String> = (0..n).filter(|&c| bad(&log[c][k]) && !bad(&solo_log[c][k])).map(|c| w.nations[c].name.clone()).take(4).collect();
            println!("  {:<12} неспокоен: одни {a:>2}, с торговлей {t:>2}; стало хуже, например: {}", pack.goods[k].name, worse.join(", "));
        }
    }
    if std::env::var("SOLO").is_ok() {
        // countries alone: which goods swing, and how
        for k in 0..pack.goods.len() {
            let bad: Vec<usize> = (0..n)
                .filter(|&c| (0..MONTHS as usize / 12).filter(|&y| swing(&solo_log[c][k][y * 12..y * 12 + 12]) > 0.1).count() > 2)
                .collect();
            if let Some(&c) = bad.first() {
                let months: Vec<String> = solo_log[c][k][84..120].iter().map(|v| format!("{:.2}", v.exp() / pack.goods[k].price)).collect();
                println!("  одна: {:<10} качается в {:>2} странах; например {} ({:.0} чел): {}", pack.goods[k].name, bad.len(), solo[c].name, solo[c].pop.iter().sum::<f32>(), months.join(" "));
            }
        }
    }
    if std::env::var("SWING").is_ok() {
        // which goods swing, and how
        for k in 0..pack.goods.len() {
            let bad: Vec<usize> = (0..n)
                .filter(|&c| (0..MONTHS as usize / 12).filter(|&y| swing(&log[c][k][y * 12..y * 12 + 12]) > 0.1).count() > 2)
                .collect();
            if let Some(&c) = bad.first() {
                let months: Vec<String> = log[c][k][96..120].iter().map(|v| format!("{:.2}", v.exp() / pack.goods[k].price)).collect();
                println!("  {:<10} качается в {:>2} странах; например {}: {}", pack.goods[k].name, bad.len(), w.nations[c].name, months.join(" "));
            }
        }
    }
    // countries with troubles have strikes and their prices jump; what trade must not do is
    // make countries more restless than they are on their own
    checks.push((
        format!("{tag}: торговля не раскачивает цены"),
        calm + 2 >= calm_solo,
        format!("спокойных стран {calm} из {n} (без торговли {calm_solo}; спокойна — не больше 2 беспокойных лет из 10)"),
    ));

    // the nearer, the more trade: a buyer takes a good from nearer makers rather than from far
    // ones. Measured against the makers it could buy from (against every way between any two
    // countries, a world where only a few countries make luxuries looked as if distance did not
    // matter, though each buyer took them from the nearest of those few)
    let (mut chosen, mut could, mut weight) = (0.0f32, 0.0f32, 0.0f32);
    for j in 0..n {
        for k in 0..pack.goods.len() {
            let got: Vec<(usize, f32)> = (0..n).filter(|&i| w.flow[i][j][k] > 0.0).map(|i| (i, w.flow[i][j][k])).collect();
            let all: f32 = got.iter().map(|x| x.1).sum();
            let makers: Vec<f32> = (0..n).filter(|&i| w.nations[i].produced[k] > 1e-3).filter_map(|i| routes[i][j].map(|r| r.loss)).collect();
            if all <= 0.0 || makers.len() < 2 {
                continue;
            }
            let value = all * w.nations[j].price[k];
            chosen += value * got.iter().map(|&(i, f)| f * routes[i][j].unwrap().loss).sum::<f32>() / all;
            could += value * makers.iter().sum::<f32>() / makers.len() as f32;
            weight += value;
        }
    }
    let (near, mean) = (chosen / weight.max(1e-9), could / weight.max(1e-9));
    let (mut load, mut carried, mut all, mut ways) = (0.0f32, 0.0f32, 0.0f32, 0);
    for i in 0..n {
        for j in 0..n {
            if let Some(r) = routes[i][j] {
                all += r.loss;
                ways += 1;
                load += w.carried[i][j] * r.loss;
                carried += w.carried[i][j];
            }
        }
    }
    // not a check: where the way is cheap next to how much makers' prices differ (rails and
    // steamers), buyers take from the cheapest maker wherever it is; where caravans lose half of
    // what they carry, from the nearest. Whether distance should weigh more is a design question
    // (makers' goods differing from each other, a fixed cost of each way), not a fault
    let lost = |l: f32| (1.0 - (-l).exp()) * 100.0;
    println!(
        "  дальность: путь от тех, у кого покупают, теряет {:.0}% товара, от всех, кто его делает, — {:.0}% (средняя перевозка {:.0}%, средний путь вообще {:.0}%)",
        lost(near),
        lost(mean),
        lost(load / carried.max(1e-9)),
        lost(all / ways.max(1) as f32)
    );

    // the story: who trades most, and what it did to them
    let mut by: Vec<(usize, f32, f32)> =
        (0..n).map(|c| (c, (0..n).map(|j| w.carried[c][j]).sum::<f32>(), (0..n).map(|i| w.carried[i][c]).sum::<f32>())).collect();
    by.sort_by(|a, b| (b.1 + b.2).total_cmp(&(a.1 + a.2)));
    for &(c, out, inn) in by.iter().take(8) {
        let x = &w.nations[c];
        let goods: Vec<String> = (0..pack.goods.len())
            .filter(|&k| x.import[k] > 0.01 || x.shipped[k] > 0.01)
            .map(|k| format!("{}{}", pack.goods[k].name, if x.shipped[k] > x.import[k] { "↑" } else { "↓" }))
            .collect();
        println!(
            "  {:<14} вывоз {:>6.1} ввоз {:>6.1} в мес | тяжесть {:>4.1}% → {:>4.1}% | люди {:.0} → {:.0} | {}",
            x.name,
            out,
            inn,
            pain(&solo[c]) * 100.0,
            pain(x) * 100.0,
            solo[c].pop.iter().sum::<f32>(),
            x.pop.iter().sum::<f32>(),
            goods.join(" ")
        );
    }
}
