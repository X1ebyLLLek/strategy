//! The folders on the left of the window: the news as a newspaper and the world's chronicle,
//! politics, the economy, science, the army (and a battle explained). Decisions go to the game as
//! keys and values (`Game::apply`): the same rules, whoever asks — the player, a bot, a save.

use eframe::egui::{self, pos2, vec2, Color32, RichText};
use strategy::military::front::Order;

use crate::Game;
use crate::words::date_words;
use strategy::military::homefront::{lines, shares};
use super::chrome::{self, Card, Tip, Tone};
use super::look::{portrait, Skin};
use super::terms::say;
use super::widgets;

mod army;
mod battle;
mod budget;
mod builds;
mod chamber;
mod diplomacy;
pub mod economy;
mod finance;
mod forecast;
mod ledger;
mod levers;
mod military;
mod nation;
mod navy;
mod overview;
mod people;
mod politics;
mod production;
mod science;
mod sheet;
mod society;
mod taxes;
mod techs;

pub use army::army;
pub use battle::battle;
pub use economy::econ;
pub use nation::nation;
pub use navy::navy;
pub use overview::forget_vital;
pub use ledger::ledger;
pub use people::persons;
pub use production::production;
pub use politics::politics;
pub use science::science;
pub use society::society;

/// The pages of a folder (none: one page).
pub(super) fn pages(tab: super::Tab) -> &'static [&'static str] {
    match tab {
        super::Tab::Econ => &economy::PAGES,
        super::Tab::Army => &army::PAGES,
        super::Tab::Nation => &nation::PAGES,
        super::Tab::Politics => &politics::PAGES,
        super::Tab::Society => &society::PAGES,
        _ => &[],
    }
}

fn act(g: &mut Game, pairs: &[(&str, String)]) {
    let form: Vec<(String, String)> = pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
    g.act(crate::tools::saves::Act::Form(form));
}

fn head(ui: &mut egui::Ui, s: &Skin, text: &str) {
    match chrome::of(ui.ctx()) {
        Some(_) => super::kit::ribbon(ui, text, Some(head_icon(text))),
        None => {
            ui.add_space(6.0);
            ui.label(RichText::new(text.to_uppercase()).family(egui::FontFamily::Name("head".into())).size(16.0).color(s.accent));
            ui.separator();
        }
    }
}

/// The icon of a part of a folder, by its heading.
fn head_icon(text: &str) -> &'static str {
    let t = text.to_lowercase();
    let pairs: [(&str, &str); 23] = [
        ("летопис", "scroll"),
        ("дипломат", "quill"),
        ("дивизи", "army"),
        ("по карману", "coins"),
        ("армия", "army"),
        ("решени", "pen"),
        ("война", "clash"),
        ("казна", "coins"),
        ("наук", "science"),
        ("хозяйств", "econ"),
        ("снабжен", "supply"),
        ("склад", "crate"),
        ("направлен", "compass"),
        ("отрасл", "gears"),
        ("разработ", "science"),
        ("почему", "binoculars"),
        ("рынок", "shop"),
        ("сосед", "castle"),
        ("трудн", "hazard"),
        ("фронт", "march"),
        ("строит", "anvil"),
        ("групп", "rally"),
        ("закон", "stamper"),
    ];
    pairs.iter().find(|(k, _)| t.contains(k)).map_or("medal", |x| x.1)
}

/// Plain words in a size and colour.
fn words(ui: &mut egui::Ui, text: impl Into<String>, size: f32, colour: Color32) -> egui::Response {
    ui.label(RichText::new(text.into()).size(size).color(colour))
}

