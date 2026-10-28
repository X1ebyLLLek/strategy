//! The window's self-checks (`play --native --uitest [--control]`): the window is drawn without a
//! screen (egui alone, what it would paint is looked at), folder by folder, at two sizes of the
//! window, and checked: the war is not hidden under the panels, numbers
//! only on hover, letters readable, the folder does not push the map aside, the groups' cards all
//! seen, counters not on each other, the camera tilted close up and straight down from afar, a
//! click found again where the hand is (in the mountains too), the icons there, a month of war
//! sending the video card only the table of provinces. `--control` spoils the window on purpose:
//! each check must fail there.

use std::sync::{Arc, Mutex};
use eframe::egui::{self, epaint, Color32, Pos2, Rect, Shape};

use crate::Game;
use super::{designer, map, App, Tab};

mod cards;
mod divs;
mod land;
mod layout;
mod policy;
mod war;
mod world;

/// A text the window paints: the words, where, how large, its colour and what lies under it, the
/// part of the window it is drawn within.
struct Text {
    text: String,
    rect: Rect,
    size: f32,
    colour: Color32,
    under: Option<Color32>,
    /// The box that lies under it (for the debug print).
    under_box: Option<Rect>,
    clip: Rect,
}

/// The texts of a frame that are seen, each with the colour of the last filled box under it
/// (painted before it); a text with a filled box painted over it is hidden.
fn texts(shapes: &[epaint::ClippedShape]) -> Vec<Text> {
    texts_of(shapes, false)
}

/// The texts of a frame, `hidden` too: those a filled box painted later covers.
fn texts_of(shapes: &[epaint::ClippedShape], hidden: bool) -> Vec<Text> {
    fn walk(shape: &Shape, clip: Rect, boxes: &mut Vec<(Rect, Color32)>, out: &mut Vec<Text>) {
        match shape {
            Shape::Vec(v) => {
                for s in v {
                    walk(s, clip, boxes, out);
                }
            }
            Shape::Rect(r) if r.fill.a() > 200 => boxes.push((r.rect.intersect(clip), r.fill)),
            Shape::Text(t) => {
                let rect = t.galley.rect.translate(t.pos.to_vec2());
                // (a text mostly cut off — a name of the map under a panel — is not seen)
                if !clip.contains(rect.center()) || t.galley.job.text.trim().is_empty() {
                    return;
                }
                let sec = t.galley.job.sections.first();
                let size = sec.map_or(14.0, |s| s.format.font_id.size);
                let mut colour = sec.map_or(t.fallback_color, |s| s.format.color);
                if colour == Color32::PLACEHOLDER {
                    colour = t.fallback_color;
                }
                if let Some(c) = t.override_text_color {
                    colour = c;
                }
                let c = rect.center();
                let found = boxes.iter().rev().find(|b| b.0.contains(c));
                let (under, under_box) = (found.map(|b| b.1), found.map(|b| b.0));
                out.push(Text { text: t.galley.job.text.clone(), rect, size, colour, under, under_box, clip });
            }
            _ => {}
        }
    }
    let mut boxes = Vec::new();
    let mut out = Vec::new();
    // (how many boxes had been painted before each text)
    let mut seen_at: Vec<usize> = Vec::new();
    for s in shapes {
        let n = out.len();
        walk(&s.shape, s.clip_rect, &mut boxes, &mut out);
        for _ in n..out.len() {
            seen_at.push(boxes.len());
        }
    }
    out.into_iter().zip(seen_at).filter(|(t, k)| hidden || !boxes[*k..].iter().any(|b| b.0.contains(t.rect.center()))).map(|(t, _)| t).collect()
}

/// The contrast of two colours (WCAG).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let lum = |c: Color32| {
        let v = |x: u8| {
            let x = x as f32 / 255.0;
            if x <= 0.03928 {
                x / 12.92
            } else {
                ((x + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * v(c.r()) + 0.7152 * v(c.g()) + 0.0722 * v(c.b())
    };
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// The window drawn once more without a screen, `w`×`h` points; what egui would paint.
fn frame(ctx: &egui::Context, app: &mut App, w: f32, h: f32) -> egui::FullOutput {
    let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(w, h))), ..Default::default() };
    ctx.run(input, |ctx| app.draw(ctx))
}

/// The window drawn once more at the time `t` (seconds), with these events (the hand moved, a
/// button pressed or let go).
fn frame_at(ctx: &egui::Context, app: &mut App, w: f32, h: f32, events: Vec<egui::Event>, t: f64) -> egui::FullOutput {
    let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(w, h))), events, time: Some(t), ..Default::default() };
    ctx.run(input, |ctx| app.draw(ctx))
}

/// The self-checks; whether all passed.
pub fn uitest(shared: Arc<Mutex<Game>>, control: bool) -> bool {
    super::SPOILED.store(control, std::sync::atomic::Ordering::Relaxed);
    let theme = shared.lock().unwrap().pack.theme.clone();
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let mut app = App::new(shared.clone(), &ctx, &theme, None);
    app.autosaved = u32::MAX;
    let mut lines: Vec<String> = Vec::new();
    let t0 = std::time::Instant::now();
    let mut say = |ok: bool, name: &str, detail: String| {
        if std::env::var("UITIME").is_ok() {
            eprintln!("{:.1} с: {name}", t0.elapsed().as_secs_f32());
        }
        lines.push(format!("[{}] {name} — {detail}", if ok { "OK" } else { "FAIL" }));
        ok
    };
    layout::folders(&shared, &mut app, &ctx, t0, &mut say);
    // (`UIONLY=layout`: the layout checks alone, for a quick look)
    if std::env::var("UIONLY").is_ok_and(|v| v == "layout") {
        for l in &lines {
            println!("{l}");
        }
        return lines.iter().all(|l| l.starts_with("[OK]"));
    }
    land::counters(&mut app, &ctx, &mut say);
    land::camera(&mut app, &ctx, &mut say);
    land::clicks(&shared, &mut app, &ctx, &mut say);
    cards::icons(&shared, &mut app, control, &mut say);
    land::war_month(&shared, &mut app, &ctx, &mut say);
    land::glide(&shared, &mut app, &ctx, &mut say);
    cards::tips(&shared, &mut app, &ctx, &mut say);
    cards::cursor(&shared, &mut app, &ctx, t0, &mut say);
    land::edges(&shared, &mut app, &ctx, &mut say);
    land::lines(&shared, &mut app, &mut say);
    world::rail(&shared, &mut say);
    world::flags(&shared, &mut say);
    land::fog(&shared, &mut say);
    land::towns(&shared, &mut app, &mut say);
    land::relief(&shared, &mut app, &mut say);
    cards::alert(&shared, &mut app, &ctx, &mut say);
    world::draft_law(&shared, &mut app, &ctx, &mut say);
    // 33–35. the country's folders: its features, the order of its economy, the priorities of
    // production
    policy::features(&shared, &mut app, &ctx, &mut say);
    policy::eco_law(&shared, &mut app, &ctx, &mut say);
    policy::priority(&shared, &mut app, &ctx, &mut say);
    policy::tax_step(&shared, &mut app, &ctx, &mut say);
    divs::stacks(&shared, &mut app, &ctx, &mut say);
    // 26–28. the war's orders: the groups' plans, the chosen divisions' way, the keys
    war::plans(&shared, &mut app, &ctx, &mut say);
    frame_at(&ctx, &mut app, 1600.0, 900.0, vec![egui::Event::PointerGone], ctx.input(|i| i.time) + 1.0 / 60.0);
    for l in &lines {
        println!("{l}");
    }
    lines.iter().all(|l| l.starts_with("[OK]"))
}
