//! The checks of economists at war: what their advice does to the war.

use super::*;

/// Economists at war: twins, the first side's economist strong and the second's weak (both
/// orders of the map's pair), against twins whose economists are both average (the control).
pub(super) fn econ_checks(pack: &Pack) -> bool {
    let started = std::time::Instant::now();
    let maps = maps(pack, 1.0, 1.0);
    println!("\n  миры {} карт устоялись за {:.0} с", maps.len(), started.elapsed().as_secs_f32());
    let preps: Vec<(usize, bool)> = (0..maps.len()).flat_map(|m| [(m, false), (m, true)]).collect();
    // per years of peace (the lucks of one share it): the first side's share of wars won, its people
    // and its army's money against the second's at the end of the peace
    let side = |a: Mind, b: Mind| -> Vec<(f32, f32, f32)> {
        let prep = Prep { econ: Some((a, b)), ..PLAIN };
        let peaces: Vec<Homefront> = preps.par_iter().map(|&(m, sw)| prepare(pack, &maps[m], m, sw, prep, 0)).collect();
        if std::env::var("DBGECON").is_ok() {
            for (p, h) in peaces.iter().enumerate().take(6) {
                let line: Vec<String> = (0..2)
                    .map(|s| {
                        let n = h.home(s);
                        let e = h.econ[s].as_ref().unwrap();
                        format!(
                            "украл {:.1} (казна {:.1}, армии в мес {:.2}, налог {:.0}%, доля армии {:.0}%), армия {:.0} частей, {}",
                            e.took, n.treasury, n.army_money(), n.tax * 100.0, n.mil_share * 100.0,
                            h.war.sides[s].army.count.iter().sum::<f32>(), e.levers.describe(pack, (pack.military, pack.tax))
                        )
                    })
                    .collect();
                println!("    подготовка {p}: {} || {}", line[0], line[1]);
            }
        }
        let jobs: Vec<(usize, u64)> = (0..preps.len()).flat_map(|p| LUCKS.iter().map(move |&k| (p, k))).collect();
        let runs: Vec<Run> = jobs.par_iter().map(|&(p, k)| run(pack, peaces[p].clone(), preps[p].0, preps[p].1, k, Cond::Twins)).collect();
        (0..preps.len())
            .map(|p| {
                let rs: Vec<&Run> = runs.iter().filter(|r| r.map == preps[p].0 && r.swapped == preps[p].1).collect();
                let won = rs.iter().map(|r| match r.winner { Some(0) => 1.0, None => 0.5, _ => 0.0 }).sum::<f32>() / rs.len().max(1) as f32;
                let h = &peaces[p];
                let people = h.home(0).people() / h.home(1).people().max(1e-9);
                let army = h.war.sides[0].army.count.iter().zip(&pack.units).map(|(n, u)| n * u.cost).sum::<f32>()
                    / h.war.sides[1].army.count.iter().zip(&pack.units).map(|(n, u)| n * u.cost).sum::<f32>().max(1e-9);
                (won, people, army)
            })
            .collect()
    };
    let strong = side(Mind::strong(), Mind::weak());
    let control = side(Mind::average(), Mind::average());
    let pick = |v: &[(f32, f32, f32)], k: usize| -> Vec<f32> { v.iter().map(|x| [x.0, x.1, x.2][k]).collect() };
    let (sw, sse) = mean_se(&pick(&strong, 0));
    let (cw, cse) = mean_se(&pick(&control, 0));
    let (sp, _) = mean_se(&pick(&strong, 1));
    let (sa, _) = mean_se(&pick(&strong, 2));
    let (cp, _) = mean_se(&pick(&control, 1));
    let (ca, _) = mean_se(&pick(&control, 2));
    println!(
        "  к началу войны (у первой стороны против второй): сильный против слабого — людей ×{:.3}, армия ×{:.2}; средний против среднего — людей ×{:.3}, армия ×{:.2}",
        sp, sa, cp, ca
    );
    let checks = vec![
        (
            "у кого экономист сильнее, тот чаще побеждает".to_string(),
            sw - 0.5 > 3.0 * sse,
            format!("сильный против слабого: побеждает в {:.0}% ± {:.0} войн ({} подготовок мира по {} везения)", sw * 100.0, sse * 100.0, strong.len(), LUCKS.len()),
        ),
        (
            "контроль: равные экономисты — орёл или решка".to_string(),
            (cw - 0.5).abs() < 3.0 * cse.max(0.01),
            format!("средний против среднего: первая сторона побеждает в {:.0}% ± {:.0}", cw * 100.0, cse * 100.0),
        ),
    ];
    report(&checks, started)
}
