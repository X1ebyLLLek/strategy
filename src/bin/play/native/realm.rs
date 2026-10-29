//! The card of a country (36th session): its flag in a frame of trim, its name in the era's heading letters, a line under it, the pacts
//! we keep with it as small icons, what its government thinks of us as a figure on a plaque of the opinion's tone and as a two-way
//! gauge along the card's foot (the middle is «сдержанно», a notch of trim; the marker runs out to the side the opinion leans to).
//! Before, the neighbours and the world's list were bare rows of a table — a flag, a name, a badge, a hairline under them.

use eframe::egui::{self, epaint, pos2, vec2, Color32, FontFamily, FontId, Rect, Response, Sense, Shape, Stroke, Ui};

use super::chrome::{self, Out};
use super::hud;
use super::kit;

/// What a country card shows.
pub struct Realm {
    pub country: usize,
    pub name: String,
    /// The line under the name (regime, standing) and its colour (none: the soft ink).
    pub line: String,
    pub line_col: Option<Color32>,
    /// What its government thinks of us, −1 … +1.
    pub opinion: f32,
    /// The icons of the pacts we keep with it.
    pub pacts: Vec<&'static str>,
    /// Picked (lit, trimmed) or not.
    pub on: bool,
    /// The band at the card's edge (none: the opinion's tone).
    pub edge: Option<Color32>,
    /// Its economy against ours as pips (level 0 … 5, colour), if it matters.
    pub power: Option<(f32, Color32)>,
    /// Room to keep clear at the right, for a button put over the card.
    pub keep: f32,
}

impl Realm {
    pub fn new(country: usize, name: impl Into<String>, opinion: f32) -> Realm {
        Realm { country, name: name.into(), line: String::new(), line_col: None, opinion, pacts: Vec::new(), on: false, edge: None, power: None, keep: 0.0 }
    }
}

/// The tone of an opinion: friendly, reserved, hostile.
pub fn tone(x: f32) -> Color32 {
    if x >= 0.3 {
        kit::GOOD
    } else if x <= -0.3 {
        kit::BAD
    } else {
        kit::MIXED
    }
}

/// Height of a card.
pub const HEIGHT: f32 = 66.0;

