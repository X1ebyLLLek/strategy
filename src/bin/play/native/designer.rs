//! The division designer in the army's folder: each template a card with its
//! battalions as a grid of signs (columns of five, as in HOI4); the chosen one open in the
//! designer — its grid (a click on a battalion takes it away), the kinds of troops (a click adds a
//! battalion), its qualities as bars against a plain division of the era (numbers on hover), how
//! much of it to build. Nothing changes in the game till it is saved.

use std::collections::HashMap;
use std::sync::Arc;
use eframe::egui::{self, pos2, vec2, Color32, Rect, RichText, Sense, Stroke};
use strategy::pack::Line;

use crate::army::templates::{self, Kind, MOST, WANT};
use crate::Game;
use strategy::military::doctrine::norm::{LEVELS, WORDS};
use super::chrome::{self, Card, Tip};
use super::look::{icon, Skin};
use super::terms::say;
use super::widgets;

mod appraise;
use appraise::appraise;

/// The template open in the designer, as the player changes it (not yet saved).
#[derive(Clone)]
pub struct Edit {
    /// Which template (None: a new one).
    pub which: Option<usize>,
    pub name: String,
    pub bat: Vec<u32>,
    pub want: usize,
    /// The kit of each kind (a level of `norm::LEVELS`), the companies of support it has.
    pub norm: Vec<usize>,
    pub support: Vec<bool>,
}

impl Edit {
    /// The template `t` (None: a new one) open in the designer.
    pub fn of(g: &Game, which: Option<usize>) -> Edit {
        let nu = g.pack.units.len();
        let level = |k: f32| (0..LEVELS.len()).min_by(|&a, &b| (LEVELS[a] - k).abs().total_cmp(&(LEVELS[b] - k).abs())).unwrap_or(1);
        match which.and_then(|k| g.templates.get(k)) {
            Some(t) => Edit {
                which,
                name: t.name.clone(),
                bat: t.bat.clone(),
                want: want_of(t.want),
                norm: (0..nu).map(|u| level(t.norm_of(u))).collect(),
                support: (0..g.pack.support.len()).map(|k| t.support.contains(&k)).collect(),
            },
            None => Edit { which: None, name: "Новая дивизия".into(), bat: vec![0; nu], want: 2, norm: vec![1; nu], support: vec![false; g.pack.support.len()] },
        }
    }

    /// The norms as the template keeps them (kinds without norms standard).
    pub fn norms(&self, pack: &strategy::pack::Pack) -> Vec<f32> {
        (0..self.bat.len()).map(|u| if pack.units[u].norms { LEVELS[self.norm.get(u).copied().unwrap_or(1).min(2)] } else { 1.0 }).collect()
    }

    /// The companies it has (their numbers in the pack).
    pub fn companies(&self) -> Vec<usize> {
        (0..self.support.len()).filter(|&k| self.support[k]).collect()
    }
}

/// The kinds' colours.
pub const KIND_COLORS: [Color32; 8] = [
    Color32::from_rgb(0x5d, 0x7a, 0x43),
    Color32::from_rgb(0x7a, 0x5f, 0x33),
    Color32::from_rgb(0x9e, 0x4a, 0x35),
    Color32::from_rgb(0x45, 0x62, 0x8e),
    Color32::from_rgb(0x80, 0x6a, 0x9e),
    Color32::from_rgb(0xa8, 0x89, 0x2f),
    Color32::from_rgb(0x4d, 0x8f, 0x8a),
    Color32::from_rgb(0x7d, 0x7d, 0x7d),
];

pub fn want_of(w: f32) -> usize {
    (0..WANT.len()).min_by(|&a, &b| (WANT[a].1 - w).abs().total_cmp(&(WANT[b].1 - w).abs())).unwrap_or(2)
}

/// The battalions in the order of the grid: front-line kinds first, then support, then the air.
fn cells(bat: &[u32], kinds: &[Kind]) -> Vec<usize> {
    let rank = |l: Line| match l {
        Line::Front => 0,
        Line::Support => 1,
        Line::Air => 2,
    };
    let mut order: Vec<usize> = (0..kinds.len()).collect();
    order.sort_by_key(|&u| (rank(kinds[u].line), u));
    order.into_iter().flat_map(|u| std::iter::repeat(u).take(bat[u] as usize)).collect()
}

