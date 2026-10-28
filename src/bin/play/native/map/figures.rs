//! The troops on the map as figures (as HOI4 stands a model by each stack of divisions): a
//! soldier of the era — a steel helmet and a rifle, a shako, a pack and a musket with its
//! bayonet, a raider's hood — a tank, a field gun, a cannon, a horseman, a technical, an
//! armoured rig; in the country's uniform (its colour worked into the era's khaki), lit as the
//! land and the houses are, each with its shadow; marching, their legs step.
//!
//! The models come from free 3D models of the world's artists (`assets/figures`, made by
//! `tools/figures3d.py`: a rigged soldier posed by his own animations — standing, four steps of the
//! march, firing — with the era's headgear and arm; a horse walking and galloping with its rider; a
//! tank, a field gun, a cannon, a technical, a rig); a figure whose file is missing is made here of
//! limbs, boxes and lumps, as before. A figure stands a few dozen points tall on the screen, like a
//! painted miniature on a staff map.

use eframe::glow::{self, HasContext};
use super::mesh::{norm, push, Model};

/// The figures: the model each stands for.
pub const SOLDIER: usize = 0;
pub const MUSKETEER: usize = 1;
pub const RAIDER: usize = 2;
pub const TANK: usize = 3;
pub const GUN: usize = 4;
pub const CANNON: usize = 5;
pub const RIDER: usize = 6;
pub const TECHNICAL: usize = 7;
pub const RIG: usize = 8;
pub const MODELS: usize = 9;

/// A figure in the world: where (world pixels), how tall (world pixels), which way it faces
/// (radians, its front along +x), its uniform's colour, what it does (0 — stands, 1 — marches,
/// 2 — fights).
#[derive(Clone, Copy, Debug)]
pub struct Figure {
    pub model: usize,
    pub at: [f32; 2],
    pub size: f32,
    pub angle: f32,
    pub colour: [f32; 3],
    pub doing: f32,
}

/// A model: vertices of (x, y, z, normal x, y, z, part, light). The part is the paint (`PAINTS`
/// in the shader) and, by tens, the limb that moves: 10 — a left leg, 20 — a right leg, 30 — a
/// horse's fore legs, 40 — its hind legs.
const FLOATS: usize = 8;

// the paints
const CLOTH: f32 = 0.0;
const SKIN: f32 = 1.0;
const LEATHER: f32 = 2.0;
const STEEL: f32 = 3.0;
const WOOD: f32 = 4.0;
const PAINTED: f32 = 5.0;
const WHITE: f32 = 6.0;
const BRASS: f32 = 7.0;
const HORSE: f32 = 8.0;
const RUBBER: f32 = 9.0;
const LEFT: f32 = 10.0;
const RIGHT: f32 = 20.0;
const FORE: f32 = 30.0;
const HIND: f32 = 40.0;

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// A limb from `a` (radius `r0`) to `b` (radius `r1`), its ends closed; darker at `ao0` end.
fn limb(m: &mut Model, a: [f32; 3], b: [f32; 3], r0: f32, r1: f32, part: f32, ao0: f32, ao1: f32) {
    let sides = 8;
    let d = norm([b[0] - a[0], b[1] - a[1], b[2] - a[2]]);
    let side = if d[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] };
    let u = norm(cross(d, side));
    let v = cross(d, u);
    let ring = |c: [f32; 3], r: f32, ang: f32| {
        let (s, co) = ang.sin_cos();
        let n = [u[0] * co + v[0] * s, u[1] * co + v[1] * s, u[2] * co + v[2] * s];
        ([c[0] + n[0] * r, c[1] + n[1] * r, c[2] + n[2] * r], n)
    };
    let tau = std::f32::consts::TAU;
    for s in 0..sides {
        let (a0, a1) = (s as f32 / sides as f32 * tau, (s + 1) as f32 / sides as f32 * tau);
        let (p00, n0) = ring(a, r0, a0);
        let (p10, n1) = ring(a, r0, a1);
        let (p01, _) = ring(b, r1, a0);
        let (p11, _) = ring(b, r1, a1);
        for (p, n, ao) in [(p00, n0, ao0), (p10, n1, ao0), (p11, n1, ao1), (p00, n0, ao0), (p11, n1, ao1), (p01, n0, ao1)] {
            push(m, p, n, part, ao);
        }
        // (the ends)
        push(m, a, [-d[0], -d[1], -d[2]], part, ao0);
        push(m, p10, [-d[0], -d[1], -d[2]], part, ao0);
        push(m, p00, [-d[0], -d[1], -d[2]], part, ao0);
        push(m, b, d, part, ao1);
        push(m, p01, d, part, ao1);
        push(m, p11, d, part, ao1);
    }
}

