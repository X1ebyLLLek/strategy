//! The folder of politics: the country's leaders, its neighbours (treaties, claims, war),
//! the war's controls.

use super::*;

/// Politics: the government, the world's mood, war and peace.
/// What the people think of the war: support as a figure, its months as a chart, what moves it.
fn war_support(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    use super::super::kit::{self, Stat};
    use crate::country::war_mood::{DEMAND_PEACE, DESERT, STRIKES};
    if !g.h.at_war {
        return;
    }
    let me = g.me;
    let sup = g.war_mood.support[me];
    let foe = g.war_mood.support[1 - me];
    let word = |x: f32| match x {
        x if x >= 0.7 => "единодушна",
        x if x >= 0.5 => "твёрдая",
        x if x >= STRIKES => "колеблется",
        x if x >= DESERT => "слабая",
        x if x >= DEMAND_PEACE => "на исходе",
        _ => "требует мира",
    };
    let tone = |x: f32| if x >= 0.5 { kit::GOOD } else if x >= STRIKES { kit::MIXED } else { kit::BAD };
    let items = vec![
        Stat::new("flag", "Поддержка войны", format!("{:.0}% · {}", sup * 100.0, word(sup)), tone(sup)),
        Stat::new("binoculars", "У врага", format!("{:.0}% · {}", foe * 100.0, word(foe)), tone(foe)),
        Stat::new("strike", "Бастуют заводы", format!("{:.0}% оружия", g.war_mood.strike[me] * 100.0), if g.war_mood.strike[me] > 0.0 { kit::BAD } else { kit::PLAIN }),
    ];
    let r = kit::stats(ui, &items, 3);
    let parts = g.war_mood.parts[me].clone();
    if let Some(resp) = r.into_iter().next() {
        resp.tip(|| {
            let mut c = Card::new("Поддержка войны").icon("flag").sub("куда тянет войну народ");
            for (name, x) in &parts {
                c = c.tone(name.clone(), format!("{:+.0}", x * 100.0), if *x >= 0.0 { Tone::Good } else { Tone::Bad });
            }
            c.note(format!("ниже {:.0} — забастовки на военных заводах, ниже {:.0} — бегут солдаты, ниже {:.0} — народ требует мира", STRIKES * 100.0, DESERT * 100.0, DEMAND_PEACE * 100.0))
        });
    }
    let hist = &g.war_mood.history[me];
    let theirs = &g.war_mood.history[1 - me];
    if hist.len() >= 2 {
        let months: Vec<u32> = hist.iter().map(|x| x.0).collect();
        let series = vec![
            ("Мы".to_string(), s.accent, hist.iter().map(|x| x.1 * 100.0).collect::<Vec<f32>>()),
            ("Враг".to_string(), Color32::from_rgb(170, 70, 60), months.iter().map(|m| theirs.iter().find(|x| x.0 == *m).map_or(0.0, |x| x.1 * 100.0)).collect()),
        ];
        let card = |i: usize| Card::new("Поддержка войны").icon("flag").sub(date_words(months[i])).row("у нас", format!("{:.0}%", series[0].2[i])).row("у врага", format!("{:.0}%", series[1].2[i]));
        widgets::chart(ui, &series, &months, 64.0, false, &card);
    }
    let mut v: Vec<(String, f32)> = parts.clone();
    v.sort_by(|a, b| a.1.abs().total_cmp(&b.1.abs()).reverse());
    let top: Vec<String> = v.iter().take(3).map(|(n, x)| format!("{} {}", n, if *x >= 0.0 { "поддерживает" } else { "подтачивает" })).collect();
    if !top.is_empty() {
        say(ui, s, &format!("Сильнее всего: {}.", top.join("; ")));
    }
}

/// The pages of the folder «Политика» (28th session).
pub const PAGES: [&str; 3] = ["Соседи", "Война и мир", "Решения"];

