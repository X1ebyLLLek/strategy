//! The folder «Флот»: the fleets against each other, the navy's share of the army's money, the
//! squadrons with their missions and zones, the shipyards and what they build, the trade by sea and
//! what threatens it, the battles at sea.

use eframe::egui::{self, vec2, Color32, RichText};

use strategy::military::navy::{self, MISSIONS};

use super::super::chrome::{Card, Tip, Tone};
use super::super::kit::{self, Stat};
use super::super::look::Skin;
use super::super::terms::say;
use super::super::widgets;
use super::{act, head};
use crate::Game;

fn money(x: f32) -> String {
    super::super::charts::num(x)
}

/// The zones the player's navy has to do with: its coast, the enemy's, the lanes of both sides'
/// trade (the busiest first).
fn zones_of_note(g: &Game) -> Vec<usize> {
    let me = g.me;
    let mut v: Vec<usize> = g.navy.home[me].clone();
    for &z in &g.navy.home[1 - me] {
        if !v.contains(&z) {
            v.push(z);
        }
    }
    for s in [me, 1 - me] {
        let t = g.trade_by_zone(s);
        let mut lanes: Vec<usize> = (0..t.len()).filter(|&z| t[z] > 0.0).collect();
        lanes.sort_by(|&a, &b| t[b].total_cmp(&t[a]));
        for z in lanes.into_iter().take(6) {
            if !v.contains(&z) {
                v.push(z);
            }
        }
    }
    v
}

pub fn navy(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    if pack.ships.is_empty() {
        say(ui, s, "В эту эпоху у стран нет флотов.");
        return;
    }
    let (ours, ours_w) = g.navy_power(me);
    let (theirs, theirs_w) = g.navy_power(1 - me);
    head(ui, s, "Флот и море");
    if g.navy.coast[me].is_empty() {
        say(ui, s, "У страны нет выхода к морю: флот ей не нужен, а торговля идёт по суше.");
    }
    let ratio = ours_w / theirs_w.max(1e-3);
    let sea = g.sea_dependence(me);
    let items = vec![
        Stat::new("anchor", "Кораблей", format!("{ours} · у врага {theirs}"), kit::PLAIN),
        Stat::new("balance", "Сила на море", if theirs_w < 0.5 { format!("{ours_w:.0} · у врага флота нет") } else { format!("{:.0} к {:.0}", ours_w, theirs_w) }, if ratio >= 1.2 { kit::GOOD } else if ratio >= 0.8 { kit::MIXED } else { kit::BAD }),
        Stat::new("route", "Торговля морем", format!("{:.0}% торговли", sea * 100.0), kit::PLAIN),
        Stat::new("broken", "Топят грузов", format!("{} в мес", money(g.navy.sunk_trade[me])), if g.navy.sunk_trade[me] > 0.0 { kit::BAD } else { kit::PLAIN }),
        Stat::new("jerrycan", "Снабжение флота", format!("{:.0}%", g.navy.fed[me] * 100.0), if g.navy.fed[me] > 0.9 { kit::GOOD } else { kit::BAD }),
        Stat::new("gears", "Стапелей", format!("{} · строится {}", g.navy.slipways[me], g.navy.yards[me].len()), kit::PLAIN),
    ];
    let r = kit::stats(ui, &items, 2);
    let notes = [
        "кораблей в строю; у врага — по разведке",
        "вес флотов в бою: пушки, торпеды, корпуса — по тому, насколько корабли целы",
        "доля нашей внешней торговли, что идёт по морю: её можно топить",
        "что враг потопил за месяц из наших грузов и грузов тех, кто торгует с нами",
        "сколько топлива и боеприпасов флот получил из того, что нужно; без них корабли стоят в портах",
        "сколько кораблей верфи строят разом: их дают побережье и развитие его провинций",
    ];
    for (i, resp) in r.into_iter().enumerate() {
        resp.tip(|| Card::new(items[i].title.clone()).icon(items[i].icon.clone()).row("сейчас", items[i].value.clone()).note(notes[i]));
    }
    let words = if ours == 0 {
        "Флота нет: морскую торговлю некому защитить."
    } else if ratio >= 1.3 {
        "Наш флот сильнее вражеского: можно запереть его порты блокадой."
    } else if ratio >= 0.8 {
        "Флоты примерно равны: бой за море решит удача и выучка."
    } else {
        "Враг сильнее на море: линейным кораблям лучше стоять в порту, а воевать рейдерами и подлодками."
    };
    say(ui, s, words);
    // the navy's share of the army's money
    let share = g.navy_share(me);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Флоту — доля армейских денег:").size(15.0).color(s.soft));
        ui.label(RichText::new(format!("{:.0}%", share * 100.0)).family(egui::FontFamily::Name("bold".into())).size(15.0).color(s.ink));
        if g.navy.share[me].is_none() {
            ui.label(RichText::new("(по обычаю)").size(13.0).color(s.soft));
        }
    });
    let levels = [0.0f32, 0.1, 0.2, 0.3, 0.45];
    let names = ["Нет", "10%", "20%", "30%", "45%"];
    let icons = ["stop", "anchor", "anchor", "anchor", "anchor"];
    let now = levels.iter().enumerate().min_by(|a, b| (a.1 - share).abs().total_cmp(&(b.1 - share).abs())).map_or(1, |x| x.0);
    let choices: Vec<(&str, &str)> = names.iter().zip(icons).map(|(n, i)| (*n, i)).collect();
    let pick = ui.scope(|ui| {
        widgets::law(ui, &choices, now, usize::MAX, &|k| {
            Card::new(format!("Флоту {}", names[k]))
                .icon("anchor")
                .sub("доля армейских денег")
                .text("Флот строит корабли и живёт на свою долю денег армии: чем она больше, тем меньше остаётся дивизиям.")
                .row("в месяц было бы", money(levels[k] * g.mil.book[me].allotted))
                .note(if k == now { "выбрано" } else { "щёлкните — решение уйдёт в правительство" })
        })
    });
    if let Some(k) = pick.inner {
        act(g, &[("navy_share", format!("{:.0}", levels[k] * 100.0))]);
    }
    squadrons(ui, s, g);
    yards(ui, s, g);
    trade(ui, s, g);
    battles(ui, s, g);
}

