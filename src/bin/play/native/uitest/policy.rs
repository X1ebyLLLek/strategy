//! The window's self-checks of the country's folders (21st session): its features seen as
//! emblems, the order of its economy taken by a click, the priority of production reaching the
//! staff.

use super::*;

/// Frames of the window at 1600×900, the clock going on by a 60th of a second each.
fn stepper(ctx: &egui::Context) -> impl FnMut(&mut App, Vec<egui::Event>) + '_ {
    let mut t = ctx.input(|i| i.time);
    move |app: &mut App, events: Vec<egui::Event>| {
        t += 1.0 / 60.0;
        frame_at(ctx, app, 1600.0, 900.0, events, t);
    }
}

/// A click at `p`: the hand there, pressed, let go, a frame after.
fn click(step: &mut impl FnMut(&mut App, Vec<egui::Event>), app: &mut App, p: Pos2) {
    for ev in [
        vec![egui::Event::PointerMoved(p)],
        vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }],
        vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }],
        vec![],
    ] {
        step(app, ev);
    }
}

/// The folder `tab` open and turned down till the spot `name` is in sight (none: it never was).
fn find(step: &mut impl FnMut(&mut App, Vec<egui::Event>), app: &mut App, ctx: &egui::Context, tab: Tab, name: &str) -> Option<Rect> {
    app.tab = Some(tab);
    app.menu = false;
    app.chosen = None;
    for _ in 0..20 {
        step(app, vec![]);
    }
    let over = Pos2::new(190.0, 520.0);
    let mut at = super::super::chrome::spots(ctx).get(name).copied();
    for _ in 0..60 {
        if at.is_some() {
            break;
        }
        step(app, vec![egui::Event::PointerMoved(over), egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, -90.0), modifiers: Default::default() }]);
        step(app, vec![]);
        at = super::super::chrome::spots(ctx).get(name).copied();
    }
    // (the scroll glides on a little after the wheel: the spot where it comes to rest — 28th
    // session: each page its own scroll, a folder now opens at its top and is turned down here)
    for _ in 0..40 {
        step(app, vec![]);
        let now = super::super::chrome::spots(ctx).get(name).copied();
        if now.is_some() && now == at {
            break;
        }
        at = now;
    }
    at
}

/// The folder slid shut before the next check.
fn close(step: &mut impl FnMut(&mut App, Vec<egui::Event>), app: &mut App) {
    app.tab = None;
    for _ in 0..30 {
        step(app, vec![egui::Event::PointerGone]);
    }
}

/// 33. the country's features are seen: in the folder «Страна» every feature of the country has
/// its emblem with its name.
pub(super) fn features(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let mut step = stepper(ctx);
    app.tab = Some(Tab::Nation);
    app.menu = false;
    let mut out = None;
    for _ in 0..24 {
        out = Some(frame(ctx, app, 1600.0, 900.0));
    }
    let names: Vec<String> = {
        let g = shared.lock().unwrap();
        g.spirits.list[g.me].iter().map(|sp| sp.name(&g.pack)).collect()
    };
    // (an emblem's name may break into two lines: its first word is enough; the folder turned
    // down a little at a time till every name has been in sight — the emblems may go below the
    // fold)
    let mut seen_by: Vec<bool> = vec![false; names.len()];
    let over = Pos2::new(190.0, 520.0);
    let look = |out: &egui::FullOutput, seen_by: &mut Vec<bool>| {
        let shown = texts_of(&out.shapes, true);
        for (i, n) in names.iter().enumerate() {
            let first = n.split_whitespace().next().unwrap_or("");
            if shown.iter().any(|t| t.text.starts_with(first) && t.rect.max.x < super::super::FOLDER + 10.0) {
                seen_by[i] = true;
            }
        }
    };
    if let Some(o) = &out {
        look(o, &mut seen_by);
    }
    for _ in 0..12 {
        if seen_by.iter().all(|&x| x) {
            break;
        }
        step(app, vec![egui::Event::PointerMoved(over), egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, -60.0), modifiers: Default::default() }]);
        for _ in 0..10 {
            let o = frame(ctx, app, 1600.0, 900.0);
            look(&o, &mut seen_by);
            out = Some(o);
        }
    }
    let seen = seen_by.iter().filter(|&&x| x).count();
    if std::env::var("UIDBG").is_ok() {
        eprintln!("UIDBG features unseen: {:?}", names.iter().zip(&seen_by).filter(|x| !*x.1).map(|x| x.0.clone()).collect::<Vec<_>>());
        let shown = out.as_ref().map(|o| texts_of(&o.shapes, true)).unwrap_or_default();
        eprintln!("UIDBG last shown: {:?}", shown.iter().filter(|t| t.rect.max.x < super::super::FOLDER + 10.0).map(|t| (t.text.clone(), t.rect.min.y as i32)).collect::<Vec<_>>());
    }
    close(&mut step, app);
    say(
        !names.is_empty() && seen == names.len(),
        "особенности страны видны эмблемами в папке «Страна»",
        format!("особенностей {}, подписей у эмблем {seen}", names.len()),
    );
}

