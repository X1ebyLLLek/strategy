//! The folder of society: how the country is governed (its form, legitimacy, unrest), the forces
//! of its society (their power, their mood and why — each reason with its number), its laws (a
//! reform a click away: who gains, who loses, how long it takes, what it does), the movements, the
//! country's great task (and three offered), its faith (tenets, who believes what), the
//! government's hard decisions (the iron hand, a form from above), its story of two hundred years,
//! and the rival's order as intelligence sees it.

use crate::country::agenda::Task;
use crate::country::realm::{favour_cost, law_eff, reform_months, FROM_ABOVE};
use strategy::world::polity::{self, faith, BLOC, BLOCS, LAW, LAWS, MOVE, MOVES};
use super::*;
use super::super::charts;
use super::super::kit::{self, Stat};

/// The colour of a force of society.
fn bloc_col(b: usize) -> Color32 {
    const C: [[u8; 3]; BLOCS] = [[150, 92, 160], [212, 186, 92], [170, 72, 60], [96, 112, 132], [70, 150, 140], [86, 130, 196], [196, 84, 84], [120, 160, 72]];
    let c = C[b % BLOCS];
    Color32::from_rgb(c[0], c[1], c[2])
}

/// A tile of a law's level: a plate of the era, the words wrapped in it; raised in the accent when
/// it is the law, greyed when no reform leads to it now.
fn level_tile(ui: &mut egui::Ui, s: &Skin, text: &str, w: f32, on: bool, enabled: bool) -> egui::Response {
    let h = 38.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, h), if enabled { egui::Sense::click() } else { egui::Sense::hover() });
    let resp = if enabled && !on { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
    if let Some(ch) = chrome::of(ui.ctx()) {
        let mut v = chrome::Out::new();
        ch.plate(&mut v, r, s, false, on, enabled && resp.hovered(), resp.is_pointer_button_down_on());
        ui.painter().extend(v);
    }
    let ink = if on { s.on_accent() } else if enabled { s.ink } else { s.ink.gamma_multiply(0.75) };
    let font = egui::FontId::new(12.5, if on { egui::FontFamily::Name("bold".into()) } else { egui::FontFamily::Proportional });
    let gal = ui.painter().layout(text.to_string(), font, ink, w - 8.0);
    ui.painter().galley(egui::pos2(r.center().x - gal.size().x / 2.0, r.center().y - gal.size().y / 2.0), gal, ink);
    resp
}

/// A tone for a share where more is better.
fn pct_word(x: f32) -> String {
    format!("{:.0}%", x * 100.0)
}

fn tone_of(x: f32, good: f32, bad: f32) -> Color32 {
    if good > bad {
        if x >= good {
            kit::GOOD
        } else if x <= bad {
            kit::BAD
        } else {
            kit::MIXED
        }
    } else if x <= good {
        kit::GOOD
    } else if x >= bad {
        kit::BAD
    } else {
        kit::MIXED
    }
}

/// The folder of society.
/// The pages of the folder «Общество» (28th session).
pub const PAGES: [&str; 4] = ["Строй и силы", "Законы", "Движения и вера", "История"];

