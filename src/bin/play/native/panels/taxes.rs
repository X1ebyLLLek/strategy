//! The page «Налоги» (35th session): every tax of the state in one place, each as a card that says
//! what it is, who pays it, what it brings the treasury, what it does to the country, with its lever
//! under it — the tiles of its choices, each with the money it would bring a month by last month's
//! figures. Over them: the tax take and its weight on the country, where it comes from, and who pays
//! how much (the poor's share against the rich's — what «flat» or «progressive» means in figures).
//! Before, the tax was one number on the budget page and «who pays» a row among the finances.

use super::*;
use super::super::charts::{num, pct};
use super::super::hud;
use super::super::kit::{self, Stat};
use crate::country::fiscal::{duty_of, estate_of, sales_of, tax_k, Choice, ESTATE, PROFIT, PROFIT_RATE, SALES, TARIFFS, TAXES};
use crate::country::policy::role;

/// The kinds of tax, by colour: income, purchases, profit, property, duties.
const KIND: [(&str, Color32); 5] = [
    ("подоходный", Color32::from_rgb(190, 150, 58)),
    ("с покупок", Color32::from_rgb(70, 130, 176)),
    ("на прибыль", Color32::from_rgb(150, 92, 164)),
    ("на имущество", Color32::from_rgb(82, 150, 108)),
    ("пошлины", Color32::from_rgb(176, 98, 62)),
];

/// The weight of the taxes on the country by the share of its output they take, in words.
fn weight(x: f32) -> (&'static str, Color32) {
    match x {
        x if x < 0.06 => ("лёгкие", kit::GOOD),
        x if x < 0.13 => ("посильные", kit::GOOD),
        x if x < 0.22 => ("заметные", kit::MIXED),
        x if x < 0.32 => ("тяжёлые", kit::BAD),
        _ => ("непосильные", kit::BAD),
    }
}

/// A stretch of bar per part, in the part's colour, its name in it where it fits; the card of a
/// part says its money and its share.
fn bar(ui: &mut egui::Ui, s: &Skin, parts: &[(&str, f32, Color32)]) {
    let (ch, _) = chrome::dress(ui);
    let total: f32 = parts.iter().map(|p| p.1.max(0.0)).sum();
    let w = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 2.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, 30.0), egui::Sense::hover());
    let area = r.shrink2(vec2(0.0, 3.0));
    let mut v = chrome::Out::new();
    v.push(egui::Shape::rect_filled(area, 2.0, Color32::from_black_alpha(34)));
    ch.bevel(&mut v, area, false, 1.0);
    ui.painter().extend(v);
    if total <= 1e-6 {
        ui.painter().text(area.center(), egui::Align2::CENTER_CENTER, "налогов не собрано", egui::FontId::new(13.0, egui::FontFamily::Proportional), s.soft);
        return;
    }
    let mut x = area.min.x;
    let mut labels = Vec::new();
    for (name, val, col) in parts.iter().filter(|p| p.1 > 1e-6) {
        let pw = area.width() * val / total;
        let seg = egui::Rect::from_min_size(egui::pos2(x, area.min.y), vec2(pw, area.height()));
        let resp = ui.interact(seg, ui.id().with(("taxbar", *name)), egui::Sense::hover());
        let mut v = chrome::Out::new();
        v.push(egui::Shape::rect_filled(seg.shrink2(vec2(0.0, 1.0)), 0.0, *col));
        ch.gloss(&mut v, seg.shrink2(vec2(0.0, 1.0)), Color32::from_white_alpha(if resp.hovered() { 90 } else { 50 }), Color32::from_black_alpha(50));
        if x > area.min.x + 0.5 {
            v.push(egui::Shape::line_segment([seg.left_top(), seg.left_bottom()], egui::Stroke::new(1.0, Color32::from_black_alpha(160))));
        }
        ui.painter().extend(v);
        let g = ui.painter().layout_no_wrap(name.to_string(), egui::FontId::new(12.5, egui::FontFamily::Name("bold".into())), widgets::ink_on(*col));
        if g.size().x + 8.0 <= pw {
            labels.push((egui::pos2(seg.center().x - g.size().x / 2.0, seg.center().y - g.size().y / 2.0), g, widgets::ink_on(*col)));
        }
        let (n, m) = (name.to_string(), *val);
        resp.tip(|| Card::new(format!("Налог: {n}")).icon("coins").row("за месяц", num(m)).row("доля всего, что собрано", pct(m / total)));
        x += pw;
    }
    for (p, g, c) in labels {
        ui.painter().galley(p, g, c);
    }
    ui.add_space(2.0);
}

