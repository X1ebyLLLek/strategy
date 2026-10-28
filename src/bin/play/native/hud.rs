//! The heads-up pieces of the window's bars, in the era's dress: round medallions for the
//! spheres and the clock (a ring of trim lit from the upper left, a dark disc, the icon in its own
//! colour), gauges of pips instead of numbers (the numbers are on the cards), dark plates with a
//! double line of trim and small diamonds at the corners, dividers with a diamond, a band of
//! ornament along a bar's edge, a shadow a bar casts on the map.

use eframe::egui::{self, epaint, pos2, vec2, Color32, Pos2, Rect, Response, Sense, Shape, Stroke, Ui};

use super::chrome::{Chrome, Out};
use super::look::Skin;

/// A ring of trim lit from the upper left: a band of light above, of shade below.
pub fn ring(v: &mut Out, c: Pos2, r: f32, w: f32, trim: Color32) {
    let n = 48;
    let light = lighten(trim, 0.45);
    let dark = darken(trim, 0.45);
    let mut m = epaint::Mesh::default();
    for i in 0..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let (s, co) = a.sin_cos();
        // (the light from the upper left: -x, -y)
        let k = ((-co - s) * 0.5 * std::f32::consts::FRAC_1_SQRT_2 + 0.5).clamp(0.0, 1.0);
        let col = lerp(dark, light, k);
        m.colored_vertex(pos2(c.x + co * (r - w), c.y + s * (r - w)), col);
        m.colored_vertex(pos2(c.x + co * r, c.y + s * r), col);
    }
    for i in 0..n as u32 {
        let k = i * 2;
        m.add_triangle(k, k + 1, k + 3);
        m.add_triangle(k, k + 3, k + 2);
    }
    v.push(Shape::mesh(m));
}

/// A disc darker at its rim (as sunk in metal), in a colour.
pub fn disc(v: &mut Out, c: Pos2, r: f32, mid: Color32, rim: Color32) {
    let n = 40;
    let mut m = epaint::Mesh::default();
    m.colored_vertex(pos2(c.x - r * 0.25, c.y - r * 0.3), mid);
    for i in 0..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        m.colored_vertex(pos2(c.x + a.cos() * r, c.y + a.sin() * r), rim);
    }
    for i in 0..n as u32 {
        m.add_triangle(0, 1 + i, 2 + i);
    }
    v.push(Shape::mesh(m));
}

pub fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()), f(a.a(), b.a()))
}

pub fn lighten(c: Color32, t: f32) -> Color32 {
    lerp(c, Color32::from_rgb(255, 250, 235), t)
}

pub fn darken(c: Color32, t: f32) -> Color32 {
    lerp(c, Color32::from_rgb(10, 8, 6), t)
}

