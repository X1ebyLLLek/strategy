//! The models of what stands on the land close up, made of parts: trees of five kinds (trunk,
//! limbs, clumps of leaves), the cards and shades of far trees, eight kinds of buildings (walls,
//! roofs, chimneys).

use super::*;

/// A small hash of a position (for the clumps' own lumps, the same each time the model is made).
fn bump(x: f32, y: f32, z: f32) -> f32 {
    let h = ((x * 127.1 + y * 311.7 + z * 74.7).sin() * 43758.547).fract();
    h.abs()
}

/// A cone from a ring at `z0` (radius `r`) up to its tip at `z1`, smooth to the light.
fn cone(m: &mut Model, z0: f32, z1: f32, r: f32, sides: usize, part: f32) {
    let h = z1 - z0;
    for s in 0..sides {
        let (a0, a1) = (s as f32 / sides as f32 * std::f32::consts::TAU, (s + 1) as f32 / sides as f32 * std::f32::consts::TAU);
        let am = (a0 + a1) / 2.0;
        let n = |a: f32| [a.cos() * h, a.sin() * h, r];
        push(m, [r * a0.cos(), r * a0.sin(), z0], n(a0), part, 1.0);
        push(m, [r * a1.cos(), r * a1.sin(), z0], n(a1), part, 1.0);
        push(m, [0.0, 0.0, z1], n(am), part, 1.0);
    }
}

/// A limb from `a` (radius `r0`) to `b` (radius `r1`): a trunk, a branch; darker at its foot.
fn limb(m: &mut Model, a: [f32; 3], b: [f32; 3], r0: f32, r1: f32, sides: usize, part: f32, ao0: f32, ao1: f32) {
    let d = norm([b[0] - a[0], b[1] - a[1], b[2] - a[2]]);
    // (two directions across the limb)
    let side = if d[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] };
    let u = norm([d[1] * side[2] - d[2] * side[1], d[2] * side[0] - d[0] * side[2], d[0] * side[1] - d[1] * side[0]]);
    let v = [d[1] * u[2] - d[2] * u[1], d[2] * u[0] - d[0] * u[2], d[0] * u[1] - d[1] * u[0]];
    let ring = |c: [f32; 3], r: f32, ang: f32| {
        let (s, co) = ang.sin_cos();
        let n = [u[0] * co + v[0] * s, u[1] * co + v[1] * s, u[2] * co + v[2] * s];
        ([c[0] + n[0] * r, c[1] + n[1] * r, c[2] + n[2] * r], n)
    };
    for s in 0..sides {
        let (a0, a1) = (s as f32 / sides as f32 * std::f32::consts::TAU, (s + 1) as f32 / sides as f32 * std::f32::consts::TAU);
        let (p00, n0) = ring(a, r0, a0);
        let (p10, n1) = ring(a, r0, a1);
        let (p01, _) = ring(b, r1, a0);
        let (p11, _) = ring(b, r1, a1);
        for (p, n, ao) in [(p00, n0, ao0), (p10, n1, ao0), (p11, n1, ao1), (p00, n0, ao0), (p11, n1, ao1), (p01, n0, ao1)] {
            push(m, p, n, part, ao);
        }
    }
}

/// A clump of leaves about `c` (radius `r`, `rz` high), lumpy; its light bent `soft` of the way
/// towards the crown's own middle `mid` (one body of leaves, not a heap of balls), darker
/// underneath and where it faces into the crown; `lit` — how much of the sky reaches the clump.
fn clump(m: &mut Model, c: [f32; 3], r: f32, rz: f32, mid: [f32; 3], soft: f32, lit: f32) {
    let (seg, rings) = (7usize, 5usize);
    let at = |i: usize, j: usize| {
        let a = (i % seg) as f32 / seg as f32 * std::f32::consts::TAU;
        let b = j as f32 / rings as f32 * std::f32::consts::PI - std::f32::consts::FRAC_PI_2;
        let (x, y, z) = (b.cos() * a.cos(), b.cos() * a.sin(), b.sin());
        let lump = if j == 0 || j == rings { 1.0 } else { 0.86 + 0.28 * bump(c[0] + x, c[1] + y, c[2] + z) };
        let p = [c[0] + x * r * lump, c[1] + y * r * lump, c[2] + z * rz * lump];
        let own = norm([x / r, y / r, z / rz]);
        let out = norm([p[0] - mid[0], p[1] - mid[1], (p[2] - mid[2]) * 0.8]);
        let n = norm([own[0] * (1.0 - soft) + out[0] * soft, own[1] * (1.0 - soft) + out[1] * soft, own[2] * (1.0 - soft) + out[2] * soft + 0.15]);
        let facing = (own[0] * out[0] + own[1] * out[1] + own[2] * out[2]).max(0.0);
        let ao = lit * (0.52 + 0.48 * (z * 0.5 + 0.5)) * (0.55 + 0.45 * facing);
        (p, n, ao)
    };
    for j in 0..rings {
        for i in 0..seg {
            let (p00, p10, p11, p01) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
            for (p, n, ao) in [p00, p10, p11, p00, p11, p01] {
                push(m, p, n, 1.0, ao);
            }
        }
    }
}