/// The card, as wide as the room; its response and its rectangle (a button may be put into it).
pub fn card(ui: &mut Ui, r: &Realm) -> (Response, Rect) {
    let (ch, s) = chrome::dress(ui);
    let icons = chrome::icons(ui.ctx());
    let w = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 2.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, HEIGHT), Sense::click());
    ui.add_space(5.0);
    if !ui.is_rect_visible(rect) {
        return (resp, rect);
    }
    let tone = tone(r.opinion);
    let mut v = Out::new();
    kit::leaf_at(&mut v, &ch, &s, rect, Some(r.edge.unwrap_or(tone)), r.on || resp.hovered());
    if r.on {
        v.push(Shape::closed_line(hud::chamfer(rect.shrink(1.5), 6.0), Stroke::new(1.6, ch.trim)));
    }
    ui.painter().extend(v);
    // the flag, in a frame
    let fr = Rect::from_min_size(rect.min + vec2(15.0, 9.0), vec2(50.0, 33.0));
    let mut v = Out::new();
    v.push(epaint::Shadow { offset: vec2(0.0, 2.0), blur: 5.0, spread: 0.0, color: Color32::from_black_alpha(100) }.as_shape(fr.expand(2.0), 2.0).into());
    ui.painter().extend(v);
    if let Some(f) = super::flags::get(ui.ctx(), r.country) {
        super::flags::paint(ui.ctx(), ui.painter(), fr, &f, ch.trim, Some(&icons));
    }
    // the words
    let x = fr.max.x + 11.0;
    let right = rect.max.x - 10.0 - r.keep;
    let badge = Rect::from_min_size(pos2(right - 46.0, rect.min.y + 8.0), vec2(46.0, 22.0));
    let mut job = egui::text::LayoutJob::simple_singleline(r.name.to_uppercase(), FontId::new(15.0, FontFamily::Name("head".into())), s.ink);
    job.wrap = egui::text::TextWrapping::truncate_at_width((badge.min.x - 8.0 - x).max(40.0));
    let gal = ui.painter().layout_job(job);
    ui.painter().galley(pos2(x, rect.min.y + 8.0), gal, s.ink);
    // (the pacts between the line and the badge, from the right; the line stops where they begin)
    let mut px = right;
    for icon in &r.pacts {
        px -= 19.0;
        super::look::icon(&icons, icon, 17.0, Color32::from_black_alpha(120)).paint_at(ui, Rect::from_min_size(pos2(px + 0.8, rect.min.y + 27.0 + 0.8), vec2(17.0, 17.0)));
        super::look::icon(&icons, icon, 17.0, s.accent).paint_at(ui, Rect::from_min_size(pos2(px, rect.min.y + 27.0), vec2(17.0, 17.0)));
    }
    let line_col = r.line_col.unwrap_or(s.soft);
    let mut job = egui::text::LayoutJob::simple_singleline(r.line.clone(), FontId::proportional(13.0), line_col);
    job.wrap = egui::text::TextWrapping::truncate_at_width((px.min(right) - 6.0 - x).max(40.0));
    let gal = ui.painter().layout_job(job);
    ui.painter().galley(pos2(x, rect.min.y + 29.0), gal, line_col);
    // the figure on its plaque
    let mut v = Out::new();
    let plate = hud::lerp(tone, Color32::BLACK, 0.32);
    let pts = hud::chamfer(badge, 5.0);
    hud::lay_poly(&mut v, &pts, &ch.bar, Color32::WHITE);
    v.push(Shape::convex_polygon(pts.clone(), plate.gamma_multiply(0.85), Stroke::NONE));
    hud::gloss(&mut v, badge.shrink(1.0), Color32::from_white_alpha(38), Color32::from_black_alpha(50));
    v.push(Shape::closed_line(pts, Stroke::new(1.0, ch.trim.gamma_multiply(0.9))));
    ui.painter().extend(v);
    ui.painter().text(badge.center(), egui::Align2::CENTER_CENTER, format!("{:+.0}", r.opinion * 100.0), FontId::new(14.0, FontFamily::Name("bold".into())), Color32::WHITE);
    // the economy against ours, as pips
    if let Some((lvl, col)) = r.power {
        let mut v = Out::new();
        hud::pips(&mut v, pos2(right - 46.0, rect.min.y + 34.0), 5, lvl, col, 9.0, 5.0);
        ui.painter().extend(v);
    }
    // the gauge along the foot: a groove, the fill from the middle to the side the opinion leans to, a notch of trim in the middle
    let g = Rect::from_min_max(pos2(x, rect.max.y - 17.0), pos2(right, rect.max.y - 9.0));
    let mut v = Out::new();
    v.push(Shape::rect_filled(g, 2.0, Color32::from_black_alpha(70)));
    ch.bevel(&mut v, g, false, 1.0);
    let (mid, half) = (g.center().x, g.width() / 2.0 - 1.5);
    let o = r.opinion.clamp(-1.0, 1.0);
    let end = mid + half * o;
    let f = Rect::from_min_max(pos2(mid.min(end), g.min.y + 1.5), pos2(mid.max(end), g.max.y - 1.5));
    if f.width() > 0.5 {
        let mut m = epaint::Mesh::default();
        let (a, b) = (hud::lighten(tone, 0.28), hud::darken(tone, 0.2));
        for (p, c) in [(f.left_top(), a), (f.right_top(), a), (f.right_bottom(), b), (f.left_bottom(), b)] {
            m.colored_vertex(p, c);
        }
        m.add_triangle(0, 1, 2);
        m.add_triangle(0, 2, 3);
        v.push(Shape::mesh(m));
    }
    // (the thirds of each side: the marks of «расположена»/«дружественна» and their opposites)
    for k in [-0.6f32, -0.3, 0.3, 0.6] {
        let tx = mid + half * k;
        v.push(Shape::line_segment([pos2(tx, g.max.y - 3.0), pos2(tx, g.max.y - 1.0)], Stroke::new(1.0, Color32::from_black_alpha(120))));
    }
    v.push(Shape::line_segment([pos2(mid, g.min.y - 2.0), pos2(mid, g.max.y + 2.0)], Stroke::new(1.4, ch.trim)));
    v.push(Shape::rect_stroke(g, 2.0, Stroke::new(1.0, Color32::from_black_alpha(150))));
    // (the marker: a small diamond of trim on the end of the fill)
    hud::diamond(&mut v, pos2(end, g.center().y), 4.2, Color32::from_black_alpha(200));
    hud::diamond(&mut v, pos2(end, g.center().y), 3.0, hud::lighten(ch.trim, 0.2));
    ui.painter().extend(v);
    (resp, rect)
}

/// One side of a war card: its leading country (for the flag), the words for its name, its colour on the bar.
pub struct Camp {
    pub country: usize,
    pub name: String,
    pub colour: Color32,
}

