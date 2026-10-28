//! The window's self-checks of the map: counters not on each other, the camera tilted close up
//! and kept to the world, a click found where the hand is, a month of war sending the video card
//! only the table of provinces, the counters gliding, the rivers and roads whole, the fog of
//! war, the towns' houses, the land's relief.

use super::*;

/// 4. counters not on each other: the map close to the front, a counter in each province
/// (the folder slid back first: while it moves, the map is fitted again every frame)
pub(super) fn counters(app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    app.tab = None;
    app.chosen = None;
    app.edit = None;
    for _ in 0..20 {
        frame(ctx, app, 1600.0, 900.0);
    }
    let (overlaps, pairs, n) = {
        app.map.as_mut().unwrap().k = map::AGG + 0.1;
        frame(ctx, app, 1600.0, 900.0);
        let mut far_pairs = 0usize;
        let mut bad = 0usize;
        let m = app.map.as_ref().unwrap();
        let c = m.counters.lock().unwrap();
        for i in 0..c.len() {
            for j in i + 1..c.len() {
                far_pairs += 1;
                let x = c[i].intersect(c[j]);
                if x.is_positive() && x.area() > 0.25 * c[i].area().min(c[j].area()) {
                    bad += 1;
                }
            }
        }
        (bad, far_pairs, c.len())
    };
    say(n >= 5 && overlaps == 0, "фишки не лежат друг на друге (вблизи фронта)", format!("фишек {n}, пар {pairs}, налезают больше чем на четверть {overlaps}"));
}

/// 5. the camera: straight down from afar, tilted close up — the land stands up (how many screen
/// points a world pixel takes near the bottom of the view against near its top)
pub(super) fn camera(app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let ratio = |app: &mut App, k: f32| -> f32 {
        app.map.as_mut().unwrap().k = k;
        frame(ctx, app, 1600.0, 900.0);
        let m = app.map.as_ref().unwrap();
        let (cam, r) = (m.cam, m.last_rect);
        let flat = |_: f32, _: f32| 0.0;
        let a = cam.to_world(egui::pos2(r.center().x, r.min.y + r.height() * 0.1), 0.0, &flat);
        let b = cam.to_world(egui::pos2(r.center().x, r.min.y + r.height() * 0.9), 0.0, &flat);
        cam.scale_at(b.0, b.1, 0.0) / cam.scale_at(a.0, a.1, 0.0)
    };
    let (near, far) = (ratio(app, 8.0), ratio(app, 0.8));
    say(
        near > 1.3 && far < 1.05,
        "камера вблизи наклоняется (земля встаёт рельефом), издали смотрит сверху",
        format!("внизу вида мир крупнее, чем вверху: вблизи в {near:.2} раза (нужно больше 1,3), издали в {far:.2} (меньше 1,05)"),
    );
}

/// 6. a click lands where the hand is, in the mountains too: the highest provinces of the war,
/// close up, each found again from its point on the screen
pub(super) fn clicks(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let (hit, tried, worst) = {
        let g = shared.lock().unwrap();
        let th = &g.h.war.th;
        let mut high: Vec<usize> = th.provs.iter().copied().filter(|&p| !g.world.provinces[p].sea).collect();
        let hgt = |p: usize| g.world.height[g.world.provinces[p].seed];
        high.sort_by(|a, b| hgt(*b).total_cmp(&hgt(*a)));
        high.truncate(12);
        drop(g);
        let (mut hit, mut worst) = (0usize, 0.0f32);
        for &p in &high {
            let g = shared.lock().unwrap();
            let seed = g.world.provinces[p].seed;
            let (sx, sy) = ((seed % g.world.w) as f32 + 0.5, (seed / g.world.w) as f32 + 0.5);
            let w = g.world.w as f32;
            drop(g);
            {
                let m = app.map.as_mut().unwrap();
                // (the province below the view's middle: a raised point is seen off where a flat
                // one would be, whether the camera tilts or not)
                m.cx = sx;
                m.cy = sy - 30.0;
                m.k = 8.0;
            }
            frame(ctx, app, 1600.0, 900.0);
            let m = app.map.as_ref().unwrap();
            let on = m.pt((sx, sy));
            let (x, y) = m.world_at(on);
            let dx = (x - sx).abs() % w;
            let err = (dx.min(w - dx).powi(2) + (y - sy).powi(2)).sqrt();
            worst = worst.max(err);
            if err < 0.75 {
                hit += 1;
            }
        }
        (hit, high.len(), worst)
    };
    say(
        tried >= 6 && hit * 6 >= tried * 5,
        "щелчок попадает туда, что под мышью, и в горах",
        format!("самые высокие провинции войны вблизи: найдены точно {hit} из {tried} (промах меньше 0,75 пикселя мира); худший промах {worst:.2}"),
    );
}

