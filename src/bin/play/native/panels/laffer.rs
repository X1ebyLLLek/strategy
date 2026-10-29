//! What a rate of the income tax would bring a month, the hiding of taxes counted (36th session): before, «≈ в казну» was the rate
//! times the people's income — as if nobody ever hid a coin — and a point of the rate was always worth the same. Now the same
//! formula the owners' month uses (`fiscal::evasion_of`, `kept_k`) is run over every rate: the higher the rate, the more of it is
//! hidden, so the yield climbs slower, flattens at a peak and then falls — the curve of Laffer. The page draws it, marks the
//! rate in force and the peak, and says what one more point of the rate is worth now.

use super::*;
use super::super::charts::num;
use super::super::hud;
use super::super::kit;
use crate::country::fiscal::{evasion_of, kept_k, tax_k};
use crate::country::policy::role;
use strategy::economy::nation::Nation;
use strategy::pack::Pack;

/// What the state's hold and the other taxes do to the hiding: the part of the formula that does not depend on the rate.
#[derive(Clone, Copy)]
pub struct Hiding {
    /// The other taxes (purchases, profit, property) as a share of the output, last month's.
    pub extra: f32,
    pub stability: f32,
    pub reach: f32,
}

impl Hiding {
    /// The share hidden at income tax `rate`.
    pub fn at(&self, rate: f32) -> f32 {
        evasion_of(rate + self.extra, self.stability, self.reach)
    }
}

/// What the income tax at `rate`, its «who pays» being `kind`, brings in a month by last month's incomes, the hiding counted.
pub fn yield_at(n: &Nation, pack: &Pack, kind: usize, rate: f32, hide: &Hiding) -> f32 {
    let ev = hide.at(rate);
    (0..pack.groups.len())
        .map(|g| {
            let r = role(pack, g);
            let k = if kind == 0 { 1.0 } else { tax_k(kind, r) };
            n.income[g].max(0.0) * (rate * k * kept_k(ev, r)).clamp(0.0, 0.95)
        })
        .sum()
}

/// The same without any hiding: what the rate would bring if everybody paid (what one more point is worth on paper).
pub fn paper_at(n: &Nation, pack: &Pack, kind: usize, rate: f32) -> f32 {
    (0..pack.groups.len())
        .map(|g| {
            let k = if kind == 0 { 1.0 } else { tax_k(kind, role(pack, g)) };
            n.income[g].max(0.0) * (rate * k).clamp(0.0, 0.95)
        })
        .sum()
}

/// The curve: the yield at each of `STEPS + 1` rates from nothing to `TOP`; and the rate where it peaks.
pub const TOP: f32 = 0.6;
const STEPS: usize = 60;

pub struct Curve {
    pub points: Vec<f32>,
    pub peak: usize,
}

impl Curve {
    /// A peak inside the range: the yield really falls after it (else the curve climbs to the end, ever slower).
    pub fn has_peak(&self) -> bool {
        self.peak + 1 < self.points.len() && self.peak > 0
    }

    /// The rate at the peak.
    pub fn peak_rate(&self) -> f32 {
        TOP * self.peak as f32 / STEPS as f32
    }
}

pub fn curve(n: &Nation, pack: &Pack, kind: usize, hide: &Hiding) -> Curve {
    let points: Vec<f32> = (0..=STEPS).map(|i| yield_at(n, pack, kind, TOP * i as f32 / STEPS as f32, hide)).collect();
    // (the first of the highest: a flat top reads as its beginning — nobody gains by going further)
    let top = points.iter().copied().fold(0.0f32, f32::max);
    let peak = points.iter().position(|&y| y >= top * 0.999).unwrap_or(0);
    Curve { points, peak }
}

/// What one more point of the rate is worth now (negative: the state already loses by raising it).
pub fn point_worth(n: &Nation, pack: &Pack, kind: usize, rate: f32, hide: &Hiding) -> f32 {
    yield_at(n, pack, kind, rate + 0.01, hide) - yield_at(n, pack, kind, rate, hide)
}

