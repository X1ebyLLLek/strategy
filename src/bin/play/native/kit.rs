//! The pieces the folders are built of, in the manner of HOI4's and Victoria's screens: a ribbon of
//! dark metal heading each part of a folder (an icon in a socket at its head, the words in the
//! era's letters, a pennant's tail); cards of paper raised off the sheet with a band of colour at
//! their edge; glossy gauges sunk into the paper with ticks at the quarters; tiles of the figures
//! that matter (an icon, a small heading, a word in its tone) on dark plates; emblems of the
//! country's features — a square of colour by how good the feature is, framed in trim, its icon,
//! its strength along its foot, an arrow while it grows or fades.

use eframe::egui::{self, epaint, pos2, vec2, Color32, FontFamily, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui};

use super::chrome::{self, Card, Out, Tip};
use super::hud;
use super::look::Skin;

/// Colours of tone: good, bad, mixed, plain (on dark plates and in emblems).
pub const GOOD: Color32 = Color32::from_rgb(78, 142, 64);
pub const BAD: Color32 = Color32::from_rgb(176, 58, 42);
pub const MIXED: Color32 = Color32::from_rgb(190, 142, 52);
pub const PLAIN: Color32 = Color32::from_rgb(92, 104, 112);

/// A heading of a part of a folder (33rd session: it was a bar of dark metal with a pennant's tail, one after another down
/// the page like the header cells of a table; now a chapter heading on the paper): a tinted band that fades out to the
/// right, trim above and a rule below, a seal at its head, the words in the era's heading letters in ink, a brass tab
/// at the left edge and a diamond at the end of the rule.
pub fn ribbon(ui: &mut Ui, text: &str, icon: Option<&str>) {
    let (ch, s) = chrome::dress(ui);
    ui.add_space(12.0);
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
    if !ui.is_rect_visible(r) {
        ui.add_space(4.0);
        return;
    }
    let band = Rect::from_min_max(r.min + vec2(0.0, 2.0), r.max - vec2(0.0, 2.0));
    let mut v = Out::new();
    // (the band: the paper's edge colour, strong at the left, gone at the right)
    hud::glow(&mut v, band, s.edge.gamma_multiply(0.62));
    hud::fade_rule(&mut v, band.min.x, band.max.x, band.min.y, ch.trim, 1.4);
    hud::fade_rule(&mut v, band.min.x, band.max.x - 6.0, band.max.y, hud::darken(ch.trim, 0.2), 1.4);
    // (a brass tab at the left edge)
    let tab = Rect::from_min_max(band.min, pos2(band.min.x + 4.5, band.max.y));
    v.push(Shape::rect_filled(tab, 0.0, ch.trim));
    hud::gloss(&mut v, tab, Color32::from_white_alpha(70), Color32::from_black_alpha(70));
    hud::diamond(&mut v, pos2(band.max.x - 4.0, band.max.y), 3.2, ch.trim);
    ui.painter().extend(v);
    let mut x = band.min.x + 14.0;
    if let Some(icon) = icon {
        hud::seal(ui, &ch, pos2(band.min.x + 25.0, r.center().y), 12.5, icon, ch.trim);
        x = band.min.x + 46.0;
    }
    let gal = ui.painter().layout_no_wrap(text.to_uppercase(), FontId::new(17.0, FontFamily::Name("head".into())), s.ink);
    let y = r.center().y - gal.size().y / 2.0;
    ui.painter().galley(pos2(x + 0.8, y + 0.8), gal.clone(), Color32::from_white_alpha(70));
    ui.painter().galley(pos2(x, y), gal, s.ink);
    ui.add_space(5.0);
}

/// A card of paper raised off the sheet: its contents with a margin, a band of `edge` colour down
/// its left side (none: plain), lit (a little lighter) when `lit`. The response is the card's.
pub fn framed<R>(ui: &mut Ui, edge: Option<Color32>, lit: bool, add: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    let (ch, s) = chrome::dress(ui);
    let back = chrome::hold(ui);
    let left = if edge.is_some() { 14.0 } else { 10.0 };
    let inner = egui::Frame::none().inner_margin(egui::Margin { left, right: 10.0, top: 8.0, bottom: 8.0 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui)
    });
    let r = inner.response.rect;
    let mut v = Out::new();
    leaf_at(&mut v, &ch, &s, r, edge, lit);
    chrome::under(ui, back, v);
    ui.add_space(5.0);
    egui::InnerResponse { inner: inner.inner, response: ui.interact(r, ui.id().with(("framed", r.min.x as i32, r.min.y as i32)), Sense::hover()) }
}

