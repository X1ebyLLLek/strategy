//! The window's dress, by the era (`assets/ui/<theme>`: CC0 photographs of ambientCG recoloured to
//! the era — the staff's olive steel, cream paper and manila folders; walnut and parchment; rusted
//! steel and canvas): the bars of steel or wood, the folder as a sheet in a frame, plates for the
//! buttons, bevels and a trim of brass, the map's edges darkened as under a lamp. Each material is
//! laid over a plain box of its mean colour: that is the ground the eye (and the window's
//! self-checks) take under the words.

use eframe::egui::{self, epaint, pos2, vec2, Color32, Rect, Response, RichText, Sense, Shape, Stroke, TextureHandle, TextureOptions, Ui};

/// Where the shapes go (a painter's list now, or a placeholder filled when the size is known).
pub type Out = Vec<Shape>;

use super::look::Skin;

mod card;
pub use card::{shown, tip_at, Card, Shown, Tip, Tone, TIP_WAIT};

/// A material: its picture (tiled), its mean colour, how many screen points a tile takes.
#[derive(Clone)]
pub struct Mat {
    pub tex: TextureHandle,
    pub mean: Color32,
    pub tile: f32,
}

#[derive(Clone)]
pub struct Chrome {
    pub bar: Mat,
    pub paper: Mat,
    pub frame: Mat,
    /// The trim (brass, gold, amber), and the light and dark of the bevels.
    pub trim: Color32,
}