/// The plot, into the room: a groove with the area under the curve filled, a mark for the peak, a line and a dot for the rate in force;
/// the hand over it says what that rate would bring.
pub fn plot(ui: &mut egui::Ui, s: &Skin, c: &Curve, rate: f32, usual: f32, took: f32) {
    let (ch, _) = chrome::dress(ui);
    let w = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 2.0);
    let (r, resp) = ui.allocate_exact_size(vec2(w, 96.0), egui::Sense::hover());
    if !ui.is_rect_visible(r) {
        return;
    }
    let area = egui::Rect::from_min_max(r.min + vec2(6.0, 8.0), r.max - vec2(6.0, 18.0));
    let top = c.points.iter().copied().fold(0.0f32, f32::max).max(1e-6) * 1.12;
    let at = |i: usize| egui::pos2(area.min.x + area.width() * i as f32 / STEPS as f32, area.max.y - area.height() * (c.points[i] / top).clamp(0.0, 1.0));
    let x_of = |rate: f32| area.min.x + area.width() * (rate / TOP).clamp(0.0, 1.0);
    let mut v = chrome::Out::new();
    v.push(egui::Shape::rect_filled(r, 3.0, Color32::from_black_alpha(46)));
    ch.bevel(&mut v, r, false, 1.0);
    // (a faint grid: the tens of the rate)
    for k in 1..6 {
        let x = area.min.x + area.width() * k as f32 / 6.0;
        v.push(egui::Shape::line_segment([egui::pos2(x, area.min.y), egui::pos2(x, area.max.y)], egui::Stroke::new(1.0, Color32::from_black_alpha(28))));
    }
    v.push(egui::Shape::line_segment([area.left_bottom(), area.right_bottom()], egui::Stroke::new(1.2, Color32::from_black_alpha(130))));
    // (the area under the curve: a strip a step wide, lighter at the top)
    let mut m = egui::epaint::Mesh::default();
    let (a, b) = (s.accent.gamma_multiply(0.62), s.accent.gamma_multiply(0.1));
    for i in 0..=STEPS {
        let p = at(i);
        m.colored_vertex(p, a);
        m.colored_vertex(egui::pos2(p.x, area.max.y), b);
        if i > 0 {
            let k = 2 * i as u32;
            m.add_triangle(k - 2, k - 1, k);
            m.add_triangle(k - 1, k + 1, k);
        }
    }
    v.push(egui::Shape::mesh(m));
    let line: Vec<egui::Pos2> = (0..=STEPS).map(at).collect();
    v.push(egui::Shape::line(line.clone(), egui::Stroke::new(3.0, Color32::from_black_alpha(120))));
    v.push(egui::Shape::line(line, egui::Stroke::new(1.8, hud::lighten(s.accent, 0.25))));
    // (the usual rate of the era: a notch of trim on the axis)
    let ux = x_of(usual);
    v.push(egui::Shape::convex_polygon(vec![egui::pos2(ux, area.max.y), egui::pos2(ux - 4.0, area.max.y + 6.0), egui::pos2(ux + 4.0, area.max.y + 6.0)], ch.trim, egui::Stroke::new(1.0, Color32::from_black_alpha(150))));
    // (the peak: a diamond and a dotted drop to the axis)
    if c.has_peak() {
        let pk = at(c.peak);
        let mut y = pk.y;
        while y < area.max.y {
            v.push(egui::Shape::line_segment([egui::pos2(pk.x, y), egui::pos2(pk.x, (y + 3.0).min(area.max.y))], egui::Stroke::new(1.0, Color32::from_black_alpha(120))));
            y += 6.0;
        }
        hud::diamond(&mut v, pk, 5.5, Color32::from_black_alpha(200));
        hud::diamond(&mut v, pk, 4.0, ch.trim);
    }
    // (the rate in force: a line and a dot on the curve)
    let rx = x_of(rate);
    let i = ((rate / TOP).clamp(0.0, 1.0) * STEPS as f32).round() as usize;
    let on = egui::pos2(rx, at(i.min(STEPS)).y);
    v.push(egui::Shape::line_segment([egui::pos2(rx, area.min.y), egui::pos2(rx, area.max.y)], egui::Stroke::new(1.5, Color32::from_black_alpha(150))));
    v.push(egui::Shape::line_segment([egui::pos2(rx, area.min.y), egui::pos2(rx, area.max.y)], egui::Stroke::new(0.8, Color32::from_rgb(250, 244, 226))));
    v.push(egui::Shape::circle_filled(on, 5.0, Color32::from_black_alpha(190)));
    v.push(egui::Shape::circle_filled(on, 3.6, Color32::from_rgb(250, 244, 226)));
    ui.painter().extend(v);
    // (the axis: 0, 20, 40, 60 per cent)
    for k in 0..=3 {
        let x = area.min.x + area.width() * k as f32 / 3.0;
        let al = match k {
            0 => egui::Align2::LEFT_TOP,
            3 => egui::Align2::RIGHT_TOP,
            _ => egui::Align2::CENTER_TOP,
        };
        ui.painter().text(egui::pos2(x, area.max.y + 7.0), al, format!("{}%", (TOP * 100.0 * k as f32 / 3.0).round()), egui::FontId::new(11.5, egui::FontFamily::Proportional), s.soft);
    }
    // (the hand over it: the rate under it and what it brings)
    let hand = resp.hover_pos().filter(|p| area.expand2(vec2(6.0, 8.0)).contains(*p)).map(|p| (((p.x - area.min.x) / area.width()).clamp(0.0, 1.0) * STEPS as f32).round() as usize);
    if let Some(k) = hand {
        let (rate_k, y) = (TOP * k as f32 / STEPS as f32, c.points[k]);
        ui.painter().circle_stroke(at(k), 4.0, egui::Stroke::new(1.5, hud::lighten(s.accent, 0.4)));
        resp.tip(|| {
            Card::new(format!("Ставка {:.0}%", rate_k * 100.0))
                .icon("coins")
                .row("в казну в месяц", num(y))
                .row("против нынешних", format!("{}{}", if y >= took { "+" } else { "−" }, num((y - took).abs())))
                .note(if c.has_peak() { format!("вершина кривой — при {:.0}%", c.peak_rate() * 100.0) } else { "кривая ещё растёт".to_string() })
        });
    } else {
        resp.tip(|| {
            Card::new("Сколько даст ставка")
                .icon("coins")
                .text("По высоте — деньги в казну в месяц при каждой ставке подоходного налога, по доходам прошлого месяца. Если есть ромб — это вершина: выше неё казна уже не прибавляет, люди прячут больше, чем налог берёт. Без ромба кривая растёт до конца, но всё медленнее. Белая линия — ваша ставка, треугольник на оси — обычная в эпохе.")
                .note("кривая статична: на деле тяжёлая ставка ещё и сокращает сами доходы")
        });
    }
}