/// A spruce: a thin trunk, seven tiers of drooping boughs narrowing to a spire — each tier a
/// ragged skirt (long and short boughs in turn), dark underneath.
pub fn spruce() -> Model {
    let mut m = Model::new();
    limb(&mut m, [0.0, 0.0, 0.0], [0.0, 0.0, 0.34], 0.045, 0.035, 5, 0.0, 0.35, 0.55);
    let tiers = 7;
    for t in 0..tiers {
        let f = t as f32 / (tiers - 1) as f32;
        let zb = 0.16 + f * 1.08;
        let zt = zb + 0.42 - 0.08 * f;
        let r = 0.40 * (1.0 - 0.80 * f) + 0.05;
        let n = 11usize;
        let twist = t as f32 * 0.9;
        let tip = [0.0, 0.0, zt];
        let rim = |k: usize| {
            let a = k as f32 / (2 * n) as f32 * std::f32::consts::TAU + twist;
            let long = k % 2 == 0;
            let rr = if long { r } else { r * 0.64 };
            let droop = if long { 0.07 } else { 0.0 };
            ([rr * a.cos(), rr * a.sin(), zb - droop], a, long)
        };
        let h = zt - zb;
        let lit = 0.62 + 0.38 * f;
        for k in 0..2 * n {
            let (p0, a0, l0) = rim(k);
            let (p1, a1, l1) = rim(k + 1);
            let nn = |a: f32| norm([a.cos() * h * 1.3, a.sin() * h * 1.3, r * 0.9 + 0.12]);
            let am = (a0 + a1) / 2.0;
            push(&mut m, p0, nn(a0), 1.0, lit * if l0 { 0.92 } else { 0.62 });
            push(&mut m, p1, nn(a1), 1.0, lit * if l1 { 0.92 } else { 0.62 });
            push(&mut m, tip, nn(am), 1.0, lit * 1.05);
            // (underneath: the shade of the boughs above)
            let under = [0.0, 0.0, zb + 0.10];
            push(&mut m, p1, [a1.cos() * 0.3, a1.sin() * 0.3, -1.0], 1.0, 0.22);
            push(&mut m, p0, [a0.cos() * 0.3, a0.sin() * 0.3, -1.0], 1.0, 0.22);
            push(&mut m, under, [0.0, 0.0, -1.0], 1.0, 0.12);
        }
    }
    m
}

/// A pine: a tall bare trunk leaning a little, a flat crown of a few clumps at its top.
pub fn pine() -> Model {
    let mut m = Model::new();
    limb(&mut m, [0.0, 0.0, 0.0], [0.03, 0.01, 1.10], 0.042, 0.022, 5, 0.0, 0.40, 0.85);
    limb(&mut m, [0.02, 0.0, 0.86], [0.18, 0.05, 1.04], 0.016, 0.008, 4, 0.0, 0.6, 0.7);
    limb(&mut m, [0.02, 0.0, 0.92], [-0.14, -0.08, 1.08], 0.014, 0.008, 4, 0.0, 0.6, 0.7);
    let mid = [0.03, 0.01, 1.16];
    clump(&mut m, [0.03, 0.01, 1.30], 0.20, 0.12, mid, 0.55, 1.0);
    for k in 0..5 {
        let a = k as f32 / 5.0 * std::f32::consts::TAU + 0.4;
        let d = 0.17 + 0.03 * (k % 2) as f32;
        clump(&mut m, [0.03 + d * a.cos(), 0.01 + d * a.sin(), 1.16 + 0.04 * (k % 3) as f32], 0.15, 0.10, mid, 0.55, 0.85);
    }
    clump(&mut m, [0.16, 0.05, 1.02], 0.11, 0.07, mid, 0.5, 0.7);
    clump(&mut m, [-0.13, -0.07, 1.06], 0.11, 0.07, mid, 0.5, 0.7);
    m
}

