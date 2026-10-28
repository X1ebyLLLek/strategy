//! The designer's judgement of a template: its properties against an ordinary division, whether the
//! country can afford it, its combat figures (fire, toughness, armour and piercing, attack and
//! defence).

use super::*;

/// What the template is: its qualities against the plain division of the era (a meter each, the
/// notch is the plain one; numbers on hover); what the country can afford of it; then the staff's
/// trials against plain divisions of the same price on three grounds.
pub(super) fn appraise(ui: &mut egui::Ui, s: &Skin, g: &Game, kinds: &[Kind], e: &Edit) {
    let me = g.me;
    let (norms, companies) = (e.norms(&g.pack), e.companies());
    let q = templates::qualities_of(&g.pack, &g.h.war.sides[me], &e.bat, &norms, &companies, kinds);
    let r = templates::plain(kinds);
    let classes = g.pack.war.as_ref().map(|w| w.classes.clone()).unwrap_or_default();
    let mut rows: Vec<(String, f32, f32, Option<&str>)> = classes.iter().enumerate().map(|(i, c)| (format!("Огонь: {c}"), q.fire[i], r.fire[i], None)).collect();
    rows.push(("Стойкость".into(), q.tough, r.tough, None));
    rows.push(("Ширина в линии".into(), q.width, r.width, Some("width")));
    rows.push(("Скорость".into(), q.speed, r.speed, None));
    rows.push(("Люди".into(), q.men, r.men, None));
    rows.push(("Цена".into(), q.cost, r.cost, None));
    rows.push(("Снабжение".into(), q.supply, r.supply, Some("supply")));
    combat(ui, g, e);
    ui.spacing_mut().item_spacing.y = 3.0;
    let size = templates::size_word(q.n);
    ui.label(RichText::new("СВОЙСТВА ПРОТИВ ОБЫЧНОЙ ДИВИЗИИ").family(egui::FontFamily::Name("head".into())).size(13.0).color(s.soft)).tip(|| {
        Card::new("Свойства шаблона").icon("army").text(format!("Дивизия {size}. Риска на полосе — обычная дивизия эпохи. Как хороша техника на деле, покажут бои: здесь она такова, какой её считает генштаб."))
    });
    let war_w = g.pack.war.as_ref().map_or(12.0, |w| w.width);
    for (name, v, base, key) in rows {
        let ratio = if base > 0.0 { v / base } else if v > 0.0 { 3.0 } else { 1.0 };
        let row = ui.horizontal(|ui| {
            let used = match key {
                Some(k) => super::super::terms::term(ui, s, &name, k),
                None => ui.label(RichText::new(&name).size(14.0)),
            }
            .rect
            .width();
            ui.add_space((128.0 - used).max(0.0));
            widgets::meter(ui, ratio, 96.0);
            // (the figure and how far from the plain division, in per cent)
            let figure = if v > 0.0 || base > 0.0 { format!("{} · {:+.0}%", super::super::charts::num(v), (ratio - 1.0) * 100.0) } else { "—".to_string() };
            ui.label(RichText::new(figure).size(13.5).color(s.ink));
        });
        let nm = name.clone();
        row.response.tip(|| {
            let mut c = Card::new(nm.clone()).icon("army").row("эта дивизия", format!("{v:.2}")).row("обычная", format!("{base:.2}"));
            if nm == "Скорость" && v > 0.0 {
                c = c.row("дней на провинцию равнины", format!("{:.0}", strategy::military::corps::STEP_DAYS / v)).row("в горах", format!("{:.0}", strategy::military::corps::STEP_DAYS * strategy::military::corps::ground_cost(strategy::world::map::Terrain::Mountains) / v));
            }
            if nm == "Ширина в линии" && v > 0.0 {
                c = c.row("встанет рядом в бою на равнине", format!("{:.0}", war_w / v));
            }
            c
        });
    }
    if e.bat.iter().sum::<u32>() == 0 {
        return;
    }
    purse(ui, s, g, &q, &norms, &e.bat);
    // the staff's trials: against plain divisions of the same price, a line and a half of them
    let trial = {
        type Key = (Vec<u32>, Vec<usize>, Vec<bool>, u32);
        static CACHE: std::sync::Mutex<Option<(Key, [[(f32, f32); 3]; 2])>> = std::sync::Mutex::new(None);
        let key: Key = (e.bat.clone(), e.norm.clone(), e.support.clone(), g.month());
        let mut cache = CACHE.lock().unwrap();
        match cache.as_ref() {
            Some((k, t)) if *k == key => *t,
            _ => {
                let t = templates::trial_with(&g.pack, &g.h.war.sides[me], &e.bat, &norms, &companies);
                *cache = Some((key, t));
                t
            }
        }
    };
    ui.add_space(6.0);
    ui.label(RichText::new("ШТАБНЫЕ ИСПЫТАНИЯ").family(egui::FontFamily::Name("head".into())).size(13.0).color(s.soft)).tip(|| {
        Card::new("Штабные испытания").icon("binoculars").text("Дивизии этого шаблона против обычных дивизий той же цены, полторы линии боя, — кто чаще берёт верх в штабных играх, в атаке и в обороне, на трёх видах местности. Тот же бой, что на войне; техника — какой её считает генштаб.")
    });
    for (gi, (_, ground)) in templates::TRIAL_GROUND.iter().enumerate() {
        let row = ui.horizontal(|ui| {
            let (lr, _) = ui.allocate_exact_size(vec2(86.0, 26.0), Sense::hover());
            ui.painter().text(pos2(lr.min.x, lr.center().y), egui::Align2::LEFT_CENTER, *ground, egui::FontId::new(14.0, egui::FontFamily::Proportional), s.ink);
            for (side, what) in [(0usize, "атака"), (1, "оборона")] {
                let (p, base) = trial[side][gi];
                let d = p - base;
                let (r, _) = ui.allocate_exact_size(vec2(146.0, 26.0), Sense::hover());
                let painter = ui.painter();
                painter.text(pos2(r.min.x, r.min.y + 7.0), egui::Align2::LEFT_CENTER, format!("{what}: {:.0}% ({:+.0})", p * 100.0, d * 100.0), egui::FontId::new(12.5, egui::FontFamily::Proportional), s.ink);
                // (a bar from the middle: right and green — better than the plain, left and red — worse)
                let b = Rect::from_min_size(pos2(r.min.x, r.min.y + 18.0), vec2(132.0, 5.0));
                painter.rect_filled(b.expand(1.0), 1.5, Color32::from_black_alpha(90));
                let mid = b.center().x;
                let len = (d.clamp(-1.0, 1.0) * b.width() / 2.0).abs().max(2.0);
                let (x0, col) = if d >= 0.0 { (mid, Color32::from_rgb(96, 160, 72)) } else { (mid - len, Color32::from_rgb(190, 72, 56)) };
                painter.rect_filled(Rect::from_min_size(pos2(x0, b.min.y), vec2(len, b.height())), 1.0, col);
                painter.line_segment([pos2(mid, b.min.y - 2.0), pos2(mid, b.max.y + 2.0)], Stroke::new(1.0, s.ink.gamma_multiply(0.6)));
            }
        });
        let ground = ground.to_string();
        row.response.tip(|| {
            Card::new(format!("Испытания {ground}"))
                .icon("binoculars")
                .row("побеждает в атаке", format!("{:.0}% (обычная — {:.0}%)", trial[0][gi].0 * 100.0, trial[0][gi].1 * 100.0))
                .row("побеждает в обороне", format!("{:.0}% (обычная — {:.0}%)", trial[1][gi].0 * 100.0, trial[1][gi].1 * 100.0))
                .note("бой той же модели, что на войне: полторы линии обычных дивизий против дивизий этого шаблона на те же деньги, атакующих в полтора раза больше; окопов нет, укрывает только местность")
                .note("техника — какой её считает генштаб: правду покажут бои")
        });
    }
}