pub fn society(ui: &mut egui::Ui, s: &Skin, g: &mut Game, page: usize) {
    let me = g.me;
    let c = g.country(me);
    let p = g.polity.of[c].clone();
    let now = g.month();
    let shares = g.polity.shares(g.realm.regions.get(c).map_or(&[][..], |v| &v[..]));
    if page == 0 {
        // the form of government
        kit::framed(ui, Some(s.accent), false, |ui| {
            ui.horizontal(|ui| {
                let icons = chrome::icons(ui.ctx());
                let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), egui::Sense::hover());
                super::super::look::icon(&icons, p.regime.icon(), 40.0, s.accent).paint_at(ui, r);
                ui.vertical(|ui| {
                    kit::bold(ui, p.regime.name(), 18.0, s.ink);
                    let since = (now as i32 - p.since).max(0) / 12;
                    let house = p.house.clone().map_or(String::new(), |h| format!(" · правит {h}"));
                    kit::words(ui, format!("{} {}{house}", since, crate::words::years_word(since as u32)), 12.5, s.soft);
                });
            });
            kit::words(ui, p.regime.about(), 13.0, s.ink);
            if let Some(y) = p.regime.elected() {
                let next = g.realm.election[me].saturating_sub(now);
                kit::words(ui, format!("Выборы раз в {y:.0} лет; следующие — через {next} мес."), 12.5, s.soft);
            }
        });
        let ours = shares.iter().find(|x| x.0 == p.state_faith).map_or(0.0, |x| x.1);
        let items = vec![
            Stat::new("crown", "Законность", charts::pct(p.legit), tone_of(p.legit, 0.6, 0.35)),
            Stat::new("revolt", "Смута", charts::pct(p.unrest), tone_of(p.unrest, 0.25, 0.5)),
            Stat::new("graduate", "Грамотность", charts::pct(p.literacy), tone_of(p.literacy, 0.5, 0.2)),
            Stat::new("faith", "Набожность", charts::pct(p.piety), kit::PLAIN),
            Stat::new("people", "Веры государства", format!("{} людей", charts::pct(ours)), tone_of(ours, 0.85, 0.6)),
            Stat::new("anvil", "Заводской уклад", charts::pct(p.industry), kit::PLAIN),
        ];
        let resp = kit::stats(ui, &items, 3);
        let content = p.content();
        let cards: [Box<dyn Fn() -> Card>; 6] = [
            Box::new(move || {
                Card::new("Законность").icon("crown").row("сейчас", charts::pct(p.legit)).text("Насколько люди считают власть законной. Растёт от побед, сытости, долгой династии или честных выборов, от довольства сильных; падает от голода, поражений, нарушенных обещаний, твёрдой руки.").row("довольство сильных", format!("{:+.0}", content * 100.0)).note("ниже 45% при смуте выше 55% — власть может пасть")
            }),
            Box::new(move || Card::new("Смута").icon("revolt").row("сейчас", charts::pct(p.unrest)).text("Сколько в стране готовых бунтовать: недовольные силы общества (по их силе), голод, движения уравнителей, ревнителей и отделенцев. Твёрдая рука и законность держат её.")),
            Box::new(move || Card::new("Грамотность").icon("graduate").row("умеют читать", charts::pct(p.literacy)).text("Растёт от школ, богатства и заводов, от веры, ценящей знание. Грамотные — сила науки и заводов, но и вольнодумцы.")),
            Box::new(move || Card::new("Набожность").icon("faith").row("сейчас", charts::pct(p.piety)).text("Чем набожнее народ, тем сильнее его вера влияет на жизнь (её заветы — на науку, торговлю, войну) и тем сильнее духовенство.")),
            Box::new(move || Card::new("Вера").icon("people").row("людей своей веры", charts::pct(ours)).text("Доля людей, исповедующих государственную веру. Иноверцы — почва для отделенцев и раскола.")),
            Box::new(move || Card::new("Заводской уклад").icon("anvil").row("сейчас", charts::pct(p.industry)).text("Насколько хозяйство ушло от земли к заводам: от этого растут промышленники и рабочие, слабеют знать и крестьяне.")),
        ];
        for (r, cd) in resp.into_iter().zip(cards.iter()) {
            r.tip(|| cd());
        }
        // the forces of society
        kit::ribbon(ui, "Силы общества", Some("people"));
        kit::words(ui, "Сила — сколько весит сила в стране; настроение — как она судит правительство (наведите — почему). Строй решает, кого слышат: те, кто в правительстве, отмечены короной.", 12.5, s.soft);
        let coalition = p.coalition();
        let heard = p.heard();
        let mut order: Vec<usize> = (0..BLOCS).collect();
        order.sort_by(|&a, &b| p.clout[b].total_cmp(&p.clout[a]));
        let cost = favour_cost(g, me);
        let treasury = g.h.home(me).treasury;
        for b in order {
            let parts = g.mood_parts(me, b);
            let laws = g.law_parts(me, b);
            let in_gov = coalition.contains(&b);
            // (31st session: a card for each force — the stripe at the left in its colour, its name and the crown on top,
            // below them two captioned bars side by side (the weight and the mood), the favour's button at the right)
            let row = kit::framed(ui, Some(bloc_col(b)), false, |ui| {
                ui.horizontal(|ui| {
                    let icons = chrome::icons(ui.ctx());
                    let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), egui::Sense::hover());
                    super::super::look::icon(&icons, BLOC[b].icon, 30.0, bloc_col(b)).paint_at(ui, r);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let full = ui.available_width() - 34.0;
                        ui.horizontal(|ui| {
                            kit::bold(ui, BLOC[b].name, 14.5, s.ink);
                            if in_gov {
                                let (cr, _) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
                                super::super::look::icon(&icons, "crown", 15.0, s.accent).paint_at(ui, cr);
                                kit::words(ui, "в правительстве", 12.0, s.soft);
                            }
                        });
                        let half = ((full - 10.0) / 2.0).max(60.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                kit::words(ui, "сила", 12.0, s.soft);
                                let (pr, _) = ui.allocate_exact_size(vec2(half, 18.0), egui::Sense::hover());
                                let bar = egui::Rect::from_min_max(pr.min + vec2(0.0, 4.0), egui::pos2(pr.max.x - 34.0, pr.max.y - 4.0));
                                ui.painter().rect_filled(bar, 2.0, Color32::from_black_alpha(45));
                                ui.painter().rect_filled(egui::Rect::from_min_size(bar.min, vec2(bar.width() * (p.clout[b] / 0.4).min(1.0), bar.height())), 2.0, bloc_col(b));
                                ui.painter().text(egui::pos2(pr.max.x, pr.center().y), egui::Align2::RIGHT_CENTER, charts::pct(p.clout[b]), egui::FontId::new(12.5, egui::FontFamily::Name("bold".into())), s.ink);
                            });
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                kit::words(ui, "настроение", 12.0, s.soft);
                                charts::mood(ui, half, p.mood[b]);
                            });
                        });
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let fav = widgets::plate(ui, "+", 26.0, false, treasury >= cost);
                        if fav
                            .tip(|| Card::new(format!("Уступки: {}", BLOC[b].name)).icon("coins").row("из казны", charts::num(cost)).row("в казне", charts::num(treasury)).text("Подачки, должности, заказы, льготы: настроение этой силы +35 (уходит за полтора-два года); остальные немного ревнуют (−3).").note(if treasury >= cost { "щёлкните — уступить" } else { "в казне не хватает денег" }))
                            .clicked()
                        {
                            act(g, &[("favour", b.to_string())]);
                        }
                    });
                });
            });
            let mood_now = p.mood[b];
            row.response.tip(|| {
                let mut cd = Card::new(BLOC[b].name).icon(BLOC[b].icon).sub(format!("сила {} · настроение {:+.0}{}", charts::pct(p.clout[b]), mood_now * 100.0, if in_gov { " · в правительстве" } else { "" })).text(BLOC[b].about);
                cd = cd.row("слышна при этом строе", charts::pct(heard[b]));
                let wants: Vec<String> = (0..LAWS).filter(|&k| polity::WANTS[b][k].0 != 9).map(|k| format!("{}: {}", LAW[k].name.to_lowercase(), LAW[k].levels[polity::WANTS[b][k].0 as usize])).collect();
                if !wants.is_empty() {
                    cd = cd.head("Хотят").text(wants.join("; "));
                }
                cd = cd.head("Почему так настроены (к чему тянется настроение)");
                let mut all: Vec<(String, f32)> = parts.iter().cloned().chain(laws.iter().cloned()).collect();
                all.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
                for (what, x) in all.iter().take(12) {
                    cd = cd.tone(what.clone(), format!("{:+.0}", x * 100.0), if *x >= 0.0 { Tone::Good } else { Tone::Bad });
                }
                cd.note("настроение идёт к сумме причин за несколько месяцев; недовольные сильные — смута и угроза власти")
            });
        }
        // the chamber, where the country has one
        super::chamber::chamber(ui, s, g);
    }
    if page == 1 {
        // the laws
        kit::ribbon(ui, "Законы", Some("scroll"));
        if let Some(pend) = g.realm.pending[me].clone() {
            kit::framed(ui, Some(kit::MIXED), true, |ui| {
                kit::bold(ui, format!("Идёт реформа — {}: «{}»", LAW[pend.law].name.to_lowercase(), LAW[pend.law].levels[pend.to as usize]), 14.0, s.ink);
                let r = kit::gauge(ui, ui.available_width(), 12.0, pend.done as f32 / pend.months.max(1) as f32, kit::MIXED);
                r.tip(|| Card::new("Реформа").row("прошло", format!("{} из {} мес.", pend.done, pend.months)));
                if kit::act_button(ui, "Остановить реформу", "stop", true, false).tip(|| Card::new("Остановить реформу").text("Сторонники разочарованы: законность −4.")).clicked() {
                    act(g, &[("reform_stop", "1".into())]);
                }
            });
        }
        kit::words(ui, "Щёлкните по соседней ступени — начнётся реформа (на одну ступень за раз). Наведите — кто выиграет, кто проиграет, что изменится.", 12.5, s.soft);
        for law in 0..LAWS {
            let lv = p.laws[law];
            // (the law's name on its own line, its three levels under it across the folder)
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                let icons = chrome::icons(ui.ctx());
                let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), egui::Sense::hover());
                super::super::look::icon(&icons, LAW[law].icon, 18.0, s.soft).paint_at(ui, r);
                kit::bold(ui, LAW[law].name, 13.5, s.ink);
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let w = ((ui.available_width() - 2.0 * 4.0) / 3.0).floor();
                for k in 0..3u8 {
                    let on = k == lv;
                    let next = (k as i32 - lv as i32).abs() == 1;
                    let allowed = next.then(|| g.reform_allowed(me, law, k));
                    let b = level_tile(ui, s, LAW[law].levels[k as usize], w, on, on || allowed.as_ref().is_some_and(|a| a.is_ok()));
                    let b = b.tip(|| {
                        let mut cd = Card::new(format!("{}: {}", LAW[law].name, LAW[law].levels[k as usize])).icon(LAW[law].icon).text(LAW[law].about[k as usize]);
                        if !on {
                            cd = cd.head("Кто за, кто против");
                            for bb in 0..BLOCS {
                                let d = polity::law_fit(bb, law, k) - polity::law_fit(bb, law, lv);
                                if d.abs() > 0.01 {
                                    cd = cd.tone(BLOC[bb].name, format!("{:+.0}", d * 60.0), if d > 0.0 { Tone::Good } else { Tone::Bad });
                                }
                            }
                            cd = cd.head("Что изменится");
                            let new = law_eff(law, k, p.piety);
                            for (what, how, good) in new.lines(&g.pack) {
                                cd = cd.tone(what, how, if good { Tone::Good } else { Tone::Bad });
                            }
                            // (labour and land work through the market: say how)
                            if law == polity::LABOUR {
                                use crate::country::realm::{MOBILITY, SERF_RENT};
                                cd = cd.row("уходят из деревни на заводы", format!("×{:.2} → ×{:.2}", MOBILITY[lv as usize], MOBILITY[k as usize]));
                                cd = cd.row("рента помещика", format!("×{:.2} → ×{:.2}", SERF_RENT[lv as usize], SERF_RENT[k as usize]));
                            }
                            if law == polity::LAND {
                                use crate::country::realm::RENT;
                                cd = cd.row("рента помещика с урожая", format!("×{:.2} → ×{:.2}", RENT[lv as usize], RENT[k as usize]));
                            }
                            if law == polity::SCHOOLS {
                                cd = cd.row("школы учат", format!("×{:.1} → ×{:.1} (если хватает денег)", [0.7, 1.0, 1.5][lv as usize], [0.7, 1.0, 1.5][k as usize]));
                            }
                            cd = cd.row("займёт", format!("{} мес.", reform_months(law, k)));
                            if let Some(v) = g.reform_vote(me, law, k) {
                                cd = cd.tone("собрание за", pct_word(v), if v >= 0.5 { Tone::Good } else { Tone::Bad });
                                if v < 0.5 {
                                    cd = cd.row("без большинства", "указом: в 1,5 раза дольше, законность −9");
                                }
                            }
                        }
                        match &allowed {
                            Some(Err(why)) => cd.note(format!("нельзя: {why}")),
                            Some(Ok(())) => cd.note("щёлкните — начать реформу (законность −3)"),
                            None if on => cd.note("действует сейчас"),
                            None => cd.note("реформа идёт на одну ступень за раз"),
                        }
                    });
                    if !on && allowed.as_ref().is_some_and(|a| a.is_ok()) && b.clicked() {
                        act(g, &[("reform", format!("{law}:{k}"))]);
                    }
                }
            });
        }
    }
    if page == 2 {
        // the movements
        kit::ribbon(ui, "Движения", Some("revolt"));
        let mut mv: Vec<usize> = (0..MOVES).collect();
        mv.sort_by(|&a, &b| p.moves[b].total_cmp(&p.moves[a]));
        let items: Vec<(String, f32, Color32)> = mv.iter().map(|&m| (MOVE[m].name.to_string(), p.moves[m], if p.moves[m] > 0.5 { kit::BAD } else if p.moves[m] > 0.3 { kit::MIXED } else { kit::PLAIN })).collect();
        let resps = charts::bars(ui, &items, &|x| charts::pct(x));
        for (r, &m) in resps.into_iter().zip(mv.iter()) {
            r.tip(|| {
                let aims: Vec<String> = MOVE[m].aims.iter().map(|&(l, v)| format!("{}: {}", LAW[l].name.to_lowercase(), LAW[l].levels[v as usize])).collect();
                let mut cd = Card::new(MOVE[m].name).icon(MOVE[m].icon).row("поддержка", charts::pct(p.moves[m])).text(MOVE[m].about);
                if !aims.is_empty() {
                    cd = cd.head("Добиваются").text(aims.join("; "));
                }
                cd.note("сильное движение требует своего; без ответа — смута, при слабой власти — переворот")
            });
        }
        // the great task
        kit::ribbon(ui, "Великая задача страны", Some("flag"));
        let top_y = ui.cursor().min.y;
        task_block(ui, s, g);
        let r = egui::Rect::from_min_max(egui::pos2(ui.min_rect().min.x, top_y), egui::pos2(ui.min_rect().max.x, ui.cursor().min.y));
        super::super::reports::glow(ui, r, "task");
        // the faith
        kit::ribbon(ui, "Вера", Some("faith"));
        faith_block(ui, s, g, &shares);
        // decisions
        kit::ribbon(ui, "Решения власти", Some("pen"));
        let cr = g.realm.crackdown[me];
        let b = kit::act_button(ui, if cr > 0 { "Твёрдая рука действует" } else { "Твёрдая рука (на год)" }, "fist", cr == 0, cr > 0);
        if b.tip(|| Card::new("Твёрдая рука").icon("fist").text("Год жёсткого порядка: смута быстро спадает, вольнодумцы, уравнители и отделенцы слабеют. Интеллигенция −30, рабочие −20, купцы −10, офицеры +10; законность −5; наука −5%.").note(if cr > 0 { format!("ещё {cr} мес.") } else { "щёлкните — ввести".into() })).clicked() && cr == 0 {
            act(g, &[("crackdown", "1".into())]);
        }
        kit::words(ui, "Сменить строй сверху (раз в пять лет; законность упадёт до 40%):", 12.5, s.soft);
        ui.horizontal_wrapped(|ui| {
            for (k, (to, word)) in FROM_ABOVE.iter().enumerate() {
                let ok = g.from_above_allowed(me, k);
                let b = kit::act_button(ui, word, to.icon(), ok.is_ok(), false);
                let b = b.tip(|| {
                    let mut cd = Card::new(*word).icon(to.icon()).sub(to.name()).text(to.about());
                    let v = to.voice();
                    cd = cd.head("Кого будут слышать");
                    for bb in 0..BLOCS {
                        cd = cd.row(BLOC[bb].name, format!("×{:.1}", v[bb]));
                    }
                    match &ok {
                        Ok(()) => cd.note("щёлкните — перемена; силы, которых станут слышать меньше, будут злы"),
                        Err(why) => cd.note(format!("нельзя: {why}")),
                    }
                });
                if ok.is_ok() && b.clicked() {
                    act(g, &[("regime", k.to_string())]);
                }
            }
        });
    }
    if page == 3 {
        // the story
        kit::ribbon(ui, "Двести лет страны", Some("scroll"));
        if p.story.is_empty() {
            kit::words(ui, "Летопись молчит: ни переворотов, ни реформ, ни расколов.", 13.0, s.soft);
        }
        for (m, line) in p.story.iter().rev().take(16) {
            let ago = (now as i32 - m).max(0) / 12;
            ui.horizontal_wrapped(|ui| {
                kit::bold(ui, if ago == 0 { "в этом году".to_string() } else { format!("{ago} {} назад", crate::words::years_word(ago as u32)) }, 12.5, s.soft);
                kit::words(ui, line.clone(), 13.0, s.ink);
            });
        }
        let (rev, coups, refs) = (p.revolutions, p.coups, p.reforms);
        kit::words(ui, format!("За двести лет: революций {rev}, переворотов {coups}, реформ {refs}."), 12.5, s.soft);
        // the rival
        kit::ribbon(ui, "Общество соседа", Some("binoculars"));
        let o = g.country(1 - me);
        let q = g.polity.of[o].clone();
        let fa = g.polity.faiths.get(q.state_faith).map_or(String::new(), |f| f.name.clone());
        let apart = g.polity.distance(p.state_faith, q.state_faith);
        kit::words(ui, format!("{} — {}; законность {}, смута {}.", g.h.home(1 - me).name, q.regime.name().to_lowercase(), charts::pct(q.legit), charts::pct(q.unrest)), 13.5, s.ink);
        kit::words(ui, format!("Вера: «{fa}» — {}.", if apart == 0.0 { "та же, что у нас" } else if apart < 0.9 { "родственная нашей (другой толк)" } else { "чужая" }), 13.0, s.ink);
        let (tm, ts) = q.top_move();
        let coal: Vec<&str> = q.coalition().iter().map(|&b| BLOC[b].name).collect();
        kit::words(ui, format!("У власти: {}. Сильнейшее движение — {} ({}).", coal.join(", ").to_lowercase(), MOVE[tm].name.to_lowercase(), charts::pct(ts)), 13.0, s.soft);
        let _ = faith::TENETS;
    }
}

