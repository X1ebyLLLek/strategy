//! The troops on the map, drawn as on a staff map (as in HOI4): a counter is a plate of the
//! side's colour with a sheen and a bevel, the sign of its main kind of troops in a light frame, the
//! number of divisions; under it two bars — supply and how dug in it is; a group's colour as a strip
//! on top and its order in a badge at the corner. Far away — one counter for each group of armies (troops behind the front counted with
//! the nearest group) and the enemy across its sector as one counter, dashed (seen through fog);
//! close up — a counter in each province with troops, those that would sit on each other merged.
//! The blows as curved arrows.

use eframe::egui::{self, pos2, vec2, Align2, Color32, FontFamily, FontId, Pos2, Rect, Stroke};
use strategy::military::front::Order;
use strategy::rng::{hash64, Rng};

use crate::Game;
use super::{MapView, HUES};

/// Closer than this (screen points per world pixel): a counter in each province.
pub const AGG: f32 = 2.6;

/// A counter to draw.
pub struct Counter<'a> {
    pub n: f32,
    pub fill: Color32,
    pub sym: Option<&'a str>,
    pub big: bool,
    pub fog: bool,
    pub hue: Option<Color32>,
    pub edge: Option<Color32>,
    pub order: Option<Order>,
    pub chosen: bool,
    pub supply: Option<f32>,
    /// How dug in (0..1), for one's own troops.
    pub dug: Option<f32>,
    /// The country whose flag the counter carries at its left (as HOI4's do).
    pub country: Option<usize>,
    /// Divisions: their strength and organisation (0..1), as HOI4's two bars, green and yellow.
    pub bars: Option<(f32, f32)>,
    /// Marks at its corner: `MARCH` — on the march, `ATTACK` — attacking, `HUNGRY` — short of
    /// supply, `OWN` — sent by the player himself (its general leaves it be), `SHORT` — short of
    /// the equipment its norm gives it.
    pub marks: u8,
}

pub const MARCH: u8 = 1;
pub const ATTACK: u8 = 2;
pub const HUNGRY: u8 = 4;
pub const OWN: u8 = 8;
pub const SHORT: u8 = 16;

/// A colour darkened (`k` of it kept).
pub fn shade(c: Color32, k: f32) -> Color32 {
    Color32::from_rgb((c.r() as f32 * k) as u8, (c.g() as f32 * k) as u8, (c.b() as f32 * k) as u8)
}

/// The space a counter takes (for moving them apart): large (a group's, far away) or small.
pub fn room(big: bool) -> egui::Vec2 {
    if big {
        vec2(110.0, 40.0)
    } else {
        vec2(92.0, 34.0)
    }
}

