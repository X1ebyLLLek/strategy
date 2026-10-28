//! The army's money in the folder «Армия»: where it goes (pay, career men, mending, supply, new kit,
//! the navy, what is left unspent) as tiles with their figures and as a chart by months; the peace
//! establishment as tiles and the mobilisation; the depots and the reservists; what the war costs.

use eframe::egui::{self, Color32, RichText};

use super::super::chrome::{self, Card, Tip, Tone};
use super::super::kit::{self, Stat};
use super::super::look::Skin;
use super::super::terms::say;
use super::super::widgets;
use super::{act, head};
use crate::army::budget::{MilBook, ESTABLISHMENTS, ESTABLISHMENT_TILES};
use crate::words::plural;
use crate::Game;

/// The colours of the items of the army's money (as on the chart).
const ITEMS: [(&str, &str, Color32); 7] = [
    ("Жалованье призванных", "people", Color32::from_rgb(196, 160, 96)),
    ("Кадровые офицеры", "medal", Color32::from_rgb(216, 196, 132)),
    ("Обслуживание техники", "spanner", Color32::from_rgb(120, 140, 150)),
    ("Снабжение: патроны, топливо", "bullets", Color32::from_rgb(186, 96, 64)),
    ("Новое снаряжение с заводов", "anvil", Color32::from_rgb(110, 150, 96)),
    ("Флот", "anchor", Color32::from_rgb(80, 120, 176)),
    ("Не потрачено", "coins", Color32::from_rgb(150, 150, 140)),
];

/// The items' names on the tiles (short: the long ones are on the chart's legend and the cards).
const TILES: [&str; 7] = ["Жалованье", "Кадровые офицеры", "Обслуживание", "Снабжение", "Новое снаряжение", "Флот", "Не потрачено"];

fn parts(b: &MilBook) -> [f32; 7] {
    [b.pay_men, b.pay_cadre, b.mend, b.supply, b.equip, b.navy, b.unspent]
}

/// A sum of money in the folder's figures.
fn money(x: f32) -> String {
    super::super::charts::num(x)
}

