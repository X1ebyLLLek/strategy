//! The lines of the map drawn over the land: the borders of countries and provinces (smooth,
//! by the era's look), dashed lines, the rivers and roads laid for the video card.

use super::*;

impl MapView {
    /// The borders of the world as smooth chains of points, with what lies on either side.
    pub(super) fn borders(g: &Game) -> Vec<(Vec<[f32; 2]>, [f32; 4], u32, u32)> {
        use strategy::world::shapes::{Shapes, FRAME, LAKE, SEAM};
        let sh = Shapes::of(&g.world);
        let side = |v: u32| -> u32 {
            let v = if v >= SEAM { v - SEAM } else { v };
            if v == LAKE || v == FRAME || v as usize >= g.world.provinces.len() || g.world.provinces[v as usize].sea {
                u32::MAX
            } else {
                v
            }
        };
        sh.chains
            .into_iter()
            .filter(|c| c.pts.len() >= 2)
            .map(|c| {
                let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
                for p in &c.pts {
                    b = [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])];
                }
                let (l, r) = (side(c.left), side(c.right));
                (c.pts, b, l, r)
            })
            .filter(|c| !(c.2 == u32::MAX && c.3 == u32::MAX))
            .collect()
    }

    /// The borders close up, lying on the land: the coast, the countries' borders, the front, the
    /// provinces' (closest).
    pub(super) fn smooth_borders(&self, painter: &egui::Painter, g: &Game) {
        let w = g.world.w as f32;
        let war = &g.h.war;
        let th = &war.th;
        let shown = |p: u32| -> (u32, u8) {
            let q = th.at.get(p as usize).copied().unwrap_or(NO);
            if q != NO {
                let side = war.owner[q as usize];
                (th.countries[side] as u32, side as u8)
            } else {
                (g.country_of[p as usize], 9)
            }
        };
        let seen = self.cam.seen(RELIEF);
        // (how much of each kind of border shows at this zoom, as in HOI4: the countries' always,
        // their colours along them from the middle zoom, the states' from the middle, the
        // provinces' close up — each coming in smoothly, not at a step)
        let ramp = |a: f32, b: f32| {
            let t = ((self.k - a) / (b - a)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        let coast_a = ramp(1.0, 1.6);
        let band_a = ramp(1.1, 2.0);
        let state_a = ramp(1.1, 1.8);
        // Provinces must become legible before the close-up view.  At this distance the ground
        // already has enough detail (especially tree canopies and rock) to hide a single dark
        // hairline, so bring the vector contours in while the GPU contours are still fading out.
        let prov_a = ramp(1.55, 2.45);
        let country_w = 1.3 + 1.3 * ramp(0.8, 3.0);
        let band_w = 1.5 + 2.0 * ramp(1.1, 4.0);
        let colour_of = |c: u32| g.countries.get(c as usize).map_or(Color32::from_gray(120), |x| Color32::from_rgb(x.1[0], x.1[1], x.1[2]));
        let state_of = |p: u32| g.state_of.get(p as usize).copied().unwrap_or(u32::MAX);
        let mut bands: Vec<(Vec<Pos2>, Color32, Color32)> = Vec::new();
        let mut countries: Vec<Vec<Pos2>> = Vec::new();
        // the front: a thick live line along where the armies meet, ours on our side (green),
        // the enemy's on his (red), light running along it (at every zoom)
        let mut front: Vec<(Vec<Pos2>, bool, usize, bool, f32, f32, f32, f32)> = Vec::new();
        let lab = war.group_of(g.me);
        let mut lit_border: Vec<(Vec<Pos2>, bool, usize, bool)> = Vec::new();
        let all_groups = self.mode == Mode::Armies;
        let chosen = self.chosen_all();
        let strong = |k: usize| all_groups || chosen.contains(&k);
        let facing_of = |k: usize| war.command[g.me].as_ref().and_then(|gs| gs.get(k)).is_some_and(|gr| gr.facing.is_some());
        for (pts, b, l, r) in &self.borders {
            // the copy of the chain nearest the view
            let mid = (b[0] + b[2]) / 2.0;
            let dx = ((self.cx - mid) / w).round() * w;
            if b[2] + dx < seen[0] || b[0] + dx > seen[2] || b[3] < seen[1] || b[1] > seen[3] {
                continue;
            }
            let at_front = *l != u32::MAX && *r != u32::MAX && g.h.at_war && {
                let (sa, sb) = (shown(*l).1, shown(*r).1);
                sa != 9 && sb != 9 && sa != sb
            };
            // (in peace: the border with the rival, lit in a group's colour where it is its stretch)
            if !at_front && *l != u32::MAX && *r != u32::MAX && !g.h.at_war && {
                let (sa, sb) = (shown(*l).1, shown(*r).1);
                sa != 9 && sb != 9 && sa != sb
            } {
                let ours = if shown(*l).1 as usize == g.me { *l } else { *r };
                let q = th.at.get(ours as usize).copied().unwrap_or(NO);
                let k = if q != NO { lab.get(q as usize).copied().unwrap_or(usize::MAX) } else { usize::MAX };
                if k != usize::MAX && !facing_of(k) {
                    let line: Vec<Pos2> = pts.iter().map(|p| self.cam.project_abs(p[0] + dx, p[1], self.z(p[0], p[1])).0).collect();
                    lit_border.push((line, shown(*l).1 as usize != g.me, k, strong(k)));
                }
            }
            // (the border with another neighbour, where a group of ours stands facing him)
            if *l != u32::MAX && *r != u32::MAX && {
                let (sa, sb) = (shown(*l).1, shown(*r).1);
                (sa as usize == g.me && sb == 9) || (sb as usize == g.me && sa == 9)
            } {
                let ours = if shown(*l).1 as usize == g.me { *l } else { *r };
                let q = th.at.get(ours as usize).copied().unwrap_or(NO);
                let k = if q != NO { lab.get(q as usize).copied().unwrap_or(usize::MAX) } else { usize::MAX };
                if k != usize::MAX && facing_of(k) {
                    let line: Vec<Pos2> = pts.iter().map(|p| self.cam.project_abs(p[0] + dx, p[1], self.z(p[0], p[1])).0).collect();
                    lit_border.push((line, shown(*l).1 as usize != g.me, k, strong(k)));
                }
            }
            if at_front {
                let line: Vec<Pos2> = pts.iter().map(|p| self.cam.project_abs(p[0] + dx, p[1], self.z(p[0], p[1])).0).collect();
                // (the chain's right-hand province on the screen is its `l`: our colour on our side;
                // the group whose stretch it is, if it is lit)
                let ours = if shown(*l).1 as usize == g.me { *l } else { *r };
                let foe = if shown(*l).1 as usize == g.me { *r } else { *l };
                let q = th.at.get(ours as usize).copied().unwrap_or(NO);
                let q_foe = th.at.get(foe as usize).copied().unwrap_or(NO);
                let k = if q != NO { lab.get(q as usize).copied().unwrap_or(usize::MAX) } else { usize::MAX };
                let fort_ours = if q != NO { war.fort.get(q as usize).copied().unwrap_or(0.0) } else { 0.0 };
                let fort_foe = if q_foe != NO { war.fort.get(q_foe as usize).copied().unwrap_or(0.0) } else { 0.0 };
                let dug_ours = if q != NO {
                    war.corps.as_ref().map_or(0.0, |c| c.divs.iter().filter(|d| d.side == g.me && d.at == q as usize).map(|d| d.dug).fold(0.0f32, f32::max))
                } else { 0.0 };
                let dug_foe = if q_foe != NO {
                    war.corps.as_ref().map_or(0.0, |c| c.divs.iter().filter(|d| d.side != g.me && d.at == q_foe as usize).map(|d| d.dug).fold(0.0f32, f32::max))
                } else { 0.0 };
                front.push((line, shown(*l).1 as usize != g.me, k, k != usize::MAX && strong(k), fort_ours, fort_foe, dug_ours, dug_foe));
                continue;
            }
            let mut halo = false;
            let stroke = if *l == u32::MAX || *r == u32::MAX {
                if coast_a <= 0.01 {
                    continue;
                }
                Stroke::new(1.4, Color32::from_rgba_unmultiplied(40, 52, 60, (200.0 * coast_a) as u8))
            } else {
                let (ca, _) = shown(*l);
                let (cb, _) = shown(*r);
                if ca != cb {
                    // (a country's border: a dark line, and each country's colour along its side)
                    let line: Vec<Pos2> = pts.iter().map(|p| self.cam.project_abs(p[0] + dx, p[1], self.z(p[0], p[1])).0).collect();
                    if band_a > 0.01 {
                        let a = (150.0 * band_a) as u8;
                        let (la, lb) = (colour_of(ca), colour_of(cb));
                        bands.push((line.clone(), Color32::from_rgba_unmultiplied(la.r(), la.g(), la.b(), a), Color32::from_rgba_unmultiplied(lb.r(), lb.g(), lb.b(), a)));
                    }
                    countries.push(line);
                    continue;
                } else if state_of(*l) != state_of(*r) {
                    if state_a <= 0.01 {
                        continue;
                    }
                    // Границы регионов: плотные, но мягкого земляного цвета, без прозрачности
                    let dynamic_state_w = 1.5 + (self.k - 1.1).max(0.0).min(1.5) * 0.3;
                    halo = true;
                    Stroke::new(dynamic_state_w, Color32::from_rgb(50, 43, 35))
                } else if prov_a > 0.01 {
                    // Делаем линии провинций тонкими и приглушенными по прозрачности (95.0 вместо 165.0),
                    // чтобы они не превращали карту в миллиметровую бумагу при отдалении.
                    let width = 0.45 + (self.k - 1.55).max(0.0).min(2.0) * 0.10;
                    let guard = 0.85 + width;
                    let line: Vec<Pos2> = pts.iter().map(|p| self.cam.project_abs(p[0] + dx, p[1], self.z(p[0], p[1])).0).collect();
                    painter.add(egui::Shape::line(
                        line.clone(),
                        Stroke::new(guard, Color32::from_rgba_unmultiplied(18, 21, 18, (130.0 * prov_a) as u8)),
                    ));
                    painter.add(egui::Shape::line(
                        line,
                        Stroke::new(width, Color32::from_rgba_unmultiplied(205, 200, 184, (120.0 * prov_a) as u8)),
                    ));
                    continue;
                } else {
                    continue;
                }
            };
            let line: Vec<Pos2> = pts.iter().map(|p| self.cam.project_abs(p[0] + dx, p[1], self.z(p[0], p[1])).0).collect();
            // (a light under-stroke: a dark line alone is lost in a forest's or a range's shade; dark on
            // light it reads on any ground, and neither is loud)
            if halo {
                painter.add(egui::Shape::line(line.clone(), Stroke::new(stroke.width + 2.2, Color32::from_rgba_unmultiplied(238, 230, 208, 62))));
            }
            painter.add(egui::Shape::line(line, stroke));
        }
        // the countries' borders over the states' and provinces': each country's colour along its
        // side (the chain's `l` lies on the side its normal points away from), the dark line on top
        let offset = |line: &[Pos2], d: f32| -> Vec<Pos2> {
            (0..line.len())
                .map(|i| {
                    let a = line[i.saturating_sub(1)];
                    let b = line[(i + 1).min(line.len() - 1)];
                    let v = b - a;
                    let n = vec2(v.y, -v.x).normalized();
                    let n = if n.x.is_finite() { n } else { vec2(0.0, 0.0) };
                    line[i] + n * d
                })
                .collect()
        };
        let off = country_w / 2.0 + band_w / 2.0;
        for (line, cl, cr) in &bands {
            if line.len() < 2 {
                continue;
            }
            painter.add(egui::Shape::line(offset(line, -off), Stroke::new(band_w, *cl)));
            painter.add(egui::Shape::line(offset(line, off), Stroke::new(band_w, *cr)));
        }
        for line in countries {
            painter.add(egui::Shape::line(line.clone(), Stroke::new(country_w + 2.6, Color32::from_rgba_unmultiplied(240, 232, 210, 74))));
            painter.add(egui::Shape::line(line, Stroke::new(country_w, Color32::from_rgba_unmultiplied(22, 18, 14, 235))));
        }
        let t = self.t();
        // (28th session: thinner — the front was a band of four colours eleven points wide)
        // (30th session: slimmer still — a thick red line round every lone province was a tangle of loops over the front)
        let half = (0.8 + 0.22 * self.k.min(4.0)).min(1.7);
        for (line, ours_left, k, strong) in &lit_border {
            // (a border an army guards in peace: lit only for the chosen armies, as HOI4 shows the
            // front of the army in hand)
            if line.len() < 2 || !*strong {
                continue;
            }
            let half = if *strong { half } else { half * 0.7 };
            let side: Vec<Pos2> = (0..line.len())
                .map(|i| {
                    let a = line[i.saturating_sub(1)];
                    let b = line[(i + 1).min(line.len() - 1)];
                    let d = b - a;
                    let n = vec2(d.y, -d.x).normalized();
                    let n = if n.x.is_finite() { n } else { vec2(0.0, 0.0) };
                    line[i] + n * if *ours_left { half } else { -half }
                })
                .collect();
            let glow = if *strong { 0.75 + 0.25 * (t * 2.4).sin() } else { 0.8 };
            painter.add(egui::Shape::line(side.clone(), Stroke::new(2.0 * half + 3.0, Color32::from_black_alpha(130))));
            painter.add(egui::Shape::line(side, Stroke::new(2.0 * half, HUES[*k % HUES.len()].gamma_multiply(glow))));
        }
        let ramp_close = ((self.k - 1.15) / 1.05).clamp(0.0, 1.0);
        for (line, ours_left, k, strong, fort_ours, fort_theirs, dug_ours, dug_theirs) in &front {
            if line.len() < 2 {
                continue;
            }
            let hue = (*k != usize::MAX).then(|| HUES[*k % HUES.len()]);
            // (each point moved to either side along the line's normal on the screen)
            let side = |sgn: f32| -> Vec<Pos2> {
                (0..line.len())
                    .map(|i| {
                        let a = line[i.saturating_sub(1)];
                        let b = line[(i + 1).min(line.len() - 1)];
                        let d = b - a;
                        let n = vec2(d.y, -d.x).normalized();
                        let n = if n.x.is_finite() { n } else { vec2(0.0, 0.0) };
                        line[i] + n * sgn
                    })
                    .collect()
            };
            let (ours, theirs) = if *ours_left { (side(0.7 * half), side(-0.8 * half)) } else { (side(-0.7 * half), side(0.8 * half)) };
            painter.add(egui::Shape::line(line.clone(), Stroke::new(2.9 * half + 1.0, Color32::from_black_alpha(105))));
            painter.add(egui::Shape::line(theirs.clone(), Stroke::new(1.5 * half, Color32::from_rgb(214, 52, 40))));
            match hue {
                // (a group's stretch, not chosen: our side as any of ours — the armies' colours only
                // for the chosen, else the front is a tangle of colours)
                Some(_) if !*strong => {
                    painter.add(egui::Shape::line(ours.clone(), Stroke::new(0.95 * half, Color32::from_rgb(110, 176, 86))));
                }
                Some(c) => {
                    let wide = if *ours_left { side(2.0 * half) } else { side(-2.0 * half) };
                    let glow = 0.75 + 0.25 * (t * 2.4).sin();
                    painter.add(egui::Shape::line(wide.clone(), Stroke::new(4.0 * half + 3.0, Color32::from_black_alpha(140))));
                    painter.add(egui::Shape::line(wide, Stroke::new(3.0 * half, c.gamma_multiply(glow))));
                }
                None => {
                    painter.add(egui::Shape::line(ours.clone(), Stroke::new(0.95 * half, Color32::from_rgb(110, 176, 86))));
                }
            }
            // (light running along it: the front is alive)
            painter.extend(egui::Shape::dashed_line_with_offset(line, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 236, 170, 70)), &[6.0], &[18.0], (t * 14.0) % 24.0));

            // Линии окопов, бастионов и дотов на рельефе карты
            if ramp_close > 0.01 {
                let draw_fort_line = |_pts: &[Pos2], sgn: f32, fort_lvl: f32, dug_lvl: f32, friendly: bool| {
                    if fort_lvl <= 0.01 && dug_lvl <= 0.15 {
                        return;
                    }
                    let t_line = side(sgn * 2.5 * half);
                    let trench_a = (ramp_close * (if fort_lvl > 0.01 { 1.0 } else { (dug_lvl * 1.5).min(1.0) })).clamp(0.0, 1.0);
                    // Земляной ров окопа
                    let ditch_col = if friendly { Color32::from_rgba_unmultiplied(42, 36, 26, (215.0 * trench_a) as u8) } else { Color32::from_rgba_unmultiplied(46, 32, 28, (215.0 * trench_a) as u8) };
                    painter.add(egui::Shape::line(t_line.clone(), Stroke::new(2.4 * half, ditch_col)));
                    // Деревянные мостки и настил на дне окопа
                    let revet_col = Color32::from_rgba_unmultiplied(92, 76, 52, (185.0 * trench_a) as u8);
                    painter.add(egui::Shape::line(t_line.clone(), Stroke::new(0.85 * half, revet_col)));

                    // Зубчатые траверсы и капониры вдоль траншеи
                    if t_line.len() >= 3 {
                        for i in (1..t_line.len() - 1).step_by(2) {
                            let p = t_line[i];
                            let prev = t_line[i - 1];
                            let next = t_line[i + 1];
                            let d = (next - prev).normalized();
                            let norm_spur = vec2(d.y, -d.x) * (sgn.signum() * 3.6 * half);
                            painter.line_segment([p, p + norm_spur], Stroke::new(1.8 * half, ditch_col));
                            painter.line_segment([p, p + norm_spur * 0.85], Stroke::new(0.75 * half, revet_col));
                        }
                    }

                    // Железобетонные ДОТы и бастионы укрепрайона
                    if fort_lvl > 0.01 {
                        let step_pts = if t_line.len() > 6 { vec![t_line[t_line.len() / 3], t_line[t_line.len() * 2 / 3]] } else { vec![t_line[t_line.len() / 2]] };
                        for b_pos in step_pts {
                            let b_r = (3.6 + 1.2 * fort_lvl.min(1.0)) * half;
                            // Мягкая тень на земле
                            painter.circle_filled(b_pos + vec2(0.5, 1.2), b_r + 1.0, Color32::from_black_alpha((130.0 * trench_a) as u8));
                            // Бетонный купол ДОТа / бастиона
                            let conc_col = if friendly { Color32::from_rgb(118, 122, 114) } else { Color32::from_rgb(128, 118, 114) };
                            painter.circle_filled(b_pos, b_r, conc_col);
                            painter.circle_stroke(b_pos, b_r, Stroke::new(1.0, Color32::from_rgb(58, 62, 58)));
                            // Орудийная / пулеметная амбразура
                            let slit_w = vec2(-sgn.signum() * 1.5, 0.0);
                            painter.line_segment([b_pos + slit_w - vec2(0.0, 1.6), b_pos + slit_w + vec2(0.0, 1.6)], Stroke::new(1.3, Color32::from_rgb(18, 16, 14)));
                            // Бронебашня для II ступени укреплений
                            if fort_lvl >= 0.45 {
                                painter.circle_filled(b_pos, b_r * 0.48, Color32::from_rgb(52, 55, 54));
                                painter.circle_stroke(b_pos, b_r * 0.48, Stroke::new(0.8, Color32::from_rgb(32, 34, 33)));
                            }
                        }

                        // Противотанковые надолбы (зубы дракона)
                        let teeth_line = side(sgn * 1.4 * half);
                        if teeth_line.len() >= 2 {
                            for i in (0..teeth_line.len()).step_by(2) {
                                let tp = teeth_line[i];
                                let tw = 1.6 * half;
                                painter.add(egui::Shape::convex_polygon(
                                    vec![tp - vec2(tw, 0.0), tp + vec2(0.0, -tw * 1.3), tp + vec2(tw, 0.0)],
                                    Color32::from_rgba_unmultiplied(148, 150, 144, (200.0 * trench_a) as u8),
                                    Stroke::new(0.6, Color32::from_black_alpha(150)),
                                ));
                            }
                        }
                    }

                    // Полоса проволочных заграждений (колючая проволока на кольях) в нейтральной полосе
                    if fort_lvl > 0.01 || dug_lvl > 0.25 {
                        let wire_line = side(sgn * 0.95 * half);
                        if wire_line.len() >= 2 {
                            let wire_col = Color32::from_rgba_unmultiplied(162, 156, 146, (200.0 * trench_a) as u8);
                            let post_col = Color32::from_rgba_unmultiplied(64, 52, 38, (220.0 * trench_a) as u8);
                            // Продольные нити колючей проволоки
                            painter.add(egui::Shape::line(wire_line.clone(), Stroke::new(0.7 * half, wire_col)));
                            // Опорные деревянные и железные колья с диагональными растяжками
                            for i in 0..wire_line.len() {
                                let wp = wire_line[i];
                                let slant = if i % 2 == 0 { vec2(0.8, -2.6) } else { vec2(-0.8, -2.6) } * half;
                                painter.line_segment([wp + vec2(0.3, 0.8), wp + slant + vec2(0.3, 0.8)], Stroke::new(1.1 * half, Color32::from_black_alpha((110.0 * trench_a) as u8)));
                                painter.line_segment([wp, wp + slant], Stroke::new(1.1 * half, post_col));
                                // Крестообразные витки проволоки между кольями
                                if i + 1 < wire_line.len() {
                                    let next_wp = wire_line[i + 1];
                                    let mid_p = pos2((wp.x + next_wp.x) * 0.5, (wp.y + next_wp.y) * 0.5) + vec2(0.0, -1.2 * half);
                                    painter.line_segment([wp + slant * 0.8, mid_p], Stroke::new(0.65 * half, wire_col));
                                    painter.line_segment([mid_p, next_wp + slant * 0.4], Stroke::new(0.65 * half, wire_col));
                                }
                            }
                        }
                    }

                    // Воронки от разрывов снарядов и выжженная земля передовой
                    if fort_lvl > 0.01 || dug_lvl > 0.35 {
                        let crater_line = side(sgn * 1.8 * half);
                        if crater_line.len() >= 3 {
                            for i in (1..crater_line.len()).step_by(3) {
                                let cp = crater_line[i] + vec2(((i * 7) % 5) as f32 - 2.0, ((i * 11) % 5) as f32 - 2.0) * half;
                                let cr_r = (2.2 + ((i * 13) % 4) as f32 * 0.5) * half;
                                // Выворот грунта вокруг воронки
                                painter.circle_filled(cp, cr_r + 1.2 * half, Color32::from_rgba_unmultiplied(52, 44, 34, (190.0 * trench_a) as u8));
                                // Тёмное обугленное дно воронки
                                painter.circle_filled(cp + vec2(0.3, 0.4), cr_r * 0.72, Color32::from_rgba_unmultiplied(22, 18, 15, (225.0 * trench_a) as u8));
                                // Струйки дыма над свежими воронками
                                if (i + (t * 2.0) as usize) % 4 == 0 {
                                    let ph = ((t * 0.4 + i as f32 * 0.3).fract()).powf(0.8);
                                    let sm_pos = cp + vec2((t * 0.8 + i as f32).sin() * 3.5 + ph * 6.0, -ph * 16.0 * half);
                                    let sm_a = ((1.0 - ph) * 90.0 * trench_a) as u8;
                                    painter.circle_filled(sm_pos, (2.0 + ph * 4.5) * half, Color32::from_rgba_unmultiplied(75, 70, 64, sm_a));
                                }
                            }
                        }
                    }
                };

                let sgn_ours = if *ours_left { 1.0 } else { -1.0 };
                draw_fort_line(&line, sgn_ours, *fort_ours, *dug_ours, true);
                draw_fort_line(&line, -sgn_ours, *fort_theirs, *dug_theirs, false);
            }
        }
    }

    /// A line on the land as running dashes (laid on the land's height).
    pub(super) fn dashes(&self, painter: &egui::Painter, pts: &[[f32; 2]], col: Color32, t: f32) {
        let mut along = 0.0f32;
        let rect = self.last_rect.expand(20.0);
        for k in 0..pts.len().saturating_sub(1) {
            let (a, b) = (pts[k], pts[k + 1]);
            let len = (b[0] - a[0]).hypot(b[1] - a[1]);
            let pa = self.cam.to_screen(a[0], a[1], self.z(a[0], a[1]) + 0.05);
            let pb = self.cam.to_screen(b[0], b[1], self.z(b[0], b[1]) + 0.05);
            if rect.contains(pa) || rect.contains(pb) {
                // (a dash every world pixel or so, running along)
                let on = ((along * 1.2 - t * 1.5).rem_euclid(1.0)) < 0.55;
                if on {
                    painter.line_segment([pa, pb], Stroke::new(4.0, Color32::from_rgba_unmultiplied(20, 16, 10, 150)));
                    painter.line_segment([pa, pb], Stroke::new(2.2, col));
                }
            }
            along += len;
        }
    }

    /// The lines of supply laid since the map was made (any side's: they lie on the land), put
    /// onto the land's ways on the video card: what is new since the last frame.
    pub(super) fn lay_lines(&mut self, g: &Game) {
        let works = &g.state.works;
        if self.laid.len() < works.len() {
            self.laid.resize(works.len(), 0);
        }
        let mut rails: Vec<Vec<[f32; 2]>> = Vec::new();
        let mut roads: Vec<Vec<[f32; 2]>> = Vec::new();
        for (k, wk) in works.iter().enumerate() {
            if wk.done > self.laid[k] && wk.done >= 2 {
                let from = self.laid[k].saturating_sub(1);
                let provs: Vec<usize> = wk.path[from..wk.done].iter().map(|&q| g.h.war.th.provs[q]).collect();
                let pts = crate::world::land::line_through(&g.world, &provs, &g.land.river_px);
                if wk.rail {
                    rails.push(pts);
                } else {
                    roads.push(pts);
                }
                self.laid[k] = wk.done;
            }
        }
        let (w, h) = (g.world.w, g.world.h);
        if !rails.is_empty() {
            let ls: Vec<(&[[f32; 2]], u8)> = rails.iter().map(|p| (&p[..], 0)).collect();
            ways::near(&mut self.ways_now, w, h, 2, 2, None, &ls, ground::WAY_RANGE);
            self.scene.lock().unwrap().set_ways(self.ways_now.clone());
        }
        if !roads.is_empty() {
            let ls: Vec<(&[[f32; 2]], u8)> = roads.iter().map(|p| (&p[..], 0)).collect();
            ways::near(&mut self.lines_now, w, h, 2, 2, None, &ls, ground::WAY_RANGE);
            self.scene.lock().unwrap().set_lines(self.lines_now.clone());
        }
    }
}
