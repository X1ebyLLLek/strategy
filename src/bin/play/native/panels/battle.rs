//! A battle explained: who fought, with what, why it went so.

use super::*;
use super::army::trophy_words;

/// One battle of the record, explained as a report: the two sides face to face with the outcome
/// stamped between them, then why it went so — each cause a bar weighed between the sides.
pub fn battle(ui: &mut egui::Ui, s: &Skin, g: &Game, i: usize) {
    let w = &g.h.war;
    let i = if i == usize::MAX { w.fought.len().saturating_sub(1) } else { i };
    let Some(f) = w.fought.get(i) else {
        ui.label("Этого боя нет.");
        return;
    };
    let Some(ch) = chrome::of(ui.ctx()) else { return };
    let pack = &g.pack;
    let name = |x: usize| g.h.home(x).name.clone();
    let col = |x: usize| {
        let c = g.countries[w.th.countries[x]].1;
        Color32::from_rgb(c[0], c[1], c[2])
    };
    let width = ui.available_width();
    // (a fight of divisions: where, how long)
    let divs = f.divs[0] > 0;
    let place = g.towns.iter().find(|t| t.0 == w.th.provs[f.province]).map(|t| t.1.clone());
    let title = match (&place, divs) {
        (Some(t), true) => format!("Бой за {t} — {}", crate::words::terrain_ru(f.terrain)),
        _ => format!("Сражение — {}", crate::words::terrain_ru(f.terrain)),
    };
    ui.label(RichText::new(title.to_uppercase()).family(egui::FontFamily::Name("head".into())).size(17.0).color(s.ink));
    if divs {
        let days = match f.days {
            0..=1 => "длился день".to_string(),
            d => format!("шёл {} {}", d, crate::words::plural(d as usize, "день", "дня", "дней")),
        };
        let note = if f.cut { " · оборонялись окружённые" } else { "" };
        ui.label(RichText::new(format!("{}, {}{note}", crate::words::date_words(f.month).to_lowercase(), days)).size(13.5).color(s.soft));

        // Визуальная плашка боевой фазы и контроль неба
        let (phase_icon, phase_name, phase_desc) = if f.cut {
            ("hazard", "Ликвидация котла", "Окружённые войска лишены подвоза снабжения и сжимаются в кольце")
        } else if f.won && f.days >= 5 {
            ("clash", "Преследование отступающих", "Фронт прорван, наступающие преследуют отходящего противника")
        } else if f.fort[0] > 0.05 {
            ("wall", "Штурм укреплённой полосы", "Осада долговременных бетонных бастионов и фортов")
        } else if f.days <= 2 {
            ("binoculars", "Встречный бой и разведка", "Сближение передовых частей и манёвренная завязка боя")
        } else if f.days <= 6 {
            ("strike", "Артиллерийская дуэль", "Подавление огневых позиций и пристрелка артиллерии")
        } else {
            ("army", "Генеральный штурм траншей", "Решающая схватка дивизий за укреплённые позиции рубежа")
        };
        let pill = |ui: &mut egui::Ui, icon: &str, text: &str, tone: Color32| {
            let font = egui::FontId::new(12.0, egui::FontFamily::Name("bold".into()));
            let tw = ui.painter().layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE).size().x;
            let (r, resp) = ui.allocate_exact_size(vec2(tw + 26.0, 22.0), egui::Sense::hover());
            ui.painter().rect_filled(r, 3.0, Color32::from_black_alpha(150));
            ui.painter().rect_stroke(r, 3.0, egui::Stroke::new(1.0, tone.gamma_multiply(0.85)));
            let icons = chrome::icons(ui.ctx());
            super::super::look::icon(&icons, icon, 13.0, tone).paint_at(ui, egui::Rect::from_center_size(pos2(r.min.x + 11.0, r.center().y), vec2(13.0, 13.0)));
            ui.painter().text(pos2(r.min.x + 20.0, r.center().y), egui::Align2::LEFT_CENTER, text, font, tone);
            resp
        };
        ui.horizontal(|ui| {
            pill(ui, phase_icon, phase_name, s.accent);
            let air_att: f32 = f.kinds[0].iter().enumerate().filter(|(u, _)| pack.units[*u].line == strategy::pack::Line::Air).map(|(_, &amt)| amt).sum();
            let air_def: f32 = f.kinds[1].iter().enumerate().filter(|(u, _)| pack.units[*u].line == strategy::pack::Line::Air).map(|(_, &amt)| amt).sum();
            if air_att > 0.01 || air_def > 0.01 {
                let (air_col, air_txt) = if air_att > air_def * 1.3 {
                    (super::super::kit::GOOD, "Господство в воздухе: нападающие")
                } else if air_def > air_att * 1.3 {
                    (super::super::kit::GOOD, "Господство в воздухе: оборона")
                } else {
                    (super::super::kit::MIXED, "Воздушный бой: небо оспорено")
                };
                pill(ui, "flag", air_txt, air_col);
            }
        }).response.tip(|| Card::new(phase_name).icon(phase_icon).text(phase_desc));
    }
    ui.add_space(4.0);
    // the two sides: flags and names, the stamp in the middle
    let (r, _) = ui.allocate_exact_size(vec2(width, 78.0), egui::Sense::hover());
    let mut v = chrome::Out::new();
    let icons = chrome::icons(ui.ctx());
    for (k, side) in [f.attacker, 1 - f.attacker].into_iter().enumerate() {
        let x = if k == 0 { r.min.x } else { r.max.x - 44.0 };
        let flag = egui::Rect::from_min_size(pos2(x, r.min.y + 4.0), vec2(44.0, 30.0));
        match super::super::flags::get(ui.ctx(), w.th.countries[side]) {
            Some(fl) => super::super::flags::paint(ui.ctx(), ui.painter(), flag, &fl, ch.trim, Some(&icons)),
            None => {
                v.push(egui::Shape::rect_filled(flag, 1.0, col(side)));
                v.push(egui::Shape::rect_stroke(flag.expand(2.0), 2.0, egui::Stroke::new(1.2, ch.trim)));
            }
        }
    }
    let stamp = egui::Rect::from_center_size(pos2(r.center().x, r.min.y + 22.0), vec2(150.0, 32.0));
    let good = (f.attacker == g.me) == f.won;
    let ink = if good { Color32::from_rgb(0x2c, 0x6a, 0x26) } else { s.accent };
    let ink = chrome::legible(ch.paper.mean, &[ink, s.ink]);
    v.push(egui::Shape::rect_stroke(stamp, 3.0, egui::Stroke::new(2.5, ink)));
    v.push(egui::Shape::rect_stroke(stamp.shrink(3.5), 2.0, egui::Stroke::new(1.0, ink)));
    ui.painter().extend(v);
    let p = ui.painter();
    p.text(stamp.center(), egui::Align2::CENTER_CENTER, if f.won { "ВЗЯТА" } else { "ОТБИТО" }, egui::FontId::new(19.0, egui::FontFamily::Name("head".into())), ink);
    p.text(pos2(r.min.x, r.min.y + 44.0), egui::Align2::LEFT_TOP, name(f.attacker), egui::FontId::new(14.0, egui::FontFamily::Name("bold".into())), s.ink);
    p.text(pos2(r.max.x, r.min.y + 44.0), egui::Align2::RIGHT_TOP, name(1 - f.attacker), egui::FontId::new(14.0, egui::FontFamily::Name("bold".into())), s.ink);
    p.text(pos2(r.min.x, r.min.y + 61.0), egui::Align2::LEFT_TOP, "наступали", egui::FontId::proportional(12.5), s.soft);
    p.text(pos2(r.max.x, r.min.y + 61.0), egui::Align2::RIGHT_TOP, "оборонялись", egui::FontId::proportional(12.5), s.soft);
    let kinds = |v: &[f32]| -> String {
        let mut parts: Vec<(f32, String)> = v.iter().zip(&pack.units).filter(|(n, _)| **n >= 0.05).map(|(n, u)| (*n, format!("{} {:.1}", u.name, n))).collect();
        parts.sort_by(|a, b| b.0.total_cmp(&a.0));
        parts.into_iter().map(|p| p.1).collect::<Vec<_>>().join(", ")
    };
    if divs {
        // (divisions, and their battalions by kind on hover)
        let bats = |v: &[f32]| -> String {
            let mut parts: Vec<(f32, String)> = v.iter().zip(&pack.units).filter(|(n, _)| **n >= 0.05).map(|(n, u)| (*n, format!("{} {:.0}", u.name, n / strategy::military::corps::BATTALION))).collect();
            parts.sort_by(|a, b| b.0.total_cmp(&a.0));
            parts.into_iter().map(|p| p.1).collect::<Vec<_>>().join(", ")
        };
        for (x, who) in [(0usize, "Наступали"), (1, "Оборонялись")] {
            let n = f.divs[x] as usize;
            let line = format!("{who}: {n} {}", crate::words::plural(n, "дивизия", "дивизии", "дивизий"));
            let b = bats(&f.kinds[x]);
            ui.label(RichText::new(line).size(14.0)).tip(|| Card::new(who).icon("army").row("батальоны", b.clone()));
        }
    } else if f.kinds[0].len() == pack.units.len() {
        ui.label(RichText::new(format!("Наступали: {}.", kinds(&f.kinds[0]))).size(14.0));
        ui.label(RichText::new(format!("Оборонялись: {}.", kinds(&f.kinds[1]))).size(14.0));
    }
    head(ui, s, "Почему так вышло");
    // each cause: its name, a bar weighed between the sides (the attackers' colour from the left,
    // the defenders' from the right), the numbers in words beside it
    let tug = |ui: &mut egui::Ui, label: &str, a: f32, b: f32, said: String| {
        ui.label(RichText::new(label).size(14.0).color(s.soft));
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 14.0), egui::Sense::hover());
        let share = if a + b > 0.0 { a / (a + b) } else { 0.5 };
        let mut v = chrome::Out::new();
        v.push(egui::Shape::rect_filled(r.expand(1.5), 2.0, Color32::from_black_alpha(160)));
        let mid = r.min.x + r.width() * share.clamp(0.0, 1.0);
        let left = egui::Rect::from_min_max(r.min, pos2(mid, r.max.y));
        let right = egui::Rect::from_min_max(pos2(mid, r.min.y), r.max);
        v.push(egui::Shape::rect_filled(left, 0.0, col(f.attacker)));
        v.push(egui::Shape::rect_filled(right, 0.0, col(1 - f.attacker)));
        ch.gloss(&mut v, r, Color32::from_white_alpha(50), Color32::from_black_alpha(50));
        v.push(egui::Shape::line_segment([pos2(r.center().x, r.min.y - 3.0), pos2(r.center().x, r.max.y + 3.0)], egui::Stroke::new(1.5, ch.trim)));
        v.push(egui::Shape::line_segment([pos2(mid, r.min.y), pos2(mid, r.max.y)], egui::Stroke::new(2.0, Color32::from_black_alpha(200))));
        ui.painter().extend(v);
        resp.tip(|| Card::new(label).icon("clash").text(said.clone()));
        ui.label(RichText::new(said).size(13.5).color(s.ink));
        ui.add_space(3.0);
    };
    let pct = |x: f32| format!("{:.0}%", x * 100.0);
    if divs {
        // the fight of divisions in words: strength, kit, experience, companies, width, supply,
        // trenches, generals, what was promised; the losses in men and the trophies
        let edge = |r: f32, more: &str, less: &str| -> String {
            match r {
                r if r >= 3.0 => format!("{more} втрое и больше"),
                r if r >= 1.8 => format!("{more} почти вдвое"),
                r if r >= 1.15 => format!("{more} заметно"),
                r if r > 1.0 / 1.15 => "почти поровну".to_string(),
                r if r > 1.0 / 1.8 => format!("{less} заметно"),
                r if r > 1.0 / 3.0 => format!("{less} почти вдвое"),
                _ => format!("{less} втрое и больше"),
            }
        };
        tug(ui, "Силы", f.ratio, 1.0, edge(f.ratio, "наступавшие сильнее", "оборонявшиеся сильнее"));
        let kit_w = |k: f32| match k {
            k if k >= 0.95 => "оснащены полностью",
            k if k >= 0.75 => "почти полностью",
            k if k >= 0.5 => "наполовину",
            _ => "почти без снаряжения",
        };
        tug(ui, "Оснащение", f.kit[0], f.kit[1], format!("наступавшие {}, оборонявшиеся {}", kit_w(f.kit[0]), kit_w(f.kit[1])));
        let exp_w = |e: f32| match e {
            e if e < 0.1 => "новички",
            e if e < 0.35 => "понюхавшие пороху",
            e if e < 0.6 => "бывалые",
            _ => "ветераны",
        };
        tug(ui, "Опыт", 0.2 + f.exp[0], 0.2 + f.exp[1], format!("наступали {}, оборонялись {}", exp_w(f.exp[0]), exp_w(f.exp[1])));
        let comp_w = |c: f32| match c {
            c if c < 0.5 => "без рот поддержки",
            c if c < 3.0 => "с немногими ротами",
            _ => "с ротами поддержки",
        };
        tug(ui, "Роты поддержки", 1.0 + f.companies[0], 1.0 + f.companies[1], format!("наступавшие {}, оборонявшиеся {}", comp_w(f.companies[0]), comp_w(f.companies[1])));
        let from = match f.sides {
            0 | 1 => "атаковали с одной стороны".to_string(),
            2 => "атаковали с двух сторон — фронт боя шире в полтора раза".to_string(),
            _ => "атаковали со всех сторон — фронт боя вдвое шире".to_string(),
        };
        tug(ui, "Охват", f.sides.max(1) as f32, 1.0, from);
        tug(ui, "Снабжение", f.supply[0], f.supply[1], format!("{} у наступавших, {} у оборонявшихся", pct(f.supply[0]), pct(f.supply[1])));
        tug(ui, "Окопы обороны", 1.0 - f.dug, f.dug, format!("окопались на {}", pct(f.dug)));
        if f.fort[0] > 0.01 {
            let drop = (f.fort[0] - f.fort[1]).max(0.0);
            let said = if drop >= 0.01 {
                format!("{} → {} (осадный огонь снёс −{:.0}%)", pct(f.fort[0]), pct(f.fort[1]), drop * 100.0)
            } else {
                format!("{} — бастионы выдержали обстрел", pct(f.fort[0]))
            };
            tug(ui, "Укрепления провинции", 1.0 - f.fort[1], f.fort[1], said);
        }
        tug(ui, "Генералы (как ведут бой)", f.lead[0], f.lead[1], format!("×{:.2} и ×{:.2}", f.lead[0], f.lead[1]));
        if f.odds != 0.5 {
            tug(ui, "Штаб обещал успех", f.odds, 1.0 - f.odds, pct(f.odds));
        }
        let men = |x: f32| -> String {
            if x < 1.0 {
                format!("{:.0} человек", (x * 1000.0 / 10.0).round() * 10.0)
            } else {
                format!("{x:.1} тыс. человек")
            }
        };
        tug(ui, "Потери убитыми и пленными", f.men[1], f.men[0], format!("наступавшие — {}, оборонявшиеся — {}", men(f.men[0]), men(f.men[1])));
        if f.captured > 0.005 {
            ui.label(RichText::new(format!("Отступая, оборонявшиеся бросили снаряжение: победители подобрали его на {}.", trophy_words(f.captured))).size(14.0).color(s.ink));
        }
        if f.cut {
            ui.label(RichText::new("Окружение: пути подвоза были отрезаны — отрезанные части потеряли снабжение и массово сдавались в плен.").size(13.5).color(s.accent));
        }
        tactics(ui, s, f);
        losses(ui, s, g, f, &col);
        armour(ui, s, g, f);
        ui.add_space(4.0);
        words(ui, "Каждый день боя огонь каждого рода войск бьёт по своим целям с поправкой на оснащение, местность, окопы, снабжение, опыт и генерала; уставшие дивизии выходят из боя: наступавшие останавливаются, оборонявшиеся отходят — бросая часть снаряжения, — а окружённые гибнут.", 13.5, s.soft);
        return;
    }
    tug(ui, "Силы (по ценности войск)", f.ratio, 1.0, format!("×{:.2} в пользу наступавших", f.ratio));
    tug(ui, "Снабжение", f.supply[0], f.supply[1], format!("{} у наступавших, {} у оборонявшихся", pct(f.supply[0]), pct(f.supply[1])));
    tug(ui, "Окопы обороны", 1.0 - f.dug, f.dug, format!("окопались на {}", pct(f.dug)));
    tug(ui, "Генералы (как ведут бой)", f.lead[0], f.lead[1], format!("×{:.2} и ×{:.2}", f.lead[0], f.lead[1]));
    tug(ui, "Штаб обещал успех", f.odds, 1.0 - f.odds, pct(f.odds));
    tug(ui, "Потери", f.lost[1], f.lost[0], format!("{:.1} у наступавших, {:.1} у оборонявшихся", f.lost[0], f.lost[1]));
    if f.plan[1] > 0.0 && f.plan[3] > 1.3 * f.plan[1] {
        ui.label(RichText::new(format!("Оборону усилили: к бою защитников ×{:.1} от того, что видели при подготовке удара.", f.plan[3] / f.plan[1])).size(14.0).color(s.accent));
    }
    ui.add_space(4.0);
    words(ui, "В бою огонь каждого рода войск бьёт по своим целям с поправкой на местность, окопы, снабжение и генерала; уставшие части выходят из линии, и сторона, у которой свежие держат меньше трети линии, отступает.", 13.5, s.soft);
}

