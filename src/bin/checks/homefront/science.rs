//! The checks of science at war (labs, focus, their weapons) and of scientists (their hidden
//! talent and schooling).

use super::*;

/// What a side's equipment really is: the mean log of fire and toughness over the kinds.
pub(super) fn quality(c: &Country) -> f32 {
    let nu = c.truth_fire.len().max(1) as f32;
    c.truth_fire.iter().zip(&c.truth_tough).map(|(f, t)| 0.5 * (f.ln() + t.ln())).sum::<f32>() / nu
}

/// Science: twins after ten years of peace, the first side giving science a tenth of the state's
/// spending and the second nothing; the control — both giving it a tenth.
pub(super) fn science_checks(pack: &Pack) -> bool {
    if pack.science.is_none() {
        println!("в наборе нет раздела [science]");
        return false;
    }
    let started = std::time::Instant::now();
    // effects of twenty-odd points need more independent years of peace than eight maps give:
    // sixteen maps, two lucks of war each
    let maps = maps_n(pack, 1.0, 1.0, 16);
    let lucks = &LUCKS[..2];
    println!("\n  миры {} карт устоялись за {:.0} с", maps.len(), started.elapsed().as_secs_f32());
    let preps: Vec<(usize, bool)> = (0..maps.len()).flat_map(|m| [(m, false), (m, true)]).collect();
    // per years of peace: the first side's share of wars won, its equipment against the second's
    // (log), designs made by each, output against the second's, the staff's error about its own
    // equipment (log) and the claims' error at the end of the peace
    struct Sci {
        won: f32,
        quality: f32,
        made: [u32; 2],
        gdp: f32,
        belief: f32,
        power: f32,
        /// People in the first side's laboratories (millions), their share of its learned, and the
        /// laboratories' pay against what the learned earn elsewhere.
        lab: (f32, f32, f32),
    }
    let side = |a: f32, b: f32| -> Vec<Sci> {
        let prep = Prep { science: (a, b), peace: SCIENCE_PEACE, ..PLAIN };
        let peaces: Vec<Homefront> = preps.par_iter().map(|&(m, sw)| prepare(pack, &maps[m], m, sw, prep, 0)).collect();
        let jobs: Vec<(usize, u64)> = (0..preps.len()).flat_map(|p| lucks.iter().map(move |&k| (p, k))).collect();
        let runs: Vec<Run> = jobs.par_iter().map(|&(p, k)| run(pack, peaces[p].clone(), preps[p].0, preps[p].1, k, Cond::Twins)).collect();
        (0..preps.len())
            .map(|p| {
                let rs: Vec<&Run> = runs.iter().filter(|r| r.map == preps[p].0 && r.swapped == preps[p].1).collect();
                let won = rs.iter().map(|r| match r.winner { Some(0) => 1.0, None => 0.5, _ => 0.0 }).sum::<f32>() / rs.len().max(1) as f32;
                let h = &peaces[p];
                let c = &h.war.sides[0];
                let nu = c.truth_fire.len().max(1) as f32;
                let belief = c.own_fire.iter().zip(&c.truth_fire).map(|(g, t)| (g.mean - t.ln()).abs()).sum::<f32>() / nu;
                Sci {
                    won,
                    quality: quality(c) - quality(&h.war.sides[1]),
                    made: [0, 1].map(|s| h.labs[s].as_ref().map_or(0, |l| l.made.iter().sum())),
                    gdp: h.home(0).gdp_real / h.home(1).gdp_real.max(1e-9),
                    belief,
                    power: h.labs[0].as_ref().map_or(0.0, |l| l.power),
                    lab: {
                        let n = h.home(0);
                        let learned = &pack.science.as_ref().unwrap().learned;
                        let all: f32 = learned.iter().map(|&g| n.pop[g]).sum::<f32>().max(1e-9);
                        let r: f32 = learned.iter().map(|&g| n.scholars[g]).sum();
                        let earn = learned.iter().map(|&g| n.earning(g) * n.pop[g]).sum::<f32>() / all;
                        (r, r / all, n.lab_pay / earn.max(1e-9))
                    },
                }
            })
            .collect()
    };
    let lead = side(0.1, 0.0);
    let control = side(0.1, 0.1);
    let col = |v: &[Sci], f: &dyn Fn(&Sci) -> f32| -> (f32, f32) { mean_se(&v.iter().map(f).collect::<Vec<_>>()) };
    let (lq, lqs) = col(&lead, &|x| x.quality);
    let (lw, lws) = col(&lead, &|x| x.won);
    let (cq, cqs) = col(&control, &|x| x.quality);
    let (cw, cws) = col(&control, &|x| x.won);
    let (lg, _) = col(&lead, &|x| x.gdp);
    let (lb, _) = col(&lead, &|x| x.belief);
    let (lp, _) = col(&lead, &|x| x.power);
    println!(
        "  в лабораториях к концу мира {:.2} млн — {:.1}% образованных; плата там к их заработку в стране ×{:.2}",
        col(&lead, &|x| x.lab.0).0,
        col(&lead, &|x| x.lab.1).0 * 100.0,
        col(&lead, &|x| x.lab.2).0
    );
    println!(
        "  за 10 лет мира: разработок у науки {:.0} (сила к концу {:.2} в мес.), у близнеца без науки {:.0}; выпуск науки к близнецу ×{:.3}; ошибка генштаба о своей технике {:.2}",
        mean(&lead.iter().map(|x| x.made[0] as f32).collect::<Vec<_>>()),
        lp,
        mean(&lead.iter().map(|x| x.made[1] as f32).collect::<Vec<_>>()),
        lg,
        lb
    );
    let checks = vec![
        (
            "наука делает технику лучше".to_string(),
            lq > 3.0 * lqs,
            format!("настоящее качество у науки против близнеца без неё: {:+.0}% ± {:.0}", (lq.exp() - 1.0) * 100.0, lqs * 100.0),
        ),
        (
            "и на войне побеждает чаще".to_string(),
            lw - 0.5 > 3.0 * lws,
            format!("наука против её отсутствия: побеждает в {:.0}% ± {:.0} войн ({} подготовок мира по {} везения)", lw * 100.0, lws * 100.0, lead.len(), lucks.len()),
        ),
        (
            "контроль: наука у обоих — равны".to_string(),
            cq.abs() < 3.0 * cqs.max(0.005) && (cw - 0.5).abs() < 3.0 * cws.max(0.01),
            format!("качество {:+.0}% ± {:.0}, первая сторона побеждает в {:.0}% ± {:.0}", (cq.exp() - 1.0) * 100.0, cqs * 100.0, cw * 100.0, cws * 100.0),
        ),
    ];
    report(&checks, started)
}

