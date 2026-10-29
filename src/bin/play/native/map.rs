//! The map in the window: the land in three dimensions, drawn by the video card (`scene`: the
//! world's heights lit by the sun, the sea with waves, the countries' colours laid over the land,
//! their borders), seen by a camera that looks straight down from afar and tilts towards the
//! horizon close up (`camera`); over it, with the same camera, the borders close up as smooth
//! lines, the countries' names, the towns, the capitals of the war, the depots, the player's
//! groups of armies as counters, the enemy seen across the front, the arrows of the blows, the
//! month's battles. Drag to move, the wheel to come closer; hover for what a province is; click a
//! counter or a sector to choose a group; with a group chosen, the right button on an enemy
//! province sends it there, on one's own tells it to hold; a click on a battle explains it.

use std::sync::{Arc, Mutex};
use eframe::egui::{self, pos2, vec2, Align2, Color32, ColorImage, FontFamily, FontId, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions};
use strategy::military::front::NO;
use strategy::world::map::NONE;

use crate::Game;
use strategy::world::map::smoothstep;
use super::chrome::{tip_at, Card, Tone};

mod borders;
mod camera;
mod card;
mod divs;
mod figures;
mod mesh;
mod ground;
mod objects;
mod paint;
mod plans;
mod probes;
mod scene;
mod seas;
mod towns;
mod troops;
mod ui;
mod view;
mod ways;
pub use camera::Camera;
pub use ground::{RELIEF, RIVER_HW, RIVER_RANGE};
pub use divs::{doing, main_kind, stacks};
pub use troops::{arrow as troops_arrow, badge, counter, sign, Counter, AGG, ATTACK, HUNGRY, MARCH, OWN, SHORT};
pub use ways::step_of;
pub use plans::{border_with, country_now};

/// The time of year as the land shows it (snow, yellowing, mud; each 0..1), at a month of the
/// game (the first is January) and its day: winter from December to February, the thaw's mud
/// in March and April, autumn from September to November; each comes and goes over weeks.
pub fn season_of(month: u32, day: u32) -> [f32; 3] {
    // (the year as a number of months from January's start)
    let t = (month % 12) as f32 + (day as f32 / 30.0).min(1.0);
    let bump = |from: f32, full: f32, fade: f32, to: f32| -> f32 {
        // (0 before `from`, rising to 1 at `full`, 1 till `fade`, 0 at `to`; across the new year)
        let x = if t < from - 6.0 { t + 12.0 } else { t };
        if x <= from || x >= to {
            0.0
        } else if x < full {
            (x - from) / (full - from)
        } else if x <= fade {
            1.0
        } else {
            1.0 - (x - fade) / (to - fade)
        }
    };
    [bump(10.6, 11.5, 13.8, 14.6), bump(7.8, 9.0, 10.4, 11.2), bump(1.9, 2.6, 3.8, 4.6)]
}

/// What a click on the map builds.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Build {
    #[default]
    None,
    Depot,
    /// Where the new divisions gather.
    Muster,
    /// A railway (or a road), from this province of the theatre once it is chosen.
    Line { rail: bool, from: Option<usize> },
}

/// The plan the player draws for the chosen groups of armies (one or several): their front (a
/// click on a neighbour — the whole border with him; or the left button drawn over provinces of
/// ours: those it went over so far), the head of their arrows (a click on a province of the
/// enemy's).
#[derive(Clone, PartialEq, Debug, Default)]
pub enum Tool {
    #[default]
    None,
    Front { gs: Vec<usize>, drawn: Vec<usize> },
    Attack { gs: Vec<usize> },
}

/// Where the chosen divisions would go if the right button were pressed where the hand is: from
/// where (the province most of them stand in, or where their ways end with Shift), to where,
/// whether one stop more (Shift); the way and the days it takes the slowest of them.
#[derive(Clone, Debug)]
pub struct Preview {
    pub key: (usize, usize, bool),
    pub way: Vec<usize>,
    pub days: f32,
}