/// A broad tree: a trunk forking into boughs, a round crown of ten clumps — one on top, a ring
/// round it, three lower inside.
pub fn broad() -> Model {
    let mut m = Model::new();
    limb(&mut m, [0.0, 0.0, 0.0], [0.0, 0.0, 0.56], 0.065, 0.048, 6, 0.0, 0.32, 0.55);
    limb(&mut m, [0.0, 0.0, 0.50], [0.20, 0.06, 0.86], 0.035, 0.018, 5, 0.0, 0.45, 0.4);
    limb(&mut m, [0.0, 0.0, 0.50], [-0.16, 0.12, 0.84], 0.032, 0.016, 5, 0.0, 0.45, 0.4);
    limb(&mut m, [0.0, 0.0, 0.54], [0.02, -0.18, 0.90], 0.03, 0.015, 5, 0.0, 0.45, 0.4);
    let mid = [0.0, 0.0, 0.94];
    clump(&mut m, [0.02, -0.02, 1.18], 0.27, 0.22, mid, 0.5, 1.0);
    for k in 0..6 {
        let a = k as f32 / 6.0 * std::f32::consts::TAU + 0.3 * (k % 2) as f32;
        let d = 0.27 + 0.03 * bump(k as f32, 1.0, 2.0);
        clump(&mut m, [d * a.cos(), d * a.sin(), 0.94 + 0.07 * (bump(k as f32, 3.0, 1.0) - 0.5)], 0.23 + 0.03 * bump(k as f32, 2.0, 5.0), 0.20, mid, 0.5, 0.88);
    }
    for k in 0..3 {
        let a = k as f32 / 3.0 * std::f32::consts::TAU + 0.9;
        clump(&mut m, [0.17 * a.cos(), 0.17 * a.sin(), 0.72], 0.20, 0.16, mid, 0.45, 0.62);
    }
    m
}

/// A birch: a slender white trunk, a narrow crown of loose clumps hanging down it.
pub fn birch() -> Model {
    let mut m = Model::new();
    limb(&mut m, [0.0, 0.0, 0.0], [0.02, 0.0, 1.28], 0.034, 0.012, 5, 0.0, 0.5, 0.9);
    let mid = [0.01, 0.0, 1.0];
    let parts = [
        ([0.02, 0.0, 1.32], 0.14, 0.18, 1.0),
        ([0.09, 0.05, 1.10], 0.15, 0.19, 0.9),
        ([-0.08, -0.04, 1.04], 0.15, 0.18, 0.86),
        ([0.06, -0.08, 0.86], 0.14, 0.17, 0.74),
        ([-0.05, 0.08, 0.80], 0.13, 0.16, 0.7),
        ([0.0, 0.0, 0.66], 0.11, 0.13, 0.6),
    ];
    for (c, r, rz, lit) in parts {
        clump(&mut m, c, r, rz, mid, 0.45, lit);
    }
    m
}

/// A bush: three or four low clumps, no trunk to speak of.
pub fn bush() -> Model {
    let mut m = Model::new();
    let mid = [0.0, 0.0, 0.18];
    clump(&mut m, [0.0, 0.0, 0.34], 0.22, 0.18, mid, 0.45, 1.0);
    clump(&mut m, [0.16, 0.06, 0.22], 0.18, 0.15, mid, 0.45, 0.8);
    clump(&mut m, [-0.12, 0.12, 0.20], 0.17, 0.14, mid, 0.45, 0.78);
    clump(&mut m, [-0.04, -0.15, 0.21], 0.17, 0.14, mid, 0.45, 0.78);
    m
}

/// A card (part 3): a quad across the screen and up it — a far tree's picture is laid on it.
pub fn card() -> Model {
    let mut m = Model::new();
    for (x, z) in [(-0.5f32, 0.0f32), (0.5, 0.0), (0.5, 1.0), (-0.5, 0.0), (0.5, 1.0), (-0.5, 1.0)] {
        push(&mut m, [x, 0.0, z], [0.0, 0.0, 1.0], 3.0, 1.0);
    }
    m
}