/// A lump: an ellipsoid about `c` of half-sizes `r` (a head, a helmet's dome, a horse's body),
/// from `z_from` of its height up (−1: whole, 0: the upper half).
fn lump(m: &mut Model, c: [f32; 3], r: [f32; 3], part: f32, z_from: f32, ao: f32) {
    let (seg, rings) = (10usize, 6usize);
    let lo = z_from.clamp(-1.0, 1.0).asin();
    let at = |i: usize, j: usize| {
        let a = (i % seg) as f32 / seg as f32 * std::f32::consts::TAU;
        let b = lo + j as f32 / rings as f32 * (std::f32::consts::FRAC_PI_2 - lo);
        let (x, y, z) = (b.cos() * a.cos(), b.cos() * a.sin(), b.sin());
        ([c[0] + x * r[0], c[1] + y * r[1], c[2] + z * r[2]], [x / r[0], y / r[1], z / r[2]], ao * (0.75 + 0.25 * (z * 0.5 + 0.5)))
    };
    for j in 0..rings {
        for i in 0..seg {
            let (p00, p10, p11, p01) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
            for (p, n, a) in [p00, p10, p11, p00, p11, p01] {
                push(m, p, n, part, a);
            }
        }
    }
    if z_from > -0.99 {
        // (the cut closed underneath)
        for i in 0..seg {
            let (a, b) = (at(i, 0), at(i + 1, 0));
            push(m, [c[0], c[1], a.0[2]], [0.0, 0.0, -1.0], part, ao * 0.6);
            push(m, b.0, [0.0, 0.0, -1.0], part, ao * 0.6);
            push(m, a.0, [0.0, 0.0, -1.0], part, ao * 0.6);
        }
    }
}

fn quad(m: &mut Model, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3], part: f32, ao: f32) {
    let n = norm(cross([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]));
    for p in [a, b, c, a, c, d] {
        push(m, p, n, part, ao);
    }
}

/// A box from `lo` to `hi`, darker at its foot; `top` — how much narrower its top is (a hull's
/// sloped armour), 0 — straight.
fn boxed(m: &mut Model, lo: [f32; 3], hi: [f32; 3], top: f32, part: f32) {
    let (x0, y0, z0, x1, y1, z1) = (lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]);
    let (tx, ty) = ((x1 - x0) * top * 0.5, (y1 - y0) * top * 0.5);
    let b = [[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]];
    let t = [[x0 + tx, y0 + ty, z1], [x1 - tx, y0 + ty, z1], [x1 - tx, y1 - ty, z1], [x0 + tx, y1 - ty, z1]];
    quad(m, t[0], t[1], t[2], t[3], part, 1.0);
    for i in 0..4 {
        let j = (i + 1) % 4;
        // (outward: the sides wound so their normals face out)
        let n = norm(cross([b[j][0] - b[i][0], b[j][1] - b[i][1], b[j][2] - b[i][2]], [0.0, 0.0, 1.0]));
        for (p, ao) in [(b[i], 0.55), (b[j], 0.55), (t[j], 1.0), (b[i], 0.55), (t[j], 1.0), (t[i], 1.0)] {
            push(m, p, n, part, ao);
        }
    }
}

/// A wheel about `c`, of radius `r`, `w` wide across y.
fn wheel(m: &mut Model, c: [f32; 3], r: f32, w: f32, part: f32) {
    limb(m, [c[0], c[1] - w / 2.0, c[2]], [c[0], c[1] + w / 2.0, c[2]], r, r, part, 0.7, 0.7);
}

