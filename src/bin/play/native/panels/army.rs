//! The folder of the army: the divisions and their templates, what the country can afford
//! («Армия по карману»), the groups of armies and the staff.

use super::*;
use super::economy::pair_chart;
use super::politics::war_controls;
use super::science::stripes;
use super::super::kit;

/// A few, several, many — of divisions, in words.
fn how_many(n: usize) -> &'static str {
    match n {
        0 => "нет",
        1..=3 => "считанные",
        4..=12 => "немного",
        13..=40 => "несколько десятков",
        41..=150 => "много",
        _ => "огромное число",
    }
}

/// The war of divisions in the army's folder: how many divisions there are and where (on the line,
/// in the groups' second lines and the reserve, gathering for blows, falling back), by which
/// templates, how much is in hand to refill them and form new ones — in words and bars, the
/// numbers on hover; how to lead them.
fn divisions(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    let war = &g.h.war;
    let Some(c) = war.corps.as_ref() else { return };
    head(ui, s, if g.h.at_war { "Дивизии в войне" } else { "Дивизии" });
    let ours: Vec<&strategy::military::corps::Division> = c.divs.iter().filter(|d| d.side == me).collect();
    let n = ours.len();
    let on_line = ours.iter().filter(|d| war.th.nbr[d.at].iter().any(|&r| war.owner[r] != me)).count();
    let reserve = ours.iter().filter(|d| d.group == strategy::military::corps::NONE).count();
    let manual = ours.iter().filter(|d| d.manual).count();
    let fighting = ours.iter().filter(|d| d.busy).count();
    let fresh = ours.iter().filter(|d| d.org < 0.5).count();
    let hungry = ours.iter().filter(|d| d.supplied < 0.5).count();
    let foe = c.divs.iter().filter(|d| d.side != me).count();
    let st = if n > 0 { ours.iter().map(|d| c.strength(&pack, d)).sum::<f32>() / n as f32 } else { 0.0 };
    let org = if n > 0 { ours.iter().map(|d| d.org).sum::<f32>() / n as f32 } else { 0.0 };
    let kit = if n > 0 { ours.iter().map(|d| c.equipped(&pack, d)).sum::<f32>() / n as f32 } else { 1.0 };
    let short = ours.iter().filter(|d| c.equipped(&pack, d) < 0.8).count();
    let line = if g.h.at_war {
        format!(
            "Дивизий у нас {}; на линии фронта — {}.{}",
            how_many(n),
            if on_line * 2 >= n { "большая часть" } else if on_line * 4 >= n { "часть" } else { "меньшая часть" },
            if hungry * 5 > n { " Многим не хватает [supply]снабжения." } else { "" }
        )
    } else {
        format!("Дивизий у нас {}; в мирное время они стоят гарнизонами за границей.", how_many(n))
    };
    {
        use super::super::kit::{self, Stat};
        let tone = |good: bool| if good { kit::GOOD } else { kit::BAD };
        let items = vec![
            Stat::new("army", "Дивизий", if foe > 0 { format!("враг ≈{}", ((foe as f32 / 10.0).round() * 10.0).max(10.0)) } else { String::new() }, kit::PLAIN),
            Stat::new("march", "На линии", format!("из {n}"), kit::PLAIN),
            Stat::new("clash", "В бою", if fighting > 0 { "сейчас" } else { "" }, if fighting > 0 { kit::MIXED } else { kit::PLAIN }),
            Stat::new("people", "Численность", if st > 0.95 { "полная" } else if st > 0.8 { "почти" } else if st > 0.5 { "поредели" } else { "мало" }, tone(st > 0.8)),
            Stat::new("rally", "Организация", if org > 0.85 { "в силах" } else if org > 0.6 { "устали" } else if org > 0.35 { "измотаны" } else { "разбиты" }, tone(org > 0.6)),
            Stat::new("crate", "Оснащение", if kit >= 0.97 { "полное" } else if kit >= 0.8 { "почти" } else if kit >= 0.5 { "неполное" } else { "нехватка" }, tone(kit >= 0.8)),
        ];
        let nums = [n.to_string(), on_line.to_string(), fighting.to_string(), format!("{:.0}%", st * 100.0), format!("{:.0}%", org * 100.0), format!("{:.0}%", kit.min(1.0) * 100.0)];
        let items: Vec<Stat> = items.into_iter().zip(nums.iter()).map(|(it, n)| it.num(n.clone())).collect();
        let r = kit::stats(ui, &items, 3);
        let tips = [
            "дивизий всего",
            "дивизий в провинциях у границы с врагом",
            "дивизий, что сражались сегодня",
            "сколько людей из полного шаблона",
            "сколько из них ещё в силах воевать",
            "сколько снаряжения из того, что даёт норма",
        ];
        let short_n = short;
        for (i, resp) in r.into_iter().enumerate() {
            let (t, v) = (items[i].title.clone(), nums[i].clone());
            resp.tip(|| {
                let c = Card::new(t).icon(items[i].icon.clone()).row("сейчас", v);
                let c = if i == 5 { c.tone("недооснащённых дивизий", short_n.to_string(), if short_n > 0 { Tone::Bad } else { Tone::Plain }) } else { c };
                c.note(tips[i])
            });
        }
    }
    ui.scope(|ui| say(ui, s, &line)).response.tip(|| {
        Card::new("Дивизии")
            .icon("army")
            .row("наших", n.to_string())
            .row("на линии фронта", on_line.to_string())
            .row("в бою сегодня", fighting.to_string())
            .row("в резерве ставки", reserve.to_string())
            .row("ведёте сами", manual.to_string())
            .row("набирают организацию", fresh.to_string())
            .tone("без снабжения", hungry.to_string(), if hungry * 5 > n { Tone::Bad } else { Tone::Plain })
            .row("у врага (по разведке)", format!("около {}", ((foe as f32 / 10.0).round() * 10.0).max(10.0)))
    });
    // the recruits: new divisions drilling behind the line
    let recruits: Vec<f32> = c.divs.iter().filter(|d| d.side == me && d.drill < 1.0).map(|d| d.drill).collect();
    if !recruits.is_empty() {
        let (n, mean) = (recruits.len(), recruits.iter().sum::<f32>() / recruits.len() as f32);
        ui.scope(|ui| say(ui, s, "У знамени сбора учатся новые [division]{дивизии}: генерал возьмёт их, когда выучатся.")).response.tip(|| {
            Card::new("Новобранцы")
                .icon("army")
                .row("дивизий учится", n.to_string())
                .row("подготовка в среднем", format!("{:.0}%", mean * 100.0))
                .row("учатся", format!("{:.0} дней за линией фронта", strategy::military::corps::TRAIN_DAYS))
                .note("можно отправить раньше — выбрать и правой кнопкой; необученные бьются слабее")
        });
    }
    ui.add_space(3.0);
    say(ui, s, if g.h.at_war { "[division]{Стопку} — щелчком или рамкой, затем правой кнопкой — куда идти (по врагу — атака; Shift — и дальше)." } else { "[division]{Стопку} — щелчком или рамкой, затем правой кнопкой — куда идти по своей земле." });
}

