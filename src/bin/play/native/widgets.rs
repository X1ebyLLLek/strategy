//! The widgets of the window in the era's dress (egui's own are plain boxes): a slider with a sunk
//! groove, the rail filled up to a knob of trim and a notch where the usual value is; a field to
//! write in, sunk into the paper; the scroll bar as a groove with a plate; bars of shares in a
//! frame with the kinds' names in them; a switch of joined plates; a meter against the usual; a bar
//! of progress; small plate-buttons; a number with − and +.

use eframe::egui::{self, pos2, vec2, Color32, CursorIcon, FontFamily, FontId, Rect, Response, Sense, Shape, Stroke, Ui};

use super::chrome::{self, Chrome, Out, Press};
use super::look::Skin;

/// A groove sunk into the paper (a slider's rail, a meter, a scroll bar).
fn groove(v: &mut Out, ch: &Chrome, r: Rect) {
    v.push(Shape::rect_filled(r, 2.0, Color32::from_black_alpha(70)));
    ch.bevel(v, r, false, 1.0);
    v.push(Shape::rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_black_alpha(110))));
}

/// A knob of trim: the metal with a sheen, a bevel, a line across its middle.
fn knob(v: &mut Out, ch: &Chrome, r: Rect, hot: bool) {
    v.push(epaint_shadow(r));
    v.push(Shape::rect_filled(r, 2.0, if hot { ch.trim.linear_multiply(1.15) } else { ch.trim }));
    ch.gloss(v, r, Color32::from_white_alpha(if hot { 90 } else { 60 }), Color32::from_black_alpha(80));
    ch.bevel(v, r, true, 1.2);
    v.push(Shape::rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_black_alpha(190))));
    let c = r.center();
    for dx in [-2.5f32, 0.0, 2.5] {
        v.push(Shape::line_segment([pos2(c.x + dx, c.y - r.height() * 0.22), pos2(c.x + dx, c.y + r.height() * 0.22)], Stroke::new(1.0, Color32::from_black_alpha(110))));
    }
}

fn epaint_shadow(r: Rect) -> Shape {
    egui::epaint::Shadow { offset: vec2(0.0, 1.5), blur: 4.0, spread: 0.0, color: Color32::from_black_alpha(110) }.as_shape(r, 2.0).into()
}

/// A slider of `value` in `range`, as wide as the room; `mark` — a notch where the usual value is.
/// The decision is the caller's: on `drag_stopped()` or a click.
pub fn slider(ui: &mut Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>, mark: Option<f32>, steps: usize) -> Response {
    let (ch, s) = chrome::dress(ui);
    let w = ui.available_width().min(340.0);
    let (r, mut resp) = ui.allocate_exact_size(vec2(w, 26.0), Sense::click_and_drag());
    let (lo, hi) = (*range.start(), *range.end());
    let rail = Rect::from_min_max(pos2(r.min.x + 9.0, r.center().y - 4.0), pos2(r.max.x - 9.0, r.center().y + 4.0));
    if let Some(p) = resp.interact_pointer_pos() {
        if resp.dragged() || resp.clicked() {
            let mut t = ((p.x - rail.min.x) / rail.width()).clamp(0.0, 1.0);
            if steps > 1 {
                t = (t * (steps - 1) as f32).round() / (steps - 1) as f32;
            }
            let nv = lo + t * (hi - lo);
            if (nv - *value).abs() > 1e-6 {
                *value = nv;
                resp.mark_changed();
            }
        }
    }
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    if ui.is_rect_visible(r) {
        let t = ((*value - lo) / (hi - lo).max(1e-9)).clamp(0.0, 1.0);
        let x = rail.min.x + t * rail.width();
        let mut v = Out::new();
        groove(&mut v, &ch, rail);
        let fill = Rect::from_min_max(rail.min + vec2(1.5, 1.5), pos2(x, rail.max.y - 1.5));
        if fill.width() > 0.5 {
            v.push(Shape::rect_filled(fill, 1.5, s.accent));
            ch.gloss(&mut v, fill, Color32::from_white_alpha(70), Color32::from_black_alpha(40));
        }
        // the steps as ticks under the rail; the usual value as a notch of trim
        let ticks = if steps > 1 { steps } else { 5 };
        for i in 0..ticks {
            let tx = rail.min.x + rail.width() * i as f32 / (ticks - 1) as f32;
            v.push(Shape::line_segment([pos2(tx, rail.max.y + 3.0), pos2(tx, rail.max.y + 6.0)], Stroke::new(1.0, s.soft.gamma_multiply(0.7))));
        }
        if let Some(m) = mark {
            let mx = rail.min.x + rail.width() * ((m - lo) / (hi - lo).max(1e-9)).clamp(0.0, 1.0);
            v.push(Shape::convex_polygon(vec![pos2(mx, rail.min.y - 1.0), pos2(mx - 4.5, rail.min.y - 7.0), pos2(mx + 4.5, rail.min.y - 7.0)], ch.trim, Stroke::new(1.0, Color32::from_black_alpha(150))));
        }
        knob(&mut v, &ch, Rect::from_center_size(pos2(x, r.center().y), vec2(14.0, 22.0)), resp.hovered() || resp.dragged());
        ui.painter().extend(v);
    }
    resp
}