/// The leaf of paper of a card into `r` (33rd session: a flat leaf, lighter than the sheet, a fine edge and a lit top line, two
/// corners cut, a diamond of trim at the other two; it was a glossy bevelled box, a cell of a table): a band of `edge` down its
/// left side, `lit` — a little brighter.
pub fn leaf_at(v: &mut Out, ch: &chrome::Chrome, s: &Skin, r: Rect, edge: Option<Color32>, lit: bool) {
    v.push(epaint::Shadow { offset: vec2(0.0, 2.0), blur: 7.0, spread: 0.0, color: Color32::from_black_alpha(52) }.as_shape(r, 2.0).into());
    let pts = hud::chamfer(r, 7.0);
    hud::lay_poly(v, &pts, &ch.paper, Color32::WHITE);
    hud::plaque(v, r, 7.0, Color32::from_white_alpha(if lit { 92 } else { 58 }), Stroke::NONE, 0);
    v.push(Shape::line_segment([pos2(r.min.x + 8.0, r.min.y + 1.0), pos2(r.max.x - 2.0, r.min.y + 1.0)], Stroke::new(1.0, Color32::from_white_alpha(120))));
    v.push(Shape::closed_line(pts, Stroke::new(1.0, s.edge.gamma_multiply(0.95))));
    hud::diamond(v, pos2(r.max.x - 2.0, r.min.y + 2.0), 2.4, ch.trim);
    hud::diamond(v, pos2(r.min.x + 2.0, r.max.y - 2.0), 2.4, ch.trim);
    if let Some(c) = edge {
        let band = Rect::from_min_size(r.min + vec2(1.0, 8.0), vec2(5.0, r.height() - 9.0));
        v.push(Shape::rect_filled(band, 1.0, c));
        hud::gloss(v, band, Color32::from_white_alpha(60), Color32::from_black_alpha(60));
    }
}

/// An icon in a round socket of dark metal ringed with trim (the colour `tone` warming the disc), at `c`, `r` across: the mark of a
/// card's head.
pub fn socket(ui: &Ui, c: Pos2, r: f32, icon: &str, tone: Color32, warm: f32) {
    let (ch, _) = chrome::dress(ui);
    let icons = chrome::icons(ui.ctx());
    let mut v = Out::new();
    v.push(Shape::circle_filled(c + vec2(0.0, 1.5), r + 0.5, Color32::from_black_alpha(110)));
    hud::ring(&mut v, c, r, (r * 0.17).max(2.0), ch.trim);
    hud::disc(&mut v, c, r - (r * 0.17).max(2.0) + 0.5, hud::lerp(Color32::from_rgb(58, 54, 46), tone, warm), Color32::from_rgb(16, 14, 12));
    ui.painter().extend(v);
    let s = r * 1.15;
    super::look::icon(&icons, icon, s, Color32::from_black_alpha(190)).paint_at(ui, Rect::from_center_size(c + vec2(1.0, 1.2), vec2(s, s)));
    super::look::icon(&icons, icon, s, hud::lighten(ch.trim, 0.3)).paint_at(ui, Rect::from_center_size(c, vec2(s, s)));
}

