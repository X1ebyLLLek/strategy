//! The tree of science: a window beside the folder with every technique of the age as a plaque in its
//! tier's column, lines from what a technique rests on to it. What the country knows is lit in brass,
//! what it may work on is open to a click, the one it works on shows its progress, the rest lie dim; a
//! mark says what the rival has that we have not. (33rd session: the folder listed the techniques as
//! rows of a table; the order they come in, and what rests on what, was nowhere to be seen.)

use eframe::egui::{self, epaint, pos2, vec2, Color32, FontFamily, FontId, Rect, RichText, Sense, Shape, Stroke};
use strategy::pack::Sphere;
use strategy::world::progress::{stage_name, Know};

use crate::Game;
use super::chrome::{self, Card, Tip};
use super::hud;
use super::kit;
use super::look::{icon, Skin};

const NODE_W: f32 = 196.0;
const NODE_H: f32 = 70.0;
const COL_GAP: f32 = 30.0;
const ROW_GAP: f32 = 14.0;
const HEAD: f32 = 34.0;

fn open_id() -> egui::Id {
    egui::Id::new("techtree-open")
}

/// Whether the tree is open.
pub fn is_open(ctx: &egui::Context) -> bool {
    // (for pictures: `SHOTTREE=1` opens it at the start)
    ctx.data(|d| d.get_temp::<bool>(open_id())).unwrap_or_else(|| std::env::var_os("SHOTTREE").is_some())
}

pub fn set_open(ctx: &egui::Context, on: bool) {
    ctx.data_mut(|d| d.insert_temp(open_id(), on));
}

fn tone(sp: Sphere) -> Color32 {
    match sp {
        Sphere::Military => kit::BAD,
        Sphere::Economy => kit::MIXED,
        Sphere::Science => Color32::from_rgb(70, 110, 160),
        Sphere::State => Color32::from_rgb(120, 100, 150),
    }
}

/// Where each technique's plaque lies (relative to the tree's corner): its tier's column, its row by the middle
/// of the rows of what it rests on (so that the lines cross as little as may be), the columns centred on the tallest.
fn layout(pack: &strategy::pack::Pack) -> (Vec<Rect>, f32, f32) {
    let top = pack.techs.iter().map(|t| t.tier).max().unwrap_or(0);
    let mut rows: Vec<f32> = vec![0.0; pack.techs.len()];
    let mut cols: Vec<Vec<usize>> = vec![Vec::new(); top + 1];
    for tier in 0..=top {
        let mut here: Vec<usize> = (0..pack.techs.len()).filter(|&t| pack.techs[t].tier == tier).collect();
        let key = |t: usize| -> f32 {
            let n = &pack.techs[t].needs;
            if n.is_empty() {
                // (nothing to rest on: by sphere, so that a kind of technique keeps together)
                100.0 + t as f32 * 0.001
            } else {
                n.iter().map(|&q| rows[q]).sum::<f32>() / n.len() as f32 + t as f32 * 0.001
            }
        };
        here.sort_by(|&a, &b| key(a).total_cmp(&key(b)));
        for (i, &t) in here.iter().enumerate() {
            rows[t] = i as f32;
        }
        cols[tier] = here;
    }
    let tallest = cols.iter().map(|c| c.len()).max().unwrap_or(1) as f32;
    let h = tallest * (NODE_H + ROW_GAP) - ROW_GAP;
    let mut at = vec![Rect::NOTHING; pack.techs.len()];
    for (tier, col) in cols.iter().enumerate() {
        let colh = col.len() as f32 * (NODE_H + ROW_GAP) - ROW_GAP;
        let y0 = HEAD + (h - colh) / 2.0;
        for (i, &t) in col.iter().enumerate() {
            at[t] = Rect::from_min_size(pos2(tier as f32 * (NODE_W + COL_GAP), y0 + i as f32 * (NODE_H + ROW_GAP)), vec2(NODE_W, NODE_H));
        }
    }
    (at, (top + 1) as f32 * (NODE_W + COL_GAP) - COL_GAP, HEAD + h)
}