/// Who pays how much: a row for each group of people — its name, a bar of the share of its income
/// the taxes took (in the colours of the taxes), the share in figures. The card of a row: what each
/// tax took of it.
fn payers(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    let me = g.me;
    let n = g.h.home(me);
    let pack = &g.pack;
    let rows: Vec<(usize, f32, [f32; 4], f32)> = (0..pack.groups.len())
        .filter(|&k| n.pop[k] > 1e-6 && n.income[k] + n.taxed_by[k][2] > 1e-6)
        .map(|k| {
            let gross = n.income[k] + n.taxed_by[k][2];
            let by = n.taxed_by[k];
            (k, by.iter().sum::<f32>() / gross, by, gross)
        })
        .collect();
    if rows.is_empty() {
        return;
    }
    let top = rows.iter().map(|r| r.1).fold(0.0f32, f32::max).max(0.12) * 1.08;
    let w = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 2.0);
    let (name_w, fig_w) = (118.0, 46.0);
    for (k, share, by, gross) in rows {
        let (r, resp) = ui.allocate_exact_size(vec2(w, 25.0), egui::Sense::hover());
        let p = ui.painter();
        let mut job = egui::text::LayoutJob::simple_singleline(pack.groups[k].name.clone(), egui::FontId::new(14.0, egui::FontFamily::Proportional), s.ink);
        job.wrap = egui::text::TextWrapping::truncate_at_width(name_w - 6.0);
        let gal = p.layout_job(job);
        p.galley(egui::pos2(r.min.x + 4.0, r.center().y - gal.size().y / 2.0), gal, s.ink);
        let area = egui::Rect::from_min_max(egui::pos2(r.min.x + name_w, r.min.y + 5.0), egui::pos2(r.max.x - fig_w, r.max.y - 5.0));
        p.rect_filled(area, 2.0, Color32::from_black_alpha(26));
        let mut x = area.min.x;
        for (i, &amount) in by.iter().enumerate() {
            let pw = area.width() * (amount / gross / top);
            if pw > 0.4 {
                p.rect_filled(egui::Rect::from_min_size(egui::pos2(x, area.min.y), vec2(pw, area.height())), 0.0, KIND[i].1);
                x += pw;
            }
        }
        let gn = p.layout_no_wrap(pct(share), egui::FontId::new(14.0, egui::FontFamily::Name("bold".into())), s.ink);
        p.galley(egui::pos2(r.max.x - 4.0 - gn.size().x, r.center().y - gn.size().y / 2.0), gn, s.ink);
        p.line_segment([egui::pos2(r.min.x, r.max.y), egui::pos2(r.max.x, r.max.y)], egui::Stroke::new(1.0, s.soft.gamma_multiply(0.22)));
        let name = pack.groups[k].name.clone();
        resp.tip(|| {
            let mut c = Card::new(name).icon("people").sub("налоги с этих людей").row("доход за месяц", num(gross)).row("всего налогов", format!("{} · {}", num(by.iter().sum()), pct(share)));
            for (i, &amount) in by.iter().enumerate() {
                if amount > 1e-6 {
                    c = c.row(KIND[i].0, format!("{} · {}", num(amount), pct(amount / gross)));
                }
            }
            c.note("доля дохода, которую забрали налоги")
        });
    }
    // (the colours of the taxes under the rows)
    let key: Vec<(String, Color32)> = KIND.iter().take(4).map(|(n, c)| (n.to_string(), *c)).collect();
    widgets::legend(ui, s, &key);
}