/// A glossy gauge into `r`: a groove sunk into the paper, the fill to `frac` in `col` (lighter at
/// its top), ticks at the quarters, a glint along its top.
pub fn gauge_at(v: &mut Out, ch: &chrome::Chrome, r: Rect, frac: f32, col: Color32) {
    v.push(Shape::rect_filled(r, 2.0, Color32::from_black_alpha(90)));
    ch.bevel(v, r, false, 1.0);
    let inner = r.shrink(1.5);
    let f = Rect::from_min_size(inner.min, vec2(inner.width() * frac.clamp(0.0, 1.0), inner.height()));
    if f.width() > 0.5 {
        let mut m = epaint::Mesh::default();
        let (a, b) = (hud::lighten(col, 0.28), hud::darken(col, 0.22));
        for (p, c) in [(f.left_top(), a), (f.right_top(), a), (f.right_bottom(), b), (f.left_bottom(), b)] {
            m.colored_vertex(p, c);
        }
        m.add_triangle(0, 1, 2);
        m.add_triangle(0, 2, 3);
        v.push(Shape::mesh(m));
        v.push(Shape::line_segment([f.left_top() + vec2(0.5, 1.0), f.right_top() + vec2(-0.5, 1.0)], Stroke::new(1.0, Color32::from_white_alpha(90))));
    }
    for k in 1..4 {
        let x = inner.min.x + inner.width() * k as f32 / 4.0;
        v.push(Shape::line_segment([pos2(x, inner.max.y - inner.height() * 0.45), pos2(x, inner.max.y)], Stroke::new(1.0, Color32::from_black_alpha(110))));
    }
    v.push(Shape::rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_black_alpha(140))));
}

/// A gauge as wide as `w`: its response (for its card).
pub fn gauge(ui: &mut Ui, w: f32, h: f32, frac: f32, col: Color32) -> Response {
    let (ch, _) = chrome::dress(ui);
    let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    if ui.is_rect_visible(r) {
        let mut v = Out::new();
        gauge_at(&mut v, &ch, r, frac, col);
        ui.painter().extend(v);
    }
    resp
}

/// A figure that matters, on its tile: its icon, a small heading, a word in its tone.
pub struct Stat {
    pub icon: String,
    pub title: String,
    pub value: String,
    pub tone: Color32,
    /// The number, when the tile shows one before its word (28th session: «цифры, а не слова»).
    pub num: Option<String>,
}

impl Stat {
    pub fn new(icon: &str, title: &str, value: impl Into<String>, tone: Color32) -> Stat {
        Stat { icon: icon.into(), title: title.into(), value: value.into(), tone, num: None }
    }

    /// The same tile with its number first, large; the word after it, small.
    pub fn num(mut self, n: impl Into<String>) -> Stat {
        self.num = Some(n.into());
        self
    }
}

