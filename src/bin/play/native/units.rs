//! The divisions the player chose (as HOI4's panel of the chosen units, low on the left over the
//! map): how many, how strong and organised they are, each with what it does; orders for them —
//! stand where you are, back to your general, to a group of armies or to the reserve; a row
//! chooses that division alone, its card tells it all.

use eframe::egui::{self, pos2, vec2, Align2, Color32, FontFamily, FontId, Rect, Sense, Stroke};

use crate::Game;
use crate::tools::saves::Act;
use super::chrome::{self, Card, Tip, Tone};
use super::{command, hud, map, App};

/// Width of the panel; height of a row.
const W: f32 = 430.0;
const ROW: f32 = 26.0;

impl App {
    /// The panel of the chosen divisions, when any are chosen in a war of divisions.
    pub(super) fn picked_panel(&mut self, ctx: &egui::Context, g: &mut Game) {
        let Some(m) = self.map.as_mut() else { return };
        let Some(c) = g.h.war.corps.as_ref() else {
            m.picked.clear();
            return;
        };
        let me = g.me;
        m.picked.retain(|id| c.find(*id).is_some_and(|i| c.divs[i].side == me));
        if m.picked.is_empty() {
            return;
        }
        let picked = m.picked.clone();
        let pack = g.pack.clone();
        let divs: Vec<strategy::military::corps::Division> = picked.iter().filter_map(|&id| c.find(id).map(|i| c.divs[i].clone())).collect();
        let n = divs.len() as f32;
        let strength = divs.iter().map(|d| c.strength(&pack, d)).sum::<f32>() / n;
        let org = divs.iter().map(|d| d.org).sum::<f32>() / n;
        let kit = divs.iter().map(|d| c.equipped(&pack, d)).sum::<f32>() / n;
        let hungry = divs.iter().filter(|d| d.supplied < 0.5).count();
        let groups = g.h.war.command[me].as_ref().map_or(0, |v| v.len());
        let free = ctx.available_rect();
        let ch = self.chrome.clone();
        let s = self.skin;
        let rows = divs.len().min(9);
        let h = 112.0 + (command::H - 26.0) + 30.0 + 20.0 + rows as f32 * ROW + 8.0;
        let at = pos2(free.min.x + 10.0, free.max.y - h - 10.0);
        let mut act: Vec<Act> = Vec::new();
        let mut only: Option<u32> = None;
        let mut close = false;
        egui::Area::new(egui::Id::new("picked-panel")).order(egui::Order::Middle).fixed_pos(at).show(ctx, |ui| {
            let r = Rect::from_min_size(at, vec2(W, h));
            let mut v = chrome::Out::new();
            v.push(egui::epaint::Shadow { offset: vec2(0.0, 4.0), blur: 16.0, spread: 0.0, color: Color32::from_black_alpha(160) }.as_shape(r, 4.0).into());
            ch.lay(&mut v, r, &ch.bar, 4.0);
            v.push(egui::Shape::rect_filled(r, 4.0, Color32::from_black_alpha(70)));
            hud::gloss(&mut v, r, Color32::from_white_alpha(18), Color32::from_black_alpha(80));
            v.push(egui::Shape::rect_stroke(r, 4.0, Stroke::new(1.5, ch.trim)));
            ui.painter().extend(v);
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(r.shrink2(vec2(12.0, 8.0))), |ui| {
                ui.set_width(W - 24.0);
                // the heading, the cross that lets them go
                ui.horizontal(|ui| {
                    let word = match divs.len() % 100 {
                        11..=14 => "дивизий",
                        _ => match divs.len() % 10 {
                            1 => "дивизия",
                            2..=4 => "дивизии",
                            _ => "дивизий",
                        },
                    };
                    ui.label(egui::RichText::new(format!("ВЫБРАНО: {} {word}", divs.len())).family(FontFamily::Name("head".into())).size(17.0).color(s.desk_ink));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if chrome::button(ui, &ch, &s, "×", vec2(26.0, 24.0), true, false).tip(|| Card::new("Снять выбор").icon("hand").text("Щелчок по пустой земле или Esc делают то же.")).clicked() {
                            close = true;
                        }
                    });
                });
                // how strong, organised and equipped they are, in bars as on the map's counters
                let (br, _) = ui.allocate_exact_size(vec2(W - 24.0, 18.0), Sense::hover());
                let half = (br.width() - 20.0) / 3.0;
                let bar = |p: &egui::Painter, x: f32, v: f32, col: Color32, label: &str| {
                    let r = Rect::from_min_size(pos2(x, br.min.y + 10.0), vec2(half, 6.0));
                    p.rect_filled(r.expand(1.0), 1.5, Color32::from_black_alpha(200));
                    p.rect_filled(Rect::from_min_size(r.min, vec2(r.width() * v.clamp(0.0, 1.0), r.height())), 1.0, col);
                    p.text(pos2(x, br.min.y + 4.0), Align2::LEFT_CENTER, label, FontId::new(12.5, FontFamily::Proportional), s.desk_ink);
                };
                bar(ui.painter(), br.min.x, strength, Color32::from_rgb(112, 196, 84), "численность");
                bar(ui.painter(), br.min.x + half + 10.0, org, Color32::from_rgb(236, 200, 64), "организация");
                bar(ui.painter(), br.min.x + 2.0 * (half + 10.0), kit.min(1.0), if kit < 0.8 { Color32::from_rgb(236, 160, 40) } else { Color32::from_rgb(130, 176, 214) }, "оснащение");
                ui.add_space(4.0);
                // the orders, each with its key
                // (the divisions have generals while there are groups of armies: in peace too)
                let at_war = groups > 0;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if command::order(ui, &ch, &s, "stop", "Стоять", "H", false, true)
                        .tip(|| Card::new("Стоять").icon("stop").text("Выбранные встают, где стоят, и окапываются. Генерал их не трогает, пока не вернёте.").note("клавиша H"))
                        .clicked()
                    {
                        act.push(Act::Halt(picked.clone()));
                    }
                    if command::order(ui, &ch, &s, "back", if at_war { "Генералу" } else { "Генштабу" }, "G", false, true)
                        .tip(|| if at_war { Card::new("Вернуть генералу").icon("back").text("Каждая выбранная дивизия снова под своим генералом: он расставит её на своём участке.").note("клавиша G") } else { Card::new("Вернуть генштабу").icon("back").text("Генштаб снова держит выбранные в гарнизонах за границей.").note("клавиша G") })
                        .clicked()
                    {
                        let mut by: Vec<(usize, Vec<u32>)> = Vec::new();
                        for d in &divs {
                            match by.iter_mut().find(|x| x.0 == d.group) {
                                Some(x) => x.1.push(d.id),
                                None => by.push((d.group, vec![d.id])),
                            }
                        }
                        for (k, ids) in by {
                            act.push(Act::Assign(ids, k));
                        }
                    }
                    if at_war
                        && command::order(ui, &ch, &s, "rally", "Новая армия", "N", false, groups > 0)
                            .tip(|| Card::new("Новая армия").icon("rally").text("Из выбранных дивизий — новая армия под новым генералом (хорошо он ведёт до 24 дивизий): её участок — ближайший к ним отрезок фронта, она держит его. Дальше ей можно провести свою линию фронта и стрелу наступления.").note("клавиша N"))
                            .clicked()
                    {
                        act.push(Act::Raise(picked.clone()));
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if groups > 0 {
                        ui.label(egui::RichText::new("в группу:").size(13.0).color(s.desk_ink.gamma_multiply(0.8)));
                    }
                    for k in 0..groups.min(8) {
                        let hue = map::HUES[k % map::HUES.len()];
                        let (r, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        let p = ui.painter();
                        p.rect_filled(r, 3.0, if resp.hovered() { hue } else { hue.gamma_multiply(0.75) });
                        p.rect_stroke(r, 3.0, Stroke::new(1.0, Color32::from_black_alpha(200)));
                        p.text(r.center(), Align2::CENTER_CENTER, (k + 1).to_string(), FontId::new(13.0, FontFamily::Name("bold".into())), Color32::from_rgb(24, 20, 14));
                        if resp.tip(|| Card::new(crate::words::group_name(k)).icon("army").text("Выбранные переходят в эту армию, под её генерала.")).clicked() {
                            act.push(Act::Assign(picked.clone(), k));
                        }
                    }
                    if groups == 0 {
                        return;
                    }
                    let (r, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    ui.painter().rect_filled(r, 3.0, if resp.hovered() { Color32::from_gray(210) } else { Color32::from_gray(160) });
                    ui.painter().rect_stroke(r, 3.0, Stroke::new(1.0, Color32::from_black_alpha(200)));
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, "Р", FontId::new(13.0, FontFamily::Name("bold".into())), Color32::from_rgb(24, 20, 14));
                    if resp.tip(|| Card::new("Резерв ставки").icon("army").text("Выбранные уходят в резерв: им приказываете вы, а когда фронт ведёт генштаб — он бросает резерв туда, где враг давит.")).clicked() {
                        act.push(Act::Assign(picked.clone(), strategy::military::corps::NONE));
                    }
                });
                ui.add_space(2.0);
                ui.label(egui::RichText::new("правой кнопкой — идти туда, с Shift — и дальше оттуда").size(12.5).color(s.desk_ink.gamma_multiply(0.8)));
                if hungry > 0 {
                    ui.label(egui::RichText::new(format!("без снабжения: {hungry} — тесно или отрезаны")).size(12.5).color(Color32::from_rgb(240, 120, 96)));
                } else {
                    ui.add_space(2.0);
                }
                // the divisions, a row each
                let list = |ui: &mut egui::Ui| {
                    for d in &divs {
                        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW - 2.0), Sense::click());
                        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        let p = ui.painter();
                        if resp.hovered() {
                            p.rect_filled(r, 2.0, Color32::from_white_alpha(16));
                        }
                        // (the sign of its main kind, in its group's colour)
                        let sb = Rect::from_min_size(r.min + vec2(2.0, 4.0), vec2(24.0, 16.0));
                        p.rect_filled(sb, 1.0, Color32::from_rgb(238, 232, 214));
                        let edge = if d.group != strategy::military::corps::NONE { map::HUES[d.group % map::HUES.len()] } else { Color32::from_gray(140) };
                        p.rect_stroke(sb, 1.0, Stroke::new(1.5, edge));
                        let kind = map::main_kind(&pack, &d.has);
                        map::sign(p, &pack.units[kind].symbol, sb.center(), 18.0, 11.0, Color32::from_rgb(29, 26, 21));
                        let name = crate::words::division_name(d.no, &c.shapes[d.side][d.shape].name);
                        let mut job = egui::text::LayoutJob::simple_singleline(name.clone(), FontId::new(13.5, FontFamily::Proportional), s.desk_ink);
                        job.wrap = egui::text::TextWrapping::truncate_at_width(196.0);
                        let gal = p.layout_job(job);
                        p.galley(pos2(sb.max.x + 6.0, r.center().y - gal.size().y / 2.0), gal, s.desk_ink);
                        // its three bars
                        let st = c.strength(&pack, d);
                        let eq = c.equipped(&pack, d);
                        let x = r.min.x + 236.0;
                        for (i, (v, col)) in [(st, Color32::from_rgb(112, 196, 84)), (d.org, Color32::from_rgb(236, 200, 64)), (eq.min(1.0), if eq < 0.8 { Color32::from_rgb(236, 160, 40) } else { Color32::from_rgb(130, 176, 214) })].into_iter().enumerate() {
                            let b = Rect::from_min_size(pos2(x, r.center().y - 8.0 + i as f32 * 5.5), vec2(48.0, 4.0));
                            p.rect_filled(b.expand(0.8), 1.0, Color32::from_black_alpha(200));
                            p.rect_filled(Rect::from_min_size(b.min, vec2(b.width() * v.clamp(0.0, 1.0), b.height())), 0.5, col);
                        }
                        // what it does
                        let what = map::doing(g, c, d);
                        let red = d.supplied < 0.5 || what == "отходит";
                        let mut job = egui::text::LayoutJob::simple_singleline(what.clone(), FontId::new(12.0, FontFamily::Proportional), if red { Color32::from_rgb(240, 120, 96) } else { s.desk_ink.gamma_multiply(0.85) });
                        job.wrap = egui::text::TextWrapping::truncate_at_width(r.max.x - x - 58.0);
                        let gal = p.layout_job(job);
                        p.galley(pos2(x + 56.0, r.center().y - gal.size().y / 2.0), gal, s.desk_ink);
                        let card = || {
                            let war = &g.h.war;
                            let shape = &c.shapes[d.side][d.shape];
                            let bats: Vec<String> = shape
                                .units
                                .iter()
                                .enumerate()
                                .filter(|(_, x)| **x > 0.0)
                                .map(|(u, x)| {
                                    let k = shape.norm_of(u);
                                    if pack.units[u].norms && k != 1.0 {
                                        format!("{:.0} {} ({})", x / strategy::military::corps::BATTALION, pack.units[u].name, strategy::military::doctrine::norm::word(k))
                                    } else {
                                        format!("{:.0} {}", x / strategy::military::corps::BATTALION, pack.units[u].name)
                                    }
                                })
                                .collect();
                            let sup: Vec<&str> = shape.support.iter().filter_map(|&k| pack.support.get(k)).map(|x| x.name.as_str()).collect();
                            let crew = pack.war.as_ref().map_or(10.0, |w| w.crew);
                            let men: f32 = d.has.iter().zip(&pack.units).map(|(x, u)| x * u.men * crew).sum();
                            let mut card = Card::new(name.clone())
                                .icon("army")
                                .sub(format!("шаблон «{}»", shape.name))
                                .row("батальоны", bats.join(", "))
                                .row("роты", if sup.is_empty() { "нет".to_string() } else { sup.join(", ") })
                                .row("людей", format!("{men:.1} тыс."))
                                .tone("численность", format!("{:.0}%", st * 100.0), if st >= 0.7 { Tone::Good } else { Tone::Bad })
                                .tone("оснащение", format!("{:.0}%", eq * 100.0), if eq >= 0.8 { Tone::Good } else { Tone::Bad })
                                .tone("организация", format!("{:.0}%", d.org * 100.0), if d.org >= 0.5 { Tone::Good } else { Tone::Bad })
                                .tone("снабжение", format!("{:.0}%", d.supplied * 100.0), if d.supplied >= 0.5 { Tone::Good } else { Tone::Bad })
                                .row("опыт", format!("{:.0}%", d.exp * 100.0))
                                .row("окопы", format!("{:.0}%", d.dug * 100.0))
                                .row("где", crate::words::terrain_ru(war.th.terrain[d.at]).to_string())
                                .row("что делает", what.clone())
                                .row("армия", if d.group == strategy::military::corps::NONE { if g.h.at_war { "резерв ставки".to_string() } else { "гарнизон".to_string() } } else { crate::words::group_name(d.group) });
                            if d.days > 0 {
                                card = card.row("в боях", format!("{} дн., потеряла {:.1} тыс. человек", d.days, d.dead * 10.0 * crew / 10.0));
                            }
                            if d.left > 0.01 {
                                card = card.tone("бросила снаряжения", format!("{:.0}% батальона", d.left / strategy::military::corps::BATTALION * 100.0), Tone::Bad);
                            }
                            if d.manual {
                                card = card.note("ведёте вы: генерал её не трогает, пока не вернёте");
                            }
                            card.note("щёлкните — выбрать только её")
                        };
                        if resp.tip(card).clicked() {
                            only = Some(d.id);
                        }
                    }
                };
                super::widgets::scroll(ui, "picked-list", rows as f32 * ROW, false, list);
            });
        });
        for a in act {
            let raise = matches!(a, Act::Raise(_));
            if g.act(a) && raise {
                // (the new group chosen, its divisions let go: its plans over the cards)
                self.chosen = g.h.war.command[me].as_ref().map(|v| v.len() - 1);
                close = true;
            }
        }
        if let Some(m) = self.map.as_mut() {
            if close {
                m.picked.clear();
            } else if let Some(id) = only {
                m.picked = vec![id];
            }
        }
    }
}
