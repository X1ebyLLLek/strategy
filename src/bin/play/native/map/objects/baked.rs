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
    Some(Baked { hx, hy, top, height, model })
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