/// The squadrons: ships by kind, how whole, the mission (tiles) and the zone (arrows).
fn squadrons(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    head(ui, s, "Эскадры");
    let zones = zones_of_note(g);
    let fleet = g.navy.fleets[me].clone();
    if fleet.iter().all(|q| q.ships.is_empty()) {
        say(ui, s, "Кораблей в строю нет.");
        return;
    }
    for (k, q) in fleet.iter().enumerate() {
        if q.ships.is_empty() {
            continue;
        }
        let mut kinds: Vec<(usize, usize)> = Vec::new();
        for v in &q.ships {
            match kinds.iter_mut().find(|x| x.0 == v.kind) {
                Some(x) => x.1 += 1,
                None => kinds.push((v.kind, 1)),
            }
        }
        let what: Vec<String> = kinds.iter().map(|&(kind, n)| format!("{} × {n}", pack.ships[kind].name)).collect();
        let health = q.ships.iter().map(|v| v.health).sum::<f32>() / q.ships.len() as f32;
        let power: f32 = q.ships.iter().map(|v| navy::power(&pack, v)).sum();
        let zone = g.navy.seas.zones.get(q.zone).map_or("—".to_string(), |z| z.name.clone());
        kit::framed(ui, None, false, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(&q.name).family(egui::FontFamily::Name("head".into())).size(16.0).color(s.ink));
                ui.label(RichText::new(format!("· адмирал {}", q.admiral)).size(13.5).color(s.soft));
            });
            ui.label(RichText::new(what.join(", ")).size(13.5).color(s.ink));
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} · {zone}{}", q.mission.name(), if q.staff { " · ведёт адмиралтейство" } else { " · ведёте вы" })).size(13.5).color(s.ink));
                widgets::meter(ui, health, ui.available_width().min(120.0)).tip(|| Card::new("Целость кораблей").icon("spanner").row("в среднем", format!("{:.0}%", health * 100.0)).row("сила в бою", format!("{power:.0}")).note("в порту корабли чинятся быстро, в море — едва"));
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for (m, (mission, name, icon, about)) in MISSIONS.iter().enumerate() {
                    let on = q.mission == *mission;
                    if kit::act_button(ui, name, icon, true, on).tip(|| Card::new(*name).icon(*icon).text(*about).note(if on { "так сейчас" } else { "щёлкните — приказ эскадре (её больше не ведёт адмиралтейство)" })).clicked() {
                        act(g, &[("sq", k.to_string()), ("mission", m.to_string())]);
                    }
                }
            });
            if !zones.is_empty() {
                ui.horizontal(|ui| {
                    let at = zones.iter().position(|&z| z == q.zone);
                    let prev = at.map_or(zones[0], |i| zones[(i + zones.len() - 1) % zones.len()]);
                    let next = at.map_or(zones[0], |i| zones[(i + 1) % zones.len()]);
                    // (the fonts have no arrows: guillemets)
                    if widgets::small(ui, "«").tip(|| Card::new("Другая зона").icon("compass").row("туда", g.navy.seas.zones[prev].name.clone())).clicked() {
                        act(g, &[("sq", k.to_string()), ("zone", prev.to_string())]);
                    }
                    ui.label(RichText::new(format!("Зона: {zone}")).size(14.0).color(s.ink)).tip(|| zone_card(g, q.zone));
                    if widgets::small(ui, "»").tip(|| Card::new("Другая зона").icon("compass").row("туда", g.navy.seas.zones[next].name.clone())).clicked() {
                        act(g, &[("sq", k.to_string()), ("zone", next.to_string())]);
                    }
                    if !q.staff && widgets::small(ui, "Адмиралтейству").tip(|| Card::new("Отдать адмиралтейству").icon("compass").text("Эскадру снова поведёт адмиралтейство: само решит, куда и зачем.")).clicked() {
                        act(g, &[("sq", k.to_string()), ("sq_staff", "1".into())]);
                    }
                });
            }
        });
        ui.add_space(4.0);
    }
}

