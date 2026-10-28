//! The colours of the provinces for the video card by the map's mode (the table of provinces),
//! the palette, the small map in the corner.

use super::*;

/// The colours of the forms of government on the map (as `Regime::ALL`): the old crowns in purples
/// and browns, the elected in blues and greens, the iron ones in reds, the church in gold.
pub const REGIME_HUES: [[f32; 3]; 9] = [[150.0, 120.0, 86.0], [128.0, 72.0, 140.0], [168.0, 110.0, 160.0], [104.0, 120.0, 190.0], [70.0, 150.0, 140.0], [96.0, 170.0, 96.0], [176.0, 60.0, 48.0], [214.0, 70.0, 60.0], [214.0, 178.0, 70.0]];

impl MapView {
    /// What the table of provinces is made from, as the world now is.
    fn paint_of(&self, g: &Game) -> Paint {
        let world: &strategy::world::map::World = &g.world;
        let war = &g.h.war;
        let th = &war.th;
        let mode = self.mode;
        // what a province is worth in the modes of the map (0..1), and its colour there
        let me = g.me;
        let sup = war.supply(me);
        let dev_max = g.prov_dev.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
        let dens: Vec<f32> = (0..world.provinces.len()).map(|p| g.prov_people[p] / world.provinces[p].area.max(1) as f32).collect();
        let dens_max = {
            let mut v: Vec<f32> = dens.clone();
            v.sort_by(|a, b| a.total_cmp(b));
            v[(v.len() * 97 / 100).min(v.len() - 1)].max(1e-9)
        };
        // (the land the past took: the country that lost it, the latest loss of the region)
        let tints: Vec<[f32; 3]> = g.countries.iter().map(|c| [c.1[0] as f32, c.1[1] as f32, c.1[2] as f32]).collect();
        let lab = war.group_of(me);
        let mut lost: std::collections::HashMap<u32, (u32, u32)> = std::collections::HashMap::new();
        for &(st, c, years) in &g.lost_lands {
            let e = lost.entry(st).or_insert((c, years));
            if years < e.1 {
                *e = (c, years);
            }
        }
        let modal: Vec<Option<[f32; 3]>> = (0..world.provinces.len())
            .map(|p| match mode {
                Mode::Supply => {
                    let q = th.at.get(p).copied().unwrap_or(NO);
                    (q != NO && war.owner[q as usize] == me).then(|| Mode::Supply.ramp(sup[q as usize]))
                }
                Mode::Dev => Some(mode.ramp(g.prov_dev[p] / dev_max)),
                Mode::People => Some(mode.ramp((dens[p] / dens_max).clamp(0.0, 1.0).sqrt())),
                Mode::Res => (!g.prov_res[p].is_empty()).then(|| mode.ramp(0.35 + 0.3 * g.prov_res[p].len() as f32)),
                Mode::Lost => lost.get(&g.prov_state[p]).map(|&(c, _)| tints[c as usize]),
                Mode::Faith => g.polity.region_faith.get(g.prov_state[p] as usize).and_then(|&f| g.polity.faiths.get(f)).map(|f| [f.hue[0] as f32, f.hue[1] as f32, f.hue[2] as f32]),
                Mode::Regime => {
                    let c = g.country_of[p] as usize;
                    g.polity.of.get(c).map(|x| REGIME_HUES[x.regime.index() % REGIME_HUES.len()])
                }
                Mode::Relations => {
                    let q = th.at.get(p).copied().unwrap_or(NO);
                    let c = if q != NO { th.countries[war.owner[q as usize]] } else { g.country_of[p] as usize };
                    let mine = g.country(me);
                    if c >= g.diplo.n {
                        None
                    } else if c == mine {
                        Some([242.0, 238.0, 226.0])
                    } else if g.diplo.enemies(c, mine) || (g.h.at_war && c == g.country(1 - me)) {
                        Some([110.0, 24.0, 24.0])
                    } else if g.diplo.either(strategy::world::diplomacy::Kind::Alliance, c, mine) {
                        Some([62.0, 104.0, 186.0])
                    } else {
                        Some(Mode::Relations.ramp((g.diplo.of(c, mine) + 1.0) / 2.0))
                    }
                }
                Mode::Seas => {
                    // (the coasts: ours and the enemy's)
                    let q = th.at.get(p).copied().unwrap_or(NO);
                    let coast = world.provinces[p].neighbors.iter().any(|&n| world.provinces[n as usize].sea);
                    (q != NO && coast).then(|| if war.owner[q as usize] == me { [96.0, 146.0, 206.0] } else { [196.0, 96.0, 74.0] })
                }
                Mode::Armies => {
                    let q = th.at.get(p).copied().unwrap_or(NO);
                    let k = if q != NO && war.owner[q as usize] == me { lab[q as usize] } else { usize::MAX };
                    (k != usize::MAX).then(|| {
                        let c = HUES[k % HUES.len()];
                        [c.r() as f32, c.g() as f32, c.b() as f32]
                    })
                }
                _ => None,
            })
            .collect();
        // each province: the country shown on it, whether it is held by the enemy of its owner,
        // which side of the war it belongs to (for the front), the country it is part of
        let shown: Vec<(u32, bool, u8, u32)> = (0..world.provinces.len())
            .map(|p| {
                let q = th.at.get(p).copied().unwrap_or(NO);
                if q != NO {
                    let q = q as usize;
                    let side = war.owner[q];
                    (th.countries[side] as u32, side != th.home[q], side as u8, th.countries[th.home[q]] as u32)
                } else {
                    (g.country_of[p], false, 9, g.country_of[p])
                }
            })
            .collect();
        Paint { shown, modal, tints, mode }
    }

