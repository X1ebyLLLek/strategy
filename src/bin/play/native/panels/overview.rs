//! The economy's first page (29th session; his words: «надо, чтобы перед игроком была не мешанина, а
//! приятный визуал, которым удобно и понятно управлять; чтобы он мог либо ставить экономистов, либо
//! сам понимал, как настроить и доработать экономику, какие графики на что и как влияют»).
//!
//! From the top: who runs the economy (you or an economist, one click to change), what needs
//! attention now (each with its cause in numbers and the buttons that mend it), the six vital signs of
//! the country as tiles with their lines of the last three years, and under the one chosen — the
//! chart with the player's own decisions marked on it, what presses on it, the levers that move it
//! (− and + right there, with what a year of the step would do, as the game itself reckons it).
//! The tiles run from money to people, the way the causes run: the treasury, output, prices, jobs,
//! want, anger.

use super::*;
use super::super::charts::{fix, num, sfix};
use super::super::hud;
use super::super::kit;
use super::super::{set_page, Tab};

mod advice;
mod detail;
pub(super) mod govern;

/// Months of the chronicle the tiles' lines show.
const WINDOW: usize = 36;

/// A vital sign of the country: what it is called, its figure now, how it stands (its tone), the
/// line of its last months.
pub(super) struct Vital {
    pub key: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub value: String,
    pub caption: String,
    pub tone: Color32,
    /// Up or down since three months ago, and whether that is good.
    pub trend: Option<(bool, bool)>,
    pub series: Vec<f32>,
    /// What the figure is and why it matters (the explanation under the tile's heading).
    pub what: &'static str,
    /// How a value of its line is written.
    pub fmt: fn(f32) -> String,
    /// Whether the line is shown as an index (the first month of the window — 100), the two
    /// countries being of different size.
    pub indexed: bool,
}

fn fmt_level(x: f32) -> String {
    format!("×{}", fix(x, 2))
}
fn fmt_pct(x: f32) -> String {
    format!("{:.0}%", x * 100.0)
}
fn fmt_anger(x: f32) -> String {
    fix(x, 2)
}
fn fmt_index(x: f32) -> String {
    format!("{x:.0}")
}

/// Up or down over the last three months (a share of the line's size, or `eps` of it at least),
/// and whether that is good.
fn trend(series: &[f32], up_good: bool, eps: f32) -> Option<(bool, bool)> {
    let n = series.len();
    if n < 4 {
        return None;
    }
    let (now, then) = (series[n - 1], series[n - 4]);
    let d = now - then;
    let scale = now.abs().max(then.abs()).max(1e-3);
    (d.abs() / scale > eps).then_some((d > 0.0, (d > 0.0) == up_good))
}

