//! The towns' buildings that come from free 3D models (`tools/buildings_bake.py`, authors in
//! `assets/buildings/CREDITS.txt`): each baked into triangles of one colour apiece, the colour carried in the
//! vertex's two last numbers (part = 100 + r·256 + g, light = b + the sky's share).

use super::*;

/// The baked files: the kind each stands for, its bytes.
const FILES: [(usize, &[u8]); 9] = [
    (ORDINARY_A, include_bytes!("../../../../../../assets/buildings/ordinary_a.bld")),
    (ORDINARY_B, include_bytes!("../../../../../../assets/buildings/ordinary_b.bld")),
    (OLDHOUSE, include_bytes!("../../../../../../assets/buildings/oldhouse.bld")),
    (COTTAGE_B, include_bytes!("../../../../../../assets/buildings/cottage.bld")),
    (FARM_A, include_bytes!("../../../../../../assets/buildings/farm_a.bld")),
    (FARM_B, include_bytes!("../../../../../../assets/buildings/farm_b.bld")),
    (BRICK, include_bytes!("../../../../../../assets/buildings/brick.bld")),
    (PANEL, include_bytes!("../../../../../../assets/buildings/panel.bld")),
    (CHURCH_B, include_bytes!("../../../../../../assets/buildings/church.bld")),
];

/// A baked building: half its width along the street, half its depth, the walls' height, its whole height — and its
/// triangles.
pub struct Baked {
    pub hx: f32,
    pub hy: f32,
    pub top: f32,
    pub height: f32,
    pub model: Model,
    pub far: Model,
}

fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
    norm([u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]])
}

fn far_box(hx: f32, hy: f32, top: f32, height: f32, model: &[f32]) -> Model {
    let (mut wr, mut wg, mut wb, mut ww) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let (mut rr, mut rg, mut rb, mut rw) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for tri in model.chunks_exact(24) {
        let ax = tri[8] - tri[0];
        let ay = tri[9] - tri[1];
        let az = tri[10] - tri[2];
        let bx = tri[16] - tri[0];
        let by = tri[17] - tri[1];
        let bz = tri[18] - tri[2];
        let area = ((ay * bz - az * by).powi(2) + (az * bx - ax * bz).powi(2) + (ax * by - ay * bx).powi(2)).sqrt().max(1e-6);
        let nz = tri[5];
        let v = (tri[6] - 100.0).max(0.0);
        let r = (v / 256.0).floor();
        let g = v - r * 256.0;
        let bl = tri[7].floor();
        if nz > 0.35 {
            rr += r * area;
            rg += g * area;
            rb += bl * area;
            rw += area;
        } else {
            wr += r * area;
            wg += g * area;
            wb += bl * area;
            ww += area;
        }
    }
    let w_rgb = if ww > 0.0 { [wr / ww, wg / ww, wb / ww] } else { [185.0, 175.0, 160.0] };
    let r_rgb = if rw > 0.0 { [rr / rw, rg / rw, rb / rw] } else { [140.0, 85.0, 65.0] };
    let pack = |c: [f32; 3], ao: f32| (100.0 + c[0].round().clamp(0.0, 255.0) * 256.0 + c[1].round().clamp(0.0, 255.0), c[2].round().clamp(0.0, 255.0) + ao);
    let (wp, wl) = pack(w_rgb, 0.84);
    let (rp, rl) = pack(r_rgb, 0.94);
    let mut m = Model::with_capacity(12 * 24);
    let quad = |m: &mut Model, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3], p: f32, l: f32| {
        let n = face_normal(a, b, c);
        for v in [a, b, c, a, c, d] {
            push(m, v, n, p, l);
        }
    };
    let tri3 = |m: &mut Model, a: [f32; 3], b: [f32; 3], c: [f32; 3], p: f32, l: f32| {
        let n = face_normal(a, b, c);
        for v in [a, b, c] {
            push(m, v, n, p, l);
        }
    };
    quad(&mut m, [-hx, -hy, 0.0], [hx, -hy, 0.0], [hx, -hy, top], [-hx, -hy, top], wp, wl);
    quad(&mut m, [hx, hy, 0.0], [-hx, hy, 0.0], [-hx, hy, top], [hx, hy, top], wp, wl);
    quad(&mut m, [hx, -hy, 0.0], [hx, hy, 0.0], [hx, hy, top], [hx, -hy, top], wp, wl);
    quad(&mut m, [-hx, hy, 0.0], [-hx, -hy, 0.0], [-hx, -hy, top], [-hx, hy, top], wp, wl);
    let ztop = height.max(top + 0.04);
    quad(&mut m, [-hx, -hy, top], [hx, -hy, top], [hx, 0.0, ztop], [-hx, 0.0, ztop], rp, rl);
    quad(&mut m, [hx, hy, top], [-hx, hy, top], [-hx, 0.0, ztop], [hx, 0.0, ztop], rp, rl);
    tri3(&mut m, [hx, -hy, top], [hx, hy, top], [hx, 0.0, ztop], wp, wl);
    tri3(&mut m, [-hx, hy, top], [-hx, -hy, top], [-hx, 0.0, ztop], wp, wl);
    m
}