/// A grid of battalions, `cell` points a side: each battalion a plate of its kind's colour with
/// its sign, in a slot sunk into the paper; with `edit`, the empty slots of a full division too.
/// The battalion clicked, if one was.
fn grid(ui: &mut egui::Ui, icons: &HashMap<String, Arc<[u8]>>, bat: &[u32], kinds: &[Kind], cell: f32, edit: bool) -> Option<usize> {
    let cs = cells(bat, kinds);
    let cols = if edit { 5 } else { cs.len().div_ceil(5).max(1) };
    let gap = if edit { 4.0 } else { 2.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(cols as f32 * (cell + gap), 5.0 * (cell + gap)), Sense::hover());
    let ch = chrome::of(ui.ctx());
    let mut clicked = None;
    for c in 0..cols {
        for r in 0..5 {
            let at = Rect::from_min_size(pos2(rect.min.x + c as f32 * (cell + gap), rect.min.y + r as f32 * (cell + gap)), vec2(cell, cell));
            match cs.get(c * 5 + r) {
                Some(&u) => {
                    let resp = ui.interact(at, ui.id().with(("bat", c, r)), if edit { Sense::click() } else { Sense::hover() });
                    let resp = if edit { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
                    let fill = KIND_COLORS[u % KIND_COLORS.len()];
                    let hot = resp.hovered() && edit;
                    let mut v = chrome::Out::new();
                    v.push(egui::Shape::rect_filled(at, 2.0, if hot { fill.linear_multiply(1.25) } else { fill }));
                    if let Some(ch) = &ch {
                        ch.gloss(&mut v, at, Color32::from_white_alpha(if edit { 60 } else { 40 }), Color32::from_black_alpha(60));
                        if edit {
                            ch.bevel(&mut v, at, true, 1.5);
                        }
                    }
                    v.push(egui::Shape::rect_stroke(at, 2.0, Stroke::new(if hot { 1.6 } else { 1.0 }, if hot { ch.as_ref().map_or(Color32::WHITE, |c| c.trim) } else { Color32::from_black_alpha(160) })));
                    ui.painter().extend(v);
                    let img = icon(icons, &kinds[u].icon, cell * 0.7, Color32::from_rgb(248, 242, 228));
                    img.paint_at(ui, Rect::from_center_size(at.center(), vec2(cell * 0.7, cell * 0.7)));
                    let name = kinds[u].name.clone();
                    let iname = kinds[u].icon.clone();
                    if resp.tip(|| Card::new(name).icon(iname).text(if edit { "Батальон дивизии." } else { "" }).note(if edit { "щёлкните — убрать" } else { "" })).clicked() {
                        clicked = Some(u);
                    }
                }
                None if edit => {
                    let mut v = chrome::Out::new();
                    v.push(egui::Shape::rect_filled(at, 2.0, Color32::from_black_alpha(30)));
                    if let Some(ch) = &ch {
                        ch.bevel(&mut v, at, false, 1.2);
                    }
                    v.push(egui::Shape::rect_stroke(at, 2.0, Stroke::new(1.0, Color32::from_black_alpha(70))));
                    ui.painter().extend(v);
                }
                None => {}
            }
        }
    }
    clicked
}

/// The army's divisions: the templates, the staff's own while it builds, the designer.
pub fn section(ui: &mut egui::Ui, s: &Skin, g: &mut Game, icons: &HashMap<String, Arc<[u8]>>, edit: &mut Option<Edit>) {
    let me = g.me;
    let c = &g.h.war.sides[me];
    let kinds = templates::kinds(&g.pack, c);
    let fixed = c.fixed;
    let staff_tpl = templates::of_plan("Дивизия генштаба", &c.plan, c);
    // what the factories give a month, in divisions
    let size = if fixed { templates::division_size(&g.templates) } else { 1.0 };
    let per_month: f32 = g.h.last_made[me].iter().sum::<f32>() / size.max(1e-6);
    let pace = match per_month {
        x if x < 0.5 => "заводы почти ничего не дают",
        x if x < 3.0 => "заводы дают по нескольку дивизий в месяц",
        x if x < 15.0 => "заводы дают десяток дивизий в месяц",
        _ => "заводы дают десятки дивизий в месяц",
    };
    say(ui, s, "[template]Шаблон — из каких батальонов собрана дивизия; по шаблонам армия заказывает заводам.");
    ui.label(format!("Сейчас {pace}.")).tip(|| Card::new("Заводы").icon("anvil").row("дивизий в месяц", format!("≈{per_month:.1}")));
    let mut acts: Vec<Vec<(&str, String)>> = Vec::new();
    if !fixed {
        ui.label(RichText::new("Сейчас армию строит генштаб — вот его дивизия. Сохраните свой шаблон, и армию будут строить по вашим.").size(13.5).color(s.soft));
        let back = chrome::hold(ui);
        let fr = egui::Frame::none().inner_margin(7.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                grid(ui, icons, &staff_tpl.bat, &kinds, 13.0, false);
                ui.vertical(|ui| {
                    ui.label(RichText::new(&staff_tpl.name).family(egui::FontFamily::Name("bold".into())).size(15.5));
                    ui.label(RichText::new(format!("так строит генштаб; дивизия {}", templates::size_word(staff_tpl.battalions()))).size(13.5));
                    if widgets::small(ui, "Взять этот шаблон себе").tip(|| Card::new("Взять себе").icon("army").text("Шаблон генштаба станет вашим, и армию будут строить по вашим шаблонам.")).clicked() {
                        acts.push(vec![("tpl_take", "1".into()), ("ctrl", "me".into())]);
                    }
                });
            });
        });
        card_under(ui, back, fr.response.rect, false, s);
        ui.add_space(4.0);
    }
    // the templates as cards, as many in a row as the folder holds (a frame's size is known only
    // once drawn: the row is broken before it); a click opens one in the designer
    let per_row = ((ui.available_width() + 8.0) / 180.0).floor().max(1.0) as usize;
    ui.horizontal_wrapped(|ui| {
        for (k, t) in g.templates.iter().enumerate() {
            if k > 0 && k % per_row == 0 {
                ui.end_row();
            }
            let on = edit.as_ref().is_some_and(|e| e.which == Some(k));
            let back = chrome::hold(ui);
            let resp = egui::Frame::none().inner_margin(6.0).show(ui, |ui| {
                ui.set_width(156.0);
                ui.horizontal(|ui| {
                    grid(ui, icons, &t.bat, &kinds, 10.0, false);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&t.name).family(egui::FontFamily::Name("bold".into())).size(14.5));
                        ui.label(RichText::new(format!("строят: {}", WANT[want_of(t.want)].0)).size(12.5));
                        ui.label(RichText::new(templates::size_word(t.battalions())).size(12.5).color(s.soft));
                        ui.label(RichText::new(kit_word(&g.pack, t)).size(12.5).color(s.soft));
                    });
                });
            });
            card_under(ui, back, resp.response.rect, on, s);
            let q = templates::qualities_of(&g.pack, &g.h.war.sides[me], &t.bat, &t.norm, &t.support, &kinds);
            let tname = t.name.clone();
            let sup: Vec<String> = t.support.iter().filter_map(|&k| g.pack.support.get(k)).map(|x| x.name.clone()).collect();
            if resp
                .response
                .interact(Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .tip(|| {
                    Card::new(tname)
                        .icon("army")
                        .row("батальонов", q.n.to_string())
                        .row("людей", format!("{:.1} тыс.", q.men))
                        .row("роты", if sup.is_empty() { "нет".to_string() } else { sup.join(", ") })
                        .note(if on { "открыт в конструкторе" } else { "щёлкните — открыть в конструкторе" })
                })
                .clicked()
            {
                *edit = if on { None } else { Some(Edit::of(g, Some(k))) };
            }
        }
        if !g.templates.is_empty() && g.templates.len() % per_row == 0 {
            ui.end_row();
        }
        let on = edit.as_ref().is_some_and(|e| e.which.is_none());
        let back = chrome::hold(ui);
        let resp = egui::Frame::none().inner_margin(6.0).show(ui, |ui| {
            ui.set_width(156.0);
            ui.set_height(64.0);
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("+ новый шаблон").family(egui::FontFamily::Name("head".into())).size(15.0));
            });
        });
        card_under(ui, back, resp.response.rect, on, s);
        if resp.response.interact(Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(|| Card::new("Новый шаблон").icon("army").text("Соберите дивизию из батальонов в конструкторе.")).clicked() {
            *edit = if on { None } else { Some(Edit::of(g, None)) };
        }
    });
    if edit.is_some() {
        ui.label(RichText::new("Шаблон открыт в конструкторе справа.").size(13.5).color(s.soft));
    }
    for a in acts {
        let form: Vec<(String, String)> = a.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        g.act(crate::tools::saves::Act::Form(form));
    }
}