/// A counter centred at `p`; the space it takes (for the hand).
pub fn counter(painter: &egui::Painter, p: Pos2, o: &Counter) -> Rect {
    // Вычисляем динамический коэффициент сжатия фишек на основе плотности стека
    let stack_compression = if o.bars.is_none() && o.n > 2.0 { 0.82f32 } else { 1.0f32 };
    
    let base_h = if o.big { 24.0 } else { 20.0 };
    let h = base_h * stack_compression; 
    
    // Уменьшаем бокс NATO-значка (sw), давая больше места под цифру
    let sw = h * 0.75; 
    // Сильно увеличиваем шрифт для цифр дивизий, как в HOI4!
    let font_size = if o.big { 16.5 } else { 14.5 };
    let font = FontId::new(font_size, FontFamily::Name("bold".into()));

    let label = format!("{}{}", if o.fog { "≈" } else { "" }, o.n.round().max(1.0) as i64);
    let galley = painter.layout_no_wrap(label, font, Color32::from_rgb(251, 246, 234));
    let tw = (galley.size().x + 10.0).max(h * 0.95) + if o.marks & MARCH != 0 { 9.0 } else { 0.0 };
    // (the flag's part at the left)
    let flag = o.country.and_then(|c| super::super::flags::get(painter.ctx(), c));
    let fw = if flag.is_some() { h * 0.95 } else { 0.0 };
    let w = fw + sw + tw;
    let r = Rect::from_min_size(p - vec2(w / 2.0, h / 2.0), vec2(w, h));
    let alpha = if o.fog { 0.78 } else { 1.0 };
    let a = |c: Color32| c.gamma_multiply(alpha);
    let bars = o.supply.is_some() as usize + o.dug.is_some() as usize + 2 * o.bars.is_some() as usize;
    let whole = Rect::from_min_max(r.min, pos2(r.max.x, r.max.y + 1.5 + bars as f32 * 4.0));
    painter.add(egui::epaint::Shadow { offset: vec2(0.0, 2.0), blur: 7.0, spread: 0.0, color: Color32::from_black_alpha(150) }.as_shape(whole, 3.0));
    // (the plate: the side's colour, lighter above, darker below; a sheen on the upper half)

    {
        let (top, bottom) = (shade(o.fill, if o.fog { 0.50 } else { 0.66 }), shade(o.fill, if o.fog { 0.34 } else { 0.40 }));
        let mut m = egui::epaint::Mesh::default();
        m.colored_vertex(r.left_top(), a(top));
        m.colored_vertex(r.right_top(), a(top));
        m.colored_vertex(r.right_bottom(), a(bottom));
        m.colored_vertex(r.left_bottom(), a(bottom));
        m.add_triangle(0, 1, 2);
        m.add_triangle(0, 2, 3);
        painter.add(m);
        let top = Rect::from_min_size(r.min, vec2(r.width(), r.height() * 0.45));
        let mut m = egui::epaint::Mesh::default();
        let (hi, lo) = (Color32::from_white_alpha(if o.fog { 12 } else { 44 }), Color32::from_white_alpha(0));
        m.colored_vertex(top.left_top(), hi);
        m.colored_vertex(top.right_top(), hi);
        m.colored_vertex(top.right_bottom(), lo);
        m.colored_vertex(top.left_bottom(), lo);
        m.add_triangle(0, 1, 2);
        m.add_triangle(0, 2, 3);
        painter.add(m);
        // (a bevel: light along the top, dark along the bottom)
        painter.line_segment([r.left_top() + vec2(2.0, 1.0), r.right_top() + vec2(-2.0, 1.0)], Stroke::new(1.0, Color32::from_white_alpha(if o.fog { 20 } else { 70 })));
        painter.line_segment([r.left_bottom() + vec2(2.0, -1.0), r.right_bottom() + vec2(-2.0, -1.0)], Stroke::new(1.0, Color32::from_black_alpha(90)));
    }
    // the flag, at the left
    if let Some(f) = &flag {
        let fr = Rect::from_min_size(r.min + vec2(3.0, 4.0), vec2(fw - 4.0, h - 8.0));
        super::super::flags::paint(painter.ctx(), painter, fr, f, Color32::from_black_alpha(200), None);
    }
    // the sign's box, as on a staff map: light, with a dark frame and a shade inside
    let sb = Rect::from_min_size(r.min + vec2(fw + 3.0, 3.0), vec2(sw - 5.0, h - 6.0));
    painter.rect_filled(sb.expand(1.0), 1.0, Color32::from_black_alpha(if o.fog { 90 } else { 170 }));
    painter.rect_filled(sb, 0.5, if o.fog { Color32::from_rgba_unmultiplied(230, 224, 208, 140) } else { Color32::from_rgb(238, 232, 214) });
    painter.line_segment([sb.left_top() + vec2(0.0, 0.5), sb.right_top() + vec2(0.0, 0.5)], Stroke::new(1.0, Color32::from_black_alpha(40)));
    let ink = if o.fog { Color32::from_rgba_unmultiplied(40, 30, 20, 150) } else { Color32::from_rgb(29, 26, 21) };
    match o.sym {
        Some(sym) => sign(painter, sym, sb.center(), sb.width() * 0.8, sb.height() * 0.74, ink),
        None => {
            painter.text(sb.center(), Align2::CENTER_CENTER, "?", FontId::new(12.0, FontFamily::Name("bold".into())), ink);
        }
    }
    // the number, with a dark shade under it
    let num_rect = Rect::from_min_max(pos2(r.min.x + fw + sw, r.min.y), r.max);
    painter.rect_filled(num_rect, 0.0, Color32::from_black_alpha(120));

    // Выводим крупную цифру ровно по центру выделенного ей темного бокса
    let at = pos2(num_rect.center().x - galley.size().x / 2.0, r.center().y - galley.size().y / 2.0);
    painter.add(egui::epaint::TextShape::new(at + vec2(1.0, 1.0), galley.clone(), Color32::BLACK).with_override_text_color(Color32::from_black_alpha(200)));
    painter.galley(at, galley, Color32::WHITE);
    // the edge: gold — chosen; a group's colour; dashed — seen through fog
    let edge = Stroke::new(if o.chosen { 2.4 } else { 1.3 }, if o.chosen { Color32::from_rgb(255, 216, 96) } else { o.edge.unwrap_or(Color32::from_rgba_unmultiplied(8, 8, 8, 230)) });
    if o.chosen {
        let t = painter.ctx().input(|i| i.time) as f32;
        let breath = 0.5 + 0.5 * (t * 4.0).sin();
        painter.rect_stroke(whole.expand(2.0 + 2.0 * breath), 4.0, Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 216, 96, (60.0 + 120.0 * breath) as u8)));
    }
    if o.fog {
        let pts = vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
        painter.extend(egui::Shape::dashed_line(&pts, edge, 3.0, 2.0));
    } else {
        painter.rect_stroke(r, 3.0, edge);
    }
    // the bars under it: supply (green, amber, red as it runs short), how dug in (tan)
    let mut y = r.max.y + 1.5;
    let bar = |y: f32, v: f32, c: Color32| {
        painter.rect_filled(Rect::from_min_size(pos2(r.min.x, y), vec2(w, 4.0)), 1.0, Color32::from_rgba_unmultiplied(10, 10, 10, 220));
        painter.rect_filled(Rect::from_min_size(pos2(r.min.x + 0.8, y + 0.8), vec2((w - 1.6) * v.clamp(0.0, 1.0), 2.4)), 0.5, c);
    };
    if let Some(s) = o.supply {
        let c = if s > 0.85 { Color32::from_rgb(143, 209, 106) } else if s > 0.55 { Color32::from_rgb(230, 195, 74) } else { Color32::from_rgb(224, 103, 74) };
        bar(y, s, c);
        y += 4.0;
    }
    if let Some((st, org)) = o.bars {
        // (as HOI4: strength green, organisation yellow)
        bar(y, st, Color32::from_rgb(112, 196, 84));
        y += 4.0;
        bar(y, org, Color32::from_rgb(236, 200, 64));
        y += 4.0;
    }
    if let Some(d) = o.dug {
        bar(y, d, Color32::from_rgb(196, 160, 104));
    }
    // a group's colour as a strip on top, its order in a badge at the corner
    if let Some(hue) = o.hue {
        painter.rect_filled(Rect::from_min_size(pos2(r.min.x + 2.0, r.min.y - 4.0), vec2(w - 4.0, 3.5)), 1.0, hue);
    }
    if let Some(order) = o.order {
        badge(painter, pos2(r.max.x + 1.0, r.min.y + 1.0), order, o.hue.unwrap_or(Color32::from_gray(220)));
    }
    // the marks of divisions: attacking — a red frame; on the march — chevrons at the right end;
    // short of supply — a red drop at the top left; sent by the player — a gold pin at the top right
    if o.marks & ATTACK != 0 && !o.chosen {
        painter.rect_stroke(r.expand(1.0), 3.0, Stroke::new(1.8, Color32::from_rgb(232, 72, 56)));
    }
    if o.marks & MARCH != 0 {
        let c = pos2(r.max.x - 5.0, r.center().y);
        for dx in [-3.0, 1.0] {
            painter.add(egui::Shape::line(vec![c + vec2(dx - 2.0, -3.5), c + vec2(dx + 1.5, 0.0), c + vec2(dx - 2.0, 3.5)], Stroke::new(1.4, Color32::from_rgb(250, 244, 226))));
        }
    }
    if o.marks & HUNGRY != 0 {
        let c = r.left_top() + vec2(1.0, 1.0);
        painter.circle_filled(c, 5.0, Color32::from_rgb(200, 52, 40));
        painter.circle_stroke(c, 5.0, Stroke::new(1.0, Color32::from_black_alpha(200)));
        painter.text(c, Align2::CENTER_CENTER, "!", FontId::new(9.0, FontFamily::Name("bold".into())), Color32::WHITE);
    }
    if o.marks & OWN != 0 {
        let c = r.right_top() + vec2(-2.0, 1.0);
        painter.circle_filled(c, 3.6, Color32::from_rgb(255, 214, 96));
        painter.circle_stroke(c, 3.6, Stroke::new(1.0, Color32::from_black_alpha(200)));
    }
    // (short of equipment: an amber triangle at the bottom left, as HOI4 warns of it)
    if o.marks & SHORT != 0 {
        let c = r.left_bottom() + vec2(2.0, -3.0);
        let tri = vec![c + vec2(0.0, -6.5), c + vec2(6.0, 4.0), c + vec2(-6.0, 4.0)];
        painter.add(egui::Shape::convex_polygon(tri, Color32::from_rgb(236, 160, 40), Stroke::new(1.0, Color32::from_black_alpha(220))));
        painter.text(c + vec2(0.0, 0.8), Align2::CENTER_CENTER, "!", FontId::new(8.5, FontFamily::Name("bold".into())), Color32::from_rgb(30, 20, 10));
    }
    Rect::from_min_size(r.min - vec2(0.0, 5.0), vec2(w + 10.0, whole.height() + 9.0))
}

