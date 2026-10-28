//! Charts and bars with their numbers (his word in the 23rd session: people must see what affects
//! what and by how much): a chart of lines over the months with a scale at its left, the years
//! along its foot, each line's last value at its end and a legend with the values now; bars with
//! their figures; a gauge of mood from hostile to content about its middle; a bar of shares.

use eframe::egui::{self, epaint, pos2, vec2, Color32, FontId, Rect, Response, Sense, Shape, Stroke, Ui};

use super::chrome::{self, Card, Out, Tip};
use super::hud;

/// A number short enough for a scale: 1234 → «1234», 12.3 → «12,3», 0.123 → «0,12».
pub fn num(x: f32) -> String {
    let a = x.abs();
    let s = if a >= 10000.0 {
        format!("{:.0} тыс.", x / 1000.0)
    } else if a >= 100.0 {
        format!("{x:.0}")
    } else if a >= 10.0 {
        format!("{x:.1}")
    } else {
        format!("{x:.2}")
    };
    s.replace('.', ",").replace('-', "−")
}

/// A number with `d` decimals, a comma for the point and a true minus: 1.5 → «1,5», −0.25 → «−0,25».
pub fn fix(x: f32, d: usize) -> String {
    format!("{x:.d$}").replace('.', ",").replace('-', "−")
}

/// `fix` with its sign always shown (+ or −); a value that rounds to nothing is written without one.
pub fn sfix(x: f32, d: usize) -> String {
    let t = fix(x, d);
    let zero = !t.chars().any(|c| c.is_ascii_digit() && c != '0');
    if zero {
        t.trim_start_matches('−').to_string()
    } else if x > 0.0 {
        format!("+{t}")
    } else {
        t
    }
}

/// A share as percent: 0.234 → «23%».
pub fn pct(x: f32) -> String {
    format!("{:.0}%", x * 100.0)
}

/// A change as signed percent: 1.05 → «+5%».
pub fn delta(x: f32) -> String {
    format!("{:+.0}%", (x - 1.0) * 100.0)
}

/// Lines over the months with numbers: the scale at the left (the top, the middle, the bottom —
/// zero when the values cross it), a tick and the year along the foot, each line's last value at
/// its end, a legend under it with the values now; under the hand a rule at the month and the
/// values of that month on a card. `fmt` writes a value.
pub fn plot(ui: &mut Ui, title: &str, series: &[(String, Color32, Vec<f32>)], months: &[u32], h: f32, fmt: &dyn Fn(f32) -> String) -> Response {
    plot_marked(ui, title, series, months, h, fmt, &[])
}

/// A decision of the player's, marked on the foot of a chart: the month, what he decided.
pub struct Marker {
    pub month: u32,
    pub text: String,
}

