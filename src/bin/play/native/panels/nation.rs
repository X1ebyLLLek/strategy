//! The folder of the country (as HOI4's country screen): its portrait — the flag, the name, the
//! story its features tell, the figures that matter; its features as emblems (why each is there,
//! what it does, whether it grows or fades); the order of its economy; the state's programmes;
//! its loans; what is known of the rival.

use crate::country::policy::{LAWS, PLANS, PLANS_AT_ONCE};
use crate::country::spirits::{Eff, Tone as SpTone};
use super::*;
use super::super::kit::{self, Stat};

/// The colour of a feature's emblem by how good it is.
fn tone_col(t: SpTone) -> Color32 {
    match t {
        SpTone::Good => kit::GOOD,
        SpTone::Bad => kit::BAD,
        SpTone::Mixed => kit::MIXED,
    }
}

/// What a set of effects does, as rows of a card.
fn eff_card(mut c: Card, e: &Eff, pack: &strategy::pack::Pack) -> Card {
    let lines = e.lines(pack);
    if lines.is_empty() {
        return c.note("ничего заметного");
    }
    for (what, how, good) in lines {
        c = c.tone(what, how, if good { Tone::Good } else { Tone::Bad });
    }
    c
}

/// The country's features as emblems (side `side`): the strongest first.
fn spirit_marks<'a>(g: &'a Game, side: usize) -> Vec<kit::Mark<'a>> {
    let now = g.month();
    let f = &g.spirits.facts[side];
    g.spirits.list[side]
        .iter()
        .map(|sp| {
            let def = sp.def();
            let trend = if sp.want > sp.level + 0.04 { 1 } else if sp.want < sp.level - 0.04 { -1 } else { 0 };
            let name = sp.name(&g.pack);
            let why = sp.why(f, &g.pack);
            let eff = sp.eff(f, &g.pack);
            let full = (def.eff)(f, &g.pack, sp.arg);
            let since = sp.since;
            let level = sp.level;
            let icon = sp.icon(&g.pack);
            let tone = def.tone;
            let title = name.clone();
            kit::Mark {
                icon: icon.clone(),
                tone: tone_col(tone),
                level,
                trend,
                fresh: now.saturating_sub(since) < 3 && since > 0,
                name,
                card: Box::new(move || {
                    let state = match trend {
                        1 => "крепнет",
                        -1 => "слабеет",
                        _ => "держится",
                    };
                    let mut c = Card::new(title.clone()).icon(icon.clone()).sub(format!("{} · сила {:.0}%", state, level * 100.0)).text(why.clone()).head("Что даёт сейчас");
                    c = eff_card(c, &eff, &g.pack);
                    if level < 0.95 {
                        c = c.head("В полную силу");
                        c = eff_card(c, &full, &g.pack);
                    }
                    c.note(if since == 0 { "с начала игры: так страну сделало её прошлое".to_string() } else { format!("появилась: {}", crate::words::date_words(since)) })
                }),
            }
        })
        .collect()
}

/// The folder of the country.
/// The pages of the folder «Страна» (28th session).
pub const PAGES: [&str; 3] = ["Обзор", "Строй и программы", "Сосед"];

