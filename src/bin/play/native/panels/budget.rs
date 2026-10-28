//! The economy folder's parts of the state's household and the people (27th session): the civil
//! budget line by line (officials, schools, doctors, police, relief — what each costs and what it
//! bought), the people (births, deaths, epidemics, the learned), the banks, the money and the debt at
//! home. Numbers stand on the tiles: what affects what must be seen.

use crate::country::civil::{ADMIN, HEALTH, LINE, LINES, ORDER, RELIEF, SCHOOLS};
use crate::country::credit::RATE;
use super::*;
use super::super::charts::{num, pct};
use super::super::kit::{self, Stat};

fn tone_up(x: f32, good: f32, bad: f32) -> Color32 {
    if x >= good {
        kit::GOOD
    } else if x <= bad {
        kit::BAD
    } else {
        kit::PLAIN
    }
}

/// The civil budget: the state's reach and what its lines bought, then each line with its levels.
pub(super) fn civil(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let income = g.income(me).max(1e-6);
    let gdp = g.h.home(me).gdp.max(1e-6);
    kit::ribbon(ui, "Бюджет: гражданские статьи", Some("scroll"));
    let c = g.civil.clone();
    let lit = g.polity_of(me).literacy;
    let asked: f32 = c.asked[me].iter().sum();
    let paid: f32 = c.paid[me].iter().sum();
    let items = vec![
        Stat::new("stamper", "Охват власти", pct(c.reach[me]), tone_up(c.reach[me], 0.75, 0.45)),
        Stat::new("graduate", "Школы", pct(c.schools[me]), tone_up(c.schools[me], 1.2, 0.6)),
        Stat::new("meds", "Медицина", pct(c.health[me]), tone_up(c.health[me], 1.2, 0.5)),
        Stat::new("shield", "Порядок", pct(c.order[me]), tone_up(c.order[me], 1.1, 0.6)),
        Stat::new("coins", "Статьи в месяц", format!("{} из {}", num(paid), num(asked)), if paid + 1e-6 < asked { kit::BAD } else { kit::PLAIN }),
        Stat::new("hand", "Долг по жалованью", if c.arrears[me] < 0.02 { "нет".to_string() } else { pct(c.arrears[me]) }, if c.arrears[me] > 0.1 { kit::BAD } else { kit::PLAIN }),
    ];
    let resp = kit::stats(ui, &items, 3);
    let ev = g.fiscal.evaded[me];
    let reach = c.reach[me];
    let (sch, hea, ord, arr) = (c.schools[me], c.health[me], c.order[me], c.arrears[me]);
    let cards: [Box<dyn Fn() -> Card>; 6] = [
        Box::new(move || Card::new("Охват власти").icon("stamper").row("сейчас", pct(reach)).row("не доплачивают налога", pct(ev)).text("Сколько из того, что казне положено, она может собрать и донести до дела. Растёт от управления (чиновники) и грамотности, падает, когда чиновникам не платят.").note("медленно: на смену чиновников нужны годы")),
        Box::new(move || Card::new("Школы").icon("graduate").row("на что хватает денег", format!("{:.0}% обычного", sch * 100.0)).row("грамотных", pct(lit)).text("Грамотность идёт за школами медленно — за поколение. Грамотные становятся служащими и учёными: растут заводы и наука.")),
        Box::new(move || Card::new("Медицина").icon("meds").row("врачей против обычного", format!("{:.0}%", hea * 100.0)).text("Меньше умирают, меньше гибнут от голода, мор короче и мягче.")),
        Box::new(move || Card::new("Порядок").icon("shield").row("полиции и судов против обычного", format!("{:.0}%", ord * 100.0)).text("Смута гаснет быстрее, собственники верят судам. Жандармерия злит интеллигенцию и рабочих.")),
        Box::new(move || Card::new("Гражданские статьи").icon("coins").row("просят в месяц", num(asked)).row("выплачено", num(paid)).row("от доходов казны", pct(asked / income)).text("Платятся из того, что казна раньше раздавала всем поровну; если не хватает — из запаса казны, но не больше половины его в месяц.")),
        Box::new(move || Card::new("Долг по жалованью").icon("hand").row("сейчас", pct(arr)).text("Чиновник, которому не платят, ворует: утечка казны растёт, охват власти падает.")),
    ];
    for (r, cd) in resp.into_iter().zip(cards.iter()) {
        r.tip(|| cd());
    }
    say(ui, s, "Каждая статья — деньги людям, которые делают дело (чиновники, учителя и врачи — образованные, полицейские и санитары — из простых людей, пособия — беднейшим), и запас, который копится годами и тает без денег.");
    let levels = c.level[me];
    for k in 0..LINES {
        let lv = levels[k] as usize;
        let money = LINE[k].cost[lv] * gdp;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.label(RichText::new(format!("{}:", LINE[k].name)).size(14.5).color(s.soft));
            ui.label(RichText::new(LINE[k].levels[lv]).family(egui::FontFamily::Name("bold".into())).size(14.5).color(s.ink));
            ui.label(RichText::new(format!("{} в месяц · {} доходов", num(money), pct(money / income))).size(13.0).color(s.soft));
        });
        let tiles: Vec<(&str, &str)> = LINE[k].levels.iter().map(|w| (*w, LINE[k].icon)).collect();
        let picked = widgets::law(ui, &tiles, lv, 1, &|x| {
            let m = LINE[k].cost[x] * gdp;
            Card::new(format!("{}: {}", LINE[k].name, LINE[k].levels[x]))
                .icon(LINE[k].icon)
                .text(LINE[k].about)
                .row("в месяц", num(m))
                .row("от выпуска", format!("{:.1}%", LINE[k].cost[x] * 100.0))
                .row("от доходов казны", pct(m / income))
                .note(if x == lv { "действует сейчас" } else { "щёлкните — изменить статью" })
        });
        if let Some(x) = picked {
            act(g, &[("civ", format!("{k}:{x}"))]);
        }
        ui.add_space(2.0);
    }
    let o = 1 - me;
    let theirs: Vec<String> = [ADMIN, SCHOOLS, HEALTH, ORDER, RELIEF].iter().map(|&k| format!("{} — {}", LINE[k].name.to_lowercase(), LINE[k].levels[g.civil.level[o][k] as usize])).collect();
    kit::words(ui, format!("У соседа: {}", theirs.join(", ")), 13.0, s.soft);
}