/// A shadow (part 4): a quad lying on the land, laid away from the sun by the vertex shader.
pub fn shade() -> Model {
    let mut m = Model::new();
    for (x, y) in [(-0.5f32, -0.5f32), (0.5, -0.5), (0.5, 0.5), (-0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
        push(&mut m, [x, y, 0.0], [0.0, 0.0, 1.0], 4.0, 1.0);
    }
    m
}

fn quad(m: &mut Model, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3], n: [f32; 3], part: f32) {
    for p in [a, b, c, a, c, d] {
        push(m, p, n, part, 1.0);
    }
}

fn tri(m: &mut Model, a: [f32; 3], b: [f32; 3], c: [f32; 3], part: f32) {
    let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
    let mut n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    if n[2] < 0.0 {
        n = [-n[0], -n[1], -n[2]];
    }
    for p in [a, b, c] {
        push(m, p, n, part, 1.0);
    }
}

/// A box from (x0, y0, z0) to (x1, y1, z1): its four walls (part `wall`) and its top (`top`).
fn boxed(m: &mut Model, lo: [f32; 3], hi: [f32; 3], wall: f32, top: f32) {
    let ([x0, y0, z0], [x1, y1, z]) = (lo, hi);
    quad(m, [x0, y0, z0], [x1, y0, z0], [x1, y0, z], [x0, y0, z], [0.0, -1.0, 0.0], wall);
    quad(m, [x1, y1, z0], [x0, y1, z0], [x0, y1, z], [x1, y1, z], [0.0, 1.0, 0.0], wall);
    quad(m, [x1, y0, z0], [x1, y1, z0], [x1, y1, z], [x1, y0, z], [1.0, 0.0, 0.0], wall);
    quad(m, [x0, y1, z0], [x0, y0, z0], [x0, y0, z], [x0, y1, z], [-1.0, 0.0, 0.0], wall);
    quad(m, [x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z], [0.0, 0.0, 1.0], top);
}

/// Walls (`hx` half-long, `hy` half-deep, `wz` high) under a gabled roof along x (its ridge `rz`
/// high), the eaves `over` beyond the walls.
fn gabled(m: &mut Model, hx: f32, hy: f32, wz: f32, rz: f32, over: f32) {
    quad(m, [-hx, -hy, 0.0], [hx, -hy, 0.0], [hx, -hy, wz], [-hx, -hy, wz], [0.0, -1.0, 0.0], 0.0);
    quad(m, [hx, hy, 0.0], [-hx, hy, 0.0], [-hx, hy, wz], [hx, hy, wz], [0.0, 1.0, 0.0], 0.0);
    quad(m, [hx, -hy, 0.0], [hx, hy, 0.0], [hx, hy, wz], [hx, -hy, wz], [1.0, 0.0, 0.0], 0.0);
    quad(m, [-hx, hy, 0.0], [-hx, -hy, 0.0], [-hx, -hy, wz], [-hx, hy, wz], [-1.0, 0.0, 0.0], 0.0);
    // the gables
    for (x, nx) in [(hx, 1.0f32), (-hx, -1.0)] {
        push(m, [x, -hy, wz], [nx, 0.0, 0.0], 0.0, 1.0);
        push(m, [x, hy, wz], [nx, 0.0, 0.0], 0.0, 1.0);
        push(m, [x, 0.0, rz], [nx, 0.0, 0.0], 0.0, 1.0);
    }
    // the roof's two slopes, over the eaves (a little below the wall's top at their edge)
    let (rx, ey) = (hx + over, hy + over);
    let drop = (rz - wz) / hy * over;
    let s = (rz - wz) / hy;
    quad(m, [-rx, -ey, wz - drop], [rx, -ey, wz - drop], [rx, 0.0, rz], [-rx, 0.0, rz], [0.0, -s, 1.0], 1.0);
    quad(m, [rx, ey, wz - drop], [-rx, ey, wz - drop], [-rx, 0.0, rz], [rx, 0.0, rz], [0.0, s, 1.0], 1.0);
}

/// A chimney (part 5) through a roof at (x, y), up to `z`.
fn chimney(m: &mut Model, x: f32, y: f32, z0: f32, z: f32, w: f32) {
    boxed(m, [x - w, y - w, z0], [x + w, y + w, z], 5.0, 5.0);
}

