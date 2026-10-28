//! The war of divisions on the map, close up (as in HOI4): in each province a stack of each
//! side's divisions — the flag, the sign of their main kind, how many, and under it two bars,
//! strength (green) and organisation (yellow); marks for the march, the attack, hunger, the
//! player's own orders. The enemy's stacks are seen as they are next to our divisions, through
//! fog next to our land (≈ how many), not at all deeper. The ways of the chosen divisions as
//! lines to where they go; every attack as a red arrow into the province attacked; every fight
//! going on as a plate with a bar of how it goes.

use eframe::egui::{self, pos2, vec2, Align2, Color32, FontFamily, FontId, Pos2, Rect, Stroke};
use strategy::military::corps::{Corps, NONE};
use strategy::pack::Line;
use strategy::rng::{hash64, Rng};

use crate::Game;
use super::troops::{arrow, counter, room, spread, Counter, ATTACK, HUNGRY, MARCH, OWN, SHORT};
use super::{MapView, HUES};

/// A stack: a side's divisions in a province, as the player sees them.
pub struct Stack {
    pub side: usize,
    pub q: usize,
    pub ids: Vec<u32>,
    /// How many (seen through fog: about so many), their mean strength and organisation.
    pub n: f32,
    pub strength: f32,
    pub org: f32,
    /// Share of the equipment their norms give them that they carry.
    pub kit: f32,
    /// The sign of the main kind of their front.
    pub sym: String,
    /// Their group, if all are of one (NONE otherwise, or the reserve).
    pub group: usize,
    pub marks: u8,
    /// Seen through fog only.
    pub fog: bool,
    /// How many of them the player chose.
    pub picked: usize,
}

/// The main kind of divisions with these units: of their front kinds, the one they have most of
/// (any kind if they have no front).
pub fn main_kind(pack: &strategy::pack::Pack, has: &[f32]) -> usize {
    let front = (0..has.len()).filter(|&u| pack.units[u].line == Line::Front && has[u] > 0.0).max_by(|&a, &b| has[a].total_cmp(&has[b]).then(b.cmp(&a)));
    front.or_else(|| (0..has.len()).max_by(|&a, &b| has[a].total_cmp(&has[b]))).unwrap_or(0)
}

/// What a division is doing, in words.
pub fn doing(g: &Game, c: &Corps, d: &strategy::military::corps::Division) -> String {
    let war = &g.h.war;
    if d.rest > 0.0 {
        return "отходит".into();
    }
    if c.fights.iter().any(|f| f.at == d.at && f.side != d.side) {
        return "обороняется".into();
    }
    match d.way.first() {
        Some(&q) if war.owner[q] != d.side => {
            if c.fights.iter().any(|f| f.at == q && f.side == d.side) {
                "атакует".into()
            } else {
                "идёт на чужую землю".into()
            }
        }
        Some(_) => format!("на марше, ещё {} пров.", d.way.len()),
        None if d.drill < 1.0 => format!("новобранцы, подготовка {:.0}%", d.drill * 100.0),
        None if d.dug > 0.6 => "окопалась".into(),
        None => "стоит".into(),
    }
}

