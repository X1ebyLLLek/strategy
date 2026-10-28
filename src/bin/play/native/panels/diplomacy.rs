//! Diplomacy, in the folder of politics: the wars of the world; every country — what its
//! government thinks of us (a number, and each reason with its number), the pacts it keeps, its
//! wars; for the one picked, what we may propose and how it would answer (said before the click:
//! the reasons are on the card), the pacts to break, an envoy to send; our share in a war of the
//! world, and the way out of it.

use crate::world::relations::{code_of, ENVOY_COST, ENVOY_PAUSE, HELP};
use strategy::world::diplomacy::Kind;
use super::*;
use super::super::charts;
use super::super::kit::{self, Stat};

/// The icon of a kind of pact.
pub fn pact_icon(k: Kind) -> &'static str {
    match k {
        Kind::Alliance => "shield",
        Kind::Peace => "scroll",
        Kind::Trade => "backforth",
        Kind::Truce => "flag",
        Kind::Guarantee => "castle",
        Kind::Tribute => "crate",
        Kind::Marriage => "crown",
    }
}

/// The tone of an opinion.
fn tone(x: f32) -> Color32 {
    if x >= 0.3 {
        kit::GOOD
    } else if x <= -0.3 {
        kit::BAD
    } else {
        kit::MIXED
    }
}

/// An opinion in words.
pub fn stand_word(x: f32) -> &'static str {
    match x {
        x if x >= 0.6 => "дружественна",
        x if x >= 0.3 => "расположена",
        x if x > -0.3 => "сдержанна",
        x if x > -0.6 => "неприязненна",
        _ => "враждебна",
    }
}

