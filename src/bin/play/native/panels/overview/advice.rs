//! What needs attention in the economy now, each with its cause in numbers and the buttons that mend
//! it — a decision sent at once, a page of the folder opened with the lever lit, or the vital sign
//! below opened for a look at its causes. The rules read what the economy shows (the figures of the
//! treasury, of want, of prices, of jobs, of anger); nothing is invented: a worry is shown only while
//! its figure is over the line. With nothing to mend, the page says where money would be best put.

use super::*;
use crate::country::fiscal::CAP;
use strategy::economy::nation::STATE_RESERVE;

/// What a button does.
pub(super) enum Do {
    /// A decision sent as it is.
    Form(Vec<(&'static str, String)>),
    /// The folder turned to a page of the economy, a lever of it lit.
    Page(usize, Option<&'static str>),
    /// One of the vital signs below opened.
    Look(usize),
}

pub(super) struct Button {
    pub label: String,
    pub tip: String,
    pub what: Do,
}

/// A worry (or a chance): how grave (0 — a chance, 1 — trouble, 2 — danger), its icon, what it is,
/// why (with numbers), the buttons.
pub(super) struct Advice {
    pub tone: u8,
    pub icon: &'static str,
    pub title: String,
    pub why: String,
    pub buttons: Vec<Button>,
}

fn form(label: &str, tip: impl Into<String>, pairs: Vec<(&'static str, String)>) -> Button {
    Button { label: label.into(), tip: tip.into(), what: Do::Form(pairs) }
}

fn page(label: &str, tip: &str, page: usize, focus: Option<&'static str>) -> Button {
    Button { label: label.into(), tip: tip.into(), what: Do::Page(page, focus) }
}

fn look(label: &str, tip: &str, k: usize) -> Button {
    Button { label: label.into(), tip: tip.into(), what: Do::Look(k) }
}

/// The worst-off group of the people by what it goes without among the vital needs: its name, the
/// good, and the share it goes without.
fn starving(g: &Game) -> Option<(String, String, f32)> {
    let n = g.h.home(g.me);
    let pack = &g.pack;
    let mut best: Option<(String, String, f32)> = None;
    for (gi, grp) in pack.groups.iter().enumerate() {
        if n.pop[gi] <= 1e-6 {
            continue;
        }
        for (k, need) in grp.needs.iter().enumerate().filter(|(_, x)| x.vital) {
            let miss = n.unmet[gi][k];
            if best.as_ref().is_none_or(|b| miss > b.2) {
                best = Some((grp.name.to_lowercase(), pack.goods[need.good].name.to_lowercase(), miss));
            }
        }
    }
    best
}

/// The group closest to rising: its name and how much of its patience is gone.
fn angriest(g: &Game) -> Option<(String, f32)> {
    let n = g.h.home(g.me);
    (0..g.pack.groups.len())
        .filter(|&k| n.pop[k] > 1e-6)
        .map(|k| (g.pack.groups[k].name.to_lowercase(), n.discontent[k] / g.pack.groups[k].patience.max(1e-6)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

/// The worries of the moment, the gravest first.
pub(super) fn advice(g: &Game) -> Vec<Advice> {
    let me = g.me;
    let n = g.h.home(me);
    let f = &g.spirits.facts[me];
    let b = g.rows.last().map(|r| r.book[me]).unwrap_or_default();
    let (income, spent) = (b.income(), b.spent());
    let net = income - spent;
    let months = if spent > 1e-6 { n.treasury / spent } else { f32::INFINITY };
    let mine = g.adviser.is_none();
    let idle = g.figures(me).idle;
    let tax = (n.tax * 100.0).round();
    let army = (n.mil_share * 100.0).round();
    let build = (n.build_share * 100.0).round();
    let mut out: Vec<Advice> = Vec::new();
    // a button that gives the economy back to you, when an economist holds the levers
    let take_over = || form("Взять хозяйство себе", "налог и стройку казны снова решаете вы", vec![("econ", "me".into())]);

    // the treasury: in debt, or melting — and what eats it
    if n.treasury < 0.0 || (net < 0.0 && spent > 0.0 && months < 12.0) {
        let lines = [("армию", b.army + b.soldiers, "army"), ("науку", b.science, "science"), ("стройку", b.build, "build"), ("гражданские статьи", b.civil, "civil")];
        let top = lines.iter().copied().max_by(|a, c| a.1.total_cmp(&c.1)).unwrap_or(("армию", 0.0, "army"));
        let debt = n.treasury < 0.0;
        let mut buttons = Vec::new();
        if mine {
            let step = n.gdp * 0.01;
            buttons.push(form("Налог +1 п.", format!("налог {tax:.0}% → {:.0}%: ≈ +{} в месяц, но людям тяжелее", tax + 1.0, num(step)), vec![("tax", format!("{:.0}", (tax + 1.0).min(80.0)))]));
            if top.2 == "army" && army >= 2.0 {
                let step = n.treasury.max(0.0) / STATE_RESERVE * 0.02;
                buttons.push(form("Армия −2 п.", format!("доля армии {army:.0}% → {:.0}%: ≈ −{} в месяц армии", army - 2.0, num(step)), vec![("army", format!("{:.0}", army - 2.0))]));
            }
        } else {
            buttons.push(take_over());
        }
        buttons.push(page("Открыть бюджет", "строки доходов и расходов с рычагами", economy::BUDGET, Some(if debt { "tax" } else { top.2 })));
        out.push(Advice {
            tone: if debt || months < 3.0 { 2 } else { 1 },
            icon: "coins",
            title: if debt { "Казна в долгах".into() } else { "Казна тает".into() },
            why: format!("{} в месяц; запаса на {:.0} мес. Больше всего уходит на {} — {:.0}% трат.", if net >= 0.0 { format!("+{}", num(net)) } else { num(net) }, months.max(0.0), top.0, top.1 / spent.max(1e-6) * 100.0),
            buttons,
        });
    }
    // want: the worst-off group goes without what it cannot live without
    if f.hunger > 0.04 {
        let (who, what, miss) = starving(g).unwrap_or(("беднейшие".into(), "самое нужное".into(), f.hunger));
        let mut buttons = Vec::new();
        if !g.fiscal.caps[me] {
            buttons.push(form("Твёрдые цены на хлеб", format!("цена нужного не выше {:.0}% обычной: бедным легче, но чего не хватает — делят в очередях", CAP * 100.0), vec![("fx_caps", "1".into())]));
        }
        if g.fiscal.dole[me] != 1 {
            buttons.push(form("Раздача бедным", "что казна не потратит, достанется тем, кто беднее", vec![("fx_dole", "1".into())]));
        }
        buttons.push(look("Разобрать", "кто и чего не получает", 4));
        out.push(Advice {
            tone: if f.hunger > 0.10 { 2 } else { 1 },
            icon: "bread",
            title: "Не хватает самого нужного".into(),
            why: format!("{what} у группы «{who}»: не достаётся {:.0}%. Нужда копит недовольство, а от голода умирают.", miss * 100.0),
            buttons,
        });
    }
    // anger (when it is not already want that explains it)
    let mood = n.anger();
    if mood >= 1.0 && f.hunger <= 0.04 {
        let (who, near) = angriest(g).unwrap_or(("люди".into(), 0.0));
        let mut buttons = Vec::new();
        if mine && tax >= 1.0 {
            buttons.push(form("Налог −1 п.", format!("налог {tax:.0}% → {:.0}%: людям легче, казна получит ≈ {} меньше", tax - 1.0, num(n.gdp * 0.01)), vec![("tax", format!("{:.0}", tax - 1.0))]));
        }
        if g.fiscal.dole[me] != 1 {
            buttons.push(form("Раздача бедным", "что казна не потратит, достанется тем, кто беднее", vec![("fx_dole", "1".into())]));
        }
        buttons.push(look("Разобрать", "кто недоволен и чем", 5));
        out.push(Advice {
            tone: if mood >= 1.5 { 2 } else { 1 },
            icon: "revolt",
            title: if mood >= 1.5 { "Народ зол".into() } else { "Народ ворчит".into() },
            why: format!("больше всех — «{who}»: {:.0}% терпения исчерпано. Цены за год {:+.0}%, без работы {:.0}%.", near * 100.0, f.inflation * 100.0, idle * 100.0),
            buttons,
        });
    }
    // prices running away
    if f.inflation > 0.08 {
        let mut buttons = Vec::new();
        if g.fiscal.print[me] > 0 {
            buttons.push(form("Остановить станок", "казна перестанет печатать деньги", vec![("fx_print", "0".into())]));
        }
        if g.fiscal.rate[me] < 2 {
            buttons.push(form("Дорогие деньги", "банк забирает деньги из оборота: цены успокоятся, стройка замедлится", vec![("fx_rate", "2".into())]));
        }
        buttons.push(look("Разобрать", "откуда берутся лишние деньги", 2));
        out.push(Advice {
            tone: if f.inflation > 0.15 { 2 } else { 1 },
            icon: "shop",
            title: "Цены растут".into(),
            why: format!("за год {:+.0}%; денег в стране прибавилось на {:+.0}%. Дорожает всё — съедает доходы людей и казны.", f.inflation * 100.0, g.money_growth(me) * 100.0),
            buttons,
        });
    }
    // a slump: prices fall, work is short
    if f.inflation < -0.03 && (idle > 0.08 || f.growth < -0.02) {
        let mut buttons = Vec::new();
        if g.fiscal.rate[me] != 0 {
            buttons.push(form("Дешёвый кредит", "банки дают легко: стройка растёт, а с ней работа", vec![("fx_rate", "0".into())]));
        }
        if mine && build < 100.0 {
            buttons.push(form("Стройка +10 п.", format!("казна строит {build:.0}% → {:.0}% остатка трат", (build + 10.0).min(100.0)), vec![("build", format!("{:.0}", (build + 10.0).min(100.0)))]));
        }
        buttons.push(look("Разобрать", "что с работой", 3));
        out.push(Advice {
            tone: 1,
            icon: "hazard",
            title: "Спад: цены падают, работы мало".into(),
            why: format!("цены за год {:+.1}%, выпуск {:+.1}%, без работы {:.0}%. Деньги стоят — надо запустить стройку.", f.inflation * 100.0, f.growth * 100.0, idle * 100.0),
            buttons,
        });
    }
    // jobs
    if idle > 0.10 {
        let mut buttons = Vec::new();
        if mine && build < 100.0 {
            buttons.push(form("Стройка +10 п.", format!("казна строит {build:.0}% → {:.0}% остатка трат: новые заводы — новые места", (build + 10.0).min(100.0)), vec![("build", format!("{:.0}", (build + 10.0).min(100.0)))]));
        } else if !mine {
            buttons.push(take_over());
        }
        buttons.push(page("Что строит казна", "отрасли, где не хватает товара, строятся выгоднее всего", economy::BUDGET, Some("build")));
        out.push(Advice {
            tone: if idle > 0.2 { 2 } else { 1 },
            icon: "hand",
            title: "Много людей без работы".into(),
            why: format!("без работы {:.0}% работающих; казна строит на {build:.0}% остатка трат. Безработные меньше покупают и злее смотрят на власть.", idle * 100.0),
            buttons,
        });
    }
    // output falling
    if f.growth < -0.03 {
        out.push(Advice {
            tone: if f.growth < -0.08 { 2 } else { 1 },
            icon: "sprout",
            title: "Выпуск падает".into(),
            why: format!("за год {:+.1}%. Смотрите, что давит на него, и где простаивают места.", f.growth * 100.0),
            buttons: vec![look("Разобрать", "что давит на выпуск", 1), page("Отрасли", "где работают и где простаивают", economy::ECONOMY, None)],
        });
    }
    // the banks
    if g.credit.crunch[me] > 0 {
        out.push(Advice {
            tone: 1,
            icon: "balance",
            title: "Кредит иссяк".into(),
            why: format!("после банковского кризиса банки ещё {} мес. дают вдвое меньше: стройка хозяев идёт медленнее.", g.credit.crunch[me]),
            buttons: vec![page("Банки и долг", "ставка, облигации, долг", economy::MONEY, None)],
        });
    }
    out.sort_by(|a, c| c.tone.cmp(&a.tone));
    out.truncate(3);
    // nothing to mend: where money would be best put
    if out.is_empty() {
        let dear: Vec<(String, f32)> = {
            let mut v: Vec<(String, f32)> = (0..g.pack.goods.len())
                .map(|k| (g.pack.goods[k].name.clone(), n.price[k] / (g.pack.goods[k].price * n.level).max(1e-9)))
                .filter(|x| x.1 > 1.5)
                .collect();
            v.sort_by(|a, c| c.1.total_cmp(&a.1));
            v.truncate(3);
            v
        };
        if dear.is_empty() {
            out.push(Advice {
                tone: 0,
                icon: "medal",
                title: "Хозяйство в порядке".into(),
                why: format!("казна {} в месяц, цены {:+.0}% за год, без работы {:.0}%. Можно копить или тратить на науку и армию.", if net >= 0.0 { format!("+{}", num(net)) } else { num(net) }, f.inflation * 100.0, idle * 100.0),
                buttons: vec![page("Открыть бюджет", "куда уходят деньги", economy::BUDGET, None)],
            });
        } else {
            let list = dear.iter().map(|(n, r)| format!("{} ×{}", n.to_lowercase(), super::super::super::charts::fix(*r, 1))).collect::<Vec<_>>().join(", ");
            let mut buttons = vec![page("Что строит казна", "выберите отрасли, где не хватает", economy::BUDGET, Some("build"))];
            buttons.push(page("Модернизация", "новые станки для отрасли", economy::ECONOMY, None));
            out.push(Advice {
                tone: 0,
                icon: "anvil",
                title: "Куда вложить деньги".into(),
                why: format!("в стране не хватает: {list} (цена против обычной). Там новые заводы окупятся быстрее всего."),
                buttons,
            });
        }
    }
    out
}

/// The worries as cards, with their buttons; what a button does is done after the cards are drawn.
pub(super) fn list(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let items = advice(g);
    let mut chosen: Option<(usize, usize)> = None;
    kit::ribbon(ui, "Что требует внимания", Some("alarm"));
    for (i, a) in items.iter().enumerate() {
        let tone = [kit::GOOD, kit::MIXED, kit::BAD][a.tone as usize];
        let resp = kit::framed(ui, Some(tone), false, |ui| {
            ui.horizontal(|ui| {
                let icons = chrome::icons(ui.ctx());
                let (ir, _) = ui.allocate_exact_size(vec2(22.0, 22.0), egui::Sense::hover());
                super::super::super::look::icon(&icons, a.icon, 22.0, kit::ink(s, tone)).paint_at(ui, ir);
                kit::bold(ui, a.title.clone(), 15.5, s.ink);
            });
            kit::words(ui, a.why.clone(), 13.0, s.soft);
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(5.0, 4.0);
                for (j, b) in a.buttons.iter().enumerate() {
                    let r = widgets::small(ui, &b.label).tip(|| Card::new(b.label.clone()).icon(a.icon).text(b.tip.clone()));
                    if i == 0 && j == 0 && ui.clip_rect().contains_rect(r.rect) {
                        chrome::spot(ui.ctx(), "advice_button", r.rect);
                    }
                    if r.clicked() {
                        chosen = Some((i, j));
                    }
                }
            });
        });
        let _ = resp;
    }
    if let Some((i, j)) = chosen.filter(|_| !super::super::super::spoiled()) {
        match &items[i].buttons[j].what {
            Do::Form(pairs) => act(g, pairs),
            Do::Page(p, focus) => {
                set_page(ui.ctx(), Tab::Econ, *p);
                if let Some(f) = focus {
                    super::super::super::reports::light(ui.ctx(), f);
                }
            }
            Do::Look(k) => choose(ui.ctx(), *k),
        }
    }
}