/// 8. a month of war sends the video card only the table of provinces, not the land again
pub(super) fn war_month(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    app.map.as_mut().unwrap().k = 1.0;
    frame(ctx, app, 1600.0, 900.0);
    let k0 = app.map.as_ref().unwrap().painted.len();
    {
        let mut g = shared.lock().unwrap();
        let before = g.h.war.owner.clone();
        // (up to eight months: a war may stand still for months)
        for _ in 0..8 {
            g.advance();
            while let Some((id, kind)) = g.ev.waiting.first().map(|e| (e.id, e.kind)) {
                let keep = matches!(kind, crate::country::events::Kind::PeaceOffer | crate::country::events::Kind::Weary);
                g.answer(id, keep as usize);
            }
            if g.h.war.owner != before {
                break;
            }
        }
    }
    frame(ctx, app, 1600.0, 900.0);
    frame(ctx, app, 1600.0, 900.0);
    let m = app.map.as_ref().unwrap();
    let after: Vec<_> = m.painted[k0..].to_vec();
    let ok = !after.is_empty() && after.iter().all(|p| p.0 == "таблица провинций" && p.3 < 0.01);
    say(
        ok,
        "месяц войны: в видеокарту уходит только таблица провинций, не вся земля",
        match after.first() {
            Some(p) => format!("сменили хозяина {} провинций; отправлено: {} — {:.2}% слоёв земли, {:.3} с", p.1, p.0, p.3 * 100.0, p.2),
            None => "земля не сменила хозяина за восемь месяцев — нечего проверить".into(),
        },
    );
}

/// 9. the groups' counters glide to their new places when the front moves (not jump), and
/// arrive: settled, a month of war, one frame, then settled again
pub(super) fn glide(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let settle = |app: &mut App| {
        for _ in 0..200 {
            frame(ctx, app, 1600.0, 900.0);
        }
        app.map.as_ref().unwrap().glides()
    };
    let d = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).hypot(a.1 - b.1);
    let (mut moved, mut smooth) = (Vec::<u64>::new(), 0usize);
    // (up to four months: some months nothing moves)
    for _ in 0..4 {
        let s0 = settle(app);
        {
            let mut g = shared.lock().unwrap();
            g.advance();
            while let Some((id, kind)) = g.ev.waiting.first().map(|e| (e.id, e.kind)) {
                let keep = matches!(kind, crate::country::events::Kind::PeaceOffer | crate::country::events::Kind::Weary);
                g.answer(id, keep as usize);
            }
        }
        frame(ctx, app, 1600.0, 900.0);
        let s1 = app.map.as_ref().unwrap().glides();
        let s2 = settle(app);
        moved = s2.keys().filter(|k| s0.get(k).is_some_and(|&a| d(a, s2[k]) > 2.0 && d(a, s2[k]) < 250.0)).copied().collect();
        smooth = moved.iter().filter(|k| s1.get(k).is_some_and(|&b| d(b, s2[k]) > 0.3 * d(s0[k], s2[k]))).count();
        if !moved.is_empty() {
            break;
        }
    }
    say(
        !moved.is_empty() && smooth == moved.len(),
        "фишки групп едут к новому месту, а не прыгают",
        format!("за месяц сдвинулось фишек {}; на первом кадре ещё в пути {smooth}", moved.len()),
    );
}

/// 17. the camera keeps to the world north and south: put far beyond its top and its bottom,
/// at several zooms, the view's edges on the ground stay within it
pub(super) fn edges(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let (w, h) = (1600.0f32, 900.0f32);
    let mut off = Vec::<String>::new();
    for (k, y) in [(1.0f32, -400.0f32), (1.0, 1500.0), (2.5, -60.0), (6.0, 1100.0), (12.0, -5.0)] {
        if let Some(m) = app.map.as_mut() {
            m.hold();
            m.k = k;
            m.cy = y;
        }
        frame(ctx, app, w, h);
        frame(ctx, app, w, h);
        let g = shared.lock().unwrap();
        let m = app.map.as_ref().unwrap();
        let c = m.cam.corners();
        let (top, bottom) = (c[0].1.min(c[1].1), c[2].1.max(c[3].1));
        let wh = g.world.h as f32;
        if top < -1.0 || bottom > wh + 1.0 {
            off.push(format!("приближение {k}: вид от {top:.0} до {bottom:.0} при мире 0–{wh:.0}"));
        }
    }
    if let Some(m) = app.map.as_mut() {
        m.refit();
    }
    say(off.is_empty(), "камера не уходит за край мира сверху и снизу", if off.is_empty() { "пять видов: далеко, средне, близко, вверху и внизу".into() } else { off.iter().take(2).cloned().collect::<Vec<_>>().join("; ") });
}

