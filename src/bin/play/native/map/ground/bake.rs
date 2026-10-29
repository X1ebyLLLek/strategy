//! The layers of the world for the video card, made once per world: the land's height as the
//! player sees it, its kinds and their photographs, water, the light (the ranges' shadows, the
//! sky in the valleys, the woods' canopy, the basins).

use super::*;

pub fn layers(world: &World, extra: &Extra) -> Layers {
    let (w, h) = (world.w, world.h);
    let hmax = world.height.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let dmax = world.height.iter().cloned().fold(0.0f32, f32::min).abs().max(1e-6);
    let mut height: Vec<f32> = world.height.iter().map(|&v| if v > 0.0 { v / hmax } else { v / dmax }).collect();
    // the heights twice as fine (the world's own: where the water is)
    let (w2, h2) = (2 * w, 2 * h);
    let mut fine = twice(&height, w, h);
    // the ground's colour: by kind, carried under the water from the land, softened between kinds
    let wet: Vec<bool> = world.terrain.iter().map(|t| matches!(t, Terrain::Ocean | Terrain::Lake)).collect();
    let mut col: Vec<[f32; 3]> = world.terrain.iter().map(|&t| ground(t)).collect();
    let mut have: Vec<bool> = wet.iter().map(|&x| !x).collect();
    for _ in 0..4 {
        let (prev, src) = (have.clone(), col.clone());
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if prev[i] {
                    continue;
                }
                let mut acc = [0.0f32; 3];
                let mut n = 0.0;
                for j in [y * w + (x + w - 1) % w, y * w + (x + 1) % w, if y > 0 { i - w } else { i }, if y + 1 < h { i + w } else { i }] {
                    if prev[j] {
                        for k in 0..3 {
                            acc[k] += src[j][k];
                        }
                        n += 1.0;
                    }
                }
                if n > 0.0 {
                    col[i] = acc.map(|a| a / n);
                    have[i] = true;
                }
            }
        }
    }
    for _ in 0..2 {
        let src = col.clone();
        col.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            let (up, down) = (y.saturating_sub(1), (y + 1).min(h - 1));
            for x in 0..w {
                let mut acc = [0.0f32; 3];
                for yy in [up, y, down] {
                    for xx in [(x + w - 1) % w, x, (x + 1) % w] {
                        let c = src[yy * w + xx];
                        for k in 0..3 {
                            acc[k] += c[k];
                        }
                    }
                }
                row[x] = acc.map(|a| a / 9.0);
            }
        });
    }
    let mut base = vec![0u8; w * h * 4];
    for i in 0..w * h {
        for k in 0..3 {
            base[i * 4 + k] = col[i][k].clamp(0.0, 255.0) as u8;
        }
        base[i * 4 + 3] = world.terrain[i] as u8;
    }
    // how much of each kind of ground lies at each pixel: soft between kinds, carried under the
    // coast (the surf lies on sand and grass, not on nothing)
    let mut wt: Vec<[f32; 8]> = world
        .terrain
        .iter()
        .map(|&t| {
            let mut v = [0.0f32; 8];
            if let Some(k) = kind(t) {
                v[k] = 1.0;
            }
            v
        })
        .collect();
    let mut have: Vec<bool> = wet.iter().map(|&x| !x).collect();
    for _ in 0..4 {
        let (prev, src) = (have.clone(), wt.clone());
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if prev[i] {
                    continue;
                }
                let mut acc = [0.0f32; 8];
                let mut n = 0.0;
                for j in [y * w + (x + w - 1) % w, y * w + (x + 1) % w, if y > 0 { i - w } else { i }, if y + 1 < h { i + w } else { i }] {
                    if prev[j] {
                        for k in 0..8 {
                            acc[k] += src[j][k];
                        }
                        n += 1.0;
                    }
                }
                if n > 0.0 {
                    wt[i] = acc.map(|a| a / n);
                    have[i] = true;
                }
            }
        }
    }
    for i in 0..w * h {
        if !have[i] {
            wt[i][0] = 1.0;
        }
    }
    for _ in 0..3 {
        let src = wt.clone();
        wt.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            let (up, down) = (y.saturating_sub(1), (y + 1).min(h - 1));
            for x in 0..w {
                let mut acc = [0.0f32; 8];
                for yy in [up, y, down] {
                    for xx in [(x + w - 1) % w, x, (x + 1) % w] {
                        let c = src[yy * w + xx];
                        for k in 0..8 {
                            acc[k] += c[k];
                        }
                    }
                }
                row[x] = acc.map(|a| a / 9.0);
            }
        });
    }
    let mut splat = vec![0u8; w * h * 8];
    for i in 0..w * h {
        for k in 0..8 {
            splat[(k / 4) * w * h * 4 + i * 4 + k % 4] = (wt[i][k] * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    // the land as it is shown (the world's own heights decide the provinces and the kinds of
    // ground; for the eye the land is laid anew): lowlands along the coasts, rising gently inland
    // with the world's own large shape; the ranges standing up as massifs — high in their middle,
    // low at their feet — carved by the relief of real mountains, the hills by real hills (USGS
    // heights: `Relief`); the plains in long low swells; a few basins far inland below the sea
    let sea = height.clone();
    let sea_fine = fine.clone();
    let wet: Vec<bool> = sea.iter().map(|&v| v <= 0.0).collect();
    let wetf: Vec<bool> = sea_fine.iter().map(|&v| v <= 0.0).collect();
    let shore = reach(&wet, w, h);
    // (the distance's creases — straight lines where two coasts are equally far — smoothed away:
    // the land's rise inland must not show them)
    let mut inland: Vec<f32> = shore.iter().map(|&d| smoothstep(0.0, 60.0, d)).collect();
    soften(&mut inland, w, h, 5, 3);
    let mtn: Vec<f32> = wt.iter().map(|v| v[3]).collect();
    let rugged: Vec<f32> = wt.iter().map(|v| v[3] + 0.35 * v[2] + 0.08 * v[6] + 0.25 * v[7]).collect();
    // (how far into a range: the middle of a wide one stands highest)
    let outside: Vec<bool> = mtn.iter().zip(&wet).map(|(&m, &x)| m < 0.5 || x).collect();
    let into = reach(&outside, w, h);
    let mut lift: Vec<f32> = into.iter().map(|&d| 1.0 - (-d / 7.0).exp()).collect();
    soften(&mut lift, w, h, 3, 3);
    let mut hilb: Vec<f32> = wt.iter().map(|v| v[2]).collect();
    soften(&mut hilb, w, h, 4, 3);
    let mut mtnb = mtn.clone();
    soften(&mut mtnb, w, h, 2, 3);
    let flatness: Vec<f32> = wt.iter().map(|v| (v[0] + 0.8 * v[2] + 0.6 * v[4] + 0.6 * v[5] + 0.7 * v[6] + 0.4 * v[1]).clamp(0.0, 1.0)).collect();
    let mut elev: Vec<f32> = (0..w * h)
        .map(|i| {
            if wet[i] {
                return 0.0;
            }
            let c = sea[i].max(0.0);
            // (the window's spoiled control: the land of one height, as before — no massifs)
            let massif = if crate::native::spoiled() { 0.0 } else { MASSIF };
            0.010 + 0.07 * inland[i] + 0.10 * c * c + massif * lift[i] + HILLS * hilb[i]
        })
        .collect();
    let rough: Vec<f32> = mtnb.iter().zip(&hilb).map(|(a, b)| a + b).collect();
    // (the spoiled control: no basins)
    let (sink, floor) = if crate::native::spoiled() { (vec![0.0; w * h], vec![0.0; w * h]) } else { below_sea(world, &sea, &shore, &rough, &extra.rivers, &mut elev) };
    // twice as fine: the shape, then the relief of the ranges and the hills over it
    let mountains = Relief::load(include_bytes!("../../../../../../assets/terrain/relief_mountains.png"));
    let hills = Relief::load(include_bytes!("../../../../../../assets/terrain/relief_hills.png"));
    fine = twice(&elev, w, h);
    for (v, &x) in fine.iter_mut().zip(&wetf) {
        if x {
            *v = 0.0;
        }
    }
    // (the land's large shape, before the relief over it: the sun reads it steeper)
    let mut shape = fine.clone();
    fine.par_chunks_mut(w2).enumerate().for_each(|(yy, row)| {
        for (xx, v) in row.iter_mut().enumerate() {
            if wetf[yy * w2 + xx] {
                continue;
            }
            let (x, y) = ((xx as f32 + 0.5) / 2.0, (yy as f32 + 0.5) / 2.0);
            let (mb, lf, hb, fl) = (bil(&mtnb, w, h, x, y), bil(&lift, w, h, x, y), bil(&hilb, w, h, x, y), bil(&flatness, w, h, x, y));
            *v += RANGE_RELIEF * mb * (0.55 + 0.65 * lf) * mountains.world(x, y, w, 48.0)
                + (HILL_RELIEF * hb + 0.012 * mb) * hills.world(x, y, w, 64.0)
                + 0.024 * fl * (fbm(x, y, 1.0 / 40.0, 3, w) - 0.5);
        }
    });
    // (the highest peak as high as the world's own: the snow lies where it lay)
    {
        let top = fine.iter().zip(&wetf).filter(|(_, &x)| !x).map(|(v, _)| *v).fold(1e-3f32, f32::max);
        let k = 1.0 / top;
        for v in fine.iter_mut().chain(shape.iter_mut()) {
            *v *= k;
        }
        for v in elev.iter_mut() {
            *v *= k;
        }
        if std::env::var("MAPDUMP").is_ok() {
            let mut land: Vec<f32> = fine.iter().zip(&wetf).filter(|(_, &x)| !x).map(|(v, _)| *v).collect();
            land.sort_by(|a, b| a.total_cmp(b));
            let q = |f: f32| land[((land.len() - 1) as f32 * f) as usize];
            eprintln!("высоты суши: вершина до деления {top:.3}; 1% {:.3}, 25% {:.3}, 50% {:.3}, 75% {:.3}, 95% {:.3}, 99% {:.3}", q(0.01), q(0.25), q(0.5), q(0.75), q(0.95), q(0.99));
        }
    }
    // the rivers: distances to their middle lines (twice as fine as the world) and their
    // half-widths; each runs in a valley of its own, deeper and wider as it grows
    let mut detail = vec![0u8; w2 * h2 * 4];
    for px in detail.chunks_exact_mut(4) {
        px[0] = 128;
    }
    let mut lines = vec![255u8; w2 * h2 * 4];
    {
        let rl: Vec<(&[[f32; 2]], u8)> = extra.rivers.iter().map(|(p, wd)| (&p[..], ((wd / 2.0) / RIVER_HW * 255.0).round().clamp(1.0, 255.0) as u8)).collect();
        // (the river's half-width goes into the detail's third channel)
        let mut hw = vec![255u8; w2 * h2 * 4];
        super::super::ways::near_by(&mut hw, w, h, 2, 0, Some(2), &rl, RIVER_RANGE, RIVER_HW / 255.0);
        super::super::ways::near_by(&mut lines, w, h, 2, 0, None, &rl, RIVER_RANGE, RIVER_HW / 255.0);
        for (d, v) in detail.chunks_exact_mut(4).zip(hw.chunks_exact(4)) {
            d[2] = if v[0] == 255 && v[1] == 255 { 0 } else { v[2] };
        }
    }
    let carve: Vec<f32> = detail
        .par_chunks_exact(4)
        .zip(lines.par_chunks_exact(4))
        .map(|(px, ln)| {
            let (a, b) = super::super::ways::step_of(ln[0], ln[1], RIVER_RANGE);
            let dc = a.hypot(b);
            if dc >= RIVER_RANGE {
                return 0.0;
            }
            let hw = px[2] as f32 / 255.0 * RIVER_HW;
            let r = (dc / (hw + 0.9)).powi(2);
            // (a great river lies in a wide valley, not a deeper one)
            (0.006 + 0.03 * hw.min(0.7)) * (-r).exp()
        })
        .collect();
    let plain = elev.clone();
    height = elev;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !wet[i] {
                let f = |dx: usize, dy: usize| fine[(2 * y + dy) * w2 + 2 * x + dx];
                let c = |dx: usize, dy: usize| carve[(2 * y + dy) * w2 + 2 * x + dx];
                let cut = (c(0, 0) + c(1, 0) + c(0, 1) + c(1, 1)) / 4.0;
                height[i] = (f(0, 0) + f(1, 0) + f(0, 1) + f(1, 1)) / 4.0 - cut;
            }
        }
    }
    // (the relief in the land's own shape — a grid every two world pixels — softened: there it
    // stood as walls, and the photographs ran down them in streaks; the land stays above the sea
    // but in the basins)
    {
        let mut add: Vec<f32> = height.iter().zip(&plain).zip(&wet).map(|((a, b), &x)| if x { 0.0 } else { a - b }).collect();
        soften(&mut add, w, h, 1, 2);
        for i in 0..w * h {
            if !wet[i] {
                height[i] = (plain[i] + add[i]).max(0.003 - 0.2 * sink[i]);
            }
        }
    }
    fine.par_chunks_mut(w2).zip(carve.par_chunks(w2)).enumerate().for_each(|(yy, (row, cr))| {
        for xx in 0..w2 {
            if !wetf[yy * w2 + xx] {
                let low = 0.002 - 0.2 * bil(&sink, w, h, (xx as f32 + 0.5) / 2.0, (yy as f32 + 0.5) / 2.0);
                row[xx] = (row[xx] - cr[xx]).max(low);
            }
        }
    });
    // the folds of the land: higher than round about — a crest, lit; lower — a gully, in shade
    // (the fine heights less their blur, kept in the detail's first channel)
    {
        let mut blur = fine.clone();
        for _ in 0..2 {
            let src = blur.clone();
            blur.par_chunks_mut(w2).enumerate().for_each(|(y, row)| {
                for x in 0..w2 {
                    let mut a = 0.0;
                    for d in -3i64..=3 {
                        a += src[y * w2 + (x as i64 + d).rem_euclid(w2 as i64) as usize];
                    }
                    row[x] = a / 7.0;
                }
            });
            let src = blur.clone();
            blur.par_chunks_mut(w2).enumerate().for_each(|(y, row)| {
                for x in 0..w2 {
                    let mut a = 0.0;
                    for d in -3i64..=3 {
                        a += src[((y as i64 + d).clamp(0, h2 as i64 - 1) as usize) * w2 + x];
                    }
                    row[x] = a / 7.0;
                }
            });
        }
        detail.par_chunks_exact_mut(4).enumerate().for_each(|(i, px)| {
            let c = (fine[i] - blur[i]) * 45.0;
            px[0] = (128.0 + c.clamp(-1.0, 1.0) * 127.0).round() as u8;
        });
    }
    // the tilled land round the towns and villages: on gentle open ground (not in forests,
    // marshes, sand, snow or on mountains), fading out with the distance, broken at its edge
    {
        let open: Vec<f32> = wt.iter().map(|v| (v[0] + 0.7 * v[2] + 0.5 * v[6]).clamp(0.0, 1.0)).collect();
        let mut farm = vec![0.0f32; w2 * h2];
        for &(fx, fy, r) in &extra.farms {
            let (x0, x1) = (((fx - r) * 2.0).floor() as i64, ((fx + r) * 2.0).ceil() as i64);
            let (y0, y1) = ((((fy - r) * 2.0).floor() as i64).max(0), (((fy + r) * 2.0).ceil() as i64).min(h2 as i64 - 1));
            for yy in y0..=y1 {
                for xx in x0..=x1 {
                    let (px, py) = ((xx as f32 + 0.5) / 2.0, (yy as f32 + 0.5) / 2.0);
                    let d = (px - fx).hypot(py - fy) / r;
                    if d >= 1.0 {
                        continue;
                    }
                    let xw = xx.rem_euclid(w2 as i64) as usize;
                    let i = yy as usize * w2 + xw;
                    let v = 1.0 - smoothstep(0.45, 1.0, d + 0.35 * (fbm(px, py, 1.0 / 4.0, 2, w) - 0.5));
                    farm[i] = farm[i].max(v);
                }
            }
        }
        // (the towns' built-up ground: the detail's last channel; no fields there)
        let mut core = vec![0.0f32; w2 * h2];
        for &(fx, fy, r) in &extra.cores {
            let (x0, x1) = (((fx - r) * 2.0).floor() as i64, ((fx + r) * 2.0).ceil() as i64);
            let (y0, y1) = ((((fy - r) * 2.0).floor() as i64).max(0), (((fy + r) * 2.0).ceil() as i64).min(h2 as i64 - 1));
            for yy in y0..=y1 {
                for xx in x0..=x1 {
                    let (px, py) = ((xx as f32 + 0.5) / 2.0, (yy as f32 + 0.5) / 2.0);
                    let d = (px - fx).hypot(py - fy) / r;
                    if d >= 1.3 {
                        continue;
                    }
                    let i = yy as usize * w2 + xx.rem_euclid(w2 as i64) as usize;
                    let v = 1.0 - smoothstep(0.55, 1.15, d + 0.35 * (fbm(px, py, 1.0 / 2.0, 2, w) - 0.5));
                    core[i] = core[i].max(v);
                }
            }
        }
        detail.par_chunks_exact_mut(4).enumerate().for_each(|(i, px)| {
            let (x, y) = ((i % w2) / 2, (i / w2) / 2);
            let j = y * w + x;
            let flat = smoothstep(0.30, 0.10, (px[0] as f32 - 128.0).abs() / 127.0);
            let land = if wet[j] { 0.0 } else { 1.0 };
            let v = farm[i] * open[j] * flat * land * (1.0 - core[i]);
            px[1] = (v * 255.0).round() as u8;
            px[3] = (core[i] * land * 255.0).round() as u8;
        });
    }
    // the ways: the steps to the main roads (after the rivers' in `lines`), to the village roads
    // and the railways (in `ways`; the railways the players build are laid there later)
    let mut ways = vec![255u8; w2 * h2 * 4];
    for (ch, kind) in [(2usize, super::super::ways::MAIN), (0, super::super::ways::LOCAL), (2, super::super::ways::RAIL)] {
        let ls: Vec<(&[[f32; 2]], u8)> = extra.ways.iter().filter(|l| l.kind == kind).map(|l| (&l.pts[..], 0)).collect();
        let buf = if kind == super::super::ways::MAIN { &mut lines } else { &mut ways };
        super::super::ways::near(buf, w, h, 2, ch, None, &ls, WAY_RANGE);
    }
    // the normals: the land's slopes (the sea is flat to the sun; its waves are the card's) — the
    // large shape steep enough to read from afar, the crests and gullies over it gentler
    let z = |x: usize, y: usize| {
        let (f, s) = (fine[y * w2 + x], shape[y * w2 + x]);
        (s * SHADE + (f - s) * SHADE_FINE) * RELIEF
    };
    let mut normal = vec![0u8; w2 * h2 * 4];
    normal.par_chunks_mut(w2 * 4).enumerate().for_each(|(y, row)| {
        let (up, down) = (y.saturating_sub(1), (y + 1).min(h2 - 1));
        for x in 0..w2 {
            let (l, r) = ((x + w2 - 1) % w2, (x + 1) % w2);
            // (neighbours half a world pixel away on either side: one world pixel apart)
            let dx = z(r, y) - z(l, y);
            let dy = (z(x, down) - z(x, up)) * 2.0 / (down - up).max(1) as f32;
            let len = (dx * dx + dy * dy + 1.0).sqrt();
            let n = [-dx / len, -dy / len, 1.0 / len];
            for k in 0..3 {
                row[x * 4 + k] = ((n[k] * 0.5 + 0.5) * 255.0).round() as u8;
            }
            // (the ground's grain, in the spare channel)
            row[x * 4 + 3] = (fbm((x as f32 + 0.5) / 2.0, (y as f32 + 0.5) / 2.0, 1.0 / 32.0, 4, w) * 255.0).round() as u8;
        }
    });
    // broad dark patches of growth on gentle ground, in the spare channel of the ground's colour
    let gentle: Vec<f32> = wt.iter().map(|v| (v[0] + v[2] + v[5] + 0.6 * v[6] + 0.5 * v[1]).clamp(0.0, 1.0)).collect();
    base.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let blot = smoothstep(0.48, 0.78, fbm(x as f32 + 37.5, y as f32 + 11.5, 1.0 / 64.0, 3, w));
            row[x * 4 + 3] = (blot * gentle[y * w + x] * 255.0).round() as u8;
        }
    });
    let (tex, tex_size) = textures();
    let (aerial, aerial_size) = aerials();
    let mut ids = vec![0u8; w * h * 4];
    for (i, &p) in world.owner.iter().enumerate() {
        let v = if p == NONE { 0 } else { p + 1 };
        ids[i * 4] = (v & 255) as u8;
        ids[i * 4 + 1] = (v >> 8) as u8;
        ids[i * 4 + 3] = 255;
    }
    let forest: Vec<f32> = wt.iter().map(|v| v[1]).collect();
    let cold: Vec<f32> = wt.iter().map(|v| v[6] + v[7]).collect();
    // (no trees on the ways and in the rivers, nor on the tilled land)
    let clear = |x: f32, y: f32| -> bool {
        let (xx, yy) = (((x * 2.0) as usize).min(w2 - 1), ((y * 2.0) as usize).min(h2 - 1));
        let i = (yy * w2 + xx) * 4;
        let dc = super::super::ways::dist_at(&lines, w2, h2, 0, x, y, RIVER_RANGE) - detail[i + 2] as f32 / 255.0 * RIVER_HW;
        let road = super::super::ways::dist_at(&lines, w2, h2, 2, x, y, WAY_RANGE).min(super::super::ways::dist_at(&ways, w2, h2, 0, x, y, WAY_RANGE)).min(super::super::ways::dist_at(&ways, w2, h2, 2, x, y, WAY_RANGE));
        dc > 0.35 && road > 0.32 && detail[i + 1] < 90 && detail[i + 3] < 60
    };
    // the forests' canopy as the card draws it from above (twice as fine as the world): where the
    // forest is, broken at its edge, with glades in it; not over the fields and the towns, cut by
    // the rivers and the ways. The trees drawn one by one stand where the canopy is thin — at
    // its edges, in its glades, alone in the open.
    let wood = |x: f32, y: f32| -> f32 {
        let (sx, sy) = (x - 0.5, (y - 0.5).clamp(0.0, h as f32 - 1.001));
        let (ix, iy) = (sx.floor(), sy.floor());
        let (fx, fy) = (sx - ix, sy - iy);
        let x0 = (ix as i64).rem_euclid(w as i64) as usize;
        let x1 = (x0 + 1) % w;
        let (y0, y1) = (iy as usize, (iy as usize + 1).min(h - 1));
        let a = forest[y0 * w + x0] * (1.0 - fx) + forest[y0 * w + x1] * fx;
        let b = forest[y1 * w + x0] * (1.0 - fx) + forest[y1 * w + x1] * fx;
        a * (1.0 - fy) + b * fy
    };
    let canopy: Vec<f32> = (0..w2 * h2)
        .into_par_iter()
        .map(|i| {
            let (xx, yy) = (i % w2, i / w2);
            let (x, y) = ((xx as f32 + 0.5) / 2.0, (yy as f32 + 0.5) / 2.0);
            let f = wood(x, y);
            if f < 0.15 || wetf[i] {
                return 0.0;
            }
            let edge = 0.34 * (fbm(x, y, 1.0 / 5.0, 3, w) - 0.5);
            let glade = 0.60 * smoothstep(0.62, 0.72, fbm(x + 91.0, y + 17.0, 1.0 / 18.0, 3, w));
            let c = smoothstep(0.42, 0.60, f + edge - glade);
            if c <= 0.0 {
                return 0.0;
            }
            let d = &detail[i * 4..i * 4 + 4];
            let dc = super::super::ways::dist_at(&lines, w2, h2, 0, x, y, RIVER_RANGE) - d[2] as f32 / 255.0 * RIVER_HW;
            let road = super::super::ways::dist_at(&lines, w2, h2, 2, x, y, WAY_RANGE).min(super::super::ways::dist_at(&ways, w2, h2, 0, x, y, WAY_RANGE)).min(super::super::ways::dist_at(&ways, w2, h2, 2, x, y, WAY_RANGE));
            c * smoothstep(0.22, 0.45, dc) * smoothstep(0.12, 0.28, road) * (1.0 - d[1] as f32 / 255.0 * 1.6).max(0.0) * (1.0 - d[3] as f32 / 255.0 * 2.0).max(0.0)
        })
        .collect();
    // (the forests are the photographs of the canopy on the land; trees one by one only to
    // compare, with `MAPTREES=1`: beside a photograph a model reads as a toy)
    let trees = if std::env::var("MAPTREES").is_ok() { super::super::objects::trees(world, &sea, &forest, &cold, &rugged, &canopy, &clear) } else { super::super::objects::Trees::default() };
    let t0 = std::time::Instant::now();
    let mut light = light(&fine, &wetf, w2, h2);
    // (the canopy; the basins below the sea: their slopes up to a half, their flat floors above)
    light.par_chunks_exact_mut(4).enumerate().for_each(|(i, px)| {
        px[2] = (canopy[i] * 255.0).round() as u8;
        let (x, y) = (((i % w2) as f32 + 0.5) / 2.0, ((i / w2) as f32 + 0.5) / 2.0);
        px[3] = ((0.5 * bil(&sink, w, h, x, y) + 0.5 * bil(&floor, w, h, x, y)).clamp(0.0, 1.0) * 255.0).round() as u8;
    });
    if std::env::var("MAPDUMP").is_ok() {
        eprintln!("свет земли: {:.2} с", t0.elapsed().as_secs_f32());
    }
    if let Ok(path) = std::env::var("MAPDUMP") {
        let im: Vec<u8> = light.chunks_exact(4).flat_map(|p| [p[0], p[1], 0]).collect();
        let _ = image::save_buffer(&path, &im, w2 as u32, h2 as u32, image::ColorType::Rgb8);
        let sh: Vec<u8> = normal.chunks_exact(4).flat_map(|p| { let n = [p[0] as f32 / 127.5 - 1.0, p[1] as f32 / 127.5 - 1.0, p[2] as f32 / 127.5 - 1.0]; let d = ((n[0] * SUN[0] + n[1] * SUN[1] + n[2] * SUN[2]).max(0.0) * 255.0) as u8; [d, d, d] }).collect();
        let _ = image::save_buffer(path.replace(".png", "_n.png"), &sh, w2 as u32, h2 as u32, image::ColorType::Rgb8);
    }
    Layers { w, h, height, sea, normal, base, ids, splat, tex, tex_size, aerial, aerial_size, trees, detail, lines, ways, light }
}