//// The designer of the template being made or changed: a window beside the folder (its left edge
/// at `x`). Where the screen is wide enough, in two columns — the template made on the left (its
/// name, its grid, the kinds of troops, how much to build), what it is on the right (its qualities
/// against the plain division, the staff's trials); else in one.
pub fn window(ui_ctx: &egui::Context, x: f32, s: &Skin, g: &mut Game, icons: &HashMap<String, Arc<[u8]>>, edit: &mut Option<Edit>) {
    let me = g.me;
    let kinds = templates::kinds(&g.pack, &g.h.war.sides[me]);
    let mut acts: Vec<Vec<(&str, String)>> = Vec::new();
    let mut close = false;
    let Some(e) = edit.as_mut() else { return };
    let title = if e.which.is_some() { "ШАБЛОН ДИВИЗИИ" } else { "НОВЫЙ ШАБЛОН" };
    let ch = super::chrome::of(ui_ctx);
    let shadow = ui_ctx.style().visuals.window_shadow;
    // (under the country's bar, above the groups' bar: what does not fit scrolls)
    let room = (ui_ctx.screen_rect().height() - 70.0 - 120.0).max(240.0);
    let wide = ui_ctx.screen_rect().width() - x >= 880.0;
    egui::Window::new(title)
        .id(egui::Id::new("designer"))
        .title_bar(false)
        .anchor(egui::Align2::LEFT_TOP, [x, 66.0])
        .collapsible(false)
        .resizable(false)
        .default_width(if wide { 820.0 } else { 414.0 })
        // (as tall as the screen allows: a window sized by its first frame stayed short)
        .max_height(room + 40.0)
        .default_height(room + 40.0)
        .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(20.0, 16.0)).shadow(shadow).rounding(3.0))
        .show(ui_ctx, |ui| {
            let back = super::chrome::hold(ui);
            ui.label(RichText::new(title).family(egui::FontFamily::Name("head".into())).size(20.0).color(s.accent));
            widgets::scroll(ui, "designer-scroll", room, false, |ui| {
                if wide {
                    ui.set_width(812.0);
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(392.0);
                            compose(ui, s, icons, &kinds, e);
                            equip(ui, s, g, icons, &kinds, e);
                            decide(ui, s, g, e, &mut acts, &mut close);
                        });
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            ui.set_width(392.0);
                            appraise(ui, s, g, &kinds, e);
                        });
                    });
                } else {
                    ui.set_width(400.0);
                    compose(ui, s, icons, &kinds, e);
                    equip(ui, s, g, icons, &kinds, e);
                    appraise(ui, s, g, &kinds, e);
                    decide(ui, s, g, e, &mut acts, &mut close);
                }
            });
            if let Some(ch) = &ch {
                let mut v = super::chrome::Out::new();
                ch.sheet(&mut v, ui.min_rect().expand2(egui::vec2(20.0, 16.0)), 8.0);
                super::chrome::under_window(ui, back, v);
            }
        });
    if close {
        *edit = None;
    }
    for a in acts {
        let form: Vec<(String, String)> = a.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        g.act(crate::tools::saves::Act::Form(form));
    }
}