/// 18. the rivers and the roads stay whole: along every line, the distance the card reads (the
/// steps of the four texels round a point mixed, as its filter mixes them) is under the line's
/// half-width — read as distances mixed (the naive way, the control) thin lines break into dots
pub(super) fn lines(shared: &Arc<Mutex<Game>>, app: &mut App, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let g = shared.lock().unwrap();
    let m = app.map.as_ref().unwrap();
    let (lines_t, ways_t, detail, ww, wh) = m.line_layers();
    let (w2, h2) = (2 * ww, 2 * wh);
    let spoil = super::super::spoiled();
    // (what the card reads at a world point: bilinear between texel middles)
    let read = |tex: &[u8], ch: usize, x: f32, y: f32, range: f32| -> f32 {
        let (u, v) = (x * 2.0 - 0.5, (y * 2.0 - 0.5).max(0.0));
        let (i0, j0) = (u.floor() as i64, v.floor() as i64);
        let (fu, fv) = (u - i0 as f32, v - j0 as f32);
        let at = |i: i64, j: i64| {
            let k = ((j.clamp(0, h2 as i64 - 1) as usize) * w2 + i.rem_euclid(w2 as i64) as usize) * 4;
            super::super::map::step_of(tex[k + ch], tex[k + ch + 1], range)
        };
        let q = [at(i0, j0), at(i0 + 1, j0), at(i0, j0 + 1), at(i0 + 1, j0 + 1)];
        let wts = [(1.0 - fu) * (1.0 - fv), fu * (1.0 - fv), (1.0 - fu) * fv, fu * fv];
        if spoil {
            q.iter().zip(wts).map(|(s, wt)| s.0.hypot(s.1) * wt).sum()
        } else {
            let (sx, sy) = q.iter().zip(wts).fold((0.0, 0.0), |a, (s, wt)| (a.0 + s.0 * wt, a.1 + s.1 * wt));
            sx.hypot(sy)
        }
    };
    let mut checked = 0usize;
    let mut broken: Vec<(String, usize, usize)> = Vec::new();
    let mut look = |name: &str, pts: &[[f32; 2]], tex: &[u8], ch: usize, range: f32, hw: &dyn Fn(f32, f32) -> f32, checked: &mut usize| {
        let (mut n, mut bad) = (0usize, 0usize);
        for s in 0..pts.len().saturating_sub(1) {
            let (a, b) = (pts[s], pts[s + 1]);
            let len = (b[0] - a[0]).hypot(b[1] - a[1]);
            let steps = (len / 0.2).ceil().max(1.0) as usize;
            for i in 0..steps {
                let t = (i as f32 + 0.5) / steps as f32;
                let (x, y) = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
                if y < 1.0 || y > wh as f32 - 1.0 {
                    continue;
                }
                n += 1;
                let d = read(tex, ch, x, y, range);
                if d > hw(x, y) {
                    bad += 1;
                    if std::env::var("LINEDBG").is_ok() && bad <= 6 {
                        eprintln!("разрыв {name}: ({x:.2},{y:.2}) читается {d:.3}, отрезок {s} из {}, длина {len:.2}", pts.len());
                    }
                }
            }
        }
        *checked += n;
        if let Some(e) = broken.iter_mut().find(|e| e.0 == name) {
            e.1 += bad;
            e.2 += n;
        } else {
            broken.push((name.to_string(), bad, n));
        }
    };
    let river_hw = |x: f32, y: f32| {
        let k = (((y * 2.0) as usize).min(h2 - 1) * w2 + ((x * 2.0).floor() as i64).rem_euclid(w2 as i64) as usize) * 4;
        (detail[k + 2] as f32 / 255.0 * 1.28 * 0.8).max(0.06)
    };
    for (pts, _, _) in g.land.rivers.iter() {
        look("реки", pts, lines_t, 0, 3.0, &river_hw, &mut checked);
    }
    for l in &g.land.ways {
        let (tex, ch, name) = match l.kind {
            crate::world::land::MAIN => (lines_t, 2, "дороги"),
            crate::world::land::LOCAL => (ways_t, 0, "сельские дороги"),
            _ => (ways_t, 2, "железные дороги"),
        };
        // (the narrowest each is drawn: as in the land's shader close up)
        let hw = match l.kind {
            crate::world::land::MAIN => 0.13,
            crate::world::land::LOCAL => 0.08,
            _ => 0.10,
        };
        look(name, &l.pts, tex, ch, 2.0, &|_, _| hw, &mut checked);
    }
    let worst = broken.iter().map(|e| e.1 as f32 / e.2.max(1) as f32).fold(0.0f32, f32::max);
    say(
        checked > 1000 && worst <= 0.01,
        "реки и дороги не рвутся: на всей их длине видеокарта читает линию",
        format!("точек {checked}; разрывов: {}", broken.iter().map(|e| format!("{} {:.1}%", e.0, e.1 as f32 / e.2.max(1) as f32 * 100.0)).collect::<Vec<_>>().join(", ")),
    );
}