/// A soldier of the era, `kind`: SOLDIER (a steel helmet, a greatcoat, a rifle at the port),
/// MUSKETEER (a tall shako, white crossbelts, a musket with its bayonet), RAIDER (a hood, a
/// poncho, a short gun). His front along +x, standing at the origin, one unit tall.
pub fn soldier(kind: usize) -> Model {
    let mut m = Model::new();
    let hip = 0.47;
    // legs and boots (the legs step on the march)
    for (s, limbp) in [(-1.0f32, LEFT), (1.0, RIGHT)] {
        let y = 0.055 * s;
        limb(&mut m, [0.0, y, hip], [0.01, y * 1.05, 0.25], 0.055, 0.045, CLOTH + limbp, 0.7, 0.8);
        limb(&mut m, [0.01, y * 1.05, 0.25], [0.0, y * 1.1, 0.06], 0.045, 0.04, if kind == MUSKETEER { WHITE } else { CLOTH } + limbp, 0.8, 0.6);
        limb(&mut m, [-0.01, y * 1.1, 0.07], [0.0, y * 1.1, 0.0], 0.045, 0.048, LEATHER + limbp, 0.6, 0.5);
        boxed(&mut m, [-0.03, y * 1.1 - 0.035, 0.0], [0.07, y * 1.1 + 0.035, 0.035], 0.2, LEATHER + limbp);
    }
    // the body: a coat to the knees (the raider's poncho wider)
    let (skirt, chest) = if kind == RAIDER { (0.17, 0.13) } else { (0.13, 0.115) };
    limb(&mut m, [0.0, 0.0, 0.31], [0.0, 0.0, hip + 0.02], skirt, 0.105, CLOTH, 0.55, 0.9);
    limb(&mut m, [0.0, 0.0, hip], [0.0, 0.0, 0.80], 0.10, chest, CLOTH, 0.85, 1.0);
    limb(&mut m, [0.0, 0.0, 0.79], [0.0, 0.0, 0.83], chest, 0.06, CLOTH, 1.0, 1.0);
    // a belt; the musketeer's crossbelts, white
    limb(&mut m, [0.0, 0.0, 0.48], [0.0, 0.0, 0.51], 0.107, 0.107, LEATHER, 0.9, 0.9);
    if kind == MUSKETEER {
        for s in [-1.0f32, 1.0] {
            limb(&mut m, [0.095, -0.07 * s, 0.79], [0.10, 0.075 * s, 0.53], 0.014, 0.014, WHITE, 1.0, 1.0);
            limb(&mut m, [-0.095, -0.07 * s, 0.79], [-0.10, 0.075 * s, 0.53], 0.014, 0.014, WHITE, 1.0, 1.0);
        }
    }
    // the pack on his back (a rolled blanket over it, for the musketeer)
    boxed(&mut m, [-0.20, -0.08, 0.55], [-0.10, 0.08, 0.77], 0.1, if kind == SOLDIER { CLOTH } else { LEATHER });
    if kind == MUSKETEER {
        limb(&mut m, [-0.16, -0.09, 0.79], [-0.16, 0.09, 0.79], 0.035, 0.035, CLOTH, 0.9, 0.9);
    }
    // the head, the neck
    limb(&mut m, [0.0, 0.0, 0.82], [0.0, 0.0, 0.87], 0.035, 0.035, SKIN, 0.7, 0.9);
    lump(&mut m, [0.01, 0.0, 0.915], [0.06, 0.055, 0.068], SKIN, -1.0, 1.0);
    // the headgear
    match kind {
        MUSKETEER => {
            // a shako: tall, a peak, a pompon
            limb(&mut m, [0.0, 0.0, 0.95], [0.0, 0.0, 1.10], 0.062, 0.072, LEATHER, 0.8, 1.0);
            boxed(&mut m, [0.03, -0.05, 0.945], [0.10, 0.05, 0.96], 0.0, LEATHER);
            lump(&mut m, [0.06, 0.0, 1.11], [0.02, 0.02, 0.03], WHITE, -1.0, 1.0);
            limb(&mut m, [0.0, 0.0, 1.0], [0.0, 0.0, 1.012], 0.066, 0.067, BRASS, 1.0, 1.0);
        }
        RAIDER => {
            // a hood over the head, a scarf over the face
            lump(&mut m, [-0.005, 0.0, 0.93], [0.08, 0.072, 0.09], CLOTH, -0.2, 0.9);
            limb(&mut m, [0.035, 0.0, 0.875], [0.06, 0.0, 0.895], 0.05, 0.045, LEATHER, 0.8, 0.8);
        }
        _ => {
            // a steel helmet: a dome and its rim
            lump(&mut m, [0.0, 0.0, 0.94], [0.085, 0.08, 0.075], PAINTED, 0.0, 1.0);
            limb(&mut m, [0.0, 0.0, 0.935], [0.0, 0.0, 0.945], 0.105, 0.10, PAINTED, 0.8, 0.9);
        }
    }
    // the arms, both hands on the weapon held across the chest (the muzzle up to his left)
    let (lh, rh) = ([0.13, -0.07, 0.70], [0.13, 0.06, 0.55]);
    for (sh, hand) in [([0.0, -0.13, 0.78], lh), ([0.0, 0.13, 0.78], rh)] {
        let el = [0.06, sh[1] * 1.15, 0.60];
        limb(&mut m, sh, el, 0.042, 0.036, CLOTH, 0.9, 0.85);
        limb(&mut m, el, hand, 0.036, 0.03, CLOTH, 0.85, 0.9);
        lump(&mut m, hand, [0.03, 0.03, 0.03], SKIN, -1.0, 1.0);
    }
    // the weapon: the stock low by the right hand, the barrel up past the left shoulder
    let (butt, muzzle) = match kind {
        MUSKETEER => ([0.10, 0.13, 0.42], [0.17, -0.16, 1.12]),
        RAIDER => ([0.12, 0.10, 0.50], [0.16, -0.10, 0.88]),
        _ => ([0.10, 0.12, 0.44], [0.17, -0.14, 1.02]),
    };
    let mid = [butt[0] + (muzzle[0] - butt[0]) * 0.42, butt[1] + (muzzle[1] - butt[1]) * 0.42, butt[2] + (muzzle[2] - butt[2]) * 0.42];
    limb(&mut m, butt, mid, 0.026, 0.02, WOOD, 0.9, 1.0);
    limb(&mut m, mid, muzzle, 0.012, 0.011, STEEL, 1.0, 1.0);
    if kind == MUSKETEER {
        let tip = [muzzle[0] + 0.012, muzzle[1] - 0.03, muzzle[2] + 0.12];
        limb(&mut m, muzzle, tip, 0.006, 0.002, WHITE, 1.0, 1.0);
    }
    m
}