/// A group's order: hold — a bar, deep — two bars, attack — a chevron.
pub fn badge(painter: &egui::Painter, p: Pos2, order: Order, color: Color32) {
    painter.circle(p, 7.5, Color32::from_rgb(26, 23, 18), Stroke::new(1.5, color));
    let ink = Color32::from_rgb(243, 236, 216);
    match order {
        Order::Hold => {
            painter.rect_filled(Rect::from_center_size(p, vec2(8.0, 2.0)), 0.0, ink);
        }
        Order::Deep => {
            painter.rect_filled(Rect::from_center_size(p - vec2(0.0, 2.1), vec2(8.0, 1.8)), 0.0, ink);
            painter.rect_filled(Rect::from_center_size(p + vec2(0.0, 2.1), vec2(8.0, 1.8)), 0.0, ink);
        }
        Order::Attack(_) => {
            painter.add(egui::Shape::line(vec![p + vec2(-2.5, -3.5), p + vec2(2.0, 0.0), p + vec2(-2.5, 3.5)], Stroke::new(1.8, ink)));
        }
    }
}

/// The sign of a kind of troops inside its frame (as the military maps draw them).
pub fn sign(painter: &egui::Painter, sym: &str, c: Pos2, w: f32, h: f32, color: Color32) {
    let st = Stroke::new((w / 12.0).max(1.2), color);
    let r = Rect::from_center_size(c, vec2(w, h));
    painter.rect_stroke(r, 0.0, st);
    let (l, rt, t, b) = (r.min.x, r.max.x, r.min.y, r.max.y);
    let line = |pts: &[(f32, f32)]| {
        painter.add(egui::Shape::line(pts.iter().map(|p| pos2(p.0, p.1)).collect(), st));
    };
    let wheels = || {
        for s in [-1.0, 1.0] {
            painter.circle_filled(pos2(c.x + s * w * 0.22, b - h * 0.16), h * 0.09, color);
        }
    };
    let ellipse = |cx: f32, cy: f32, rx: f32, ry: f32| {
        let pts: Vec<Pos2> = (0..=24).map(|i| {
            let a = i as f32 / 24.0 * std::f32::consts::TAU;
            pos2(cx + rx * a.cos(), cy + ry * a.sin())
        }).collect();
        painter.add(egui::Shape::line(pts, st));
    };
    match sym {
        "armour" => ellipse(c.x, c.y, w * 0.32, h * 0.26),
        "wheeled-armour" => {
            ellipse(c.x, c.y - h * 0.06, w * 0.3, h * 0.22);
            wheels();
        }
        "antitank" | "rockets" => line(&[(l, b), (c.x, t), (rt, b)]),
        "artillery" => {
            painter.circle_filled(c, h * 0.16, color);
        }
        "mortar" => {
            painter.circle_filled(pos2(c.x, c.y + h * 0.18), h * 0.12, color);
            line(&[(c.x, c.y + h * 0.1), (c.x, t + h * 0.18)]);
            line(&[(c.x - w * 0.08, t + h * 0.32), (c.x, t + h * 0.16), (c.x + w * 0.08, t + h * 0.32)]);
        }
        "air" => {
            for s in [-1.0, 1.0] {
                ellipse(c.x + s * w * 0.13, c.y, w * 0.13, h * 0.16);
            }
        }
        "antiair" => {
            let pts: Vec<Pos2> = (0..=16).map(|i| {
                let a = std::f32::consts::PI * (1.0 + i as f32 / 16.0);
                pos2(c.x + h * 0.42 * a.cos(), b + h * 0.42 * a.sin())
            }).collect();
            painter.add(egui::Shape::line(pts, st));
        }
        "cavalry" => line(&[(l, b), (rt, t)]),
        "recon" => {
            line(&[(l, b), (rt, t)]);
            wheels();
        }
        "light" => {
            line(&[(l, t), (rt, b)]);
            line(&[(l, b), (rt, t)]);
            line(&[(c.x - w * 0.12, t + h * 0.02), (c.x, t - h * 0.2), (c.x + w * 0.12, t + h * 0.02)]);
        }
        _ => {
            line(&[(l, t), (rt, b)]);
            line(&[(l, b), (rt, t)]);
        }
    }
}