/// A war of the world as a card: the flags of the two leading countries at the ends, crossed swords between them, the names under
/// the flags (`+ 2` when allies stand with them), a bar split by who leads (`lead`, 0 … 1 — the first camp's share) and a line with
/// the cause and how long it has gone on. The response is the card's.
pub fn war(ui: &mut Ui, a: &Camp, b: &Camp, lead: f32, line: &str) -> Response {
    let (ch, s) = chrome::dress(ui);
    let icons = chrome::icons(ui.ctx());
    let w = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 2.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 82.0), Sense::hover());
    ui.add_space(5.0);
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let mut v = Out::new();
    kit::leaf_at(&mut v, &ch, &s, rect, Some(kit::BAD), resp.hovered());
    ui.painter().extend(v);
    let fsize = vec2(38.0, 25.0);
    for (camp, left) in [(a, true), (b, false)] {
        let at = if left { pos2(rect.min.x + 16.0, rect.min.y + 9.0) } else { pos2(rect.max.x - 12.0 - fsize.x, rect.min.y + 9.0) };
        let fr = Rect::from_min_size(at, fsize);
        let mut v = Out::new();
        v.push(Shape::rect_filled(fr.expand(2.0), 2.0, camp.colour));
        v.push(Shape::rect_stroke(fr.expand(2.0), 2.0, Stroke::new(1.0, Color32::from_black_alpha(170))));
        ui.painter().extend(v);
        if let Some(f) = super::flags::get(ui.ctx(), camp.country) {
            super::flags::paint(ui.ctx(), ui.painter(), fr, &f, ch.trim, Some(&icons));
        }
        let room = rect.width() / 2.0 - fsize.x - 34.0;
        let mut job = egui::text::LayoutJob::simple_singleline(camp.name.to_uppercase(), FontId::new(13.5, FontFamily::Name("head".into())), s.ink);
        job.wrap = egui::text::TextWrapping::truncate_at_width(room.max(40.0));
        let gal = ui.painter().layout_job(job);
        let gx = if left { fr.max.x + 9.0 } else { fr.min.x - 9.0 - gal.size().x };
        ui.painter().galley(pos2(gx, fr.center().y - gal.size().y / 2.0), gal, s.ink);
    }
    let mid = pos2(rect.center().x, rect.min.y + 9.0 + fsize.y / 2.0);
    super::look::icon(&icons, "clash", 22.0, Color32::from_black_alpha(130)).paint_at(ui, Rect::from_center_size(mid + vec2(0.8, 1.0), vec2(22.0, 22.0)));
    super::look::icon(&icons, "clash", 22.0, s.accent).paint_at(ui, Rect::from_center_size(mid, vec2(22.0, 22.0)));
    // the bar: the first camp's share from the left, the second's from the right, a stroke between them
    let bar = Rect::from_min_max(pos2(rect.min.x + 16.0, rect.min.y + 44.0), pos2(rect.max.x - 12.0, rect.min.y + 55.0));
    let mut v = Out::new();
    v.push(Shape::rect_filled(bar, 2.0, Color32::from_black_alpha(90)));
    ch.bevel(&mut v, bar, false, 1.0);
    let inner = bar.shrink(1.5);
    let cut = inner.min.x + inner.width() * lead.clamp(0.02, 0.98);
    for (seg, col) in [(Rect::from_min_max(inner.min, pos2(cut, inner.max.y)), a.colour), (Rect::from_min_max(pos2(cut, inner.min.y), inner.max), b.colour)] {
        let mut m = epaint::Mesh::default();
        let (top, bot) = (hud::lighten(col, 0.25), hud::darken(col, 0.22));
        for (p, c) in [(seg.left_top(), top), (seg.right_top(), top), (seg.right_bottom(), bot), (seg.left_bottom(), bot)] {
            m.colored_vertex(p, c);
        }
        m.add_triangle(0, 1, 2);
        m.add_triangle(0, 2, 3);
        v.push(Shape::mesh(m));
    }
    v.push(Shape::line_segment([pos2(cut, bar.min.y - 2.0), pos2(cut, bar.max.y + 2.0)], Stroke::new(2.0, Color32::from_black_alpha(200))));
    v.push(Shape::line_segment([pos2(cut, bar.min.y - 2.0), pos2(cut, bar.max.y + 2.0)], Stroke::new(1.0, Color32::from_rgb(250, 244, 226))));
    v.push(Shape::line_segment([pos2(bar.center().x, bar.max.y - 3.0), pos2(bar.center().x, bar.max.y)], Stroke::new(1.0, Color32::from_black_alpha(160))));
    v.push(Shape::rect_stroke(bar, 2.0, Stroke::new(1.0, Color32::from_black_alpha(160))));
    ui.painter().extend(v);
    let mut job = egui::text::LayoutJob::simple_singleline(line.to_string(), FontId::proportional(12.5), s.soft);
    job.wrap = egui::text::TextWrapping::truncate_at_width(bar.width());
    let gal = ui.painter().layout_job(job);
    ui.painter().galley(pos2(bar.min.x, bar.max.y + 6.0), gal, s.soft);
    resp
}