/// A tank, its gun to the front (+x): a hull of sloped armour on two tracks, a turret, its gun.
pub fn tank() -> Model {
    let mut m = Model::new();
    for s in [-1.0f32, 1.0] {
        let y = 0.30 * s;
        boxed(&mut m, [-0.62, y - 0.11, 0.02], [0.62, y + 0.11, 0.20], 0.15, RUBBER);
        for k in 0..6 {
            wheel(&mut m, [-0.48 + k as f32 * 0.19, y + 0.115 * s, 0.10], 0.075, 0.02, STEEL);
        }
    }
    boxed(&mut m, [-0.66, -0.36, 0.17], [0.66, 0.36, 0.34], 0.18, PAINTED);
    boxed(&mut m, [0.40, -0.30, 0.17], [0.74, 0.30, 0.28], 0.45, PAINTED);
    // the turret, its hatch, the gun
    limb(&mut m, [-0.06, 0.0, 0.33], [-0.06, 0.0, 0.50], 0.25, 0.21, PAINTED, 0.7, 1.0);
    limb(&mut m, [-0.10, 0.0, 0.50], [-0.10, 0.0, 0.53], 0.08, 0.08, PAINTED, 0.9, 1.0);
    boxed(&mut m, [0.10, -0.08, 0.36], [0.26, 0.08, 0.47], 0.2, PAINTED);
    limb(&mut m, [0.24, 0.0, 0.42], [0.95, 0.0, 0.44], 0.04, 0.035, PAINTED, 1.0, 1.0);
    limb(&mut m, [0.88, 0.0, 0.44], [0.97, 0.0, 0.44], 0.05, 0.05, STEEL, 1.0, 1.0);
    m
}

/// A gun on its carriage, its barrel to the front (+x): a field gun with its shield
/// (`old`: a cannon of bronze on wooden wheels, no shield).
pub fn gun(old: bool) -> Model {
    let mut m = Model::new();
    let (r, metal, wood) = if old { (0.22, BRASS, WOOD) } else { (0.17, PAINTED, RUBBER) };
    for s in [-1.0f32, 1.0] {
        wheel(&mut m, [0.0, 0.26 * s, r], r, 0.05, wood);
        limb(&mut m, [0.0, 0.26 * s, r], [0.0, 0.24 * s, r], 0.05, 0.05, STEEL, 1.0, 1.0);
        // the trail to the ground behind
        limb(&mut m, [0.0, 0.10 * s, r + 0.02], [-0.75, 0.04 * s, 0.03], 0.035, 0.03, if old { WOOD } else { PAINTED }, 0.9, 0.7);
    }
    limb(&mut m, [0.0, -0.26, r], [0.0, 0.26, r], 0.035, 0.035, STEEL, 0.9, 0.9);
    // the barrel, raised a little; the breech
    let (b0, b1) = ([-0.15, 0.0, r + 0.08], [0.95, 0.0, r + 0.27]);
    limb(&mut m, b0, b1, if old { 0.075 } else { 0.05 }, if old { 0.055 } else { 0.04 }, metal, 0.9, 1.0);
    lump(&mut m, b0, [0.08, 0.08, 0.08], metal, -1.0, 0.9);
    if !old {
        // the shield
        quad(&mut m, [0.12, -0.24, r - 0.05], [0.12, 0.24, r - 0.05], [0.10, 0.22, r + 0.33], [0.10, -0.22, r + 0.33], PAINTED, 1.0);
        quad(&mut m, [0.12, 0.24, r - 0.05], [0.12, -0.24, r - 0.05], [0.10, -0.22, r + 0.33], [0.10, 0.22, r + 0.33], PAINTED, 0.8);
    }
    m
}