/// The six vital signs, from the money to the people.
pub(super) fn vitals(g: &Game) -> Vec<Vital> {
    let me = g.me;
    let n = g.h.home(me);
    let f = &g.spirits.facts[me];
    let rows: Vec<&crate::game::Row> = g.rows.iter().rev().take(WINDOW).collect::<Vec<_>>().into_iter().rev().collect();
    let mine = |get: &dyn Fn(&crate::game::Row, usize) -> f32| -> Vec<f32> { rows.iter().map(|r| get(r, me)).collect() };
    let b = g.rows.last().map(|r| r.book[me]).unwrap_or_default();
    let (income, spent) = (b.income(), b.spent());
    let net = income - spent;
    let months = if spent > 1e-6 { n.treasury / spent } else { f32::INFINITY };
    let idle = g.figures(me).idle;
    let (good, mixed, bad) = (kit::GOOD, kit::MIXED, kit::BAD);
    let signed = |x: f32| if x >= 0.0 { format!("+{}", num(x)) } else { num(x) };

    let treasury = Vital {
        key: "treasury",
        name: "Казна",
        icon: "coins",
        value: num(n.treasury),
        caption: format!("{} в месяц", signed(net)),
        tone: if n.treasury < 0.0 || (net < 0.0 && months < 6.0) { bad } else if net < 0.0 || months < 1.0 { mixed } else { good },
        trend: trend(&mine(&|r, s| r.book[s].treasury), true, 0.03),
        series: mine(&|r, s| r.book[s].treasury),
        what: "Деньги государства. Каждый месяц казна берёт налоги и платит армии, науке, стройке, чиновникам, а что осталось — раздаёт людям. Нет денег — нет ни армии, ни школ, ни заводов.",
        fmt: num,
        indexed: false,
    };
    let output = Vital {
        key: "output",
        name: "Выпуск",
        icon: "sprout",
        value: num(n.gdp_real),
        caption: format!("{}% за год", sfix(f.growth * 100.0, 1)),
        // (in the first year growth is not yet a figure to worry over: a country that has stood still for
        // lack of months is not marked)
        tone: if f.growth < -0.03 { bad } else if f.growth < 0.005 && g.rows.len() > 12 { mixed } else { good },
        trend: trend(&mine(&|r, s| r.gdp[s]), true, 0.004),
        series: mine(&|r, s| r.gdp[s]),
        what: "Всё, что страна делает за месяц, в неизменных ценах: хлеб, одежда, машины, снаряжение. От него зависят и налоги, и армия, и достаток людей. На графике — рост против начала окна (там 100), у соседа так же.",
        fmt: fmt_index,
        indexed: true,
    };
    let prices = Vital {
        key: "prices",
        name: "Цены",
        icon: "shop",
        value: format!("×{}", fix(n.level, 2)),
        caption: format!("{}% за год", sfix(f.inflation * 100.0, 1)),
        tone: if f.inflation > 0.12 || f.inflation < -0.05 { bad } else if f.inflation > 0.05 || f.inflation < -0.02 { mixed } else { good },
        trend: trend(&mine(&|r, s| r.level[s]), false, 0.004),
        series: mine(&|r, s| r.level[s]),
        what: "Во сколько раз цены выше, чем в начале игры. Растут от лишних денег (станок, дешёвый кредит), нехватки товара и пошлин; съедают доходы людей и казны.",
        fmt: fmt_level,
        indexed: false,
    };
    let jobs = Vital {
        key: "jobs",
        name: "Работа",
        icon: "hand",
        value: format!("{:.0}%", idle * 100.0),
        caption: "без работы".into(),
        tone: if idle >= 0.10 { bad } else if idle >= 0.05 { mixed } else { good },
        trend: trend(&mine(&|r, s| r.idle[s]), false, 0.08),
        series: mine(&|r, s| r.idle[s]),
        what: "Доля работающих групп, которым не нашлось места. Безработные меньше покупают и злее смотрят на власть. Места дают заводы: их строят хозяева и казна.",
        fmt: fmt_pct,
        indexed: false,
    };
    let want = Vital {
        key: "want",
        name: "Нужда",
        icon: "bread",
        value: format!("{:.0}%", f.hunger * 100.0),
        caption: "не хватает".into(),
        tone: if f.hunger > 0.10 { bad } else if f.hunger > 0.02 { mixed } else { good },
        trend: trend(&mine(&|r, s| r.hunger[s]), false, 0.08),
        series: mine(&|r, s| r.hunger[s]),
        what: "Какая доля самого нужного (хлеба, топлива…) не достаётся самой нуждающейся группе. К бедности привыкают, к голоду нет: он убивает людей и быстрее всего копит недовольство.",
        fmt: fmt_pct,
        indexed: false,
    };
    let anger = n.anger();
    let wrath = Vital {
        key: "anger",
        name: "Недовольство",
        icon: "revolt",
        value: fix(anger, 2),
        caption: if anger < 0.5 { "спокойно".into() } else if anger < 1.5 { "ворчат".into() } else { "волнения близко".into() },
        tone: if anger < 0.5 { good } else if anger < 1.5 { mixed } else { bad },
        trend: trend(&mine(&|r, s| r.anger[s]), false, 0.05),
        series: mine(&|r, s| r.anger[s]),
        what: "Копится, когда жизнь становится хуже, чем привыкли, и от голода; понемногу остывает. Когда у группы переполнится терпение — забастовки и волнения, власть шатается.",
        fmt: fmt_anger,
        indexed: false,
    };
    vec![treasury, output, prices, jobs, want, wrath]
}

/// The tone's mark: a disc with a tick (good), a bang (mixed), a cross (bad).
fn mark(p: &egui::Painter, c: egui::Pos2, tone: Color32) {
    p.circle_filled(c, 8.0, tone);
    p.circle_stroke(c, 8.0, egui::Stroke::new(1.0, Color32::from_black_alpha(150)));
    let w = egui::Stroke::new(1.8, Color32::WHITE);
    if tone == kit::GOOD {
        p.add(egui::Shape::line(vec![c + vec2(-3.6, 0.2), c + vec2(-1.0, 2.8), c + vec2(3.8, -2.8)], w));
    } else if tone == kit::BAD {
        p.line_segment([c + vec2(-3.0, -3.0), c + vec2(3.0, 3.0)], w);
        p.line_segment([c + vec2(-3.0, 3.0), c + vec2(3.0, -3.0)], w);
    } else {
        p.line_segment([c + vec2(0.0, -4.0), c + vec2(0.0, 1.2)], w);
        p.circle_filled(c + vec2(0.0, 3.8), 1.1, Color32::WHITE);
    }
}

