//! The player's plans on the map, as in HOI4. The front of the chosen groups (one or several:
//! cut among them along it): a click on a neighbouring country — the whole border with it (the
//! enemy of the war, or any other neighbour: its border is guarded), the border under the hand
//! lit before the click; or the left button drawn over provinces of ours — that stretch. The
//! arrow of an attack laid with a click on the enemy's (pale and still till it is carried out).
//! Where the chosen divisions would go if the right button were pressed where the hand is — the
//! way, and the days it takes the slowest of them (with Shift: on from where their ways end).

use eframe::egui::{self, vec2, Color32, FontFamily, FontId, Pos2, Rect, Stroke};
use strategy::military::front::NO;

use crate::Game;
use crate::tools::saves::Act;
use super::troops::arrow;
use super::{MapView, Preview, Tool, HUES};

/// The country of the world a province of the world belongs to now (in the theatre: its holder's;
/// u32::MAX: none — the sea).
pub fn country_now(g: &Game, p: usize) -> u32 {
    if g.world.provinces[p].sea {
        return u32::MAX;
    }
    let war = &g.h.war;
    match war.th.at.get(p).copied() {
        Some(q) if q != NO => war.th.countries[war.owner[q as usize]] as u32,
        _ => g.country_of[p],
    }
}

/// The provinces of the theatre that are ours and touch the land of country `c` (of the world).
pub fn border_with(g: &Game, c: u32) -> Vec<usize> {
    let war = &g.h.war;
    let me = g.me;
    (0..war.owner.len())
        .filter(|&q| war.owner[q] == me)
        .filter(|&q| g.world.provinces[war.th.provs[q]].neighbors.iter().any(|&r| country_now(g, r as usize) == c))
        .collect()
}

/// The foreign land (not the sea) province `q` of ours touches: the enemy of the war first (the
/// rival in peace), else the neighbour it touches most; None — inland.
fn touches(g: &Game, q: usize) -> Option<u32> {
    let war = &g.h.war;
    let mine = war.th.countries[g.me] as u32;
    let foe = war.th.countries[1 - g.me] as u32;
    let mut count: Vec<(u32, usize)> = Vec::new();
    for &r in &g.world.provinces[war.th.provs[q]].neighbors {
        let c = country_now(g, r as usize);
        if c == foe {
            return Some(foe);
        }
        if c != mine && c != u32::MAX {
            match count.iter_mut().find(|x| x.0 == c) {
                Some(x) => x.1 += 1,
                None => count.push((c, 1)),
            }
        }
    }
    count.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).map(|x| x.0)
}

