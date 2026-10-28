//! The checks of groups of armies: generals (bold, talented, careful), orders, sectors, the
//! staff's reserve.

use super::*;

/// The group of side `s` whose anchor comes nearest to province `t`.
fn nearest_group(c: &Campaign, s: usize, t: usize) -> usize {
    let d = c.distance_from(t);
    let groups = c.command[s].as_ref().unwrap();
    (0..groups.len()).min_by_key(|&g| (groups[g].anchor.iter().map(|&p| d[p]).min().unwrap_or(u32::MAX), g)).unwrap_or(0)
}

/// The group of side `s` facing group `g` of the other side: the one most of whose sector's
/// provinces lie nearest to that group's anchor.
fn facing(c: &Campaign, s: usize, g: usize) -> usize {
    let o = 1 - s;
    let lab = c.group_of(s);
    let anchor = &c.command[o].as_ref().unwrap()[g].anchor;
    let k = c.command[s].as_ref().unwrap().len();
    let mut votes = vec![0usize; k];
    for &p in anchor {
        for &q in c.th.nbr[p].iter().chain(std::iter::once(&p)) {
            if lab[q] < k {
                votes[lab[q]] += 1;
            }
        }
    }
    (0..k).max_by_key(|&j| (votes[j], usize::MAX - j)).unwrap_or(0)
}

/// Side 0 leads by groups that hold, except group `g`, told `order`; side 1 as its staff likes.
fn one_order(c: &mut Campaign, g: usize, order: Order) {
    if let Some(groups) = c.command[0].as_mut() {
        groups[g].order = order;
    }
}