/// A template's battalions in words: so many of each kind, with its kit when not standard.
fn shape_words(pack: &strategy::pack::Pack, sh: &strategy::military::corps::Shape) -> String {
    sh.units
        .iter()
        .enumerate()
        .filter(|(_, x)| **x > 0.0)
        .map(|(u, x)| {
            let k = sh.norm_of(u);
            if pack.units[u].norms && k != 1.0 {
                format!("{:.0} {} ({})", x / strategy::military::corps::BATTALION, pack.units[u].name, strategy::military::doctrine::norm::word(k))
            } else {
                format!("{:.0} {}", x / strategy::military::corps::BATTALION, pack.units[u].name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The army is a matter of the economy: what share of the state's spending it gets, what the
/// factories and the call-up give it a month, what holds it back (men or equipment), the kit in
/// store against the men, the trophies; and how the staff shapes its divisions for what the
/// country can afford — in words, numbers on hover.
fn purse(ui: &mut egui::Ui, s: &Skin, g: &Game) {
    let me = g.me;
    let pack = &g.pack;
    let war = &g.h.war;
    let Some(c) = war.corps.as_ref() else { return };
    let side = &war.sides[me];
    let n = g.h.home(me);
    head(ui, s, "Армия по карману");
    // the money: the army's share of the state's spending
    let share = n.mil_share;
    let money = match share {
        x if x < 0.15 => "Армии идёт малая доля трат казны",
        x if x < 0.35 => "Армии идёт заметная доля трат казны",
        x if x < 0.6 => "Армии идёт большая доля трат казны",
        _ => "Почти все траты казны идут армии",
    };
    // what the factories gave last month: battalions, and divisions of the pattern built
    let made: f32 = g.h.last_made[me].iter().sum::<f32>() / strategy::military::corps::BATTALION;
    let size = c.wants[me].iter().enumerate().filter(|x| *x.1 > 0.0).filter_map(|x| c.shapes[me].get(x.0)).map(|x| x.battalions()).fold(0.0f32, f32::max).max(1.0);
    let divs = made / size;
    let pace = match divs {
        x if x < 0.5 => "заводы почти ничего не дают",
        x if x < 3.0 => "заводы дают по нескольку дивизий в месяц",
        x if x < 15.0 => "заводы дают около десятка дивизий в месяц",
        _ => "заводы дают десятки дивизий в месяц",
    };
    let line = format!("{money}; {pace}.");
    ui.scope(|ui| say(ui, s, &line)).response.tip(|| {
        Card::new("Армия по карману")
            .icon("coins")
            .row("доля трат казны", format!("{:.0}%", share * 100.0))
            .row("батальонов за месяц", format!("{made:.0}"))
            .row("дивизий за месяц", format!("≈{divs:.1}"))
            .note("армию делает экономика: заводы — снаряжение, народ — солдат, казна платит обоим")
    });
    // what holds it back: men to call up, the goods of its kit
    let draftable = n.draftable();
    let crew = pack.war.as_ref().map_or(10.0, |w| w.crew);
    let men_month: f32 = g.h.last_made[me].iter().zip(&pack.units).map(|(m, u)| m * u.men * crew / 1000.0).sum();
    let rel = |i: usize| n.price[i] / (pack.goods[i].price * n.level).max(1e-9);
    let mut kit_goods: Vec<usize> = Vec::new();
    for (u, unit) in pack.units.iter().enumerate() {
        if side.plan.get(u).copied().unwrap_or(0.0) > 0.03 {
            for &(i, _) in &unit.equip {
                if !kit_goods.contains(&i) {
                    kit_goods.push(i);
                }
            }
        }
    }
    let dear: Vec<String> = kit_goods.iter().filter(|&&i| rel(i) > 1.3).map(|&i| pack.goods[i].name.clone()).collect();
    let men_short = draftable < 6.0 * men_month.max(1e-6);
    let hold = if men_short {
        "Людей на призыв почти не осталось — армия упирается в людей.".to_string()
    } else if !dear.is_empty() {
        format!("Армию сдерживает снаряжение: стране дорого — {}.", dear.join(", "))
    } else {
        "Людей и снаряжения хватает: армия растёт, сколько дают деньги.".to_string()
    };
    ui.scope(|ui| say(ui, s, &hold)).response.tip(|| {
        let mut card = Card::new("Что сдерживает армию").icon("hazard").row("можно призвать ещё", format!("{draftable:.2} млн")).row("призвано за месяц", format!("{:.0} тыс.", men_month * 1000.0));
        for &i in &kit_goods {
            card = card.tone(pack.goods[i].name.clone(), format!("×{:.1} обычной цены", rel(i)), if rel(i) > 1.3 { Tone::Bad } else { Tone::Plain });
        }
        card
    });
    // the kit in store against the men in store; trophies
    let men_pool: f32 = c.pool[me].iter().sum();
    let kit_pool: f32 = c.kit[me].iter().sum();
    let want_kit: f32 = (0..c.pool[me].len()).map(|u| c.pool[me][u] * side.norm_of(u)).sum();
    let trophies: f32 = c.trophies[me].iter().sum();
    let left: f32 = c.abandoned[me].iter().sum();
    let store = if men_pool < 0.05 {
        "На складах пусто: всё идёт на пополнение дивизий.".to_string()
    } else if kit_pool < 0.7 * want_kit {
        "Людей на складах больше, чем оружия: новые дивизии выйдут недооснащёнными.".to_string()
    } else {
        "Снаряжения на складах хватает на призванных.".to_string()
    };
    let store = if trophies > 0.05 { format!("{store} Берём трофеи у врага.") } else { store };
    ui.scope(|ui| say(ui, s, &store)).response.tip(|| {
        Card::new("Склады армии")
            .icon("crate")
            .row("людей ждут дивизий", format!("{:.0} батальонов", men_pool / strategy::military::corps::BATTALION))
            .row("снаряжения", format!("{:.0} батальонов", kit_pool / strategy::military::corps::BATTALION))
            .row("трофеев взято", format!("{:.1} батальона", trophies / strategy::military::corps::BATTALION))
            .tone("брошено при отходах", format!("{:.1} батальона", left / strategy::military::corps::BATTALION), if left > trophies { Tone::Bad } else { Tone::Plain })
    });
    // how the staff shapes the divisions for what the country can afford, and why
    if let Some(sh) = c.shapes[me].iter().rev().find(|x| x.name.starts_with("Дивизия генштаба")).filter(|_| c.by_plan[me]) {
        let front = war.front(me).len().max(1) as f32;
        let bats: f32 = (c.pool[me].iter().sum::<f32>() + c.divs.iter().filter(|d| d.side == me).map(|d| d.has.iter().sum::<f32>()).sum::<f32>()) / strategy::military::corps::BATTALION;
        let per = bats / front;
        let b = sh.battalions();
        let size = if b <= strategy::military::corps::FEWEST + 1.0 {
            "мелкие: армии мало на длинную границу — дивизий нужно много, и каждая невелика"
        } else if b >= war.most_battalions - 1.0 {
            "крупные: армия велика против границы"
        } else {
            "среднего размера: армии на границу в самый раз"
        };
        let ks: Vec<f32> = (0..pack.units.len()).filter(|&u| pack.units[u].norms && sh.units[u] > 0.0).map(|u| sh.norm_of(u)).collect();
        let mean = if ks.is_empty() { 1.0 } else { ks.iter().sum::<f32>() / ks.len() as f32 };
        let kit = if mean < 0.9 {
            "оснащение лёгкое — снаряжение стране дороже людей"
        } else if mean > 1.1 {
            "оснащение усиленное — люди стране дороже снаряжения"
        } else {
            "оснащение штатное"
        };
        let sup: Vec<&str> = sh.support.iter().filter_map(|&k| pack.support.get(k)).map(|x| x.name.as_str()).collect();
        let companies = match sup.len() {
            0 => "рот поддержки нет: дивизии слишком малы, чтобы роты окупались".to_string(),
            k if k == pack.support.len() => "все роты поддержки".to_string(),
            _ => format!("роты: {}", sup.join(", ")),
        };
        // (an army beyond what its front holds keeps the rest in its stores)
        let divs = c.divs.iter().filter(|d| d.side == me).count() as f32;
        let full = divs >= war.most_per_front * front - 0.5;
        let stores = c.pool[me].iter().sum::<f32>() / strategy::military::corps::BATTALION / b.max(1.0);
        let mut line = format!("Генштаб строит дивизии {size}; {kit}; {companies}.");
        if full && stores >= 1.0 {
            line += " Фронт больше дивизий не вместит: снаряжение новых ждёт на складах и идёт на пополнение.";
        }
        let name = sh.name.clone();
        let words = shape_words(pack, sh);
        ui.scope(|ui| say(ui, s, &line)).response.tip(|| {
            Card::new(name)
                .icon("army")
                .row("батальонов", format!("{b:.0}"))
                .row("батальоны", words.clone())
                .row("армии на провинцию границы", format!("{per:.0} батальонов"))
                .row("дивизий на провинцию фронта", format!("{:.1} (не больше {:.0})", divs / front, war.most_per_front))
                .row("на складах", format!("на {stores:.0} таких дивизий"))
                .note("генштаб хочет на каждую провинцию фронта несколько дивизий — линию, смену и резерв; армия большая — дивизии крупнее, малая — мельче")
                .note("дивизия строится вокруг линии: орудий за ней не больше, чем войск в ней, и пехота — основа тех, кто держит линию")
                .note("оснащение он выбирает штабными играми по ценам своей страны; роты — по тому, чего не хватало, пока они окупаются")
        });
    }
}

/// The law of the draft: five laws as tiles (the custom one of the times marked), how many serve
/// now and how many more may be called; a click sends the choice to the government.
fn draft_law(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    use strategy::economy::nation::LAWS;
    let me = g.me;
    let n = g.h.home(me);
    let ppl = n.people().max(1e-6);
    let serving: f32 = n.drafted.iter().sum();
    let more = n.draftable();
    let now = g.law(me);
    let custom = g.custom_law();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        ui.label(RichText::new("Закон о призыве:").size(15.0).color(s.soft));
        ui.label(RichText::new(LAWS[now].name).family(egui::FontFamily::Name("bold".into())).size(15.0).color(s.ink));
    });
    let choices: Vec<(&str, &str)> = LAWS.iter().map(|l| (l.short, l.icon)).collect();
    let row = ui.scope(|ui| widgets::law(ui, &choices, now, custom, &|k| {
        let l = &LAWS[k];
        Card::new(l.name)
            .icon(l.icon)
            .sub("закон о призыве")
            .row("служат не больше", format!("{:.1}% каждой группы", l.cap * 100.0))
            .row("жалованье", format!("{:.0}% заработка", l.pay * 100.0))
            .row("служат сейчас", format!("{:.1}% людей", serving / ppl * 100.0))
            .text("Чем шире призыв, тем больше солдат и тем они дешевле армии — но они уходят с работы, а их семьи живут на жалованье.")
            .note(if k == now { "выбрано" } else if k == custom { "так велит обычай; щёлкните — решение уйдёт в правительство" } else { "щёлкните — решение уйдёт в правительство" })
    }));
    // (for the window's self-checks: the row of tiles, when it is in sight from top to bottom)
    let (clip, r) = (ui.clip_rect(), row.response.rect);
    if clip.min.y <= r.min.y && r.max.y <= clip.max.y && clip.intersects(r) {
        chrome::spot(ui.ctx(), "draft", r);
    }
    if let Some(k) = row.inner {
        act(g, &[("draft", k.to_string())]);
    }
    // (the numbers on the card: in the folder, words)
    let share = serving / ppl;
    let under = match share {
        x if x < 0.02 => "Под ружьём немногие",
        x if x < 0.06 => "Под ружьём заметная часть народа",
        x if x < 0.12 => "Под ружьём едва ли не каждый десятый",
        _ => "Под ружьём почти все, кто может",
    };
    let room = if more < 0.1 * serving.max(1e-6) { "призывать почти некого — закон упёрся в потолок" } else { "призвать можно ещё многих" };
    let line = format!("{under}; {room}.");
    ui.scope(|ui| say(ui, s, &line)).response.tip(|| Card::new("Под ружьём").icon("army").row("служат", format!("{serving:.2} млн")).row("можно призвать ещё", format!("{more:.2} млн")).note("потолок задаёт закон о призыве"));
}

/// The army: how big, what it is, what is built (the staff's or the player's templates), the
/// war's buttons, the depots, the front's groups.
/// The pages of the army's folder (28th session: the armies and their orders stood at the foot of a
/// scroll of four screens).
pub const PAGES: [&str; 5] = ["Армии", "Шаблоны", "Снабжение", "Бюджет", "Состав"];

/// The army's folder, page by page: the armies (the divisions in figures, each army with its
/// general, its divisions and its orders, where new divisions gather, war and peace); the
/// templates; the supply (depots, lines); the army's money (what it costs, the draft, the peace
/// establishment, the war's cost); what the army is made of.
#[allow(clippy::too_many_arguments)]
pub fn army(ui: &mut egui::Ui, s: &Skin, g: &mut Game, chosen: &mut Option<usize>, build: &mut super::super::map::Build, icons: &std::collections::HashMap<String, std::sync::Arc<[u8]>>, edit: &mut Option<super::super::designer::Edit>, page: usize) {
    match page {
        1 => templates(ui, s, g, icons, edit),
        2 => supply(ui, s, g, build),
        3 => money(ui, s, g),
        4 => makeup(ui, s, g),
        _ => armies(ui, s, g, chosen, build),
    }
}

/// The page «Армии».
fn armies(ui: &mut egui::Ui, s: &Skin, g: &mut Game, chosen: &mut Option<usize>, build: &mut super::super::map::Build) {
    let me = g.me;
    if g.h.war.corps.is_some() {
        divisions(ui, s, g);
    }
    // the front: the groups, their generals and orders
    if let Some(groups) = g.h.war.command[me].clone() {
        head(ui, s, "Армии");
        say(ui, s, "Армия — генерал и его дивизии (хорошо ведёт до 24). Щёлкните армию — здесь, карточкой внизу или клавишей с её номером: над карточками появятся её приказы — линия фронта (F, протянуть вдоль границы), наступление (A), «Исполнить» (Enter).");
        ui.add_space(4.0);
        // (each army's divisions in figures: how many, their men, order and kit)
        let figures: Vec<(usize, f32, f32, f32)> = (0..groups.len())
            .map(|k| {
                let Some(c) = g.h.war.corps.as_ref() else { return (0, 0.0, 0.0, 0.0) };
                let ds: Vec<&strategy::military::corps::Division> = c.divs.iter().filter(|d| d.side == me && d.group == k).collect();
                let n = ds.len();
                if n == 0 {
                    return (0, 0.0, 0.0, 0.0);
                }
                let m = n as f32;
                (n, ds.iter().map(|d| c.strength(&g.pack, d)).sum::<f32>() / m, ds.iter().map(|d| d.org).sum::<f32>() / m, ds.iter().map(|d| c.equipped(&g.pack, d).min(1.0)).sum::<f32>() / m)
            })
            .collect();
        let sup = g.h.war.supply(me);
        let lab = g.h.war.group_of(me);
        let ch = chrome::of(ui.ctx());
        for (k, gr) in groups.iter().enumerate() {
            let on = *chosen == Some(k);
            let gen = gr.general.as_ref().map_or(String::new(), |x| x.name.clone());
            let order = match gr.order {
                Order::Hold => "держит участок",
                Order::Deep => "держит в глубину",
                Order::Attack(_) => "наступает",
            };
            let (mut a, mut b) = (0.0f32, 0.0f32);
            for (q, &x) in lab.iter().enumerate() {
                if x == k {
                    let n: f32 = g.h.war.troops[me][q].iter().sum();
                    a += sup[q] * n;
                    b += n;
                }
            }
            let fed = if b > 0.0 { a / b } else { g.h.fed[me] };
            let hue = super::super::map::HUES[k % 8];
            let has_general = gr.general.is_some();
            let groups_n = groups.len();
            let mut disbanded = false;
            let back = chrome::hold(ui);
            let resp = egui::Frame::none().inner_margin(egui::Margin { left: 12.0, right: 8.0, top: 7.0, bottom: 7.0 }).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if let Some(img) = portrait(&g.face_of(&gen, "general"), 40.0) {
                        let r = ui.add(img).rect;
                        if let Some(ch) = &ch {
                            ui.painter().rect_stroke(r.expand(1.0), 2.0, egui::Stroke::new(1.5, ch.trim));
                        }
                    }
                    ui.vertical(|ui| {
                        let (dn, st, org, eq) = figures[k];
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(crate::words::group_name(k)).family(egui::FontFamily::Name("head".into())).size(16.0));
                            ui.label(RichText::new(format!("{dn}/24")).family(egui::FontFamily::Name("bold".into())).size(14.5).color(if dn > 24 { kit::BAD } else { s.ink }));
                        });
                        ui.label(RichText::new(if gen.is_empty() { "без генерала".to_string() } else { gen.clone() }).family(egui::FontFamily::Name("bold".into())).size(14.0));
                        ui.label(RichText::new(format!("{order}{}", if gr.staff { " · ведёт генштаб" } else { " · ведёте вы" })).size(13.5));
                        gauges(ui, s, &[("людей", st, kit::GOOD), ("орг.", org, Color32::from_rgb(214, 176, 60)), ("осн.", eq, Color32::from_rgb(90, 130, 170)), ("снабж.", fed, Color32::from_rgb(150, 120, 80))]);
                        if on {
                            ui.add_space(3.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                                if widgets::small(ui, "Держать").tip(|| Card::new("Держать участок").icon("shield").text("Группа держит свой участок фронта, не наступая.")).clicked() {
                                    act(g, &[("grp", k.to_string()), ("order", "hold".into())]);
                                }
                                if widgets::small(ui, "В глубину").tip(|| Card::plain("Глубокая оборона", super::super::terms::note("deep").map_or("", |n| n.1)).icon("wire")).clicked() {
                                    act(g, &[("grp", k.to_string()), ("order", "deep".into())]);
                                }
                                if !gr.staff && widgets::small(ui, "Генштабу").tip(|| Card::new("Отдать генштабу").icon("compass").text("Группу снова поведёт генштаб: сам решит, держать или бить.")).clicked() {
                                    act(g, &[("grp", k.to_string()), ("order", "staff".into())]);
                                }
                                // (28th session: the generals and the armies themselves)
                                let pick_id = egui::Id::new(("general-pick", k));
                                let picking = ui.ctx().data(|d| d.get_temp::<bool>(pick_id)).unwrap_or(false);
                                if widgets::small(ui, if picking { "Сменить генерала ▲" } else { "Сменить генерала" }).tip(|| Card::new("Сменить генерала").icon("medal").text("Список генералов резерва: их прошлое и дела. Назначенный примет армию, нынешний уйдёт в резерв.").note("о таланте и смелости скажут только дела")).clicked() {
                                    ui.ctx().data_mut(|d| d.insert_temp(pick_id, !picking));
                                }
                                if has_general && widgets::small(ui, "Снять").tip(|| Card::new("Снять генерала").icon("stop").text("Генерал уйдёт в резерв; армию поведёт штаб без командующего — каждый сам по себе, без его взгляда на участок.")).clicked() {
                                    act(g, &[("grp", k.to_string()), ("general_off", "1".into())]);
                                }
                                if groups_n > 1 && widgets::small(ui, "Расформировать").tip(|| Card::new("Расформировать армию").icon("staff").text("Её дивизии уйдут в резерв ставки, участок — соседней армии, генерал — в резерв генералов. Номера следующих армий сдвинутся.")).clicked() {
                                    act(g, &[("grp", k.to_string()), ("disband", "1".into())]);
                                    disbanded = true;
                                }
                            });
                        }
                    });
                });
            });
            let r = resp.response.rect;
            if let Some(ch) = &ch {
                let mut v = chrome::Out::new();
                ch.lay(&mut v, r, &ch.paper, 2.0);
                ch.gloss(&mut v, r, Color32::from_white_alpha(if on { 20 } else { 45 }), Color32::from_black_alpha(if on { 45 } else { 30 }));
                ch.bevel(&mut v, r, !on, 1.5);
                v.push(egui::Shape::rect_filled(egui::Rect::from_min_size(r.min, vec2(5.0, r.height())), 1.0, hue));
                v.push(egui::Shape::rect_stroke(r, 2.0, egui::Stroke::new(if on { 2.2 } else { 1.0 }, if on { ch.trim } else { Color32::from_black_alpha(130) })));
                chrome::under(ui, back, v);
            }
            let fights = gr.general.as_ref().map(|x| (x.won, x.lost, x.took));
            let rr = resp.response.interact(egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).tip(|| {
                let mut c = Card::new(crate::words::group_name(k)).face(g.face_of(&gen, "general")).sub(gen.clone()).row("приказ", order).tone("снабжение", format!("{:.0}%", fed * 100.0), if fed > 0.85 { Tone::Good } else { Tone::Bad });
                if let Some((w, l, t)) = fights {
                    c = c.row("побед", w.to_string()).row("поражений", l.to_string()).row("взял провинций", t.to_string());
                }
                c.note(if on { "выбрана: правой кнопкой по карте — приказ" } else { "щёлкните — выбрать" })
            });
            if rr.clicked() {
                *chosen = if on { None } else { Some(k) };
            }
            ui.add_space(4.0);
            // the reserve of generals, to choose one for this army
            if on && ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new(("general-pick", k)))).unwrap_or(false) {
                let spare = g.spare.clone();
                ui.indent(("spare", k), |ui| {
                    kit::words(ui, if spare.is_empty() { "Резерв генералов пуст." } else { "Резерв генералов:" }, 13.5, s.soft);
                    for (i, x) in spare.iter().enumerate() {
                        let record = if x.won + x.lost == 0 { "в боях этой войны не был".to_string() } else { format!("побед {}, поражений {}, взял провинций {}", x.won, x.lost, x.took) };
                        let past = x.past.iter().rev().take(2).cloned().collect::<Vec<_>>().join("; ");
                        let body = if past.is_empty() { record.clone() } else { format!("{record}\n{past}") };
                        if person(ui, s, g, &x.name, "general", false, &body) {
                            act(g, &[("grp", k.to_string()), ("general", i.to_string())]);
                            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("general-pick", k)), false));
                        }
                    }
                    if spare.len() < 12 && widgets::small(ui, "Вызвать ещё одного в резерв").tip(|| Card::new("Новый человек в резерв").icon("medal").text("Генштаб назовёт полковника, которому доверяет. Каков он — покажут дела.")).clicked() {
                        act(g, &[("general_new", "1".into())]);
                    }
                });
                ui.add_space(4.0);
            }
            if disbanded {
                *chosen = None;
                break;
            }
        }
    }
    // where the new divisions gather (and, in a war, the group they join: the bar of a group's plans)
    if let Some(c) = g.h.war.corps.as_ref() {
        head(ui, s, "Новые дивизии");
        let at = c.muster[me].filter(|&q| g.h.war.owner[q] == me);
        let to = c.muster_group[me];
        ui.label(match (at, to) {
            (Some(_), Some(k)) => format!("Собираются у знамени на карте и идут в {}.", crate::words::group_into(k)),
            (Some(_), None) => "Собираются у знамени на карте; группу им даёт генштаб.".to_string(),
            (None, Some(k)) => format!("Собираются у столицы или склада ближе к фронту и идут в {}.", crate::words::group_into(k)),
            (None, None) => "Собираются у столицы или склада ближе к фронту; группу им даёт генштаб.".to_string(),
        });
        let w = ui.available_width();
        let on = *build == super::super::map::Build::Muster;
        if widgets::plate(ui, if on { "Щёлкните по своей провинции… (отмена)" } else { "Место сбора…" }, w, on, true)
            .tip(|| Card::new("Место сбора").icon("flag").text("Новые дивизии будут собираться в выбранной вами провинции, а не у столицы. Группу, куда они пойдут, выбирают в полосе планов группы («Пополнение»)."))
            .clicked()
        {
            *build = if on { super::super::map::Build::None } else { super::super::map::Build::Muster };
        }
        if at.is_some() && widgets::plate(ui, "Собирать, где решит генштаб", w, false, true).tip(|| Card::new("Место сбора — генштабу").icon("staff").text("Новые дивизии снова собираются у столицы или у склада ближе к фронту.")).clicked() {
            act(g, &[("muster", "-".into())]);
        }
    }
    head(ui, s, "Война и мир");
    war_controls(ui, s, g);
}

