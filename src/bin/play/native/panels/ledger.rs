//! The folder of the ledger (his word in the 23rd session: «нужны графики/цифры, чтобы люди
//! понимали, как, куда, зачем можно вкладываться»): the treasury's month — what it took and paid,
//! each with its number; charts over the months, ours against the rival's (the economy, the
//! treasury, the government, science and the army, the forces of society); where each of the
//! country's multipliers comes from, thing by thing; and a forecast — the same world lived a year
//! ahead with one decision changed against none (the rival and luck live too: it is the game
//! itself, not a formula), so that the player sees what a decision does before he takes it.

use crate::country::spirits::Eff;
use super::forecast;
use super::*;
use super::super::charts;
use super::super::kit::{self, Stat};

/// The folder of the ledger.
pub fn ledger(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let Some(last) = g.rows.last().cloned() else { return };
    let b = last.book[me];
    let income = g.income(me);
    // the figures now
    let n = g.h.home(me);
    let growth = n.gdp_real / g.gdp0[me].max(1e-9) - 1.0;
    let items = vec![
        Stat::new("coins", "Выпуск в месяц", format!("{} ({:+.1}%)", charts::num(n.gdp_real), growth * 100.0), if growth >= 0.0 { kit::GOOD } else { kit::BAD }),
        Stat::new("balance", "Казна", charts::num(n.treasury), if n.treasury > 3.0 * income { kit::GOOD } else if n.treasury > 0.0 { kit::MIXED } else { kit::BAD }),
        Stat::new("stamper", "Налог", charts::pct(n.tax), kit::PLAIN),
        Stat::new("army", "Армии из трат", charts::pct(n.mil_share), kit::PLAIN),
        Stat::new("science", "Науке из трат", charts::pct(n.science_share), kit::PLAIN),
        Stat::new("shop", "Цены", format!("×{:.2}", n.level), if n.level < 1.3 { kit::PLAIN } else { kit::BAD }),
    ];
    let resp = kit::stats(ui, &items, 3);
    let rival = g.h.home(1 - me).gdp_real;
    let (gdp, tr, lvl) = (n.gdp_real, n.treasury, n.level);
    let cards: [Box<dyn Fn() -> Card>; 6] = [
        Box::new(move || Card::new("Выпуск").icon("coins").row("в месяц", charts::num(gdp)).row("к началу игры", format!("{:+.1}%", growth * 100.0)).row("у соседа", charts::num(rival)).note("всё, что страна произвела за месяц, в ценах начала игры")),
        Box::new(move || Card::new("Казна").icon("balance").row("в казне", charts::num(tr)).row("доход казны в месяц", charts::num(income)).row("запас", format!("{:.1} мес. доходов", tr / income.max(1e-6)))),
        Box::new(|| Card::new("Налог").icon("stamper").text("Доля доходов, которую забирает казна. Выше налог — больше денег армии, науке и стройке, но меньше у людей: они меньше покупают, злее и беднее.")),
        Box::new(|| Card::new("Армия").icon("army").text("Доля трат казны на армию: снаряжение, снабжение, жалованье. Больше армии — меньше остаётся на раздачу и стройку.")),
        Box::new(|| Card::new("Наука").icon("science").text("Доля трат казны на лаборатории: больше учёных, быстрее техника — но учёные уходят с заводов.")),
        Box::new(move || Card::new("Цены").icon("shop").row("уровень цен", format!("×{lvl:.2}")).text("Рост цен съедает доходы людей. Растут от печатного станка, дешёвого кредита, нехватки, пошлин.")),
    ];
    for (r, c) in resp.into_iter().zip(cards.iter()) {
        r.tip(|| c());
    }
    // the treasury's month
    kit::ribbon(ui, "Казна за месяц", Some("balance"));
    let inc = b.income();
    let out = b.spent();
    kit::words(ui, format!("Пришло {} · ушло {} · итог {:+.1}", charts::num(inc), charts::num(out), inc - out), 13.5, s.ink);
    kit::bold(ui, "Доходы", 13.0, s.soft);
    let ins: Vec<(String, f32, Color32)> = [
        ("налоги", b.tax, kit::GOOD),
        ("пошлины", b.duty, kit::GOOD),
        ("казённые отрасли", b.sectors, kit::GOOD),
        ("прочее: займы, станок", b.other.max(0.0), kit::PLAIN),
    ]
    .into_iter()
    .filter(|x| x.1 > 1e-4)
    .map(|(a, x, c)| (a.to_string(), x, c))
    .collect();
    let rs = charts::bars(ui, &ins, &|x| charts::num(x));
    for (r, (name, x, _)) in rs.into_iter().zip(ins.iter()) {
        r.tip(|| Card::new(name.clone()).row("за месяц", charts::num(*x)).row("доля доходов", charts::pct(*x / inc.max(1e-9))));
    }
    kit::bold(ui, "Расходы", 13.0, s.soft);
    let outs: Vec<(String, f32, Color32, &str)> = [
        ("снабжение армии", b.army, kit::BAD, "закупки армии на рынке: оружие, боеприпасы, еда, горючее"),
        ("жалованье солдатам", b.soldiers, kit::BAD, "плата служащим по закону о призыве"),
        ("наука", b.science, Color32::from_rgb(86, 130, 196), "жалованье учёным лабораторий"),
        ("стройка казны", b.build, kit::MIXED, "заводы, которые строит казна"),
        ("субсидии отраслям", b.subsidy, kit::MIXED, "деньги выбранным отраслям на сырьё"),
        ("раздача людям", b.dole, kit::PLAIN, "что казна не потратила, раздаётся людям по выбранному правилу"),
        ("прочее: программы, долги", (-b.other).max(0.0), kit::PLAIN, "государственные программы, проценты по займам, решения в событиях"),
    ]
    .into_iter()
    .filter(|x| x.1 > 1e-4)
    .map(|(a, x, c, w)| (a.to_string(), x, c, w))
    .collect();
    let items: Vec<(String, f32, Color32)> = outs.iter().map(|x| (x.0.clone(), x.1, x.2)).collect();
    let rs = charts::bars(ui, &items, &|x| charts::num(x));
    for (r, (name, x, _, what)) in rs.into_iter().zip(outs.iter()) {
        r.tip(|| Card::new(name.clone()).row("за месяц", charts::num(*x)).row("доля расходов", charts::pct(*x / out.max(1e-9))).text(*what));
    }
    // the charts
    kit::ribbon(ui, "Графики", Some("chart"));
    let pick_id = egui::Id::new("ledger-chart");
    let picked: usize = ui.ctx().data(|d| d.get_temp(pick_id)).unwrap_or(0);
    let names = ["Хозяйство", "Казна", "Власть", "Наука и армия", "Силы общества"];
    if let Some(k) = widgets::switch(ui, &names, picked) {
        ui.ctx().data_mut(|d| d.insert_temp(pick_id, k));
    }
    let rows = &g.rows;
    let months: Vec<u32> = rows.iter().map(|r| r.month).collect();
    let mine = Color32::from_rgb(236, 190, 80);
    let theirs = Color32::from_rgb(196, 84, 70);
    let us = g.h.home(me).name.clone();
    let them = g.h.home(1 - me).name.clone();
    let two = |f: &dyn Fn(&crate::game::Row, usize) -> f32| -> Vec<(String, Color32, Vec<f32>)> {
        vec![(us.clone(), mine, rows.iter().map(|r| f(r, me)).collect()), (them.clone(), theirs, rows.iter().map(|r| f(r, 1 - me)).collect())]
    };
    match picked {
        0 => {
            charts::plot(ui, "Выпуск в месяц", &two(&|r, s| r.gdp[s]), &months, 150.0, &charts::num);
            charts::plot(ui, "Уровень цен", &two(&|r, s| r.level[s]), &months, 120.0, &|x| format!("×{:.2}", x));
            charts::plot(ui, "Недовольство народа", &two(&|r, s| r.anger[s]), &months, 120.0, &|x| format!("{x:.2}"));
        }
        1 => {
            charts::plot(ui, "Казна", &two(&|r, s| r.book[s].treasury), &months, 150.0, &charts::num);
            let flows = vec![
                ("доходы".to_string(), kit::GOOD, rows.iter().map(|r| r.book[me].income()).collect::<Vec<f32>>()),
                ("расходы".to_string(), kit::BAD, rows.iter().map(|r| r.book[me].spent()).collect::<Vec<f32>>()),
            ];
            charts::plot(ui, "Доходы и расходы казны в месяц", &flows, &months, 140.0, &charts::num);
            let parts = vec![
                ("армия".to_string(), Color32::from_rgb(176, 58, 42), rows.iter().map(|r| r.book[me].army + r.book[me].soldiers).collect::<Vec<f32>>()),
                ("наука".to_string(), Color32::from_rgb(86, 130, 196), rows.iter().map(|r| r.book[me].science).collect()),
                ("стройка".to_string(), kit::MIXED, rows.iter().map(|r| r.book[me].build + r.book[me].subsidy).collect()),
                ("раздача".to_string(), kit::PLAIN, rows.iter().map(|r| r.book[me].dole).collect()),
            ];
            charts::plot(ui, "Куда уходят деньги", &parts, &months, 150.0, &charts::num);
        }
        2 => {
            let lines = vec![
                ("устойчивость".to_string(), mine, rows.iter().map(|r| r.stability[me] * 100.0).collect::<Vec<f32>>()),
                ("законность".to_string(), Color32::from_rgb(120, 170, 220), rows.iter().map(|r| r.legit[me] * 100.0).collect()),
                ("смута".to_string(), theirs, rows.iter().map(|r| r.unrest[me] * 100.0).collect()),
            ];
            charts::plot(ui, "Власть (из 100)", &lines, &months, 160.0, &|x| format!("{x:.0}"));
            charts::plot(ui, "Устойчивость: мы и сосед", &two(&|r, s| r.stability[s] * 100.0), &months, 120.0, &|x| format!("{x:.0}"));
        }
        3 => {
            charts::plot(ui, "Темп науки (лет обычной науки в месяц)", &two(&|r, s| r.science[s]), &months, 140.0, &|x| format!("{x:.2}"));
            charts::plot(ui, "Армия (дивизий)", &two(&|r, s| r.army[s]), &months, 140.0, &|x| format!("{x:.0}"));
            if rows.iter().any(|r| r.held[0] < 0.999 || r.held[1] < 0.999) {
                charts::plot(ui, "Свои люди под своей властью, %", &two(&|r, s| r.held[s] * 100.0), &months, 120.0, &|x| format!("{x:.0}"));
            }
        }
        _ => {
            let cols = [[150, 92, 160], [212, 186, 92], [170, 72, 60], [96, 112, 132], [70, 150, 140], [86, 130, 196], [196, 84, 84], [120, 160, 72]];
            let lines: Vec<(String, Color32, Vec<f32>)> = (0..strategy::world::polity::BLOCS).map(|b| (strategy::world::polity::BLOC[b].name.to_string(), Color32::from_rgb(cols[b][0], cols[b][1], cols[b][2]), rows.iter().map(|r| r.moods[b] * 100.0).collect())).collect();
            charts::plot(ui, "Настроения сил общества (−100…+100)", &lines, &months, 190.0, &|x| format!("{x:+.0}"));
        }
    }
    // where the multipliers come from
    kit::ribbon(ui, "Откуда что берётся", Some("pie"));
    kit::words(ui, "Каждый множитель страны — произведение всех причин. Раскройте строку: видно, что именно и насколько его двигает.", 12.5, s.soft);
    let parts = g.eff_sources(me);
    let total = g.effs[me].clone();
    type Get = fn(&Eff) -> f32;
    let fields: [(&str, Get, bool); 9] = [
        ("Урожай и всё, что даёт земля", |e| e.out_land, true),
        ("Заводы и рудники", |e| e.out_industry, true),
        ("Военные заводы", |e| e.out_military, true),
        ("Товары для людей", |e| e.out_civil, true),
        ("Сила науки", |e| e.science, true),
        ("Заводы осваивают образцы", |e| e.learn, true),
        ("Огонь в наступлении", |e| e.attack, true),
        ("Огонь в обороне", |e| e.defend, true),
        ("Сколько можно призвать", |e| e.draft, true),
    ];
    for (name, get, up_good) in fields {
        let t = get(&total);
        let mut items: Vec<(String, f32)> = parts.iter().map(|(w, e)| (w.clone(), get(e))).filter(|x| (x.1 - 1.0).abs() >= 0.004).collect();
        items.sort_by(|a, b| (b.1 - 1.0).abs().total_cmp(&(a.1 - 1.0).abs()));
        let good = (t >= 1.0) == up_good;
        egui::CollapsingHeader::new(RichText::new(format!("{name}: {}", charts::delta(t))).size(13.5).color(if (t - 1.0).abs() < 0.004 { s.ink } else if good { kit::GOOD } else { kit::BAD }))
            .id_salt(("eff", name))
            .show(ui, |ui| {
                if items.is_empty() {
                    kit::words(ui, "ничего не двигает", 12.5, s.soft);
                }
                for (w, x) in items {
                    ui.horizontal(|ui| {
                        let c = if (x >= 1.0) == up_good { kit::GOOD } else { kit::BAD };
                        kit::bold(ui, charts::delta(x), 12.5, c);
                        kit::words(ui, w, 12.5, s.ink);
                    });
                }
            });
    }
    {
        let stab: Vec<(String, f32)> = parts.iter().map(|(w, e)| (w.clone(), e.stability)).filter(|x| x.1.abs() >= 0.0003).collect();
        let t = total.stability;
        egui::CollapsingHeader::new(RichText::new(format!("Устойчивость власти: {:+.0} пунктов", t * 2000.0)).size(13.5).color(if t >= 0.0 { kit::GOOD } else { kit::BAD })).id_salt("eff-stab").show(ui, |ui| {
            let mut stab = stab;
            stab.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
            for (w, x) in stab {
                ui.horizontal(|ui| {
                    kit::bold(ui, format!("{:+.0}", x * 2000.0), 12.5, if x >= 0.0 { kit::GOOD } else { kit::BAD });
                    kit::words(ui, w, 12.5, s.ink);
                });
            }
            kit::words(ui, "пункты — куда со временем придёт устойчивость от этой причины (из 100)", 12.0, s.soft);
        });
    }
    // the forecast
    kit::ribbon(ui, "Прогноз: что будет через год, если…", Some("forecast"));
    kit::words(ui, "Тот же мир проживается на 12 месяцев вперёд — с решением и без него. Сосед, рынок и случай тоже живут: это сама игра, а не формула; только войны в ней не начинаются — военное положение перекрыло бы любое решение. Числа — разница с «ничего не менять».", 12.5, s.soft);
    let f = forecast::state();
    let (running, rows_f, fm, secs) = (f.running, f.rows.clone(), f.month, f.secs);
    let stale = f.stale(g);
    let label = if running { "Считается…" } else if rows_f.is_empty() { "Посчитать прогноз" } else { "Пересчитать прогноз" };
    let b = kit::act_button(ui, label, "forecast", !running, false);
    if b.tip(|| Card::new("Прогноз").icon("forecast").text("Займёт несколько секунд: мир проживается вперёд по разу на каждое решение (на всех ядрах процессора). Игра не останавливается.")).clicked() && !running {
        forecast::start(g);
    }
    if running {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
    }
    if !rows_f.is_empty() {
        if stale {
            kit::words(ui, format!("Посчитано на {} — с тех пор мир изменился.", crate::words::date_words(fm)), 12.5, kit::MIXED);
        } else {
            kit::words(ui, format!("Посчитано за {secs:.0} с."), 12.0, s.soft);
        }
        let base = rows_f[0].clone();
        egui::Grid::new("forecast-grid").striped(true).spacing(vec2(8.0, 3.0)).show(ui, |ui| {
            for h in ["решение", "выпуск", "казна", "народ", "власть", "наука", "армия"] {
                ui.label(RichText::new(h).size(12.0).color(s.soft));
            }
            ui.end_row();
            for o in &rows_f {
                let first = o.name == base.name;
                let r = ui.label(RichText::new(&o.name).size(12.5).color(s.ink));
                r.tip(|| Card::new(o.name.clone()).icon("forecast").text(o.about.clone()).row("выпуск через год", charts::num(o.gdp)).row("казна", charts::num(o.treasury)).row("недовольство", format!("{:.2}", o.anger)).row("устойчивость", format!("{:.0}", o.stability * 100.0)).row("законность", format!("{:.0}", o.legit * 100.0)).row("смута", format!("{:.0}", o.unrest * 100.0)).row("техник знает", format!("{:.0}", o.techs)).row("дивизий", format!("{:.0}", o.army)).row("цены", format!("×{:.2}", o.level)));
                let cell = |ui: &mut egui::Ui, d: f32, text: String, up_good: bool| {
                    let col = if first || d.abs() < 1e-3 { s.ink } else if (d > 0.0) == up_good { kit::GOOD } else { kit::BAD };
                    ui.label(RichText::new(text).size(12.5).color(col));
                };
                if first {
                    cell(ui, 0.0, charts::num(o.gdp), true);
                    cell(ui, 0.0, charts::num(o.treasury), true);
                    cell(ui, 0.0, format!("{:.2}", o.anger), false);
                    cell(ui, 0.0, format!("{:.0}", o.stability * 100.0), true);
                    cell(ui, 0.0, format!("{:.2}", o.science), true);
                    cell(ui, 0.0, format!("{:.0}", o.army), true);
                } else {
                    let dg = o.gdp / base.gdp.max(1e-9) - 1.0;
                    cell(ui, dg, format!("{:+.1}%", dg * 100.0), true);
                    let dt = o.treasury - base.treasury;
                    cell(ui, dt, format!("{:+.1}", dt), true);
                    let da = o.anger - base.anger;
                    cell(ui, da, format!("{:+.2}", da), false);
                    let ds = (o.stability - base.stability) * 100.0;
                    cell(ui, ds, format!("{:+.0}", ds), true);
                    let dsc = o.science / base.science.max(1e-9) - 1.0;
                    cell(ui, dsc, format!("{:+.0}%", dsc * 100.0), true);
                    let da2 = o.army - base.army;
                    cell(ui, da2, format!("{:+.0}", da2), true);
                }
                ui.end_row();
            }
        });
        kit::words(ui, "Выпуск и наука — в процентах к «ничего не менять»; казна, дивизии — разница; народ — недовольство (меньше — лучше); власть — устойчивость из 100.", 12.0, s.soft);
    }
}