/// The stacks of both sides as the player sees them (the enemy's: on the spot next to our
/// divisions, through fog next to our land, not deeper).
pub fn stacks(g: &Game, picked: &[u32]) -> Vec<Stack> {
    let war = &g.h.war;
    let Some(c) = war.corps.as_ref() else { return Vec::new() };
    let me = g.me;
    let n = war.owner.len();
    let mut near = vec![false; n];
    let mut ours_by = vec![false; n];
    for d in c.divs.iter().filter(|d| d.side == me) {
        near[d.at] = true;
        for &r in &war.th.nbr[d.at] {
            near[r] = true;
        }
    }
    for q in 0..n {
        if war.owner[q] == me {
            for &r in &war.th.nbr[q] {
                ours_by[r] = true;
            }
        }
    }
    let mut by: Vec<Vec<usize>> = vec![Vec::new(); 2 * n];
    for (i, d) in c.divs.iter().enumerate() {
        by[d.side * n + d.at].push(i);
    }
    let pack = &g.pack;
    let mut out = Vec::new();
    for side in 0..2 {
        for q in 0..n {
            let list = &by[side * n + q];
            if list.is_empty() {
                continue;
            }
            let fog = side != me && !near[q];
            if side != me && !near[q] && !ours_by[q] {
                continue;
            }
            let k = list.len() as f32;
            let mut has = vec![0.0f32; pack.units.len()];
            let (mut st, mut org, mut kit) = (0.0, 0.0, 0.0);
            let mut marks = 0u8;
            let (mut short_n, mut hungry_n) = (0usize, 0usize);
            let mut groups: Vec<usize> = Vec::new();
            for &i in list {
                let d = &c.divs[i];
                for (u, x) in d.has.iter().enumerate() {
                    has[u] += x;
                }
                st += c.strength(pack, d);
                org += d.org;
                let eq = c.equipped(pack, d);
                kit += eq;
                if eq < 0.8 {
                    short_n += 1;
                }
                if let Some(&w) = d.way.first() {
                    marks |= if war.owner[w] != side { ATTACK } else { MARCH };
                }
                if d.supplied < 0.5 {
                    hungry_n += 1;
                }
                if d.manual {
                    marks |= OWN;
                }
                if !groups.contains(&d.group) {
                    groups.push(d.group);
                }
            }
            // (28th session: a warning on the counter only when most of its divisions suffer — one
            // hungry division of a stack lit nearly every counter, and the warning meant nothing)
            if short_n * 2 > list.len() {
                marks |= SHORT;
            }
            if hungry_n * 2 > list.len() {
                marks |= HUNGRY;
            }
            let seen_n = if fog {
                let mut r = Rng::new(hash64(war.seed ^ 0xF06 ^ ((war.month as u64) << 20) ^ ((q as u64) << 2) ^ me as u64));
                (k * (war.fog[me] * r.normal()).exp()).round().max(1.0)
            } else {
                k
            };
            out.push(Stack {
                side,
                q,
                ids: list.iter().map(|&i| c.divs[i].id).collect(),
                n: seen_n,
                strength: st / k,
                org: org / k,
                kit: kit / k,
                sym: pack.units[main_kind(pack, &has)].symbol.clone(),
                group: if groups.len() == 1 { groups[0] } else { NONE },
                marks: if side == me { marks } else { marks & ATTACK },
                fog,
                picked: if side == me { list.iter().filter(|&&i| picked.contains(&c.divs[i].id)).count() } else { 0 },
            });
        }
    }
    out
}