/// What the country can afford of the template, in words (numbers on hover): how many of it a month
/// its factories give as they work now; how many its men still to be called up fill; which of the
/// goods its soldiers' kit takes are dear for it.
fn purse(ui: &mut egui::Ui, s: &Skin, g: &Game, q: &templates::Qualities, norms: &[f32], bat: &[u32]) {
    let me = g.me;
    let pack = &g.pack;
    let c = &g.h.war.sides[me];
    let n = g.h.home(me);
    ui.add_space(6.0);
    ui.label(RichText::new("ПО КАРМАНУ ЛИ СТРАНЕ").family(egui::FontFamily::Name("head".into())).size(13.0).color(s.soft)).tip(|| {
        Card::new("По карману ли стране").icon("coins").text("Армию делает экономика: заводы дают снаряжение, народ — солдат, казна платит. Здесь — сколько таких дивизий даёт страна, хватит ли людей и что из снаряжения ей дорого.")
    });
    // the factories: the worth of what they made last month against this division's price
    let made: f32 = g.h.last_made[me].iter().zip(&c.cost).map(|(m, k)| m * k).sum();
    let per_month = made / q.cost.max(1e-6);
    let pace = match per_month {
        x if x < 0.5 => "заводы почти не дадут таких",
        x if x < 3.0 => "заводы дадут по нескольку в месяц",
        x if x < 15.0 => "заводы дадут около десятка в месяц",
        _ => "заводы дадут десятки в месяц",
    };
    // the men: who can still be called up (millions), against its men (thousands)
    let by_men = n.draftable() * 1000.0 / q.men.max(1e-6);
    let men = match by_men {
        x if x < 3.0 => "людей на призыв почти не осталось",
        x if x < 30.0 => "людей хватит на немногие",
        x if x < 200.0 => "людей хватит на многие",
        _ => "людей хватит с избытком",
    };
    // the goods of its kit and their prices against the usual
    let mut goods: Vec<(usize, f32)> = Vec::new();
    for (u, unit) in pack.units.iter().enumerate().filter(|(u, _)| bat.get(*u).is_some_and(|&b| b > 0)) {
        for &(i, a) in &unit.equip {
            let k = norms.get(u).copied().unwrap_or(1.0);
            let amount = a * k * bat[u] as f32 * BATTALION_OF;
            if let Some(x) = goods.iter_mut().find(|x| x.0 == i) {
                x.1 += amount;
            } else {
                goods.push((i, amount));
            }
        }
    }
    let rel = |i: usize| n.price[i] / (pack.goods[i].price * n.level).max(1e-9);
    let dear: Vec<String> = goods.iter().filter(|x| rel(x.0) > 1.3).map(|x| pack.goods[x.0].name.clone()).collect();
    for (text, tip, bad) in [
        (pace.to_string(), format!("≈{per_month:.1} таких дивизий в месяц по нынешним заводам"), per_month < 0.5),
        (men.to_string(), format!("можно призвать ещё {:.2} млн человек — ≈{by_men:.0} таких дивизий", n.draftable()), by_men < 3.0),
        (if dear.is_empty() { "снаряжение стране по средствам".to_string() } else { format!("дорого стране: {}", dear.join(", ")) }, goods.iter().map(|x| format!("{} ×{:.1}", pack.goods[x.0].name, rel(x.0))).collect::<Vec<_>>().join(", "), !dear.is_empty()),
    ] {
        let col = if bad { super::super::panels::ink_of(s, Color32::from_rgb(190, 60, 40)) } else { s.ink };
        ui.label(RichText::new(text).size(14.0).color(col)).tip(|| Card::new("По карману").icon("coins").text(tip.clone()));
    }
}