/// A card of a person: the portrait in a frame, the name, what he does; a button to appoint him.
fn person(ui: &mut egui::Ui, s: &Skin, g: &Game, name: &str, post: &str, on: bool, body: &str) -> bool {
    let mut clicked = false;
    let ch = chrome::of(ui.ctx());
    let back = chrome::hold(ui);
    let fr = egui::Frame::none().inner_margin(egui::Margin { left: 8.0, right: 8.0, top: 7.0, bottom: 7.0 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            if let Some(img) = portrait(&g.face_of(name, post), 46.0) {
                let r = ui.add(img).rect;
                if let Some(ch) = &ch {
                    ui.painter().rect_stroke(r.expand(1.0), 2.0, egui::Stroke::new(1.5, ch.trim));
                }
            }
            ui.vertical(|ui| {
                ui.label(RichText::new(name).family(egui::FontFamily::Name("bold".into())).size(15.5));
                if !body.is_empty() {
                    ui.label(RichText::new(body).size(13.5));
                }
                if on {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("на посту").family(egui::FontFamily::Name("bold".into())).size(13.5).color(s.accent));
                    });
                } else if widgets::small(ui, "Назначить").tip(|| Card::new(format!("Назначить: {name}")).face(g.face_of(name, post)).text("Сфера перейдёт к нему; о его уме и честности скажут только его дела.").note("забрать сферу себе можно в любой момент")).clicked() {
                    clicked = true;
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
        if on {
            v.push(egui::Shape::rect_filled(egui::Rect::from_min_size(r.min, vec2(4.0, r.height())), 1.0, s.accent));
        }
        chrome::under(ui, back, v);
    }
    ui.add_space(3.0);
    clicked
}

/// The newspaper's name, by the era.
fn masthead(theme: &str) -> &'static str {
    match theme {
        "parchment" => "Ведомости",
        "rust" => "Листок",
        _ => "Вестник",
    }
}

