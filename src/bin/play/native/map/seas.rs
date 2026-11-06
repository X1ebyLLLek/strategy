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
        let t_anim = self.t();
        for (wi, way) in ways.iter().take(12).rev().enumerate() {
            let v = value(way.to) / most;
            let lost = g.navy.lost[me].get(way.to).copied().unwrap_or(0.0);
            let raided = lost > 0.02;
            let col = if raided { Color32::from_rgba_unmultiplied(214, 70, 50, 180) } else { Color32::from_rgba_unmultiplied(70, 140, 220, 150) };
            let glow_col = if raided { Color32::from_rgba_unmultiplied(230, 80, 60, 45) } else { Color32::from_rgba_unmultiplied(50, 110, 190, 40) };
            let width = 2.0 + 5.0 * v.sqrt();
            let mut segs: Vec<Vec<Pos2>> = Vec::new();
            let mut line: Vec<Pos2> = Vec::new();
            for &p in &way.path {
                let q = at(p);
                // (the map closes east to west: a way across the seam breaks there)
                if line.last().is_some_and(|l: &Pos2| (l.x - q.x).abs() > half * 0.5) {
                    if line.len() >= 2 {
                        segs.push(std::mem::take(&mut line));
                    }
                    line.clear();
                }
                line.push(q);
            }
            if line.len() >= 2 {
                segs.push(line);
            }
            for seg in segs {
                painter.add(egui::Shape::line(seg.clone(), Stroke::new(width + 4.0, glow_col)));
                painter.add(egui::Shape::line(seg.clone(), Stroke::new(width, col)));
                // animated merchant convoys gliding along the sea lane
                let mut lens: Vec<f32> = Vec::with_capacity(seg.len() - 1);
                let mut total = 0.0f32;
                for pair in seg.windows(2) {
                    let d = pair[0].distance(pair[1]);
                    lens.push(d);
                    total += d;
                }
                if total < 32.0 {
                    continue;
                }
                let spacing = (76.0 - 26.0 * v.sqrt()).clamp(42.0, 76.0);
                let count = (total / spacing).floor().max(1.0) as usize;
                let hull_fill = if raided { Color32::from_rgb(255, 200, 110) } else { Color32::from_rgb(235, 246, 255) };
                let hull_rim = if raided { Color32::from_rgb(190, 45, 35) } else { Color32::from_rgb(35, 85, 145) };
                let wake_col = if raided { Color32::from_rgba_unmultiplied(240, 120, 90, 115) } else { Color32::from_rgba_unmultiplied(190, 225, 255, 115) };
                for k in 0..count {
                    let u = ((k as f32 / count as f32) + t_anim * 0.04 + wi as f32 * 0.19).fract();
                    let target = u * total;
                    let mut acc = 0.0f32;
                    for ( idx, &seg_len) in lens.iter().enumerate() {
                        if acc + seg_len >= target || idx + 1 == lens.len() {
                            let local = if seg_len > 1e-4 { ((target - acc) / seg_len).clamp(0.0, 1.0) } else { 0.0 };
                            let a_pt = seg[idx];
                            let b_pt = seg[idx + 1];
                            let pos = a_pt + (b_pt - a_pt) * local;
                            if room.contains(pos) && seg_len > 1.0 {
                                let dir = (b_pt - a_pt) / seg_len;
                                let perp = vec2(-dir.y, dir.x);
                                painter.line_segment([pos - dir * 8.0, pos - dir * 2.0], Stroke::new(1.6, wake_col));
                                let hull = vec![pos + dir * 4.4, pos + perp * 2.3, pos - dir * 3.4, pos - perp * 2.3];
                                painter.add(egui::Shape::convex_polygon(hull, hull_fill, Stroke::new(0.9, hull_rim)));
                            }
                            break;
                        }
                        acc += seg_len;
                    }
                }
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
            let gal = painter.layout_no_wrap(zone.name.clone(), FontId::new(12.5, FontFamily::Name("bold".into())), Color32::from_rgb(235, 242, 252));
            let plate = Rect::from_center_size(p, gal.size() + vec2(14.0, 10.0));
            // soft shadow
            painter.rect_filled(plate.translate(vec2(1.5, 2.0)), 4.0, Color32::from_black_alpha(100));
            // body
            painter.rect_filled(plate, 4.0, Color32::from_rgba_unmultiplied(16, 26, 42, 225));
            if g.h.at_war {
                let c = g.navy.control[me].get(z).copied().unwrap_or(0.0);
                let e = g.navy.control[1 - me].get(z).copied().unwrap_or(0.0);
                if c + e > 0.0 {
                    let bar = Rect::from_min_max(pos2(plate.min.x + 2.0, plate.max.y - 3.5), pos2(plate.max.x - 2.0, plate.max.y - 1.0));
                    let split = bar.min.x + bar.width() * c / (c + e);
                    painter.rect_filled(Rect::from_min_max(bar.min, pos2(split, bar.max.y)), 1.0, Color32::from_rgb(70, 150, 230));
                    painter.rect_filled(Rect::from_min_max(pos2(split, bar.min.y), bar.max), 1.0, Color32::from_rgb(220, 75, 60));
                    // golden divider
                    painter.line_segment([pos2(split, bar.min.y - 1.0), pos2(split, bar.max.y + 1.0)], Stroke::new(1.2, Color32::from_rgb(240, 205, 110)));
                }
                if g.navy.raids[1 - me].get(z).copied().unwrap_or(0.0) > 0.0 && ours.get(z).copied().unwrap_or(0.0) > 0.0 {
                    let rpos = plate.right_top() + vec2(4.0, -4.0);
                    painter.circle_filled(rpos, 6.0, Color32::from_rgb(220, 55, 45));
                    painter.circle_stroke(rpos, 6.0, Stroke::new(1.5, Color32::WHITE));
                    painter.text(rpos, Align2::CENTER_CENTER, "!", FontId::new(10.0, FontFamily::Name("bold".into())), Color32::WHITE);
                }
                if g.navy.clashes.iter().rev().take(6).any(|b| b.zone == zone.name && b.month + 2 >= g.month()) {
                    let bpos = plate.left_top() + vec2(-4.0, -4.0);
                    painter.circle_filled(bpos, 6.5, Color32::from_rgb(45, 22, 18));
                    painter.circle_stroke(bpos, 6.5, Stroke::new(1.4, Color32::from_rgb(240, 185, 80)));
                    painter.line_segment([bpos + vec2(-3.0, -3.0), bpos + vec2(3.0, 3.0)], Stroke::new(1.4, Color32::from_rgb(245, 210, 120)));
                    painter.line_segment([bpos + vec2(-3.0, 3.0), bpos + vec2(3.0, -3.0)], Stroke::new(1.4, Color32::from_rgb(245, 210, 120)));
                }
            }
            painter.rect_stroke(plate, 4.0, Stroke::new(1.2, Color32::from_rgba_unmultiplied(140, 185, 230, 180)));
            painter.galley(plate.center() - gal.size() / 2.0, gal, Color32::WHITE);
        }
        // the squadrons: counters with mission badge, ship count and health bar
        let mut zone_counts: std::collections::HashMap<(usize, usize), usize> = std::collections::HashMap::new();
        for s in [me, 1 - me] {
            for q in g.navy.fleets[s].iter().filter(|q| !q.ships.is_empty()) {
                if s != me && !(g.h.at_war && g.navy.fleets[me].iter().any(|x| x.zone == q.zone && x.mission.at_sea())) {
                    continue;
                }
                let Some(zone) = seas.zones.get(q.zone) else { continue };
                let count = zone_counts.entry((s, q.zone)).or_insert(0);
                let stack_offset = vec2((*count % 3) as f32 * 42.0, (*count / 3) as f32 * 22.0);
                *count += 1;

                let base_off = if s == me { vec2(-34.0, -26.0) } else { vec2(20.0, -26.0) };
                let p = at(zone.centre) + base_off + stack_offset;
                if !room.contains(p) {
                    continue;
                }
                let rim_col = if s == me { Color32::from_rgb(80, 160, 240) } else { Color32::from_rgb(230, 80, 65) };
                let (m_col, mark) = match q.mission {
                    Mission::Port => (Color32::from_rgb(85, 105, 125), "П"),
                    Mission::Raid => (Color32::from_rgb(205, 60, 50), "Р"),
                    Mission::Blockade => (Color32::from_rgb(215, 155, 45), "Б"),
                    Mission::Escort => (Color32::from_rgb(55, 160, 95), "К"),
                    Mission::Patrol => (Color32::from_rgb(60, 130, 210), "Д"),
                };
                let box_size = vec2(38.0, 19.0);
                let counter = Rect::from_center_size(p, box_size);

                // shadow
                painter.rect_filled(counter.translate(vec2(1.2, 1.6)), 3.0, Color32::from_black_alpha(110));
                // dark navy body
                painter.rect_filled(counter, 3.0, Color32::from_rgba_unmultiplied(14, 20, 32, 235));

                // mission badge on the left
                let badge_rect = Rect::from_min_max(counter.min, pos2(counter.min.x + 14.0, counter.max.y));
                painter.rect_filled(badge_rect, 3.0, m_col);
                let m_gal = painter.layout_no_wrap(mark.to_string(), FontId::new(10.5, FontFamily::Name("bold".into())), Color32::WHITE);
                painter.galley(badge_rect.center() - m_gal.size() / 2.0, m_gal, Color32::WHITE);

                // ship count on the right
                let count_rect = Rect::from_min_max(pos2(counter.min.x + 14.0, counter.min.y), counter.max);
                let c_gal = painter.layout_no_wrap(q.ships.len().to_string(), FontId::new(11.0, FontFamily::Name("bold".into())), Color32::from_rgb(240, 245, 255));
                painter.galley(count_rect.center() - c_gal.size() / 2.0, c_gal, Color32::WHITE);

                // health strip at the bottom
                let avg_hp = if q.ships.is_empty() { 1.0 } else { q.ships.iter().map(|v| v.health).sum::<f32>() / q.ships.len() as f32 };
                let hp_ratio = avg_hp.clamp(0.0, 1.0);
                let hp_col = if hp_ratio > 0.75 {
                    Color32::from_rgb(65, 190, 110)
                } else if hp_ratio > 0.45 {
                    Color32::from_rgb(225, 170, 50)
                } else {
                    Color32::from_rgb(225, 60, 50)
                };
                let hp_bar = Rect::from_min_max(pos2(counter.min.x + 1.0, counter.max.y - 2.5), pos2(counter.min.x + 1.0 + (box_size.x - 2.0) * hp_ratio, counter.max.y - 0.5));
                painter.rect_filled(hp_bar, 0.5, hp_col);

                // border with side colour
                painter.rect_stroke(counter, 3.0, Stroke::new(1.3, rim_col));
            }
        }
    }
}
