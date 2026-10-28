//! The checks of the war of divisions (`checks corps --checks`): wars on many maps, each check with
//! its control.

use super::*;

/// One map's pair, ready to fight.
struct Map {
    world: World,
    pol: Politics,
    pair: (usize, usize),
}

/// A war of the checks, as it ended, and what was seen in it.
struct War {
    c: Campaign,
    book: Book,
    swapped: bool,
}

/// What happened to the checks' wars: every map, both ways round, so many lucks each.
fn wars(pack: &Pack, maps: &[Map], lucks: &[u64], months: u32, money: [f32; 2], twins: bool, set: impl Fn(&mut Campaign) + Sync) -> Vec<War> {
    use rayon::prelude::*;
    let jobs: Vec<(usize, bool, u64)> = (0..maps.len()).flat_map(|m| [false, true].into_iter().flat_map(move |sw| lucks.iter().copied().map(move |k| (m, sw, k)))).collect();
    jobs.par_iter()
        .map(|&(m, swapped, k)| {
            let t = &maps[m];
            let pair = if swapped { (t.pair.1, t.pair.0) } else { t.pair };
            let mut c = campaign_of(pack, &t.world, &t.pol, pair, money, hash64(k * 1000 + m as u64), twins);
            set(&mut c);
            let mut book = Book::default();
            live(pack, &mut c, months, &mut book, |_, _, _| {});
            War { c, book, swapped }
        })
        .collect()
}

/// Share of wars side 0 won (a draw counts a half).
fn first_won(ws: &[War]) -> f32 {
    ws.iter().map(|w| match winner(&w.c) { Some(0) => 1.0, None => 0.5, _ => 0.0 }).sum::<f32>() / ws.len().max(1) as f32
}

/// Share of the front of the `sides` (provinces their capital reaches) with a division of their own
/// standing there or a step away from it; and with their divisions worth at least half the enemy's
/// across.
fn held_front(ws: &[War], sides: &[usize]) -> (f32, f32) {
    let (mut a, mut m, mut b) = (0.0, 0.0, 0.0);
    for w in ws {
        for &s in sides {
            a += w.book.covered[s];
            m += w.book.matched[s];
            b += w.book.front[s];
        }
    }
    (a / f32::max(b, 1.0), m / f32::max(b, 1.0))
}

/// Side `s` with `share` of the equipment its norms give the kinds that have norms: its
/// divisions, its stores, and what its factories make (they make as many, of lighter kit).
fn starve_kit(pack: &Pack, c: &mut Campaign, s: usize, share: f32) {
    let nu = pack.units.len();
    let side = &mut c.sides[s];
    side.norm_free = false;
    side.norm = (0..nu).map(|u| if pack.units[u].norms { share } else { 1.0 }).collect();
    let k = c.corps.as_mut().unwrap();
    for sh in k.shapes[s].iter_mut() {
        sh.norm = (0..nu).map(|u| if pack.units[u].norms { share } else { 1.0 }).collect();
    }
    for d in k.divs.iter_mut().filter(|d| d.side == s) {
        for u in 0..nu {
            if pack.units[u].norms {
                d.kit[u] *= share;
            }
        }
    }
    for u in 0..nu {
        if pack.units[u].norms {
            k.kit[s][u] *= share;
        }
    }
}

/// The mean norm side `s`'s staff gives the kinds with norms, by its plan.
fn mean_norm(pack: &Pack, c: &Campaign, s: usize) -> f32 {
    let side = &c.sides[s];
    let (mut w, mut n) = (0.0, 0.0);
    for u in 0..pack.units.len() {
        if pack.units[u].norms {
            w += side.plan[u];
            n += side.plan[u] * side.norm_of(u);
        }
    }
    if w > 0.0 {
        n / w
    } else {
        1.0
    }
}

