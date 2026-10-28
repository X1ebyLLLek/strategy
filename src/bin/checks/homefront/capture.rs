//! The checks of trophies: what a side learns from the weapons it takes.

use super::*;

/// Trophies: twins with the same science, the second side's equipment a third better when the
/// war begins; how much of the gap the first side closes by the end of the war when its
/// laboratories study the enemy's units, against the same war when they do not. The control: no
/// gap — trophies change nothing.
pub(super) fn capture_checks(pack: &Pack) -> bool {
    if pack.science.is_none() {
        println!("в наборе нет раздела [science]");
        return false;
    }
    const AHEAD: f32 = 0.3;
    let started = std::time::Instant::now();
    let maps = maps(pack, 1.0, 1.0);
    println!("\n  миры {} карт устоялись за {:.0} с", maps.len(), started.elapsed().as_secs_f32());
    let preps: Vec<(usize, bool)> = (0..maps.len()).flat_map(|m| [(m, false), (m, true)]).collect();
    let prep = Prep { science: (0.1, 0.1), ..PLAIN };
    let peaces: Vec<Homefront> = preps.par_iter().map(|&(m, sw)| prepare(pack, &maps[m], m, sw, prep, 0)).collect();
    // into the war four ways: the second side ahead or not, trophies studied or not
    let ways = [(AHEAD, true), (AHEAD, false), (0.0, true), (0.0, false)];
    let variant = |h: &Homefront, ahead: f32, capture: bool| -> Homefront {
        let mut h = h.clone();
        if ahead > 0.0 {
            for u in 0..pack.units.len() {
                h.war.sides[1].redesign(u, (ahead, ahead), (0.0, 0.0), 0.0);
            }
        }
        for lab in h.labs.iter_mut().flatten() {
            lab.capture = capture;
        }
        h
    };
    let jobs: Vec<(usize, u64, usize)> =
        (0..preps.len()).flat_map(|p| LUCKS.iter().flat_map(move |&k| (0..4).map(move |w| (p, k, w)))).collect();
    let runs: Vec<(usize, Run)> = jobs
        .par_iter()
        .map(|&(p, k, w)| (p, run(pack, variant(&peaces[p], ways[w].0, ways[w].1), preps[p].0, preps[p].1, k, Cond::Twins)))
        .collect();
    // per years of peace: the gap closed thanks to trophies (without them less one with them), and
    // the first side's wars won with and without them
    let per = |w: usize, f: &dyn Fn(&Run) -> f32| -> Vec<f32> {
        (0..preps.len())
            .map(|p| {
                let v: Vec<f32> = runs.iter().zip(&jobs).filter(|(r, j)| r.0 == p && j.2 == w).map(|(r, _)| f(&r.1)).collect();
                mean(&v)
            })
            .collect()
    };
    let gap = |r: &Run| r.gap;
    let won = |r: &Run| match r.winner { Some(0) => 1.0, None => 0.5, _ => 0.0 };
    let closed: Vec<f32> = per(1, &gap).iter().zip(per(0, &gap)).map(|(a, b)| a - b).collect();
    let control: Vec<f32> = per(3, &gap).iter().zip(per(2, &gap)).map(|(a, b)| a - b).collect();
    let (cm, cs) = mean_se(&closed);
    let (km, ks) = mean_se(&control);
    let (gw, _) = mean_se(&per(0, &won));
    let (nw, _) = mean_se(&per(1, &won));
    let (ge, _) = mean_se(&per(1, &gap));
    println!(
        "  отстающий (техника врага лучше на {:.0}%): к концу войны разрыв без трофеев {:.0}%; трофеи сокращают его ещё на {:.1} ± {:.1} пункта; побеждает в {:.0}% войн с трофеями и {:.0}% без них",
        (AHEAD.exp() - 1.0) * 100.0,
        (ge.exp() - 1.0) * 100.0,
        cm * 100.0,
        cs * 100.0,
        gw * 100.0,
        nw * 100.0
    );
    let checks = vec![
        (
            "трофеи: отстающий догоняет врага быстрее".to_string(),
            cm > 3.0 * cs,
            format!("разрыв меньше на {:.1} ± {:.1} пункта ({} подготовок мира по {} везения)", cm * 100.0, cs * 100.0, preps.len(), LUCKS.len()),
        ),
        (
            "контроль: нет разрыва — трофеи ничего не меняют".to_string(),
            km.abs() < 3.0 * ks.max(0.001),
            format!("разница {:+.1} ± {:.1} пункта", km * 100.0, ks * 100.0),
        ),
    ];
    report(&checks, started)
}
