//! The guide (29th session, F1 or the «?» under the bar): the window said nothing of itself — where
//! to look, what a number is, what moves it, where the lever is, what a counter on the map means,
//! how an army is led. Four pages: the screen (a plan with numbered places), the figures of the
//! country (what it is, what moves it, where to change it — a button to the folder), the map and
//! the war (the counters and lines drawn live, as they are on the map), leading the army (the
//! steps, the keys). The words are the game's own (`terms`), not new ones.

use super::*;
use eframe::egui::{pos2, vec2, Align2, FontFamily, FontId, Rect, Sense, Shape, Stroke};

use super::map::{self, counter, Counter, ATTACK, HUNGRY, MARCH, OWN, SHORT};

const PAGES: [&str; 4] = ["Экран", "Показатели страны", "Карта и война", "Армия"];

impl App {
    /// The window of the guide over everything, when it is open.
    pub(super) fn guide(&mut self, ctx: &egui::Context, g: &mut Game) {
        if !self.guide_open {
            return;
        }
        let s = self.skin;
        let ch = self.chrome.clone();
        let screen = ctx.screen_rect();
        let size = vec2((screen.width() - 80.0).clamp(640.0, 900.0), (screen.height() - 120.0).clamp(420.0, 660.0));
        let mut close = false;
        let mut go: Option<(Tab, usize)> = None;
        egui::Area::new(egui::Id::new("guide")).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).order(egui::Order::Foreground).show(ctx, |ui| {
            // (a veil over the map, under the window, so that the window reads as the thing to look at)
            ui.painter().with_clip_rect(screen).rect_filled(screen, 0.0, Color32::from_black_alpha(120));
            let back = chrome::hold(ui);
            let fr = egui::Frame::none().inner_margin(egui::Margin::symmetric(16.0, 12.0)).show(ui, |ui| {
                ui.set_width(size.x - 32.0);
                ui.set_height(size.y - 24.0);
                // the heading, the pages, the close
                ui.horizontal(|ui| {
                    ui.label(RichText::new("ПУТЕВОДИТЕЛЬ").font(FontId::new(17.0, FontFamily::Name("head".into()))).color(s.desk_ink));
                    ui.add_space(14.0);
                    for (i, name) in PAGES.iter().enumerate() {
                        if chip(ui, &ch, name, self.guide_page == i).clicked() {
                            self.guide_page = i;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if chip(ui, &ch, "Закрыть · Esc", false).clicked() {
                            close = true;
                        }
                    });
                });
                ui.add_space(6.0);
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(1.0, ch.trim.gamma_multiply(0.7)));
                ui.add_space(8.0);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    match self.guide_page {
                        0 => page_screen(ui, &s, &ch, g),
                        1 => page_figures(ui, &s, &ch, g, &mut go),
                        2 => page_map(ui, &s, &ch, g),
                        _ => page_army(ui, &s, &ch, &mut go),
                    }
                });
            });
            let mut v = chrome::Out::new();
            hud::plate(&mut v, fr.response.rect, &ch, 5.0);
            chrome::under(ui, back, v);
        });
        if let Some((tab, page)) = go {
            self.tab = Some(tab);
            set_page(ctx, tab, page);
            close = true;
        }
        if close {
            self.guide_open = false;
        }
    }

    /// The small «?» that opens the guide (at peace the only thing of the briefing's corner).
    pub(super) fn guide_chip(&mut self, ctx: &egui::Context) {
        if self.guide_open || self.tab.is_some() || self.menu {
            return;
        }
        let s = self.skin;
        let ch = self.chrome.clone();
        egui::Area::new(egui::Id::new("guide-chip")).anchor(Align2::LEFT_TOP, [12.0, 76.0]).order(egui::Order::Middle).show(ctx, |ui| {
            let (r, resp) = ui.allocate_exact_size(vec2(170.0, 26.0), Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            let mut v = chrome::Out::new();
            hud::plate(&mut v, r, &ch, 4.0);
            if resp.hovered() {
                v.push(Shape::rect_filled(r, 4.0, Color32::from_white_alpha(14)));
            }
            ui.painter().extend(v);
            ui.painter().text(r.center(), Align2::CENTER_CENTER, "?  Путеводитель · F1", FontId::new(13.5, FontFamily::Name("bold".into())), s.desk_ink);
            if resp.tip(|| Card::new("Путеводитель").icon("binoculars").text("Что на экране, что значат показатели и что на них влияет, как читать значки и линии на карте, как вести армию.").note("F1")).clicked() {
                self.guide_open = true;
            }
        });
    }
}

