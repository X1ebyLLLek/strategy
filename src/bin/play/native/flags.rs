//! The countries' flags, each its own: made from the country's colour (deepened into a colour of
//! heraldry, beside a metal — white or gold) and its history (a country that lost lands carries a
//! broken shield or a fist; one that won its wars, crossed swords; a mountain country, peaks; a
//! country of fields, a sheaf; a rich one, an anvil), in the dress of its age: in the age of
//! muskets a cloth with a coat of arms under a crown and a fringe; in the staff's age plain stripes,
//! crosses and a star in the canton; in the wastes a home-made banner — ragged, faded, the sign
//! daubed on. One function paints a flag into any rectangle (`paint`): the top bar, the cards, the
//! newspaper, the counters.

use std::collections::HashMap;
use std::sync::Arc;
use eframe::egui::{self, pos2, vec2, Color32, Pos2, Rect, Shape, Stroke};
use strategy::rng::mix64;

/// How the cloth is divided.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cut {
    Plain,
    /// Two bands across (top, bottom) or down (hoist, fly).
    Across2,
    Down2,
    /// Three bands across or down.
    Across3,
    Down3,
    /// A cross set towards the hoist (the northern cross), a cross in the middle, a saltire.
    Nordic,
    Cross,
    Saltire,
    /// A canton (the upper hoist quarter) on a field.
    Canton,
    /// A band from the lower hoist to the upper fly.
    Bend,
    /// Stripes across, so many.
    Stripes(u8),
    /// A triangle from the hoist.
    Hoist,
    /// A border round the field.
    Border,
}

/// The age a flag is made in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Age {
    Muskets,
    Staff,
    Wastes,
}

#[derive(Clone, Debug)]
pub struct Flag {
    pub cut: Cut,
    /// The field, the second colour, the third.
    pub cols: [Color32; 3],
    /// The sign (an icon's name) and its colour; whether it sits in a shield under a crown.
    pub sign: Option<String>,
    pub sign_col: Color32,
    pub arms: bool,
    /// A star instead of a sign (the staff's age), its colour.
    pub star: Option<Color32>,
    pub age: Age,
    pub seed: u64,
}

fn unit(h: u64) -> f32 {
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// A colour's hue, saturation, value (for the self-checks).
pub fn hsv_of(c: Color32) -> (f32, f32, f32) {
    hsv([c.r(), c.g(), c.b()])
}

/// A colour's hue (0..1), saturation, value.
fn hsv(c: [u8; 3]) -> (f32, f32, f32) {
    let [r, g, b] = c.map(|x| x as f32 / 255.0);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let d = mx - mn;
    let h = if d < 1e-6 {
        0.0
    } else if mx == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if mx == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    (h, if mx > 0.0 { d / mx } else { 0.0 }, mx)
}

fn from_hsv(h: f32, s: f32, v: f32) -> Color32 {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - f * s), v * (1.0 - (1.0 - f) * s));
    let (r, g, b) = match (i as i32).rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

/// The metals of heraldry and black.
const WHITE: Color32 = Color32::from_rgb(238, 234, 222);
const GOLD: Color32 = Color32::from_rgb(214, 168, 52);
const BLACK: Color32 = Color32::from_rgb(28, 26, 26);

/// A country's colour (the map's, pale) deepened into a colour of a cloth: its hue kept.
fn tincture(c: [u8; 3]) -> Color32 {
    let (h, s, v) = hsv(c);
    if s < 0.12 {
        // (a grey country: black or a dark grey-blue)
        return if v < 0.5 { BLACK } else { Color32::from_rgb(52, 62, 78) };
    }
    from_hsv(h, (0.55 + 0.6 * s).min(0.85), (0.38 + 0.35 * v).min(0.72))
}

/// What a country's history gives its flag.
#[derive(Clone, Debug, Default)]
pub struct Story {
    pub won: usize,
    pub lost: usize,
    pub lost_lands: bool,
    pub mountains: bool,
    pub fields: bool,
    pub forest: bool,
    pub rich: bool,
    pub sea: bool,
}