/// Colours of the groups of armies (as on the page).
pub const HUES: [Color32; 8] = [
    Color32::from_rgb(0xe8, 0xb8, 0x30),
    Color32::from_rgb(0x3f, 0xb8, 0xaf),
    Color32::from_rgb(0xe0, 0x6c, 0x9f),
    Color32::from_rgb(0x86, 0xc2, 0x5a),
    Color32::from_rgb(0x9b, 0x7f, 0xd4),
    Color32::from_rgb(0xef, 0x8f, 0x3a),
    Color32::from_rgb(0x5b, 0x9b, 0xe0),
    Color32::from_rgb(0xc9, 0xc9, 0x5a),
];

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Political,
    Terrain,
    Supply,
    Dev,
    People,
    Res,
    Lost,
    Faith,
    Regime,
    Relations,
    Armies,
    Seas,
}

impl Mode {
    pub const ALL: [(Mode, &'static str, &'static str); 12] = [
        (Mode::Political, "Страны", "кто чем владеет"),
        (Mode::Terrain, "Рельеф", "горы, леса, болота: где обороняться легче"),
        (Mode::Supply, "Снабжение", "сколько доходит до ваших войск: от столицы и складов"),
        (Mode::Dev, "Развитие", "развитые и отсталые края"),
        (Mode::People, "Люди", "где сколько живёт"),
        (Mode::Res, "Недра", "месторождения: где что добывают"),
        (Mode::Lost, "Былое", "земли, которые войны прошлого отняли у одной страны и отдали другой: цветом той, что их потеряла"),
        (Mode::Faith, "Веры", "чью веру исповедуют области: веры расходятся от соседа к соседу, раскалываются и гибнут"),
        (Mode::Regime, "Строй", "как правят странами мира: монархии, республики, диктатуры, теократии"),
        (Mode::Relations, "Отношения", "что правительства мира думают о нас: от красного (недруги) к зелёному (друзья); синие — наши союзники, тёмно-красные — те, с кем мы воюем"),
        (Mode::Armies, "Армии", "в войну: ваша земля цветом той группы армий, чей это участок фронта и тыл за ним"),
        (Mode::Seas, "Моря", "морские зоны и пути нашей торговли (толщина — сколько везут, красным — где топят); в войну — кто держит море; знаки — эскадры: П порт, Г господство, К конвои, Р рейды, Б блокада"),
    ];

    /// The colour of a value 0..1 in the mode (supply: red to green; the rest: pale to dark, as on
    /// the page).
    pub fn ramp(self, t: f32) -> [f32; 3] {
        const RAMP: [[f32; 3]; 5] = [[243.0, 234.0, 208.0], [233.0, 199.0, 126.0], [219.0, 141.0, 66.0], [184.0, 83.0, 43.0], [123.0, 36.0, 25.0]];
        const SUPPLY: [[f32; 3]; 3] = [[168.0, 48.0, 42.0], [217.0, 165.0, 49.0], [76.0, 140.0, 58.0]];
        // (the heights as the card colours them: `hypsoColour` in the land's shader)
        const HYPSO: [[f32; 3]; 8] = [[51.0, 97.0, 77.0], [92.0, 143.0, 92.0], [143.0, 179.0, 107.0], [209.0, 204.0, 133.0], [204.0, 163.0, 102.0], [158.0, 112.0, 77.0], [158.0, 148.0, 143.0], [245.0, 245.0, 245.0]];
        let stops: &[[f32; 3]] = match self {
            Mode::Supply | Mode::Relations => &SUPPLY,
            Mode::Terrain => &HYPSO,
            _ => &RAMP,
        };
        let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
        let i = (t as usize).min(stops.len() - 2);
        let f = t - i as f32;
        [0, 1, 2].map(|j| stops[i][j] + (stops[i + 1][j] - stops[i][j]) * f)
    }

