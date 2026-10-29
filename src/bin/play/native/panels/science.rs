//! The folder of science: the spheres the scientists work in, their focus and progress.

use super::*;
use super::economy::lever;

/// Science: its share, who heads it; its spheres as cards — each kind of troops and the economy:
/// how far the next design is, how much of science's strength goes to it (the player who heads
/// science himself sets it by a click), how the staff rates what we have; the last designs.
pub fn science(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    if g.h.labs[me].is_none() {
        ui.label("В этой эпохе науки нет.");
        return;
    }
    let pack = g.pack.clone();
    let n = g.h.home(me);
    lever(ui, s, "Науке из трат казны:", n.science_share, 50.0, pack.science.as_ref().map_or(0.05, |x| x.share), ["Крохи", "Скупо", "Как водится", "Щедро", "Очень щедро"], ["quill", "pen", "science", "graduate", "binoculars"], [0.3, 0.6, 1.0, 1.6, 2.4], false, "science", g);
    super::techs::techs(ui, s, g);
    head(ui, s, "Направления");
    let led = g.scholar.is_none();
    say(ui, s, if led { "Учёные работают над каждым направлением; щёлкните — сделать упор на нём. Что выйдет, неизвестно до [trials]испытаний." } else { "Упор выбирает глава науки. Что выйдет, неизвестно до [trials]испытаний." });
    ui.add_space(3.0);
    let icons = chrome::icons(ui.ctx());
    let ch = chrome::of(ui.ctx());
    let mut pick: Option<Option<usize>> = None;
    if let Some(lab) = g.h.labs[me].clone() {
        let k = pack.units.len() + 1;
        let cols = 2usize;
        let gap = 6.0;
        let w = ((ui.available_width() - gap) / cols as f32).floor();
        let h = 74.0;
        let rows = k.div_ceil(cols);
        let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), rows as f32 * (h + gap) - gap), egui::Sense::hover());
        let top = lab.focus.iter().cloned().fold(0.0f32, f32::max).max(1e-9);
        let even = 1.0 / k as f32;
        let c = &g.h.war.sides[me];
        for j in 0..k {
            let r = egui::Rect::from_min_size(area.min + vec2((j % cols) as f32 * (w + gap), (j / cols) as f32 * (h + gap)), vec2(w, h));
            let (name, icon) = if j < pack.units.len() { (pack.units[j].name.clone(), pack.units[j].icon.clone()) } else { ("хозяйство".to_string(), "gears".to_string()) };
            let focus = lab.focus.get(j).copied().unwrap_or(0.0);
            let progress = lab.progress.get(j).copied().unwrap_or(0.0).fract();
            let made = lab.made.get(j).copied().unwrap_or(0);
            let strong = focus > even * 1.5;
            let resp = ui.interact(r, ui.id().with(("sphere", j)), if led { egui::Sense::click() } else { egui::Sense::hover() });
            let resp = if led { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
            if let Some(ch) = &ch {
                let mut v = chrome::Out::new();
                super::super::kit::leaf_at(&mut v, ch, s, r, if strong { Some(s.accent) } else { None }, resp.hovered());
                if strong {
                    v.push(egui::Shape::closed_line(super::super::hud::chamfer(r, 7.0), egui::Stroke::new(1.8, ch.trim)));
                }
                ui.painter().extend(v);
            }
            let ir = egui::Rect::from_min_size(r.min + vec2(7.0, 9.0), vec2(30.0, 30.0));
            super::super::kit::socket(ui, egui::pos2(r.min.x + 25.0, r.min.y + 27.0), 15.5, &icon, s.accent, if strong { 0.55 } else { 0.2 });
            let p = ui.painter();
            // (33rd session: the name a size smaller before it is cut — «противотанков…»)
            let avail = w - 59.0;
            let mut fs = 14.0;
            while fs > 12.0 && p.layout_no_wrap(name.clone(), egui::FontId::new(fs, egui::FontFamily::Name("bold".into())), s.ink).size().x > avail {
                fs -= 0.5;
            }
            let mut job = egui::text::LayoutJob::simple_singleline(name.clone(), egui::FontId::new(fs, egui::FontFamily::Name("bold".into())), s.ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(avail);
            let gal = p.layout_job(job);
            p.galley(egui::pos2(ir.max.x + 14.0, r.min.y + 8.0), gal, s.ink);
            // the strength it gets: five pips against the strongest sphere
            let mut v = chrome::Out::new();
            super::super::hud::pips(&mut v, egui::pos2(ir.max.x + 14.0, r.min.y + 31.0), 5, focus / top * 5.0, s.accent, 11.0, 5.0);
            p.extend(v);
            // the way to the next design
            let bar = egui::Rect::from_min_size(egui::pos2(r.min.x + 7.0, r.max.y - 22.0), vec2(w - 14.0, 6.0));
            p.rect_filled(bar.expand(1.0), 1.5, Color32::from_black_alpha(80));
            p.rect_filled(egui::Rect::from_min_size(bar.min, vec2(bar.width() * progress, bar.height())), 1.0, Color32::from_rgb(70, 110, 160));
            // what the staff believes of ours, and the designs made, in words
            let rated = if j < pack.units.len() { ((c.own_fire[j].mean + c.own_tough[j].mean) / 2.0).exp() } else { 1.0 };
            let words = if j >= pack.units.len() {
                if made == 0 { "новшеств ещё не было".to_string() } else { "новшества были".to_string() }
            } else if rated > 1.12 {
                "генштаб доволен".to_string()
            } else if rated < 0.9 {
                "генштаб недоволен".to_string()
            } else {
                "как у всех".to_string()
            };
            p.text(egui::pos2(r.min.x + 7.0, r.max.y - 8.0), egui::Align2::LEFT_CENTER, words, egui::FontId::new(12.0, egui::FontFamily::Proportional), s.soft);
            let behind = if j < pack.units.len() { lab.behind.get(j).copied().unwrap_or(0.0) } else { 0.0 };
            if behind > 0.1 {
                super::super::look::icon(&icons, "hazard", 15.0, Color32::from_rgb(196, 70, 52)).paint_at(ui, egui::Rect::from_min_size(egui::pos2(r.max.x - 21.0, r.min.y + 6.0), vec2(15.0, 15.0)));
            }
            let tip = resp.tip(|| {
                let mut card = Card::new(name.clone())
                    .icon(icon.clone())
                    .row("сил науки", format!("{:.0}%", focus * 100.0))
                    .row("до следующей разработки", format!("{:.0}%", progress * 100.0))
                    .row("разработок", made.to_string());
                if j < pack.units.len() {
                    card = card.row("генштаб считает огонь", format!("×{:.2}", c.own_fire[j].mean.exp())).row("и стойкость", format!("×{:.2}", c.own_tough[j].mean.exp()));
                }
                if behind > 0.1 {
                    card = card.tone("трофеи", "у врага лучше", Tone::Bad);
                }
                card.note(if led { "щёлкните — упор на этом направлении" } else { "упор выбирает глава науки" })
            });
            if led && tip.clicked() {
                pick = Some(Some(j));
            }
        }
        if led {
            ui.add_space(4.0);
            if widgets::small(ui, "Всё поровну").tip(|| Card::new("Без упора").icon("science").text("Силы науки поровну на все направления.")).clicked() {
                pick = Some(None);
            }
        }
        head(ui, s, "Последние разработки");
        if lab.works.is_empty() {
            words(ui, "Пока ничего. Что разработки стоят на деле, покажут учения и бои.", 15.0, s.soft);
        }
        for wk in lab.works.iter().rev().take(6) {
            let (name, icon) = match wk.kind {
                Some(u) => (pack.units[u].name.clone(), pack.units[u].icon.clone()),
                None => ("хозяйство".to_string(), "gears".to_string()),
            };
            let (good, bad) = (ink_of(s, Color32::from_rgb(70, 130, 60)), ink_of(s, Color32::from_rgb(170, 60, 46)));
            let (word, col) = match (wk.kind, wk.adopted) {
                (None, _) if wk.gain.0 >= 0.0 => ("удалось", good),
                (None, _) => ("неудача", bad),
                (Some(_), true) => ("принята", good),
                (Some(_), false) => ("отвергнута", bad),
            };
            // (a row of two lines: the sphere and what came of it; the date under them)
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), egui::Sense::hover());
            let p = ui.painter();
            super::super::look::icon(&icons, &icon, 22.0, s.ink).paint_at(ui, egui::Rect::from_min_size(r.min + vec2(0.0, 4.0), vec2(22.0, 22.0)));
            let name_g = p.layout_no_wrap(name.clone(), egui::FontId::new(14.5, egui::FontFamily::Proportional), s.ink);
            let nw = name_g.size().x;
            p.galley(egui::pos2(r.min.x + 30.0, r.min.y + 1.0), name_g, s.ink);
            p.text(egui::pos2(r.min.x + 38.0 + nw, r.min.y + 10.0), egui::Align2::LEFT_CENTER, word, egui::FontId::new(14.0, egui::FontFamily::Name("bold".into())), col);
            p.text(egui::pos2(r.min.x + 30.0, r.min.y + 29.0), egui::Align2::LEFT_CENTER, crate::words::date_words(wk.month), egui::FontId::new(12.5, egui::FontFamily::Proportional), s.soft);
            let row = resp;
            let claim = wk.claim;
            row.tip(|| {
                Card::new(name.clone())
                    .icon(icon.clone())
                    .row("конструкторы обещают огонь", format!("{:+.0}%", (claim.0.exp() - 1.0) * 100.0))
                    .row("и стойкость", format!("{:+.0}%", (claim.1.exp() - 1.0) * 100.0))
                    .note("что разработка стоит на деле, покажут учения и бои")
            });
        }
    }
    if let Some(p) = pick {
        act(g, &[("scifocus", p.map_or("all".to_string(), |j| j.to_string()))]);
    }
    head(ui, s, "Кто ведёт науку");
    let scholar = g.scholar;
    if person(ui, s, g, "Вы сами", "me", scholar.is_none(), "упор науки решаете вы") {
        act(g, &[("scihead", "me".into())]);
    }
    for k in 0..g.scholars.len() {
        let name = g.scholars[k].0.clone();
        let body = g.roster.scholars.get(k).map_or(String::new(), |&id| super::economy::candidate_words(g, id, true));
        if person(ui, s, g, &name, "science", scholar == Some(k), &body) {
            act(g, &[("scihead", format!("sci{k}"))]);
        }
    }
}

// A bar of shares of the kinds of troops (by their colours, named where they fit), the numbers on
/// its card.
pub(super) fn stripes(ui: &mut egui::Ui, s: &Skin, g: &Game, sh: &[f32], title: &str) {
    ui.label(RichText::new(title.to_uppercase()).family(egui::FontFamily::Name("head".into())).size(13.0).color(s.soft));
    let names: Vec<String> = g.pack.units.iter().map(|u| u.name.clone()).collect();
    let colours: Vec<Color32> = (0..names.len()).map(|u| super::super::designer::KIND_COLORS[u % super::super::designer::KIND_COLORS.len()]).collect();
    let resp = widgets::shares(ui, sh, &names, &colours);
    resp.tip(|| {
        let mut c = Card::new(title).icon("army");
        let mut order: Vec<usize> = (0..sh.len()).filter(|&u| sh[u] >= 0.005).collect();
        order.sort_by(|&a, &b| sh[b].total_cmp(&sh[a]));
        for u in order {
            c = c.row(names[u].clone(), format!("{:.0}%", sh[u] * 100.0));
        }
        c.note("доли по ценности войск")
    });
    ui.add_space(3.0);
}
