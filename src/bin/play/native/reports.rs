//! The reports at the right under the bar (28th session; his words: «непонятно, чем заняться
//! игроку… кликать можно на много разных вещей, но непонятно зачем»): the country's goal first —
//! the great task taken, how far it has come, what helps it; then the few things that matter now,
//! each in words with its cause in numbers («казна тает −2,8 в месяц: армия — 43% трат») and a click
//! that opens the very page and lights the very lever that mends it. HOI4's alerts say that
//! something is wrong; these say why, and where to put the hand.

use eframe::egui::{self, pos2, vec2, Color32, FontFamily, FontId, Rect, Sense, Stroke};

use super::*;
use super::charts::num;

/// One report: its icon, what it is, why (with numbers), what to do; how grave (0 — a chance,
/// 1 — trouble, 2 — danger); where the hand goes: the folder, its page, the lever lit there.
pub(super) struct Report {
    pub icon: &'static str,
    pub title: String,
    pub cause: String,
    pub todo: String,
    pub tone: u8,
    pub tab: Tab,
    pub page: usize,
    pub focus: Option<&'static str>,
    pub rows: Vec<(String, String)>,
}

/// The goal of the country: the great task taken (its title, how far, months left, what helps it
/// and where), or how many are offered.
pub(super) struct Goal {
    pub icon: &'static str,
    pub title: String,
    pub measure: String,
    pub progress: Option<f32>,
    pub left: u32,
    pub help: &'static str,
    pub tab: Tab,
    pub page: usize,
    pub focus: Option<&'static str>,
    pub reward: String,
}

/// The goal now.
pub(super) fn goal(g: &Game) -> Option<Goal> {
    use crate::country::agenda::Goal as G;
    let me = g.me;
    let Some(t) = g.agenda.now[me].clone() else {
        let n = g.agenda.offers[me].len();
        return (n > 0).then(|| Goal {
            icon: "flag",
            title: "Выберите цель страны".into(),
            measure: format!("силы общества предлагают {n} {}", crate::words::plural(n, "задачу", "задачи", "задач")),
            progress: None,
            left: 0,
            help: "выполненная в срок — законность и преимущество на 5 лет",
            tab: Tab::Society,
            page: 2,
            focus: Some("task"),
            reward: String::new(),
        });
    };
    let (help, tab, page, focus): (&'static str, Tab, usize, Option<&'static str>) = match t.goal {
        G::Industry => ("стройка казны — в «Бюджете»", Tab::Econ, panels::economy::BUDGET, Some("build")),
        G::Harvest => ("стройка казны на земле — в «Бюджете»", Tab::Econ, panels::economy::BUDGET, Some("build")),
        G::Literacy => ("школы — гражданские статьи в «Бюджете»", Tab::Econ, panels::economy::BUDGET, Some("civil")),
        G::Army => ("доля армии в тратах — в «Бюджете»", Tab::Econ, panels::economy::BUDGET, Some("army")),
        G::Treasury => ("налог и траты — в «Бюджете»", Tab::Econ, panels::economy::BUDGET, Some("tax")),
        G::Tech(_) => ("наука — упор на это направление", Tab::Science, 0, None),
        G::Reform(..) => ("реформа — в «Обществе», «Законы»", Tab::Society, 1, None),
        G::Calm => ("порядок, призрение, твёрдая рука — «Общество»", Tab::Society, 2, None),
        G::Outgrow => ("стройка и рынок — в «Бюджете»", Tab::Econ, panels::economy::BUDGET, Some("build")),
        G::Faith => ("вера и движения — «Общество»", Tab::Society, 2, None),
        G::Revanche(_) => ("война за область — «Политика»", Tab::Politics, 1, None),
    };
    Some(Goal {
        icon: t.goal.icon(),
        title: g.task_title(&t),
        measure: g.task_words(me, &t),
        progress: Some(g.task_progress(me, &t)),
        left: t.until.saturating_sub(g.month()),
        help,
        tab,
        page,
        focus,
        reward: t.goal.reward_words().to_string(),
    })
}