impl MapView {
    /// The stretch of a border a stroke of the front tool gives (as in HOI4: drawn along the
    /// border, it takes the border between where it began and where it ended, not only the
    /// provinces under the hand): the province of ours on a border nearest on the screen to the
    /// stroke's start; the border it lies on (with the enemy first); the province of that border
    /// nearest to the stroke's end; the shortest way between them along that border, and every
    /// province of that border the stroke went over. The stretch and the country it faces (None —
    /// the enemy of the war, the rival in peace).
    pub(crate) fn stretch(&self, g: &Game, drawn: &[usize]) -> Option<(Vec<usize>, Option<u32>)> {
        let war = &g.h.war;
        let me = g.me;
        let (&first, &last) = (drawn.first()?, drawn.last()?);
        let at = |q: usize| self.pt(Self::at(g, war.th.provs[q]));
        let ours: Vec<(usize, u32)> = (0..war.owner.len()).filter(|&q| war.owner[q] == me).filter_map(|q| touches(g, q).map(|c| (q, c))).collect();
        // (the ends of the stroke as the hand made them; else the middles of their provinces)
        let (p0, p1) = match self.stroke {
            Some([s, e]) => (self.pt(s), self.pt(e)),
            None => (at(first), at(last)),
        };
        let near = |p: Pos2, set: &mut dyn Iterator<Item = usize>| -> Option<usize> { set.min_by(|&a, &b| (at(a) - p).length_sq().total_cmp(&(at(b) - p).length_sq()).then(a.cmp(&b))) };
        // (as HOI4: the stroke's ends snap to the nearest piece of a border of ours — the province
        // of ours along it, and the country across it; without the stroke's points — to the
        // nearest province of ours on a border)
        let snapped = self.stroke.and_then(|[s, e]| {
            let (qa, c) = self.border_near(g, s, None)?;
            let (qz, _) = self.border_near(g, e, Some(c))?;
            Some((qa, c, qz))
        });
        let (a, c, z) = match snapped {
            Some(x) => x,
            None => {
                let a = match ours.iter().find(|x| x.0 == first) {
                    Some(x) => x.0,
                    None => near(p0, &mut ours.iter().map(|x| x.0))?,
                };
                let c = touches(g, a)?;
                let line = border_with(g, c);
                let z = if line.contains(&last) { last } else { near(p1, &mut line.iter().copied())? };
                (a, c, z)
            }
        };
        let foe = war.th.countries[1 - me] as u32;
        let line: Vec<usize> = border_with(g, c);
        // (the shortest way from a to z over the provinces of that border)
        let mut from: Vec<usize> = vec![usize::MAX; war.owner.len()];
        from[a] = a;
        let mut q = std::collections::VecDeque::from([a]);
        while let Some(p) = q.pop_front() {
            if p == z {
                break;
            }
            for &r in &war.th.nbr[p] {
                if from[r] == usize::MAX && line.contains(&r) {
                    from[r] = p;
                    q.push_back(r);
                }
            }
        }
        let mut out: Vec<usize> = Vec::new();
        if from[z] != usize::MAX {
            let mut p = z;
            while p != a {
                out.push(p);
                p = from[p];
            }
        } else {
            out.push(z);
        }
        out.push(a);
        out.reverse();
        // (a province of that border the way cut a corner past — touching two of the way's — is
        // the stretch's too)
        let corners: Vec<usize> = line.iter().copied().filter(|q| !out.contains(q) && war.th.nbr[*q].iter().filter(|r| out.contains(r)).count() >= 2).collect();
        out.extend(corners);
        for &p in drawn {
            if line.contains(&p) && !out.contains(&p) {
                out.push(p);
            }
        }
        Some((out, (c != foe).then_some(c)))
    }

    /// The piece of a border of ours nearest to world point `w` (with country `want` only, if
    /// given): the province of ours along it (of the theatre) and the country across it.
    fn border_near(&self, g: &Game, w: (f32, f32), want: Option<u32>) -> Option<(usize, u32)> {
        let war = &g.h.war;
        let mine = war.th.countries[g.me] as u32;
        let ww = g.world.w as f32;
        let wrap = |dx: f32| dx - (dx / ww).round() * ww;
        let mut best: Option<(u32, u32)> = None;
        let mut bd = f32::MAX;
        for (pts, b, l, r) in &self.borders {
            if *l == u32::MAX || *r == u32::MAX {
                continue;
            }
            // (a chain whose box is farther than the best piece yet is passed by)
            let bx = wrap((b[0] + b[2]) / 2.0 - w.0).abs() - (b[2] - b[0]) / 2.0;
            let by = (w.1 - (b[1] + b[3]) / 2.0).abs() - (b[3] - b[1]) / 2.0;
            let (bx, by) = (bx.max(0.0), by.max(0.0));
            if bx * bx + by * by > bd {
                continue;
            }
            let (cl, cr) = (country_now(g, *l as usize), country_now(g, *r as usize));
            let (ours, other) = if cl == mine && cr != mine {
                (*l, cr)
            } else if cr == mine && cl != mine {
                (*r, cl)
            } else {
                continue;
            };
            if want.is_some_and(|c| c != other) {
                continue;
            }
            for p in pts {
                let dx = wrap(p[0] - w.0);
                let d = dx * dx + (p[1] - w.1) * (p[1] - w.1);
                if d < bd {
                    bd = d;
                    best = Some((ours, other));
                }
            }
        }
        let (p, c) = best?;
        let q = war.th.at.get(p as usize).copied().filter(|&q| q != NO)? as usize;
        (war.owner[q] == g.me).then_some((q, c))
    }