impl MapView {
    /// The war of divisions close up: the fights going on, the attacks and the chosen divisions'
    /// ways, the stacks over them. Our stacks go to `self.stacks_drawn` (where, which divisions,
    /// the province), the enemy's to `self.foe_drawn`, the fights to `self.fights_drawn`.
    pub(super) fn divisions(&self, painter: &egui::Painter, rect: Rect, g: &Game) {
        let war = &g.h.war;
        let Some(c) = war.corps.as_ref() else { return };
        let th = &war.th;
        let me = g.me;
        let at = |q: usize| self.pt(Self::at(g, th.provs[q]));
        let on_screen = |p: Pos2| rect.expand(80.0).contains(p);
        let colour = |s: usize| {
            let k = g.countries[th.countries[s]].1;
            Color32::from_rgb(k[0], k[1], k[2])
        };
        let picked = &self.picked;
        let t = self.t();
        // the ways of the chosen divisions: a line through the provinces ahead, dots at each, an
        // arrow's head where it ends (red into the enemy's land)
        let mut drawn: Vec<(usize, usize)> = Vec::new();
        for d in c.divs.iter().filter(|d| d.side == me && picked.contains(&d.id) && !d.way.is_empty()) {
            let end = *d.way.last().unwrap();
            if drawn.contains(&(d.at, end)) {
                continue;
            }
            drawn.push((d.at, end));
            let mut pts = vec![at(d.at)];
            pts.extend(d.way.iter().map(|&q| at(q)));
            if pts.iter().all(|p| !on_screen(*p)) {
                continue;
            }
            let foe = war.owner[end] != me;
            let col = if foe { Color32::from_rgb(236, 76, 56) } else { Color32::from_rgb(140, 220, 110) };
            painter.add(egui::Shape::line(pts.clone(), Stroke::new(5.0, Color32::from_black_alpha(150))));
            let dash = egui::Shape::dashed_line(&pts, Stroke::new(2.6, col), 9.0, 5.0);
            painter.extend(dash);
            for p in &pts[1..pts.len() - 1] {
                painter.circle_filled(*p, 3.0, col);
            }
            let (a, z) = (pts[pts.len() - 2], pts[pts.len() - 1]);
            let dir = (z - a).normalized();
            let side = vec2(-dir.y, dir.x) * 7.0;
            painter.add(egui::Shape::convex_polygon(vec![z, z - dir * 13.0 + side, z - dir * 13.0 - side], col, Stroke::new(1.2, Color32::from_black_alpha(200))));
        }
        // the attacks: ours red, the enemy's onto our land dark red (as many divisions, as thick)
        let mut attacks: Vec<(usize, usize, usize, usize)> = Vec::new();
        for d in &c.divs {
            let Some(&q) = d.way.first() else { continue };
            if war.owner[q] == d.side || !c.fights.iter().any(|f| f.at == q && f.side == d.side) {
                continue;
            }
            // (the enemy's attacks shown where they come onto our land)
            if d.side != me && war.owner[q] != me {
                continue;
            }
            match attacks.iter_mut().find(|x| x.0 == d.side && x.1 == d.at && x.2 == q) {
                Some(x) => x.3 += 1,
                None => attacks.push((d.side, d.at, q, 1)),
            }
        }
        for &(side, from, q, n) in &attacks {
            let (a, z) = (at(from), at(q));
            if !on_screen(a) && !on_screen(z) {
                continue;
            }
            let col = if side == me { Color32::from_rgba_unmultiplied(232, 64, 48, 235) } else { Color32::from_rgba_unmultiplied(120, 20, 20, 235) };
            // (the arrow stops short of the province's middle, where the fight's plate stands)
            let z2 = a + (z - a) * 0.78;
            arrow(painter, a, z2, (3.5 + 1.6 * (n as f32).sqrt()).min(10.0), col, 0.10, (t * 0.9).fract());
        }
        // the fights going on: a plate of crossed swords, a bar of how it goes (ours green, the
        // enemy's red), flashes and smoke
        let mut fights = self.fights_drawn.lock().unwrap();
        fights.clear();
        for (i, f) in c.fights.iter().enumerate() {
            let p = at(f.at) + vec2(0.0, -24.0);
            if !on_screen(p) {
                continue;
            }
            let ours = if f.side == me { f.balance } else { 1.0 - f.balance };
            let seed = (f.at as f32 * 0.618).fract();
            let flash = (0.5 + 0.5 * ((t * 3.1 + seed * 6.3).sin())).powi(3);
            painter.circle_filled(p, 12.0 + 5.0 * flash, Color32::from_rgba_unmultiplied(255, 150, 40, (70.0 * flash) as u8));
            for j in 0..4 {
                let ph = (t * 0.33 + j as f32 / 4.0 + seed).fract();
                let drift = vec2(((j as f32 * 2.1 + seed * 5.0).sin()) * 7.0 + ph * 10.0, -ph * 26.0);
                painter.circle_filled(p + drift, 4.0 + ph * 9.0, Color32::from_rgba_unmultiplied(70, 66, 60, ((1.0 - ph) * (1.0 - ph) * 110.0) as u8));
            }
            let col = if ours >= 0.5 { Color32::from_rgb(255, 214, 64) } else { Color32::from_rgb(255, 90, 74) };
            let plate = Rect::from_center_size(p, vec2(24.0, 19.0));
            painter.rect_filled(plate.expand(1.5), 3.0, Color32::from_black_alpha(200));
            painter.rect_filled(plate, 3.0, Color32::from_rgb(40, 34, 28));
            painter.rect_stroke(plate, 3.0, Stroke::new(1.5, col));
            let icons = super::super::chrome::icons(painter.ctx());
            super::super::flags::image(painter.ctx(), painter, &icons, "clash", plate.shrink2(vec2(4.0, 2.0)), col);
            let bar = Rect::from_min_size(pos2(plate.min.x - 7.0, plate.max.y + 3.0), vec2(plate.width() + 14.0, 5.0));
            painter.rect_filled(bar.expand(1.0), 1.5, Color32::from_black_alpha(210));
            let split = bar.min.x + bar.width() * ours.clamp(0.0, 1.0);
            painter.rect_filled(Rect::from_min_max(bar.min, pos2(split, bar.max.y)), 1.0, Color32::from_rgb(110, 190, 80));
            painter.rect_filled(Rect::from_min_max(pos2(split, bar.min.y), bar.max), 1.0, Color32::from_rgb(210, 60, 46));
            fights.push((plate.expand(4.0), i));
        }
        drop(fights);
        // the stacks: a side's that would sit on each other merged into the larger (as HOI4's
        // counters gather when the map is seen from further), then moved apart
        let mut list = stacks(g, picked);
        list.sort_by(|a, b| b.ids.len().cmp(&a.ids.len()).then(a.q.cmp(&b.q)));
        if !crate::native::spoiled() {
            let size = room(false);
            let mut merged: Vec<Stack> = Vec::new();
            for s in list {
                let p = at(s.q);
                if let Some(m) = merged.iter_mut().find(|m| m.side == s.side && m.fog == s.fog && {
                    let d = at(m.q) - p;
                    d.x.abs() < size.x * 0.8 && d.y.abs() < size.y * 1.1
                }) {
                    let (a, b) = (m.ids.len() as f32, s.ids.len() as f32);
                    m.strength = (m.strength * a + s.strength * b) / (a + b);
                    m.org = (m.org * a + s.org * b) / (a + b);
                    m.kit = (m.kit * a + s.kit * b) / (a + b);
                    m.n += s.n;
                    m.marks |= s.marks;
                    m.picked += s.picked;
                    if m.group != s.group {
                        m.group = NONE;
                    }
                    m.ids.extend(s.ids);
                    continue;
                }
                merged.push(s);
            }
            list = merged;
        }
        let mut items: Vec<(Pos2, usize)> = list.iter().enumerate().filter(|(_, s)| on_screen(at(s.q))).map(|(i, s)| (at(s.q), i)).collect();
        // (the enemy's a little up and to the side of the province's middle, ours on it)
        for it in items.iter_mut() {
            if list[it.1].side != me {
                it.0 += vec2(0.0, -6.0);
            }
        }
        if !crate::native::spoiled() {
            let mut ps: Vec<Pos2> = items.iter().map(|x| x.0).collect();
            let sizes = vec![room(false) + vec2(0.0, 8.0); ps.len()];
            spread(&mut ps, &sizes);
            for (it, p) in items.iter_mut().zip(ps) {
                it.0 = p;
            }
        }
        let mut ours = self.stacks_drawn.lock().unwrap();
        let mut foes = self.foe_drawn.lock().unwrap();
        ours.clear();
        foes.clear();
        let mut figs = self.figures.lock().unwrap();
        figs.clear();
        for (p, i) in items {
            let s = &list[i];
            // the troops' figure by the stack (not for what fog hides): standing at the counter's
            // top edge, facing the enemy's capital — or where its first division marches
            if !s.fog {
                figs.push(self.figure(g, c, s, p));
            }
            let hue = (s.side == me && s.group != NONE).then(|| HUES[s.group % HUES.len()]);
            
            // HOI4 Схлопывание: если дивизий много, мы уменьшаем фишку и делаем её компактной
            let is_huge_stack = s.n > 2.0; 
            
            let r = counter(
                painter,
                p,
                &Counter {
                    n: s.n,
                    fill: colour(s.side),
                    sym: Some(s.sym.as_str()),
                    big: false,
                    fog: s.fog,
                    hue: None,
                    edge: hue,
                    order: None,
                    chosen: s.picked > 0,
                    supply: None,
                    dug: None,
                    country: Some(th.countries[s.side]),
                    // Скрываем громоздкие полоски индивидуального XP/орги, если это плотный стек
                    bars: if is_huge_stack { None } else { (!s.fog).then_some((s.strength, s.org)) },
                    marks: s.marks,
                },
            );
            // (the chosen group's stacks: a ring of its colour round them, breathing — where its
            // divisions stand, at a glance)
            if s.side == me && s.group != NONE && self.chosen == Some(s.group) {
                let hue = HUES[s.group % HUES.len()];
                let glow = 0.6 + 0.4 * (t * 3.0).sin().abs();
                painter.rect_stroke(r.expand(4.0), 4.0, Stroke::new(5.0, Color32::from_black_alpha(110)));
                painter.rect_stroke(r.expand(3.5), 4.0, Stroke::new(2.5, hue.gamma_multiply(glow)));
            }
            // (how many of the stack are chosen, when not all)
            if s.picked > 0 && s.picked < s.ids.len() {
                let c0 = r.right_bottom() + vec2(-6.0, -6.0);
                painter.circle_filled(c0, 8.0, Color32::from_rgb(255, 216, 96));
                painter.text(c0, Align2::CENTER_CENTER, s.picked.to_string(), FontId::new(10.0, FontFamily::Name("bold".into())), Color32::from_rgb(30, 24, 12));
            }
            self.counters.lock().unwrap().push(r);
            if s.side == me {
                // (the spoiled control of the window: drawn, but not found by a click)
                if !crate::native::spoiled() {
                    ours.push((r, s.ids.clone(), s.q));
                }
            } else {
                foes.push((r, s.q));
            }
        }
    }