impl Flag {
    /// The flag of country number `c` (its colour, its story) in an age (`theme`: the pack's).
    pub fn of(c: usize, colour: [u8; 3], story: &Story, theme: &str, seed: u64) -> Flag {
        // (the spoiled control: a plain cloth in the map's own pale colour)
        if super::spoiled() {
            let col = Color32::from_rgb(colour[0], colour[1], colour[2]);
            return Flag { cut: Cut::Plain, cols: [col, WHITE, BLACK], sign: None, sign_col: WHITE, arms: false, star: None, age: Age::Staff, seed };
        }
        let age = match theme {
            "parchment" => Age::Muskets,
            "rust" => Age::Wastes,
            _ => Age::Staff,
        };
        let mut h = mix64(seed ^ (c as u64).wrapping_mul(0xF1A6) ^ 0x5EED);
        let mut next = || {
            h = mix64(h);
            unit(h)
        };
        let main = tincture(colour);
        let (hue, _, _) = hsv(colour);
        let metal = if next() < 0.55 { WHITE } else { GOLD };
        // (a third colour: the opposite hue, or black, or the other metal)
        let r = next();
        let third = if r < 0.4 {
            from_hsv((hue + 0.5).fract(), 0.7, 0.5)
        } else if r < 0.7 {
            BLACK
        } else if metal == WHITE {
            GOLD
        } else {
            WHITE
        };
        let pick = |x: f32, table: &[(Cut, f32)]| -> Cut {
            let total: f32 = table.iter().map(|t| t.1).sum();
            let mut acc = 0.0;
            for t in table {
                acc += t.1 / total;
                if x < acc {
                    return t.0;
                }
            }
            table[table.len() - 1].0
        };
        let x = next();
        let cut = match age {
            Age::Muskets => pick(x, &[(Cut::Plain, 3.0), (Cut::Across2, 2.0), (Cut::Down2, 1.0), (Cut::Cross, 1.5), (Cut::Saltire, 1.2), (Cut::Border, 1.5)]),
            Age::Staff => pick(x, &[(Cut::Across3, 2.5), (Cut::Down3, 2.0), (Cut::Nordic, 1.2), (Cut::Across2, 1.0), (Cut::Canton, 1.0), (Cut::Stripes(5 + 2 * (next() < 0.5) as u8), 0.8), (Cut::Hoist, 0.8), (Cut::Bend, 0.6)]),
            Age::Wastes => pick(x, &[(Cut::Plain, 3.0), (Cut::Across2, 2.0), (Cut::Bend, 1.5), (Cut::Hoist, 1.2), (Cut::Stripes(3), 1.0)]),
        };
        // the sign, from the story
        let sign = if story.lost_lands && next() < 0.7 {
            Some(if age == Age::Wastes || next() < 0.5 { "fist" } else { "broken" })
        } else if story.won > story.lost + 1 && next() < 0.7 {
            Some(if age == Age::Muskets { "clash" } else { "medal" })
        } else if story.mountains && next() < 0.7 {
            Some("mountains")
        } else if story.rich && next() < 0.6 {
            Some(if age == Age::Wastes { "mining" } else { "anvil" })
        } else if story.fields && next() < 0.6 {
            Some("grain")
        } else if story.sea && next() < 0.5 {
            Some("compass")
        } else {
            match age {
                Age::Muskets => Some(if next() < 0.5 { "crown" } else { "castle" }),
                Age::Staff => None,
                Age::Wastes => Some(["fist", "ruins", "bandit", "shield", "clash"][(next() * 5.0) as usize % 5]),
            }
        };
        let (sign, star) = match age {
            Age::Staff => {
                // (the staff's age: a sign on few flags, a star on some)
                let s = if next() < 0.35 { sign } else { None };
                let star = if s.is_none() && next() < 0.35 { Some(if metal == WHITE { WHITE } else { GOLD }) } else { None };
                (s, star)
            }
            _ => (sign, None),
        };
        let cols = [main, metal, third];
        // the sign's colour: what stands out on the field it lies on
        let sign_col = if age == Age::Muskets { GOLD } else if metal == WHITE && cut == Cut::Plain { WHITE } else { metal };
        Flag { cut, cols, sign: sign.map(|s| s.to_string()), sign_col, arms: age == Age::Muskets, star, age, seed: h }
    }
}

