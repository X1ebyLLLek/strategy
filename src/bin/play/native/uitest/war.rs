//! The window's self-checks of the war's orders (part of `uitest`): the groups' plans as in HOI4
//! — a group chosen by its key, its stretch of the front drawn with the mouse, the arrow of its
//! attack laid and carried out by a key; the chosen divisions' way shown before the order (the
//! days it takes), sent on with Shift, formed into a new group by a key; the clock's speed by
//! a key. The spoiled control: the keys, the tools and the preview do nothing — each check fails.

use std::sync::{Arc, Mutex};
use eframe::egui::{self, Pos2, Rect};

use crate::Game;
use crate::tools::saves::Act;
use super::super::App;
use super::super::map::Tool;

/// The window drawn once more at the time `t`, with these events and keys held (Shift, Ctrl).
fn frame_with(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>, t: f64, modifiers: egui::Modifiers) {
    let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 900.0))), events, time: Some(t), modifiers, ..Default::default() };
    let _ = ctx.run(input, |ctx| app.draw(ctx));
}

fn key(k: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers }
}

/// The checks; each said through `say` (whether it passed, its name, what was found).
pub(super) fn plans(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    app.tab = None;
    app.menu = false;
    app.chosen = None;
    let none = egui::Modifiers::default();
    let shift = egui::Modifiers { shift: true, ..Default::default() };
    let mut t = ctx.input(|i| i.time) + 1.0;
    let mut step = |app: &mut App, events: Vec<egui::Event>, m: egui::Modifiers| {
        t += 1.0 / 60.0;
        frame_with(ctx, app, events, t, m);
    };
    let (at_war, groups) = {
        let g = shared.lock().unwrap();
        (g.h.at_war && g.h.war.corps.is_some(), g.h.war.command[g.me].as_ref().map_or(0, |v| v.len()))
    };
    if !at_war || groups < 2 {
        say(false, "планы групп: клавиша, линия фронта мышью, стрела и «Исполнить»", format!("нет войны дивизиями с двумя группами (групп {groups})"));
        return;
    }
    // the stretch to draw: up to three provinces of ours along the front, neighbours, in the second
    // group's sector (the first takes it from it); the view on its middle, close
    let (line, mid) = {
        let g = shared.lock().unwrap();
        let war = &g.h.war;
        let part = war.sectors_of(g.me).get(1).cloned().unwrap_or_default();
        let mut run: Vec<usize> = Vec::new();
        // (the longest run of neighbours from any province of the part, up to three)
        for &a in &part {
            let mut r = vec![a];
            while r.len() < 3 {
                let last = *r.last().unwrap();
                match war.th.nbr[last].iter().copied().find(|x| part.contains(x) && !r.contains(x)) {
                    Some(x) => r.push(x),
                    None => break,
                }
            }
            if r.len() > run.len() {
                run = r;
            }
        }
        let mid = run.get(run.len() / 2).map(|&q| {
            let seed = g.world.provinces[war.th.provs[q]].seed;
            ((seed % g.world.w) as f32, (seed / g.world.w) as f32)
        });
        (run, mid)
    };
    if let (Some(m), Some((x, y))) = (app.map.as_mut(), mid) {
        m.hold();
        m.k = 3.0;
        m.cx = x;
        m.cy = y;
        m.picked.clear();
        m.drafts.clear();
        m.tool = Tool::None;
    }
    for _ in 0..4 {
        step(app, vec![egui::Event::PointerGone], none);
    }
    // (a province's point on the screen, where a click finds that province again)
    let spot = |app: &App, q: usize| -> Option<Pos2> {
        let g = shared.lock().unwrap();
        let m = app.map.as_ref()?;
        let p = g.h.war.th.provs[q];
        let c = m.screen_of(&g, p);
        [egui::vec2(0.0, 0.0), egui::vec2(2.0, 2.0), egui::vec2(-3.0, 1.0), egui::vec2(1.0, -3.0), egui::vec2(4.0, 4.0)]
            .into_iter()
            .map(|d| c + d)
            // (not under a panel: the map's own layer is the top one there)
            .find(|&pt| m.last_rect.shrink(40.0).contains(pt) && m.province_at(&g, pt) == Some(p) && ctx.layer_id_at(pt).is_none_or(|l| l.order == egui::Order::Background))
    };
    // 26. the key 1 chooses the first group (its bar of plans comes); F and the left button drawn
    // over the stretch give it to the group
    let mut why: Vec<String> = Vec::new();
    step(app, vec![key(egui::Key::Num1, none)], none);
    for _ in 0..3 {
        step(app, vec![], none);
    }
    let chosen_ok = app.chosen == Some(0);
    let bar_ok = ctx.memory(|m| m.area_rect(egui::Id::new("planbar"))).is_some_and(|r| r.width() > 300.0);
    if !chosen_ok || !bar_ok {
        why.push(format!("клавиша 1: выбрана {:?}, полоса планов {}", app.chosen, if bar_ok { "есть" } else { "нет" }));
    }
    step(app, vec![key(egui::Key::F, none)], none);
    step(app, vec![], none);
    let tool_ok = matches!(app.map.as_ref().map(|m| &m.tool), Some(Tool::Front { gs, .. }) if gs == &vec![0]);
    let pts: Vec<Pos2> = line.iter().filter_map(|&q| spot(app, q)).collect();
    let before = shared.lock().unwrap().journal.len();
    if tool_ok && pts.len() == line.len() && !pts.is_empty() {
        let mut evs = vec![vec![egui::Event::PointerMoved(pts[0])], vec![egui::Event::PointerButton { pos: pts[0], button: egui::PointerButton::Primary, pressed: true, modifiers: none }]];
        for w in pts.windows(2) {
            for i in 1..=6 {
                evs.push(vec![egui::Event::PointerMoved(w[0] + (w[1] - w[0]) * (i as f32 / 6.0))]);
            }
        }
        if pts.len() == 1 {
            evs.push(vec![egui::Event::PointerMoved(pts[0] + egui::vec2(6.0, 0.0))]);
            evs.push(vec![egui::Event::PointerMoved(pts[0])]);
        }
        let end = *pts.last().unwrap();
        evs.push(vec![egui::Event::PointerButton { pos: end, button: egui::PointerButton::Primary, pressed: false, modifiers: none }]);
        evs.push(vec![]);
        for ev in evs {
            step(app, ev, none);
        }
    } else {
        let parts: Vec<usize> = {
            let g = shared.lock().unwrap();
            g.h.war.sectors_of(g.me).iter().map(|p| p.len()).collect()
        };
        if std::env::var("UIDBG").is_ok() {
            let g = shared.lock().unwrap();
            let w = &g.h.war;
            for gr in w.command[g.me].as_ref().unwrap() {
                let front = w.front(g.me);
                eprintln!("UIDBG anchor {:?} facing {:?} on front {}", gr.anchor, gr.facing, gr.anchor.iter().filter(|p| front.contains(p)).count());
            }
            eprintln!("UIDBG front {:?}", w.front(g.me));
        }
        why.push(format!("инструмент линии фронта {}; точек на экране {} из {} (участки групп {parts:?})", if tool_ok { "взят" } else { "не взят (F)" }, pts.len(), line.len()));
    }
    let drawn_ok = {
        let g = shared.lock().unwrap();
        let act = g.journal[before..].iter().find_map(|x| if let Act::Front(k, None, v) = &x.2 { (k.len() == 1).then(|| (k[0], v.clone())) } else { None });
        let mine = g.h.war.sectors_of(g.me).first().cloned().unwrap_or_default();
        let ok = act.as_ref().is_some_and(|(k, v)| *k == 0 && line.iter().all(|q| v.contains(q))) && line.iter().all(|q| mine.contains(q));
        if !ok {
            why.push(format!("линия: в журнале {:?}, у группы теперь {} из {} провинций", act.map(|a| a.1.len()), line.iter().filter(|q| mine.contains(q)).count(), line.len()));
        }
        ok
    };
    say(
        chosen_ok && bar_ok && tool_ok && drawn_ok,
        "планы группы: клавиша 1 выбирает её, F и мышь проводят её линию фронта (участок её и в ядре)",
        if why.is_empty() { format!("проведено по {} провинциям, участок теперь первой группы", line.len()) } else { why.join("; ") },
    );
    // 27. the arrow: A, a click on the enemy's province by the stretch — a plan (not yet an
    // order); Enter carries it out (the group attacks there)
    let mut why: Vec<String> = Vec::new();
    let target = {
        let g = shared.lock().unwrap();
        let war = &g.h.war;
        line.iter().flat_map(|&q| war.th.nbr[q].iter().copied()).find(|&r| war.owner[r] != g.me)
    };
    step(app, vec![key(egui::Key::A, none)], none);
    step(app, vec![], none);
    let tool_ok = matches!(app.map.as_ref().map(|m| &m.tool), Some(Tool::Attack { gs }) if gs == &vec![0]);
    let mut planned = false;
    let mut ordered = false;
    match target.and_then(|q| spot(app, q).map(|p| (q, p))) {
        Some((q, p)) if tool_ok => {
            let before = shared.lock().unwrap().journal.len();
            for ev in [vec![egui::Event::PointerMoved(p)], vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: none }], vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: none }], vec![]] {
                step(app, ev, none);
            }
            planned = app.map.as_ref().is_some_and(|m| m.drafts.contains(&(0, q))) && shared.lock().unwrap().journal.len() == before;
            if !planned {
                why.push(format!("стрела: планов {:?}, записей в журнале {}", app.map.as_ref().map(|m| m.drafts.clone()), shared.lock().unwrap().journal.len() - before));
            }
            step(app, vec![key(egui::Key::Enter, none)], none);
            step(app, vec![], none);
            let g = shared.lock().unwrap();
            let order = g.h.war.command[g.me].as_ref().and_then(|gs| gs.first()).map(|gr| gr.order);
            ordered = order == Some(strategy::military::front::Order::Attack(q)) && app.map.as_ref().is_some_and(|m| m.drafts.is_empty());
            if !ordered {
                why.push(format!("Enter: приказ группы {order:?}, цель {q}"));
            }
        }
        _ => why.push(format!("инструмент стрелы {}; цель на экране {}", if tool_ok { "взят" } else { "не взят (A)" }, target.is_some_and(|q| spot(app, q).is_some()))),
    }
    say(tool_ok && planned && ordered, "стрела наступления: A и щелчок по врагу — план, Enter — группа наступает туда", if why.is_empty() { "план лёг стрелой, Enter отдал приказ".into() } else { why.join("; ") });
    // 28. the chosen divisions: the hand over a province of ours shows their way and its days;
    // the right button sends them, Shift and the right button — on from there; N forms a new
    // group of them; + makes the clock faster
    let mut why: Vec<String> = Vec::new();
    app.chosen = None;
    // (the divisions of the province of ours on the screen where most of ours stand — taken from
    // the war itself, not from the stacks drawn: this check fails on its own, not on theirs)
    let stack = {
        let g = shared.lock().unwrap();
        let c = g.h.war.corps.as_ref().unwrap();
        let mut by: Vec<(usize, Vec<u32>)> = Vec::new();
        for d in c.divs.iter().filter(|d| d.side == g.me && d.rest <= 0.0) {
            match by.iter_mut().find(|x| x.0 == d.at) {
                Some(x) => x.1.push(d.id),
                None => by.push((d.at, vec![d.id])),
            }
        }
        by.sort_by_key(|x| (std::cmp::Reverse(x.1.len()), x.0));
        drop(g);
        by.into_iter().find(|x| spot(app, x.0).is_some())
    };
    let Some((from, ids)) = stack else {
        say(false, "выбранные дивизии: путь и дни при наведении, Shift — путь дальше, N — новая группа", "своих дивизий на виду нет".into());
        return;
    };
    if let Some(m) = app.map.as_mut() {
        m.picked = ids.clone();
    }
    // (the panel of the chosen divisions comes first: the points are chosen where it is not)
    for _ in 0..3 {
        step(app, vec![egui::Event::PointerGone], none);
    }
    // (two provinces of ours, a few steps behind the line, on the screen)
    let (a, b) = {
        let g = shared.lock().unwrap();
        let war = &g.h.war;
        let d = war.distance_from(from);
        let mut cand: Vec<usize> = (0..war.owner.len()).filter(|&q| war.owner[q] == g.me && q != from && d[q] >= 2 && d[q] <= 5).collect();
        cand.sort_by_key(|&q| (d[q], q));
        drop(g);
        let on: Vec<usize> = cand.into_iter().filter(|&q| spot(app, q).is_some()).collect();
        (on.first().copied(), on.iter().copied().find(|&q| Some(q) != on.first().copied()))
    };
    let (mut seen, mut sent, mut on_ok) = (false, false, false);
    if let (Some(a), Some(b)) = (a, b) {
        let (pa, pb) = (spot(app, a).unwrap(), spot(app, b).unwrap());
        step(app, vec![egui::Event::PointerMoved(pa)], none);
        for _ in 0..3 {
            step(app, vec![], none);
        }
        let pv = app.map.as_ref().and_then(|m| m.preview.clone());
        seen = pv.as_ref().is_some_and(|p| p.key.1 == a && !p.way.is_empty() && p.days > 0.0 && p.way.last() == Some(&a));
        if !seen {
            let g = shared.lock().unwrap();
            let found = g.h.war.corps.as_ref().map_or(0, |c| ids.iter().filter(|&&id| c.find(id).is_some()).count());
            let at = app.map.as_ref().and_then(|m| m.theatre_at(&g, pa));
            let layer = ctx.layer_id_at(pa).map(|l| format!("{:?}", l.order));
            why.push(format!("при наведении путь {:?} (точка {pa:?}, провинция там {at:?} вместо {a}, слой {layer:?}, выбранных найдено {found} из {})", pv.map(|p| (p.key, p.way.len(), p.days)), ids.len()));
        }
        let right = |p: Pos2, m: egui::Modifiers| vec![vec![egui::Event::PointerMoved(p)], vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Secondary, pressed: true, modifiers: m }], vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Secondary, pressed: false, modifiers: m }], vec![]];
        for ev in right(pa, none) {
            step(app, ev, none);
        }
        let before = shared.lock().unwrap().journal.len();
        for ev in right(pb, shift) {
            step(app, ev, shift);
        }
        let g = shared.lock().unwrap();
        let c = g.h.war.corps.as_ref().unwrap();
        let ways: Vec<Vec<usize>> = ids.iter().filter_map(|&id| c.find(id)).map(|i| c.divs[i].way.clone()).collect();
        sent = ways.iter().any(|w| w.contains(&a));
        on_ok = g.journal[before..].iter().any(|x| matches!(&x.2, Act::SendOn(v, q) if *q == b && *v == ids)) && ways.iter().any(|w| w.last() == Some(&b) && w.contains(&a));
        if !sent || !on_ok {
            why.push(format!("пути: через первую {}, до второй через первую {} (приказов «и дальше» в журнале {})", ways.iter().filter(|w| w.contains(&a)).count(), ways.iter().filter(|w| w.last() == Some(&b) && w.contains(&a)).count(), g.journal[before..].iter().filter(|x| matches!(x.2, Act::SendOn(..))).count()));
        }
    } else {
        let g = shared.lock().unwrap();
        let d = g.h.war.distance_from(from);
        let near = (0..g.h.war.owner.len()).filter(|&q| g.h.war.owner[q] == g.me && q != from && d[q] >= 2 && d[q] <= 5).count();
        why.push(format!("своих провинций в двух-пяти шагах на экране нет (всего таких {near}; стопка в {from})"));
    }
    // N: a new group of them
    let me = shared.lock().unwrap().me;
    let groups0 = shared.lock().unwrap().h.war.command[me].as_ref().map_or(0, |v| v.len());
    step(app, vec![key(egui::Key::N, none)], none);
    step(app, vec![], none);
    let raised = {
        let g = shared.lock().unwrap();
        let gs = g.h.war.command[me].as_ref().map_or(0, |v| v.len());
        let c = g.h.war.corps.as_ref().unwrap();
        let all_in = ids.iter().filter_map(|&id| c.find(id)).all(|i| c.divs[i].group == groups0);
        let ok = gs == groups0 + 1 && all_in && app.chosen == Some(groups0);
        if !ok {
            why.push(format!("N: групп {groups0} → {gs}, все в новой {all_in}, выбрана {:?}", app.chosen));
        }
        ok
    };
    // +: faster
    let speed0 = shared.lock().unwrap().speed;
    step(app, vec![key(egui::Key::Minus, none)], none);
    step(app, vec![key(egui::Key::Plus, none)], none);
    step(app, vec![key(egui::Key::Plus, none)], none);
    let speed1 = shared.lock().unwrap().speed;
    let faster = speed1 == (speed0.saturating_sub(1).max(1) + 2).min(5);
    if !faster {
        why.push(format!("скорость: было {speed0}, после −, +, + стало {speed1}"));
    }
    say(
        seen && sent && on_ok && raised && faster,
        "выбранные дивизии: путь и дни при наведении, Shift — путь дальше, N — новая группа, + и − — скорость",
        if why.is_empty() { format!("выбрано {}, путь виден, «и дальше» лёг, новая группа собрана, скорость {speed0} → {speed1}", ids.len()) } else { why.join("; ") },
    );
    // 29. where the new divisions go: the key 2 and R — the second group takes them all; the
    // place of gathering chosen on the map (the army's folder) — they gather there (the stores
    // topped up for two divisions: what is checked is where they go, not whether they are made)
    let mut why: Vec<String> = Vec::new();
    if let Some(m) = app.map.as_mut() {
        m.picked.clear();
    }
    for _ in 0..30 {
        step(app, vec![], none);
    }
    step(app, vec![key(egui::Key::Num2, none)], none);
    step(app, vec![], none);
    step(app, vec![key(egui::Key::R, none)], none);
    step(app, vec![], none);
    let fed_ok = shared.lock().unwrap().h.war.corps.as_ref().is_some_and(|c| c.muster_group[me] == Some(1));
    if !fed_ok {
        why.push(format!("R: выбрана {:?}, новые идут в {:?}", app.chosen, shared.lock().unwrap().h.war.corps.as_ref().map(|c| c.muster_group[me])));
    }
    // (a province of ours behind the line, on the screen)
    let place = {
        let g = shared.lock().unwrap();
        let war = &g.h.war;
        let front = war.front(g.me);
        let d = war.distance_from_all(&front);
        let mut cand: Vec<usize> = (0..war.owner.len()).filter(|&q| war.owner[q] == g.me && d[q] >= 2).collect();
        cand.sort_by_key(|&q| (d[q], q));
        drop(g);
        cand.into_iter().find(|&q| spot(app, q).is_some())
    };
    let mut gathered = false;
    match place {
        Some(q) => {
            if let Some(m) = app.map.as_mut() {
                m.build = super::super::map::Build::Muster;
            }
            let p = spot(app, q).unwrap();
            for ev in [vec![egui::Event::PointerMoved(p)], vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: none }], vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: none }], vec![]] {
                step(app, ev, none);
            }
            let mut g = shared.lock().unwrap();
            let set = g.h.war.corps.as_ref().is_some_and(|c| c.muster[me] == Some(q));
            if !set {
                why.push("место сбора щелчком не легло".into());
            }
            // (two divisions' worth in the stores, then the month lived)
            let pack = g.pack.clone();
            let c = g.h.war.corps.as_mut().unwrap();
            let last = c.next;
            // (and what the divisions in the field lack: the stores refill them first — after hard
            // fighting the two divisions' worth went into the gaps, and no new one was formed)
            let nu = pack.units.len();
            let mut lack = vec![0.0f32; nu];
            let mut lack_kit = vec![0.0f32; nu];
            for d in c.divs.iter().filter(|d| d.side == me) {
                if let Some(sh) = c.shapes[me].get(d.shape) {
                    for u in 0..nu {
                        lack[u] += (sh.units[u] - d.has[u]).max(0.0);
                        lack_kit[u] += (sh.units[u] * sh.norm_of(u) - d.kit.get(u).copied().unwrap_or(0.0)).max(0.0);
                    }
                }
            }
            if let Some(sh) = c.shapes[me].last().cloned() {
                for u in 0..nu {
                    c.pool[me][u] += 2.5 * sh.units[u] * (1.0 + sh.overhead(&pack)) + lack[u];
                    c.kit[me][u] += 2.5 * sh.units[u] * sh.norm_of(u) * (1.0 + sh.overhead(&pack)) + lack_kit[u];
                }
            }
            g.advance();
            while let Some((id, kind)) = g.ev.waiting.first().map(|e| (e.id, e.kind)) {
                let keep = matches!(kind, crate::country::events::Kind::PeaceOffer | crate::country::events::Kind::Weary);
                g.answer(id, keep as usize);
            }
            let c = g.h.war.corps.as_ref().unwrap();
            let new: Vec<&strategy::military::corps::Division> = c.divs.iter().filter(|d| d.side == me && d.id >= last).collect();
            gathered = set && !new.is_empty() && new.iter().all(|d| d.group == 1 && (d.at == q || d.way.first().is_some() || d.gone > 0.0));
            let there = new.iter().filter(|d| d.at == q).count();
            gathered = gathered && there * 2 >= new.len();
            if !gathered {
                why.push(format!("новых {}, у знамени {there}, во второй группе {}", new.len(), new.iter().filter(|d| d.group == 1).count()));
            }
        }
        None => why.push("своей провинции за линией на экране нет".into()),
    }
    say(fed_ok && gathered, "новые дивизии: R — в эту группу, «Место сбора» — собираются у знамени", if why.is_empty() { "новые дивизии собрались у знамени и пошли во вторую группу".into() } else { why.join("; ") });
    if let Some(m) = app.map.as_mut() {
        m.picked.clear();
        m.build = super::super::map::Build::None;
        m.refit();
    }
    app.chosen = None;
    // 30. before the war (as in HOI4, the plans are made in peace): a new world at peace — the
    // player's groups of armies already stand along the border with their generals; the key 1
    // chooses the first, A and a click lay its arrow, Enter does nothing in peace; the war
    // declared, the groups stay as they were (their stretches, the plan) and Enter sends the
    // first to attack along its arrow
    let mut why: Vec<String> = Vec::new();
    {
        let mut g = shared.lock().unwrap();
        let (seed, me, path) = (g.seed, g.me, g.path.clone());
        let mut fresh = Game::new(strategy::pack::Pack::load(&path).expect("набор эпохи"), seed, me);
        fresh.path = path;
        *g = fresh;
    }
    app.map = None;
    app.chosen = None;
    for _ in 0..6 {
        step(app, vec![egui::Event::PointerGone], none);
    }
    let (peace, groups, anchors) = {
        let g = shared.lock().unwrap();
        let gs = g.h.war.command[g.me].clone();
        (!g.h.at_war, gs.as_ref().map_or(0, |v| v.len()), gs.map(|v| v.iter().map(|x| x.anchor.clone()).collect::<Vec<_>>()))
    };
    if !peace || groups == 0 {
        why.push(format!("мир {peace}, групп армий в мирное время {groups}"));
    }
    // (the view on the first group's stretch, close)
    let first = {
        let g = shared.lock().unwrap();
        let war = &g.h.war;
        let part = war.sectors_of(g.me).first().cloned().unwrap_or_default();
        let q = part.get(part.len() / 2).copied();
        let target = part.iter().flat_map(|&p| war.th.nbr[p].iter().copied()).find(|&r| war.owner[r] != g.me);
        q.map(|q| {
            let seed = g.world.provinces[war.th.provs[q]].seed;
            ((seed % g.world.w) as f32, (seed / g.world.w) as f32)
        })
        .zip(target)
    };
    let (mut planned, mut waited, mut kept, mut went) = (false, false, false, false);
    if let Some(((x, y), target)) = first {
        if let Some(m) = app.map.as_mut() {
            m.hold();
            m.k = 3.0;
            m.cx = x;
            m.cy = y;
        }
        for _ in 0..4 {
            step(app, vec![egui::Event::PointerGone], none);
        }
        step(app, vec![key(egui::Key::Num1, none)], none);
        step(app, vec![], none);
        step(app, vec![key(egui::Key::A, none)], none);
        step(app, vec![], none);
        match spot(app, target) {
            Some(p) => {
                for ev in [vec![egui::Event::PointerMoved(p)], vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: none }], vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: none }], vec![]] {
                    step(app, ev, none);
                }
            }
            None => why.push("цель стрелы не на экране".into()),
        }
        planned = app.chosen == Some(0) && app.map.as_ref().is_some_and(|m| m.drafts.contains(&(0, target)));
        if !planned {
            why.push(format!("в мире: выбрана {:?}, планы {:?}", app.chosen, app.map.as_ref().map(|m| m.drafts.clone())));
        }
        step(app, vec![key(egui::Key::Enter, none)], none);
        step(app, vec![], none);
        {
            let g = shared.lock().unwrap();
            let order = g.h.war.command[g.me].as_ref().and_then(|gs| gs.first()).map(|gr| gr.order);
            waited = order == Some(strategy::military::front::Order::Hold) && app.map.as_ref().is_some_and(|m| m.drafts.contains(&(0, target)));
            if !waited {
                why.push(format!("Enter в мире: приказ {order:?}"));
            }
        }
        // the war
        {
            let mut g = shared.lock().unwrap();
            g.act(Act::Form(vec![("declare".into(), "1".into())]));
            let now: Option<Vec<Vec<usize>>> = g.h.war.command[g.me].as_ref().map(|v| v.iter().map(|x| x.anchor.clone()).collect());
            kept = g.h.at_war && now == anchors && g.h.war.command[g.me].as_ref().is_some_and(|v| v.iter().all(|x| x.general.is_some()));
            if !kept {
                why.push(format!("после объявления войны: групп {}, участки {}", now.as_ref().map_or(0, |v| v.len()), if now == anchors { "те же" } else { "другие" }));
            }
        }
        step(app, vec![], none);
        step(app, vec![key(egui::Key::Enter, none)], none);
        step(app, vec![], none);
        let g = shared.lock().unwrap();
        let order = g.h.war.command[g.me].as_ref().and_then(|gs| gs.first()).map(|gr| gr.order);
        went = order == Some(strategy::military::front::Order::Attack(target));
        if !went {
            why.push(format!("Enter в войну: приказ {order:?}"));
        }
    } else {
        why.push("у первой группы нет участка у границы".into());
    }
    say(
        peace && groups > 0 && planned && waited && kept && went,
        "до войны: группы армий стоят у границы, план рисуется в мирное время, с войной исполняется",
        if why.is_empty() { format!("групп в мирное время {groups}; план лёг в мире, Enter ждал войны; с объявлением группы те же, Enter повёл первую по стреле") } else { why.join("; ") },
    );
    app.chosen = None;
    // 31. two generals on the whole front, and a front against another neighbour: the key 1 and
    // Shift with 2 choose two groups, F and a click on the enemy's land give them the whole
    // border with him — cut in two, each its half, the other groups off it; the key 3 and F, a
    // click on another neighbouring country — the third group faces him along all the border
    // with him
    let mut why: Vec<String> = Vec::new();
    app.more.clear();
    let (front, foe_q, other) = {
        let g = shared.lock().unwrap();
        let war = &g.h.war;
        let front = war.front(g.me);
        let foe_q = front.iter().flat_map(|&p| war.th.nbr[p].iter().copied()).find(|&r| war.owner[r] != g.me);
        // (another country touching our land: its provinces of the world, and the border with it)
        let mine = war.th.countries[g.me] as u32;
        let foe = war.th.countries[1 - g.me] as u32;
        let mut other: Option<(u32, usize)> = None;
        'find: for q in 0..war.owner.len() {
            if war.owner[q] != g.me {
                continue;
            }
            for &r in &g.world.provinces[war.th.provs[q]].neighbors {
                let c = super::super::map::country_now(&g, r as usize);
                if c != mine && c != foe && c != u32::MAX {
                    other = Some((c, r as usize));
                    break 'find;
                }
            }
        }
        (front, foe_q, other)
    };
    // (a small country may lead its army by two groups: a third is raised for the check, as the
    // player raises one)
    {
        let mut g = shared.lock().unwrap();
        let me = g.me;
        if g.h.war.command[me].as_ref().map_or(0, |v| v.len()) < 3 {
            // (three divisions of the largest group: the others keep theirs)
            let ids: Vec<u32> = g.h.war.corps.as_ref().map_or(Vec::new(), |c| {
                let k = g.h.war.command[me].as_ref().map_or(0, |v| v.len());
                let big = (0..k).max_by_key(|&gr| c.divs.iter().filter(|d| d.side == me && d.group == gr).count()).unwrap_or(0);
                c.divs.iter().filter(|d| d.side == me && d.group == big).take(3).map(|d| d.id).collect()
            });
            if !ids.is_empty() {
                g.act(crate::tools::saves::Act::Raise(ids));
            }
        }
    }
    let groups_now = {
        let g = shared.lock().unwrap();
        g.h.war.command[g.me].as_ref().map_or(0, |v| v.len())
    };
    let look = |app: &mut App, p: usize| {
        if let Some(m) = app.map.as_mut() {
            let g = shared.lock().unwrap();
            let seed = g.world.provinces[p].seed;
            m.hold();
            m.k = 2.4;
            m.cx = (seed % g.world.w) as f32;
            m.cy = (seed / g.world.w) as f32;
        }
    };
    let click_at = |app: &mut App, p: Pos2, step: &mut dyn FnMut(&mut App, Vec<egui::Event>, egui::Modifiers)| {
        for ev in [vec![egui::Event::PointerMoved(p)], vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: none }], vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: none }], vec![]] {
            step(app, ev, none);
        }
    };
    let mut split_ok = false;
    match foe_q {
        Some(fq) if groups_now >= 3 => {
            // (the lock let go before `look` takes it again)
            let wp = shared.lock().unwrap().h.war.th.provs[fq];
            look(app, wp);
            for _ in 0..3 {
                step(app, vec![egui::Event::PointerGone], none);
            }
            step(app, vec![key(egui::Key::Num1, none)], none);
            step(app, vec![], none);
            step(app, vec![key(egui::Key::Num2, shift)], shift);
            step(app, vec![], none);
            let two = app.chosen == Some(0) && app.more == vec![1];
            step(app, vec![key(egui::Key::F, none)], none);
            step(app, vec![], none);
            let before = shared.lock().unwrap().journal.len();
            match spot(app, fq) {
                Some(p) => {
                    let mut st = |a: &mut App, e: Vec<egui::Event>, m: egui::Modifiers| step(a, e, m);
                    click_at(app, p, &mut st);
                }
                None => why.push("вражеская провинция у фронта не на экране".into()),
            }
            let g = shared.lock().unwrap();
            let parts = g.h.war.sectors_of(g.me);
            let a = parts.first().cloned().unwrap_or_default();
            let b = parts.get(1).cloned().unwrap_or_default();
            let rest: usize = parts.iter().skip(2).map(|p| p.len()).sum();
            let act = g.journal[before..].iter().any(|x| matches!(&x.2, Act::Front(gs, None, _) if gs == &vec![0, 1]));
            let covered = front.iter().all(|q| a.contains(q) || b.contains(q));
            let halves = !a.is_empty() && !b.is_empty() && (a.len() as f32 - b.len() as f32).abs() <= 0.5 * front.len() as f32;
            split_ok = two && act && covered && halves && rest == 0;
            if !split_ok {
                why.push(format!("весь фронт двум: выбраны {:?}+{:?}, приказ {act}, у первой {}, у второй {}, у прочих {rest} из {}", app.chosen, app.more, a.len(), b.len(), front.len()));
            }
        }
        _ => why.push(format!("нет фронта или мало групп ({groups_now})")),
    }
    let mut other_ok = false;
    match other {
        Some((c, p)) => {
            look(app, p);
            for _ in 0..3 {
                step(app, vec![egui::Event::PointerGone], none);
            }
            step(app, vec![key(egui::Key::Num3, none)], none);
            step(app, vec![], none);
            step(app, vec![key(egui::Key::F, none)], none);
            step(app, vec![], none);
            // (a point of that country's province on the screen)
            let pt = {
                let g = shared.lock().unwrap();
                let m = app.map.as_ref().unwrap();
                let c0 = m.screen_of(&g, p);
                [egui::vec2(0.0, 0.0), egui::vec2(3.0, 3.0), egui::vec2(-4.0, 2.0), egui::vec2(2.0, -4.0)]
                    .into_iter()
                    .map(|d| c0 + d)
                    .find(|&x| m.last_rect.shrink(40.0).contains(x) && m.province_at(&g, x) == Some(p) && ctx.layer_id_at(x).is_none_or(|l| l.order == egui::Order::Background))
            };
            match pt {
                Some(x) => {
                    let mut st = |a: &mut App, e: Vec<egui::Event>, m: egui::Modifiers| step(a, e, m);
                    click_at(app, x, &mut st);
                    let g = shared.lock().unwrap();
                    let want = super::super::map::border_with(&g, c);
                    let gr = g.h.war.command[g.me].as_ref().and_then(|gs| gs.get(2)).cloned();
                    let mine = g.h.war.sectors_of(g.me).get(2).cloned().unwrap_or_default();
                    other_ok = gr.as_ref().is_some_and(|x| x.facing == Some(c)) && !want.is_empty() && want.iter().all(|q| mine.contains(q)) && mine.len() == want.len();
                    if !other_ok {
                        why.push(format!("фронт против соседа: группа смотрит на {:?} (нужно {c}), её провинций {} из {}", gr.and_then(|x| x.facing), mine.len(), want.len()));
                    }
                }
                None => why.push("соседняя страна не на экране".into()),
            }
        }
        None => why.push("у страны нет других соседей — проверить нечем".into()),
    }
    say(
        split_ok && other_ok,
        "фронт: двум группам весь фронт одним щелчком (пополам), третьей — граница с другим соседом",
        if why.is_empty() { format!("весь фронт ({} провинций) поделён между первой и второй, третья держит границу с соседом", front.len()) } else { why.join("; ") },
    );
    app.chosen = None;
    app.more.clear();
    // 36. the front drawn as in HOI4 (28th session): the key 1, F, and the left button drawn on
    // the enemy's side of the border from by one province of ours to by another four along it —
    // the group takes the whole stretch between them (before, only provinces of ours under the
    // hand: here none)
    let mut why: Vec<String> = Vec::new();
    let pick = {
        let g = shared.lock().unwrap();
        let war = &g.h.war;
        let me = g.me;
        let foe = war.th.countries[1 - me] as u32;
        let line = war.along(&super::super::map::border_with(&g, foe));
        // (five in a row, each touching the next; the enemy's province by the first and by the last)
        let by = |q: usize| war.th.nbr[q].iter().copied().find(|&r| war.owner[r] != me);
        let mut found = None;
        let mid = line.len() / 2;
        let order: Vec<usize> = (0..line.len().saturating_sub(4)).map(|i| if i % 2 == 0 { mid.saturating_sub(2) + i / 2 } else { mid.saturating_sub(2).saturating_sub(i / 2 + 1) }).filter(|&i| i + 4 < line.len()).collect();
        for i in order {
            let run = &line[i..i + 5];
            if run.windows(2).all(|w| war.th.nbr[w[0]].contains(&w[1])) {
                if let (Some(a), Some(z)) = (by(run[0]), by(run[4])) {
                    found = Some((run.to_vec(), a, z, war.th.provs[run[2]]));
                    break;
                }
            }
        }
        found
    };
    let mut stretch_ok = false;
    match pick {
        Some((run, a, z, centre)) => {
            look(app, centre);
            for _ in 0..3 {
                step(app, vec![egui::Event::PointerGone], none);
            }
            step(app, vec![key(egui::Key::Num1, none)], none);
            step(app, vec![], none);
            step(app, vec![key(egui::Key::F, none)], none);
            step(app, vec![], none);
            // (on the border between our province and the enemy's, a few points into his side —
            // where a hand drawing along the border passes)
            let across = |app: &App, ours: usize, foe: usize| -> Option<Pos2> {
                let g = shared.lock().unwrap();
                let m = app.map.as_ref()?;
                let b = m.border_point(&g, ours, foe)?;
                let f = m.screen_of(&g, g.h.war.th.provs[foe]);
                Some(b + (f - b).normalized() * 3.0)
            };
            match (across(app, run[0], a), across(app, run[4], z)) {
                (Some(pa), Some(pz)) => {
                    let before = shared.lock().unwrap().journal.len();
                    let mut evs = vec![vec![egui::Event::PointerMoved(pa)], vec![egui::Event::PointerButton { pos: pa, button: egui::PointerButton::Primary, pressed: true, modifiers: none }]];
                    for i in 1..=12 {
                        evs.push(vec![egui::Event::PointerMoved(pa + (pz - pa) * (i as f32 / 12.0))]);
                    }
                    evs.push(vec![egui::Event::PointerButton { pos: pz, button: egui::PointerButton::Primary, pressed: false, modifiers: none }]);
                    evs.push(vec![]);
                    for ev in evs {
                        step(app, ev, none);
                    }
                    let g = shared.lock().unwrap();
                    let act = g.journal[before..].iter().find_map(|x| if let Act::Front(k, None, v) = &x.2 { (k == &vec![0]).then(|| v.clone()) } else { None });
                    let mine = g.h.war.sectors_of(g.me).first().cloned().unwrap_or_default();
                    let got = act.as_ref().map_or(0, |v| run.iter().filter(|q| v.contains(q)).count());
                    stretch_ok = got == run.len() && run.iter().all(|q| mine.contains(q));
                    if std::env::var("UIDBG").is_ok() {
                        let war = &g.h.war;
                        let nb: Vec<Vec<usize>> = run.iter().map(|&q| war.th.nbr[q].iter().copied().filter(|r| run.contains(r)).collect()).collect();
                        eprintln!("UIDBG stretch run {run:?} nbrs-in-run {nb:?} act {act:?} a {a} z {z}");
                    }
                    if !stretch_ok {
                        why.push(format!("протяжка: в журнале {}, из пяти провинций участка в приказе {got}, у группы {}", if act.is_some() { "приказ есть" } else { "приказа нет" }, run.iter().filter(|q| mine.contains(q)).count()));
                    }
                }
                _ => why.push("провинции врага у участка не на экране".into()),
            }
        }
        None => why.push("нет пяти провинций подряд вдоль границы с врагом".into()),
    }
    say(
        stretch_ok,
        "линия фронта протяжкой, как в HOI4: от провинции до провинции по стороне врага — группе весь участок границы между ними",
        if why.is_empty() { "проведено от первой до пятой провинции участка — все пять у первой группы".into() } else { why.join("; ") },
    );
    app.chosen = None;
    app.more.clear();
    // 38. the generals and the armies (28th session): a general from the reserve takes the first
    // army (the old one goes to the reserve), the second's is relieved, the last army is
    // disbanded — its divisions in the staff's reserve, the armies one fewer
    let mut g = shared.lock().unwrap();
    let me = g.me;
    let form = |pairs: &[(&str, &str)]| crate::tools::saves::Act::Form(pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect());
    if g.spare.is_empty() {
        g.act(form(&[("general_new", "1")]));
    }
    let n0 = g.h.war.command[me].as_ref().map_or(0, |v| v.len());
    let first_was = g.h.war.command[me].as_ref().and_then(|v| v.first()).and_then(|x| x.general.as_ref()).map(|x| x.name.clone());
    let spare0 = g.spare.first().map(|x| x.name.clone());
    g.act(form(&[("grp", "0"), ("general", "0")]));
    let first_now = g.h.war.command[me].as_ref().and_then(|v| v.first()).and_then(|x| x.general.as_ref()).map(|x| x.name.clone());
    let swapped = first_now.is_some() && first_now == spare0 && (first_was.is_none() || g.spare.iter().any(|x| Some(&x.name) == first_was.as_ref()));
    g.act(form(&[("grp", "1"), ("general_off", "1")]));
    let relieved = n0 < 2 || g.h.war.command[me].as_ref().and_then(|v| v.get(1)).is_some_and(|x| x.general.is_none());
    let last = n0.saturating_sub(1);
    let ids: Vec<u32> = g.h.war.corps.as_ref().map_or(Vec::new(), |c| c.divs.iter().filter(|d| d.side == me && d.group == last).map(|d| d.id).collect());
    g.act(form(&[("grp", &last.to_string()), ("disband", "1")]));
    let n1 = g.h.war.command[me].as_ref().map_or(0, |v| v.len());
    let in_reserve = g.h.war.corps.as_ref().is_some_and(|c| ids.iter().all(|&id| c.find(id).is_some_and(|i| c.divs[i].group == strategy::military::corps::NONE)));
    let bad_group = g.h.war.corps.as_ref().is_some_and(|c| c.divs.iter().any(|d| d.side == me && d.group != strategy::military::corps::NONE && d.group >= n1));
    drop(g);
    let ok = n0 >= 2 && swapped && relieved && n1 + 1 == n0 && in_reserve && !bad_group;
    say(
        ok,
        "генералы и армии: смена генерала из резерва, снятие, расформирование (дивизии — в резерв ставки)",
        format!("армий {n0} → {n1}; смена {swapped}, снятие {relieved}, дивизий расформированной {} в резерве {in_reserve}, номера групп в порядке {}", ids.len(), !bad_group),
    );
}