/// Small gauges in a row, each its word, its bar and its percent (an army's men, order, kit,
/// supply).
fn gauges(ui: &mut egui::Ui, s: &Skin, items: &[(&str, f32, Color32)]) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), egui::Sense::hover());
    let cell = w / items.len().max(1) as f32;
    let p = ui.painter();
    for (i, (word, x, col)) in items.iter().enumerate() {
        let c = egui::Rect::from_min_size(egui::pos2(r.min.x + i as f32 * cell, r.min.y), vec2(cell - 6.0, r.height()));
        let x = x.clamp(0.0, 1.0);
        p.text(c.left_top(), egui::Align2::LEFT_TOP, *word, egui::FontId::proportional(12.0), s.soft);
        p.text(c.right_top(), egui::Align2::RIGHT_TOP, format!("{:.0}%", x * 100.0), egui::FontId::new(12.0, egui::FontFamily::Name("bold".into())), if x < 0.5 { kit::BAD } else { s.ink });
        let bar = egui::Rect::from_min_max(egui::pos2(c.min.x, c.max.y - 7.0), egui::pos2(c.max.x, c.max.y - 2.0));
        p.rect_filled(bar, 1.5, Color32::from_black_alpha(70));
        p.rect_filled(egui::Rect::from_min_size(bar.min, vec2(bar.width() * x, bar.height())), 1.5, *col);
    }
}

