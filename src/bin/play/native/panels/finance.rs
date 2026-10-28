//! The state's finances in the folder of the economy: the figures a minister reads (the treasury's
//! income by its sources, trade, the rich's share, what a working man's pay buys), and the levers —
//! who pays the tax, to whom the state hands back, duties, credit, the press, the labour law, a cap
//! on the price of bread; subsidies and state ownership of each trade.

use crate::country::fiscal::{Choice, CAP, DOLES, LABOUR, PRINTS, PUBLIC, PUBLIC_WORDS, RATES, SUBSIDY, SUBSIDY_WORDS, TARIFFS, TAXES};
use super::*;
use super::super::kit::{self, Stat};

/// A lever as a row of tiles (the law widget): its label, its choices; the chosen one sent as
/// `key` when another is clicked.
fn choices(ui: &mut egui::Ui, s: &Skin, g: &mut Game, label: &str, list: &[Choice; 4], chosen: usize, usual: usize, key: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        ui.label(RichText::new(label).size(14.5).color(s.soft));
        ui.label(RichText::new(list[chosen].name).family(egui::FontFamily::Name("bold".into())).size(14.5).color(s.ink));
    });
    let tiles: Vec<(&str, &str)> = list.iter().map(|c| (c.name, c.icon)).collect();
    let picked = widgets::law(ui, &tiles, chosen, usual, &|k| {
        Card::new(list[k].name).icon(list[k].icon).sub(label.trim_end_matches(':').to_string()).text(list[k].about).note(if k == chosen { "действует сейчас" } else { "щёлкните — решение уйдёт в правительство" })
    });
    if let Some(k) = picked {
        act(g, &[(key, k.to_string())]);
    }
    ui.add_space(2.0);
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
        let items = vec![
            Stat::new("stamper", "Налоги", share(fig.taxes), kit::PLAIN),
            Stat::new("wall", "Пошлины", share(fig.duties), kit::PLAIN),
            Stat::new("anvil", "Казённые отрасли", share(fig.public), kit::PLAIN),
            Stat::new("backforth", "Торговля", trade, kit::PLAIN),
            Stat::new("gem", "Доля богатых", rich, if fig.owners > 0.45 { kit::BAD } else { kit::PLAIN }),
            Stat::new("bread", "Плата бедных", pay, if worst < 1.0 { kit::BAD } else { kit::GOOD }),
        ];
        let conf = fig.confidence;
        let mut items = items;
        items.push(Stat::new("sprout", "Деловой климат", if conf < 0.6 { "паника" } else if conf < 0.85 { "тревожный" } else if conf < 1.15 { "спокойный" } else { "подъём" }, if conf < 0.85 { kit::BAD } else if conf > 1.15 { kit::GOOD } else { kit::PLAIN }));
        items.push(Stat::new("backforth", "Капитал за границей", share(fig.fled), if fig.fled > 0.3 * income { kit::BAD } else { kit::PLAIN }));
        items.push(Stat::new("hand", "Уклонение от налога", if fig.evaded < 0.03 { "почти нет" } else if fig.evaded < 0.12 { "заметное" } else { "повальное" }, if fig.evaded > 0.12 { kit::BAD } else { kit::PLAIN }));
        // the economy as a whole: output over a year, prices over a year, men out of work, the press
        let (gr, inf, idle, printed) = (fig.growth, fig.inflation, fig.idle, fig.printed);
        items.push(Stat::new("sprout", "Выпуск за год", match gr { x if x < -0.03 => "падает", x if x < 0.01 => "стоит", x if x < 0.04 => "растёт", _ => "быстро растёт" }, if gr < -0.03 { kit::BAD } else if gr > 0.01 { kit::GOOD } else { kit::PLAIN }));
        items.push(Stat::new("coins", "Цены за год", match inf { x if x < -0.03 => "падают", x if x < 0.04 => "спокойны", x if x < 0.15 => "растут", _ => "скачут" }, if !(-0.03..=0.15).contains(&inf) { kit::BAD } else { kit::PLAIN }));
        items.push(Stat::new("hand", "Без работы", match idle { x if x < 0.04 => "почти никого", x if x < 0.1 => "немного", x if x < 0.2 => "много", _ => "каждый пятый" }, if idle >= 0.1 { kit::BAD } else { kit::PLAIN }));
        items.push(Stat::new("stamper", "Станок", if printed <= 0.0 { "стоит" } else { share(printed) }, if printed > 0.1 * income { kit::BAD } else { kit::PLAIN }));
        let resp = kit::stats(ui, &items, 3);
        let f2 = fig.clone();
        let cards: [Box<dyn Fn() -> Card>; 6] = [
            Box::new(move || Card::new("Налоги").icon("stamper").row("в месяц", format!("{:.2}", f2.taxes)).note("налог берут с доходов групп — как велит шкала налога")),
            Box::new({
                let d = fig.duties;
                move || Card::new("Пошлины").icon("wall").row("в месяц", format!("{d:.2}")).note("доля цены чужих товаров, проданных у нас")
            }),
            Box::new({
                let p = fig.public;
                move || Card::new("Казённые отрасли").icon("anvil").row("в месяц", format!("{p:.2}")).note("доля прибыли отраслей, которыми владеет казна")
            }),
            Box::new({
                let (e, i) = (fig.exports, fig.imports);
                move || Card::new("Торговля").icon("backforth").row("вывоз в месяц", format!("{e:.2}")).row("ввоз в месяц", format!("{i:.2}"))
            }),
            Box::new({
                let o = fig.owners;
                move || Card::new("Доля богатых").icon("gem").row("собственникам из всех доходов", format!("{:.0}%", o * 100.0))
            }),
            Box::new({
                let w = fig.wages.clone();
                move || {
                    let mut c = Card::new("Что покупает плата").icon("bread");
                    for (name, x) in &w {
                        c = c.row(name.clone(), format!("{:.0}% нужд", x * 100.0));
                    }
                    c.note("плата работающего против того, что стоят нужды его группы")
                }
            }),
        ];
        let (fled, ev) = (fig.fled, fig.evaded);
        let more: [Box<dyn Fn() -> Card>; 7] = [
            Box::new(move || Card::new("Деловой климат").icon("sprout").row("уверенность собственников", format!("{:.0}%", conf * 100.0)).note("растёт с выпуском и спокойной властью; падает от инфляции, войны, волнений, отнятых заводов, высоких налогов и сильных профсоюзов. От неё зависит, сколько строят хозяева")),
            Box::new(move || Card::new("Капитал за границей").icon("backforth").row("увезено", format!("{fled:.2}")).note("когда собственникам страшно, они увозят деньги; вернутся, когда климат станет лучше")),
            Box::new(move || Card::new("Уклонение от налога").icon("hand").row("не доплачивают", format!("{:.0}% налога", ev * 100.0)).note("чем выше налог и слабее власть, тем больше прячут — больше всех богатые")),
            Box::new(move || Card::new("Выпуск за год").icon("sprout").row("рост", format!("{:+.1}%", gr * 100.0)).note("всё, что страна сделала, в ценах прошлого года")),
            Box::new(move || Card::new("Цены за год").icon("coins").row("рост цен", format!("{:+.1}%", inf * 100.0)).note("станок, нехватка и война поднимают цены; застой роняет")),
            Box::new(move || Card::new("Без работы").icon("hand").row("из работающих", format!("{:.1}%", idle * 100.0)).note("безработные тратят меньше и злее смотрят на власть")),
            Box::new(move || Card::new("Печатный станок").icon("stamper").row("напечатано в месяц", format!("{printed:.2}")).note("деньги без налогов, но цены растут тем быстрее, чем больше печатают")),
        ];
        let all: Vec<&Box<dyn Fn() -> Card>> = cards.iter().chain(more.iter()).collect();
        for (r, c) in resp.into_iter().zip(all) {
            r.tip(|| c());
        }
    }
    say(ui, s, "Каждый рычаг меняет само хозяйство: у кого сколько денег, что выгодно строить, что везут из-за границы. Что из этого выйдет — покажут цены, стройка и настроение людей.");
    let fx = g.fiscal.clone();
    choices(ui, s, g, "Кто платит налог:", &TAXES, fx.tax[me], 0, "fx_tax");
    choices(ui, s, g, "Кому раздаёт казна:", &DOLES, fx.dole[me], 0, "fx_dole");
    choices(ui, s, g, "Пошлины:", &TARIFFS, fx.tariff[me], 0, "fx_tariff");
    choices(ui, s, g, "Кредит:", &RATES, fx.rate[me], 1, "fx_rate");
    choices(ui, s, g, "Печатный станок:", &PRINTS, fx.print[me], 0, "fx_print");
    {
        let k = g.bond_level(me);
        let usual = if g.h.at_war { 1 } else { 0 };
        choices(ui, s, g, "Военные займы:", &crate::country::war_mood::BONDS, k, usual, "fx_bonds");
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
    choices(ui, s, g, "Закон о труде:", &LABOUR, fx.labour[me], 1, "fx_labour");
    ui.horizontal(|ui| {
        let on = fx.caps[me];
        let text = if on { "Снять твёрдые цены" } else { "Твёрдые цены на хлеб" };
        if kit::act_button(ui, text, "bread", true, on)
            .tip(|| Card::new("Твёрдые цены").icon("bread").text(format!("Цена того, без чего не живут, не выше {:.0}% обычной: бедным легче, но чего не хватает — делят в очередях, а делать это становится менее выгодно.", CAP * 100.0)).note(if on { "действуют сейчас" } else { "щёлкните — ввести" }))
            .clicked()
        {
            act(g, &[("fx_caps", if on { "0".into() } else { "1".into() })]);
        }
    });
    ui.horizontal(|ui| {
        let on = g.fiscal.embargo[me];
        let rival = g.h.home(1 - me).name.clone();
        let text = if on { "Снять эмбарго" } else { "Эмбарго соседу" };
        if kit::act_button(ui, text, "wall", !g.h.at_war, on)
            .tip(|| Card::new("Эмбарго").icon("wall").text(format!("Запретить торговлю с {rival}: его товары к нам не идут, наши к нему тоже. Соседу хуже, если он живёт привозом, — но и нам дороже то, что везли от него.")).note(if on { "действует: щёлкните — снять" } else { "напряжённость с соседом вырастет" }))
            .clicked()
        {
            act(g, &[("fx_embargo", if on { "0".into() } else { "1".into() })]);
        }
        if g.fiscal.embargo[1 - me] {
            ui.label(RichText::new("сосед закрыл для нас свою торговлю").size(13.0).color(kit::ink(s, kit::BAD)));
        }
    });
    // the trades: subsidies and state ownership
    kit::words(ui, "ОТРАСЛИ: СУБСИДИИ И КАЗЁННАЯ ДОЛЯ", 12.0, s.soft);
    let n = g.h.home(me).clone();
    let mut sub_click: Option<(usize, u8)> = None;
    let mut pub_click: Option<(usize, u8)> = None;
    for (k, sec) in pack.sectors.iter().enumerate() {
        if n.cap.get(k).copied().unwrap_or(0.0) <= 1e-4 && sec.build.is_empty() {
            continue;
        }
        let sub = fx.subsidy[me].get(k).copied().unwrap_or(0);
        let own = fx.public[me].get(k).copied().unwrap_or(0);
        ui.horizontal(|ui| {
            let mut job = egui::text::LayoutJob::simple_singleline(sec.name.clone(), egui::FontId::new(14.0, egui::FontFamily::Proportional), s.ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(150.0);
            let gal = ui.painter().layout_job(job);
            let (lr, _) = ui.allocate_exact_size(vec2(156.0, 24.0), egui::Sense::hover());
            ui.painter().galley(egui::pos2(lr.min.x, lr.center().y - gal.size().y / 2.0), gal, s.ink);
            let (words_sub, words_pub) = (SUBSIDY_WORDS, PUBLIC_WORDS);
            if kit::act_button(ui, words_sub[sub as usize], "coins", true, sub > 0)
                .tip(|| Card::new(format!("Субсидия: {}", sec.name)).icon("coins").row("сейчас", format!("{:.1}% бюджета в месяц", SUBSIDY[sub as usize] * 100.0)).text("Деньги казны отрасли: она держит выпуск, когда невыгодно, и строится быстрее.").note("щёлкните — следующая ступень"))
                .clicked()
            {
                sub_click = Some((k, (sub + 1) % 4));
            }
            if kit::act_button(ui, words_pub[own as usize], "anvil", true, own > 0)
                .tip(|| Card::new(format!("Владелец: {}", sec.name)).icon("anvil").row("казне идёт", format!("{:.0}% прибыли", PUBLIC[own as usize] * 100.0)).text("Прибыль казённой доли идёт в казну, а не собственникам. Отобрать отрасль — собственники долго не простят, власть пошатнётся; вернуть — можно в любой момент.").note("щёлкните — следующая ступень"))
                .clicked()
            {
                pub_click = Some((k, (own + 1) % 4));
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
