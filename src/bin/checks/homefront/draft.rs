//! The checks of the law of the draft: twins live by the peace law in peace; at war the first
//! goes over to the war law (`WAR_LAW`), the second keeps the peace one. The law holds its
//! ceiling, a wider draft puts more men under arms, and it tells in the war — each against its
//! control: the same wars with the law changing nothing (all may serve as before, at the pay of
//! work).

use strategy::economy::nation::{LAWS, PEACE_LAW, WAR_LAW};
use super::*;

/// What the paired wars of one condition showed, per preparation (map and side): the most of the
/// people serving at once on each side (the worst of its wars), and the means over its lucks of
/// the first side's lead — in people serving, in its own land kept, in output and discontent over
/// the war's first half year.
struct Paired {
    most: Vec<[f32; 2]>,
    serving: Vec<f32>,
    kept: Vec<f32>,
    output: Vec<f32>,
    anger: Vec<f32>,
}

fn wars(pack: &Pack, maps: &[Map0], law: bool) -> Paired {
    let preps: Vec<(usize, bool)> = (0..maps.len()).flat_map(|m| [(m, false), (m, true)]).collect();
    let peaces: Vec<Homefront> = preps
        .par_iter()
        .map(|&(m, sw)| {
            let pair = if sw { (maps[m].pair.1, maps[m].pair.0) } else { maps[m].pair };
            let mut h = sandbox(pack, &maps[m].world, &maps[m].pol, pair, maps[m].twins.clone(), hash64(m as u64 * 2 + sw as u64) ^ 0xD2AF, true);
            if law {
                for s in 0..2 {
                    h.home_mut(s).set_law(PEACE_LAW);
                }
            }
            for _ in 0..PEACE {
                h.peace(pack);
            }
            if law {
                h.home_mut(0).set_law(WAR_LAW);
            }
            h
        })
        .collect();
    let jobs: Vec<(usize, u64)> = (0..preps.len()).flat_map(|p| LUCKS.iter().map(move |&k| (p, k))).collect();
    let runs: Vec<Run> = jobs.par_iter().map(|&(p, k)| run(pack, peaces[p].clone(), preps[p].0, preps[p].1, k, Cond::Twins)).collect();
    let half = |v: &[f32]| mean(&v[..v.len().min(6)]);
    let mut out = Paired { most: Vec::new(), serving: Vec::new(), kept: Vec::new(), output: Vec::new(), anger: Vec::new() };
    for &(m, sw) in &preps {
        let rs: Vec<&Run> = runs.iter().filter(|r| r.map == m && r.swapped == sw).collect();
        out.most.push([0, 1].map(|s| rs.iter().map(|r| r.drafted[s]).fold(0.0, f32::max)));
        out.serving.push(mean(&rs.iter().map(|r| r.drafted[0] - r.drafted[1]).collect::<Vec<_>>()));
        out.kept.push(mean(&rs.iter().map(|r| r.held[1].last().copied().unwrap_or(0.0) - r.held[0].last().copied().unwrap_or(0.0)).collect::<Vec<_>>()));
        out.output.push(mean(&rs.iter().map(|r| half(&r.gdp[0]) - half(&r.gdp[1])).collect::<Vec<_>>()));
        out.anger.push(mean(&rs.iter().map(|r| half(&r.anger[0]) - half(&r.anger[1])).collect::<Vec<_>>()));
    }
    out
}

pub(super) fn draft_checks(pack: &Pack) -> bool {
    let started = std::time::Instant::now();
    let maps = maps(pack, 1.0, 1.0);
    println!("\n  миры {} карт устоялись за {:.0} с", maps.len(), started.elapsed().as_secs_f32());
    let (wide, narrow) = (&LAWS[WAR_LAW], &LAWS[PEACE_LAW]);
    let with = wars(pack, &maps, true);
    let without = wars(pack, &maps, false);
    let mut checks: Vec<(String, bool, String)> = Vec::new();
    for (name, p) in [("закон", &with), ("контроль: закон без действия", &without)] {
        let most = [0, 1].map(|s| p.most.iter().map(|x| x[s]).fold(0.0, f32::max));
        let (sv, sv_se) = mean_se(&p.serving);
        let (kp, kp_se) = mean_se(&p.kept);
        let (out, out_se) = mean_se(&p.output);
        let (an, an_se) = mean_se(&p.anger);
        println!(
            "  {name}: служили разом самое большее {:.1}% и {:.1}% людей; у первой больше на {:.1} ± {:.1} п.; своей земли удержала больше на {:.1} ± {:.1} п.; выпуск первого полугода {:+.3} ± {:.3}, недовольство {:+.2} ± {:.2}",
            most[0] * 100.0, most[1] * 100.0, sv * 100.0, sv_se * 100.0, kp * 100.0, kp_se * 100.0, out, out_se, an, an_se
        );
        // 1. the law's ceiling holds — and is reached (else nothing was checked)
        let held = most[0] <= wide.cap * 1.02 && most[1] <= narrow.cap * 1.02 && most[1] >= narrow.cap * 0.8;
        checks.push((
            format!("{name}: закон держит потолок призыва"),
            held,
            format!("служили разом самое большее {:.1}% (потолок {:.0}%) и {:.1}% (потолок {:.0}%)", most[0] * 100.0, wide.cap * 100.0, most[1] * 100.0, narrow.cap * 100.0),
        ));
        // 2. a wider draft, more men under arms
        checks.push((format!("{name}: шире призыв — больше солдат"), sv > 0.01 && sv > 3.0 * sv_se, format!("у первой больше на {:.1} ± {:.1} п. людей", sv * 100.0, sv_se * 100.0)));
        // 3. it tells in the war: the side with the wider draft keeps more of its land
        checks.push((format!("{name}: шире призыв — крепче на войне"), kp > 2.0 * kp_se && kp > 0.0, format!("своей земли удержала больше на {:.1} ± {:.1} п.", kp * 100.0, kp_se * 100.0)));
    }
    // the control must fail every check of its own
    for c in checks.iter_mut().skip(3) {
        c.1 = !c.1;
        c.0 = format!("{} — не выходит", c.0);
    }
    report(&checks, started)
}
