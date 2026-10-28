//! The chosen vital sign laid open: its chart (ours and the rival's, the player's own decisions marked
//! along the foot, so that the line shows what followed what), what presses on it now (each cause with
//! its figure, those that help to the right of the middle, those that harm to the left), and the
//! levers that move it — the numbers with − and + right there, the laws with a step either way, the
//! pages elsewhere — each with what a year of the step would do, as the game itself reckons it (the
//! forecast: the same world lived ahead, `forecast.rs`).

use super::*;
use super::super::forecast::{self, Outcome};
use super::super::super::charts::{self, fix, num, sfix, Marker};
use crate::country::civil::LINE;
use crate::country::fiscal::{Choice, DOLES, LABOUR, PRINTS, RATES, TARIFFS, TAXES};
use crate::country::policy::LAWS;
use crate::tools::saves::Act;

// ---- the decisions on the chart ----------------------------------------------------------------

/// The decisions of the player's that bear on a vital sign: what each is called in words. (The key of
/// a decision form is the game's: `tax`, `fx_rate`…; those that bear on none are left out.)
fn marks(g: &Game, vital: &str) -> Vec<Marker> {
    let relevant: &[&str] = match vital {
        "treasury" => &["tax", "army", "science", "build", "fx_tax", "fx_tariff", "fx_print", "econ", "bonds_sell", "bonds_redeem", "ecolaw"],
        "output" => &["build", "modern", "ecolaw", "econ", "fx_sub", "fx_pub", "fx_tariff", "civ"],
        "prices" => &["fx_rate", "fx_print", "fx_tariff", "fx_caps", "tax"],
        "jobs" => &["build", "fx_sub", "fx_labour", "modern", "econ"],
        "want" => &["fx_caps", "fx_dole", "tax", "build"],
        _ => &["tax", "fx_dole", "fx_caps", "crackdown", "ecolaw", "civ", "fx_labour"],
    };
    let mut out = Vec::new();
    for (month, _, a) in &g.journal {
        let Act::Form(form) = a else { continue };
        for (k, v) in form {
            if !relevant.contains(&k.as_str()) {
                continue;
            }
            let pick = |list: &[Choice; 4]| v.trim().parse::<usize>().ok().and_then(|i| list.get(i)).map_or(v.clone(), |c| c.name.to_lowercase());
            let text = match k.as_str() {
                "tax" => format!("налог {v}%"),
                "army" => format!("доля армии {v}%"),
                "science" => format!("доля науки {v}%"),
                "build" => format!("стройка {v}%"),
                "fx_tax" => pick(&TAXES),
                "fx_dole" => format!("раздача: {}", pick(&DOLES)),
                "fx_tariff" => pick(&TARIFFS),
                "fx_labour" => format!("закон о труде: {}", pick(&LABOUR)),
                "fx_rate" => pick(&RATES),
                "fx_print" => format!("печатный станок: {}", pick(&PRINTS)),
                "fx_caps" => if v.trim() == "1" { "твёрдые цены введены".into() } else { "твёрдые цены сняты".into() },
                "econ" => match v.strip_prefix("adv").and_then(|x| x.parse::<usize>().ok()).and_then(|i| g.pool.get(i)) {
                    Some(p) => format!("хозяйством стал править {}", p.0),
                    None => "хозяйство — снова сами".into(),
                },
                "ecolaw" => v.trim().parse::<usize>().ok().and_then(|i| LAWS.get(i)).map_or(v.clone(), |l| format!("строй: {}", l.name.to_lowercase())),
                "modern" => v.trim().parse::<usize>().ok().and_then(|i| g.pack.sectors.get(i)).map_or(v.clone(), |s| format!("модернизация: {}", s.name)),
                "civ" => match v.split_once(':').and_then(|(a, b)| Some((a.trim().parse::<usize>().ok()?, b.trim().parse::<usize>().ok()?))) {
                    Some((i, l)) if i < LINE.len() && l < 4 => format!("{}: {}", LINE[i].name.to_lowercase(), LINE[i].levels[l]),
                    _ => continue,
                },
                "crackdown" => "твёрдая рука".into(),
                "bonds_sell" => "облигации проданы".into(),
                "bonds_redeem" => "облигации выкуплены".into(),
                "fx_sub" | "fx_pub" => "субсидии и казённая доля".into(),
                _ => continue,
            };
            out.push(Marker { month: *month, text: text.chars().next().map_or(String::new(), |c| c.to_uppercase().collect::<String>() + &text[c.len_utf8()..]) });
        }
    }
    out
}