/// The companies' check: in each war, side 0's divisions of odd number get every company of the
/// pack (a copy of their template), the even ones none; four months of war, day by day — how dug
/// in those standing are, men lost for good per day of fighting, equipment left behind per day of
/// fighting: with against without. `spoiled`: companies that give nothing.
/// The half (0: with companies, 1: without) division number `no` falls into, by chance.
fn half(no: u32) -> usize {
    (hash64(no as u64 ^ 0xC0A1) % 2) as usize
}

fn companies(pack: &Pack, maps: &[Map], lucks: &[u64], spoiled: bool) -> ([f32; 3], [f32; 3]) {
    use rayon::prelude::*;
    let mut p2;
    let pack = if spoiled {
        p2 = Pack::load(&PACK_PATH.read().unwrap()).unwrap();
        for x in p2.support.iter_mut() {
            (x.dig, x.rough, x.see, x.org, x.saved, x.eat, x.fire, x.salvage) = (1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0);
        }
        &p2
    } else {
        pack
    };
    let jobs: Vec<(usize, u64)> = (0..maps.len()).flat_map(|m| lucks.iter().map(move |&k| (m, k))).collect();
    let sums: Vec<[[f64; 4]; 2]> = jobs
        .par_iter()
        .map(|&(m, k)| {
            let t = &maps[m];
            let mut c = campaign_of(pack, &t.world, &t.pol, t.pair, [1.0, 1.0], hash64(k * 1000 + m as u64), true);
            let all: Vec<usize> = (0..pack.support.len()).collect();
            let firsts: Vec<u32>;
            {
                let k = c.corps.as_mut().unwrap();
                // (a copy of each template with every company, and one with none)
                let n = k.shapes[0].len();
                for (tag, sup) in [(" (с ротами)", all.clone()), (" (без рот)", Vec::new())] {
                    for i in 0..n {
                        let mut sh = k.shapes[0][i].clone();
                        sh.name += tag;
                        sh.support = sup.clone();
                        k.shapes[0].push(sh);
                        k.wants[0].push(0.0);
                    }
                }
                // (halves by chance: numbers go round a stretch's provinces in turn — with a stretch
                // of an even length the odd ones all stood on the same provinces, and the control saw
                // the ground, not the companies)
                for d in k.divs.iter_mut().filter(|d| d.side == 0) {
                    d.shape += if half(d.no) == 0 { n } else { 2 * n };
                }
                firsts = k.divs.iter().filter(|d| d.side == 0).map(|d| d.id).collect();
            }
            // [with, without] × [dug sum, standing days, fight days, dead, left]
            let mut sum = [[0.0f64; 4]; 2];
            for _ in 0..120 {
                c.day(pack);
                let k = c.corps.as_ref().unwrap();
                for d in k.divs.iter().filter(|d| d.side == 0 && firsts.contains(&d.id)) {
                    let w = half(d.no);
                    if d.way.is_empty() && !d.busy && d.rest <= 0.0 {
                        sum[w][0] += d.dug as f64;
                        sum[w][1] += 1.0;
                    }
                }
                let day = k.day;
                if c.fallen.is_some() {
                    break;
                }
                if day % 30 == 0 {
                    c.step(pack);
                }
            }
            let k = c.corps.as_ref().unwrap();
            let mut out = [[0.0f64; 4]; 2];
            for w in 0..2 {
                out[w][0] = sum[w][0];
                out[w][1] = sum[w][1];
            }
            // (fights: men lost for good and equipment left behind, per day in a fight)
            let mut fd = [0.0f64; 2];
            let mut dead = [0.0f64; 2];
            let mut left = [0.0f64; 2];
            for d in k.divs.iter().filter(|d| d.side == 0 && firsts.contains(&d.id)) {
                let w = half(d.no);
                fd[w] += d.days as f64;
                dead[w] += d.dead as f64;
                left[w] += d.left as f64;
            }
            for w in 0..2 {
                out[w][2] = dead[w] / fd[w].max(1.0);
                out[w][3] = left[w] / fd[w].max(1.0);
            }
            out
        })
        .collect();
    let mut tot = [[0.0f64; 4]; 2];
    for x in &sums {
        for w in 0..2 {
            for j in 0..4 {
                tot[w][j] += x[w][j];
            }
        }
    }
    let ratio = |a: f64, b: f64| if b > 0.0 { (a / b) as f32 } else { 1.0 };
    let whole = [ratio(tot[0][0] / tot[0][1].max(1.0), tot[1][0] / tot[1][1].max(1.0)), ratio(tot[0][2], tot[1][2]), ratio(tot[0][3], tot[1][3])];
    // (the error of each ratio's logarithm: by map, its lucks sharing the map's war)
    let per_job = |x: &[[f64; 4]; 2]| [ratio(x[0][0] / x[0][1].max(1.0), x[1][0] / x[1][1].max(1.0)), ratio(x[0][2], x[1][2]), ratio(x[0][3], x[1][3])];
    let mut err = [0.0f32; 3];
    for (j, e) in err.iter_mut().enumerate() {
        let by_map: Vec<f32> = (0..maps.len())
            .map(|m| {
                let v: Vec<f32> = (0..lucks.len()).map(|l| per_job(&sums[m * lucks.len() + l])[j].max(1e-6).ln()).collect();
                v.iter().sum::<f32>() / v.len().max(1) as f32
            })
            .collect();
        let mean = by_map.iter().sum::<f32>() / by_map.len().max(1) as f32;
        *e = (by_map.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / (by_map.len().max(2) - 1) as f32 / by_map.len().max(1) as f32).sqrt();
    }
    (whole, err)
}