    /// The middle of the border between provinces `a` and `b` of the theatre on the screen (its
    /// longest piece), if they touch — where a hand drawing along that border passes.
    pub(crate) fn border_point(&self, g: &Game, a: usize, b: usize) -> Option<Pos2> {
        let (pa, pb) = (g.h.war.th.provs[a] as u32, g.h.war.th.provs[b] as u32);
        let w = g.world.w as f32;
        let chain = self.borders.iter().filter(|c| (c.2 == pa && c.3 == pb) || (c.2 == pb && c.3 == pa)).max_by_key(|c| c.0.len())?;
        let p = chain.0[chain.0.len() / 2];
        let dx = ((self.cx - p[0]) / w).round() * w;
        Some(self.cam.project_abs(p[0] + dx, p[1], self.z(p[0], p[1])).0)
    }

    /// The province of the theatre under a point of the screen, if any.
    pub(crate) fn theatre_at(&self, g: &Game, pp: Pos2) -> Option<usize> {
        self.province_at(g, pp).and_then(|p| g.h.war.th.at.get(p).copied()).filter(|&q| q != NO).map(|q| q as usize)
    }

    /// The neighbouring country under a point of the screen whose border with us the front tool
    /// would take (not ours, not the sea, touching our land).
    pub fn neighbour_at(&self, g: &Game, pp: Pos2) -> Option<u32> {
        let p = self.province_at(g, pp)?;
        let c = country_now(g, p);
        let mine = g.h.war.th.countries[g.me] as u32;
        (c != u32::MAX && c != mine && !border_with(g, c).is_empty()).then_some(c)
    }

    /// The tool of the plan takes the left button. The front: a click on a neighbouring country —
    /// the whole border with it for the chosen groups; drawn over provinces of ours — that
    /// stretch (let go — it is theirs). The attack: a click on the enemy's lays the arrow's head
    /// for each chosen group. Whether the button was the tool's.
    pub(super) fn use_tool(&mut self, resp: &egui::Response, g: &mut Game) -> bool {
        let me = g.me;
        // (the spoiled control of the self-checks: the tools take nothing)
        if crate::native::spoiled() {
            return false;
        }
        match self.tool.clone() {
            Tool::None => false,
            Tool::Front { gs, mut drawn } => {
                let drag = resp.dragged_by(egui::PointerButton::Primary);
                let done = resp.drag_stopped() || resp.clicked();
                if !drag && !done {
                    return false;
                }
                // (a click on a neighbour: the whole border with him)
                if resp.clicked() && drawn.is_empty() {
                    if let Some(c) = resp.interact_pointer_pos().and_then(|pp| self.neighbour_at(g, pp)) {
                        let foe = g.h.war.th.countries[1 - me] as u32;
                        let line = border_with(g, c);
                        g.act(Act::Front(gs, (c != foe).then_some(c), line));
                        self.tool = Tool::None;
                        return true;
                    }
                }
                // (the way the hand went this frame, every few points: a quick stroke skips none;
                // the enemy's side of the border too — the stroke is snapped to the border)
                if let Some(pp) = resp.interact_pointer_pos() {
                    let here = self.world_at(pp);
                    // (a drag is known only when the hand has gone a few points: the stroke begins
                    // where the button was pressed)
                    let origin = resp.ctx.input(|i| i.pointer.press_origin()).unwrap_or(pp);
                    if drawn.is_empty() {
                        if let Some(q) = self.theatre_at(g, origin) {
                            drawn.push(q);
                        }
                    }
                    self.stroke = Some(match (self.stroke, drawn.len() <= 1) {
                        (Some([s, _]), false) => [s, here],
                        _ => [self.world_at(origin), here],
                    });
                    let d = if drag { resp.drag_delta() } else { vec2(0.0, 0.0) };
                    let steps = (d.length() / 6.0).ceil().max(1.0) as usize;
                    for i in 0..=steps {
                        let p = pp - d * (1.0 - i as f32 / steps as f32);
                        if let Some(q) = self.theatre_at(g, p) {
                            // (the first is where it began, the last where the hand is now)
                            if drawn.last() != Some(&q) {
                                drawn.push(q);
                            }
                        }
                    }
                }
                if done {
                    if let Some((line, facing)) = self.stretch(g, &drawn) {
                        g.act(Act::Front(gs, facing, line));
                    }
                    self.stroke = None;
                    self.tool = Tool::None;
                } else {
                    self.tool = Tool::Front { gs, drawn };
                }
                true
            }
            Tool::Attack { gs } => {
                if !resp.clicked() {
                    return false;
                }
                let q = resp.interact_pointer_pos().and_then(|pp| self.theatre_at(g, pp));
                if let Some(q) = q.filter(|&q| g.h.war.owner[q] != me) {
                    self.drafts.retain(|d| !gs.contains(&d.0));
                    for &k in &gs {
                        self.drafts.push((k, q));
                    }
                    self.tool = Tool::None;
                }
                true
            }
        }
    }