/// The flags of a game's world: each country's story read from the world's past (its wars won
/// and lost, the lands it lost) and from its land (the ground of its capital, its deposits, the
/// sea at its door).
pub fn of_world(g: &crate::Game) -> Vec<Flag> {
    use strategy::world::map::Terrain;
    (0..g.countries.len())
        .map(|c| {
            let won = g.past.iter().filter(|w| w.winner.is_some_and(|k| w.sides[k] == c)).count();
            let lost = g.past.iter().filter(|w| w.winner.is_some_and(|k| w.sides[1 - k] == c)).count();
            let lost_lands = g.lost_lands.iter().any(|l| l.1 as usize == c);
            let cap = g.capitals.get(c).copied().unwrap_or(usize::MAX);
            let mut story = Story { won, lost, lost_lands, ..Default::default() };
            if cap < g.world.provinces.len() {
                let p = &g.world.provinces[cap];
                story.mountains = matches!(p.terrain, Terrain::Mountains | Terrain::Hills);
                story.fields = p.terrain == Terrain::Plains;
                story.forest = matches!(p.terrain, Terrain::Forest | Terrain::Jungle);
                story.rich = g.prov_res.get(cap).is_some_and(|r| r.len() >= 2);
                story.sea = p.neighbors.iter().any(|&q| g.world.provinces[q as usize].sea);
            }
            Flag::of(c, g.countries[c].1, &story, &g.pack.theme, g.seed)
        })
        .collect()
}

/// The flags of a world, kept in the window's memory (each country by its number).
#[derive(Clone, Default)]
pub struct Flags(pub Arc<Vec<Flag>>);

pub fn set(ctx: &egui::Context, f: Vec<Flag>) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("flags"), Flags(Arc::new(f))));
}

pub fn get(ctx: &egui::Context, c: usize) -> Option<Flag> {
    let f: Option<Flags> = ctx.data(|d| d.get_temp(egui::Id::new("flags")));
    f.and_then(|f| f.0.get(c).cloned())
}

/// The cloth's outline: a rectangle, or (the wastes) ragged at the fly.
fn outline(r: Rect, f: &Flag) -> Vec<Pos2> {
    if f.age != Age::Wastes {
        return vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()];
    }
    let mut pts = vec![r.left_top()];
    let n = 9;
    let mut h = f.seed;
    for i in 0..=n {
        h = mix64(h);
        let t = i as f32 / n as f32;
        let tear = r.width() * (0.02 + 0.07 * unit(h));
        pts.push(pos2(r.right() - tear, r.top() + r.height() * t));
    }
    pts.push(r.left_bottom());
    pts
}