fn mat(ctx: &egui::Context, name: &str, bytes: &[u8], tile: f32) -> Mat {
    let img = image::load_from_memory(bytes).map(|i| i.to_rgba8()).ok();
    let (size, px) = match &img {
        Some(i) => ([i.width() as usize, i.height() as usize], i.as_raw().clone()),
        None => ([1, 1], vec![128, 128, 128, 255]),
    };
    let n = (size[0] * size[1]).max(1) as f64;
    let mut sum = [0f64; 3];
    for c in px.chunks_exact(4) {
        for k in 0..3 {
            sum[k] += c[k] as f64;
        }
    }
    let mean = Color32::from_rgb((sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8);
    let image = egui::ColorImage::from_rgba_unmultiplied(size, &px);
    let opts = TextureOptions { wrap_mode: egui::TextureWrapMode::Repeat, ..TextureOptions::LINEAR };
    Mat { tex: ctx.load_texture(name, image, opts), mean, tile }
}

impl Chrome {
    pub fn load(ctx: &egui::Context, theme: &str) -> Chrome {
        macro_rules! set {
            ($t:literal) => {
                (
                    &include_bytes!(concat!("../../../../assets/ui/", $t, "/bar.jpg"))[..],
                    &include_bytes!(concat!("../../../../assets/ui/", $t, "/paper.jpg"))[..],
                    &include_bytes!(concat!("../../../../assets/ui/", $t, "/frame.jpg"))[..],
                )
            };
        }
        let ((bar, paper, frame), trim) = match theme {
            "parchment" => (set!("parchment"), Color32::from_rgb(0xc9, 0xa2, 0x4a)),
            "rust" => (set!("rust"), Color32::from_rgb(0xd8, 0x9a, 0x2a)),
            _ => (set!("staff"), Color32::from_rgb(0xb8, 0x97, 0x5a)),
        };
        Chrome { bar: mat(ctx, "ui-bar", bar, 256.0), paper: mat(ctx, "ui-paper", paper, 384.0), frame: mat(ctx, "ui-frame", frame, 192.0), trim }
    }

    /// A material over a rectangle (its mean colour beneath as a plain box; the picture over it a
    /// little transparent, so that the grain stays soft under words).
    pub fn lay(&self, p: &mut Out, r: Rect, m: &Mat, rounding: f32) {
        p.push(Shape::rect_filled(r, rounding, m.mean));
        let uv = Rect::from_min_max(pos2(r.min.x / m.tile, r.min.y / m.tile), pos2(r.max.x / m.tile, r.max.y / m.tile));
        p.push(Shape::Rect(epaint::RectShape { rect: r, rounding: rounding.into(), fill: Color32::from_white_alpha(200), stroke: Stroke::NONE, blur_width: 0.0, fill_texture_id: m.tex.id(), uv }));
    }

    /// Light on the upper and left edges, shade on the lower and right (raised); the other way
    /// round (sunk).
    pub fn bevel(&self, p: &mut Out, r: Rect, raised: bool, w: f32) {
        let (a, b) = (Color32::from_white_alpha(46), Color32::from_black_alpha(120));
        let (tl, br) = if raised { (a, b) } else { (b, a) };
        let h = w / 2.0;
        p.push(Shape::line_segment([pos2(r.min.x, r.min.y + h), pos2(r.max.x, r.min.y + h)], Stroke::new(w, tl)));
        p.push(Shape::line_segment([pos2(r.min.x + h, r.min.y), pos2(r.min.x + h, r.max.y)], Stroke::new(w, tl)));
        p.push(Shape::line_segment([pos2(r.min.x, r.max.y - h), pos2(r.max.x, r.max.y - h)], Stroke::new(w, br)));
        p.push(Shape::line_segment([pos2(r.max.x - h, r.min.y), pos2(r.max.x - h, r.max.y)], Stroke::new(w, br)));
    }

    /// A soft sheen from the top down (light above, shade below).
    pub fn gloss(&self, p: &mut Out, r: Rect, top: Color32, bottom: Color32) {
        let mut m = epaint::Mesh::default();
        m.colored_vertex(r.left_top(), top);
        m.colored_vertex(r.right_top(), top);
        m.colored_vertex(r.right_bottom(), bottom);
        m.colored_vertex(r.left_bottom(), bottom);
        m.add_triangle(0, 1, 2);
        m.add_triangle(0, 2, 3);
        p.push(Shape::mesh(m));
    }

    /// A sheet in a frame (the folder, an event, the menu): the frame's band, bevelled; the paper
    /// inside with a line of trim and a soft shade along its edges.
    pub fn sheet(&self, p: &mut Out, r: Rect, band: f32) {
        self.lay(p, r, &self.frame, 3.0);
        self.bevel(p, r, true, 2.0);
        let inner = r.shrink(band);
        self.lay(p, inner, &self.paper, 1.0);
        self.bevel(p, inner, false, 1.5);
        // (the shade inside the edges: as if the sheet lay a little below the frame)
        // (33rd session: the shade deeper and wider — the sheet is an old page, darker at its edges, not a flat card)
        let s = 22.0f32.min(inner.width() / 4.0).min(inner.height() / 4.0);
        let dark = Color32::from_black_alpha(52);
        let clear = Color32::TRANSPARENT;
        let mut strip = |a: Rect, from_top: bool, from_left: bool, vertical: bool| {
            let mut m = epaint::Mesh::default();
            let (c0, c1) = if vertical { if from_top { (dark, clear) } else { (clear, dark) } } else if from_left { (dark, clear) } else { (clear, dark) };
            if vertical {
                m.colored_vertex(a.left_top(), c0);
                m.colored_vertex(a.right_top(), c0);
                m.colored_vertex(a.right_bottom(), c1);
                m.colored_vertex(a.left_bottom(), c1);
            } else {
                m.colored_vertex(a.left_top(), c0);
                m.colored_vertex(a.right_top(), c1);
                m.colored_vertex(a.right_bottom(), c1);
                m.colored_vertex(a.left_bottom(), c0);
            }
            m.add_triangle(0, 1, 2);
            m.add_triangle(0, 2, 3);
            p.push(Shape::mesh(m));
        };
        strip(Rect::from_min_size(inner.min, vec2(inner.width(), s)), true, true, true);
        strip(Rect::from_min_max(pos2(inner.min.x, inner.max.y - s), inner.max), false, true, true);
        strip(Rect::from_min_size(inner.min, vec2(s, inner.height())), true, true, false);
        strip(Rect::from_min_max(pos2(inner.max.x - s, inner.min.y), inner.max), true, false, false);
        // (33rd session: a double rule — trim outside, ink inside — and brackets with a diamond at the four corners, instead of one line)
        let line = inner.shrink(4.5);
        p.push(Shape::rect_stroke(line, 0.0, Stroke::new(1.2, self.trim.gamma_multiply(0.85))));
        p.push(Shape::rect_stroke(line.shrink(3.0), 0.0, Stroke::new(0.7, Color32::from_black_alpha(90))));
        let (len, col) = (18.0, self.trim);
        super::hud::bracket(p, line.left_top(), 1.0, 1.0, len, col);
        super::hud::bracket(p, line.right_top(), -1.0, 1.0, len, col);
        super::hud::bracket(p, line.left_bottom(), 1.0, -1.0, len, col);
        super::hud::bracket(p, line.right_bottom(), -1.0, -1.0, len, col);
    }

    /// A plate for a button: on a bar (dark) or on paper (light); chosen — the accent; under the
    /// hand — a little lighter.
    pub fn plate(&self, p: &mut Out, r: Rect, skin: &Skin, dark: bool, on: bool, hot: bool, down: bool) {
        if dark && !on {
            self.lay(p, r, &self.bar, 2.0);
            self.gloss(p, r, Color32::from_white_alpha(if hot { 40 } else { 24 }), Color32::from_black_alpha(70));
            self.bevel(p, r, !down, 1.5);
            p.push(Shape::rect_stroke(r, 2.0, Stroke::new(1.0, if hot { self.trim } else { Color32::from_black_alpha(150) })));
            return;
        }
        // (33rd session: a button on paper is a label with two cut corners, a lit top line and a fine edge — it was a
        // bevelled grey box; the chosen one is the accent, with a line of trim)
        let c = 4.0f32.min(r.height() / 3.0);
        let pts = super::hud::chamfer(r, c);
        if !down {
            p.push(epaint::Shadow { offset: vec2(0.0, 1.5), blur: 3.0, spread: 0.0, color: Color32::from_black_alpha(if hot { 90 } else { 60 }) }.as_shape(r, 2.0).into());
        }
        if on {
            p.push(Shape::convex_polygon(pts.clone(), skin.accent, Stroke::NONE));
            self.gloss(p, r.shrink(1.0), Color32::from_white_alpha(46), Color32::from_black_alpha(54));
            p.push(Shape::closed_line(pts, Stroke::new(1.3, self.trim)));
        } else {
            super::hud::lay_poly(p, &pts, &self.paper, Color32::WHITE);
            self.gloss(p, r.shrink(1.0), Color32::from_white_alpha(if hot { 120 } else { 80 }), Color32::from_black_alpha(if down { 60 } else { 26 }));
            p.push(Shape::line_segment([pos2(r.min.x + c + 1.0, r.min.y + 1.0), pos2(r.max.x - 2.0, r.min.y + 1.0)], Stroke::new(1.0, Color32::from_white_alpha(140))));
            p.push(Shape::closed_line(pts, Stroke::new(1.0, if hot { skin.accent } else { skin.edge })));
        }
    }

    /// The map's edges darkened, as under a lamp.
    pub fn vignette(&self, p: &mut Out, r: Rect, depth: f32) {
        let inner = r.shrink(depth);
        let dark = Color32::from_black_alpha(110);
        let mut m = epaint::Mesh::default();
        for q in [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()] {
            m.colored_vertex(q, dark);
        }
        for q in [inner.left_top(), inner.right_top(), inner.right_bottom(), inner.left_bottom()] {
            m.colored_vertex(q, Color32::TRANSPARENT);
        }
        for i in 0..4u32 {
            let j = (i + 1) % 4;
            m.add_triangle(i, j, 4 + j);
            m.add_triangle(i, 4 + j, 4 + i);
        }
        p.push(Shape::mesh(m));
    }

    /// A heading in a folder: the words in the era's heading letters, a rule of trim under them
    /// with a diamond in the middle.
    pub fn heading(&self, ui: &mut Ui, skin: &Skin, text: &str) {
        ui.add_space(8.0);
        if !text.is_empty() {
            ui.label(RichText::new(text.to_uppercase()).family(egui::FontFamily::Name("head".into())).size(16.0).color(skin.accent));
        }
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 7.0), Sense::hover());
        let y = r.center().y;
        let p = ui.painter();
        p.line_segment([pos2(r.min.x, y), pos2(r.max.x, y)], Stroke::new(1.0, self.trim));
        let c = pos2(r.min.x + 18.0, y);
        p.add(Shape::convex_polygon(vec![c + vec2(-5.0, 0.0), c + vec2(0.0, -3.5), c + vec2(5.0, 0.0), c + vec2(0.0, 3.5)], self.trim, Stroke::NONE));
    }
}

