//! Who runs the economy, at the head of the overview: you, or an economist — one click to change.
//! Below the name, what he does now and how the country has fared in his time against the rival's
//! and against the months before him (nothing hidden is shown: his schooling, family, views and
//! fame are what anyone may know of a man; his mind and honesty are told only by his deeds).

use super::*;
use super::super::super::flags;

/// Whether the list of those who might run the economy is open.
fn open_id() -> egui::Id {
    egui::Id::new("econ-choose")
}

/// How the country has fared in the economist's time (months of the chronicle since he took the
/// post): output and anger and the treasury, ours and the rival's output over the same months, and
/// ours over as many months before.
pub(super) struct Tenure {
    pub months: u32,
    pub output: f32,
    pub rival_output: f32,
    pub before: Option<f32>,
    pub anger: f32,
    pub treasury: f32,
}

pub(super) fn tenure(g: &Game) -> Option<Tenure> {
    let me = g.me;
    let since = g.since[0];
    let last = g.rows.last()?;
    let first = g.rows.iter().find(|r| r.month >= since)?;
    let months = last.month.saturating_sub(first.month);
    if months == 0 {
        return None;
    }
    let growth = |a: f32, b: f32| b / a.max(1e-9) - 1.0;
    let before = g.rows.iter().rev().find(|r| r.month + months <= first.month).map(|r| growth(r.gdp[me], first.gdp[me]));
    Some(Tenure {
        months,
        output: growth(first.gdp[me], last.gdp[me]),
        rival_output: growth(first.gdp[1 - me], last.gdp[1 - me]),
        before,
        anger: last.anger[me] - first.anger[me],
        treasury: last.book[me].treasury - first.book[me].treasury,
    })
}

/// Words for a count of months: «3 мес.», «1 г. 2 мес.».
fn span(months: u32) -> String {
    match (months / 12, months % 12) {
        (0, m) => format!("{m} мес."),
        (y, 0) => format!("{y} г."),
        (y, m) => format!("{y} г. {m} мес."),
    }
}

/// The views of a candidate on a bar from «свобода хозяев» to «сильная казна»: three notches, a
/// marker on the one he is known for.
fn views(ui: &mut egui::Ui, s: &Skin, state: f32) {
    let (ch, _) = chrome::dress(ui);
    let w = ui.available_width().min(260.0);
    let (r, resp) = ui.allocate_exact_size(vec2(w, 18.0), egui::Sense::hover());
    if ui.is_rect_visible(r) {
        let bar = egui::Rect::from_min_max(egui::pos2(r.min.x + 4.0, r.center().y - 2.5), egui::pos2(r.max.x - 4.0, r.center().y + 2.5));
        let mut v = chrome::Out::new();
        v.push(egui::Shape::rect_filled(bar, 2.0, Color32::from_black_alpha(60)));
        ch.bevel(&mut v, bar, false, 1.0);
        for k in 0..3 {
            let x = bar.min.x + bar.width() * k as f32 / 2.0;
            v.push(egui::Shape::line_segment([egui::pos2(x, bar.min.y - 3.0), egui::pos2(x, bar.max.y + 3.0)], egui::Stroke::new(1.0, s.soft.gamma_multiply(0.6))));
        }
        let at = if state > 0.3 { 2 } else if state < -0.3 { 0 } else { 1 };
        let x = bar.min.x + bar.width() * at as f32 / 2.0;
        v.push(egui::Shape::circle_filled(egui::pos2(x, bar.center().y), 6.5, s.accent));
        v.push(egui::Shape::circle_stroke(egui::pos2(x, bar.center().y), 6.5, egui::Stroke::new(1.2, Color32::from_black_alpha(190))));
        ui.painter().extend(v);
    }
    resp.tip(|| Card::new("Взгляды на хозяйство").icon("balance").text("Слева — «хозяева знают лучше»: меньше налогов, казна не строит. Справа — «казна планирует»: налог выше, казна строит сама. Посредине — как получится.").note("так о нём известно; чего он стоит на деле, скажут его дела"));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let third = (w - 8.0) / 3.0;
        for (k, t) in ["свобода хозяев", "как получится", "сильная казна"].iter().enumerate() {
            let (rr, _) = ui.allocate_exact_size(vec2(if k == 1 { third } else { third + 4.0 }, 14.0), egui::Sense::hover());
            let align = [egui::Align2::LEFT_CENTER, egui::Align2::CENTER_CENTER, egui::Align2::RIGHT_CENTER][k];
            let x = [rr.min.x, rr.center().x, rr.max.x][k];
            ui.painter().text(egui::pos2(x, rr.center().y), align, *t, egui::FontId::proportional(12.0), s.soft);
        }
    });
}