/// Tiles of figures, `cols` to a row, on dark plates: the icon in trim at the left, the heading
/// small above the word; a band of the tone at the foot. Their responses (for their cards).
pub fn stats(ui: &mut Ui, items: &[Stat], cols: usize) -> Vec<Response> {
    let (ch, s) = chrome::dress(ui);
    let icons = chrome::icons(ui.ctx());
    let gap = 5.0;
    // (fewer to a row when a heading or a word would not fit its tile: down to two)
    let fits = |cols: usize| {
        let w = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let tw = w - 46.0;
        items.iter().all(|it| {
            let t = ui.painter().layout_no_wrap(it.title.to_uppercase(), FontId::new(12.0, FontFamily::Name("head".into())), s.desk_ink).size().x;
            let v = ui.painter().layout_no_wrap(it.value.clone(), FontId::new(14.5, FontFamily::Name("bold".into())), s.desk_ink).size().x;
            let v = match &it.num {
                Some(n) => ui.painter().layout_no_wrap(n.clone(), FontId::new(16.0, FontFamily::Name("bold".into())), s.desk_ink).size().x,
                None => v,
            };
            t <= tw && v <= tw
        })
    };
    let mut cols = cols.max(1);
    while cols > 2 && !fits(cols) {
        cols -= 1;
    }
    let w = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).floor();
    let h = 48.0;
    let rows = items.len().div_ceil(cols);
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), rows as f32 * (h + gap) - gap), Sense::hover());
    let mut out = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let r = Rect::from_min_size(area.min + vec2((i % cols) as f32 * (w + gap), (i / cols) as f32 * (h + gap)), vec2(w, h));
        let resp = ui.interact(r, ui.id().with(("stat", i, it.title.as_str())), Sense::hover());
        let mut v = Out::new();
        // (33rd session: a plaque with two cut corners, the tone glowing from its left edge, its icon in a round socket set into
        // the plaque's edge, a rule of the tone at its foot — they were identical dark slabs, a grid of cells)
        let pts = hud::chamfer(r, 8.0);
        v.push(epaint::Shadow { offset: vec2(0.0, 2.0), blur: 6.0, spread: 0.0, color: Color32::from_black_alpha(95) }.as_shape(r, 3.0).into());
        hud::lay_poly(&mut v, &pts, &ch.bar, Color32::WHITE);
        hud::plaque(&mut v, r, 8.0, Color32::from_black_alpha(if resp.hovered() { 96 } else { 128 }), Stroke::NONE, 0);
        hud::glow(&mut v, Rect::from_min_max(r.min + vec2(9.0, 2.0), pos2(r.min.x + r.width() * 0.8, r.max.y - 2.0)), it.tone.gamma_multiply(if resp.hovered() { 0.55 } else { 0.38 }));
        hud::gloss(&mut v, r.shrink(1.0), Color32::from_white_alpha(18), Color32::from_black_alpha(40));
        v.push(Shape::closed_line(pts, Stroke::new(1.2, ch.trim.gamma_multiply(0.85))));
        let sock = pos2(r.min.x + 22.0, r.center().y - 1.0);
        v.push(Shape::circle_filled(sock + vec2(0.0, 1.5), 16.0, Color32::from_black_alpha(120)));
        hud::ring(&mut v, sock, 15.5, 2.6, ch.trim);
        hud::disc(&mut v, sock, 13.0, hud::lerp(Color32::from_rgb(58, 54, 46), it.tone, 0.35), Color32::from_rgb(16, 14, 12));
        let (fx0, fx1) = (r.min.x + 42.0, r.max.x - 10.0);
        hud::fade_rule(&mut v, fx0, fx1, r.max.y - 5.0, hud::lighten(it.tone, 0.1), 2.6);
        ui.painter().extend(v);
        let ir = Rect::from_center_size(sock, vec2(18.0, 18.0));
        super::look::icon(&icons, &it.icon, 18.0, Color32::from_black_alpha(200)).paint_at(ui, ir.translate(vec2(1.0, 1.2)));
        super::look::icon(&icons, &it.icon, 18.0, hud::lighten(ch.trim, 0.25)).paint_at(ui, ir);
        let x = r.min.x + 42.0;
        let tw = w - 46.0;
        let mut job = egui::text::LayoutJob::simple_singleline(it.title.to_uppercase(), FontId::new(12.0, FontFamily::Name("head".into())), s.desk_ink.gamma_multiply(0.78));
        job.wrap = egui::text::TextWrapping::truncate_at_width(tw);
        let g1 = ui.painter().layout_job(job);
        ui.painter().galley(pos2(x, r.min.y + 6.0), g1, s.desk_ink);
        let val_col = hud::lerp(s.desk_ink, hud::lighten(it.tone, 0.35), 0.45);
        match &it.num {
            // (the number large; the word after it small and softer, if it fits)
            Some(n) => {
                let gn = ui.painter().layout_no_wrap(n.clone(), FontId::new(16.0, FontFamily::Name("bold".into())), val_col);
                let nw = gn.size().x;
                ui.painter().galley(pos2(x, r.min.y + 20.0), gn, val_col);
                // (the word after the number only whole: a word cut short — «пол…» — says nothing, and
                // the card of the tile has it)
                let room = tw - nw - 6.0;
                // (29th session: the word whole or not at all — «пол...» cut in the middle read as a fault;
                // the card of the tile says it in full)
                if room > 24.0 && !it.value.is_empty() {
                    let gw = ui.painter().layout_no_wrap(it.value.clone(), FontId::new(12.5, FontFamily::Proportional), s.desk_ink.gamma_multiply(0.8));
                    if gw.size().x <= room {
                        ui.painter().galley(pos2(x + nw + 6.0, r.min.y + 23.0), gw, s.desk_ink);
                    }
                }
            }
            None => {
                let mut job = egui::text::LayoutJob::simple_singleline(it.value.clone(), FontId::new(14.5, FontFamily::Name("bold".into())), val_col);
                job.wrap = egui::text::TextWrapping::truncate_at_width(tw);
                let g2 = ui.painter().layout_job(job);
                ui.painter().galley(pos2(x, r.min.y + 21.5), g2, val_col);
            }
        }
        out.push(resp);
    }
    ui.add_space(4.0);
    out
}