/// What each side lost of each kind of its troops, as a table: the kind's icon and name, a bar of
/// the attackers' losses from the middle to the left, of the defenders' to the right (battalions
/// on the card).
fn losses(ui: &mut egui::Ui, s: &Skin, g: &Game, f: &strategy::military::front::Fought, col: &dyn Fn(usize) -> Color32) {
    let pack = &g.pack;
    let nu = pack.units.len();
    if f.lost_kinds[0].len() != nu {
        return;
    }
    let rows: Vec<usize> = (0..nu).filter(|&u| f.lost_kinds[0][u] + f.lost_kinds[1][u] > 0.002 || f.kinds[0].get(u).copied().unwrap_or(0.0) + f.kinds[1].get(u).copied().unwrap_or(0.0) > 0.05).collect();
    if rows.is_empty() {
        return;
    }
    super::super::kit::ribbon(ui, "Потери по родам войск", Some("broken"));
    let most = rows.iter().map(|&u| f.lost_kinds[0][u].max(f.lost_kinds[1][u])).fold(1e-6f32, f32::max);
    let icons = chrome::icons(ui.ctx());
    let Some(ch) = chrome::of(ui.ctx()) else { return };
    let bat = strategy::military::corps::BATTALION;
    ui.horizontal(|ui| {
        ui.label(RichText::new("наступавшие").size(12.0).color(s.soft));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("оборонявшиеся").size(12.0).color(s.soft));
        });
    });
    for u in rows {
        let unit = &pack.units[u];
        let (la, ld) = (f.lost_kinds[0][u], f.lost_kinds[1][u]);
        let (ka, kd) = (f.kinds[0].get(u).copied().unwrap_or(0.0), f.kinds[1].get(u).copied().unwrap_or(0.0));
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), egui::Sense::hover());
        let mid = r.center().x;
        let half = r.width() / 2.0 - 70.0;
        let mut v = chrome::Out::new();
        // (the attackers' bar to the left of the name plate, the defenders' to the right)
        let a = egui::Rect::from_min_max(pos2(mid - 62.0 - half * la / most, r.min.y + 7.0), pos2(mid - 62.0, r.max.y - 7.0));
        let d = egui::Rect::from_min_max(pos2(mid + 62.0, r.min.y + 7.0), pos2(mid + 62.0 + half * ld / most, r.max.y - 7.0));
        for (b, c) in [(a, col(f.attacker)), (d, col(1 - f.attacker))] {
            if b.width() > 0.5 {
                super::super::kit::gauge_at(&mut v, &ch, b, 1.0, c);
            }
        }
        let plate = egui::Rect::from_center_size(r.center(), vec2(120.0, 22.0));
        super::super::hud::sunk(&mut v, plate, &ch, 2.0);
        ui.painter().extend(v);
        super::super::look::icon(&icons, &unit.icon, 16.0, ch.trim).paint_at(ui, egui::Rect::from_center_size(pos2(plate.min.x + 13.0, plate.center().y), vec2(16.0, 16.0)));
        let mut job = egui::text::LayoutJob::simple_singleline(unit.name.clone(), egui::FontId::new(12.0, egui::FontFamily::Proportional), s.desk_ink);
        job.wrap = egui::text::TextWrapping::truncate_at_width(92.0);
        let gal = ui.painter().layout_job(job);
        ui.painter().galley(pos2(plate.min.x + 24.0, plate.center().y - gal.size().y / 2.0), gal, s.desk_ink);
        let p = ui.painter();
        p.text(pos2(r.min.x, r.center().y), egui::Align2::LEFT_CENTER, format!("{:.1}", la / bat), egui::FontId::new(12.0, egui::FontFamily::Name("bold".into())), s.ink);
        p.text(pos2(r.max.x, r.center().y), egui::Align2::RIGHT_CENTER, format!("{:.1}", ld / bat), egui::FontId::new(12.0, egui::FontFamily::Name("bold".into())), s.ink);
        let name = unit.name.clone();
        let icon = unit.icon.clone();
        resp.tip(|| {
            Card::new(name)
                .icon(icon)
                .row("наступавшие: были / потеряли", format!("{:.1} / {:.1} батальона", ka / bat, la / bat))
                .row("оборонявшиеся: были / потеряли", format!("{:.1} / {:.1} батальона", kd / bat, ld / bat))
                .note("потеряно насовсем: убиты и в плену; раненых, которых спасли госпитали, здесь нет")
        });
    }
}