/// The section of diplomacy.
pub fn diplomacy(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me_c = g.country(g.me);
    let n = g.countries.len().min(g.diplo.n);
    let live: Vec<usize> = (0..n).filter(|&c| c != me_c && g.rel.adj.get(c).is_some_and(|a| !a.is_empty())).collect();
    head(ui, s, "Дипломатия");
    let allies = g.diplo.allies(me_c).len();
    let pacts = g.diplo.pacts_of(me_c).len();
    let friends = live.iter().filter(|&&c| g.diplo.of(c, me_c) >= 0.3).count();
    let foes = live.iter().filter(|&&c| g.diplo.of(c, me_c) <= -0.3).count();
    let got = g.rel.got.get(me_c).copied().unwrap_or(0.0) - g.rel.paid.get(me_c).copied().unwrap_or(0.0);
    let items = [
        Stat::new("shield", "Союзников", allies.to_string(), if allies > 0 { kit::GOOD } else { kit::MIXED }),
        Stat::new("scroll", "Договоров", pacts.to_string(), kit::MIXED),
        Stat::new("people", "Друзей", friends.to_string(), kit::GOOD),
        Stat::new("clash", "Недругов", foes.to_string(), if foes > 2 { kit::BAD } else { kit::MIXED }),
    ];
    let rs = kit::stats(ui, &items, 2);
    rs[2].clone().tip(|| Card::new("Друзья").icon("people").text("Страны, чьё правительство думает о нас не хуже +30: с ними возможен союз."));
    rs[3].clone().tip(|| Card::new("Недруги").icon("clash").text("Страны, чьё правительство думает о нас хуже −30: они охотнее нападут и не придут на помощь."));
    if got.abs() > 1e-4 {
        let line = if got > 0.0 { format!("Договоры и войны дают казне {} в месяц (дань, помощь союзников).", charts::num(got)) } else { format!("Договоры и войны стоят казне {} в месяц (дань, помощь союзникам).", charts::num(-got)) };
        words(ui, line, 13.5, super::super::hud::lerp(if got > 0.0 { kit::GOOD } else { kit::BAD }, s.ink, 0.45));
    }
    // our share in a war of the world
    if let Some(w) = g.diplo.war_of(me_c) {
        let war = g.diplo.wars[w].clone();
        let side = war.side_of(me_c).unwrap_or(0);
        let head = war.sides[side][0];
        if head != me_c && !(g.h.at_war && war.sides[1 - side].contains(&g.country(1 - g.me))) {
            let cost = HELP * g.h.home(g.me).state_budget();
            kit::framed(ui, Some(kit::BAD), false, |ui| {
                kit::bold(ui, format!("Мы воюем на стороне {} против {}", g.countries[head].0, g.countries[war.sides[1 - side][0]].0), 14.5, s.ink);
                words(ui, format!("{} · каждый месяц союзнику уходит {} из казны", war.cause, charts::num(cost)), 13.0, s.soft);
                if kit::act_button(ui, "Выйти из войны", "stop", true, false).tip(|| Card::new("Сепаратный мир").icon("flag").text("Мы заключаем перемирие с врагами союзника. Союз с ним будет расторгнут, и о нашем вероломстве будут помнить.")).clicked() {
                    act(g, &[("leave", "1".into())]);
                }
            });
        }
    }
    // the wars of the world
    if !g.diplo.wars.is_empty() {
        kit::words(ui, "Войны в мире", 14.0, s.accent);
        let now = g.month() as i32;
        for war in g.diplo.wars.clone() {
            let side = |k: usize| war.sides[k].iter().map(|&c| g.countries[c].0.as_str()).collect::<Vec<_>>().join(" + ");
            let lead = (war.score + 1.0) / 2.0;
            let r = ui.horizontal(|ui| {
                ui.set_width(ui.available_width());
                words(ui, format!("{} — {}", side(0), side(1)), 13.0, s.ink);
            });
            let bar = charts::shares(ui, &[(side(0), lead.max(0.02), Color32::from_rgb(176, 96, 60)), (side(1), (1.0 - lead).max(0.02), Color32::from_rgb(84, 112, 150))], 8.0);
            let months = (now - war.since).max(0) / 12 * 12 + (now - war.since).max(0) % 12;
            let cause = war.cause.clone();
            let card = move || Card::new("Война в мире").icon("clash").row("причина", cause.clone()).row("идёт", format!("{months} мес.")).row("перевес нападающих", format!("{:+.0}", war.score * 100.0)).note("её ведут без фронта: решают сила хозяйств и удача; конец — перемирие, у сдавшихся — дань");
            r.response.tip(card.clone());
            bar.tip(card);
        }
        ui.add_space(4.0);
    }
    // every country, by what it thinks of us
    let mut order = live.clone();
    order.sort_by(|&a, &b| g.diplo.of(b, me_c).total_cmp(&g.diplo.of(a, me_c)).then(a.cmp(&b)));
    let pick_id = egui::Id::new("diplo-pick");
    // (for pictures: a country picked by `DIPLO_PICK=c`)
    let shot_pick = std::env::var("DIPLO_PICK").ok().and_then(|v| v.parse::<usize>().ok());
    let picked: Option<usize> = ui.ctx().data(|d| d.get_temp(pick_id)).or(shot_pick).filter(|c: &usize| live.contains(c));
    kit::words(ui, "Страны мира: что их правительства думают о нас", 14.0, s.accent);
    let ch = chrome::of(ui.ctx());
    let icons = chrome::icons(ui.ctx());
    let w = ui.available_width();
    let mut click = None;
    for &c in &order {
        let op = g.diplo.of(c, me_c);
        let (row, resp) = ui.allocate_exact_size(vec2(w, 40.0), egui::Sense::click());
        if !ui.is_rect_visible(row) {
            continue;
        }
        let on = picked == Some(c);
        let mut v = chrome::Out::new();
        if let Some(ch) = &ch {
            if on {
                ch.plate(&mut v, row, s, false, false, true, false);
            } else if resp.hovered() {
                v.push(egui::Shape::rect_filled(row, 2.0, s.accent.gamma_multiply(0.12)));
            }
            v.push(egui::Shape::line_segment([pos2(row.min.x, row.max.y - 0.5), pos2(row.max.x, row.max.y - 0.5)], egui::Stroke::new(0.8, s.soft.gamma_multiply(0.35))));
        }
        ui.painter().extend(v);
        let fr = egui::Rect::from_min_size(row.min + vec2(6.0, 7.0), vec2(39.0, 26.0));
        if let (Some(f), Some(ch)) = (super::super::flags::get(ui.ctx(), c), &ch) {
            super::super::flags::paint(ui.ctx(), ui.painter(), fr, &f, ch.trim, Some(&icons));
        }
        let x = fr.max.x + 8.0;
        let regime = g.polity.of.get(c).map_or("", |p| p.regime.name());
        ui.painter().text(pos2(x, row.min.y + 4.0), egui::Align2::LEFT_TOP, g.countries[c].0.to_uppercase(), egui::FontId::new(13.5, egui::FontFamily::Name("head".into())), s.ink);
        let war_word = if g.diplo.war_of(c).is_some() { " · воюет" } else { "" };
        ui.painter().text(pos2(x, row.min.y + 21.0), egui::Align2::LEFT_TOP, format!("{}{war_word}", regime.to_lowercase()), egui::FontId::proportional(12.0), s.soft);
        // the pacts with us, as icons; the opinion as a number on its tone
        let mut px = row.max.x - 60.0;
        for p in g.diplo.pacts_of(c).iter().filter(|p| p.has(me_c)) {
            px -= 20.0;
            super::super::look::icon(&icons, pact_icon(p.kind), 17.0, s.accent).paint_at(ui, egui::Rect::from_min_size(pos2(px, row.min.y + 11.0), vec2(17.0, 17.0)));
        }
        let badge = egui::Rect::from_min_size(pos2(row.max.x - 54.0, row.min.y + 9.0), vec2(48.0, 22.0));
        // (darkened under the white figure: it reads on every tone)
        ui.painter().rect_filled(badge, 3.0, super::super::hud::lerp(tone(op), Color32::BLACK, 0.3));
        ui.painter().text(badge.center(), egui::Align2::CENTER_CENTER, format!("{:+.0}", op * 100.0), egui::FontId::new(13.5, egui::FontFamily::Name("bold".into())), Color32::WHITE);
        let name = g.countries[c].0.clone();
        let reasons = g.reasons_words(c, me_c);
        let card = || {
            let mut cd = Card::new(name.clone()).sub(format!("{} к нам: {:+.0}", stand_word(op), op * 100.0));
            for (why, x) in reasons.iter().take(6) {
                cd = cd.row(*why, format!("{:+.0}", x * 100.0));
            }
            cd.note("щелчок — подробно и что можно предложить")
        };
        if resp.tip(card).clicked() {
            click = Some(c);
        }
    }
    if let Some(c) = click {
        let next = if picked == Some(c) { None } else { Some(c) };
        ui.ctx().data_mut(|d| match next {
            Some(c) => d.insert_temp(pick_id, c),
            None => d.remove::<usize>(pick_id),
        });
    }
    if let Some(c) = picked {
        ui.add_space(6.0);
        country(ui, s, g, c);
    }
}