/// An emblem of a feature into `r` (a square): a shadow, a frame of trim with a bevel, inside a
/// field of the tone's colour lit from its middle, the icon with its shade, the strength as a bar
/// of trim along its foot, an arrow at its corner while it grows (up) or fades (down).
#[allow(clippy::too_many_arguments)]
pub fn emblem(ui: &Ui, r: Rect, icon: &str, tone: Color32, level: f32, trend: i8, hot: bool, fresh: bool) {
    let (ch, s) = chrome::dress(ui);
    let icons = chrome::icons(ui.ctx());
    let mut v = Out::new();
    v.push(epaint::Shadow { offset: vec2(0.0, 3.0), blur: 7.0, spread: 0.0, color: Color32::from_black_alpha(120) }.as_shape(r, 3.0).into());
    // the frame of trim
    v.push(Shape::rect_filled(r, 3.0, if hot { hud::lighten(ch.trim, 0.2) } else { ch.trim }));
    hud::gloss(&mut v, r, Color32::from_white_alpha(70), Color32::from_black_alpha(90));
    let field = r.shrink(4.0);
    v.push(Shape::rect_filled(field.expand(1.0), 2.0, Color32::from_black_alpha(200)));
    // the field: the tone lit from its middle, dark at its edges
    let mut m = epaint::Mesh::default();
    let mid = hud::lighten(tone, if hot { 0.28 } else { 0.18 });
    let rim = hud::darken(tone, 0.55);
    m.colored_vertex(field.center() - vec2(0.0, field.height() * 0.12), mid);
    let corners = [field.left_top(), field.right_top(), field.right_bottom(), field.left_bottom()];
    for (i, c) in corners.iter().enumerate() {
        m.colored_vertex(*c, rim);
        let next = corners[(i + 1) % 4];
        m.colored_vertex(pos2((c.x + next.x) / 2.0, (c.y + next.y) / 2.0), hud::lerp(rim, mid, 0.35));
    }
    for i in 0..4u32 {
        let c = 1 + 2 * i;
        let half = c + 1;
        let next = 1 + 2 * ((i + 1) % 4);
        m.add_triangle(0, c, half);
        m.add_triangle(0, half, next);
    }
    v.push(Shape::mesh(m));
    // (a faint lattice, as embossed)
    for k in 1..4 {
        let x = field.min.x + field.width() * k as f32 / 4.0;
        v.push(Shape::line_segment([pos2(x, field.min.y), pos2(x, field.max.y)], Stroke::new(0.6, Color32::from_white_alpha(10))));
    }
    v.push(Shape::rect_stroke(field, 2.0, Stroke::new(1.0, Color32::from_white_alpha(40))));
    // the strength along its foot
    let bar = Rect::from_min_max(pos2(field.min.x + 3.0, field.max.y - 6.0), pos2(field.max.x - 3.0, field.max.y - 3.0));
    v.push(Shape::rect_filled(bar, 1.0, Color32::from_black_alpha(170)));
    v.push(Shape::rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * level.clamp(0.0, 1.0), bar.height())), 1.0, hud::lighten(ch.trim, 0.2)));
    ui.painter().extend(v);
    let size = field.width() * 0.56;
    let ir = Rect::from_center_size(field.center() - vec2(0.0, 2.0), vec2(size, size));
    super::look::icon(&icons, icon, size, Color32::from_black_alpha(220)).paint_at(ui, ir.translate(vec2(1.5, 2.0)));
    super::look::icon(&icons, icon, size, hud::lighten(s.desk_ink, 0.2)).paint_at(ui, ir);
    let mut v = Out::new();
    if trend != 0 {
        let c = pos2(r.max.x - 7.0, r.min.y + 7.0);
        v.push(Shape::circle_filled(c, 6.5, Color32::from_black_alpha(200)));
        hud::arrow(&mut v, c, trend > 0, if trend > 0 { Color32::from_rgb(140, 210, 110) } else { Color32::from_rgb(230, 120, 90) });
    }
    if fresh {
        hud::diamond(&mut v, pos2(r.min.x + 7.0, r.min.y + 7.0), 4.5, Color32::from_rgb(250, 220, 120));
    }
    ui.painter().extend(v);
}