/// The chart of a vital sign: ours (and the rival's, when they are alike in scale) over the chronicle,
/// the decisions marked.
fn chart(ui: &mut egui::Ui, s: &Skin, g: &Game, v: &Vital) {
    let me = g.me;
    let rows: Vec<&crate::game::Row> = g.rows.iter().rev().take(120).collect::<Vec<_>>().into_iter().rev().collect();
    if rows.len() < 3 {
        kit::words(ui, "График пойдёт, когда пройдёт несколько месяцев: запустите часы (пробел).", 13.0, s.soft);
        return;
    }
    let months: Vec<u32> = rows.iter().map(|r| r.month).collect();
    let pick = |side: usize| -> Vec<f32> {
        let all: Vec<f32> = rows
            .iter()
            .map(|r| match v.key {
                "treasury" => r.book[side].treasury,
                "output" => r.gdp[side],
                "prices" => r.level[side],
                "jobs" => r.idle[side],
                "want" => r.hunger[side],
                _ => r.anger[side],
            })
            .collect();
        if v.indexed {
            let first = all.first().copied().unwrap_or(1.0).abs().max(1e-9);
            all.into_iter().map(|x| x / first * 100.0).collect()
        } else {
            all
        }
    };
    let rival = g.h.home(1 - me).name.clone();
    let mut series = vec![(g.h.home(me).name.clone(), s.accent, pick(me))];
    if v.key != "treasury" {
        series.push((rival, Color32::from_rgb(74, 104, 138), pick(1 - me)));
    }
    let fmt: fn(f32) -> String = if v.indexed { fmt_index } else { v.fmt };
    let ms = marks(g, v.key);
    charts::plot_marked(ui, &format!("{} по месяцам", v.name), &series, &months, 150.0, &fmt, &ms);
    if !ms.is_empty() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), egui::Sense::hover());
            ui.painter().add(egui::Shape::convex_polygon(vec![r.left_bottom(), r.right_bottom(), egui::pos2(r.center().x, r.top())], s.accent, egui::Stroke::new(0.8, Color32::from_black_alpha(160))));
            kit::words(ui, "— ваши решения: наведите на месяц, чтобы прочитать.", 12.5, s.soft);
        });
    }
}

// ---- what presses on it ------------------------------------------------------------------------

/// One cause: its name, its figure in words, how much it helps (+1) or harms (−1), what it is.
struct Driver {
    name: String,
    value: String,
    good: f32,
    note: String,
}

fn driver(name: impl Into<String>, value: impl Into<String>, good: f32, note: impl Into<String>) -> Driver {
    Driver { name: name.into(), value: value.into(), good: good.clamp(-1.0, 1.0), note: note.into() }
}

