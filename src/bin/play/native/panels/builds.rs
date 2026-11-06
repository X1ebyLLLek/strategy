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
    kit::ribbon(ui, "Что строит казна", Some("anvil"));
    kit::words(ui, "Отмеченные отрасли казна строит по очереди; ничего не отмечено — строит, где людям хуже всего. Сверху те, чего не хватает: там завод окупится скорее.", 13.0, s.soft);
    ui.add_space(4.0);
    let mut order: Vec<usize> = (0..pack.sectors.len()).collect();
    order.sort_by(|&a, &b| dearness(g, b).total_cmp(&dearness(g, a)));
    let queue = g.state.queue.clone();
    let mut toggled: Option<usize> = None;
    let full_w = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 2.0);

    for k in order {
        let sec = &pack.sectors[k];
        let n = g.h.home(me);
        let ratio = dearness(g, k);
        let (jobs, busy) = (n.cap.get(k).copied().unwrap_or(0.0), n.worked.get(k).copied().unwrap_or(0.0));
        let now_building = n.build_focus == Some(k);
        let pos_in_queue = queue.iter().position(|&x| x == k);
        let on = pos_in_queue.is_some();

        let edge = if now_building {
            Some(Color32::from_rgb(220, 175, 60))
        } else if on {
            Some(s.accent)
        } else {
            None
        };

        let card_res = kit::framed(ui, edge, now_building, |ui| {
            let (r, resp) = ui.allocate_exact_size(vec2(full_w.max(280.0) - 16.0, 38.0), egui::Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            if ui.is_rect_visible(r) {
                let mut v = chrome::Out::new();
                if resp.hovered() {
                    v.push(egui::Shape::rect_filled(r, 2.0, Color32::from_black_alpha(16)));
                }

                // queue box (shows #1, #2... when queued)
                let b = egui::Rect::from_min_size(egui::pos2(r.min.x + 4.0, r.center().y - 10.0), vec2(20.0, 20.0));
                ch.lay(&mut v, b, &ch.paper, 2.0);
                if on {
                    v.push(egui::Shape::rect_filled(b, 2.0, s.accent));
                    let q_num = format!("#{}", pos_in_queue.unwrap() + 1);
                    let q_gal = ui.painter().layout_no_wrap(q_num, egui::FontId::new(11.0, egui::FontFamily::Name("bold".into())), Color32::WHITE);
                    v.push(egui::Shape::galley(b.center() - q_gal.size() / 2.0, q_gal, Color32::WHITE));
                } else {
                    v.push(egui::Shape::rect_filled(b, 2.0, Color32::from_black_alpha(24)));
                    ch.bevel(&mut v, b, false, 1.0);
                    v.push(egui::Shape::rect_stroke(b, 2.0, egui::Stroke::new(if resp.hovered() { 1.5 } else { 1.0 }, if resp.hovered() { ch.trim } else { Color32::from_black_alpha(120) })));
                }

                // goods icon in socket
                let icon = sec.outputs.first().map_or("gears".to_string(), |o| pack.goods[o.0].icon.clone());
                let sock_pos = egui::pos2(r.min.x + 38.0, r.center().y);
                let tone = if ratio > 1.3 { kit::BAD } else if ratio < 0.77 { kit::GOOD } else { kit::MIXED };
                kit::socket(ui, sock_pos, 13.0, &icon, kit::ink(s, tone), 0.25);

                // gauge and status on the right
                let gw = 56.0f32;
                let right_edge = r.max.x - 4.0;
                let word = if now_building { "строится" } else if ratio > 1.3 { "дефицит" } else if ratio < 0.8 { "избыток" } else { "норма" };
                let wcol = if now_building { kit::ink(s, kit::MIXED) } else if ratio > 1.3 { kit::ink(s, kit::BAD) } else if ratio < 0.8 { kit::ink(s, kit::GOOD) } else { s.soft };
                let word_font = egui::FontId::new(12.0, if ratio > 1.3 || now_building { egui::FontFamily::Name("bold".into()) } else { egui::FontFamily::Proportional });
                let w_gal = ui.painter().layout_no_wrap(word.to_string(), word_font, wcol);
                let word_w = w_gal.size().x;
                v.push(egui::Shape::galley(egui::pos2(right_edge - word_w, r.center().y - w_gal.size().y / 2.0), w_gal, wcol));

                // price ratio text
                let ratio_text = format!("×{:.2}", ratio);
                let r_gal = ui.painter().layout_no_wrap(ratio_text, egui::FontId::new(11.5, egui::FontFamily::Name("bold".into())), kit::ink(s, tone));
                let r_w = r_gal.size().x;
                let r_pos = egui::pos2(right_edge - word_w - r_w - 8.0, r.center().y - r_gal.size().y / 2.0);
                v.push(egui::Shape::galley(r_pos, r_gal, kit::ink(s, tone)));

                // mini price gauge
                let bar = egui::Rect::from_min_size(egui::pos2(r_pos.x - gw - 6.0, r.center().y - 3.5), vec2(gw, 7.0));
                v.push(egui::Shape::rect_filled(bar, 1.5, Color32::from_black_alpha(32)));
                ch.bevel(&mut v, bar, false, 0.8);
                let x = ((ratio.max(1e-3).ln() / 4f32.ln()).clamp(-1.0, 1.0) + 1.0) / 2.0;
                let (mid, at) = (bar.center().x, bar.min.x + bar.width() * x);
                v.push(egui::Shape::rect_filled(egui::Rect::from_min_max(egui::pos2(mid.min(at), bar.min.y + 0.5), egui::pos2(mid.max(at).max(mid.min(at) + 2.0), bar.max.y - 0.5)), 1.0, tone));
                v.push(egui::Shape::line_segment([egui::pos2(mid, bar.min.y - 1.5), egui::pos2(mid, bar.max.y + 1.5)], egui::Stroke::new(1.0, ch.trim)));

                ui.painter().extend(v);

                // title and jobs text in the middle
                let title_x = r.min.x + 58.0;
                let avail_title_w = (bar.min.x - title_x - 6.0).max(60.0);
                let mut title_job = egui::text::LayoutJob::simple_singleline(sec.name.clone(), egui::FontId::new(13.0, egui::FontFamily::Name("bold".into())), s.ink);
                title_job.wrap = egui::text::TextWrapping::truncate_at_width(avail_title_w);
                let t_gal = ui.painter().layout_job(title_job);
                ui.painter().galley(egui::pos2(title_x, r.center().y - 11.0), t_gal, s.ink);

                let sub_text = format!("{jobs:.2} млн мест · занято {:.0}%", busy / jobs.max(1e-9) * 100.0);
                let mut sub_job = egui::text::LayoutJob::simple_singleline(sub_text, egui::FontId::proportional(11.0), s.soft);
                sub_job.wrap = egui::text::TextWrapping::truncate_at_width(avail_title_w);
                let s_gal = ui.painter().layout_job(sub_job);
                ui.painter().galley(egui::pos2(title_x, r.center().y + 1.0), s_gal, s.soft);
            }
            resp
        });

        let resp = card_res.inner;
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
        ui.add_space(2.0);
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
