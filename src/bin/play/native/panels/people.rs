//! The folder of people: those who matter in the country and in the rival's — who holds which post,
//! the living of note by their callings (their lives on their cards: where they came from, their
//! posts, their deeds, their pupils, comrades and rivals), the great dead, the chronicle of people.
//! What makes a person what he is (talent, honesty, beliefs) nobody sees: only his life.

use strategy::world::folk::{Calling, Person, Post};
use super::*;
use super::super::kit::{self, Stat};

/// A person's card: his life as anyone may read it.
fn life_card(g: &Game, p: &Person) -> Card {
    let now = g.month() as i32;
    let post = p.post().map_or(p.calling.word().to_string(), |x| x.word().to_string());
    let mut c = Card::new(p.name.clone()).face(g.face_of(&p.name, if p.calling == Calling::General { "general" } else { "civil" })).sub(format!("{post} · {}", p.fame_word()));
    for line in p.life(&g.folk, now) {
        c = c.text(line);
    }
    if p.calling == Calling::General && (p.won + p.lost) > 0 {
        c = c.row("побед в боях", p.won.to_string()).row("поражений", p.lost.to_string()).row("взял провинций", p.took.to_string());
    }
    if p.works > 0 {
        c = c.row("открытий", p.works.to_string());
    }
    c.note("ум, честность и убеждения человека не видны никому — только его дела")
}

/// A row of a person: the portrait, the name, his calling and post, his age and fame; his life on
/// the card.
fn person_row(ui: &mut egui::Ui, s: &Skin, g: &Game, id: u32, note: &str) {
    let p = g.folk.get(id).clone();
    let now = g.month() as i32;
    // (30th session: the stripe at the left in the calling's colour — soldiers red, scholars blue, economists amber,
    // statesmen violet — so that a list of thirty people falls into kinds at a glance; the calling as a small tag,
    // the age and fame on a line of their own, the last deed set apart in italics-grey)
    let tone = match p.calling {
        Calling::General => kit::BAD,
        Calling::Scientist => Color32::from_rgb(70, 110, 160),
        Calling::Economist => kit::MIXED,
        Calling::Statesman => Color32::from_rgb(120, 90, 150),
    };
    let resp = kit::framed(ui, Some(if p.alive() { tone } else { s.soft }), false, |ui| {
        ui.horizontal(|ui| {
            let post_kind = if p.calling == Calling::General { "general" } else { "civil" };
            if let Some(img) = portrait(&g.face_of(&p.name, post_kind), 44.0) {
                ui.add(img);
            } else {
                let icons = chrome::icons(ui.ctx());
                let (r, _) = ui.allocate_exact_size(vec2(44.0, 55.0), egui::Sense::hover());
                super::super::look::icon(&icons, p.calling.icon(), 34.0, s.soft).paint_at(ui, r.shrink(3.0));
            }
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                kit::bold(ui, p.name.clone(), 15.5, s.ink);
                let what = match p.post() {
                    Some(x) => format!("{} · {}", p.calling.word(), x.word()),
                    None => p.calling.word().to_string(),
                };
                kit::words(ui, what, 13.0, kit::ink(s, tone));
                let age = if p.alive() { format!("{} лет", p.age(now) as u32) } else { format!("умер в {} лет", p.age(now) as u32) };
                kit::words(ui, format!("{age} · {}", p.fame_word()), 12.5, s.soft);
                if !note.is_empty() {
                    kit::words(ui, note, 12.5, s.ink);
                } else if let Some(d) = p.deeds.last() {
                    kit::words(ui, d.text.clone(), 12.5, s.ink.gamma_multiply(0.85));
                }
            });
        });
    });
    resp.response.tip(|| life_card(g, &p));
}

/// A post of the government and who holds it.
fn post_row(ui: &mut egui::Ui, s: &Skin, g: &Game, title: &str, who: Option<u32>, empty: &str) {
    kit::words(ui, title.to_uppercase(), 12.0, s.soft);
    match who {
        Some(id) => person_row(ui, s, g, id, ""),
        None => {
            kit::words(ui, empty, 14.0, s.ink);
            ui.add_space(3.0);
        }
    }
}

