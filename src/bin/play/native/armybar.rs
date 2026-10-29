//! The groups of armies along the bottom of the window: a card for each, chosen by a click or
//! its key, with the plan bar of the chosen ones over them.

use super::*;

impl App {
    /// The groups of armies along the bottom: a tray of dark metal under an ornamented edge, a
    /// card for each — a plate with the group's colour down its side, the general's portrait in a
    /// frame, the group, its order with its badge, a gauge of its supply.
    pub(super) fn armybar(&mut self, ctx: &egui::Context, g: &mut Game) {
        let me = g.me;
        let Some(groups) = g.h.war.command[me].clone() else { return };
        let s = self.skin;
        // the cards in as many rows as they need: the modes of the map keep the bar's right end
        let w = if spoiled() { 400.0 } else { 214.0 };
        let modes = ctx.memory(|m| m.area_rect(egui::Id::new("modes"))).map_or(460.0, |r| r.width() + 24.0);
        let room = ctx.screen_rect().width() - modes - 12.0;
        let (mut cw, chh, gap) = (w, 66.0, 8.0);
        // (after the armies: the staff's reserve, and the fleet when the era has one and we a coast)
        let fleet = !g.pack.ships.is_empty() && !g.navy.coast[me].is_empty();
        let extras = if spoiled() || g.h.war.corps.is_none() { 0 } else { 1 + fleet as usize };
        let cards = groups.len() + extras;
        let mut per_row = if spoiled() { groups.len().max(1) } else { ((room + gap) / (cw + gap)).floor().max(1.0) as usize };
        // (more cards than a row holds: narrower, down to the narrowest that reads, so that they
        // stand in one row as HOI4's — 28th session: two rows took a fifth of the map)
        if !spoiled() && cards > per_row {
            cw = (((room + gap) / cards as f32).floor() - gap).clamp(158.0, w);
            per_row = ((room + gap) / (cw + gap)).floor().max(1.0) as usize;
        }
        // each group's supply: of its provinces' troops (in a war of divisions: of its divisions;
        // how many it has)
        let sup = g.h.war.supply(me);
        let lab = g.h.war.group_of(me);
        let mut acc = vec![(0.0f32, 0.0f32, 0.0f32); groups.len()];
        for (q, &k) in lab.iter().enumerate() {
            if k < groups.len() {
                let n: f32 = g.h.war.troops[me][q].iter().sum();
                acc[k].0 += sup[q] * n;
                acc[k].1 += n;
                acc[k].2 += 1.0;
            }
        }
        let mut members: Vec<Vec<u32>> = vec![Vec::new(); groups.len()];
        let mut fit = vec![(0.0f32, 0.0f32); groups.len()];
        if let Some(c) = g.h.war.corps.as_ref() {
            for a in acc.iter_mut() {
                a.0 = 0.0;
                a.1 = 0.0;
            }
            for d in c.divs.iter().filter(|d| d.side == me && d.group < groups.len()) {
                acc[d.group].0 += d.supplied.min(1.0);
                acc[d.group].1 += 1.0;
                members[d.group].push(d.id);
                fit[d.group].0 += c.strength(&g.pack, d);
                fit[d.group].1 += d.org;
            }
        }
        let drafts: Vec<usize> = self.map.as_ref().map_or(Vec::new(), |m| m.drafts.iter().map(|d| d.0).collect());
        // (a group whose front was given to others stands without one, in reserve)
        let parts = g.h.war.sectors_of(me);
        // each group's fights this month (it attacked from its land, or was attacked in it), and
        // how many went its way
        let mut fights = vec![(0u32, 0u32); groups.len()];
        let war = &g.h.war;
        for f in war.fought.iter().filter(|f| f.month == war.month) {
            let q = if f.attacker == me { f.from } else { f.province };
            if let Some(&k) = lab.get(q) {
                if k < groups.len() {
                    fights[k].0 += 1;
                    fights[k].1 += ((f.attacker == me) == f.won) as u32;
                }
            }
        }
        self.cards.clear();
        let dress = self.chrome.clone();
        egui::TopBottomPanel::bottom("armybar").frame(egui::Frame::none().fill(s.desk).inner_margin(egui::Margin { left: 8.0, right: 8.0, top: 10.0, bottom: 6.0 })).show(ctx, |ui| {
            let tray = ui.max_rect().expand2(egui::vec2(8.0, 0.0));
            let tray = egui::Rect::from_min_max(egui::pos2(tray.min.x, tray.min.y - 10.0), egui::pos2(tray.max.x, tray.max.y + 6.0));
            let mut v = chrome::Out::new();
            dress.lay(&mut v, tray, &dress.bar, 0.0);
            v.push(egui::Shape::rect_filled(tray, 0.0, Color32::from_black_alpha(80)));
            hud::gloss(&mut v, tray, Color32::from_white_alpha(20), Color32::from_black_alpha(90));
            hud::band(&mut v, tray.min.x, tray.max.x, tray.min.y + 4.0, &dress);
            ui.painter().extend(v);
            // (the shade the tray casts up onto the map)
            let fg = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("tray-shade")));
            let mut v = chrome::Out::new();
            hud::cast(&mut v, egui::Rect::from_min_max(egui::pos2(tray.min.x, tray.min.y - 9.0), egui::pos2(tray.max.x, tray.min.y)), false);
            fg.extend(v);
            let rows = cards.div_ceil(per_row).max(1);
            let (area, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), rows as f32 * (chh + gap) - gap), egui::Sense::hover());
            for (k, gr) in groups.iter().enumerate() {
                let at = egui::Rect::from_min_size(area.min + egui::vec2((k % per_row) as f32 * (cw + gap), (k / per_row) as f32 * (chh + gap)), egui::vec2(cw, chh));
                let on = self.chosen == Some(k) || (self.chosen.is_some() && self.more.contains(&k));
                let resp = ui.interact(at, egui::Id::new(("group-card", k)), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                let hot = resp.hovered();
                let gen = gr.general.as_ref().map_or(String::new(), |x| x.name.clone());
                let order = match gr.order {
                    _ if parts.get(k).is_none_or(|p| p.is_empty()) => "в резерве",
                    _ if gr.facing.is_some() => "граница",
                    _ if drafts.contains(&k) && !matches!(gr.order, strategy::military::front::Order::Attack(_)) => "план атаки",
                    strategy::military::front::Order::Hold => "держит",
                    strategy::military::front::Order::Deep => "в глубине",
                    strategy::military::front::Order::Attack(_) => "наступает",
                };
                let fed = if acc[k].1 > 0.0 { acc[k].0 / acc[k].1 } else { g.h.fed[me] };
                let hue = map::HUES[k % map::HUES.len()];
                // the card: a dark plate raised a little under the hand, gold round it when chosen
                let lift = if hot || on { egui::vec2(0.0, -2.0) } else { egui::Vec2::ZERO };
                let at2 = at.translate(lift);
                let mut v = chrome::Out::new();
                if on {
                    let t = ctx.input(|i| i.time) as f32;
                    let glow = 0.55 + 0.45 * (t * 3.0).sin().abs();
                    v.push(egui::epaint::Shadow { offset: egui::Vec2::ZERO, blur: 14.0, spread: 2.0, color: dress.trim.gamma_multiply(0.7 * glow) }.as_shape(at2, 3.0).into());
                }
                hud::plate(&mut v, at2, &dress, 3.0);
                if hot {
                    v.push(egui::Shape::rect_filled(at2, 3.0, Color32::from_white_alpha(12)));
                }
                // (the group's colour: a band down the left, a corner of it at the top right)
                v.push(egui::Shape::rect_filled(egui::Rect::from_min_size(at2.min + egui::vec2(3.0, 3.0), egui::vec2(5.0, at2.height() - 6.0)), 1.0, hue));
                v.push(egui::Shape::convex_polygon(vec![at2.right_top() + egui::vec2(-16.0, 3.0), at2.right_top() + egui::vec2(-3.0, 3.0), at2.right_top() + egui::vec2(-3.0, 16.0)], hue, egui::Stroke::NONE));
                // (the portrait's frame)
                let face = egui::Rect::from_min_size(at2.min + egui::vec2(14.0, 6.0), egui::vec2(38.0, 48.0));
                v.push(egui::Shape::rect_filled(face.expand(2.5), 2.0, Color32::from_black_alpha(220)));
                // (the supply gauge in the card's foot: the crate, the pips as wide as the room leaves, the
                // count of divisions after them)
                let x = face.max.x + 9.0;
                let pip_col = if fed > 0.85 { Color32::from_rgb(150, 210, 110) } else if fed > 0.6 { Color32::from_rgb(240, 196, 92) } else { Color32::from_rgb(240, 110, 90) };
                let count = g.h.war.corps.as_ref().map(|c| {
                    let n = acc[k].1 as usize;
                    let size = g.h.war.army_size(c, g.me);
                    (n, size, format!("{n}/{size}"))
                });
                let count_w = count.as_ref().map_or(0.0, |c| ui.painter().layout_no_wrap(c.2.clone(), egui::FontId::proportional(12.5), s.desk_ink).size().x);
                let foot_y = at2.max.y - 14.0;
                let pips_room = (at2.max.x - 8.0 - (x + 17.0) - if count_w > 0.0 { count_w + 6.0 } else { 0.0 }).max(30.0);
                let pip_w = ((pips_room - 4.0 * 2.0) / 5.0).clamp(5.0, 10.0);
                let pips = hud::pips(&mut v, egui::pos2(x + 17.0, foot_y + 3.0), 5, fed * 5.0, pip_col, pip_w, 5.0);
                // (its divisions' strength and organisation, upright at the right edge)
                if acc[k].1 > 0.0 {
                    command::upright(&mut v, egui::Rect::from_min_max(egui::pos2(at2.max.x - 21.0, at2.min.y + 8.0), egui::pos2(at2.max.x - 8.0, at2.max.y - 22.0)), fit[k].0 / acc[k].1, fit[k].1 / acc[k].1);
                }
                if on {
                    v.push(egui::Shape::rect_stroke(at2.expand(1.0), 3.0, egui::Stroke::new(2.0, hud::lighten(dress.trim, 0.3))));
                }
                ui.painter().extend(v);
                if let Some(img) = look::portrait(&g.face_of(&gen, "general"), 38.0) {
                    img.paint_at(ui, face);
                }
                ui.painter().rect_stroke(face.expand(1.0), 2.0, egui::Stroke::new(1.5, dress.trim));
                // (in battle this month: crossed swords on the portrait's corner, flaring — gold if
                // the fights went our way more than not, red if not)
                if fights[k].0 > 0 {
                    let t = ctx.input(|i| i.time) as f32;
                    let flare = 0.5 + 0.5 * (t * 3.2 + k as f32).sin();
                    let ours = 2 * fights[k].1 >= fights[k].0;
                    let col = if ours { Color32::from_rgb(255, 210, 90) } else { Color32::from_rgb(255, 104, 80) };
                    let c = face.right_bottom() + egui::vec2(-1.0, -2.0);
                    let mut v = chrome::Out::new();
                    v.push(egui::Shape::circle_filled(c, 10.0 + 2.0 * flare, col.gamma_multiply(0.18 + 0.22 * flare)));
                    v.push(egui::Shape::circle_filled(c, 9.0, Color32::from_black_alpha(170)));
                    v.push(egui::Shape::circle_stroke(c, 9.0, egui::Stroke::new(1.2, col.gamma_multiply(0.8))));
                    ui.painter().extend(v);
                    let icons = chrome::icons(ctx);
                    look::icon(&icons, "army", 13.0, col).paint_at(ui, egui::Rect::from_center_size(c, egui::vec2(13.0, 13.0)));
                }
                // the crate before the gauge
                let icons = chrome::icons(ctx);
                look::icon(&icons, "crate", 13.0, s.desk_ink.gamma_multiply(0.8)).paint_at(ui, egui::Rect::from_min_size(egui::pos2(x, foot_y), egui::vec2(13.0, 13.0)));
                // the group's name, its order with the order's badge
                let name = crate::words::group_name(k);
                let room_w = at2.max.x - x - 24.0;
                // (a long name in smaller letters before it is cut short)
                let size = [15.5f32, 14.0, 12.5].into_iter().find(|&z| ui.painter().layout_no_wrap(name.to_uppercase(), egui::FontId::new(z, egui::FontFamily::Name("head".into())), s.desk_ink).size().x <= room_w).unwrap_or(12.5);
                let font = egui::FontId::new(size, egui::FontFamily::Name("head".into()));
                let mut job = egui::text::LayoutJob::simple_singleline(name.to_uppercase(), font, s.desk_ink);
                job.wrap = egui::text::TextWrapping::truncate_at_width(room_w);
                let gal = ui.painter().layout_job(job);
                ui.painter().galley(egui::pos2(x, at2.min.y + 5.0), gal, s.desk_ink);
                map::badge(ui.painter(), egui::pos2(x + 7.0, at2.min.y + 35.0), gr.order, hue);
                let mut job = egui::text::LayoutJob::simple_singleline(order.to_string(), egui::FontId::proportional(13.5), s.desk_ink);
                job.wrap = egui::text::TextWrapping::truncate_at_width(room_w - 4.0);
                let gal = ui.painter().layout_job(job);
                ui.painter().galley(egui::pos2(x + 18.0, at2.min.y + 27.0), gal, s.desk_ink);
                // (its divisions out of what a general leads well: red over it)
                if let Some((n, size, text)) = &count {
                    let col = if n > size { Color32::from_rgb(240, 110, 90) } else { s.desk_ink.gamma_multiply(0.85) };
                    ui.painter().text(egui::pos2(pips.max.x + 6.0, foot_y + 6.5), egui::Align2::LEFT_CENTER, text, egui::FontId::proportional(12.5), col);
                }
                self.cards.push(at);
                let general = gr.general.clone();
                let staff = gr.staff;
                let fought = fights[k];
                let card = || {
                    let mut c = Card::new(name.clone()).face(g.face_of(&gen, "general")).sub(if gen.is_empty() { "без генерала".to_string() } else { format!("генерал {gen}") }).row("приказ", match order { "граница" => "держит границу", "держит" => "держит участок", "в глубине" => "держит в глубину", "план атаки" => "план наступления", o => o }).row("ведёт", if staff { "генштаб" } else { "вы" }).row("дивизий", format!("{:.0} (хорошо ведёт до {})", acc[k].1, strategy::military::front::ARMY)).row("провинций", format!("{:.0}", acc[k].2)).tone("снабжение", format!("{:.0}%", fed * 100.0), if fed > 0.85 { chrome::Tone::Good } else { chrome::Tone::Bad });
                    if fought.0 > 0 {
                        c = c.tone("боёв в этом месяце", format!("{} · наших {}", fought.0, fought.1), if 2 * fought.1 >= fought.0 { chrome::Tone::Good } else { chrome::Tone::Bad });
                    }
                    if let Some(x) = &general {
                        c = c.row("побед", x.won.to_string()).row("поражений", x.lost.to_string()).row("взял провинций", x.took.to_string());
                        if !x.past.is_empty() {
                            c = c.head("Прошлое");
                            for l in x.past.iter().take(4) {
                                c = c.note(l.clone());
                            }
                        }
                    }
                    if acc[k].1 > 0.0 {
                        c = c.row("численность", format!("{:.0}%", fit[k].0 / acc[k].1 * 100.0)).row("организация", format!("{:.0}%", fit[k].1 / acc[k].1 * 100.0));
                    }
                    if k < 9 {
                        c = c.note(format!("клавиша {} — выбрать; Ctrl+{} — выбранные дивизии в неё", k + 1, k + 1));
                    }
                    if members[k].is_empty() {
                        c.note("щёлкните — выбрать группу: над карточками — её планы")
                    } else {
                        c.note("щёлкните — выбрать группу: над карточками — её планы (линия фронта, наступление); с Shift — добавить к выбранным").note("двойной щелчок — выбрать её дивизии: правой кнопкой — вести их самому")
                    }
                };
                let resp = resp.tip(card);
                if resp.double_clicked() && !members[k].is_empty() {
                    self.chosen = None;
                    if let Some(m) = self.map.as_mut() {
                        m.picked = members[k].clone();
                    }
                } else if resp.clicked() && ctx.input(|i| i.modifiers.shift) {
                    self.toggle_group(k);
                    if let Some(m) = self.map.as_mut() {
                        m.picked.clear();
                    }
                } else if resp.clicked() {
                    self.chosen = if on && self.more.is_empty() { None } else { Some(k) };
                    self.more.clear();
                    if let Some(m) = self.map.as_mut() {
                        m.picked.clear();
                        // (28th session: an army chosen whose stretch is off the screen — the map goes
                        // to it, so that its plans are drawn where one sees them)
                        if self.chosen == Some(k) {
                            if let Some(&q) = parts.get(k).and_then(|p| p.get(p.len() / 2)) {
                                let p = g.h.war.th.provs[q];
                                if !m.last_rect.shrink(60.0).contains(m.screen_of(g, p)) {
                                    m.look_at(g, p);
                                }
                            }
                        }
                    }
                }
            }
            // the staff's reserve and the fleet, after the armies
            for i in 0..extras {
                let k = groups.len() + i;
                let at = egui::Rect::from_min_size(area.min + egui::vec2((k % per_row) as f32 * (cw + gap), (k / per_row) as f32 * (chh + gap)), egui::vec2(cw, chh));
                let resp = ui.interact(at, egui::Id::new(("extra-card", i)), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                let hot = resp.hovered();
                let at2 = at.translate(if hot { egui::vec2(0.0, -2.0) } else { egui::Vec2::ZERO });
                let (title, icon, hue) = if i == 0 { ("РЕЗЕРВ СТАВКИ", "staff", Color32::from_rgb(196, 186, 160)) } else { ("ФЛОТ", "anchor", Color32::from_rgb(110, 160, 214)) };
                let mut v = chrome::Out::new();
                hud::plate(&mut v, at2, &dress, 3.0);
                if hot {
                    v.push(egui::Shape::rect_filled(at2, 3.0, Color32::from_white_alpha(12)));
                }
                v.push(egui::Shape::rect_filled(egui::Rect::from_min_size(at2.min + egui::vec2(3.0, 3.0), egui::vec2(5.0, at2.height() - 6.0)), 1.0, hue));
                let socket = egui::Rect::from_min_size(at2.min + egui::vec2(14.0, 9.0), egui::vec2(38.0, 42.0));
                v.push(egui::Shape::rect_filled(socket, 3.0, Color32::from_black_alpha(150)));
                v.push(egui::Shape::rect_stroke(socket, 3.0, egui::Stroke::new(1.2, dress.trim)));
                ui.painter().extend(v);
                let icons = chrome::icons(ctx);
                look::icon(&icons, icon, 28.0, hue).paint_at(ui, egui::Rect::from_center_size(socket.center(), egui::vec2(28.0, 28.0)));
                let x = socket.max.x + 9.0;
                // (a line in the card's foot, cut short rather than run over its edge)
                let foot = |ui: &egui::Ui, text: String, col: Color32| {
                    let mut job = egui::text::LayoutJob::simple_singleline(text, egui::FontId::proportional(12.5), col);
                    job.wrap = egui::text::TextWrapping::truncate_at_width(at2.max.x - 8.0 - x);
                    let gal = ui.painter().layout_job(job);
                    ui.painter().galley(egui::pos2(x, at2.max.y - 7.0 - gal.size().y), gal, col);
                };
                // (33rd session: «РЕЗЕРВ СТАВКИ» ran under the card's edge in a narrow card — set smaller, down to 12 pt, to fit)
                let mut tsize = 14.5;
                while tsize > 12.0 && ui.painter().layout_no_wrap(title.to_string(), egui::FontId::new(tsize, egui::FontFamily::Name("head".into())), s.desk_ink).size().x > at2.max.x - 6.0 - x {
                    tsize -= 0.5;
                }
                ui.painter().text(egui::pos2(x, at2.min.y + 6.0 + (14.5 - tsize) * 0.4), egui::Align2::LEFT_TOP, title, egui::FontId::new(tsize, egui::FontFamily::Name("head".into())), s.desk_ink);
                if i == 0 {
                    // the reserve: divisions outside the armies, recruits still drilling, the depots
                    let (reserve, ids, recruits) = g.h.war.corps.as_ref().map_or((0, Vec::new(), 0), |c| {
                        let mine: Vec<&strategy::military::corps::Division> = c.divs.iter().filter(|d| d.side == me && d.group == strategy::military::corps::NONE).collect();
                        (mine.len(), mine.iter().map(|d| d.id).collect::<Vec<u32>>(), c.divs.iter().filter(|d| d.side == me && d.drill < 1.0).count())
                    });
                    let stored = g.mil.stored[me].iter().sum::<f32>() / strategy::military::corps::BATTALION;
                    let line = if g.mobilised(me) && !g.h.at_war { format!("{reserve} див. · мобилизация") } else { format!("{reserve} {}", crate::words::plural(reserve, "дивизия", "дивизии", "дивизий")) };
                    ui.painter().text(egui::pos2(x, at2.min.y + 26.0), egui::Align2::LEFT_TOP, line, egui::FontId::proportional(13.5), s.desk_ink);
                    foot(ui, format!("запас {stored:.0} бат."), s.desk_ink.gamma_multiply(0.85));
                    let est = g.est_divisions(me);
                    let all = g.h.war.corps.as_ref().map_or(0, |c| c.divs.iter().filter(|d| d.side == me).count());
                    let resp = resp.tip(|| {
                        Card::new("Резерв ставки")
                            .icon("staff")
                            .text("Дивизии вне армий: генерал хорошо ведёт до 24, остальные ждут в резерве — генштаб бросает их туда, где прорыв, и из них формирует новые армии.")
                            .row("дивизий в резерве", reserve.to_string())
                            .row("учатся (новобранцы)", recruits.to_string())
                            .row("дивизий всего / по штату", format!("{all} / {est:.0}"))
                            .row("снаряжения на складах", format!("{stored:.0} батальонов"))
                            .row("резервистов", format!("{:.2} млн", g.mil.reservists[me]))
                            .note("двойной щелчок — выбрать дивизии резерва на карте; щелчок — папка «Армия»")
                    });
                    if resp.double_clicked() && !ids.is_empty() {
                        self.chosen = None;
                        if let Some(m) = self.map.as_mut() {
                            m.picked = ids;
                        }
                    } else if resp.clicked() {
                        self.tab = Some(Tab::Army);
                    }
                } else {
                    // the fleet: ships, the balance at sea, the trade sunk
                    let (n, ours) = g.navy_power(me);
                    let (_, theirs) = g.navy_power(1 - me);
                    let ratio = ours / theirs.max(1e-3);
                    let col = if ratio >= 1.2 { Color32::from_rgb(150, 210, 110) } else if ratio >= 0.8 { Color32::from_rgb(240, 196, 92) } else { Color32::from_rgb(240, 110, 90) };
                    ui.painter().text(egui::pos2(x, at2.min.y + 26.0), egui::Align2::LEFT_TOP, format!("{n} {}", crate::words::plural(n, "корабль", "корабля", "кораблей")), egui::FontId::proportional(13.5), s.desk_ink);
                    let sunk = g.navy.sunk_trade[me];
                    let line = if theirs < 0.5 {
                        "врага нет".to_string()
                    } else if sunk > 0.0 {
                        format!("×{} · топят грузы", super::charts::fix(ratio, 1))
                    } else {
                        format!("сила ×{}", super::charts::fix(ratio, 1))
                    };
                    foot(ui, line, col);
                    let resp = resp.tip(|| {
                        Card::new("Флот")
                            .icon("anchor")
                            .row("кораблей", n.to_string())
                            .row("сила на море", format!("{ours:.0} против {theirs:.0}"))
                            .row("торговля морем", format!("{:.0}%", g.sea_dependence(me) * 100.0))
                            .tone("потоплено грузов за месяц", super::charts::num(sunk), if sunk > 0.0 { chrome::Tone::Bad } else { chrome::Tone::Plain })
                            .note("щелчок — папка «Флот»")
                    });
                    if resp.clicked() {
                        self.tab = Some(Tab::Navy);
                    }
                }
                self.cards.push(at);
            }
        });
    }
}