/// A horseman, to the front (+x): a horse (its legs step on the march) and its rider with his
/// shako and sabre.
pub fn rider() -> Model {
    let mut m = Model::new();
    // the horse: body, neck, head, tail, four legs
    lump(&mut m, [0.0, 0.0, 0.62], [0.36, 0.14, 0.15], HORSE, -1.0, 1.0);
    limb(&mut m, [0.26, 0.0, 0.66], [0.42, 0.0, 0.90], 0.085, 0.06, HORSE, 0.9, 1.0);
    limb(&mut m, [0.40, 0.0, 0.92], [0.58, 0.0, 0.84], 0.06, 0.04, HORSE, 1.0, 1.0);
    limb(&mut m, [0.33, 0.0, 0.97], [0.38, 0.0, 1.0], 0.012, 0.01, LEATHER, 1.0, 1.0);
    limb(&mut m, [-0.34, 0.0, 0.68], [-0.46, 0.0, 0.40], 0.03, 0.015, LEATHER, 0.9, 0.9);
    for (x, part) in [(0.22f32, FORE), (-0.24, HIND)] {
        for s in [-1.0f32, 1.0] {
            let y = 0.08 * s;
            limb(&mut m, [x, y, 0.55], [x + 0.02, y, 0.27], 0.045, 0.03, HORSE + part, 0.8, 0.7);
            limb(&mut m, [x + 0.02, y, 0.27], [x, y, 0.02], 0.028, 0.026, HORSE + part, 0.7, 0.5);
            limb(&mut m, [x, y, 0.04], [x, y, 0.0], 0.03, 0.032, LEATHER + part, 0.5, 0.4);
        }
    }
    // the saddle cloth, the rider
    boxed(&mut m, [-0.14, -0.15, 0.72], [0.10, 0.15, 0.77], 0.0, CLOTH);
    limb(&mut m, [-0.02, 0.0, 0.77], [0.0, 0.0, 1.07], 0.085, 0.10, CLOTH, 0.85, 1.0);
    for s in [-1.0f32, 1.0] {
        limb(&mut m, [0.0, 0.07 * s, 0.80], [0.12, 0.12 * s, 0.66], 0.04, 0.035, WHITE, 0.9, 0.9);
        limb(&mut m, [0.12, 0.12 * s, 0.66], [0.10, 0.13 * s, 0.50], 0.035, 0.03, LEATHER, 0.9, 0.8);
    }
    limb(&mut m, [0.0, 0.0, 1.06], [0.0, 0.0, 1.10], 0.03, 0.03, SKIN, 0.8, 0.9);
    lump(&mut m, [0.01, 0.0, 1.14], [0.055, 0.05, 0.062], SKIN, -1.0, 1.0);
    limb(&mut m, [0.0, 0.0, 1.17], [0.0, 0.0, 1.30], 0.058, 0.066, LEATHER, 0.8, 1.0);
    lump(&mut m, [0.05, 0.0, 1.31], [0.018, 0.018, 0.028], WHITE, -1.0, 1.0);
    // the sabre raised
    limb(&mut m, [0.0, 0.12, 0.98], [0.10, 0.17, 0.92], 0.03, 0.028, CLOTH, 1.0, 1.0);
    limb(&mut m, [0.10, 0.17, 0.92], [0.24, 0.20, 1.30], 0.008, 0.004, STEEL, 1.0, 1.0);
    m
}