/// The reports of the moment, the gravest first (at most `most`).
pub(super) fn reports(g: &Game, most: usize) -> Vec<Report> {
    let me = g.me;
    let pack = &g.pack;
    let n = g.h.home(me);
    let month = g.month();
    let b = g.rows.last().map(|r| r.book[me]).unwrap_or_default();
    let (income, spent) = (b.income(), b.spent());
    let net = income - spent;
    let mut out: Vec<Report> = Vec::new();
    let pct = |x: f32| format!("{:.0}%", x * 100.0);
    // the treasury: in debt, or melting — and what eats it
    if n.treasury < 0.0 || (net < 0.0 && spent > 0.0 && n.treasury / spent < 12.0) {
        let lines = [("армия", b.army + b.soldiers, "army"), ("наука", b.science, "science"), ("стройка", b.build, "build"), ("гражданские статьи", b.civil, "civil"), ("раздача людям", b.dole, "build")];
        let top = lines.iter().copied().max_by(|a, c| a.1.total_cmp(&c.1)).unwrap_or(("армия", 0.0, "army"));
        let months = if spent > 1e-6 { (n.treasury / spent).max(0.0) } else { 0.0 };
        let debt = n.treasury < 0.0;
        out.push(Report {
            icon: "coins",
            title: if debt { "Казна в долгах".into() } else { "Казна тает".into() },
            cause: format!("{} в месяц; больше всего тратит {} — {}", num(net), top.0, pct(top.1 / spent.max(1e-6))),
            todo: "Поднимите налог или урежьте самую большую статью — строки и рычаги в «Бюджете».".into(),
            tone: if debt || months < 3.0 { 2 } else { 1 },
            tab: Tab::Econ,
            page: panels::economy::BUDGET,
            focus: Some(if debt { "tax" } else { top.2 }),
            rows: vec![("в казне".into(), num(n.treasury)), ("итог месяца".into(), num(net)), ("запаса на".into(), format!("{months:.0} мес.")), (top.0.into(), pct(top.1 / spent.max(1e-6)))],
        });
    }
    // the war: supply, kit, generals
    if let (true, Some(c)) = (g.h.at_war, g.h.war.corps.as_ref()) {
        let ours: Vec<&strategy::military::corps::Division> = c.divs.iter().filter(|d| d.side == me).collect();
        let all = ours.len().max(1);
        let hungry = ours.iter().filter(|d| d.supplied < 0.5).count();
        let fed = g.h.fed[me];
        if fed < 0.85 || hungry * 5 > all {
            out.push(Report {
                icon: "crate",
                title: if fed < 0.6 { "Голод в войсках".into() } else { "Снабжения не хватает".into() },
                cause: format!("доходит {}; голодают {hungry} из {all} дивизий", pct(fed)),
                todo: "Склад ближе к фронту или железная дорога к худшему участку — страница «Снабжение».".into(),
                tone: if fed < 0.6 { 2 } else { 1 },
                tab: Tab::Army,
                page: 2,
                focus: None,
                rows: vec![("доходит до войск".into(), pct(fed)), ("голодных дивизий".into(), format!("{hungry}"))],
            });
        }
        let short = ours.iter().filter(|d| c.equipped(pack, d) < 0.8).count();
        if short * 4 > all {
            let made: f32 = g.arms.lines[me].iter().map(|l| l.made).sum::<f32>() / strategy::military::corps::BATTALION;
            out.push(Report {
                icon: "gears",
                title: "Дивизиям не хватает снаряжения".into(),
                cause: format!("недооснащены {short} из {all}; заводы дают {made:.0} бат. в месяц"),
                todo: "Больше денег армии («Бюджет») или упор заводов на нужный род («Производство»).".into(),
                tone: if short * 2 > all { 2 } else { 1 },
                tab: Tab::Econ,
                page: panels::economy::BUDGET,
                focus: Some("army"),
                rows: vec![("недооснащённых".into(), format!("{short}")), ("выпуск, бат./мес.".into(), format!("{made:.1}"))],
            });
        }
    }
    if let Some(gs) = g.h.war.command[me].as_ref() {
        let lone: Vec<usize> = (0..gs.len()).filter(|&k| gs[k].general.is_none()).collect();
        if !lone.is_empty() {
            out.push(Report {
                icon: "medal",
                title: if lone.len() == 1 { format!("{} без генерала", crate::words::group_name(lone[0])) } else { format!("Армий без генерала: {}", lone.len()) },
                cause: format!("в резерве генералов {}", g.spare.len()),
                todo: "Выберите армию в «Армии» → «Сменить генерала».".into(),
                tone: 1,
                tab: Tab::Army,
                page: 0,
                focus: None,
                rows: vec![],
            });
        }
    }
    // the people: anger and its cause; idle hands
    let mood = n.anger();
    if mood >= 1.0 {
        let worst = (0..n.hardship.len()).max_by(|&a, &c| (n.hardship[a] * n.pop[a].max(1e-6).sqrt()).total_cmp(&(n.hardship[c] * n.pop[c].max(1e-6).sqrt())));
        let who = worst.and_then(|k| pack.groups.get(k)).map_or("бедные".to_string(), |x| x.name.to_lowercase());
        let infl = g.spirits.facts[me].inflation;
        out.push(Report {
            icon: "revolt",
            title: if mood >= 1.5 { "Народ зол".into() } else { "Народ ворчит".into() },
            cause: format!("тяжелее всех — {who}; цены за год {:+.0}%", infl * 100.0),
            todo: "Призрение и медицина (гражданские статьи) и налог — в «Бюджете»; чего не хватает — «Хозяйство», «Рынок».".into(),
            tone: if mood >= 1.5 { 2 } else { 1 },
            tab: Tab::Econ,
            page: panels::economy::BUDGET,
            focus: Some("civil"),
            rows: vec![("недовольство".into(), format!("{mood:.2}")), ("цены за год".into(), format!("{:+.1}%", infl * 100.0))],
        });
    }
    let idle = {
        let (mut x, mut all) = (0.0f32, 0.0f32);
        for (k, u) in n.unemployed.iter().enumerate() {
            if let Some(u) = u {
                x += u * n.pop[k];
                all += n.pop[k];
            }
        }
        x / all.max(1e-9)
    };
    if idle > 0.15 && g.adviser.is_none() {
        out.push(Report {
            icon: "hand",
            title: "Много людей без работы".into(),
            cause: format!("без работы {}; казна строит на {} остатка трат", pct(idle), pct(n.build_share)),
            todo: "Стройка казны — новые заводы и места; «Что строит казна» — где не хватает товара.".into(),
            tone: if idle > 0.25 { 1 } else { 0 },
            tab: Tab::Econ,
            page: panels::economy::BUDGET,
            focus: Some("build"),
            rows: vec![("без работы".into(), pct(idle))],
        });
    }
    // the order of society
    let p = g.polity_of(me);
    if p.unrest > 0.4 {
        let (m, x) = p.top_move();
        out.push(Report {
            icon: "revolt",
            title: if p.unrest > strategy::world::polity::RESTLESS { "Смута у порога революции".into() } else { "Смута растёт".into() },
            cause: format!("смута {}; сильнее всех — {} ({})", pct(p.unrest), strategy::world::polity::MOVE[m].name.to_lowercase(), pct(x)),
            todo: "Чего хотят движения и чем ответить — «Общество», «Движения и вера».".into(),
            tone: if p.unrest > strategy::world::polity::RESTLESS { 2 } else { 1 },
            tab: Tab::Society,
            page: 2,
            focus: None,
            rows: vec![("смута".into(), pct(p.unrest)), ("законность".into(), pct(p.legit))],
        });
    }
    let st = g.state.stability[me];
    if st < 0.5 {
        out.push(Report {
            icon: "crown",
            title: "Власть шатается".into(),
            cause: format!("устойчивость {}; законность {}", pct(st), pct(p.legit)),
            todo: "Слабое правительство не обоснует войну; голод и поражения расшатывают его — «Политика».".into(),
            tone: if st < crate::world::statecraft::SHAKY { 2 } else { 1 },
            tab: Tab::Politics,
            page: 0,
            focus: None,
            rows: vec![("устойчивость".into(), pct(st))],
        });
    }
    // what waits for a word
    let waiting = g.arms.offers[me].iter().filter(|o| o.trial == 0).count();
    if waiting > 0 {
        out.push(Report {
            icon: "science",
            title: "Новый образец ждёт решения".into(),
            cause: format!("образцов: {waiting} — в производство, испытать или отказаться"),
            todo: "«Производство»: карточки новых образцов вверху.".into(),
            tone: 0,
            tab: Tab::Arms,
            page: 0,
            focus: None,
            rows: vec![],
        });
    }
    if g.policy.running[me].iter().any(|r| r.waited > 0 && r.waited + 2 > month.saturating_sub(r.since).saturating_sub(r.done)) && n.treasury <= 0.0 {
        out.push(Report {
            icon: "scroll",
            title: "Программы стоят без денег".into(),
            cause: "в казне нечем платить".into(),
            todo: "Заём или меньше трат — «Страна», «Строй и программы».".into(),
            tone: 1,
            tab: Tab::Nation,
            page: 1,
            focus: None,
            rows: vec![],
        });
    }
    // peace: the neighbour's war, our claim, the armies' balance
    if !g.h.at_war {
        if g.state.claim[1 - me].is_some() {
            out.push(Report {
                icon: "alarm",
                title: "Сосед готовит войну".into(),
                cause: "обосновывает цель войны против нас".into(),
                todo: "Армия и склады у границы, мобилизация — «Армия».".into(),
                tone: 1,
                tab: Tab::Army,
                page: 0,
                focus: None,
                rows: vec![],
            });
        }
        if let Some(c) = &g.state.claim[me] {
            if c.ready <= month {
                out.push(Report {
                    icon: "flag",
                    title: "Цель войны обоснована".into(),
                    cause: "можно объявлять войну".into(),
                    todo: "«Политика», «Война и мир» — объявить, когда армия готова.".into(),
                    tone: 0,
                    tab: Tab::Politics,
                    page: 1,
                    focus: None,
                    rows: vec![],
                });
            }
        }
        if let Some(r) = g.rows.last() {
            if r.army[me] < 0.6 * r.army[1 - me] {
                out.push(Report {
                    icon: "army",
                    title: "Армия соперника больше".into(),
                    cause: format!("у нас {:.0} дивизий, у него {:.0}", r.army[me], r.army[1 - me]),
                    todo: "Доля армии в тратах — «Бюджет»; штат мирного времени — «Армия», «Бюджет».".into(),
                    tone: 1,
                    tab: Tab::Econ,
                    page: panels::economy::BUDGET,
                    focus: Some("army"),
                    rows: vec![],
                });
            }
        }
    }
    out.sort_by(|a, c| c.tone.cmp(&a.tone));
    out.truncate(most);
    out
}

