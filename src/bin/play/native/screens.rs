//! The windows over the map: a new world being made, the start screen, the settings, an
//! event as a sheet in the middle of the window (the clock waits for a choice).

use super::*;

impl App {
    /// A world made in a thread of its own (the window stays alive meanwhile).
    pub(super) fn build(&mut self, title: &'static str, of: u32, job: impl FnOnce(&std::sync::atomic::AtomicU32) -> Result<Game, String> + Send + 'static) {
        let (tx, rx) = std::sync::mpsc::channel();
        let done = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let d = done.clone();
        std::thread::spawn(move || {
            let _ = tx.send(job(&d));
        });
        self.building = Some(Building { rx, title, done, of });
    }

    /// The menu: go on, save the game, load one, a new world, leave.
    pub(super) fn new_world(&mut self, ctx: &egui::Context, g: &mut Game) {
        if !self.menu {
            return;
        }
        let s = self.skin;
        let wide = egui::vec2(300.0, 34.0);
        let ch = self.chrome.clone();
        let shadow = ctx.style().visuals.window_shadow;
        egui::Window::new("МЕНЮ")
            .id(egui::Id::new("menu"))
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(24.0, 20.0)).shadow(shadow).rounding(3.0))
            .show(ctx, |ui| {
                let back = chrome::hold(ui);
                ui.set_width(300.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("МЕНЮ").family(egui::FontFamily::Name("head".into())).size(26.0).color(s.ink));
                });
                ch.heading(ui, &s, "");
                if let Some(note) = &self.said {
                    ui.label(RichText::new(note).color(s.accent));
                    ui.add_space(4.0);
                }
                ui.spacing_mut().item_spacing.y = 7.0;
                if chrome::button(ui, &ch, &s, "Продолжить", wide, false, false).clicked() {
                    self.menu = false;
                    self.said = None;
                    self.confirm_new = false;
                    self.loading = false;
                }
                if chrome::button(ui, &ch, &s, "Сохранить игру", wide, false, false).tip(|| Card::new("Сохранить игру").icon("scroll").text("В папку saves: страна и дата в имени файла.")).clicked() {
                    self.said = Some(match g.save_file() {
                        Ok(f) => format!("Сохранено: {}", f.file_stem().map_or(String::new(), |x| x.to_string_lossy().into_owned())),
                        Err(e) => format!("Не сохранилось: {e}"),
                    });
                }
                if chrome::button(ui, &ch, &s, "Загрузить игру", wide, false, self.loading).clicked() {
                    self.loading = !self.loading;
                    self.confirm_new = false;
                }
                if self.loading {
                    let saves = crate::tools::saves::list();
                    if saves.is_empty() {
                        ui.label(RichText::new("Сохранений пока нет.").size(13.0).color(s.soft));
                    }
                    let mut pick: Option<PathBuf> = None;
                    widgets::scroll(ui, "saves", 260.0, false, |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        for (file, name, _) in saves.iter().take(40) {
                            let b = chrome::press(ui, &ch, &s, chrome::Press { text: name, size: egui::vec2(ui.available_width(), 28.0), font: 14.0, ..chrome::Press::default() });
                            if b.tip(|| Card::new(name.clone()).icon("scroll").text("Загрузить: мир строится и проживается заново с теми же решениями.")).clicked() {
                                pick = Some(file.clone());
                            }
                        }
                    });
                    if let Some(file) = pick {
                        let text = std::fs::read_to_string(&file).unwrap_or_default();
                        let of = text.lines().find_map(|l| l.strip_prefix("now ").and_then(|r| r.split(' ').next()).and_then(|m| m.parse::<u32>().ok())).unwrap_or(0);
                        self.loading = false;
                        self.said = None;
                        self.build("ЗАГРУЖАЮ СОХРАНЕНИЕ…", of, move |done| {
                            Game::load_text(&text, &mut |m, _| done.store(m, std::sync::atomic::Ordering::Relaxed)).map_err(|e| format!("Не загрузилось: {e}"))
                        });
                    }
                }
                if chrome::button(ui, &ch, &s, "Новый мир", wide, false, self.confirm_new).tip(|| Card::new("Новый мир").icon("compass").text("Новая карта, новые страны, новая история.")).clicked() {
                    self.confirm_new = !self.confirm_new;
                    self.loading = false;
                }
                if self.confirm_new {
                    ui.label("Новая карта, новые страны, новая история. Эту игру сохраните, если она ещё нужна.");
                    if chrome::button(ui, &ch, &s, "Строить новый мир", wide, false, true).clicked() {
                        self.confirm_new = false;
                        let (path, me) = (g.path.clone(), g.me);
                        self.build("СТРОИМ НОВЫЙ МИР…", 0, move |_| {
                            let pack = strategy::pack::Pack::load(&path).map_err(|e| format!("Набор эпохи не открылся: {e}"))?;
                            let seed = crate::fresh_seed(&pack);
                            let mut new = Game::new(pack, seed, me);
                            new.path = path;
                            Ok(new)
                        });
                    }
                }
                if chrome::button(ui, &ch, &s, "Настройки", wide, false, self.settings_open).tip(|| Card::new("Настройки").icon("gears").text("Масштаб окна, весь экран, как скоро приходят подсказки.")).clicked() {
                    self.settings_open = !self.settings_open;
                }
                if chrome::button(ui, &ch, &s, "Клавиши", wide, false, self.keys_open).tip(|| Card::new("Клавиши").icon("hand").text("Все клавиши игры: время, группы армий и их планы, приказы дивизиям, карта.")).clicked() {
                    self.keys_open = !self.keys_open;
                    self.loading = false;
                    self.confirm_new = false;
                }
                if self.keys_open {
                    widgets::scroll(ui, "keys", 300.0, false, |ui| {
                        ui.spacing_mut().item_spacing.y = 3.0;
                        command::keys_page(ui, &ch, &s);
                    });
                }
                if chrome::button(ui, &ch, &s, "Выйти из игры", wide, false, false).tip(|| Card::new("Выйти из игры").icon("castle").text("Игра сохранится сама (автосохранение).")).clicked() {
                    let _ = g.save_named(AUTOSAVE);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                let mut v = chrome::Out::new();
                ch.sheet(&mut v, ui.min_rect().expand2(egui::vec2(24.0, 20.0)), 8.0);
                chrome::under_window(ui, back, v);
            });
    }

    /// The start of a game, over the world's map: the age (each a seal: a new world of that age
    /// on the same map), the map (another), the player's country and its rival (with their flags;
    /// a click on the map plays another), the settings, the game begun.
    pub(super) fn start_screen(&mut self, ctx: &egui::Context, g: &mut Game) {
        let s = self.skin;
        let ch = self.chrome.clone();
        egui::Area::new(egui::Id::new("start")).anchor(egui::Align2::LEFT_CENTER, [28.0, 0.0]).show(ctx, |ui| {
            let back = chrome::hold(ui);
            let fr = egui::Frame::none().inner_margin(egui::Margin::symmetric(26.0, 22.0)).show(ui, |ui| {
                ui.set_width(380.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("НОВАЯ ИГРА").family(egui::FontFamily::Name("head".into())).size(30.0).color(s.ink));
                });
                ch.heading(ui, &s, "Эпоха");
                let ages: [(&str, &str, &str, &str); 3] = [("packs/musket.toml", "Мушкеты", "musket", "линейная пехота, кавалерия и пушки; тракты вместо железных дорог"), ("packs/industrial.toml", "Штаб", "helmet", "заводы, танки и самолёты, железные дороги"), ("packs/wasteland.toml", "Пустоши", "bandit", "после конца света: банды, ржавые машины, руины")];
                let now = ages.iter().position(|a| g.path.ends_with(a.0.trim_start_matches("packs/"))).unwrap_or(1);
                let choices: Vec<(&str, &str)> = ages.iter().map(|a| (a.1, a.2)).collect();
                if let Some(k) = widgets::law(ui, &choices, now, now, &|k| Card::new(ages[k].1).icon(ages[k].2).text(ages[k].3).note(if k == now { "эта эпоха сейчас" } else { "щёлкните — тот же мир в этой эпохе" })) {
                    let (path, seed) = (ages[k].0.to_string(), g.seed);
                    self.build("СТРОИМ МИР…", 0, move |_| {
                        let pack = strategy::pack::Pack::load(&path).map_err(|e| format!("Набор эпохи не открылся: {e}"))?;
                        let mut new = Game::new(pack, seed, 0);
                        new.path = path;
                        Ok(new)
                    });
                }
                ch.heading(ui, &s, "Карта");
                ui.label(RichText::new(format!("Карта № {}", g.seed)).size(15.0).color(s.soft));
                if widgets::plate(ui, "Другая карта", ui.available_width(), false, true).tip(|| Card::new("Другая карта").icon("compass").text("Новая земля, новые страны, новая история — в той же эпохе.")).clicked() {
                    let path = g.path.clone();
                    self.build("СТРОИМ НОВЫЙ МИР…", 0, move |_| {
                        let pack = strategy::pack::Pack::load(&path).map_err(|e| format!("Набор эпохи не открылся: {e}"))?;
                        let seed = crate::fresh_seed(&pack);
                        let mut new = Game::new(pack, seed, 0);
                        new.path = path;
                        Ok(new)
                    });
                }
                ch.heading(ui, &s, "Ваша страна");
                let me = g.me;
                let icons = chrome::icons(ui.ctx());
                for (k, side) in [me, 1 - me].into_iter().enumerate() {
                    let c = g.h.war.th.countries[side];
                    let (row, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 46.0), egui::Sense::hover());
                    let fr = egui::Rect::from_min_size(row.min + egui::vec2(2.0, 6.0), egui::vec2(52.0, 34.0));
                    if let Some(f) = flags::get(ui.ctx(), c) {
                        flags::paint(ui.ctx(), ui.painter(), fr, &f, ch.trim, Some(&icons));
                    }
                    let n = g.h.home(side);
                    ui.painter().text(egui::pos2(fr.max.x + 12.0, row.min.y + 5.0), egui::Align2::LEFT_TOP, n.name.to_uppercase(), egui::FontId::new(18.0, egui::FontFamily::Name("head".into())), s.ink);
                    let what = if k == 0 { "вы" } else { "главный соперник" };
                    ui.painter().text(egui::pos2(fr.max.x + 12.0, row.min.y + 27.0), egui::Align2::LEFT_TOP, what, egui::FontId::proportional(13.5), if k == 0 { s.accent } else { s.soft });
                }
                ui.add_space(2.0);
                ui.label(RichText::new("Щёлкните по любой стране на карте, чтобы играть за неё; соперника позже можно сменить в «Политике».").size(14.0).color(s.soft));
                ui.add_space(10.0);
                ui.spacing_mut().item_spacing.y = 7.0;
                let w = ui.available_width();
                if widgets::plate(ui, "Начать игру", w, true, true).tip(|| Card::new("Начать игру").icon("play").text("Время стоит, пока вы не пустите его (пробел).").note("сначала откроется папка «Страна»: какая она, чем сильна и что её тянет вниз")).clicked() {
                    self.start = false;
                    g.paused = true;
                    // (the country as its past made it: the first thing the player sees)
                    self.tab = Some(Tab::Nation);
                }
                ui.horizontal(|ui| {
                    let half = (w - 8.0) / 2.0;
                    if widgets::plate(ui, "Настройки", half, self.settings_open, true).clicked() {
                        self.settings_open = !self.settings_open;
                    }
                    if widgets::plate(ui, "Загрузить игру", half, false, true).tip(|| Card::new("Загрузить игру").icon("scroll").text("Список сохранений откроется в меню.")).clicked() {
                        self.start = false;
                        self.menu = true;
                        self.loading = true;
                    }
                });
            });
            let mut v = chrome::Out::new();
            ch.sheet(&mut v, fr.response.rect, 8.0);
            chrome::under(ui, back, v);
        });
    }

    /// The settings: the scale of the interface (as the window's size makes room), the full
    /// screen, how soon the cards come under the hand; kept for the next time.
    pub(super) fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let s = self.skin;
        let ch = self.chrome.clone();
        let mut set = self.settings;
        let mut close = false;
        egui::Window::new("НАСТРОЙКИ").id(egui::Id::new("settings")).title_bar(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).collapsible(false).resizable(false).frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(24.0, 20.0))).show(ctx, |ui| {
            let back = chrome::hold(ui);
            ui.set_width(360.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("НАСТРОЙКИ").family(egui::FontFamily::Name("head".into())).size(26.0).color(s.ink));
            });
            ch.heading(ui, &s, "Размер интерфейса");
            let scales = [0.85f32, 1.0, 1.15, 1.3];
            let now = scales.iter().position(|&x| (x - set.scale).abs() < 0.01).unwrap_or(1);
            if let Some(k) = widgets::switch(ui, &["Мельче", "Обычно", "Крупнее", "Крупно"], now) {
                set.scale = scales[k];
            }
            ch.heading(ui, &s, "Окно");
            if let Some(k) = widgets::switch(ui, &["В окне", "Весь экран"], set.full as usize) {
                set.full = k == 1;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(set.full));
            }
            ch.heading(ui, &s, "Подсказки приходят");
            let waits = [0.05f64, 0.22, 0.6];
            let now = waits.iter().position(|&x| (x - set.tip).abs() < 0.01).unwrap_or(1);
            if let Some(k) = widgets::switch(ui, &["Сразу", "Обычно", "Не спеша"], now) {
                set.tip = waits[k];
            }
            ui.add_space(10.0);
            if widgets::plate(ui, "Готово", ui.available_width(), true, true).clicked() {
                close = true;
            }
            let mut v = chrome::Out::new();
            ch.sheet(&mut v, ui.min_rect().expand2(egui::vec2(24.0, 20.0)), 8.0);
            chrome::under_window(ui, back, v);
        });
        if set != self.settings {
            self.settings = set;
            set.apply(ctx);
            set.save();
        }
        if close {
            self.settings_open = false;
        }
    }

    /// An event waiting for the player's choice: a sheet in the middle of the window, its icon
    /// beside the title.
    pub(super) fn event(&mut self, ctx: &egui::Context, g: &mut Game) {
        let Some(ev) = g.ev.waiting.first().cloned() else { return };
        let s = self.skin;
        let ch = self.chrome.clone();
        let shadow = ctx.style().visuals.window_shadow;
        // (the event's picture: its sign and two beside it)
        let (icon, left, right) = match ev.kind {
            crate::country::events::Kind::Unrest(_) => ("revolt", "people", "fist"),
            crate::country::events::Kind::Famine(_) => ("grain", "people", "broken"),
            crate::country::events::Kind::Treasury => ("coins", "castle", "stamper"),
            crate::country::events::Kind::Thief => ("stamper", "coins", "binoculars"),
            crate::country::events::Kind::General(_) => ("medal", "army", "compass"),
            crate::country::events::Kind::Claim => ("flag", "scroll", "army"),
            crate::country::events::Kind::Weary => ("fist", "people", "broken"),
            crate::country::events::Kind::Coup => ("crown", "castle", "fist"),
            crate::country::events::Kind::PeaceOffer => ("scroll", "flag", "quill"),
            crate::country::events::Kind::Lenders => ("coins", "stamper", "castle"),
            crate::country::events::Kind::Dear => ("shop", "coins", "people"),
            crate::country::events::Kind::Reform => ("graduate", "quill", "anvil"),
            crate::country::events::Kind::Genius(..) => ("science", "graduate", "coins"),
            crate::country::events::Kind::Hero(_) => ("medal", "people", "crown"),
            crate::country::events::Kind::Panic => ("coins", "people", "broken"),
            crate::country::events::Kind::Flight => ("backforth", "coins", "castle"),
            crate::country::events::Kind::AllyCalls(_) => ("shield", "flag", "army"),
            crate::country::events::Kind::Offer(..) => ("scroll", "quill", "crown"),
            crate::country::events::Kind::Realm(e) => match e {
                crate::country::realm::Ev::Revolution(_) => ("revolt", "people", "flag"),
                crate::country::realm::Ev::Officers => ("helmet", "castle", "fist"),
                crate::country::realm::Ev::Demand(..) => ("quill", "people", "scroll"),
                crate::country::realm::Ev::Schism => ("faith", "quill", "broken"),
                crate::country::realm::Ev::Election => ("vote", "people", "scroll"),
                crate::country::realm::Ev::Separatists => ("flag", "broken", "people"),
            },
        };
        egui::Window::new(ev.title.to_uppercase())
            .id(egui::Id::new(("event", ev.id)))
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .fixed_size([520.0, 10.0])
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(26.0, 22.0)).shadow(shadow).rounding(3.0))
            .show(ctx, |ui| {
                let back = chrome::hold(ui);
                ui.label(RichText::new(ev.title.to_uppercase()).family(egui::FontFamily::Name("head".into())).size(24.0).color(s.accent));
                ui.add_space(4.0);
                // the picture: a painting of the moment (public domain: `assets/events`), toned to
                // the era, darker at its edges, in a double frame of trim; the event's sign in a
                // medallion at its corner
                let (r, _) = ui.allocate_exact_size(egui::vec2(468.0, 189.0), egui::Sense::hover());
                let (file, bytes): (&str, &'static [u8]) = match ev.kind {
                    crate::country::events::Kind::Unrest(_) => ("unrest", include_bytes!("../../../../assets/events/unrest.jpg")),
                    crate::country::events::Kind::Famine(_) => ("famine", include_bytes!("../../../../assets/events/famine.jpg")),
                    crate::country::events::Kind::Treasury => ("treasury", include_bytes!("../../../../assets/events/treasury.jpg")),
                    crate::country::events::Kind::Thief => ("thief", include_bytes!("../../../../assets/events/thief.jpg")),
                    crate::country::events::Kind::General(_) => ("general", include_bytes!("../../../../assets/events/general.jpg")),
                    crate::country::events::Kind::Claim => ("claim", include_bytes!("../../../../assets/events/claim.jpg")),
                    crate::country::events::Kind::Weary => ("weary", include_bytes!("../../../../assets/events/weary.jpg")),
                    crate::country::events::Kind::Coup => ("coup", include_bytes!("../../../../assets/events/coup.jpg")),
                    crate::country::events::Kind::PeaceOffer => ("peace", include_bytes!("../../../../assets/events/peace.jpg")),
                    crate::country::events::Kind::Lenders => ("lenders", include_bytes!("../../../../assets/events/lenders.jpg")),
                    crate::country::events::Kind::Dear => ("dear", include_bytes!("../../../../assets/events/dear.jpg")),
                    crate::country::events::Kind::Reform => ("reform", include_bytes!("../../../../assets/events/reform.jpg")),
                    // (the people's and the owners' moments borrow the nearest paintings)
                    crate::country::events::Kind::Genius(..) => ("reform", include_bytes!("../../../../assets/events/reform.jpg")),
                    crate::country::events::Kind::Hero(_) => ("general", include_bytes!("../../../../assets/events/general.jpg")),
                    crate::country::events::Kind::Panic => ("lenders", include_bytes!("../../../../assets/events/lenders.jpg")),
                    crate::country::events::Kind::Flight => ("treasury", include_bytes!("../../../../assets/events/treasury.jpg")),
                    crate::country::events::Kind::AllyCalls(_) => ("claim", include_bytes!("../../../../assets/events/claim.jpg")),
                    crate::country::events::Kind::Offer(..) => ("peace", include_bytes!("../../../../assets/events/peace.jpg")),
                    crate::country::events::Kind::Realm(e) => match e {
                        crate::country::realm::Ev::Revolution(_) | crate::country::realm::Ev::Separatists => ("unrest", include_bytes!("../../../../assets/events/unrest.jpg")),
                        crate::country::realm::Ev::Officers => ("coup", include_bytes!("../../../../assets/events/coup.jpg")),
                        crate::country::realm::Ev::Demand(..) | crate::country::realm::Ev::Schism | crate::country::realm::Ev::Election => ("reform", include_bytes!("../../../../assets/events/reform.jpg")),
                    },
                };
                let tone = match g.pack.theme.as_str() {
                    "parchment" => Color32::from_rgb(242, 226, 192),
                    "rust" => Color32::from_rgb(214, 200, 178),
                    _ => Color32::from_rgb(246, 238, 222),
                };
                ui.painter().rect_filled(r, 2.0, Color32::from_rgb(20, 18, 15));
                egui::Image::from_bytes(format!("bytes://event/{file}.jpg"), egui::load::Bytes::Static(bytes)).fit_to_exact_size(r.size()).tint(tone).paint_at(ui, r);
                let mut v = chrome::Out::new();
                ch.vignette(&mut v, r, 38.0);
                v.push(egui::Shape::rect_stroke(r.shrink(4.0), 1.0, egui::Stroke::new(1.5, ch.trim)));
                v.push(egui::Shape::rect_stroke(r.shrink(7.5), 1.0, egui::Stroke::new(0.8, ch.trim.gamma_multiply(0.7))));
                ch.bevel(&mut v, r, false, 2.0);
                ui.painter().with_clip_rect(r).extend(v);
                let _ = (left, right);
                super::hud::seal(ui, &ch, r.left_bottom() + egui::vec2(30.0, -30.0), 22.0, icon, ch.trim);
                ui.add_space(2.0);
                ui.label(RichText::new(&ev.text).size(16.0));
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.y = 7.0;
                for (k, (what, tip)) in ev.choices.iter().enumerate() {
                    if chrome::button(ui, &ch, &s, what, egui::vec2(468.0, 34.0), false, false).tip(|| Card::new(what.clone()).icon(icon).text(tip.clone())).clicked() {
                        g.act(crate::tools::saves::Act::Answer(ev.id, k));
                    }
                }
                let mut v = chrome::Out::new();
                ch.sheet(&mut v, ui.min_rect().expand2(egui::vec2(26.0, 22.0)), 8.0);
                chrome::under_window(ui, back, v);
            });
    }
}
