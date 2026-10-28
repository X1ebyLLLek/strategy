//! The folder of the economy: the levers of the state, the charts of a pair of countries,
//! the market's goods, the sectors, the treasury.

use super::*;

/// A lever of the state as a law: five choices (each a share of the state's spending: `levels`
/// times the age's usual one, or the shares themselves when `plain`), side by side as tiles with
/// their names and icons, the one nearest the present share chosen; the numbers on each choice's
/// card. A click sends the choice to the government.
#[allow(clippy::too_many_arguments)]
pub(super) fn lever(ui: &mut egui::Ui, s: &Skin, label: &str, value: f32, max: f32, usual: f32, names: [&str; 5], icons: [&str; 5], levels: [f32; 5], plain: bool, key: &str, g: &mut Game) {
    let shares: Vec<f32> = levels.iter().map(|&l| (if plain { l } else { l * usual }).min(max / 100.0)).collect();
    let chosen = (0..5).min_by(|&a, &b| (shares[a] - value).abs().total_cmp(&(shares[b] - value).abs())).unwrap_or(2);
    let usual_k = (0..5).min_by(|&a, &b| (shares[a] - usual).abs().total_cmp(&(shares[b] - usual).abs())).unwrap_or(2);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        ui.label(RichText::new(label).size(15.0).color(s.soft));
        ui.label(RichText::new(names[chosen]).family(egui::FontFamily::Name("bold".into())).size(15.0).color(s.ink));
    });
    let choices: Vec<(&str, &str)> = names.iter().zip(icons.iter()).map(|(a, b)| (*a, *b)).collect();
    let now = value;
    let picked = widgets::law(ui, &choices, chosen, usual_k, &|k| {
        Card::new(names[k]).icon(icons[k]).sub(label.trim_end_matches(':').to_string()).row("доля трат", format!("{:.0}%", shares[k] * 100.0)).row("сейчас", format!("{:.0}%", now * 100.0)).row("обычно в эпохе", format!("{:.0}%", usual * 100.0)).note(if k == chosen { "выбрано" } else { "щёлкните — решение уйдёт в правительство" })
    });
    if let Some(k) = picked {
        act(g, &[(key, format!("{:.0}", shares[k] * 100.0))]);
    }
    ui.add_space(4.0);
}

/// A chart of a pair of the record's numbers over the months (ours and the rival's), with its
/// heading; the numbers on the card under the hand.
pub(super) fn pair_chart(ui: &mut egui::Ui, s: &Skin, g: &Game, title: &str, what: &str, get: &dyn Fn(&crate::game::Row, usize) -> f32, fmt: &dyn Fn(f32) -> String) {
    let me = g.me;
    let rows: Vec<&crate::game::Row> = g.rows.iter().rev().take(240).collect::<Vec<_>>().into_iter().rev().collect();
    let months: Vec<u32> = rows.iter().map(|r| r.month).collect();
    let rival = g.h.war.th.countries[1 - me];
    let rc = g.countries[rival].1;
    let theirs = Color32::from_rgb(rc[0], rc[1], rc[2]);
    let theirs = super::super::hud::darken(theirs, 0.25);
    let series = vec![
        (g.h.home(me).name.clone(), s.accent, rows.iter().map(|r| get(r, me)).collect::<Vec<f32>>()),
        (g.h.home(1 - me).name.clone(), theirs, rows.iter().map(|r| get(r, 1 - me)).collect::<Vec<f32>>()),
    ];
    ui.horizontal(|ui| {
        ui.label(RichText::new(title.to_uppercase()).family(egui::FontFamily::Name("head".into())).size(13.0).color(s.soft));
        for (name, col, _) in &series {
            let (r, _) = ui.allocate_exact_size(vec2(14.0, 10.0), egui::Sense::hover());
            ui.painter().rect_filled(r.shrink2(vec2(0.0, 2.0)), 1.0, *col);
            ui.label(RichText::new(name).size(13.0).color(s.soft));
        }
    });
    let card = |i: usize| {
        let r = rows[i];
        Card::new(title).icon("scroll").sub(crate::words::date_words(r.month)).row(g.h.home(me).name.clone(), fmt(get(r, me))).row(g.h.home(1 - me).name.clone(), fmt(get(r, 1 - me))).note(what)
    };
    widgets::chart(ui, &series, &months, 74.0, false, &card);
    ui.add_space(4.0);
}