    /// The mode's icon (of the sprite).
    pub fn icon(self) -> &'static str {
        match self {
            Mode::Political => "flag",
            Mode::Terrain => "mountains",
            Mode::Supply => "crate",
            Mode::Dev => "gears",
            Mode::People => "people",
            Mode::Res => "mining",
            Mode::Lost => "ruins",
            Mode::Faith => "faith",
            Mode::Regime => "vote",
            Mode::Relations => "quill",
            Mode::Armies => "staff",
            Mode::Seas => "anchor",
        }
    }

    /// The words at the ends of the mode's scale.
    pub fn ends(self) -> Option<(&'static str, &'static str)> {
        match self {
            Mode::Supply => Some(("скудно", "вдоволь")),
            Mode::Terrain => Some(("ниже моря", "горы")),
            Mode::Dev => Some(("отсталые", "развитые")),
            Mode::People => Some(("мало людей", "много")),
            Mode::Res => Some(("нет недр", "богатые")),
            Mode::Relations => Some(("недруги", "друзья")),
            _ => None,
        }
    }
}

/// Metres of the highest peak (the heights in words: the card of a province).
const PEAK_M: f32 = 5600.0;

/// What the table of provinces is made from (`MapView::paint_of`).
struct Paint {
    /// Each province: the country shown on it, whether the enemy of its owner holds it, its side
    /// in the war (9: none), the country it belongs to at home; its colour in the map's mode, if
    /// the mode colours it.
    shown: Vec<(u32, bool, u8, u32)>,
    modal: Vec<Option<[f32; 3]>>,
    tints: Vec<[f32; 3]>,
    mode: Mode,
}

/// A country's name laid on its land, as on a painted map: along the land's long axis, bent along
/// its middle line (world pixels).
pub struct Label {
    name: String,
    /// The middle of the land, the axis (radians), half the length the name takes along it, the
    /// bend (the middle line's offset across the axis: a·u² + b·u + c), the letters' height.
    mx: f32,
    my: f32,
    angle: f32,
    half: f32,
    bend: [f32; 3],
    size: f32,
}

impl Label {
    /// The point of the name's line at `u` along the axis (world pixels).
    fn at(&self, u: f32) -> (f32, f32) {
        let v = self.bend[0] * u * u + self.bend[1] * u + self.bend[2];
        let (sn, cs) = self.angle.sin_cos();
        (self.mx + u * cs - v * sn, self.my + u * sn + v * cs)
    }
}

/// The parabola a·u² + b·u + c nearest the points (least squares); a line or a point if there
/// are too few.
fn fit2(p: &[(f32, f32)]) -> [f32; 3] {
    if p.len() < 3 {
        let c = p.iter().map(|q| q.1).sum::<f32>() / p.len().max(1) as f32;
        return [0.0, 0.0, c];
    }
    // the normal equations, solved by Cramer's rule
    let mut m = [[0.0f64; 3]; 3];
    let mut r = [0.0f64; 3];
    for &(u, v) in p {
        let (u, v) = (u as f64, v as f64);
        let b = [u * u, u, 1.0];
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += b[i] * b[j];
            }
            r[i] += b[i] * v;
        }
    }
    let det = |a: [[f64; 3]; 3]| a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1]) - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0]) + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
    let d = det(m);
    if d.abs() < 1e-9 {
        return [0.0, 0.0, (r[2] / m[2][2].max(1.0)) as f32];
    }
    let mut out = [0.0f32; 3];
    for k in 0..3 {
        let mut a = m;
        for i in 0..3 {
            a[i][k] = r[i];
        }
        out[k] = (det(a) / d) as f32;
    }
    out
}

/// What the map asks of the window after a frame.
#[derive(Default)]
pub struct Asks {
    pub group: Option<Option<usize>>,
    /// A group added to the chosen or taken out of them (Shift and a click on its badge).
    pub toggle: Option<usize>,
    pub battle: Option<usize>,
    pub open_army: bool,
    /// A country clicked while the player chooses his (the start of a game).
    pub country: Option<usize>,
}

