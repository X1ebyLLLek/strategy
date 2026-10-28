//! The folder of production (as HOI4's production screen, without factories to click): what the
//! factories make in a month; the new models the laboratories offer — what their makers promise,
//! what trials showed, what the factories would lose in turning to them — to put into production,
//! to try first, or to turn down; each kind's line — its model, how well the factories know it
//! (their skill: a kit costs them less the better they know it), how much of the army has the new
//! model, what a kit costs, what was made, the government's priority; the models of the past.

use crate::army::arms::{retain, PRIO, PRIO_USUAL, TRIAL_MONTHS};
use super::*;
use super::super::kit::{self, Stat};

fn sign(x: f32) -> String {
    format!("{:+.0}%", x * 100.0)
}

/// The folder of production.
pub fn production(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    let nu = pack.units.len();
    if g.arms.lines[me].len() != nu {
        kit::words(ui, "Армии в этой эпохе нет — делать нечего.", 14.0, s.soft);
        return;
    }
    // the figures of the month
    {
        let lines = &g.arms.lines[me];
        let made: f32 = lines.iter().map(|l| l.made).sum::<f32>() / strategy::military::corps::BATTALION;
        let army: f32 = g.h.war.sides[me].army.count.iter().sum();
        let w: f32 = g.h.war.sides[me].army.count.iter().sum::<f32>().max(1e-9);
        let skill: f32 = lines.iter().zip(&g.h.war.sides[me].army.count).map(|(l, a)| l.skill * a).sum::<f32>() / w;
        let refit: f32 = lines.iter().zip(&g.h.war.sides[me].army.count).map(|(l, a)| l.refit * a).sum::<f32>() / w;
        let law = crate::country::policy::LAWS[g.policy.law[me]].name;
        let lot = |x: f32| match x {
            x if x < 1.0 => "почти ничего",
            x if x < 10.0 => "немного",
            x if x < 60.0 => "заметно",
            x if x < 300.0 => "много",
            _ => "очень много",
        };
        let items = vec![
            Stat::new("gears", "Выпуск в месяц", lot(made), if made > 1.0 { kit::GOOD } else { kit::MIXED }),
            Stat::new("anvil", "Навык заводов", kit::grade(skill), if skill > 0.7 { kit::GOOD } else if skill > 0.45 { kit::MIXED } else { kit::BAD }),
            Stat::new("army", "Армия", lot(army / strategy::military::corps::BATTALION / 10.0), kit::PLAIN),
            Stat::new("spanner", "Новыми образцами", kit::share(refit), if refit > 0.8 { kit::GOOD } else { kit::MIXED }),
        ];
        let (army_b, skill_p, refit_p) = (army / strategy::military::corps::BATTALION, skill * 100.0, refit * 100.0);
        let nums = [format!("{made:.0} бат."), format!("{skill_p:.0}%"), format!("{army_b:.0} бат."), format!("{refit_p:.0}%")];
        let items: Vec<Stat> = items.into_iter().zip(nums.iter()).map(|(it, n)| it.num(n.clone())).collect();
        let mut r = kit::stats(ui, &items, 2).into_iter();
        let (r0, r1) = (r.next().unwrap(), r.next().unwrap());
        r1.tip(|| Card::new("Навык заводов").icon("anvil").row("в среднем", format!("{skill_p:.0}%")).row("армия, батальонов", format!("{army_b:.0}")).row("новыми образцами", format!("{refit_p:.0}%")).text("Как хорошо заводы знают то, что делают (в среднем по армии). Чем лучше знают, тем меньше товаров уходит на один комплект.").note("растёт, пока делают; падает при переходе на новый образец; предел поднимают промышленность, перевооружение, военная экономика"));
        r0.tip(|| Card::new("Выпуск").icon("gears").row("батальонов за месяц", format!("{made:.0}")).row("хозяйство", law.to_string()).note("сколько заводы сделали за прошлый месяц (людей и снаряжения на батальоны)"));
    }
    offers(ui, s, g);
    kit::ribbon(ui, "Линии производства", Some("gears"));
    let mut set: Option<(usize, usize)> = None;
    let mut spotted = false;
    for u in 0..nu {
        let army = g.h.war.sides[me].army.count[u];
        let l = g.arms.lines[me][u].clone();
        let unit = &pack.units[u];
        let making = g.h.war.sides[me].making.get(u).copied().unwrap_or(0.0);
        // (kinds the army neither has nor makes, and nobody asks for, are folded away)
        if army < 0.01 && l.made < 1e-4 && making <= 0.0 && l.prio == PRIO_USUAL && l.models.len() == 1 {
            continue;
        }
        let cost = g.make_cost(me, u);
        let m = l.model().clone();
        let resp = kit::framed(ui, None, false, |ui| {
            ui.horizontal(|ui| {
                let icons = chrome::icons(ui.ctx());
                let (ir, _) = ui.allocate_exact_size(vec2(40.0, 40.0), egui::Sense::hover());
                if let Some(ch) = chrome::of(ui.ctx()) {
                    let mut v = chrome::Out::new();
                    super::super::hud::sunk(&mut v, ir.shrink(2.0), &ch, 3.0);
                    ui.painter().extend(v);
                    super::super::look::icon(&icons, &unit.icon, 26.0, ch.trim).paint_at(ui, egui::Rect::from_center_size(ir.center(), vec2(26.0, 26.0)));
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    kit::bold(ui, &m.name, 15.0, s.ink);
                    kit::words(ui, format!("{} · {}", unit.name, if m.gen == 1 { "исходный образец" } else { "новый образец" }), 12.5, s.soft);
                });
            });
            ui.add_space(3.0);
            // the skill and the refit, as gauges
            let w = ui.available_width();
            let half = (w - 10.0) / 2.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.vertical(|ui| {
                    ui.set_width(half);
                    ui.label(RichText::new(format!("навык заводов: {:.0}%", l.skill * 100.0)).size(12.0).color(s.soft));
                    let col = if l.skill > 0.7 { kit::GOOD } else if l.skill > 0.45 { kit::MIXED } else { kit::BAD };
                    kit::gauge(ui, half, 9.0, l.skill, col);
                });
                ui.vertical(|ui| {
                    ui.set_width(half);
                    ui.label(RichText::new(format!("новым образцом: {:.0}%", l.refit * 100.0)).size(12.0).color(s.soft));
                    kit::gauge(ui, half, 9.0, l.refit, Color32::from_rgb(90, 130, 170));
                });
            });
            ui.add_space(2.0);
            let made = l.made / strategy::military::corps::BATTALION;
            let cost_word = if cost > 1.15 { "дорого" } else if cost < 0.9 { "дёшево" } else { "как обычно" };
            let made_w = if made < 0.05 { "не делают" } else if made < 1.0 { "делают понемногу" } else if made < 10.0 { "делают" } else { "делают много" };
            ui.label(RichText::new(format!("{made:.1} бат. в месяц ({made_w}) · комплект ×{cost:.2} ({cost_word})")).size(12.5).color(s.ink));
            // the priority: five small plates
            let names: Vec<&str> = PRIO.iter().map(|p| p.1).collect();
            let short = ["Нет", "Меньше", "Штаб", "Больше", "Всё"];
            let row = ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                let w = (ui.available_width() - 12.0) / 5.0;
                for k in 0..5 {
                    let b = widgets::plate(ui, short[k], w, l.prio == k, true);
                    if b.tip(|| Card::new(names[k]).icon("gears").row("от доли генштаба", format!("×{:.1}", PRIO[k].0)).note(if k == PRIO_USUAL { "сколько делать — решает генштаб по своему плану" } else { "заводы получат больше (меньше) денег армии на этот род" })).clicked() && l.prio != k {
                        set = Some((u, k));
                    }
                }
            });
            // (for the window's self-checks: the first line's plates, when in sight)
            let (clip, r) = (ui.clip_rect(), row.response.rect);
            if !spotted && clip.min.y <= r.min.y && r.max.y <= clip.max.y {
                chrome::spot(ui.ctx(), "prio", r);
            }
            spotted = true;
        });
        let models = l.models.clone();
        let (sk, rf, total) = (l.skill, l.refit, l.total);
        let name = m.name.clone();
        resp.response.tip(|| {
            let mut c = Card::new(name.clone()).icon(unit.icon.clone()).sub(format!("{} · поколение {}", unit.name, m.gen)).row("сделано за месяц", format!("{:.1} батальона", l.made / strategy::military::corps::BATTALION)).row("навык заводов", format!("{:.0}%", sk * 100.0)).row("армия с этим образцом", format!("{:.0}%", rf * 100.0)).tone("комплект к обычной цене", format!("×{cost:.2}"), if cost > 1.1 { Tone::Bad } else if cost < 0.95 { Tone::Good } else { Tone::Plain }).row("сделано за войну и мир", format!("{:.0} батальонов", total / strategy::military::corps::BATTALION));
            if unit.armor > 0.0 || unit.pierce > 0.0 {
                c = c.row("броня / пробитие", format!("{:.0} / {:.0}", unit.armor, unit.pierce));
            }
            c = c.row("в наступлении / в обороне", format!("×{:.2} / ×{:.2}", unit.attack, unit.defend)).row("сложность производства", format!("×{:.1}", unit.complexity));
            if models.len() > 1 {
                c = c.head("Образцы");
                for md in models.iter().rev().take(5) {
                    let what = match md.tried {
                        Some(t) => format!("испытан: {}", sign(crate::army::arms::pct(t))),
                        None if md.gen == 1 => "исходный".to_string(),
                        None => format!("обещали {}", sign(crate::army::arms::pct(md.claim))),
                    };
                    c = c.row(md.name.clone(), format!("{what}, с {}", crate::words::date_words(md.month)));
                }
            }
            c.note("истинную силу образца покажут испытания или бои")
        });
    }
    if let Some((u, k)) = set {
        act(g, &[("prio", format!("{u}:{k}"))]);
    }
    // the models turned down
    let refused: Vec<String> = g.arms.refused.iter().filter(|r| r.0 == me).rev().take(6).map(|r| r.1.clone()).collect();
    if !refused.is_empty() {
        kit::ribbon(ui, "Отвергнутые образцы", Some("stop"));
        kit::words(ui, refused.join(", "), 13.0, s.soft);
    }
}