/// A field to write in, sunk into the paper (trimmed while it is written in).
pub fn field(ui: &mut Ui, text: &mut String, width: f32, limit: usize) -> Response {
    let (ch, s) = chrome::dress(ui);
    let back = chrome::hold(ui);
    let resp = egui::TextEdit::singleline(text)
        .char_limit(limit)
        .desired_width(width)
        .frame(false)
        .margin(egui::Margin::symmetric(7.0, 5.0))
        .text_color(s.ink)
        .font(FontId::proportional(15.0))
        .show(ui)
        .response;
    let r = resp.rect;
    let mut v = Out::new();
    ch.lay(&mut v, r, &ch.paper, 2.0);
    v.push(Shape::rect_filled(r, 2.0, Color32::from_black_alpha(22)));
    ch.bevel(&mut v, r, false, 1.5);
    let focus = resp.has_focus();
    v.push(Shape::rect_stroke(r, 2.0, Stroke::new(if focus { 1.6 } else { 1.0 }, if focus { ch.trim } else { Color32::from_black_alpha(120) })));
    chrome::under(ui, back, v);
    resp
}

/// Contents that scroll, at most `max_h` high, with the era's scroll bar on the right: a groove
/// and a plate of trim to drag (the wheel scrolls too); the contents keep clear of the bar.
pub fn scroll<R>(ui: &mut Ui, salt: &str, max_h: f32, fill: bool, add: impl FnOnce(&mut Ui) -> R) -> R {
    let (ch, s) = chrome::dress(ui);
    let gutter = 14.0;
    let width = ui.available_width();
    let mut area = egui::ScrollArea::vertical().id_salt(salt);
    // (for pictures: the folder scrolled down by so many points)
    if let Some(y) = std::env::var("SHOTSCROLL").ok().and_then(|v| v.parse::<f32>().ok()) {
        area = area.vertical_scroll_offset(y);
    }
    let out = area
        .max_height(max_h)
        .auto_shrink([false, !fill])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.set_width(width - gutter);
            add(ui)
        });
    let inner = out.inner_rect;
    let (content, view) = (out.content_size.y, inner.height());
    if content > view + 1.0 {
        let bar = Rect::from_min_max(pos2(inner.max.x - gutter + 4.0, inner.min.y + 2.0), pos2(inner.max.x, inner.max.y - 2.0));
        let len = (view / content * bar.height()).max(30.0).min(bar.height());
        let room = (bar.height() - len).max(1.0);
        let most = content - view;
        let mut state = out.state;
        let resp = ui.interact(bar, out.id.with("bar"), Sense::click_and_drag()).on_hover_cursor(CursorIcon::PointingHand);
        if resp.dragged() {
            state.offset.y = (state.offset.y + resp.drag_delta().y * most / room).clamp(0.0, most);
            state.store(ui.ctx(), out.id);
        } else if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                let top = bar.min.y + state.offset.y / most * room;
                let page = view * 0.9;
                state.offset.y = if p.y < top { state.offset.y - page } else { state.offset.y + page }.clamp(0.0, most);
                state.store(ui.ctx(), out.id);
            }
        }
        let top = bar.min.y + (state.offset.y / most).clamp(0.0, 1.0) * room;
        let mut v = Out::new();
        groove(&mut v, &ch, bar);
        let h = Rect::from_min_size(pos2(bar.min.x + 1.0, top), vec2(bar.width() - 2.0, len));
        v.push(epaint_shadow(h));
        v.push(Shape::rect_filled(h, 2.0, if resp.hovered() || resp.dragged() { ch.trim.linear_multiply(1.15) } else { ch.trim }));
        ch.gloss(&mut v, h, Color32::from_white_alpha(70), Color32::from_black_alpha(70));
        ch.bevel(&mut v, h, true, 1.0);
        v.push(Shape::rect_stroke(h, 2.0, Stroke::new(1.0, Color32::from_black_alpha(180))));
        let c = h.center();
        for dy in [-3.0f32, 0.0, 3.0] {
            v.push(Shape::line_segment([pos2(c.x - 2.5, c.y + dy), pos2(c.x + 2.5, c.y + dy)], Stroke::new(1.0, Color32::from_black_alpha(120))));
        }
        let _ = s;
        ui.painter().with_clip_rect(inner.expand(2.0)).extend(v);
    }
    out.inner
}

