//! What the window's self-checks read of the map: the land's relief, the towns' houses.

use super::*;

impl MapView {
    /// The self-check of the land's relief: the median height (as shown) of the mountains' pixels
    /// deep in a range (6 world pixels and more from its edge) and of those at its edge (within
    /// 1.5), the land's pixels below the sea's level.
    pub fn relief_check(&self, g: &Game) -> (f32, f32, usize) {
        use strategy::world::map::Terrain;
        let l = &self.ground;
        let outside: Vec<bool> = (0..l.w * l.h).map(|i| g.world.terrain[i] != Terrain::Mountains).collect();
        let into = ground::reach(&outside, l.w, l.h);
        let mut deep: Vec<f32> = Vec::new();
        let mut edge: Vec<f32> = Vec::new();
        let mut below = 0;
        for i in 0..l.w * l.h {
            if l.sea[i] <= 0.0 {
                continue;
            }
            if !outside[i] {
                if into[i] >= 6.0 {
                    deep.push(l.height[i]);
                } else if into[i] <= 1.5 {
                    edge.push(l.height[i]);
                }
            }
            if l.height[i] < 0.0 {
                below += 1;
            }
        }
        let med = |v: &mut Vec<f32>| {
            if v.is_empty() {
                return 0.0;
            }
            v.sort_by(|a, b| a.total_cmp(b));
            v[v.len() / 2]
        };
        (med(&mut deep), med(&mut edge), below)
    }

    /// The self-check of the towns: how many houses there are, and the ones not standing as a
    /// house stands in a town — its long side along the nearest street or road (within 0.2 of a
    /// radian), its front within reach of it (1.6 world pixels), off it, on dry land.
    pub fn towns_check(&self, g: &Game) -> (usize, Vec<String>) {
        let mut bad = Vec::new();
        for h in &self.houses {
            let p = [h[0], h[1]];
            let mut best = (f32::MAX, 0.0f32);
            for s in &self.streets {
                for w in s.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                    let l2 = (dx * dx + dy * dy).max(1e-9);
                    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0);
                    let d = (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy);
                    if d < best.0 {
                        best = (d, dy.atan2(dx));
                    }
                }
            }
            let turn = h[3] * std::f32::consts::TAU;
            let off = (turn - best.1).rem_euclid(std::f32::consts::PI);
            let off = off.min(std::f32::consts::PI - off);
            let (xi, yi) = ((h[0].floor() as i64).rem_euclid(g.world.w as i64) as usize, (h[1].floor().max(0.0) as usize).min(g.world.h - 1));
            let dry = g.world.height[yi * g.world.w + xi] > 0.0;
            if best.0 > 1.6 || best.0 < 0.12 || off > 0.2 || !dry {
                bad.push(format!("({:.1},{:.1}): до улицы {:.2}, поворот {:.2}{}", h[0], h[1], best.0, off, if dry { "" } else { ", в воде" }));
            }
        }
        (self.houses.len(), bad)
    }
}