/// The new models waiting for the government's word: cards with their promise, their trials, the
/// skill the factories would lose; three choices.
fn offers(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    let list = g.arms.offers[me].clone();
    kit::ribbon(ui, "Новые образцы", Some("science"));
    if list.is_empty() {
        let hint = if g.h.labs[me].is_some() { "Конструкторы работают; новых образцов пока нет." } else { "Науки в этой эпохе нет: образцы не меняются." };
        kit::words(ui, hint, 13.5, s.soft);
        return;
    }
    let mut choice: Option<(&str, u64)> = None;
    for o in &list {
        let unit = &pack.units[o.kind];
        let line = &g.arms.lines[me][o.kind];
        let keep = retain(&pack, o.kind, o.claim);
        let at = Color32::from_rgb(70, 120, 170);
        let resp = kit::framed(ui, Some(at), true, |ui| {
            ui.horizontal(|ui| {
                if let Some(ch) = chrome::of(ui.ctx()) {
                    let (sr, _) = ui.allocate_exact_size(vec2(38.0, 38.0), egui::Sense::hover());
                    super::super::hud::seal(ui, &ch, sr.center(), 18.0, &unit.icon, Color32::from_rgb(170, 210, 240));
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    kit::bold(ui, &o.name, 15.5, s.ink);
                    kit::words(ui, format!("{} · сменит {}", unit.name, line.model().name), 12.5, s.soft);
                });
            });
            ui.add_space(2.0);
            let much = |x: f32| match x {
                x if x > 0.25 => "намного больше",
                x if x > 0.08 => "больше",
                x if x > -0.03 => "как прежде",
                _ => "меньше",
            };
            let promise = format!("Конструкторы обещают: огня {}, стойкости {}.", much(o.claim.0.exp() - 1.0), much(o.claim.1.exp() - 1.0));
            let (cf, ct) = (sign(o.claim.0.exp() - 1.0), sign(o.claim.1.exp() - 1.0));
            ui.scope(|ui| kit::words(ui, promise, 13.5, s.ink)).response.tip(|| Card::new("Обещание").icon("science").row("огонь", cf.clone()).row("стойкость", ct.clone()).note("конструкторы обычно хвалят своё сверх меры"));
            match (o.trial, o.tried) {
                (t, _) if t > 0 => {
                    kit::words(ui, "Идут испытания.", 13.0, s.soft);
                    kit::gauge(ui, ui.available_width(), 8.0, 1.0 - t as f32 / TRIAL_MONTHS as f32, Color32::from_rgb(90, 130, 170));
                }
                (_, Some(t)) => {
                    let good = 0.5 * (t.0 + t.1) > 0.0;
                    let much = |x: f32| match x {
                        x if x > 0.25 => "намного больше",
                        x if x > 0.04 => "больше",
                        x if x > -0.04 => "как прежде",
                        _ => "меньше",
                    };
                    let (tf, tt) = (sign(t.0.exp() - 1.0), sign(t.1.exp() - 1.0));
                    ui.scope(|ui| kit::words(ui, format!("Испытания показали: огня {}, стойкости {}.", much(t.0.exp() - 1.0), much(t.1.exp() - 1.0)), 13.5, kit::ink(s, if good { kit::GOOD } else { kit::BAD }))).response.tip(|| Card::new("Испытания").icon("binoculars").row("огонь", tf.clone()).row("стойкость", tt.clone()));
                }
                _ => {
                    kit::words(ui, "Не испытан: чего он стоит, покажет только бой.", 12.5, s.soft);
                }
            }
            let keep_w = if keep > 0.7 { "почти весь" } else if keep > 0.5 { "больше половины" } else { "меньше половины" };
            ui.scope(|ui| kit::words(ui, format!("Заводы сохранят {keep_w} навыка; армия перевооружится по мере выпуска."), 12.5, s.soft)).response.tip(|| Card::new("Навык заводов").icon("anvil").row("сохранят", format!("{:.0}%", keep * 100.0)));
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if kit::act_button(ui, "В производство", "gears", true, false).clicked() {
                    choice = Some(("model_ok", o.id));
                }
                let can_try = o.trial == 0 && o.tried.is_none();
                if kit::act_button(ui, "Испытать", "binoculars", can_try, false).tip(|| Card::new("Испытания").icon("binoculars").row("срок", format!("{TRIAL_MONTHS} мес.")).note("испытания скажут, чего образец стоит на самом деле (почти точно)")).clicked() {
                    choice = Some(("model_try", o.id));
                }
                if kit::act_button(ui, "Отказ", "stop", true, false).clicked() {
                    choice = Some(("model_no", o.id));
                }
            });
        });
        let name = o.name.clone();
        let month = o.month;
        resp.response.tip(|| Card::new(name.clone()).icon(unit.icon.clone()).sub(format!("представлен: {}", crate::words::date_words(month))).text("Каждый новый образец — ставка: заводы начнут учиться заново, а чего он стоит — неизвестно, пока не испытан или не побывал в бою.").note("образец ждёт решения до двух лет"));
    }
    if let Some((key, id)) = choice {
        act(g, &[(key, id.to_string())]);
    }
}