/// A zone's card: whose coasts, whose trade passes it, who holds it.
fn zone_card(g: &Game, z: usize) -> Card {
    let me = g.me;
    let Some(zone) = g.navy.seas.zones.get(z) else { return Card::new("Зона") };
    let ours = g.trade_by_zone(me).get(z).copied().unwrap_or(0.0);
    let theirs = g.trade_by_zone(1 - me).get(z).copied().unwrap_or(0.0);
    let mut c = Card::new(zone.name.clone()).icon("compass").sub("морская зона");
    if g.navy.home[me].contains(&z) {
        c = c.row("берег", "наш");
    }
    if g.navy.home[1 - me].contains(&z) {
        c = c.row("берег", "врага");
    }
    c = c.row("наших грузов в месяц", money(ours)).row("грузов врага в месяц", money(theirs));
    if g.h.at_war {
        let ctrl = g.navy.control[me].get(z).copied().unwrap_or(0.0);
        c = c.tone("наша доля моря", format!("{:.0}%", ctrl * 100.0), if ctrl >= 0.5 { Tone::Good } else { Tone::Bad });
        let raid = g.navy.raids[1 - me].get(z).copied().unwrap_or(0.0);
        if raid > 0.0 {
            c = c.tone("рейдеры врага", format!("{raid:.1}"), Tone::Bad);
        }
    }
    c
}

/// The shipyards: the slipways, what is building, what may be laid down.
fn yards(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    head(ui, s, "Верфи");
    if g.navy.coast[me].is_empty() {
        say(ui, s, "Верфей нет: нет моря.");
        return;
    }
    let ways = g.navy.slipways[me];
    let staff = g.navy.yard_staff[me];
    say(ui, s, &format!(
        "Стапелей {ways}: столько кораблей строится разом, остальные ждут очереди. {}",
        if staff { "Что закладывать, решает адмиралтейство — пока вы не заложите корабль сами." } else { "Что закладывать, решаете вы." }
    ));
    let queue = g.navy.yards[me].clone();
    for (i, b) in queue.iter().enumerate() {
        let k = &pack.ships[b.kind];
        ui.horizontal(|ui| {
            let busy = i < ways;
            let name = format!("{} {}{}", k.name, b.name, if busy { "" } else { " · ждёт стапеля" });
            ui.label(RichText::new(name).size(14.0).color(if busy { s.ink } else { s.soft }));
            let left = ((1.0 - b.done) * k.months as f32).ceil();
            widgets::meter(ui, b.done, ui.available_width() - 34.0).tip(|| Card::new(k.name.clone()).icon(k.icon.clone()).row("построен", format!("{:.0}%", b.done * 100.0)).row("ещё месяцев (если хватит стали)", format!("{left:.0}")));
            if widgets::small(ui, "×").tip(|| Card::new("Снять со стапеля").icon("stop").text("Корабль разберут: потраченное не вернуть.")).clicked() {
                act(g, &[("unlay", i.to_string())]);
            }
        });
    }
    if !staff && widgets::small(ui, "Верфи — адмиралтейству").tip(|| Card::new("Верфи — адмиралтейству").icon("compass").text("Адмиралтейство само будет решать, что закладывать.")).clicked() {
        act(g, &[("yards", "staff".into())]);
    }
    ui.add_space(3.0);
    let scale = pack.war.as_ref().map_or(0.02, |w| w.scale);
    let n = g.h.home(me);
    let prices = n.price.clone();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (kind, k) in pack.ships.iter().enumerate() {
            let cost: f32 = k.equip.iter().map(|&(i, a)| a * scale * prices[i]).sum();
            let goods: Vec<String> = k.equip.iter().map(|&(i, a)| format!("{} {a}", pack.goods[i].name)).collect();
            let role = strategy::pack::ROLES.iter().find(|r| r.0 == k.role).map_or("", |r| r.1);
            if kit::act_button(ui, &format!("Заложить: {}", k.name), &k.icon, true, false)
                .tip(|| {
                    Card::new(k.name.clone())
                        .icon(k.icon.clone())
                        .sub(role)
                        .text(k.about.clone())
                        .row("стоит", money(cost))
                        .row("из чего", goods.join(", "))
                        .row("строится", format!("{} мес.", k.months))
                        .row("экипаж", format!("{:.0} чел.", k.crew * 1000.0))
                        .row("пушки / броня / корпус", format!("{} / {} / {}", k.gun, k.armor, k.hull))
                        .row("торпеды / против подлодок", format!("{} / {}", k.torpedo, k.asw))
                        .row("рейды / охрана конвоев", format!("{} / {}", k.raid, k.escort))
                        .note("щёлкните — корабль встанет в очередь верфей")
                })
                .clicked()
            {
                act(g, &[("lay", kind.to_string())]);
            }
        }
    });
}

