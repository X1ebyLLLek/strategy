//! The window's self-checks of what the hand meets: the icons, the cards of what it rests on (whole
//! on the screen, fading in, readable), the era's cursor and its looks, an alert under the bar.

use super::*;

/// 7. the icons: every sphere's and every kind's is in the set
pub(super) fn icons(shared: &Arc<Mutex<Game>>, app: &mut App, control: bool, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let mut want: Vec<String> = ["econ", "science", "army", "castle", "news", "play", "pause"].iter().map(|s| s.to_string()).collect();
    want.extend(shared.lock().unwrap().pack.units.iter().map(|u| u.icon.clone()));
    if control {
        want.push("nothing".into());
    }
    let missing: Vec<&String> = want.iter().filter(|n| !app.icons.contains_key(*n)).collect();
    say(missing.is_empty(), "значки на месте", format!("нужно {}, нет {}{}", want.len(), missing.len(), if missing.is_empty() { String::new() } else { format!(" ({})", missing.iter().take(3).map(|s| s.as_str()).collect::<Vec<_>>().join(", ")) }));
}

/// 13–14. the cards of what the hand rests on: over the country's bar, the clock, the map's last
/// mode (the window's right bottom corner), a group's card, a province at the map's right, a
/// slider in a folder — each card comes (fading in), lies whole on the screen, does not cover
/// what the hand rests on nor the hand, and reads
pub(super) fn tips(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let (w, h) = (1600.0f32, 900.0f32);
    let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(w, h));
    app.tab = None;
    app.chosen = None;
    app.edit = None;
    app.map = None;
    for _ in 0..20 {
        frame(ctx, app, w, h);
    }
    let spots = super::super::chrome::spots(ctx);
    let mut targets: Vec<(String, Pos2, Option<Tab>)> = Vec::new();
    for (name, key) in [("слово верхней полосы", "word"), ("дата", "date"), ("последний режим карты", "mode")] {
        if let Some(r) = spots.get(key) {
            targets.push((name.to_string(), r.center(), None));
        }
    }
    if let Some(r) = app.cards.first() {
        targets.push(("карточка группы".into(), r.center(), None));
    }
    // (the province of the war farthest right on the map, not under anything)
    {
        let g = shared.lock().unwrap();
        let m = app.map.as_ref().unwrap();
        let free = |p: Pos2| m.last_rect.shrink(30.0).contains(p) && !m.last_mini.expand(30.0).contains(p);
        let land = |p: Pos2| m.province_at(&g, p).is_some_and(|x| !g.world.provinces[x].sea);
        let best = g.h.war.th.provs.iter().map(|&p| m.screen_of(&g, p)).filter(|&p| free(p) && land(p)).max_by(|a, b| a.x.total_cmp(&b.x));
        if let Some(p) = best {
            targets.push(("провинция у правого края карты".into(), p, None));
        }
    }
    targets.push(("ползунок в папке".into(), Pos2::ZERO, Some(Tab::Econ)));
    let (mut tip_bad_place, mut tip_bad_read, mut tip_bad_fade, mut tip_seen) = (Vec::<String>::new(), Vec::<String>::new(), Vec::<String>::new(), 0usize);
    for (name, mut p, tab) in targets.clone() {
        // (the hand away first, so that the rest begins anew)
        let mut t = ctx.input(|i| i.time);
        app.tab = tab;
        // (the economy's laws in tiles: its page «Деньги», 28th session)
        if tab == Some(Tab::Econ) {
            super::super::set_page(ctx, Tab::Econ, 2);
        }
        for _ in 0..20 {
            t += 1.0 / 60.0;
            frame_at(ctx, app, w, h, vec![egui::Event::PointerGone], t);
        }
        if tab.is_some() {
            // (the folder turned down till a law's tiles are in sight, and come to rest)
            let over = Pos2::new(190.0, 520.0);
            for _ in 0..40 {
                if super::super::chrome::spots(ctx).get("slider").is_some() {
                    break;
                }
                t += 1.0 / 60.0;
                frame_at(ctx, app, w, h, vec![egui::Event::PointerMoved(over), egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, -90.0), modifiers: Default::default() }], t);
            }
            for _ in 0..30 {
                t += 1.0 / 60.0;
                frame_at(ctx, app, w, h, vec![egui::Event::PointerMoved(over)], t);
            }
            for _ in 0..20 {
                t += 1.0 / 60.0;
                frame_at(ctx, app, w, h, vec![egui::Event::PointerGone], t);
            }
            match super::super::chrome::spots(ctx).get("slider") {
                Some(r) => p = r.center(),
                None => {
                    tip_bad_place.push(format!("{name}: не найден"));
                    continue;
                }
            }
        }
        let (mut first, mut last): (Option<f32>, Option<(super::super::chrome::Shown, egui::FullOutput)>) = (None, None);
        for _ in 0..45 {
            t += 1.0 / 60.0;
            let out = frame_at(ctx, app, w, h, vec![egui::Event::PointerMoved(p)], t);
            if let Some(s) = super::super::chrome::shown(ctx) {
                first.get_or_insert(s.alpha);
                last = Some((s, out));
            }
        }
        let Some((s, out)) = last else {
            tip_bad_place.push(format!("{name}: подсказки нет"));
            continue;
        };
        tip_seen += 1;
        let hand = super::super::cursor::drawn(ctx).map_or(Rect::from_min_size(p, egui::vec2(24.0, 28.0)), |d| d.rect);
        if !screen.contains_rect(s.rect) {
            tip_bad_place.push(format!("{name}: за краем экрана ({:.0},{:.0}–{:.0},{:.0})", s.rect.min.x, s.rect.min.y, s.rect.max.x, s.rect.max.y));
        } else if s.rect.intersects(s.anchor.shrink(1.0)) || s.rect.intersects(hand.shrink(2.0)) {
            tip_bad_place.push(format!("{name}: закрывает то, на что наведена мышь"));
        }
        // (the card's words lie on its plate; a word of the map under the card's edge lies on none)
        let inside: Vec<Text> = texts(&out.shapes).into_iter().filter(|x| s.rect.contains(x.rect.center()) && x.under.is_some()).collect();
        if inside.is_empty() {
            tip_bad_read.push(format!("{name}: слов не видно"));
        }
        for x in &inside {
            let r = x.under.map_or(0.0, |bg| contrast(x.colour, bg));
            if x.size < 12.0 || r < if x.size >= 18.0 { 3.0 } else { 4.5 } {
                tip_bad_read.push(format!("{name}: «{}» {:.0} пт, контраст {r:.1}", x.text.chars().take(16).collect::<String>(), x.size));
                break;
            }
        }
        if !(first.unwrap_or(1.0) < 0.999 && s.alpha >= 0.999) {
            tip_bad_fade.push(format!("{name}: сразу целиком"));
        }
    }
    app.tab = None;
    say(
        tip_seen == targets.len() && tip_bad_place.is_empty() && tip_bad_fade.is_empty(),
        "подсказка целиком на экране, не закрывает то, на что наведена мышь, и появляется плавно",
        if tip_bad_place.is_empty() && tip_bad_fade.is_empty() { format!("карточек {tip_seen} из {}: строка, дата, режим в углу, карточка группы, провинция у края, ползунок", targets.len()) } else { tip_bad_place.iter().chain(&tip_bad_fade).take(3).cloned().collect::<Vec<_>>().join("; ") },
    );
    say(
        tip_seen == targets.len() && tip_bad_read.is_empty(),
        "подсказка читается: не мельче 12 пт, контраст по WCAG",
        if tip_bad_read.is_empty() { format!("карточек {tip_seen}, все надписи в них читаются") } else { tip_bad_read.iter().take(3).cloned().collect::<Vec<_>>().join("; ") },
    );
}