/// A button as a plate with words: on a bar (dark) or on paper.
pub fn button(ui: &mut Ui, ch: &Chrome, skin: &Skin, text: &str, size: egui::Vec2, dark: bool, on: bool) -> Response {
    press(ui, ch, skin, Press { text, size, dark, on, ..Press::default() })
}

/// What a plate-button is: its words and icon, its size, on a bar (dark) or on paper, chosen,
/// whether it can be pressed, the size of its letters.
pub struct Press<'a> {
    pub text: &'a str,
    pub icon: Option<egui::Image<'static>>,
    pub size: egui::Vec2,
    pub dark: bool,
    pub on: bool,
    pub enabled: bool,
    pub font: f32,
}

impl Default for Press<'_> {
    fn default() -> Self {
        Press { text: "", icon: None, size: vec2(0.0, 30.0), dark: false, on: false, enabled: true, font: 15.0 }
    }
}

/// A plate-button: words, an icon, or an icon and words; a hand over it (when it can be pressed);
/// pressed, it sinks a little.
pub fn press(ui: &mut Ui, ch: &Chrome, skin: &Skin, b: Press) -> Response {
    let font = egui::FontId::new(b.font, egui::FontFamily::Name("head".into()));
    let ink = if b.on { skin.on_accent() } else if b.dark { skin.desk_ink } else { skin.ink };
    let ink = if b.enabled { ink } else { ink.gamma_multiply(0.45) };
    let galley = (!b.text.is_empty()).then(|| ui.painter().layout_no_wrap(b.text.to_string(), font, ink));
    let icon_w = if b.icon.is_some() { b.size.y - 12.0 } else { 0.0 };
    let words_w = galley.as_ref().map_or(0.0, |g| g.size().x);
    let gap = if icon_w > 0.0 && words_w > 0.0 { 6.0 } else { 0.0 };
    let want = vec2(b.size.x.max(icon_w + gap + words_w + if words_w > 0.0 { 20.0 } else { 12.0 }), b.size.y.max(galley.as_ref().map_or(0.0, |g| g.size().y) + 8.0));
    let (r, resp) = ui.allocate_exact_size(want, if b.enabled { Sense::click() } else { Sense::hover() });
    let resp = if b.enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
    if ui.is_rect_visible(r) {
        let down = b.enabled && resp.is_pointer_button_down_on();
        let mut v = Out::new();
        ch.plate(&mut v, r, skin, b.dark, b.on, b.enabled && resp.hovered(), down);
        if !b.enabled {
            v.push(Shape::rect_filled(r, 2.0, Color32::from_black_alpha(40)));
        }
        ui.painter().extend(v);
        let sink = if down { 1.0 } else { 0.0 };
        let total = icon_w + gap + words_w;
        let x0 = r.center().x - total / 2.0;
        if let Some(img) = b.icon {
            let ir = Rect::from_min_size(pos2(x0, r.center().y - icon_w / 2.0 + sink), vec2(icon_w, icon_w));
            img.paint_at(ui, if words_w > 0.0 { ir } else { Rect::from_center_size(r.center() + vec2(0.0, sink), vec2(icon_w, icon_w)) });
        }
        if let Some(g) = galley {
            let gs = g.size();
            ui.painter().galley(pos2(x0 + icon_w + gap, r.center().y - gs.y / 2.0 + sink), g, ink);
        }
    }
    resp
}