/// One emblem in a grid: its icon, colour, strength, trend, name, and its card.
pub struct Mark<'a> {
    pub icon: String,
    pub tone: Color32,
    pub level: f32,
    pub trend: i8,
    pub fresh: bool,
    pub name: String,
    pub card: Box<dyn Fn() -> Card + 'a>,
}

/// Emblems in a grid, `cols` to a row: each over its name (two lines at most); the card of each
/// under the hand.
pub fn emblems(ui: &mut Ui, marks: &[Mark], cols: usize, side: f32) {
    let (_, s) = chrome::dress(ui);
    let cols = cols.max(1);
    let gap = 6.0;
    let w = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).floor();
    let h = side + 36.0;
    let rows = marks.len().div_ceil(cols);
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), rows as f32 * (h + gap)), Sense::hover());
    for (i, m) in marks.iter().enumerate() {
        let cell = Rect::from_min_size(area.min + vec2((i % cols) as f32 * (w + gap), (i / cols) as f32 * (h + gap)), vec2(w, h));
        let resp = ui.interact(cell, ui.id().with(("emblem", i, m.name.as_str())), Sense::hover());
        let er = Rect::from_center_size(pos2(cell.center().x, cell.min.y + side / 2.0 + 2.0), vec2(side, side));
        if ui.is_rect_visible(cell) {
            emblem(ui, er, &m.icon, m.tone, m.level, m.trend, resp.hovered(), m.fresh);
            let gal = ui.painter().layout(m.name.clone(), FontId::new(12.5, FontFamily::Name("bold".into())), s.ink, w - 2.0);
            let gal = if gal.rows.len() > 2 {
                let mut job = egui::text::LayoutJob::simple(m.name.clone(), FontId::new(12.0, FontFamily::Name("bold".into())), s.ink, w - 2.0);
                job.wrap.max_rows = 2;
                ui.painter().layout_job(job)
            } else {
                gal
            };
            // (each row centred)
            let y0 = er.max.y + 4.0;
            let gx = cell.center().x - gal.size().x / 2.0;
            ui.painter().galley(pos2(gx, y0), gal, s.ink);
        }
        resp.tip(|| (m.card)());
    }
}

/// A plate-button with an icon and words; greyed out when it cannot be pressed.
pub fn act_button(ui: &mut Ui, text: &str, icon: &str, enabled: bool, on: bool) -> Response {
    let (ch, s) = chrome::dress(ui);
    let icons = chrome::icons(ui.ctx());
    let ink = if on { s.on_accent() } else { s.ink };
    chrome::press(ui, &ch, &s, chrome::Press { text, icon: Some(super::look::icon(&icons, icon, 18.0, ink)), size: vec2(0.0, 28.0), on, enabled, font: 13.5, ..chrome::Press::default() })
}

/// Words in a size and colour, wrapped to the room.
pub fn words(ui: &mut Ui, text: impl Into<String>, size: f32, colour: Color32) -> Response {
    ui.add(egui::Label::new(egui::RichText::new(text.into()).size(size).color(colour)).wrap())
}

/// Words in the bold letters.
pub fn bold(ui: &mut Ui, text: impl Into<String>, size: f32, colour: Color32) -> Response {
    ui.add(egui::Label::new(egui::RichText::new(text.into()).size(size).color(colour).family(FontFamily::Name("bold".into()))).wrap())
}

/// A share (0..1) in a word: none, a little, a part, half, most, nearly all, all.
pub fn share(x: f32) -> &'static str {
    match x {
        x if x <= 0.005 => "нет",
        x if x < 0.15 => "малая часть",
        x if x < 0.4 => "заметная часть",
        x if x < 0.6 => "половина",
        x if x < 0.9 => "большая часть",
        x if x < 0.995 => "почти все",
        _ => "все",
    }
}