/// 15–16. the cursor: the system's hidden, the era's drawn last (over everything) where the
/// hand is in that very frame; its look by what the hand does — the arrow over the map, a hand
/// over buttons, an open hand dragging the map, a sword or a shield with a group chosen over
/// the enemy's land or ours, a crate when a depot is laid
pub(super) fn cursor(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, t0: std::time::Instant, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let (w, h) = (1600.0f32, 900.0f32);
    for _ in 0..20 {
        frame(ctx, app, w, h);
    }
    let (enemy, own) = {
        let g = shared.lock().unwrap();
        let m = app.map.as_ref().unwrap();
        let war = &g.h.war;
        // (a free place: on the map, off the small map, off the counters and the month's battles —
        // over those the hand is the right cursor)
        let counters = m.counters.lock().unwrap().clone();
        let fights: Vec<Pos2> = war.fought.iter().filter(|f| f.month + 1 >= war.month).map(|f| m.screen_of(&g, war.th.provs[f.province])).collect();
        let free = |p: Pos2| m.last_rect.shrink(40.0).contains(p) && !m.last_mini.expand(30.0).contains(p) && !counters.iter().any(|r| r.expand(40.0).contains(p)) && !fights.iter().any(|b| b.distance(p) < 40.0);
        let at = |side_me: bool| war.th.provs.iter().enumerate().filter(|(q, _)| (war.owner[*q] == g.me) == side_me).map(|(_, &p)| m.screen_of(&g, p) + egui::vec2(2.0, 2.0)).find(|&p| free(p) && m.province_at(&g, p).is_some_and(|x| war.th.at.get(x).is_some_and(|&qq| qq != strategy::military::front::NO && (war.owner[qq as usize] == g.me) == side_me)));
        (at(false), at(true))
    };
    let spots = super::super::chrome::spots(ctx);
    use super::super::cursor::Kind;
    let mut tries: Vec<(&str, Option<Pos2>, Kind, u8)> = vec![
        ("над картой", own, Kind::Arrow, 0),
        ("над кнопкой сферы", spots.get("sphere").map(|r| r.center()), Kind::Hand, 0),
        ("над режимом карты", spots.get("mode").map(|r| r.center()), Kind::Hand, 0),
        ("группа выбрана, враг под рукой", enemy, Kind::Attack, 2),
        ("группа выбрана, своя земля", own, Kind::Hold, 2),
        ("склад", own, Kind::Depot, 3),
        ("тащит карту", own, Kind::Grab, 1),
    ];
    let (mut cur_hidden, mut cur_top, mut cur_place, mut cur_kind) = (Vec::<String>::new(), Vec::<String>::new(), Vec::<String>::new(), Vec::<String>::new());
    let mut looks: std::collections::HashMap<Kind, std::collections::HashSet<egui::TextureId>> = Default::default();
    for (name, p, want, how) in tries.drain(..) {
        if std::env::var("UITIME").is_ok() {
            eprintln!("{:.1} с: курсор {name} {p:?}", t0.elapsed().as_secs_f32());
        }
        let Some(p) = p else {
            cur_kind.push(format!("{name}: места не нашлось"));
            continue;
        };
        app.chosen = (how == 2).then_some(0);
        if let Some(m) = app.map.as_mut() {
            m.build = if how == 3 { super::super::map::Build::Depot } else { super::super::map::Build::None };
        }
        let mut t = ctx.input(|i| i.time);
        let p0 = p + egui::vec2(37.0, -23.0);
        let mut step = |events: Vec<egui::Event>| {
            t += 1.0 / 60.0;
            if std::env::var("UITIME").is_ok() {
                eprintln!("  шаг {:.1} с", t0.elapsed().as_secs_f32());
            }
            frame_at(ctx, app, w, h, events, t)
        };
        step(vec![egui::Event::PointerMoved(p0)]);
        step(vec![egui::Event::PointerMoved(p0)]);
        // (the hand comes: the cursor must be there in this very frame)
        // (in a war of divisions the map is dragged by the right button: the left draws a box)
        let drag_button = if shared.lock().unwrap().h.war.corps.is_some() { egui::PointerButton::Secondary } else { egui::PointerButton::Primary };
        let out = if how == 1 {
            let press = |pos: Pos2, pressed: bool| egui::Event::PointerButton { pos, button: drag_button, pressed, modifiers: Default::default() };
            step(vec![egui::Event::PointerMoved(p), press(p, true)]);
            step(vec![egui::Event::PointerMoved(p + egui::vec2(12.0, 6.0))]);
            step(vec![egui::Event::PointerMoved(p + egui::vec2(30.0, 14.0))])
        } else {
            let first = step(vec![egui::Event::PointerMoved(p)]);
            let d = super::super::cursor::drawn(ctx);
            // (the picture is drawn a little ahead of a fast hand — half a frame of its speed, at
            // most 24 points: `cursor::draw`; a frame late, it would be where the hand was)
            if !d.is_some_and(|d| d.pointer == p && d.rect.expand(24.0).contains(p)) {
                cur_place.push(format!("{name}: курсор отстал от руки"));
            }
            drop(first);
            step(vec![egui::Event::PointerMoved(p)]);
            step(vec![egui::Event::PointerMoved(p)])
        };
        let d = super::super::cursor::drawn(ctx);
        if out.platform_output.cursor_icon != egui::CursorIcon::None {
            cur_hidden.push(format!("{name}: системный курсор {:?}", out.platform_output.cursor_icon));
        }
        // (the last thing painted is the cursor's picture, where it was drawn)
        match (out.shapes.last(), d) {
            (Some(egui::epaint::ClippedShape { shape: Shape::Mesh(mesh), .. }), Some(d)) if (mesh.calc_bounds().center() - d.rect.center()).length() < 1.0 => {
                looks.entry(d.kind).or_default().insert(mesh.texture_id);
            }
            _ => cur_top.push(format!("{name}: поверх него что-то ещё")),
        }
        if d.map(|d| d.kind) != Some(want) {
            cur_kind.push(format!("{name}: {:?} вместо {want:?}", d.map(|d| d.kind)));
        }
        if how == 1 {
            let t2 = ctx.input(|i| i.time) + 1.0 / 60.0;
            frame_at(ctx, app, w, h, vec![egui::Event::PointerButton { pos: p + egui::vec2(30.0, 14.0), button: drag_button, pressed: false, modifiers: Default::default() }], t2);
        }
    }
    app.chosen = None;
    if let Some(m) = app.map.as_mut() {
        m.build = super::super::map::Build::None;
    }
    let all_looks: std::collections::HashSet<egui::TextureId> = looks.values().flatten().copied().collect();
    let kinds_seen = looks.len();
    say(
        cur_hidden.is_empty() && cur_top.is_empty() && cur_place.is_empty(),
        "курсор эпохи: системный спрятан, свой нарисован поверх всего и там, где рука в этом же кадре",
        if cur_hidden.is_empty() && cur_top.is_empty() && cur_place.is_empty() { "в 7 положениях руки".into() } else { cur_hidden.iter().chain(&cur_top).chain(&cur_place).take(3).cloned().collect::<Vec<_>>().join("; ") },
    );
    say(
        cur_kind.is_empty() && kinds_seen == 6 && all_looks.len() == 6,
        "курсор меняет вид: стрелка, рука, ладонь, атака, держать, склад",
        if cur_kind.is_empty() && all_looks.len() == 6 { format!("видов {kinds_seen}, картинок {}", all_looks.len()) } else { format!("{}; картинок разных {}", cur_kind.iter().take(3).cloned().collect::<Vec<_>>().join("; "), all_looks.len()) },
    );
}