/// A cottage: one storey, a steep gabled roof, a chimney.
pub fn cottage() -> Model {
    let mut m = Model::new();
    gabled(&mut m, 0.22, 0.15, 0.19, 0.38, 0.035);
    chimney(&mut m, 0.10, 0.05, 0.25, 0.44, 0.025);
    m
}

/// A town house: two storeys (three, every other one — the shader raises it), its ridge along
/// the street, two chimneys at the gables.
pub fn townhouse() -> Model {
    let mut m = Model::new();
    gabled(&mut m, 0.20, 0.18, 0.34, 0.56, 0.02);
    chimney(&mut m, 0.16, 0.02, 0.40, 0.62, 0.025);
    chimney(&mut m, -0.16, -0.02, 0.40, 0.62, 0.025);
    m
}

/// A barn: long and low under a great roof.
pub fn barn() -> Model {
    let mut m = Model::new();
    gabled(&mut m, 0.34, 0.16, 0.15, 0.40, 0.04);
    m
}

/// A house under a hipped roof (four slopes up to a short ridge): a hall, a merchant's house.
pub fn hiphouse() -> Model {
    let mut m = Model::new();
    let (hx, hy, wz, rz, o) = (0.25f32, 0.21f32, 0.30f32, 0.52f32, 0.035f32);
    for (a, b, n) in [([-hx, -hy], [hx, -hy], [0.0, -1.0]), ([hx, hy], [-hx, hy], [0.0, 1.0]), ([hx, -hy], [hx, hy], [1.0, 0.0]), ([-hx, hy], [-hx, -hy], [-1.0, 0.0])] {
        quad(&mut m, [a[0], a[1], 0.0], [b[0], b[1], 0.0], [b[0], b[1], wz], [a[0], a[1], wz], [n[0], n[1], 0.0], 0.0);
    }
    let (rx, ry) = (hx + o, hy + o);
    let drop = (rz - wz) / hy * o;
    let ridge = hx - hy;
    let (r0, r1) = ([-ridge, 0.0, rz], [ridge, 0.0, rz]);
    let e = wz - drop;
    let (c00, c10, c11, c01) = ([-rx, -ry, e], [rx, -ry, e], [rx, ry, e], [-rx, ry, e]);
    let s = (rz - wz) / hy;
    quad(&mut m, c00, c10, r1, r0, [0.0, -s, 1.0], 1.0);
    quad(&mut m, c11, c01, r0, r1, [0.0, s, 1.0], 1.0);
    tri(&mut m, c10, c11, r1, 1.0);
    tri(&mut m, c01, c00, r0, 1.0);
    chimney(&mut m, -0.08, 0.10, 0.40, 0.62, 0.025);
    m
}

/// A church: a nave under a steep roof, at its +x end a square tower with a spire.
pub fn church() -> Model {
    let mut m = Model::new();
    gabled(&mut m, 0.30, 0.15, 0.32, 0.54, 0.02);
    let (x0, x1, h) = (0.26f32, 0.46f32, 0.10f32);
    boxed(&mut m, [x0, -h, 0.0], [x1, h, 0.84], 0.0, 0.0);
    let (cx, top) = ((x0 + x1) / 2.0, 1.32f32);
    let base = 0.84;
    let o = 0.015;
    let c = [[x0 - o, -h - o, base], [x1 + o, -h - o, base], [x1 + o, h + o, base], [x0 - o, h + o, base]];
    for k in 0..4 {
        tri(&mut m, c[k], c[(k + 1) % 4], [cx, 0.0, top], 1.0);
    }
    m
}

/// A block of flats: four storeys, a flat roof behind a parapet, a stair-head on the roof.
pub fn flats() -> Model {
    let mut m = Model::new();
    let (hx, hy, z) = (0.34f32, 0.21f32, 0.64f32);
    boxed(&mut m, [-hx, -hy, 0.0], [hx, hy, z], 0.0, 1.0);
    // (the parapet: a low rim round the roof)
    let (t, pz) = (0.02f32, z + 0.035);
    boxed(&mut m, [-hx, -hy, z], [hx, -hy + t, pz], 0.0, 0.0);
    boxed(&mut m, [-hx, hy - t, z], [hx, hy, pz], 0.0, 0.0);
    boxed(&mut m, [-hx, -hy, z], [-hx + t, hy, pz], 0.0, 0.0);
    boxed(&mut m, [hx - t, -hy, z], [hx, hy, pz], 0.0, 0.0);
    boxed(&mut m, [-0.08, -0.05, z], [0.06, 0.08, z + 0.10], 0.0, 1.0);
    m
}