/// The great task: the one taken (its measure, how far it has come, the months left, the reward),
/// or three offered.
fn task_block(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let now = g.month();
    if let Some(t) = g.agenda.now[me].clone() {
        let title = g.task_title(&t);
        let words = g.task_words(me, &t);
        let prog = g.task_progress(me, &t);
        let left = t.until.saturating_sub(now);
        kit::framed(ui, Some(kit::GOOD), true, |ui| {
            ui.horizontal(|ui| {
                let icons = chrome::icons(ui.ctx());
                let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), egui::Sense::hover());
                super::super::look::icon(&icons, t.goal.icon(), 30.0, s.accent).paint_at(ui, r);
                ui.vertical(|ui| {
                    kit::bold(ui, title.clone(), 16.0, s.ink);
                    kit::words(ui, format!("просили: {} · осталось {left} мес.", t.who.name().to_lowercase()), 12.5, s.soft);
                });
            });
            kit::words(ui, words.clone(), 13.0, s.ink);
            let r = kit::gauge(ui, ui.available_width(), 14.0, prog, if prog >= 1.0 { kit::GOOD } else { kit::MIXED });
            r.tip(|| Card::new("Как идёт задача").row("сделано", charts::pct(prog)).row("до срока", format!("{left} мес.")));
            kit::words(ui, format!("Награда: законность +10, {} довольны; {}.", t.who.name().to_lowercase(), t.goal.reward_words()), 12.5, s.soft);
            kit::words(ui, format!("Провал: законность −8, {} разочарованы.", t.who.name().to_lowercase()), 12.5, s.soft);
            if kit::act_button(ui, "Отказаться от задачи", "stop", true, false).tip(|| Card::new("Отказаться").text("Законность −4; через месяц силы общества предложат новые задачи.")).clicked() {
                act(g, &[("task_drop", "1".into())]);
            }
        });
        return;
    }
    let offers: Vec<Task> = g.agenda.offers[me].clone();
    if offers.is_empty() {
        kit::words(ui, "Силы общества пока ничего не предлагают.", 13.0, s.soft);
        return;
    }
    kit::words(ui, "Силы общества и движения предлагают, чем стране заняться. Задача — одна за раз; выполнить в срок — награда, провалить — разочарование тех, кто просил.", 12.5, s.soft);
    for (k, t) in offers.iter().enumerate() {
        let title = g.task_title(t);
        let words = g.task_words(me, t);
        kit::framed(ui, Some(s.accent), false, |ui| {
            ui.horizontal(|ui| {
                let icons = chrome::icons(ui.ctx());
                let (r, _) = ui.allocate_exact_size(vec2(28.0, 28.0), egui::Sense::hover());
                super::super::look::icon(&icons, t.goal.icon(), 26.0, s.accent).paint_at(ui, r);
                ui.vertical(|ui| {
                    kit::bold(ui, title.clone(), 14.5, s.ink);
                    kit::words(ui, format!("просят: {} · срок {} мес.", t.who.name().to_lowercase(), t.goal.months()), 12.5, s.soft);
                });
            });
            kit::words(ui, words.clone(), 12.5, s.ink);
            kit::words(ui, format!("Награда: {}.", t.goal.reward_words()), 12.5, s.soft);
            if kit::act_button(ui, "Взять задачу", "flag", true, false).clicked() {
                act(g, &[("task", k.to_string())]);
            }
        });
    }
}

