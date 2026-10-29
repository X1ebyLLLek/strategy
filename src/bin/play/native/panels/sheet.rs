//! The economy's first page (28th session: «человеку чрезмерно неудобно взаимодействовать»): the
//! treasury's month as a sheet — what it took in and paid out, line by line, each with its number
//! and a bar of its share; the levers of the money (the tax, the army's, science's and building's
//! shares) stand in the very line they change, as − and + with the number between, and the card of
//! each says what one step is worth a month. Before, the same levers were five-tile laws scattered
//! over a folder of four screens, the treasury a bar without numbers.

use super::*;
use super::super::charts::num;
use super::super::kit::{self, Stat};
use super::super::reports::glow;
use super::super::hud;

/// One line of the sheet: its name and a note after it, the money, a bar of its share of `of`
/// behind it; − and + with `step` between them when the line has a lever (greyed out when it is not
/// the player's). The line's response (for its card) and the step pressed (−1, 0, +1).
#[allow(clippy::too_many_arguments)]
fn line(ui: &mut egui::Ui, s: &Skin, name: &str, note: &str, money: f32, of: f32, colour: Color32, step: Option<(&str, bool)>) -> (egui::Response, i8) {
    line_marked(ui, s, name, note, money, of, colour, step, None)
}

/// `line`, its + marked for the window's self-checks under `mark` (when in sight).
#[allow(clippy::too_many_arguments)]
fn line_marked(ui: &mut egui::Ui, s: &Skin, name: &str, note: &str, money: f32, of: f32, colour: Color32, step: Option<(&str, bool)>, mark: Option<&'static str>) -> (egui::Response, i8) {
    // (as wide as the folder's sight, not as its widest line: the levers stay in view)
    let w = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 2.0);
    let lever_w = 104.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, 27.0), egui::Sense::hover());
    let body = egui::Rect::from_min_max(r.min, egui::pos2(r.max.x - lever_w, r.max.y));
    let p = ui.painter();
    // (the bar of its share behind the words; a hairline under each line)
    let k = if of > 0.0 { (money / of).clamp(0.0, 1.0) } else { 0.0 };
    if k > 0.0 {
        let bar = egui::Rect::from_min_size(body.min + vec2(0.0, 3.0), vec2((body.width() * k).max(2.0), body.height() - 6.0));
        p.rect_filled(bar, 2.0, colour.gamma_multiply(0.28));
        p.rect_filled(egui::Rect::from_min_size(bar.min, vec2(3.0, bar.height())), 1.0, colour);
    }
    p.line_segment([egui::pos2(r.min.x, r.max.y), egui::pos2(r.max.x, r.max.y)], egui::Stroke::new(1.0, s.soft.gamma_multiply(0.25)));
    let money_txt = if money.abs() < 0.005 { "—".to_string() } else { num(money) };
    let gm = p.layout_no_wrap(money_txt, egui::FontId::new(15.0, egui::FontFamily::Name("bold".into())), s.ink);
    let mx = body.max.x - 6.0 - gm.size().x;
    p.galley(egui::pos2(mx, r.center().y - gm.size().y / 2.0), gm, s.ink);
    let gn = p.layout_no_wrap(name.to_string(), egui::FontId::new(14.5, egui::FontFamily::Proportional), s.ink);
    let nx = body.min.x + 8.0;
    let nw = gn.size().x;
    p.galley(egui::pos2(nx, r.center().y - gn.size().y / 2.0), gn, s.ink);
    if !note.is_empty() {
        let room = mx - nx - nw - 14.0;
        if room > 20.0 {
            let mut job = egui::text::LayoutJob::simple_singleline(note.to_string(), egui::FontId::new(12.5, egui::FontFamily::Proportional), s.soft);
            job.wrap = egui::text::TextWrapping::truncate_at_width(room);
            let g = p.layout_job(job);
            p.galley(egui::pos2(nx + nw + 6.0, r.center().y - g.size().y / 2.0 + 1.0), g, s.soft);
        }
    }
    let mut out = 0i8;
    if let Some((what, enabled)) = step {
        let lr = egui::Rect::from_min_max(egui::pos2(r.max.x - lever_w + 4.0, r.min.y), egui::pos2(r.max.x, r.max.y));
        let (d, plus) = ui.allocate_new_ui(egui::UiBuilder::new().max_rect(lr), |ui| widgets::stepper(ui, what, lr.width(), enabled)).inner;
        if let Some(m) = mark.filter(|_| ui.clip_rect().contains_rect(plus)) {
            chrome::spot(ui.ctx(), m, plus);
        }
        out = d;
    }
    (resp, out)
}