/// The colour of words that reads on a colour: white or the desk's dark.
pub fn ink_on(c: Color32) -> Color32 {
    let w = super::uitest::contrast(Color32::WHITE, c);
    let d = super::uitest::contrast(Color32::from_rgb(24, 20, 14), c);
    if w >= d {
        Color32::WHITE
    } else {
        Color32::from_rgb(24, 20, 14)
    }
}

/// A bar of shares in a sunk frame: each part its colour with a sheen, divided by dark lines, its
/// name inside where it fits. The response (for a card with the numbers).
pub fn shares(ui: &mut Ui, sh: &[f32], names: &[String], colours: &[Color32]) -> Response {
    let ch = chrome::dress(ui).0;
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
    if ui.is_rect_visible(r) {
        let mut v = Out::new();
        v.push(Shape::rect_filled(r.expand(1.5), 2.5, Color32::from_black_alpha(150)));
        let inner = r.shrink(1.5);
        v.push(Shape::rect_filled(inner, 1.5, Color32::from_black_alpha(60)));
        let total: f32 = sh.iter().sum::<f32>().max(1e-9);
        let mut x = inner.min.x;
        let mut labels = Vec::new();
        for (u, &part) in sh.iter().enumerate() {
            let w = inner.width() * part / total;
            if w < 0.5 {
                continue;
            }
            let seg = Rect::from_min_max(pos2(x, inner.min.y), pos2((x + w).min(inner.max.x), inner.max.y));
            let col = colours[u % colours.len().max(1)];
            v.push(Shape::rect_filled(seg, 0.0, col));
            ch.gloss(&mut v, seg, Color32::from_white_alpha(55), Color32::from_black_alpha(55));
            if x > inner.min.x + 0.5 {
                v.push(Shape::line_segment([pos2(x, inner.min.y), pos2(x, inner.max.y)], Stroke::new(1.0, Color32::from_black_alpha(170))));
            }
            if let Some(name) = names.get(u) {
                let g = ui.painter().layout_no_wrap(name.clone(), FontId::new(12.5, FontFamily::Name("bold".into())), ink_on(col));
                if g.size().x + 8.0 <= seg.width() {
                    labels.push((pos2(seg.center().x - g.size().x / 2.0, seg.center().y - g.size().y / 2.0), g, ink_on(col)));
                }
            }
            x += w;
        }
        ch.bevel(&mut v, inner, false, 1.0);
        v.push(Shape::rect_stroke(r.expand(1.5), 2.5, Stroke::new(1.0, ch.trim.gamma_multiply(0.8))));
        ui.painter().extend(v);
        for (p, g, c) in labels {
            ui.painter().galley(p, g, c);
        }
    }
    resp
}

/// The kinds' colours and names (what the bars of shares show), in as many rows as they need.
pub fn legend(ui: &mut Ui, s: &Skin, items: &[(String, Color32)]) {
    let w = ui.available_width();
    let font = FontId::proportional(13.0);
    let laid: Vec<_> = items.iter().map(|(n, c)| (ui.painter().layout_no_wrap(n.clone(), font.clone(), s.soft), *c)).collect();
    let row_h = 18.0;
    // (the rows broken by hand: a chip, its name, a gap)
    let mut rows = 1;
    let mut x = 0.0;
    for (g, _) in &laid {
        let iw = 16.0 + g.size().x;
        if x > 0.0 && x + iw > w {
            rows += 1;
            x = 0.0;
        }
        x += iw + 12.0;
    }
    let (r, _) = ui.allocate_exact_size(vec2(w, rows as f32 * row_h), Sense::hover());
    let (mut x, mut y) = (r.min.x, r.min.y);
    for (g, c) in laid {
        let iw = 16.0 + g.size().x;
        if x > r.min.x && x + iw > r.max.x {
            x = r.min.x;
            y += row_h;
        }
        let chip = Rect::from_min_size(pos2(x, y + 3.0), vec2(12.0, 12.0));
        ui.painter().rect_filled(chip, 2.0, c);
        ui.painter().rect_stroke(chip, 2.0, Stroke::new(1.0, Color32::from_black_alpha(150)));
        let gw = g.size().x;
        ui.painter().galley(pos2(x + 16.0, y + 9.0 - g.size().y / 2.0), g, s.soft);
        x += 16.0 + gw + 12.0;
    }
}