/// The page «Шаблоны».
fn templates(ui: &mut egui::Ui, s: &Skin, g: &mut Game, icons: &std::collections::HashMap<String, std::sync::Arc<[u8]>>, edit: &mut Option<super::super::designer::Edit>) {
    let me = g.me;
    head(ui, s, "Дивизии");
    let fixed = g.h.war.sides[me].fixed;
    match widgets::switch(ui, &["Решает генштаб", "По моим шаблонам"], if fixed { 1 } else { 0 }) {
        Some(0) => act(g, &[("ctrl", "staff".into())]),
        Some(_) => act(g, &[("ctrl", "me".into())]),
        None => {}
    }
    ui.add_space(4.0);
    super::super::designer::section(ui, s, g, icons, edit);
}

/// The page «Снабжение».
fn supply(ui: &mut egui::Ui, s: &Skin, g: &mut Game, build: &mut super::super::map::Build) {
    let me = g.me;
    if g.h.at_war {
        let fed = g.h.fed[me];
        ui.add_space(3.0);
        ui.scope(|ui| {
            say(ui, s, if fed > 0.9 { "[supply]Снабжение в порядке." } else if fed > 0.6 { "[supply]Снабжения не хватает." } else { "Войска голодают без патронов и хлеба: [supply]снабжение не доходит." });
        })
        .response
        .tip(|| Card::new("Снабжение").icon("crate").tone("доходит до войск", format!("{:.0}%", fed * 100.0), if fed > 0.9 { Tone::Good } else { Tone::Bad }));
    }
    head(ui, s, "Склады снабжения");
    let have = g.h.war.hubs[me].iter().filter(|&&q| g.h.war.owner[q] == me).count();
    ui.label(match have {
        0 => "Складов нет.".to_string(),
        1 => "Один склад.".to_string(),
        _ => "Складов несколько.".to_string(),
    });
    let w = ui.available_width();
    let depot = *build == super::super::map::Build::Depot;
    if widgets::plate(ui, if depot { "Щёлкните по своей провинции… (отмена)" } else { "Построить склад…" }, w, depot, true).tip(|| Card::new("Склад снабжения").icon("crate").text("Снабжение пойдёт и от склада, если до него можно дойти от столицы.").row("складов", have.to_string())).clicked() {
        *build = if depot { super::super::map::Build::None } else { super::super::map::Build::Depot };
    }
    // the lines of supply: a railway (in the ages of steam) or a road through one's own land
    head(ui, s, "Линии снабжения");
    say(ui, s, "Снабжение идёт от столицы и складов шаг за шагом по провинциям. [line_supply]{Линия снабжения} к фронту — и войска там сытее.");
    ui.add_space(3.0);
    let mine = g.state.works.iter().filter(|x| x.side == me).count();
    let busy = g.state.works.iter().filter(|x| x.side == me && !x.finished()).count();
    if mine > 0 {
        ui.label(format!("Линий проложено: {}{}", mine - busy, if busy > 0 { format!(", прокладывается: {busy}") } else { String::new() }));
    }
    let kinds: Vec<bool> = if g.has_rails() { vec![true, false] } else { vec![false] };
    for rail in kinds {
        let on = matches!(*build, super::super::map::Build::Line { rail: r, .. } if r == rail);
        let name = g.line_name(rail);
        let (per, _) = g.line_cost(me, rail, 1);
        let label = if on { "Щёлкните: откуда, затем куда… (отмена)".to_string() } else { format!("Проложить: {name}…") };
        if widgets::plate(ui, &label, w, on, true)
            .tip(|| Card::new(name).icon("crate").text("Щёлкните по своей провинции, откуда начать, затем — куда: линия пройдёт кратчайшим путём по вашей земле.").row("провинция стоит", format!("{per:.2}")).row("в месяц", if rail { "1 провинция" } else { "2 провинции" }).row("шаг снабжения", if rail { "0.4" } else { "0.65" }))
            .clicked()
        {
            *build = if on { super::super::map::Build::None } else { super::super::map::Build::Line { rail, from: None } };
        }
    }
}