/// 21. the fog of war: over the enemy's land the staff does not see — three steps and more from
/// ours — and nowhere else (not on ours, not on the enemy's land along the front)
pub(super) fn fog(shared: &Arc<Mutex<Game>>, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let g = shared.lock().unwrap();
    let war = &g.h.war;
    let fog = super::super::map::MapView::fogged(&g);
    let own: Vec<usize> = (0..war.owner.len()).filter(|&q| war.owner[q] == g.me).collect();
    let d = war.distance_from_all(&own);
    let deep: Vec<usize> = (0..war.owner.len()).filter(|&q| war.owner[q] != g.me && d[q] != strategy::military::front::NO && d[q] >= 3).collect();
    let near: Vec<usize> = (0..war.owner.len()).filter(|&q| d[q] != strategy::military::front::NO && d[q] <= 2).collect();
    let covered = deep.iter().filter(|&&q| fog.get(q).copied().unwrap_or(false)).count();
    let wrong = near.iter().filter(|&&q| fog.get(q).copied().unwrap_or(false)).count();
    say(
        g.h.at_war && !deep.is_empty() && covered == deep.len() && wrong == 0,
        "туман войны лежит над тем, чего генштаб не видит, и только там",
        format!("в глубине вражеской земли {} провинций, под туманом {covered}; наших и прифронтовых под туманом {wrong}", deep.len()),
    );
}

/// 22. the towns: each house stands as a house stands in a town — its long side along the
/// nearest street or road, its front within reach of it, off it, on dry land (the spoiled
/// control: the houses turned any way)
pub(super) fn towns(shared: &Arc<Mutex<Game>>, app: &mut App, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let g = shared.lock().unwrap();
    let (n, bad) = app.map.as_ref().map_or((0, vec!["карты нет".to_string()]), |m| m.towns_check(&g));
    say(
        n > 100 && (bad.len() as f32) <= 0.05 * n as f32,
        "дома стоят вдоль улиц, лицом к улице, не на дорогах и не в воде",
        format!("домов {n}, не так стоят {} ({:.1}%){}", bad.len(), bad.len() as f32 / n.max(1) as f32 * 100.0, bad.first().map_or(String::new(), |b| format!("; например {b}"))),
    );
}

/// (measured on maps 3, 7, 11, 19: massifs ×1.96–2.48, the land of one height ×0.93–1.07)
pub(super) fn relief(shared: &Arc<Mutex<Game>>, app: &mut App, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    const RANGE_RISE: f32 = 1.4;
    // 24. the land's relief: the ranges stand as massifs — high in their middle, low at their
    // feet (not a land of one height, crumpled) — and there is land below the sea's level (the
    // spoiled control: the land of one height as before, no basins)
    {
        let g = shared.lock().unwrap();
        let (deep, edge, below) = app.map.as_ref().map_or((0.0, 1.0, 0), |v| v.relief_check(&g));
        say(
            deep >= RANGE_RISE * edge && below >= 100,
            "рельеф: горы — массивы, в середине хребта выше, чем у подножия; есть земля ниже моря",
            format!("медиана высоты гор в глубине хребта {:.3}, у края {:.3} (×{:.2}); земли ниже моря {below} пикселей", deep, edge, deep / edge.max(1e-6)),
        );
    }
}