/// A small button on the dark plate: the page tabs and the close.
pub(super) fn chip(ui: &mut egui::Ui, ch: &chrome::Chrome, text: &str, on: bool) -> egui::Response {
    let font = FontId::new(13.5, FontFamily::Name("bold".into()));
    let g = ui.painter().layout_no_wrap(text.to_string(), font, Color32::from_rgb(236, 228, 206));
    let (r, resp) = ui.allocate_exact_size(g.size() + vec2(20.0, 10.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let p = ui.painter();
    let fill = if on { hud::darken(ch.trim, 0.55) } else if resp.hovered() { Color32::from_rgb(52, 47, 40) } else { Color32::from_rgb(34, 31, 26) };
    p.rect_filled(r, 3.0, fill);
    p.rect_stroke(r, 3.0, Stroke::new(if on { 1.8 } else { 1.0 }, if on { hud::lighten(ch.trim, 0.2) } else { ch.trim.gamma_multiply(0.7) }));
    p.galley(r.center() - g.size() / 2.0, g, Color32::from_rgb(236, 228, 206));
    resp
}

/// A paragraph in the window's ink.
fn words(ui: &mut egui::Ui, s: &Skin, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).size(14.0).color(s.desk_ink)).wrap());
}

fn head(ui: &mut egui::Ui, ch: &chrome::Chrome, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text).font(FontId::new(14.5, FontFamily::Name("head".into()))).color(hud::lighten(ch.trim, 0.3)));
}

/// The plan of the window: a drawing with numbered places and what each is.
fn page_screen(ui: &mut egui::Ui, s: &Skin, ch: &chrome::Chrome, g: &Game) {
    words(ui, s, "Куда смотреть: сверху — как живёт страна, справа — что важно сейчас, внизу — армии и карта. Цифры на чертеже — те же, что в списке.");
    let w = ui.available_width().min(560.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, w * 0.54), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, 3.0, Color32::from_rgb(26, 30, 28));
    p.rect_stroke(r, 3.0, Stroke::new(1.5, ch.trim));
    let at = |fx: f32, fy: f32, fw: f32, fh: f32| Rect::from_min_size(r.min + vec2(fx * r.width(), fy * r.height()), vec2(fw * r.width(), fh * r.height()));
    let places = [
        (at(0.0, 0.0, 1.0, 0.09), "1", Color32::from_rgb(214, 188, 120)),
        (at(0.02, 0.12, 0.27, 0.30), "2", Color32::from_rgb(244, 140, 110)),
        (at(0.70, 0.12, 0.28, 0.26), "3", Color32::from_rgb(236, 170, 60)),
        (at(0.0, 0.88, 0.62, 0.12), "4", Color32::from_rgb(160, 210, 170)),
        (at(0.72, 0.56, 0.26, 0.28), "5", Color32::from_rgb(132, 206, 214)),
        (at(0.64, 0.88, 0.36, 0.12), "6", Color32::from_rgb(196, 160, 232)),
    ];
    // (the map's land: a few provinces' lines, a front, so that the drawing reads as a map)
    for (i, (x, y)) in [(0.35, 0.3), (0.5, 0.5), (0.42, 0.68), (0.58, 0.25)].iter().enumerate() {
        let c = r.min + vec2(x * r.width(), y * r.height());
        p.circle_stroke(c, 24.0 + 6.0 * i as f32, Stroke::new(1.0, Color32::from_white_alpha(28)));
    }
    p.line_segment([r.min + vec2(0.42 * r.width(), 0.14 * r.height()), r.min + vec2(0.46 * r.width(), 0.84 * r.height())], Stroke::new(3.0, Color32::from_rgb(214, 52, 40)));
    for (rc, n, col) in places {
        p.rect_filled(rc, 2.0, col.gamma_multiply(0.22));
        p.rect_stroke(rc, 2.0, Stroke::new(1.4, col));
        let c = rc.center();
        p.circle_filled(c, 11.0, Color32::from_rgb(30, 26, 20));
        p.circle_stroke(c, 11.0, Stroke::new(1.5, col));
        p.text(c, Align2::CENTER_CENTER, n, FontId::new(14.0, FontFamily::Name("bold".into())), col);
    }
    ui.add_space(4.0);
    let modes: Vec<&str> = map::Mode::ALL.iter().map(|m| m.1).collect();
    for (n, what) in [
        ("1", "Верхняя полоса: слева папки (страна, экономика, армия…), в середине — как живёт страна (народ, казна, власть, снабжение), справа время: пауза, скорость, меню. Щёлкните показатель — откроется папка, где его меняют."),
        ("2", "Карточка «Война» (только в войне): кто с кем, сколько дивизий у каждой стороны, что делать сейчас. Щелчок по заголовку сворачивает её."),
        ("3", "Доклады: что важно сейчас, почему (в числах) и где это исправить. Щелчок ведёт к нужному рычагу и подсвечивает его."),
        ("4", "Внизу — армии: карточка с генералом и приказом на каждую. Щёлкните карточку (или клавишу 1…9) — над ней появятся приказы. Правее — резерв ставки и флот."),
        ("5", "Малая карта мира: рамка — где вы смотрите; щелчок переносит туда."),
        ("6", "Режимы карты (кнопки внизу справа): по ним карта показывает разное. Всего режимов — "),
    ] {
        ui.horizontal_top(|ui| {
            let (rc, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
            ui.painter().circle_stroke(rc.center(), 10.0, Stroke::new(1.5, ch.trim));
            ui.painter().text(rc.center(), Align2::CENTER_CENTER, n, FontId::new(13.0, FontFamily::Name("bold".into())), s.desk_ink);
            let text = if n == "6" { format!("{what}{}: {}.", modes.len(), modes.join(", ").to_lowercase()) } else { what.to_string() };
            words(ui, s, &text);
        });
    }
    let _ = g;
}