/// The army's money: tiles with what each item took last month, a chart of the items by months, and
/// in words what holds the army's size.
pub fn budget(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    let me = g.me;
    let b = g.mil.book[me];
    head(ui, s, "Военный бюджет");
    if b.allotted <= 0.0 && b.spent() <= 0.0 {
        say(ui, s, "Книги военного министерства откроются в конце месяца.");
        return;
    }
    let all = b.allotted.max(b.spent()).max(1e-9);
    let pc = |x: f32| format!("{} · {:.0}%", money(x), x / all * 100.0);
    let p = parts(&b);
    let items: Vec<Stat> = (0..ITEMS.len())
        .filter(|&i| i != 5 || g.navy_power(me).0 > 0 || p[5] > 0.0)
        .map(|i| Stat::new(ITEMS[i].1, TILES[i], pc(p[i]), if i == 6 && p[6] > 0.25 * all { kit::MIXED } else { ITEMS[i].2 }))
        .collect();
    let shown: Vec<usize> = (0..ITEMS.len()).filter(|&i| i != 5 || g.navy_power(me).0 > 0 || p[5] > 0.0).collect();
    let tips = [
        "солдатам и матросам по закону о призыве: их семьи живут на это",
        "офицеры и сержанты по профессии: им платят, как платит их ремесло, при любом законе",
        "запчасти и мастерские: без них техника изнашивается и выходит из строя",
        "что войска сжигают на учениях и в боях; на войне — главная статья",
        "заказы заводам: снаряжение новых частей, замена износа, запасы складов",
        "снабжение кораблей и постройка на верфях",
        "что казна выделила, а армия не взяла: в мирное время армия покупает лишь то, что нужно её штату; остаётся в казне",
    ];
    let r = kit::stats(ui, &items, 2);
    for (j, resp) in r.into_iter().enumerate() {
        let i = shown[j];
        resp.tip(|| Card::new(ITEMS[i].0).icon(ITEMS[i].1).row("за месяц", money(p[i])).row("доля бюджета", format!("{:.0}%", p[i] / all * 100.0)).note(tips[i]));
    }
    // the chart: the items month by month, one over another
    let books: Vec<&MilBook> = g.mil.books[me].iter().rev().take(120).collect::<Vec<_>>().into_iter().rev().collect();
    if books.len() >= 2 {
        let months: Vec<u32> = books.iter().map(|x| x.month).collect();
        let series: Vec<(String, Color32, Vec<f32>)> = (0..6).map(|i| (ITEMS[i].0.to_string(), ITEMS[i].2, books.iter().map(|x| parts(x)[i]).collect())).collect();
        ui.label(RichText::new("КУДА ИДУТ ДЕНЬГИ АРМИИ".to_string()).family(egui::FontFamily::Name("head".into())).size(13.0).color(s.soft));
        let card = |k: usize| {
            let x = books[k];
            let mut c = Card::new("Военный бюджет").icon("coins").sub(crate::words::date_words(x.month)).row("выделено", money(x.allotted));
            for (i, v) in parts(x).iter().enumerate() {
                if *v > 0.0 {
                    c = c.row(ITEMS[i].0, money(*v));
                }
            }
            c.row("дивизий", x.divisions.to_string()).row("служат", format!("{:.2} млн", x.serving))
        };
        widgets::chart(ui, &series, &months, 80.0, true, &card);
        widgets::legend(ui, s, &(0..6).map(|i| (ITEMS[i].0.to_string(), ITEMS[i].2)).collect::<Vec<_>>());
        ui.add_space(3.0);
    }
    // the arms works: how many, how much of the civil industry converted, what they make
    let w = g.works(me);
    let items = vec![
        Stat::new("anvil", "Военные заводы", format!("{:.2} млн мест", w.jobs), kit::PLAIN),
        Stat::new("gears", "Конверсия", format!("{:.0}% заводов", w.share * 100.0), if w.share > 0.0 { kit::MIXED } else { kit::PLAIN }),
        Stat::new("pistols", "Делают оружия", format!("{} в мес", money(w.made)), kit::PLAIN),
        Stat::new("route", "Из-за границы", format!("{} в мес", money(w.bought_abroad)), kit::PLAIN),
    ];
    let r = kit::stats(ui, &items, 2);
    let notes = [
        "рабочие места отраслей, делающих военные товары; растут, когда хозяевам выгодно строить",
        "в войну государство переводит часть гражданских заводов на оружие — тем больше, чем жёстче хозяйственный строй; переналаженный завод даёт лишь часть выпуска настоящего",
        "военные товары, сделанные за месяц (по обычным ценам)",
        "военные товары, привезённые за месяц из-за границы (по обычным ценам)",
    ];
    for (i, resp) in r.into_iter().enumerate() {
        resp.tip(|| Card::new(items[i].title.clone()).icon(items[i].icon.clone()).row("сейчас", items[i].value.clone()).note(notes[i]));
    }
    // in words: what the money is mostly, and what holds the army
    let main = (0..6).max_by(|&a, &c| p[a].total_cmp(&p[c])).unwrap_or(0);
    let line = match main {
        0 | 1 => "Больше всего армия тратит на людей: жалованье солдат и офицеров.",
        2 => "Больше всего уходит на обслуживание техники.",
        3 => "Больше всего армия сжигает: патроны и топливо — так стоит война.",
        4 => "Больше всего армия заказывает заводам: новое снаряжение.",
        _ => "Больше всего денег армии забирает флот.",
    };
    let mended = if b.mended < 0.9 { format!(" Обслуживанию не хватает запчастей ({:.0}% нужного): техника изнашивается.", b.mended * 100.0) } else { String::new() };
    let left = if p[6] > 0.2 * all && !g.h.at_war { " Армии выделено больше, чем нужно её штату: остаток остаётся в казне — можно урезать долю армии или поднять штат." } else { "" };
    let text = format!("{line}{mended}{left}");
    ui.scope(|ui| say(ui, s, &text)).response.tip(|| {
        Card::new("Деньги армии")
            .icon("coins")
            .row("выделено за месяц", money(b.allotted))
            .row("потрачено", money(b.spent()))
            .tone("обслужено техники", format!("{:.0}%", b.mended * 100.0), if b.mended > 0.9 { Tone::Good } else { Tone::Bad })
            .note("армия — это люди, которым платят, и заводы, которым заказывают: казна платит обоим")
    });
}