/// A switch of joined plates (one of them chosen); the one clicked.
pub fn switch(ui: &mut Ui, options: &[&str], chosen: usize) -> Option<usize> {
    let (ch, s) = chrome::dress(ui);
    let room = ui.available_width() - 2.0;
    let widest = |font: f32| options.iter().map(|o| ui.painter().layout_no_wrap(o.to_string(), egui::FontId::new(font, egui::FontFamily::Name("head".into())), s.ink).size().x).fold(0.0f32, f32::max);
    // (the plates in one row while their words fit, in a smaller hand if need be; else in two rows)
    let n = options.len().max(1);
    let (font, per_row) = if widest(14.5) + 14.0 <= room / n as f32 {
        (14.5, n)
    } else if widest(12.5) + 12.0 <= room / n as f32 {
        (12.5, n)
    } else {
        (14.0, n.div_ceil(2))
    };
    let w = (room / per_row as f32).floor();
    let mut out = None;
    for row in options.chunks(per_row).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (j, name) in row.1.iter().enumerate() {
                let k = row.0 * per_row + j;
                if chrome::press(ui, &ch, &s, Press { text: name, size: vec2(w, 30.0), on: k == chosen, font, ..Press::default() }).clicked() && k != chosen {
                    out = Some(k);
                }
            }
        });
    }
    out
}

/// A small plate-button on paper.
pub fn small(ui: &mut Ui, text: &str) -> Response {
    let (ch, s) = chrome::dress(ui);
    chrome::press(ui, &ch, &s, Press { text, size: vec2(0.0, 27.0), font: 14.0, ..Press::default() })
}

/// A number moved by − and +: its figure between two plates, `w` wide in all; −1, 0 or +1 for the plate
/// pressed, and the rectangle of the + plate (for the window's self-checks). Greyed out and deaf
/// when it may not be moved.
pub fn stepper(ui: &mut Ui, text: &str, w: f32, enabled: bool) -> (i8, Rect) {
    let (ch, s) = chrome::dress(ui);
    let (r, _) = ui.allocate_exact_size(vec2(w, 27.0), Sense::hover());
    let bw = 25.0;
    let minus = Rect::from_min_size(r.min, vec2(bw, r.height()));
    let plus = Rect::from_min_size(pos2(r.max.x - bw, r.min.y), vec2(bw, r.height()));
    let b1 = ui.put(minus, |ui: &mut Ui| chrome::press(ui, &ch, &s, Press { text: "−", size: minus.size(), font: 17.0, enabled, ..Press::default() }));
    let b2 = ui.put(plus, |ui: &mut Ui| chrome::press(ui, &ch, &s, Press { text: "+", size: plus.size(), font: 17.0, enabled, ..Press::default() }));
    let g = ui.painter().layout_no_wrap(text.to_string(), FontId::new(14.5, FontFamily::Name("bold".into())), if enabled { s.ink } else { s.soft });
    ui.painter().galley(pos2(r.center().x - g.size().x / 2.0, r.center().y - g.size().y / 2.0), g, s.ink);
    let out = if enabled && b1.clicked() {
        -1
    } else if enabled && b2.clicked() {
        1
    } else {
        0
    };
    (out, b2.rect)
}

/// A plate-button on paper, chosen (the accent) or not, that may be greyed out.
pub fn plate(ui: &mut Ui, text: &str, w: f32, on: bool, enabled: bool) -> Response {
    let (ch, s) = chrome::dress(ui);
    chrome::press(ui, &ch, &s, Press { text, size: vec2(w, 30.0), on, enabled, ..Press::default() })
}