/// The folder of people.
pub fn persons(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let c = g.country(me);
    let o = g.country(1 - me);
    let now = g.month() as i32;
    // the figures: how many of note, geniuses as fame shows them, the great dead
    {
        let living = g.folk.all_living(c);
        let by = |k: Calling| living.iter().filter(|&&id| g.folk.get(id).calling == k).count();
        let famous = living.iter().filter(|&&id| g.folk.get(id).fame >= 1.4).count();
        let dead = g.folk.great_dead(c, 1.4).len();
        let word = |n: usize| match n {
            0 => "никого".to_string(),
            1..=2 => "единицы".to_string(),
            3..=6 => "несколько".to_string(),
            _ => "много".to_string(),
        };
        let items = vec![
            Stat::new("helmet", "Военные", word(by(Calling::General)), kit::PLAIN),
            Stat::new("science", "Учёные", word(by(Calling::Scientist)), if by(Calling::Scientist) == 0 { kit::BAD } else { kit::PLAIN }),
            Stat::new("coins", "Экономисты", word(by(Calling::Economist)), if by(Calling::Economist) == 0 { kit::BAD } else { kit::PLAIN }),
            Stat::new("crown", "Политики", word(by(Calling::Statesman)), kit::PLAIN),
            Stat::new("medal", "Знаменитые", word(famous), if famous > 0 { kit::GOOD } else { kit::MIXED }),
            Stat::new("scroll", "Великие в прошлом", word(dead), kit::PLAIN),
        ];
        let nums = [by(Calling::General), by(Calling::Scientist), by(Calling::Economist), by(Calling::Statesman), famous, dead];
        let resp = kit::stats(ui, &items, 3);
        for (i, r) in resp.into_iter().enumerate() {
            let t = items[i].title.clone();
            let n = nums[i];
            r.tip(|| Card::new(t).icon(items[i].icon.clone()).row("людей", n.to_string()).note("люди, чьи имена известны: их рождает страна — её образованные, её войны, её богатство; гениев может не быть вовсе"));
        }
    }
    // the government
    kit::ribbon(ui, "Правительство", Some("crown"));
    let eco = g.adviser.and_then(|k| g.roster.pool.get(k).copied());
    let sci = g.scholar.and_then(|k| g.roster.scholars.get(k).copied());
    post_row(ui, s, g, "Хозяйство", eco, "ведёте вы сами (папка «Экономика» — назначить советника)");
    if g.h.labs[me].is_some() {
        post_row(ui, s, g, "Наука", sci, "ведёте вы сами (папка «Наука» — назначить главу)");
    }
    post_row(ui, s, g, "Генеральный штаб", g.folk.holder(c, Post::Staff), "место начальника штаба пусто");
    if let Some(r) = g.folk.holder(c, Post::Ruler) {
        post_row(ui, s, g, "Глава государства", Some(r), "");
    }
    // generals leading the groups
    let leading: Vec<u32> = g.h.war.command[me].as_ref().map_or(Vec::new(), |gs| gs.iter().filter_map(|x| x.general.as_ref()).filter_map(|x| g.folk.find(c, &x.name)).collect());
    if !leading.is_empty() {
        kit::words(ui, "ВО ГЛАВЕ ГРУПП АРМИЙ", 12.0, s.soft);
        for id in leading {
            person_row(ui, s, g, id, "");
        }
    }
    // the people of the country by callings
    kit::ribbon(ui, "Люди страны", Some("people"));
    for calling in Calling::ALL {
        let list = g.folk.living(c, calling);
        if list.is_empty() {
            kit::words(ui, format!("{}: нет никого, кто бы был на слуху", heading(calling)), 13.5, s.soft);
            continue;
        }
        kit::words(ui, heading(calling).to_uppercase(), 12.0, s.soft);
        for &id in list.iter().take(5) {
            person_row(ui, s, g, id, "");
        }
        if list.len() > 5 {
            kit::words(ui, format!("и ещё {} — менее известные", list.len() - 5), 12.5, s.soft);
        }
    }
    // the rulers of the country, from the earliest remembered
    let mut rulers: Vec<(i32, Option<i32>, u32)> = Vec::new();
    for p in g.folk.people.iter().filter(|p| p.country == c) {
        for st in p.posts.iter().filter(|x| x.post == Post::Ruler) {
            rulers.push((st.from, st.to, p.id));
        }
    }
    rulers.sort_by_key(|x| x.0);
    if !rulers.is_empty() {
        kit::ribbon(ui, "Правители страны", Some("crown"));
        for (from, to, id) in rulers.iter().rev().take(8).rev() {
            let p = g.folk.get(*id).clone();
            let when = match to {
                Some(t) => format!("{} — {}", strategy::world::folk::year_word(*from), strategy::world::folk::year_word(*t)),
                None => format!("с {}", strategy::world::folk::year_word(*from)),
            };
            let r = ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(when).size(12.5).color(s.soft));
                ui.label(RichText::new(p.name.clone()).family(egui::FontFamily::Name("bold".into())).size(14.0).color(s.ink));
                ui.label(RichText::new(p.calling.word()).size(12.5).color(s.soft));
            });
            r.response.tip(|| life_card(g, &p));
        }
    }
    // the great dead
    let dead = g.folk.great_dead(c, 1.0);
    if !dead.is_empty() {
        kit::ribbon(ui, "Великие имена прошлого", Some("scroll"));
        for &id in dead.iter().take(6) {
            person_row(ui, s, g, id, "");
        }
    }
    // the rival
    kit::ribbon(ui, "Кто правит у соперника", Some("binoculars"));
    let rival = g.h.home(1 - me).name.clone();
    post_row(ui, s, g, &format!("Глава государства ({rival})"), g.folk.holder(o, Post::Ruler), "власть ни в чьих руках");
    if let Some((what, why, _)) = g.course_words(1 - me) {
        let resp = kit::words(ui, format!("Курс: {what}"), 14.0, s.ink);
        resp.tip(|| Card::new("Курс соседа").icon("binoculars").text(why.clone()).note("по донесениям разведки и делам его правительства"));
    }
    post_row(ui, s, g, "Хозяйство", g.roster.foe_econ, "никто");
    if g.pack.science.is_some() {
        post_row(ui, s, g, "Наука", g.roster.foe_science, "никто");
    }
    post_row(ui, s, g, "Генеральный штаб", g.folk.holder(o, Post::Staff), "место пусто");
    let theirs: Vec<u32> = g.folk.all_living(o).into_iter().filter(|&id| g.folk.get(id).fame >= 1.0 && g.folk.get(id).post().is_none_or(|x| x == Post::Command || x == Post::Chair)).take(4).collect();
    if !theirs.is_empty() {
        kit::words(ui, "ИЗВЕСТНЫЕ ЛЮДИ СОСЕДА", 12.0, s.soft);
        for id in theirs {
            person_row(ui, s, g, id, "");
        }
    }
    // the chronicle of people
    let lines: Vec<(i32, usize, String)> = g.folk.chronicle.iter().rev().filter(|x| x.1 == c || x.1 == o).take(14).cloned().collect();
    if !lines.is_empty() {
        kit::ribbon(ui, "Летопись людей", Some("quill"));
        for (m, cc, line) in lines {
            let who = if cc == c { String::new() } else { format!("{}: ", g.h.home(1 - me).name) };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(strategy::world::folk::year_word(m)).size(12.0).color(s.soft));
                ui.label(RichText::new(format!("{who}{line}")).size(13.5).color(s.ink));
            });
        }
    }
    let _ = now;
}

/// The heading of a calling's people.
fn heading(c: Calling) -> &'static str {
    match c {
        Calling::General => "Военные",
        Calling::Scientist => "Учёные",
        Calling::Economist => "Экономисты",
        Calling::Statesman => "Политики",
    }
}