pub fn politics(ui: &mut egui::Ui, s: &Skin, g: &mut Game, page: usize) {
    let me = g.me;
    let foe = 1 - me;
    if page == 0 {
        use super::super::kit::{self, Stat};
        let (st, st2, tension) = (g.state.stability[me], g.state.stability[foe], g.state.tension);
        let shaky = crate::world::statecraft::SHAKY;
        let tone = |x: f32| if x >= shaky { kit::GOOD } else { kit::BAD };
        let items = vec![
            Stat::new("crown", "Наша власть", g.stability_word(me), tone(st)),
            Stat::new("crown", "Власть соседа", g.stability_word(foe), tone(st2)),
            Stat::new("alarm", "В мире", g.tension_word(), if tension < 0.35 { kit::GOOD } else if tension < 0.65 { kit::MIXED } else { kit::BAD }),
        ];
        let r = kit::stats(ui, &items, 3);
        let mut r = r.into_iter();
        if let Some(x) = r.next() {
            x.tip(|| Card::new("Правительство").icon("crown").row("устойчивость", format!("{st:.2}")).row("у соседа", format!("{st2:.2}")).note("слабое правительство не смеет затевать войну"));
        }
        if let Some(x) = r.next() {
            x.tip(|| Card::new("Правительство соседа").icon("crown").row("устойчивость", format!("{st2:.2}")).row("у нас", format!("{st:.2}")));
        }
        if let Some(x) = r.next() {
            x.tip(|| Card::new("Напряжённость").icon("alarm").row("в мире", format!("{tension:.2}")).note("растёт, когда кто-то готовит войну"));
        }
        neighbours(ui, s, g);
        super::diplomacy::diplomacy(ui, s, g);
    }
    if page == 1 {
        head(ui, s, "Война и мир");
        war_controls(ui, s, g);
        war_support(ui, s, g);
        rival_sheet(ui, s, g);
        if g.state.grudge[me] {
            ui.label("Сосед держит земли, которые наша страна потеряла в прошлых войнах.");
        }
        if g.state.grudge[foe] {
            words(ui, format!("{} считает своими часть наших земель.", g.h.home(foe).name), 15.0, s.soft);
        }
    }
    if page == 2 {
        head(ui, s, "Трудные решения");
        let mine: Vec<(u32, String, String, bool)> = g.news.iter().rev().filter(|e| e.tab == "politics").take(12).map(|e| (e.month, e.text.clone(), e.tip.clone(), e.hot)).collect();
        if mine.is_empty() {
            words(ui, "Пока ничего.", 15.0, s.soft);
        }
        // (33rd session: each decision a card — the date in its heading letters, the words under it, a band of colour
        // at its edge: red for what burned, gold for the rest — not a bare column of lines)
        for (m, l, tip, hot) in mine {
            let edge = if hot { super::super::kit::BAD } else { Color32::from_rgb(196, 160, 72) };
            let resp = super::super::kit::framed(ui, Some(edge), hot, |ui| {
                ui.horizontal(|ui| {
                    if let Some(ch) = chrome::of(ui.ctx()) {
                        let (sr, _) = ui.allocate_exact_size(vec2(30.0, 30.0), egui::Sense::hover());
                        super::super::hud::seal(ui, &ch, sr.center(), 14.0, if hot { "alarm" } else { "scroll" }, ch.trim);
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(date_words(m).to_uppercase()).family(egui::FontFamily::Name("head".into())).size(12.5).color(s.soft));
                        super::super::kit::words(ui, l, 14.0, s.ink);
                    });
                });
            });
            if !tip.is_empty() {
                resp.response.tip(|| Card::new("Решение").icon("scroll").text(tip.clone()));
            }
        }
    }
}