/// The window of the tree, beside the folder (`x`: its left edge).
pub fn window(ctx: &egui::Context, x: f32, s: &Skin, g: &mut Game) {
    if !is_open(ctx) {
        return;
    }
    let me = g.me;
    let c = g.country(me);
    let o = g.country(1 - me);
    if g.know.len() <= c.max(o) || g.pack.techs.is_empty() {
        return;
    }
    let pack = g.pack.clone();
    let know = g.know[c].clone();
    let rival = g.know[o].clone();
    let seen_v = g.tech_seen(c);
    let rate = g.research_rate(me) * 12.0;
    let led = g.scholar.is_none();
    let ch = chrome::of(ctx);
    let icons = chrome::icons(ctx);
    let shadow = ctx.style().visuals.window_shadow;
    let room_h = (ctx.screen_rect().height() - 70.0 - 120.0).max(260.0);
    let room_w = (ctx.screen_rect().width() - x - 24.0).clamp(420.0, 1180.0);
    let (at, tw, th) = layout(&pack);
    let mut close = false;
    let mut pick: Option<usize> = None;
    egui::Window::new("Древо науки")
        .id(egui::Id::new("techtree"))
        .title_bar(false)
        .anchor(egui::Align2::LEFT_TOP, [x, 66.0])
        .collapsible(false)
        .resizable(false)
        .fixed_size(vec2(room_w, room_h + 40.0))
        .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(22.0, 18.0)).shadow(shadow).rounding(3.0))
        .show(ctx, |ui| {
            let back = chrome::hold(ui);
            ui.set_width(room_w - 44.0);
            // the head: the name, how much is known, the stage; the close button
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("ДРЕВО НАУКИ").family(FontFamily::Name("head".into())).size(24.0).color(s.accent));
                    let stage = know.stage(&pack);
                    ui.label(RichText::new(format!("знаем {} из {} · стадия: {}", know.count(), pack.techs.len(), stage_name(&pack, stage))).size(13.5).color(s.soft));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if let Some(ch) = &ch {
                        let b = ui.add_sized(vec2(30.0, 28.0), |ui: &mut egui::Ui| chrome::press(ui, ch, s, chrome::Press { text: "×", size: vec2(30.0, 28.0), font: 20.0, ..chrome::Press::default() }));
                        if b.tip(|| Card::new("Закрыть древо").text("Esc — то же самое.")).clicked() {
                            close = true;
                        }
                    }
                });
            });
            // the key: what each mark of a plaque says
            ui.add_space(4.0);
            legend(ui, s, &ch);
            ui.add_space(4.0);
            let inner_h = (room_h - 74.0).max(180.0);
            egui::ScrollArea::both().id_salt("techtree-scroll").max_height(inner_h).auto_shrink([false, false]).show(ui, |ui| {
                let (area, _) = ui.allocate_exact_size(vec2(tw.max(ui.available_width()) + 4.0, th + 8.0), Sense::hover());
                let origin = area.min + vec2(((area.width() - tw) / 2.0).max(0.0), 4.0);
                let node = |t: usize| at[t].translate(origin.to_vec2());
                // (the tier headings)
                let top = pack.techs.iter().map(|t| t.tier).max().unwrap_or(0);
                for tier in 0..=top {
                    let cx = origin.x + tier as f32 * (NODE_W + COL_GAP) + NODE_W / 2.0;
                    let all = pack.techs.iter().filter(|t| t.tier == tier).count();
                    let have = (0..pack.techs.len()).filter(|&t| pack.techs[t].tier == tier && know.known[t]).count();
                    let name = if tier == 0 { "ИЗДАВНА".to_string() } else { format!("ЯРУС {tier}") };
                    let gal = ui.painter().layout_no_wrap(name, FontId::new(15.0, FontFamily::Name("head".into())), s.ink);
                    ui.painter().galley(pos2(cx - gal.size().x / 2.0, origin.y + 1.0), gal, s.ink);
                    let sub = ui.painter().layout_no_wrap(format!("{have} из {all}"), FontId::proportional(12.0), s.soft);
                    ui.painter().galley(pos2(cx - sub.size().x / 2.0, origin.y + 18.0), sub, s.soft);
                    if let Some(ch) = &ch {
                        let mut v = chrome::Out::new();
                        hud::fade_rule(&mut v, cx - NODE_W / 2.0, cx + NODE_W / 2.0, origin.y + HEAD - 3.0, ch.trim, 1.4);
                        ui.painter().extend(v);
                    }
                }
                // (the lines, under the plaques: lit where the source is known)
                for t in 0..pack.techs.len() {
                    for &n in &pack.techs[t].needs {
                        let (a, b) = (node(n), node(t));
                        let (p0, p3) = (pos2(a.max.x, a.center().y), pos2(b.min.x, b.center().y));
                        let dx = (p3.x - p0.x).abs().max(20.0) * 0.5;
                        let pts = [p0, p0 + vec2(dx, 0.0), p3 - vec2(dx, 0.0), p3];
                        let lit = know.known[n];
                        let both = lit && know.known[t];
                        let col = if both { ch.as_ref().map_or(s.accent, |c| c.trim) } else if lit { s.accent } else { s.soft.gamma_multiply(0.55) };
                        ui.painter().add(Shape::CubicBezier(epaint::CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::new(if lit { 4.2 } else { 3.0 }, Color32::from_black_alpha(34)))));
                        ui.painter().add(Shape::CubicBezier(epaint::CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, Stroke::new(if both { 2.4 } else if lit { 2.0 } else { 1.4 }, col))));
                    }
                }
                // (the plaques)
                let open = know.open(&pack);
                for t in 0..pack.techs.len() {
                    let tech = &pack.techs[t];
                    let r = node(t);
                    if !ui.is_rect_visible(r.expand(4.0)) {
                        continue;
                    }
                    let resp = ui.interact(r, ui.id().with(("tree-node", t)), if open.contains(&t) && led { Sense::click() } else { Sense::hover() });
                    let known = know.known[t];
                    let working = know.target == Some(t);
                    let can = open.contains(&t);
                    let hot = resp.hovered();
                    let tn = tone(tech.sphere);
                    let mut v = chrome::Out::new();
                    let pts = hud::chamfer(r, 8.0);
                    v.push(epaint::Shadow { offset: vec2(0.0, 2.0), blur: if known { 8.0 } else { 5.0 }, spread: 0.0, color: Color32::from_black_alpha(if known { 80 } else { 48 }) }.as_shape(r, 3.0).into());
                    if let Some(ch) = &ch {
                        let shade = if known { 255 } else if can { 236 } else { 190 };
                        hud::lay_poly(&mut v, &pts, &ch.paper, Color32::from_gray(shade));
                    } else {
                        v.push(Shape::convex_polygon(pts.clone(), s.paper, Stroke::NONE));
                    }
                    if known {
                        hud::glow(&mut v, Rect::from_min_max(r.min + vec2(9.0, 2.0), pos2(r.min.x + r.width() * 0.85, r.max.y - 2.0)), Color32::from_rgb(214, 176, 72).gamma_multiply(0.30));
                    } else if hot && can {
                        hud::glow(&mut v, Rect::from_min_max(r.min + vec2(9.0, 2.0), pos2(r.min.x + r.width() * 0.85, r.max.y - 2.0)), s.accent.gamma_multiply(0.20));
                    }
                    let edge = if working { Stroke::new(2.6, s.accent) } else if known { Stroke::new(1.8, ch.as_ref().map_or(s.edge, |c| c.trim)) } else if can { Stroke::new(if hot { 2.0 } else { 1.3 }, if hot { s.accent } else { s.edge }) } else { Stroke::new(1.0, s.edge.gamma_multiply(0.7)) };
                    v.push(Shape::closed_line(pts, edge));
                    // (the sphere's strip at the left edge)
                    v.push(Shape::rect_filled(Rect::from_min_size(r.min + vec2(1.0, 9.0), vec2(4.0, r.height() - 10.0)), 1.0, tn.gamma_multiply(if known || can { 1.0 } else { 0.5 })));
                    // (the socket of the icon)
                    let sock = pos2(r.min.x + 29.0, r.center().y);
                    if let Some(ch) = &ch {
                        v.push(Shape::circle_filled(sock + vec2(0.0, 1.5), 19.0, Color32::from_black_alpha(100)));
                        hud::ring(&mut v, sock, 18.5, 2.8, if known { hud::lighten(ch.trim, 0.1) } else if can { ch.trim } else { hud::darken(ch.trim, 0.45) });
                        hud::disc(&mut v, sock, 15.7, hud::lerp(Color32::from_rgb(58, 54, 46), tn, if known { 0.55 } else if can { 0.35 } else { 0.12 }), Color32::from_rgb(16, 14, 12));
                    }
                    // (the progress along the foot of the one being worked on)
                    if working {
                        let cost = Know::cost(&pack, t, seen_v.get(t).copied().unwrap_or(0.0)).max(0.05);
                        let done = (know.progress[t] / cost).clamp(0.0, 1.0);
                        let bar = Rect::from_min_max(pos2(r.min.x + 52.0, r.max.y - 9.0), pos2(r.max.x - 10.0, r.max.y - 4.5));
                        v.push(Shape::rect_filled(bar, 2.0, Color32::from_black_alpha(80)));
                        v.push(Shape::rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * done, bar.height())), 2.0, Color32::from_rgb(70, 130, 190)));
                    }
                    ui.painter().extend(v);
                    let ink = if known || can { Color32::WHITE } else { Color32::from_white_alpha(120) };
                    icon(&icons, &tech.icon, 22.0, Color32::from_black_alpha(190)).paint_at(ui, Rect::from_center_size(sock + vec2(1.0, 1.2), vec2(22.0, 22.0)));
                    icon(&icons, &tech.icon, 22.0, ink).paint_at(ui, Rect::from_center_size(sock, vec2(22.0, 22.0)));
                    // (the name: whole, over two lines at most)
                    let tx = r.min.x + 54.0;
                    let name_col = if working { s.accent } else if known || can { s.ink } else { s.soft };
                    // (a long word whole: the font a little smaller when the longest word of the name would be broken)
                    let avail = r.max.x - tx - 22.0;
                    let longest = tech.name.split_whitespace().map(|w| ui.painter().layout_no_wrap(w.to_string(), FontId::new(13.0, FontFamily::Name("bold".into())), name_col).size().x).fold(0.0f32, f32::max);
                    let fs = if longest > avail { 12.0 } else { 13.0 };
                    let mut job = egui::text::LayoutJob::simple(tech.name.clone(), FontId::new(fs, FontFamily::Name("bold".into())), name_col, avail);
                    job.wrap.max_rows = 3;
                    let gal = ui.painter().layout_job(job);
                    let sub = if known { "изучено".to_string() } else if working { "в работе".to_string() } else if can { tech.sphere.word().to_string() } else { "закрыто".to_string() };
                    let subg = ui.painter().layout_no_wrap(sub, FontId::proportional(12.0), if working { s.accent } else { s.soft });
                    let total = gal.size().y + subg.size().y + 1.0;
                    let y0 = r.center().y - total / 2.0 - if working { 3.0 } else { 0.0 };
                    let gh = gal.size().y;
                    ui.painter().galley(pos2(tx, y0), gal, name_col);
                    ui.painter().galley(pos2(tx, y0 + gh + 1.0), subg, s.soft);
                    // (what is known: a check in a brass disc at the corner; what the rival has and we have not: a mark of red)
                    if known {
                        let c = pos2(r.max.x - 9.0, r.min.y + 9.0);
                        let mut v = chrome::Out::new();
                        v.push(Shape::circle_filled(c, 7.5, ch.as_ref().map_or(s.accent, |c| c.trim)));
                        v.push(Shape::circle_stroke(c, 7.5, Stroke::new(1.0, Color32::from_black_alpha(160))));
                        v.push(Shape::line(vec![c + vec2(-3.4, 0.2), c + vec2(-1.0, 2.8), c + vec2(3.6, -2.6)], Stroke::new(2.0, Color32::from_rgb(40, 30, 10))));
                        ui.painter().extend(v);
                    } else if rival.known.get(t).copied().unwrap_or(false) {
                        let c = pos2(r.max.x - 9.0, r.min.y + 9.0);
                        ui.painter().circle_filled(c, 7.5, kit::BAD);
                        ui.painter().circle_stroke(c, 7.5, Stroke::new(1.0, Color32::from_black_alpha(160)));
                        icon(&icons, "binoculars", 11.0, Color32::WHITE).paint_at(ui, Rect::from_center_size(c, vec2(11.0, 11.0)));
                    }
                    // the card
                    let tip = g.tech_tip(t);
                    let world = seen_v.get(t).copied().unwrap_or(0.0);
                    let needs: Vec<String> = tech.needs.iter().map(|&n| format!("{}{}", pack.techs[n].name, if know.known[n] { "" } else { " (нет)" })).collect();
                    let cost = Know::cost(&pack, t, world);
                    let years = ((cost - know.progress[t]).max(0.0) / rate.max(1e-3)).max(0.0);
                    let rival_has = rival.known.get(t).copied().unwrap_or(false);
                    let when = know.when[t];
                    let by = know.by[t].map(|id| g.folk.get(id).name.clone());
                    let clicked = resp.tip(|| {
                        let mut card = Card::new(tech.name.clone()).icon(tech.icon.clone()).sub(format!("{} · ярус {}", tech.sphere.word(), tech.tier)).text(tech.about.clone()).text(tip.clone());
                        if known {
                            card = card.row("освоена", if tech.tier == 0 { "издавна".to_string() } else { strategy::world::folk::year_word(when) });
                            if let Some(b) = &by {
                                card = card.row("открыл", b.clone());
                            }
                        } else {
                            card = card.row("знают в мире", kit::share(world).to_string());
                            card = card.row("осталось", format!("{years:.1} лет обычной науки"));
                        }
                        if !needs.is_empty() {
                            card = card.row("опирается на", needs.join(", "));
                        }
                        if rival_has && !known {
                            card = card.tone("у соседа", "есть", super::chrome::Tone::Bad);
                        }
                        card.note(if known { "уже освоена" } else if can && led { "щёлкните — учёные возьмутся за неё" } else if can { "выбирает глава науки" } else { "сперва — то, на что она опирается" })
                    });
                    if clicked.clicked() && can && led {
                        pick = Some(t);
                    }
                }
            });
            if let Some(ch) = &ch {
                let mut v = chrome::Out::new();
                ch.sheet(&mut v, ui.min_rect().expand2(vec2(22.0, 18.0)), 8.0);
                chrome::under_window(ui, back, v);
            }
        });
    if close {
        set_open(ctx, false);
    }
    if let Some(t) = pick {
        g.act(crate::tools::saves::Act::Form(vec![("tech".to_string(), t.to_string())]));
    }
}

