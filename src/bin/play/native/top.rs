//! The country's bar on top (the flag, the spheres of the state, how the country lives in
//! words with their icons, the clock) and the banner of the clock under it.

use super::*;

/// The height of the country's bar.
pub(super) const H: f32 = 64.0;

/// The width of a sphere's cell when it carries its name (wide windows).
const CELL: f32 = 50.0;

/// The room the clock and the menu's medallion take at the bar's right end.
const CLOCK: f32 = 300.0;

impl App {
    /// The country's bar: the flag in a frame hanging over the bar's edge and the name; the
    /// spheres as medallions; how the country lives (each an icon in its socket, a word and a
    /// gauge — the numbers on the cards); the clock in a plate of its own, the menu.
    pub(super) fn top(&mut self, ctx: &egui::Context, g: &mut Game) {
        let s = self.skin;
        let ch = self.chrome.clone();
        let screen = ctx.screen_rect();
        // (a narrow window: the words without their headings; narrower still — the icons and
        // gauges alone, the words on their cards)
        let narrow = screen.width() < 1500.0;
        let bare = screen.width() < 1280.0;
        // (the month's figures kept; each word's arrow compares with three months back)
        let me = g.me;
        let now = [g.h.home(me).anger(), g.h.home(me).gdp_real, g.state.stability[me], g.h.fed[me], g.state.tension];
        let month = g.month();
        if self.hist.last().is_some_and(|h| h.0 > month) {
            self.hist.clear();
        }
        // (a game just opened: the months its rows keep — anger and output; the rest as now)
        if self.hist.is_empty() {
            for r in g.rows.iter().rev().take(6).rev().filter(|r| r.month < month) {
                self.hist.push((r.month, [r.anger[me], r.gdp[me], now[2], now[3], now[4]]));
            }
        }
        if self.hist.last().is_none_or(|h| h.0 != month) {
            self.hist.push((month, now));
            if self.hist.len() > 36 {
                self.hist.remove(0);
            }
        }
        let then = self.hist.iter().rev().find(|h| h.0 + 3 <= month).or_else(|| self.hist.first().filter(|h| h.0 < month)).map(|h| h.1);
        let arrow = |i: usize, up_good: bool, eps: f32| -> Option<(bool, bool)> {
            let d = now[i] - then?[i];
            let rel = if i == 1 { d / now[1].abs().max(1e-6) } else { d };
            (rel.abs() > eps).then_some((rel > 0.0, (rel > 0.0) == up_good))
        };
        let trends = [arrow(0, false, 0.05), arrow(1, true, 0.005), arrow(2, true, 0.02), arrow(3, true, 0.02), arrow(4, false, 0.02)];
        if self.tab == Some(Tab::News) || self.news_seen > g.news.len() {
            self.news_seen = g.news.len();
        }
        let unread = g.news[self.news_seen..].iter().any(|n| n.hot);
        let alarms = alerts(g);
        egui::TopBottomPanel::top("top").exact_height(H).frame(egui::Frame::none().fill(s.desk).inner_margin(egui::Margin { left: 0.0, right: 10.0, top: 0.0, bottom: 0.0 })).show(ctx, |ui| {
            let bar = ui.max_rect().expand2(egui::vec2(10.0, 0.0));
            let mut v = chrome::Out::new();
            ch.lay(&mut v, bar, &ch.bar, 0.0);
            v.push(egui::Shape::rect_filled(bar, 0.0, Color32::from_black_alpha(60)));
            hud::gloss(&mut v, bar, Color32::from_white_alpha(30), Color32::from_black_alpha(90));
            v.push(egui::Shape::line_segment([bar.left_top() + egui::vec2(0.0, 0.5), bar.right_top() + egui::vec2(0.0, 0.5)], egui::Stroke::new(1.0, Color32::from_white_alpha(40))));
            hud::band(&mut v, bar.min.x, bar.max.x, bar.max.y - 4.0, &ch);
            ui.painter().extend(v);
            let me = g.me;
            let country = g.h.war.th.countries[me];
            let c = g.countries[country].1;
            let col = Color32::from_rgb(c[0], c[1], c[2]);
            let people = g.h.home(me).people();
            // the flag: in a frame of trim, hanging a little below the bar (drawn over the map)
            // (30th session: the flag in the bar's own middle, the same margin above, below and at the left — it hung
            // over the window's edge and below the bar, the one lopsided thing in a symmetric bar)
            let frame = egui::Rect::from_min_size(egui::pos2(bar.min.x + 10.0 + 12.0, bar.min.y + 4.0), egui::vec2(92.0, H - 12.0));
            {
                let fg = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("top-flag")));
                let mut v = chrome::Out::new();
                // (the shade the bar casts on the map)
                hud::cast(&mut v, egui::Rect::from_min_max(egui::pos2(bar.min.x, bar.max.y), egui::pos2(bar.max.x, bar.max.y + 9.0)), true);
                hud::plate(&mut v, frame, &ch, 3.0);
                fg.extend(v);
                let cloth = frame.shrink2(egui::vec2(8.0, 7.0));
                match flags::get(ctx, country) {
                    Some(f) => flags::paint(ctx, &fg, cloth, &f, ch.trim, Some(&self.icons)),
                    None => {
                        fg.rect_filled(cloth, 1.0, col);
                    }
                }
            }
            ui.horizontal_centered(|ui| {
                let (_, fr) = ui.allocate_exact_size(egui::vec2(122.0, H - 4.0), egui::Sense::hover());
                let fr = ui.interact(frame, egui::Id::new("top-flag-hit"), egui::Sense::hover()).union(fr);
                fr.tip(|| Card::new(g.h.home(me).name.clone()).banner(country, col).sub("ваша страна").row("людей", format!("{people:.0} млн")).row("правительство", g.stability_word(me)).row("в мире", g.tension_word()));
                // (the country's name only when the words of how it lives keep their numbers
                // beside it: the flag says whose country it is; the spheres take 51 points each)
                let name = g.h.home(me).name.to_uppercase();
                let font = egui::FontId::new(25.0, egui::FontFamily::Name("head".into()));
                let gal = ui.painter().layout_no_wrap(name.clone(), font.clone(), s.desk_ink);
                // (as many spheres as the folders shown: science and the navy only when the era has them)
                let count = 9.0 + g.h.labs[me].is_some() as u8 as f32 + (!g.pack.ships.is_empty()) as u8 as f32;
                let spheres = count * 51.0 + 36.0 + 4.0 * 11.0;
                let room = ui.available_width() - (gal.size().x + 6.0) - spheres - CLOCK;
                if !narrow && room >= Self::lives_need(ui, g, s, narrow, bare, false) {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(gal.size().x + 6.0, H - 12.0), egui::Sense::hover());
                    let at = egui::pos2(r.min.x, r.center().y - gal.size().y / 2.0);
                    ui.painter().galley(at, gal, s.desk_ink);
                }
                ui.add_space(8.0);
                let mut v = chrome::Out::new();
                let x = ui.cursor().min.x;
                hud::divider(&mut v, x, bar.min.y + 12.0, bar.max.y - 14.0, &ch);
                ui.painter().extend(v);
                ui.add_space(10.0);
                // (the spheres' medallions shrink to leave the words of how it lives and the
                // clock their room)
                // (smaller first, so that the words keep their numbers; if even that is not enough,
                // as small as the words alone need)
                // (a margin for the spacing between the parts of the bar: with eleven spheres the
                // numbers missed their room by a few points)
                let radius = |lean: bool| ((ui.available_width() - 18.0 - 30.0 - 4.0 * 11.0 - Self::lives_need(ui, g, s, narrow, bare, lean) - CLOCK) / count - 3.0 - 4.0) / 2.0;
                // (with eleven spheres the medallions give a little more of their size to the numbers)
                let r = if radius(false) >= 13.5 { radius(false) } else { radius(true) }.clamp(13.0, 22.0);
                // (30th session: on a wide window every sphere carries its name under it — eleven round
                // icons with only a hint on the hand were a guessing game; a little smaller to make room)
                let caps = screen.width() >= 1760.0 && radius(false) >= (CELL + 3.0 - 7.0) / 2.0;
                let r = if caps { r.min(16.0) } else { r };
                self.spheres(ui, ctx, g, &ch, s, &alarms, unread, r, caps);
                ui.add_space(8.0);
                let mut v = chrome::Out::new();
                let x = ui.cursor().min.x;
                hud::divider(&mut v, x, bar.min.y + 12.0, bar.max.y - 14.0, &ch);
                ui.painter().extend(v);
                ui.add_space(10.0);
                // (a word of the bar clicked: its folder, at its page)
                if let Some((t, k)) = self.lives(ui, g, &ch, s, narrow, bare, &trends, people) {
                    self.tab = Some(t);
                    set_page(ctx, t, k);
                }
                self.clock(ui, ctx, g, &ch, s, month);
            });
        });
        // (28th session: the reports with their causes, at the right, in place of the round alerts)
        let _ = &alarms;
        self.reports_column(ctx, g);
    }

    /// the spheres
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    fn spheres(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, g: &Game, ch: &chrome::Chrome, s: Skin, alarms: &[Alert], unread: bool, r: f32, caps: bool) {
        let me = g.me;
        let tabs: Vec<(Tab, &str, &str, &str, Color32)> = [
            (Tab::Nation, "flag", "Страна", "Какая она: её особенности из прошлого и настоящего, хозяйственный строй, государственные программы, займы.", Color32::from_rgb(214, 188, 120)),
            (Tab::Econ, "econ", "Экономика", "Налоги, стройка, кто правит хозяйством.", Color32::from_rgb(232, 196, 104)),
            (Tab::Arms, "gears", "Производство", "Что делают заводы: образцы снаряжения, навык заводов, новые разработки, что делать больше.", Color32::from_rgb(196, 176, 150)),
            (Tab::Science, "science", "Наука", "Сколько тратить на науку и кто её ведёт; разработки.", Color32::from_rgb(132, 206, 214)),
            (Tab::Army, "army", "Армия", "Состав армии, дивизии и их шаблоны, склады и линии снабжения, фронт и группы армий.", Color32::from_rgb(226, 128, 104)),
            (Tab::Navy, "anchor", "Флот", "Эскадры и их задачи, верфи, морская торговля и что ей грозит, бои на море.", Color32::from_rgb(110, 160, 214)),
            (Tab::Politics, "castle", "Политика", "Правительство, соседи, цель войны, война и мир, трудные решения.", Color32::from_rgb(196, 160, 232)),
            (Tab::Society, "faith", "Общество", "Строй и законность, силы общества и их настроения, законы и реформы, движения, вера, великая задача страны.", Color32::from_rgb(222, 170, 120)),
            (Tab::People, "people", "Деятели", "Люди страны и соседа: правители, генералы, учёные, экономисты — их жизни, дела и связи.", Color32::from_rgb(206, 180, 150)),
            (Tab::Ledger, "chart", "Сводка", "Бюджет казны с числами, графики по месяцам, откуда берётся каждый множитель, прогноз решений на год.", Color32::from_rgb(160, 210, 170)),
            (Tab::News, "news", "События", "Вести страны и летопись мира.", Color32::from_rgb(222, 214, 190)),
        ]
        .into_iter()
        .filter(|t| t.0 != Tab::Science || g.h.labs[me].is_some())
        .filter(|t| t.0 != Tab::Navy || !g.pack.ships.is_empty())
        .collect();
        ui.spacing_mut().item_spacing.x = 3.0;
        // (29th session: the spheres in groups — the country; the economy; the war; the state's
        // people and neighbours; the reports — a thin line between them, so that eleven discs read
        // as five things)
        let group = |t: Tab| match t {
            Tab::Nation => 0,
            Tab::Econ | Tab::Arms | Tab::Science => 1,
            Tab::Army | Tab::Navy => 2,
            Tab::Politics | Tab::Society | Tab::People => 3,
            _ => 4,
        };
        let mut last_group = None;
        for (tab, icon, name, what, hue) in tabs {
            if last_group.is_some_and(|k| k != group(tab)) {
                let bar = ui.max_rect();
                ui.add_space(4.0);
                let x = ui.cursor().min.x;
                let mut v = chrome::Out::new();
                v.push(egui::Shape::line_segment([egui::pos2(x, bar.min.y + 16.0), egui::pos2(x, bar.max.y - 16.0)], egui::Stroke::new(1.0, ch.trim.gamma_multiply(0.55))));
                ui.painter().extend(v);
                ui.add_space(4.0);
            }
            last_group = Some(group(tab));
            let on = self.tab == Some(tab);
            let b = if caps {
                // (a cell: the disc over its name, the pair in the bar's middle; the name a word that fits)
                let short = if tab == Tab::Arms { "Заводы" } else { name };
                let ink = if on { s.desk_ink } else { s.desk_ink.gamma_multiply(0.74) };
                let mut got = None;
                ui.allocate_ui_with_layout(egui::vec2(CELL, 2.0 * r + 4.0 + 12.0), egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let m = hud::medallion(ui, &ch, &s, icon, r, hue, on, ("sphere", icon));
                    let (cr, _) = ui.allocate_exact_size(egui::vec2(CELL, 12.0), egui::Sense::hover());
                    // (the font shrinks to the cell: the other eras' letters are wider, and ЭКОНОМИКА ran into ЗАВОДЫ)
                    let word = short.to_uppercase();
                    let wide = ui.painter().layout_no_wrap(word.clone(), egui::FontId::new(10.5, egui::FontFamily::Name("head".into())), ink).size().x;
                    let size = (10.5 * (CELL - 3.0) / wide.max(1.0)).min(10.5);
                    ui.painter().text(egui::pos2(cr.center().x, cr.min.y), egui::Align2::CENTER_TOP, word, egui::FontId::new(size, egui::FontFamily::Name("head".into())), ink);
                    got = Some(m);
                });
                got.expect("the disc is drawn")
            } else {
                hud::medallion(ui, &ch, &s, icon, r, hue, on, ("sphere", icon))
            };
            chrome::spot(ctx, "sphere", b.rect);
            // (a mark on the sphere when something there wants the eye)
            let worst = alarms.iter().filter(|a| a.tab == tab).map(|a| a.tone).max();
            let mark = if tab == Tab::News && unread { Some(1) } else { worst.filter(|&t| t > 0) };
            if let Some(t) = mark {
                hud::badge(ctx, b.rect.right_top() + egui::vec2(-6.0, 6.0), if t >= 2 { Color32::from_rgb(214, 64, 48) } else { Color32::from_rgb(226, 150, 40) });
            }
            if b.tip(|| Card::new(name).icon(icon).text(what).note(if on { "папка открыта — щёлкните, чтобы закрыть" } else { "щёлкните — откроется папка слева" })).clicked() {
                self.tab = if on { None } else { Some(tab) };
            }
        }
    }

    /// how the country lives: an icon in a socket, a word under its small heading, a
    /// gauge of five pips (how well it goes)
    #[allow(clippy::too_many_arguments)]
    fn lives(&self, ui: &mut egui::Ui, g: &Game, ch: &chrome::Chrome, s: Skin, narrow: bool, bare: bool, trends: &[Option<(bool, bool)>; 5], people: f32) -> Option<(Tab, usize)> {
        let me = g.me;
        let n = g.h.home(me).clone();
        let n = &n;
        let mood = n.anger();
        // the words and their numbers; when with their numbers they would crowd the clock (about
        // 300 points at the right), the numbers go to the cards
        let mood_word = match mood {
            a if a < 0.5 => "спокоен",
            a if a < 1.5 => "ворчит",
            _ => "зол",
        };
        let growth = n.gdp_real / g.gdp0[me].max(1e-9) - 1.0;
        let econ_word = if growth > 0.05 { "растёт" } else if growth < -0.05 { "хиреет" } else { "держится" };
        // (the treasury, as HOI4's resources: what is in it and how the month ended — 28th session,
        // in place of the economy's growth since the start, which is on its card)
        let b = g.rows.last().map(|r| r.book[me]).unwrap_or_default();
        let (tre, net, spent) = (n.treasury, b.income() - b.spent(), b.spent());
        let reserve = if spent > 1e-6 { tre / spent } else { 12.0 };
        let tre_trend = g.rows.iter().rev().nth(3).map(|r| r.book[me].treasury).and_then(|t0| {
            let d = tre - t0;
            (d.abs() > 0.02 * tre.abs().max(0.1)).then_some((d > 0.0, d > 0.0))
        });
        let st = g.state.stability[me];
        let power = g.stability_word(me);
        let fed = g.h.fed[me];
        let supply_word = if fed > 0.9 { "в порядке" } else if fed > 0.6 { if narrow { "мало" } else { "не хватает" } } else if narrow { "голод" } else { "голод в войсках" };
        let tension = g.state.tension;
        let tw = g.tension_word();
        let full = Self::lives_words(g, narrow);
        let lean = ui.available_width() - CLOCK < Self::lives_need(ui, g, s, narrow, bare, false);
        let text_of = |head: &str| full.iter().find(|x| x.0 == head).map_or(String::new(), |x| x.1.clone());
        let mut open: Option<(Tab, usize)> = None;
        let warn = Color32::from_rgb(240, 196, 92);
        let bad = Color32::from_rgb(255, 140, 110);
        let good = Color32::from_rgb(150, 210, 110);
        let word = |ui: &mut egui::Ui, icon: &str, hue: Color32, head: &str, text: &str, state: u8, level: f32, trend: Option<(bool, bool)>, card: &dyn Fn() -> Card| {
            let colour = match state {
                0 => s.desk_ink,
                1 => warn,
                _ => bad,
            };
            let gauge = match state {
                0 => good,
                1 => warn,
                _ => bad,
            };
            let text = if spoiled() { format!("{text} {text} {text} {text}") } else { text.to_string() };
            let hg = ui.painter().layout_no_wrap(head.to_uppercase(), egui::FontId::new(12.0, egui::FontFamily::Name("head".into())), s.desk_ink.gamma_multiply(0.8));
            let wg = ui.painter().layout_no_wrap(text, egui::FontId::proportional(15.5), colour);
            let gw: f32 = 5.0 * 8.0 + 4.0 * 2.0 + 14.0;
            let tw = if bare { gw.max(0.0) } else if narrow { wg.size().x.max(gw) } else { hg.size().x.max(wg.size().x).max(gw) };
            let (r, resp) = ui.allocate_exact_size(egui::vec2(36.0 + 7.0 + tw, H - 12.0), egui::Sense::click());
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            let c = egui::pos2(r.min.x + 18.0, r.center().y - 1.0);
            let mut v = chrome::Out::new();
            v.push(egui::Shape::circle_filled(c + egui::vec2(0.0, 1.5), 17.0, Color32::from_black_alpha(110)));
            hud::ring(&mut v, c, 16.5, 2.5, ch.trim);
            hud::disc(&mut v, c, 14.0, Color32::from_rgb(54, 50, 42), Color32::from_rgb(14, 12, 10));
            let x = r.min.x + 43.0;
            let gy = if bare { r.center().y - 3.0 } else { r.max.y - 9.0 };
            let pr = hud::pips(&mut v, egui::pos2(x, gy), 5, level, gauge, 8.0, 5.0);
            // (the arrow: which way it went over three months — green if for the better)
            if let Some((up, better)) = trend {
                hud::arrow(&mut v, egui::pos2(pr.max.x + 8.0, pr.center().y), up, if better { good } else { bad });
            }
            ui.painter().extend(v);
            let icons = chrome::icons(ui.ctx());
            look::icon(&icons, icon, 20.0, Color32::from_black_alpha(200)).paint_at(ui, egui::Rect::from_center_size(c + egui::vec2(1.0, 1.5), egui::vec2(20.0, 20.0)));
            look::icon(&icons, icon, 20.0, if state == 0 { hue } else { colour }).paint_at(ui, egui::Rect::from_center_size(c, egui::vec2(20.0, 20.0)));
            if bare {
            } else if narrow {
                ui.painter().galley(egui::pos2(x, gy - 4.0 - wg.size().y), wg, colour);
            } else {
                let hh = hg.size().y;
                let y0 = gy - 2.0 - wg.size().y - hh;
                ui.painter().galley(egui::pos2(x, y0), hg, s.desk_ink);
                ui.painter().galley(egui::pos2(x, y0 + hh), wg, colour);
            }
            chrome::spot(ui.ctx(), "word", resp.rect);
            let resp = resp.tip(|| match trend {
                Some((_, better)) => card().tone("за три месяца", if better { "лучше" } else { "хуже" }, if better { chrome::Tone::Good } else { chrome::Tone::Bad }),
                None => card().row("за три месяца", "без перемен"),
            });
            ui.add_space(10.0);
            resp.clicked()
        };
        let mood_text = if spoiled() || lean { mood_word.to_string() } else { text_of("народ") };
        if word(ui, "people", Color32::from_rgb(226, 206, 160), "народ", &mood_text, if mood < 0.5 { 0 } else if mood < 1.5 { 1 } else { 2 }, (5.0 - mood * 1.7).clamp(0.3, 5.0), trends[0], &|| {
            let (t, x) = terms::note("discontent").unwrap_or(("", ""));
            Card::new("Народ").icon("people").sub(mood_word).tone("недовольство", format!("{mood:.2}"), if mood < 0.5 { chrome::Tone::Good } else if mood < 1.5 { chrome::Tone::Plain } else { chrome::Tone::Bad }).row("людей", format!("{people:.0} млн")).head(t).text(x).note("щёлкните — общество")
        }) {
            open = Some((Tab::Society, 0));
        }
        let econ_text = if spoiled() || lean { super::charts::num(tre) } else { text_of("казна") };
        // (a narrow window at war: the supply's word instead of the treasury's)
        let _ = trends[1];
        if !(g.h.at_war && narrow)
            && word(ui, "coins", Color32::from_rgb(236, 198, 92), "казна", &econ_text, if tre < 0.0 { 2 } else if net < 0.0 && reserve < 3.0 { 1 } else { 0 }, (reserve / 2.0).clamp(0.3, 5.0), tre_trend, &|| {
                Card::new("Казна")
                    .icon("coins")
                    .tone("в казне", super::charts::num(tre), if tre < 0.0 { chrome::Tone::Bad } else { chrome::Tone::Plain })
                    .row("доходы за месяц", super::charts::num(b.income()))
                    .row("расходы за месяц", super::charts::num(spent))
                    .tone("итог месяца", format!("{}{}", if net >= 0.0 { "+" } else { "" }, super::charts::num(net)), if net >= 0.0 { chrome::Tone::Good } else { chrome::Tone::Bad })
                    .row("налог", format!("{:.0}%", n.tax * 100.0))
                    .row("армии из трат", format!("{:.0}%", n.mil_share * 100.0))
                    .tone(format!("выпуск к началу игры ({econ_word})"), format!("{:+.0}%", growth * 100.0), if growth > 0.0 { chrome::Tone::Good } else { chrome::Tone::Bad })
                    .note("щёлкните — бюджет: каждая статья и её рычаг")
            })
        {
            open = Some((Tab::Econ, panels::economy::BUDGET));
        }
        let power_text = if spoiled() || lean { power.to_string() } else { text_of("власть") };
        if word(ui, "crown", Color32::from_rgb(214, 178, 236), "власть", &power_text, if st < crate::world::statecraft::SHAKY { 2 } else if st < 0.5 { 1 } else { 0 }, st * 5.0, trends[2], &|| {
            let (t, x) = terms::note("stability").unwrap_or(("", ""));
            Card::new("Власть").icon("crown").sub(power).tone("устойчивость", format!("{st:.2}"), if st < crate::world::statecraft::SHAKY { chrome::Tone::Bad } else { chrome::Tone::Plain }).row("у соседа", g.stability_word(1 - me)).head(t).text(x).note("щёлкните — политика")
        }) {
            open = Some((Tab::Politics, 0));
        }
        if g.h.at_war {
            // (a narrow window has room only for a short word)
            let w = supply_word;
            let w_text = if spoiled() || lean { w.to_string() } else { text_of("снабжение") };
            if word(ui, "crate", Color32::from_rgb(170, 214, 120), "снабжение", &w_text, if fed > 0.9 { 0 } else if fed > 0.6 { 1 } else { 2 }, fed * 5.0, trends[3], &|| {
                let (t, x) = terms::note("supply").unwrap_or(("", ""));
                Card::new("Снабжение").icon("crate").sub(w).tone("доходит до войск", format!("{:.0}%", fed * 100.0), if fed > 0.9 { chrome::Tone::Good } else { chrome::Tone::Bad }).head(t).text(x).note("щёлкните — снабжение армии")
            }) {
                open = Some((Tab::Army, 2));
            }
        } else if !narrow {
            let tw_text = if spoiled() || lean { tw.to_string() } else { text_of("в мире") };
            if word(ui, "alarm", Color32::from_rgb(232, 168, 110), "в мире", &tw_text, if tension < 0.4 { 0 } else if tension < 0.7 { 1 } else { 2 }, (1.0 - tension) * 5.0, trends[4], &|| Card::new("Напряжённость").icon("alarm").sub(tw).row("в мире", format!("{tension:.2}")).note("растёт, когда кто-то готовит войну; щёлкните — политика")) {
                open = Some((Tab::Politics, 0));
            }
        }
        open
    }

    /// The words of how the country lives with their numbers (heading, words).
    fn lives_words(g: &Game, narrow: bool) -> Vec<(&'static str, String)> {
        let me = g.me;
        let n = g.h.home(me);
        let mood = n.anger();
        let mood_word = match mood {
            a if a < 0.5 => "спокоен",
            a if a < 1.5 => "ворчит",
            _ => "зол",
        };
        let b = g.rows.last().map(|r| r.book[me]).unwrap_or_default();
        let net = b.income() - b.spent();
        let st = g.state.stability[me];
        let fed = g.h.fed[me];
        let supply_word = if fed > 0.9 { "в порядке" } else if fed > 0.6 { if narrow { "мало" } else { "не хватает" } } else if narrow { "голод" } else { "голод в войсках" };
        let mut v = vec![("народ", format!("{mood_word} · {:.1}", mood).replace('.', ","))];
        if !(g.h.at_war && narrow) {
            v.push(("казна", format!("{} · {}{}", super::charts::num(n.treasury), if net >= 0.0 { "+" } else { "" }, super::charts::num(net))));
        }
        v.push(("власть", format!("{} · {:.0}", g.stability_word(me), st * 100.0)));
        if g.h.at_war {
            v.push(("снабжение", format!("{supply_word} · {:.0}%", fed * 100.0)));
        } else if !narrow {
            v.push(("в мире", format!("{} · {:.0}", g.tension_word(), g.state.tension * 100.0)));
        }
        v
    }

    /// The room the words of how the country lives take, with their numbers or (`lean`) without
    /// (points).
    fn lives_need(ui: &egui::Ui, g: &Game, s: Skin, narrow: bool, bare: bool, lean: bool) -> f32 {
        Self::lives_words(g, narrow)
            .iter()
            .map(|(head, text)| {
                let text = if lean { text.split(" · ").next().unwrap_or("").to_string() } else { text.clone() };
                let hw = ui.painter().layout_no_wrap(head.to_uppercase(), egui::FontId::new(12.0, egui::FontFamily::Name("head".into())), s.desk_ink).size().x;
                let ww = ui.painter().layout_no_wrap(text, egui::FontId::proportional(15.5), s.desk_ink).size().x;
                let gw: f32 = 5.0 * 8.0 + 4.0 * 2.0 + 14.0;
                36.0 + 7.0 + if bare { gw } else if narrow { ww.max(gw) } else { hw.max(ww).max(gw) } + 10.0
            })
            .sum()
    }

    /// the clock, on the right: the menu, the date over the speeds in a plate of its
    /// own, the medallion of time
    fn clock(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, g: &mut Game, ch: &chrome::Chrome, s: Skin, month: u32) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if hud::medallion(ui, &ch, &s, "scroll", 19.0, Color32::from_rgb(222, 214, 190), self.menu, "menu").tip(|| Card::new("Меню").icon("scroll").text("Сохранить, загрузить, новый мир, выйти.").note("Esc — то же самое")).clicked() {
                self.menu = !self.menu;
                g.paused = true;
            }
            let date = g.date();
            let font = egui::FontId::new(17.0, egui::FontFamily::Name("bold".into()));
            let dg = ui.painter().layout_no_wrap(date.clone(), font, s.desk_ink);
            let pw = dg.size().x.max(5.0 * 14.0 + 4.0 * 4.0) + 24.0 + 24.0;
            let (plate, _) = ui.allocate_exact_size(egui::vec2(pw, H - 14.0), egui::Sense::hover());
            let plate = plate.translate(egui::vec2(0.0, -2.0));
            let mut v = chrome::Out::new();
            hud::sunk(&mut v, plate, &ch, 3.0);
            ui.painter().extend(v);
            let dr = egui::Rect::from_min_size(egui::pos2(plate.center().x - dg.size().x / 2.0 + 10.0, plate.min.y + 4.0), dg.size());
            ui.painter().galley(dr.min, dg, s.desk_ink);
            // (the season's sign before the date; the month's run along the plate's foot)
            let (season, sign, tint) = match month % 12 {
                11 | 0 | 1 => ("зима", "snow", Color32::from_rgb(200, 224, 244)),
                2..=4 => ("весна", "sprout", Color32::from_rgb(160, 214, 120)),
                5..=7 => ("лето", "sun", Color32::from_rgb(244, 206, 96)),
                _ => ("осень", "leaf", Color32::from_rgb(226, 150, 72)),
            };
            let icons = chrome::icons(ctx);
            let sr = egui::Rect::from_center_size(egui::pos2(dr.min.x - 13.0, dr.center().y), egui::vec2(17.0, 17.0));
            look::icon(&icons, sign, 17.0, Color32::from_black_alpha(200)).paint_at(ui, sr.translate(egui::vec2(1.0, 1.0)));
            look::icon(&icons, sign, 17.0, tint).paint_at(ui, sr);
            let run = ((g.day as f32 + 1.0) / g.days().max(1) as f32).clamp(0.0, 1.0);
            let (x0, x1, yb) = (plate.min.x + 6.0, plate.max.x - 6.0, plate.max.y - 3.0);
            let mut v = chrome::Out::new();
            v.push(egui::Shape::line_segment([egui::pos2(x0, yb), egui::pos2(x1, yb)], egui::Stroke::new(2.0, Color32::from_black_alpha(160))));
            v.push(egui::Shape::line_segment([egui::pos2(x0, yb), egui::pos2(x0 + (x1 - x0) * run, yb)], egui::Stroke::new(2.0, ch.trim.gamma_multiply(0.85))));
            ui.painter().extend(v);
            let d = ui.interact(dr, egui::Id::new("date"), egui::Sense::hover());
            chrome::spot(ctx, "date", d.rect);
            let month = g.month();
            d.tip(|| Card::new("Сегодня").icon(sign).sub(date.clone()).row("время года", season).row("месяцев игры", month.to_string()).note("месяц мира проживается в конце месяца; черта под датой — сколько его прошло"));
            // (the speeds: five bars under the date)
            let y = plate.max.y - 13.0;
            let x0 = plate.center().x - (5.0 * 14.0 + 4.0 * 4.0) / 2.0;
            for k in 1..=5usize {
                let r = egui::Rect::from_min_size(egui::pos2(x0 + (k - 1) as f32 * 18.0, y), egui::vec2(14.0, 7.0));
                let resp = ui.interact(r.expand2(egui::vec2(2.0, 4.0)), egui::Id::new(("speed", k)), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                let on = k <= g.speed;
                let c = if on { if g.paused { ch.trim.gamma_multiply(0.55) } else { ch.trim } } else { Color32::from_gray(64) };
                let c = if resp.hovered() { hud::lighten(c, 0.3) } else { c };
                let mut v = chrome::Out::new();
                v.push(egui::Shape::rect_filled(r.expand(1.0), 1.0, Color32::from_black_alpha(200)));
                v.push(egui::Shape::rect_filled(r, 1.0, c));
                if on {
                    v.push(egui::Shape::line_segment([r.left_top() + egui::vec2(0.5, 0.8), r.right_top() + egui::vec2(-0.5, 0.8)], egui::Stroke::new(1.0, hud::lighten(c, 0.4))));
                }
                ui.painter().extend(v);
                if resp.tip(|| Card::new(format!("Скорость {k}")).icon("hourglass").text(["день за полсекунды", "день за четверть секунды", "десять дней в секунду", "двадцать пять дней в секунду", "как можно быстрее"][k - 1]).note("клавиши + и −")).clicked() {
                    g.speed = k;
                }
            }
            // (slower and faster at the row's ends)
            for (faster, name, x) in [(false, "slower", x0 - 17.0), (true, "faster", x0 + 5.0 * 18.0 + 1.0)] {
                let r = egui::Rect::from_min_size(egui::pos2(x, y - 3.0), egui::vec2(13.0, 13.0));
                let resp = ui.interact(r.expand(3.0), egui::Id::new(("speed-step", faster)), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                let tint = if resp.hovered() { hud::lighten(ch.trim, 0.4) } else { ch.trim.gamma_multiply(0.85) };
                look::icon(&icons, name, 13.0, Color32::from_black_alpha(200)).paint_at(ui, r.translate(egui::vec2(1.0, 1.0)));
                look::icon(&icons, name, 13.0, tint).paint_at(ui, r);
                let card = || if faster { Card::new("Быстрее").icon("faster").text("Время идёт быстрее.").note("клавиша +") } else { Card::new("Медленнее").icon("slower").text("Время идёт медленнее.").note("клавиша −") };
                if resp.tip(card).clicked() {
                    g.speed = if faster { (g.speed + 1).min(5) } else { g.speed.saturating_sub(1).max(1) };
                }
            }
            let paused = g.paused;
            if hud::medallion(ui, &ch, &s, if paused { "play" } else { "pause" }, 22.0, if paused { Color32::from_rgb(240, 214, 140) } else { s.desk_ink }, paused, "clock")
                .tip(|| Card::new(if paused { "Время стоит" } else { "Время идёт" }).icon(if paused { "play" } else { "pause" }).text(if paused { "Щёлкните, чтобы пустить время." } else { "Щёлкните, чтобы остановить время." }).note("пробел — то же самое"))
                .clicked()
            {
                g.paused = !g.paused;
                g.alert = None;
            }
        });
    }

    /// Why the clock stopped, under the bar.
    pub(super) fn banner(&mut self, ctx: &egui::Context, g: &mut Game) {
        let Some(alert) = g.alert.clone() else { return };
        let s = self.skin;
        let ch = self.chrome.clone();
        egui::TopBottomPanel::top("banner").frame(egui::Frame::none().fill(s.paper2).inner_margin(egui::Margin::symmetric(14.0, 7.0))).show(ctx, |ui| {
            // (a strip of paper pinned under the bar, its lower edge trimmed)
            let r = ui.max_rect().expand2(egui::vec2(14.0, 7.0));
            let mut v = chrome::Out::new();
            ch.lay(&mut v, r, &ch.paper, 0.0);
            ch.gloss(&mut v, r, Color32::from_black_alpha(30), Color32::from_white_alpha(20));
            v.push(egui::Shape::line_segment([egui::pos2(r.min.x, r.max.y - 1.5), egui::pos2(r.max.x, r.max.y - 1.5)], egui::Stroke::new(1.5, ch.trim)));
            ui.painter().extend(v);
            ui.horizontal(|ui| {
                let at = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover()).0;
                look::icon(&self.icons, "alarm", 20.0, s.accent).paint_at(ui, at);
                ui.label(RichText::new("ЧАСЫ ВСТАЛИ").family(egui::FontFamily::Name("head".into())).size(16.0).color(s.accent));
                ui.label(&alert);
                if chrome::button(ui, &ch, &s, "Дальше", egui::vec2(90.0, 28.0), false, true).clicked() {
                    g.paused = false;
                    g.alert = None;
                }
                if chrome::button(ui, &ch, &s, "События", egui::vec2(90.0, 28.0), false, false).clicked() {
                    self.tab = Some(Tab::News);
                }
            });
        });
    }
}
