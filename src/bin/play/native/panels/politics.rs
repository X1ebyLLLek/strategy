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

/// The pages of the folder «Политика» (28th session; 38th session: diplomacy on its own page).
pub const PAGES: [&str; 4] = ["Соседи", "Дипломатия", "Война и мир", "Решения"];

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
    }
    if page == 1 {
        super::diplomacy::diplomacy(ui, s, g);
    }
    if page == 2 {
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
        war_goals_guide(ui, s);
    }
    if page == 3 {
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
        // (36th session: each neighbour a card of its own — the flag in a frame, the standing under the name, what their government
        // thinks of us as a figure and a gauge, their economy against ours as pips; before, rows of a table)
        let can = blocked.is_none() && !is_rival;
        let opinion = g.diplo.of(c, g.country(me));
        let col = if ratio > 1.25 { Color32::from_rgb(200, 70, 50) } else if ratio < 0.8 { Color32::from_rgb(90, 150, 60) } else { Color32::from_rgb(200, 150, 50) };
        let mut rc = super::super::realm::Realm::new(c, name.clone(), opinion);
        rc.line = stand.to_string();
        rc.line_col = Some(if is_rival { s.accent } else { s.soft });
        rc.edge = is_rival.then_some(s.accent);
        rc.on = is_rival;
        rc.power = Some(((2.5 * ratio).clamp(0.2, 5.0), col));
        rc.keep = if can { 96.0 } else { 0.0 };
        let (hit, row) = super::super::realm::card(ui, &rc);
        let mut b_resp = None;
        // (the button only when the rival may be changed; why not — on the card)
        if can {
            let b = egui::Rect::from_min_size(pos2(row.max.x - 98.0, row.min.y + 8.0), vec2(86.0, 24.0));
            let resp = ui.put(b, |ui: &mut egui::Ui| widgets::plate(ui, "Цель", 86.0, false, can));
            b_resp = Some(resp);
            // (32nd session: the button put into the row dragged the cursor up under itself; back under the card)
            ui.advance_cursor_after_rect(egui::Rect::from_min_size(row.min, vec2(row.width(), super::super::realm::HEIGHT + 2.0)));
        }
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
        let foe = 1 - me;
        let foe_name = &g.h.home(foe).name;
        let who = g.state.aggressor.map_or(String::new(), |x| format!(" Войну {} {}.", strategy::world::names::agree(&g.h.home(x).name, "начала"), g.h.home(x).name));
        let war_m = g.h.war.month;
        let tenure_str = if war_m < 12 {
            format!("{war_m} {}", crate::words::plural(war_m as usize, "месяц", "месяца", "месяцев"))
        } else {
            let y = war_m / 12;
            let m = war_m % 12;
            if m == 0 {
                format!("{y} {}", crate::words::plural(y as usize, "год", "года", "лет"))
            } else {
                format!("{y} г. {m} мес.")
            }
        };

        // Header frame with war status & capital state
        let fallen = g.h.war.fallen;
        let (frame_tone, frame_title) = if fallen == Some(foe) {
            (super::super::kit::GOOD, format!("Идёт война ({tenure_str}).{who} Столица врага пала!"))
        } else if fallen == Some(me) {
            (super::super::kit::BAD, format!("Идёт война ({tenure_str}).{who} Наша столица захвачена!"))
        } else {
            (super::super::kit::BAD, format!("Идёт война ({tenure_str}).{who}"))
        };

        super::super::kit::framed(ui, Some(frame_tone), true, |ui| {
            super::super::kit::bold(ui, frame_title, 14.0, s.ink);
        });

        // Theater stats: battles, casualties, provinces, population
        let (mut my_wins, mut foe_wins) = (0usize, 0usize);
        let (mut my_men_lost, mut foe_men_lost) = (0.0f32, 0.0f32);
        for f in &g.h.war.fought {
            if f.attacker == me {
                my_men_lost += f.men[0];
                foe_men_lost += f.men[1];
                if f.won { my_wins += 1; } else { foe_wins += 1; }
            } else {
                my_men_lost += f.men[1];
                foe_men_lost += f.men[0];
                if !f.won { my_wins += 1; } else { foe_wins += 1; }
            }
        }
        let total_battles = g.h.war.fought.len();
        let took_from_foe = (0..g.h.war.owner.len()).filter(|&p| g.h.war.owner[p] == me && g.h.war.th.home[p] == foe).count();
        let lost_to_foe = (0..g.h.war.owner.len()).filter(|&p| g.h.war.owner[p] == foe && g.h.war.th.home[p] == me).count();
        let held_pop = g.h.war.people();

        use super::super::kit::Stat;
        let stat_items = vec![
            Stat::new("clash", "Сражения", format!("Побед {my_wins} из {total_battles}"), if my_wins >= foe_wins { super::super::kit::GOOD } else { super::super::kit::MIXED }),
            Stat::new("broken", "Потери войск", format!("{my_men_lost:.1} тыс. / {foe_men_lost:.1} тыс."), if my_men_lost <= foe_men_lost { super::super::kit::GOOD } else { super::super::kit::BAD }),
            Stat::new("flag", "Земли театра", format!("+{took_from_foe} / -{lost_to_foe}"), if took_from_foe >= lost_to_foe { super::super::kit::GOOD } else { super::super::kit::BAD }),
            Stat::new("people", "Люди под властью", format!("{:.0}% / {:.0}%", held_pop[me] * 100.0, held_pop[foe] * 100.0), if held_pop[me] >= held_pop[foe] { super::super::kit::GOOD } else { super::super::kit::MIXED }),
        ];
        let r_stats = super::super::kit::stats(ui, &stat_items, 2);
        if let Some(r0) = r_stats.get(0) {
            r0.clone().tip(|| Card::new("Сражения").icon("clash").row("наших побед", my_wins.to_string()).row("побед противника", foe_wins.to_string()).row("всего боёв", total_battles.to_string()));
        }
        if let Some(r1) = r_stats.get(1) {
            r1.clone().tip(|| Card::new("Потери личного состава").icon("broken").row("наши безвозвратные потери", format!("{my_men_lost:.1} тыс. бойцов")).row("потери противника", format!("{foe_men_lost:.1} тыс. бойцов")).note("учитываются погибшие, раненые и пропавшие без вести"));
        }
        if let Some(r2) = r_stats.get(2) {
            r2.clone().tip(|| Card::new("Контроль провинций").icon("flag").row("занято провинций врага", took_from_foe.to_string()).row("потеряно своих провинций", lost_to_foe.to_string()).row("всего в театре", g.h.war.owner.len().to_string()));
        }
        if let Some(r3) = r_stats.get(3) {
            r3.clone().tip(|| Card::new("Население под контролем").icon("people").row("у нас осталось", format!("{:.1}%", held_pop[me] * 100.0)).row("у врага осталось", format!("{:.1}%", held_pop[foe] * 100.0)).note("утрата городов и населения истощает тыл и подрывает волю к войне"));
        }

        ui.add_space(4.0);

        // Diplomatic peace assessment
        let bot_takes = g.takes_peace(foe);
        let stance_tone = if bot_takes { super::super::kit::GOOD } else { super::super::kit::MIXED };
        let stance_text = if bot_takes {
            format!("{foe_name} готова принять мир: потеря территорий и истощение вынуждают искать соглашения.")
        } else {
            format!("{foe_name} отвергает мир: правительство противника уверено в победе и продолжает наступление.")
        };

        super::super::kit::framed(ui, Some(stance_tone), false, |ui| {
            ui.label(RichText::new("Дипломатическая оценка шансов на мир:").family(egui::FontFamily::Name("bold".into())).size(12.5).color(s.soft));
            super::super::kit::words(ui, &stance_text, 13.0, s.ink);
        });

        ui.add_space(3.0);
        let btn_label = if bot_takes {
            "Предложить мир (враг примет)"
        } else {
            "Предложить мир (враг отвергнет)"
        };
        if widgets::plate(ui, btn_label, w, bot_takes, true).tip(|| {
            let mut c = Card::new("Мирные переговоры").icon("scroll")
                .row("условия соглашения", "фиксация фактической линии фронта")
                .row("позиция противника", if bot_takes { "согласен на мир" } else { "отвергает мир" });
            if bot_takes {
                c = c.note("При заключении мира каждая сторона оставляет за собой все занятые на карте земли. Война закончится, начнётся перемирие.");
            } else {
                c = c.note("Противник считает своё положение выгодным. Чтобы заставить его просить мира, нанесите поражение его войскам или займите больше провинций.");
            }
            c
        }).clicked() {
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
                let left = c.ready - now;
                let total = crate::world::statecraft::JUSTIFY.max(1);
                let frac = (1.0 - left as f32 / total as f32).clamp(0.05, 0.98);
                super::super::kit::framed(ui, Some(super::super::kit::MIXED), true, |ui| {
                    super::super::kit::bold(ui, format!("Цель войны обосновывается — ещё {}.", crate::words::tenure(left)), 14.0, s.ink);
                    let r = super::super::kit::gauge(ui, ui.available_width(), 12.0, frac, super::super::kit::MIXED);
                    r.tip(|| Card::new("Повод к войне").icon("scroll").row("осталось", crate::words::tenure(left)));
                });

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

fn war_goals_guide(ui: &mut egui::Ui, s: &Skin) {
    ui.add_space(4.0);
    super::super::kit::ribbon(ui, "Зачем и как воевать: стратегия", Some("flag"));
    let goals = [
        ("Сырьевой суверенитет", "Захват месторождений угля, руды и нефти: без сырья заводы встают, а танки замирают без топлива."),
        ("Подрыв военной машины", "Захват промышленных центров соседа лишает его армию пополнения снаряжением."),
        ("Контрибуции и рынки", "Разгром врага принуждает его платить ежемесячную дань (до 4% бюджета) и открывает пути без пошлин."),
        ("Исторический реванш", "Возврат утраченных земель укрепляет власть и снимает народное недовольство."),
    ];
    for (title, about) in goals {
        let resp = ui.vertical(|ui| {
            ui.label(RichText::new(format!("• {title}")).family(egui::FontFamily::Name("bold".into())).size(12.5).color(s.ink));
            super::super::kit::words(ui, about, 12.0, s.soft);
        });
        resp.response.tip(|| Card::new(title).icon("flag").text(about));
    }
}