/// The faith: the state's (its tenets, its founder, how old), who believes what in the country.
fn faith_block(ui: &mut egui::Ui, s: &Skin, g: &Game, shares: &[(usize, f32)]) {
    let me = g.me;
    let c = g.country(me);
    let p = &g.polity.of[c];
    let now = g.month() as i32;
    let Some(fa) = g.polity.faiths.get(p.state_faith) else {
        kit::words(ui, "Веры нет.", 13.0, s.soft);
        return;
    };
    let age = ((now - fa.founded).max(0) / 12) as u32;
    kit::framed(ui, Some(Color32::from_rgb(fa.hue[0], fa.hue[1], fa.hue[2])), false, |ui| {
        kit::bold(ui, fa.name.clone(), 16.0, s.ink);
        let from = fa.parent.and_then(|x| g.polity.faiths.get(x)).map_or(String::new(), |x| format!(", отделилась от «{}»", x.name));
        kit::words(ui, format!("{} {} назад; основатель — {}{from}. Закон: {}.", age, crate::words::years_word(age), fa.founder, p.law_word(polity::CHURCH)), 12.5, s.soft);
        let creed = fa.creed();
        if !creed.is_empty() {
            kit::words(ui, format!("Учит: {}.", creed.join("; ")), 13.0, s.ink);
        }
        let items: Vec<(String, f32, Color32)> = (0..faith::TENETS).map(|t| (faith::TENET_WORDS[t].0.to_string(), fa.tenets[t], if fa.tenets[t] >= 0.0 { kit::GOOD } else { kit::BAD })).collect();
        let resps = charts::bars(ui, &items, &|x| format!("{:+.0}", x * 100.0));
        for (r, t) in resps.into_iter().zip(0..faith::TENETS) {
            r.tip(|| Card::new(faith::TENET_WORDS[t].0).row("сила завета", format!("{:+.0}", fa.tenets[t] * 100.0)).text(format!("+: {}", faith::TENET_WORDS[t].1)).text(format!("−: {}", faith::TENET_WORDS[t].2)).note("заветы веры действуют тем сильнее, чем набожнее народ"));
        }
    });
    let parts: Vec<(String, f32, Color32)> = shares.iter().map(|&(f, x)| (g.polity.faiths.get(f).map_or(String::new(), |q| q.name.clone()), x, g.polity.faiths.get(f).map_or(Color32::GRAY, |q| Color32::from_rgb(q.hue[0], q.hue[1], q.hue[2])))).collect();
    let r = charts::shares(ui, &parts, 16.0);
    r.tip(|| {
        let mut cd = Card::new("Кто во что верит").icon("faith");
        for (name, x, _) in &parts {
            cd = cd.row(name.clone(), charts::pct(*x));
        }
        cd.note("по областям страны; веры переходят от соседа к соседу, государственная церковь обращает своих иноверцев")
    });
}
