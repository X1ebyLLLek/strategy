//! The map's mode «Моря»: the ways of our trade over the sea (as thick as what goes on them; red
//! where the enemy sinks it), the zones of the sea with their names (at war, a bar of who holds the
//! sea there), our squadrons where they sail and the enemy's where ours see them.

use super::*;
use strategy::military::navy::Mission;

impl MapView {
    pub(super) fn seas_draw(&self, painter: &egui::Painter, rect: Rect, g: &Game) {
        if self.mode != Mode::Seas || g.pack.ships.is_empty() && g.navy.ways[g.me].is_empty() {
            return;
        }
        let me = g.me;
        let seas = &g.navy.seas;
        let at = |p: usize| self.pt(Self::at(g, p));
        let room = rect.expand(80.0);
        let half = rect.width().max(rect.height());
        // the ways of our trade: the biggest first, as thick as what they carry
        let a = g.country(me);
        let w = &g.h.world;
        let value = |j: usize| w.carried.get(a).and_then(|r| r.get(j)).copied().unwrap_or(0.0) + w.carried.get(j).and_then(|r| r.get(a)).copied().unwrap_or(0.0);
        let mut ways: Vec<&crate::navy::seas::SeaWay> = g.navy.ways[me].iter().filter(|x| !x.zones.is_empty() && value(x.to) > 0.0).collect();
        ways.sort_by(|x, y| value(y.to).total_cmp(&value(x.to)));
        let most = ways.first().map_or(1.0, |x| value(x.to)).max(1e-9);
        for way in ways.iter().take(12).rev() {
            let v = value(way.to) / most;
            let lost = g.navy.lost[me].get(way.to).copied().unwrap_or(0.0);
            let col = if lost > 0.02 { Color32::from_rgba_unmultiplied(214, 70, 50, 200) } else { Color32::from_rgba_unmultiplied(70, 130, 200, 170) };
            let width = 1.5 + 5.0 * v.sqrt();
            let mut line: Vec<Pos2> = Vec::new();
            for &p in &way.path {
                let q = at(p);
                // (the map closes east to west: a way across the seam breaks there)
                if line.last().is_some_and(|l: &Pos2| (l.x - q.x).abs() > half * 0.5) {
                    if line.len() >= 2 {
                        painter.add(egui::Shape::line(std::mem::take(&mut line), Stroke::new(width, col)));
                    }
                    line.clear();
                }
                line.push(q);
            }
            if line.len() >= 2 {
                painter.add(egui::Shape::line(line, Stroke::new(width, col)));
            }
        }
        // the zones: those of our coast, the enemy's and of our and his lanes
        let ours = g.trade_by_zone(me);
        let theirs = g.trade_by_zone(1 - me);
        for (z, zone) in seas.zones.iter().enumerate() {
            let near = g.navy.home[me].contains(&z) || g.navy.home[1 - me].contains(&z) || ours.get(z).copied().unwrap_or(0.0) > 0.0 || theirs.get(z).copied().unwrap_or(0.0) > 0.0;
            if !near {
                continue;
            }
            let p = at(zone.centre);
            if !room.contains(p) {
                continue;
            }
            let gal = painter.layout_no_wrap(zone.name.clone(), FontId::new(12.5, FontFamily::Name("bold".into())), Color32::from_rgb(232, 240, 250));
            let plate = Rect::from_center_size(p, gal.size() + vec2(12.0, 8.0));
            painter.rect_filled(plate, 3.0, Color32::from_rgba_unmultiplied(18, 30, 48, 200));
            if g.h.at_war {
                let c = g.navy.control[me].get(z).copied().unwrap_or(0.0);
                let e = g.navy.control[1 - me].get(z).copied().unwrap_or(0.0);
                if c + e > 0.0 {
                    let bar = Rect::from_min_max(pos2(plate.min.x, plate.max.y - 3.0), plate.max);
                    let split = bar.min.x + bar.width() * c / (c + e);
                    painter.rect_filled(Rect::from_min_max(bar.min, pos2(split, bar.max.y)), 0.0, Color32::from_rgb(70, 140, 220));
                    painter.rect_filled(Rect::from_min_max(pos2(split, bar.min.y), bar.max), 0.0, Color32::from_rgb(200, 70, 50));
                }
                if g.navy.raids[1 - me].get(z).copied().unwrap_or(0.0) > 0.0 && ours.get(z).copied().unwrap_or(0.0) > 0.0 {
                    painter.circle_stroke(plate.right_top() + vec2(4.0, -4.0), 5.0, Stroke::new(2.0, Color32::from_rgb(220, 60, 40)));
                }
            }
            painter.rect_stroke(plate, 3.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(150, 190, 230, 160)));
            painter.galley(plate.center() - gal.size() / 2.0, gal, Color32::WHITE);
        }
        // the squadrons: ours where they are; the enemy's where ours sail too (at war)
        for s in [me, 1 - me] {
            for q in g.navy.fleets[s].iter().filter(|q| !q.ships.is_empty()) {
                if s != me && !(g.h.at_war && g.navy.fleets[me].iter().any(|x| x.zone == q.zone && x.mission.at_sea())) {
                    continue;
                }
                let Some(zone) = seas.zones.get(q.zone) else { continue };
                let off = if s == me { vec2(-26.0, -22.0) } else { vec2(26.0, -22.0) };
                let p = at(zone.centre) + off;
                if !room.contains(p) {
                    continue;
                }
                let col = if s == me { Color32::from_rgb(70, 140, 220) } else { Color32::from_rgb(200, 70, 50) };
                painter.circle_filled(p, 9.0, Color32::from_rgba_unmultiplied(14, 20, 30, 220));
                painter.circle_stroke(p, 9.0, Stroke::new(2.0, col));
                // (letters: the map's fonts have no ship signs)
                let mark = match q.mission {
                    Mission::Port => "П",
                    Mission::Raid => "Р",
                    Mission::Blockade => "Б",
                    Mission::Escort => "К",
                    Mission::Patrol => "Г",
                };
                let gal = painter.layout_no_wrap(format!("{mark}{}", q.ships.len()), FontId::new(11.0, FontFamily::Name("bold".into())), Color32::WHITE);
                painter.galley(p - gal.size() / 2.0, gal, Color32::WHITE);
            }
        }
    }
}