/// A technical (a pickup with a gun on its bed), to the front (+x); `rig`: a big armoured truck
/// of scrap plates.
pub fn truck(rig: bool) -> Model {
    let mut m = Model::new();
    let k = if rig { 1.25 } else { 1.0 };
    let r = 0.13 * k;
    for x in if rig { vec![-0.45, -0.15, 0.45] } else { vec![-0.38, 0.38] } {
        for s in [-1.0f32, 1.0] {
            wheel(&mut m, [x * k, 0.25 * k * s, r], r, 0.09 * k, RUBBER);
        }
    }
    if rig {
        // armoured plates, a sloped front, a gun turret of scrap
        boxed(&mut m, [-0.70, -0.30, 0.12], [0.62, 0.30, 0.55], 0.10, PAINTED);
        boxed(&mut m, [0.40, -0.28, 0.12], [0.80, 0.28, 0.44], 0.5, PAINTED);
        boxed(&mut m, [-0.40, -0.18, 0.55], [0.05, 0.18, 0.70], 0.2, STEEL);
        limb(&mut m, [0.0, 0.0, 0.63], [0.62, 0.0, 0.66], 0.035, 0.03, STEEL, 1.0, 1.0);
        for i in 0..4 {
            let x = -0.6 + i as f32 * 0.32;
            boxed(&mut m, [x, -0.315, 0.20], [x + 0.2, -0.30, 0.48], 0.0, RUBBER);
        }
    } else {
        // the bed, the cab, the gun on its post
        boxed(&mut m, [-0.55, -0.24, 0.13], [0.55, 0.24, 0.30], 0.0, PAINTED);
        boxed(&mut m, [0.05, -0.22, 0.30], [0.40, 0.22, 0.50], 0.25, PAINTED);
        boxed(&mut m, [0.40, -0.22, 0.20], [0.62, 0.22, 0.34], 0.2, PAINTED);
        limb(&mut m, [-0.25, 0.0, 0.30], [-0.25, 0.0, 0.52], 0.025, 0.025, STEEL, 0.9, 1.0);
        limb(&mut m, [-0.35, 0.0, 0.54], [0.05, 0.0, 0.60], 0.03, 0.022, STEEL, 1.0, 1.0);
        boxed(&mut m, [-0.31, -0.06, 0.49], [-0.20, 0.06, 0.58], 0.0, STEEL);
    }
    m
}

/// The shadow's plate: a square the shader stretches away from the sun.
fn plate() -> Model {
    let mut m = Model::new();
    let n = [0.0, 0.0, 1.0];
    for p in [[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.5, 0.5, 0.0], [-0.5, -0.5, 0.0], [0.5, 0.5, 0.0], [-0.5, 0.5, 0.0]] {
        push(&mut m, p, n, 0.0, 1.0);
    }
    m
}

/// How long and how wide each model's shadow lies (its height, its breadth, in its units).
const SHADOWS: [[f32; 2]; MODELS] = [[1.0, 0.22], [1.1, 0.22], [1.0, 0.26], [0.5, 0.75], [0.45, 0.5], [0.45, 0.55], [1.25, 0.45], [0.55, 0.55], [0.7, 0.7]];

struct Mesh {
    vao: glow::VertexArray,
    inst: glow::Buffer,
    verts: i32,
}

/// A figure's frames (meshes): standing, the four steps of the march, fighting.
#[derive(Clone, Copy)]
struct Look {
    stand: usize,
    march: [usize; 4],
    fight: usize,
}

pub struct Gpu {
    program: glow::Program,
    models: Vec<Mesh>,
    looks: Vec<Look>,
    shadow: Mesh,
}

/// The figures' files (`assets/figures`, `tools/figures3d.py`): each model's frames.
macro_rules! fig {
    ($name:literal) => {
        include_bytes!(concat!("../../../../../assets/figures/", $name, ".bin")) as &[u8]
    };
}

fn files(model: usize) -> Vec<&'static [u8]> {
    match model {
        SOLDIER => vec![fig!("soldier_industrial_stand"), fig!("soldier_industrial_m0"), fig!("soldier_industrial_m1"), fig!("soldier_industrial_m2"), fig!("soldier_industrial_m3"), fig!("soldier_industrial_fight")],
        MUSKETEER => vec![fig!("soldier_musket_stand"), fig!("soldier_musket_m0"), fig!("soldier_musket_m1"), fig!("soldier_musket_m2"), fig!("soldier_musket_m3"), fig!("soldier_musket_fight")],
        RAIDER => vec![fig!("soldier_raider_stand"), fig!("soldier_raider_m0"), fig!("soldier_raider_m1"), fig!("soldier_raider_m2"), fig!("soldier_raider_m3"), fig!("soldier_raider_fight")],
        RIDER => vec![fig!("rider_stand"), fig!("rider_m0"), fig!("rider_m1"), fig!("rider_m2"), fig!("rider_m3"), fig!("rider_fight")],
        TANK => vec![fig!("tank")],
        GUN => vec![fig!("gun")],
        CANNON => vec![fig!("cannon")],
        TECHNICAL => vec![fig!("technical")],
        RIG => vec![fig!("rig")],
        _ => Vec::new(),
    }
}