/// How much light each place of the land gets, twice as fine as the world, RGBA: the sun (the
/// shadows the ranges throw: the steepest rise of the land towards the sun against the sun's own
/// height, soft at the edge), the sky (less in the valleys, deep among higher land round about).
fn light(fine: &[f32], wet: &[bool], w2: usize, h2: usize) -> Vec<u8> {
    let l = (SUN[0] * SUN[0] + SUN[1] * SUN[1]).sqrt();
    let (dx, dy) = (SUN[0] / l, SUN[1] / l);
    // (fine texels are half a world pixel: steps in world pixels are two texels)
    let sun_rise = SUN[2] / l;
    let zf = |x: f32, y: f32| -> f32 {
        let y = y.clamp(0.0, h2 as f32 - 1.001);
        let (ix, iy) = (x.floor(), y.floor());
        let (fx, fy) = (x - ix, y - iy);
        let x0 = (ix as i64).rem_euclid(w2 as i64) as usize;
        let x1 = (x0 + 1) % w2;
        let (y0, y1) = (iy as usize, (iy as usize + 1).min(h2 - 1));
        let a = fine[y0 * w2 + x0] * (1.0 - fx) + fine[y0 * w2 + x1] * fx;
        let b = fine[y1 * w2 + x0] * (1.0 - fx) + fine[y1 * w2 + x1] * fx;
        (a * (1.0 - fy) + b * fy) * RELIEF * SHADOW_LIFT
    };
    let top = fine.iter().cloned().fold(0.0f32, f32::max) * RELIEF * SHADOW_LIFT;
    let sun_angle = sun_rise.atan();
    let mut sun = vec![255u8; w2 * h2];
    sun.par_chunks_mut(w2).enumerate().for_each(|(y, row)| {
        for x in 0..w2 {
            let z0 = fine[y * w2 + x];
            if wet[y * w2 + x] {
                continue;
            }
            let z0 = z0 * RELIEF * SHADOW_LIFT;
            // (as far as a ray towards the sun could still meet higher land)
            let reach = (top - z0) / sun_rise;
            // (from a world pixel off: the slope at the place itself is the sun's angle to it —
            // counted there once — and its small folds would hatch the shadow's edge)
            let mut best = -1.0f32;
            let mut t = 1.0f32;
            while t <= reach {
                let rise = (zf(x as f32 + dx * 2.0 * t, y as f32 + dy * 2.0 * t) - z0) / t;
                best = best.max(rise);
                t += 0.5 + t * 0.06;
            }
            let v = smoothstep(-0.22, 0.22, sun_angle - best.atan());
            row[x] = (v * 255.0).round() as u8;
        }
    });
    // (the shadow's edge softened: a few texels' box twice)
    for _ in 0..2 {
        let src = sun.clone();
        sun.par_chunks_mut(w2).enumerate().for_each(|(y, row)| {
            for x in 0..w2 {
                let mut a = 0u32;
                for yy in [y.saturating_sub(1), y, (y + 1).min(h2 - 1)] {
                    for d in [w2 - 1, 0, 1] {
                        a += src[yy * w2 + (x + d) % w2] as u32;
                    }
                }
                row[x] = (a / 9) as u8;
            }
        });
    }
    // the sky: the land against its round about, blurred over a few world pixels (three box
    // passes: near a bell) — deeper than its round about, less sky
    let mut blur: Vec<f32> = fine.to_vec();
    let r = 10i64;
    for _ in 0..3 {
        let src = blur.clone();
        blur.par_chunks_mut(w2).enumerate().for_each(|(y, row)| {
            let mut a: f32 = (-r..=r).map(|d| src[y * w2 + d.rem_euclid(w2 as i64) as usize]).sum();
            for x in 0..w2 {
                row[x] = a / (2 * r + 1) as f32;
                a += src[y * w2 + (x as i64 + r + 1).rem_euclid(w2 as i64) as usize] - src[y * w2 + (x as i64 - r).rem_euclid(w2 as i64) as usize];
            }
        });
        let src = blur.clone();
        let col = |x: usize, y: i64| src[(y.clamp(0, h2 as i64 - 1) as usize) * w2 + x];
        let mut out = vec![0.0f32; w2 * h2];
        // (down the columns, a band of columns at a time)
        let cols: Vec<Vec<f32>> = (0..w2)
            .into_par_iter()
            .map(|x| {
                let mut c = vec![0.0f32; h2];
                let mut a: f32 = (-r..=r).map(|d| col(x, d)).sum();
                for y in 0..h2 {
                    c[y] = a / (2 * r + 1) as f32;
                    a += col(x, y as i64 + r + 1) - col(x, y as i64 - r);
                }
                c
            })
            .collect();
        for (x, c) in cols.iter().enumerate() {
            for y in 0..h2 {
                out[y * w2 + x] = c[y];
            }
        }
        blur = out;
    }
    let mut out = vec![255u8; w2 * h2 * 4];
    out.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
        px[0] = sun[i];
        let deep = (blur[i] - fine[i]).max(0.0) * RELIEF;
        px[1] = ((-0.15 * deep).exp() * 255.0).round() as u8;
        px[2] = 0;
    });
    out
}