/// The template's figures of battle, as HOI4 shows them: breakthrough and defence (its fire as it
/// attacks and as it holds), soft and hard attack (on men and on armour), armour and piercing (as
/// HOI4 reckons a division's: the best and the mean together), how much of it wears armour, what a
/// division costs the factories now. Against the rival's best piercing and armour.
fn combat(ui: &mut egui::Ui, g: &Game, e: &Edit) {
    use super::super::kit::{self, Stat};
    let pack = &g.pack;
    let me = g.me;
    let nu = pack.units.len();
    let n: Vec<f32> = (0..nu).map(|u| e.bat.get(u).copied().unwrap_or(0) as f32 * strategy::military::corps::BATTALION).collect();
    let all: f32 = n.iter().sum();
    if all <= 0.0 {
        return;
    }
    let norms = e.norms(pack);
    let nf = |u: usize| strategy::military::doctrine::norm::fire(norms[u]);
    let classes = pack.war.as_ref().map(|w| w.classes.len()).unwrap_or(0);
    let total = |u: usize| (0..classes).map(|c| pack.units[u].fire[c]).sum::<f32>();
    let breakthrough: f32 = (0..nu).map(|u| n[u] * total(u) * pack.units[u].attack * nf(u)).sum::<f32>() / strategy::military::corps::BATTALION;
    let defence: f32 = (0..nu).map(|u| n[u] * total(u) * pack.units[u].defend * nf(u)).sum::<f32>() / strategy::military::corps::BATTALION;
    // (soft: the class of the line's men; hard: the class of the most armoured kind)
    let soft = pack.units.iter().find(|u| u.line == Line::Front && u.armor <= 0.0).map_or(0, |u| u.class);
    let hard = pack.units.iter().filter(|u| u.armor > 0.0).max_by(|a, b| a.armor.total_cmp(&b.armor)).map(|u| u.class);
    let attack_on = |c: usize| (0..nu).map(|u| n[u] * pack.units[u].fire.get(c).copied().unwrap_or(0.0) * nf(u)).sum::<f32>() / strategy::military::corps::BATTALION;
    let mean = |f: &dyn Fn(usize) -> f32| (0..nu).map(|u| n[u] * f(u)).sum::<f32>() / all;
    let best = |f: &dyn Fn(usize) -> f32| (0..nu).filter(|&u| n[u] > 0.0).map(f).fold(0.0f32, f32::max);
    let armor = 0.3 * best(&|u| pack.units[u].armor) + 0.7 * mean(&|u| pack.units[u].armor);
    let pierce = 0.4 * best(&|u| pack.units[u].pierce) + 0.6 * mean(&|u| pack.units[u].pierce);
    let hardness = mean(&|u| if pack.units[u].armor > 0.0 { 1.0 } else { 0.0 });
    let cost: f32 = (0..nu).map(|u| n[u] * strategy::military::corps::worth_at(pack, u, norms[u]) * g.make_cost(me, u)).sum();
    let plain: f32 = (0..nu).map(|u| n[u] * strategy::military::corps::worth_at(pack, u, norms[u])).sum();
    // the era's plain division (ten of its plainest front kind) for comparison
    let plain_b = templates::plain_bat(&templates::kinds(pack, &g.h.war.sides[me]));
    let pn: Vec<f32> = (0..nu).map(|u| plain_b.get(u).copied().unwrap_or(0) as f32 * strategy::military::corps::BATTALION).collect();
    let p_break: f32 = (0..nu).map(|u| pn[u] * total(u) * pack.units[u].attack).sum::<f32>() / strategy::military::corps::BATTALION;
    let p_def: f32 = (0..nu).map(|u| pn[u] * total(u) * pack.units[u].defend).sum::<f32>() / strategy::military::corps::BATTALION;
    let p_on = |c: usize| (0..nu).map(|u| pn[u] * pack.units[u].fire.get(c).copied().unwrap_or(0.0)).sum::<f32>() / strategy::military::corps::BATTALION;
    let p_cost: f32 = (0..nu).map(|u| pn[u] * strategy::military::corps::worth_at(pack, u, 1.0)).sum();
    let ratio = |a: f32, b: f32| if b > 0.0 { a / b } else if a > 0.0 { 3.0 } else { 1.0 };
    // the rival's best piercing and armour, of the kinds its army fields
    let theirs = &g.h.war.sides[1 - me].army.count;
    let their_pierce = (0..nu).filter(|&u| theirs[u] > 0.01).map(|u| pack.units[u].pierce).fold(0.0f32, f32::max);
    let their_armor = (0..nu).filter(|&u| theirs[u] > 0.01).map(|u| pack.units[u].armor).fold(0.0f32, f32::max);
    ui.add_space(4.0);
    ui.label(RichText::new("БОЕВЫЕ ХАРАКТЕРИСТИКИ").family(egui::FontFamily::Name("head".into())).size(13.0).color(chrome::skin(ui.ctx()).soft));
    let tone_r = |r: f32| if r > 1.15 { kit::GOOD } else if r < 0.85 { kit::BAD } else { kit::PLAIN };
    let (rb, rd, rs) = (ratio(breakthrough, p_break), ratio(defence, p_def), ratio(attack_on(soft), p_on(soft)));
    // (each figure with how far it stands from the plain division's, in per cent)
    let fig = |x: f32, r: f32| format!("{} ({:+.0}%)", super::super::charts::num(x), (r - 1.0) * 100.0);
    let mut items = vec![
        Stat::new("march", "Прорыв", fig(breakthrough, rb), tone_r(rb)),
        Stat::new("shield", "Оборона", fig(defence, rd), tone_r(rd)),
        Stat::new("helmet", "По пехоте", fig(attack_on(soft), rs), tone_r(rs)),
    ];
    let mut nums = vec![format!("{breakthrough:.1} (обычная {p_break:.1})"), format!("{defence:.1} (обычная {p_def:.1})"), format!("{:.1} (обычная {:.1})", attack_on(soft), p_on(soft))];
    if let Some(h) = hard {
        let rh = ratio(attack_on(h), p_on(h));
        items.push(Stat::new("tank", "По броне", fig(attack_on(h), rh), if attack_on(h) > 0.0 && pierce >= their_armor { kit::GOOD } else { kit::MIXED }));
        items.push(Stat::new("shield", "Броня", if armor <= 0.0 { "нет".to_string() } else { format!("{} · {}", super::super::charts::num(armor), if armor > their_pierce { "не пробить" } else { "пробивают" }) }, if armor > their_pierce { kit::GOOD } else { kit::PLAIN }));
        items.push(Stat::new("shell", "Пробитие", format!("{} · {}", super::super::charts::num(pierce), if their_armor <= 0.0 { "брони нет" } else if pierce >= their_armor { "пробивает" } else { "не пробивает" }), if pierce >= their_armor { kit::GOOD } else { kit::BAD }));
        items.push(Stat::new("gears", "Бронированность", format!("{:.0}%", hardness * 100.0), kit::PLAIN));
        nums.extend([format!("{:.1} (обычная {:.1})", attack_on(h), p_on(h)), format!("{armor:.1}"), format!("{pierce:.1}"), format!("{:.0}%", hardness * 100.0)]);
    }
    let rc = ratio(cost, p_cost);
    items.push(Stat::new("anvil", "Цена на заводах", fig(cost * 10.0, rc), if cost > plain * 1.1 { kit::BAD } else if cost < plain * 0.95 { kit::GOOD } else { kit::PLAIN }));
    nums.push(format!("{:.0} (обычная {:.0})", cost * 10.0, p_cost * 10.0));
    let resp = kit::stats(ui, &items, 3);
    let notes = [
        "огонь дивизии в наступлении: роды, созданные для удара (танки), бьют сильнее, роды обороны — слабее",
        "огонь дивизии в обороне: пехота и пушки держат крепче, чем бьют",
        "огонь по людям — пехоте, расчётам, всадникам",
        "огонь по бронированным целям",
        "броня дивизии (как в HOI4: 30% лучшей и 70% средней): огонь, что её не пробивает, большей частью отскакивает",
        "пробитие (40% лучшего и 60% среднего): пробивает ли она броню врага",
        "доля бронированных частей",
        "сколько товаров уходит на дивизию с учётом навыка заводов",
    ];
    for (i, r) in resp.into_iter().enumerate() {
        let it = &items[i];
        let k = match it.title.as_str() {
            "Прорыв" => 0,
            "Оборона" => 1,
            "По пехоте" => 2,
            "По броне" => 3,
            "Броня" => 4,
            "Пробитие" => 5,
            "Бронированность" => 6,
            _ => 7,
        };
        let (title, value) = (it.title.clone(), nums.get(i).cloned().unwrap_or_default());
        r.tip(|| {
            let mut c = Card::new(title).icon("army").row("у шаблона", value).note(notes[k]);
            if k == 4 || k == 5 {
                c = c.row("лучшее пробитие у врага", format!("{their_pierce:.0}")).row("лучшая броня у врага", format!("{their_armor:.0}"));
            }
            if k == 7 {
                c = c.row("по ценам набора", format!("{:.0}", plain * 10.0));
            }
            c
        });
    }
}

/// (the battalions' goods are counted per battalion of each kind)
const BATTALION_OF: f32 = templates::BATTALION;