/// A figure from its file: b"FIG1", the vertex count, eight floats a vertex (none if the file
/// is not one).
fn read(bytes: &[u8]) -> Option<Model> {
    if bytes.len() < 8 || &bytes[..4] != b"FIG1" {
        return None;
    }
    let n = u32::from_le_bytes(bytes[4..8].try_into().ok()?) as usize;
    let body = &bytes[8..];
    if body.len() != n * FLOATS * 4 || n == 0 {
        return None;
    }
    Some(body.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
}

/// The frame of a figure now: standing, a step of the march (by the clock, each figure in its own
/// step), fighting.
fn frame(l: &Look, f: &Figure, time: f32) -> usize {
    if f.doing > 1.5 {
        l.fight
    } else if f.doing > 0.5 {
        let own = (f.at[0] * 12.9898 + f.at[1] * 78.233).sin().abs().fract();
        let k = ((time * 2.6 + own) * 4.0).floor() as i64;
        l.march[k.rem_euclid(4) as usize]
    } else {
        l.stand
    }
}

unsafe fn compile(gl: &glow::Context, kind: u32, src: &str) -> Result<glow::Shader, String> {
    let sh = gl.create_shader(kind)?;
    gl.shader_source(sh, src);
    gl.compile_shader(sh);
    if !gl.get_shader_compile_status(sh) {
        return Err(gl.get_shader_info_log(sh));
    }
    Ok(sh)
}

unsafe fn mesh(gl: &glow::Context, model: &Model) -> Result<Mesh, String> {
    let vao = gl.create_vertex_array()?;
    let vbo = gl.create_buffer()?;
    let inst = gl.create_buffer()?;
    gl.bind_vertex_array(Some(vao));
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
    let bytes: Vec<u8> = model.iter().flat_map(|x| x.to_le_bytes()).collect();
    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
    let stride = (FLOATS * 4) as i32;
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
    gl.enable_vertex_attrib_array(1);
    gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 12);
    gl.enable_vertex_attrib_array(2);
    gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, stride, 24);
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(inst));
    gl.enable_vertex_attrib_array(3);
    gl.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, 32, 0);
    gl.vertex_attrib_divisor(3, 1);
    gl.enable_vertex_attrib_array(4);
    gl.vertex_attrib_pointer_f32(4, 4, glow::FLOAT, false, 32, 16);
    gl.vertex_attrib_divisor(4, 1);
    gl.bind_vertex_array(None);
    Ok(Mesh { vao, inst, verts: (model.len() / FLOATS) as i32 })
}

pub unsafe fn make(gl: &glow::Context) -> Result<Gpu, String> {
    let vs = compile(gl, glow::VERTEX_SHADER, VERTEX)?;
    let fs = compile(gl, glow::FRAGMENT_SHADER, FRAGMENT)?;
    let program = gl.create_program()?;
    gl.attach_shader(program, vs);
    gl.attach_shader(program, fs);
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        return Err(gl.get_program_info_log(program));
    }
    gl.delete_shader(vs);
    gl.delete_shader(fs);
    let made = [soldier(SOLDIER), soldier(MUSKETEER), soldier(RAIDER), tank(), gun(false), gun(true), rider(), truck(false), truck(true)];
    let mut models = Vec::new();
    let mut looks = Vec::new();
    for (k, m) in made.iter().enumerate() {
        // (the artists' model where its frames are all there, else the one made here)
        let frames: Vec<Model> = files(k).into_iter().filter_map(read).collect();
        let base = models.len();
        if frames.len() == 6 {
            for f in &frames {
                models.push(mesh(gl, f)?);
            }
            looks.push(Look { stand: base, march: [base + 1, base + 2, base + 3, base + 4], fight: base + 5 });
        } else if frames.len() == 1 {
            models.push(mesh(gl, &frames[0])?);
            looks.push(Look { stand: base, march: [base; 4], fight: base });
        } else {
            models.push(mesh(gl, m)?);
            looks.push(Look { stand: base, march: [base; 4], fight: base });
        }
    }
    let shadow = mesh(gl, &plate())?;
    Ok(Gpu { program, models, looks, shadow })
}

/// What the figures need of a frame: the copies of the world in view (shift, matrix), the
/// world's size, the land's height scale, the camera's distance, the clock, the camera's tilt.
pub struct View<'a> {
    pub copies: &'a [(f32, [f32; 16], [f32; 3])],
    pub size: (f32, f32),
    pub relief: f32,
    pub dist: f32,
    pub time: f32,
    pub pitch: f32,
    pub light: glow::Texture,
}

