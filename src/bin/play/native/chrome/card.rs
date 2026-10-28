//! Tooltip cards of the era (`Card`): what a card holds, when it shows (after a short rest of the pointer, fading in), how it is laid out — title in the era's font, a sign, rows with values in a column — and where it stands (beside the thing hovered, never over it or the pointer, always on the screen).

use super::*;

/// What a tooltip says: a title in the era's heading letters with a mark beside it (an icon, a
/// flag, a portrait), a line under the title, then lines — plain text (wrapped), rows of
/// a name and a value (the values in a column on the right), small headings, notes, rules.
#[derive(Clone, Default)]
pub struct Card {
    pub title: String,
    pub sub: String,
    pub mark: Mark,
    pub lines: Vec<Line>,
}

#[derive(Clone, Default)]
pub enum Mark {
    #[default]
    None,
    Icon(String),
    Flag(Color32),
    /// A country's own flag (by its number; the colour if the flags are not made yet).
    Banner(usize, Color32),
    Face(String),
}

#[derive(Clone)]
pub enum Line {
    Text(String),
    Row(String, String, Tone),
    Head(String),
    Note(String),
    Rule,
}

/// How a row's value reads: plainly, as good news, as bad.
#[derive(Clone, Copy, PartialEq)]
pub enum Tone {
    Plain,
    Good,
    Bad,
}

impl Card {
    pub fn new(title: impl Into<String>) -> Card {
        Card { title: title.into(), ..Default::default() }
    }

    /// A title and words (each line of them a line of the card).
    pub fn plain(title: impl Into<String>, text: &str) -> Card {
        let mut c = Card::new(title);
        for l in text.lines().filter(|l| !l.trim().is_empty()) {
            c = c.text(l);
        }
        c
    }

    pub fn sub(mut self, s: impl Into<String>) -> Card {
        self.sub = s.into();
        self
    }

    pub fn icon(mut self, name: impl Into<String>) -> Card {
        self.mark = Mark::Icon(name.into());
        self
    }

    pub fn flag(mut self, c: Color32) -> Card {
        self.mark = Mark::Flag(c);
        self
    }

    /// A country's own flag as the card's mark.
    pub fn banner(mut self, country: usize, c: Color32) -> Card {
        self.mark = Mark::Banner(country, c);
        self
    }

    pub fn face(mut self, file: impl Into<String>) -> Card {
        let f = file.into();
        if !f.is_empty() {
            self.mark = Mark::Face(f);
        }
        self
    }

    pub fn text(mut self, s: impl Into<String>) -> Card {
        let s = s.into();
        if !s.trim().is_empty() {
            self.lines.push(Line::Text(s));
        }
        self
    }

    pub fn row(self, k: impl Into<String>, v: impl Into<String>) -> Card {
        self.tone(k, v, Tone::Plain)
    }

    pub fn bad(self, k: impl Into<String>, v: impl Into<String>) -> Card {
        self.tone(k, v, Tone::Bad)
    }

    pub fn tone(mut self, k: impl Into<String>, v: impl Into<String>, t: Tone) -> Card {
        self.lines.push(Line::Row(k.into(), v.into(), t));
        self
    }

    pub fn head(mut self, s: impl Into<String>) -> Card {
        self.lines.push(Line::Head(s.into()));
        self
    }

    pub fn note(mut self, s: impl Into<String>) -> Card {
        let s = s.into();
        if !s.trim().is_empty() {
            self.lines.push(Line::Note(s));
        }
        self
    }

    pub fn rule(mut self) -> Card {
        self.lines.push(Line::Rule);
        self
    }
}

/// How long the hand rests before a card comes, and how long it takes to come (seconds).
pub const TIP_WAIT: f64 = 0.22;

/// How long the hand rests before a card comes (the settings may make it quicker or slower).
pub fn tip_wait(ctx: &egui::Context) -> f64 {
    ctx.data(|d| d.get_temp::<f64>(egui::Id::new("tip-wait"))).unwrap_or(TIP_WAIT)
}

pub const TIP_FADE: f64 = 0.16;

/// What the hand rests on (and since when), the pass it was seen in, the pass a card was claimed in.
#[derive(Clone, Copy)]
struct Rest {
    id: egui::Id,
    since: f64,
    pass: u64,
    claimed: u64,
}