pub struct MapView {
    /// The world point in the middle of the view, and screen points per world pixel there.
    pub cx: f32,
    pub cy: f32,
    pub k: f32,
    pub mode: Mode,
    /// What the next click builds: a depot of supply, a line of supply (from its first province,
    /// once chosen), nothing.
    pub build: Build,
    /// The player chooses his country (the start of a game): a click on the map names one.
    pub picking: bool,
    /// The world's ways as on the video card (the steps to them), with the lines of supply laid
    /// since put on them; how many provinces of each line are on them.
    ways_now: Vec<u8>,
    lines_now: Vec<u8>,
    laid: Vec<usize>,
    /// Each line of supply under way, as a line on the land (by its number), for the dashes.
    planned: std::collections::HashMap<usize, Vec<[f32; 2]>>,
    /// The height of the highest peak (world pixels).
    top: f32,
    /// The land on the video card, and the world's layers it was made of.
    scene: Arc<Mutex<scene::Scene>>,
    ground: Arc<scene::Layers>,
    /// What the table of provinces was made for: the owners of the theatre, the mode, the month
    /// (supply), a war on.
    made_for: (Vec<usize>, u8, u32, bool),
    /// The player moved the view (else it is fitted again to the space the panels leave), and the
    /// space it was fitted to.
    moved: bool,
    fit_rect: Rect,
    labels: Vec<Label>,
    fitted: bool,
    /// The small map in the corner.
    mini: Option<TextureHandle>,
    /// A zoom to make once the view is fitted, and a place to look at (for pictures of the window).
    pub zoom_once: f32,
    pub center_once: Option<(f32, f32)>,
    /// The borders as smooth lines (from `shapes`), for the map close up: points, bounds, the
    /// provinces on either side (u32::MAX: water, a lake, the map's edge).
    borders: Vec<(Vec<[f32; 2]>, [f32; 4], u32, u32)>,
    /// What was sent to the video card each time the land changed hands or the mode changed
    /// (the table of provinces or everything, how many provinces changed, seconds, the share of
    /// the world's layers sent).
    pub painted: Vec<(&'static str, usize, f32, f32)>,
    /// Where the map was last drawn on the screen, its small map, the counters on it (for the
    /// window's self-checks), the camera of the last frame.
    pub last_rect: Rect,
    pub last_mini: Rect,
    pub counters: std::sync::Mutex<Vec<Rect>>,
    pub cam: Camera,
    born: std::time::Instant,
    /// The era's ground (see `scene::Frame::era`).
    era: f32,
    /// Where each group's counter is now on its way (world pixels; it glides to where the group
    /// stands), and the seconds since the last frame.
    anim: Mutex<std::collections::HashMap<u64, (f32, f32)>>,
    /// The tops of the factories' chimneys (world pixels: x, y, height above the land), for their
    /// smoke close up.
    chimneys: Vec<(f32, f32, f32)>,
    /// The houses of the towns (x, y, size, turn) and the streets and roads (for the self-check:
    /// each house's front to a street).
    houses: Vec<[f32; 4]>,
    streets: Vec<Vec<[f32; 2]>>,
    dt: f32,
    /// The divisions the player chose (their ids); where he began drawing a box to choose them
    /// (the left button held down over the map in a war of divisions).
    pub picked: Vec<u32>,
    pub boxing: Option<Pos2>,
    /// The plan being drawn; the arrows drawn and not yet carried out (the group, the head: a
    /// province of the theatre); the group chosen this frame (its sector is lit along the front);
    /// where the chosen divisions would go.
    pub tool: Tool,
    /// Where the stroke of the front tool began and where the hand is now (world points: the
    /// stretch snaps to them, not to the middles of the provinces under them).
    pub stroke: Option<[(f32, f32); 2]>,
    pub drafts: Vec<(usize, usize)>,
    pub chosen: Option<usize>,
    pub preview: Option<Preview>,
    /// The other chosen groups (with Shift), as the window keeps them; the groups' badges drawn
    /// close up (where, which group).
    pub also: Vec<usize>,
    pub badges: Mutex<Vec<(Rect, usize)>>,
    /// Where our stacks of divisions were drawn (the counter, the divisions, the province), the
    /// enemy's (the counter, the province), the fights going on (the plate, the fight).
    pub stacks_drawn: Mutex<Vec<(Rect, Vec<u32>, usize)>>,
    pub foe_drawn: Mutex<Vec<(Rect, usize)>>,
    pub fights_drawn: Mutex<Vec<(Rect, usize)>>,
    /// The troops' figures standing by the stacks this frame (the video card draws them with the
    /// land).
    figures: Arc<Mutex<Vec<figures::Figure>>>,
}

impl MapView {
    /// What the video card spent on the frames drawn so far: the land, the trees and houses (s).
    pub fn spent(&self) -> Vec<(f32, f32)> {
        self.scene.lock().unwrap().spent.clone()
    }