/// A flag painted into `r` (the cloth, its bands, its sign; a sheen as of cloth; a frame of
/// `trim`). `icons`: the window's icons, for the sign.
pub fn paint(ctx: &egui::Context, painter: &egui::Painter, r: Rect, f: &Flag, trim: Color32, icons: Option<&HashMap<String, Arc<[u8]>>>) {
    let mut v: Vec<Shape> = Vec::new();
    let [a, b, c] = f.cols;
    let (w, h) = (r.width(), r.height());
    let band = |v: &mut Vec<Shape>, x0: f32, y0: f32, x1: f32, y1: f32, col: Color32| {
        v.push(Shape::rect_filled(Rect::from_min_max(pos2(r.left() + w * x0, r.top() + h * y0), pos2(r.left() + w * x1, r.top() + h * y1)), 0.0, col));
    };
    v.push(Shape::rect_filled(r.expand(1.5), 1.5, Color32::from_black_alpha(220)));
    band(&mut v, 0.0, 0.0, 1.0, 1.0, a);
    // where the sign goes (relative middle, size of the shorter side)
    let mut spot = (0.5f32, 0.5f32, 0.62f32);
    match f.cut {
        Cut::Plain => {}
        Cut::Across2 => band(&mut v, 0.0, 0.5, 1.0, 1.0, b),
        Cut::Down2 => band(&mut v, 0.5, 0.0, 1.0, 1.0, b),
        Cut::Across3 => {
            band(&mut v, 0.0, 1.0 / 3.0, 1.0, 2.0 / 3.0, b);
            band(&mut v, 0.0, 2.0 / 3.0, 1.0, 1.0, c);
            spot = (0.5, 0.5, 0.30);
        }
        Cut::Down3 => {
            band(&mut v, 1.0 / 3.0, 0.0, 2.0 / 3.0, 1.0, b);
            band(&mut v, 2.0 / 3.0, 0.0, 1.0, 1.0, c);
        }
        Cut::Nordic => {
            band(&mut v, 0.30, 0.0, 0.42, 1.0, b);
            band(&mut v, 0.0, 0.42, 1.0, 0.58, b);
            band(&mut v, 0.335, 0.0, 0.385, 1.0, c);
            band(&mut v, 0.0, 0.47, 1.0, 0.53, c);
            spot = (0.15, 0.21, 0.32);
        }
        Cut::Cross => {
            band(&mut v, 0.42, 0.0, 0.58, 1.0, b);
            band(&mut v, 0.0, 0.40, 1.0, 0.60, b);
            spot = (0.5, 0.5, 0.42);
        }
        Cut::Saltire => {
            let t = h * 0.13;
            for (p, q) in [(r.left_top(), r.right_bottom()), (r.left_bottom(), r.right_top())] {
                v.push(Shape::line_segment([p, q], Stroke::new(t, b)));
            }
            spot = (0.5, 0.5, 0.40);
        }
        Cut::Canton => {
            band(&mut v, 0.0, 0.5, 1.0, 1.0, c);
            band(&mut v, 0.0, 0.0, 0.42, 0.5, b);
            spot = (0.21, 0.25, 0.36);
        }
        Cut::Bend => {
            let t = h * 0.22;
            v.push(Shape::line_segment([r.left_bottom(), r.right_top()], Stroke::new(t, b)));
            v.push(Shape::line_segment([r.left_bottom(), r.right_top()], Stroke::new(t * 0.35, c)));
            spot = (0.22, 0.25, 0.34);
        }
        Cut::Stripes(n) => {
            for k in 0..n as usize {
                if k % 2 == 1 {
                    band(&mut v, 0.0, k as f32 / n as f32, 1.0, (k + 1) as f32 / n as f32, b);
                }
            }
            if n > 3 {
                band(&mut v, 0.0, 0.0, 0.4, (n as f32 / 2.0).ceil() / n as f32, c);
                spot = (0.2, 0.28, 0.34);
            }
        }
        Cut::Hoist => {
            band(&mut v, 0.0, 0.5, 1.0, 1.0, c);
            v.push(Shape::convex_polygon(vec![r.left_top(), pos2(r.left() + w * 0.42, r.center().y), r.left_bottom()], b, Stroke::NONE));
            spot = (0.14, 0.5, 0.32);
        }
        Cut::Border => {
            let k = h * 0.11;
            v.push(Shape::rect_stroke(r.shrink(k / 2.0), 0.0, Stroke::new(k, b)));
        }
    }
    painter.extend(std::mem::take(&mut v));
    // the sign: a coat of arms (a shield under a crown) in the age of muskets, else the sign
    // itself; or a star
    let side = w.min(h * 1.4) * spot.2;
    let at = pos2(r.left() + w * spot.0, r.top() + h * spot.1);
    if f.arms {
        let sw = side * 0.95;
        let sh = sw * 1.15;
        let top = at.y - sh * 0.42;
        let shield = vec![
            pos2(at.x - sw / 2.0, top),
            pos2(at.x + sw / 2.0, top),
            pos2(at.x + sw / 2.0, top + sh * 0.55),
            pos2(at.x, top + sh),
            pos2(at.x - sw / 2.0, top + sh * 0.55),
        ];
        painter.add(Shape::convex_polygon(shield.clone(), Color32::from_rgb(150, 30, 26), Stroke::new(1.0, GOLD)));
        // (a small crown above it)
        let cy = top - side * 0.12;
        let cw = sw * 0.62;
        painter.add(Shape::convex_polygon(vec![pos2(at.x - cw / 2.0, cy + side * 0.08), pos2(at.x + cw / 2.0, cy + side * 0.08), pos2(at.x + cw / 2.0, cy - side * 0.03), pos2(at.x, cy + side * 0.02), pos2(at.x - cw / 2.0, cy - side * 0.03)], GOLD, Stroke::NONE));
        if let (Some(sign), Some(icons)) = (&f.sign, icons) {
            image(ctx, painter, icons, sign, Rect::from_center_size(pos2(at.x, top + sh * 0.42), vec2(sw * 0.72, sw * 0.72)), GOLD);
        }
    } else if let Some(sc) = f.star {
        star(painter, at, side * 0.45, sc);
    } else if let (Some(sign), Some(icons)) = (&f.sign, icons) {
        let col = if f.age == Age::Wastes { f.sign_col.gamma_multiply(0.85) } else { f.sign_col };
        image(ctx, painter, icons, sign, Rect::from_center_size(at, vec2(side, side)), col);
    }
    // the cloth: folds (light and dark bands across), a sheen; in the wastes stains and a tear
    let mut v: Vec<Shape> = Vec::new();
    let folds = 4;
    for i in 0..folds {
        let x0 = r.left() + w * i as f32 / folds as f32;
        let fr = Rect::from_min_size(pos2(x0, r.top()), vec2(w / folds as f32, h));
        let mesh_l = Color32::from_white_alpha(if f.age == Age::Wastes { 8 } else { 22 });
        let mesh_d = Color32::from_black_alpha(if f.age == Age::Wastes { 40 } else { 26 });
        v.push(gradient_x(fr, mesh_l, mesh_d));
    }
    if f.age == Age::Wastes {
        let mut hh = f.seed;
        for _ in 0..3 {
            hh = mix64(hh);
            let p = pos2(r.left() + w * unit(hh), r.top() + h * unit(mix64(hh ^ 3)));
            v.push(Shape::circle_filled(p, h * (0.08 + 0.12 * unit(mix64(hh ^ 5))), Color32::from_rgba_unmultiplied(60, 45, 25, 60)));
        }
        // (the fly torn: the frame's colour over the tear)
        let out = outline(r, f);
        let mut tear: Vec<Pos2> = out[1..out.len() - 1].to_vec();
        tear.push(r.right_bottom() + vec2(2.0, 2.0));
        tear.insert(0, r.right_top() + vec2(2.0, -2.0));
        for k in 0..tear.len() - 1 {
            let q = [tear[k], tear[k + 1], pos2(r.right() + 2.0, tear[k + 1].y), pos2(r.right() + 2.0, tear[k].y)];
            v.push(Shape::convex_polygon(q.to_vec(), Color32::from_black_alpha(230), Stroke::NONE));
        }
    }
    v.push(gradient_y(r, Color32::from_white_alpha(40), Color32::from_black_alpha(50)));
    if f.age == Age::Muskets {
        // (a fringe of gold along the fly)
        let n = (h / 2.5) as usize;
        for i in 0..n {
            let y = r.top() + h * (i as f32 + 0.5) / n as f32;
            v.push(Shape::line_segment([pos2(r.right(), y), pos2(r.right() + 2.5, y)], Stroke::new(1.0, GOLD)));
        }
    }
    v.push(Shape::rect_stroke(r.expand(1.5), 1.5, Stroke::new(1.2, trim)));
    painter.extend(v);
}