/// The card shown this frame (for the window's self-checks): where it lies, what the hand rests
/// on, how far it has come (0..1).
#[derive(Clone)]
pub struct Shown {
    pub rect: Rect,
    pub anchor: Rect,
    pub alpha: f32,
    pub pass: u64,
}

/// The card of the frame just drawn (for the window's self-checks, after the frame).
pub fn shown(ctx: &egui::Context) -> Option<Shown> {
    let pass = ctx.cumulative_pass_nr();
    let s = ctx.data(|d| d.get_temp::<Shown>(egui::Id::new("tip-shown")))?;
    (s.pass + 1 == pass).then_some(s)
}

/// The card of what the hand rests on — the one way every tooltip of the window is shown: it comes
/// when the hand has rested `TIP_WAIT` on the thing (`id`, lying at `anchor`), fading in and rising
/// into place; it lies beside the thing — below it, else above, else to a side — never over it nor
/// over the hand, and always whole on the screen. While a button is held no card shows. Of things
/// inside one another the innermost speaks (it asks first).
pub fn tip_at(ctx: &egui::Context, id: egui::Id, anchor: Rect, card: impl FnOnce() -> Card) {
    let (now, pointer, down) = ctx.input(|i| (i.time, i.pointer.hover_pos(), i.pointer.any_down()));
    let Some(pointer) = pointer else { return };
    let pass = ctx.cumulative_pass_nr();
    let key = egui::Id::new("tip-rest");
    let mut r = ctx.data(|d| d.get_temp::<Rest>(key)).unwrap_or(Rest { id, since: now, pass, claimed: u64::MAX });
    if r.claimed == pass {
        return;
    }
    if r.id != id || r.pass + 1 < pass || down {
        r.id = id;
        r.since = now;
    }
    r.pass = pass;
    r.claimed = pass;
    ctx.data_mut(|d| d.insert_temp(key, r));
    if down {
        return;
    }
    let t = now - r.since - tip_wait(ctx);
    if t < 0.0 {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(-t + 0.005));
        return;
    }
    let k = (t / TIP_FADE).min(1.0) as f32;
    let a = 1.0 - (1.0 - k).powi(3);
    if a < 1.0 {
        ctx.request_repaint();
    }
    let c = card();
    let rect = card_show(ctx, &c, anchor, pointer, a);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("tip-shown"), Shown { rect, anchor, alpha: a, pass }));
}

/// Tooltips on what a response is: `.tip(|| Card…)`.
pub trait Tip: Sized {
    fn tip(self, card: impl FnOnce() -> Card) -> Self;
}

impl Tip for Response {
    fn tip(self, card: impl FnOnce() -> Card) -> Response {
        if self.hovered() {
            tip_at(&self.ctx, self.id, self.rect, card);
        }
        self
    }
}

/// A card laid out: its pieces placed from its top left corner, its size.
struct Laid {
    size: egui::Vec2,
    mark: Option<Rect>,
    pieces: Vec<(egui::Pos2, std::sync::Arc<egui::Galley>)>,
    rules: Vec<f32>,
    leaders: Vec<(f32, f32, f32)>,
}