/// The key under the title: a small plaque for each state of a technique, and the rival's mark.
fn legend(ui: &mut egui::Ui, s: &Skin, ch: &Option<chrome::Chrome>) {
    let items: [(&str, u8); 5] = [("знаем", 0), ("можно изучать", 1), ("изучается", 2), ("закрыто", 3), ("есть у соседа, у нас нет", 4)];
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
    let mut x = r.min.x;
    for (name, k) in items {
        let chip = Rect::from_min_size(pos2(x, r.min.y + 3.0), vec2(22.0, 14.0));
        let trim = ch.as_ref().map_or(s.edge, |c| c.trim);
        match k {
            0 => {
                ui.painter().rect_filled(chip, 3.0, Color32::from_rgb(232, 214, 160));
                ui.painter().rect_stroke(chip, 3.0, Stroke::new(1.6, trim));
            }
            1 => {
                ui.painter().rect_filled(chip, 3.0, s.paper2);
                ui.painter().rect_stroke(chip, 3.0, Stroke::new(1.3, s.accent));
            }
            2 => {
                ui.painter().rect_filled(chip, 3.0, s.paper2);
                ui.painter().rect_stroke(chip, 3.0, Stroke::new(2.4, s.accent));
                ui.painter().rect_filled(Rect::from_min_size(chip.min + vec2(3.0, 9.0), vec2(9.0, 3.0)), 1.0, Color32::from_rgb(70, 130, 190));
            }
            3 => {
                ui.painter().rect_filled(chip, 3.0, s.paper2.gamma_multiply(0.6));
                ui.painter().rect_stroke(chip, 3.0, Stroke::new(1.0, s.edge.gamma_multiply(0.7)));
            }
            _ => {
                ui.painter().circle_filled(chip.center(), 7.0, kit::BAD);
            }
        }
        let g = ui.painter().layout_no_wrap(name.to_string(), FontId::proportional(12.5), s.soft);
        ui.painter().galley(pos2(chip.max.x + 6.0, r.center().y - g.size().y / 2.0), g.clone(), s.soft);
        x += 22.0 + 6.0 + g.size().x + 18.0;
    }
}