/// The pages of the economy's folder (28th session).
pub const PAGES: [&str; 4] = ["Бюджет", "Хозяйство", "Деньги", "Люди"];

/// The economy, page by page: the budget (the treasury's sheet with its levers, what the state
/// builds, the civil lines, who runs the economy); the economy itself (figures, charts, what weighs
/// on it, the market, the sectors, modernisation); the money (the state's finances, the banks and
/// the debt); the people.
pub fn econ(ui: &mut egui::Ui, s: &Skin, g: &mut Game, page: usize) {
    match page {
        1 => economy(ui, s, g),
        2 => {
            super::finance::finance(ui, s, g);
            super::budget::money(ui, s, g);
        }
        3 => super::budget::people(ui, s, g),
        _ => budget(ui, s, g),
    }
}

/// The page «Бюджет».
fn budget(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    super::sheet::sheet(ui, s, g);
    if g.adviser.is_none() {
        head(ui, s, "Что строит казна");
        words(ui, "Отмеченные отрасли казна строит по очереди, месяц за месяцем; ничего не отмечено — где людям хуже всего. Цена против обычной: дороже ×1,3 — товара не хватает.", 13.5, s.soft);
        ui.add_space(3.0);
        let mut queue = g.state.queue.clone();
        let mut changed = false;
        for (k, sec) in pack.sectors.iter().enumerate() {
            let n = g.h.home(me);
            let ratio = sec.outputs.first().map_or(1.0, |o| n.price[o.0] / (pack.goods[o.0].price * n.level).max(1e-9));
            let word = if ratio > 1.3 { "не хватает" } else if ratio < 0.8 { "в избытке" } else { "как обычно" };
            let (jobs, busy) = (n.cap.get(k).copied().unwrap_or(0.0), n.worked.get(k).copied().unwrap_or(0.0));
            let now_building = n.build_focus == Some(k);
            let mut on = queue.contains(&k);
            ui.horizontal(|ui| {
                if widgets::check(ui, &mut on, &sec.name).changed() {
                    changed = true;
                }
                let name = sec.name.clone();
                ui.label(RichText::new(format!("×{ratio:.2} · {word}{}", if now_building { " · строится" } else { "" })).size(13.5).color(if ratio > 1.3 { s.accent } else { s.soft }))
                    .tip(|| {
                        Card::new(name)
                            .icon("gears")
                            .tone("цена к обычной", format!("×{ratio:.2}"), if ratio > 1.3 { Tone::Bad } else { Tone::Plain })
                            .row("рабочих мест", format!("{:.2} млн", jobs))
                            .row("из них заняты", format!("{:.0}%", busy / jobs.max(1e-9) * 100.0))
                            .note(if now_building { "казна строит её в этом месяце" } else { "дорого — значит, не хватает" })
                    });
            });
            if on && !queue.contains(&k) {
                queue.push(k);
            } else if !on {
                queue.retain(|&x| x != k);
            }
        }
        if changed {
            let mut pairs: Vec<(&str, String)> = vec![("queue_set", "1".into())];
            let keys: Vec<String> = queue.iter().map(|k| format!("q{k}")).collect();
            for key in &keys {
                pairs.push((key.as_str(), "1".into()));
            }
            act(g, &pairs);
        }
    }
    super::budget::civil(ui, s, g);
    head(ui, s, "Кто правит хозяйством");
    say(ui, s, "Хозяйство можно вести самому или отдать [adviser]советнику.");
    ui.add_space(3.0);
    let adviser = g.adviser;
    if person(ui, s, g, "Вы сами", "me", adviser.is_none(), "налог, стройку и что строить решаете вы") {
        act(g, &[("econ", "me".into())]);
    }
    for k in 0..g.pool.len() {
        let name = g.pool[k].0.clone();
        let on = adviser == Some(k);
        // (what anyone may know of him: his schooling, family, views, his last deeds; in office —
        // what he does and what his years came to)
        let mut body = g.roster.pool.get(k).map_or(String::new(), |&id| candidate_words(g, id, false));
        if on {
            if let Some(e) = g.h.econ[me].as_ref() {
                body = format!("{}\n{}", crate::words::levers_words(&e.levers, &pack), body);
                for y in e.years.iter().rev().take(3) {
                    body += &format!("\n{}-й год при нём: выпуск {:+.1}%, благосостояние {:+.1}%, волнений {}", y.year, y.output * 100.0, y.welfare * 100.0, y.unrest);
                }
                if e.years.is_empty() {
                    body += "\nитоги его первого года — через год";
                }
            }
        }
        if person(ui, s, g, &name, "econ", on, &body) {
            act(g, &[("econ", format!("adv{k}"))]);
        }
    }
    ui.add_space(6.0);
    words(ui, format!("У {} хозяйством правит {}.", g.h.home(1 - me).name, g.foe_adviser), 13.5, s.soft);
}