/// A heading of a part of the sheet, with the part's total at the right.
fn total(ui: &mut egui::Ui, s: &Skin, name: &str, money: f32, colour: Color32) {
    let w = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 2.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), egui::Sense::hover());
    // (33rd session: a plaque with cut corners, the colour glowing from its left, a rule of trim under it)
    let mut v = chrome::Out::new();
    hud::plaque(&mut v, r, 6.0, colour.gamma_multiply(0.14), egui::Stroke::new(1.0, colour.gamma_multiply(0.55)), 0);
    hud::glow(&mut v, egui::Rect::from_min_max(r.min + vec2(6.0, 1.0), egui::pos2(r.min.x + r.width() * 0.6, r.max.y - 1.0)), colour.gamma_multiply(0.30));
    v.push(egui::Shape::rect_filled(egui::Rect::from_min_size(r.min + vec2(0.0, 5.0), vec2(4.0, r.height() - 5.0)), 0.0, colour));
    ui.painter().extend(v);
    let p = ui.painter();
    let g = p.layout_no_wrap(name.to_uppercase(), egui::FontId::new(14.5, egui::FontFamily::Name("head".into())), s.ink);
    p.galley(egui::pos2(r.min.x + 12.0, r.center().y - g.size().y / 2.0), g, s.ink);
    let g = p.layout_no_wrap(num(money), egui::FontId::new(15.5, egui::FontFamily::Name("bold".into())), s.ink);
    p.galley(egui::pos2(r.max.x - 104.0 - 6.0 - g.size().x, r.center().y - g.size().y / 2.0), g, s.ink);
}

/// The treasury's month as two bars of one scale — what came in over what went out: each source and
/// each line a stretch in its colour (its name in it where it fits), the card of a stretch with its
/// number and share. A glance says whether the month left something over, and who took the most.
fn flows(ui: &mut egui::Ui, s: &Skin, income: &[(&str, f32, Color32)], spent: &[(&str, f32, Color32)]) {
    let (ch, _) = chrome::dress(ui);
    let all_in: f32 = income.iter().map(|x| x.1.max(0.0)).sum();
    let all_out: f32 = spent.iter().map(|x| x.1.max(0.0)).sum();
    let top = all_in.max(all_out).max(1e-6);
    let label_w = 62.0;
    let w = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 2.0);
    for (title, parts, total) in [("приходит", income, all_in), ("уходит", spent, all_out)] {
        let (r, _) = ui.allocate_exact_size(vec2(w, 30.0), egui::Sense::hover());
        ui.painter().text(egui::pos2(r.min.x + 1.0, r.center().y), egui::Align2::LEFT_CENTER, title.to_uppercase(), egui::FontId::new(12.5, egui::FontFamily::Name("head".into())), s.soft);
        let area = egui::Rect::from_min_max(egui::pos2(r.min.x + label_w, r.min.y + 3.0), egui::pos2(r.max.x, r.max.y - 3.0));
        let mut v = chrome::Out::new();
        v.push(egui::Shape::rect_filled(area, 2.0, Color32::from_black_alpha(34)));
        ch.bevel(&mut v, area, false, 1.0);
        ui.painter().extend(v);
        let mut x = area.min.x;
        let mut labels = Vec::new();
        for (name, val, col) in parts.iter().filter(|p| p.1 > 1e-4) {
            let pw = area.width() * val / top;
            let seg = egui::Rect::from_min_size(egui::pos2(x, area.min.y), vec2(pw, area.height()));
            let resp = ui.interact(seg, ui.id().with(("flow", title, *name)), egui::Sense::hover());
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
            resp.tip(|| Card::new(n).icon("coins").row("за месяц", num(m)).row(format!("доля того, что {title}"), format!("{:.0}%", m / total.max(1e-6) * 100.0)));
            x += pw;
        }
        for (p, g, c) in labels {
            ui.painter().galley(p, g, c);
        }
    }
    ui.add_space(2.0);
}

