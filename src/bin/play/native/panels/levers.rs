//! Rows of levers, the way every folder of the economy shows them (29th session; before, a law was a
//! row of four tiles with words cut in the middle, a number a plate of five steps): the lever's icon
//! and name at the left, its control at the right — − and + around a figure, ‹ and › around the name of
//! the law in force, a switch — and under it, in a line of its own, what the choice in force means.
//! What a row asks of the game is returned (the decision as keys and values), not done.

use super::*;
use super::super::kit;
use crate::country::fiscal::Choice;

/// The icon and name that head a row.
fn head(ui: &mut egui::Ui, s: &Skin, icon: &str, name: &str) {
    let icons = chrome::icons(ui.ctx());
    let (ir, _) = ui.allocate_exact_size(vec2(22.0, 22.0), egui::Sense::hover());
    super::super::look::icon(&icons, icon, 20.0, s.ink).paint_at(ui, ir);
    kit::bold(ui, name.to_string(), 14.5, s.ink);
}

/// The hairline between rows.
pub(super) fn rule(ui: &mut egui::Ui, s: &Skin) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 7.0), egui::Sense::hover());
    ui.painter().line_segment([egui::pos2(r.min.x, r.center().y), egui::pos2(r.max.x, r.center().y)], egui::Stroke::new(0.8, s.soft.gamma_multiply(0.3)));
}

/// A number moved by − and +: its figure between the plates; −1, 0 or +1 for the plate pressed, and
/// the + plate's rectangle (for the window's self-checks). `enabled` false: greyed out and deaf.
pub(super) fn step(ui: &mut egui::Ui, s: &Skin, icon: &str, name: &str, now: &str, enabled: bool, about: &str) -> (i8, egui::Rect) {
    let mut out = (0, egui::Rect::NOTHING);
    ui.horizontal(|ui| {
        head(ui, s, icon, name);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            out = widgets::stepper(ui, now, 112.0, enabled);
        });
    });
    kit::words(ui, about.to_string(), 12.5, s.soft);
    out
}

/// A law of four choices: ‹ the one in force › — a press goes one step to a side; the choice asked
/// for, if one was.
pub(super) fn pick(ui: &mut egui::Ui, s: &Skin, icon: &str, name: &str, list: &[Choice; 4], now: usize) -> Option<usize> {
    let mut asked = None;
    ui.horizontal(|ui| {
        head(ui, s, icon, name);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // (the name of the law as wide as what the lever's own name leaves: it is cut short, not run
            // over the plates)
            let cw = (ui.available_width() - 2.0 * 28.0 - 2.0 * ui.spacing().item_spacing.x).clamp(60.0, 150.0);
            let next = (now + 1 < list.len()).then_some(now + 1);
            let prev = now.checked_sub(1);
            let b = widgets::plate(ui, "›", 28.0, false, next.is_some());
            // (a place for the window's self-checks: a control with a card of its own)
            if next.is_some() && ui.clip_rect().contains_rect(b.rect) {
                chrome::spot(ui.ctx(), "slider", b.rect);
            }
            if let Some(k) = next {
                if b.tip(|| Card::new(list[k].name).icon(list[k].icon).text(list[k].about).note("щёлкните — перейти к этому")).clicked() {
                    asked = Some(k);
                }
            }
            let (cw, _) = ui.allocate_exact_size(vec2(cw, 27.0), egui::Sense::hover());
            let mut job = egui::text::LayoutJob::simple_singleline(list[now].name.to_string(), egui::FontId::new(13.5, egui::FontFamily::Name("bold".into())), s.ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(cw.width());
            let gal = ui.painter().layout_job(job);
            ui.painter().galley(egui::pos2(cw.center().x - gal.size().x / 2.0, cw.center().y - gal.size().y / 2.0), gal, s.ink);
            let b = widgets::plate(ui, "‹", 28.0, false, prev.is_some());
            if let Some(k) = prev {
                if b.tip(|| Card::new(list[k].name).icon(list[k].icon).text(list[k].about).note("щёлкните — перейти к этому")).clicked() {
                    asked = Some(k);
                }
            }
        });
    });
    kit::words(ui, list[now].about, 12.5, s.soft);
    asked
}

/// A switch: a plate that says how it stands (`words`: off, on) and is lit while it is on; true if
/// it was pressed.
#[allow(clippy::too_many_arguments)]
pub(super) fn switch(ui: &mut egui::Ui, s: &Skin, icon: &str, name: &str, on: bool, words: [&str; 2], enabled: bool, about: &str) -> bool {
    let mut pressed = false;
    ui.horizontal(|ui| {
        head(ui, s, icon, name);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let b = widgets::plate(ui, words[on as usize], 92.0, on, enabled).tip(|| Card::new(name.to_string()).icon(icon.to_string()).text(about.to_string()).note(if on { "щёлкните — снять" } else { "щёлкните — ввести" }));
            pressed = b.clicked();
        });
    });
    kit::words(ui, about.to_string(), 12.5, s.soft);
    pressed
}

/// A line that sends the hand to another page: the button at the right.
pub(super) fn open(ui: &mut egui::Ui, s: &Skin, icon: &str, name: &str, button: &str, about: &str) -> bool {
    let mut pressed = false;
    ui.horizontal(|ui| {
        head(ui, s, icon, name);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            pressed = widgets::small(ui, button).clicked();
        });
    });
    kit::words(ui, about.to_string(), 12.5, s.soft);
    pressed
}