/// The causes of a vital sign, from what the country's books show now.
fn drivers(g: &Game, key: &str) -> Vec<Driver> {
    let me = g.me;
    let n = g.h.home(me);
    let pack = &g.pack;
    let f = &g.spirits.facts[me];
    let mut out = Vec::new();
    match key {
        "treasury" => {
            let b = g.rows.last().map(|r| r.book[me]).unwrap_or_default();
            let top = b.income().max(b.spent()).max(1e-6);
            let ins = [("налоги", b.tax, "что взяли с доходов людей и хозяев"), ("пошлины", b.duty, "доля цены чужих товаров"), ("казённые заводы", b.sectors, "прибыль отраслей, которыми владеет казна")];
            for (name, x, note) in ins {
                if x > 0.005 {
                    out.push(driver(name, format!("+{}", num(x)), x / top, note));
                }
            }
            let outs = [
                ("армия", b.army + b.soldiers, "снаряжение и жалованье"),
                ("стройка", b.build + b.subsidy, "заводы казны и субсидии"),
                ("наука", b.science, "жалованье учёных"),
                ("гражданские статьи", b.civil, "чиновники, школы, врачи, порядок, призрение"),
                ("долг", b.bonds, "проценты и выкуп облигаций"),
                ("раздача людям", b.dole, "что осталось — людям на душу"),
            ];
            let mut outs: Vec<_> = outs.into_iter().filter(|x| x.1 > 0.005).collect();
            outs.sort_by(|a, c| c.1.total_cmp(&a.1));
            for (name, x, note) in outs.into_iter().take(4) {
                out.push(driver(name, format!("−{}", num(x)), -x / top, note));
            }
        }
        "output" => {
            let e = &g.effs[me];
            let rows: [(&str, f32, &str); 4] = [
                ("земля", e.out_land, "урожайность: особенности страны, законы, программы"),
                ("заводы и рудники", e.out_industry, "производительность заводов"),
                ("арсеналы", e.out_military, "производительность военных заводов"),
                ("товары людям", e.out_civil, "производительность отраслей для людей"),
            ];
            for (name, x, note) in rows {
                out.push(driver(name, format!("{:+.0}%", (x - 1.0) * 100.0), (x - 1.0) / 0.2, note));
            }
            let cap: f32 = n.cap.iter().sum();
            let worked: f32 = n.worked.iter().sum();
            let wasted = if cap > 1e-6 { (1.0 - worked / cap).clamp(0.0, 1.0) } else { 0.0 };
            out.push(driver("простой мест", format!("{:.0}%", wasted * 100.0), -wasted / 0.3, "рабочие места, что стоят без дела: нет сырья или покупателей"));
            out.push(driver("без работы", format!("{:.0}%", g.figures(me).idle * 100.0), -(g.figures(me).idle - 0.05) / 0.2, "свободные руки, которых не взяли заводы"));
        }
        "prices" => {
            let mg = g.money_growth(me);
            out.push(driver("деньги против товаров", format!("{:+.0}% за год", (mg - f.growth) * 100.0), -(mg - f.growth) / 0.15, "если денег прибывает быстрее, чем товаров, цены растут"));
            let pr = g.fiscal.print[me];
            out.push(driver("печатный станок", PRINTS[pr].name.to_lowercase(), -(pr as f32) / 3.0, PRINTS[pr].about));
            let rt = g.fiscal.rate[me];
            out.push(driver("кредит", RATES[rt].name.to_lowercase(), [-0.6, 0.0, 0.5, 0.7][rt], RATES[rt].about));
            let tf = g.fiscal.tariff[me];
            out.push(driver("пошлины", TARIFFS[tf].name.to_lowercase(), -(tf as f32) * 0.25, TARIFFS[tf].about));
            let mut dear: Vec<(String, f32)> = (0..pack.goods.len()).map(|k| (pack.goods[k].name.to_lowercase(), n.price[k] / (pack.goods[k].price * n.level).max(1e-9))).filter(|x| x.1 > 1.3).collect();
            dear.sort_by(|a, c| c.1.total_cmp(&a.1));
            let names = dear.iter().take(2).map(|x| format!("{} ×{}", x.0, fix(x.1, 1))).collect::<Vec<_>>().join(", ");
            out.push(driver("нехватка товаров", if dear.is_empty() { "нет".to_string() } else { names }, -(dear.len() as f32) / 4.0, "дорогое против обычного — значит, не хватает: строить там"));
        }
        "jobs" => {
            for (gi, grp) in pack.groups.iter().enumerate() {
                if let Some(u) = n.unemployed[gi] {
                    out.push(driver(grp.name.to_lowercase(), format!("{:.0}% без работы", u * 100.0), -(u - 0.03) / 0.25, "доля этой группы, которой нет места"));
                }
            }
            let lb = g.fiscal.labour[me];
            out.push(driver("закон о труде", LABOUR[lb].name.to_lowercase(), 0.0, LABOUR[lb].about));
            out.push(driver("стройка казны", format!("{:.0}% остатка трат", n.build_share * 100.0), n.build_share - 0.3, "казна строит заводы — новые места"));
        }
        "want" => {
            let mut rows: Vec<(String, f32)> = Vec::new();
            for (gi, grp) in pack.groups.iter().enumerate() {
                if n.pop[gi] <= 1e-6 {
                    continue;
                }
                for (k, need) in grp.needs.iter().enumerate() {
                    let miss = n.unmet[gi][k];
                    if miss > 0.01 && (need.vital || miss > 0.15) {
                        rows.push((format!("{} — {}", pack.goods[need.good].name.to_lowercase(), grp.name.to_lowercase()), miss));
                    }
                }
            }
            rows.sort_by(|a, c| c.1.total_cmp(&a.1));
            if rows.is_empty() {
                out.push(driver("всем хватает нужного", "—", 0.6, "самое нужное достаётся всем группам"));
            }
            for (name, miss) in rows.into_iter().take(6) {
                out.push(driver(name, format!("{:.0}% не хватает", miss * 100.0), -miss / 0.3, "доля потребности, которую группа не покрыла"));
            }
            let cap = g.fiscal.caps[me];
            out.push(driver("твёрдые цены", if cap { "введены" } else { "нет" }, 0.0, "потолок цен на нужное: легче бедным, но чего не хватает — делят в очередях"));
        }
        _ => {
            for (gi, grp) in pack.groups.iter().enumerate() {
                if n.pop[gi] <= 1e-6 {
                    continue;
                }
                let near = n.discontent[gi] / grp.patience.max(1e-6);
                let note = format!("{}: недовольство против терпения; переполнится — волнения{}", grp.name, if n.unrest[gi] > 0 { " (идут сейчас)" } else { "" });
                out.push(driver(grp.name.to_lowercase(), if n.unrest[gi] > 0 { "восстали".to_string() } else { format!("{:.0}% терпения", near * 100.0) }, -near, note));
            }
            if g.h.at_war {
                out.push(driver("война", format!("призвано {:.1}% людей", f.drafted * 100.0), -(f.drafted / 0.05), "потери и призыв ложатся на семьи"));
            }
        }
    }
    out
}