pub fn nation(ui: &mut egui::Ui, s: &Skin, g: &mut Game, page: usize) {
    let me = g.me;
    let pack = g.pack.clone();
    if page == 0 {
        portrait_block(ui, s, g);
        // the figures that matter
        {
            let f = &g.spirits.facts[me];
            let n = g.h.home(me);
            let rich = if f.rich > 1.3 { "богаче мира" } else if f.rich > 0.85 { "как в мире" } else if f.rich > 0.55 { "беднее мира" } else { "нищета" };
            let st = g.state.stability[me];
            let money = if f.treasury < 0.0 { "в долгах" } else if f.treasury < 3.0 { "почти пуста" } else if f.treasury < 18.0 { "есть запас" } else { "полна" };
            let war = match (f.last_war, f.last_won) {
                (Some(y), _) if y < 0.5 => "идёт война".to_string(),
                (Some(y), Some(true)) => if y < 15.0 { "недавняя победа".to_string() } else { "давняя победа".to_string() },
                (Some(y), Some(false)) => if y < 15.0 { "недавнее поражение".to_string() } else { "давнее поражение".to_string() },
                (Some(y), None) => if y < 15.0 { "недавно, вничью".to_string() } else { "давно, вничью".to_string() },
                (None, _) => "не помнят".to_string(),
            };
            let items = vec![
                Stat::new("people", "Людей", match f.people { x if x < 5.0 => "малолюдна", x if x < 40.0 => "средняя", x if x < 150.0 => "многолюдна", _ => "огромна" }, kit::PLAIN),
                Stat::new("coins", "На человека", rich, if f.rich > 0.85 { kit::GOOD } else { kit::BAD }),
                Stat::new("crown", "Власть", g.stability_word(me), if st > 0.6 { kit::GOOD } else if st > 0.35 { kit::MIXED } else { kit::BAD }),
                Stat::new("econ", "Казна", money, if f.treasury > 3.0 { kit::GOOD } else if f.treasury >= 0.0 { kit::MIXED } else { kit::BAD }),
                Stat::new("revolt", "Народ", if f.anger < 0.5 { "спокоен" } else if f.anger < 1.2 { "ворчит" } else { "зол" }, if f.anger < 0.5 { kit::GOOD } else if f.anger < 1.2 { kit::MIXED } else { kit::BAD }),
                Stat::new("clash", "Последняя война", war, kit::PLAIN),
            ];
            let nums = [
                format!("{:.1} млн", f.people),
                format!("×{:.2}", f.rich),
                format!("{:.0}%", st * 100.0),
                format!("{:.1} мес.", f.treasury),
                format!("{:.2}", f.anger),
                f.last_war.map_or("—".to_string(), |y| if y < 0.5 { "сейчас".to_string() } else { format!("{y:.0} л.") }),
            ];
            let items: Vec<Stat> = items.into_iter().zip(nums.iter()).map(|(it, n)| it.num(n.clone())).collect();
            let resp = kit::stats(ui, &items, 3);
            let (pp, an, gdp, tr) = (f.people, f.anger, n.gdp_real, f.treasury);
            let (rv, dv, wv, lv, tv) = (f.rich, f.debt, f.wars, f.lost, f.taken);
            let last_y = f.last_war.map_or("—".to_string(), |y| format!("{y:.0}"));
            let cards: [Box<dyn Fn() -> Card>; 6] = [
                Box::new(move || Card::new("Люди").icon("people").row("людей", format!("{pp:.1} млн"))),
                Box::new(move || Card::new("Выпуск на человека").icon("coins").row("к среднему по миру", format!("×{rv:.2}")).row("выпуск в месяц", format!("{gdp:.1}"))),
                Box::new(move || Card::new("Власть").icon("crown").row("устойчивость", format!("{:.0}%", st * 100.0)).note("её расшатывают голод, недовольство, война, долги; крепят победы и спокойствие")),
                Box::new(move || Card::new("Казна").icon("econ").row("запас", format!("{tr:.1} мес. доходов")).row("долг", format!("{dv:.1} мес. доходов"))),
                Box::new(move || Card::new("Народ").icon("revolt").row("недовольство", format!("{an:.2}"))),
                Box::new(move || Card::new("Войны").icon("clash").row("лет с последней войны", last_y.clone()).row("войн за век", wv.to_string()).row("отнятых краёв", lv.to_string()).row("взятых краёв", tv.to_string())),
            ];
            for (r, c) in resp.into_iter().zip(cards.iter()) {
                r.tip(|| c());
            }
        }
        // the features
        kit::ribbon(ui, "Особенности страны", Some("medal"));
        {
            // (the spoiled window of the self-checks forgets the country's features)
            let marks = if crate::spoiled() { Vec::new() } else { spirit_marks(g, me) };
            if marks.is_empty() {
                kit::words(ui, "Ничем особенным страна не выделяется — пока.", 14.0, s.soft);
            } else {
                kit::emblems(ui, &marks, 3, 62.0);
            }
        }
        kit::words(ui, "Особенности вырастают из того, что есть в стране, и уходят вместе с причиной: их не выбрать, но можно изменить причину.", 12.5, s.soft);
    }
    if page == 1 {
        // the order of the economy
        kit::ribbon(ui, "Хозяйственный строй", Some("anvil"));
        let law = g.policy.law[me];
        let at_war = g.h.at_war;
        let choices: Vec<(&str, &str)> = LAWS.iter().map(|l| (l.name, l.icon)).collect();
        let picked = ui.scope(|ui| widgets::law(ui, &choices, law, crate::country::policy::USUAL_LAW, &|k| {
            let c = Card::new(LAWS[k].name).icon(LAWS[k].icon).text(LAWS[k].about).head(if at_war { "Что даёт на войне" } else { "Что даёт в мирное время" });
            let c = eff_card(c, &crate::country::policy::law_eff(k, at_war), &pack);
            c.note(if k == law { "действует сейчас" } else { "щёлкните — правительство перейдёт к нему (власть немного пошатнётся)" })
        }));
        // (for the window's self-checks: the row of tiles, when it is in sight from top to bottom)
        let (clip, r) = (ui.clip_rect(), picked.response.rect);
        if clip.min.y <= r.min.y && r.max.y <= clip.max.y && clip.intersects(r) {
            chrome::spot(ui.ctx(), "ecolaw", r);
        }
        if let Some(k) = picked.inner {
            act(g, &[("ecolaw", k.to_string())]);
        }
        // the programmes
        kit::ribbon(ui, "Государственные программы", Some("scroll"));
        programmes(ui, s, g);
        // loans
        kit::ribbon(ui, "Займы", Some("coins"));
        loans(ui, s, g);
    }
    if page == 2 {
        // the rival
        kit::ribbon(ui, "Что известно о сопернике", Some("binoculars"));
        let o = 1 - me;
        kit::words(ui, g.portrait_words(o), 13.5, s.ink);
        ui.add_space(3.0);
        let theirs = spirit_marks(g, o);
        if theirs.is_empty() {
            kit::words(ui, "Особенностей не видно.", 13.5, s.soft);
        } else {
            kit::emblems(ui, &theirs, 4, 44.0);
        }
        let rl = g.policy.law[o];
        kit::words(ui, format!("Его хозяйство: «{}».", LAWS[rl].name), 13.5, s.soft);
        // its stage and its course, as intelligence sees them
        if g.know.len() > g.country(o) {
            let st = g.know[g.country(o)].stage(&pack);
            kit::words(ui, format!("Стадия развития: {}.", strategy::world::progress::stage_name(&pack, st)), 13.5, s.soft);
        }
        if let Some((what, why, icon)) = g.course_words(o) {
            let r = kit::words(ui, format!("Курс его правительства: «{what}»."), 13.5, s.ink);
            r.tip(|| Card::new("Курс соседа").icon(icon).text(why.clone()).note("курс держится годами и меняется с правителем, войной или бедой"));
        }
    }
}

