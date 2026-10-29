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

/// 44. the rules of supply (34th session): a depot is not laid where the capital cannot be reached by one's own land; the
/// farther from the heads of supply the dearer and slower; a depot the enemy takes is lost; a side whose stores do not cover
/// the army it has raises no new division, one whose stores do raises them
pub(super) fn supply_rules(shared: &Arc<Mutex<Game>>, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let mut g = shared.lock().unwrap().clone();
    let me = g.me;
    // (the land's railways taken away: the heads of supply are the capital alone)
    g.h.war.grade = Vec::new();
    g.state.works.clear();
    g.h.war.hubs[me].clear();
    let reach = g.h.war.reach(me);
    let own: Vec<usize> = (0..g.h.war.owner.len()).filter(|&q| g.h.war.owner[q] == me).collect();
    // (cost and time: a province next to the capital against the farthest the capital reaches)
    let near = own.iter().copied().find(|&q| reach[q] == 1);
    let far = own.iter().copied().filter(|&q| reach[q] != strategy::military::front::NO).max_by_key(|&q| (reach[q], std::cmp::Reverse(q)));
    let terms = match (near, far) {
        (Some(a), Some(b)) => {
            let (ca, ma) = g.hub_terms(me, a);
            let (cb, mb) = g.hub_terms(me, b);
            Some((ca, ma, cb, mb, reach[b]))
        }
        _ => None,
    };
    let terms_ok = terms.is_some_and(|(ca, ma, cb, mb, _)| cb > ca * 1.5 && mb > ma);
    // (an enemy province cannot hold our depot; a province of ours cut off from the capital cannot either)
    g.h.home_mut(me).treasury += 1e6;
    let foe_prov = (0..g.h.war.owner.len()).find(|&q| g.h.war.owner[q] != me && g.h.war.th.nbr[q].iter().any(|&r| g.h.war.owner[r] == me));
    let refused = foe_prov.is_some_and(|q| !g.build_depot(q));
    let built = far.is_some_and(|q| {
        let before = g.state.building.len();
        g.build_depot(q) && g.state.building.len() == before + 1
    });
    // (a depot laid, then its province taken: gone from the list)
    let mut lost_ok = false;
    if let Some(q) = near {
        g.h.war.hubs[me].push(q);
        let before = g.h.war.hubs[me].len();
        g.h.war.owner[q] = 1 - me;
        let _ = g.statecraft();
        lost_ok = before >= 1 && !g.h.war.hubs[me].contains(&q);
    }
    say(terms_ok && refused && built && lost_ok, "склад: чужую провинцию не заложить, дальше от голов снабжения дороже и дольше, взятый врагом пропадает", format!("цена/срок {:?}; чужая отказана {refused}; дальняя заложена {built}; потерян при взятии {lost_ok}", terms.map(|t| (format!("{:.2}", t.0), t.1, format!("{:.2}", t.2), t.3, t.4))));

    // the cover of the stores and the raising of divisions: the same war twice, the stores empty and full
    let mut run = |cover: f32, stores: f32| -> (usize, usize) {
        let mut h = shared.lock().unwrap().clone();
        let s = h.me;
        let pack = h.pack.clone();
        h.h.war.stores[s] = stores;
        h.h.war.cover[s] = cover;
        let nu = pack.units.len();
        let before = h.h.war.corps.as_ref().map_or(0, |c| c.divs.iter().filter(|d| d.side == s).count());
        if let Some(sh) = h.h.war.corps.as_mut().and_then(|c| c.shapes[s].last().cloned()) {
            let c = h.h.war.corps.as_mut().unwrap();
            for u in 0..nu {
                c.pool[s][u] += 6.0 * sh.units[u];
                c.kit[s][u] += 6.0 * sh.units[u] * sh.norm_of(u);
            }
        }
        h.h.war.month_divs(&pack);
        let after = h.h.war.corps.as_ref().map_or(0, |c| c.divs.iter().filter(|d| d.side == s).count());
        (before, after)
    };
    let (b0, a0) = run(0.2, 0.3);
    let (b1, a1) = run(f32::INFINITY, 1.0);
    say(a0 <= b0 && a1 > b1, "армия не растёт, пока склады не кормят её: при пустых складах новых дивизий нет, при полных — есть", format!("пустые склады: было {b0}, стало {a0}; полные: было {b1}, стало {a1}"));
}