/// The page «Хозяйство»: the figures and charts, what weighs on the economy, the market, the
/// sectors, modernisation.
fn economy(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let n = g.h.home(me);
    let mood = match n.anger() {
        a if a < 0.5 => "невелико",
        a if a < 1.5 => "растёт",
        _ => "велико",
    };
    let (pp, an) = (n.people(), n.anger());
    {
        use super::super::kit::{self, Stat};
        let f = &g.spirits.facts[me];
        let idle: f32 = {
            let (mut out, mut all) = (0.0f32, 0.0f32);
            for (k, u) in n.unemployed.iter().enumerate() {
                if let Some(u) = u {
                    out += u * n.pop[k];
                    all += n.pop[k];
                }
            }
            out / all.max(1e-9)
        };
        let crowd_w = match pp {
            x if x < 5.0 => "малолюдна",
            x if x < 40.0 => "средняя",
            x if x < 150.0 => "многолюдна",
            _ => "огромна",
        };
        let items = vec![
            Stat::new("people", "Людей", crowd_w, kit::PLAIN),
            Stat::new("revolt", "Недовольство", mood, if an < 0.5 { kit::GOOD } else if an < 1.5 { kit::MIXED } else { kit::BAD }),
            Stat::new("sprout", "Выпуск за год", kit::change(f.growth, "растёт", "падает"), if f.growth >= 0.0 { kit::GOOD } else { kit::BAD }),
            Stat::new("coins", "Цены за год", kit::change(f.inflation, "растут", "падают"), if f.inflation > 0.08 { kit::BAD } else { kit::PLAIN }),
            Stat::new("hand", "Без работы", if idle < 0.02 { "почти никого" } else if idle < 0.08 { "немногие" } else if idle < 0.2 { "многие" } else { "очень многие" }, if idle > 0.1 { kit::BAD } else { kit::GOOD }),
            Stat::new("backforth", "Из-за границы", if f.imports < 0.03 { "почти ничего" } else if f.imports < 0.12 { "немного" } else if f.imports < 0.3 { "заметно" } else { "много" }, kit::PLAIN),
        ];
        let nums = [format!("{pp:.1} млн"), format!("{an:.2}"), format!("{:+.1}%", f.growth * 100.0), format!("{:+.1}%", f.inflation * 100.0), format!("{:.0}%", idle * 100.0), format!("{:.0}%", f.imports * 100.0)];
        let items: Vec<Stat> = items.into_iter().zip(nums.iter()).map(|(it, n)| it.num(n.clone())).collect();
        let resp = kit::stats(ui, &items, 3);
        let tips: [&str; 6] = [
            "людей в стране, включая тех, кто служит",
            "чем выше, тем ближе волнения: голод, налоги, война, дороговизна",
            "выпуск в неизменных ценах против того, что был год назад",
            "уровень цен против прошлого года",
            "доля тех, кто ищет работу и не находит",
            "доля купленного в стране, что пришло из-за границы",
        ];
        for (i, r) in resp.into_iter().enumerate() {
            let (t, v) = (items[i].title.clone(), nums[i].clone());
            r.tip(|| Card::new(t).icon(items[i].icon.clone()).row("сейчас", v).note(tips[i]));
        }
    }
    pair_chart(ui, s, g, "Выпуск", "выпуск в неизменных ценах", &|r, k| r.gdp[k], &|v| format!("{v:.1}"));
    pair_chart(ui, s, g, "Недовольство", "чем выше, тем ближе волнения", &|r, k| r.anger[k], &|v| format!("{v:.2}"));
    influences(ui, s, g);
    market(ui, s, g);
    sectors(ui, s, g);
    modern(ui, s, g);
}

