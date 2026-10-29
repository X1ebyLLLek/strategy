//! The state's finances in the folder of the economy: the figures a minister reads (the treasury's
//! income by its sources, trade, the rich's share, what a working man's pay buys), and the levers —
//! who pays the tax, to whom the state hands back, duties, credit, the press, the labour law, a cap
//! on the price of bread; subsidies and state ownership of each trade.

use crate::country::fiscal::{dole_weights, Choice, CAP, DOLES, INVEST_K, LABOUR, PRINTS, PRINT_SHARE, RATES, TARIFFS, TAXES, WAGE_K};
use super::super::charts::num;
use super::*;
use super::super::kit::{self, Stat};

/// A law of four choices as a row (29th session): its name, ‹ the one in force ›, what it means; the
/// choice asked for is sent as `key`.
fn choices(ui: &mut egui::Ui, s: &Skin, g: &mut Game, label: &str, icon: &str, list: &'static [Choice; 4], chosen: usize, key: &str) {
    if let Some(k) = levers::pick(ui, s, icon, label, list, chosen) {
        act(g, &[(key, k.to_string())]);
    }
    levers::rule(ui, s);
}

/// A law of four choices as a card of paper (35th session): what it is and the choice in force, a good and a bad of it,
/// the tiles of its choices — the card of each with what it would do in figures (`rows`); the choice asked for is sent as `key`.
#[allow(clippy::too_many_arguments)]
fn law_card(ui: &mut egui::Ui, s: &Skin, g: &mut Game, icon: &str, title: &str, list: &'static [Choice; 4], short: [&str; 4], chosen: usize, usual: usize, key: &str, pro: &str, con: &str, rows: &dyn Fn(usize) -> Vec<(String, String)>) {
    let mut asked = None;
    kit::framed(ui, Some(s.accent), false, |ui| {
        ui.horizontal(|ui| {
            let (ir, _) = ui.allocate_exact_size(vec2(28.0, 28.0), egui::Sense::hover());
            if ui.is_rect_visible(ir) {
                kit::socket(ui, ir.center(), 13.0, icon, s.accent, 0.3);
            }
            kit::bold(ui, title.to_string(), 15.5, s.ink);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                kit::bold(ui, list[chosen].name.to_string(), 14.0, s.ink);
            });
        });
        kit::words(ui, list[chosen].about, 13.0, s.soft);
        ui.add_space(2.0);
        super::taxes::pro_con(ui, s, pro, con);
        ui.add_space(3.0);
        let marks: Vec<(&str, &str)> = (0..4).map(|k| (short[k], list[k].icon)).collect();
        asked = widgets::law(ui, &marks, chosen, usual, &|k| {
            let mut c = Card::new(format!("{title}: {}", list[k].name)).icon(list[k].icon).text(list[k].about);
            for (name, v) in rows(k) {
                c = c.row(name, v);
            }
            c.note(if k == chosen { "действует сейчас" } else { "щёлкните — решение уйдёт в правительство" })
        })
        .filter(|&k| k != chosen);
    });
    if let Some(k) = asked {
        act(g, &[(key, k.to_string())]);
    }
}