/// A good and a bad of a tax, one line each.
fn pro_con(ui: &mut egui::Ui, s: &Skin, pro: &str, con: &str) {
    for (mark, text, col) in [("+", pro, kit::GOOD), ("−", con, kit::BAD)] {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let (r, _) = ui.allocate_exact_size(vec2(14.0, 16.0), egui::Sense::hover());
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, mark, egui::FontId::new(16.0, egui::FontFamily::Name("bold".into())), hud::lighten(col, 0.12));
            kit::words(ui, text.to_string(), 13.0, s.ink);
        });
    }
}

/// One tax as a card of paper: its heading (icon, name, what it took last month and its share of the
/// take), who pays, what it gives and what it costs; `lever` draws its controls inside.
#[allow(clippy::too_many_arguments)]
fn card(ui: &mut egui::Ui, s: &Skin, colour: Color32, icon: &str, title: &str, took: f32, of_take: f32, payers: &str, pro: &str, con: &str, lever: impl FnOnce(&mut egui::Ui)) {
    kit::framed(ui, Some(colour), false, |ui| {
        ui.horizontal(|ui| {
            let (ir, _) = ui.allocate_exact_size(vec2(28.0, 28.0), egui::Sense::hover());
            if ui.is_rect_visible(ir) {
                kit::socket(ui, ir.center(), 13.0, icon, colour, 0.3);
            }
            kit::bold(ui, title.to_string(), 15.5, s.ink);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let word = if took < 1e-6 { "не взимается".to_string() } else { format!("{} · {}", num(took), pct(of_take)) };
                kit::bold(ui, word, 14.0, if took < 1e-6 { s.soft } else { s.ink });
            });
        });
        kit::words(ui, format!("Платят: {payers}"), 13.0, s.soft);
        ui.add_space(2.0);
        pro_con(ui, s, pro, con);
        ui.add_space(3.0);
        lever(ui);
    });
}

/// The tiles of a lever of four choices; each tile's card gives the money it would bring a month by
/// last month's figures, what it would change against now, and what it means. The choice made, if
/// another was clicked.
#[allow(clippy::too_many_arguments)]
fn tiles(ui: &mut egui::Ui, title: &str, list: &'static [Choice; 4], short: [&str; 4], chosen: usize, usual: usize, estimate: &dyn Fn(usize) -> f32, who: &dyn Fn(usize) -> Vec<(String, f32)>, gdp: f32, now: f32) -> Option<usize> {
    let marks: Vec<(&str, &str)> = (0..4).map(|k| (short[k], list[k].icon)).collect();
    widgets::law(ui, &marks, chosen, usual, &|k| {
        let m = estimate(k);
        let mut c = Card::new(format!("{title}: {}", list[k].name))
            .icon(list[k].icon)
            .text(list[k].about)
            .row("≈ в казну в месяц", num(m))
            .row("от выпуска", format!("{:.1}%", m / gdp * 100.0))
            .row("против нынешнего", format!("{}{}", if m >= now { "+" } else { "−" }, num((m - now).abs())));
        let rows = who(k);
        if !rows.is_empty() {
            c = c.rule().head("Доля дохода, что заберёт налог");
            for (name, x) in rows {
                c = c.row(name, format!("{:.1}%", x * 100.0));
            }
        }
        c.note(if k == chosen { "действует сейчас" } else { "щёлкните — решение уйдёт в правительство" })
    })
}

/// The groups of people with their income before tax (what a tax's share is a share of), for the cards of the tiles.
fn by_group(n: &strategy::economy::nation::Nation, pack: &strategy::pack::Pack, part: &dyn Fn(usize) -> f32) -> Vec<(String, f32)> {
    (0..pack.groups.len())
        .filter(|&g| n.pop[g] > 1e-6 && n.income[g] + n.taxed_by[g][2] > 1e-6)
        .map(|g| (pack.groups[g].name.clone(), part(g) / (n.income[g] + n.taxed_by[g][2])))
        .collect()
}