/// An arrow along a gentle curve (bent to the left of its way), chevrons along it.
pub fn arrow(painter: &egui::Painter, a: Pos2, z: Pos2, width: f32, color: Color32, bend: f32, phase: f32) {
    let len = a.distance(z);
    if len < 4.0 {
        return;
    }
    let n = vec2(-(z.y - a.y), z.x - a.x) / len;
    let ctl = pos2((a.x + z.x) / 2.0, (a.y + z.y) / 2.0) + n * len * bend;
    let head = (len * 0.4).min(8.0 + width * 1.6);
    let t = 1.0 - head / len;
    let at = |s: f32| pos2((1.0 - s) * (1.0 - s) * a.x + 2.0 * (1.0 - s) * s * ctl.x + s * s * z.x, (1.0 - s) * (1.0 - s) * a.y + 2.0 * (1.0 - s) * s * ctl.y + s * s * z.y);
    let b = at(t);
    let curve: Vec<Pos2> = (0..=20).map(|i| at(t * i as f32 / 20.0)).collect();
    painter.add(egui::Shape::line(curve.clone(), Stroke::new(width + 2.4, Color32::from_black_alpha(190))));
    painter.add(egui::Shape::line(curve, Stroke::new(width, color)));
    let d = (z - b).normalized();
    let side = vec2(-d.y, d.x) * (width / 2.0 + head / 2.2);
    painter.add(egui::Shape::convex_polygon(vec![z, b + side, b - side], color, Stroke::new(1.2, Color32::from_black_alpha(190))));
    // chevrons along the way (no more than fit a screen)
    let k = ((len / (width * 2.6)) as usize).clamp(1, 64);
    let ink = Stroke::new((width * 0.2).max(1.0), Color32::from_black_alpha(90));
    for i in 0..k {
        let s = ((i as f32 + 0.5 + phase) / k as f32).fract() * t;
        let p = at(s);
        let v = (at((s + 0.01).min(1.0)) - p).normalized();
        let c = width * 0.3;
        painter.add(egui::Shape::line(vec![p - v * c + vec2(-v.y, v.x) * c, p, p - v * c - vec2(-v.y, v.x) * c], ink));
    }
}