    pub fn new(g: &Game) -> MapView {
        // what the land carries: the rivers, the towns and the villages round which it is tilled,
        // the ways between them (railways in the ages that have them)
        let land = g.land.clone();
        let villages = &land.villages;
        let mut farms: Vec<(f32, f32, f32)> = g.towns.iter().map(|t| {
            let (x, y) = Self::at(g, t.0);
            (x, y, match t.2 { 3 => 12.0, 2 => 9.0, _ => 6.5 })
        }).collect();
        farms.extend(villages.iter().map(|&v| {
            let (x, y) = Self::at(g, v);
            (x, y, 4.5)
        }));
        // the towns and the villages laid out: their streets (drawn on the land with the ways),
        // the built-up ground round their middles, their buildings
        let mut towns: Vec<(f32, f32, u8)> = g.towns.iter().map(|t| {
            let (x, y) = Self::at(g, t.0);
            (x, y, t.2)
        }).collect();
        towns.extend(villages.iter().map(|&v| {
            let (x, y) = Self::at(g, v);
            (x, y, 0u8)
        }));
        let era = match g.pack.theme.as_str() {
            "rust" => 1.0,
            "parchment" => 2.0,
            _ => 0.0,
        };
        let rivers: Vec<(Vec<[f32; 2]>, f32)> = land.rivers.iter().map(|(p, _, wd)| (p.clone(), *wd)).collect();
        let dry = |x: f32, y: f32| {
            let (ww, wh) = (g.world.w, g.world.h);
            let i = (y.floor().clamp(0.0, wh as f32 - 1.0) as usize) * ww + (x.floor() as i64).rem_euclid(ww as i64) as usize;
            g.world.height[i] > 0.0 && !matches!(g.world.terrain[i], strategy::world::map::Terrain::Lake | strategy::world::map::Terrain::Ocean)
        };
        let layout = towns::lay(&towns, &land.ways, &rivers, &dry, era);
        let mut all_ways = land.ways.clone();
        all_ways.extend(layout.streets.iter().cloned());
        let extra = ground::Extra { rivers, ways: all_ways, farms, cores: layout.cores.clone() };
        let ground = Arc::new(ground::layers(&g.world, &extra));
        // the countries' names: along the long axis of their land, bent along its middle line
        let w = g.world.w;
        let n = g.countries.len();
        let mut pts: Vec<Vec<(f32, f32)>> = vec![Vec::new(); n];
        for i in (0..g.world.owner.len()).step_by(3) {
            let p = g.world.owner[i];
            if p == NONE {
                continue;
            }
            let c = g.country_of[p as usize];
            if c == u32::MAX {
                continue;
            }
            pts[c as usize].push(((i % w) as f32, (i / w) as f32));
        }
        let labels = pts
            .iter()
            .enumerate()
            .filter(|(_, v)| v.len() > 400)
            .map(|(c, v)| {
                // (x unwrapped around the first point: the map goes round)
                let x0 = v[0].0;
                let wf = w as f32;
                let un: Vec<(f32, f32)> = v.iter().map(|&(x, y)| (if x - x0 > wf / 2.0 { x - wf } else if x0 - x > wf / 2.0 { x + wf } else { x }, y)).collect();
                let k = un.len() as f32;
                let (mx, my) = (un.iter().map(|p| p.0).sum::<f32>() / k, un.iter().map(|p| p.1).sum::<f32>() / k);
                let (mut sxx, mut syy, mut sxy) = (0.0f32, 0.0f32, 0.0f32);
                for &(x, y) in &un {
                    let (dx, dy) = (x - mx, y - my);
                    sxx += dx * dx;
                    syy += dy * dy;
                    sxy += dx * dy;
                }
                let mut angle = 0.5 * (2.0 * sxy).atan2(sxx - syy);
                // (read left to right, never steeper than 45°)
                if angle.cos() < 0.0 {
                    angle += std::f32::consts::PI;
                }
                angle = angle.clamp(-0.8, 0.8);
                let (sn, cs) = angle.sin_cos();
                let uv: Vec<(f32, f32)> = un.iter().map(|&(x, y)| ((x - mx) * cs + (y - my) * sn, -(x - mx) * sn + (y - my) * cs)).collect();
                let mut us: Vec<f32> = uv.iter().map(|p| p.0).collect();
                us.sort_by(|a, b| a.total_cmp(b));
                let (u0, u1) = (us[us.len() / 20], us[us.len() * 19 / 20]);
                // the middle line: the mean offset across the axis in eight stretches along it,
                // fitted by a gentle parabola
                let bins = 8;
                let mut acc = vec![(0.0f32, 0.0f32); bins];
                for &(u, vv) in &uv {
                    let b = (((u - u0) / (u1 - u0).max(1.0)) * bins as f32).floor();
                    if b >= 0.0 && (b as usize) < bins {
                        acc[b as usize].0 += vv;
                        acc[b as usize].1 += 1.0;
                    }
                }
                let mid: Vec<(f32, f32)> = acc.iter().enumerate().filter(|(_, a)| a.1 > 0.0).map(|(i, a)| (u0 + (u1 - u0) * (i as f32 + 0.5) / bins as f32, a.0 / a.1)).collect();
                let bend = fit2(&mid);
                let spread = (uv.iter().map(|p| p.1 * p.1).sum::<f32>() / k).sqrt();
                let name = g.countries[c].0.to_uppercase();
                let letters = name.chars().count().max(1) as f32;
                let half = 0.36 * (u1 - u0);
                let size = (1.1 * spread).min(2.0 * half / letters * 1.15).clamp(4.0, 80.0);
                // (the bend no deeper than a third of the letters' height over the name's length)
                let curve = bend[0].clamp(-0.35 * size / (half * half).max(1.0), 0.35 * size / (half * half).max(1.0));
                Label { name, mx: mx.rem_euclid(wf), my, angle, half, bend: [curve, bend[1].clamp(-0.3, 0.3), bend[2].clamp(-spread, spread)], size }
            })
            .collect();
        let mut scene = scene::Scene::new(ground.clone());
        // (kept for the self-check of the towns: the houses along the streets, and the streets)
        let houses: Vec<[f32; 4]> = [objects::COTTAGE, objects::TOWNHOUSE, objects::BARN, objects::HIPHOUSE, objects::BLOCK, objects::ORDINARY_A, objects::ORDINARY_B, objects::OLDHOUSE, objects::COTTAGE_B, objects::FARM_A, objects::FARM_B, objects::BRICK, objects::PANEL].iter().flat_map(|&k| layout.built.builds[k].iter().copied()).collect();
        let streets: Vec<Vec<[f32; 2]>> = extra.ways.iter().filter(|l| l.kind != ways::RAIL).map(|l| l.pts.clone()).collect();
        let built = layout.built;
        // (each factory's two chimneys, turned as the video card turns the factory)
        let chimneys: Vec<(f32, f32, f32)> = built.builds[objects::FACTORY]
            .iter()
            .flat_map(|f| {
                let (sn, cs) = (f[3] * std::f32::consts::TAU).sin_cos();
                [(0.3f32, -0.1f32, 1.15f32), (0.1, -0.1, 0.95)].map(|(x, y, z)| (f[0] + (cs * x - sn * y) * f[2], f[1] + (sn * x + cs * y) * f[2], z * f[2]))
            })
            .collect();
        scene.set_houses(built);
        MapView {
            cx: 0.0,
            cy: 0.0,
            k: 1.0,
            mode: Mode::Political,
            build: Build::None,
            picking: false,
            ways_now: ground.ways.clone(),
            lines_now: ground.lines.clone(),
            laid: Vec::new(),
            planned: Default::default(),
            top: ground.height.iter().cloned().fold(1.0f32, f32::max) * RELIEF,
            scene: Arc::new(Mutex::new(scene)),
            ground,
            made_for: (Vec::new(), 255, 0, false),
            moved: false,
            fit_rect: Rect::NOTHING,
            labels,
            fitted: false,
            mini: None,
            zoom_once: 1.0,
            center_once: None,
            borders: Self::borders(g),
            painted: Vec::new(),
            last_rect: Rect::NOTHING,
            last_mini: Rect::NOTHING,
            counters: Default::default(),
            cam: Camera::new(0.0, 0.0, 1.0, Rect::from_min_size(Pos2::ZERO, vec2(1.0, 1.0)), w as f32, false),
            born: std::time::Instant::now(),
            anim: Default::default(),
            chimneys,
            houses,
            streets,
            dt: 0.0,
            picked: Vec::new(),
            boxing: None,
            tool: Tool::None,
            stroke: None,
            drafts: Vec::new(),
            chosen: None,
            preview: None,
            also: Vec::new(),
            badges: Mutex::new(Vec::new()),
            stacks_drawn: Mutex::new(Vec::new()),
            foe_drawn: Mutex::new(Vec::new()),
            fights_drawn: Mutex::new(Vec::new()),
            figures: Arc::new(Mutex::new(Vec::new())),
            era: match g.pack.theme.as_str() {
                "rust" => 1.0,
                "parchment" => 2.0,
                _ => 0.0,
            },
        }
    }