    /// The table of provinces for the video card (see `scene::PAL_W`), and its rows.
    pub(super) fn palette(&self, g: &Game) -> (Vec<u8>, usize) {
        let pt = self.paint_of(g);
        let n = g.world.provinces.len() + 1;
        let rows = (n + scene::PAL_W - 1) / scene::PAL_W * 3;
        let mut v = vec![0u8; scene::PAL_W * rows * 4];
        let colour = |c: u32| -> [f32; 3] {
            if c == u32::MAX {
                [190.0, 190.0, 180.0]
            } else {
                pt.tints[c as usize]
            }
        };
        let rgba = |c: [f32; 3]| [c[0].clamp(0.0, 255.0) as u8, c[1].clamp(0.0, 255.0) as u8, c[2].clamp(0.0, 255.0) as u8, 255];
        let fogged = Self::fogged(g);
        for p in 0..g.world.provinces.len() {
            let id = p + 1;
            let at = |row: usize| (((id / scene::PAL_W) * 3 + row) * scene::PAL_W + id % scene::PAL_W) * 4;
            let (cty, held, side, home) = pt.shown[p];
            let tint = match pt.modal[p] {
                Some(t) => Some(t),
                None if matches!(pt.mode, Mode::Political | Mode::Terrain) => Some(colour(cty)),
                None => None,
            };
            if let Some(t) = tint {
                v[at(0)..at(0) + 4].copy_from_slice(&rgba(t));
            }
            // stripes: the land the past took — of the country that holds it now over the one
            // that lost it; the land the enemy holds — of its owner over the occupier
            let stripe = if pt.mode == Mode::Lost && pt.modal[p].is_some() {
                Some(colour(cty))
            } else if held && pt.mode == Mode::Political {
                Some(colour(home))
            } else {
                None
            };
            if let Some(s) = stripe {
                v[at(1)..at(1) + 4].copy_from_slice(&rgba(s));
            }
            let c = if cty == u32::MAX { 0 } else { cty + 1 };
            let q = g.h.war.th.at.get(p).copied().unwrap_or(NO);
            let fog = q != NO && fogged.get(q as usize).copied().unwrap_or(false);
            v[at(2)..at(2) + 4].copy_from_slice(&[(c & 255) as u8, (c >> 8) as u8, side, if g.world.provinces[p].sea { 255 } else if fog { 128 } else { 0 }]);
        }
        (v, rows)
    }

    /// The small map: the land lit, the countries' colours over it (from the table of provinces).
    pub(super) fn small_map(&mut self, ctx: &egui::Context, pal: &[u8]) {
        let (mw, mh) = (300usize, 150usize);
        let (w, h) = (self.ground.w, self.ground.h);
        let mut px = Vec::with_capacity(mw * mh);
        for y in 0..mh {
            for x in 0..mw {
                let (wx, wy) = ((x * w + w / 2) / mw, (y * h + h / 2) / mh);
                let mut c = self.ground.lit(wx, wy);
                let i = wy * w + wx;
                let id = self.ground.ids[i * 4] as usize + 256 * self.ground.ids[i * 4 + 1] as usize;
                if id > 0 && self.ground.sea[i] >= 0.0 {
                    let at = ((id / scene::PAL_W) * 3 * scene::PAL_W + id % scene::PAL_W) * 4;
                    if at + 3 < pal.len() && pal[at + 3] > 0 {
                        let lum = (0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]) / 110.0;
                        for k in 0..3 {
                            c[k] = c[k] * 0.4 + pal[at + k] as f32 * lum.clamp(0.55, 1.25) * 0.6;
                        }
                    }
                }
                px.push(Color32::from_rgb(c[0].clamp(0.0, 255.0) as u8, c[1].clamp(0.0, 255.0) as u8, c[2].clamp(0.0, 255.0) as u8));
            }
        }
        let small = ColorImage { size: [mw, mh], pixels: px };
        match &mut self.mini {
            Some(t) => t.set(small, TextureOptions::LINEAR),
            None => self.mini = Some(ctx.load_texture("mini", small, TextureOptions::LINEAR)),
        }
    }
}