/// The words under the plot: how much of a point of the rate reaches the treasury (the rest is hidden), where the rate stands against
/// the peak if there is one.
pub fn verdict(ui: &mut egui::Ui, s: &Skin, rate: f32, c: &Curve, worth: f32, paper: f32, took: f32, modelled: f32) {
    // (a point on paper is what it would bring if nobody hid anything; the share that arrives is the hiding's price)
    let reach = if paper > 1e-6 { (worth / paper).clamp(-1.0, 1.0) } else { 1.0 };
    let (text, col) = if worth < 0.0 {
        (format!("Ставка выше вершины ({:.0}%): ещё пункт даст {} в месяц — казна теряет. Снижать ставку выгодно и казне.", c.peak_rate() * 100.0, num(worth)), kit::BAD)
    } else if reach < 0.5 {
        (format!("До казны доходит лишь {:.0}% прибавки от пункта ставки ({} из {} на бумаге): остальное прячут. Выше — почти зря.", reach * 100.0, num(worth), num(paper)), kit::MIXED)
    } else if c.has_peak() && rate + 0.05 >= c.peak_rate() {
        (format!("До вершины ({:.0}%) недалеко: пункт ставки даёт {} в месяц — почти всё съедает уклонение.", c.peak_rate() * 100.0, num(worth)), kit::MIXED)
    } else {
        (format!("Пункт ставки даёт {} в месяц — {:.0}% от того, что вышло бы на бумаге ({}); остальное прячут.", num(worth), reach * 100.0, num(paper)), kit::GOOD)
    };
    kit::words(ui, text, 12.5, hud::lerp(col, s.ink, 0.45));
    if took > 1e-6 && ((modelled - took) / took).abs() > 0.15 {
        kit::words(ui, format!("Взято в прошлом месяце: {} (кривая считает {}: уклонение меняется вслед за ставкой не сразу).", num(took), num(modelled)), 12.0, s.soft);
    }
}