/// The country picked: its opinion of us and why, its pacts, what we may do.
fn country(ui: &mut egui::Ui, s: &Skin, g: &mut Game, c: usize) {
    let me_c = g.country(g.me);
    let op = g.diplo.of(c, me_c);
    let name = g.countries[c].0.clone();
    kit::framed(ui, Some(tone(op)), false, |ui| {
        kit::bold(ui, name.to_uppercase(), 16.0, s.ink);
        let p = g.polity.of.get(c).cloned();
        if let Some(p) = &p {
            let faith = g.polity.faiths.get(p.state_faith).map_or(String::new(), |f| f.name.clone());
            let ours = g.polity.of.get(me_c).map_or(usize::MAX, |x| x.state_faith);
            let same = if ours == p.state_faith { "наша вера" } else if g.polity.distance(ours, p.state_faith) < 0.75 { "вера одного корня с нашей" } else { "чужая вера" };
            words(ui, format!("{} · «{faith}» ({same})", p.regime.name()), 13.0, s.soft);
        }
        let gdp = |x: usize| g.h.world.nations.get(x).map_or(1e-6, |n| n.gdp.max(1e-6));
        words(ui, format!("Хозяйство к нашему: ×{:.2}", gdp(c) / gdp(me_c)), 13.0, s.soft);
        ui.add_space(3.0);
        words(ui, format!("Их правительство {} к нам: {:+.0}", stand_word(op), op * 100.0), 14.0, s.ink);
        charts::mood(ui, ui.available_width(), op);
        // why: each reason with its number
        let reasons = g.reasons_words(c, me_c);
        if !reasons.is_empty() {
            let items: Vec<(String, f32, Color32)> = reasons.iter().map(|(w, x)| (w.to_string(), *x, if *x >= 0.0 { kit::GOOD } else { kit::BAD })).collect();
            charts::bars(ui, &items, &|x| format!("{:+.0}", x * 100.0));
            words(ui, "Мнение каждый год сдвигается к сумме причин (на треть пути).", 12.0, s.soft);
        }
        // its pacts and wars
        let mine: Vec<(Kind, i32, i32)> = g.diplo.pacts_of(c).iter().filter(|p| p.has(me_c)).map(|p| (p.kind, p.since, p.until)).collect();
        let others: Vec<String> = g.diplo.pacts_of(c).iter().filter(|p| !p.has(me_c) && p.kind != Kind::Truce).map(|p| format!("{} — {}", p.kind.name().to_lowercase(), g.countries[p.other(c)].0)).collect();
        let now = g.month() as i32;
        for (k, since, until) in &mine {
            let left = ((until - now).max(0) / 12) as u32;
            let age = ((now - since).max(0) / 12) as u32;
            let kept = if age >= 1 { format!(" уже {age} {}", crate::words::years_word(age)) } else { String::new() };
            ui.horizontal(|ui| {
                let icons = chrome::icons(ui.ctx());
                let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::hover());
                super::super::look::icon(&icons, pact_icon(*k), 18.0, s.accent).paint_at(ui, r);
                words(ui, format!("{} с нами{kept} — ещё {left} {}", k.name(), crate::words::years_word(left)), 13.5, s.ink).tip(|| Card::new(k.name()).icon(pact_icon(*k)).text(k.about()));
            });
            if matches!(k, Kind::Alliance | Kind::Peace | Kind::Trade | Kind::Guarantee | Kind::Tribute) {
                let what = if *k == Kind::Tribute { "Перестать платить / брать дань" } else { "Расторгнуть" };
                if kit::act_button(ui, what, "broken", true, false).tip(|| Card::new(format!("Расторгнуть: {}", k.name().to_lowercase())).icon("broken").text(match k {
                    Kind::Alliance | Kind::Peace | Kind::Guarantee => "Договор будет расторгнут. Его правительство запомнит это надолго: мнение о нас упадёт.",
                    Kind::Tribute => "Дань прекратится. Тот, кто её брал, не простит — может начать войну.",
                    _ => "Пошлины на товары между нами вернутся.",
                })).clicked() {
                    act(g, &[("unpact", format!("{}:{c}", code_of(*k)))]);
                }
            }
        }
        if !others.is_empty() {
            words(ui, format!("Её договоры: {}", others.join("; ")), 12.5, s.soft);
        }
        if let Some(w) = g.diplo.war_of(c) {
            let war = &g.diplo.wars[w];
            let side = war.side_of(c).unwrap_or(0);
            let foes: Vec<&str> = war.sides[1 - side].iter().map(|&x| g.countries[x].0.as_str()).collect();
            words(ui, format!("Воюет против: {} ({})", foes.join(", "), war.cause), 13.0, kit::BAD);
        }
        // what we may propose: each with its answer, told before the click
        ui.add_space(4.0);
        words(ui, "Предложить", 14.0, s.accent);
        for k in [Kind::Alliance, Kind::Peace, Kind::Trade, Kind::Guarantee, Kind::Marriage] {
            if mine.iter().any(|m| m.0 == k) {
                continue;
            }
            let ans = g.answer_of(k, c);
            let ok = ans.is_ok();
            let why = match &ans {
                Ok(x) | Err(x) => x.clone(),
            };
            let label = if k == Kind::Guarantee { "Гарантировать независимость".to_string() } else { k.name().to_string() };
            let resp = kit::act_button(ui, &label, pact_icon(k), true, false);
            let card = || Card::new(k.name()).icon(pact_icon(k)).text(k.about()).sub(if ok { "согласятся" } else { "откажут" }).note(why.clone());
            if resp.tip(card).clicked() {
                act(g, &[("pact", format!("{}:{c}", code_of(k)))]);
            }
        }
        // an envoy with gifts
        let cost = ENVOY_COST * g.h.home(g.me).state_budget();
        let last = g.rel.envoys.iter().find(|e| e.0 == c).map(|e| e.1);
        let wait = last.map_or(0, |m| (m + ENVOY_PAUSE).saturating_sub(g.month()));
        let can = wait == 0 && g.h.home(g.me).treasury >= cost;
        let warm = g.diplo.warmth.get(c * g.diplo.n + me_c).copied().unwrap_or(0.0);
        if kit::act_button(ui, &format!("Посольство с дарами — {}", charts::num(cost)), "quill", can, false)
            .tip(|| {
                Card::new("Посольство").icon("quill").text("Дары и переговоры: их правительство станет к нам теплее (+12, не больше +40 всего; тепло понемногу остывает).").row("стоит", charts::num(cost)).row("тепло сейчас", format!("{:+.0}", warm * 100.0)).note(if wait > 0 {
                    format!("следующее посольство — через {wait} мес.")
                } else if !can {
                    "в казне не хватает денег".to_string()
                } else {
                    "мнение подрастёт к концу года".to_string()
                })
            })
            .clicked()
            && can
        {
            act(g, &[("envoy", c.to_string())]);
        }
    });
}