/// The page «Налоги».
pub(super) fn taxes(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    let (ng, nk) = (pack.groups.len(), pack.goods.len());
    let n = g.h.home(me).clone();
    let gdp = n.gdp.max(1e-6);
    let fx = g.fiscal.clone();
    let mine = g.adviser.is_none();
    let b = g.rows.last().map(|r| r.book[me]).unwrap_or_default();
    let kinds = [b.kinds[0], b.kinds[1], b.kinds[2], b.kinds[3], b.duty];
    let take: f32 = kinds.iter().sum();
    let (word, tone) = weight(take / gdp);
    let ev = g.fiscal.evaded[me];
    let conf = g.fiscal.confidence[me];
    kit::ribbon(ui, "Налоги страны", Some("coins"));
    let items = vec![
        Stat::new("coins", "Собрано в месяц", format!("{:.0}% выпуска", take / gdp * 100.0), kit::PLAIN).num(num(take)),
        Stat::new("balance", "Тяжесть налогов", word, tone),
        Stat::new("hand", "Уклонение", if ev < 0.03 { "почти нет" } else if ev < 0.12 { "заметное" } else { "повальное" }, if ev > 0.12 { kit::BAD } else { kit::PLAIN }),
        Stat::new("sprout", "Деловой климат", if conf < 0.6 { "паника" } else if conf < 0.85 { "тревожный" } else if conf < 1.15 { "спокойный" } else { "подъём" }, if conf < 0.85 { kit::BAD } else if conf > 1.15 { kit::GOOD } else { kit::PLAIN }),
    ];
    let resp = kit::stats(ui, &items, 2);
    let cards: [Box<dyn Fn() -> Card>; 4] = [
        Box::new(move || Card::new("Собрано в месяц").icon("coins").row("всего", num(take)).row("от выпуска страны", pct(take / gdp)).text("Все налоги и пошлины за прошлый месяц. Больше берёшь — богаче казна, беднее те, с кого берут.")),
        Box::new(move || Card::new("Тяжесть налогов").icon("balance").row("доля выпуска", pct(take / gdp)).text("Чем тяжелее налоги, тем больше люди прячут доходы, тем меньше строят хозяева и тем сильнее ропот.")),
        Box::new(move || Card::new("Уклонение").icon("hand").row("не доплачивают", pct(ev)).text("Растёт с подоходным налогом и слабой властью, падает от управления и порядка. Больше всех прячут богатые.")),
        Box::new(move || Card::new("Деловой климат").icon("sprout").row("уверенность собственников", pct(conf)).text("Налоги на прибыль и имущество пугают хозяев: они строят меньше и увозят деньги за границу.")),
    ];
    for (r, c) in resp.into_iter().zip(cards.iter()) {
        r.tip(|| c());
    }
    say(ui, s, "Пять налогов. У каждого свой плательщик, свой доход казны и своя цена для страны. Выберите плитку: на ней — сколько это даст казне в месяц по цифрам прошлого месяца.");
    kit::ribbon(ui, "Откуда деньги", Some("scroll"));
    let parts: Vec<(&str, f32, Color32)> = (0..5).map(|k| (KIND[k].0, kinds[k], KIND[k].1)).collect();
    bar(ui, s, &parts);
    kit::ribbon(ui, "Кто сколько платит", Some("people"));
    say(ui, s, "Доля дохода каждой группы, которую забрали налоги. Если бедные платят долю больше богатых — налоги ложатся тяжелее на них.");
    payers(ui, s, g);

    // ---- the income tax: the rate, and who pays it
    let tax = n.tax;
    let usual = pack.tax;
    kit::ribbon(ui, "Подоходный налог", Some("stamper"));
    let took = kinds[0];
    let pay_words = match fx.tax[me] {
        1 => "богатые вдвое-втрое больше бедных",
        2 => "почти одни собственники",
        3 => "бедные больше богатых",
        _ => "все одной долей дохода",
    };
    card(
        ui,
        s,
        KIND[0].1,
        "stamper",
        "Налог с доходов",
        took,
        took / take.max(1e-6),
        pay_words,
        "самый крупный и предсказуемый доход казны: ставкой управляют быстро",
        "каждый пункт — меньше денег у людей и хозяев: меньше покупают и строят; выше 20% начинают прятать доходы",
        |ui| {
            let (d, _) = super::levers::step(ui, s, "coins", "Ставка", &format!("{}%", (tax * 100.0).round()), mine, &format!("обычная в эпохе — {:.0}%. Шаг в 1 пункт даёт ≈ {}{} в месяц.", usual * 100.0, if tax > 0.0 { "+" } else { "" }, num(gdp * 0.01)));
            if d != 0 && !super::super::spoiled() {
                act(g, &[("tax", format!("{:.0}", ((tax * 100.0).round() + d as f32).clamp(0.0, 80.0)))]);
            }
            super::levers::rule(ui, s);
            kit::bold(ui, "С кого брать", 14.0, s.ink);
            let est = |k: usize| -> f32 { tax_estimate(&n, &pack, k, tax) };
            let now = est(fx.tax[me]);
            let who = |k: usize| by_group(&n, &pack, &|gr| n.income[gr].max(0.0) * (tax * if k == 0 { 1.0 } else { tax_k(k, role(&pack, gr)) }).clamp(0.0, 0.95));
            if let Some(k) = tiles(ui, "Подоходный", &TAXES, ["Плоский", "Лестница", "Роскошь", "Подушный"], fx.tax[me], 0, &est, &who, gdp, now) {
                act(g, &[("fx_tax", k.to_string())]);
            }
            let mut line = String::new();
            for gr in 0..ng {
                if n.pop[gr] > 1e-6 && n.unemployed[gr].is_some() {
                    let k = if fx.tax[me] == 0 { 1.0 } else { tax_k(fx.tax[me], role(&pack, gr)) };
                    if !line.is_empty() {
                        line.push_str(", ");
                    }
                    line.push_str(&format!("{} — {:.0}%", pack.groups[gr].name.to_lowercase(), (tax * k).clamp(0.0, 0.95) * 100.0));
                }
            }
            kit::words(ui, format!("Ставка для каждой группы: {line}."), 12.5, s.soft);
        },
    );

    // ---- the tax at the till
    let at = |k: usize| -> f32 { (0..ng).map(|gr| (0..nk).map(|j| n.spent[gr][j] * sales_of(&pack, k, j)).sum::<f32>()).sum() };
    let now = at(fx.sales[me]);
    card(
        ui,
        s,
        KIND[1].1,
        "shop",
        "Налог с покупок",
        kinds[1],
        kinds[1] / take.max(1e-6),
        "все, кто покупает: больше тратит — больше платит",
        "собирается без суеты, трудно спрятать: богатые платят с роскоши",
        "бедные тратят на покупки всё, поэтому доля их дохода выше; покупают меньше — заводам хуже",
        |ui| {
            let who = |k: usize| by_group(&n, &pack, &|gr| (0..nk).map(|j| n.spent[gr][j] * sales_of(&pack, k, j)).sum());
            if let Some(k) = tiles(ui, "Налог с покупок", &SALES, ["Нет", "Акциз 5%", "Акциз 12%", "На всё 10%"], fx.sales[me], 0, &at, &who, gdp, now) {
                act(g, &[("fx_sales", k.to_string())]);
            }
        },
    );

    // ---- the tax on profit
    let at = |k: usize| -> f32 { n.payout * PROFIT_RATE[k] };
    let now = at(fx.profit[me]);
    card(
        ui,
        s,
        KIND[2].1,
        "gears",
        "Налог на прибыль",
        kinds[2],
        kinds[2] / take.max(1e-6),
        "собственники заводов и земель — с того, что заводы им выплачивают",
        "бедных и рабочих не касается; казне — много, когда заводов много",
        "у владельцев меньше денег на новые заводы: стройка стихает, деловой климат падает",
        |ui| {
            // (split between the owners' groups as the tax now falls on them; evenly while it falls on none)
            let paid: f32 = n.taxed_by.iter().map(|t| t[2]).sum();
            let owners = (0..ng).filter(|&o| role(&pack, o) == 3).count().max(1) as f32;
            let who = |k: usize| -> Vec<(String, f32)> {
                by_group(&n, &pack, &|gr| if role(&pack, gr) == 3 { n.payout * PROFIT_RATE[k] * if paid > 1e-6 { n.taxed_by[gr][2] / paid } else { 1.0 / owners } } else { 0.0 }).into_iter().filter(|x| x.1 > 0.0).collect()
            };
            if let Some(k) = tiles(ui, "Налог на прибыль", &PROFIT, ["Нет", "10%", "20%", "35%"], fx.profit[me], 0, &at, &who, gdp, now) {
                act(g, &[("fx_profit", k.to_string())]);
            }
        },
    );

    // ---- the tax on property
    let at = |k: usize| -> f32 { (0..ng).map(|gr| n.money[gr].max(0.0) * estate_of(k, role(&pack, gr))).sum() };
    let now = at(fx.estate[me]);
    card(
        ui,
        s,
        KIND[3].1,
        "village",
        "Налог на имущество",
        kinds[3],
        kinds[3] / take.max(1e-6),
        "у кого накоплены деньги: больше всего у собственников",
        "бьёт по накопленному, а не по заработанному: беднякам почти ничего",
        "собственники прячут и вывозят деньги; тяжёлый налог на всех — люди без сбережений на чёрный день",
        |ui| {
            let who = |k: usize| by_group(&n, &pack, &|gr| n.money[gr].max(0.0) * estate_of(k, role(&pack, gr)));
            if let Some(k) = tiles(ui, "Налог на имущество", &ESTATE, ["Нет", "Капитал", "Тяжёлый", "На всех"], fx.estate[me], 0, &at, &who, gdp, now) {
                act(g, &[("fx_estate", k.to_string())]);
            }
        },
    );

    // ---- duties
    let at = |k: usize| -> f32 { (0..nk).map(|j| duty_of(&pack, k, j) * n.import_sold[j] * n.price[j]).sum() };
    let now = at(fx.tariff[me]);
    card(
        ui,
        s,
        KIND[4].1,
        "wall",
        "Пошлины на ввоз",
        kinds[4],
        kinds[4] / take.max(1e-6),
        "покупатели привозного: цена чужих товаров растёт на пошлину",
        "защищают свои заводы и дают казне деньги с чужих товаров",
        "всё привозное дороже; высокие пошлины душат торговлю и злят соседей",
        |ui| {
            if let Some(k) = tiles(ui, "Пошлины", &TARIFFS, ["Свободно", "Умеренные", "Защита", "Автаркия"], fx.tariff[me], 0, &at, &|_| Vec::new(), gdp, now) {
                act(g, &[("fx_tariff", k.to_string())]);
            }
        },
    );
    ui.add_space(8.0);
}

/// What the income tax would bring at rate `tax` if its «who pays» were `kind`, by last month's
/// incomes (what people earned before tax).
fn tax_estimate(n: &strategy::economy::nation::Nation, pack: &strategy::pack::Pack, kind: usize, tax: f32) -> f32 {
    (0..pack.groups.len())
        .map(|g| {
            let k = if kind == 0 { 1.0 } else { tax_k(kind, role(pack, g)) };
            n.income[g].max(0.0) * (tax * k).clamp(0.0, 0.95)
        })
        .sum()
}