/// The neighbours: each country whose land touches ours — its flag, its name, how strong its
/// economy is against ours (a gauge; the numbers on the card), how it stands to us; the rival
/// marked; in peace a button to turn on another.
fn neighbours(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    head(ui, s, "Соседи");
    let me = g.me;
    let rival = g.h.war.th.countries[1 - me];
    let mine = g.h.war.th.countries[me];
    let blocked = g.rival_blocked();
    say(ui, s, "Войну можно готовить против любого соседа: выберите [rival]{главного соперника}, затем обоснуйте [claim]{цель войны}.");
    ui.add_space(4.0);
    let ours = g.h.world.nations[mine].gdp_real.max(1e-9);
    let ch = chrome::of(ui.ctx());
    let icons = chrome::icons(ui.ctx());
    let w = ui.available_width();
    let mut pick = None;
    for c in g.neighbours() {
        let name = g.countries[c].0.clone();
        let theirs = g.h.world.nations[c].gdp_real;
        let ratio = theirs / ours;
        let is_rival = c == rival;
        let past_wars = g.past.iter().filter(|x| (x.sides[0] == c && x.sides[1] == mine) || (x.sides[1] == c && x.sides[0] == mine)).count();
        let grudge = g.past_lost.get(mine).is_some_and(|l| l.iter().any(|&(st, _)| (0..g.prov_state.len()).any(|p| g.prov_state[p] as usize == st && g.country_of[p] as usize == c)));
        let stand = if is_rival && g.h.at_war {
            "война"
        } else if is_rival {
            "главный соперник"
        } else if grudge {
            "держит наши земли"
        } else if past_wars > 0 {
            "воевали прежде"
        } else {
            "сосед"
        };
        let (row, _) = ui.allocate_exact_size(vec2(w, 46.0), egui::Sense::hover());
        let mut v = chrome::Out::new();
        if let Some(ch) = &ch {
            if is_rival {
                ch.plate(&mut v, row, s, false, false, true, false);
                v.push(egui::Shape::rect_filled(egui::Rect::from_min_size(row.min, vec2(4.0, row.height())), 1.0, s.accent));
            } else {
                v.push(egui::Shape::line_segment([pos2(row.min.x, row.max.y - 0.5), pos2(row.max.x, row.max.y - 0.5)], egui::Stroke::new(0.8, s.soft.gamma_multiply(0.4))));
            }
        }
        ui.painter().extend(v);
        let fr = egui::Rect::from_min_size(row.min + vec2(10.0, 8.0), vec2(45.0, 30.0));
        if let (Some(f), Some(ch)) = (super::super::flags::get(ui.ctx(), c), &ch) {
            super::super::flags::paint(ui.ctx(), ui.painter(), fr, &f, ch.trim, Some(&icons));
        }
        let x = fr.max.x + 10.0;
        ui.painter().text(pos2(x, row.min.y + 6.0), egui::Align2::LEFT_TOP, name.to_uppercase(), egui::FontId::new(15.0, egui::FontFamily::Name("head".into())), s.ink);
        // (29th session: the words stop where the gauge begins — «держит наши былые земли» ran under it)
        let mut job = egui::text::LayoutJob::simple_singleline(stand.to_string(), egui::FontId::proportional(13.5), if is_rival { s.accent } else { s.soft });
        // (32nd session: the gauge moved under the button, so the words have the row's width — «держит наши былые земли» no longer cut)
        job.wrap = egui::text::TextWrapping::truncate_at_width((row.max.x - 158.0 - x).max(40.0));
        let sg = ui.painter().layout_job(job);
        ui.painter().galley(pos2(x, row.min.y + 25.0), sg, s.soft);
        // (what their government thinks of us: the figure on a badge of its tone, as in the world's list)
        let opinion = g.diplo.of(c, g.country(me));
        let bx = row.max.x - if blocked.is_none() && !is_rival { 98.0 + 52.0 } else { 58.0 };
        let badge = egui::Rect::from_min_size(pos2(bx, row.min.y + 5.0), vec2(48.0, 22.0));
        ui.painter().rect_filled(badge, 3.0, super::super::hud::lerp(super::diplomacy::tone(opinion), Color32::BLACK, 0.3));
        ui.painter().text(badge.center(), egui::Align2::CENTER_CENTER, format!("{:+.0}", opinion * 100.0), egui::FontId::new(13.5, egui::FontFamily::Name("bold".into())), Color32::WHITE);
        // (their economy against ours: five pips, ours in the middle)
        let mut v = chrome::Out::new();
        let lvl = (2.5 * ratio).clamp(0.2, 5.0);
        let col = if ratio > 1.25 { Color32::from_rgb(200, 70, 50) } else if ratio < 0.8 { Color32::from_rgb(90, 150, 60) } else { Color32::from_rgb(200, 150, 50) };
        let gr = super::super::hud::pips(&mut v, pos2(row.max.x - 76.0, row.min.y + 37.0), 5, lvl, col, 9.0, 5.0);
        ui.painter().extend(v);
        let hit = ui.interact(row, ui.id().with(("neighbour", c)), egui::Sense::hover());
        let can = blocked.is_none() && !is_rival;
        let mut b_resp = None;
        // (the button only when the rival may be changed; why not — on the card)
        if can {
            let b = egui::Rect::from_min_size(pos2(row.max.x - 92.0, row.min.y + 4.0), vec2(86.0, 22.0));
            let label = "Цель";
            let resp = ui.put(b, |ui: &mut egui::Ui| widgets::plate(ui, label, 86.0, false, can));
            b_resp = Some(resp);
        }
        // (32nd session: the button put into the row dragged the cursor up under itself — the rows crowded; back under the row)
        ui.advance_cursor_after_rect(row);
        let _ = gr;
        let card = || {
            let mut cd = Card::new(name.clone()).banner(c, Color32::from_rgb(g.countries[c].1[0], g.countries[c].1[1], g.countries[c].1[2])).sub(stand).row("хозяйство к нашему", format!("×{ratio:.2}")).row("войн с нами в прошлом", past_wars.to_string());
            if is_rival {
                cd = cd.note("против него готовится война");
            } else if let Some(why) = blocked {
                cd = cd.note(format!("сменить соперника нельзя: {why}"));
            } else {
                cd = cd.note("«Цель» — сделать главным соперником: против него пойдёт подготовка к войне");
            }
            cd
        };
        hit.tip(card);
        if let Some(r) = b_resp {
            if r.tip(card).clicked() && can {
                pick = Some(c);
            }
        }
    }
    if let Some(c) = pick {
        g.act(crate::tools::saves::Act::Rival(c));
    }
}