/// `plot` with the player's decisions marked along its foot (a small wedge at the month, the words
/// on the card of that month): the chart then shows what followed what.
pub fn plot_marked(ui: &mut Ui, title: &str, series: &[(String, Color32, Vec<f32>)], months: &[u32], h: f32, fmt: &dyn Fn(f32) -> String, marks: &[Marker]) -> Response {
    let (ch, s) = chrome::dress(ui);
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let n = months.len();
    let mut v = Out::new();
    v.push(Shape::rect_filled(r, 2.0, Color32::from_black_alpha(26)));
    ch.bevel(&mut v, r, false, 1.0);
    v.push(Shape::rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_black_alpha(90))));
    let font = FontId::proportional(12.0);
    // (the scale's figures darker than the soft ink: they sit on the shaded field)
    let scale_ink = hud::lerp(s.soft, s.ink, 0.6);
    // (the room for the scale at the left and the last values at the right)
    let left = 46.0;
    let right = 54.0;
    let inner = Rect::from_min_max(r.min + vec2(left, 30.0), r.max - vec2(right, 16.0));
    ui.painter().text(r.min + vec2(8.0, 3.0), egui::Align2::LEFT_TOP, title, FontId::new(12.5, egui::FontFamily::Name("bold".into())), s.ink);
    if n < 2 || series.is_empty() {
        ui.painter().extend(v);
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "пока мало месяцев", FontId::proportional(13.0), s.soft);
        return resp;
    }
    let all = series.iter().flat_map(|x| x.2.iter().copied()).filter(|x| x.is_finite());
    let (mut lo, mut hi) = all.fold((f32::MAX, f32::MIN), |a, x| (a.0.min(x), a.1.max(x)));
    if lo > 0.0 && lo < hi * 0.6 {
        lo = 0.0;
    }
    if hi - lo < 1e-6 {
        hi = lo + 1.0;
    }
    let pad = (hi - lo) * 0.08;
    let (lo, hi) = (if lo == 0.0 { 0.0 } else { lo - pad }, hi + pad);
    let x_of = |i: usize| inner.left() + inner.width() * i as f32 / (n - 1) as f32;
    let y_of = |val: f32| inner.bottom() - inner.height() * ((val - lo) / (hi - lo)).clamp(0.0, 1.0);
    // the scale
    for k in 0..=4 {
        let val = lo + (hi - lo) * k as f32 / 4.0;
        let y = y_of(val);
        v.push(Shape::line_segment([pos2(inner.left(), y), pos2(inner.right(), y)], Stroke::new(0.6, Color32::from_black_alpha(if k == 0 { 70 } else { 35 }))));
        if k % 2 == 0 {
            ui.painter().text(pos2(inner.left() - 5.0, y), egui::Align2::RIGHT_CENTER, fmt(val), font.clone(), scale_ink);
        }
    }
    if lo < 0.0 && hi > 0.0 {
        let y = y_of(0.0);
        v.push(Shape::line_segment([pos2(inner.left(), y), pos2(inner.right(), y)], Stroke::new(1.0, Color32::from_black_alpha(110))));
        ui.painter().text(pos2(inner.left() - 5.0, y), egui::Align2::RIGHT_CENTER, "0", font.clone(), scale_ink);
    }
    // the years
    let mut last_label = f32::MIN;
    for (i, &m) in months.iter().enumerate() {
        if m % 12 == 0 && i > 0 {
            let x = x_of(i);
            v.push(Shape::line_segment([pos2(x, inner.bottom() - 4.0), pos2(x, inner.bottom())], Stroke::new(1.0, ch.trim)));
            v.push(Shape::line_segment([pos2(x, inner.top()), pos2(x, inner.bottom())], Stroke::new(0.5, Color32::from_black_alpha(25))));
            if x - last_label > 44.0 {
                ui.painter().text(pos2(x, inner.bottom() + 2.0), egui::Align2::CENTER_TOP, format!("{}-й г.", m / 12 + 1), font.clone(), scale_ink);
                last_label = x;
            }
        }
    }
    // the lines
    let mut ends: Vec<(f32, Color32, String)> = Vec::new();
    for (k, (_, col, vals)) in series.iter().enumerate() {
        let pts: Vec<egui::Pos2> = (0..n).map(|i| pos2(x_of(i), y_of(vals.get(i).copied().unwrap_or(0.0)))).collect();
        if k == 0 {
            let mut mesh = epaint::Mesh::default();
            for p in &pts {
                mesh.colored_vertex(*p, col.gamma_multiply(0.22));
                mesh.colored_vertex(pos2(p.x, inner.bottom()), col.gamma_multiply(0.03));
            }
            for i in 0..(n - 1) as u32 {
                let j = i * 2;
                mesh.add_triangle(j, j + 1, j + 3);
                mesh.add_triangle(j, j + 3, j + 2);
            }
            v.push(Shape::mesh(mesh));
        }
        v.push(Shape::line(pts.clone(), Stroke::new(3.2, Color32::from_black_alpha(60))));
        v.push(Shape::line(pts.clone(), Stroke::new(2.0, *col)));
        if let Some(&p) = pts.last() {
            v.push(Shape::circle_filled(p, 3.5, *col));
            v.push(Shape::circle_stroke(p, 3.5, Stroke::new(1.0, Color32::from_black_alpha(160))));
            ends.push((p.y, *col, fmt(vals.last().copied().unwrap_or(0.0))));
        }
    }
    // the player's decisions: a wedge at the foot, on the month's rule
    for m in marks.iter().filter(|m| months.first().is_some_and(|&a| m.month >= a)) {
        if let Some(i) = months.iter().position(|&x| x >= m.month) {
            let x = x_of(i);
            v.push(Shape::line_segment([pos2(x, inner.top()), pos2(x, inner.bottom())], Stroke::new(0.8, s.accent.gamma_multiply(0.45))));
            v.push(Shape::convex_polygon(vec![pos2(x - 4.0, inner.bottom() - 0.5), pos2(x + 4.0, inner.bottom() - 0.5), pos2(x, inner.bottom() - 8.0)], s.accent, Stroke::new(0.8, Color32::from_black_alpha(160))));
        }
    }
    // the hand
    let mut at = None;
    if let Some(p) = resp.hover_pos() {
        let i = (((p.x - inner.left()) / inner.width()) * (n - 1) as f32).round().clamp(0.0, (n - 1) as f32) as usize;
        let x = x_of(i);
        v.push(Shape::line_segment([pos2(x, inner.top()), pos2(x, inner.bottom())], Stroke::new(1.0, ch.trim)));
        at = Some(i);
    }
    ui.painter().extend(v);
    // the last values at the right, not over one another
    ends.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut y_prev = f32::MIN;
    for (y, col, text) in ends {
        let y = y.max(y_prev + 13.0);
        y_prev = y;
        // (in the line's colour, but dark enough to read on the paper: a gold line's figure too)
        ui.painter().text(pos2(inner.right() + 6.0, y), egui::Align2::LEFT_CENTER, text, FontId::new(12.0, egui::FontFamily::Name("bold".into())), hud::lerp(col, s.ink, 0.65));
    }
    // the legend
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        for (name, col, vals) in series {
            let (sw, _) = ui.allocate_exact_size(vec2(14.0, 10.0), Sense::hover());
            ui.painter().rect_filled(sw, 2.0, *col);
            ui.label(egui::RichText::new(format!("{name}: {}", fmt(vals.last().copied().unwrap_or(0.0)))).size(12.5).color(s.ink));
        }
    });
    match at {
        Some(i) => resp.tip(|| {
            let mut c = Card::new(title.to_string()).sub(crate::words::date_words(months[i]));
            for (name, _, vals) in series {
                c = c.row(name.clone(), fmt(vals.get(i).copied().unwrap_or(0.0)));
            }
            // (the decisions of that month and the one before: what the line may answer to)
            let near: Vec<&Marker> = marks.iter().filter(|m| m.month + 1 >= months[i] && m.month <= months[i] && months.first().is_some_and(|&a| m.month >= a)).collect();
            if !near.is_empty() {
                c = c.head("ваши решения");
                for m in near {
                    c = c.text(m.text.clone());
                }
            }
            c
        }),
        None => resp,
    }
}