/// A vital sign as a tile: its icon and name, a mark of how it stands, the figure large with its
/// caption, the line of its months at the right; the one chosen is raised and framed in the accent.
fn tile(ui: &mut egui::Ui, s: &Skin, v: &Vital, size: egui::Vec2, chosen: bool) -> egui::Response {
    let (ch, _) = chrome::dress(ui);
    let icons = chrome::icons(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if !ui.is_rect_visible(r) {
        return resp;
    }
    let hot = resp.hovered();
    let r = if chosen { r.translate(vec2(0.0, -1.0)) } else { r };
    let mut sh = chrome::Out::new();
    if chosen {
        sh.push(egui::epaint::Shadow { offset: vec2(0.0, 3.0), blur: 8.0, spread: 0.0, color: Color32::from_black_alpha(120) }.as_shape(r, 3.0).into());
    }
    ch.lay(&mut sh, r, &ch.paper, 2.0);
    if chosen {
        // (a tint of the trim: lighter paper takes more of it than dark, where the words must still read)
        sh.push(egui::Shape::rect_filled(r, 2.0, ch.trim.gamma_multiply(if s.paper.r() < 128 { 0.07 } else { 0.22 })));
    }
    let dark = s.paper.r() < 128;
    ch.gloss(&mut sh, r, Color32::from_white_alpha(if chosen { if dark { 40 } else { 80 } } else if hot { 64 } else { 40 }), Color32::from_black_alpha(30));
    ch.bevel(&mut sh, r, !chosen, 1.2);
    let foot = egui::Rect::from_min_max(egui::pos2(r.min.x + 1.5, r.max.y - 5.0), egui::pos2(r.max.x - 1.5, r.max.y - 1.5));
    sh.push(egui::Shape::rect_filled(foot, 1.0, v.tone));
    sh.push(egui::Shape::rect_stroke(r, 2.0, egui::Stroke::new(if chosen { 2.4 } else { 1.0 }, if chosen { hud::lighten(ch.trim, 0.1) } else { Color32::from_black_alpha(110) })));
    if chosen {
        sh.push(egui::Shape::rect_stroke(r.expand(1.2), 3.0, egui::Stroke::new(1.0, Color32::from_black_alpha(170))));
    }
    ui.painter().extend(sh);
    let p = ui.painter();
    // (its icon and name, the mark of how it stands at the corner)
    super::super::look::icon(&icons, v.icon, 18.0, s.ink).paint_at(ui, egui::Rect::from_min_size(r.min + vec2(9.0, 7.0), vec2(18.0, 18.0)));
    p.text(r.min + vec2(32.0, 16.0), egui::Align2::LEFT_CENTER, v.name.to_uppercase(), egui::FontId::new(13.0, egui::FontFamily::Name("head".into())), if chosen { s.ink } else { s.soft });
    mark(p, egui::pos2(r.max.x - 15.0, r.min.y + 16.0), v.tone);
    // (the figure, an arrow after it when it moves, the caption under it)
    let gv = p.layout_no_wrap(v.value.clone(), egui::FontId::new(22.0, egui::FontFamily::Name("bold".into())), s.ink);
    let vw = gv.size().x;
    p.galley(r.min + vec2(10.0, 28.0), gv, s.ink);
    if let Some((up, good)) = v.trend {
        let mut a = chrome::Out::new();
        hud::arrow(&mut a, r.min + vec2(10.0 + vw + 12.0, 42.0), up, if good { kit::GOOD } else { kit::BAD });
        p.extend(a);
    }
    let mut job = egui::text::LayoutJob::simple_singleline(v.caption.clone(), egui::FontId::proportional(12.5), s.soft);
    job.wrap = egui::text::TextWrapping::truncate_at_width(r.width() * 0.52);
    let gc = p.layout_job(job);
    p.galley(r.min + vec2(10.0, 55.0), gc, s.soft);
    // (the line of its months)
    let spark = egui::Rect::from_min_max(egui::pos2(r.max.x - r.width() * 0.40 - 8.0, r.min.y + 32.0), egui::pos2(r.max.x - 10.0, r.max.y - 12.0));
    super::super::charts::spark(p, spark, &v.series, hud::darken(s.accent, 0.1));
    resp
}

/// The places of the tiles for the window's self-checks.
const SPOTS: [&str; 6] = ["vital0", "vital1", "vital2", "vital3", "vital4", "vital5"];

/// The tiles, two to a row.
fn tiles(ui: &mut egui::Ui, s: &Skin, vs: &[Vital], chosen: usize) -> Option<usize> {
    let gap = 6.0;
    let w = ((ui.available_width() - gap) / 2.0).floor();
    let h = 76.0;
    let mut out = None;
    for (row, pair) in vs.chunks(2).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (j, v) in pair.iter().enumerate() {
                let k = row * 2 + j;
                let resp = tile(ui, s, v, vec2(w, h), k == chosen);
                if ui.clip_rect().contains_rect(resp.rect) {
                    chrome::spot(ui.ctx(), SPOTS[k], resp.rect);
                }
                let (name, what) = (v.name, v.what);
                let (value, caption) = (v.value.clone(), v.caption.clone());
                let resp = resp.tip(|| Card::new(name).icon(v.icon).row("сейчас", value).row("за год", caption).text(what).note(if k == chosen { "разобрано ниже" } else { "щёлкните — разобрать ниже" }));
                if resp.clicked() {
                    out = Some(k);
                }
            }
        });
        ui.add_space(gap - 2.0);
    }
    out
}