/// A five-pointed star.
fn star(painter: &egui::Painter, c: Pos2, r: f32, col: Color32) {
    let pts: Vec<Pos2> = (0..10)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
            let rr = if i % 2 == 0 { r } else { r * 0.42 };
            pos2(c.x + rr * a.cos(), c.y + rr * a.sin())
        })
        .collect();
    // (a star is not convex: drawn as five triangles about its middle)
    let mut v = Vec::new();
    for i in 0..10 {
        v.push(Shape::convex_polygon(vec![c, pts[i], pts[(i + 1) % 10]], col, Stroke::NONE));
    }
    painter.extend(v);
}

/// An icon of the window painted by a painter (no `Ui` needed), tinted.
pub fn image(ctx: &egui::Context, painter: &egui::Painter, icons: &HashMap<String, Arc<[u8]>>, name: &str, r: Rect, tint: Color32) {
    let img = super::look::icon(icons, name, r.width(), tint);
    if let Ok(egui::load::TexturePoll::Ready { texture }) = img.load_for_size(ctx, r.size()) {
        painter.image(texture.id, r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
    }
}

/// A rectangle shaded from left to right, top to bottom.
fn gradient_x(r: Rect, a: Color32, b: Color32) -> Shape {
    let mut m = egui::Mesh::default();
    m.colored_vertex(r.left_top(), a);
    m.colored_vertex(r.right_top(), b);
    m.colored_vertex(r.right_bottom(), b);
    m.colored_vertex(r.left_bottom(), a);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    Shape::mesh(m)
}

fn gradient_y(r: Rect, a: Color32, b: Color32) -> Shape {
    let mut m = egui::Mesh::default();
    m.colored_vertex(r.left_top(), a);
    m.colored_vertex(r.right_top(), a);
    m.colored_vertex(r.right_bottom(), b);
    m.colored_vertex(r.left_bottom(), b);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    Shape::mesh(m)
}
