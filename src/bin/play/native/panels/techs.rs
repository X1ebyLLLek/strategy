//! What the country knows: its stage of development, the techniques its science may work on (the
//! one it works on, how far; a click — work on another), what it knows already (who gave it and
//! when, on the card), the kinds of troops it cannot field for want of a technique, and what the
//! rival knows that it does not (as far as intelligence sees).

use strategy::pack::Sphere;
use strategy::world::progress::{stage_name, Know};
use super::*;
use super::super::kit::{self, Stat};

/// Years in a word (a cost of research at the present pace).
fn years_word(y: f32) -> &'static str {
    match y {
        y if y < 0.6 => "месяцы",
        y if y < 1.5 => "около года",
        y if y < 4.0 => "несколько лет",
        y if y < 10.0 => "годы",
        _ => "десятилетие и больше",
    }
}

/// A sphere's word and icon.
fn sphere_icon(sp: Sphere) -> &'static str {
    match sp {
        Sphere::Economy => "gears",
        Sphere::Military => "army",
        Sphere::Science => "science",
        Sphere::State => "crown",
    }
}

/// The section of techniques in the folder of science.
pub(super) fn techs(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let c = g.country(me);
    let o = g.country(1 - me);
    if g.know.len() <= c.max(o) || g.pack.techs.is_empty() {
        return;
    }
    let pack = g.pack.clone();
    let know = g.know[c].clone();
    let stage = know.stage(&pack);
    let rival_stage = g.know[o].stage(&pack);
    let rate = g.research_rate(me) * 12.0;
    kit::ribbon(ui, "Техника и стадия развития", Some("gears"));
    {
        let items = vec![
            Stat::new("sprout", "Стадия", stage_name(&pack, stage), kit::PLAIN),
            Stat::new("binoculars", "У соседа", stage_name(&pack, rival_stage), if rival_stage > stage { kit::BAD } else if rival_stage < stage { kit::GOOD } else { kit::PLAIN }),
            Stat::new("science", "Темп науки", if rate < 0.6 { "медленный" } else if rate < 1.1 { "обычный" } else if rate < 2.0 { "быстрый" } else { "очень быстрый" }, kit::PLAIN),
        ];
        let resp = kit::stats(ui, &items, 3);
        let next = know.next_stage(&pack);
        let (known, all) = (know.count(), pack.techs.len());
        let tips: [Box<dyn Fn() -> Card>; 3] = [
            Box::new(move || Card::new("Стадия развития").icon("sprout").row("знает техник", format!("{known} из {all}")).row("до следующей стадии", format!("{:.0}%", next * 100.0)).note("стадия — сколько ярусов техники эпохи страна освоила")),
            Box::new(move || Card::new("Стадия соседа").icon("binoculars").note("по донесениям разведки")),
            Box::new(move || Card::new("Темп науки").icon("science").row("лет обычной науки за год", format!("{rate:.2}")).note("от развития страны и её лабораторий (людей в них); техника, которую знает мир, дешевле")),
        ];
        for (r, t) in resp.into_iter().zip(tips.iter()) {
            r.tip(|| t());
        }
        kit::gauge(ui, ui.available_width(), 8.0, next, Color32::from_rgb(110, 150, 90)).tip(|| Card::new("До следующей стадии").icon("sprout").row("готово", format!("{:.0}%", next * 100.0)));
        // (33rd session: the tree of science in a window of its own)
        ui.add_space(6.0);
        let (ch, sk) = chrome::dress(ui);
        let ic = chrome::icons(ui.ctx());
        let is_open = super::super::techtree::is_open(ui.ctx());
        let w = ui.available_width();
        let b = chrome::press(ui, &ch, &sk, chrome::Press { text: "Древо науки", icon: Some(super::super::look::icon(&ic, "compass", 24.0, if is_open { sk.on_accent() } else { sk.ink })), size: vec2(w, 40.0), font: 16.0, on: is_open, ..chrome::Press::default() });
        if b.tip(|| Card::new("Древо науки").icon("science").text("Все техники эпохи по ярусам: что на чём стоит, что страна знает, над чем работает и что уже есть у соседа.").note("Esc закрывает")).clicked() {
            super::super::techtree::set_open(ui.ctx(), !is_open);
        }
    }
    // kinds it cannot field
    let locked: Vec<String> = know.locked(&pack).iter().enumerate().filter(|x| *x.1).map(|(u, _)| pack.units[u].name.clone()).collect();
    if !locked.is_empty() {
        kit::words(ui, format!("Нельзя выставить, нет техники: {}", locked.join(", ")), 13.5, kit::ink(s, kit::BAD));
    }
    // what it works on
    let seen_v = g.tech_seen(c);
    let seen = |t: usize| seen_v.get(t).copied().unwrap_or(0.0);
    let led = g.scholar.is_none();
    match know.target {
        Some(t) => {
            let cost = Know::cost(&pack, t, seen(t)).max(0.05);
            let done = (know.progress[t] / cost).min(1.0);
            let tip = g.tech_tip(t);
            let tech = &pack.techs[t];
            let when = years_word((cost - know.progress[t]).max(0.0) / rate.max(1e-3));
            let resp = kit::framed(ui, Some(s.accent), true, |ui| {
                ui.horizontal(|ui| {
                    let (sr, _) = ui.allocate_exact_size(vec2(50.0, 50.0), egui::Sense::hover());
                    kit::socket(ui, sr.center(), 24.0, &tech.icon, Color32::from_rgb(70, 110, 160), 0.5);
                    ui.vertical(|ui| {
                        kit::words(ui, "СЕЙЧАС ИЗУЧАЕТСЯ", 12.0, s.soft);
                        kit::bold(ui, tech.name.clone(), 16.0, s.ink);
                        kit::words(ui, format!("{} · {}", tech.sphere.word(), when), 12.5, s.soft);
                    });
                });
                ui.add_space(3.0);
                kit::gauge(ui, ui.available_width(), 10.0, done, Color32::from_rgb(70, 110, 160));
            });
            resp.response.tip(|| Card::new(tech.name.clone()).icon(tech.icon.clone()).text(tech.about.clone()).row("готово", format!("{:.0}%", done * 100.0)).row("осталось", when).text(tip.clone()));
        }
        None => {
            kit::words(ui, if led { "Учёные не выбрали главного дела — выберите, над чем работать." } else { "Глава науки выберет, над чем работать." }, 14.0, s.soft);
        }
    }
    // what it may work on
    kit::words(ui, "МОЖНО ИЗУЧАТЬ", 12.0, s.soft);
    let mut open = know.open(&pack);
    open.sort_by(|&a, &b| Know::cost(&pack, a, seen(a)).total_cmp(&Know::cost(&pack, b, seen(b))));
    let mut pick = None;
    if open.is_empty() {
        kit::words(ui, "Всё, что знает эпоха, страна уже знает.", 13.5, s.soft);
    }
    let icons = chrome::icons(ui.ctx());
    for &t in &open {
        let tech = &pack.techs[t];
        let on = know.target == Some(t);
        let cost = Know::cost(&pack, t, seen(t));
        // (30th session: a card of two lines — the name whole, over as many lines as it needs, under it the
        // sphere and how long; the button at the right. The names were cut at 200 points: «Танковый дизель и шир…»)
        let when = years_word((cost - know.progress[t]).max(0.0) / rate.max(1e-3));
        let full = ui.available_width();
        let button_w = 104.0;
        let name_w = (full - 12.0 - 44.0 - button_w - 10.0).max(80.0);
        let mut job = egui::text::LayoutJob::simple(tech.name.clone(), egui::FontId::new(14.5, egui::FontFamily::Name("bold".into())), if on { s.accent } else { s.ink }, name_w);
        job.wrap.max_rows = 2;
        let name_gal = ui.painter().layout_job(job);
        let sub_gal = ui.painter().layout_no_wrap(format!("{} · {}", tech.sphere.word(), when), egui::FontId::proportional(12.5), s.soft);
        let h = (name_gal.rect.height() + sub_gal.rect.height() + 22.0).max(48.0);
        let (row, rresp) = ui.allocate_exact_size(vec2(full, h), egui::Sense::hover());
        let tone = match tech.sphere {
            Sphere::Military => kit::BAD,
            Sphere::Economy => kit::MIXED,
            Sphere::Science => Color32::from_rgb(70, 110, 160),
            Sphere::State => kit::PLAIN,
        };
        {
            let (ch, sk) = chrome::dress(ui);
            let mut v = chrome::Out::new();
            kit::leaf_at(&mut v, &ch, &sk, row, Some(tone), on);
            if on {
                v.push(egui::Shape::closed_line(super::super::hud::chamfer(row, 7.0), egui::Stroke::new(1.6, s.accent)));
            }
            ui.painter().extend(v);
        }
        kit::socket(ui, egui::pos2(row.min.x + 28.0, row.center().y), 15.0, &tech.icon, tone, 0.4);
        let tx = row.min.x + 50.0;
        let name_h = name_gal.rect.height();
        let top = row.center().y - (name_h + sub_gal.rect.height() + 2.0) / 2.0;
        ui.painter().galley(egui::pos2(tx, top), name_gal, s.ink);
        ui.painter().galley(egui::pos2(tx, top + name_h + 2.0), sub_gal, s.soft);
        let brect = egui::Rect::from_min_size(egui::pos2(row.max.x - button_w - 6.0, row.center().y - 14.0), vec2(button_w, 28.0));
        ui.allocate_ui_at_rect(brect, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if on {
                    ui.label(RichText::new("в работе").size(13.0).color(s.accent));
                } else if kit::act_button(ui, "Изучать", "science", true, false).clicked() {
                    pick = Some(t);
                }
            });
        });
        // (32nd session: the button put at the row's side pulled the cursor up to its own foot — the next card began
        // inside this one, over its second line; the cursor goes back under the whole row)
        ui.advance_cursor_after_rect(row);
        ui.add_space(2.0);
        let resp = rresp;
        let tip = g.tech_tip(t);
        let world = seen(t);
        let needs: Vec<String> = tech.needs.iter().map(|&n| pack.techs[n].name.clone()).collect();
        resp.tip(|| {
            let mut card = Card::new(tech.name.clone()).icon(tech.icon.clone()).sub(format!("{} · ярус {}", tech.sphere.word(), tech.tier)).text(tech.about.clone()).text(tip.clone());
            card = card.row("знают в мире", kit::share(world).to_string());
            if !needs.is_empty() {
                card = card.row("опирается на", needs.join(", "));
            }
            card.note(if world > 0.3 { "чужое знание дешевле: его видят, покупают, перенимают" } else { "мало кто знает: придётся дойти самим" })
        });
    }
    if let Some(t) = pick {
        act(g, &[("tech", t.to_string())]);
    }
    // what the rival knows and we do not
    let (ours, theirs) = g.tech_gap(me);
    if !theirs.is_empty() || !ours.is_empty() {
        kit::words(ui, "СРАВНЕНИЕ С СОСЕДОМ", 12.0, s.soft);
        if !theirs.is_empty() {
            let names: Vec<String> = theirs.iter().map(|&t| pack.techs[t].name.clone()).collect();
            kit::words(ui, format!("У соседа есть, у нас нет: {}", names.join(", ")), 13.5, kit::ink(s, kit::BAD));
        }
        if !ours.is_empty() {
            let names: Vec<String> = ours.iter().map(|&t| pack.techs[t].name.clone()).collect();
            kit::words(ui, format!("У нас есть, у соседа нет: {}", names.join(", ")), 13.5, kit::ink(s, kit::GOOD));
        }
    }
    // what it knows
    kit::words(ui, "СТРАНА ЗНАЕТ", 12.0, s.soft);
    let known: Vec<usize> = (0..pack.techs.len()).filter(|&t| know.known[t]).collect();
    let side = 38.0;
    let cols = (((ui.available_width() + 5.0) / (side + 5.0)).floor() as usize).max(1);
    let rows = known.len().div_ceil(cols);
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), rows as f32 * (side + 5.0)), egui::Sense::hover());
    for (i, &t) in known.iter().enumerate() {
        let r = egui::Rect::from_min_size(area.min + vec2((i % cols) as f32 * (side + 5.0), (i / cols) as f32 * (side + 5.0)), vec2(side, side));
        let resp = ui.interact(r, ui.id().with(("known", t)), egui::Sense::hover());
        let tone = match pack.techs[t].sphere {
            Sphere::Military => kit::BAD,
            Sphere::Economy => kit::MIXED,
            Sphere::Science => Color32::from_rgb(70, 110, 160),
            Sphere::State => Color32::from_rgb(120, 100, 150),
        };
        kit::socket(ui, r.center(), side / 2.0 - 1.0, &pack.techs[t].icon, tone, if resp.hovered() { 0.8 } else { 0.55 });
        let when = know.when[t];
        let by = know.by[t].map(|id| g.folk.get(id).name.clone());
        let tip = g.tech_tip(t);
        resp.tip(|| {
            let tech = &pack.techs[t];
            let mut card = Card::new(tech.name.clone()).icon(sphere_icon(tech.sphere)).sub(format!("{} · ярус {}", tech.sphere.word(), tech.tier)).text(tech.about.clone()).text(tip.clone());
            card = card.row("освоена", if tech.tier == 0 { "издавна".to_string() } else { strategy::world::folk::year_word(when) });
            if let Some(b) = &by {
                card = card.row("открыл", b.clone());
            }
            card
        });
    }
    ui.add_space(4.0);
}