/// The name of a folder, for the notes.
fn folder_name(t: Tab) -> &'static str {
    match t {
        Tab::Econ => "Экономика",
        Tab::Nation => "Страна",
        Tab::Arms => "Производство",
        Tab::Science => "Наука",
        Tab::Army => "Армия",
        Tab::Navy => "Флот",
        Tab::Politics => "Политика",
        Tab::News => "События",
        Tab::People => "Деятели",
        Tab::Society => "Общество",
        Tab::Ledger => "Сводка",
        Tab::Battle(_) => "Бой",
    }
}

/// The lever lit by a report: its key and when it was asked for.
fn focus_id() -> egui::Id {
    egui::Id::new("report-focus")
}

/// A lever of a page lit for a few seconds after a report sent the hand to it (and the page turned
/// to it once): call where the lever named `key` is drawn, with its rectangle.
pub fn glow(ui: &egui::Ui, r: Rect, key: &str) {
    let ctx = ui.ctx().clone();
    let Some((k, at, scrolled)) = ctx.data(|d| d.get_temp::<(String, f64, bool)>(focus_id())) else { return };
    if k != key {
        return;
    }
    let now = ctx.input(|i| i.time);
    let age = now - at;
    if age > 4.0 {
        return;
    }
    if !scrolled {
        ui.scroll_to_rect(r, Some(egui::Align::Center));
        ctx.data_mut(|d| d.insert_temp(focus_id(), (k.clone(), at, true)));
    }
    let pulse = 0.55 + 0.45 * ((age * 6.0).sin().abs() as f32);
    let fade = (1.0 - (age as f32 - 3.0).max(0.0)).clamp(0.0, 1.0);
    let col = Color32::from_rgb(255, 214, 96).gamma_multiply(pulse * fade);
    ui.painter().rect_stroke(r.expand(2.0), 3.0, Stroke::new(3.0, col));
    ctx.request_repaint();
}