fn lay_card(ctx: &egui::Context, c: &Card, skin: &Skin, paper: Color32) -> Laid {
    let spoil = super::super::spoiled();
    let pad = 12.0;
    let head = egui::FontFamily::Name("head".into());
    let bold = egui::FontFamily::Name("bold".into());
    let (body, small) = if spoil { (10.0, 9.0) } else { (14.5, 13.0) };
    let ink = if spoil { paper.gamma_multiply(0.82) } else { skin.ink };
    let soft = if spoil { paper.gamma_multiply(0.85) } else { legible(paper, &[skin.soft, skin.ink]) };
    let light = paper.r() as u32 + paper.g() as u32 + paper.b() as u32 > 380;
    let good = if light { [Color32::from_rgb(0x2c, 0x6a, 0x26), Color32::from_rgb(0x1d, 0x4a, 0x18)] } else { [Color32::from_rgb(0x9f, 0xd8, 0x86), Color32::from_rgb(0xc8, 0xf0, 0xb0)] };
    let good = legible(paper, &good);
    let bad = if light { [skin.accent, Color32::from_rgb(0x8a, 0x12, 0x0c)] } else { [Color32::from_rgb(0xff, 0x9c, 0x7c), Color32::from_rgb(0xff, 0xc8, 0xb0)] };
    let bad = legible(paper, &bad);
    let mark = match c.mark {
        Mark::None => egui::Vec2::ZERO,
        Mark::Face(_) => egui::vec2(32.0, 40.0),
        Mark::Flag(_) | Mark::Banner(..) => egui::vec2(38.0, 25.0),
        Mark::Icon(_) => egui::vec2(32.0, 32.0),
    };
    let gal = |text: &str, size: f32, family: egui::FontFamily, colour: Color32, wrap: f32| ctx.fonts(|f| f.layout(text.to_string(), egui::FontId::new(size, family), colour, wrap));
    // the widths the words would take unwrapped: the card as wide as they need, within bounds
    let title_text = c.title.to_uppercase();
    let mut natural = gal(&title_text, 18.0, head.clone(), ink, f32::INFINITY).size().x + if mark.x > 0.0 { mark.x + 10.0 } else { 0.0 };
    natural = natural.max(gal(&c.sub, small, egui::FontFamily::Proportional, soft, f32::INFINITY).size().x + if mark.x > 0.0 { mark.x + 10.0 } else { 0.0 });
    for l in &c.lines {
        let w = match l {
            Line::Text(t) => gal(t, body, egui::FontFamily::Proportional, ink, f32::INFINITY).size().x,
            Line::Note(t) => gal(t, small, egui::FontFamily::Proportional, soft, f32::INFINITY).size().x,
            Line::Head(t) => gal(&t.to_uppercase(), small, head.clone(), skin.accent, f32::INFINITY).size().x,
            Line::Row(k, v, _) => gal(k, small + 0.5, egui::FontFamily::Proportional, soft, f32::INFINITY).size().x + 24.0 + gal(v, body, bold.clone(), ink, f32::INFINITY).size().x,
            Line::Rule => 0.0,
        };
        natural = natural.max(w);
    }
    let width = (natural + 2.0 * pad).clamp(if spoil { 120.0 } else { 210.0 }, 340.0);
    let inner = width - 2.0 * pad;
    let mut pieces = Vec::new();
    let mut rules = Vec::new();
    let mut leaders = Vec::new();
    // the head: the mark, the title and the line under it beside it
    let mut y = pad;
    let tx = pad + if mark.x > 0.0 { mark.x + 10.0 } else { 0.0 };
    let title = gal(&title_text, 18.0, head.clone(), ink, inner - (tx - pad));
    let sub = (!c.sub.is_empty()).then(|| gal(&c.sub, small, egui::FontFamily::Proportional, soft, inner - (tx - pad)));
    let text_h = title.size().y + sub.as_ref().map_or(0.0, |s| s.size().y + 1.0);
    let head_h = text_h.max(mark.y);
    let ty = y + (head_h - text_h) / 2.0;
    let sub_y = ty + title.size().y + 1.0;
    pieces.push((egui::pos2(tx, ty), title));
    if let Some(s) = sub {
        pieces.push((egui::pos2(tx, sub_y), s));
    }
    let mark_rect = (mark.x > 0.0).then(|| Rect::from_min_size(egui::pos2(pad, y + (head_h - mark.y) / 2.0), mark));
    y += head_h + 5.0;
    if !c.lines.is_empty() {
        rules.push(y + 3.0);
        y += 9.0;
    }
    for l in &c.lines {
        match l {
            Line::Text(t) => {
                let g = gal(t, body, egui::FontFamily::Proportional, ink, inner);
                let h = g.size().y;
                pieces.push((egui::pos2(pad, y), g));
                y += h + 3.0;
            }
            Line::Note(t) => {
                let g = gal(t, small, egui::FontFamily::Proportional, soft, inner);
                let h = g.size().y;
                pieces.push((egui::pos2(pad, y + 1.0), g));
                y += h + 4.0;
            }
            Line::Head(t) => {
                y += 3.0;
                let g = gal(&t.to_uppercase(), small, head.clone(), legible(paper, &[skin.accent, skin.ink]), inner);
                let h = g.size().y;
                pieces.push((egui::pos2(pad, y), g));
                y += h + 2.0;
            }
            Line::Row(k, v, tone) => {
                let colour = match tone {
                    Tone::Plain => ink,
                    Tone::Good => good,
                    Tone::Bad => bad,
                };
                // (30th session: a value too long for the right-hand column — a sentence — goes under its name, the width of
                // the card and set from the left: right-aligned in a column of 60% it was a ragged stripe of broken lines)
                let natural_v = gal(v, body, bold.clone(), colour, f32::INFINITY).size().x;
                if natural_v > inner * 0.6 {
                    let kg = gal(k, small + 0.5, egui::FontFamily::Proportional, soft, inner);
                    let vg = gal(v, body, bold.clone(), colour, inner);
                    let kh = kg.size().y;
                    pieces.push((egui::pos2(pad, y), kg));
                    pieces.push((egui::pos2(pad, y + kh), vg.clone()));
                    y += kh + vg.size().y + 4.0;
                    continue;
                }
                let vg = gal(v, body, bold.clone(), colour, inner * 0.6);
                let kg = gal(k, small + 0.5, egui::FontFamily::Proportional, soft, inner - vg.size().x - 16.0);
                let h = vg.size().y.max(kg.size().y);
                let kw = kg.size().x;
                let vx = pad + inner - vg.size().x;
                // (the name and the value joined by a dotted leader, the values in a column)
                leaders.push((pad + kw + 6.0, vx - 6.0, y + h - 5.0));
                pieces.push((egui::pos2(pad, y + h - kg.size().y), kg));
                pieces.push((egui::pos2(vx, y + h - vg.size().y), vg));
                y += h + 3.0;
            }
            Line::Rule => {
                rules.push(y + 4.0);
                y += 9.0;
            }
        }
    }
    Laid { size: egui::vec2(width, y + pad - 2.0), mark: mark_rect, pieces, rules, leaders }
}