    /// A counter's place on its way to `to` (world pixels): it glides there in about half a
    /// second, the shorter way round the world; a far jump (a new front) is made at once.
    pub(super) fn glide(&self, key: u64, to: (f32, f32)) -> (f32, f32) {
        let w = self.ground.w as f32;
        if crate::native::spoiled() {
            self.anim.lock().unwrap().insert(key, to);
            return to;
        }
        let mut m = self.anim.lock().unwrap();
        let e = m.entry(key).or_insert(to);
        let mut dx = to.0 - e.0;
        if dx > w / 2.0 {
            dx -= w;
        } else if dx < -w / 2.0 {
            dx += w;
        }
        let dy = to.1 - e.1;
        if dx.hypot(dy) > 250.0 {
            *e = to;
            return to;
        }
        let a = 1.0 - (-self.dt * 5.0).exp();
        e.0 += dx * a;
        e.1 += dy * a;
        *e
    }

    /// Where the groups' counters are on their way (for the window's self-checks).
    pub fn glides(&self) -> std::collections::HashMap<u64, (f32, f32)> {
        self.anim.lock().unwrap().clone()
    }

    /// Seconds since the map was made (for what moves on it).
    pub(super) fn t(&self) -> f32 {
        self.born.elapsed().as_secs_f32() % 3600.0
    }