/// The pack's file (the spoiled control reads it anew).
pub(super) static PACK_PATH: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

pub(super) fn checks(pack: &Pack, months: u32) -> bool {
    let t0 = std::time::Instant::now();
    let maps: Vec<Map> = (1..=6u64)
        .filter_map(|seed| {
            let world = map::generate(&Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 }).0;
            let pol = countries::generate(&world, seed, 30, &pack);
            pick(&world, &pol).map(|pair| Map { world, pol, pair })
        })
        .collect();
    let lucks = [11u64, 12, 13];
    let mut checks: Vec<(String, bool, String)> = Vec::new();
    let even = wars(pack, &maps, &lucks, months, [1.0, 1.0], true, |_| {});
    let strong = wars(pack, &maps, &lucks, months, [2.0, 1.0], true, |_| {});
    let idle = wars(pack, &maps, &lucks, months, [1.0, 1.0], true, |c| c.corps.as_mut().unwrap().idle = [true, false]);
    let fed = wars(pack, &maps, &lucks, months, [1.0, 1.0], true, |c| c.corps.as_mut().unwrap().fed = true);
    if std::env::var("WARS").is_ok() {
        for (i, w) in strong.iter().enumerate() {
            let p = w.c.people();
            let k = w.c.corps.as_ref().unwrap();
            let n = |s: usize| k.divs.iter().filter(|d| d.side == s).count();
            println!("    война {i} (наоборот: {}): своих людей {:.0}%/{:.0}%, провинций {:?}, дивизий {}/{}, капитуляция {:?}, боёв {}", w.swapped, p[0] * 100.0, p[1] * 100.0, w.c.held(), n(0), n(1), w.c.fallen, w.c.fought.len());
        }
    }
    // 1. twins: the first side wins as often as not (a toss of a coin); twice the money wins
    let (e, s2) = (first_won(&even), first_won(&strong));
    checks.push((
        "близнецы — орёл или решка; вдвое богаче — побеждает".into(),
        (0.3..=0.7).contains(&e) && s2 >= 0.75,
        format!("{} войн по {months} мес (6 карт, обе стороны, 3 везения): первая сторона побеждает в {:.0}%; с вдвое большими деньгами — в {:.0}%", even.len(), e * 100.0, s2 * 100.0),
    ));
    // 2. the front has no holes: on nearly every province of the front its capital reaches a
    // division of its own stands, or is a step from it on its way there; the control — one side's
    // generals asleep, its divisions where they stood when the war began — leaves that side's line
    // full of holes as the front moves. (A control that spoiled both sides was blind: spreading
    // both sides' divisions over all their land, hundreds of divisions still left one nearly
    // everywhere — 95% against 88–93%; nobody moving at all, the front never moved either.)
    // Whether the line is as strong as the enemy across it is shown, not checked: 72% of it is
    // worth at least half the enemy's, against 62% for the sleeping side.
    let ((h, m), (hi, mi)) = (held_front(&even, &[0, 1]), held_front(&idle, &[0]));
    checks.push((
        "фронт без дыр: на провинциях линии стоят дивизии".into(),
        h >= 0.9 && hi < h - 0.1,
        format!(
            "прикрыто {:.0}% провинций фронта (стоит дивизия или в шаге от неё), своих не меньше половины врага напротив — на {:.0}%; контроль — у одной стороны генералы спят: у неё {:.0}% ({:.0}%)",
            h * 100.0,
            m * 100.0,
            hi * 100.0,
            mi * 100.0
        ),
    ));
    // 3. fights go on for days, and strength decides them: the more the attackers outweigh the
    // defenders, the more often they win
    let fought: Vec<&strategy::military::front::Fought> = even.iter().chain(&strong).flat_map(|w| w.c.fought.iter()).collect();
    let mut days: Vec<u32> = even.iter().chain(&strong).flat_map(|w| w.book.days.iter().copied()).collect();
    days.sort_unstable();
    let median = days.get(days.len() / 2).copied().unwrap_or(0);
    let bin = |lo: f32, hi: f32| {
        let f: Vec<_> = fought.iter().filter(|f| f.ratio >= lo && f.ratio < hi).collect();
        (f.iter().filter(|f| f.won).count() as f32 / f.len().max(1) as f32, f.len())
    };
    if std::env::var("DAYS").is_ok() {
        for (lo, hi) in [(0.0, 0.7), (0.7, 1.5), (1.5, 3.0), (3.0, 1e9)] {
            let mut d: Vec<u32> = fought.iter().filter(|f| f.ratio >= lo && f.ratio < hi).map(|f| f.days).collect();
            d.sort_unstable();
            let q = |k: usize| d.get(d.len() * k / 10).copied().unwrap_or(0);
            println!("    сила {lo}..{hi}: боёв {}, дни 10%/50%/90%: {}/{}/{}; первый день {:.0}%", d.len(), q(1), q(5), q(9), 100.0 * d.iter().filter(|&&x| x <= 1).count() as f32 / d.len().max(1) as f32);
        }
    }
    let (weak, nw) = bin(0.0, 0.7);
    let (equal, ne) = bin(0.7, 1.5);
    let (big, nb) = bin(3.0, f32::INFINITY);
    // (a fight of equals goes on for days; a defender three times weaker is mostly overrun in a
    // day — that is the whole median, so it is measured on equals)
    let mut eq_days: Vec<u32> = fought.iter().filter(|f| f.ratio >= 0.7 && f.ratio < 1.5).map(|f| f.days).collect();
    eq_days.sort_unstable();
    let eq_mid = eq_days.get(eq_days.len() / 2).copied().unwrap_or(0);
    let eq_long = eq_days.get(eq_days.len() * 9 / 10).copied().unwrap_or(0);
    checks.push((
        "бои идут днями, исход решает сила".into(),
        (2..=20).contains(&eq_mid) && eq_long >= 4 && big > equal + 0.15 && equal > weak + 0.1 && nw >= 10 && nb >= 10,
        format!(
            "боёв {}; бой равных длится {eq_mid} дн. (каждый десятый — {eq_long}+), всех — {median}; наступавшие слабее ×0,7 побеждают в {:.0}% ({nw}), равные — в {:.0}% ({ne}), сильнее ×3 — в {:.0}% ({nb})",
            fought.len(),
            weak * 100.0,
            equal * 100.0,
            big * 100.0
        ),
    ));
    // 4. the cut off melt away (no supply), the others do not; the control — nobody hungry — melts
    // nobody
    let melt = |ws: &[War]| {
        let (cl, cd, fl, fd) = ws.iter().fold((0.0, 0.0, 0.0, 0.0), |a, w| (a.0 + w.book.cut_loss, a.1 + w.book.cut_days, a.2 + w.book.free_loss, a.3 + w.book.free_days));
        (cl / f32::max(cd, 1.0), fl / f32::max(fd, 1.0), cd)
    };
    let (cut, free, cd) = melt(&even);
    let (cut_f, _, _) = melt(&fed);
    checks.push((
        "окружённые без снабжения тают".into(),
        cd >= 100.0 && cut > 3.0 * free && cut > 3.0 * cut_f,
        format!("окружённые теряют {:.2}% численности в день вне боя ({cd:.0} дивизие-дней), прочие {:.3}%; контроль — все сыты: окружённые {:.3}%", cut * 100.0, free * 100.0, cut_f * 100.0),
    ));
    // 5. the same war twice is the same war; another luck is another
    {
        let t = &maps[0];
        let one = |k: u64| {
            let mut c = campaign_of(pack, &t.world, &t.pol, t.pair, [1.0, 1.0], k, true);
            let mut b = Book::default();
            live(pack, &mut c, 6, &mut b, |_, _, _| {});
            let k = c.corps.as_ref().unwrap();
            (c.owner.clone(), k.divs.iter().map(|d| (d.id, d.at, (d.org * 1e4) as i64, (d.has.iter().sum::<f32>() * 1e4) as i64)).collect::<Vec<_>>(), c.fought.len())
        };
        let (a, b, x) = (one(5), one(5), one(6));
        checks.push(("одна и та же война — до бита".into(), a == b && a != x, format!("повтор совпал: {}; другое везение отличается: {}", a == b, a != x)));
    }
    // 6. the kit decides: the same men with half the equipment their norm gives (the kinds that
    // have norms) do worse than their twins in the same wars (the same maps, sides and lucks: a
    // pair each) — by the share of their own people each side holds at the end. (Who won the war,
    // yes or no, over 36 wars spread ±12 points: a real fall of 14–16 lay at the threshold.)
    let light = wars(pack, &maps, &lucks, months, [1.0, 1.0], true, |c| starve_kit(pack, c, 0, 0.5));
    let (lw, ew) = (first_won(&light), first_won(&even));
    let held = |w: &War| {
        let p = w.c.people();
        p[0] - p[1]
    };
    let d: Vec<f32> = light.iter().zip(&even).map(|(a, b)| held(a) - held(b)).collect();
    let mean = d.iter().sum::<f32>() / d.len().max(1) as f32;
    let sd = (d.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / (d.len().max(2) - 1) as f32).sqrt();
    let se = sd / (d.len().max(1) as f32).sqrt();
    let worse = d.iter().filter(|&&x| x < 0.0).count();
    checks.push((
        "оснащение решает: те же люди с половиной снаряжения воюют хуже".into(),
        mean < 0.0 && -mean >= 3.0 * se,
        format!(
            "у первой стороны половина снаряжения пехоты (и заводы дают ей столько же): в тех же войнах она удерживает своих людей против врага на {:.1} ± {:.1} пункта меньше, хуже в {worse} из {}; войн выиграла {:.0}% против {:.0}% у близнецов",
            -mean * 100.0,
            se * 100.0,
            d.len(),
            lw * 100.0,
            ew * 100.0
        ),
    ));
    // 7. the size of a division is what the army can give each province of its front: a quarter
    // of the money, smaller divisions (the staff's first pattern, at the war's start — the
    // eras' armies are big against their fronts, and twice the money only finds both at the
    // ceiling); the control — the twins — about the same
    let poor = wars(pack, &maps, &lucks, 1, [0.25, 1.0], true, |_| {});
    let size = |w: &War, s: usize| w.c.corps.as_ref().unwrap().shapes[s].get(1).map_or(0.0, |x| x.battalions());
    let gap = |ws: &[War]| ws.iter().map(|w| size(w, 1) - size(w, 0)).sum::<f32>() / ws.len().max(1) as f32;
    let smaller = poor.iter().filter(|w| size(w, 0) < size(w, 1) || size(w, 1) <= strategy::military::corps::FEWEST).count() as f32 / poor.len().max(1) as f32;
    let (gp, ge) = (gap(&poor), gap(&even));
    let sizes: Vec<f32> = even.iter().flat_map(|w| [size(w, 0), size(w, 1)]).collect();
    let (lo, hi) = (sizes.iter().copied().fold(f32::INFINITY, f32::min), sizes.iter().copied().fold(0.0, f32::max));
    let poor_sizes: Vec<f32> = poor.iter().map(|w| size(w, 0)).collect();
    let (plo, phi) = (poor_sizes.iter().copied().fold(f32::INFINITY, f32::min), poor_sizes.iter().copied().fold(0.0, f32::max));
    checks.push((
        "размер дивизии — по армии и фронту: вчетверо беднее — дивизии мельче".into(),
        gp >= 2.0 && smaller >= 0.75 && ge.abs() < gp / 2.0,
        format!(
            "вчетверо беднее — дивизия генштаба мельче в среднем на {gp:.1} батальона ({plo:.0}–{phi:.0}), мельче (или у обоих у пола) в {:.0}% войн; близнецы — разница {ge:+.1}, размеры от {lo:.0} до {hi:.0}",
            smaller * 100.0
        ),
    ));
    // 8. the kit is chosen by prices: a staff that pays thrice for the goods its soldiers' kit
    // is made of chooses a lighter one than its neighbour; the control — the same prices — none
    // lighter more often than the other
    let kit_goods: Vec<usize> = (0..pack.goods.len()).filter(|&i| pack.units.iter().any(|u| u.norms && u.equip.iter().any(|e| e.0 == i))).collect();
    let mut dear = vec![1.0f32; pack.goods.len()];
    for &i in &kit_goods {
        dear[i] = 3.0;
    }
    // (twelve maps: the maps are what is independent here, and the industrial age's gap of about a
    // seventh of a norm needs that many for three errors)
    let more: Vec<Map> = (7..=12u64)
        .filter_map(|seed| {
            let world = map::generate(&Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 }).0;
            let pol = countries::generate(&world, seed, 30, &pack);
            pick(&world, &pol).map(|pair| Map { world, pol, pair })
        })
        .collect();
    let maps12: Vec<&Map> = maps.iter().chain(&more).collect();
    let wars12 = |set: &(dyn Fn(&mut Campaign) + Sync)| -> Vec<War> { maps12.iter().flat_map(|m| wars(pack, std::slice::from_ref(*m), &lucks, 6, [1.0, 1.0], true, set)).collect() };
    *PRICES.write().unwrap() = [Some(dear), None];
    let pricey = wars12(&|_| {});
    *PRICES.write().unwrap() = [None, None];
    let lighter = |ws: &[War]| {
        let mut n = 0.0;
        let mut by: std::collections::BTreeMap<(usize, bool), Vec<f32>> = Default::default();
        for (i, w) in ws.iter().enumerate() {
            let (a, b) = (mean_norm(pack, &w.c, 0), mean_norm(pack, &w.c, 1));
            n += if a < b - 1e-3 { 1.0 } else if a > b + 1e-3 { 0.0 } else { 0.5 };
            by.entry((i / (2 * lucks.len()), w.swapped)).or_default().push(b - a);
        }
        let per: Vec<f32> = by.values().map(|v| v.iter().sum::<f32>() / v.len() as f32).collect();
        let gap = per.iter().sum::<f32>() / per.len().max(1) as f32;
        let se = (per.iter().map(|x| (x - gap).powi(2)).sum::<f32>() / (per.len().max(2) - 1) as f32 / per.len().max(1) as f32).sqrt();
        (n / ws.len().max(1) as f32, gap, se)
    };
    let even6: Vec<War> = wars12(&|_| {});
    let ((lp, gp, gp_se), (le, ge_, ge_se)) = (lighter(&pricey), lighter(&even6));
    checks.push((
        "норма оснащения — по ценам: где снаряжение дорого, генштаб вооружает легче".into(),
        // (by the mean gap of the norms beyond the control's, with their errors over the independent
        // preparations — the share of 36 wars, ±8 points, sat on its threshold of 70%; it is shown)
        gp > 0.1 && gp - ge_ > 3.0 * (gp_se * gp_se + ge_se * ge_se).sqrt(),
        format!(
            "снаряжение пехоты втрое дороже — генштаб выбирает норму легче соседа в {:.0}% войн (средняя норма ниже на {gp:.2} ± {gp_se:.2}); контроль — цены те же: легче в {:.0}% (разница {ge_:+.2} ± {ge_se:.2})",
            lp * 100.0,
            le * 100.0
        ),
    ));
    // 9. the companies give what they promise: in the same wars, half the first side's divisions
    // with every company and half with none (by their numbers) — sappers dig in faster, hospitals
    // lose fewer men for good, repairmen leave less behind; the control — companies that give
    // nothing — no difference
    let ((with, _), (ctl, ctl_err)) = (companies(pack, &maps, &lucks, false), companies(pack, &maps, &lucks, true));
    let fmt = |v: &[f32; 3]| format!("окопы ×{:.2}, безвозвратные потери ×{:.2}, брошенное снаряжение ×{:.2}", v[0], v[1], v[2]);
    checks.push((
        "роты поддержки дают своё".into(),
        // (the control: no difference beyond its own noise — each ratio within three errors of 1;
        // a band of ±15% was too narrow for the equipment left behind, a rare thing)
        with[0] >= 1.2 && with[1] <= 0.85 && with[2] <= 0.75 && (0..3).all(|j| ctl[j].max(1e-6).ln().abs() <= 3.0 * ctl_err[j]),
        format!("дивизии с ротами против дивизий без них в тех же войнах: {}; контроль — роты без пользы: {} (ошибка ×{:.2}, ×{:.2}, ×{:.2})", fmt(&with), fmt(&ctl), ctl_err[0].exp(), ctl_err[1].exp(), ctl_err[2].exp()),
    ));
    // 10. recruits drill before they fight: new divisions stand behind the line till they are
    // trained (their generals leave them be), and a green army fights worse. The controls: recruits
    // free for orders at once (they go to the front), and the same wars with the first side's army
    // as good as its twin's but green (each war against itself with a trained army)
    let free = wars(pack, &maps, &lucks, months, [1.0, 1.0], true, |c| c.corps.as_mut().unwrap().drill_free = true);
    let green = wars(pack, &maps, &lucks, months, [1.0, 1.0], true, |c| {
        let k = c.corps.as_mut().unwrap();
        k.drill_free = true;
        for d in k.divs.iter_mut().filter(|d| d.side == 0) {
            d.drill = 0.0;
        }
    });
    // (the recruits at the war's end: how many of them their generals put on the line — standing on
    // a province of the front or on their way to one, not sent by the player, not falling back;
    // counted as "on their way" anywhere, recruits going back from the line to drill passed for sent)
    let sent = |ws: &[War]| -> (usize, f32) {
        let (mut n, mut away) = (0usize, 0usize);
        for w in ws {
            let k = w.c.corps.as_ref().unwrap();
            for s in 0..2 {
                for d in k.divs.iter().filter(|d| d.side == s && d.drill < 1.0) {
                    n += 1;
                    let to = d.way.last().copied().unwrap_or(d.at);
                    if w.c.th.nbr[to].iter().any(|&q| w.c.owner[q] != s) && !d.manual && d.rest <= 0.0 {
                        away += 1;
                    }
                }
            }
        }
        (n, away as f32 / n.max(1) as f32)
    };
    let ((rn, ra), (fn_, fa)) = (sent(&even), sent(&free));
    // (who those on the line are: alone there, holding a hole; on a side that lost most of its
    // people, with hardly any land behind the line to drill in)
    {
        let (mut on, mut alone, mut beaten) = (0usize, 0usize, 0usize);
        for w in &even {
            let k = w.c.corps.as_ref().unwrap();
            let people = w.c.people();
            for s in 0..2 {
                for d in k.divs.iter().filter(|d| d.side == s && d.drill < 1.0 && !d.manual && d.rest <= 0.0) {
                    let to = d.way.last().copied().unwrap_or(d.at);
                    if !w.c.th.nbr[to].iter().any(|&q| w.c.owner[q] != s) {
                        continue;
                    }
                    on += 1;
                    alone += !k.divs.iter().any(|x| x.side == s && x.drill >= 1.0 && x.at == to) as usize;
                    beaten += (people[s] < 0.6) as usize;
                }
            }
        }
        println!("  сведения: новобранцев на линии {on}: одни там (держат дыру) {alone}, у стороны, потерявшей больше 40% своих людей, {beaten}");
    }
    // (each map and side with its lucks: how much more of its people the green side lost)
    let mut by: std::collections::BTreeMap<(usize, bool), Vec<f32>> = Default::default();
    for (i, (a, b)) in green.iter().zip(&free).enumerate() {
        by.entry((i / (2 * lucks.len()), a.swapped)).or_default().push(a.c.people()[0] - b.c.people()[0]);
    }
    let per: Vec<f32> = by.values().map(|v| v.iter().sum::<f32>() / v.len() as f32).collect();
    let mean = per.iter().sum::<f32>() / per.len().max(1) as f32;
    let se = (per.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / (per.len().max(2) - 1) as f32 / per.len().max(1) as f32).sqrt();
    let (gw, fw) = (first_won(&green), first_won(&free));
    checks.push((
        "новобранцы учатся у знамени: генерал берёт их, только когда некем держать линию".into(),
        rn >= 10 && ra + 0.2 <= fa,
        format!("новобранцев к концу войн {rn}, генералы ведут {:.0}% (контроль — свободны для приказов сразу: {fn_}, ведут {:.0}%)", ra * 100.0, fa * 100.0),
    ));
    println!(
        "  сведения: армия новобранцев против обученного близнеца удержала своих людей меньше на {:.1} ± {:.1} п., побеждает в {:.0}% (обученная — в {:.0}%)",
        -mean * 100.0,
        se * 100.0,
        gw * 100.0,
        fw * 100.0
    );
    // (information) what the war games promised against what came of it; the pace of a day
    let promised: Vec<&&strategy::military::front::Fought> = fought.iter().filter(|f| f.odds != 0.5).collect();
    let p = promised.iter().map(|f| f.odds).sum::<f32>() / promised.len().max(1) as f32;
    let won = promised.iter().filter(|f| f.won).count() as f32 / promised.len().max(1) as f32;
    let ms = even.iter().map(|w| w.book.ms).sum::<f32>() / even.iter().map(|w| w.book.daysn).sum::<f32>().max(1.0);
    let ended = even.iter().filter(|w| w.c.fallen.is_some()).count();
    let swapped = even.iter().filter(|w| w.swapped).count();
    println!(
        "  сведения: удары генералов — обещано {:.0}%, взято {:.0}% ({} ударов); день войны {:.2} мс; капитуляций у близнецов {ended} из {} ({swapped} войн наоборот)",
        p * 100.0,
        won * 100.0,
        promised.len(),
        ms,
        even.len()
    );
    let mut ok = true;
    for (name, pass, why) in &checks {
        println!("  [{}] {name}: {why}", if *pass { "OK" } else { "FAIL" });
        ok &= *pass;
    }
    println!("  проверки дивизий: {}/{} за {:.0} с", checks.iter().filter(|c| c.1).count(), checks.len(), t0.elapsed().as_secs_f32());
    ok
}