/// A round button: a ring of trim, a dark disc (the accent when chosen), the icon in its own
/// colour — brighter under the hand. Its response.
#[allow(clippy::too_many_arguments)]
pub fn medallion(ui: &mut Ui, ch: &Chrome, skin: &Skin, icon: &str, r: f32, hue: Color32, on: bool, id: impl std::hash::Hash) -> Response {
    let (rect, _) = ui.allocate_exact_size(vec2(2.0 * r + 4.0, 2.0 * r + 4.0), Sense::hover());
    let resp = ui.interact(rect, egui::Id::new(("medallion", id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let hot = resp.hovered();
    let down = resp.is_pointer_button_down_on();
    let c = rect.center() + if down { vec2(0.0, 1.0) } else { vec2(0.0, 0.0) };
    let mut v = Out::new();
    v.push(Shape::circle_filled(c + vec2(0.0, 2.0), r + 1.0, Color32::from_black_alpha(120)));
    if on {
        v.push(Shape::circle_filled(c, r + 3.5, hue.gamma_multiply(0.35)));
    }
    ring(&mut v, c, r, 3.2, if on || hot { lighten(ch.trim, 0.15) } else { ch.trim });
    let (mid, rim) = if on { (lighten(skin.accent, 0.12), darken(skin.accent, 0.35)) } else { (Color32::from_rgb(58, 54, 46), Color32::from_rgb(16, 14, 12)) };
    disc(&mut v, c, r - 3.0, mid, rim);
    v.push(Shape::circle_stroke(c, r - 3.4, Stroke::new(1.0, Color32::from_black_alpha(160))));
    ui.painter().extend(v);
    let tint = if on { skin.on_accent() } else if hot { lighten(hue, 0.25) } else { hue };
    let icons = super::chrome::icons(ui.ctx());
    let s = r * 1.05;
    // (a shade under the icon, then the icon)
    super::look::icon(&icons, icon, s, Color32::from_black_alpha(200)).paint_at(ui, Rect::from_center_size(c + vec2(1.0, 1.5), vec2(s, s)));
    super::look::icon(&icons, icon, s, tint).paint_at(ui, Rect::from_center_size(c, vec2(s, s)));
    resp
}

/// A gauge of pips: `of` small bars, the first `level` (0..of, fractions as a half-lit pip) lit
/// in a colour, the rest dark.
pub fn pips(v: &mut Out, at: Pos2, of: usize, level: f32, col: Color32, w: f32, h: f32) -> Rect {
    let gap = 2.0;
    for i in 0..of {
        let r = Rect::from_min_size(pos2(at.x + i as f32 * (w + gap), at.y), vec2(w, h));
        v.push(Shape::rect_filled(r.expand(0.8), 1.0, Color32::from_black_alpha(170)));
        let lit = (level - i as f32).clamp(0.0, 1.0);
        let c = if lit > 0.0 { lerp(Color32::from_rgb(40, 38, 34), col, 0.35 + 0.65 * lit) } else { Color32::from_rgb(40, 38, 34) };
        v.push(Shape::rect_filled(r, 1.0, c));
        if lit > 0.0 {
            v.push(Shape::line_segment([r.left_top() + vec2(0.5, 0.5), r.right_top() + vec2(-0.5, 0.5)], Stroke::new(1.0, lighten(c, 0.4))));
        }
    }
    Rect::from_min_size(at, vec2(of as f32 * (w + gap) - gap, h))
}

/// A dark plate sunk into a bar: near-black metal, a line of trim round it, small diamonds at its
/// corners.
pub fn sunk(v: &mut Out, r: Rect, ch: &Chrome, rounding: f32) {
    v.push(Shape::rect_filled(r, rounding, Color32::from_rgb(20, 18, 15)));
    gloss(v, r, Color32::from_white_alpha(10), Color32::from_black_alpha(80));
    ch.bevel(v, r, false, 1.5);
    v.push(Shape::rect_stroke(r.expand(1.0), rounding, Stroke::new(1.0, ch.trim.gamma_multiply(0.8))));
    for c in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()] {
        diamond(v, c, 3.2, ch.trim);
    }
}

/// A raised dark plate: dark metal with a sheen, a double line of trim.
pub fn plate(v: &mut Out, r: Rect, ch: &Chrome, rounding: f32) {
    v.push(epaint::Shadow { offset: vec2(0.0, 3.0), blur: 8.0, spread: 0.0, color: Color32::from_black_alpha(140) }.as_shape(r, rounding).into());
    ch.lay(v, r, &ch.bar, rounding);
    v.push(Shape::rect_filled(r, rounding, Color32::from_black_alpha(110)));
    gloss(v, r, Color32::from_white_alpha(26), Color32::from_black_alpha(70));
    ch.bevel(v, r, true, 1.5);
    v.push(Shape::rect_stroke(r, rounding, Stroke::new(1.5, ch.trim)));
    v.push(Shape::rect_stroke(r.shrink(3.0), rounding, Stroke::new(0.8, ch.trim.gamma_multiply(0.55))));
}

pub fn diamond(v: &mut Out, c: Pos2, s: f32, col: Color32) {
    v.push(Shape::convex_polygon(vec![c + vec2(-s, 0.0), c + vec2(0.0, -s), c + vec2(s, 0.0), c + vec2(0.0, s)], col, Stroke::new(0.8, Color32::from_black_alpha(160))));
}

/// A divider between groups on a bar: a thin upright line of trim with a diamond in its middle.
pub fn divider(v: &mut Out, x: f32, y0: f32, y1: f32, ch: &Chrome) {
    v.push(Shape::line_segment([pos2(x + 1.0, y0), pos2(x + 1.0, y1)], Stroke::new(1.0, Color32::from_black_alpha(150))));
    v.push(Shape::line_segment([pos2(x, y0), pos2(x, y1)], Stroke::new(1.0, ch.trim.gamma_multiply(0.7))));
    diamond(v, pos2(x, (y0 + y1) / 2.0), 3.0, ch.trim);
}

/// A sheen from the top down.
pub fn gloss(v: &mut Out, r: Rect, top: Color32, bottom: Color32) {
    let mut m = epaint::Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.right_bottom(), bottom);
    m.colored_vertex(r.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    v.push(Shape::mesh(m));
}

/// A band of ornament along a bar's edge (at `y`): a double line of trim, a row of small diamonds
/// between them.
pub fn band(v: &mut Out, x0: f32, x1: f32, y: f32, ch: &Chrome) {
    v.push(Shape::line_segment([pos2(x0, y - 2.5), pos2(x1, y - 2.5)], Stroke::new(1.2, ch.trim)));
    v.push(Shape::line_segment([pos2(x0, y + 2.5), pos2(x1, y + 2.5)], Stroke::new(1.2, darken(ch.trim, 0.25))));
    v.push(Shape::line_segment([pos2(x0, y), pos2(x1, y)], Stroke::new(3.0, Color32::from_black_alpha(140))));
    let mut x = x0 + 12.0;
    while x < x1 - 6.0 {
        diamond(v, pos2(x, y), 1.8, ch.trim.gamma_multiply(0.85));
        x += 24.0;
    }
}

/// The shade a bar casts on what lies under it (`r`: the strip to darken, darkest at `from_top`).
pub fn cast(v: &mut Out, r: Rect, from_top: bool) {
    let (a, b) = (Color32::from_black_alpha(110), Color32::TRANSPARENT);
    let (t, bt) = if from_top { (a, b) } else { (b, a) };
    gloss(v, r, t, bt);
}

/// A medallion that is only a mark (no button): the ring, the disc, the icon in its colour.
pub fn seal(ui: &Ui, ch: &Chrome, c: Pos2, r: f32, icon: &str, hue: Color32) {
    let mut v = Out::new();
    v.push(Shape::circle_filled(c + vec2(0.0, 2.0), r + 1.0, Color32::from_black_alpha(120)));
    ring(&mut v, c, r, 3.2, ch.trim);
    disc(&mut v, c, r - 3.0, Color32::from_rgb(58, 54, 46), Color32::from_rgb(16, 14, 12));
    ui.painter().extend(v);
    let icons = super::chrome::icons(ui.ctx());
    let s = r * 1.05;
    super::look::icon(&icons, icon, s, Color32::from_black_alpha(200)).paint_at(ui, Rect::from_center_size(c + vec2(1.0, 1.5), vec2(s, s)));
    super::look::icon(&icons, icon, s, hue).paint_at(ui, Rect::from_center_size(c, vec2(s, s)));
}

/// A small arrow, up or down, in a colour: which way a figure went.
pub fn arrow(v: &mut Out, c: Pos2, up: bool, col: Color32) {
    let (w, h) = (4.5, 4.5);
    let pts = if up {
        vec![c + vec2(0.0, -h), c + vec2(w, h * 0.8), c + vec2(-w, h * 0.8)]
    } else {
        vec![c + vec2(0.0, h), c + vec2(-w, -h * 0.8), c + vec2(w, -h * 0.8)]
    };
    v.push(Shape::convex_polygon(pts, col, Stroke::new(1.0, Color32::from_black_alpha(170))));
}

/// A mark on a medallion: a small disc in a colour with "!", beating a little — something there
/// wants the eye. Drawn over the bar (its own layer), so that the medallion's neighbours do not
/// cover it.
pub fn badge(ctx: &egui::Context, c: Pos2, col: Color32) {
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("badges")));
    let t = ctx.input(|i| i.time) as f32;
    let r = 7.5 + 0.8 * (0.5 + 0.5 * (t * 4.0).sin());
    p.circle_filled(c + vec2(0.0, 1.0), r + 1.0, Color32::from_black_alpha(140));
    p.circle_filled(c, r, col);
    p.circle_stroke(c, r, Stroke::new(1.2, lighten(col, 0.5)));
    p.text(c + vec2(0.0, 0.5), egui::Align2::CENTER_CENTER, "!", egui::FontId::new(12.0, egui::FontFamily::Name("bold".into())), Color32::WHITE);
}