/// Bars with their figures: a label at the left, the bar to its share of the largest, the number
/// at the right. Their responses (for their cards).
pub fn bars(ui: &mut Ui, items: &[(String, f32, Color32)], fmt: &dyn Fn(f32) -> String) -> Vec<Response> {
    let (ch, s) = chrome::dress(ui);
    let top = items.iter().map(|x| x.1.abs()).fold(0.0f32, f32::max).max(1e-9);
    let w = ui.available_width();
    let label_w = (w * 0.42).min(190.0);
    let num_w = 62.0;
    let mut out = Vec::new();
    for (name, val, col) in items {
        let (r, resp) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
        if ui.is_rect_visible(r) {
            let mut job = egui::text::LayoutJob::simple_singleline(name.clone(), FontId::proportional(13.0), s.ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(label_w - 6.0);
            let gal = ui.painter().layout_job(job);
            ui.painter().galley(pos2(r.min.x, r.center().y - gal.size().y / 2.0), gal, s.ink);
            let bar = Rect::from_min_max(pos2(r.min.x + label_w, r.min.y + 4.0), pos2(r.max.x - num_w, r.max.y - 4.0));
            let mut v = Out::new();
            v.push(Shape::rect_filled(bar, 2.0, Color32::from_black_alpha(40)));
            ch.bevel(&mut v, bar, false, 1.0);
            let f = Rect::from_min_size(bar.min, vec2(bar.width() * (val.abs() / top).clamp(0.0, 1.0), bar.height()));
            if f.width() > 0.5 {
                v.push(Shape::rect_filled(f, 2.0, *col));
                v.push(Shape::line_segment([f.left_top() + vec2(0.5, 1.0), f.right_top() + vec2(-0.5, 1.0)], Stroke::new(1.0, Color32::from_white_alpha(70))));
            }
            if resp.hovered() {
                v.push(Shape::rect_stroke(r.expand(1.0), 2.0, Stroke::new(1.0, ch.trim)));
            }
            ui.painter().extend(v);
            ui.painter().text(pos2(r.max.x - 2.0, r.center().y), egui::Align2::RIGHT_CENTER, fmt(*val), FontId::new(13.0, egui::FontFamily::Name("bold".into())), s.ink);
        }
        out.push(resp);
    }
    out
}

/// A gauge of mood (−1 hostile … +1 content) about its middle: red to the left, green to the
/// right, the number beside it.
pub fn mood(ui: &mut Ui, w: f32, x: f32) -> Response {
    let (ch, s) = chrome::dress(ui);
    let (r, resp) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
    if ui.is_rect_visible(r) {
        let bar = Rect::from_min_max(r.min + vec2(0.0, 4.0), pos2(r.max.x - 34.0, r.max.y - 4.0));
        let mut v = Out::new();
        v.push(Shape::rect_filled(bar, 2.0, Color32::from_black_alpha(45)));
        ch.bevel(&mut v, bar, false, 1.0);
        let mid = bar.center().x;
        let x = x.clamp(-1.0, 1.0);
        let end = mid + bar.width() / 2.0 * x;
        let col = if x >= 0.0 { hud::lerp(super::kit::MIXED, super::kit::GOOD, x.min(1.0)) } else { hud::lerp(super::kit::MIXED, super::kit::BAD, (-x).min(1.0)) };
        let f = Rect::from_min_max(pos2(mid.min(end), bar.min.y), pos2(mid.max(end), bar.max.y));
        if f.width() > 0.5 {
            v.push(Shape::rect_filled(f, 1.0, col));
        }
        v.push(Shape::line_segment([pos2(mid, bar.min.y - 2.0), pos2(mid, bar.max.y + 2.0)], Stroke::new(1.0, ch.trim)));
        ui.painter().extend(v);
        ui.painter().text(pos2(r.max.x, r.center().y), egui::Align2::RIGHT_CENTER, format!("{:+.0}", x * 100.0), FontId::new(12.5, egui::FontFamily::Name("bold".into())), s.ink);
    }
    resp
}

/// A bar of shares (parts of a whole), each in its colour; the names and shares on its card.
pub fn shares(ui: &mut Ui, parts: &[(String, f32, Color32)], h: f32) -> Response {
    let (ch, _) = chrome::dress(ui);
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let all: f32 = parts.iter().map(|p| p.1.max(0.0)).sum::<f32>().max(1e-9);
    let mut v = Out::new();
    let mut x = r.min.x;
    for (_, val, col) in parts {
        let pw = r.width() * val.max(0.0) / all;
        let pr = Rect::from_min_size(pos2(x, r.min.y), vec2(pw, h));
        v.push(Shape::rect_filled(pr, 0.0, *col));
        x += pw;
    }
    ch.bevel(&mut v, r, false, 1.0);
    v.push(Shape::rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_black_alpha(120))));
    ui.painter().extend(v);
    resp
}