/// A factory: a long hall of brick under north-light roofs (a row of sheds, each a long slope of
/// iron and a steep glazed face — part 6), a lower shed beside it under a gabled roof, two tall
/// brick chimneys (at the hall's back: (0.3, −0.1) and (0.1, −0.1), 1.15 and 0.95 high — the smoke
/// rises from there).
pub fn factory() -> Model {
    let mut m = Model::new();
    let (x0, x1, y0, y1, wz) = (-0.48f32, 0.48f32, -0.26f32, 0.2f32, 0.30f32);
    for (a, b, n) in [([x0, y0], [x1, y0], [0.0, -1.0]), ([x1, y1], [x0, y1], [0.0, 1.0]), ([x1, y0], [x1, y1], [1.0, 0.0]), ([x0, y1], [x0, y0], [-1.0, 0.0])] {
        quad(&mut m, [a[0], a[1], 0.0], [b[0], b[1], 0.0], [b[0], b[1], wz], [a[0], a[1], wz], [n[0], n[1], 0.0], 0.0);
    }
    let teeth = 4;
    let w = (x1 - x0) / teeth as f32;
    let th = 0.11f32;
    for k in 0..teeth {
        let (xa, xb) = (x0 + k as f32 * w, x0 + (k + 1) as f32 * w);
        quad(&mut m, [xa, y0, wz], [xa, y1, wz], [xb, y1, wz + th], [xb, y0, wz + th], [-th, 0.0, w], 1.0);
        quad(&mut m, [xb, y0, wz], [xb, y1, wz], [xb, y1, wz + th], [xb, y0, wz + th], [1.0, 0.0, 0.0], 6.0);
        for (y, ny) in [(y0, -1.0f32), (y1, 1.0)] {
            push(&mut m, [xa, y, wz], [0.0, ny, 0.0], 0.0, 1.0);
            push(&mut m, [xb, y, wz], [0.0, ny, 0.0], 0.0, 1.0);
            push(&mut m, [xb, y, wz + th], [0.0, ny, 0.0], 0.0, 1.0);
        }
    }
    let mut shed = Model::new();
    gabled(&mut shed, 0.25, 0.11, 0.20, 0.30, 0.02);
    for v in shed.chunks_exact_mut(FLOATS) {
        v[0] -= 0.15;
        v[1] += 0.31;
    }
    m.extend(shed);
    limb(&mut m, [0.3, -0.1, 0.0], [0.3, -0.1, 1.15], 0.065, 0.055, 8, 5.0, 1.0, 1.0);
    limb(&mut m, [0.3, -0.1, 1.07], [0.3, -0.1, 1.15], 0.075, 0.075, 8, 5.0, 1.0, 1.0);
    limb(&mut m, [0.1, -0.1, 0.0], [0.1, -0.1, 0.95], 0.08, 0.055, 8, 5.0, 1.0, 1.0);
    limb(&mut m, [0.1, -0.1, 0.88], [0.1, -0.1, 0.95], 0.068, 0.068, 8, 5.0, 1.0, 1.0);
    m
}

/// A windmill: a tapering tower, a cap, four sails (part 2: they turn about the hub).
pub fn mill() -> Model {
    let mut m = Model::new();
    limb(&mut m, [0.0, 0.0, 0.0], [0.0, 0.0, 0.7], 0.2, 0.13, 8, 0.0, 1.0, 1.0);
    cone(&mut m, 0.68, 0.9, 0.16, 8, 1.0);
    let hub = [0.0f32, -0.2, 0.66];
    for k in 0..4 {
        let a = k as f32 * std::f32::consts::FRAC_PI_2;
        let (s, c) = a.sin_cos();
        // (a sail: a narrow board from the hub outwards, in the plane facing the wind)
        let along = [c, 0.0, s];
        let across = [-s * 0.07, 0.0, c * 0.07];
        let at = |d: f32, w: f32| [hub[0] + along[0] * d + across[0] * w, hub[1], hub[2] + along[2] * d + across[2] * w];
        for p in [at(0.05, -1.0), at(0.5, -1.0), at(0.5, 1.0), at(0.05, -1.0), at(0.5, 1.0), at(0.05, 1.0)] {
            push(&mut m, p, [0.0, -1.0, 0.15], 2.0, 1.0);
        }
    }
    m
}