/// The portrait: the flag large in a frame, the name, the law, the story of the country.
fn portrait_block(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    let me = g.me;
    let country = g.h.war.th.countries[me];
    let c = g.countries[country].1;
    let col = Color32::from_rgb(c[0], c[1], c[2]);
    kit::framed(ui, Some(col), false, |ui| {
        ui.horizontal(|ui| {
            let (fr, _) = ui.allocate_exact_size(vec2(92.0, 60.0), egui::Sense::hover());
            if let Some(ch) = chrome::of(ui.ctx()) {
                let mut v = chrome::Out::new();
                super::super::hud::plate(&mut v, fr, &ch, 2.0);
                ui.painter().extend(v);
                let cloth = fr.shrink2(vec2(6.0, 6.0));
                match super::super::flags::get(ui.ctx(), country) {
                    Some(fl) => {
                        let icons = chrome::icons(ui.ctx());
                        super::super::flags::paint(ui.ctx(), ui.painter(), cloth, &fl, ch.trim, Some(&icons));
                    }
                    None => {
                        ui.painter().rect_filled(cloth, 1.0, col);
                    }
                }
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(RichText::new(g.h.home(me).name.to_uppercase()).family(egui::FontFamily::Name("head".into())).size(23.0).color(s.ink));
                ui.label(RichText::new(format!("хозяйство: {}", LAWS[g.policy.law[me]].name.to_lowercase())).size(13.0).color(s.soft));
                if g.know.len() > g.country(me) {
                    let st = g.know[g.country(me)].stage(&g.pack);
                    ui.label(RichText::new(format!("стадия развития: {}", strategy::world::progress::stage_name(&g.pack, st))).size(13.0).color(s.soft));
                }
                ui.label(RichText::new(g.date()).size(13.0).color(s.soft));
            });
        });
        ui.add_space(3.0);
        kit::words(ui, g.portrait_words(me), 14.0, s.ink);
    });
}