/// The people: births and deaths, the growth, epidemics, the learned.
pub(super) fn people(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    kit::ribbon(ui, "Население", Some("people"));
    let d = g.demo.clone();
    let growth = g.people_growth(me);
    let lit = g.polity_of(me).literacy;
    let learned = g.learned_share(me);
    let pop = g.h.home(me).people();
    let towns = 1.0 - g.spirits.facts[me].land_share;
    let ill = d.epidemic[me] > 0;
    let items = vec![
        Stat::new("people", "Людей", format!("{} млн", num(pop)), kit::PLAIN),
        Stat::new("sprout", "Прирост за год", format!("{:+.1}%", growth * 100.0), tone_up(growth, 0.004, -0.002)),
        Stat::new("village", "Рождается", format!("{:.1}% в год", d.births[me] * 100.0), kit::PLAIN),
        Stat::new("broken", "Умирает", format!("{:.1}% в год", d.deaths[me] * 100.0), tone_up(-d.deaths[me], -0.018, -0.03)),
        Stat::new("graduate", "Образованных", pct(learned), kit::PLAIN),
        Stat::new("meds", "Мор", if ill { format!("идёт, {} мес.", d.epidemic[me]) } else { "нет".to_string() }, if ill { kit::BAD } else { kit::PLAIN }),
    ];
    let resp = kit::stats(ui, &items, 3);
    let (b, dd, plague, dead, schooled) = (d.births[me], d.deaths[me], d.plague[me], d.plague_dead[me], d.schooled[me]);
    let cards: [Box<dyn Fn() -> Card>; 6] = [
        Box::new(move || Card::new("Население").icon("people").row("миллионов", num(pop)).row("в городах и на заводах", pct(towns))),
        Box::new(move || Card::new("Прирост").icon("sprout").row("за год", format!("{:+.2}%", growth * 100.0)).text("Рождения минус смерти, голод, мор и война.")),
        Box::new(move || Card::new("Рождаемость").icon("village").row("в год", format!("{:.2}%", b * 100.0)).row("грамотных", pct(lit)).text("Бедная неграмотная деревня рожает больше всех; грамотность, город и достаток снижают рождаемость — через поколение.")),
        Box::new(move || Card::new("Смертность").icon("broken").row("в год", format!("{:.2}%", dd * 100.0)).text("Падает от врачей и сытости; голод убивает сверх неё.")),
        Box::new(move || Card::new("Образованные").icon("graduate").row("доля людей", pct(learned)).row("выучилось за месяц", format!("{} тыс.", num(schooled * 1000.0))).text("Грамотные из простых людей становятся служащими, учёными, инженерами — столько, сколько позволяет грамотность страны.")),
        Box::new(move || Card::new("Мор").icon("meds").row("сейчас", if ill { format!("{:.2}% людей в месяц", plague * 100.0) } else { "нет".into() }).row("умерло от мора в игре", format!("{} тыс.", num(dead * 1000.0))).text("Приходит с войной, голодом и теснотой городов; врачи делают его короче и мягче.")),
    ];
    for (r, cd) in resp.into_iter().zip(cards.iter()) {
        r.tip(|| cd());
    }
    kit::words(ui, "Рождаемость и смертность у каждой группы свои: деревня рожает и хоронит больше города. Врачи, сытость и грамотность меняют их медленно — плоды видны через годы.", 13.0, s.soft);
    groups(ui, s, g);
}