/// Counters moved apart till none sits on another (a few rounds, each pair pushed along the line
/// between them).
pub fn apart<'a, T>(mut todo: Vec<(Pos2, Counter<'a>, T)>) -> Vec<(Pos2, Counter<'a>, T)> {
    let mut ps: Vec<Pos2> = todo.iter().map(|t| t.0).collect();
    let sizes: Vec<egui::Vec2> = todo.iter().map(|t| room(t.1.big)).collect();
    spread(&mut ps, &sizes);
    for (t, p) in todo.iter_mut().zip(ps) {
        t.0 = p;
    }
    todo
}

/// Points of boxes of these sizes moved apart till no box sits on another (a few rounds, each
/// pair pushed out along the shorter way).
pub fn spread(ps: &mut [Pos2], sizes: &[egui::Vec2]) {
    for _ in 0..24 {
        let mut moved = false;
        for i in 0..ps.len() {
            for j in i + 1..ps.len() {
                let s = (sizes[i] + sizes[j]) / 2.0;
                let d = ps[j] - ps[i];
                let (ox, oy) = (s.x - d.x.abs(), s.y - d.y.abs());
                if ox <= 0.0 || oy <= 0.0 {
                    continue;
                }
                moved = true;
                // (the boxes are wide: up or down is the cheaper way out unless they sit nearly level)
                let push = if oy * 2.5 < ox { vec2(0.0, (oy / 2.0 + 0.5) * if d.y >= 0.0 { 1.0 } else { -1.0 }) } else { vec2((ox / 2.0 + 0.5) * if d.x >= 0.0 { 1.0 } else { -1.0 }, 0.0) };
                ps[i] -= push;
                ps[j] += push;
            }
        }
        if !moved {
            break;
        }
    }
}