/// How well something goes (0..1), in a word.
pub fn grade(x: f32) -> &'static str {
    match x {
        x if x < 0.3 => "плохо",
        x if x < 0.55 => "слабо",
        x if x < 0.75 => "сносно",
        x if x < 0.9 => "хорошо",
        _ => "отлично",
    }
}

/// A colour of words on the paper for a tone (darkened till it reads).
pub fn ink(s: &Skin, base: Color32) -> Color32 {
    super::panels::ink_of(s, base)
}

/// One side of a head-to-head card (33rd session).
pub struct Side {
    pub country: usize,
    pub name: String,
    pub colour: Color32,
}

/// A row of a head-to-head card: what is compared, the two figures (their shares make the bar's
/// split) and the words at the bar's ends.
pub struct VsRow {
    pub label: String,
    pub a: f32,
    pub b: f32,
    pub a_text: String,
    pub b_text: String,
}

impl VsRow {
    pub fn new(label: &str, a: f32, b: f32, a_text: impl Into<String>, b_text: impl Into<String>) -> VsRow {
        VsRow { label: label.into(), a, b, a_text: a_text.into(), b_text: b_text.into() }
    }
}

/// Two countries face to face: their flags at the ends of a card, crossed swords between them,
/// then a bar for each thing compared — the two figures at its ends, the split of the bar by their
/// shares, each side in its country's colour (the second turned red where the two colours are alike),
/// the leading figure bold. `dark`: on a dark plate that is already there (over the map); else on paper (in a folder).
pub fn versus(ui: &mut Ui, a: &Side, b: &Side, rows: &[VsRow], dark: bool) -> Response {
    let (ch, s) = chrome::dress(ui);
    let icons = chrome::icons(ui.ctx());
    let w = ui.available_width();
    let (head_h, row_h) = (74.0, 38.0);
    let (r, resp) = ui.allocate_exact_size(vec2(w, head_h + rows.len() as f32 * row_h + 6.0), Sense::hover());
    if !ui.is_rect_visible(r) {
        return resp;
    }
    let (ink, soft) = if dark { (s.desk_ink, s.desk_ink.gamma_multiply(0.78)) } else { (s.ink, s.soft) };
    let mut v = Out::new();
    if dark {
        // (on a dark plate already: the card of the war over the map has its own)
    } else {
        leaf_at(&mut v, &ch, &s, r, None, false);
    }
    ui.painter().extend(v);
    let dist = |p: Color32, q: Color32| ((p.r() as f32 - q.r() as f32).powi(2) + (p.g() as f32 - q.g() as f32).powi(2) + (p.b() as f32 - q.b() as f32).powi(2)).sqrt();
    let (ca, cb) = (a.colour, if dist(a.colour, b.colour) < 80.0 { Color32::from_rgb(190, 60, 48) } else { b.colour });
    // the flags, each in a frame of its colour, and under them the names
    let fsize = vec2(58.0, 37.0);
    let pad = 14.0;
    for (side, left) in [(a, true), (b, false)] {
        let at = if left { pos2(r.min.x + pad, r.min.y + 10.0) } else { pos2(r.max.x - pad - fsize.x, r.min.y + 10.0) };
        let fr = Rect::from_min_size(at, fsize);
        let col = if left { ca } else { cb };
        let mut v = Out::new();
        v.push(epaint::Shadow { offset: vec2(0.0, 2.0), blur: 5.0, spread: 0.0, color: Color32::from_black_alpha(110) }.as_shape(fr.expand(2.0), 2.0).into());
        v.push(Shape::rect_filled(fr.expand(2.5), 2.0, col));
        v.push(Shape::rect_stroke(fr.expand(2.5), 2.0, Stroke::new(1.0, Color32::from_black_alpha(170))));
        ui.painter().extend(v);
        if let Some(f) = super::flags::get(ui.ctx(), side.country) {
            super::flags::paint(ui.ctx(), ui.painter(), fr, &f, ch.trim, Some(&icons));
        }
        let mut job = egui::text::LayoutJob::simple_singleline(side.name.to_uppercase(), FontId::new(14.0, FontFamily::Name("head".into())), ink);
        job.wrap = egui::text::TextWrapping::truncate_at_width(w / 2.0 - 22.0);
        let gal = ui.painter().layout_job(job);
        let x = if left { fr.min.x } else { fr.max.x - gal.size().x };
        ui.painter().galley(pos2(x, fr.max.y + 7.0), gal, ink);
    }
    let mid = pos2(r.center().x, r.min.y + 10.0 + fsize.y / 2.0);
    let sw = if dark { 30.0 } else { 38.0 };
    super::look::icon(&icons, "clash", sw, Color32::from_black_alpha(150)).paint_at(ui, Rect::from_center_size(mid + vec2(1.0, 1.5), vec2(sw, sw)));
    super::look::icon(&icons, "clash", sw, if dark { hud::lighten(ch.trim, 0.2) } else { s.accent }).paint_at(ui, Rect::from_center_size(mid, vec2(sw, sw)));
    // the bars
    let mut y = r.min.y + head_h;
    for row in rows {
        let (x0, x1) = (r.min.x + pad, r.max.x - pad);
        let total = (row.a.max(0.0) + row.b.max(0.0)).max(1e-9);
        let share = if row.a.max(0.0) + row.b.max(0.0) <= 0.0 { 0.5 } else { row.a.max(0.0) / total };
        let lead_a = row.a > row.b * 1.001;
        let lead_b = row.b > row.a * 1.001;
        let face = |lead: bool| if lead { FontId::new(13.5, FontFamily::Name("bold".into())) } else { FontId::proportional(13.5) };
        let (fa, fb) = (face(lead_a), face(lead_b));
        let p = ui.painter();
        p.text(pos2(x0, y + 1.0), egui::Align2::LEFT_TOP, &row.a_text, fa, if lead_a { ink } else { soft });
        p.text(pos2(x1, y + 1.0), egui::Align2::RIGHT_TOP, &row.b_text, fb, if lead_b { ink } else { soft });
        p.text(pos2(r.center().x, y + 3.0), egui::Align2::CENTER_TOP, row.label.to_uppercase(), FontId::new(12.0, FontFamily::Name("head".into())), soft);
        let bar = Rect::from_min_max(pos2(x0, y + 20.0), pos2(x1, y + 31.0));
        let mut v = Out::new();
        v.push(Shape::rect_filled(bar, 2.0, Color32::from_black_alpha(if dark { 130 } else { 90 })));
        ch.bevel(&mut v, bar, false, 1.0);
        let inner = bar.shrink(1.5);
        let cut = inner.min.x + inner.width() * share.clamp(0.0, 1.0);
        for (seg, col) in [(Rect::from_min_max(inner.min, pos2(cut, inner.max.y)), ca), (Rect::from_min_max(pos2(cut, inner.min.y), inner.max), cb)] {
            if seg.width() > 0.5 {
                let mut m = epaint::Mesh::default();
                let (top, bot) = (hud::lighten(col, 0.25), hud::darken(col, 0.22));
                for (p, c) in [(seg.left_top(), top), (seg.right_top(), top), (seg.right_bottom(), bot), (seg.left_bottom(), bot)] {
                    m.colored_vertex(p, c);
                }
                m.add_triangle(0, 1, 2);
                m.add_triangle(0, 2, 3);
                v.push(Shape::mesh(m));
            }
        }
        v.push(Shape::line_segment([pos2(cut, bar.min.y - 2.0), pos2(cut, bar.max.y + 2.0)], Stroke::new(2.0, Color32::from_black_alpha(200))));
        v.push(Shape::line_segment([pos2(cut, bar.min.y - 2.0), pos2(cut, bar.max.y + 2.0)], Stroke::new(1.0, if dark { ch.trim } else { Color32::from_rgb(250, 244, 226) })));
        let mid = bar.center().x;
        v.push(Shape::line_segment([pos2(mid, bar.max.y - 3.0), pos2(mid, bar.max.y)], Stroke::new(1.0, Color32::from_black_alpha(160))));
        v.push(Shape::rect_stroke(bar, 2.0, Stroke::new(1.0, Color32::from_black_alpha(160))));
        ui.painter().extend(v);
        y += row_h;
    }
    resp
}