/// The people by groups (29th session; the page was six tiles and a blank folder): for each group how
/// many it is, and three gauges, the longer the better — whether it has what it needs, whether there
/// is work for it, how much of its patience is left; the cards say which goods it goes without.
fn groups(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let pack = g.pack.clone();
    kit::ribbon(ui, "Кто как живёт", Some("village"));
    kit::words(ui, "У каждой группы три полоски — чем длиннее, тем лучше: хватает ли нужного, есть ли работа, сколько терпения осталось.", 12.5, s.soft);
    ui.add_space(3.0);
    let n = g.h.home(me);
    let all: f32 = n.pop.iter().sum::<f32>().max(1e-9);
    let tone = |good: f32| if good >= 0.85 { kit::GOOD } else if good >= 0.6 { kit::MIXED } else { kit::BAD };
    for (gi, grp) in pack.groups.iter().enumerate() {
        if n.pop[gi] <= 1e-6 {
            continue;
        }
        let needs = 1.0 - n.hardship[gi].clamp(0.0, 1.0);
        let work = n.unemployed[gi].map(|u| 1.0 - u.clamp(0.0, 1.0));
        let patience = 1.0 - (n.discontent[gi] / grp.patience.max(1e-6)).clamp(0.0, 1.0);
        let gauges: [(&str, Option<f32>, String); 3] = [
            ("нужное", Some(needs), format!("{:.0}%", needs * 100.0)),
            ("работа", work, work.map_or("—".to_string(), |w| format!("{:.0}%", w * 100.0))),
            ("терпение", Some(patience), if n.unrest[gi] > 0 { "восстали".to_string() } else { format!("{:.0}%", patience * 100.0) }),
        ];
        let worst = [Some(needs), work, Some(patience)].into_iter().flatten().fold(1.0f32, f32::min);
        kit::framed(ui, Some(tone(worst)), false, |ui| {
            ui.horizontal(|ui| {
                kit::bold(ui, grp.name.clone(), 15.0, s.ink);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{} млн · {:.0}%", num(n.pop[gi]), n.pop[gi] / all * 100.0)).size(13.0).color(s.soft));
                });
            });
            let gap = 10.0;
            let w = ((ui.available_width() - 2.0 * gap) / 3.0).floor();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (k, (name, frac, text)) in gauges.iter().enumerate() {
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        let (r, _) = ui.allocate_exact_size(vec2(w, 16.0), egui::Sense::hover());
                        ui.painter().text(r.left_center(), egui::Align2::LEFT_CENTER, *name, egui::FontId::proportional(12.5), s.soft);
                        ui.painter().text(r.right_center(), egui::Align2::RIGHT_CENTER, text, egui::FontId::new(12.5, egui::FontFamily::Name("bold".into())), s.ink);
                        let resp = kit::gauge(ui, w, 10.0, frac.unwrap_or(0.0), frac.map_or(kit::PLAIN, tone));
                        let (gname, miss, disc, pat, unrest) = (grp.name.clone(), (0..grp.needs.len()).map(|j| (pack.goods[grp.needs[j].good].name.clone(), n.unmet[gi][j])).filter(|x| x.1 > 0.01).collect::<Vec<_>>(), n.discontent[gi], grp.patience, n.unrest[gi]);
                        resp.tip(|| {
                            let mut c = Card::new(gname).icon("people");
                            match k {
                                0 => {
                                    c = c.tone("хватает нужного", format!("{:.0}%", needs * 100.0), if needs >= 0.85 { Tone::Good } else { Tone::Bad });
                                    for (good, m) in &miss {
                                        c = c.row(format!("не хватает: {}", good.to_lowercase()), format!("{:.0}%", m * 100.0));
                                    }
                                    c.note("доля потребностей группы, что покрыты, с весом важности: хлеб весит больше, чем всё остальное")
                                }
                                1 => c.row("без работы", work.map_or("работы не ищет".to_string(), |w| format!("{:.0}%", (1.0 - w) * 100.0))).note("доля группы, которой не нашлось места на заводах и в хозяйствах"),
                                _ => c.row("накоплено недовольства", format!("{disc:.2} из {pat:.2}")).row("волнения", if unrest > 0 { format!("идут ещё {unrest} мес.") } else { "нет".to_string() }).note("когда терпение кончится — забастовки и мятеж: группа рвёт связи с властью"),
                            }
                        });
                    });
                }
            });
        });
    }
}