/// The template as it is made: its name, its grid of battalions (a click takes one away), the
/// kinds of troops (a click adds one).
fn compose(ui: &mut egui::Ui, s: &Skin, icons: &HashMap<String, Arc<[u8]>>, kinds: &[Kind], e: &mut Edit) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Название").size(15.0).color(s.soft));
        widgets::field(ui, &mut e.name, 250.0, 40);
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if let Some(u) = grid(ui, icons, &e.bat, kinds, 28.0, true) {
            e.bat[u] -= 1;
        }
        ui.add_space(6.0);
        ui.vertical(|ui| {
            let n = e.bat.iter().sum::<u32>();
            ui.label(RichText::new("БАТАЛЬОНЫ").family(egui::FontFamily::Name("head".into())).size(13.0).color(s.soft));
            ui.label(RichText::new(templates::size_word(n)).family(egui::FontFamily::Name("bold".into())).size(15.0));
            ui.add_space(4.0);
            say(ui, s, "Щёлкните по роду войск — добавить [battalion]батальон; по батальону в сетке — убрать.");
        });
    });
    ui.add_space(4.0);
    let full = e.bat.iter().sum::<u32>() >= MOST;
    let ch = chrome::of(ui.ctx());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
        for (u, k) in kinds.iter().enumerate() {
            let Some(ch) = &ch else { break };
            let img = icon(icons, &k.icon, 18.0, KIND_COLORS[u % KIND_COLORS.len()].linear_multiply(1.4));
            let b = chrome::press(ui, ch, s, chrome::Press { text: &k.name, icon: Some(img), size: vec2(0.0, 30.0), enabled: !full, font: 13.5, ..chrome::Press::default() });
            let (name, ic) = (k.name.clone(), k.icon.clone());
            if b.tip(|| Card::new(name).icon(ic).text(if full { "Дивизия полна: больше батальонов не вместить." } else { "Щёлкните — добавить батальон этого рода." })).clicked() {
                e.bat[u] += 1;
            }
        }
    });
}