/// The programmes: those running as cards with their progress; those that can be begun as rows
/// to begin them.
fn programmes(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    let income = g.income(me);
    let running = g.policy.running[me].clone();
    if running.is_empty() {
        kit::words(ui, "Казна не ведёт ни одной программы.", 13.5, s.soft);
    }
    let mut stop = None;
    for r in &running {
        let p = &PLANS[r.plan];
        let resp = kit::framed(ui, Some(Color32::from_rgb(196, 160, 72)), false, |ui| {
            ui.horizontal(|ui| {
                if let Some(ch) = chrome::of(ui.ctx()) {
                    let (sr, _) = ui.allocate_exact_size(vec2(36.0, 36.0), egui::Sense::hover());
                    super::super::hud::seal(ui, &ch, sr.center(), 17.0, p.icon, ch.trim);
                }
                ui.vertical(|ui| {
                    kit::bold(ui, p.name, 15.0, s.ink);
                    let k = r.done as f32 / p.months.max(1) as f32;
                    let stage = if k < 0.34 { "только начата" } else if k < 0.67 { "в разгаре" } else { "близка к концу" };
                    let line = if r.waited > 0 && r.done < p.months { format!("{stage} · ждала денег") } else { stage.to_string() };
                    kit::words(ui, line, 12.5, s.soft);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::small(ui, "Остановить").tip(|| Card::new("Остановить программу").icon(p.icon).text("Всё, что за неё уже заплачено, пропадёт.")).clicked() {
                        stop = Some(r.plan);
                    }
                });
            });
            kit::gauge(ui, ui.available_width(), 10.0, r.done as f32 / p.months as f32, Color32::from_rgb(196, 160, 72));
        });
        let (run, done) = ((p.running)(), (p.done)());
        resp.response.tip(|| {
            let c = Card::new(p.name).icon(p.icon).text(p.about).row("в месяц", if p.cost >= 0.0 { format!("{:.0}% доходов казны ({:.2})", p.cost * 100.0, p.cost * income) } else { format!("приносит {:.0}% доходов", -p.cost * 100.0) });
            let c = eff_card(c.head("Пока идёт"), &run, &pack);
            eff_card(c.head("Когда завершена"), &done, &pack)
        });
    }
    if let Some(k) = stop {
        act(g, &[("plan_stop", k.to_string())]);
    }
    ui.add_space(4.0);
    let room = g.policy.running[me].len() < PLANS_AT_ONCE;
    let mut begin = None;
    for (k, p) in PLANS.iter().enumerate() {
        if g.policy.running[me].iter().any(|r| r.plan == k) {
            continue;
        }
        let finished = g.policy.finished[me].iter().any(|f| f.0 == k) && !matches!(p.id, "austerity" | "works");
        let resp = ui.horizontal(|ui| {
            let icons = chrome::icons(ui.ctx());
            let (ir, _) = ui.allocate_exact_size(vec2(22.0, 22.0), egui::Sense::hover());
            super::super::look::icon(&icons, p.icon, 20.0, if finished { s.soft } else { s.ink }).paint_at(ui, ir);
            // (33rd session: «Патриотическое воспитание» was cut to «Патриотическое вос…»; a long name is set smaller, down to 12.5 pt, before it is cut)
            let family = if finished { egui::FontFamily::Proportional } else { egui::FontFamily::Name("bold".into()) };
            let ink = if finished { s.soft } else { s.ink };
            let mut size = 14.0;
            while size > 12.5 && ui.painter().layout_no_wrap(p.name.to_string(), egui::FontId::new(size, family.clone()), ink).size().x > 172.0 {
                size -= 0.5;
            }
            let mut job = egui::text::LayoutJob::simple_singleline(p.name.to_string(), egui::FontId::new(size, family), ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(174.0);
            let gal = ui.painter().layout_job(job);
            let (lr, _) = ui.allocate_exact_size(vec2(176.0, 22.0), egui::Sense::hover());
            ui.painter().galley(egui::pos2(lr.min.x, lr.center().y - gal.size().y / 2.0), gal, s.ink);
            let cost = if p.cost < 0.0 { "даёт деньги" } else if p.months <= 18 { "недолго" } else if p.months <= 36 { "годы" } else { "долго" };
            ui.label(RichText::new(cost).size(12.5).color(s.soft));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if finished {
                    ui.label(RichText::new("сделано").size(12.5).color(kit::ink(s, kit::GOOD)));
                } else if kit::act_button(ui, "Начать", "play", room, false).clicked() {
                    begin = Some(k);
                }
            });
        });
        let (run, done) = ((p.running)(), (p.done)());
        resp.response.tip(|| {
            let c = Card::new(p.name).icon(p.icon).text(p.about).row("срок", format!("{} мес.", p.months)).row("в месяц", if p.cost >= 0.0 { format!("{:.0}% доходов казны ({:.2})", p.cost * 100.0, p.cost * income) } else { format!("приносит {:.0}% доходов", -p.cost * 100.0) });
            let c = eff_card(c.head("Пока идёт"), &run, &pack);
            let c = eff_card(c.head("Когда завершена"), &done, &pack);
            c.note(if finished { "уже сделано: плоды остаются навсегда" } else if !room { "одновременно — не больше двух программ" } else { "денег не хватит — программа будет ждать" })
        });
    }
    if let Some(k) = begin {
        act(g, &[("plan_go", k.to_string())]);
    }
}