/// Whether armour told in the fight: a side's armour its enemy's fire could not pierce, in words.
fn armour(ui: &mut egui::Ui, s: &Skin, g: &Game, f: &strategy::military::front::Fought) {
    let pack = &g.pack;
    let nu = pack.units.len();
    let has = |x: usize, u: usize| f.kinds[x].get(u).copied().unwrap_or(0.0) > 0.05;
    for x in 0..2 {
        let y = 1 - x;
        let pierce = (0..nu).filter(|&u| has(y, u)).map(|u| pack.units[u].pierce).fold(0.0f32, f32::max);
        let thick: Vec<&str> = (0..nu).filter(|&u| has(x, u) && pack.units[u].armor > pierce).map(|u| pack.units[u].name.as_str()).collect();
        if !thick.is_empty() {
            let who = if x == 0 { "наступавших" } else { "оборонявшихся" };
            let line = format!("Броня {who} ({}) оказалась не по зубам врагу: большая часть его огня отскакивала.", thick.join(", "));
            super::super::kit::framed(ui, Some(super::super::kit::GOOD), false, |ui| {
                ui.label(RichText::new("Броневой щит").family(egui::FontFamily::Name("bold".into())).size(13.0).color(s.ink));
                super::super::kit::words(ui, &line, 13.0, s.ink);
            });
        }
        let pierced: Vec<&str> = (0..nu).filter(|&u| has(x, u) && pack.units[u].armor > 0.0 && pierce >= pack.units[u].armor).map(|u| pack.units[u].name.as_str()).collect();
        if !pierced.is_empty() && x == 0 {
            let line = format!("Противотанковая оборона: орудия защитников уверенно пробивали броню ({}), сбивая наступательный порыв.", pierced.join(", "));
            super::super::kit::framed(ui, Some(super::super::kit::BAD), false, |ui| {
                ui.label(RichText::new("Противотанковый заслон").family(egui::FontFamily::Name("bold".into())).size(13.0).color(s.ink));
                super::super::kit::words(ui, &line, 13.0, s.ink);
            });
        }
    }
}

