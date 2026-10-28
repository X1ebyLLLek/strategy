//! The economy's and the order's self-checks (`play --econtest [--pack …] [--seed N] [--control]`,
//! 27th session): each lever of the state's household is tried against a twin that differs only by
//! it, from the same world — relief feeds the poor, schools make readers and the learned, doctors
//! make fewer die, cheap credit swells the loans, the money and the prices, bonds move the owners'
//! money into the treasury and pay it back, serfdom keeps people on the land, a tax on the rich
//! angers the industrialists through their own books. `--control` switches the new mechanisms off
//! (`country::econ_spoiled`): every check must fail then, or it does not measure what it says.

use std::time::Instant;
use strategy::pack::Pack;
use strategy::world::polity;

use crate::country::civil::{HEALTH, RELIEF, SCHOOLS};
use crate::Game;

/// Months of each side's life with the player's lever set (the bots keep their own).
fn run(base: &Game, form: &[(&str, &str)], months: u32, before: &dyn Fn(&mut Game)) -> Game {
    let mut g = base.clone();
    before(&mut g);
    if !form.is_empty() {
        let f: Vec<(String, String)> = form.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        g.apply(&f);
    }
    for _ in 0..months {
        g.advance();
        while let Some(id) = g.ev.waiting.first().map(|e| e.id) {
            g.answer(id, 0);
        }
    }
    g
}

/// The player's lot of force `b` (what its people consumed a head, at the prices of the start).
fn lot(g: &Game, b: usize) -> f32 {
    g.lots(g.me)[b]
}