/// The page «Бюджет»: the treasury in figures, the month's sheet with its levers.
pub(super) fn sheet(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let n = g.h.home(me);
    let b = g.rows.last().map(|r| r.book[me]).unwrap_or_default();
    let income = b.income();
    let spent = b.spent();
    let net = income - spent;
    let treasury = n.treasury;
    let gdp = n.gdp.max(1e-6);
    let budget = treasury.max(0.0) / strategy::economy::nation::STATE_RESERVE;
    let (tax, army, sci, build) = (n.tax, n.mil_share, n.science_share, n.build_share);
    let mine = g.adviser.is_none();
    let labs = g.h.labs[me].is_some();
    // the treasury in four figures
    {
        let months = if spent > 1e-6 { treasury / spent } else { f32::INFINITY };
        let items = vec![
            Stat::new("coins", "В казне", if treasury < 0.0 { "в долгах" } else if months < 1.0 { "почти пусто" } else if months < 6.0 { "запас" } else { "полна" }, if treasury < 0.0 { kit::BAD } else if months < 1.0 { kit::MIXED } else { kit::GOOD }).num(num(treasury)),
            Stat::new("sprout", "Доходы в месяц", format!("{:.0}% выпуска", income / gdp * 100.0), kit::PLAIN).num(num(income)),
            Stat::new("hand", "Расходы в месяц", "", kit::PLAIN).num(num(spent)),
            Stat::new("balance", "Итог месяца", if net >= 0.0 { "в плюс" } else { "в минус" }, if net >= 0.0 { kit::GOOD } else { kit::BAD }).num(format!("{}{}", if net >= 0.0 { "+" } else { "" }, num(net))),
        ];
        let r = kit::stats(ui, &items, 2);
        let tips: [String; 4] = [
            format!("Запас казны; при нынешних тратах его хватит на {}.", if months.is_finite() { format!("{months:.0} мес.") } else { "сколько угодно".into() }),
            "Налоги, пошлины, доходы казённых заводов за прошлый месяц.".into(),
            "Армия, наука, стройка, гражданские статьи, долг и раздача людям за прошлый месяц.".into(),
            "Доходы минус расходы: на столько выросла (упала) казна.".into(),
        ];
        for (i, resp) in r.into_iter().enumerate() {
            let t = items[i].title.clone();
            let tip = tips[i].clone();
            resp.tip(|| Card::new(t).icon("coins").text(tip));
        }
    }
    if g.rows.is_empty() {
        say(ui, s, "Итоги первого месяца появятся через месяц.");
    }
    say(ui, s, if mine { "Статью меняют кнопки «−» и «+» в её строке; подсказка скажет, сколько это денег в месяц." } else { "Налог и стройку ведёт [adviser]советник; армию и науку решаете вы." });
    ui.add_space(3.0);
    flows(
        ui,
        s,
        &[("налоги", b.tax, kit::GOOD), ("пошлины", b.duty, hud::lighten(kit::GOOD, 0.25)), ("заводы казны", b.sectors, hud::darken(kit::GOOD, 0.2)), ("прочее", b.other.max(0.0), kit::PLAIN)],
        &[
            ("армия", b.army, Color32::from_rgb(176, 58, 42)),
            ("жалованье", b.soldiers, Color32::from_rgb(150, 70, 52)),
            ("наука", b.science, Color32::from_rgb(86, 130, 196)),
            ("стройка", b.build + b.subsidy, kit::MIXED),
            ("гражданские", b.civil, Color32::from_rgb(120, 110, 160)),
            ("долг", b.bonds, Color32::from_rgb(110, 110, 110)),
            ("раздача", b.dole, kit::PLAIN),
            ("прочее", (-b.other).max(0.0), hud::lighten(kit::PLAIN, 0.2)),
        ],
    );
    let top = income.max(spent).max(1e-6);
    let green = kit::GOOD;
    // what the treasury took in
    total(ui, s, "Доходы", income, green);
    let step_tax = gdp * 0.01;
    let (r, d) = line_marked(ui, s, "Налоги", "", b.tax, top, green, Some((&format!("{}%", (tax * 100.0).round()), mine)), Some("tax_plus"));
    glow(ui, r.rect, "tax");
    r.tip(|| Card::new("Налог с доходов").icon("coins").row("взято за месяц", num(b.tax)).row("налог", format!("{:.0}%", tax * 100.0)).row("+1 пункт налога", format!("≈ +{} в месяц", num(step_tax))).text("Больше налог — больше казна, но у людей и хозяев меньше денег: меньше покупают, меньше строят; тяжёлый налог злит тех, кто его платит.").note(if mine { "− и + справа: шаг — 1 пункт" } else { "налог ведёт советник" }));
    // (the spoiled control of the self-checks: the + of the tax does nothing)
    if d != 0 && !super::super::spoiled() {
        act(g, &[("tax", format!("{:.0}", ((tax * 100.0).round() + d as f32).clamp(0.0, 80.0)))]);
    }
    let (r, _) = line(ui, s, "Пошлины", "", b.duty, top, green, None);
    r.tip(|| Card::new("Пошлины").icon("backforth").row("за месяц", num(b.duty)).note("политика пошлин — на странице «Деньги»"));
    if b.sectors > 0.005 {
        let (r, _) = line(ui, s, "Казённые заводы", "", b.sectors, top, green, None);
        r.tip(|| Card::new("Казённые заводы").icon("gears").row("за месяц", num(b.sectors)).note("доля прибыли отраслей, которыми владеет казна («Деньги»)"));
    }
    if b.other > 0.005 {
        let (r, _) = line(ui, s, "Прочее", "займы, события", b.other, top, green, None);
        r.tip(|| Card::new("Прочие доходы").icon("scroll").row("за месяц", num(b.other)).note("займы, станок, события месяца"));
    }
    ui.add_space(6.0);
    // what it paid out
    let red = kit::BAD;
    total(ui, s, "Расходы", spent, red);
    let step_army = budget * 0.02;
    let (r, d) = line(ui, s, "Армия", "снаряжение", b.army, top, Color32::from_rgb(176, 58, 42), Some((&format!("{}%", (army * 100.0).round()), true)));
    glow(ui, r.rect, "army");
    r.tip(|| Card::new("Армии из трат казны").icon("army").row("на снаряжение за месяц", num(b.army)).row("жалованье", num(b.soldiers)).row("доля трат казны", format!("{:.0}%", army * 100.0)).row("+2 пункта", format!("≈ +{} в месяц армии", num(step_army))).text("Армия получает свою долю того, что казна тратит за месяц (половину запаса казны); на эти деньги заводы делают снаряжение и платят жалованье.").note("− и + справа: шаг — 2 пункта"));
    if d != 0 {
        act(g, &[("army", format!("{:.0}", ((army * 100.0).round() + 2.0 * d as f32).clamp(0.0, 100.0)))]);
    }
    if b.soldiers > 0.005 {
        let (r, _) = line(ui, s, "Армия", "жалованье", b.soldiers, top, Color32::from_rgb(150, 70, 52), None);
        r.tip(|| Card::new("Жалованье").icon("army").row("за месяц", num(b.soldiers)).note("сколько получают служащие — закон о призыве и кадровые офицеры (папка «Армия», страница «Бюджет»)"));
    }
    if labs {
        let step_sci = budget * 0.01;
        let (r, d) = line(ui, s, "Наука", "", b.science, top, Color32::from_rgb(86, 130, 196), Some((&format!("{:.0}%", sci * 100.0), true)));
        glow(ui, r.rect, "science");
        r.tip(|| Card::new("Наука").icon("science").row("за месяц", num(b.science)).row("доля трат казны", format!("{:.0}%", sci * 100.0)).row("+1 пункт", format!("≈ +{} в месяц", num(step_sci))).text("Жалованье учёных: чем лучше платят против другой работы, тем больше образованных идёт в лаборатории.").note("− и + справа: шаг — 1 пункт"));
        if d != 0 {
            act(g, &[("science", format!("{:.1}", (sci * 100.0 + d as f32).clamp(0.0, 50.0)))]);
        }
    }
    let rest = (budget * (1.0 - army - sci)).max(0.0);
    let (r, d) = line(ui, s, "Стройка", "заводы казны", b.build, top, kit::MIXED, Some((&format!("{}%", (build * 100.0).round()), mine)));
    glow(ui, r.rect, "build");
    r.tip(|| Card::new("Казна строит").icon("anvil").row("за месяц", num(b.build)).row("доля того, что осталось после армии и науки", format!("{:.0}%", build * 100.0)).row("+10 пунктов", format!("≈ +{} в месяц", num(rest * 0.1))).text("Казна ставит новые заводы в отраслях из списка «Что строит казна» ниже (или там, где людям хуже всего). Строит, сколько есть свободных рук и сырья — лишние деньги остаются людям.").note(if mine { "− и + справа: шаг — 10 пунктов" } else { "стройку ведёт советник" }));
    if d != 0 {
        act(g, &[("build", format!("{:.0}", ((build * 100.0).round() + 10.0 * d as f32).clamp(0.0, 100.0)))]);
    }
    if b.subsidy > 0.005 {
        let (r, _) = line(ui, s, "Субсидии", "", b.subsidy, top, kit::MIXED, None);
        r.tip(|| Card::new("Субсидии").icon("coins").row("за месяц", num(b.subsidy)).note("отраслям, которым казна помогает («Деньги»)"));
    }
    let (r, _) = line(ui, s, "Гражданские статьи", "ниже", b.civil, top, Color32::from_rgb(120, 110, 160), None);
    glow(ui, r.rect, "civil");
    r.tip(|| Card::new("Гражданские статьи").icon("scroll").row("за месяц", num(b.civil)).note("управление, школы, медицина, порядок, призрение — ниже на этой странице"));
    if b.bonds > 0.005 {
        let (r, _) = line(ui, s, "Долг", "проценты и выкуп", b.bonds, top, Color32::from_rgb(110, 110, 110), None);
        r.tip(|| Card::new("Облигации").icon("scroll").row("за месяц", num(b.bonds)).note("проценты и выкуп облигаций у своих («Деньги»)"));
    }
    let (r, _) = line(ui, s, "Раздача людям", "что осталось", b.dole, top, kit::PLAIN, None);
    r.tip(|| Card::new("Раздача людям").icon("people").row("за месяц", num(b.dole)).text("Что казна не потратила на статьи выше, возвращается людям поровну на душу: это не потеря — люди покупают на эти деньги.").note("меньше раздача — больше казна тратит на дело"));
    if b.other < -0.005 {
        let (r, _) = line(ui, s, "Прочее", "программы, займы", -b.other, top, kit::PLAIN, None);
        r.tip(|| Card::new("Прочие расходы").icon("scroll").row("за месяц", num(-b.other)).note("программы, проценты по займам за границей, события"));
    }
    ui.add_space(4.0);
    total(ui, s, if net >= 0.0 { "Итог: казна растёт" } else { "Итог: казна тает" }, net, if net >= 0.0 { green } else { red });
    ui.add_space(8.0);
}