/// The buttons of war and peace.
pub fn war_controls(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let now = g.month();
    let w = ui.available_width();
    if g.h.at_war {
        let who = g.state.aggressor.map_or(String::new(), |x| format!(" Войну {} {}.", strategy::world::names::agree(&g.h.home(x).name, "начала"), g.h.home(x).name));
        ui.label(format!("Идёт война.{who}"));
        if widgets::plate(ui, "Предложить мир", w, false, true).tip(|| Card::new("Предложить мир").icon("scroll").text("Мир на том, что каждый держит. Враг согласится, если не побеждает.")).clicked() {
            g.act(crate::tools::saves::Act::Peace);
        }
        return;
    }
    if now < g.state.truce_until {
        say(ui, s, &format!("[truce]{{Мирный договор}}: новую [claim]{{цель войны}} можно начать не раньше, чем через {}.", crate::words::tenure(g.state.truce_until - now)));
    } else {
        match g.state.claim[me] {
            None if g.state.stability[me] < crate::world::statecraft::SHAKY => {
                ui.label(format!("Правительство {} — затевать войну оно не смеет.", g.stability_word(me)));
            }
            None => {
                let rival = g.h.war.th.countries[1 - me];
                let rname = g.countries[rival].0.clone();
                let rc = g.countries[rival].1;
                if widgets::plate(ui, &format!("Обосновать цель войны: {rname}"), w, true, true).tip(|| Card::new("Цель войны").banner(rival, Color32::from_rgb(rc[0], rc[1], rc[2])).sub(format!("против: {rname}")).text("Полгода правительство готовит повод к войне. Мир это видит: напряжённость растёт, сосед может успеть подготовиться.").note("другого соседа можно выбрать выше, в «Соседях»")).clicked() {
                    g.act(crate::tools::saves::Act::Claim);
                }
            }
            Some(c) if now < c.ready => {
                ui.label(format!("Цель войны обосновывается — ещё {}.", crate::words::tenure(c.ready - now)));
            }
            Some(_) => {
                if widgets::plate(ui, "Объявить войну", w, true, true).tip(|| Card::new("Объявить войну").icon("army").text("Цель войны готова. Генштаб и группы армий начнут, как только война будет объявлена.")).clicked() {
                    act(g, &[("declare", "1".into())]);
                }
            }
        }
    }
    if g.state.claim[1 - me].is_some() {
        ui.label(RichText::new(format!("Разведка: {} готовит войну против нас.", g.h.home(1 - me).name)).color(s.accent));
    }
}