/// 23. an alert under the bar: the treasury in debt — the alert «Казна в долгах» comes, a click
/// on it opens the economy (the spoiled control: no alerts)
pub(super) fn alert(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let saved = {
        let mut g = shared.lock().unwrap();
        let me = g.me;
        let t = g.h.home(me).treasury;
        g.h.home_mut(me).treasury = -50.0;
        t
    };
    app.tab = None;
    app.menu = false;
    let mut t = ctx.input(|i| i.time);
    for _ in 0..3 {
        t += 1.0 / 60.0;
        frame_at(ctx, app, 1600.0, 900.0, vec![], t);
    }
    let shown = super::super::chrome::spots(ctx).get("alert").copied();
    let mut opened = false;
    if let Some(r) = shown {
        let c = r.center();
        for ev in [vec![egui::Event::PointerMoved(c)], vec![egui::Event::PointerButton { pos: c, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }], vec![egui::Event::PointerButton { pos: c, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }], vec![]] {
            t += 1.0 / 60.0;
            frame_at(ctx, app, 1600.0, 900.0, ev, t);
        }
        opened = app.tab == Some(Tab::Econ);
    }
    {
        let mut g = shared.lock().unwrap();
        let me = g.me;
        g.h.home_mut(me).treasury = saved;
    }
    app.tab = None;
    say(shown.is_some() && opened, "тревога под полосой: долги казны видны, щелчок открывает «Экономику»", format!("тревога {}; папка {}", if shown.is_some() { "есть" } else { "нет" }, if opened { "открылась" } else { "не открылась" }));
}