impl Layers {
    /// The height at a world point (bilinear; x goes round), as the land stands: world pixels.
    pub fn z(&self, x: f32, y: f32) -> f32 {
        let (w, h) = (self.w as f32, self.h as f32);
        if y < 0.0 || y >= h - 1.0 {
            return 0.0;
        }
        let x = (x - 0.5).rem_euclid(w);
        let y = (y - 0.5).max(0.0);
        let (ix, iy) = (x.floor() as usize, y.floor() as usize);
        let (fx, fy) = (x - ix as f32, y - iy as f32);
        let at = |xx: usize, yy: usize| self.height[yy.min(self.h - 1) * self.w + xx % self.w];
        let v = (at(ix, iy) * (1.0 - fx) + at(ix + 1, iy) * fx) * (1.0 - fy) + (at(ix, iy + 1) * (1.0 - fx) + at(ix + 1, iy + 1) * fx) * fy;
        v * RELIEF
    }

    /// The lit colour of the ground at a world pixel (for the small map), or the sea's.
    pub fn lit(&self, x: usize, y: usize) -> [f32; 3] {
        let i = y * self.w + x;
        let hgt = self.sea[i];
        if hgt < 0.0 {
            let d = (-hgt).sqrt().min(1.0);
            return [61.0 - 50.0 * d, 128.0 - 95.0 * d, 140.0 - 71.0 * d];
        }
        let j = ((2 * y) * (2 * self.w) + 2 * x) * 4;
        let n = [0, 1, 2].map(|k| self.normal[j + k] as f32 / 127.5 - 1.0);
        let sun = [-0.45f32, -0.55, 0.70];
        let l = (sun[0] * sun[0] + sun[1] * sun[1] + sun[2] * sun[2]).sqrt();
        let d = ((n[0] * sun[0] + n[1] * sun[1] + n[2] * sun[2]) / l).max(0.0);
        [0, 1, 2].map(|k| self.base[i * 4 + k] as f32 * (0.45 + 0.75 * d))
    }
}