/// The peace establishment as tiles (the custom one marked), the mobilisation, the depots and the
/// reservists.
pub fn establishment(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    head(ui, s, "Штат и мобилизация");
    let per_now = g.establishment(me);
    let front = g.h.war.front(me).len().max(1) as f32;
    let chosen = g.mil.chosen[me].unwrap_or_else(|| ESTABLISHMENTS.iter().position(|e| (e.0 - per_now).abs() < 0.01).unwrap_or(1));
    let choices: Vec<(&str, &str)> = ESTABLISHMENT_TILES.iter().zip(["helmet", "army", "shield", "clash", "fist"]).map(|(t, i)| (*t, i)).collect();
    let divs = g.h.war.corps.as_ref().map_or(0, |c| c.divs.iter().filter(|d| d.side == me).count());
    let per_div = g.mil.book[me].allotted / divs.max(1) as f32;
    let row = ui.scope(|ui| {
        widgets::law(ui, &choices, chosen, 1, &|k| {
            let e = &ESTABLISHMENTS[k];
            Card::new(e.1)
                .icon(choices[k].1)
                .sub("штат мирного времени")
                .row("дивизий на провинцию границы", format!("{:.2}", e.0))
                .row("дивизий в штате", format!("{:.0}", e.0 * front))
                .row("дивизий сейчас", divs.to_string())
                .row("бюджета на дивизию", money(per_div))
                .text(e.2)
                .note(if k == chosen { "выбрано" } else { "щёлкните — решение уйдёт в правительство" })
        })
    });
    if let Some(k) = row.inner {
        act(g, &[("establish", k.to_string())]);
    }
    if g.h.at_war {
        say(ui, s, "Идёт война: армия на военном штате — столько дивизий, сколько вмещает фронт.");
    } else {
        let on = g.mil.mobilised[me].is_some();
        let theirs = g.mil.mobilised[1 - me].is_some();
        if theirs {
            say(ui, s, "Сосед объявил мобилизацию: его резервисты в полках.");
        }
        let w = ui.available_width();
        let label = if on { "Отменить мобилизацию" } else { "Объявить мобилизацию" };
        let b = kit::act_button(ui, label, if on { "stop" } else { "horn" }, true, on);
        let _ = w;
        if b.tip(|| {
            Card::new(label)
                .icon("horn")
                .text(if on {
                    "Резервисты разойдутся по домам и на работу, снаряжение вернётся на склады."
                } else {
                    "Резервисты из запаса идут в полки к снаряжению со складов: армия вырастет до военного штата за считанные недели. Люди уйдут с работы, казне — жалованье, соседу — тревога."
                })
                .row("на складах снаряжения", format!("{:.0} батальонов", g.mil.stored[me].iter().sum::<f32>() / strategy::military::corps::BATTALION))
                .row("резервистов", format!("{:.2} млн", g.mil.reservists[me]))
        })
        .clicked()
        {
            act(g, &[("mobilise", if on { "0" } else { "1" }.into())]);
        }
    }
    // the depots and the reservists
    let stored: f32 = g.mil.stored[me].iter().sum::<f32>() / strategy::military::corps::BATTALION;
    let kit_k = {
        let u: f32 = g.mil.stored[me].iter().sum();
        let k: f32 = g.mil.stored_kit[me].iter().sum();
        if u > 0.0 { k / u } else { 1.0 }
    };
    let res = g.mil.reservists[me];
    let n = g.h.home(me);
    let serving: f32 = n.drafted.iter().sum();
    let items = vec![
        Stat::new("crate", "Мобзапас", format!("{stored:.0} {}", plural(stored.round() as usize, "батальон", "батальона", "батальонов")), kit::PLAIN),
        Stat::new("spanner", "Его оснащение", format!("{:.0}%", kit_k.min(1.0) * 100.0), if kit_k > 0.8 { kit::GOOD } else { kit::BAD }),
        Stat::new("people", "Резервисты", format!("{res:.2} млн"), kit::PLAIN),
        Stat::new("helmet", "Служат", format!("{serving:.2} млн"), kit::PLAIN),
    ];
    let r = kit::stats(ui, &items, 2);
    let notes = [
        "снаряжение без людей на складах: к нему придут резервисты при мобилизации; стареет понемногу",
        "сколько снаряжения из положенного: склады тоже надо обслуживать",
        "отслужившие и ушедшие домой: при мобилизации они придут обученными",
        "сколько людей сейчас в армии и на флоте",
    ];
    for (i, resp) in r.into_iter().enumerate() {
        resp.tip(|| Card::new(items[i].title.clone()).icon(items[i].icon.clone()).row("сейчас", items[i].value.clone()).note(notes[i]));
    }
    if let Some(c) = g.h.war.corps.as_ref() {
        let hold = c.fresh_drill[me];
        if g.mobilised(me) && hold > 0.0 && stored > 0.5 {
            say(ui, s, &format!("Мобилизация идёт: дивизии из запаса выходят обученными ({:.0}% резервистов).", hold * 100.0));
        }
    }
}