/// Light the lever `key` for a few seconds, the page turning to it once (as a report's click does).
pub(super) fn light(ctx: &egui::Context, key: &str) {
    let now = ctx.input(|i| i.time);
    ctx.data_mut(|d| d.insert_temp(focus_id(), (key.to_string(), now, false)));
}

impl App {
    /// Send the hand to a report's place: the folder, its page, the lever lit.
    fn go_to(&mut self, ctx: &egui::Context, tab: Tab, page: usize, focus: Option<&'static str>) {
        self.tab = Some(tab);
        set_page(ctx, tab, page);
        if let Some(f) = focus {
            let now = ctx.input(|i| i.time);
            ctx.data_mut(|d| d.insert_temp(focus_id(), (f.to_string(), now, false)));
        }
    }

    /// The column of the goal and the reports at the right under the bar.
    pub(super) fn reports_column(&mut self, ctx: &egui::Context, g: &Game) {
        if spoiled() {
            return;
        }
        let Some(ch) = chrome::of(ctx) else { return };
        let s = self.skin;
        let open_id = egui::Id::new("reports-open");
        let open = ctx.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(true);
        let goal = goal(g);
        let list = reports(g, 5);
        // (29th session: three reports at most till the player asks for the rest — five cards and the
        // goal made a column of a third of the map's height over the front, and every card shouted)
        let all_id = egui::Id::new("reports-all");
        let all = ctx.data(|d| d.get_temp::<bool>(all_id)).unwrap_or(false);
        let shown = if all { list.len() } else { list.len().min(3) };
        let w = 330.0;
        let screen = ctx.screen_rect();
        let x0 = screen.max.x - w - 12.0;
        let mut go: Option<(Tab, usize, Option<&'static str>)> = None;
        egui::Area::new(egui::Id::new("reports")).fixed_pos(pos2(x0, top::H + 8.0)).order(egui::Order::Middle).show(ctx, |ui| {
            ui.set_width(w);
            ui.spacing_mut().item_spacing.y = 4.0;
            let icons = chrome::icons(ctx);
            // the heading: how many, a click folds the column
            let (hr, hresp) = ui.allocate_exact_size(vec2(w, 22.0), Sense::click());
            let hresp = hresp.on_hover_cursor(egui::CursorIcon::PointingHand);
            let mut v = chrome::Out::new();
            ch.lay(&mut v, hr, &ch.bar, 2.0);
            v.push(egui::Shape::rect_filled(hr, 2.0, Color32::from_black_alpha(120)));
            ui.painter().extend(v);
            let grave = list.iter().filter(|r| r.tone >= 2).count();
            let head = if list.is_empty() && goal.is_none() { "ДОКЛАДЫ · СРОЧНОГО НЕТ".to_string() } else { format!("ДОКЛАДЫ · {}{}", list.len(), if grave > 0 { format!(" · срочных {grave}") } else { String::new() }) };
            ui.painter().text(pos2(hr.min.x + 8.0, hr.center().y), egui::Align2::LEFT_CENTER, head, FontId::new(13.5, FontFamily::Name("head".into())), s.desk_ink);
            ui.painter().text(pos2(hr.max.x - 10.0, hr.center().y), egui::Align2::RIGHT_CENTER, if open { "свернуть" } else { "развернуть" }, FontId::proportional(12.0), s.desk_ink.gamma_multiply(0.75));
            if hresp.tip(|| Card::new("Доклады").icon("scroll").text("Цель страны и то, что важно сейчас: что случилось, почему (в числах) и где это исправить. Щелчок по докладу открывает нужную страницу и подсвечивает рычаг.")).clicked() {
                ctx.data_mut(|d| d.insert_temp(open_id, !open));
            }
            if !open {
                return;
            }
            // the goal of the country
            if let Some(gl) = &goal {
                let h = if gl.progress.is_some() { 74.0 } else { 54.0 };
                let (r, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                let mut v = chrome::Out::new();
                ch.lay(&mut v, r, &ch.bar, 2.0);
                v.push(egui::Shape::rect_filled(r, 2.0, Color32::from_black_alpha(if resp.hovered() { 60 } else { 95 })));
                ch.bevel(&mut v, r, true, 1.2);
                v.push(egui::Shape::rect_stroke(r, 2.0, Stroke::new(1.5, s.accent)));
                v.push(egui::Shape::rect_filled(Rect::from_min_size(r.min, vec2(5.0, r.height())), 1.0, s.accent));
                ui.painter().extend(v);
                let ir = Rect::from_center_size(pos2(r.min.x + 26.0, r.min.y + 26.0), vec2(26.0, 26.0));
                look::icon(&icons, gl.icon, 26.0, s.accent).paint_at(ui, ir);
                let x = r.min.x + 48.0;
                ui.painter().text(pos2(x, r.min.y + 5.0), egui::Align2::LEFT_TOP, "ЦЕЛЬ СТРАНЫ", FontId::new(11.5, FontFamily::Name("head".into())), s.desk_ink.gamma_multiply(0.75));
                let tw = r.max.x - x - 8.0;
                let mut job = egui::text::LayoutJob::simple_singleline(gl.title.clone(), FontId::new(14.5, FontFamily::Name("bold".into())), s.desk_ink);
                job.wrap = egui::text::TextWrapping::truncate_at_width(tw);
                let gt = ui.painter().layout_job(job);
                ui.painter().galley(pos2(x, r.min.y + 19.0), gt, s.desk_ink);
                let mut job = egui::text::LayoutJob::simple_singleline(gl.measure.clone(), FontId::proportional(12.5), s.desk_ink.gamma_multiply(0.85));
                job.wrap = egui::text::TextWrapping::truncate_at_width(tw);
                let gm = ui.painter().layout_job(job);
                ui.painter().galley(pos2(x, r.min.y + 37.0), gm, s.desk_ink);
                if let Some(pr) = gl.progress {
                    let bar = Rect::from_min_size(pos2(x, r.min.y + 56.0), vec2(tw - 70.0, 8.0));
                    ui.painter().rect_filled(bar, 2.0, Color32::from_black_alpha(140));
                    ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * pr.clamp(0.0, 1.0), bar.height())), 2.0, if pr >= 1.0 { kit::GOOD } else { s.accent });
                    ui.painter().text(pos2(r.max.x - 8.0, bar.center().y), egui::Align2::RIGHT_CENTER, format!("{:.0}% · {} мес.", pr * 100.0, gl.left), FontId::new(12.5, FontFamily::Name("bold".into())), s.desk_ink);
                }
                let (title, measure, help, reward) = (gl.title.clone(), gl.measure.clone(), gl.help, gl.reward.clone());
                let (tab, page, focus) = (gl.tab, gl.page, gl.focus);
                if resp
                    .tip(|| {
                        let mut c = Card::new(title.clone()).icon(gl.icon).text(measure.clone()).row("как помочь", help.to_string());
                        if !reward.is_empty() {
                            c = c.row("награда", reward.clone());
                        }
                        c.note(format!("щёлкните — «{}»", folder_name(tab)))
                    })
                    .clicked()
                {
                    go = Some((tab, page, focus));
                }
            }
            // the reports
            for (i, rp) in list.iter().enumerate().take(shown) {
                let (r, resp) = ui.allocate_exact_size(vec2(w, 50.0), Sense::click());
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                if i == 0 {
                    chrome::spot(ctx, "alert", r);
                }
                let hue = match rp.tone {
                    0 => Color32::from_rgb(150, 210, 110),
                    1 => Color32::from_rgb(236, 170, 60),
                    _ => Color32::from_rgb(230, 84, 64),
                };
                let mut v = chrome::Out::new();
                ch.lay(&mut v, r, &ch.bar, 2.0);
                v.push(egui::Shape::rect_filled(r, 2.0, Color32::from_black_alpha(if resp.hovered() { 70 } else { 110 })));
                ch.bevel(&mut v, r, true, 1.2);
                v.push(egui::Shape::rect_stroke(r, 2.0, Stroke::new(1.0, ch.trim.gamma_multiply(0.6))));
                v.push(egui::Shape::rect_filled(Rect::from_min_size(r.min, vec2(5.0, r.height())), 1.0, hue));
                ui.painter().extend(v);
                let c = pos2(r.min.x + 25.0, r.center().y);
                ui.painter().circle_filled(c, 15.0, hue.gamma_multiply(0.25));
                ui.painter().circle_stroke(c, 15.0, Stroke::new(1.5, hue));
                look::icon(&icons, rp.icon, 19.0, hue).paint_at(ui, Rect::from_center_size(c, vec2(19.0, 19.0)));
                let x = r.min.x + 47.0;
                let tw = r.max.x - x - 20.0;
                let mut job = egui::text::LayoutJob::simple_singleline(rp.title.clone(), FontId::new(14.5, FontFamily::Name("bold".into())), s.desk_ink);
                job.wrap = egui::text::TextWrapping::truncate_at_width(tw);
                let gt = ui.painter().layout_job(job);
                ui.painter().galley(pos2(x, r.min.y + 7.0), gt, s.desk_ink);
                let mut job = egui::text::LayoutJob::simple_singleline(rp.cause.clone(), FontId::proportional(12.5), s.desk_ink.gamma_multiply(0.85));
                job.wrap = egui::text::TextWrapping::truncate_at_width(tw);
                let gc = ui.painter().layout_job(job);
                ui.painter().galley(pos2(x, r.min.y + 27.0), gc, s.desk_ink);
                ui.painter().text(pos2(r.max.x - 9.0, r.center().y), egui::Align2::RIGHT_CENTER, "›", FontId::new(22.0, FontFamily::Name("bold".into())), s.desk_ink.gamma_multiply(0.7));
                let card = || {
                    let mut c = Card::new(rp.title.clone()).icon(rp.icon).sub(rp.cause.clone()).text(rp.todo.clone());
                    for (a, b) in &rp.rows {
                        c = c.row(a.clone(), b.clone());
                    }
                    c.note(format!("щёлкните — «{}»{}", folder_name(rp.tab), if rp.focus.is_some() { ", рычаг подсветится" } else { "" }))
                };
                if resp.tip(card).clicked() {
                    go = Some((rp.tab, rp.page, rp.focus));
                }
            }
            if list.len() > 3 {
                let (r, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                let mut v = chrome::Out::new();
                ch.lay(&mut v, r, &ch.bar, 2.0);
                v.push(egui::Shape::rect_filled(r, 2.0, Color32::from_black_alpha(if resp.hovered() { 70 } else { 120 })));
                v.push(egui::Shape::rect_stroke(r, 2.0, Stroke::new(1.0, ch.trim.gamma_multiply(0.5))));
                ui.painter().extend(v);
                let what = if all { "скрыть остальные".to_string() } else { format!("ещё докладов: {}", list.len() - 3) };
                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, what, FontId::new(12.5, FontFamily::Name("bold".into())), s.desk_ink);
                if resp.clicked() {
                    ctx.data_mut(|d| d.insert_temp(all_id, !all));
                }
            }
        });
        if let Some((t, p, f)) = go {
            self.go_to(ctx, t, p, f);
        }
    }
}