/// The page «Бюджет»: what the army costs and what it is paid from.
fn money(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    ui.add_space(3.0);
    purse(ui, s, g);
    ui.add_space(3.0);
    draft_law(ui, s, g);
    ui.add_space(3.0);
    super::military::budget(ui, s, g);
    super::military::establishment(ui, s, g);
    super::military::war_cost(ui, s, g);
}

/// The page «Состав»: what the army is made of, by kind and by template.
fn makeup(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    let c = &g.h.war.sides[me];
    let l = lines(&pack, &c.army.count, g.border);
    let count = c.army.count.iter().sum::<f32>();
    ui.scope(|ui| {
        say(ui, s, match l {
            l if l < 0.5 => "Армии почти нет — граница открыта.",
            l if l < 1.5 => "Армия тонкой [line]линией вдоль границы.",
            l if l < 3.0 => "Армия держит границу в одну-две [line]линии.",
            _ => "Армия велика — граница прикрыта в несколько [line]линий.",
        });
    })
    .response
    .tip(|| Card::new("Армия").icon("army").row("дивизий", format!("{count:.0}")).row("линий вдоль границы", format!("{l:.1}")));
    ui.add_space(4.0);
    let sh = shares(&pack, &c.army.count);
    let plan = c.plan.clone();
    let foe_mix = c.foe_mix.clone();
    pair_chart(ui, s, g, "Армии по месяцам", "дивизий в армии", &|r, k| r.army[k], &|v| format!("{v:.0} дивизий"));
    // (what our army is made of, month by month: the kinds of troops one over another)
    {
        let rows: Vec<&crate::game::Row> = g.rows.iter().rev().take(240).collect::<Vec<_>>().into_iter().rev().collect();
        let months: Vec<u32> = rows.iter().map(|r| r.month).collect();
        let kinds: Vec<usize> = (0..pack.units.len()).filter(|&u| rows.iter().any(|r| r.mix.get(u).copied().unwrap_or(0.0) > 0.0)).collect();
        let series: Vec<(String, Color32, Vec<f32>)> = kinds.iter().map(|&u| (pack.units[u].name.clone(), super::super::designer::KIND_COLORS[u % super::super::designer::KIND_COLORS.len()], rows.iter().map(|r| r.mix.get(u).copied().unwrap_or(0.0) * pack.units[u].cost).collect())).collect();
        ui.label(RichText::new("СОСТАВ ПО МЕСЯЦАМ".to_string()).family(egui::FontFamily::Name("head".into())).size(13.0).color(s.soft));
        let card = |i: usize| {
            let r = rows[i];
            let mut c = Card::new("Состав армии").icon("army").sub(crate::words::date_words(r.month));
            let total: f32 = kinds.iter().map(|&u| r.mix.get(u).copied().unwrap_or(0.0) * pack.units[u].cost).sum::<f32>().max(1e-9);
            for &u in &kinds {
                let v = r.mix.get(u).copied().unwrap_or(0.0) * pack.units[u].cost;
                c = c.row(pack.units[u].name.clone(), format!("{:.0}%", v / total * 100.0));
            }
            c.note("доли по ценности войск")
        };
        widgets::chart(ui, &series, &months, 74.0, true, &card);
        ui.add_space(4.0);
    }
    stripes(ui, s, g, &sh, "Состав");
    stripes(ui, s, g, &plan, "Генштаб хочет");
    stripes(ui, s, g, &foe_mix, "Враг сквозь туман");
    // (the kinds' colours named once, under the bars)
    let used: Vec<(String, Color32)> = (0..pack.units.len())
        .filter(|&u| sh.get(u).copied().unwrap_or(0.0) + plan.get(u).copied().unwrap_or(0.0) + foe_mix.get(u).copied().unwrap_or(0.0) > 0.003)
        .map(|u| (pack.units[u].name.clone(), super::super::designer::KIND_COLORS[u % super::super::designer::KIND_COLORS.len()]))
        .collect();
    widgets::legend(ui, s, &used);
    by_template(ui, s, g);
}

