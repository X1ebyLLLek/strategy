//! The window's self-checks of its layout: folder by folder, at three sizes of the window — the
//! war not hidden under the panels, numbers beside the words on the country's bar (his word in the
//! 23rd session: people must see the figures), letters readable, the folder keeping
//! its width, the country's bar not crowded, the groups' cards all seen.

use super::*;

/// the folders to look at, and what each check found
pub(super) fn folders(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, t0: std::time::Instant, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    // (each folder, and each page of a folder that has pages: 29th session — the checks looked at the
    // first page of each; the second page of the economy was never read)
    let tabs: Vec<(Option<Tab>, usize, String)> = {
        let g = shared.lock().unwrap();
        let mut folders: Vec<(Tab, &str)> = vec![(Tab::Nation, "страна"), (Tab::Econ, "экономика"), (Tab::Arms, "производство")];
        if g.h.labs[g.me].is_some() {
            folders.push((Tab::Science, "наука"));
        }
        folders.extend([(Tab::Army, "армия"), (Tab::Politics, "политика"), (Tab::Society, "общество"), (Tab::Ledger, "сводка"), (Tab::News, "события")]);
        let mut v: Vec<(Option<Tab>, usize, String)> = vec![(None, 0, "без папки".into())];
        for (tab, name) in folders {
            let pages = super::super::panels::pages(tab);
            if pages.is_empty() {
                v.push((Some(tab), 0, name.to_string()));
            }
            for (k, page) in pages.iter().enumerate() {
                v.push((Some(tab), k, format!("{name}/{}", page.to_lowercase())));
            }
        }
        v
    };
    let mut hidden: Vec<(String, f32)> = Vec::new();
    let mut digits: Vec<String> = Vec::new();
    let (mut small, mut dim) = (Vec::<String>::new(), Vec::<String>::new());
    let (mut shown_n, mut worst_ratio) = (0usize, 99.0f32);
    let mut pushed: Vec<String> = Vec::new();
    let mut overflow: Vec<String> = Vec::new();
    let mut cards_bad: Vec<String> = Vec::new();
    let mut crowded: Vec<String> = Vec::new();
    // (the window as large as is usual, smaller, and the smallest the game allows)
    for (w, h) in [(1600.0, 900.0), (1280.0, 720.0), (1100.0, 640.0)] {
        // (the last folder slid shut first: the map is fitted with no folder, then they open)
        app.tab = None;
        for _ in 0..20 {
            frame(ctx, app, w, h);
        }
        app.map = None;
        for (tab, page, name) in &tabs {
            app.tab = *tab;
            if let Some(t) = tab {
                super::super::set_page(ctx, *t, *page);
            }
            app.chosen = None;
            app.edit = None;
            if *tab == Some(Tab::Army) {
                app.chosen = Some(0);
                let g = shared.lock().unwrap();
                app.edit = (!g.templates.is_empty()).then(|| designer::Edit::of(&g, Some(0)));
            }
            // (as many frames as a folder takes to slide out and a window to come)
            if std::env::var("UITIME").is_ok() {
                eprintln!("{:.1} с: папка {name} {w}x{h}", t0.elapsed().as_secs_f32());
            }
            let mut out = frame(ctx, app, w, h);
            for _ in 0..17 {
                out = frame(ctx, app, w, h);
            }
            let g = shared.lock().unwrap();
            let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(w, h));
            // what covers the map: the panels, the modes, the small map
            let mut covers: Vec<(&str, Rect)> = Vec::new();
            let panel = |id: &str| egui::containers::panel::PanelState::load(ctx, egui::Id::new(id)).map(|p| p.rect);
            covers.extend(panel("top").map(|r| ("верхняя полоса", r)));
            if g.alert.is_some() {
                covers.extend(panel("banner").map(|r| ("полоса часов", r)));
            }
            if g.h.at_war {
                covers.extend(panel("armybar").map(|r| ("полоса групп", r)));
            }
            let folder = if tab.is_some() { panel("folder") } else { None };
            covers.extend(folder.map(|r| ("папка", r)));
            let modes = ctx.memory(|m| m.area_rect(egui::Id::new("modes")));
            covers.extend(modes.map(|r| ("режимы", r)));
            let m = app.map.as_ref().unwrap();
            covers.push(("мини-карта", m.last_mini));
            // 1. the war is not hidden: the provinces along the line where the two countries meet
            let war = &g.h.war;
            let line: Vec<usize> = war.front(0).into_iter().chain(war.front(1)).collect();
            let under = line.iter().filter(|&&q| {
                let p = m.screen_of(&g, war.th.provs[q]);
                !screen.contains(p) || covers.iter().any(|c| c.1.contains(p))
            });
            hidden.push((format!("{name} {w}×{h}"), under.count() as f32 / line.len().max(1) as f32));
            // the texts of the panels (not the map's)
            let all = texts(&out.shapes);
            // (the designer's window covers the map while a template is made — on purpose, as in
            // HOI4 — so it counts for the words, not for the map)
            let mut panes: Vec<Rect> = covers.iter().filter(|c| c.0 != "мини-карта").map(|c| c.1).collect();
            if *tab == Some(Tab::Army) && app.edit.is_some() {
                panes.extend(ctx.memory(|m| m.area_rect(egui::Id::new("designer"))));
            }
            let ui_texts: Vec<&Text> = all.iter().filter(|t| panes.iter().any(|r| r.contains(t.rect.center()))).collect();
            let date = g.date();
            if tab.is_none() && w >= 1500.0 {
                // (the country's bar: its words carry their numbers — «ворчит · 0,9», «растёт · +4%»)
                let bar: Vec<&&Text> = ui_texts.iter().filter(|t| t.rect.center().y < 64.0 && t.text != date).collect();
                let with = bar.iter().filter(|t| t.text.contains(" · ") && t.text.chars().any(|c| c.is_ascii_digit())).count();
                if with < 3 {
                    digits.push(format!("на верхней полосе слов с числами {with} из {} ({name}, {w}×{h})", bar.len()));
                }
            }
            for t in &ui_texts {
                shown_n += 1;
                if t.size < 12.0 {
                    small.push(t.text.chars().take(20).collect());
                }
                if let Some(bg) = t.under {
                    let r = contrast(t.colour, bg);
                    worst_ratio = worst_ratio.min(r);
                    if r < if t.size >= 18.0 { 3.0 } else { 4.5 } {
                        dim.push(format!("{} ({r:.1})", t.text.chars().take(20).collect::<String>()));
                        if std::env::var("UIDBG").is_ok() {
                            eprintln!("бледно: «{}» {:?} на {:?}, {:.0} пт, {name}", t.text, t.colour, bg, t.size);
                        }
                    }
                }
            }
            // the country's bar: no words on each other (those a plate hides too)
            if let Some(top) = panel("top") {
                let every = texts_of(&out.shapes, true);
                let words: Vec<&Text> = every.iter().filter(|t| top.contains(t.rect.center())).collect();

                for i in 0..words.len() {
                    for j in i + 1..words.len() {
                        if words[i].rect.shrink(1.0).intersects(words[j].rect.shrink(1.0)) {
                            let x = format!("«{}» и «{}» ({w}×{h})", words[i].text, words[j].text);
                            if !crowded.contains(&x) {
                                crowded.push(x);
                            }
                        }
                    }
                }
                if let Some(t) = words.iter().find(|t| !screen.contains_rect(t.rect) || !top.expand(1.0).contains_rect(t.rect)) {
                    crowded.push(format!("«{}» за краем полосы ({w}×{h})", t.text.chars().take(20).collect::<String>()));
                }
                // (the clock is at the bar's right end: crowded out, it is not drawn at all)
                if !words.iter().any(|t| t.text == date && screen.contains_rect(t.rect)) {
                    crowded.push(format!("дату вытеснило из полосы ({w}×{h})"));
                }
            }
            // 2. the folder keeps its width: the map begins where it ends, no words run past it
            if let Some(f) = folder {
                let gap = m.last_rect.min.x - f.max.x;
                if gap.abs() > 2.0 || f.width() > super::super::FOLDER + 1.0 {
                    pushed.push(format!("{name} {w}×{h}: папка {:.0}, щель {gap:.0}", f.width()));
                    if std::env::var("UIDBG").is_ok() {
                        for t in all.iter().filter(|t| t.rect.max.x > super::super::FOLDER - 10.0 && t.rect.min.x < super::super::FOLDER) {
                            eprintln!("широко: «{}» {:?}", t.text, t.rect);
                        }
                        fn wide(s: &Shape, out: &mut Vec<String>) {
                            match s {
                                Shape::Vec(v) => v.iter().for_each(|x| wide(x, out)),
                                Shape::Rect(r) if r.rect.max.x > super::super::FOLDER + 5.0 && r.rect.min.x < super::super::FOLDER => out.push(format!("{:?} заливка {:?}", r.rect, r.fill)),
                                _ => {}
                            }
                        }
                        let mut o = Vec::new();
                        out.shapes.iter().for_each(|c| wide(&c.shape, &mut o));
                        for x in o.iter().take(12) {
                            eprintln!("широко: {x}");
                        }
                    }
                }
                for t in all.iter().filter(|t| f.contains(t.clip.center()) && t.rect.min.x >= f.min.x && t.rect.min.x < f.max.x) {
                    if t.rect.max.x > f.max.x + 1.0 {
                        overflow.push(format!("«{}» ({name})", t.text.chars().take(24).collect::<String>()));
                    }
                }
            }
            // 3. the groups' cards all seen: on the screen, not under the modes or the small map
            if g.h.at_war {
                if let Some(groups) = war.command[g.me].as_ref() {
                    for k in 0..groups.len() {
                        let n = crate::words::group_name(k);
                        match app.cards.get(k) {
                            Some(r) if screen.contains_rect(*r) && !modes.is_some_and(|x| x.intersects(*r)) && !m.last_mini.intersects(*r) => {}
                            _ => {
                                let x = format!("{n} ({w}×{h})");
                                if !cards_bad.contains(&x) {
                                    cards_bad.push(x);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // (the folders' pages put back to the first: the checks after these look at the first pages)
    for (tab, page, _) in &tabs {
        if let Some(t) = tab.filter(|_| *page > 0) {
            super::super::set_page(ctx, t, 0);
        }
    }
    let worst = hidden.iter().cloned().fold(("везде 0".to_string(), 0.0f32), |a, b| if b.1 > a.1 { b } else { a });
    say(
        worst.1 <= 0.03,
        "карта не закрыта: линия, где страны встречаются, видна при любой открытой папке",
        format!("под панелями или за краем самое большее {:.1}% её провинций ({}); порог 3%", worst.1 * 100.0, worst.0),
    );
    say(digits.is_empty(), "числа на виду: у слов верхней полосы есть числа", if digits.is_empty() { format!("проверено надписей {shown_n}") } else { digits.iter().take(3).cloned().collect::<Vec<_>>().join("; ") });
    say(
        small.is_empty() && dim.is_empty(),
        "текст читается: не мельче 12 пт, контраст по WCAG",
        format!("надписей {shown_n}; мельче 12: {}; бледных: {}{}; худший контраст {worst_ratio:.1}", small.len(), dim.len(), if dim.is_empty() { String::new() } else { format!(" — {}", dim.iter().take(3).cloned().collect::<Vec<_>>().join("; ")) }),
    );
    say(
        pushed.is_empty() && overflow.is_empty(),
        "папка не раздвигается: карта сразу за ней, слова не вылезают",
        if pushed.is_empty() && overflow.is_empty() { "во всех папках при трёх размерах окна".into() } else { format!("{} {}", pushed.iter().take(2).cloned().collect::<Vec<_>>().join("; "), overflow.iter().take(3).cloned().collect::<Vec<_>>().join("; ")) },
    );
    say(crowded.is_empty(), "верхняя полоса: надписи не налезают друг на друга", if crowded.is_empty() { "при трёх размерах окна".into() } else { crowded.iter().take(2).cloned().collect::<Vec<_>>().join("; ") });
    let at_war = shared.lock().unwrap().h.at_war;
    say(
        at_war && cards_bad.is_empty(),
        "карточки всех групп армий видны",
        if !at_war { "войны нет — нечего проверить".into() } else if cards_bad.is_empty() { "при трёх размерах окна".into() } else { format!("не видны: {}", cards_bad.join(", ")) },
    );
}