/// The rival face to face with us (33rd session: the page was one button and a white void): the two flags and the
/// bars of what is set against what — economy, people, the government's hold, in war the people's support for it
/// and the divisions — then his intentions and the lands in dispute.
fn rival_sheet(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    use super::super::kit::{self, Side, Stat, VsRow};
    let me = g.me;
    let foe = 1 - me;
    let rival = g.h.war.th.countries[foe];
    let mine = g.h.war.th.countries[me];
    head(ui, s, if g.h.at_war { "Противостояние" } else { "Соперник" });
    let colour = |c: usize| {
        let k = g.countries[c].1;
        Color32::from_rgb(k[0], k[1], k[2])
    };
    let (us, them) = (Side { country: mine, name: g.countries[mine].0.clone(), colour: colour(mine) }, Side { country: rival, name: g.countries[rival].0.clone(), colour: colour(rival) });
    let (gdp_a, gdp_b) = (g.h.world.nations[mine].gdp_real.max(0.0), g.h.world.nations[rival].gdp_real.max(0.0));
    let ratio = gdp_b / gdp_a.max(1e-9);
    let (pop_a, pop_b) = (g.h.world.nations[mine].pop.iter().sum::<f32>(), g.h.world.nations[rival].pop.iter().sum::<f32>());
    let shaky = crate::world::statecraft::SHAKY;
    let (st_me, st_foe) = (g.state.stability[me], g.state.stability[foe]);
    let mut rows = vec![
        VsRow::new("Хозяйство", gdp_a, gdp_b, format!("{}", super::super::charts::num(gdp_a)), format!("{}", super::super::charts::num(gdp_b))),
        VsRow::new("Население", pop_a, pop_b, format!("{} млн", super::super::charts::num(pop_a)), format!("{} млн", super::super::charts::num(pop_b))),
        VsRow::new("Власть держится", st_me.max(0.0), st_foe.max(0.0), g.stability_word(me), g.stability_word(foe)),
    ];
    if g.h.at_war {
        let (sa, sb) = (g.war_mood.support[me], g.war_mood.support[foe]);
        rows.push(VsRow::new("Поддержка войны", sa, sb, format!("{:.0}%", sa * 100.0), format!("{:.0}%", sb * 100.0)));
        if let Some(c) = g.h.war.corps.as_ref() {
            let ours = c.divs.iter().filter(|d| d.side == me).count() as f32;
            let theirs = c.divs.iter().filter(|d| d.side == foe).count() as f32;
            rows.push(VsRow::new("Дивизии на карте", ours, theirs, format!("{ours:.0}"), format!("{theirs:.0}")));
        }
    }
    let resp = kit::versus(ui, &us, &them, &rows, false);
    resp.tip(|| Card::new("Лицом к лицу").icon("clash").row("хозяйство соперника к нашему", format!("×{ratio:.2}")).row("у нас власть", format!("{st_me:.2}")).row("у них власть", format!("{st_foe:.2}")).note("хозяйство — реальный выпуск; в долгой войне оно решает больше числа дивизий"));
    ui.add_space(4.0);
    let dispute = if g.state.grudge[foe] { ("претендует на наши земли", kit::BAD) } else if g.state.grudge[me] { ("держит наши былые земли", kit::MIXED) } else { ("споров о земле нет", kit::GOOD) };
    let intent = if g.state.claim[foe].is_some() { ("готовит войну против нас", kit::BAD) } else { ("мирен", kit::GOOD) };
    let items = vec![
        Stat::new("binoculars", "Намерения", intent.0, intent.1),
        Stat::new("flag", "Земли", dispute.0, dispute.1),
        Stat::new("crown", "Их власть", g.stability_word(foe), if st_foe >= shaky { kit::GOOD } else { kit::BAD }),
        Stat::new("balance", "Хозяйство", format!("×{ratio:.2} к нашему"), if ratio > 1.25 { kit::BAD } else if ratio < 0.8 { kit::GOOD } else { kit::MIXED }),
    ];
    kit::stats(ui, &items, 2);
    ui.add_space(4.0);
    say(ui, s, if g.h.at_war { "Слабость соперника — повод для наступления, но [supply]{снабжение} решает больше, чем число." } else { "Прежде чем обосновывать [claim]{цель войны}, сверьте хозяйство и власть соперника с нашими." });
}