/// The loans: what is owed against the most lenders give, the interest asked now; to take one, to
/// pay off.
fn loans(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let income = g.income(me);
    let debt = g.policy.debt[me];
    let most = crate::country::policy::DEBT_MOST * income;
    let rate = g.loan_rate(me);
    let amount = crate::country::policy::LOAN_MONTHS * income;
    let paid = g.policy.paid[me].0;
    let words = if debt <= 1e-6 {
        "Страна никому не должна."
    } else if debt < 6.0 * income {
        "Долг невелик: проценты казна платит без труда."
    } else if debt < 15.0 * income {
        "Долг заметный: проценты съедают часть доходов."
    } else {
        "Долг огромен: казна работает на кредиторов."
    };
    let (dm, pm) = (debt / income, paid / income * 100.0);
    ui.scope(|ui| kit::words(ui, words, 13.5, s.ink)).response.tip(|| Card::new("Долг").icon("coins").row("должны", format!("{dm:.0} мес. доходов казны")).row("платим в месяц", format!("{pm:.0}% доходов")));
    let col = if debt / most.max(1e-9) > 0.6 { kit::BAD } else { kit::MIXED };
    kit::gauge(ui, ui.available_width(), 10.0, debt / most.max(1e-9), col).tip(|| Card::new("Долг").icon("coins").row("должны", format!("{debt:.1}")).row("дадут ещё", format!("{:.1}", (most - debt).max(0.0))).row("процент сейчас", format!("{:.1}% в месяц", rate * 100.0)).note("кредиторы просят больше с шаткой власти, с воюющей страны и с той, что уже в долгах"));
    ui.add_space(3.0);
    ui.horizontal(|ui| {
        let can = debt + amount <= most;
        if kit::act_button(ui, "Взять заём", "coins", can, false).tip(|| Card::new("Внешний заём").icon("coins").row("сумма", format!("{amount:.1} ({:.0} мес. доходов)", crate::country::policy::LOAN_MONTHS)).row("под", format!("{:.1}% в месяц", rate * 100.0)).row("срок", format!("{} мес.", crate::country::policy::LOAN_TERM)).note("деньги сразу в казну; платить — каждый месяц, долг виден соседям")).clicked() {
            act(g, &[("loan", "1".into())]);
        }
        if kit::act_button(ui, "Погасить", "hand", debt > 1e-6 && g.h.home(me).treasury > 0.0, false).tip(|| Card::new("Погасить долг").icon("hand").text("Отдать кредиторам, сколько есть в казне (до четырёх пятых запаса)."))
            .clicked()
        {
            act(g, &[("repay", "1".into())]);
        }
    });
}