//// The market, as Victoria's: a tile for each good — its icon and name, how dear it is against
/// the usual (a gauge from cheap to dear, an arrow if it rises or falls), whether it comes from
/// abroad — the numbers on the tile's card.
fn market(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    let me = g.me;
    let pack = &g.pack;
    let n = g.h.home(me);
    let icons = chrome::icons(ui.ctx());
    head(ui, s, "Рынок");
    let cols = 2usize;
    let gap = 6.0;
    let w = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).floor();
    let h = 52.0;
    let goods: Vec<usize> = (0..pack.goods.len()).collect();
    let rows = goods.len().div_ceil(cols);
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), rows as f32 * (h + gap) - gap), egui::Sense::hover());
    let ch = chrome::of(ui.ctx());
    for (i, &k) in goods.iter().enumerate() {
        let r = egui::Rect::from_min_size(area.min + vec2((i % cols) as f32 * (w + gap), (i / cols) as f32 * (h + gap)), vec2(w, h));
        let good = &pack.goods[k];
        let usual = (good.price * n.level).max(1e-9);
        let dear = n.price[k] / usual;
        let rising = n.price[k] / n.trend[k].max(1e-9);
        let made = n.produced.get(k).copied().unwrap_or(0.0);
        let import = n.import.get(k).copied().unwrap_or(0.0);
        let export = n.shipped.get(k).copied().unwrap_or(0.0);
        let sold = n.sold.get(k).copied().unwrap_or(0.0);
        let from_abroad = import / (import + made).max(1e-9);
        let resp = ui.interact(r, ui.id().with(("good", k)), egui::Sense::hover());
        let p = ui.painter();
        // (the tile: paper raised off the page, a band of how dear along its foot)
        if let Some(ch) = &ch {
            let mut v = chrome::Out::new();
            ch.lay(&mut v, r, &ch.paper, 2.0);
            ch.gloss(&mut v, r, Color32::from_white_alpha(if resp.hovered() { 70 } else { 45 }), Color32::from_black_alpha(30));
            ch.bevel(&mut v, r, true, 1.2);
            v.push(egui::Shape::rect_stroke(r, 2.0, egui::Stroke::new(1.0, Color32::from_black_alpha(110))));
            p.extend(v);
        }
        let ir = egui::Rect::from_min_size(r.min + vec2(6.0, 6.0), vec2(26.0, 26.0));
        super::super::look::icon(&icons, &good.icon, 26.0, s.ink).paint_at(ui, ir);
        let mut job = egui::text::LayoutJob::simple_singleline(good.name.clone(), egui::FontId::new(13.5, egui::FontFamily::Name("bold".into())), s.ink);
        job.wrap = egui::text::TextWrapping::truncate_at_width(w - 40.0);
        let gal = p.layout_job(job);
        p.galley(egui::pos2(ir.max.x + 5.0, r.min.y + 6.0), gal, s.ink);
        // how dear: cheap (green) … usual (the notch) … dear (red), on a log scale ×¼ … ×4
        let x = ((dear.max(1e-3).ln() / 4f32.ln()).clamp(-1.0, 1.0) + 1.0) / 2.0;
        let bar = egui::Rect::from_min_size(egui::pos2(ir.max.x + 5.0, r.min.y + 28.0), vec2(w - 46.0, 5.0));
        p.rect_filled(bar.expand(1.0), 1.5, Color32::from_black_alpha(80));
        let col = if dear > 1.3 { Color32::from_rgb(196, 70, 52) } else if dear < 0.77 { Color32::from_rgb(86, 150, 70) } else { Color32::from_rgb(200, 168, 80) };
        let mid = bar.min.x + bar.width() / 2.0;
        let at = bar.min.x + bar.width() * x;
        p.rect_filled(egui::Rect::from_min_max(egui::pos2(mid.min(at), bar.min.y), egui::pos2(mid.max(at).max(mid.min(at) + 2.0), bar.max.y)), 1.0, col);
        p.line_segment([egui::pos2(mid, bar.min.y - 2.0), egui::pos2(mid, bar.max.y + 2.0)], egui::Stroke::new(1.0, s.ink.gamma_multiply(0.7)));
        // the word, the arrow of the last months, the mark of goods from abroad
        let word = if dear > 2.0 { "очень дорого" } else if dear > 1.3 { "дорого" } else if dear < 0.5 { "очень дёшево" } else if dear < 0.77 { "дёшево" } else { "как обычно" };
        p.text(egui::pos2(ir.max.x + 5.0, r.max.y - 11.0), egui::Align2::LEFT_CENTER, word, egui::FontId::new(12.0, egui::FontFamily::Proportional), s.soft);
        if (rising - 1.0).abs() > 0.04 {
            let mut v = chrome::Out::new();
            super::super::hud::arrow(&mut v, egui::pos2(r.max.x - 10.0, r.min.y + 13.0), rising > 1.0, if rising > 1.0 { Color32::from_rgb(196, 70, 52) } else { Color32::from_rgb(86, 150, 70) });
            p.extend(v);
        }
        if from_abroad > 0.3 {
            super::super::look::icon(&icons, "backforth", 14.0, s.accent).paint_at(ui, egui::Rect::from_min_size(egui::pos2(r.max.x - 18.0, r.max.y - 18.0), vec2(14.0, 14.0)));
        }
        let by = n.bought_by.get(k).copied().unwrap_or([0.0; 5]);
        let all_by: f32 = by.iter().sum::<f32>().max(1e-9);
        let name = good.name.clone();
        let ic = good.icon.clone();
        resp.tip(|| {
            let mut c = Card::new(name)
                .icon(ic)
                .tone("цена к обычной", format!("×{dear:.2}"), if dear > 1.3 { Tone::Bad } else if dear < 0.77 { Tone::Good } else { Tone::Plain })
                .row("за последние месяцы", if rising > 1.04 { format!("дорожает (×{rising:.2})") } else if rising < 0.96 { format!("дешевеет (×{rising:.2})") } else { "почти не меняется".into() })
                .row("делают у нас", format!("{made:.2}"))
                .row("продано", format!("{sold:.2}"));
            if import > 0.005 {
                c = c.row("привозят", format!("{import:.2} ({:.0}% нужного)", from_abroad * 100.0));
            }
            if export > 0.005 {
                c = c.row("вывозят", format!("{export:.2}"));
            }
            for (j, who) in ["люди", "хозяева на стройку", "отрасли", "армия", "казна на стройку"].iter().enumerate() {
                if by[j] / all_by > 0.05 {
                    c = c.row(format!("покупают: {who}"), format!("{:.0}%", by[j] / all_by * 100.0));
                }
            }
            c.note("дорого — значит, не хватает; дёшево — в избытке")
        });
    }
    ui.add_space(4.0);
}