/// The checks of groups of armies: orders are carried out (hold, attack along the arrow, deep
/// defence), the staff that leads by groups is a fair player and answers a fist.
pub(super) fn groups_checks(pack: &Pack, months: u32) -> bool {
    let started = std::time::Instant::now();
    let maps: Vec<Theatre0> = (1..=8u64)
        .filter_map(|seed| {
            let world = map::generate(&Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 }).0;
            let pol = countries::generate(&world, seed, 30, &pack);
            pick(&world, &pol).map(|pair| Theatre0 { world, pol, pair })
        })
        .collect();
    use rayon::prelude::*;
    // a war of twins on every map, both ways round, with each of `lucks`; `setup` arranges how the
    // sides lead before the first month; `watch` reads the war after each month
    let run_on = |maps: &[Theatre0], lucks: &[u64], months: u32, setup: &(dyn Fn(&mut Campaign) + Sync)| -> Vec<Campaign> {
        let jobs: Vec<(usize, bool, u64)> = (0..maps.len()).flat_map(|m| [false, true].into_iter().flat_map(move |sw| lucks.iter().copied().map(move |k| (m, sw, k)))).collect();
        jobs.par_iter()
            .map(|&(m, swapped, k)| {
                let t = &maps[m];
                let pair = if swapped { (t.pair.1, t.pair.0) } else { t.pair };
                let mut c = campaign_of(pack, &t.world, &t.pol, pair, [1.0, 1.0], hash64(k * 1000 + m as u64), true);
                setup(&mut c);
                for _ in 0..months {
                    c.step(pack);
                    if c.fallen.is_some() {
                        break;
                    }
                }
                c
            })
            .collect()
    };
    let run = |lucks: &[u64], months: u32, setup: &(dyn Fn(&mut Campaign) + Sync)| run_on(&maps, lucks, months, setup);
    let mut checks: Vec<(String, bool, String)> = Vec::new();
    let war = months.min(24);
    // the group of side 0 nearest the enemy's capital
    let target = |c: &Campaign| nearest_group(c, 0, c.th.capital[1]);
    let held = |c: &Campaign, g: usize| -> (f32, f32) {
        // provinces of the band of group g: the enemy's taken, its own lost
        let lab = c.group_of(0);
        let taken = (0..c.owner.len()).filter(|&p| lab[p] == g && c.owner[p] == 0 && c.th.home[p] == 1).count() as f32;
        let lost = (0..c.owner.len()).filter(|&p| lab[p] == g && c.owner[p] == 1 && c.th.home[p] == 0).count() as f32;
        (taken, lost)
    };

    // 1. hold or attack: the same war, the same group, two orders (the group with half the army)
    let all_lucks = [11u64, 12, 13, 14, 15, 16];
    let strong = |c: &mut Campaign, g: usize, order: Order| {
        let groups = c.command[0].as_mut().unwrap();
        let k = groups.len();
        for (j, gr) in groups.iter_mut().enumerate() {
            gr.share = if j == g { 0.5 } else { 0.3 / (k - 1).max(1) as f32 };
        }
        one_order(c, g, order);
    };
    let hold = run(&all_lucks, war, &|c| {
        c.by_groups(0, false);
        let g = target(c);
        strong(c, g, Order::Hold);
    });
    let attack = run(&all_lucks, war, &|c| {
        c.by_groups(0, false);
        let g = target(c);
        let cap1 = c.th.capital[1];
        strong(c, g, Order::Attack(cap1));
    });
    let (mut dt, mut dl, mut ta, mut th, mut la, mut lh) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (h, a) in hold.iter().zip(&attack) {
        let g = target(h);
        let (t1, l1) = held(h, g);
        let (t2, l2) = held(a, g);
        dt.push(t2 - t1);
        dl.push(l1 - l2);
        ta.push(t2);
        th.push(t1);
        la.push(l2);
        lh.push(l1);
    }
    let (mdt, sdt) = mean_se(&dt);
    let _ = dl;
    let blows_held: usize = hold.iter().map(|c| c.fought.iter().filter(|f| f.attacker == 0).count()).sum();
    let blows_told: usize = attack.iter().map(|c| c.fought.iter().filter(|f| f.attacker == 0).count()).sum();
    checks.push((
        "приказ исполняется: «держать» не атакует, «наступать» берёт".into(),
        blows_held == 0 && blows_told > 0 && mdt > 3.0 * sdt && mdt > 0.5,
        format!(
            "группа с половиной армии у вражеской столицы, {} войн по {war} мес.: ударов при «держать» {blows_held}, при «наступать» {blows_told}; взято провинций врага {:.1} и {:.1} (разница {mdt:+.1} ± {sdt:.1}); сведения: своих потеряно {:.1} и {:.1}",
            dt.len(),
            mean_se(&th).0,
            mean_se(&ta).0,
            mean_se(&lh).0,
            mean_se(&la).0
        ),
    ));

    // 2. along the arrow: the same group attacks towards the capital, or (control) towards an enemy
    // province as far from the sector but far from the capital
    let other_head = |c: &Campaign| -> Option<usize> {
        let g = target(c);
        let anchor = &c.command[0].as_ref().unwrap()[g].anchor;
        let mut from = vec![u32::MAX; c.owner.len()];
        for &p in anchor {
            let d = c.distance_from(p);
            for r in 0..d.len() {
                from[r] = from[r].min(d[r]);
            }
        }
        let cap = c.th.capital[1];
        let dc = c.distance_from(cap);
        (0..c.owner.len())
            .filter(|&r| c.th.home[r] == 1 && from[r].abs_diff(from[cap]) <= 2)
            .max_by_key(|&r| (dc[r], r))
            .filter(|&r| dc[r] >= 4)
    };
    let aside = run(&all_lucks, war, &|c| {
        c.by_groups(0, false);
        let g = target(c);
        let head = other_head(c).unwrap_or(c.th.capital[1]);
        strong(c, g, Order::Attack(head));
    });
    // each successful blow: how many steps nearer the capital it brought the front (from where it
    // was struck to the province taken), with the arrow on the capital and with it aside
    let (mut fa, mut fb) = (Vec::new(), Vec::new());
    for (a, b) in attack.iter().zip(&aside) {
        if other_head(a).is_none() {
            continue;
        }
        let dc = a.distance_from(a.th.capital[1]);
        let step = |c: &Campaign| -> Option<f32> {
            let won: Vec<f32> = c.fought.iter().filter(|f| f.attacker == 0 && f.won).map(|f| dc[f.from] as f32 - dc[f.province] as f32).collect();
            (!won.is_empty()).then(|| won.iter().sum::<f32>() / won.len() as f32)
        };
        if let (Some(x), Some(y)) = (step(a), step(b)) {
            fa.push(x);
            fb.push(y);
        }
    }
    let diffs: Vec<f32> = fa.iter().zip(&fb).map(|(a, b)| a - b).collect();
    let (md, sd) = mean_se(&diffs);
    let (ma, sa) = mean_se(&fa);
    checks.push((
        "наступают по стрелке".into(),
        ma > 3.0 * sa && md > 3.0 * sd,
        format!(
            "удачный удар приближает к столице на {ma:.2} шага при стрелке на неё и на {:.2} при стрелке в сторону (контроль); разница {md:+.2} ± {sd:.2}, {} войн",
            mean_se(&fb).0,
            diffs.len()
        ),
    ));

    // 3. deep defence: the enemy strikes a fist at one sector (side 1, its group nearest our
    // capital, 60% of its army); our group there holds — or holds deep
    let fist_at = |c: &mut Campaign| {
        c.by_groups(1, false);
        let h = nearest_group(c, 1, c.th.capital[0]);
        let cap0 = c.th.capital[0];
        let groups = c.command[1].as_mut().unwrap();
        let k = groups.len();
        for (j, gr) in groups.iter_mut().enumerate() {
            gr.share = if j == h { 0.6 } else { 0.2 / (k - 1).max(1) as f32 };
            gr.order = if j == h { Order::Attack(cap0) } else { Order::Hold };
        }
        h
    };
    // (longer wars and twice the maps: a fist takes few provinces a year, and the price needs
    // them; the error by leaving one map out at a time — the wars of a map share its ground)
    let long = months.min(36);
    let more: Vec<Theatre0> = (9..=16u64)
        .filter_map(|seed| {
            let world = map::generate(&Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 }).0;
            let pol = countries::generate(&world, seed, 30, &pack);
            pick(&world, &pol).map(|pair| Theatre0 { world, pol, pair })
        })
        .collect();
    let lined_more = run_on(&more, &all_lucks, long, &|c| {
        c.by_groups(0, false);
        fist_at(c);
    });
    let deep_more = run_on(&more, &all_lucks, long, &|c| {
        c.by_groups(0, false);
        let h = fist_at(c);
        let g = facing(c, 0, h);
        one_order(c, g, Order::Deep);
    });
    let lined_first = run_on(&maps, &all_lucks, long, &|c| {
        c.by_groups(0, false);
        fist_at(c);
    });
    let deep_first = run_on(&maps, &all_lucks, long, &|c| {
        c.by_groups(0, false);
        let h = fist_at(c);
        let g = facing(c, 0, h);
        one_order(c, g, Order::Deep);
    });
    let lined: Vec<Campaign> = lined_first.into_iter().chain(lined_more).collect();
    let deep: Vec<Campaign> = deep_first.into_iter().chain(deep_more).collect();
    // the price of a province: what the striking side lost attacking in the band of our group,
    // for each province it took there
    let price = |c: &Campaign, g: usize| -> (f32, f32) {
        let lab = c.group_of(0);
        let (mut spent, mut took) = (0.0f32, 0.0f32);
        for f in &c.fought {
            if f.attacker == 1 && lab[f.province] == g {
                spent += f.lost[0];
                took += f.won as u8 as f32;
            }
        }
        (spent, took)
    };
    // per war: (spent, took) in line and in depth; the price is the ratio of the totals over all
    // wars, its error by leaving one war out at a time (jackknife)
    let mut per: Vec<[f32; 4]> = Vec::new();
    let (mut lost_l, mut lost_d) = (Vec::new(), Vec::new());
    for (l, d) in lined.iter().zip(&deep) {
        let h = nearest_group(l, 1, l.th.capital[0]);
        let g = facing(l, 0, h);
        let (sl, tl) = price(l, g);
        let (sd, td) = price(d, g);
        per.push([sl, tl, sd, td]);
        lost_l.push(held(l, g).1);
        lost_d.push(held(d, g).1);
    }
    let ratio = |rows: &[[f32; 4]]| -> f32 {
        let t: [f32; 4] = rows.iter().fold([0.0; 4], |a, r| [a[0] + r[0], a[1] + r[1], a[2] + r[2], a[3] + r[3]]);
        (t[2] / t[3].max(1e-9)).ln() - (t[0] / t[1].max(1e-9)).ln()
    };
    let all_r = ratio(&per);
    let per_map = 2 * all_lucks.len();
    let maps_n = per.len() / per_map;
    let n = maps_n as f32;
    let leave: Vec<f32> = (0..maps_n).map(|i| ratio(&[&per[..i * per_map], &per[(i + 1) * per_map..]].concat())).collect();
    let mean_leave = leave.iter().sum::<f32>() / n;
    let se_r = ((n - 1.0) / n * leave.iter().map(|x| (x - mean_leave) * (x - mean_leave)).sum::<f32>()).sqrt();
    let tot: [f32; 4] = per.iter().fold([0.0; 4], |a, r| [a[0] + r[0], a[1] + r[1], a[2] + r[2], a[3] + r[3]]);
    let (ml, _) = mean_se(&lost_l);
    let (mdp, _) = mean_se(&lost_d);
    checks.push((
        "глубокая оборона: прорыв дороже".into(),
        all_r > 3.0 * se_r,
        format!(
            "враг бьёт кулаком в один участок ({} войн на {maps_n} картах по {long} мес.): за взятую провинцию он теряет при обороне в линию {:.0}, в глубину {:.0} (×{:.2}, ошибка логарифма по картам {:.2}); провинций взял {:.0} и {:.0}; потеряно на войну {ml:.1} и {mdp:.1}",
            lined.len(),
            tot[0] / tot[1].max(1.0),
            tot[2] / tot[3].max(1.0),
            all_r.exp(),
            se_r,
            tot[1],
            tot[3]
        ),
    ));

    // 4. two staffs leading by groups, twins: heads or tails
    let twins = run(&all_lucks, war, &|c| {
        c.by_groups(0, true);
        c.by_groups(1, true);
    });
    let first: f32 = twins.iter().map(|c| match winner(c) { Some(0) => 1.0, None => 0.5, _ => 0.0 }).sum::<f32>() / twins.len() as f32;
    checks.push((
        "два одинаковых генштаба по группам — орёл или решка".into(),
        (0.35..=0.65).contains(&first),
        format!("{} войн: первая сторона побеждает в {:.0}%", twins.len(), first * 100.0),
    ));

    // 5. the staff answers a fist: side 0 masses 60% of its army on one sector and attacks; side
    // 1's staff, leading by groups, raises the share of its group facing it (control: side 0 even)
    let watch = |fist: bool| -> Vec<f32> {
        let mut jobs: Vec<(usize, bool, u64)> = Vec::new();
        for m in 0..maps.len() {
            for sw in [false, true] {
                for &k in &all_lucks {
                    jobs.push((m, sw, k));
                }
            }
        }
        jobs.par_iter()
            .map(|&(m, swapped, k)| {
                let t = &maps[m];
                let pair = if swapped { (t.pair.1, t.pair.0) } else { t.pair };
                let mut c = campaign_of(pack, &t.world, &t.pol, pair, [1.0, 1.0], hash64(k * 1000 + m as u64), true);
                c.by_groups(0, false);
                c.by_groups(1, true);
                let g = target(&c);
                if fist {
                    let cap1 = c.th.capital[1];
                    let groups = c.command[0].as_mut().unwrap();
                    let n = groups.len();
                    for (j, gr) in groups.iter_mut().enumerate() {
                        gr.share = if j == g { 0.6 } else { 0.2 / (n - 1).max(1) as f32 };
                        gr.order = if j == g { Order::Attack(cap1) } else { Order::Hold };
                    }
                }
                let h = facing(&c, 1, g);
                c.step(pack);
                let before = c.command[1].as_ref().unwrap()[h].share;
                c.step(pack);
                c.step(pack);
                c.command[1].as_ref().unwrap()[h].share - before
            })
            .collect()
    };
    let rise_fist = watch(true);
    let rise_even = watch(false);
    let d: Vec<f32> = rise_fist.iter().zip(&rise_even).map(|(a, b)| a - b).collect();
    let (md, sd) = mean_se(&d);
    checks.push((
        "генштаб отвечает на кулак: усиливает участок напротив".into(),
        md > 0.05 && md > 3.0 * sd,
        format!(
            "доля армии на участке напротив кулака за два месяца: {:+.0} пунктов (контроль — ровный фронт: {:+.0}); разница {:+.0} ± {:.0}, {} войн",
            mean_se(&rise_fist).0 * 100.0,
            mean_se(&rise_even).0 * 100.0,
            md * 100.0,
            sd * 100.0,
            d.len()
        ),
    ));

    // 6. generals: the same group, the same war, told the same — led by different men. The bold
    // strikes more often and at poorer odds than the cautious (a weak group, a seventh of the army:
    // with half of it, a group has the force for every target near the arrow's head, and the
    // number of its blows says nothing of its general); the gifted strikes more truly than the
    // ungifted (with the same men, the same: the control)
    let led = |bold: f32, talent: f32, share: f32| -> Vec<Campaign> {
        run(&all_lucks, war, &move |c: &mut Campaign| {
            c.by_groups(0, false);
            let g = target(c);
            let cap1 = c.th.capital[1];
            strong(c, g, Order::Attack(cap1));
            let groups = c.command[0].as_mut().unwrap();
            let k = groups.len();
            for (j, gr) in groups.iter_mut().enumerate() {
                gr.share = if j == g { share } else { (0.8 - share) / (k - 1).max(1) as f32 };
            }
            c.command[0].as_mut().unwrap()[g].general = Some(General { name: "N".into(), education: 0.6, experience: 0.2, talent, bold, won: 0, lost: 0, took: 0, past: Vec::new() });
        })
    };
    // per war: blows of the group, mean odds promised, share won
    let blows = |cs: &[Campaign]| -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let (mut n, mut odds, mut won) = (Vec::new(), Vec::new(), Vec::new());
        for c in cs {
            let g = target(c);
            let lab = c.group_of(0);
            let mine: Vec<&strategy::military::front::Fought> = c.fought.iter().filter(|f| f.attacker == 0 && lab[f.from] == g).collect();
            n.push(mine.len() as f32);
            if !mine.is_empty() {
                odds.push(mine.iter().map(|f| f.odds).sum::<f32>() / mine.len() as f32);
                won.push(mine.iter().filter(|f| f.won).count() as f32 / mine.len() as f32);
            }
        }
        (n, odds, won)
    };
    let (bn, bo, _) = blows(&led(0.9, 0.5, 0.15));
    let (cn, co, _) = blows(&led(-0.9, 0.5, 0.15));
    let (_, _, gw) = blows(&led(0.0, 0.95, 0.5));
    let (_, _, uw) = blows(&led(0.0, 0.05, 0.5));
    let (_, _, sw) = blows(&led(0.0, 0.05, 0.5));
    let dn: Vec<f32> = bn.iter().zip(&cn).map(|(a, b)| a - b).collect();
    let (mdn, sdn) = mean_se(&dn);
    let (mbo, sbo) = mean_se(&bo);
    let (mco, sco) = mean_se(&co);
    let (mg, sg) = mean_se(&gw);
    let (mu, su) = mean_se(&uw);
    let (ms, _) = mean_se(&sw);
    let bold_ok = mdn > 3.0 * sdn && mco - mbo > 3.0 * (sbo * sbo + sco * sco).sqrt();
    let gift_ok = mg - mu > 3.0 * (sg * sg + su * su).sqrt() && (ms - mu).abs() < 1e-6;
    checks.push((
        "генерал: смелый бьёт чаще при худших шансах, талантливый — вернее".into(),
        bold_ok && gift_ok,
        format!(
            "ударов за войну у смелого больше на {mdn:.1} ± {sdn:.1}, обещанные шансы {:.0}% против {:.0}% у осторожного; удачных ударов у талантливого {:.0}%, у бесталанного {:.0}% (± {:.0}); тот же человек заново — {:.0}%",
            mbo * 100.0,
            mco * 100.0,
            mg * 100.0,
            mu * 100.0,
            (sg * sg + su * su).sqrt() * 100.0,
            ms * 100.0
        ),
    ));

    // what a staff leading by groups is against one leading as one (info)
    let mixed = run(&all_lucks, war, &|c| c.by_groups(0, true));
    let w: f32 = mixed.iter().map(|c| match winner(c) { Some(0) => 1.0, None => 0.5, _ => 0.0 }).sum::<f32>() / mixed.len() as f32;
    println!("  сведения: генштаб по группам против генштаба по всему фронту — побеждает в {:.0}% ({} войн)", w * 100.0, mixed.len());

    println!("\nпроверки групп армий ({:.0} с):", started.elapsed().as_secs_f32());
    let mut ok = true;
    for (name, pass, detail) in &checks {
        ok &= pass;
        println!("  [{}] {:<44} {}", if *pass { "OK" } else { "FAIL" }, name, detail);
    }
    ok
}
