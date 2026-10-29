//! The pieces the folders are built of, in the manner of HOI4's and Victoria's screens: a ribbon of
//! dark metal heading each part of a folder (an icon in a socket at its head, the words in the
//! era's letters, a pennant's tail); cards of paper raised off the sheet with a band of colour at
//! their edge; glossy gauges sunk into the paper with ticks at the quarters; tiles of the figures
//! that matter (an icon, a small heading, a word in its tone) on dark plates; emblems of the
//! country's features — a square of colour by how good the feature is, framed in trim, its icon,
//! its strength along its foot, an arrow while it grows or fades.

use eframe::egui::{self, epaint, pos2, vec2, Color32, FontFamily, FontId, Rect, Response, Sense, Shape, Stroke, Ui};

use super::chrome::{self, Card, Out, Tip};
use super::hud;
use super::look::Skin;

/// Colours of tone: good, bad, mixed, plain (on dark plates and in emblems).
pub const GOOD: Color32 = Color32::from_rgb(78, 142, 64);
pub const BAD: Color32 = Color32::from_rgb(176, 58, 42);
pub const MIXED: Color32 = Color32::from_rgb(190, 142, 52);
pub const PLAIN: Color32 = Color32::from_rgb(92, 104, 112);

/// A ribbon heading a part of a folder: dark metal across the folder, trim above and below, an
/// icon in a round socket at its head, the words in the era's heading letters, a pennant's tail at
/// its end.
pub fn ribbon(ui: &mut Ui, text: &str, icon: Option<&str>) {
    let (ch, s) = chrome::dress(ui);
    ui.add_space(10.0);
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 28.0), Sense::hover());
    if !ui.is_rect_visible(r) {
        ui.add_space(4.0);
        return;
    }
    let body = Rect::from_min_max(r.min + vec2(if icon.is_some() { 12.0 } else { 0.0 }, 2.0), r.max - vec2(0.0, 2.0));
    let tail = 12.0;
    let mut v = Out::new();
    // (its points: the band's corners, the pennant's tail cut into its right end)
    let p = [body.left_top(), pos2(body.max.x, body.min.y), pos2(body.max.x - tail, body.center().y), pos2(body.max.x, body.max.y), body.left_bottom()];
    let band = Rect::from_min_max(body.min, pos2(body.max.x - tail, body.max.y));
    v.push(Shape::rect_filled(band.translate(vec2(0.0, 2.5)), 0.0, Color32::from_black_alpha(70)));
    // (the band: the bar's metal, darkened, with a sheen)
    ch.lay(&mut v, band, &ch.bar, 0.0);
    v.push(Shape::rect_filled(band, 0.0, hud::darken(s.desk, 0.1).gamma_multiply(0.82)));
    hud::gloss(&mut v, band, Color32::from_white_alpha(22), Color32::from_black_alpha(60));
    // (the tail: two triangles of the same dark metal)
    let (top, bot) = (hud::lighten(s.desk, 0.05), hud::darken(s.desk, 0.3));
    let mut m = epaint::Mesh::default();
    m.colored_vertex(band.right_top(), top);
    m.colored_vertex(p[1], top);
    m.colored_vertex(p[2], hud::lerp(top, bot, 0.5));
    m.colored_vertex(p[3], bot);
    m.colored_vertex(band.right_bottom(), bot);
    m.add_triangle(0, 1, 2);
    m.add_triangle(2, 3, 4);
    v.push(Shape::mesh(m));
    v.push(Shape::line_segment([p[0], p[1]], Stroke::new(1.4, ch.trim)));
    v.push(Shape::line_segment([p[4], p[3]], Stroke::new(1.4, hud::darken(ch.trim, 0.2))));
    v.push(Shape::line_segment([p[1], p[2]], Stroke::new(1.2, ch.trim)));
    v.push(Shape::line_segment([p[2], p[3]], Stroke::new(1.2, hud::darken(ch.trim, 0.2))));
    ui.painter().extend(v);
    let mut x = body.min.x + 10.0;
    if let Some(icon) = icon {
        hud::seal(ui, &ch, pos2(r.min.x + 13.0, r.center().y), 13.0, icon, ch.trim);
        x = r.min.x + 34.0;
    }
    let gal = ui.painter().layout_no_wrap(text.to_uppercase(), FontId::new(15.0, FontFamily::Name("head".into())), s.desk_ink);
    let y = r.center().y - gal.size().y / 2.0;
    ui.painter().galley(pos2(x + 1.0, y + 1.0), gal.clone(), Color32::from_black_alpha(200));
    ui.painter().galley(pos2(x, y), gal, s.desk_ink);
    ui.add_space(5.0);
}