/// The template's equipment: the kit of each of its kinds that has norms (light, standard,
/// reinforced: machine guns, mortars, lorries to a battalion — cheaper and weaker, or dearer and
/// stronger), and its companies of support (one to a division, serving the whole of it).
fn equip(ui: &mut egui::Ui, s: &Skin, g: &Game, icons: &HashMap<String, Arc<[u8]>>, kinds: &[Kind], e: &mut Edit) {
    let pack = &g.pack;
    let with: Vec<usize> = (0..kinds.len()).filter(|&u| pack.units[u].norms && e.bat[u] > 0).collect();
    ui.add_space(6.0);
    ui.label(RichText::new("ОСНАЩЕНИЕ").family(egui::FontFamily::Name("head".into())).size(13.0).color(s.soft)).tip(|| {
        Card::new("Оснащение")
            .icon("supply")
            .text("Сколько пулемётов, миномётов, грузовиков на батальон. Лёгкое — вдвое меньше снаряжения: дешевле, но огонь слабее и потери больше; усиленное — в полтора раза больше: дороже, сильнее, ест больше боеприпасов и топлива.")
            .note("где снаряжение дорого, а людей много, выгоднее лёгкое; где люди дороги — усиленное")
    });
    if with.is_empty() {
        ui.label(RichText::new("у этих родов войск оснащение — сама техника: его меняют разработки, не норма").size(13.0).color(s.soft));
    }
    for u in with {
        ui.horizontal(|ui| {
            let img = icon(icons, &kinds[u].icon, 18.0, KIND_COLORS[u % KIND_COLORS.len()].linear_multiply(1.4));
            img.paint_at(ui, Rect::from_min_size(ui.cursor().min + vec2(0.0, 6.0), vec2(18.0, 18.0)));
            ui.add_space(22.0);
            let (r, _) = ui.allocate_exact_size(vec2(118.0, 30.0), Sense::hover());
            let mut job = egui::text::LayoutJob::simple_singleline(kinds[u].name.clone(), egui::FontId::new(14.0, egui::FontFamily::Proportional), s.ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(116.0);
            let gal = ui.painter().layout_job(job);
            ui.painter().galley(pos2(r.min.x, r.center().y - gal.size().y / 2.0), gal, s.ink);
            if let Some(k) = widgets::switch(ui, &WORDS, e.norm[u].min(2)) {
                e.norm[u] = k;
            }
        });
    }
    if pack.support.is_empty() {
        return;
    }
    ui.add_space(6.0);
    ui.label(RichText::new("РОТЫ ПОДДЕРЖКИ").family(egui::FontFamily::Name("head".into())).size(13.0).color(s.soft)).tip(|| {
        Card::new("Роты поддержки").icon("gears").text("Одна рота на дивизию, служит всей дивизии: крупной столько же, сколько мелкой, за ту же цену. Чем крупнее дивизия, тем дешевле ей роты.")
    });
    let Some(ch) = chrome::of(ui.ctx()) else { return };
    let level = g.h.war.sides[g.me].cost.iter().sum::<f32>() / pack.units.iter().map(|u| u.cost).sum::<f32>().max(1e-6);
    let base: f32 = (0..kinds.len()).map(|u| e.bat[u] as f32 * kinds[u].cost).sum::<f32>().max(1e-6);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
        for (k, x) in pack.support.iter().enumerate() {
            let on = e.support.get(k).copied().unwrap_or(false);
            let img = icon(icons, &x.icon, 18.0, if on { s.on_accent() } else { s.ink });
            let b = chrome::press(ui, &ch, s, chrome::Press { text: &x.name, icon: Some(img), size: vec2(0.0, 30.0), on, font: 13.5, ..chrome::Press::default() });
            let (name, about, ic) = (x.name.clone(), x.about.clone(), x.icon.clone());
            let share = x.cost * level / base;
            let men = x.men * pack.war.as_ref().map_or(10.0, |w| w.crew) * 1000.0;
            if b.tip(|| {
                Card::new(name)
                    .icon(ic)
                    .text(about)
                    .row("людей", format!("{men:.0}"))
                    .row("к цене дивизии", format!("+{:.0}%", share * 100.0))
                    .note(if on { "щёлкните — убрать" } else { "щёлкните — добавить" })
            })
            .clicked()
            {
                if let Some(v) = e.support.get_mut(k) {
                    *v = !*v;
                }
            }
        }
    });
}