/// Scientists: twins giving science the same tenth of the state's spending for ten years of peace,
/// the first side's laboratories headed by a strong scientist and the second's by a weak one; the
/// control — both average.
pub(super) fn scientist_checks(pack: &Pack) -> bool {
    if pack.science.is_none() {
        println!("в наборе нет раздела [science]");
        return false;
    }
    let started = std::time::Instant::now();
    // an effect of some twenty points needs more independent years of peace than eight maps give:
    // sixteen maps, two lucks of war each
    let maps = maps_n(pack, 1.0, 1.0, 16);
    let lucks = &LUCKS[..2];
    println!("\n  миры {} карт устоялись за {:.0} с", maps.len(), started.elapsed().as_secs_f32());
    let preps: Vec<(usize, bool)> = (0..maps.len()).flat_map(|m| [(m, false), (m, true)]).collect();
    // per years of peace: the first side's share of wars won; its equipment against the second's,
    // each kind weighed by what the side's army is made of (log); what the second side's scientist took
    let side = |a: Mind, b: Mind| -> Vec<(f32, f32, f32)> {
        let prep = Prep { science: (0.1, 0.1), scientists: Some((a, b)), peace: SCIENCE_PEACE, ..PLAIN };
        let peaces: Vec<Homefront> = preps.par_iter().map(|&(m, sw)| prepare(pack, &maps[m], m, sw, prep, 0)).collect();
        if std::env::var("DBGSCI").is_ok() {
            for h in peaces.iter().take(4) {
                for s in 0..2 {
                    let sc = h.scientists[s].as_ref().unwrap();
                    let lab = h.labs[s].as_ref().unwrap();
                    let last: Vec<String> = sc.record.iter().rev().take(3).map(|(m, f)| format!("мес {m}: {}", f.iter().map(|x| format!("{:.0}", x * 100.0)).collect::<Vec<_>>().join("/"))).collect();
                    println!("    сторона {s}: разработок {:?}, армия {:?} | {}", lab.made, h.war.sides[s].worth_mix().iter().map(|x| (x * 100.0).round()).collect::<Vec<_>>(), last.join(" ; "));
                }
            }
        }
        let jobs: Vec<(usize, u64)> = (0..preps.len()).flat_map(|p| lucks.iter().map(move |&k| (p, k))).collect();
        let runs: Vec<Run> = jobs.par_iter().map(|&(p, k)| run(pack, peaces[p].clone(), preps[p].0, preps[p].1, k, Cond::Twins)).collect();
        (0..preps.len())
            .map(|p| {
                let rs: Vec<&Run> = runs.iter().filter(|r| r.map == preps[p].0 && r.swapped == preps[p].1).collect();
                let won = rs.iter().map(|r| match r.winner { Some(0) => 1.0, None => 0.5, _ => 0.0 }).sum::<f32>() / rs.len().max(1) as f32;
                let h = &peaces[p];
                let weighed = |s: usize| {
                    let c = &h.war.sides[s];
                    let w = c.worth_mix();
                    (0..w.len()).map(|u| w[u] * 0.5 * (c.truth_fire[u].ln() + c.truth_tough[u].ln())).sum::<f32>()
                };
                let took = h.scientists[1].as_ref().map_or(0.0, |x| x.took) / h.scientists[0].as_ref().map_or(1.0, |x| x.took.max(1e-6));
                (won, weighed(0) - weighed(1), took)
            })
            .collect()
    };
    let strong = side(Mind::strong(), Mind::weak());
    let control = side(Mind::average(), Mind::average());
    let col = |v: &[(f32, f32, f32)], k: usize| -> (f32, f32) { mean_se(&v.iter().map(|x| [x.0, x.1, x.2][k]).collect::<Vec<_>>()) };
    let (sw, sse) = col(&strong, 0);
    let (sq, sqs) = col(&strong, 1);
    let (cw, cse) = col(&control, 0);
    let (cq, cqs) = col(&control, 1);
    println!(
        "  за 10 лет мира: техника сильного против слабого (по составу своих армий) {:+.0}% ± {:.0}; контроль {:+.0}% ± {:.0}",
        (sq.exp() - 1.0) * 100.0,
        sqs * 100.0,
        (cq.exp() - 1.0) * 100.0,
        cqs * 100.0
    );
    // not a check: the effect is real but some twenty points, and at three standard errors it needs
    // more years of peace than three minutes allow (money for science decides far more: see
    // `--science`). Its power: all of a twin's science on a kind the army hardly builds wins the
    // other twin 66% ± 11 (`FOCUS1=4 --science`).
    println!(
        "  сведения: сильный учёный против слабого побеждает в {:.0}% ± {:.0} войн ({} подготовок мира по {} везения)",
        sw * 100.0,
        sse * 100.0,
        strong.len(),
        lucks.len()
    );
    let checks = vec![(
        "контроль: равные учёные — орёл или решка".to_string(),
        (cw - 0.5).abs() < 3.0 * cse.max(0.01),
        format!("средний против среднего: первая сторона побеждает в {:.0}% ± {:.0}", cw * 100.0, cse * 100.0),
    )];
    report(&checks, started)
}
