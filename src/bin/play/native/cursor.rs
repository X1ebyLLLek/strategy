//! The window's own cursor (the system's is hidden): drawn last, over everything, where the hand is
//! this very frame (a little ahead of it when it moves fast, as far as the frame lags behind). By
//! the era — the staff's brass arrow, the age of muskets' quill, the wastes' rusted arrow
//! (`assets/ui/cursors`, made by `tools/make_cursors.py`); by what the hand does — the arrow, a
//! hand over what can be clicked, an open hand dragging the map, a sword (attack) or a shield
//! (hold) for an order of the right button, a crate for a depot, the I-beam over words to write,
//! an hourglass while a world is built.

use eframe::egui::{self, pos2, vec2, Color32, CursorIcon, Rect};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Kind {
    Arrow,
    Hand,
    Grab,
    Attack,
    Hold,
    Depot,
    Text,
    Wait,
}

/// The cursor's size on the screen (points).
pub const SIZE: f32 = 32.0;

impl Kind {
    pub const ALL: [Kind; 8] = [Kind::Arrow, Kind::Hand, Kind::Grab, Kind::Attack, Kind::Hold, Kind::Depot, Kind::Text, Kind::Wait];

    fn index(self) -> usize {
        Kind::ALL.iter().position(|&k| k == self).unwrap_or(0)
    }

    pub fn name(self) -> &'static str {
        ["arrow", "hand", "grab", "attack", "hold", "depot", "text", "wait"][self.index()]
    }

    /// Where the hand's point is in the picture (of 64; as `tools/make_cursors.py` drew it).
    fn hot(self) -> (f32, f32) {
        match self {
            Kind::Hand => (17.0, 4.0),
            Kind::Grab | Kind::Text | Kind::Wait => (32.0, 32.0),
            _ => (5.0, 4.0),
        }
    }
}

macro_rules! era {
    ($t:literal) => {
        [
            include_bytes!(concat!("../../../../assets/ui/cursors/", $t, "/arrow.svg")),
            include_bytes!(concat!("../../../../assets/ui/cursors/", $t, "/hand.svg")),
            include_bytes!(concat!("../../../../assets/ui/cursors/", $t, "/grab.svg")),
            include_bytes!(concat!("../../../../assets/ui/cursors/", $t, "/attack.svg")),
            include_bytes!(concat!("../../../../assets/ui/cursors/", $t, "/hold.svg")),
            include_bytes!(concat!("../../../../assets/ui/cursors/", $t, "/depot.svg")),
            include_bytes!(concat!("../../../../assets/ui/cursors/", $t, "/text.svg")),
            include_bytes!(concat!("../../../../assets/ui/cursors/", $t, "/wait.svg")),
        ]
    };
}

const STAFF: [&[u8]; 8] = era!("staff");
const PARCHMENT: [&[u8]; 8] = era!("parchment");
const RUST: [&[u8]; 8] = era!("rust");

/// The picture of a cursor of an era.
pub fn image(theme: &str, kind: Kind) -> egui::Image<'static> {
    let (set, name) = match theme {
        "parchment" => (&PARCHMENT, "parchment"),
        "rust" => (&RUST, "rust"),
        _ => (&STAFF, "staff"),
    };
    let bytes: &'static [u8] = set[kind.index()];
    egui::Image::from_bytes(format!("bytes://cursor/{name}/{}.svg", kind.name()), egui::load::Bytes::Static(bytes)).fit_to_exact_size(vec2(SIZE, SIZE))
}

/// The map asks for a look of the cursor this frame (a sword, a shield, a crate, the open hand).
pub fn want(ctx: &egui::Context, kind: Kind) {
    // (the pass asked first: the context is not to be asked from inside its own lock)
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("cursor-want"), (pass, kind)));
}

/// What was drawn this frame (for the window's self-checks): the look, where, and
/// where the hand was.
#[derive(Clone, Copy, Debug)]
pub struct Drawn {
    pub kind: Kind,
    pub rect: Rect,
    pub pointer: egui::Pos2,
}

/// The cursor for the frame (called last): the look the map asked for, else what the widgets
/// under the hand asked egui for (a hand, the I-beam, a drag); the system's cursor hidden; the
/// picture drawn over everything where the hand is. The spoiled control: the system's cursor
/// left, the picture under the panels, always the arrow, where the hand was a frame ago.
pub fn draw(ctx: &egui::Context, theme: &str, busy: bool) -> Option<Drawn> {
    let spoil = super::spoiled();
    let pass = ctx.cumulative_pass_nr();
    let asked = ctx.data(|d| d.get_temp::<(u64, Kind)>(egui::Id::new("cursor-want"))).filter(|w| w.0 == pass).map(|w| w.1);
    let icon = ctx.output(|o| o.cursor_icon);
    let kind = if busy {
        Kind::Wait
    } else {
        asked.unwrap_or(match icon {
            CursorIcon::PointingHand => Kind::Hand,
            CursorIcon::Grab | CursorIcon::Grabbing | CursorIcon::Move | CursorIcon::AllScroll => Kind::Grab,
            CursorIcon::Text | CursorIcon::VerticalText => Kind::Text,
            CursorIcon::Wait | CursorIcon::Progress => Kind::Wait,
            _ => Kind::Arrow,
        })
    };
    let (pos, velocity, dt) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.velocity(), i.stable_dt));
    let last_key = egui::Id::new("cursor-last");
    let before = ctx.data(|d| d.get_temp::<egui::Pos2>(last_key));
    let pos = pos?;
    ctx.data_mut(|d| d.insert_temp(last_key, pos));
    if !spoil {
        ctx.set_cursor_icon(CursorIcon::None);
    }
    let (kind, at) = if spoil {
        (Kind::Arrow, before.unwrap_or(pos))
    } else {
        // (a little ahead of a fast hand: the frame shows it a moment late)
        let ahead = velocity * dt.clamp(0.0, 1.0 / 30.0) * 0.5;
        (kind, pos + if ahead.length() < 24.0 { ahead } else { vec2(0.0, 0.0) })
    };
    let (hx, hy) = kind.hot();
    let k = SIZE / 64.0;
    let rect = Rect::from_min_size(pos2(at.x - hx * k, at.y - hy * k), vec2(SIZE, SIZE));
    let layer = if spoil { egui::LayerId::background() } else { egui::LayerId::new(egui::Order::Debug, egui::Id::new("cursor")) };
    let painter = ctx.layer_painter(layer);
    let img = image(theme, kind);
    match img.load_for_size(ctx, rect.size()) {
        Ok(egui::load::TexturePoll::Ready { texture }) => {
            painter.image(texture.id, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        _ => {
            // (the picture not ready yet: a plain arrow meanwhile)
            let p = at;
            painter.add(egui::Shape::convex_polygon(vec![p, p + vec2(0.0, 20.0), p + vec2(13.0, 13.0)], Color32::from_rgb(230, 200, 120), egui::Stroke::new(1.5, Color32::BLACK)));
        }
    }
    let drawn = Drawn { kind, rect, pointer: pos };
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("cursor-drawn"), (pass, drawn)));
    Some(drawn)
}

/// What the cursor drew in the frame just drawn (for the window's self-checks, after the frame).
pub fn drawn(ctx: &egui::Context) -> Option<Drawn> {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<(u64, Drawn)>(egui::Id::new("cursor-drawn"))).filter(|x| x.0 + 1 == pass).map(|x| x.1)
}
