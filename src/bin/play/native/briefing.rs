//! The war's briefing over the map (29th session): who fights whom, how many divisions each side has
//! on the map, and one line of what to do now — the window gave no answer to "where do I look, what
//! do I command". Over the map's top left, folded to its heading with one click.

use super::*;
use eframe::egui::{vec2, Align2, FontFamily, FontId, Pos2, Rect, Sense, Stroke};

use super::map::stacks;

impl App {
    /// The card of the war, with the side's colours, the counts, and what the player can do next.
    pub(super) fn briefing(&mut self, ctx: &egui::Context, g: &mut Game) {
        if !g.h.at_war || self.tab.is_some() || self.menu {
            return;
        }
        let war = &g.h.war;
        let me = g.me;
        let th = &war.th;
        let s = self.skin;
        let ch = self.chrome.clone();
        let colour_of = |c: usize| {
            let k = g.countries[c].1;
            Color32::from_rgb(k[0], k[1], k[2])
        };
        let colour = |side: usize| colour_of(th.countries[side]);
        let (mut ours, mut hungry, mut fighting) = (0usize, 0usize, 0usize);
        if let Some(c) = war.corps.as_ref() {
            for d in c.divs.iter().filter(|d| d.side == me) {
                ours += 1;
                hungry += (d.supplied < 0.5) as usize;
            }
            fighting = c.fights.len();
        }
        let foe_seen: f32 = stacks(g, &[]).iter().filter(|x| x.side != me).map(|x| x.n).sum();
        let picked = self.map.as_ref().map_or(0, |m| m.picked.len());
        let drawing = self.map.as_ref().is_some_and(|m| m.tool != map::Tool::None);
        let groups = war.command[me].as_ref().map_or(0, |gs| gs.len());
        // what to do now: the next step, by what the player has in hand
        let (hint_head, hint) = if drawing {
            ("Рисуете план", "Щёлкните по вражеской провинции — стрела наступления; Enter — исполнить; Esc — отмена.")
        } else if picked > 0 {
            ("Выбрано дивизий", "ПКМ по своей земле — идти; по врагу — атаковать; H — стоять; Esc — снять выбор.")
        } else if let Some(k) = self.chosen {
            let _ = k;
            ("Армия выбрана", "A — стрела наступления; F — протянуть её фронт; D — оборона; Esc — снять выбор.")
        } else if groups > 0 {
            ("Что делать", "Выберите армию: карточка внизу или клавиша 1…9. Затем отдайте ей приказ. Время идёт — Пробел.")
        } else {
            ("Что делать", "Щёлкните по стопке дивизий на карте, затем ПКМ по провинции — туда идти или атаковать.")
        };
        // the chosen army: its general, and the way to change him and to the new divisions
        let chosen_info = self.chosen.and_then(|k| war.command[me].as_ref().and_then(|gs| gs.get(k)).map(|gr| (k, gr.general.as_ref().map(|x| x.name.clone()))));
        let mut open_army: Option<usize> = None;
        let open = self.brief_open;
        let mut toggle = false;
        let mut open_guide = false;
        let hot = hungry * 2 > ours.max(1);
        egui::Area::new(egui::Id::new("briefing")).anchor(Align2::LEFT_TOP, [12.0, 76.0]).order(egui::Order::Middle).show(ctx, |ui| {
            let back = chrome::hold(ui);
            let fr = egui::Frame::none().inner_margin(egui::Margin::symmetric(10.0, 8.0)).show(ui, |ui| {
                ui.set_width(250.0);
                let head = FontId::new(13.0, FontFamily::Name("head".into()));
                let body = FontId::new(13.5, FontFamily::Proportional);
                // the heading: a fold, "ВОЙНА", the two sides' flags and names
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::click());
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.clicked() {
                    toggle = true;
                }
                let p = ui.painter();
                // (the fold: a small triangle, down when open — the heading's font has no such glyph)
                let tc = r.left_center() + vec2(6.0, 0.0);
                let tri = if open { vec![tc + vec2(-5.0, -3.0), tc + vec2(5.0, -3.0), tc + vec2(0.0, 4.0)] } else { vec![tc + vec2(-3.0, -5.0), tc + vec2(-3.0, 5.0), tc + vec2(4.0, 0.0)] };
                p.add(egui::Shape::convex_polygon(tri, s.desk_ink, Stroke::NONE));
                p.text(r.left_center() + vec2(18.0, 0.0), Align2::LEFT_CENTER, "ВОЙНА", head.clone(), s.desk_ink);
                let chip = |p: &egui::Painter, at: Pos2, side: usize| {
                    let c = colour(side);
                    let rr = Rect::from_center_size(at, vec2(30.0, 17.0));
                    if let Some(f) = super::flags::get(ctx, th.countries[side]) {
                        super::flags::paint(ctx, p, rr, &f, Color32::from_black_alpha(200), None);
                    } else {
                        p.rect_filled(rr, 2.0, c);
                    }
                    p.rect_stroke(rr.expand(1.0), 2.0, Stroke::new(2.0, if side == me { Color32::from_rgb(236, 230, 212) } else { Color32::from_rgb(214, 52, 40) }));
                };
                // (the «?» opens the guide)
                let q = Rect::from_center_size(r.right_center() - vec2(12.0, 0.0), vec2(22.0, 22.0));
                let qr = ui.interact(q, egui::Id::new("guide-q"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                if qr.clicked() {
                    open_guide = true;
                    toggle = false;
                }
                let p = ui.painter();
                p.circle_filled(q.center(), 10.0, if qr.hovered() { Color32::from_rgb(70, 62, 48) } else { Color32::from_rgb(34, 31, 26) });
                p.circle_stroke(q.center(), 10.0, Stroke::new(1.3, ch.trim));
                p.text(q.center(), Align2::CENTER_CENTER, "?", FontId::new(13.0, FontFamily::Name("bold".into())), s.desk_ink);
                if !open {
                    chip(p, r.right_center() - vec2(64.0, 0.0), me);
                    chip(p, r.right_center() - vec2(30.0 + 8.0, 0.0) - vec2(0.0, 0.0) + vec2(-36.0 + 36.0, 0.0), 1 - me);
                    return;
                }
                ui.add_space(4.0);
                let side = |c: usize| super::kit::Side { country: c, name: g.countries[c].0.clone(), colour: colour_of(c) };
                let (sa, sb) = (g.war_mood.support[me], g.war_mood.support[1 - me]);
                let vs_rows = [
                    super::kit::VsRow::new("Дивизии", ours as f32, foe_seen, ours.to_string(), format!("≈{foe_seen:.0}")),
                    super::kit::VsRow::new("Поддержка войны", sa, sb, format!("{:.0}%", sa * 100.0), format!("{:.0}%", sb * 100.0)),
                ];
                super::kit::versus(ui, &side(th.countries[me]), &side(th.countries[1 - me]), &vs_rows, true);
                ui.add_space(2.0);
                let row = |ui: &mut egui::Ui, what: &str, value: String, warn: bool| {
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 19.0), Sense::hover());
                    let p = ui.painter();
                    p.text(r.left_center(), Align2::LEFT_CENTER, what, body.clone(), s.desk_ink.gamma_multiply(0.85));
                    p.text(r.right_center(), Align2::RIGHT_CENTER, value, FontId::new(14.0, FontFamily::Name("bold".into())), if warn { Color32::from_rgb(244, 140, 110) } else { s.desk_ink });
                };
                if fighting > 0 || hungry > 0 {
                    row(ui, if hungry > 0 { "Боёв · без снабжения" } else { "Боёв идёт" }, if hungry > 0 { format!("{fighting} · {hungry}") } else { format!("{fighting}") }, hot);
                }
                ui.add_space(5.0);
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0, ch.trim.gamma_multiply(0.6)));
                ui.add_space(5.0);
                ui.label(RichText::new(hint_head).font(head).color(hud::lighten(ch.trim, 0.3)));
                ui.add(egui::Label::new(RichText::new(hint).font(body.clone()).color(s.desk_ink)).wrap());
                if let Some((k, gen)) = &chosen_info {
                    ui.add_space(5.0);
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                    ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0, ch.trim.gamma_multiply(0.6)));
                    ui.add_space(4.0);
                    ui.label(RichText::new(crate::words::group_name(*k)).font(FontId::new(13.0, FontFamily::Name("head".into()))).color(hud::lighten(ch.trim, 0.3)));
                    match gen {
                        Some(n) => ui.label(RichText::new(format!("Генерал: {n}")).size(13.5).color(s.desk_ink)),
                        None => ui.label(RichText::new("Без генерала: армию ведёт штаб, каждый сам по себе.").size(13.5).color(Color32::from_rgb(244, 140, 110))),
                    };
                    ui.horizontal(|ui| {
                        if super::guide::chip(ui, &ch, if gen.is_some() { "Сменить генерала" } else { "Назначить генерала" }, false).clicked() {
                            open_army = Some(0);
                        }
                    });
                    ui.horizontal(|ui| {
                        if super::guide::chip(ui, &ch, "Новые дивизии: шаблоны…", false).clicked() {
                            open_army = Some(1);
                        }
                    });
                }
                if hot {
                    ui.add_space(3.0);
                    ui.add(egui::Label::new(RichText::new("Голодают: режим карты «Снабжение» покажет, где обрыв.").font(body).color(Color32::from_rgb(244, 140, 110))).wrap());
                }
            });
            let mut v = chrome::Out::new();
            hud::plate(&mut v, fr.response.rect, &ch, 4.0);
            chrome::under(ui, back, v);
        });
        if toggle {
            self.brief_open = !self.brief_open;
        }
        if open_guide {
            self.guide_open = true;
        }
        if let Some(p) = open_army {
            self.tab = Some(Tab::Army);
            set_page(ctx, Tab::Army, p);
        }
    }
}
