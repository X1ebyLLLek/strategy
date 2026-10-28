//! The checks of the war on the map: the staff's beliefs, supply, pockets, the war's length,
//! each against its control.

use super::*;

pub(super) fn checks(pack: &Pack, months: u32) -> bool {
    let started = std::time::Instant::now();
    let maps: Vec<Theatre0> = (1..=8u64)
        .filter_map(|seed| {
            let world = map::generate(&Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 }).0;
            let pol = countries::generate(&world, seed, 30, &pack);
            pick(&world, &pol).map(|pair| Theatre0 { world, pol, pair })
        })
        .collect();
    // every map, both ways round, two lucks each
    let run = |money: [f32; 2], fog: [f32; 2], twins: bool, old_pockets: bool| -> Vec<(Campaign, bool)> {
        let jobs: Vec<(usize, bool, u64)> = (0..maps.len()).flat_map(|m| [false, true].into_iter().flat_map(move |sw| LUCKS.iter().copied().map(move |k| (m, sw, k)))).collect();
        use rayon::prelude::*;
        jobs.par_iter()
            .map(|&(m, swapped, k)| {
                let t = &maps[m];
                let pair = if swapped { (t.pair.1, t.pair.0) } else { t.pair };
                let mut c = campaign_of(pack, &t.world, &t.pol, pair, money, hash64(k * 1000 + m as u64), twins);
                c.fog = fog;
                c.pockets = c.pockets && !old_pockets;
                for _ in 0..months {
                    c.step(pack);
                    if c.fallen.is_some() {
                        break;
                    }
                }
                (c, swapped)
            })
            .collect()
    };
    let share_won = |cs: &[(Campaign, bool)], side: usize| {
        let w: f32 = cs.iter().map(|(c, _)| match winner(c) { Some(x) if x == side => 1.0, None => 0.5, _ => 0.0 }).sum();
        w / cs.len() as f32
    };
    let mut checks: Vec<(String, bool, String)> = Vec::new();

    let even = run([1.0, 1.0], [0.5, 0.5], true, false);

    // sectors of the front: every province of the front in one of them, each hanging together,
    // of about the size asked; the control cuts the same front by the provinces' numbers
    {
        let size = 6;
        let starts: Vec<Campaign> = maps
            .iter()
            .flat_map(|t| [false, true].map(|sw| campaign_of(pack, &t.world, &t.pol, if sw { (t.pair.1, t.pair.0) } else { t.pair }, [1.0, 1.0], 1, true)))
            .collect();
        let fronts: Vec<&Campaign> = starts.iter().chain(even.iter().map(|(c, _)| c)).collect();
        let whole = |c: &Campaign, part: &[usize]| -> bool {
            // a sector hangs together over the theatre's neighbours inside it
            let mut seen = vec![part[0]];
            let mut i = 0;
            while i < seen.len() {
                for &r in &c.th.nbr[seen[i]] {
                    if part.contains(&r) && !seen.contains(&r) {
                        seen.push(r);
                    }
                }
                i += 1;
            }
            seen.len() == part.len()
        };
        let (mut parts, mut together, mut sized, mut covered, mut ctrl_parts, mut ctrl_together) = (0, 0, 0, 0, 0, 0);
        for c in &fronts {
            for s in 0..2 {
                let front = c.front(s);
                let cut = c.sectors(s, size);
                let mut all: Vec<usize> = cut.iter().flatten().copied().collect();
                all.sort_unstable();
                let mut want = front.clone();
                want.sort_unstable();
                covered += (all == want) as usize;
                for part in &cut {
                    parts += 1;
                    together += whole(c, part) as usize;
                    sized += (part.len() >= size / 2 && part.len() <= size * 3 / 2 + 1) as usize;
                }
                // the control: the same number of pieces, by the provinces' numbers
                let k = cut.len().max(1);
                for i in 0..k {
                    let part = &front[i * front.len() / k..(i + 1) * front.len() / k];
                    if !part.is_empty() {
                        ctrl_parts += 1;
                        ctrl_together += whole(c, part) as usize;
                    }
                }
            }
        }
        let share = together as f32 / parts.max(1) as f32;
        let ctrl = ctrl_together as f32 / ctrl_parts.max(1) as f32;
        checks.push((
            "участки фронта: вдоль линии, без дыр".into(),
            covered == fronts.len() * 2 && share >= 0.9 && ctrl < share - 0.2,
            format!(
                "фронтов {} (начало и конец войн), весь фронт поделен: {}; участков {parts}, цельных {:.0}%, размером около {size}: {:.0}%; контроль (по номерам) — цельных {:.0}%",
                fronts.len() * 2, covered, share * 100.0, sized as f32 / parts.max(1) as f32 * 100.0, ctrl * 100.0
            ),
        ));
    }
    let first = share_won(&even, 0);
    let mut lengths: Vec<u32> = even.iter().map(|(c, _)| c.month).collect();
    lengths.sort_unstable();
    let ended = even.iter().filter(|(c, _)| c.fallen.is_some()).count();
    checks.push((
        "контроль: у близнецов первая сторона — орёл или решка".into(),
        (0.35..=0.65).contains(&first),
        format!("{} войн (8 карт, обе стороны каждой, по {} везений): первая сторона побеждает в {:.0}%", even.len(), LUCKS.len(), first * 100.0),
    ));
    // pockets give up: the land behind the front is no sieve of islands — a province cut off
    // from its side's capital is rare (control: the old rule, pockets melting slowly and giving up
    // only to a battle of their own)
    {
        let sieve = |cs: &[(Campaign, bool)]| -> f32 {
            let (cut, all) = cs.iter().fold((0.0f32, 0.0f32), |(a, b), (c, _)| (a + c.cut_off[0] + c.cut_off[1], b + c.month as f32 * c.owner.len() as f32));
            cut / all.max(1.0)
        };
        let old = run([1.0, 1.0], [0.5, 0.5], true, true);
        let (now, then) = (sieve(&even), sieve(&old));
        let len = |cs: &[(Campaign, bool)]| cs.iter().map(|(c, _)| c.month as f32).sum::<f32>() / cs.len() as f32;
        let gave: u32 = even.iter().map(|(c, _)| c.surrendered[0] + c.surrendered[1]).sum();
        checks.push((
            "окружённые сдаются: за фронтом не решето".into(),
            now * 2.0 < then,
            format!(
                "отрезанных от своей столицы провинций в среднем {:.2}% театра (по старому правилу — {:.2}%); сдалось окружённых провинций {:.1} за войну; война длится {:.0} мес. (по старому {:.0})",
                now * 100.0,
                then * 100.0,
                gave as f32 / even.len() as f32,
                len(&even),
                len(&old)
            ),
        ));
    }
    if std::env::var("BYMAP").is_ok() {
        // who wins on each map, each way round: first side's wins of the lucks
        for (m, t) in maps.iter().enumerate() {
            let line: Vec<String> = [false, true]
                .iter()
                .map(|&sw| {
                    let mine: Vec<&(Campaign, bool)> = even.iter().filter(|(c, w)| *w == sw && c.th.countries == if sw { [t.pair.1, t.pair.0] } else { [t.pair.0, t.pair.1] }).collect();
                    let w = mine.iter().filter(|(c, _)| winner(c) == Some(0)).count();
                    format!("{} первой: {}/{}", if sw { "вторая" } else { "первая" }, w, mine.len())
                })
                .collect();
            println!("    карта {}: {}", m + 1, line.join(", "));
        }
    }
    // the battles of all the wars of equals
    let all: Vec<&strategy::military::front::Fought> = even.iter().flat_map(|(c, _)| c.fought.iter()).collect();
    let rate = |f: &dyn Fn(&&strategy::military::front::Fought) -> bool| {
        let pick: Vec<&&strategy::military::front::Fought> = all.iter().filter(|x| f(x)).collect();
        (pick.iter().filter(|x| x.won).count() as f32 / pick.len().max(1) as f32, pick.len())
    };
    // what an attack needed: the staff attacks only where its games promise success, and takes
    // the ground and supply into account, so the share of attacks that succeed comes out the
    // same everywhere; the force it had to bring does not
    let need = |f: &dyn Fn(&&strategy::military::front::Fought) -> bool| {
        let mut r: Vec<f32> = all.iter().filter(|x| f(x) && x.won).map(|x| x.ratio).collect();
        r.sort_by(|a, b| a.total_cmp(b));
        (r.get(r.len() / 2).copied().unwrap_or(0.0), r.len())
    };
    if std::env::var("BANDS").is_ok() {
        // success by ground at the same odds of force
        for (lo, hi) in [(0.5f32, 1.0f32), (1.0, 1.5), (1.5, 2.0), (2.0, 3.0), (3.0, 100.0)] {
            let row = |ts: &[Terrain]| {
                let b: Vec<&&strategy::military::front::Fought> = all.iter().filter(|x| x.ratio >= lo && x.ratio < hi && ts.contains(&x.terrain)).collect();
                format!("{:.0}% из {}", b.iter().filter(|x| x.won).count() as f32 / b.len().max(1) as f32 * 100.0, b.len())
            };
            println!("    перевес ×{lo}–{hi}: в поле {}, в горах и холмах {}", row(&[Terrain::Plains, Terrain::Desert]), row(&[Terrain::Mountains, Terrain::Hills]));
        }
        let mut sizes: Vec<f32> = all.iter().filter(|x| matches!(x.terrain, Terrain::Plains)).map(|x| x.ratio).collect();
        sizes.sort_by(|a, b| a.total_cmp(b));
        let _ = sizes;
    }
    // how often a province of the front changes hands in a month, by its ground
    let turn = |ts: &[Terrain]| {
        let (mut t, mut e) = (0.0f32, 0.0f32);
        for (c, _) in &even {
            for &x in ts {
                t += c.taken[x as usize];
                e += c.exposed[x as usize];
            }
        }
        (t / e.max(1.0), e)
    };
    let (open, e_open) = turn(&[Terrain::Plains, Terrain::Desert]);
    let (rough, e_rough) = turn(&[Terrain::Mountains, Terrain::Hills]);
    let (open_need, _) = need(&|x| matches!(x.terrain, Terrain::Plains | Terrain::Desert));
    let (rough_need, _) = need(&|x| matches!(x.terrain, Terrain::Mountains | Terrain::Hills));
    // the same force achieves less in rough country: attacks with between one and two times
    // the defenders' worth. (How often a province of the front changes hands is shown, not
    // checked: the staff brings more to the mountains, or strikes elsewhere, and the front there
    // moves only a fifth slower.)
    let band = |ts: &[Terrain]| {
        let b: Vec<&&strategy::military::front::Fought> = all.iter().filter(|x| x.ratio >= 1.0 && x.ratio < 2.0 && ts.contains(&x.terrain)).collect();
        (b.iter().filter(|x| x.won).count() as f32 / b.len().max(1) as f32, b.len())
    };
    let (b_open, nb_open) = band(&[Terrain::Plains, Terrain::Desert]);
    let (b_rough, nb_rough) = band(&[Terrain::Mountains, Terrain::Hills]);
    checks.push((
        "в горах и холмах та же сила добивается меньшего".into(),
        b_rough + 0.1 <= b_open,
        format!(
            "атаки с перевесом ×1–2 удаются: в поле {:.0}% (из {nb_open}), в горах и холмах {:.0}% (из {nb_rough}); перевес удачного наступления ×{open_need:.1} и ×{rough_need:.1}; провинция фронта переходит из рук в руки за месяц в {:.0}% и {:.0}% случаев",
            b_open * 100.0,
            b_rough * 100.0,
            open * 100.0,
            rough * 100.0
        ),
    ));
    let _ = (e_open, e_rough);
    let (fed, n_fed) = need(&|x| x.supply[0] >= 0.99);
    let (hungry, n_hungry) = need(&|x| x.supply[0] <= 0.6);
    checks.push((
        "далеко от снабжения наступление выдыхается".into(),
        hungry >= 1.2 * fed,
        format!("перевес удачного наступления: при полном снабжении ×{fed:.1} (из {n_fed}), при скудном ×{hungry:.1} (из {n_hungry})"),
    ));
    let mut ratios: Vec<f32> = all.iter().map(|x| x.ratio).collect();
    ratios.sort_by(|a, b| a.total_cmp(b));
    let median = ratios.get(ratios.len() / 2).copied().unwrap_or(0.0);
    let (big, n_big) = rate(&|x| x.ratio >= 2.0);
    let (small, n_small) = rate(&|x| x.ratio < 1.0);
    checks.push((
        "бьют там, где сильнее, и перевес помогает".into(),
        median >= 1.5 && big >= small + 0.1,
        format!(
            "перевес наступающего (по ценности войск) в середине ряда ×{median:.1}; удачных при перевесе ≥×2 {:.0}% (из {n_big}), при меньшинстве {:.0}% (из {n_small})",
            big * 100.0,
            small * 100.0
        ),
    ));
    // the staff learns how far its war games flatter its attacks: early in a war its attacks
    // succeed less often than promised, later as often
    let promise = |f: &dyn Fn(&&strategy::military::front::Fought) -> bool| {
        let pick: Vec<&&strategy::military::front::Fought> = all.iter().filter(|x| f(x)).collect();
        let p = pick.iter().map(|x| x.odds).sum::<f32>() / pick.len().max(1) as f32;
        let w = pick.iter().filter(|x| x.won).count() as f32 / pick.len().max(1) as f32;
        (p, w, pick.len())
    };
    let (p0, w0, n0) = promise(&|x| x.month <= 6);
    let (p1, w1, n1) = promise(&|x| x.month > 18);
    if std::env::var("CALIB").is_ok() {
        // late battles against pockets and against the living front, apart
        let (pc, wc, nc) = promise(&|x| x.month > 18 && x.cut);
        let (pf, wf, nf) = promise(&|x| x.month > 18 && !x.cut);
        let ws: Vec<f32> = even.iter().flat_map(|(c, _)| [c.sides[0].wariness, c.sides[1].wariness]).collect();
        println!("  сведения: настороженность в конце войн в среднем {:+.2}, у потолка (2) {:.0}%", ws.iter().sum::<f32>() / ws.len().max(1) as f32, ws.iter().filter(|&&w| w >= 1.99).count() as f32 / ws.len().max(1) as f32 * 100.0);
        let failed: Vec<&&strategy::military::front::Fought> = all.iter().filter(|x| x.month > 18 && !x.cut && !x.won).collect();
        let wonl: Vec<&&strategy::military::front::Fought> = all.iter().filter(|x| x.month > 18 && !x.cut && x.won).collect();
        let mid = |v: &[&&strategy::military::front::Fought], f: &dyn Fn(&strategy::military::front::Fought) -> f32| -> f32 {
            let mut r: Vec<f32> = v.iter().map(|x| f(x)).filter(|x| x.is_finite()).collect();
            r.sort_by(|a, b| a.total_cmp(b));
            r.get(r.len() / 2).copied().unwrap_or(0.0)
        };
        for (name, v) in [("неудачные", &failed), ("удачные", &wonl)] {
            println!(
                "  сведения: {name} поздние удары по живому фронту ({}): в середине ряда атакующих к бою ×{:.2} от плана, защитников ×{:.2} от плана; перевес при плане ×{:.2}, в бою ×{:.2}",
                v.len(),
                mid(v, &|x| x.plan[2] / x.plan[0].max(1e-9)),
                mid(v, &|x| x.plan[3] / x.plan[1].max(1e-9)),
                mid(v, &|x| x.plan[0] / x.plan[1].max(1e-9)),
                mid(v, &|x| x.plan[2] / x.plan[3].max(1e-9))
            );
        }
        println!("  сведения: после полутора лет против окружённых обещано {:.0}%, удалось {:.0}% (из {nc}); против живого фронта {:.0}% и {:.0}% (из {nf})", pc * 100.0, wc * 100.0, pf * 100.0, wf * 100.0);
    }
    checks.push((
        "генштаб учится верить своим играм ровно настолько, насколько они верны".into(),
        (p1 - w1).abs() <= 0.1 && (p1 - w1).abs() < (p0 - w0).abs(),
        format!(
            "первые полгода: обещано {:.0}%, удалось {:.0}% (из {n0}); после полутора лет: обещано {:.0}%, удалось {:.0}% (из {n1})",
            p0 * 100.0,
            w0 * 100.0,
            p1 * 100.0,
            w1 * 100.0
        ),
    ));
    let rich = run([2.0, 1.0], [0.5, 0.5], true, false);
    let r = share_won(&rich, 0);
    checks.push(("вдвое богаче — побеждает".into(), r >= 0.8, format!("сторона с вдвое большим бюджетом побеждает в {:.0}% войн", r * 100.0)));
    let blind = run([1.0, 1.0], [3.0, 0.5], true, false);
    let b = share_won(&blind, 0);
    checks.push((
        "слепой проигрывает".into(),
        b <= 0.4,
        format!("близнец, видящий врага в густом тумане, побеждает в {:.0}% войн (видящий ясно — в {:.0}%)", b * 100.0, (1.0 - b) * 100.0),
    ));
    println!(
        "  войны равных: длились {} мес. в середине ряда (от {} до {}), капитуляцией кончились {} из {}",
        lengths[lengths.len() / 2],
        lengths[0],
        lengths[lengths.len() - 1],
        ended,
        even.len()
    );
    println!("\nпроверки ({:.0} с):", started.elapsed().as_secs_f32());
    let mut ok = true;
    for (name, pass, detail) in &checks {
        ok &= pass;
        println!("  [{}] {:<44} {}", if *pass { "OK" } else { "FAIL" }, name, detail);
    }
    ok
}
