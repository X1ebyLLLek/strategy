//! «Что строит казна» (29th session: before, a column of check boxes with «×0,79 · в избытке» after
//! each, in the order of the pack): the trades sorted by how short their goods are — those that the
//! country lacks first, where a new factory pays soonest — each with a gauge from cheap to dear around
//! the usual price, and the box to tick it for the treasury's building. Ticked ones are built in turn;
//! with none ticked the treasury builds where people fare worst.

use super::*;
use super::super::kit;

/// The price of a trade's first good against the usual (the usual price follows the level of prices).
fn dearness(g: &Game, k: usize) -> f32 {
    let n = g.h.home(g.me);
    g.pack.sectors[k].outputs.first().map_or(1.0, |o| n.price[o.0] / (g.pack.goods[o.0].price * n.level).max(1e-9))
}

pub(super) fn builds(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    let (ch, _) = chrome::dress(ui);
    let icons = chrome::icons(ui.ctx());
    kit::ribbon(ui, "Что строит казна", Some("anvil"));
    kit::words(ui, "Отмеченные отрасли казна строит по очереди; ничего не отмечено — строит, где людям хуже всего. Сверху те, чего не хватает: там завод окупится скорее.", 13.0, s.soft);
    ui.add_space(3.0);
    let mut order: Vec<usize> = (0..pack.sectors.len()).collect();
    order.sort_by(|&a, &b| dearness(g, b).total_cmp(&dearness(g, a)));
    let queue = g.state.queue.clone();
    let mut toggled: Option<usize> = None;
    let w = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 2.0);
    for k in order {
        let sec = &pack.sectors[k];
        let n = g.h.home(me);
        let ratio = dearness(g, k);
        let (jobs, busy) = (n.cap.get(k).copied().unwrap_or(0.0), n.worked.get(k).copied().unwrap_or(0.0));
        let now_building = n.build_focus == Some(k);
        let on = queue.contains(&k);
        let (r, resp) = ui.allocate_exact_size(vec2(w, 28.0), egui::Sense::click());
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if ui.is_rect_visible(r) {
            let mut v = chrome::Out::new();
            if resp.hovered() {
                v.push(egui::Shape::rect_filled(r, 2.0, Color32::from_black_alpha(14)));
            }
            // (the box)
            let b = egui::Rect::from_min_size(egui::pos2(r.min.x + 3.0, r.center().y - 9.0), vec2(18.0, 18.0));
            ch.lay(&mut v, b, &ch.paper, 2.0);
            v.push(egui::Shape::rect_filled(b, 2.0, Color32::from_black_alpha(26)));
            ch.bevel(&mut v, b, false, 1.2);
            v.push(egui::Shape::rect_stroke(b, 2.0, egui::Stroke::new(if resp.hovered() { 1.6 } else { 1.0 }, if resp.hovered() { ch.trim } else { Color32::from_black_alpha(140) })));
            if on {
                let p = |x: f32, y: f32| egui::pos2(b.min.x + x, b.min.y + y);
                v.push(egui::Shape::line(vec![p(3.5, 9.0), p(7.5, 13.5), p(15.0, 3.5)], egui::Stroke::new(2.8, s.accent)));
            }
            // (the gauge from cheap to dear, the usual price at its middle, on a scale of ×¼ … ×4)
            let gw = (w * 0.30).clamp(80.0, 120.0);
            let bar = egui::Rect::from_min_size(egui::pos2(r.max.x - gw - 70.0, r.center().y - 4.0), vec2(gw, 8.0));
            v.push(egui::Shape::rect_filled(bar, 2.0, Color32::from_black_alpha(34)));
            ch.bevel(&mut v, bar, false, 1.0);
            let x = ((ratio.max(1e-3).ln() / 4f32.ln()).clamp(-1.0, 1.0) + 1.0) / 2.0;
            let col = if ratio > 1.3 { kit::BAD } else if ratio < 0.77 { kit::GOOD } else { kit::MIXED };
            let (mid, at) = (bar.center().x, bar.min.x + bar.width() * x);
            v.push(egui::Shape::rect_filled(egui::Rect::from_min_max(egui::pos2(mid.min(at), bar.min.y + 1.0), egui::pos2(mid.max(at).max(mid.min(at) + 2.0), bar.max.y - 1.0)), 1.0, col));
            v.push(egui::Shape::line_segment([egui::pos2(mid, bar.min.y - 2.0), egui::pos2(mid, bar.max.y + 2.0)], egui::Stroke::new(1.0, ch.trim)));
            ui.painter().extend(v);
            // (its icon and name)
            let icon = sec.outputs.first().map_or("gears".to_string(), |o| pack.goods[o.0].icon.clone());
            super::super::look::icon(&icons, &icon, 18.0, s.ink).paint_at(ui, egui::Rect::from_min_size(egui::pos2(r.min.x + 28.0, r.center().y - 9.0), vec2(18.0, 18.0)));
            let name_w = bar.min.x - (r.min.x + 52.0) - 6.0;
            let mut job = egui::text::LayoutJob::simple_singleline(sec.name.clone(), egui::FontId::proportional(14.0), s.ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(name_w);
            let gal = ui.painter().layout_job(job);
            ui.painter().galley(egui::pos2(r.min.x + 52.0, r.center().y - gal.size().y / 2.0), gal, s.ink);
            // (the word at the right: how short it is; «строится» when the treasury builds it now)
            let word = if now_building { "строится" } else if ratio > 1.3 { "не хватает" } else if ratio < 0.8 { "в избытке" } else { "как обычно" };
            let wcol = if now_building { kit::ink(s, kit::MIXED) } else if ratio > 1.3 { kit::ink(s, kit::BAD) } else { s.soft };
            ui.painter().text(egui::pos2(r.max.x - 2.0, r.center().y), egui::Align2::RIGHT_CENTER, word, egui::FontId::new(12.5, if ratio > 1.3 || now_building { egui::FontFamily::Name("bold".into()) } else { egui::FontFamily::Proportional }), wcol);
        }
        let name = sec.name.clone();
        let resp = resp.tip(|| {
            Card::new(name)
                .icon("gears")
                .tone("цена его товара к обычной", format!("×{}", super::super::charts::fix(ratio, 2)), if ratio > 1.3 { Tone::Bad } else if ratio < 0.77 { Tone::Good } else { Tone::Plain })
                .row("рабочих мест", format!("{jobs:.2} млн"))
                .row("из них заняты", format!("{:.0}%", busy / jobs.max(1e-9) * 100.0))
                .note(if on { "отмечена: казна построит её по очереди — щёлкните, чтобы снять" } else { "дорого — значит, не хватает; щёлкните, чтобы казна строила её" })
        });
        if resp.clicked() {
            toggled = Some(k);
        }
    }
    if let Some(k) = toggled {
        let mut queue = queue;
        if queue.contains(&k) {
            queue.retain(|&x| x != k);
        } else {
            queue.push(k);
        }
        let mut pairs: Vec<(&str, String)> = vec![("queue_set", "1".into())];
        let keys: Vec<String> = queue.iter().map(|k| format!("q{k}")).collect();
        for key in &keys {
            pairs.push((key.as_str(), "1".into()));
        }
        act(g, &pairs);
    }
}