    /// The figure that stands for stack `s` drawn at `p` on the screen: the model of its main kind
    /// in the era, in its country's uniform, facing the enemy or its march, a few dozen points tall.
    fn figure(&self, g: &Game, c: &Corps, s: &Stack, p: Pos2) -> super::figures::Figure {
        use super::figures as f;
        let war = &g.h.war;
        let th = &war.th;
        let era = self.era.round() as i32;
        let model = match s.sym.as_str() {
            "armour" => {
                if era == 1 {
                    f::RIG
                } else {
                    f::TANK
                }
            }
            "wheeled-armour" => f::TECHNICAL,
            "antitank" | "artillery" | "antiair" => {
                if era == 2 {
                    f::CANNON
                } else {
                    f::GUN
                }
            }
            "cavalry" => f::RIDER,
            _ => match era {
                1 => f::RAIDER,
                2 => f::MUSKETEER,
                _ => f::SOLDIER,
            },
        };
        let (ww, wh) = (g.world.w as f32, g.world.h as f32);
        // (standing on the stack's own province, a little behind its counter — a point of the
        // world, not of the screen: 28th session, it was the ground under the counter's edge, and
        // as the counters gathered, moved apart and the camera tilted with the zoom, the figures
        // jumped from place to place)
        let _ = p;
        let base = Self::at(g, th.provs[s.q]);
        let at = [base.0.rem_euclid(ww), (base.1 - 10.0 / self.k.max(0.5)).clamp(0.0, wh - 1.0)];
        // which way: where its first division marches, else the enemy's capital
        let toward = s
            .ids
            .iter()
            .filter_map(|&id| c.find(id))
            .find_map(|i| c.divs[i].way.first().copied())
            .unwrap_or(th.capital[1 - s.side]);
        let to = Self::at(g, th.provs[toward]);
        let mut dx = to.0 - at[0];
        if dx.abs() > ww / 2.0 {
            dx -= ww * dx.signum();
        }
        let angle = (to.1 - at[1]).atan2(dx);
        let k = g.countries[th.countries[s.side]].1;
        let flag = [k[0] as f32 / 255.0, k[1] as f32 / 255.0, k[2] as f32 / 255.0];
        // (the country's colour worked into the era's cloth: khaki, dust, or the bright coats of
        // the old wars)
        let (cloth, share) = match era {
            1 => ([0.40, 0.36, 0.28], 0.30),
            2 => ([0.30, 0.30, 0.36], 0.70),
            _ => ([0.34, 0.34, 0.24], 0.30),
        };
        let colour = [0, 1, 2].map(|j| (cloth[j] * (1.0 - share) + flag[j] * share) * 0.9);
        let fighting = c.fights.iter().any(|x| x.at == s.q || (x.side == s.side && s.marks & ATTACK != 0));
        let doing = if fighting {
            2.0
        } else if s.marks & (MARCH | ATTACK) != 0 {
            1.0
        } else {
            0.0
        };
        // (a few dozen points tall on the screen; far away they shrink away smoothly, as HOI4's
        // models give way to the counters)
        let t = ((self.k - 0.5) / 0.35).clamp(0.0, 1.0);
        // Meshes are normalised by height, not by their footprint.  A tank is almost twice as
        // long as a soldier is tall, so giving every mesh the same size made armour span several
        // provinces and hide both counters and borders.  Keep its on-screen presence, but scale
        // wide vehicles and guns by their footprint.
        let footprint = match model {
            f::TANK => 0.42,
            f::RIG => 0.50,
            f::TECHNICAL => 0.62,
            f::GUN | f::CANNON => 0.60,
            f::RIDER => 0.78,
            _ => 1.0,
        };
        let size = 64.0 / self.k.max(0.5) * t * t * (3.0 - 2.0 * t) * footprint;
        super::figures::Figure { model, at, size, angle, colour, doing }
    }