/// The figures over the land just drawn (its height map bound to unit 1).
pub unsafe fn draw(gl: &glow::Context, g: &Gpu, v: &View, figures: &[Figure]) {
    if figures.is_empty() {
        return;
    }
    gl.use_program(Some(g.program));
    let u = |name: &str| gl.get_uniform_location(g.program, name);
    gl.uniform_1_i32(u("uHeight").as_ref(), 1);
    gl.active_texture(glow::TEXTURE14);
    gl.bind_texture(glow::TEXTURE_2D, Some(v.light));
    gl.uniform_1_i32(u("uLight").as_ref(), 14);
    gl.active_texture(glow::TEXTURE0);
    gl.uniform_2_f32(u("uSize").as_ref(), v.size.0, v.size.1);
    gl.uniform_1_f32(u("uRelief").as_ref(), v.relief);
    gl.uniform_1_f32(u("uDist").as_ref(), v.dist);
    gl.uniform_1_f32(u("uTime").as_ref(), v.time);
    let (sp, cp) = v.pitch.sin_cos();
    gl.uniform_3_f32(u("uToward").as_ref(), 0.0, sp, cp);
    gl.enable(glow::DEPTH_TEST);
    gl.depth_func(glow::LEQUAL);
    gl.disable(glow::CULL_FACE);
    // the copies of each model, and of all for the shadows
    let pack = |f: &Figure| {
        // (30th session: the size is the world's own — the figure stands on the land and grows with it)
        let smoothed_size = f.size;
        [
            f.at[0],
            f.at[1],
            smoothed_size,
            f.angle,
            f.colour[0],
            f.colour[1],
            f.colour[2],
            f.doing + f.model as f32 * 4.0
        ]
    };
    let mut by: Vec<Vec<f32>> = vec![Vec::new(); g.models.len()];
    let mut all: Vec<f32> = Vec::new();
    for f in figures.iter().filter(|f| f.model < MODELS && f.model < g.looks.len()) {
        by[frame(&g.looks[f.model], f, v.time)].extend(pack(f));
        all.extend(pack(f));
    }
    let upload = |m: &Mesh, data: &[f32]| {
        let bytes: Vec<u8> = data.iter().flat_map(|x| x.to_le_bytes()).collect();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(m.inst));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STREAM_DRAW);
    };
    upload(&g.shadow, &all);
    for (k, m) in g.models.iter().enumerate() {
        upload(m, &by[k]);
    }
    let shadows: Vec<f32> = SHADOWS.iter().flatten().copied().collect();
    gl.uniform_2_f32_slice(u("uShadow").as_ref(), &shadows);
    for (_, mvp, _) in v.copies {
        gl.uniform_matrix_4_f32_slice(u("uMvp").as_ref(), false, mvp);
        // the shadows first, darkening the land
        gl.enable(glow::BLEND);
        gl.blend_func(glow::ZERO, glow::ONE_MINUS_SRC_ALPHA);
        gl.depth_mask(false);
        gl.enable(glow::POLYGON_OFFSET_FILL);
        gl.polygon_offset(-1.0, -4.0);
        gl.uniform_1_i32(u("uMode").as_ref(), 3);
        gl.bind_vertex_array(Some(g.shadow.vao));
        gl.draw_arrays_instanced(glow::TRIANGLES, 0, g.shadow.verts, (all.len() / 8) as i32);
        // the ring under each figure's feet in its country's colour (a dark figure on dark land is lost; a lit
        // ring says whose it is and where it stands)
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        gl.uniform_1_i32(u("uMode").as_ref(), 4);
        gl.draw_arrays_instanced(glow::TRIANGLES, 0, g.shadow.verts, (all.len() / 8) as i32);
        gl.disable(glow::POLYGON_OFFSET_FILL);
        gl.disable(glow::BLEND);
        gl.depth_mask(true);
        gl.uniform_1_i32(u("uMode").as_ref(), 0);
        for (k, m) in g.models.iter().enumerate() {
            let n = (by[k].len() / 8) as i32;
            if n > 0 {
                gl.bind_vertex_array(Some(m.vao));
                gl.draw_arrays_instanced(glow::TRIANGLES, 0, m.verts, n);
            }
        }
    }
    gl.bind_vertex_array(None);
}

const VERTEX: &str = include_str!("shaders/figures.vert");

const FRAGMENT: &str = include_str!("shaders/figures.frag");