/// The banks, the money and the debt at home: the credit's depth, the money's growth, a crisis, the
/// bonds; the buttons of the debt.
pub(super) fn money(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let income = g.income(me).max(1e-6);
    kit::ribbon(ui, "Банки, деньги и долг", Some("balance"));
    let cr = g.credit.clone();
    let rate = RATE[g.fiscal.rate[me].min(3)];
    let mg = g.money_growth(me);
    let inf = g.spirits.facts[me].inflation;
    let depth = cr.loans[me] / g.coin_base(me).max(1e-6);
    let aim = g.credit_depth(me);
    let bond_rate = g.bond_rate_now(me);
    let foreign = g.policy.debt[me];
    let items = vec![
        Stat::new("coins", "Ставка банка", format!("{:.1}% в год", rate * 1200.0), kit::PLAIN),
        Stat::new("balance", "Займы банков", format!("{:.0}% денег", depth * 100.0), if depth > 1.25 * aim.max(0.05) { kit::BAD } else { kit::PLAIN }),
        Stat::new("chart", "Деньги за год", format!("{:+.0}%", mg * 100.0), tone_up(-(mg - inf).abs(), -0.03, -0.12)),
        Stat::new("hazard", "Кредит", if cr.crunch[me] > 0 { format!("иссяк, {} мес.", cr.crunch[me]) } else { "есть".to_string() }, if cr.crunch[me] > 0 { kit::BAD } else { kit::GOOD }),
        Stat::new("scroll", "Облигации у своих", format!("{:.1} мес. доходов", cr.bonds[me] / income), if cr.bonds[me] > 18.0 * income { kit::BAD } else { kit::PLAIN }),
        Stat::new("backforth", "Долг за границей", format!("{:.1} мес. доходов", foreign / income), if foreign > 12.0 * income { kit::BAD } else { kit::PLAIN }),
    ];
    let resp = kit::stats(ui, &items, 3);
    say(ui, s, "Банки дают хозяевам в долг новые деньги и забирают их при возврате: дешёвый кредит раздувает стройку и цены, дорогой сжимает. Казна может занять у своих — тогда их деньги идут в казну, а не в стройку.");
    let (lent, repaid, crises, paid) = (cr.lent[me], cr.repaid[me], cr.crises[me], cr.bond_paid[me]);
    let (loans, bonds) = (cr.loans[me], cr.bonds[me]);
    let cards: [Box<dyn Fn() -> Card>; 6] = [
        Box::new(move || Card::new("Ставка банка").icon("coins").row("в месяц", format!("{:.2}%", rate * 100.0)).text("Её задаёт политика кредита. Дешёвые деньги — банки дают больше, хозяева строят, цены растут; дорогие — требуют долги назад, стройка стоит, цены успокаиваются.")),
        Box::new(move || Card::new("Займы банков").icon("balance").row("долг хозяев банкам", num(loans)).row("к деньгам, что нужны хозяйству", pct(depth)).row("банки стремятся к", pct(aim)).row("дали за месяц", num(lent)).row("вернули за месяц", num(repaid)).text("Заём — новые деньги, возврат — деньги исчезают. Раздутый кредит в спад или при скачке цен кончается банковским кризисом.")),
        Box::new(move || Card::new("Деньги в стране").icon("chart").row("рост за год", format!("{:+.1}%", mg * 100.0)).row("цены за год", format!("{:+.1}%", inf * 100.0)).text("Денег больше от займов банков, станка и вывоза; меньше — от возврата долгов и ввоза. Если денег прибывает быстрее товаров, растут цены.")),
        Box::new(move || Card::new("Кредит").icon("hazard").row("банковских кризисов в игре", format!("{crises}")).text("После кризиса полтора года банки дают вдвое меньше; вкладчики теряют часть сбережений.")),
        Box::new(move || Card::new("Облигации у своих").icon("scroll").row("долг", num(bonds)).row("процент сейчас", format!("{:.2}% в месяц", bond_rate * 100.0)).row("платим в месяц", num(paid)).text("Казна занимает у собственников и образованных: их деньги идут в казну, а не в стройку; проценты и долг возвращаются им из налогов за десять лет. Шаткой власти, воюющей и задолжавшей дают дороже.")),
        Box::new(move || Card::new("Долг за границей").icon("backforth").row("долг", num(foreign)).text("Займы иностранных банкиров: деньги приходят из-за границы, проценты уходят туда.")),
    ];
    for (r, cd) in resp.into_iter().zip(cards.iter()) {
        r.tip(|| cd());
    }
    ui.horizontal_wrapped(|ui| {
        if kit::act_button(ui, "Продать облигации", "scroll", true, false)
            .tip(|| Card::new("Заём у своих").icon("scroll").text(format!("Три месяца доходов казны у собственников и образованных под {:.2}% в месяц на десять лет. Их деньги не пойдут в стройку.", bond_rate * 100.0)))
            .clicked()
        {
            act(g, &[("bonds_sell", "1".into())]);
        }
        if kit::act_button(ui, "Выкупить облигации", "coins", cr.bonds[me] > 0.0, false)
            .tip(|| Card::new("Выкупить облигации").icon("coins").text("Казна возвращает долг держателям из своего запаса (до 60% его): деньги снова у собственников."))
            .clicked()
        {
            act(g, &[("bonds_redeem", "1".into())]);
        }
        if kit::act_button(ui, "Отказаться платить", "hazard", cr.bonds[me] > 0.0, false)
            .tip(|| Card::new("Отказ от долга").icon("hazard").text("Половина внутреннего долга списана. Собственники, купцы и знать в ярости, деловой климат рушится, следующие десять лет занимать дороже.").note("решение необратимо"))
            .clicked()
        {
            act(g, &[("bonds_repudiate", "1".into())]);
        }
    });
}