/// The news as a newspaper — its name over a double rule, the country and the date; the latest
/// story as the lead, the rest under their dates divided by thin rules — and the world's chronicle
/// as a line of years.
pub fn news(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    let Some(ch) = chrome::of(ui.ctx()) else { return };
    let theme = g.pack.theme.clone();
    let w = ui.available_width();
    // the masthead
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(masthead(&theme).to_uppercase()).family(egui::FontFamily::Name("head".into())).size(30.0).color(s.ink));
    });
    let (r, _) = ui.allocate_exact_size(vec2(w, 6.0), egui::Sense::hover());
    ui.painter().line_segment([pos2(r.min.x, r.min.y + 1.0), pos2(r.max.x, r.min.y + 1.0)], egui::Stroke::new(2.0, s.ink));
    ui.painter().line_segment([pos2(r.min.x, r.max.y - 1.0), pos2(r.max.x, r.max.y - 1.0)], egui::Stroke::new(0.8, s.ink));
    ui.horizontal(|ui| {
        let (fr, _) = ui.allocate_exact_size(vec2(24.0, 16.0), egui::Sense::hover());
        if let Some(fl) = super::flags::get(ui.ctx(), g.h.war.th.countries[g.me]) {
            let icons = chrome::icons(ui.ctx());
            super::flags::paint(ui.ctx(), ui.painter(), fr.shrink(1.0), &fl, s.ink, Some(&icons));
        }
        words(ui, g.h.home(g.me).name.to_uppercase(), 12.5, s.soft);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            words(ui, g.date(), 12.5, s.soft);
        });
    });
    let (r, _) = ui.allocate_exact_size(vec2(w, 4.0), egui::Sense::hover());
    ui.painter().line_segment([pos2(r.min.x, r.center().y), pos2(r.max.x, r.center().y)], egui::Stroke::new(0.8, s.ink));
    ui.add_space(4.0);
    if g.news.is_empty() {
        words(ui, "Пока тихо: вестей нет.", 15.0, s.soft);
    }
    for (i, e) in g.news.iter().rev().take(40).enumerate() {
        let resp = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(RichText::new(date_words(e.month).to_uppercase()).family(egui::FontFamily::Name("head".into())).size(12.5).color(s.soft));
                let (size, family) = if i == 0 { (18.0, "bold") } else if e.hot { (15.5, "bold") } else { (15.0, "") };
                let text = RichText::new(&e.text).size(size).color(if e.hot || i == 0 { s.ink } else { s.soft });
                let text = if family.is_empty() { text } else { text.family(egui::FontFamily::Name(family.into())) };
                ui.label(text);
            })
            .response;
        if !e.tip.is_empty() {
            resp.tip(|| Card::new(date_words(e.month)).icon("news").text(e.text.clone()).rule().note(e.tip.clone()));
        }
        // (a thin rule between stories; under the lead, a double one)
        let (r, _) = ui.allocate_exact_size(vec2(w, 7.0), egui::Sense::hover());
        let y = r.center().y;
        if i == 0 {
            ui.painter().line_segment([pos2(r.min.x, y - 1.5), pos2(r.max.x, y - 1.5)], egui::Stroke::new(1.2, s.ink));
            ui.painter().line_segment([pos2(r.min.x, y + 1.5), pos2(r.max.x, y + 1.5)], egui::Stroke::new(0.6, s.ink));
        } else {
            ui.painter().line_segment([pos2(r.min.x + w * 0.3, y), pos2(r.max.x - w * 0.3, y)], egui::Stroke::new(0.8, s.soft.gamma_multiply(0.6)));
        }
    }
    if !g.past.is_empty() {
        head(ui, s, "Летопись мира");
        // the years down a line on the left, a dot for each war; who struck whom, how it ended
        for w in g.past.iter().rev() {
            let resp = ui
                .horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(16.0, 40.0), egui::Sense::hover());
                    ui.painter().line_segment([pos2(r.center().x, r.min.y - 4.0), pos2(r.center().x, r.max.y + 4.0)], egui::Stroke::new(1.5, ch.trim));
                    ui.painter().circle(pos2(r.center().x, r.min.y + 9.0), 4.5, if w.winner.is_some() { s.accent } else { s.soft }, egui::Stroke::new(1.2, ch.trim));
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let ago = w.brief();
                        let (when, rest) = ago.split_once(" назад ").map_or((String::new(), ago.clone()), |(a, b)| (format!("{a} назад"), b.to_string()));
                        ui.label(RichText::new(when.to_uppercase()).family(egui::FontFamily::Name("head".into())).size(13.0).color(s.accent));
                        let (who, how) = rest.split_once(" — ").map_or((rest.clone(), String::new()), |(a, b)| (a.to_string(), b.to_string()));
                        ui.label(RichText::new(who).family(egui::FontFamily::Name("bold".into())).size(15.0).color(s.ink));
                        if !how.is_empty() {
                            ui.label(RichText::new(how).size(14.0).color(s.soft));
                        }
                    });
                })
                .response;
            resp.tip(|| Card::new("Летопись").icon("scroll").text(w.words()));
            ui.add_space(2.0);
        }
    }
    // the chronicle of societies: revolutions and coups, reforms, houses, faiths founded and split
    // (of our country and the rival's first, then the world's)
    if !g.polity.chronicle.is_empty() {
        head(ui, s, "Летопись обществ и вер");
        let pair = [g.country(0), g.country(1)];
        let now = g.month() as i32;
        let ours: Vec<&(i32, usize, String)> = g.polity.chronicle.iter().filter(|x| pair.contains(&x.1)).collect();
        let world: Vec<&(i32, usize, String)> = g.polity.chronicle.iter().filter(|x| !pair.contains(&x.1)).collect();
        for (title, list, k) in [("Наши страны", ours, 24usize), ("Мир", world, 30usize)] {
            if list.is_empty() {
                continue;
            }
            ui.label(RichText::new(title.to_uppercase()).family(egui::FontFamily::Name("head".into())).size(13.0).color(s.soft));
            for (m, _, line) in list.iter().rev().take(k) {
                let ago = (now - m).max(0) / 12;
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(if ago == 0 { "в этом году".to_string() } else { format!("{ago} {} назад", crate::words::years_word(ago as u32)) }).size(12.5).color(s.accent));
                    ui.label(RichText::new(line.as_str()).size(13.5).color(s.ink));
                });
            }
        }
    }
}

/// A colour for words of good or bad news on the folder's paper: `base` darkened (or lightened, on
/// a dark paper) till it reads as well as the folder's words must (contrast 4.6 or more).
pub(super) fn ink_of(s: &Skin, base: Color32) -> Color32 {
    let paper = s.paper;
    let light = super::uitest::contrast(Color32::WHITE, paper) > super::uitest::contrast(Color32::BLACK, paper);
    let mut c = base;
    for _ in 0..24 {
        if super::uitest::contrast(c, paper) >= 4.6 {
            break;
        }
        c = if light { super::hud::lighten(c, 0.12) } else { super::hud::darken(c, 0.12) };
    }
    c
}