/// A meter: a groove filled to `ratio` of the usual at its middle (a notch of trim there); the
/// fill in the accent from about the usual, else soft.
pub fn meter(ui: &mut Ui, ratio: f32, w: f32) -> Response {
    let (ch, s) = chrome::dress(ui);
    let (r, resp) = ui.allocate_exact_size(vec2(w, 12.0), Sense::hover());
    if ui.is_rect_visible(r) {
        let mut v = Out::new();
        groove(&mut v, &ch, r);
        let f = Rect::from_min_size(r.min + vec2(1.5, 1.5), vec2((r.width() - 3.0) * (ratio * 0.5).clamp(0.0, 1.0), r.height() - 3.0));
        if f.width() > 0.5 {
            v.push(Shape::rect_filled(f, 1.0, if ratio >= 0.85 { s.accent } else { s.soft }));
            ch.gloss(&mut v, f, Color32::from_white_alpha(60), Color32::from_black_alpha(40));
        }
        v.push(Shape::line_segment([pos2(r.center().x, r.min.y - 3.0), pos2(r.center().x, r.max.y + 3.0)], Stroke::new(2.0, ch.trim)));
        v.push(Shape::line_segment([pos2(r.center().x, r.min.y - 3.0), pos2(r.center().x, r.max.y + 3.0)], Stroke::new(0.8, Color32::from_black_alpha(160))));
        ui.painter().extend(v);
    }
    resp
}

/// A bar of how far something has come (0..1), filled with trim.
pub fn progress(ui: &mut Ui, done: f32, w: f32) -> Response {
    let ch = chrome::dress(ui).0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, 16.0), Sense::hover());
    let mut v = Out::new();
    groove(&mut v, &ch, r);
    let f = Rect::from_min_size(r.min + vec2(2.0, 2.0), vec2((r.width() - 4.0) * done.clamp(0.0, 1.0), r.height() - 4.0));
    if f.width() > 0.5 {
        v.push(Shape::rect_filled(f, 1.0, ch.trim));
        ch.gloss(&mut v, f, Color32::from_white_alpha(90), Color32::from_black_alpha(70));
    }
    ui.painter().extend(v);
    resp
}

/// A law: its choices side by side as tiles — an icon over the choice's name, the chosen one lit
/// in the accent and raised, the usual one of the age marked with a diamond of trim. The choice
/// clicked, if another. (Each tile's numbers are on its card: `card(k)`.)
pub fn law(ui: &mut Ui, choices: &[(&str, &str)], chosen: usize, usual: usize, card: &dyn Fn(usize) -> chrome::Card) -> Option<usize> {
    use chrome::Tip;
    let (ch, s) = chrome::dress(ui);
    let n = choices.len().max(1);
    let gap = 4.0;
    let w = ((ui.available_width() - gap * (n as f32 - 1.0)) / n as f32).floor();
    let h = 66.0;
    let icons = chrome::icons(ui.ctx());
    let mut out = None;
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (k, (name, icon)) in choices.iter().enumerate() {
            let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
            let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
            let on = k == chosen;
            if on && ui.clip_rect().contains_rect(r) {
                chrome::spot(ui.ctx(), "slider", r);
            }
            let hot = resp.hovered();
            let r2 = if on { r.translate(vec2(0.0, -1.5)) } else { r };
            let mut v = Out::new();
            if on {
                v.push(egui::epaint::Shadow { offset: vec2(0.0, 2.0), blur: 8.0, spread: 0.0, color: Color32::from_black_alpha(120) }.as_shape(r2, 3.0).into());
            }
            ch.plate(&mut v, r2, &s, false, on, hot, resp.is_pointer_button_down_on());
            if k == usual {
                let c = pos2(r2.max.x - 8.0, r2.min.y + 8.0);
                v.push(Shape::convex_polygon(vec![c + vec2(-4.0, 0.0), c + vec2(0.0, -4.0), c + vec2(4.0, 0.0), c + vec2(0.0, 4.0)], ch.trim, Stroke::new(0.8, Color32::from_black_alpha(150))));
            }
            ui.painter().extend(v);
            let ink = if on { s.on_accent() } else { s.ink };
            super::look::icon(&icons, icon, 22.0, ink).paint_at(ui, Rect::from_center_size(pos2(r2.center().x, r2.min.y + 18.0), vec2(22.0, 22.0)));
            let gal = ui.painter().layout(name.to_string(), FontId::new(13.0, if on { FontFamily::Name("bold".into()) } else { FontFamily::Proportional }), ink, w - 8.0);
            let at = pos2(r2.center().x - gal.size().x / 2.0, r2.min.y + 33.0);
            ui.painter().galley(at, gal, ink);
            if resp.tip(|| card(k)).clicked() && !on {
                out = Some(k);
            }
        }
    });
    ui.add_space(2.0);
    out
}