    /// The land's height at a world point, as the camera sees it (world pixels).
    pub fn z(&self, x: f32, y: f32) -> f32 {
        self.ground.z(x, y)
    }

    /// A world point (on the land) on the screen, as the map was last drawn.
    pub fn pt(&self, (x, y): (f32, f32)) -> Pos2 {
        self.cam.to_screen(x, y, self.z(x, y))
    }

    /// The fog of war, by the provinces of the theatre: the enemy's land more than two steps from
    /// ours — the staff does not see it (its troops there are guesses: the counters say «≈»).
    /// (The spoiled control: no fog.)
    pub fn fogged(g: &Game) -> Vec<bool> {
        let war = &g.h.war;
        if !g.h.at_war || crate::native::spoiled() {
            return Vec::new();
        }
        let own: Vec<usize> = (0..war.owner.len()).filter(|&q| war.owner[q] == g.me).collect();
        let d = war.distance_from_all(&own);
        (0..war.owner.len()).map(|q| war.owner[q] != g.me && d[q] != NO && d[q] > 2).collect()
    }

    /// The layers of the lines as the video card has them: the rivers' and main roads' steps,
    /// the village roads' and railways' steps, the detail (the rivers' half-widths in its third
    /// channel); the world's size.
    pub fn line_layers(&self) -> (&[u8], &[u8], &[u8], usize, usize) {
        (&self.ground.lines, &self.ground.ways, &self.ground.detail, self.ground.w, self.ground.h)
    }

    /// A province's point on the map (world pixels).
    /// The chosen groups: the first chosen, then the others added with Shift.
    pub fn chosen_all(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.chosen.into_iter().collect();
        for &k in &self.also {
            if !v.contains(&k) {
                v.push(k);
            }
        }
        v
    }
}