/// A card of paper raised off the sheet: its contents with a margin, a band of `edge` colour down
/// its left side (none: plain), lit (a little lighter) when `lit`. The response is the card's.
pub fn framed<R>(ui: &mut Ui, edge: Option<Color32>, lit: bool, add: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    let (ch, s) = chrome::dress(ui);
    let back = chrome::hold(ui);
    let left = if edge.is_some() { 12.0 } else { 9.0 };
    let inner = egui::Frame::none().inner_margin(egui::Margin { left, right: 9.0, top: 7.0, bottom: 7.0 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui)
    });
    let r = inner.response.rect;
    let mut v = Out::new();
    v.push(epaint::Shadow { offset: vec2(0.0, 2.0), blur: 6.0, spread: 0.0, color: Color32::from_black_alpha(70) }.as_shape(r, 2.0).into());
    ch.lay(&mut v, r, &ch.paper, 2.0);
    ch.gloss(&mut v, r, Color32::from_white_alpha(if lit { 70 } else { 44 }), Color32::from_black_alpha(26));
    ch.bevel(&mut v, r, true, 1.2);
    v.push(Shape::rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_black_alpha(100))));
    if let Some(c) = edge {
        let band = Rect::from_min_size(r.min + vec2(1.0, 1.0), vec2(5.0, r.height() - 2.0));
        v.push(Shape::rect_filled(band, 1.0, c));
        hud::gloss(&mut v, band, Color32::from_white_alpha(60), Color32::from_black_alpha(60));
    }
    let _ = s;
    chrome::under(ui, back, v);
    ui.add_space(4.0);
    egui::InnerResponse { inner: inner.inner, response: ui.interact(r, ui.id().with(("framed", r.min.x as i32, r.min.y as i32)), Sense::hover()) }
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
        let tw = w - 38.0;
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
    let h = 44.0;
    let rows = items.len().div_ceil(cols);
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), rows as f32 * (h + gap) - gap), Sense::hover());
    let mut out = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let r = Rect::from_min_size(area.min + vec2((i % cols) as f32 * (w + gap), (i / cols) as f32 * (h + gap)), vec2(w, h));
        let resp = ui.interact(r, ui.id().with(("stat", i, it.title.as_str())), Sense::hover());
        let mut v = Out::new();
        v.push(epaint::Shadow { offset: vec2(0.0, 2.0), blur: 5.0, spread: 0.0, color: Color32::from_black_alpha(90) }.as_shape(r, 2.0).into());
        ch.lay(&mut v, r, &ch.bar, 2.0);
        v.push(Shape::rect_filled(r, 2.0, Color32::from_black_alpha(if resp.hovered() { 70 } else { 105 })));
        hud::gloss(&mut v, r, Color32::from_white_alpha(22), Color32::from_black_alpha(60));
        ch.bevel(&mut v, r, true, 1.2);
        v.push(Shape::rect_stroke(r, 2.0, Stroke::new(1.0, ch.trim.gamma_multiply(0.75))));
        let foot = Rect::from_min_max(pos2(r.min.x + 2.0, r.max.y - 4.0), pos2(r.max.x - 2.0, r.max.y - 2.0));
        v.push(Shape::rect_filled(foot, 0.5, it.tone));
        ui.painter().extend(v);
        let ir = Rect::from_center_size(pos2(r.min.x + 18.0, r.center().y - 1.0), vec2(22.0, 22.0));
        super::look::icon(&icons, &it.icon, 22.0, Color32::from_black_alpha(200)).paint_at(ui, ir.translate(vec2(1.0, 1.5)));
        super::look::icon(&icons, &it.icon, 22.0, ch.trim).paint_at(ui, ir);
        let x = r.min.x + 34.0;
        let tw = w - 38.0;
        let mut job = egui::text::LayoutJob::simple_singleline(it.title.to_uppercase(), FontId::new(12.0, FontFamily::Name("head".into())), s.desk_ink.gamma_multiply(0.78));
        job.wrap = egui::text::TextWrapping::truncate_at_width(tw);
        let g1 = ui.painter().layout_job(job);
        ui.painter().galley(pos2(x, r.min.y + 5.0), g1, s.desk_ink);
        let val_col = hud::lerp(s.desk_ink, hud::lighten(it.tone, 0.35), 0.45);
        match &it.num {
            // (the number large; the word after it small and softer, if it fits)
            Some(n) => {
                let gn = ui.painter().layout_no_wrap(n.clone(), FontId::new(16.0, FontFamily::Name("bold".into())), val_col);
                let nw = gn.size().x;
                ui.painter().galley(pos2(x, r.min.y + 18.5), gn, val_col);
                // (the word after the number only whole: a word cut short — «пол…» — says nothing, and
                // the card of the tile has it)
                let room = tw - nw - 6.0;
                // (29th session: the word whole or not at all — «пол...» cut in the middle read as a fault;
                // the card of the tile says it in full)
                if room > 24.0 && !it.value.is_empty() {
                    let gw = ui.painter().layout_no_wrap(it.value.clone(), FontId::new(12.5, FontFamily::Proportional), s.desk_ink.gamma_multiply(0.8));
                    if gw.size().x <= room {
                        ui.painter().galley(pos2(x + nw + 6.0, r.min.y + 21.5), gw, s.desk_ink);
                    }
                }
            }
            None => {
                let mut job = egui::text::LayoutJob::simple_singleline(it.value.clone(), FontId::new(14.5, FontFamily::Name("bold".into())), val_col);
                job.wrap = egui::text::TextWrapping::truncate_at_width(tw);
                let g2 = ui.painter().layout_job(job);
                ui.painter().galley(pos2(x, r.min.y + 20.0), g2, val_col);
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
        v.push(epaint::Shadow { offset: vec2(0.0, 2.0), blur: 6.0, spread: 0.0, color: Color32::from_black_alpha(70) }.as_shape(r, 2.0).into());
        ch.lay(&mut v, r, &ch.paper, 2.0);
        ch.gloss(&mut v, r, Color32::from_white_alpha(44), Color32::from_black_alpha(26));
        ch.bevel(&mut v, r, true, 1.2);
        v.push(Shape::rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_black_alpha(100))));
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