/// The divisions by template (a bar each, as long as its share), and what is in hand to refill
/// them and to form new ones.
fn by_template(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    let war = &g.h.war;
    let Some(c) = war.corps.as_ref() else { return };
    let ours: Vec<&strategy::military::corps::Division> = c.divs.iter().filter(|d| d.side == me).collect();
    let n = ours.len();
    head(ui, s, "Дивизии по шаблонам");
    ui.add_space(4.0);
    // by template: a bar each, as long as its share of the divisions
    let shapes = &c.shapes[me];
    let mut by: Vec<(usize, usize)> = (0..shapes.len()).map(|k| (k, ours.iter().filter(|d| d.shape == k).count())).filter(|x| x.1 > 0).collect();
    by.sort_by(|a, b| b.1.cmp(&a.1));
    for (k, cnt) in by {
        let sh = &shapes[k];
        let building = c.wants[me].get(k).copied().unwrap_or(0.0) > 0.0;
        let row = ui.horizontal(|ui| {
            let name = if building { format!("{} · формируются", sh.name) } else { sh.name.clone() };
            let mut job = egui::text::LayoutJob::simple_singleline(name, egui::FontId::new(14.0, egui::FontFamily::Proportional), s.ink);
            job.wrap = egui::text::TextWrapping::truncate_at_width(200.0);
            let gal = ui.painter().layout_job(job);
            let (lr, _) = ui.allocate_exact_size(vec2(206.0, 20.0), egui::Sense::hover());
            ui.painter().galley(egui::pos2(lr.min.x, lr.center().y - gal.size().y / 2.0), gal, s.ink);
            widgets::meter(ui, cnt as f32 / n.max(1) as f32 * 2.0, ui.available_width() - 4.0);
        });
        let bats = shape_words(&pack, sh);
        let sup: Vec<&str> = sh.support.iter().filter_map(|&k| pack.support.get(k)).map(|x| x.name.as_str()).collect();
        let mean = ours.iter().filter(|d| d.shape == k).map(|d| c.strength(&pack, d)).sum::<f32>() / cnt as f32;
        let name = sh.name.clone();
        row.response.tip(|| {
            Card::new(name)
                .icon("army")
                .row("дивизий", cnt.to_string())
                .row("батальоны", bats.clone())
                .row("роты", if sup.is_empty() { "нет".to_string() } else { sup.join(", ") })
                .row("численность в среднем", format!("{:.0}%", mean * 100.0))
                .note(if building { "новые дивизии формируются по нему" } else { "новые по нему не формируются: только пополняются" })
        });
    }
    // what is in hand: enough to refill and to form so many more
    let pool = &c.pool[me];
    let next = c.wants[me].iter().enumerate().filter(|x| *x.1 > 0.0).map(|x| x.0).next();
    let more = next.map_or(0.0, |k| {
        let full = &shapes[k].units;
        (0..full.len()).filter(|&u| full[u] > 0.0).map(|u| pool[u] / full[u]).fold(f32::INFINITY, f32::min)
    });
    let more = if more.is_finite() { more } else { 0.0 };
    let words = match more {
        x if x < 0.5 => "Запаса на новые дивизии нет: всё, что делают заводы, идёт на пополнение.",
        x if x < 3.0 => "Запаса хватит на пополнение и на считанные новые дивизии.",
        x if x < 15.0 => "Запаса хватит на пополнение и на несколько новых дивизий.",
        _ => "Запас велик: новые дивизии формируются каждый месяц.",
    };
    ui.scope(|ui| say(ui, s, words)).response.tip(|| {
        let mut card = Card::new("Запас снаряжения и людей").icon("crate").row("новых дивизий хватит на", format!("{more:.0}"));
        for (u, x) in pool.iter().enumerate().filter(|x| *x.1 > 0.05) {
            card = card.row(pack.units[u].name.clone(), format!("{:.0} батальонов", x / strategy::military::corps::BATTALION));
        }
        card.note("пополняет дивизии там, куда доходит снабжение; что остаётся — новые дивизии у столицы")
    });
}

/// Equipment taken, in words: how many battalions it would arm.
pub(super) fn trophy_words(kit: f32) -> String {
    let b = kit / strategy::military::corps::BATTALION;
    match b {
        b if b < 0.5 => "малую часть батальона".to_string(),
        b if b < 1.5 => "целый батальон".to_string(),
        b if b < 5.0 => "несколько батальонов".to_string(),
        _ => "целую дивизию".to_string(),
    }
}