/// 34. the order of the economy: a click on another law's tile — the decision in the journal, the
/// country lives by it (its effects laid on the economy next month).
pub(super) fn eco_law(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    use crate::country::policy::LAWS;
    let mut step = stepper(ctx);
    super::super::set_page(ctx, Tab::Nation, 1);
    let row = find(&mut step, app, ctx, Tab::Nation, "ecolaw");
    let (me, before, now) = {
        let g = shared.lock().unwrap();
        (g.me, g.journal.len(), g.policy.law[g.me])
    };
    let folder = egui::containers::panel::PanelState::load(ctx, egui::Id::new("folder")).map_or(Rect::NOTHING, |p| p.rect);
    let middle = |r: Rect, k: usize| Pos2::new(r.min.x + (k as f32 + 0.5) * r.width() / LAWS.len() as f32, r.center().y);
    let k = row.and_then(|r| [2, 3, 0, 1].into_iter().find(|&k| k != now && folder.contains(middle(r, k)))).unwrap_or(2);
    let (mut sent, mut lives) = (false, false);
    if let Some(r) = row {
        click(&mut step, app, middle(r, k));
        let mut g = shared.lock().unwrap();
        sent = g.journal[before..].iter().any(|x| matches!(&x.2, crate::tools::saves::Act::Form(f) if f.iter().any(|(a, b)| a == "ecolaw" && *b == k.to_string())));
        // (its effects as the next month lays them: the military trades' output)
        g.apply_mods();
        let want = crate::country::policy::law_eff(k, g.h.at_war).out_military;
        lives = g.policy.law[me] == k && (g.effs[me].out_military / g.spirit_eff(me).out_military / want - 1.0).abs() < 0.02;
    }
    {
        let mut g = shared.lock().unwrap();
        g.policy.law[me] = now;
        g.apply_mods();
    }
    close(&mut step, app);
    say(
        row.is_some() && sent && lives,
        "хозяйственный строй: щелчок по плитке — решение в журнале, страна живёт по закону",
        match row {
            Some(_) => format!("«{}» → «{}»: в журнале {}, в стране {}", LAWS[now].name, LAWS[k].name, if sent { "есть" } else { "нет" }, if lives { "действует" } else { "не действует" }),
            None => "плиток строя в папке «Страна» не видно".into(),
        },
    );
}

/// 37. the budget's sheet (28th session): in the economy's page «Бюджет» the + in the tax's own
/// line — the decision in the journal (the tax a point higher), the country's tax raised.
pub(super) fn tax_step(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let mut step = stepper(ctx);
    {
        let mut g = shared.lock().unwrap();
        g.adviser = None;
    }
    super::super::set_page(ctx, Tab::Econ, 0);
    let plus = find(&mut step, app, ctx, Tab::Econ, "tax_plus");
    let (me, before, now) = {
        let g = shared.lock().unwrap();
        (g.me, g.journal.len(), g.h.home(g.me).tax)
    };
    let want = format!("{:.0}", ((now * 100.0).round() + 1.0).clamp(0.0, 80.0));
    let (mut sent, mut lives) = (false, false);
    if let Some(r) = plus {
        click(&mut step, app, r.center());
        let g = shared.lock().unwrap();
        sent = g.journal[before..].iter().any(|x| matches!(&x.2, crate::tools::saves::Act::Form(f) if f.iter().any(|(a, b)| a == "tax" && *b == want)));
        lives = (g.h.home(me).tax * 100.0 - want.parse::<f32>().unwrap_or(-1.0)).abs() < 0.01;
    }
    {
        let mut g = shared.lock().unwrap();
        g.h.home_mut(me).tax = now;
    }
    close(&mut step, app);
    say(
        plus.is_some() && sent && lives,
        "бюджет: «+» в строке налога — решение в журнале, налог страны на пункт выше",
        match plus {
            Some(_) => format!("налог {}% → {want}%: в журнале {}, в стране {}", (now * 100.0).round(), if sent { "есть" } else { "нет" }, if lives { "действует" } else { "не действует" }),
            None => "кнопки «+» у налога на странице «Бюджет» не видно".into(),
        },
    );
}

/// 35. the priority of production: in the folder «Производство» a click on «Больше» of the first
/// line — the decision in the journal, the staff's share of that kind raised.
pub(super) fn priority(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    use crate::army::arms::{PRIO, PRIO_USUAL};
    let mut step = stepper(ctx);
    let row = find(&mut step, app, ctx, Tab::Arms, "prio");
    let (me, before, u) = {
        let g = shared.lock().unwrap();
        let me = g.me;
        // (the first line the folder shows: as it chooses them)
        let u = (0..g.pack.units.len()).find(|&u| {
            let l = &g.arms.lines[me][u];
            !(g.h.war.sides[me].army.count[u] < 0.01 && l.made < 1e-4 && g.h.war.sides[me].making.get(u).copied().unwrap_or(0.0) <= 0.0 && l.prio == PRIO_USUAL && l.models.len() == 1)
        });
        (me, g.journal.len(), u)
    };
    let k = 3;
    let (mut sent, mut lives) = (false, false);
    if let (Some(r), Some(u)) = (row, u) {
        let p = Pos2::new(r.min.x + (k as f32 + 0.5) * r.width() / 5.0, r.center().y);
        click(&mut step, app, p);
        let mut g = shared.lock().unwrap();
        sent = g.journal[before..].iter().any(|x| matches!(&x.2, crate::tools::saves::Act::Form(f) if f.iter().any(|(a, b)| a == "prio" && *b == format!("{u}:{k}"))));
        g.arms_before();
        let c = &g.h.war.sides[me];
        lives = g.arms.lines[me][u].prio == k && (c.prio.is_empty() || (c.make_share(u) - c.making[u] * PRIO[k].0).abs() < 1e-6) && !c.prio.is_empty();
    }
    if let Some(u) = u {
        let mut g = shared.lock().unwrap();
        g.arms.lines[me][u].prio = PRIO_USUAL;
        g.arms_before();
    }
    close(&mut step, app);
    say(
        row.is_some() && sent && lives,
        "приоритет выпуска: щелчок «Больше» — решение в журнале, генштаб даёт роду больше денег",
        match row {
            Some(_) => format!("в журнале {}, у генштаба {}", if sent { "есть" } else { "нет" }, if lives { "доля поднята" } else { "доля прежняя" }),
            None => "плиток приоритета в папке «Производство» не видно".into(),
        },
    );
}
