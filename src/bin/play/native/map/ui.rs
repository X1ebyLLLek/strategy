//! The map drawn each frame and what the hand does on it: the land, the lines, the labels, the
//! counters and the divisions, the tools (front lines, arrows, depots), the clicks.

use super::*;

impl MapView {
    /// The map for a frame: the land, what is on it, and what the hand does.
    pub fn ui(&mut self, ui: &mut egui::Ui, g: &mut Game, chosen: Option<usize>) -> Asks {
        let mut asks = Asks::default();
        self.dt = ui.input(|i| i.stable_dt).min(0.1);
        let rect = ui.max_rect();
        let resp = ui.allocate_rect(rect, Sense::click_and_drag());
        let (w, h) = (g.world.w as f32, g.world.h as f32);
        self.view_fit(g, rect);
        self.province_table(ui.ctx(), g);
        // the hand: drag (the ground under the hand follows it), the wheel (the point under the
        // hand stays under it)
        let flat = |_: f32, _: f32| 0.0;
        let on_small = resp.interact_pointer_pos().is_some_and(|p| p.x > rect.max.x - 315.0 && p.y > rect.max.y - 215.0 && p.y < rect.max.y - 59.0);
        // (in a war of divisions the left button chooses them — a click or a box — and the map is
        // moved by the right or the middle button, as in HOI4; else by the left)
        // (the army stands in divisions in peace too: they are chosen and sent over one's own land)
        let corps = g.h.war.corps.is_some() && !self.picking;
        self.picked.retain(|id| g.h.war.corps.as_ref().is_some_and(|c| c.find(*id).is_some()));
        let pan = if corps { resp.dragged_by(egui::PointerButton::Secondary) || resp.dragged_by(egui::PointerButton::Middle) } else { resp.dragged_by(egui::PointerButton::Primary) || resp.dragged_by(egui::PointerButton::Middle) };
        self.chosen = chosen;
        if chosen.is_none() || g.h.war.command[g.me].is_none() {
            self.tool = Tool::None;
        }
        let drawing = self.tool != Tool::None;
        if corps && resp.drag_started_by(egui::PointerButton::Primary) && !on_small && !drawing {
            self.boxing = ui.input(|i| i.pointer.press_origin());
        }
        if pan && !on_small {
            if let Some(p) = resp.interact_pointer_pos() {
                let cam = self.camera(rect);
                let d = resp.drag_delta();
                let (a, b) = (cam.to_world(p - d, 0.0, &flat), cam.to_world(p, 0.0, &flat));
                self.cx += a.0 - b.0;
                self.cy += a.1 - b.1;
                self.moved = true;
            }
        }
        if let Some(hp) = resp.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let before = self.camera(rect).to_world(hp, 0.0, &flat);
                let fit = (rect.width() / w).max(rect.height() / h);
                // (closer than before: the game's provinces are smaller — 28th session; as HOI4,
                // a handful of provinces across the screen at the closest)
                self.k = (self.k * (scroll * 0.0025).exp()).clamp(fit, 22.0);
                for _ in 0..3 {
                    let now = self.camera(rect).to_world(hp, 0.0, &flat);
                    self.cx += before.0 - now.0;
                    self.cy += before.1 - now.1;
                }
                self.moved = true;
            }
        }
        self.keep_on_world(rect, h);
        self.cx = self.cx.rem_euclid(w);
        self.cam = self.camera(rect);
        let painter = ui.painter_at(rect);
        self.last_rect = rect;
        self.counters.lock().unwrap().clear();
        // the land, by the video card; it moves (the sea): look again soon
        let mut frame = self.frame_of(&self.cam);
        frame.war = g.h.at_war;
        frame.season = season_of(g.month(), g.day);
        painter.add(egui::Shape::Callback(scene::paint(self.scene.clone(), rect, frame)));
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        let dress = super::super::chrome::of(ui.ctx());
        if let Some(ch) = &dress {
            let mut v = super::super::chrome::Out::new();
            ch.vignette(&mut v, rect, 150.0);
            painter.extend(v);
        }
        // the borders as smooth lines, close up
        self.smooth_borders(&painter, g);
        self.names(&painter, rect, g);
        self.hover_outline(&painter, &resp, g);
        self.marks(&painter, &resp, g);
        self.smoke(&painter, rect);
        // the month's battles (gold where it went our way) under the troops; the troops, the
        // arrows of the orders and the blows over them (in a war of divisions: the fights going
        // on, the attacks, the stacks — `divs`)
        let mut hits: Vec<(Rect, usize)> = Vec::new();
        self.stacks_drawn.lock().unwrap().clear();
        self.foe_drawn.lock().unwrap().clear();
        self.fights_drawn.lock().unwrap().clear();
        let battle_hits = if g.h.at_war && !corps { self.battles(&painter, g) } else { Vec::new() };
        if g.h.at_war {
            self.arrows(&painter, rect, g, chosen);
        }
        if corps && self.k >= AGG {
            self.divisions(&painter, rect, g);
            self.preview_draw(&painter, rect, g);
            self.badges_draw(&painter, rect, g);
        } else {
            self.badges.lock().unwrap().clear();
            self.figures.lock().unwrap().clear();
            self.troops(&painter, rect, g, chosen, &mut hits);
        }
        self.plan_draw(&painter, rect, g);
        self.seas_draw(&painter, rect, g);
        self.box_choose(&painter, ui, &resp, corps, &mut asks);
        // the small map in the corner: the view's ground on it; a click there brings the view
        // (above the modes' plate when it stands over the map's foot — 29th session)
        let foot = ui.ctx().memory(|m| m.area_rect(egui::Id::new("modes"))).map_or(rect.bottom(), |r| r.min.y.min(rect.bottom()));
        // (30th session: a window of 1280 points lost a quarter of its map to a fixed 300-point plate — it grows with
        // the window, from 210 to 300)
        let (mw, mh) = ((rect.width() * 0.24).clamp(210.0, 300.0), 0.0);
        let mh = mh + mw / 2.0;
        let mm = Rect::from_min_size(pos2(rect.right() - mw - 12.0, foot.min(rect.bottom() - 6.0) - 14.0 - mh), vec2(mw, mh));
        self.last_mini = mm.expand(7.0);
        if let Some(t) = &self.mini {
            // (in a bezel of the bar's material, with a line of trim)
            match &dress {
                Some(ch) => {
                    let mut v = super::super::chrome::Out::new();
                    let bez = mm.expand(7.0);
                    ch.lay(&mut v, bez, &ch.bar, 3.0);
                    ch.bevel(&mut v, bez, true, 2.0);
                    v.push(egui::Shape::rect_stroke(bez, 3.0, Stroke::new(1.5, ch.trim)));
                    v.push(egui::Shape::rect_filled(mm.expand(1.5), 1.0, Color32::BLACK));
                    painter.extend(v);
                }
                None => {
                    painter.rect_filled(mm.expand(3.0), 2.0, Color32::from_rgb(30, 28, 24));
                }
            }
            painter.image(t.id(), mm, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            let on_mm = painter.with_clip_rect(mm);
            let corners = self.cam.corners();
            for copy in [-1.0f32, 0.0, 1.0] {
                let pts: Vec<Pos2> = corners.iter().chain(corners.first()).map(|&(x, y)| pos2(mm.min.x + (x / w + copy) * mm.width(), mm.min.y + y / h * mm.height())).collect();
                on_mm.add(egui::Shape::line(pts, Stroke::new(1.5, Color32::from_rgb(255, 214, 64))));
            }
        }
        let on_mini = resp.interact_pointer_pos().is_some_and(|p| mm.contains(p));
        if on_mini && (resp.clicked() || resp.dragged()) {
            if let Some(pp) = resp.interact_pointer_pos() {
                self.cx = (pp.x - mm.min.x) / mm.width() * w;
                self.cy = (pp.y - mm.min.y) / mm.height() * h;
                self.moved = true;
            }
        }
        // what the hand does
        let pointer = resp.hover_pos().filter(|p| !mm.contains(*p));
        let dragging = pan && !on_small;
        let mut used = false;
        if !on_mini && !dragging {
            used = self.use_tool(&resp, g);
        }
        if resp.clicked() && !on_mini && !used {
            if let Some(pp) = resp.interact_pointer_pos() {
                self.left_click(ui, g, pp, chosen, corps, &hits, &battle_hits, &mut asks);
            }
        }
        self.right_click(ui, &resp, g, chosen, corps);
        let shift = ui.input(|i| i.modifiers.shift);
        self.look_ahead(g, if corps && !self.picked.is_empty() && !dragging { pointer } else { None }, shift);
        let ctx = ui.ctx().clone();
        self.hover(&ctx, g, pointer, dragging, corps, chosen, &hits, &battle_hits);
        asks
    }

    /// the view: the war on the screen, in the space the panels leave (fitted again when a
    /// folder opens or closes, till the player moves the map himself)
    fn view_fit(&mut self, g: &Game, rect: Rect) {
        if (!self.fitted || (!self.moved && rect != self.fit_rect && !crate::native::spoiled())) && rect.width() > 10.0 {
            self.fit(g, rect);
            self.fitted = true;
            self.fit_rect = rect;
            if self.zoom_once != 1.0 {
                self.k *= self.zoom_once;
                self.moved = true;
            }
            if let Some((x, y)) = self.center_once.take() {
                (self.cx, self.cy) = (x, y);
                self.moved = true;
            }
        }
        // (the spoiled control: not fitted again and not moved, as a naive map would — the land
        // stays where it was on the screen, and a folder opening simply lies over the war)
        if crate::native::spoiled() && self.fitted && rect != self.fit_rect {
            let dc = rect.center() - self.fit_rect.center();
            self.cx += dc.x / self.k;
            self.cy += dc.y / self.k;
            self.fit_rect = rect;
        }
    }

    /// the table of provinces anew when the land changed hands or the mode changed (the land
    /// itself stays on the card; the spoiled control sends it all again)
    fn province_table(&mut self, ctx: &egui::Context, g: &Game) {
        let key = (
            g.h.war.owner.clone(),
            self.mode as u8,
            match self.mode {
                Mode::Supply | Mode::Seas => g.month(),
                Mode::Faith => g.polity.region_faith.iter().fold(29u64, |h, &k| strategy::rng::hash64(h ^ k as u64)) as u32,
                Mode::Regime => g.polity.of.iter().fold(31u64, |h, x| strategy::rng::hash64(h ^ x.regime.index() as u64)) as u32,
                Mode::Relations => g.diplo.opinion.iter().fold(37u64, |h, &x| strategy::rng::hash64(h ^ (x * 20.0).round() as i64 as u64)) as u32 ^ g.diplo.pacts.len() as u32 ^ (g.diplo.wars.len() as u32) << 16,
                Mode::Armies => g.h.war.group_of(g.me).iter().fold(17u64, |h, &k| strategy::rng::hash64(h ^ k as u64)) as u32,
                _ => 0,
            },
            g.h.at_war,
        );
        if key != self.made_for {
            let t0 = std::time::Instant::now();
            let old = &self.made_for;
            let changed = if old.0.len() == key.0.len() { (0..key.0.len()).filter(|&q| key.0[q] != old.0[q]).count() } else { key.0.len() };
            let (pal, rows) = self.palette(g);
            self.small_map(ctx, &pal);
            let l = &self.ground;
            let whole = (l.height.len() * 4 + l.normal.len() + l.base.len() + l.ids.len()) as f32;
            let mut scene = self.scene.lock().unwrap();
            let sent = scene.set_palette(pal, rows) as f32;
            let (how, share) = if crate::native::spoiled() {
                scene.resend(self.ground.clone());
                ("целиком", (whole + sent) / whole)
            } else {
                ("таблица провинций", sent / whole)
            };
            drop(scene);
            self.made_for = key;
            self.painted.push((how, changed, t0.elapsed().as_secs_f32(), share));
        }
    }

    /// the names: the countries' first, then the towns' (the capitals from afar, the larger
    /// towns nearer, all close up); a name that would sit on one already written is left out
    /// (a town keeps its dot).
    /// (close up the countries' names fade, as on a staff map: the land and the troops matter)
    fn names(&self, painter: &egui::Painter, rect: Rect, g: &Game) {
        let fade = (1.6 - self.k * 0.4).clamp(0.15, 1.0);
        let mut taken: Vec<Rect> = Vec::new();
        let mut countries: Vec<&Label> = self.labels.iter().collect();
        countries.sort_by(|a, b| (b.size * b.half).total_cmp(&(a.size * a.half)));
        let mut towns: Vec<&(usize, String, u8)> = g.towns.iter().filter(|t| self.k >= match t.2 { 3 => 0.9, 2 => 1.6, _ => 2.6 }).collect();
        towns.sort_by(|a, b| b.2.cmp(&a.2));
        // (close up the countries' names are faint: the towns' may lie over them)
        let solid = fade >= 0.5;
        for pass in 0..2 {
            if pass == 0 {
                for l in &countries {
                    let chars: Vec<char> = l.name.chars().collect();
                    let n = chars.len().max(1) as f32;
                    let (cx, cy) = l.at(0.0);
                    let scale = self.cam.scale_at(cx, cy, 0.0);
                    let px = l.size * scale;
                    if px < 9.0 || !rect.expand(300.0).contains(self.cam.to_screen(cx, cy, 0.0)) {
                        continue;
                    }
                    // each letter on the name's line, turned along it, as large as the land there
                    // seen by the camera
                    let mut placed: Vec<(Pos2, f32, f32, char)> = Vec::new();
                    let mut boxes: Vec<Rect> = Vec::new();
                    for (i, ch) in chars.iter().enumerate() {
                        let u = -l.half + 2.0 * l.half * (i as f32 + 0.5) / n;
                        let (x, y) = l.at(u);
                        let (a, b) = (self.cam.to_screen(x, y, 0.0), {
                            let (x2, y2) = l.at(u + 0.5);
                            self.cam.to_screen(x2, y2, 0.0)
                        });
                        let size = (l.size * self.cam.scale_at(x, y, 0.0)).clamp(9.0, 54.0);
                        let ang = (b.y - a.y).atan2(b.x - a.x);
                        boxes.push(Rect::from_center_size(a, vec2(size * 0.75, size * 0.9)));
                        placed.push((a, ang, size, *ch));
                    }
                    if boxes.iter().any(|b| taken.iter().any(|t| t.intersects(b.shrink(2.0)))) {
                        continue;
                    }
                    // (29th session: a name cut by the screen's edge reads as a fault — it waits
                    // till the map is moved)
                    if boxes.iter().filter(|b| !rect.contains_rect(**b)).count() * 10 > boxes.len() * 3 {
                        continue;
                    }
                    if solid {
                        taken.extend(boxes);
                    }
                    for (a, ang, size, ch) in placed {
                        let font = FontId::new(size, FontFamily::Name("head".into()));
                        let galley = painter.layout_no_wrap(ch.to_string(), font, Color32::WHITE);
                        // (turned about the letter's middle)
                        let half = galley.size() / 2.0;
                        let (sn, cs) = ang.sin_cos();
                        let corner = a - vec2(half.x * cs - half.y * sn, half.x * sn + half.y * cs);
                        let ink = Color32::from_rgba_unmultiplied(245, 238, 222, (200.0 * fade) as u8);
                        let shade = Color32::from_black_alpha((110.0 * fade) as u8);
                        painter.add(egui::epaint::TextShape::new(corner + vec2(1.5, 1.5), galley.clone(), shade).with_override_text_color(shade).with_angle(ang));
                        painter.add(egui::epaint::TextShape::new(corner, galley, ink).with_override_text_color(ink).with_angle(ang));
                    }
                }
            } else {
                for (p, name, size) in &towns {
                    let pt = self.pt(Self::at(g, *p));
                    if !rect.contains(pt) {
                        continue;
                    }
                    let r = 2.0 + *size as f32;
                    painter.circle_filled(pt, r, Color32::from_rgb(250, 244, 228));
                    painter.circle_stroke(pt, r, Stroke::new(1.2, Color32::from_rgb(30, 26, 20)));
                    let font = FontId::new(11.0 + 2.0 * *size as f32, FontFamily::Name("bold".into()));
                    let galley = painter.layout_no_wrap(name.clone(), font.clone(), Color32::WHITE);
                    let at = pt + vec2(r + 4.0, 0.0);
                    let box_ = Rect::from_min_size(at - vec2(0.0, galley.size().y / 2.0), galley.size());
                    if taken.iter().any(|t| t.intersects(box_)) {
                        continue;
                    }
                    taken.push(box_.union(Rect::from_center_size(pt, vec2(2.0 * r, 2.0 * r))));
                    painter.text(at + vec2(1.0, 1.0), Align2::LEFT_CENTER, name, font.clone(), Color32::from_black_alpha(190));
                    painter.text(at, Align2::LEFT_CENTER, name, font, Color32::from_rgb(252, 246, 230));
                }
            }
        }
        // (the great rivers' names last: they give way to the countries' and the towns')
        self.river_names(painter, rect, &mut taken);
    }

    /// The capitals of the war, the player's depots of supply, the lines of supply (laid ones on
    /// the land's ways, unfinished ones as dashes, the one planned under the hand).
    fn marks(&mut self, painter: &egui::Painter, resp: &egui::Response, g: &Game) {
        let war = &g.h.war;
        let th = &war.th;
        let me = g.me;
        // the capitals of the war
        for s in 0..2 {
            let p = self.pt(Self::at(g, th.provs[th.capital[s]]));
            painter.circle_filled(p, 8.0, Color32::from_rgba_unmultiplied(20, 18, 14, 230));
            painter.circle_stroke(p, 8.0, Stroke::new(1.6, Color32::from_rgb(232, 199, 106)));
            painter.circle_filled(p, 3.5, Color32::from_rgb(232, 199, 106));
        }
        // the player's depots of supply (34th session: a crate in a dark disc with a green rim, a name beside it near; and the
        // depots being built, with the months left — they were not drawn at all)
        let now = g.month();
        let near = self.k >= 1.6;
        let mut depot = |q: usize, building: Option<u32>| {
            let p = self.pt(Self::at(g, th.provs[q]));
            if !painter.clip_rect().expand(30.0).contains(p) {
                return;
            }
            let rim = if building.is_some() { Color32::from_rgb(232, 199, 106) } else { Color32::from_rgb(159, 211, 106) };
            painter.circle_filled(p + vec2(0.0, 1.5), 11.0, Color32::from_black_alpha(110));
            painter.circle_filled(p, 10.0, Color32::from_rgba_unmultiplied(20, 18, 14, 235));
            painter.circle_stroke(p, 10.0, Stroke::new(if building.is_some() { 1.4 } else { 1.8 }, rim));
            if building.is_some() {
                // (a ring of dashes while it is laid)
                for k in 0..12 {
                    let a = k as f32 / 12.0 * std::f32::consts::TAU + self.t();
                    painter.circle_filled(p + vec2(a.cos(), a.sin()) * 13.5, 1.2, rim);
                }
            }
            // (the glyph in shapes: a crate with its slats, or an hourglass)
            if building.is_some() {
                painter.add(egui::Shape::convex_polygon(vec![p + vec2(-4.5, -5.5), p + vec2(4.5, -5.5), p + vec2(0.0, 0.0)], rim, Stroke::NONE));
                painter.add(egui::Shape::convex_polygon(vec![p + vec2(-4.5, 5.5), p + vec2(4.5, 5.5), p + vec2(0.0, 0.0)], rim, Stroke::NONE));
            } else {
                let r = Rect::from_center_size(p, vec2(10.0, 8.0));
                painter.rect_filled(r, 1.0, rim.gamma_multiply(0.35));
                painter.rect_stroke(r, 1.0, Stroke::new(1.3, rim));
                painter.line_segment([r.left_top(), r.right_bottom()], Stroke::new(1.0, rim));
                painter.line_segment([r.right_top(), r.left_bottom()], Stroke::new(1.0, rim));
            }
            if near {
                let text = match building {
                    Some(r) => format!("склад · {} мес.", r.saturating_sub(now).max(1)),
                    None => "склад".to_string(),
                };
                let font = FontId::new(12.5, FontFamily::Name("bold".into()));
                let w = painter.layout_no_wrap(text.clone(), font.clone(), rim).size().x;
                let pill = Rect::from_min_size(p + vec2(12.0, -8.5), vec2(w + 10.0, 17.0));
                painter.rect_filled(pill, 8.0, Color32::from_rgba_unmultiplied(20, 18, 14, 215));
                painter.rect_stroke(pill, 8.0, Stroke::new(1.0, rim.gamma_multiply(0.7)));
                painter.text(pill.center(), Align2::CENTER_CENTER, text, font, rim);
            }
        };
        for &q in &war.hubs[me] {
            if war.owner[q] == me {
                depot(q, None);
            }
        }
        for &(side, q, ready) in &g.state.building {
            if side == me {
                depot(q, Some(ready));
            }
        }
        // the lines of supply: laid provinces go onto the land's ways (the card draws them); the
        // player's unfinished ones are dashes; the way planned under the hand while laying one
        self.lay_lines(g);
        let t_dash = self.t();
        for (k, wk) in g.state.works.iter().enumerate() {
            if wk.side != me || wk.finished() {
                continue;
            }
            let pts = self.planned.entry(k).or_insert_with(|| {
                let provs: Vec<usize> = wk.path.iter().map(|&q| th.provs[q]).collect();
                crate::world::land::line_through(&g.world, &provs, &g.land.river_px)
            });
            let pts = pts.clone();
            let col = if wk.short { Color32::from_rgb(200, 80, 60) } else { Color32::from_rgb(232, 199, 106) };
            self.dashes(&painter, &pts, col, t_dash);
        }
        if let (Build::Line { from: Some(a), .. }, Some(hp)) = (self.build, resp.hover_pos()) {
            if let Some(p) = self.province_at(g, hp) {
                let q = th.at.get(p).copied().unwrap_or(NO);
                if q != NO {
                    if let Some(path) = crate::world::land::plan(war, me, a, q as usize) {
                        let pts: Vec<[f32; 2]> = path.iter().map(|&q| {
                            let (x, y) = Self::at(g, th.provs[q]);
                            [x, y]
                        }).collect();
                        let ok = path.len() <= crate::world::statecraft::LINE_MOST;
                        self.dashes(&painter, &pts, if ok { Color32::from_rgb(240, 220, 150) } else { Color32::from_rgb(210, 70, 60) }, t_dash);
                    }
                }
            }
            let p0 = self.pt(Self::at(g, th.provs[a]));
            painter.circle_filled(p0, 6.0, Color32::from_rgb(232, 199, 106));
            painter.circle_stroke(p0, 6.0, Stroke::new(1.5, Color32::from_rgb(40, 30, 20)));
        }
    }

    /// The province under the hand outlined (30th session): the card in the corner says which land it
    /// is, but nothing on the map showed where it ends — a warm line round it, a dark shade outside.
    fn hover_outline(&self, painter: &egui::Painter, resp: &egui::Response, g: &Game) {
        let Some(hp) = resp.hover_pos() else { return };
        if resp.dragged() || self.picking {
            return;
        }
        let Some(p) = self.province_at(g, hp).filter(|&p| !g.world.provinces[p].sea) else { return };
        let p = p as u32;
        let w = g.world.w as f32;
        let seen = self.cam.seen(RELIEF);
        let ramp = |a: f32, b: f32| {
            let t = ((self.k - a) / (b - a)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        // (bolder as the province grows on the screen, never thicker than a line of the eye)
        let width = 1.6 + 1.4 * ramp(1.0, 4.0);
        let breath = 0.82 + 0.18 * (self.t() * 3.0).sin();
        for (pts, b, l, r) in &self.borders {
            if *l != p && *r != p {
                continue;
            }
            let mid = (b[0] + b[2]) / 2.0;
            let dx = ((self.cx - mid) / w).round() * w;
            if b[2] + dx < seen[0] || b[0] + dx > seen[2] || b[3] < seen[1] || b[1] > seen[3] {
                continue;
            }
            let line: Vec<Pos2> = pts.iter().map(|q| self.cam.project_abs(q[0] + dx, q[1], self.z(q[0], q[1])).0).collect();
            painter.add(egui::Shape::line(line.clone(), Stroke::new(width + 3.4, Color32::from_black_alpha(120))));
            painter.add(egui::Shape::line(line, Stroke::new(width, Color32::from_rgba_unmultiplied(255, 236, 170, (235.0 * breath) as u8))));
        }
    }

    /// the factories' smoke, close up: puffs rising from the chimneys, growing and fading
    fn smoke(&self, painter: &egui::Painter, rect: Rect) {
        let houses = smoothstep(2.6, 4.0, self.k);
        if houses > 0.05 && !self.chimneys.is_empty() {
            let t = self.t();
            for (i, &(x, y, z)) in self.chimneys.iter().enumerate() {
                let base = self.cam.to_screen(x, y, self.z(x, y) + z * houses);
                if !rect.expand(40.0).contains(base) {
                    continue;
                }
                let scale = self.cam.scale_at(x, y, 0.0);
                let seed = (i as f32 * 0.618).fract();
                for j in 0..5 {
                    let ph = (t * 0.22 + j as f32 / 5.0 + seed).fract();
                    let drift = vec2((ph * 1.6 + seed * 3.0).sin() * 0.25 + ph * 0.9, -ph * 2.2) * scale;
                    let r = (0.12 + ph * 0.45) * scale;
                    let a = ((1.0 - ph) * (1.0 - ph) * 120.0 * houses) as u8;
                    painter.circle_filled(base + drift, r, Color32::from_rgba_unmultiplied(92, 88, 84, a));
                }
            }
        }
    }

    /// The month's battles (gold where it went our way), each where the hand finds it.
    fn battles(&self, painter: &egui::Painter, g: &Game) -> Vec<(Pos2, usize)> {
        let war = &g.h.war;
        let th = &war.th;
        let me = g.me;
        let mut battle_hits: Vec<(Pos2, usize)> = Vec::new();
        let t = self.t();
        for (i, f) in war.fought.iter().enumerate().filter(|(_, f)| f.month == war.month) {
            let p = self.pt(Self::at(g, th.provs[f.province])) + vec2(10.0, -10.0);
            let good = (f.attacker == me) == f.won;
            let col = if good { Color32::from_rgb(255, 214, 64) } else { Color32::from_rgb(255, 90, 74) };
            // (the fight: a flash that comes and goes, smoke drifting up and away)
            let seed = (f.province as f32 * 0.618).fract();
            let flash = (0.5 + 0.5 * ((t * 3.1 + seed * 6.3).sin())).powi(3);
            painter.circle_filled(p, 12.0 + 5.0 * flash, Color32::from_rgba_unmultiplied(255, 150, 40, (70.0 * flash) as u8));
            for j in 0..4 {
                let ph = (t * 0.33 + j as f32 / 4.0 + seed).fract();
                let drift = vec2(((j as f32 * 2.1 + seed * 5.0).sin()) * 7.0 + ph * 10.0, -ph * 26.0);
                painter.circle_filled(p + drift, 4.0 + ph * 9.0, Color32::from_rgba_unmultiplied(70, 66, 60, ((1.0 - ph) * (1.0 - ph) * 110.0) as u8));
            }
            // (the battle as in HOI4: crossed swords on a plate, under it a bar of how it goes —
            // our side green, the enemy's red, moving through the month towards how it ends)
            let plate = Rect::from_center_size(p, vec2(24.0, 19.0));
            painter.rect_filled(plate.expand(1.5), 3.0, Color32::from_black_alpha(200));
            painter.rect_filled(plate, 3.0, Color32::from_rgb(40, 34, 28));
            painter.rect_stroke(plate, 3.0, Stroke::new(1.5, col));
            let icons = super::super::chrome::icons(painter.ctx());
            super::super::flags::image(painter.ctx(), &painter, &icons, "clash", plate.shrink2(vec2(4.0, 2.0)), col);
            let ours = if f.attacker == me { f.ratio / (1.0 + f.ratio) } else { 1.0 / (1.0 + f.ratio) };
            let ours = if (f.attacker == me) == f.won { ours.max(0.55) } else { ours.min(0.45) };
            let k = ((g.day as f32 + 1.0) / 30.0).clamp(0.0, 1.0);
            let shown = 0.5 + (ours - 0.5) * k;
            let bar = Rect::from_min_size(pos2(plate.min.x - 5.0, plate.max.y + 3.0), vec2(plate.width() + 10.0, 5.0));
            painter.rect_filled(bar.expand(1.0), 1.5, Color32::from_black_alpha(210));
            let split = bar.min.x + bar.width() * shown;
            painter.rect_filled(Rect::from_min_max(bar.min, pos2(split, bar.max.y)), 1.0, Color32::from_rgb(110, 190, 80));
            painter.rect_filled(Rect::from_min_max(pos2(split, bar.min.y), bar.max), 1.0, Color32::from_rgb(210, 60, 46));
            painter.line_segment([pos2(bar.center().x, bar.min.y - 1.0), pos2(bar.center().x, bar.max.y + 1.0)], Stroke::new(1.0, Color32::from_white_alpha(160)));
            battle_hits.push((p, i));
        }
        battle_hits
    }

    /// the box the player draws to choose divisions
    fn box_choose(&mut self, painter: &egui::Painter, ui: &egui::Ui, resp: &egui::Response, corps: bool, asks: &mut Asks) {
        if let (Some(a), true) = (self.boxing, corps) {
            if let Some(b) = resp.interact_pointer_pos().or(resp.hover_pos()) {
                let r = Rect::from_two_pos(a, b);
                painter.rect_filled(r, 1.0, Color32::from_rgba_unmultiplied(255, 216, 96, 28));
                painter.rect_stroke(r, 1.0, Stroke::new(1.5, Color32::from_rgb(255, 216, 96)));
            }
        }
        if self.boxing.is_some() && !resp.dragged_by(egui::PointerButton::Primary) {
            if let (Some(a), Some(b)) = (self.boxing.take(), ui.input(|i| i.pointer.interact_pos())) {
                let r = Rect::from_two_pos(a, b);
                if r.width() > 4.0 && r.height() > 4.0 {
                    let shift = ui.input(|i| i.modifiers.shift);
                    let got = self.in_box(r);
                    if !shift {
                        self.picked.clear();
                    }
                    for id in got {
                        if !self.picked.contains(&id) {
                            self.picked.push(id);
                        }
                    }
                    if !self.picked.is_empty() {
                        asks.group = Some(None);
                    }
                }
            }
        }
    }

    /// A click of the left button at `pp`: a group's badge, a battle, a stack of ours, a group's
    /// counter, a province (a country picked, a muster, a depot, a line laid, nothing chosen).
    #[allow(clippy::too_many_arguments)]
    fn left_click(&mut self, ui: &egui::Ui, g: &mut Game, pp: Pos2, chosen: Option<usize>, corps: bool, hits: &[(Rect, usize)], battle_hits: &[(Pos2, usize)], asks: &mut Asks) {
        let me = g.me;
        let stack_at = |pp: Pos2| self.stacks_drawn.lock().unwrap().iter().find(|(r, _, _)| r.contains(pp)).map(|x| x.1.clone());
        let shift = ui.input(|i| i.modifiers.shift);
        let badge = self.badges.lock().unwrap().iter().find(|(r, _)| r.contains(pp)).map(|x| x.1);
        if let Some(k) = badge {
            self.picked.clear();
            if shift {
                asks.toggle = Some(k);
            } else {
                asks.group = Some(Some(k));
            }
        } else if let Some(&(_, i)) = battle_hits.iter().find(|(p, _)| p.distance(pp) < 10.0) {
            asks.battle = Some(i);
        } else if let (true, Some(ids)) = (corps && self.build == Build::None, stack_at(pp)) {
            // a stack of ours: its divisions chosen (with Shift: added, or taken out if all
            // were chosen)
            if shift {
                if ids.iter().all(|id| self.picked.contains(id)) {
                    self.picked.retain(|id| !ids.contains(id));
                } else {
                    for id in ids {
                        if !self.picked.contains(&id) {
                            self.picked.push(id);
                        }
                    }
                }
            } else {
                self.picked = ids;
            }
            asks.group = Some(None);
        } else if let Some(&(_, k)) = hits.iter().find(|(r, _)| r.contains(pp)) {
            self.picked.clear();
            asks.group = Some(if chosen == Some(k) { None } else { Some(k) });
            asks.open_army = true;
        } else if let Some(p) = self.province_at(g, pp) {
            let q = g.h.war.th.at.get(p).copied().unwrap_or(NO);
            if self.picking {
                let c = g.country_of[p];
                if c != u32::MAX && !g.world.provinces[p].sea {
                    asks.country = Some(c as usize);
                }
            } else if self.build == Build::Muster {
                if q != NO && g.h.war.owner[q as usize] == me {
                    g.act(crate::tools::saves::Act::Form(vec![("muster".into(), p.to_string())]));
                    self.build = Build::None;
                }
            } else if self.build == Build::Depot {
                if q != NO {
                    if g.act(crate::tools::saves::Act::Depot(q as usize)) {
                        self.build = Build::None;
                    }
                }
            } else if let Build::Line { rail, from } = self.build {
                if q != NO && g.h.war.owner[q as usize] == me {
                    match from {
                        None => self.build = Build::Line { rail, from: Some(q as usize) },
                        Some(a) => {
                            if let Some(path) = crate::world::land::plan(&g.h.war, me, a, q as usize) {
                                if g.act(crate::tools::saves::Act::Line(rail, path)) {
                                    self.build = Build::None;
                                }
                            }
                        }
                    }
                }
            } else if corps {
                // (a click on the land: nothing chosen any more)
                if !shift {
                    self.picked.clear();
                    asks.group = Some(None);
                }
            } else if q != NO && g.h.at_war {
                let lab = g.h.war.group_of(me);
                let k = lab[q as usize];
                if k != usize::MAX && g.h.war.owner[q as usize] == me {
                    asks.group = Some(Some(k));
                }
            }
        }
    }

    /// (the right button while building or drawing a plan: no more of it)
    fn right_click(&mut self, ui: &egui::Ui, resp: &egui::Response, g: &mut Game, chosen: Option<usize>, corps: bool) {
        let me = g.me;
        if resp.secondary_clicked() && self.build != Build::None {
            self.build = Build::None;
        } else if resp.secondary_clicked() && self.tool != Tool::None {
            self.tool = Tool::None;
        } else if resp.secondary_clicked() && corps && !self.picked.is_empty() {
            // the chosen divisions go there: over our land, or into the enemy's (an attack); with
            // Shift, on from where their ways end
            if let Some(p) = resp.interact_pointer_pos().and_then(|pp| self.province_at(g, pp)) {
                let q = g.h.war.th.at.get(p).copied().unwrap_or(NO);
                // (in peace, only over one's own land)
                if q != NO && (g.h.at_war || g.h.war.owner[q as usize] == me) {
                    if ui.input(|i| i.modifiers.shift) {
                        g.act(crate::tools::saves::Act::SendOn(self.picked.clone(), q as usize));
                    } else {
                        g.act(crate::tools::saves::Act::Send(self.picked.clone(), q as usize));
                    }
                    self.preview = None;
                }
            }
        } else if resp.secondary_clicked() {
            if let (Some(k), Some(pp)) = (chosen, resp.interact_pointer_pos()) {
                if let Some(p) = self.province_at(g, pp) {
                    let q = g.h.war.th.at.get(p).copied().unwrap_or(NO);
                    if q != NO && g.h.at_war {
                        let order = if g.h.war.owner[q as usize] != me { "attack" } else { "hold" };
                        g.act(crate::tools::saves::Act::Form(vec![("grp".into(), k.to_string()), ("order".into(), order.into()), ("target".into(), p.to_string())]));
                    }
                }
            }
        }
    }

    /// the cursor's look and the card of what the hand rests on: a battle, a group's counter,
    /// a province
    #[allow(clippy::too_many_arguments)]
    fn hover(&self, ctx: &egui::Context, g: &Game, pointer: Option<Pos2>, dragging: bool, corps: bool, chosen: Option<usize>, hits: &[(Rect, usize)], battle_hits: &[(Pos2, usize)]) {
        let me = g.me;
        if dragging {
            super::super::cursor::want(&ctx, super::super::cursor::Kind::Grab);
        } else if let Some(pp) = pointer {
            let war = &g.h.war;
            if let Some(&(bp, i)) = battle_hits.iter().find(|(p, _)| p.distance(pp) < 10.0) {
                super::super::cursor::want(&ctx, super::super::cursor::Kind::Hand);
                let f = &war.fought[i];
                let good = (f.attacker == me) == f.won;
                tip_at(&ctx, egui::Id::new(("battle", i)), Rect::from_center_size(bp, vec2(18.0, 18.0)), || {
                    Card::new("Бой")
                        .icon("clash")
                        .sub(crate::words::terrain_ru(f.terrain))
                        .row("наступали", g.h.home(f.attacker).name.clone())
                        .tone("итог", if f.won { "провинция взята" } else { "наступление отбито" }, if good { Tone::Good } else { Tone::Bad })
                        .row("силы", format!("×{:.2} в пользу наступавших", f.ratio))
                        .row("штаб обещал", format!("{:.0}%", f.odds * 100.0))
                        .note("щёлкните — разбор боя")
                });
            } else if let Some((r, k)) = self.badges.lock().unwrap().iter().find(|(r, _)| r.contains(pp)).copied() {
                super::super::cursor::want(&ctx, super::super::cursor::Kind::Hand);
                if let Some(gr) = war.command[me].as_ref().and_then(|gs| gs.get(k)) {
                    let gen = gr.general.as_ref().map_or(String::new(), |x| x.name.clone());
                    let facing = gr.facing.and_then(|c| g.countries.get(c as usize)).map(|c| c.0.clone());
                    tip_at(&ctx, egui::Id::new(("badge", k)), r, || {
                        let mut c = Card::new(crate::words::group_name(k)).face(g.face_of(&gen, "general")).sub(if gen.is_empty() { "без генерала".to_string() } else { format!("генерал {gen}") });
                        if let Some(n) = &facing {
                            c = c.row("держит границу", n.clone());
                        }
                        c.note("щёлкните — выбрать группу; с Shift — добавить к выбранным")
                    });
                }
            } else if let Some((r, ids, q)) = self.stacks_drawn.lock().unwrap().iter().find(|(r, _, _)| r.contains(pp)).cloned() {
                super::super::cursor::want(&ctx, super::super::cursor::Kind::Hand);
                tip_at(&ctx, egui::Id::new(("stack", q)), r, || self.stack_card(g, &ids, q, true));
            } else if let Some((r, q)) = self.foe_drawn.lock().unwrap().iter().find(|(r, _)| r.contains(pp)).copied() {
                super::super::cursor::want(&ctx, if self.picked.is_empty() { super::super::cursor::Kind::Arrow } else { super::super::cursor::Kind::Attack });
                let ids: Vec<u32> = war.corps.as_ref().map_or(Vec::new(), |c| c.divs.iter().filter(|d| d.side != me && d.at == q).map(|d| d.id).collect());
                tip_at(&ctx, egui::Id::new(("foe-stack", q)), r, || self.stack_card(g, &ids, q, false));
            } else if let Some((r, i)) = self.fights_drawn.lock().unwrap().iter().find(|(r, _)| r.contains(pp)).copied() {
                super::super::cursor::want(&ctx, super::super::cursor::Kind::Hand);
                tip_at(&ctx, egui::Id::new(("fight", i)), r, || self.fight_card(g, i));
            } else if let Some(&(r, k)) = hits.iter().find(|(r, _)| r.contains(pp)) {
                super::super::cursor::want(&ctx, super::super::cursor::Kind::Hand);
                if let Some(gr) = war.command[me].as_ref().and_then(|gs| gs.get(k)) {
                    let gen = gr.general.as_ref().map_or(String::new(), |x| x.name.clone());
                    let order = match gr.order {
                        strategy::military::front::Order::Hold => "держит участок",
                        strategy::military::front::Order::Deep => "держит в глубину",
                        strategy::military::front::Order::Attack(_) => "наступает",
                    };
                    tip_at(&ctx, egui::Id::new(("counter", k)), r, || {
                        Card::new(crate::words::group_name(k))
                            .face(g.face_of(&gen, "general"))
                            .sub(if gen.is_empty() { "без генерала".to_string() } else { format!("генерал {gen}") })
                            .row("приказ", order)
                            .row("ведёт", if gr.staff { "генштаб" } else { "вы" })
                            .note(if chosen == Some(k) { "выбрана: правой кнопкой по карте — приказ" } else { "щёлкните — выбрать группу" })
                    });
                }
            } else if let Some(p) = self.province_at(g, pp).filter(|&p| !g.world.provinces[p].sea) {
                let q = war.th.at.get(p).copied().unwrap_or(NO);
                if self.build != Build::None {
                    super::super::cursor::want(&ctx, super::super::cursor::Kind::Depot);
                } else if let Tool::Front { .. } = self.tool {
                    super::super::cursor::want(&ctx, if q != NO && war.owner[q as usize] == me { super::super::cursor::Kind::Hold } else { super::super::cursor::Kind::Arrow });
                } else if let Tool::Attack { .. } = self.tool {
                    super::super::cursor::want(&ctx, if q != NO && war.owner[q as usize] != me { super::super::cursor::Kind::Attack } else { super::super::cursor::Kind::Arrow });
                } else if (chosen.is_some() || (corps && !self.picked.is_empty())) && g.h.at_war && q != NO {
                    super::super::cursor::want(&ctx, if war.owner[q as usize] != me { super::super::cursor::Kind::Attack } else { super::super::cursor::Kind::Hold });
                } else if corps && !self.picked.is_empty() && q != NO && war.owner[q as usize] == me {
                    super::super::cursor::want(&ctx, super::super::cursor::Kind::Hold);
                }
                let at = self.world_at(pp);
                tip_at(&ctx, egui::Id::new("province"), Rect::from_center_size(pp, vec2(14.0, 14.0)), || self.province_card(g, p, chosen, at));
            }
        }
    }
}