/// The section of finances.
pub(super) fn finance(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    let fig = g.figures(me);
    let income = g.income(me).max(1e-6);
    kit::ribbon(ui, "Финансы государства", Some("coins"));
    {
        let share = |x: f32| -> &'static str {
            match x / income {
                y if y < 0.02 => "почти ничего",
                y if y < 0.1 => "немного",
                y if y < 0.3 => "заметно",
                _ => "много",
            }
        };
        let balance = fig.exports - fig.imports;
        let trade = if fig.exports + fig.imports < 1e-3 {
            "торговли нет"
        } else if balance > 0.15 * (fig.exports + fig.imports) {
            "вывоз больше"
        } else if balance < -0.15 * (fig.exports + fig.imports) {
            "ввоз больше"
        } else {
            "поровну"
        };
        let rich = match fig.owners {
            x if x < 0.2 => "небольшая",
            x if x < 0.35 => "заметная",
            x if x < 0.5 => "большая",
            _ => "огромная",
        };
        let worst = fig.wages.iter().map(|w| w.1).fold(f32::INFINITY, f32::min);
        let pay = match worst {
            x if !x.is_finite() => "—",
            x if x < 0.8 => "впроголодь",
            x if x < 1.1 => "еле хватает",
            x if x < 1.6 => "хватает",
            _ => "с избытком",
        };
        let balance = fig.exports - fig.imports;
        let _ = balance;
        let conf = fig.confidence;
        let printed = fig.printed;
        let mut items = vec![
            Stat::new("backforth", "Торговля", trade, kit::PLAIN),
            Stat::new("gem", "Доля богатых", rich, if fig.owners > 0.45 { kit::BAD } else { kit::PLAIN }),
            Stat::new("bread", "Плата бедных", pay, if worst < 1.0 { kit::BAD } else { kit::GOOD }),
        ];
        items.push(Stat::new("sprout", "Деловой климат", if conf < 0.6 { "паника" } else if conf < 0.85 { "тревожный" } else if conf < 1.15 { "спокойный" } else { "подъём" }, if conf < 0.85 { kit::BAD } else if conf > 1.15 { kit::GOOD } else { kit::PLAIN }));
        items.push(Stat::new("backforth", "Капитал за границей", share(fig.fled), if fig.fled > 0.3 * income { kit::BAD } else { kit::PLAIN }));
        items.push(Stat::new("hand", "Уклонение от налога", if fig.evaded < 0.03 { "почти нет" } else if fig.evaded < 0.12 { "заметное" } else { "повальное" }, if fig.evaded > 0.12 { kit::BAD } else { kit::PLAIN }));
        items.push(Stat::new("stamper", "Станок", if printed <= 0.0 { "стоит" } else { share(printed) }, if printed > 0.1 * income { kit::BAD } else { kit::PLAIN }));
        let resp = kit::stats(ui, &items, 3);
        let (fled, ev) = (fig.fled, fig.evaded);
        let (ex, im, ow) = (fig.exports, fig.imports, fig.owners);
        let w = fig.wages.clone();
        let cards: Vec<Box<dyn Fn() -> Card>> = vec![
            Box::new(move || Card::new("Торговля").icon("backforth").row("вывоз в месяц", format!("{ex:.2}")).row("ввоз в месяц", format!("{im:.2}"))),
            Box::new(move || Card::new("Доля богатых").icon("gem").row("собственникам из всех доходов", format!("{:.0}%", ow * 100.0))),
            Box::new(move || {
                let mut c = Card::new("Что покупает плата").icon("bread");
                for (name, x) in &w {
                    c = c.row(name.clone(), format!("{:.0}% нужд", x * 100.0));
                }
                c.note("плата работающего против того, что стоят нужды его группы")
            }),
            Box::new(move || Card::new("Деловой климат").icon("sprout").row("уверенность собственников", format!("{:.0}%", conf * 100.0)).note("растёт с выпуском и спокойной властью; падает от инфляции, войны, волнений, отнятых заводов, высоких налогов и сильных профсоюзов. От неё зависит, сколько строят хозяева")),
            Box::new(move || Card::new("Капитал за границей").icon("backforth").row("увезено", format!("{fled:.2}")).note("когда собственникам страшно, они увозят деньги; вернутся, когда климат станет лучше")),
            Box::new(move || Card::new("Уклонение от налога").icon("hand").row("не доплачивают", format!("{:.0}% налога", ev * 100.0)).note("чем выше налог и слабее власть, тем больше прячут — больше всех богатые")),
            Box::new(move || Card::new("Печатный станок").icon("stamper").row("напечатано в месяц", format!("{printed:.2}")).note("деньги без налогов, но цены растут тем быстрее, чем больше печатают")),
        ];
        for (r, c) in resp.into_iter().zip(cards.iter()) {
            r.tip(|| c());
        }
    }
    kit::words(ui, "Налоги и пошлины — на странице «Налоги». Остальные рычаги меняют само хозяйство: у кого сколько денег, что выгодно строить, что везут из-за границы. Что выйдет — покажут цены, стройка и настроение людей.", 13.0, s.soft);
    let fx = g.fiscal.clone();
    {
        let n = g.h.home(me).clone();
        let at_war = g.h.at_war;
        let book = g.rows.last().map(|r| r.book[me]).unwrap_or_default();
        let dole = |k: usize| -> Vec<(String, String)> {
            let w = dole_weights(&pack, &n, k, at_war);
            let total: f32 = (0..w.len()).map(|gr| n.pop[gr] * w[gr]).sum::<f32>().max(1e-9);
            let mut v = vec![("раздача в месяц".to_string(), num(book.dole))];
            for gr in 0..w.len() {
                if n.pop[gr] > 1e-6 {
                    v.push((pack.groups[gr].name.clone(), format!("{:.0}% раздачи", n.pop[gr] * w[gr] / total * 100.0)));
                }
            }
            v
        };
        law_card(ui, s, g, "bread", "Кому раздаёт казна", &DOLES, ["Поровну", "Бедным", "Семьям", "Хозяевам"], fx.dole[me], 0, "fx_dole", "что казна не потратила, возвращается людям и идёт в покупки", "кому раздают — тем и достаётся: бедным спокойствие, предпринимателям стройка", &dole);
        let rate = |k: usize| -> Vec<(String, String)> {
            vec![("стройка частных заводов".into(), format!("×{:.2}", INVEST_K[k])), ("печатный станок".into(), if k == 3 { "нельзя".into() } else { "можно".into() })]
        };
        law_card(ui, s, g, "coins", "Кредит", &RATES, ["Дешёвый", "Обычная", "Дорогой", "Золото"], fx.rate[me], 1, "fx_rate", "дешёвые деньги — владельцы строят быстрее; дорогие — цены спокойнее", "дешёвые — цены растут, а при долгом кредите и пузырь; дорогие — стройка медленнее", &rate);
        let press = |k: usize| -> Vec<(String, String)> {
            let m = if fx.rate[me] == 3 { 0.0 } else { PRINT_SHARE[k] * income };
            vec![("напечатает в месяц".into(), num(m)), ("доля доходов казны".into(), format!("{:.0}%", PRINT_SHARE[k] * 100.0))]
        };
        law_card(ui, s, g, "stamper", "Печатный станок", &PRINTS, ["Стоит", "Понемногу", "Щедро", "Без удержу"], fx.print[me], 0, "fx_print", "деньги казне сразу, без налогов и займов", "чем больше печатают, тем быстрее растут цены и дешевеют деньги", &press);
    }
    {
        let k = g.bond_level(me);
        choices(ui, s, g, "Военные займы", "scroll", &crate::country::war_mood::BONDS, k, "fx_bonds");
        let owed: f32 = g.war_mood.owed[me].iter().sum();
        if owed > 0.0 || g.war_mood.took[me] > 0.0 {
            let line = format!(
                "Казна должна по займам {}; за месяц взяла {}, заплатила процентов и долга {}.",
                super::super::charts::num(owed),
                super::super::charts::num(g.war_mood.took[me]),
                super::super::charts::num(g.war_mood.paid[me])
            );
            say(ui, s, &line);
        }
    }
    {
        let wage = |k: usize| -> Vec<(String, String)> { vec![("доля работников в добавленной стоимости".into(), format!("×{:.2}", WAGE_K[k]))] };
        law_card(ui, s, g, "hand", "Закон о труде", &LABOUR, ["Без защиты", "Обычай", "Профсоюзы", "Контроль"], fx.labour[me], 1, "fx_labour", "работникам доля больше: они покупают больше и спокойнее", "хозяевам меньше прибыли: строят меньше, уверенность падает", &wage);
    }
    {
        let on = fx.caps[me];
        if levers::switch(ui, s, "bread", "Твёрдые цены на хлеб", on, ["Нет", "Введены"], true, &format!("цена того, без чего не живут, не выше {:.0}% обычной: бедным легче, но чего не хватает — делят в очередях, а делать это становится менее выгодно", CAP * 100.0)) {
            act(g, &[("fx_caps", if on { "0".into() } else { "1".into() })]);
        }
        levers::rule(ui, s);
    }
    {
        let on = g.fiscal.embargo[me];
        let rival = g.h.home(1 - me).name.clone();
        let about = format!("запретить торговлю с {rival}: его товары к нам не идут, наши к нему тоже; соседу хуже, если он живёт привозом, — но и нам дороже то, что везли от него{}", if g.fiscal.embargo[1 - me] { ". Сосед закрыл для нас свою торговлю" } else { "" });
        if levers::switch(ui, s, "wall", "Эмбарго соседу", on, ["Нет", "Введено"], !g.h.at_war, &about) {
            act(g, &[("fx_embargo", if on { "0".into() } else { "1".into() })]);
        }
        levers::rule(ui, s);
    }
    // the trades: subsidies and state ownership, each a figure with − and +
    kit::bold(ui, "Отрасли: субсидии и казённая доля", 14.0, s.ink);
    kit::words(ui, "Субсидия — доля бюджета казны в месяц деньгами отрасли: она держит места и строится быстрее. Казённая доля — часть прибыли, что идёт казне, а не собственникам; отобрать отрасль — собственники долго не простят.", 12.5, s.soft);
    let n = g.h.home(me).clone();
    let mut sub_click: Option<(usize, u8)> = None;
    let mut pub_click: Option<(usize, u8)> = None;
    let label_w = 150.0;
    let gap = ui.spacing().item_spacing.x;
    let colw = ((ui.available_width() - label_w - 2.0 * gap - 2.0) / 2.0).clamp(90.0, 124.0);
    ui.horizontal(|ui| {
        ui.add_space(label_w + gap);
        for t in ["субсидия", "казне"] {
            let (r, _) = ui.allocate_exact_size(vec2(colw, 16.0), egui::Sense::hover());
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, t.to_uppercase(), egui::FontId::new(12.0, egui::FontFamily::Name("head".into())), s.soft);
        }
    });
    for (k, sec) in pack.sectors.iter().enumerate() {
        if n.cap.get(k).copied().unwrap_or(0.0) <= 1e-4 && sec.build.is_empty() {
            continue;
        }
        let sub = fx.subsidy[me].get(k).copied().unwrap_or(0);
        let own = fx.public[me].get(k).copied().unwrap_or(0);
        ui.horizontal(|ui| {
            let mut job = egui::text::LayoutJob::simple_singleline(sec.name.clone(), egui::FontId::new(14.0, egui::FontFamily::Proportional), s.ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(label_w - 4.0);
            let gal = ui.painter().layout_job(job);
            let (lr, _) = ui.allocate_exact_size(vec2(label_w, 27.0), egui::Sense::hover());
            ui.painter().galley(egui::pos2(lr.min.x, lr.center().y - gal.size().y / 2.0), gal, s.ink);
            let words_sub = ["нет", "1%", "2,5%", "5%"];
            let words_pub = ["нет", "¼", "½", "вся"];
            let (d, _) = widgets::stepper(ui, words_sub[sub as usize], colw, true);
            if d != 0 {
                sub_click = Some((k, (sub as i32 + d as i32).clamp(0, 3) as u8));
            }
            let (d, _) = widgets::stepper(ui, words_pub[own as usize], colw, true);
            if d != 0 {
                pub_click = Some((k, (own as i32 + d as i32).clamp(0, 3) as u8));
            }
        });
    }
    if let Some((k, lv)) = sub_click {
        act(g, &[("fx_sub", format!("{k}:{lv}"))]);
    }
    if let Some((k, lv)) = pub_click {
        act(g, &[("fx_pub", format!("{k}:{lv}"))]);
    }
    // the rival's finances, as far as they show
    let o = 1 - me;
    let theirs = format!("У соседа: {}, {}, {}", TAXES[g.fiscal.tax[o]].name.to_lowercase(), TARIFFS[g.fiscal.tariff[o]].name.to_lowercase(), RATES[g.fiscal.rate[o]].name.to_lowercase());
    kit::words(ui, theirs, 13.0, s.soft);
}