impl MapView {
    /// The troops of the war: the counters (far: by groups; close up: by provinces), the arrows of
    /// the groups' orders and of the staff's blows. The counters that are groups go to `hits`.
    pub(super) fn troops(&self, painter: &egui::Painter, rect: Rect, g: &Game, chosen: Option<usize>, hits: &mut Vec<(Rect, usize)>) {
        let w = g.world.w as f32;
        let war = &g.h.war;
        let th = &war.th;
        let me = g.me;
        let pack = &g.pack;
        let size = if war.sides[me].fixed { crate::army::templates::division_size(&g.templates) } else { 1.0 }.max(1e-6);
        let sup = war.supply(me);
        let fog = war.fog[me];
        let colour = |s: usize| {
            let c = g.countries[th.countries[s]].1;
            Color32::from_rgb(c[0], c[1], c[2])
        };
        let at = |q: usize| self.pt(Self::at(g, th.provs[q]));
        let on_screen = |p: Pos2| rect.expand(60.0).contains(p);
        // the enemy as the staff sees it (through fog), province by province
        let mut foe_count = vec![0.0f32; th.provs.len()];
        if let Some(c) = war.corps.as_ref() {
            for d in c.divs.iter().filter(|d| d.side != me) {
                foe_count[d.at] += 1.0;
            }
        }
        let seen = |q: usize| -> f32 {
            let n: f32 = if war.corps.is_some() { foe_count[q] } else { war.troops[1 - me][q].iter().sum() };
            if n <= 0.0 {
                0.0
            } else {
                n * (fog * Rng::new(hash64(war.seed ^ 0xF06 ^ (q as u64) << 16 ^ war.month as u64)).normal()).exp()
            }
        };
        // the main kind among provinces, by worth
        let main = |qs: &[usize]| -> Option<usize> {
            let mut worth = vec![0.0f32; pack.units.len()];
            for &q in qs {
                for (u, x) in war.troops[me][q].iter().enumerate() {
                    worth[u] += x * pack.units[u].cost;
                }
            }
            (0..worth.len()).filter(|&u| worth[u] > 0.0).max_by(|&a, &b| worth[a].total_cmp(&worth[b]))
        };
        // (in a war of divisions: how many divisions of ours stand there, counted as they are)
        let mut count = vec![0.0f32; th.provs.len()];
        if let Some(c) = war.corps.as_ref() {
            for d in c.divs.iter().filter(|d| d.side == me) {
                count[d.at] += 1.0;
            }
        }
        let corps = war.corps.is_some();
        let size = if corps { 1.0 } else { size };
        let units = |q: usize| -> f32 { if corps { count[q] } else { war.troops[me][q].iter().sum() } };
        let groups = war.command[me].as_ref();
        let lab = war.group_of(me);
        if self.k < AGG {
            // far away: by the groups (at peace, by the stretches of the border)
            let parts = if groups.is_some() { war.sectors_of(me) } else { war.sectors(me, crate::SECTOR) };
            let mid = |part: &[usize]| -> Option<usize> {
                let pts: Vec<(f32, f32)> = part.iter().map(|&q| Self::at(g, th.provs[q])).collect();
                let x0 = pts.first()?.0;
                let unwrap = |x: f32| if x - x0 > w / 2.0 { x - w } else if x0 - x > w / 2.0 { x + w } else { x };
                let mx = pts.iter().map(|p| unwrap(p.0)).sum::<f32>() / pts.len() as f32;
                let my = pts.iter().map(|p| p.1).sum::<f32>() / pts.len() as f32;
                (0..part.len()).min_by(|&a, &b| ((unwrap(pts[a].0) - mx).powi(2) + (pts[a].1 - my).powi(2)).total_cmp(&((unwrap(pts[b].0) - mx).powi(2) + (pts[b].1 - my).powi(2)))).map(|i| part[i])
            };
            let anchors: Vec<Option<usize>> = parts.iter().map(|p| mid(p)).collect();
            // each province with troops counted with its sector, or with the nearest one
            let mut own: Vec<Vec<usize>> = vec![Vec::new(); parts.len()];
            let sector_of = {
                let mut v = vec![usize::MAX; th.provs.len()];
                for (k, part) in parts.iter().enumerate() {
                    for &q in part {
                        v[q] = k;
                    }
                }
                v
            };
            for q in 0..th.provs.len() {
                if war.owner[q] != me || units(q) <= 0.005 {
                    continue;
                }
                let mut s = sector_of[q];
                if s == usize::MAX {
                    let (x, y) = Self::at(g, th.provs[q]);
                    let mut best = f32::MAX;
                    for (k, a) in anchors.iter().enumerate() {
                        if let Some(a) = a {
                            let (ax, ay) = Self::at(g, th.provs[*a]);
                            let dx = (ax - x).abs().min(w - (ax - x).abs());
                            let d = dx * dx + (ay - y).powi(2);
                            if d < best {
                                best = d;
                                s = k;
                            }
                        }
                    }
                }
                if s != usize::MAX {
                    own[s].push(q);
                }
            }
            // (drawn after they are moved apart: the sectors are short, the counters wide)
            let mut todo: Vec<(Pos2, Counter, Option<usize>)> = Vec::new();
            for (k, part) in parts.iter().enumerate() {
                let Some(anchor) = anchors[k] else { continue };
                let p = self.pt(self.glide(k as u64, Self::at(g, th.provs[anchor])));
                // the enemy across the sector
                let mut across: Vec<usize> = part.iter().flat_map(|&q| th.nbr[q].iter().copied()).filter(|&q| war.owner[q] != me).collect();
                across.sort_unstable();
                across.dedup();
                let foe: f32 = across.iter().map(|&q| seen(q)).sum();
                if foe > 0.5 && !across.is_empty() {
                    // (the enemy's middle across the sector, on the map; gliding as the group's)
                    let (ax, _) = Self::at(g, th.provs[anchor]);
                    let unwrap = |x: f32| if x - ax > w / 2.0 { x - w } else if ax - x > w / 2.0 { x + w } else { x };
                    let mid = across.iter().fold((0.0f32, 0.0f32), |s, &q| {
                        let (x, y) = Self::at(g, th.provs[q]);
                        (s.0 + unwrap(x), s.1 + y)
                    });
                    let n = across.len() as f32;
                    let e = self.pt(self.glide(1_000_000 + k as u64, (mid.0 / n, mid.1 / n)));
                    let d = (e - p).normalized();
                    let e = e + d * 30.0;
                    if on_screen(e) {
                        todo.push((e, Counter { bars: None, marks: 0, n: foe, fill: colour(1 - me), sym: None, big: false, fog: true, hue: None, edge: None, order: None, chosen: false, supply: None, dug: None, country: Some(war.th.countries[1 - me]) }, None));
                    }
                }
                let total: f32 = own[k].iter().map(|&q| units(q)).sum();
                if total / size < 0.5 || !on_screen(p) {
                    continue;
                }
                let fed = own[k].iter().map(|&q| sup[q] * units(q)).sum::<f32>() / total.max(1e-9);
                let dug = own[k].iter().map(|&q| war.dug[me][q] * units(q)).sum::<f32>() / total.max(1e-9);
                let gr = groups.and_then(|gs| gs.get(k));
                let hue = gr.map(|_| HUES[k % HUES.len()]);
                let kind = main(&own[k]);
                todo.push((
                    p,
                    Counter {
                        bars: None,
                        marks: 0,
                        n: total / size,
                        fill: colour(me),
                        sym: Some(kind.map_or("infantry", |u| pack.units[u].symbol.as_str())),
                        big: true,
                        fog: false,
                        hue,
                        edge: hue,
                        order: gr.map(|x| x.order),
                        chosen: chosen == Some(k) && gr.is_some(),
                        supply: g.h.at_war.then_some(fed),
                        dug: g.h.at_war.then_some(dug),
                        country: Some(war.th.countries[me]),
                    },
                    gr.map(|_| k),
                ));
            }
            let todo = if crate::native::spoiled() { todo } else { apart(todo) };
            for (p, c, k) in todo {
                let r = counter(painter, p, &c);
                self.counters.lock().unwrap().push(r);
                if let Some(k) = k {
                    hits.push((r, k));
                }
            }
        } else {
            // close up: a counter in each province with troops; the enemy's at the front (deep
            // behind it, a faint dot)
            struct Item {
                p: Pos2,
                n: f32,
                foe: bool,
                qs: Vec<usize>,
                g: usize,
            }
            let mut items: Vec<Item> = Vec::new();
            for q in 0..th.provs.len() {
                let p = at(q);
                if !on_screen(p) {
                    continue;
                }
                let owner = war.owner[q];
                if owner != me {
                    let f = seen(q);
                    if f > 0.05 {
                        if th.nbr[q].iter().any(|&x| war.owner[x] != owner) {
                            items.push(Item { p, n: f, foe: true, qs: vec![q], g: usize::MAX });
                        } else {
                            painter.circle_filled(p, 3.0, shade(colour(1 - me), 0.5).gamma_multiply(0.6));
                        }
                    }
                }
                let m = units(q);
                if m > 0.005 && owner == me {
                    items.push(Item { p, n: m, foe: false, qs: vec![q], g: lab.get(q).copied().unwrap_or(usize::MAX) });
                }
            }
            items.sort_by(|a, b| b.n.total_cmp(&a.n));
            let mut placed: Vec<Item> = Vec::new();
            for it in items {
                if let Some(o) = placed.iter_mut().find(|o| !crate::native::spoiled() && o.foe == it.foe && (o.p.x - it.p.x).abs() < 54.0 && (o.p.y - it.p.y).abs() < 28.0) {
                    o.n += it.n;
                    o.qs.extend(it.qs);
                    continue;
                }
                placed.push(it);
            }
            if !crate::native::spoiled() {
                let mut ps: Vec<Pos2> = placed.iter().map(|i| i.p).collect();
                let sizes = vec![room(false); ps.len()];
                spread(&mut ps, &sizes);
                for (i, p) in placed.iter_mut().zip(ps) {
                    i.p = p;
                }
            }
            for it in &placed {
                if it.foe {
                    let r = counter(painter, it.p, &Counter { bars: None, marks: 0, n: it.n, fill: colour(1 - me), sym: None, big: false, fog: true, hue: None, edge: None, order: None, chosen: false, supply: None, dug: None, country: Some(war.th.countries[1 - me]) });
                    self.counters.lock().unwrap().push(r);
                    continue;
                }
                let fed = it.qs.iter().map(|&q| sup[q] * units(q)).sum::<f32>() / it.n.max(1e-9);
                let dug = it.qs.iter().map(|&q| war.dug[me][q] * units(q)).sum::<f32>() / it.n.max(1e-9);
                let grp = (it.g != usize::MAX && groups.is_some()).then_some(it.g);
                let kind = main(&it.qs);
                let r = counter(painter, it.p, &Counter {
                    bars: None,
                    marks: 0,
                    n: it.n / size,
                    fill: colour(me),
                    sym: Some(kind.map_or("infantry", |u| pack.units[u].symbol.as_str())),
                    big: false,
                    fog: false,
                    hue: None,
                    edge: grp.map(|k| HUES[k % HUES.len()]),
                    order: None,
                    chosen: grp.is_some() && grp == chosen,
                    supply: g.h.at_war.then_some(fed),
                    dug: g.h.at_war.then_some(dug),
                    country: Some(war.th.countries[me]),
                });
                self.counters.lock().unwrap().push(r);
                if let Some(k) = grp {
                    hits.push((r, k));
                }
            }
        }
    }

