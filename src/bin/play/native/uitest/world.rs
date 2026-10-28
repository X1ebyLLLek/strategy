//! The window's self-checks of the world it shows: a railway laid to the front feeds it, the
//! countries' flags their own.

use super::*;

/// 19. a railway laid to the front feeds it: the player's own province farthest from the
/// capital (the land's railways taken away), a railway laid to it; its supply rises
pub(super) fn rail(shared: &Arc<Mutex<Game>>, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let mut g2 = shared.lock().unwrap().clone();
    let me = g2.me;
    g2.h.war.grade = Vec::new();
    g2.state.works.clear();
    let sup0 = g2.h.war.supply(me);
    let cap = g2.h.war.th.capital[me];
    let own: Vec<usize> = (0..g2.h.war.owner.len()).filter(|&q| g2.h.war.owner[q] == me && sup0[q] > 0.0).collect();
    // (the farthest within twelve steps: there the supply falls off, and a line brings it back)
    let far = own.iter().copied().filter_map(|q| crate::world::land::plan(&g2.h.war, me, cap, q).map(|p| (q, p.len()))).filter(|x| x.1 <= 13).max_by_key(|x| (x.1, std::cmp::Reverse(x.0))).map(|x| x.0);
    let (ok, detail) = match far {
        Some(q) => {
            let path = crate::world::land::plan(&g2.h.war, me, cap, q).unwrap();
            let n = path.len();
            g2.h.home_mut(me).treasury += 1e6;
            let rail = g2.has_rails();
            let begun = g2.begin_line(me, rail, path);
            for _ in 0..n + 2 {
                g2.lay_month();
            }
            let s1 = g2.h.war.supply(me)[q];
            (begun && s1 >= sup0[q] + 0.1, format!("провинция в {n} шагах: снабжение было {:.2}, с линией ({}) {s1:.2}", sup0[q], g2.line_name(rail)))
        }
        None => (false, "своей провинции, до которой можно проложить линию, нет".into()),
    };
    say(ok, "линия снабжения к фронту кормит войска (снабжение растёт)", detail);
}

/// 20. the flags: each country's own, none the same as a neighbour's, in the deep colours of a
/// cloth (not the map's pale ones)
pub(super) fn flags(shared: &Arc<Mutex<Game>>, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let g = shared.lock().unwrap();
    let fl = super::super::flags::of_world(&g);
    let mut same = Vec::<String>::new();
    let mut pale = Vec::<String>::new();
    let key = |f: &super::super::flags::Flag| format!("{:?}{:?}{:?}{:?}", f.cut, f.cols, f.sign, f.star);
    for c in 0..fl.len() {
        let (_, s, v) = super::super::flags::hsv_of(fl[c].cols[0]);
        // (a cloth's colour is deep: saturated, or dark; the map's are muted — at most 0.6
        // saturated and at least 0.65 bright)
        if !(s >= 0.7 || v <= 0.35) {
            pale.push(g.countries[c].0.clone());
        }
        let nb: std::collections::BTreeSet<usize> = (0..g.world.provinces.len())
            .filter(|&p| g.country_of[p] as usize == c)
            .flat_map(|p| g.world.provinces[p].neighbors.iter().map(|&q| g.country_of[q as usize] as usize).collect::<Vec<_>>())
            .filter(|&d| d != c && d < fl.len())
            .collect();
        for d in nb {
            if d > c && key(&fl[c]) == key(&fl[d]) {
                same.push(format!("{} и {}", g.countries[c].0, g.countries[d].0));
            }
        }
    }
    say(
        same.is_empty() && pale.is_empty() && fl.len() == g.countries.len(),
        "флаги стран свои: у соседей разные, в густых цветах полотнища",
        if same.is_empty() && pale.is_empty() { format!("флагов {}", fl.len()) } else { format!("одинаковые: {}; бледные: {}", same.iter().take(2).cloned().collect::<Vec<_>>().join(", "), pale.iter().take(3).cloned().collect::<Vec<_>>().join(", ")) },
    );
}

/// The law of the draft: in the army's folder a click on another law's tile sends the choice to
/// the government (in the journal) and the country lives by it at once (the spoiled control: the
/// law changes nothing)
pub(super) fn draft_law(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    use strategy::economy::nation::LAWS;
    app.tab = Some(Tab::Army);
    // (the law of the draft: the army's page «Бюджет», 28th session)
    super::super::set_page(ctx, Tab::Army, 3);
    app.menu = false;
    app.chosen = None;
    let mut t = ctx.input(|i| i.time);
    let mut step = |app: &mut App, events: Vec<egui::Event>| {
        t += 1.0 / 60.0;
        frame_at(ctx, app, 1600.0, 900.0, events, t);
    };
    for _ in 0..20 {
        step(app, vec![]);
    }
    // (the folder turned down till the law is in sight)
    let over = Pos2::new(190.0, 520.0);
    let mut row = super::super::chrome::spots(ctx).get("draft").copied();
    for _ in 0..60 {
        if row.is_some() {
            break;
        }
        step(app, vec![egui::Event::PointerMoved(over), egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, -90.0), modifiers: Default::default() }]);
        step(app, vec![]);
        row = super::super::chrome::spots(ctx).get("draft").copied();
    }
    let (me, before, now) = {
        let g = shared.lock().unwrap();
        (g.me, g.journal.len(), g.law(g.me))
    };
    // (another law, its tile's middle within the folder)
    let folder = egui::containers::panel::PanelState::load(ctx, egui::Id::new("folder")).map_or(Rect::NOTHING, |p| p.rect);
    let middle = |r: Rect, k: usize| Pos2::new(r.min.x + (k as f32 + 0.5) * r.width() / LAWS.len() as f32, r.center().y);
    let k = row.and_then(|r| [4, 2, 1, 0, 3].into_iter().find(|&k| k != now && folder.contains(middle(r, k)))).unwrap_or(if now == 4 { 3 } else { 4 });
    let (mut sent, mut lives) = (false, false);
    if let Some(r) = row {
        let p = middle(r, k);
        for ev in [vec![egui::Event::PointerMoved(p)], vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }], vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }], vec![]] {
            step(app, ev);
        }
        let g = shared.lock().unwrap();
        sent = g.journal[before..].iter().any(|x| matches!(&x.2, crate::tools::saves::Act::Form(f) if f.iter().any(|(a, b)| a == "draft" && *b == k.to_string())));
        lives = (g.h.home(me).draft_cap - LAWS[k].cap).abs() < 1e-6 && (g.h.home(me).draft_pay - LAWS[k].pay).abs() < 1e-6;
    }
    // (back to the custom law: the checks after this one see the world as it was)
    {
        let mut g = shared.lock().unwrap();
        g.draft[me] = None;
        g.keep_laws();
    }
    // (the folder slid shut again before the next check)
    app.tab = None;
    for _ in 0..30 {
        step(app, vec![egui::Event::PointerGone]);
    }
    say(
        row.is_some() && sent && lives,
        "закон о призыве: щелчок по плитке — решение уходит в правительство, страна живёт по нему",
        match row {
            Some(_) => format!("«{}» → «{}»: в журнале {}, в стране {}", LAWS[now].name, LAWS[k].name, if sent { "есть" } else { "нет" }, if lives { "действует" } else { "не действует" }),
            None => "плиток закона в папке «Армия» не видно".into(),
        },
    );
}