    /// The card of a stack: ours — each division (up to ten), its strength, organisation and what
    /// it does; the enemy's — how many, and, seen on the spot, how strong.
    pub(super) fn stack_card(&self, g: &Game, ids: &[u32], q: usize, ours: bool) -> super::super::chrome::Card {
        use super::super::chrome::{Card, Tone};
        let war = &g.h.war;
        let Some(c) = war.corps.as_ref() else { return Card::new("Дивизии") };
        let me = g.me;
        let side = if ours { me } else { 1 - me };
        let pack = &g.pack;
        let divs: Vec<&strategy::military::corps::Division> = ids.iter().filter_map(|&id| c.find(id).map(|i| &c.divs[i])).collect();
        let place = crate::words::terrain_ru(war.th.terrain[q]);
        if !ours {
            let s = stacks(g, &[]).into_iter().find(|s| s.q == q && s.side == side);
            let mut card = Card::new(format!("Враг: {}", g.h.home(side).name)).icon("army").sub(place.to_string());
            match s {
                Some(s) if s.fog => {
                    card = card.row("дивизий", format!("около {:.0}", s.n)).note("видно сквозь туман: точно — только рядом с нашими дивизиями");
                }
                Some(s) => {
                    card = card.row("дивизий", format!("{:.0}", s.n)).row("численность", format!("{:.0}%", s.strength * 100.0)).row("организация", format!("{:.0}%", s.org * 100.0)).row("оснащение", format!("{:.0}%", s.kit * 100.0));
                    if !self.picked.is_empty() && g.h.at_war {
                        card = card.note("правой кнопкой — атаковать выбранными");
                    }
                }
                None => {}
            }
            return card;
        }
        let mut card = Card::new(if divs.len() == 1 { "Дивизия".to_string() } else { format!("Дивизий: {}", divs.len()) }).icon("army").sub(place.to_string());
        for d in divs.iter().take(10) {
            let name = crate::words::division_name(d.no, &c.shapes[d.side][d.shape].name);
            let st = c.strength(pack, d);
            let line = format!("{:.0}% · {:.0}% · {}", st * 100.0, d.org * 100.0, doing(g, c, d));
            card = card.tone(name, line, if d.org < strategy::military::corps::ORG_OUT || st < 0.5 { Tone::Bad } else { Tone::Plain });
        }
        if divs.len() > 10 {
            card = card.note(format!("и ещё {}", divs.len() - 10));
        }
        let hungry = divs.iter().filter(|d| d.supplied < 0.5).count();
        if hungry > 0 {
            card = card.tone("без снабжения", format!("{hungry}"), Tone::Bad);
        }
        let short = divs.iter().filter(|d| c.equipped(pack, d) < 0.8).count();
        if short > 0 {
            card = card.tone("не хватает снаряжения", format!("{short}"), Tone::Bad);
        }
        let green = divs.iter().filter(|d| d.drill < 1.0).count();
        if green > 0 {
            card = card.tone("новобранцы", format!("{green} — учатся за линией фронта, генерал их не берёт"), Tone::Plain);
        }
        card.note("численность · организация · что делает").note("щёлкните — выбрать (Shift — добавить); рамкой — несколько стопок; правой кнопкой по карте — куда идти")
    }