/// One who might run the economy: his portrait, name, what is known of him, his views on the
/// bar; a button to appoint him (or the mark that he is in the post).
fn candidate(ui: &mut egui::Ui, s: &Skin, g: &Game, k: usize, on: bool) -> bool {
    let mut clicked = false;
    let ch = chrome::of(ui.ctx());
    let back = chrome::hold(ui);
    let name = g.pool[k].0.clone();
    let id = g.roster.pool.get(k).copied();
    let fr = egui::Frame::none().inner_margin(egui::Margin { left: 9.0, right: 9.0, top: 7.0, bottom: 7.0 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            if let Some(img) = portrait(&g.face_of(&name, "econ"), 44.0) {
                let r = ui.add(img).rect;
                if let Some(ch) = &ch {
                    ui.painter().rect_stroke(r.expand(1.0), 2.0, egui::Stroke::new(1.5, ch.trim));
                }
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.horizontal(|ui| {
                    kit::bold(ui, name.clone(), 15.5, s.ink);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if on {
                            ui.label(RichText::new("на посту").family(egui::FontFamily::Name("bold".into())).size(13.5).color(s.accent));
                        } else {
                            let b = widgets::small(ui, "Назначить");
                            if k == 0 && ui.clip_rect().contains_rect(b.rect) {
                                chrome::spot(ui.ctx(), "appoint", b.rect);
                            }
                            if b.tip(|| Card::new(format!("Назначить: {name}")).face(g.face_of(&name, "econ")).text("Налог и стройку казны он решает сам; армию и науку — по-прежнему вы. Через год будут видны его итоги.").note("взять хозяйство обратно можно в любой момент")).clicked() {
                                clicked = true;
                            }
                        }
                    });
                });
                if let Some(id) = id {
                    let p = g.folk.get(id);
                    let now = g.month() as i32;
                    kit::words(ui, format!("{:.0} лет · {} · {}", p.age(now), p.schooling_word(), p.origin_word()), 13.0, s.soft);
                    views(ui, s, p.state);
                    let fame = p.fame_word();
                    if let Some(d) = p.deeds.last() {
                        kit::words(ui, format!("{fame}. {}: {}", strategy::world::folk::year_word(d.month), d.text), 12.5, s.soft);
                    } else {
                        kit::words(ui, fame, 12.5, s.soft);
                    }
                }
            });
        });
    });
    if let Some(ch) = &ch {
        let r = fr.response.rect;
        let mut v = chrome::Out::new();
        ch.lay(&mut v, r, &ch.paper, 2.0);
        ch.gloss(&mut v, r, Color32::from_white_alpha(if on { 10 } else { 50 }), Color32::from_black_alpha(if on { 50 } else { 30 }));
        ch.bevel(&mut v, r, !on, 1.5);
        v.push(egui::Shape::rect_stroke(r, 2.0, egui::Stroke::new(if on { 2.0 } else { 1.0 }, if on { s.accent } else { Color32::from_black_alpha(120) })));
        chrome::under(ui, back, v);
    }
    ui.add_space(3.0);
    clicked
}