/// The chosen vital sign (kept while the window lives).
fn chosen_id() -> egui::Id {
    egui::Id::new("econ-vital")
}

/// Make the vital sign `k` the chosen one (for the self-checks, and for the buttons that point at one).
pub(super) fn choose(ctx: &egui::Context, k: usize) {
    ctx.data_mut(|d| d.insert_temp(chosen_id(), k));
}

/// Forget the vital sign chosen (the first is the one open next time: for the window's self-checks).
pub fn forget_vital(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<usize>(chosen_id()));
}

/// The page «Обзор».
pub fn overview(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let vs = vitals(g);
    // (`SHOTVITAL=k`: for pictures, the k-th vital sign open from the first frame)
    let first = std::env::var("SHOTVITAL").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
    let chosen = ui.ctx().data(|d| d.get_temp::<usize>(chosen_id())).unwrap_or(first).min(vs.len() - 1);
    // (the forecast the levers' notes read: reckoned once, when the page is first opened; after that
    // the player asks for it again)
    let f = super::forecast::state();
    if !super::super::spoiled() && !f.running {
        let now = ui.ctx().input(|i| i.time);
        let last_id = egui::Id::new("econ-forecast-last");
        let since_id = egui::Id::new("econ-forecast-since");
        let last = ui.ctx().data(|d| d.get_temp::<f64>(last_id));
        if f.rows.is_empty() {
            super::forecast::start(g);
            ui.ctx().data_mut(|d| d.insert_temp(last_id, now));
        } else if f.month == g.month() && f.stamp != g.stamp && !g.h.at_war && last.is_none_or(|t| now - t > 10.0) {
            // (a decision taken since: the forecast is reckoned anew when the hand has let the levers
            // alone for a moment, so that a run of presses of + is reckoned once; not more than once
            // in ten seconds, and not in a war, whose days keep changing the world)
            let from = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with::<f64>(since_id, || now));
            if now - from > 1.2 {
                super::forecast::start(g);
                ui.ctx().data_mut(|d| {
                    d.insert_temp(last_id, now);
                    d.remove::<f64>(since_id);
                });
            } else {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
            }
        } else {
            ui.ctx().data_mut(|d| d.remove::<f64>(since_id));
        }
    }
    govern::strip(ui, s, g);
    advice::list(ui, s, g);
    kit::ribbon(ui, "Пульс страны", Some("chart"));
    kit::words(ui, "От денег к людям, как идут причины. Щёлкните показатель: ниже его график, причины и рычаги.", 13.0, s.soft);
    ui.add_space(2.0);
    if let Some(k) = tiles(ui, s, &vs, chosen) {
        // (the spoiled control of the self-checks: a tile does not answer)
        if !super::super::spoiled() {
            choose(ui.ctx(), k);
        }
    }
    detail::detail(ui, s, g, &vs, chosen);
    ui.add_space(8.0);
}