    /// The arrows of the groups' orders, then the staff's blows for next month (under the
    /// counters, so that the numbers stay readable).
    pub(super) fn arrows(&self, painter: &egui::Painter, rect: Rect, g: &Game, chosen: Option<usize>) {
        let war = &g.h.war;
        let th = &war.th;
        let me = g.me;
        let at = |q: usize| self.pt(Self::at(g, th.provs[q]));
        let groups = war.command[me].as_ref();
        // (an arrow whose end is far off the screen — behind the camera, close up — is not drawn:
        // its length on the screen is all but endless)
        let room = rect.expand(rect.width().max(rect.height()) * 1.5);
        if let Some(gs) = groups {
            let parts = war.sectors_of(me);
            for (k, gr) in gs.iter().enumerate() {
                let Order::Attack(target) = gr.order else { continue };
                let Some(part) = parts.get(k).filter(|p| !p.is_empty()) else { continue };
                let from = part[part.len() / 2];
                let a = at(from);
                let z = at(target);
                if !room.contains(a) || !room.contains(z) {
                    continue;
                }
                let c = HUES[k % HUES.len()].gamma_multiply(if chosen.is_none() || chosen == Some(k) { 0.95 } else { 0.6 });
                arrow(painter, a, z, if chosen == Some(k) { 11.0 } else { 8.0 }, c, 0.18, (self.t() * 0.9).fract());
            }
        }
        for b in war.staged.iter().filter(|b| b.0 == me) {
            let a = at(b.1);
            let z = at(b.2);
            if (!rect.expand(200.0).contains(a) && !rect.expand(200.0).contains(z)) || !room.contains(a) || !room.contains(z) {
                continue;
            }
            let n: f32 = b.3.iter().sum();
            arrow(painter, a, z, (2.5 + 1.4 * n.sqrt()).min(9.0), Color32::from_rgba_unmultiplied(255, 214, 64, 230), 0.12, (self.t() * 0.9).fract());
        }
    }
}