/// The window's dress kept where any folder can find it.
pub fn of(ctx: &egui::Context) -> Option<Chrome> {
    ctx.data(|d| d.get_temp::<Chrome>(egui::Id::new("chrome")))
}

/// A placeholder for what goes under a part of the window whose size is known only after it is
/// drawn; `under` fills it.
pub fn hold(ui: &Ui) -> egui::layers::ShapeIdx {
    ui.painter().add(Shape::Noop)
}

pub fn under(ui: &Ui, at: egui::layers::ShapeIdx, shapes: Out) {
    // (within the part's clip: a card scrolled out of a folder is not seen)
    let mut p = ui.painter().clone();
    let bounds = shapes.iter().fold(Rect::NOTHING, |r, s| r.union(s.visual_bounding_rect()));
    p.set_clip_rect(bounds.expand(2.0).intersect(ui.clip_rect().expand(3.0)));
    p.set(at, Shape::Vec(shapes));
}

/// `under` for a window's sheet: a window's margins lie outside its contents' clip, so the sheet
/// is clipped to its own bounds.
pub fn under_window(ui: &Ui, at: egui::layers::ShapeIdx, shapes: Out) {
    let mut p = ui.painter().clone();
    let bounds = shapes.iter().fold(Rect::NOTHING, |r, s| r.union(s.visual_bounding_rect()));
    p.set_clip_rect(bounds.expand(2.0).intersect(ui.ctx().screen_rect()));
    p.set(at, Shape::Vec(shapes));
}