/// The figures of the country: what each is, what moves it, where it is changed.
fn page_figures(ui: &mut egui::Ui, s: &Skin, ch: &chrome::Chrome, g: &Game, go: &mut Option<(Tab, usize)>) {
    words(ui, s, "Четыре числа в верхней полосе говорят, как идут дела. Каждое можно навести мышью — там числа и подробности; щёлкнуть — попасть туда, где его меняют.");
    let note = |k: &str| terms::note(k).map_or("", |n| n.1);
    let rows: [(&str, &str, &str, String, &str, Tab, usize); 5] = [
        ("people", "Народ", "Насколько люди довольны жизнью.", format!("{} Растёт от нехватки нужного (хлеба, одежды, топлива) и особенно от голода.", note("discontent")), "Общество", Tab::Society, 0),
        ("coins", "Казна", "Деньги государства: доходы минус расходы за месяц.", "Растёт с налогом и выпуском хозяйства; тает от трат, особенно на армию (в войне — от её доли трат). Минус — долги и займы.".to_string(), "Экономика (налог, стройка), Сводка (бюджет с числами)", Tab::Econ, 0),
        ("crown", "Власть", "Устойчивость правительства.", note("stability").to_string(), "Политика", Tab::Politics, 0),
        ("crate", "Снабжение", "Сколько нужного доходит до войск (только в войне).", note("supply").to_string(), "Армия, страница «Снабжение»: склады, железные дороги", Tab::Army, 2),
        ("alarm", "В мире", "Напряжённость между странами (в мире без вашей войны).", "Растёт, когда кто-то готовит войну. Цель войны — полгода подготовки, и мир это видит.".to_string(), "Политика", Tab::Politics, 0),
    ];
    let icons = chrome::icons(ui.ctx());
    for (icon, name, what, how, wheres, tab, page) in rows {
        let back = chrome::hold(ui);
        let fr = egui::Frame::none().inner_margin(egui::Margin::symmetric(10.0, 8.0)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rc, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
                look::icon(&icons, icon, 28.0, Color32::from_rgb(236, 198, 120)).paint_at(ui, rc);
                ui.vertical(|ui| {
                    ui.label(RichText::new(name).font(FontId::new(15.0, FontFamily::Name("bold".into()))).color(s.desk_ink));
                    words(ui, s, what);
                });
            });
            ui.horizontal_top(|ui| {
                ui.label(RichText::new("От чего зависит:").size(13.5).color(hud::lighten(ch.trim, 0.3)));
                words(ui, s, &how);
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Где менять:").size(13.5).color(hud::lighten(ch.trim, 0.3)));
                ui.label(RichText::new(wheres).size(13.5).color(s.desk_ink));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if chip(ui, ch, "Открыть", false).clicked() {
                        *go = Some((tab, page));
                    }
                });
            });
        });
        let mut v = chrome::Out::new();
        v.push(Shape::rect_filled(fr.response.rect, 3.0, Color32::from_black_alpha(70)));
        v.push(Shape::rect_stroke(fr.response.rect, 3.0, Stroke::new(1.0, ch.trim.gamma_multiply(0.5))));
        chrome::under(ui, back, v);
    }
    head(ui, ch, "Где искать цифры");
    words(ui, s, "Папка «Сводка» — бюджет казны с числами, графики по месяцам и откуда берётся каждый множитель. «События» — вести страны и летопись мира. Подсказки на любом слове с подчёркиванием объясняют само слово.");
    ui.horizontal_wrapped(|ui| {
        if chip(ui, ch, "Открыть «Сводку»: бюджет и графики", false).clicked() {
            *go = Some((Tab::Ledger, 0));
        }
        if chip(ui, ch, "Открыть «События»", false).clicked() {
            *go = Some((Tab::News, 0));
        }
        if chip(ui, ch, "Деятели: кто есть кто", false).clicked() {
            *go = Some((Tab::People, 0));
        }
    });
    let _ = g;
}