/// A cause as a row: its name at the left, a bar from the middle (green to the right where it helps,
/// red to the left where it harms), its figure at the right; the explanation on its card.
fn driver_row(ui: &mut egui::Ui, s: &Skin, d: &Driver) {
    let (ch, _) = chrome::dress(ui);
    let w = ui.available_width().min(ui.clip_rect().max.x - ui.cursor().min.x - 2.0);
    let (r, resp) = ui.allocate_exact_size(vec2(w, 24.0), egui::Sense::hover());
    if ui.is_rect_visible(r) {
        let name_w = (w * 0.38).min(160.0);
        let val_w = (w * 0.30).min(112.0);
        let bar = egui::Rect::from_min_max(egui::pos2(r.min.x + name_w + 4.0, r.center().y - 4.0), egui::pos2(r.max.x - val_w - 4.0, r.center().y + 4.0));
        let mut v = chrome::Out::new();
        v.push(egui::Shape::rect_filled(bar, 2.0, Color32::from_black_alpha(34)));
        ch.bevel(&mut v, bar, false, 1.0);
        let mid = bar.center().x;
        let len = bar.width() / 2.0 * d.good.abs();
        if len > 0.6 {
            let col = if d.good > 0.0 { kit::GOOD } else { kit::BAD };
            let f = if d.good > 0.0 { egui::Rect::from_min_max(egui::pos2(mid, bar.min.y + 1.0), egui::pos2(mid + len, bar.max.y - 1.0)) } else { egui::Rect::from_min_max(egui::pos2(mid - len, bar.min.y + 1.0), egui::pos2(mid, bar.max.y - 1.0)) };
            v.push(egui::Shape::rect_filled(f, 1.0, col));
        }
        v.push(egui::Shape::line_segment([egui::pos2(mid, bar.min.y - 2.0), egui::pos2(mid, bar.max.y + 2.0)], egui::Stroke::new(1.0, ch.trim)));
        ui.painter().extend(v);
        let mut job = egui::text::LayoutJob::simple_singleline(d.name.clone(), egui::FontId::proportional(13.5), s.ink);
        job.wrap = egui::text::TextWrapping::truncate_at_width(name_w);
        let gal = ui.painter().layout_job(job);
        ui.painter().galley(egui::pos2(r.min.x + 2.0, r.center().y - gal.size().y / 2.0), gal, s.ink);
        let mut job = egui::text::LayoutJob::simple_singleline(d.value.clone(), egui::FontId::new(13.0, egui::FontFamily::Name("bold".into())), s.ink);
        job.wrap = egui::text::TextWrapping::truncate_at_width(val_w);
        let gal = ui.painter().layout_job(job);
        ui.painter().galley(egui::pos2(r.max.x - 2.0 - gal.size().x, r.center().y - gal.size().y / 2.0), gal, s.ink);
    }
    resp.tip(|| Card::new(d.name.clone()).row("сейчас", d.value.clone()).tone("влияет", if d.good > 0.05 { "помогает" } else if d.good < -0.05 { "вредит" } else { "почти не влияет" }, if d.good > 0.05 { Tone::Good } else if d.good < -0.05 { Tone::Bad } else { Tone::Plain }).text(d.note.clone()));
}

