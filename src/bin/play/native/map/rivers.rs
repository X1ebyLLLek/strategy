//! The great rivers' names, written along them as on an atlas (33rd session): each letter on the river's
//! line, turned along it, in the lighter ink of water; only rivers wide enough to matter, and only at the
//! zooms where a name along them fits and reads.

use super::*;
use strategy::rng::{hash64, Rng};

/// A river with its name: its line (world pixels, the seam of the map kept whole), the length to each
/// point, its widest width and where along it the wide reach is.
pub struct RiverLabel {
    name: String,
    pts: Vec<[f32; 2]>,
    cum: Vec<f32>,
    mid: f32,
}

/// The names of the rivers that are great enough: a name each, from the world's seed.
pub fn labels(g: &Game) -> Vec<RiverLabel> {
    let rivers = &g.land.rivers;
    let mut rng = Rng::new(hash64(g.seed ^ 0x51FE));
    let mut taken = std::collections::HashSet::new();
    let mut out = Vec::new();
    for chain in crate::world::land::river_chains(rivers) {
        let widest = chain.iter().map(|&i| rivers[i].2).fold(0.0f32, f32::max);
        if widest < 1.3 {
            continue;
        }
        // (the chain's pieces as one line, each piece's first point left out but the first piece's)
        let mut pts: Vec<[f32; 2]> = Vec::new();
        let mut width: Vec<f32> = Vec::new();
        for &i in &chain {
            for (k, p) in rivers[i].0.iter().enumerate() {
                if k == 0 && !pts.is_empty() {
                    continue;
                }
                pts.push(*p);
                width.push(rivers[i].2);
            }
        }
        let mut cum = vec![0.0f32];
        for w in pts.windows(2) {
            cum.push(cum.last().unwrap() + (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]));
        }
        let len = *cum.last().unwrap();
        if len < 45.0 {
            continue;
        }
        // (the wide reach's middle: the median length of the points nearly as wide as the widest)
        let wide: Vec<f32> = cum.iter().zip(&width).filter(|(_, &w)| w >= 0.8 * widest).map(|(&c, _)| c).collect();
        let mid = wide.get(wide.len() / 2).copied().unwrap_or(len / 2.0);
        let name = strategy::world::names::river(&mut rng, &mut taken);
        out.push(RiverLabel { name, pts, cum, mid });
    }
    out
}

impl RiverLabel {
    /// The point `s` along the river (world pixels), and the direction there.
    fn at(&self, s: f32) -> ([f32; 2], [f32; 2]) {
        let s = s.clamp(0.0, *self.cum.last().unwrap());
        let k = self.cum.partition_point(|&c| c <= s).clamp(1, self.pts.len() - 1);
        let (a, b) = (self.pts[k - 1], self.pts[k]);
        let t = ((s - self.cum[k - 1]) / (self.cum[k] - self.cum[k - 1]).max(1e-6)).clamp(0.0, 1.0);
        ([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t], [b[0] - a[0], b[1] - a[1]])
    }
}

impl MapView {
    /// The great rivers' names along them; a name that would lie over one already written (`taken`) waits.
    pub(super) fn river_names(&self, painter: &egui::Painter, rect: Rect, taken: &mut Vec<Rect>) {
        // (from the zoom where the rivers are rivers on the screen, to where the land fills it and a name along a river is lost)
        if self.k < 1.15 || self.k > 6.5 {
            return;
        }
        let fade = ((self.k - 1.15) / 0.5).clamp(0.0, 1.0) * ((6.5 - self.k) / 1.5).clamp(0.0, 1.0);
        for r in &self.river_labels {
            let chars: Vec<char> = r.name.chars().collect();
            let (c0, _) = r.at(r.mid);
            let centre = self.cam.to_screen(c0[0], c0[1], 0.0);
            if !rect.expand(200.0).contains(centre) {
                continue;
            }
            // (letters as large as a town's name on the screen, a little larger the nearer)
            let size = (12.0 + 0.5 * self.k).min(17.0);
            let advance = size * 0.62;
            let total_px = advance * chars.len() as f32;
            let scale = self.cam.scale_at(c0[0], c0[1], 0.0).max(1e-3);
            let span = total_px / scale;
            let len = *r.cum.last().unwrap();
            if span > len * 0.9 {
                continue;
            }
            let start = (r.mid - span / 2.0).clamp(0.0, len - span);
            // (read left to right: the line walked from its western end)
            let (p0, _) = r.at(start);
            let (p1, _) = r.at(start + span);
            let (s0, s1) = (self.cam.to_screen(p0[0], p0[1], 0.0), self.cam.to_screen(p1[0], p1[1], 0.0));
            let back = s1.x < s0.x;
            let mut placed: Vec<(Pos2, f32, char)> = Vec::new();
            let mut boxes: Vec<Rect> = Vec::new();
            for (i, ch) in chars.iter().enumerate() {
                let s = start + span * (i as f32 + 0.5) / chars.len() as f32;
                let s = if back { start + span - (s - start) } else { s };
                let (p, d) = r.at(s);
                let a = self.cam.to_screen(p[0], p[1], 0.0);
                let b = self.cam.to_screen(p[0] + d[0], p[1] + d[1], 0.0);
                let mut ang = (b.y - a.y).atan2(b.x - a.x);
                if back {
                    ang += std::f32::consts::PI;
                }
                boxes.push(Rect::from_center_size(a, vec2(size * 0.7, size * 0.9)));
                placed.push((a, ang, *ch));
            }
            if boxes.iter().any(|b| taken.iter().any(|t| t.intersects(b.shrink(2.0)))) || boxes.iter().filter(|b| !rect.contains_rect(**b)).count() * 10 > boxes.len() * 3 {
                continue;
            }
            taken.extend(boxes);
            for (a, ang, ch) in placed {
                let font = FontId::new(size, FontFamily::Name("bold".into()));
                let galley = painter.layout_no_wrap(ch.to_string(), font, Color32::WHITE);
                let half = galley.size() / 2.0;
                let (sn, cs) = ang.sin_cos();
                let corner = a - vec2(half.x * cs - half.y * sn, half.x * sn + half.y * cs);
                let ink = Color32::from_rgba_unmultiplied(196, 228, 246, (235.0 * fade) as u8);
                let shade = Color32::from_rgba_unmultiplied(8, 28, 46, (190.0 * fade) as u8);
                for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (1.4, 1.4)] {
                    painter.add(egui::epaint::TextShape::new(corner + vec2(dx, dy), galley.clone(), shade).with_override_text_color(shade).with_angle(ang));
                }
                painter.add(egui::epaint::TextShape::new(corner, galley, ink).with_override_text_color(ink).with_angle(ang));
            }
        }
    }
}