/// The strip: who runs the economy, what he does and how it has gone; the list of those who might.
pub(in super::super) fn strip(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    // (`SHOTGOV=1`: for pictures, the list of those who might run the economy open)
    let open = ui.ctx().data(|d| d.get_temp::<bool>(open_id())).unwrap_or(std::env::var("SHOTGOV").is_ok());
    let adviser = g.adviser;
    let mut toggle = false;
    kit::framed(ui, Some(if adviser.is_some() { kit::MIXED } else { s.accent }), false, |ui| {
        ui.horizontal(|ui| {
            // (a face: the economist's portrait, or the flag when it is you who rules)
            let (face, name) = match adviser {
                Some(k) => (portrait(&g.face_of(&g.pool[k].0, "econ"), 40.0), g.pool[k].0.clone()),
                None => (None, "Вы сами".to_string()),
            };
            match face {
                Some(img) => {
                    let r = ui.add(img).rect;
                    if let Some(ch) = chrome::of(ui.ctx()) {
                        ui.painter().rect_stroke(r.expand(1.0), 2.0, egui::Stroke::new(1.5, ch.trim));
                    }
                }
                None => {
                    let (fr, _) = ui.allocate_exact_size(vec2(60.0, 40.0), egui::Sense::hover());
                    if let Some(fl) = flags::get(ui.ctx(), g.h.war.th.countries[me]) {
                        let icons = chrome::icons(ui.ctx());
                        flags::paint(ui.ctx(), ui.painter(), fr.shrink(1.0), &fl, s.ink, Some(&icons));
                    }
                    ui.painter().rect_stroke(fr, 1.0, egui::Stroke::new(1.0, Color32::from_black_alpha(140)));
                }
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(RichText::new("ХОЗЯЙСТВО ВЕДЁТ").family(egui::FontFamily::Name("head".into())).size(12.5).color(s.soft));
                kit::bold(ui, name, 17.0, s.ink);
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let text = if open { "Свернуть" } else if adviser.is_some() { "Сменить" } else { "Поставить советника" };
                let b = widgets::small(ui, text);
                if ui.clip_rect().contains_rect(b.rect) {
                    chrome::spot_last(ui.ctx(), "govern_button", b.rect);
                }
                if b.tip(|| Card::new("Кто правит хозяйством").icon("balance").text("Хозяйство можно вести самому или отдать советнику-экономисту: налог и стройку казны решает он, армию и науку — всегда вы. Он не всемогущ и не всегда честен — о нём судят по делам.").note("забрать хозяйство себе можно в любой момент")).clicked() {
                    toggle = true;
                }
            });
        });
        ui.add_space(2.0);
        match (adviser, g.h.econ[me].as_ref()) {
            (Some(_), Some(e)) => {
                kit::words(ui, crate::words::levers_words(&e.levers, &pack), 13.5, s.ink);
                match tenure(g) {
                    Some(t) if t.months >= 3 => {
                        let mut line = format!("За {}: выпуск {:+.1}% (у соседа {:+.1}%)", span(t.months), t.output * 100.0, t.rival_output * 100.0);
                        if let Some(b) = t.before {
                            line += &format!(", до него за столько же — {:+.1}%", b * 100.0);
                        }
                        kit::words(ui, line, 13.0, s.soft).tip(|| {
                            Card::new("Итоги советника")
                                .icon("chart")
                                .row("времени у дел", span(t.months))
                                .tone("выпуск при нём", format!("{:+.1}%", t.output * 100.0), if t.output >= t.rival_output { Tone::Good } else { Tone::Bad })
                                .row("выпуск у соседа за то же время", format!("{:+.1}%", t.rival_output * 100.0))
                                .row("недовольство", format!("{:+.2}", t.anger))
                                .row("казна", format!("{:+.1}", t.treasury))
                                .note("честно судить можно не раньше чем через год; казнокрадство в цифрах не видно")
                        });
                    }
                    _ => {
                        kit::words(ui, "Итоги — когда он поработает хотя бы три месяца.", 13.0, s.soft);
                    }
                }
                if let Some(d) = e.record.last() {
                    kit::words(ui, format!("Последнее его решение ({}): {}", crate::words::date_words(d.month), d.what), 12.5, s.soft);
                }
            }
            _ => {
                kit::words(ui, "Налог и стройку казны решаете вы: рычаги ниже и в «Бюджете».", 13.0, s.soft);
            }
        }
    });
    if toggle {
        ui.ctx().data_mut(|d| d.insert_temp(open_id(), !open));
    }
    if open {
        let mut pick: Option<Option<usize>> = None;
        let me_on = adviser.is_none();
        // (you yourself: the way back, when someone else holds the economy)
        if !me_on {
            kit::framed(ui, None, false, |ui| {
                ui.horizontal(|ui| {
                    kit::bold(ui, "Вы сами", 15.0, s.ink);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::small(ui, "Взять хозяйство себе").clicked() {
                            pick = Some(None);
                        }
                    });
                });
                kit::words(ui, "Налог и стройка — ваши рычаги: они ниже, в «Бюджете» и в «Деньгах».", 13.0, s.soft);
            });
        }
        kit::words(ui, "Кого из троих поставить? Образование, семья, взгляды и известность видны всем; ум и честность — только по делам.", 13.0, s.soft);
        ui.add_space(2.0);
        for k in 0..g.pool.len() {
            if candidate(ui, s, g, k, adviser == Some(k)) {
                pick = Some(Some(k));
            }
        }
        if let Some(who) = pick.filter(|_| !super::super::super::spoiled()) {
            act(g, &[("econ", match who { Some(k) => format!("adv{k}"), None => "me".into() })]);
            ui.ctx().data_mut(|d| d.insert_temp(open_id(), false));
        }
    }
    ui.add_space(2.0);
}