/// The generals' tactics in the fight: each side's most used, the days of each on the card.
fn tactics(ui: &mut egui::Ui, s: &Skin, f: &strategy::military::front::Fought) {
    use strategy::military::corps::TACTICS;
    let top = |x: usize| (0..TACTICS.len()).filter(|&t| f.tactics[x][t] > 0).max_by_key(|&t| f.tactics[x][t]);
    let (Some(a), Some(d)) = (top(0), top(1)) else { return };
    let a_tac = &TACTICS[a];
    let d_tac = &TACTICS[d];
    let a_counters_d = a_tac.counters.contains(&(d as u8));
    let d_counters_a = d_tac.counters.contains(&(a as u8));
    let edge_text = if a_counters_d {
        "Тактический перевес: наступающие успешно предугадали манёвр обороны"
    } else if d_counters_a {
        "Тактический перевес: защитники переиграли план атаки"
    } else {
        "Тактическое равновесие генералов: равная борьба замыслов"
    };
    let edge_tone = if a_counters_d || d_counters_a { super::super::kit::GOOD } else { super::super::kit::PLAIN };

    super::super::kit::framed(ui, Some(edge_tone), false, |ui| {
        ui.label(RichText::new(format!("Тактический поединок: «{}» против «{}»", a_tac.name, d_tac.name)).family(egui::FontFamily::Name("bold".into())).size(13.5).color(s.ink));
        super::super::kit::words(ui, edge_text, 12.5, s.soft);
    }).response.tip(|| {
        let mut c = Card::new("Тактика генералов").icon("compass");
        for (x, who) in [(0usize, "наступавшие"), (1, "оборонявшиеся")] {
            c = c.head(who);
            for t in 0..TACTICS.len() {
                if f.tactics[x][t] > 0 {
                    let counters: Vec<&str> = TACTICS[t].counters.iter().map(|&k| TACTICS[k as usize].name).collect();
                    let note = if counters.is_empty() { String::new() } else { format!(" (бьёт: {})", counters.join(", ")) };
                    c = c.row(format!("{}{note}", TACTICS[t].name), format!("{} дн.", f.tactics[x][t]));
                }
            }
        }
        c.note("каждые несколько дней генерал выбирает тактику; толковый — ту, что бьёт тактику врага: тактика, которую побили, ничего не даёт")
    });
}