/// A chart without numbers: lines over the months in a sunk frame (faint rules across, a tick at
/// each new year along the bottom), each line in its colour with a dot at its end; `stacked` —
/// the series one over another as bands (a whole and its parts). Under the hand a rule at the
/// month and the numbers of that month on its card (`card(k)`: the k-th month of `months`).
pub fn chart(ui: &mut Ui, series: &[(String, Color32, Vec<f32>)], months: &[u32], h: f32, stacked: bool, card: &dyn Fn(usize) -> chrome::Card) -> Response {
    use chrome::Tip;
    let (ch, s) = chrome::dress(ui);
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let n = months.len();
    let mut v = Out::new();
    v.push(Shape::rect_filled(r, 2.0, Color32::from_black_alpha(26)));
    ch.bevel(&mut v, r, false, 1.0);
    v.push(Shape::rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_black_alpha(90))));
    let inner = r.shrink2(vec2(6.0, 6.0));
    for k in 1..4 {
        let y = inner.bottom() - inner.height() * k as f32 / 4.0;
        v.push(Shape::line_segment([pos2(inner.left(), y), pos2(inner.right(), y)], Stroke::new(0.6, Color32::from_black_alpha(40))));
    }
    if n < 2 {
        ui.painter().extend(v);
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "пока мало месяцев", FontId::proportional(13.0), s.soft);
        return resp;
    }
    // (the whole at each month, if stacked)
    let total = |i: usize| -> f32 { series.iter().map(|x| x.2.get(i).copied().unwrap_or(0.0).max(0.0)).sum() };
    let top = if stacked { (0..n).map(total).fold(0.0f32, f32::max) } else { series.iter().flat_map(|x| x.2.iter().copied()).fold(0.0f32, f32::max) }.max(1e-9) * 1.08;
    let x_of = |i: usize| inner.left() + inner.width() * i as f32 / (n - 1) as f32;
    let y_of = |val: f32| inner.bottom() - inner.height() * (val / top).clamp(0.0, 1.0);
    // the years
    for (i, &m) in months.iter().enumerate() {
        if m % 12 == 0 && i > 0 {
            let x = x_of(i);
            v.push(Shape::line_segment([pos2(x, inner.bottom() - 4.0), pos2(x, inner.bottom())], Stroke::new(1.0, ch.trim)));
            v.push(Shape::line_segment([pos2(x, inner.top()), pos2(x, inner.bottom())], Stroke::new(0.5, Color32::from_black_alpha(25))));
        }
    }
    if stacked {
        let mut base = vec![0.0f32; n];
        for (_, col, vals) in series {
            let mut mesh = egui::epaint::Mesh::default();
            for i in 0..n {
                let lo = base[i];
                let hi = lo + vals.get(i).copied().unwrap_or(0.0).max(0.0);
                let x = x_of(i);
                mesh.colored_vertex(pos2(x, y_of(lo)), col.gamma_multiply(0.85));
                mesh.colored_vertex(pos2(x, y_of(hi)), *col);
                base[i] = hi;
            }
            for i in 0..(n - 1) as u32 {
                let k = i * 2;
                mesh.add_triangle(k, k + 1, k + 3);
                mesh.add_triangle(k, k + 3, k + 2);
            }
            v.push(Shape::mesh(mesh));
            let edge: Vec<egui::Pos2> = (0..n).map(|i| pos2(x_of(i), y_of(base[i]))).collect();
            v.push(Shape::line(edge, Stroke::new(1.0, Color32::from_black_alpha(90))));
        }
    } else {
        for (k, (_, col, vals)) in series.iter().enumerate() {
            let pts: Vec<egui::Pos2> = (0..n).map(|i| pos2(x_of(i), y_of(vals.get(i).copied().unwrap_or(0.0)))).collect();
            if k == 0 {
                // (a soft fill under the first line)
                let mut mesh = egui::epaint::Mesh::default();
                for p in &pts {
                    mesh.colored_vertex(*p, col.gamma_multiply(0.25));
                    mesh.colored_vertex(pos2(p.x, inner.bottom()), col.gamma_multiply(0.04));
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
            }
        }
    }
    // the month under the hand
    let mut at = None;
    if let Some(p) = resp.hover_pos() {
        let i = (((p.x - inner.left()) / inner.width()) * (n - 1) as f32).round().clamp(0.0, (n - 1) as f32) as usize;
        let x = x_of(i);
        v.push(Shape::line_segment([pos2(x, inner.top()), pos2(x, inner.bottom())], Stroke::new(1.0, ch.trim)));
        at = Some(i);
    }
    ui.painter().extend(v);
    match at {
        Some(i) => resp.tip(|| card(i)),
        None => resp,
    }
}
