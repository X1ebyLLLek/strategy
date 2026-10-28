//! The checks of what armies grow from: the years of peace, the economy and the staff's
//! beliefs raise them.

use super::*;

/// What the years of peace made of an army (the army checks need no war).
struct Raised {
    key: (usize, bool, u64),
    /// Shares of each kind in each side's army, and what each kind costs each side.
    start: [Vec<f32>; 2],
    cost: [Vec<f32>; 2],
}

/// What armies grow from: what a country makes, trades and pays its men.
pub(super) fn army_checks(pack: &Pack) -> bool {
    let started = std::time::Instant::now();
    let poor: f32 = std::env::var("POOR").ok().and_then(|v| v.parse().ok()).unwrap_or(POOR);
    let maps = maps(pack, 1.0, poor);
    println!("\n  миры {} карт устоялись за {:.0} с", maps.len(), started.elapsed().as_secs_f32());
    let raise = |prep: Prep| -> Vec<Raised> {
        let keys: Vec<(usize, bool, u64)> =
            (0..maps.len()).flat_map(|m| [false, true].into_iter().flat_map(move |sw| ARMY_LUCKS.iter().map(move |&k| (m, sw, k)))).collect();
        keys.par_iter()
            .map(|&(m, sw, k)| {
                let h = prepare(pack, &maps[m], m, sw, prep, k);
                if std::env::var("DBGCOST").is_ok() {
                    let n = h.home(0);
                    let rel: Vec<String> = (0..pack.goods.len()).map(|i| format!("{} {:.2}", pack.goods[i].id, n.price[i] / (pack.goods[i].price * n.level))).collect();
                    println!("    карта {} {} везение {k}: плата солдату {:.3}, уровень цен {:.3}, ВВП на человека {:.2}; цены к обычным: {}", m + 1, if prep.other { "изменённая" } else { "близнецы" }, n.soldier_pay(), n.level, n.gdp_real / n.people(), rel.join(", "));
                    println!("      деньги армии {:.3}, казна {:.2}, заказ {:?}, план {:?}, армия {:?}", n.army_money(), n.treasury, n.army_wants.iter().map(|x| (x * 100.0).round() / 100.0).collect::<Vec<_>>(), h.war.sides[0].plan.iter().map(|x| (x * 100.0).round() / 100.0).collect::<Vec<_>>(), h.war.sides[0].army.count.iter().map(|x| x.round()).collect::<Vec<_>>());
                }
                Raised {
                    key: (m, sw, k),
                    start: [shares(pack, &h.war.sides[0].army.count), shares(pack, &h.war.sides[1].army.count)],
                    cost: [h.war.sides[0].cost.clone(), h.war.sides[1].cost.clone()],
                }
            })
            .collect()
    };
    let plain = raise(PLAIN);
    // the paired difference of `f` (with the plain twins minus with `other`), its mean and error
    let paired = |other: &[Raised], f: &dyn Fn(&Raised) -> f32| -> (f32, f32) {
        let d: Vec<f32> = other.iter().map(|r| f(plain.iter().find(|p| p.key == r.key).unwrap()) - f(r)).collect();
        mean_se(&d)
    };
    let nu = pack.units.len();
    let front: Vec<usize> = (0..nu).filter(|&u| pack.units[u].line == Line::Front).collect();
    let cheap = *front.iter().min_by(|&&a, &&b| (pack.units[a].cost / pack.units[a].width).total_cmp(&(pack.units[b].cost / pack.units[b].width))).unwrap();
    let built: Vec<f32> = (0..nu).map(|u| mean(&plain.iter().flat_map(|r| [r.start[0][u], r.start[1][u]]).collect::<Vec<_>>())).collect();
    // the good: of those the era's cheapest soldier does without, what the twins' armies need
    // most (by its worth in what they build); the heavy kind: of the kinds of the front line
    // they build, the one that leans on it most (by its share of the kind's worth). (Chosen as
    // "what they build most after the soldier", it came out anti-tank guns, and their most
    // telling good arms, which every kind needs.)
    let lean = |u: usize, i: usize| pack.units[u].equip.iter().find(|e| e.0 == i).map_or(0.0, |e| e.1 * pack.goods[i].price / pack.units[u].cost);
    let good = (0..pack.goods.len())
        .filter(|&i| lean(cheap, i) == 0.0)
        .max_by(|&a, &b| {
            let need = |i: usize| (0..nu).map(|u| built[u] * lean(u, i)).sum::<f32>();
            need(a).total_cmp(&need(b))
        })
        .unwrap();
    let heavy = *front.iter().filter(|&&u| u != cheap && built[u] >= 0.05).max_by(|&&a, &&b| lean(a, good).total_cmp(&lean(b, good))).unwrap_or(&cheap);
    // the control's good: one no kind of unit uses, that can be carried and is nobody's bread,
    // on which people spend most
    let used = |i: usize| pack.units.iter().any(|u| u.equip.iter().chain(&u.supply).any(|e| e.0 == i));
    let spend = |i: usize| pack.groups.iter().flat_map(|g| g.needs.iter().filter(move |n| n.good == i).map(move |n| g.share * n.amount * pack.goods[i].price)).sum::<f32>();
    let vital = |i: usize| pack.groups.iter().any(|g| g.needs.iter().any(|n| n.good == i && n.vital));
    let other = (0..pack.goods.len()).filter(|&i| !used(i) && pack.goods[i].tradable && !vital(i)).max_by(|&a, &b| spend(a).total_cmp(&spend(b))).unwrap();
    let mut checks: Vec<(String, bool, String)> = Vec::new();

    // without trade, the same country with and without the good; and short of a good its army
    // does not use (the control)
    let alone = raise(Prep { autarky: true, ..PLAIN });
    let alone_short = raise(Prep { autarky: true, short: Some(good), ..PLAIN });
    let alone_other = raise(Prep { autarky: true, short: Some(other), ..PLAIN });
    let traded_short = raise(Prep { short: Some(good), ..PLAIN });
    let share = |rs: &[Raised]| mean(&rs.iter().map(|r| r.start[0][heavy]).collect::<Vec<_>>());
    let drop = |a: &[Raised], b: &[Raised]| -> (f32, f32) {
        let d: Vec<f32> = b.iter().map(|r| a.iter().find(|p| p.key == r.key).unwrap().start[0][heavy] - r.start[0][heavy]).collect();
        mean_se(&d)
    };
    let (d, se) = drop(&alone, &alone_short);
    let (dc, sec) = drop(&alone, &alone_other);
    let (dt, set) = drop(&plain, &traded_short);
    checks.push((
        format!("без «{}» — меньше «{}»", pack.goods[good].name, pack.units[heavy].name),
        d > 3.0 * se && d - dc > 3.0 * (se * se + sec * sec).sqrt(),
        format!(
            "доля «{}» в армии к войне у страны без торговли: с «{}» {:.0}%, когда её делают вчетверо меньше, {:.0}% ({:+.1} ± {:.1} пункта); контроль — вчетверо меньше «{}», которая армии не нужна: {:+.1} ± {:.1}; с торговлей (нехватку можно привезти): {:.0}% и {:.0}% ({:+.1} ± {:.1})",
            pack.units[heavy].name,
            pack.goods[good].name,
            share(&alone) * 100.0,
            share(&alone_short) * 100.0,
            -d * 100.0,
            se * 100.0,
            pack.goods[other].name,
            -dc * 100.0,
            sec * 100.0,
            share(&plain) * 100.0,
            share(&traded_short) * 100.0,
            -dt * 100.0,
            set * 100.0
        ),
    ));

    // the same country, four times less developed: its men earn less, what it makes costs more
    // of their work. Shown, not checked: in this era men are a third of what a soldier costs and
    // arms are dear the world over, so the poor twin builds only a little more of the simplest
    // soldier — and its neighbour, which did not change at all, shifts as much, answering the
    // poor twin's army: the enemy decides the make-up more than the price of men. (Ten times
    // less developed, a country lives off its land, its treasury holds no money and it has no
    // army at all.)
    let poor = raise(Prep { other: true, ..PLAIN });
    if std::env::var("DBGCOST").is_ok() {
        for (name, rs) in [("развитый", &plain), ("бедный", &poor)] {
            let c: Vec<String> = (0..nu).map(|u| format!("{} {:.1}", pack.units[u].name, mean(&rs.iter().map(|r| r.cost[0][u]).collect::<Vec<_>>()))).collect();
            println!("    {name}: {}", c.join(", "));
        }
    }
    let (dp, sep) = paired(&poor, &|r| -r.start[0][cheap]);
    let (dn, sen) = paired(&poor, &|r| -r.start[1][cheap]);
    let ratio = |rs: &[Raised], s: usize| mean(&rs.iter().map(|r| r.cost[s][heavy] / r.cost[s][cheap]).collect::<Vec<_>>());
    println!(
        "  палки против танков (сведения): доля «{}» у вчетверо менее развитого близнеца {:.0}%, у развитого {:.0}% ({:+.1} ± {:.1} пункта); «{}» дороже «{}» в {:.1} раза у бедного и в {:.1} — у развитого; сосед-близнец, у которого ничего не менялось, сдвинулся на {:+.1} ± {:.1}",
        pack.units[cheap].name,
        mean(&poor.iter().map(|r| r.start[0][cheap]).collect::<Vec<_>>()) * 100.0,
        mean(&plain.iter().map(|r| r.start[0][cheap]).collect::<Vec<_>>()) * 100.0,
        dp * 100.0,
        sep * 100.0,
        pack.units[heavy].name,
        pack.units[cheap].name,
        ratio(&poor, 0),
        ratio(&plain, 0),
        dn * 100.0,
        sen * 100.0
    );
    report(&checks, started)
}