/// A line of the last values in `r` with no scale: a dot at its end, a soft fill under it. A nearly
/// flat series stays flat (the range is at least a few percent of its size), not stretched to the
/// whole height.
pub fn spark(p: &egui::Painter, r: Rect, vals: &[f32], col: Color32) {
    let vals: Vec<f32> = vals.iter().copied().filter(|x| x.is_finite()).collect();
    if vals.len() < 2 {
        p.line_segment([pos2(r.left(), r.center().y), pos2(r.right(), r.center().y)], Stroke::new(1.0, col.gamma_multiply(0.4)));
        return;
    }
    let (mut lo, mut hi) = vals.iter().fold((f32::MAX, f32::MIN), |a, &x| (a.0.min(x), a.1.max(x)));
    let mid = (lo + hi) / 2.0;
    let least = mid.abs().max(1e-3) * 0.06;
    if hi - lo < least {
        lo = mid - least / 2.0;
        hi = mid + least / 2.0;
    }
    let n = vals.len();
    let pts: Vec<egui::Pos2> = vals.iter().enumerate().map(|(i, &x)| pos2(r.left() + r.width() * i as f32 / (n - 1) as f32, r.bottom() - r.height() * ((x - lo) / (hi - lo)).clamp(0.0, 1.0))).collect();
    let mut mesh = epaint::Mesh::default();
    for q in &pts {
        mesh.colored_vertex(*q, col.gamma_multiply(0.30));
        mesh.colored_vertex(pos2(q.x, r.bottom()), col.gamma_multiply(0.02));
    }
    for i in 0..(n - 1) as u32 {
        let j = i * 2;
        mesh.add_triangle(j, j + 1, j + 3);
        mesh.add_triangle(j, j + 3, j + 2);
    }
    p.add(Shape::mesh(mesh));
    p.add(Shape::line(pts.clone(), Stroke::new(1.8, col)));
    if let Some(&q) = pts.last() {
        p.circle_filled(q, 2.6, col);
        p.circle_stroke(q, 2.6, Stroke::new(1.0, Color32::from_black_alpha(150)));
    }
}
