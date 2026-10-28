//! The main self-checks of the home front: the economy carries the war (supply, the draft,
//! losses, blockade, occupation), each against its control.

use super::*;

pub(super) fn checks(pack: &Pack) -> bool {
    let started = std::time::Instant::now();
    let maps = maps(pack, 2.0, 1.0);
    println!("\n  миры {} карт устоялись за {:.0} с", maps.len(), started.elapsed().as_secs_f32());
    // the years of peace of each map and side (the same for every luck), then the wars
    let wars = |prep: Prep, conds: &[Cond]| -> Vec<Run> {
        let preps: Vec<(usize, bool)> = (0..maps.len()).flat_map(|m| [(m, false), (m, true)]).collect();
        let peaces: Vec<Homefront> = preps.par_iter().map(|&(m, sw)| prepare(pack, &maps[m], m, sw, prep, 0)).collect();
        let jobs: Vec<(usize, u64, Cond)> =
            conds.iter().flat_map(|&c| (0..preps.len()).flat_map(move |p| LUCKS.iter().map(move |&k| (p, k, c)))).collect();
        jobs.par_iter().map(|&(p, k, c)| run(pack, peaces[p].clone(), preps[p].0, preps[p].1, k, c)).collect()
    };
    let mut all = wars(PLAIN, &[Cond::Twins, Cond::Peace, Cond::Costless, Cond::Blockade, Cond::Unheld]);
    all.extend(wars(Prep { other: true, ..PLAIN }, &[Cond::Rich]));
    let of = |c: Cond| -> Vec<&Run> { all.iter().filter(|r| r.cond == c).collect() };
    let pair_of = |a: &Run, rs: &[&Run]| -> usize { rs.iter().position(|b| b.map == a.map && b.swapped == a.swapped && b.luck == a.luck).unwrap() };
    let won = |rs: &[&Run]| rs.iter().map(|r| match r.winner { Some(0) => 1.0, None => 0.5, _ => 0.0 }).sum::<f32>() / rs.len().max(1) as f32;
    let mut checks: Vec<(String, bool, String)> = Vec::new();

    let twins = of(Cond::Twins);
    let first = won(&twins);
    let mut months: Vec<u32> = twins.iter().map(|r| r.months).collect();
    months.sort_unstable();
    checks.push((
        "контроль: у близнецов первая сторона — орёл или решка".into(),
        (0.35..=0.65).contains(&first),
        format!(
            "{} войн (8 карт, обе стороны, по {} везений): первая сторона побеждает в {:.0}%; войны длятся {} мес. в середине ряда, капитуляцией кончились {}",
            twins.len(),
            LUCKS.len(),
            first * 100.0,
            months[months.len() / 2],
            twins.iter().filter(|r| r.months < WAR).count()
        ),
    ));

    // the rich one is the first country of the map's pair: the first side, or the second when
    // the sides are swapped
    let rich = of(Cond::Rich);
    let r = rich.iter().map(|x| match x.winner { Some(w) if w == x.swapped as usize => 1.0, None => 0.5, _ => 0.0 }).sum::<f32>() / rich.len().max(1) as f32;
    checks.push(("армию даёт экономика: вдвое больше людей и недр — побеждает".into(), r >= 0.8, format!("побеждает в {:.0}% войн", r * 100.0)));
    if std::env::var("BYMAP").is_ok() {
        for mi in 0..maps.len() {
            let rs: Vec<&Run> = twins.iter().copied().filter(|r| r.map == mi).collect();
            println!(
                "    карта {}: людей {:.0} млн, война {:.0} мес., погибло {:.1}%, служили в пике {:.1}%, привозного {:.0}%, первая побеждает {:.0}%",
                mi + 1,
                rs[0].people,
                mean(&rs.iter().map(|r| r.months as f32).collect::<Vec<_>>()),
                mean(&rs.iter().flat_map(|r| r.dead).collect::<Vec<_>>()) * 100.0,
                mean(&rs.iter().flat_map(|r| r.drafted).collect::<Vec<_>>()) * 100.0,
                mean(&rs.iter().map(|r| r.imported[0]).collect::<Vec<_>>()) * 100.0,
                won(&rs) * 100.0
            );
        }
    }

    // the blockaded side: its supply and its battles, against itself in the same war unblockaded
    let block = of(Cond::Blockade);
    let fed_drop: Vec<f32> = block.iter().map(|r| mean(&twins[pair_of(r, &twins)].fed[0]) - mean(&r.fed[0])).collect();
    let won_drop: Vec<f32> = block.iter().map(|r| twins[pair_of(r, &twins)].won - r.won).collect();
    let (fd, fse) = mean_se(&fed_drop);
    let (wd, wse) = mean_se(&won_drop);
    // battles lost for the supply lost, in proportion (no shortage, no harm): a blockade hurts as
    // much as a country lives on what comes from abroad. (Asked for fewer victories in every
    // blockade, the check failed in the wastes, whose army buys a sixth abroad and lost a seventh
    // of its supply; asked for a slope across wars, it failed where every blockade cut about as
    // much and only the level said anything.)
    let n = fed_drop.len().max(2) as f32;
    let sxx: f32 = fed_drop.iter().map(|x| x * x).sum();
    let b = fed_drop.iter().zip(&won_drop).map(|(x, y)| x * y).sum::<f32>() / sxx.max(1e-9);
    let resid: f32 = fed_drop.iter().zip(&won_drop).map(|(x, y)| (y - b * x).powi(2)).sum();
    let bse = (resid / (n - 1.0) / sxx.max(1e-9)).sqrt();
    let imp = mean(&twins.iter().map(|r| r.imported[0]).collect::<Vec<_>>());
    checks.push((
        "блокада роняет снабжение, нехватка — успехи на фронте".into(),
        fd > 3.0 * fse && b > 3.0 * bse,
        format!(
            "снабжение блокированного за войну {:.0}% (без блокады {:.0}%); на каждые 10 пунктов потерянного снабжения выигранных боёв меньше на {:.1} ± {:.1} пункта (в среднем {:.0}% против {:.0}%, разница {:.1} ± {:.1}); побеждает в войне {:.0}% (без блокады {:.0}%); привозного в покупках армии без блокады {:.0}%",
            mean(&block.iter().map(|r| mean(&r.fed[0])).collect::<Vec<_>>()) * 100.0,
            mean(&twins.iter().map(|r| mean(&r.fed[0])).collect::<Vec<_>>()) * 100.0,
            b * 10.0,
            bse * 10.0,
            mean(&block.iter().map(|r| r.won).collect::<Vec<_>>()) * 100.0,
            mean(&twins.iter().map(|r| r.won).collect::<Vec<_>>()) * 100.0,
            wd * 100.0,
            wse * 100.0,
            won(&block) * 100.0,
            first * 100.0,
            imp * 100.0
        ),
    ));

    // discontent over the months the war lasted, against the same countries at peace those
    // months; the same for a war that asks nothing more of the people (the control)
    let peace = of(Cond::Peace);
    let costless = of(Cond::Costless);
    let rise = |rs: &[&Run]| -> Vec<f32> {
        rs.iter()
            .flat_map(|r| {
                let p = peace[pair_of(r, &peace)];
                (0..2).map(move |s| {
                    let n = r.anger[s].len().min(p.anger[s].len());
                    if n == 0 { 0.0 } else { mean(&r.anger[s][..n]) - mean(&p.anger[s][..n]) }
                })
            })
            .collect()
    };
    let (rw, rwse) = mean_se(&rise(&twins));
    let (rc, rcse) = mean_se(&rise(&costless));
    let unrest = |rs: &[&Run]| mean(&rs.iter().flat_map(|r| (0..2).map(|s| *r.unrest[s].last().unwrap_or(&0) as f32 / r.months.max(1) as f32 * 12.0)).collect::<Vec<_>>());
    let calm = mean(&peace.iter().flat_map(|r| (0..2).map(|s| mean(&r.anger[s]))).collect::<Vec<_>>());
    checks.push((
        "война поднимает недовольство".into(),
        rw > 3.0 * rwse && rw > rc + 3.0 * (rwse * rwse + rcse * rcse).sqrt(),
        format!(
            "недовольство за месяцы войны выше, чем у тех же стран в мире, на {:.2} ± {:.2} (в мире {:.2}); волнений в год: война {:.1}, мир {:.1}; война, которая ничего не просит у тыла (контроль): {:+.2} ± {:.2}, волнений {:.1}",
            rw,
            rwse,
            calm,
            unrest(&twins),
            unrest(&peace),
            rc,
            rcse,
            unrest(&costless)
        ),
    ));
    // a country's output against the share of its land the enemy holds, month by month: the
    // slope with occupation, and in the same wars where what the enemy holds stays in the
    // economy (the control: the side losing land also loses men and money to the war)
    let slope = |rs: &[&Run]| -> (f32, usize) {
        let pts: Vec<(f32, f32)> = rs.iter().flat_map(|r| (0..2).flat_map(move |s| r.held[s].iter().copied().zip(r.gdp[s].iter().copied()))).collect();
        let n = pts.len().max(1) as f32;
        let (mx, my) = (pts.iter().map(|p| p.0).sum::<f32>() / n, pts.iter().map(|p| p.1).sum::<f32>() / n);
        let sxy: f32 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
        let sxx: f32 = pts.iter().map(|p| (p.0 - mx) * (p.0 - mx)).sum();
        (sxy / sxx.max(1e-9), pts.iter().filter(|p| p.0 >= 0.2).count())
    };
    let unheld = of(Cond::Unheld);
    let (k, nk) = slope(&twins);
    let (kc, _) = slope(&unheld);
    let ended = |rs: &[&Run]| rs.iter().filter(|r| r.months < WAR).count();
    let lasted = |rs: &[&Run]| {
        let mut m: Vec<u32> = rs.iter().map(|r| r.months).collect();
        m.sort_unstable();
        m[m.len() / 2]
    };
    checks.push((
        "занятая земля уходит из экономики".into(),
        (-1.5..=-0.5).contains(&(k - kc)),
        format!(
            "выпуск страны падает на {:.2} на каждую долю занятой земли (контроль — война, где занятое остаётся в экономике: {:.2}; месяцев с занятой пятой частью и больше {nk}); капитуляцией кончились {} войн из {} (в контроле {}), война длится {} мес. в середине ряда (в контроле {})",
            -k,
            -kc,
            ended(&twins),
            twins.len(),
            ended(&unheld),
            lasted(&twins),
            lasted(&unheld)
        ),
    ));
    // money never appears or vanishes: not with the army's purchases, the soldiers' pay, nor
    // what the enemy holds set apart
    let drift = all.iter().map(|r| ((r.money.1 - r.money.0) / r.money.0).abs()).fold(0.0f64, f64::max);
    checks.push(("деньги мира сохраняются".into(), drift < 1e-3, format!("наибольший уход денег мира за войну: {:.1e} от всех", drift)));
    println!(
        "  потери близнецов за войну: {:.2}% людей, служили в пике {:.1}% людей",
        mean(&twins.iter().flat_map(|r| r.dead).collect::<Vec<_>>()) * 100.0,
        mean(&twins.iter().flat_map(|r| r.drafted).collect::<Vec<_>>()) * 100.0
    );
    let _ = men_of;
    report(&checks, started)
}