/// What the war has cost each side so far.
pub fn war_cost(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    if !g.h.at_war {
        return;
    }
    let me = g.me;
    head(ui, s, "Цена войны");
    let w = g.mil.war_cost[me];
    let f = g.mil.war_cost[1 - me];
    let n = g.h.home(me);
    let gdp = n.gdp.max(1e-9);
    let items = vec![
        Stat::new("coins", "Сверх мирных трат", money(w.money), kit::BAD),
        Stat::new("broken", "Убито", format!("{:.0} тыс.", w.dead * 1000.0), kit::BAD),
        Stat::new("crate", "Снаряжения потеряно", format!("{:.0} {}", w.kit_lost, plural(w.kit_lost.round() as usize, "батальон", "батальона", "батальонов")), kit::MIXED),
        Stat::new("scroll", "Долга взято", money(w.debt), if w.debt > 0.0 { kit::MIXED } else { kit::PLAIN }),
    ];
    let r = kit::stats(ui, &items, 2);
    let foe = [money(f.money), format!("{:.0} тыс.", f.dead * 1000.0), format!("{:.0} батальонов", f.kit_lost), money(f.debt)];
    for (i, resp) in r.into_iter().enumerate() {
        resp.tip(|| {
            let c = Card::new(items[i].title.clone()).icon(items[i].icon.clone()).row("у нас", items[i].value.clone()).row("у врага (по разведке)", foe[i].clone());
            let c = if i == 0 { c.row("месяцев выпуска страны", format!("{:.1}", w.money / gdp)) } else { c };
            c.note(format!("за {} мес. войны", w.months))
        });
    }
    if g.navy_power(me).0 > 0 || g.navy.ships_lost[me] > 0 {
        let line = format!(
            "На море потеряно кораблей: наших {}, вражеских {}; потоплено грузов: наших {}, вражеских {}.",
            g.navy.ships_lost[me],
            g.navy.ships_lost[1 - me],
            money(g.navy.goods_lost[me]),
            money(g.navy.goods_lost[1 - me])
        );
        say(ui, s, &line);
    }
    let _ = chrome::of(ui.ctx());
}