fn f32_at(b: &[u8], i: usize) -> f32 {
    f32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

fn header(b: &[u8]) -> Option<(f32, f32, f32, f32, usize)> {
    if b.len() < 24 || &b[0..4] != b"BLD1" {
        return None;
    }
    let n = u32::from_le_bytes([b[20], b[21], b[22], b[23]]) as usize;
    Some((f32_at(b, 4), f32_at(b, 8), f32_at(b, 12), f32_at(b, 16), n))
}

/// The kind's baked building whole (None: it is a made one).
pub fn load(kind: usize) -> Option<Baked> {
    let (_, b) = FILES.iter().find(|(k, _)| *k == kind)?;
    let (hx, hy, top, height, n) = header(b)?;
    if b.len() < 24 + n * 32 {
        return None;
    }
    let model: Model = (0..n * 8).map(|i| f32_at(b, 24 + i * 4)).collect();
    let far = far_box(hx, hy, top, height, &model);
    Some(Baked { hx, hy, top, height, model, far })
}

/// A baked kind's footprint (half the length along the street, half the depth) — the header only.
pub fn footprint(kind: usize) -> Option<(f32, f32)> {
    let (_, b) = FILES.iter().find(|(k, _)| *k == kind)?;
    let (hx, hy, ..) = header(b)?;
    Some((hx, hy))
}

/// The house the towns put where a kind of the old procedural set was meant: the baked ones, by the era and a chance
/// `r` (0..1). The barns and the mills stay made.
pub fn variant(kind: usize, era: f32, r: f32) -> usize {
    let pick = |list: &[(usize, f32)]| -> usize {
        let total: f32 = list.iter().map(|x| x.1).sum();
        let mut t = r * total;
        for &(k, w) in list {
            if t < w {
                return k;
            }
            t -= w;
        }
        list[list.len() - 1].0
    };
    let old = era > 1.5;
    let wastes = era > 0.5 && era < 1.5;
    match kind {
        COTTAGE if old => pick(&[(FARM_A, 0.35), (FARM_B, 0.35), (OLDHOUSE, 0.30)]),
        COTTAGE if wastes => pick(&[(OLDHOUSE, 0.45), (COTTAGE_B, 0.30), (FARM_A, 0.25)]),
        COTTAGE => pick(&[(COTTAGE_B, 0.32), (OLDHOUSE, 0.24), (FARM_A, 0.22), (FARM_B, 0.22)]),
        TOWNHOUSE if old => pick(&[(ORDINARY_A, 0.30), (ORDINARY_B, 0.70)]),
        TOWNHOUSE => pick(&[(ORDINARY_A, 0.5), (ORDINARY_B, 0.5)]),
        HIPHOUSE => pick(&[(COTTAGE_B, 0.30), (FARM_A, 0.25), (FARM_B, 0.25), (OLDHOUSE, 0.20)]),
        BLOCK if wastes => PANEL,
        BLOCK => pick(&[(BRICK, 0.6), (PANEL, 0.4)]),
        CHURCH => CHURCH_B,
        k => k,
    }
}
