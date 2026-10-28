//! The checks that a poor country lives (on its land, in kind).

use super::*;

/// How a less developed twin lives, on one map: its people against the start and the rich
/// twin's, after the world's pre-history and ten years later; how much of their vital needs each
/// gets in kind (from their own land and work); output a head against the rich twin's.
struct Life {
    start: f32,
    poor: (f32, f32),
    rich: (f32, f32),
    kind: (f32, f32),
    output: f32,
}

/// Share of a country's vital needs met in kind (what people grew or took as pay in kind).
fn in_kind(pack: &Pack, n: &Nation) -> f32 {
    let (mut kind, mut all) = (0.0f32, 0.0f32);
    for (g, grp) in pack.groups.iter().enumerate() {
        for need in grp.needs.iter().filter(|x| x.vital) {
            let j = need.good;
            kind += n.kind[g][j];
            all += n.kind[g][j] + n.spent[g][j] / n.price[j].max(1e-9);
        }
    }
    kind / all.max(1e-9)
}

fn lives(pack: &Pack, dev: f32, count: u64) -> Vec<Life> {
    let count: u64 = std::env::var("MAPS").ok().and_then(|v| v.parse().ok()).unwrap_or(count);
    (1..=count)
        .into_par_iter()
        .filter_map(|seed| {
            let world = map::generate(&Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 }).0;
            let pol = countries::generate(&world, seed, 30, &pack);
            let (p, r) = pick(&world, &pol)?;
            let start = average(&setup_of(&pol, p), &setup_of(&pol, r)).pop;
            let mut w = trading(pack, &world, &pol, (p, r), true, 1.0, dev);
            let before = (w.nations[p].people(), w.nations[r].people());
            for _ in 0..120 {
                w.step(pack);
            }
            let (np, nr) = (&w.nations[p], &w.nations[r]);
            Some(Life {
                start,
                poor: (before.0, np.people()),
                rich: (before.1, nr.people()),
                kind: (in_kind(pack, np), in_kind(pack, nr)),
                output: (np.gdp_real / np.people().max(1e-9)) / (nr.gdp_real / nr.people().max(1e-9)).max(1e-9),
            })
        })
        .collect()
}

/// A poor country lives: a twin half or a quarter as developed as its neighbour keeps its people
/// (poorer, more of it from its own land) instead of dying out — it did, while land had no limit
/// and hunger drove nobody to it (the control, the engine as it was: `FREELAND`).
pub(super) fn poor_checks(pack: &Pack) -> bool {
    let started = std::time::Instant::now();
    let mut checks: Vec<(String, bool, String)> = Vec::new();
    // it lives: on every map it keeps at least half the share of the rich twin's people that
    // Malthus gives (as many as its land feeds: about `dev` of them) and loses at most a tenth in
    // the ten years after the pre-history
    let alive = |l: &Life, dev: f32| l.poor.1 >= 0.5 * dev * l.rich.1 && l.poor.1 >= 0.9 * l.poor.0;
    // the control shows the check can fail in this era: without limited land the poor twin dies
    // out somewhere (at one level or the other — in the industrial era the old engine killed only
    // the one four times as backward)
    let (mut dead_ctrl, mut all_ctrl) = (0, 0);
    for dev in [0.5f32, 0.25] {
        let live = lives(pack, dev, 8);
        std::env::set_var("FREELAND", "1");
        let ctrl = lives(pack, dev, 8);
        std::env::remove_var("FREELAND");
        let show = |v: &[Life]| -> String {
            v.iter().map(|l| format!("{:.0}→{:.0}→{:.0}/{:.0}", l.start, l.poor.0, l.poor.1, l.rich.1)).collect::<Vec<_>>().join(" ")
        };
        println!("  развитие ×{dev}: люди бедного — в начале → после предыстории → ещё через 10 лет / у богатого");
        println!("    с землёй:      {}", show(&live));
        println!("    без предела:   {}", show(&ctrl));
        let ok = live.iter().filter(|l| alive(l, dev)).count();
        let ok_ctrl = ctrl.iter().filter(|l| alive(l, dev)).count();
        let ratio: Vec<f32> = live.iter().map(|l| l.poor.1 / l.rich.1.max(1e-9)).collect();
        checks.push((
            format!("развитие ×{dev}: бедный близнец живёт"),
            !live.is_empty() && ok == live.len(),
            format!("живёт на {ok} из {} карт; людей против богатого {:.0}% (от {:.0}% до {:.0}%)", live.len(), mean(&ratio) * 100.0,
                ratio.iter().copied().fold(f32::INFINITY, f32::min) * 100.0, ratio.iter().copied().fold(0.0, f32::max) * 100.0),
        ));
        println!("    контроль без предела земли: живёт на {ok_ctrl} из {} карт", ctrl.len());
        dead_ctrl += ctrl.len() - ok_ctrl;
        all_ctrl += ctrl.len();
        let kind_p: Vec<f32> = live.iter().map(|l| l.kind.0).collect();
        let kind_r: Vec<f32> = live.iter().map(|l| l.kind.1).collect();
        let out: Vec<f32> = live.iter().map(|l| l.output).collect();
        println!(
            "    сведения: жизненно нужного натурой у бедного {:.0}%, у богатого {:.0}%; выпуск на человека у бедного {:.0}% от богатого",
            mean(&kind_p) * 100.0, mean(&kind_r) * 100.0, mean(&out) * 100.0
        );
    }
    checks.push(("контроль: без предела земли бедный вымирает".into(), dead_ctrl > 0, format!("вымирает на {dead_ctrl} из {all_ctrl} карт (оба отставания)")));
    report(&checks, started)
}