/// Where a card of `size` lies for a thing at `anchor` with the hand at `pointer`: below the thing,
/// else above, else to its right or left (the hand's own picture kept clear too), whole on the
/// screen. The spoiled control: at the hand, as it comes.
pub fn place_card(screen: Rect, size: egui::Vec2, anchor: Rect, pointer: egui::Pos2) -> egui::Pos2 {
    if super::super::spoiled() {
        return pointer + egui::vec2(10.0, 10.0);
    }
    let room = screen.shrink(6.0);
    let hand = Rect::from_min_size(pointer - egui::vec2(2.0, 2.0), egui::vec2(28.0, 32.0));
    let keep = anchor.union(hand);
    let fit_x = |x: f32| x.clamp(room.min.x, (room.max.x - size.x).max(room.min.x));
    let fit_y = |y: f32| y.clamp(room.min.y, (room.max.y - size.y).max(room.min.y));
    let tries = [
        egui::pos2(fit_x(pointer.x - 16.0), keep.max.y + 8.0),
        egui::pos2(fit_x(pointer.x - 16.0), anchor.min.y.min(pointer.y - 4.0) - 8.0 - size.y),
        egui::pos2(keep.max.x + 10.0, fit_y(pointer.y - 14.0)),
        egui::pos2(anchor.min.x.min(pointer.x - 4.0) - 10.0 - size.x, fit_y(pointer.y - 14.0)),
    ];
    for p in tries {
        let r = Rect::from_min_size(p, size);
        if room.contains_rect(r) && !r.intersects(keep) {
            return p;
        }
    }
    egui::pos2(fit_x(pointer.x + 24.0), fit_y(keep.max.y + 8.0))
}