    /// Where the chosen divisions would go from the hand's province (over a stack of ours too: the
    /// right button sends them there as well; none off the theatre, or — in peace — over the
    /// enemy's land); laid anew only when the hand moves to another province.
    pub(super) fn look_ahead(&mut self, g: &Game, pointer: Option<Pos2>, shift: bool) {
        let war = &g.h.war;
        let (Some(pp), Some(c), false) = (pointer, war.corps.as_ref(), crate::native::spoiled()) else {
            self.preview = None;
            return;
        };
        let Some(to) = self.theatre_at(g, pp).filter(|&q| g.h.at_war || war.owner[q] == g.me) else {
            self.preview = None;
            return;
        };
        let start = |i: usize| {
            let d = &c.divs[i];
            if shift {
                d.way.last().copied().unwrap_or(d.at)
            } else {
                d.at
            }
        };
        // (from the province most of them stand in)
        let idx: Vec<usize> = self.picked.iter().filter_map(|&id| c.find(id)).collect();
        let mut count: Vec<(usize, usize)> = Vec::new();
        for &i in &idx {
            let f = start(i);
            match count.iter_mut().find(|x| x.0 == f) {
                Some(x) => x.1 += 1,
                None => count.push((f, 1)),
            }
        }
        let Some(&(from, _)) = count.iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))) else {
            self.preview = None;
            return;
        };
        let key = (from, to, shift);
        if self.preview.as_ref().is_some_and(|p| p.key == key) {
            return;
        }
        let way = war.way_to(g.me, from, to);
        let mut days = 0.0f32;
        for &i in idx.iter().filter(|&&i| start(i) == from).take(12) {
            let mut a = from;
            let mut t = 0.0;
            for &b in &way {
                t += war.step_days(&g.pack, &c.divs[i].has, a, b);
                a = b;
            }
            days = days.max(t);
        }
        self.preview = Some(Preview { key, way, days });
    }

    /// The way the chosen divisions would take: a line of light through the provinces, the head
    /// at its end, how many days on a plate by it (red into the enemy's land: an attack).
    pub(super) fn preview_draw(&self, painter: &egui::Painter, rect: Rect, g: &Game) {
        let Some(pv) = &self.preview else { return };
        if pv.way.is_empty() {
            return;
        }
        let war = &g.h.war;
        let th = &war.th;
        let at = |q: usize| self.pt(Self::at(g, th.provs[q]));
        let mut pts = vec![at(pv.key.0)];
        pts.extend(pv.way.iter().map(|&q| at(q)));
        let room = rect.expand(rect.width().max(rect.height()));
        if pts.iter().any(|p| !room.contains(*p)) {
            return;
        }
        let foe = war.owner[pv.key.1] != g.me;
        let col = if foe { Color32::from_rgb(240, 84, 60) } else { Color32::from_rgb(255, 222, 120) };
        let t = self.t();
        painter.add(egui::Shape::line(pts.clone(), Stroke::new(6.5, Color32::from_black_alpha(140))));
        painter.extend(egui::Shape::dashed_line_with_offset(&pts, Stroke::new(3.2, col), &[11.0], &[6.0], 17.0 - (t * 34.0) % 17.0));
        for p in &pts[1..pts.len() - 1] {
            painter.circle_filled(*p, 3.6, col);
            painter.circle_stroke(*p, 3.6, Stroke::new(1.0, Color32::from_black_alpha(200)));
        }
        let z = *pts.last().unwrap();
        let b = pts[pts.len() - 2];
        let d = (z - b).normalized();
        if d.x.is_finite() {
            let side = vec2(-d.y, d.x) * 8.0;
            let base = z - d * 14.0;
            painter.add(egui::Shape::convex_polygon(vec![z, base + side, base - side], col, Stroke::new(1.5, Color32::from_black_alpha(200))));
        }
        // the days on a plate under the head
        let n = pv.days.ceil().max(1.0) as usize;
        let mut words = format!("≈ {n} {}", crate::words::plural(n, "день", "дня", "дней"));
        if foe {
            words += " · атака";
        }
        if pv.key.2 {
            words = format!("и дальше: {words}");
        }
        let gal = painter.layout_no_wrap(words, FontId::new(15.0, FontFamily::Name("bold".into())), Color32::from_rgb(250, 244, 226));
        let plate = Rect::from_center_size(z + vec2(0.0, 26.0), gal.size() + vec2(16.0, 8.0));
        painter.rect_filled(plate.translate(vec2(0.0, 2.0)), 4.0, Color32::from_black_alpha(110));
        painter.rect_filled(plate, 4.0, Color32::from_rgba_unmultiplied(22, 20, 16, 235));
        painter.rect_stroke(plate, 4.0, Stroke::new(1.5, col));
        painter.galley(plate.center() - gal.size() / 2.0, gal, Color32::WHITE);
    }

    /// The chains of the border between our land and country `c` as they lie on the screen (the
    /// copy of each nearest the view).
    pub(super) fn border_lines(&self, g: &Game, c: u32) -> Vec<Vec<Pos2>> {
        let w = g.world.w as f32;
        let seen = self.cam.seen(super::RELIEF);
        let mine = g.h.war.th.countries[g.me] as u32;
        let mut out = Vec::new();
        for (pts, b, l, r) in &self.borders {
            if *l == u32::MAX || *r == u32::MAX {
                continue;
            }
            let (cl, cr) = (country_now(g, *l as usize), country_now(g, *r as usize));
            if !((cl == mine && cr == c) || (cr == mine && cl == c)) {
                continue;
            }
            let mid = (b[0] + b[2]) / 2.0;
            let dx = ((self.cx - mid) / w).round() * w;
            if b[2] + dx < seen[0] || b[0] + dx > seen[2] || b[3] < seen[1] || b[1] > seen[3] {
                continue;
            }
            out.push(pts.iter().map(|p| self.cam.project_abs(p[0] + dx, p[1], self.z(p[0], p[1])).0).collect());
        }
        out
    }

    /// The plans of the groups not carried out yet (pale, still arrows from the middle of each
    /// group's stretch of the front, a ring at the head); the banner where new divisions gather;
    /// the front being given: the border under the hand lit (a click takes it whole), or the
    /// stretch being drawn.
    pub(super) fn plan_draw(&self, painter: &egui::Painter, rect: Rect, g: &Game) {
        let war = &g.h.war;
        if war.command[g.me].is_none() {
            return;
        }
        let th = &war.th;
        let at = |q: usize| self.pt(Self::at(g, th.provs[q]));
        let room = rect.expand(rect.width().max(rect.height()) * 1.5);
        let t = self.t();
        let chosen = self.chosen_all();
        if !self.drafts.is_empty() {
            let parts = war.sectors_of(g.me);
            for &(k, q) in &self.drafts {
                let Some(part) = parts.get(k).filter(|p| !p.is_empty()) else { continue };
                let (a, z) = (at(part[part.len() / 2]), at(q));
                if !room.contains(a) || !room.contains(z) {
                    continue;
                }
                let hue = HUES[k % HUES.len()];
                let lit = chosen.is_empty() || chosen.contains(&k);
                arrow(painter, a, z, if lit { 10.0 } else { 7.0 }, hue.gamma_multiply(if lit { 0.55 } else { 0.3 }), 0.18, 0.0);
                painter.circle_stroke(z, 10.0 + 2.0 * (t * 3.0).sin(), Stroke::new(2.0, hue));
                if lit {
                    let gal = painter.layout_no_wrap("план".into(), FontId::new(13.0, FontFamily::Name("bold".into())), Color32::from_rgb(250, 244, 226));
                    let plate = Rect::from_center_size(z + vec2(0.0, -24.0), gal.size() + vec2(12.0, 6.0));
                    painter.rect_filled(plate, 3.0, Color32::from_rgba_unmultiplied(22, 20, 16, 225));
                    painter.rect_stroke(plate, 3.0, Stroke::new(1.2, hue));
                    painter.galley(plate.center() - gal.size() / 2.0, gal, Color32::WHITE);
                }
            }
        }
        // where the new divisions gather: a banner on its pole, in our colours
        if let Some(q) = war.corps.as_ref().and_then(|c| c.muster[g.me]).filter(|&q| war.owner[q] == g.me) {
            let p = at(q);
            if rect.expand(40.0).contains(p) {
                let k = g.countries[th.countries[g.me]].1;
                let ours = Color32::from_rgb(k[0], k[1], k[2]);
                let top = p + vec2(0.0, -34.0);
                painter.line_segment([p + vec2(1.0, 1.0), top + vec2(1.0, 1.0)], Stroke::new(3.0, Color32::from_black_alpha(160)));
                painter.line_segment([p, top], Stroke::new(2.2, Color32::from_rgb(60, 46, 30)));
                let wave = (t * 2.0).sin() * 2.0;
                let flag = vec![top, top + vec2(22.0, 5.0 + wave), top + vec2(0.0, 12.0)];
                painter.add(egui::Shape::convex_polygon(flag, ours, Stroke::new(1.2, Color32::from_black_alpha(200))));
                painter.circle_filled(p, 3.5, Color32::from_rgb(250, 230, 160));
                let gal = painter.layout_no_wrap("сбор".into(), FontId::new(13.0, FontFamily::Name("bold".into())), Color32::from_rgb(250, 244, 226));
                let plate = Rect::from_min_size(top + vec2(26.0, -2.0), gal.size() + vec2(10.0, 4.0));
                painter.rect_filled(plate, 3.0, Color32::from_rgba_unmultiplied(22, 20, 16, 220));
                painter.galley(plate.min + vec2(5.0, 2.0), gal, Color32::WHITE);
            }
        }
        if let Tool::Front { gs, drawn } = &self.tool {
            let hue = HUES[gs.first().copied().unwrap_or(0) % HUES.len()];
            let pp = painter.ctx().input(|i| i.pointer.hover_pos());
            let mut hint = if gs.len() > 1 { "щелчок по соседней стране — весь фронт с ней, поровну; протяните вдоль границы — участок" } else { "щелчок по соседней стране — весь фронт с ней; протяните вдоль границы — участок" }.to_string();
            if drawn.is_empty() {
                // (the border under the hand, lit whole: what a click gives)
                if let Some(c) = pp.and_then(|p| self.neighbour_at(g, p)) {
                    let glow = 0.7 + 0.3 * (t * 4.0).sin().abs();
                    for line in self.border_lines(g, c) {
                        painter.add(egui::Shape::line(line.clone(), Stroke::new(11.0, Color32::from_black_alpha(140))));
                        painter.add(egui::Shape::line(line, Stroke::new(7.0, hue.gamma_multiply(glow))));
                    }
                    let name = g.countries.get(c as usize).map_or(String::new(), |x| x.0.clone());
                    hint = format!("щелчок — весь фронт: {name}");
                }
            } else if let Some((line, facing)) = self.stretch(g, drawn) {
                // (the stretch of the border the stroke gives, lit along its border; its ends ringed)
                let foe = g.h.war.th.countries[1 - g.me] as u32;
                let c = facing.unwrap_or(foe);
                let glow = 0.75 + 0.25 * (t * 4.0).sin().abs();
                for chain in self.border_lines(g, c) {
                    painter.add(egui::Shape::line(chain, Stroke::new(3.0, hue.gamma_multiply(0.35))));
                }
                let ordered = g.h.war.along(&line);
                let pts: Vec<Pos2> = ordered.iter().map(|&q| at(q)).filter(|p| room.contains(*p)).collect();
                if pts.len() >= 2 {
                    painter.add(egui::Shape::line(pts.clone(), Stroke::new(11.0, Color32::from_black_alpha(150))));
                    painter.add(egui::Shape::line(pts.clone(), Stroke::new(7.0, hue.gamma_multiply(glow))));
                }
                for (i, p) in pts.iter().enumerate() {
                    let end = i == 0 || i + 1 == pts.len();
                    let r = if end { 8.0 } else { 4.5 };
                    painter.circle_filled(*p, r, hue);
                    painter.circle_stroke(*p, r, Stroke::new(1.5, Color32::from_black_alpha(220)));
                }
                let name = g.countries.get(c as usize).map_or(String::new(), |x| x.0.clone());
                let n = line.len();
                hint = format!("отпустите — участок: {n} {} · граница: {name}", crate::words::plural(n, "провинция", "провинции", "провинций"));
            }
            // (what to do, by the hand — on the screen: left of the hand near the right edge)
            if let Some(pp) = pp {
                let gal = painter.layout_no_wrap(hint, FontId::new(13.5, FontFamily::Proportional), Color32::from_rgb(250, 244, 226));
                let size = gal.size() + vec2(12.0, 6.0);
                let mut at = pp + vec2(22.0, 18.0);
                if at.x + size.x > rect.max.x - 8.0 {
                    at.x = pp.x - 14.0 - size.x;
                }
                at.x = at.x.max(rect.min.x + 8.0);
                let plate = Rect::from_min_size(at, size);
                painter.rect_filled(plate, 3.0, Color32::from_rgba_unmultiplied(22, 20, 16, 225));
                painter.rect_stroke(plate, 3.0, Stroke::new(1.2, hue));
                painter.galley(plate.min + vec2(6.0, 3.0), gal, Color32::WHITE);
            }
        }
    }

    /// The badge of each group of ours at the middle of its stretch of the front (close up, where
    /// the groups' counters are not drawn): a shield of its colour with its number; the chosen
    /// ones larger, ringed. A click on one chooses the group (with Shift — adds it).
    pub(super) fn badges_draw(&self, painter: &egui::Painter, rect: Rect, g: &Game) {
        let war = &g.h.war;
        let mut out = self.badges.lock().unwrap();
        out.clear();
        let Some(gs) = war.command[g.me].as_ref() else { return };
        let th = &war.th;
        let parts = war.sectors_of(g.me);
        let chosen = self.chosen_all();
        for (k, part) in parts.iter().enumerate().take(gs.len()) {
            if part.is_empty() {
                continue;
            }
            let p = self.pt(Self::at(g, th.provs[part[part.len() / 2]])) + vec2(0.0, -38.0);
            if !rect.shrink(20.0).contains(p) {
                continue;
            }
            let on = chosen.contains(&k);
            let hue = HUES[k % HUES.len()];
            let r = if on { 15.0 } else { 12.0 };
            let shield = |c: Pos2, r: f32| vec![c + vec2(-r, -r), c + vec2(r, -r), c + vec2(r, r * 0.35), c + vec2(0.0, r * 1.25), c + vec2(-r, r * 0.35)];
            painter.add(egui::Shape::convex_polygon(shield(p + vec2(1.5, 2.0), r), Color32::from_black_alpha(150), Stroke::NONE));
            painter.add(egui::Shape::convex_polygon(shield(p, r), hue, Stroke::new(if on { 2.5 } else { 1.5 }, if on { Color32::from_rgb(255, 236, 170) } else { Color32::from_black_alpha(220) })));
            let label = if k < 9 { (k + 1).to_string() } else { "•".to_string() };
            painter.text(p + vec2(0.0, 1.0), egui::Align2::CENTER_CENTER, label, FontId::new(if on { 16.0 } else { 13.0 }, FontFamily::Name("bold".into())), Color32::from_rgb(24, 20, 14));
            out.push((Rect::from_center_size(p + vec2(0.0, 2.0), vec2(2.0 * r + 4.0, 2.4 * r + 4.0)), k));
        }
    }
}