/// A place of the window worth finding again (for the self-checks): the first thing of that name
/// drawn this frame.
pub fn spot(ctx: &egui::Context, name: &'static str, r: Rect) {
    keep_spot(ctx, name, r, false);
}

/// `spot`, the last thing of that name drawn this frame.
pub fn spot_last(ctx: &egui::Context, name: &'static str, r: Rect) {
    keep_spot(ctx, name, r, true);
}

fn keep_spot(ctx: &egui::Context, name: &'static str, r: Rect, last: bool) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        let m = d.get_temp_mut_or_default::<std::collections::HashMap<&'static str, (u64, Rect)>>(egui::Id::new("spots"));
        if last || m.get(name).map_or(true, |x| x.0 != pass) {
            m.insert(name, (pass, r));
        }
    });
}

/// The places of the window in the frame just drawn (see `spot`; for the self-checks, after it).
pub fn spots(ctx: &egui::Context) -> std::collections::HashMap<&'static str, Rect> {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<std::collections::HashMap<&'static str, (u64, Rect)>>(egui::Id::new("spots"))).unwrap_or_default().into_iter().filter(|(_, v)| v.0 + 1 == pass).map(|(k, v)| (k, v.1)).collect()
}

/// The window's dress and the era's colours, for a widget that paints itself.
pub fn dress(ui: &egui::Ui) -> (Chrome, Skin) {
    (of(ui.ctx()).expect("одежда окна"), skin(ui.ctx()))
}

/// The era's colours, kept where any part of the window can find them.
pub fn skin(ctx: &egui::Context) -> Skin {
    ctx.data(|d| d.get_temp::<Skin>(egui::Id::new("skin"))).unwrap_or_else(|| Skin::of(""))
}

/// The icons of the sprite, kept where any part of the window can find them.
pub fn icons(ctx: &egui::Context) -> std::sync::Arc<std::collections::HashMap<String, std::sync::Arc<[u8]>>> {
    ctx.data(|d| d.get_temp(egui::Id::new("icons"))).unwrap_or_default()
}

/// The window's dress, colours and icons put where every part of the window finds them.
pub fn keep(ctx: &egui::Context, ch: &Chrome, skin: Skin, icons: &std::collections::HashMap<String, std::sync::Arc<[u8]>>) {
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("chrome"), ch.clone());
        d.insert_temp(egui::Id::new("skin"), skin);
        d.insert_temp(egui::Id::new("icons"), std::sync::Arc::new(icons.clone()));
    });
}

/// A colour of words for good or bad news that reads on a ground: the first of the colours given
/// with a contrast of 4.5 there (else the last).
pub fn legible(on: Color32, choices: &[Color32]) -> Color32 {
    *choices.iter().find(|&&c| super::uitest::contrast(c, on) >= 4.5).unwrap_or(choices.last().unwrap_or(&Color32::BLACK))
}

// ---- the cards of what the hand rests on ------------------------------------------------------