    /// The card of a fight going on: who attacks, how many on each side, its day, how it goes.
    pub(super) fn fight_card(&self, g: &Game, i: usize) -> super::super::chrome::Card {
        use super::super::chrome::{Card, Tone};
        let war = &g.h.war;
        let Some(c) = war.corps.as_ref() else { return Card::new("Бой") };
        let Some(f) = c.fights.get(i) else { return Card::new("Бой") };
        let me = g.me;
        let atts = c.divs.iter().filter(|d| d.side == f.side && d.way.first() == Some(&f.at)).count();
        let defs = c.divs.iter().filter(|d| d.side != f.side && d.at == f.at).count();
        let ours = if f.side == me { f.balance } else { 1.0 - f.balance };
        let mut card = Card::new("Бой")
            .icon("clash")
            .sub(crate::words::terrain_ru(war.th.terrain[f.at]).to_string())
            .row("наступает", g.h.home(f.side).name.clone())
            .row("дивизий", format!("{atts} против {defs}"))
            .row("идёт", format!("{}-й день", f.days))
            .tone("перевес", format!("{:.0}% наш", ours * 100.0), if ours >= 0.5 { Tone::Good } else { Tone::Bad });
        if f.side == me && f.odds != 0.5 {
            card = card.row("генерал обещал", format!("{:.0}%", f.odds * 100.0));
        }
        if f.days > 0 {
            use strategy::military::corps::TACTICS;
            let t = |x: usize| TACTICS.get(f.tactic[x] as usize).map_or("—", |t| t.name);
            let beat = |x: usize| TACTICS.get(f.tactic[x] as usize).is_some_and(|t| t.counters.contains(&f.tactic[1 - x]));
            let (ours_i, theirs_i) = if f.side == me { (0, 1) } else { (1, 0) };
            card = card
                .tone("наша тактика", t(ours_i), if beat(ours_i) { Tone::Good } else if beat(theirs_i) { Tone::Bad } else { Tone::Plain })
                .row("тактика врага", t(theirs_i));
        }
        card.note("полоса под знаком — свежие силы: наши зелёные, врага красные; тактика, которую бьёт тактика врага, ничего не даёт")
    }

    /// The divisions of ours whose stacks lie in the box (the player draws it with the left button).
    pub fn in_box(&self, b: Rect) -> Vec<u32> {
        let mut out = Vec::new();
        for (r, ids, _) in self.stacks_drawn.lock().unwrap().iter() {
            if b.intersects(*r) {
                out.extend(ids.iter().copied());
            }
        }
        out
    }
}