/// The trade by sea: the lanes it takes, the threats on them.
fn trade(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    let me = g.me;
    head(ui, s, "Морская торговля");
    let t = g.trade_by_zone(me);
    let mut lanes: Vec<usize> = (0..t.len()).filter(|&z| t[z] > 0.0).collect();
    lanes.sort_by(|&a, &b| t[b].total_cmp(&t[a]));
    if lanes.is_empty() {
        say(ui, s, "По морю мы почти ничего не возим.");
        return;
    }
    let most = t[lanes[0]].max(1e-9);
    for &z in lanes.iter().take(8) {
        let raid = g.navy.raids[1 - me].get(z).copied().unwrap_or(0.0);
        let name = g.navy.seas.zones[z].name.clone();
        ui.horizontal(|ui| {
            let (lr, _) = ui.allocate_exact_size(vec2(180.0, 20.0), egui::Sense::hover());
            let mut job = egui::text::LayoutJob::simple_singleline(name.clone(), egui::FontId::new(14.0, egui::FontFamily::Proportional), s.ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(176.0);
            let gal = ui.painter().layout_job(job);
            ui.painter().galley(egui::pos2(lr.min.x, lr.center().y - gal.size().y / 2.0), gal, s.ink);
            widgets::meter(ui, t[z] / most, ui.available_width() - 4.0).tip(|| {
                let c = zone_card(g, z);
                if raid > 0.0 { c.note("здесь охотятся рейдеры врага: охрана конвоев сбережёт грузы") } else { c }
            });
        });
        if raid > 0.0 {
            ui.label(RichText::new("   здесь рейдеры врага").size(12.5).color(Color32::from_rgb(196, 70, 50)));
        }
    }
}

/// The last battles at sea.
fn battles(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    let me = g.me;
    if g.navy.clashes.is_empty() {
        return;
    }
    head(ui, s, "Бои на море");
    for b in g.navy.clashes.iter().rev().take(6) {
        let ours = b.clash.sunk[me].len();
        let theirs = b.clash.sunk[1 - me].len();
        let held = match b.clash.held {
            Some(x) if x == me => "море за нами",
            Some(_) => "море за врагом",
            None => "без победителя",
        };
        let line = format!("{} — {}: потоплено наших {ours}, вражеских {theirs}; {held}", crate::words::date_words(b.month), b.zone);
        let pack = g.pack.clone();
        ui.label(RichText::new(line).size(13.5).color(s.ink)).tip(|| {
            Card::new(format!("Бой: {}", b.zone))
                .icon("anchor")
                .row("сила флотов перед боем", format!("{:.0} к {:.0}", b.clash.weight[me], b.clash.weight[1 - me]))
                .row("наши потери", crate::navy::sunk_words(&pack, &b.clash.sunk[me]))
                .row("потери врага", crate::navy::sunk_words(&pack, &b.clash.sunk[1 - me]))
                .row("ушли с повреждениями", format!("наших {}, вражеских {}", b.clash.hurt[me], b.clash.hurt[1 - me]))
        });
    }
}