/// The sectors: where the people work — a bar each of its share of the jobs, how fully it works,
/// whether it pays (an arrow) — the numbers on the card.
fn sectors(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    let me = g.me;
    let pack = &g.pack;
    let n = g.h.home(me);
    let icons = chrome::icons(ui.ctx());
    head(ui, s, "Отрасли");
    let all: f32 = n.worked.iter().sum::<f32>().max(1e-9);
    let most = n.worked.iter().cloned().fold(0.0f32, f32::max).max(1e-9);
    for (k, sec) in pack.sectors.iter().enumerate() {
        let worked = n.worked.get(k).copied().unwrap_or(0.0);
        let cap = n.cap.get(k).copied().unwrap_or(0.0);
        let activity = n.activity.get(k).copied().unwrap_or(0.0);
        let profit = n.profit.get(k).copied().unwrap_or(0.0);
        let icon = sec.outputs.first().map_or("gears".to_string(), |o| pack.goods[o.0].icon.clone());
        let row = ui.horizontal(|ui| {
            let (ir, _) = ui.allocate_exact_size(vec2(20.0, 20.0), egui::Sense::hover());
            super::super::look::icon(&icons, &icon, 18.0, s.ink).paint_at(ui, ir);
            let mut job = egui::text::LayoutJob::simple_singleline(sec.name.clone(), egui::FontId::new(14.0, egui::FontFamily::Proportional), s.ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(132.0);
            let gal = ui.painter().layout_job(job);
            let (lr, _) = ui.allocate_exact_size(vec2(136.0, 20.0), egui::Sense::hover());
            ui.painter().galley(egui::pos2(lr.min.x, lr.center().y - gal.size().y / 2.0), gal, s.ink);
            // its people: a bar as long as its share of the largest sector's
            let (br, _) = ui.allocate_exact_size(vec2((ui.available_width() - 44.0).max(40.0), 20.0), egui::Sense::hover());
            let p = ui.painter();
            let b = egui::Rect::from_min_size(egui::pos2(br.min.x, br.center().y - 4.0), vec2(br.width(), 8.0));
            p.rect_filled(b.expand(1.0), 1.5, Color32::from_black_alpha(70));
            p.rect_filled(egui::Rect::from_min_size(b.min, vec2(b.width() * worked / most, b.height())), 1.0, s.accent.gamma_multiply(0.85));
            // how fully it works (the part of its jobs idle shaded)
            if cap > 0.0 {
                let idle = (1.0 - activity).clamp(0.0, 1.0);
                let wb = b.width() * worked / most;
                p.rect_filled(egui::Rect::from_min_size(egui::pos2(b.min.x + wb * (1.0 - idle), b.min.y), vec2(wb * idle, b.height())), 1.0, Color32::from_black_alpha(90));
            }
            let mut v = chrome::Out::new();
            if profit.abs() > 1e-4 {
                super::super::hud::arrow(&mut v, egui::pos2(br.max.x + 12.0, br.center().y), profit > 0.0, if profit > 0.0 { Color32::from_rgb(86, 150, 70) } else { Color32::from_rgb(196, 70, 52) });
            }
            p.extend(v);
        });
        let name = sec.name.clone();
        row.response.tip(|| {
            Card::new(name)
                .icon(icon.clone())
                .row("работают", format!("{worked:.2} млн ({:.0}% всех)", worked / all * 100.0))
                .row("рабочих мест", format!("{cap:.2} млн"))
                .tone("загрузка", format!("{:.0}%", activity * 100.0), if activity < 0.8 { Tone::Bad } else { Tone::Plain })
                .tone("прибыль", format!("{profit:.2}"), if profit < 0.0 { Tone::Bad } else { Tone::Good })
                .note("тёмная часть полосы — места, что стоят без дела (нет сырья или покупателей)")
        });
    }
    ui.add_space(4.0);
}

/// What weighs on the economy now (the country's features, its law, its programmes): each kind of
/// output as a tile with its total, where it comes from on the card.
fn influences(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    use super::super::kit::{self, Stat};
    let me = g.me;
    let e = g.effs[me].clone();
    let rows: [(&str, &str, f32, &dyn Fn(&crate::country::spirits::Eff) -> f32); 4] = [
        ("wheat", "Земля", e.out_land, &|x| x.out_land),
        ("anvil", "Заводы", e.out_industry, &|x| x.out_industry),
        ("pistols", "Арсеналы", e.out_military, &|x| x.out_military),
        ("box", "Товары людям", e.out_civil, &|x| x.out_civil),
    ];
    head(ui, s, "Что влияет на хозяйство");
    let word = |v: f32| match v {
        v if v > 1.15 => "много больше",
        v if v > 1.005 => "больше",
        v if v < 0.87 => "много меньше",
        v if v < 0.995 => "меньше",
        _ => "как обычно",
    };
    let items: Vec<Stat> = rows.iter().map(|(icon, name, v, _)| Stat::new(icon, name, word(*v), if *v > 1.005 { kit::GOOD } else if *v < 0.995 { kit::BAD } else { kit::PLAIN })).collect();
    let resp = kit::stats(ui, &items, 2);
    // (each source: the country's features, the law, the programmes)
    let f = &g.spirits.facts[me];
    let mut sources: Vec<(String, crate::country::spirits::Eff)> = g.spirits.list[me].iter().map(|sp| (sp.name(&g.pack), sp.eff(f, &g.pack))).collect();
    sources.push((format!("закон: {}", crate::country::policy::LAWS[g.policy.law[me]].name), crate::country::policy::law_eff(g.policy.law[me], g.h.at_war)));
    for r in &g.policy.running[me] {
        sources.push((format!("идёт: {}", crate::country::policy::PLANS[r.plan].name), (crate::country::policy::PLANS[r.plan].running)()));
    }
    for &(p, _) in &g.policy.finished[me] {
        sources.push((format!("сделано: {}", crate::country::policy::PLANS[p].name), (crate::country::policy::PLANS[p].done)()));
    }
    for (i, r) in resp.into_iter().enumerate() {
        let (icon, name, v, get) = rows[i];
        let src: Vec<(String, f32)> = sources.iter().map(|(n, x)| (n.clone(), get(x))).filter(|x| (x.1 - 1.0).abs() >= 0.005).collect();
        r.tip(|| {
            let mut c = Card::new(name).icon(icon).tone("всего", format!("{:+.0}%", (v - 1.0) * 100.0), if v >= 1.0 { Tone::Good } else { Tone::Bad });
            if src.is_empty() {
                c = c.note("ничто не прибавляет и не отнимает");
            }
            for (n, x) in &src {
                c = c.tone(n.clone(), format!("{:+.0}%", (x - 1.0) * 100.0), if *x >= 1.0 { Tone::Good } else { Tone::Bad });
            }
            c.note("особенности страны, её хозяйственный строй и программы — в папке «Страна»")
        });
    }
    if e.leak > 0.002 {
        let lk = e.leak;
        ui.scope(|ui| words(ui, "Часть денег казны пропадает: долги, смута, новые земли.", 13.5, s.accent)).response.tip(|| Card::new("Казна теряет").icon("coins").row("доля трат", format!("{:.0}%", lk * 100.0)));
    }
}

/// The modernisation of a trade: the one going on with its months; else the trades to choose from
/// — how dear their goods are, how many times each was modernised, what the next would add.
fn modern(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    use super::super::kit;
    use crate::country::policy::{modern_gain, MODERN_COST, MODERN_MONTHS};
    let me = g.me;
    let pack = g.pack.clone();
    head(ui, s, "Модернизация отраслей");
    let income = g.income(me);
    if let Some((k, done)) = g.policy.modern[me] {
        let times = g.policy.modernised[me].get(k).copied().unwrap_or(0);
        let mut stop = false;
        kit::framed(ui, Some(Color32::from_rgb(196, 160, 72)), false, |ui| {
            ui.horizontal(|ui| {
                kit::bold(ui, &pack.sectors[k].name, 15.0, s.ink);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::small(ui, "Остановить").clicked() {
                        stop = true;
                    }
                });
            });
            kit::words(ui, if done * 3 < MODERN_MONTHS { "работы только начались" } else if done * 3 < 2 * MODERN_MONTHS { "работы в разгаре" } else { "работы заканчиваются" }, 12.5, s.soft);
            let left = MODERN_MONTHS.saturating_sub(done);
            let gain = modern_gain(times);
            kit::gauge(ui, ui.available_width(), 10.0, done as f32 / MODERN_MONTHS as f32, Color32::from_rgb(196, 160, 72)).tip(|| Card::new("Модернизация").icon("gears").row("осталось", format!("{left} мес.")).row("даст", format!("+{:.0}% производительности", gain * 100.0)));
        });
        if stop {
            act(g, &[("modern_stop", "1".into())]);
        }
        return;
    }
    ui.scope(|ui| words(ui, "Новые станки и методы для одной отрасли: полтора года работы за деньги казны. Каждая следующая модернизация той же отрасли даёт меньше.", 13.0, s.soft)).response.tip(|| Card::new("Модернизация").icon("gears").row("срок", format!("{MODERN_MONTHS} мес.")).row("в месяц", format!("{:.0}% доходов казны", MODERN_COST * 100.0)));
    let mut pick = None;
    let n = g.h.home(me);
    for (k, sec) in pack.sectors.iter().enumerate() {
        if n.worked.get(k).copied().unwrap_or(0.0) <= 0.005 {
            continue;
        }
        let times = g.policy.modernised[me].get(k).copied().unwrap_or(0);
        let ratio = sec.outputs.first().map_or(1.0, |o| n.price[o.0] / (pack.goods[o.0].price * n.level).max(1e-9));
        let row = ui.horizontal(|ui| {
            let icons = chrome::icons(ui.ctx());
            let icon = sec.outputs.first().map_or("gears".to_string(), |o| pack.goods[o.0].icon.clone());
            let (ir, _) = ui.allocate_exact_size(vec2(20.0, 20.0), egui::Sense::hover());
            super::super::look::icon(&icons, &icon, 18.0, s.ink).paint_at(ui, ir);
            let mut job = egui::text::LayoutJob::simple_singleline(sec.name.clone(), egui::FontId::new(14.0, egui::FontFamily::Proportional), s.ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(170.0);
            let gal = ui.painter().layout_job(job);
            let (lr, _) = ui.allocate_exact_size(vec2(176.0, 20.0), egui::Sense::hover());
            ui.painter().galley(egui::pos2(lr.min.x, lr.center().y - gal.size().y / 2.0), gal, s.ink);
            let mark = "◆".repeat(times as usize);
            let what = if ratio > 1.3 { "не хватает" } else if ratio < 0.8 { "в избытке" } else { "как обычно" };
            ui.label(RichText::new(if mark.is_empty() { what.to_string() } else { format!("{mark} {what}") }).size(12.5).color(if ratio > 1.3 { s.accent } else { s.soft }));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::small(ui, "Начать").clicked() {
                    pick = Some(k);
                }
            });
        });
        let name = sec.name.clone();
        row.response.tip(|| Card::new(name).icon("gears").tone("цена её товара к обычной", format!("×{ratio:.2}"), if ratio > 1.3 { Tone::Bad } else { Tone::Plain }).row("модернизирована", format!("{times} раз")).row("даст", format!("+{:.0}% производительности", modern_gain(times) * 100.0)).row("в месяц", format!("{:.2} из казны", MODERN_COST * income)).note("дорого — значит, её товара не хватает: туда выгоднее всего"));
    }
    if let Some(k) = pick {
        act(g, &[("modern", k.to_string())]);
    }
}

/// What anyone may know of person `id` as a candidate for a post: his schooling and family, the
/// views he is known for, his fame, his last deed (his mind and honesty nobody sees).
pub(super) fn candidate_words(g: &Game, id: u32, science: bool) -> String {
    let p = g.folk.get(id);
    let now = g.month() as i32;
    let views = if science {
        if p.love >= g.pack.units.len() {
            "занимается хозяйством и наукой мирной".to_string()
        } else {
            format!("занимается военным делом: {}", g.pack.units[p.love].name.to_lowercase())
        }
    } else if p.state > 0.3 {
        "известен как сторонник сильной казны и плана".to_string()
    } else if p.state < -0.3 {
        "известен как сторонник свободы хозяев".to_string()
    } else {
        "известен как прагматик".to_string()
    };
    let mut out = format!("{} лет; {}; {}; {}; {}", p.age(now) as u32, p.schooling_word(), p.origin_word(), views, p.fame_word());
    if let Some(d) = p.deeds.last() {
        out += &format!("\n{}: {}", strategy::world::folk::year_word(d.month), d.text);
    }
    out
}