/// The counters and lines of the map, drawn as they are drawn on it.
fn page_map(ui: &mut egui::Ui, s: &Skin, ch: &chrome::Chrome, g: &Game) {
    let th = &g.h.war.th;
    let me = g.me;
    let col = |side: usize| {
        let c = g.countries[th.countries[side]].1;
        Color32::from_rgb(c[0], c[1], c[2])
    };
    words(ui, s, "Значок на карте — стопка дивизий в провинции. Цвет пластины — страна; у врага ещё красная рамка. Вблизи над значком стоит модель войск (чем больше стопка, тем больше фигур).");
    let sample = |ui: &mut egui::Ui, side: usize, o: &dyn Fn(Counter<'static>) -> Counter<'static>, text: &str| {
        ui.horizontal(|ui| {
            let (rc, _) = ui.allocate_exact_size(vec2(130.0, 40.0), Sense::hover());
            let base = Counter { n: 4.0, fill: col(side), sym: Some("infantry"), big: false, fog: false, hue: None, edge: None, order: None, chosen: false, supply: None, dug: None, country: None, bars: None, marks: 0, foe: side != me };
            counter(ui.painter(), rc.center() - vec2(0.0, 4.0), &o(base));
            words(ui, s, text);
        });
    };
    head(ui, ch, "Значки");
    sample(ui, me, &|c| c, "Наши: пластина цвета вашей страны, значок рода войск, число дивизий.");
    sample(ui, 1 - me, &|c| c, "Враг: красная рамка. Видно, когда он рядом с нашими дивизиями.");
    sample(ui, 1 - me, &|c| Counter { fog: true, n: 7.0, ..c }, "≈ и пунктирная рамка — враг виден сквозь туман: число приблизительное.");
    sample(ui, me, &|c| Counter { chosen: true, ..c }, "Золотая рамка — выбрано вами.");
    sample(ui, me, &|c| Counter { country: Some(th.countries[me]), bars: Some((0.8, 0.5)), ..c }, "С флагом и полосками — выбранная стопка, стопка под мышью, в бою или в беде. Зелёная полоска — численность, жёлтая — организация.");
    head(ui, ch, "Отметки на значке");
    sample(ui, me, &|c| Counter { marks: MARCH, ..c }, "›› — на марше.");
    sample(ui, me, &|c| Counter { marks: ATTACK, ..c }, "Красная рамка — наступает.");
    sample(ui, me, &|c| Counter { marks: HUNGRY, ..c }, "Красный «!» в углу — больше половины дивизий без снабжения.");
    sample(ui, me, &|c| Counter { marks: SHORT, ..c }, "Жёлтый треугольник — не хватает снаряжения.");
    sample(ui, me, &|c| Counter { marks: OWN, ..c }, "Золотая точка — стопку послали вы сами: генерал её не трогает.");
    head(ui, ch, "Линии и стрелки");
    let line = |ui: &mut egui::Ui, text: &str, f: &dyn Fn(&egui::Painter, Rect)| {
        ui.horizontal(|ui| {
            let (rc, _) = ui.allocate_exact_size(vec2(130.0, 30.0), Sense::hover());
            f(ui.painter(), rc);
            words(ui, s, text);
        });
    };
    line(ui, "Фронт — где армии соприкасаются: красная линия — со стороны врага, тонкая зелёная — с нашей.", &|p, rc| {
        let (a, b) = (rc.left_center() + vec2(8.0, 0.0), rc.right_center() - vec2(8.0, 0.0));
        p.line_segment([a, b], Stroke::new(8.0, Color32::from_black_alpha(150)));
        p.line_segment([a + vec2(0.0, -2.0), b + vec2(0.0, -2.0)], Stroke::new(3.0, Color32::from_rgb(214, 52, 40)));
        p.line_segment([a + vec2(0.0, 2.0), b + vec2(0.0, 2.0)], Stroke::new(1.8, Color32::from_rgb(110, 176, 86)));
    });
    line(ui, "Широкая стрела цвета армии — приказ наступать (A). Красная стрела — идущая атака; тонкая жёлтая — удар, который генштаб готовит на следующий месяц.", &|p, rc| {
        map::troops_arrow(p, rc.left_center() + vec2(8.0, 0.0), rc.right_center() - vec2(8.0, 0.0), 8.0, Color32::from_rgb(232, 184, 48), 0.18, 0.0);
    });
    line(ui, "Плашка со скрещёнными мечами — идёт бой; полоска под ней — перевес: зелёная — наш, красная — врага.", &|p, rc| {
        let plate = Rect::from_center_size(rc.center(), vec2(24.0, 19.0));
        p.rect_filled(plate.expand(1.5), 3.0, Color32::from_black_alpha(200));
        p.rect_filled(plate, 3.0, Color32::from_rgb(40, 34, 28));
        p.rect_stroke(plate, 3.0, Stroke::new(1.5, Color32::from_rgb(255, 214, 64)));
        let bar = Rect::from_min_size(pos2(plate.min.x - 7.0, plate.max.y + 3.0), vec2(plate.width() + 14.0, 5.0));
        p.rect_filled(bar, 1.0, Color32::from_rgb(210, 60, 46));
        p.rect_filled(Rect::from_min_max(bar.min, pos2(bar.min.x + bar.width() * 0.62, bar.max.y)), 1.0, Color32::from_rgb(110, 190, 80));
    });
    head(ui, ch, "Цвет земли");
    words(ui, s, "В обычном режиме земля окрашена цветом страны; захваченная врагом провинция — полосами цвета прежнего хозяина. Тёмная дымка над землёй врага — туман войны: там вы его не видите. Другие режимы (кнопки справа внизу) красят карту по снабжению, развитию, людям, вере и так далее.");
}

/// How an army is led: the steps and where the other things are.
fn page_army(ui: &mut egui::Ui, s: &Skin, ch: &chrome::Chrome, go: &mut Option<(Tab, usize)>) {
    words(ui, s, "Армия — это генерал и его дивизии (хорошо ведёт до 24). Дивизии стоят в провинциях на карте; армии стоят внизу карточками.");
    let step = |ui: &mut egui::Ui, n: &str, title: &str, text: &str| {
        ui.horizontal_top(|ui| {
            let (rc, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
            ui.painter().circle_filled(rc.center(), 11.0, Color32::from_rgb(30, 26, 20));
            ui.painter().circle_stroke(rc.center(), 11.0, Stroke::new(1.5, ch.trim));
            ui.painter().text(rc.center(), Align2::CENTER_CENTER, n, FontId::new(13.0, FontFamily::Name("bold".into())), s.desk_ink);
            ui.vertical(|ui| {
                ui.label(RichText::new(title).font(FontId::new(14.5, FontFamily::Name("bold".into()))).color(s.desk_ink));
                words(ui, s, text);
            });
        });
    };
    step(ui, "1", "Выберите армию", "Щёлкните её карточку внизу или нажмите клавишу 1…9 (Shift — добавить ещё одну: приказы пойдут всем).");
    step(ui, "2", "Дайте приказ", "F — протянуть её фронт вдоль границы (или щёлкнуть по соседней стране: весь фронт с ней); A и щелчок по врагу — стрела наступления, Enter — исполнить; D — оборона в глубину; G — отдать генштабу.");
    step(ui, "3", "Или ведите дивизии сами", "Щёлкните стопку на карте (рамкой — несколько, с Shift — добавить). Правая кнопка по карте — идти туда; по врагу — атаковать; H — стоять; N — новая армия из выбранных; Ctrl+цифра — перевести в армию.");
    step(ui, "4", "Новые дивизии", "Собираются у столицы или склада; «Место сбора» переносит знамя в вашу провинцию. R — новые дивизии пойдут в выбранную армию. Что в них входит, решает шаблон дивизии.");
    step(ui, "5", "Генерал", "В папке «Армия», страница «Армии», выберите армию: «Сменить генерала» покажет резерв генералов с их делами, «Снять» оставит армию без командующего. О таланте говорят только дела.");
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if chip(ui, ch, "Папка «Армия»: армии и генералы", false).clicked() {
            *go = Some((Tab::Army, 0));
        }
        if chip(ui, ch, "Шаблоны дивизий", false).clicked() {
            *go = Some((Tab::Army, 1));
        }
        if chip(ui, ch, "Снабжение", false).clicked() {
            *go = Some((Tab::Army, 2));
        }
    });
    head(ui, ch, "Все клавиши");
    let mut sk = *s;
    sk.ink = s.desk_ink;
    command::keys_page(ui, ch, &sk);
}