/// A template's kit and companies in words (for its card).
fn kit_word(pack: &strategy::pack::Pack, t: &templates::Template) -> String {
    let ks: Vec<f32> = (0..t.bat.len()).filter(|&u| pack.units[u].norms && t.bat[u] > 0).map(|u| t.norm_of(u)).collect();
    let kit = if ks.is_empty() {
        "техника".to_string()
    } else {
        let mean = ks.iter().sum::<f32>() / ks.len() as f32;
        format!("оснащение {}", strategy::military::doctrine::norm::word(mean))
    };
    match t.support.len() {
        0 => format!("{kit}, без рот"),
        1 => format!("{kit}, одна рота"),
        2..=4 => format!("{kit}, несколько рот"),
        _ => format!("{kit}, все роты"),
    }
}

/// How much of it to build, and the buttons: save, delete, close.
fn decide(ui: &mut egui::Ui, s: &Skin, g: &Game, e: &mut Edit, acts: &mut Vec<Vec<(&'static str, String)>>, close: &mut bool) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Строить:").size(15.0).color(s.soft));
        let (r, _) = ui.allocate_exact_size(vec2(96.0, 20.0), Sense::hover());
        ui.painter().text(pos2(r.min.x, r.center().y), egui::Align2::LEFT_CENTER, WANT[e.want].0, egui::FontId::new(15.0, egui::FontFamily::Name("bold".into())), s.ink);
        let mut w = e.want as f32;
        ui.scope(|ui| {
            ui.set_width(ui.available_width());
            if widgets::slider(ui, &mut w, 0.0..=(WANT.len() - 1) as f32, None, WANT.len()).changed() {
                e.want = (w.round() as usize).min(WANT.len() - 1);
            }
        });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let csv = e.bat.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",");
        let which = e.which.map_or("new".to_string(), |k| k.to_string());
        let empty = e.bat.iter().sum::<u32>() == 0;
        if widgets::plate(ui, "Сохранить", 130.0, true, !empty).tip(|| Card::new("Сохранить шаблон").icon("stamper").text("Армия будет строить по нему, если строят по вашим шаблонам; в войне новые дивизии формируются по нему.")).clicked() {
            let norm = e.norm.iter().map(|k| k.to_string()).collect::<Vec<_>>().join(",");
            let sup = e.companies().iter().map(|k| k.to_string()).collect::<Vec<_>>().join(",");
            acts.push(vec![("tpl", which.clone()), ("tpl_bat", csv), ("tpl_name", e.name.clone()), ("tpl_want", e.want.to_string()), ("tpl_norm", norm), ("tpl_sup", sup)]);
            if e.which.is_none() {
                e.which = Some(g.templates.len());
            }
        }
        if e.which.is_some_and(|k| k < g.templates.len()) && widgets::plate(ui, "Удалить", 100.0, false, true).clicked() {
            acts.push(vec![("tpl", which), ("tpl_del", "1".into())]);
            *close = true;
        }
        if widgets::plate(ui, "Закрыть", 100.0, false, true).clicked() {
            *close = true;
        }
    });
}

// Under a card of a template: a plate of paper raised off the page (sunk and trimmed when it is
/// the one open).
fn card_under(ui: &egui::Ui, back: egui::layers::ShapeIdx, r: Rect, on: bool, s: &Skin) {
    let Some(ch) = chrome::of(ui.ctx()) else { return };
    let mut v = chrome::Out::new();
    ch.lay(&mut v, r, &ch.paper, 2.0);
    ch.gloss(&mut v, r, Color32::from_white_alpha(if on { 15 } else { 50 }), Color32::from_black_alpha(if on { 45 } else { 30 }));
    ch.bevel(&mut v, r, !on, 1.5);
    v.push(egui::Shape::rect_stroke(r, 2.0, Stroke::new(if on { 2.0 } else { 1.0 }, if on { s.accent } else { Color32::from_black_alpha(120) })));
    chrome::under(ui, back, v);
}