pub fn econtest(pack: Pack, seed: u64) -> bool {
    let started = Instant::now();
    let spoil = crate::country::econ_spoiled();
    let base = Game::new(pack, seed, 0);
    let me = base.me;
    let none = |_: &mut Game| {};
    let mut checks: Vec<(String, bool, String)> = Vec::new();
    let war = |a: &Game, b: &Game| if a.h.at_war || b.h.at_war { " (шла война)" } else { "" };
    // (ECONDBG=1: each force's mood, thing by thing, after a year of the custom; the land's share
    // year by year under serfdom and free labour)
    if std::env::var_os("ECONDBG2").is_some() {
        for rate in ["0", "2"] {
            let mut g = base.clone();
            g.apply(&[("fx_rate".to_string(), rate.to_string())]);
            for m in 0..36 {
                g.advance();
                while let Some(id) = g.ev.waiting.first().map(|e| e.id) {
                    g.answer(id, 0);
                }
                if m % 6 == 5 {
                    let n = g.h.home(me);
                    let money: Vec<String> = n.money.iter().map(|x| format!("{x:.1}")).collect();
                    println!(
                        "  ставка {rate} мес {}: займы {:.1} (дали {:.2}, вернули {:.2}), выпуск {:.1}, деньги групп [{}], казна {:.1}, касса {:.1}, всего {:.1}; ввоз {:.2} вывоз {:.2}; утекло капитала {:.1}; уверенность {:.2}, invest_k {:.2}",
                        m + 1,
                        g.credit.loans[me], g.credit.lent[me], g.credit.repaid[me], n.gdp,
                        money.join(" "), n.treasury, n.cash.iter().sum::<f32>(), g.money_supply(me),
                        n.remit_out.iter().sum::<f32>(), n.remit_in.iter().sum::<f32>(),
                        g.fiscal.fled[me], g.fiscal.confidence[me], n.invest_k
                    );
                }
            }
        }
    }
    if std::env::var_os("ECONDBG").is_some() {
        let g = run(&base, &[], 12, &none);
        for b in 0..polity::BLOCS {
            let parts: Vec<String> = g.mood_parts(me, b).iter().map(|(w, x)| format!("{w} {x:+.2}")).collect();
            println!("  {} {:+.2}: {}", polity::BLOC[b].name, g.moods_now(me)[b], parts.join("; "));
        }
        for lv in [0u8, 2] {
            let mut g = base.clone();
            let c = g.country(me);
            g.polity.of[c].laws[polity::LABOUR] = lv;
            let mut line = format!("  труд {lv}:");
            for m in 0..60 {
                g.advance();
                while let Some(id) = g.ev.waiting.first().map(|e| e.id) {
                    g.answer(id, 0);
                }
                if m % 12 == 11 {
                    let n = g.h.home(me);
                    let parts: Vec<String> = (0..g.pack.groups.len()).map(|k| format!("{} {:.2}{}", g.pack.groups[k].name, n.pop[k], n.unemployed[k].map_or(String::new(), |u| format!("/{:.0}%", u * 100.0)))).collect();
                    line += &format!(" | {}г: {}{}", m / 12 + 1, parts.join(" "), if g.h.at_war { " ВОЙНА" } else { "" });
                }
            }
            println!("{line}");
        }
    }

    // 1. relief feeds the poor: their lot better than the twin's without it
    let all_lines = |lv: u8, k: usize| format!("{k}:{lv}");
    let r3 = all_lines(3, RELIEF);
    let r0 = all_lines(0, RELIEF);
    let a = run(&base, &[("civ", &r3)], 24, &none);
    let b = run(&base, &[("civ", &r0)], 24, &none);
    let poor = |g: &Game| 0.5 * (lot(g, polity::PEASANTS) + lot(g, polity::WORKERS));
    // (the want of the plain people — their unmet needs, weighed by how many they are: relief goes
    // to the poorest, where the dole went to all by head)
    let want = |g: &Game| {
        let n = g.h.home(g.me);
        let gs: Vec<usize> = (0..g.pack.groups.len()).filter(|&k| crate::country::policy::role(&g.pack, k) <= 1).collect();
        let pop: f32 = gs.iter().map(|&k| n.pop[k]).sum::<f32>().max(1e-9);
        gs.iter().map(|&k| n.hardship[k] * n.pop[k]).sum::<f32>() / pop
    };
    let (wa, wb) = (want(&a), want(&b));
    let k = poor(&a) / poor(&b).max(1e-9);
    checks.push(("призрение: у бедных меньше нужды".into(), wa < wb - 0.005, format!("за 2 года нужда крестьян и рабочих {:.1}% со «всеобщим» призрением против {:.1}% без него; их достаток ×{k:.3}{}", wa * 100.0, wb * 100.0, war(&a, &b))));

    // 2. schools make readers and the learned (five years)
    let s3 = all_lines(3, SCHOOLS);
    let s0 = all_lines(0, SCHOOLS);
    let a = run(&base, &[("civ", &s3)], 60, &none);
    let b = run(&base, &[("civ", &s0)], 60, &none);
    let (la, lb) = (a.polity_of(me).literacy, b.polity_of(me).literacy);
    let (ea, eb) = (a.learned_share(me), b.learned_share(me));
    checks.push(("школы учат: грамотных и образованных больше".into(), la - lb > 0.01 && ea >= eb, format!("за 5 лет грамотность {la:.3} против {lb:.3}, образованных {:.2}% против {:.2}%{}", ea * 100.0, eb * 100.0, war(&a, &b))));

    // 3. doctors: fewer die (five years)
    let h3 = all_lines(3, HEALTH);
    let h0 = all_lines(0, HEALTH);
    let a = run(&base, &[("civ", &h3)], 36, &none);
    let b = run(&base, &[("civ", &h0)], 36, &none);
    let (da, db) = (a.demo.deaths[me], b.demo.deaths[me]);
    let (pa, pb) = (a.h.home(me).people(), b.h.home(me).people());
    checks.push(("врачи: умирают меньше, людей больше".into(), db - da > 0.003 && pa > pb, format!("за 3 года смертность {:.2}% против {:.2}% в год, людей {pa:.1} против {pb:.1} млн{}", da * 100.0, db * 100.0, war(&a, &b))));

    // 4. cheap credit swells the loans and the money, and the owners buy with it — abroad too, in a
    // world of one money (three years). (Whether they build with it is shown, not required: the
    // engine's owners build where hands are free and a job pays, and money alone does not make
    // either — found in the 27th session.)
    let a = run(&base, &[("fx_rate", "0")], 36, &none);
    let b = run(&base, &[("fx_rate", "2")], 36, &none);
    let (ca, cb) = (a.credit.loans[me], b.credit.loans[me]);
    let (ma, mb) = (a.money_supply(me), b.money_supply(me));
    let (pa, pb) = (a.h.home(me).level, b.h.home(me).level);
    let jobs = |g: &Game| {
        let n = g.h.home(g.me);
        (0..g.pack.sectors.len()).filter(|&k| !g.pack.sectors[k].land).map(|k| n.cap[k]).sum::<f32>()
    };
    let imports = |g: &Game| {
        let n = g.h.home(g.me);
        (0..g.pack.goods.len()).map(|i| n.import[i] * n.price[i]).sum::<f32>()
    };
    let (ja, jb) = (jobs(&a), jobs(&b));
    checks.push((
        "дешёвый кредит: займов и денег больше, больше ввоз".into(),
        ca > 1.2 * cb && ma > 1.01 * mb && imports(&a) > imports(&b),
        format!(
            "за 3 года займы ×{:.2}, мест на заводах ×{:.3}; денег ×{:.3}, цены ×{:.3}, ввоз ×{:.2} против дорогих денег{}",
            ca / cb.max(1e-9),
            ja / jb.max(1e-9),
            ma / mb.max(1e-9),
            pa / pb.max(1e-9),
            imports(&a) / imports(&b).max(1e-9),
            war(&a, &b)
        ),
    ));

    // 5. bonds: the owners' money goes into the treasury, and comes back with interest
    let owners: Vec<usize> = (0..base.pack.groups.len()).filter(|&g| crate::country::policy::role(&base.pack, g) >= 2).collect();
    let held = |g: &Game| owners.iter().map(|&k| g.h.home(me).money[k]).sum::<f32>();
    let mut g = base.clone();
    let (t0, h0) = (g.h.home(me).treasury, held(&g));
    g.apply(&[("bonds_sell".to_string(), "1".to_string())]);
    let (t1, h1) = (g.h.home(me).treasury, held(&g));
    let moved = (t1 - t0, h0 - h1);
    let bonds = g.credit.bonds[me];
    let later = run(&g, &[], 24, &none);
    let paid = later.credit.bond_paid[me];
    checks.push((
        "облигации: деньги собственников в казну и обратно с процентом".into(),
        moved.0 > 0.0 && (moved.0 - moved.1).abs() <= 0.01 * moved.0.abs().max(1e-6) && paid > 0.0 && later.credit.bonds[me] < bonds,
        format!("казне +{:.2}, у собственников и образованных −{:.2}; через 2 года платим {:.3} в месяц, долг {:.2} → {:.2}", moved.0, moved.1, paid, bonds, later.credit.bonds[me]),
    ));

    // 6. serfdom keeps people on the land (three years, the same world but the law of labour: the
    // world of seed 1 goes to war in the fourth, and the occupation then moves the shares)
    let set = |lv: u8| {
        move |g: &mut Game| {
            let c = g.country(g.me);
            g.polity.of[c].laws[polity::LABOUR] = lv;
        }
    };
    let land = |g: &Game| {
        let n = g.h.home(g.me);
        (0..g.pack.groups.len()).filter(|&k| crate::country::policy::role(&g.pack, k) == 0).map(|k| n.pop[k]).sum::<f32>() / n.people().max(1e-9)
    };
    let a = run(&base, &[], 36, &set(0));
    let b = run(&base, &[], 36, &set(2));
    let (fa, fb) = (land(&a), land(&b));
    let law = |g: &Game| g.polity_of(g.me).laws[polity::LABOUR];
    checks.push(("неволя держит людей на земле".into(), fa > fb + 0.002, format!("за 3 года на земле {:.1}% людей при неволе против {:.1}% при вольном труде (закон к концу: {} и {}){}", fa * 100.0, fb * 100.0, law(&a), law(&b), war(&a, &b))));

    // 7. a tax on the rich: the industrialists feel it in their own books
    let a = run(&base, &[("fx_tax", "2")], 12, &none);
    let b = run(&base, &[("fx_tax", "0")], 12, &none);
    let burden = |g: &Game| g.burdens(g.me)[polity::CAPITAL];
    // (their mood's part from the tax they pay — measured in their books; their want, «как живут»,
    // answers the tax too, through the market, and is shown)
    let part = |g: &Game| g.mood_parts(g.me, polity::CAPITAL).iter().filter(|p| p.0.starts_with("налог берёт")).map(|p| p.1).sum::<f32>();
    let mood = |g: &Game| g.moods_now(g.me)[polity::CAPITAL];
    let (ba, bb, pa, pb) = (burden(&a), burden(&b), part(&a), part(&b));
    checks.push((
        "налог на роскошь злит промышленников через их доход".into(),
        ba > bb + 0.05 && pa < pb - 0.1,
        format!("за год налог берёт {:.0}% против {:.0}% их дохода; часть настроения от налога {:+.2} против {:+.2}, всё настроение {:+.2} против {:+.2}{}", ba * 100.0, bb * 100.0, pa, pb, mood(&a), mood(&b), war(&a, &b)),
    ));

    let ok = checks.iter().filter(|c| c.1).count();
    println!("проверки хозяйства и власти{} ({:.0} с):", if spoil { " — КОНТРОЛЬ: механизмы выключены, все должны упасть" } else { "" }, started.elapsed().as_secs_f32());
    for (name, good, why) in &checks {
        println!("  [{}] {:<46} {why}", if *good { "OK" } else { "FAIL" }, name);
    }
    println!("итого: {ok} из {}", checks.len());
    if spoil {
        ok == 0
    } else {
        ok == checks.len()
    }
}