/// A card on the screen, `a` of the way in: a sheet in a frame with its shadow, the head, the
/// lines. Where it lies.
fn card_show(ctx: &egui::Context, c: &Card, anchor: Rect, pointer: egui::Pos2, a: f32) -> Rect {
    let Some(ch) = of(ctx) else { return Rect::NOTHING };
    let skin = skin(ctx);
    let laid = lay_card(ctx, c, &skin, ch.paper.mean);
    let at = place_card(ctx.screen_rect(), laid.size, anchor, pointer);
    let rect = Rect::from_min_size(at, laid.size);
    let rise = (1.0 - a) * 6.0;
    let icons = icons(ctx);
    egui::Area::new(egui::Id::new("tip-card"))
        .order(egui::Order::Tooltip)
        .fixed_pos(at + egui::vec2(0.0, rise))
        .interactable(false)
        .fade_in(false)
        .constrain(false)
        .show(ctx, |ui| {
            ui.set_opacity(a);
            let (r, _) = ui.allocate_exact_size(laid.size, Sense::hover());
            let mut v = Out::new();
            v.push(epaint::Shadow { offset: vec2(0.0, 5.0), blur: 16.0, spread: 1.0, color: Color32::from_black_alpha(150) }.as_shape(r, 4.0).into());
            ch.lay(&mut v, r, &ch.frame, 3.0);
            ch.bevel(&mut v, r, true, 1.5);
            let inner = r.shrink(4.0);
            ch.lay(&mut v, inner, &ch.paper, 2.0);
            ch.bevel(&mut v, inner, false, 1.0);
            v.push(Shape::rect_stroke(inner.shrink(2.5), 1.5, Stroke::new(1.0, ch.trim.gamma_multiply(0.7))));
            for &y in &laid.rules {
                let y = r.min.y + y;
                v.push(Shape::line_segment([pos2(r.min.x + 12.0, y), pos2(r.max.x - 12.0, y)], Stroke::new(1.0, ch.trim)));
                let c = pos2(r.min.x + 22.0, y);
                v.push(Shape::convex_polygon(vec![c + vec2(-4.0, 0.0), c + vec2(0.0, -2.8), c + vec2(4.0, 0.0), c + vec2(0.0, 2.8)], ch.trim, Stroke::NONE));
            }
            for &(x0, x1, y) in &laid.leaders {
                let mut x = r.min.x + x0;
                while x < r.min.x + x1 {
                    v.push(Shape::circle_filled(pos2(x, r.min.y + y), 0.7, skin.soft.gamma_multiply(0.6)));
                    x += 4.0;
                }
            }
            // the mark: an icon in a socket of trim, a flag, a portrait in a frame
            if let Some(m) = laid.mark {
                let m = m.translate(r.min.to_vec2());
                match &c.mark {
                    Mark::Icon(_) => {
                        v.push(Shape::circle_filled(m.center(), m.width() / 2.0, skin.desk));
                        v.push(Shape::circle_stroke(m.center(), m.width() / 2.0, Stroke::new(1.5, ch.trim)));
                    }
                    Mark::Flag(col) => {
                        v.push(Shape::rect_filled(m.expand(1.5), 1.5, Color32::from_black_alpha(220)));
                        v.push(Shape::rect_filled(m, 1.0, *col));
                        v.push(Shape::rect_filled(Rect::from_min_size(m.min + vec2(0.0, m.height() * 0.37), vec2(m.width(), m.height() * 0.26)), 0.0, Color32::from_rgb(240, 236, 226)));
                        ch.gloss(&mut v, m, Color32::from_white_alpha(50), Color32::from_black_alpha(60));
                        v.push(Shape::rect_stroke(m.expand(1.5), 1.5, Stroke::new(1.2, ch.trim)));
                    }
                    Mark::Banner(country, col) => match super::super::flags::get(ctx, *country) {
                        Some(f) => {
                            let painter = ui.painter().clone();
                            painter.extend(std::mem::take(&mut v));
                            super::super::flags::paint(ctx, &painter, m, &f, ch.trim, Some(&icons));
                        }
                        None => {
                            v.push(Shape::rect_filled(m, 1.0, *col));
                            v.push(Shape::rect_stroke(m.expand(1.5), 1.5, Stroke::new(1.2, ch.trim)));
                        }
                    },
                    Mark::Face(_) => {
                        v.push(Shape::rect_filled(m.expand(2.0), 2.0, Color32::from_black_alpha(200)));
                    }
                    Mark::None => {}
                }
                ui.painter().extend(v);
                match &c.mark {
                    Mark::Icon(name) => super::super::look::icon(&icons, name, m.width() * 0.62, skin.desk_ink).paint_at(ui, Rect::from_center_size(m.center(), m.size() * 0.62)),
                    Mark::Face(f) => {
                        if let Some(img) = super::super::look::portrait(f, m.width()) {
                            img.paint_at(ui, m);
                        }
                        ui.painter().rect_stroke(m.expand(1.0), 2.0, Stroke::new(1.5, ch.trim));
                    }
                    _ => {}
                }
            } else {
                ui.painter().extend(v);
            }
            for (p, g) in &laid.pieces {
                ui.painter().galley(r.min + p.to_vec2(), g.clone(), skin.ink);
            }
        });
    rect
}
