//! The window's frame: what is drawn each frame and in what order — the map under everything,
//! the bars, the open folder, the map's modes, the windows over them.

use super::*;

impl App {
    /// One frame of the window (without the pictures of it): what the player sees and does, the
    /// cursor last, over everything.
    pub(super) fn draw(&mut self, ctx: &egui::Context) {
        let busy = self.draw_inner(ctx);
        cursor::draw(ctx, &self.theme, busy);
    }

    /// The frame without the cursor; whether a world is being built.
    fn draw_inner(&mut self, ctx: &egui::Context) -> bool {
        // a new world (or a save lived again), when it is ready
        if let Some(b) = &self.building {
            match b.rx.try_recv() {
                Ok(Ok(new)) => {
                    let theme = new.pack.theme.clone();
                    *self.shared.lock().unwrap() = new;
                    self.restyle(ctx, &theme);
                    self.map = None;
                    self.chosen = None;
                    self.edit = None;
                    self.building = None;
                    self.menu = false;
                }
                Ok(Err(e)) => {
                    self.said = Some(e);
                    self.building = None;
                    self.menu = true;
                }
                Err(_) => {
                    let done = b.done.load(std::sync::atomic::Ordering::Relaxed);
                    let line = if b.of > 0 { format!("прожито заново {done} из {} месяцев", b.of) } else { String::new() };
                    let ch = self.chrome.clone();
                    egui::CentralPanel::default().frame(egui::Frame::none().fill(self.skin.desk)).show(ctx, |ui| {
                        // (the bar's material over the whole window, darker at its edges)
                        let r = ui.max_rect();
                        let mut v = chrome::Out::new();
                        ch.lay(&mut v, r, &ch.bar, 0.0);
                        ch.vignette(&mut v, r, r.height() * 0.45);
                        v.push(egui::Shape::line_segment([egui::pos2(r.center().x - 260.0, r.min.y + r.height() * 0.4 - 12.0), egui::pos2(r.center().x + 260.0, r.min.y + r.height() * 0.4 - 12.0)], egui::Stroke::new(1.5, ch.trim)));
                        ui.painter().extend(v);
                        // (the picture of the age: its seal, large, light behind it)
                        let c = egui::pos2(r.center().x, r.min.y + r.height() * 0.4 - 120.0);
                        let mut v = chrome::Out::new();
                        for k in 0..24 {
                            let a = k as f32 / 24.0 * std::f32::consts::TAU + ctx.input(|i| i.time) as f32 * 0.05;
                            let (s1, c1) = a.sin_cos();
                            let (s2, c2) = (a + 0.08).sin_cos();
                            v.push(egui::Shape::convex_polygon(vec![c, c + egui::vec2(c1, s1) * 220.0, c + egui::vec2(c2, s2) * 220.0], ch.trim.gamma_multiply(0.10), egui::Stroke::NONE));
                        }
                        ui.painter().extend(v);
                        let icon = match self.theme.as_str() {
                            "parchment" => "musket",
                            "rust" => "bandit",
                            _ => "helmet",
                        };
                        hud::seal(ui, &ch, c, 64.0, icon, ch.trim);
                        ui.vertical_centered(|ui| {
                            ui.add_space(ui.available_height() * 0.4);
                            ui.label(RichText::new(b.title).family(egui::FontFamily::Name("head".into())).size(32.0).color(self.skin.desk_ink));
                            if b.of > 0 {
                                ui.add_space(8.0);
                                widgets::progress(ui, done as f32 / b.of as f32, 360.0).tip(|| Card::new("Мир проживается заново").icon("hourglass").text(line.clone()));
                            }
                        });
                    });
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    return true;
                }
            }
        }
        let shared = self.shared.clone();
        let mut g = shared.lock().unwrap();
        // every new year the game saves itself
        let now = g.month();
        // (not while pictures or the self-checks are taken: `autosaved` is u32::MAX then)
        if now > 0 && now % 12 == 0 && self.autosaved != now && self.autosaved != u32::MAX && self.shot.is_none() && g.auto.is_none() {
            self.autosaved = now;
            let _ = g.save_named(AUTOSAVE);
        }
        if self.map.is_none() {
            flags::set(ctx, flags::of_world(&g));
            let mut m = map::MapView::new(&g);
            if let Some(shot) = &self.shot {
                m.zoom_once = shot.zoom;
                m.center_once = shot.center.as_deref().and_then(|c| {
                    let th = &g.h.war.th;
                    let at = |p: usize| {
                        let seed = g.world.provinces[p].seed;
                        ((seed % g.world.w) as f32, (seed / g.world.w) as f32)
                    };
                    if c == "high" {
                        let p = th.provs.iter().copied().filter(|&p| !g.world.provinces[p].sea).max_by(|&a, &b| g.world.height[g.world.provinces[a].seed].total_cmp(&g.world.height[g.world.provinces[b].seed]))?;
                        Some(at(p))
                    } else if c == "river" || c == "mouth" {
                        // (the world's widest river: its middle, or the last point downstream)
                        let (pts, _, _) = g.land.rivers.iter().max_by(|a, b| a.2.total_cmp(&b.2).then(a.0.len().cmp(&b.0.len())))?;
                        let q = if c == "mouth" { *pts.last()? } else { pts[pts.len() / 2] };
                        Some((q[0].rem_euclid(g.world.w as f32), q[1]))
                    } else if c == "depot" {
                        // (the first depot of ours, standing or being laid)
                        let w = &g.h.war;
                        let q = w.hubs[g.me].first().copied().or_else(|| g.state.building.iter().find(|b| b.0 == g.me).map(|b| b.1))?;
                        Some(at(th.provs[q]))
                    } else if c == "forest" {
                        // (the largest forest province of the war)
                        let p = th.provs.iter().copied().filter(|&p| g.world.provinces[p].terrain == strategy::world::map::Terrain::Forest).max_by_key(|&p| g.world.provinces[p].area)?;
                        Some(at(p))
                    } else if c == "basin" {
                        // (the first basin below the sea: `ground::below_sea` prints them with MAPDUMP)
                        let at: Vec<f32> = std::env::var("MAPBASIN").ok()?.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                        Some((*at.first()?, *at.get(1)?))
                    } else if c == "hills" || c == "plains" {
                        // (the largest province of hills / plains of the war's lands)
                        use strategy::world::map::Terrain::{Hills, Plains};
                        let kind = if c == "hills" { Hills } else { Plains };
                        let ps = &g.world.provinces;
                        let p = th.provs.iter().copied().filter(|&p| ps[p].terrain == kind).max_by_key(|&p| ps[p].area)?;
                        Some(at(p))
                    } else if c == "marsh" {
                        // (the largest marsh of the war's lands, else of the world)
                        use strategy::world::map::Terrain::Marsh;
                        let ps = &g.world.provinces;
                        let p = th.provs.iter().copied().filter(|&p| ps[p].terrain == Marsh).max_by_key(|&p| ps[p].area).or_else(|| (0..ps.len()).filter(|&p| ps[p].terrain == Marsh).max_by_key(|&p| ps[p].area))?;
                        Some(at(p))
                    } else if c == "range" {
                        // (the heart of the world's widest mountain range: the mountain province
                        // with most mountain neighbours)
                        use strategy::world::map::Terrain::Mountains;
                        let ps = &g.world.provinces;
                        let p = (0..ps.len()).filter(|&p| ps[p].terrain == Mountains).max_by_key(|&p| (ps[p].neighbors.iter().filter(|&&q| ps[q as usize].terrain == Mountains).count(), ps[p].area))?;
                        Some(at(p))
                    } else if c == "town" {
                        // (the largest town of the war's lands)
                        let t = g.towns.iter().filter(|t| th.at.get(t.0).is_some_and(|&q| q != strategy::military::front::NO)).max_by_key(|t| t.2)?;
                        Some(at(t.0))
                    } else {
                        let (x, y) = c.split_once(',')?;
                        Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
                    }
                });
                if std::env::var("MAPDUMP").is_ok() {
                    eprintln!("снимок: середина {:?}", m.center_once);
                }
                if let Some(mode) = map::Mode::ALL.iter().find(|x| Some(x.1) == shot.mode.as_deref()) {
                    m.mode = mode.0;
                }
            }
            self.map = Some(m);
        }
        // the start of a game: the world's map, the player chooses his age, map and country
        if self.start {
            self.start_screen(ctx, &mut g);
            self.settings_window(ctx);
            egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
                if let Some(m) = self.map.as_mut() {
                    m.picking = true;
                    let asks = m.ui(ui, &mut g, None);
                    if let Some(c) = asks.country {
                        if c != g.h.war.th.countries[g.me] {
                            let (path, seed) = (g.path.clone(), g.seed);
                            self.build("СТРОИМ МИР ВАШЕЙ СТРАНЫ…", 0, move |_| {
                                let pack = strategy::pack::Pack::load(&path).map_err(|e| format!("Набор эпохи не открылся: {e}"))?;
                                let mut new = Game::new_as(pack, seed, 0, Some(c));
                                new.path = path;
                                Ok(new)
                            });
                        }
                    }
                }
            });
            return false;
        }
        if let Some(m) = self.map.as_mut() {
            m.picking = false;
        }
        // (the first game played: the guide opens by itself, once, with the clock waiting)
        if self.shot.is_none() && !self.settings.guide_seen && g.auto.is_none() {
            self.guide_open = true;
            self.settings.guide_seen = true;
            self.settings.save();
            g.paused = true;
        }
        self.settings_window(ctx);
        appear(ctx, egui::Id::new("menu-appear"), egui::Id::new("menu"), self.menu);
        let ev_id = g.ev.waiting.first().map(|e| egui::Id::new(("event", e.id)));
        appear(ctx, egui::Id::new("event-appear"), ev_id.unwrap_or(egui::Id::new("event-none")), ev_id.is_some());
        appear(ctx, egui::Id::new("designer-appear"), egui::Id::new("designer"), self.edit.is_some() && self.tab == Some(Tab::Army));
        self.top(ctx, &mut g);
        self.banner(ctx, &mut g);
        self.event(ctx, &mut g);
        self.new_world(ctx, &mut g);
        if g.h.war.command[g.me].is_some() {
            self.armybar(ctx, &mut g);
        }
        self.folder(ctx, &mut g);
        self.modes(ctx);
        self.picked_panel(ctx, &mut g);
        self.planbar(ctx, &mut g);
        self.briefing(ctx, &mut g);
        if !g.h.at_war {
            self.guide_chip(ctx);
        }
        self.guide(ctx, &mut g);
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            if let Some(m) = self.map.as_mut() {
                m.also = self.more.clone();
                let asks = m.ui(ui, &mut g, self.chosen);
                if let Some(c) = asks.group {
                    self.chosen = c;
                    self.more.clear();
                }
                if let Some(k) = asks.toggle {
                    self.toggle_group(k);
                }
                if asks.open_army {
                    self.tab = Some(Tab::Army);
                }
                if let Some(i) = asks.battle {
                    self.tab = Some(Tab::Battle(i));
                }
            }
        });
        drop(g);
        // keys: space — the clock; Esc — close the folder; the keys of the war (`command`: the
        // groups by their numbers, the orders, the speed by + and −)
        self.war_keys(ctx);
        let (space, esc) = ctx.input(|i| (i.key_pressed(egui::Key::Space), i.key_pressed(egui::Key::Escape)));
        if !ctx.wants_keyboard_input() {
            let mut g = self.shared.lock().unwrap();
            if space {
                g.paused = !g.paused;
                g.alert = None;
            }
            // (Esc: the menu closes; else the plan being drawn, the chosen divisions, the chosen
            // group, what is open; with nothing open, the menu opens)
            if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
                self.guide_open = !self.guide_open;
            }
            if esc && self.guide_open {
                self.guide_open = false;
            } else if esc {
                let open = self.tab.is_some() || self.map.as_ref().is_some_and(|m| m.build != map::Build::None);
                let picked = self.map.as_ref().is_some_and(|m| !m.picked.is_empty());
                let drawing = self.map.as_ref().is_some_and(|m| m.tool != map::Tool::None);
                if self.menu {
                    self.menu = false;
                } else if drawing {
                    if let Some(m) = self.map.as_mut() {
                        m.tool = map::Tool::None;
                    }
                } else if self.chosen.is_some() && !picked && self.tab != Some(Tab::Army) {
                    self.chosen = None;
                    self.more.clear();
                } else if picked {
                    if let Some(m) = self.map.as_mut() {
                        m.picked.clear();
                    }
                } else if techtree::is_open(ctx) {
                    techtree::set_open(ctx, false);
                } else if open {
                    self.tab = None;
                    if let Some(m) = self.map.as_mut() {
                        m.build = map::Build::None;
                    }
                } else {
                    self.menu = true;
                    g.paused = true;
                }
            }
        }
        false
    }

    /// The folder of the sphere that is open: a sheet in a frame sliding out of the window's left
    /// edge (and back into it when closed); the designer of a template beside the army's.
    pub(super) fn folder(&mut self, ctx: &egui::Context, g: &mut Game) {
        if self.tab.is_some() {
            self.shown_tab = self.tab;
        }
        let t = ctx.animate_bool_with_time(egui::Id::new("folder-open"), self.tab.is_some(), 0.22);
        let Some(tab) = self.shown_tab.filter(|_| t > 0.0) else { return };
        let k = if spoiled() { 1.0 } else { 1.0 - (1.0 - t).powi(3) };
        let width = (FOLDER * k).round().max(1.0);
        let s = self.skin;
        let ch = self.chrome.clone();
        let mut chosen = self.chosen;
        let mut build = self.map.as_ref().map(|m| m.build).unwrap_or_default();
        let mut close = false;
        let icons = self.icons.clone();
        let edit = &mut self.edit;
        egui::SidePanel::left("folder").exact_width(width).resizable(false).show_separator_line(false).frame(egui::Frame::none()).show(ctx, |ui| {
            let panel = ui.max_rect();
            // (the whole sheet, its right edge at the panel's: what has not slid out yet is beyond
            // the window's left edge)
            let sheet = egui::Rect::from_min_size(egui::pos2(panel.max.x - FOLDER, panel.min.y), egui::vec2(FOLDER, panel.height()));
            let mut v = chrome::Out::new();
            ch.sheet(&mut v, sheet, 7.0);
            // (its shadow on the map)
            v.push(egui::Shape::Mesh({
                let mut m = egui::epaint::Mesh::default();
                let r = egui::Rect::from_min_max(egui::pos2(sheet.max.x, sheet.min.y), egui::pos2(sheet.max.x + 10.0, sheet.max.y));
                m.colored_vertex(r.left_top(), Color32::from_black_alpha(90));
                m.colored_vertex(r.right_top(), Color32::TRANSPARENT);
                m.colored_vertex(r.right_bottom(), Color32::TRANSPARENT);
                m.colored_vertex(r.left_bottom(), Color32::from_black_alpha(90));
                m.add_triangle(0, 1, 2);
                m.add_triangle(0, 2, 3);
                m
            }));
            ui.painter().with_clip_rect(ctx.screen_rect()).extend(v);
            let inner = egui::Rect::from_min_max(sheet.min + egui::vec2(16.0, 14.0), sheet.max - egui::vec2(16.0, 12.0));
            let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
            ui.set_clip_rect(inner.expand2(egui::vec2(8.0, 6.0)).intersect(panel));
            let ui = &mut ui;
            // the folder's banner: a dark plate, the sphere's seal, its name and the country's,
            // the close button
            let (title, icon, hue, line) = match tab {
                Tab::Nation => ("Страна", "flag", Color32::from_rgb(214, 188, 120), "особенности, строй, программы"),
                Tab::Econ => ("Экономика", "econ", Color32::from_rgb(232, 196, 104), "хозяйство, казна, стройка"),
                Tab::Arms => ("Производство", "gears", Color32::from_rgb(196, 176, 150), "заводы, образцы, выпуск"),
                Tab::Science => ("Наука", "science", Color32::from_rgb(132, 206, 214), "разработки и испытания"),
                Tab::Army => ("Армия", "army", Color32::from_rgb(226, 128, 104), "войска, снабжение, фронт"),
                Tab::Navy => ("Флот", "anchor", Color32::from_rgb(110, 160, 214), "эскадры, верфи, морская торговля"),
                Tab::Politics => ("Политика", "castle", Color32::from_rgb(196, 160, 232), "правительство и соседи"),
                Tab::News => ("События", "news", Color32::from_rgb(222, 214, 190), "вести и летопись"),
                Tab::People => ("Деятели", "people", Color32::from_rgb(206, 180, 150), "люди страны и соседа"),
                Tab::Society => ("Общество", "faith", Color32::from_rgb(222, 170, 120), "строй, силы общества, законы, вера"),
                Tab::Ledger => ("Сводка", "chart", Color32::from_rgb(160, 210, 170), "бюджет, графики, причины, прогноз"),
                Tab::Battle(_) => ("Бой", "clash", Color32::from_rgb(226, 128, 104), "отчёт о бое"),
            };
            let (hr, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 64.0), egui::Sense::hover());
            let mut v = chrome::Out::new();
            hud::plate(&mut v, hr, &ch, 3.0);
            // (33rd session: the banner a masthead — the sphere's colour glowing from the seal, its mark large and faint at the
            // right, a rule of the colour along its foot; it was a plain dark plate with a small seal)
            hud::glow(&mut v, egui::Rect::from_min_max(hr.min + egui::vec2(4.0, 4.0), egui::pos2(hr.min.x + hr.width() * 0.75, hr.max.y - 4.0)), hue.gamma_multiply(0.30));
            hud::fade_rule(&mut v, hr.min.x + 8.0, hr.max.x - 8.0, hr.max.y - 6.0, hue.gamma_multiply(0.9), 2.4);
            ui.painter().extend(v);
            let icons_all = chrome::icons(ctx);
            super::look::icon(&icons_all, icon, 78.0, hue.gamma_multiply(0.16)).paint_at(ui, egui::Rect::from_center_size(egui::pos2(hr.max.x - 74.0, hr.center().y - 2.0), egui::vec2(78.0, 78.0)).intersect(hr.shrink(3.0)));
            hud::seal(ui, &ch, egui::pos2(hr.min.x + 33.0, hr.center().y - 1.0), 24.0, icon, hue);
            let tg = ui.painter().layout_no_wrap(title.to_uppercase(), egui::FontId::new(27.0, egui::FontFamily::Name("head".into())), s.desk_ink);
            let sub = format!("{} · {line}", g.h.home(g.me).name);
            let sg = ui.painter().layout_no_wrap(sub, egui::FontId::proportional(13.0), s.desk_ink.gamma_multiply(0.85));
            let x = hr.min.x + 68.0;
            let y0 = hr.center().y - (tg.size().y + sg.size().y - 2.0) / 2.0 - 2.0;
            let th = tg.size().y;
            ui.painter().galley(egui::pos2(x + 1.2, y0 + 1.5), tg.clone(), Color32::from_black_alpha(200));
            ui.painter().galley(egui::pos2(x, y0), tg, s.desk_ink);
            ui.painter().galley(egui::pos2(x, y0 + th - 2.0), sg, s.desk_ink);
            let close_at = egui::Rect::from_center_size(egui::pos2(hr.max.x - 24.0, hr.center().y), egui::vec2(30.0, 28.0));
            let b = ui.put(close_at, |ui: &mut egui::Ui| chrome::press(ui, &ch, &s, chrome::Press { text: "×", size: egui::vec2(30.0, 28.0), font: 20.0, ..chrome::Press::default() }));
            if b.tip(|| Card::new("Закрыть папку").text("Esc — то же самое; карта займёт всё окно.")).clicked() {
                close = true;
            }
            ui.add_space(4.0);
            // the folder's pages, over its scroll (each page its own scroll)
            let pages = panels::pages(tab);
            let mut page = page_of(ctx, tab).min(pages.len().max(1) - 1);
            if !pages.is_empty() {
                if let Some(k) = widgets::tabs(ui, pages, page) {
                    page = k;
                    set_page(ctx, tab, k);
                }
                ui.add_space(6.0);
            }
            if spoiled() {
                ui.label("налог 25%, стройка 40%");
                ui.label(RichText::new("бледная мелочь").size(10.0).color(Color32::from_gray(150)));
                ui.add(egui::Label::new("строка шире папки: она не переносится и раздвигает папку, налезая на карту").wrap_mode(egui::TextWrapMode::Extend));
            }
            let h = ui.available_height();
            // (each folder and each page its own scroll: one shared, a folder opened where another
            // was left)
            widgets::scroll(ui, &format!("folder-scroll-{title}-{page}"), h, true, |ui| match tab {
                Tab::Nation => panels::nation(ui, &s, g, page),
                Tab::Econ => panels::econ(ui, &s, g, page),
                Tab::Arms => panels::production(ui, &s, g),
                Tab::Science => panels::science(ui, &s, g),
                Tab::Army => panels::army(ui, &s, g, &mut chosen, &mut build, &icons, edit, page),
                Tab::Navy => panels::navy(ui, &s, g),
                Tab::Politics => panels::politics(ui, &s, g, page),
                Tab::News => panels::news(ui, &s, g),
                Tab::People => panels::persons(ui, &s, g),
                Tab::Society => panels::society(ui, &s, g, page),
                Tab::Ledger => panels::ledger(ui, &s, g),
                Tab::Battle(i) => panels::battle(ui, &s, g, i),
            });
        });
        if close {
            self.tab = None;
        }
        if self.tab.is_some() {
            self.chosen = chosen;
            if let Some(m) = self.map.as_mut() {
                m.build = build;
            }
        }
        if tab == Tab::Army && self.tab == Some(Tab::Army) && k >= 1.0 {
            designer::window(ctx, FOLDER + 12.0, &s, g, &self.icons, &mut self.edit);
        }
        // (the tree of science beside its folder; it closes with the folder)
        if tab == Tab::Science && self.tab == Some(Tab::Science) && k >= 1.0 {
            techtree::window(ctx, FOLDER + 12.0, &s, g);
        } else if self.tab != Some(Tab::Science) {
            techtree::set_open(ctx, false);
        }
    }

    /// The map's modes, at the bottom right: plates with their icons; the scale of the mode above
    /// them.
    pub(super) fn modes(&mut self, ctx: &egui::Context) {
        let Some(m) = self.map.as_mut() else { return };
        let s = self.skin;
        let ch = self.chrome.clone();
        let icons = &self.icons;
        egui::Area::new(egui::Id::new("modes")).anchor(egui::Align2::RIGHT_BOTTOM, [-12.0, -12.0]).show(ctx, |ui| {
            let back = chrome::hold(ui);
            let fr = egui::Frame::none().inner_margin(egui::Margin::symmetric(8.0, 6.0)).show(ui, |ui| {
                // (29th session: twelve modes in two rows of six, the name of the shown one over
                // them — a row of twelve unnamed discs took half of the army bar's width)
                let (name_now, tip_now) = map::Mode::ALL.iter().find(|x| x.0 == m.mode).map_or(("", ""), |x| (x.1, x.2));
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("КАРТА · {}", name_now.to_uppercase())).family(egui::FontFamily::Name("head".into())).size(13.0).color(s.desk_ink));
                });
                let _ = tip_now;
                if let Some((lo, hi)) = m.mode.ends() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(lo).size(12.0).color(s.desk_ink));
                        let (r, _) = ui.allocate_exact_size(egui::vec2(110.0, 10.0), egui::Sense::hover());
                        let p = ui.painter();
                        p.rect_filled(r.expand(1.5), 2.0, Color32::from_black_alpha(200));
                        for i in 0..30 {
                            let c = m.mode.ramp(i as f32 / 29.0);
                            let x = r.min.x + r.width() * i as f32 / 30.0;
                            p.rect_filled(egui::Rect::from_min_size(egui::pos2(x, r.min.y), egui::vec2(r.width() / 30.0 + 0.5, r.height())), 0.0, Color32::from_rgb(c[0] as u8, c[1] as u8, c[2] as u8));
                        }
                        p.rect_stroke(r.expand(1.5), 2.0, egui::Stroke::new(1.0, ch.trim));
                        ui.label(RichText::new(hi).size(12.0).color(s.desk_ink));
                    });
                }
                ui.add_space(2.0);
                let _ = icons;
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 3.0);
                for row in map::Mode::ALL.chunks(6) {
                    ui.horizontal(|ui| {
                        for &(mode, name, tip) in row {
                            let on = m.mode == mode;
                            let icon = mode.icon();
                            let b = hud::medallion(ui, &ch, &s, icon, 16.0, s.desk_ink, on, ("mode", icon));
                            chrome::spot_last(ctx, "mode", b.rect);
                            if b.tip(|| Card::new(format!("Карта: {}", name.to_lowercase())).icon(icon).text(tip).note(if on { "показана сейчас" } else { "щёлкните, чтобы показать" })).clicked() {
                                m.mode = mode;
                            }
                        }
                    });
                }
            });
            let mut v = chrome::Out::new();
            let r = fr.response.rect;
            hud::plate(&mut v, r, &ch, 4.0);
            chrome::under(ui, back, v);
        });
    }
}