// ---- the levers --------------------------------------------------------------------------------

/// What a year of a step does to the figures the vital sign cares about: each as a word and a number
/// coloured by whether it is good news.
fn effects(vital: &str, o: &Outcome, base: &Outcome) -> Vec<(String, i8)> {
    // (name, change, how it is written, whether more is good — none: neither, below which it does not count)
    type Metric = (&'static str, f32, fn(f32) -> String, Option<bool>, f32);
    let ratio = |a: f32, b: f32| a / b.max(1e-9) - 1.0;
    let taxes: Metric = ("налоги", o.tax - base.tax, |d| format!("{}/мес", sfix(d, 1)), Some(true), 0.3);
    let army: Metric = ("армии", o.army_money - base.army_money, |d| format!("{}/мес", sfix(d, 1)), None, 0.3);
    let output: Metric = ("выпуск", ratio(o.gdp, base.gdp), |d| format!("{}%", sfix(d * 100.0, 1)), Some(true), 0.002);
    let anger: Metric = ("недовольство", o.anger - base.anger, |d| sfix(d, 2), Some(false), 0.02);
    let idle: Metric = ("без работы", o.idle - base.idle, |d| format!("{} п.", sfix(d * 100.0, 1)), Some(false), 0.003);
    let prices: Metric = ("цены", ratio(o.level, base.level), |d| format!("{}%", sfix(d * 100.0, 1)), Some(false), 0.003);
    let want: Metric = ("нужда", o.hunger - base.hunger, |d| format!("{} п.", sfix(d * 100.0, 1)), Some(false), 0.003);
    let chosen: Vec<Metric> = match vital {
        "treasury" => vec![taxes, army, output, anger],
        "output" => vec![output, idle, prices],
        "prices" => vec![prices, output, anger],
        "jobs" => vec![idle, output, taxes],
        "want" => vec![want, anger, taxes],
        _ => vec![anger, want, taxes],
    };
    chosen
        .into_iter()
        .filter(|m| m.1.abs() >= m.4)
        .map(|m| (format!("{} {}", m.0, (m.2)(m.1)), match m.3 { Some(up_good) => if (m.1 > 0.0) == up_good { 1 } else { -1 }, None => 0 }))
        .collect()
}

/// The line under a lever: «−5 п.: налоги −9,0/мес · армии −3,7/мес · выпуск +0,3%» — what a year of the step
/// does, each figure coloured by whether it is good news.
fn effect_line(ui: &mut egui::Ui, s: &Skin, vital: &str, f: &forecast::Forecast, key: &str, what: &str) {
    let Some((o, base)) = f.pair(key) else { return };
    let tokens = effects(vital, o, base);
    let font = egui::FontId::proportional(12.5);
    let bold = egui::FontId::new(12.5, egui::FontFamily::Name("bold".into()));
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    job.append(&format!("{what}:  "), 0.0, egui::TextFormat { font_id: bold.clone(), color: s.ink, ..Default::default() });
    if tokens.is_empty() {
        job.append("почти ничего не изменит", 0.0, egui::TextFormat { font_id: font.clone(), color: s.soft, ..Default::default() });
    }
    for (i, (t, tone)) in tokens.iter().enumerate() {
        if i > 0 {
            job.append("  ·  ", 0.0, egui::TextFormat { font_id: font.clone(), color: s.soft, ..Default::default() });
        }
        let col = match tone {
            1 => kit::ink(s, kit::GOOD),
            -1 => kit::ink(s, kit::BAD),
            _ => s.ink,
        };
        job.append(t, 0.0, egui::TextFormat { font_id: bold.clone(), color: col, ..Default::default() });
    }
    ui.add(egui::Label::new(job).wrap());
}

/// A lever of the vital sign.
enum Lever {
    /// A number moved by − and +: what the decision form is called, the figure now, the figures a
    /// step down and a step up lead to; the forecast's keys for the step down and up and what they are
    /// called in words.
    Step { name: &'static str, icon: &'static str, key: &'static str, now: String, down: Option<String>, up: Option<String>, enabled: bool, about: String, fx: [(&'static str, &'static str); 2], spot: Option<&'static str> },
    /// A law with four choices: a step to either side.
    Pick { name: &'static str, icon: &'static str, key: &'static str, list: &'static [Choice; 4], now: usize, fx: [(&'static str, &'static str); 2] },
    /// A switch.
    Switch { name: &'static str, icon: &'static str, key: &'static str, on: bool, about: String, fx: [(&'static str, &'static str); 1] },
    /// Another page of the folder.
    Open { name: &'static str, icon: &'static str, page: usize, focus: Option<&'static str>, button: &'static str, about: &'static str },
}

const NONE: (&str, &str) = ("", "");

/// The levers of a vital sign.
fn levers(g: &Game, key: &str) -> Vec<Lever> {
    let me = g.me;
    let n = g.h.home(me);
    let mine = g.adviser.is_none();
    let tax = (n.tax * 100.0).round();
    let army = (n.mil_share * 100.0).round();
    let build = (n.build_share * 100.0).round();
    let sci = (n.science_share * 100.0 * 10.0).round() / 10.0;
    let step = |name, icon, key, now: f32, d: f32, lo: f32, hi: f32, enabled: bool, about: String, fx, spot| Lever::Step {
        name,
        icon,
        key,
        now: if key == "science" { format!("{now:.1}%") } else { format!("{now:.0}%") },
        // (the shares are written as whole points, but for the science's)
        down: (now - d >= lo - 1e-3).then(|| if key == "science" { format!("{:.1}", now - d) } else { format!("{:.0}", now - d) }),
        up: (now + d <= hi + 1e-3).then(|| if key == "science" { format!("{:.1}", now + d) } else { format!("{:.0}", now + d) }),
        enabled,
        about,
        fx,
        spot,
    };
    let tax_l = step("Налог", "stamper", "tax", tax, 1.0, 0.0, 80.0, mine, if mine { format!("сколько казна берёт с доходов; +1 п. ≈ +{} в месяц, но людям тяжелее", num(n.gdp * 0.01)) } else { "налог ведёт советник".into() }, [("tax-", "−5 п."), ("tax+", "+5 п.")], Some("ov_tax_plus"));
    let build_l = step("Стройка казны", "anvil", "build", build, 10.0, 0.0, 100.0, mine, if mine { "доля остатка трат, что идёт в новые заводы".into() } else { "стройку ведёт советник".into() }, [("build-", "не строить"), ("build+", "строить всё")], None);
    let army_l = step("Армия", "army", "army", army, 2.0, 0.0, 100.0, true, "доля трат казны на снаряжение и жалованье".into(), [("army-", "−10 п."), ("army+", "+10 п.")], None);
    let sci_l = step("Наука", "science", "science", sci, 1.0, 0.0, 50.0, true, "жалованье учёных: чем больше, тем больше образованных идёт в лаборатории".into(), [NONE, ("sci+", "вдвое больше")], None);
    let tariff = Lever::Pick { name: "Пошлины", icon: "wall", key: "fx_tariff", list: &TARIFFS, now: g.fiscal.tariff[me], fx: [("tariff-", "свободная торговля"), ("tariff+", "защита промышленности")] };
    let rate = Lever::Pick { name: "Кредит", icon: "coins", key: "fx_rate", list: &RATES, now: g.fiscal.rate[me], fx: [("rate-", "дешёвый кредит"), ("rate+", "дорогие деньги")] };
    let print = Lever::Pick { name: "Печатный станок", icon: "stamper", key: "fx_print", list: &PRINTS, now: g.fiscal.print[me], fx: [NONE, NONE] };
    let dole = Lever::Pick { name: "Кому раздаёт казна", icon: "bread", key: "fx_dole", list: &DOLES, now: g.fiscal.dole[me], fx: [NONE, NONE] };
    let labour = Lever::Pick { name: "Закон о труде", icon: "hand", key: "fx_labour", list: &LABOUR, now: g.fiscal.labour[me], fx: [NONE, NONE] };
    let caps = Lever::Switch { name: "Твёрдые цены на хлеб", icon: "bread", key: "fx_caps", on: g.fiscal.caps[me], about: "потолок цены на нужное: бедным легче, но чего не хватает — делят в очередях".into(), fx: [("caps+", "ввести")] };
    match key {
        "treasury" => {
            // (the science's share is a lever only where the age has laboratories)
            let mut v = vec![tax_l, army_l, build_l];
            if g.h.labs[me].is_some() {
                v.push(sci_l);
            }
            v.push(tariff);
            v
        }
        "output" => vec![build_l, Lever::Open { name: "Модернизация отраслей", icon: "gears", page: economy::ECONOMY, focus: None, button: "Открыть", about: "новые станки одной отрасли — полтора года работ за деньги казны" }, Lever::Open { name: "Школы", icon: "graduate", page: economy::BUDGET, focus: Some("civil"), button: "Открыть", about: "грамотные становятся служащими и учёными — через годы растут заводы и наука" }, tariff, tax_l],
        "prices" => vec![rate, print, tariff, caps],
        "jobs" => vec![build_l, labour, Lever::Open { name: "Субсидии отраслям", icon: "coins", page: economy::MONEY, focus: None, button: "Открыть", about: "деньги казны отрасли: она держит места и строится быстрее" }],
        "want" => vec![caps, dole, tax_l, Lever::Open { name: "Рынок", icon: "shop", page: economy::ECONOMY, focus: None, button: "Открыть", about: "что дорого — того не хватает, туда строить" }],
        _ => vec![tax_l, dole, caps, Lever::Open { name: "Призрение и порядок", icon: "shield", page: economy::BUDGET, focus: Some("civil"), button: "Открыть", about: "гражданские статьи: смута гаснет быстрее, бедным легче" }],
    }
}

/// What a lever's row asks the game to do, after the rows are drawn.
enum Ask {
    Form(Vec<(&'static str, String)>),
    Page(usize, Option<&'static str>),
}

fn row(ui: &mut egui::Ui, s: &Skin, vital: &str, f: &forecast::Forecast, lv: &Lever, asked: &mut Option<Ask>) {
    match lv {
        Lever::Step { name, icon, key, now, down, up, enabled, about, fx, spot } => {
            let (d, plus) = levers::step(ui, s, icon, name, now, *enabled, about);
            if let Some(sp) = spot {
                if ui.clip_rect().contains_rect(plus) {
                    chrome::spot(ui.ctx(), sp, plus);
                }
            }
            // (the spoiled control of the self-checks: the + of the tax does nothing)
            if !super::super::super::spoiled() || *key != "tax" {
                match d {
                    -1 => *asked = down.clone().map(|v| Ask::Form(vec![(*key, v)])),
                    1 => *asked = up.clone().map(|v| Ask::Form(vec![(*key, v)])),
                    _ => {}
                }
            }
            for (k, what) in fx.iter().filter(|x| !x.0.is_empty()) {
                effect_line(ui, s, vital, f, k, what);
            }
        }
        Lever::Pick { name, icon, key, list, now, fx } => {
            if let Some(k) = levers::pick(ui, s, icon, name, list, *now) {
                *asked = Some(Ask::Form(vec![(*key, k.to_string())]));
            }
            for (k, what) in fx.iter().filter(|x| !x.0.is_empty()) {
                effect_line(ui, s, vital, f, k, what);
            }
        }
        Lever::Switch { name, icon, key, on, about, fx } => {
            if levers::switch(ui, s, icon, name, *on, ["Нет", "Введены"], true, about) {
                *asked = Some(Ask::Form(vec![(*key, if *on { "0".into() } else { "1".into() })]));
            }
            if !*on {
                for (k, what) in fx.iter().filter(|x| !x.0.is_empty()) {
                    effect_line(ui, s, vital, f, k, what);
                }
            }
        }
        Lever::Open { name, icon, page, focus, button, about } => {
            if levers::open(ui, s, icon, name, button, about) {
                *asked = Some(Ask::Page(*page, *focus));
            }
        }
    }
    levers::rule(ui, s);
}

// ---- the whole ---------------------------------------------------------------------------------

/// The chosen vital sign laid open.
pub(super) fn detail(ui: &mut egui::Ui, s: &Skin, g: &mut Game, vs: &[Vital], k: usize) {
    let v = &vs[k];
    kit::ribbon(ui, &format!("Разбор: {}", v.name), Some(v.icon));
    kit::words(ui, v.what, 13.0, s.soft);
    ui.add_space(3.0);
    chart(ui, s, g, v);
    ui.add_space(4.0);
    // what presses on it
    kit::bold(ui, "Что на него влияет", 14.0, s.ink);
    kit::words(ui, "Справа от черты — что помогает, слева — что вредит; наведите на строку.", 12.5, s.soft);
    for d in drivers(g, v.key) {
        driver_row(ui, s, &d);
    }
    ui.add_space(6.0);
    // the levers, and the forecast of what a step does in a year
    kit::bold(ui, "Чем его двигать", 14.0, s.ink);
    let f = forecast::state();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if f.running {
            ui.label(RichText::new("Считаю, что даст каждый шаг: мир проживается на год вперёд…").size(12.5).color(s.soft));
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
        } else if f.rows.is_empty() {
            if widgets::small(ui, "Посчитать, что даст шаг").tip(|| Card::new("Прогноз").icon("forecast").text("Тот же мир проживается на год вперёд с каждым решением и без него: сосед, рынок и случай тоже живут. Займёт несколько секунд; игра не останавливается.")).clicked() {
                forecast::start(g);
            }
        } else {
            let note = if f.stale(g) { format!("Прогноз от {} — мир с тех пор изменился:", crate::words::date_words(f.month)) } else { format!("Что даст шаг через год (войн в прогнозе нет), {:.0} с:", f.secs) };
            ui.label(RichText::new(note).size(12.5).color(if f.stale(g) { kit::ink(s, kit::MIXED) } else { s.soft }));
            if widgets::small(ui, "Пересчитать").tip(|| Card::new("Прогноз").icon("forecast").text("Заново прожить год вперёд с теми же решениями. Займёт несколько секунд.")).clicked() {
                forecast::start(g);
            }
        }
    });
    let mut asked: Option<Ask> = None;
    let list = levers(g, v.key);
    for lv in &list {
        row(ui, s, v.key, &f, lv, &mut asked);
    }
    match asked {
        Some(Ask::Form(pairs)) => act(g, &pairs),
        Some(Ask::Page(p, focus)) => {
            set_page(ui.ctx(), Tab::Econ, p);
            if let Some(fo) = focus {
                super::super::super::reports::light(ui.ctx(), fo);
            }
        }
        None => {}
    }
}
