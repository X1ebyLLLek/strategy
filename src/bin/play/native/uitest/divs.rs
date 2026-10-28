//! The window's self-checks of the divisions as in HOI4: stacks chosen by a click and a box, sent
//! by the right button (an order in the journal).

use super::*;

/// 25. the divisions as in HOI4: close up, our stacks are on the map; a click on one chooses its
/// divisions (their panel comes), a box over several chooses them all, the right button on a
/// province of ours sends them there — an order in the journal, their ways laid (the spoiled
/// control: the stacks drawn, but a click does not find them)
pub(super) fn stacks(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    app.tab = None;
    app.menu = false;
    app.chosen = None;
    let (w, h) = (1600.0, 900.0);
    let front_at = {
        let g = shared.lock().unwrap();
        let war = &g.h.war;
        let f = war.front(g.me);
        let q = f.get(f.len() / 2).copied();
        q.map(|q| {
            let seed = g.world.provinces[war.th.provs[q]].seed;
            ((seed % g.world.w) as f32, (seed / g.world.w) as f32)
        })
    };
    if let (Some(m), Some((x, y))) = (app.map.as_mut(), front_at) {
        m.hold();
        m.k = 4.5;
        m.cx = x;
        m.cy = y;
        m.picked.clear();
    }
    let mut t = ctx.input(|i| i.time);
    let mut step = |app: &mut App, events: Vec<egui::Event>| {
        t += 1.0 / 60.0;
        frame_at(ctx, app, w, h, events, t)
    };
    for _ in 0..4 {
        step(app, vec![]);
    }
    let click = |button: egui::PointerButton, p: Pos2| -> Vec<Vec<egui::Event>> {
        vec![
            vec![egui::Event::PointerMoved(p)],
            vec![egui::Event::PointerButton { pos: p, button, pressed: true, modifiers: Default::default() }],
            vec![egui::Event::PointerButton { pos: p, button, pressed: false, modifiers: Default::default() }],
            vec![],
        ]
    };
    // (a stack well inside the map, off the panels and the small map)
    let free_rect = |m: &super::super::map::MapView| m.last_rect.shrink2(egui::vec2(60.0, 80.0));
    let stacks: Vec<(Rect, Vec<u32>, usize)> = app.map.as_ref().map_or(Vec::new(), |m| m.stacks_drawn.lock().unwrap().iter().filter(|s| free_rect(m).contains_rect(s.0) && !m.last_mini.expand(20.0).intersects(s.0)).cloned().collect());
    let drawn = stacks.len();
    let mut why = Vec::<String>::new();
    // a click on one
    let mut chosen_ok = false;
    if let Some((r, ids, _)) = stacks.first().cloned() {
        for ev in click(egui::PointerButton::Primary, r.center()) {
            step(app, ev);
        }
        let got = app.map.as_ref().map_or(Vec::new(), |m| m.picked.clone());
        chosen_ok = !ids.is_empty() && got == ids;
        if !chosen_ok {
            why.push(format!("щелчок по стопке: выбрано {} из {}", got.len(), ids.len()));
        }
    } else {
        why.push("стопок на виду нет".into());
    }
    // a box over two (where they are now: the click above may have moved the map — a panel came)
    let stacks: Vec<(Rect, Vec<u32>, usize)> = app.map.as_ref().map_or(Vec::new(), |m| m.stacks_drawn.lock().unwrap().iter().filter(|s| free_rect(m).contains_rect(s.0) && !m.last_mini.expand(20.0).intersects(s.0)).cloned().collect());
    // (two whose box lies off the panel of the chosen divisions: a press there is the panel's)
    let panel = ctx.memory(|m| m.area_rect(egui::Id::new("picked-panel"))).unwrap_or(Rect::NOTHING);
    let pair = (0..stacks.len()).flat_map(|i| (i + 1..stacks.len()).map(move |j| (i, j))).find(|&(i, j)| !stacks[i].0.union(stacks[j].0).expand(8.0).intersects(panel));
    let mut boxed_ok = false;
    if let Some((i, j)) = pair {
        let (a, b) = (stacks[i].0, stacks[j].0);
        let r = a.union(b).expand(4.0);
        let (p0, p1) = (r.min, r.max);
        let evs = vec![
            vec![egui::Event::PointerMoved(p0)],
            vec![egui::Event::PointerButton { pos: p0, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }],
            vec![egui::Event::PointerMoved(p0 + (p1 - p0) * 0.5)],
            vec![egui::Event::PointerMoved(p1)],
            vec![egui::Event::PointerButton { pos: p1, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }],
            vec![],
        ];
        for ev in evs {
            step(app, ev);
        }
        let got = app.map.as_ref().map_or(Vec::new(), |m| m.picked.clone());
        boxed_ok = stacks[i].1.iter().chain(&stacks[j].1).all(|id| got.contains(id));
        if !boxed_ok {
            why.push(format!("рамка: выбрано {}", got.len()));
        }
    } else if stacks.len() >= 2 {
        why.push("двух стопок вне панели выбранных нет".into());
    }
    // the right button on a province of ours, a step behind the line
    let mut sent_ok = false;
    let target = {
        let g = shared.lock().unwrap();
        let m = app.map.as_ref().unwrap();
        let war = &g.h.war;
        let counters = m.counters.lock().unwrap().clone();
        (0..war.owner.len())
            .filter(|&q| war.owner[q] == g.me && !war.front(g.me).contains(&q))
            .map(|q| (q, m.screen_of(&g, war.th.provs[q]) + egui::vec2(2.0, 2.0)))
            // (not under the panel of the chosen divisions, low on the left)
            .filter(|&(_, p)| !(p.x < m.last_rect.min.x + 470.0 && p.y > m.last_rect.max.y - 420.0))
            .find(|&(q, p)| free_rect(m).contains(p) && !counters.iter().any(|r| r.expand(6.0).contains(p)) && m.province_at(&g, p).and_then(|x| war.th.at.get(x).copied()) == Some(q as u32))
    };
    let picked_now = app.map.as_ref().map_or(Vec::new(), |m| m.picked.clone());
    if let (Some((q, p)), false) = (target, picked_now.is_empty()) {
        let before = shared.lock().unwrap().journal.len();
        for ev in click(egui::PointerButton::Secondary, p) {
            step(app, ev);
        }
        let g = shared.lock().unwrap();
        let last = g.journal.get(before).map(|x| x.2.clone());
        let ways = g.h.war.corps.as_ref().map_or(0, |c| c.divs.iter().filter(|d| picked_now.contains(&d.id) && (d.at == q || d.way.last() == Some(&q))).count());
        sent_ok = matches!(last, Some(crate::tools::saves::Act::Send(ref v, to)) if to == q && *v == picked_now) && ways > 0;
        if !sent_ok {
            why.push(format!("приказ: в журнале {:?}, идут туда {ways} (записей после щелчка {}, выбрано {}, в провинцию {q}, хозяин {})", last.map(|a| matches!(a, crate::tools::saves::Act::Send(..))), g.journal.len() - before, picked_now.len(), g.h.war.owner[q]));
        }
    } else if !picked_now.is_empty() {
        why.push("своей свободной провинции на виду нет".into());
    }
    if let Some(m) = app.map.as_mut() {
        m.picked.clear();
        m.refit();
    }
    say(
        drawn >= 2 && chosen_ok && boxed_ok && sent_ok,
        "дивизии: стопка выбирается щелчком и рамкой, правая кнопка ведёт их (приказ в журнале)",
        if why.is_empty() { format!("стопок на виду {drawn}; щелчок, рамка и приказ сработали") } else { format!("стопок на виду {drawn}; {}", why.join("; ")) },
    );
}
