//! The window's self-checks of the economy's overview (29th session): a tile of a vital sign opens
//! its breakdown, the lever in the breakdown moves the tax, a button of the advice does what it says,
//! an economist is appointed by a click, the levers tell what a year of a step would do.

use super::policy::{click, close, find, stepper};
use super::*;
use crate::tools::saves::Act;

/// The folder turned back to its top (the scroll of a page stays where the last check left it).
fn to_top(step: &mut impl FnMut(&mut App, Vec<egui::Event>), app: &mut App, on: usize, ctx: &egui::Context) {
    app.tab = Some(Tab::Econ);
    app.menu = false;
    app.chosen = None;
    super::super::set_page(ctx, Tab::Econ, on);
    let over = Pos2::new(190.0, 520.0);
    for _ in 0..30 {
        step(app, vec![egui::Event::PointerMoved(over), egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, 400.0), modifiers: Default::default() }]);
    }
    for _ in 0..30 {
        step(app, vec![]);
    }
}

/// The folder turned down, a little at a time, till a text that holds `needle` (in lower case) is in
/// sight; whether it came.
fn scroll_to(step: &mut impl FnMut(&mut App, Vec<egui::Event>), app: &mut App, ctx: &egui::Context, needle: &str) -> bool {
    let over = Pos2::new(190.0, 520.0);
    for _ in 0..40 {
        let out = frame_at(ctx, app, 1600.0, 900.0, vec![egui::Event::PointerMoved(over)], ctx.input(|i| i.time) + 1.0 / 60.0);
        if texts(&out.shapes).iter().any(|t| t.text.to_lowercase().contains(needle)) {
            return true;
        }
        step(app, vec![egui::Event::PointerMoved(over), egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, -90.0), modifiers: Default::default() }]);
        for _ in 0..4 {
            step(app, vec![]);
        }
    }
    false
}

/// 39. the vital signs: a click on the tile of «Нужда» opens its breakdown («Разбор: нужда») — the one that
/// was open (the treasury's) gives way to it.
pub(super) fn vitals(app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let mut step = stepper(ctx);
    super::super::panels::forget_vital(ctx);
    to_top(&mut step, app, super::super::panels::economy::OVERVIEW, ctx);
    let tile = find(&mut step, app, ctx, Tab::Econ, "vital4");
    let mut now = false;
    if let Some(r) = tile {
        click(&mut step, app, r.center());
        for _ in 0..4 {
            step(app, vec![]);
        }
        now = scroll_to(&mut step, app, ctx, "разбор: нужда");
    }
    super::super::panels::forget_vital(ctx);
    close(&mut step, app);
    say(
        tile.is_some() && now,
        "обзор: щелчок по плитке показателя открывает его разбор",
        format!("плитка «Нужда» {}; после щелчка разбор «Нужда» {}", if tile.is_some() { "на виду" } else { "не видна" }, if now { "открыт" } else { "не открыт" }),
    );
}

/// 40. the levers of the breakdown: in the breakdown of the treasury the + of the tax — the decision in
/// the journal, the tax a point higher.
pub(super) fn tax_lever(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let mut step = stepper(ctx);
    {
        let mut g = shared.lock().unwrap();
        g.adviser = None;
    }
    super::super::panels::forget_vital(ctx);
    to_top(&mut step, app, super::super::panels::economy::OVERVIEW, ctx);
    let plus = find(&mut step, app, ctx, Tab::Econ, "ov_tax_plus");
    let (me, before, now) = {
        let g = shared.lock().unwrap();
        (g.me, g.journal.len(), g.h.home(g.me).tax)
    };
    let want = format!("{:.0}", ((now * 100.0).round() + 1.0).clamp(0.0, 80.0));
    let (mut sent, mut lives) = (false, false);
    if let Some(r) = plus {
        click(&mut step, app, r.center());
        let g = shared.lock().unwrap();
        sent = g.journal[before..].iter().any(|x| matches!(&x.2, Act::Form(f) if f.iter().any(|(a, b)| a == "tax" && *b == want)));
        lives = (g.h.home(me).tax * 100.0 - want.parse::<f32>().unwrap_or(-1.0)).abs() < 0.01;
    }
    {
        let mut g = shared.lock().unwrap();
        g.h.home_mut(me).tax = now;
    }
    close(&mut step, app);
    say(
        plus.is_some() && sent && lives,
        "обзор: «+» налога в разборе казны — решение в журнале, налог на пункт выше",
        match plus {
            Some(_) => format!("налог {}% → {want}%: в журнале {}, в стране {}", (now * 100.0).round(), if sent { "есть" } else { "нет" }, if lives { "действует" } else { "не действует" }),
            None => "кнопки «+» у налога в разборе казны не видно".into(),
        },
    );
}

/// 41. the advice: its first button does what it says — a decision into the journal, a page of the
/// folder turned, or a vital sign opened.
pub(super) fn advice(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let mut step = stepper(ctx);
    // (a treasury in debt: the advice of the first rank is there)
    let saved = {
        let mut g = shared.lock().unwrap();
        g.adviser = None;
        let me = g.me;
        let t = g.h.home(me).treasury;
        g.h.home_mut(me).treasury = -50.0;
        t
    };
    super::super::panels::forget_vital(ctx);
    to_top(&mut step, app, super::super::panels::economy::OVERVIEW, ctx);
    let button = find(&mut step, app, ctx, Tab::Econ, "advice_button");
    let (journal, state) = {
        let g = shared.lock().unwrap();
        (g.journal.len(), g.h.home(g.me).tax)
    };
    let mut did = false;
    if let Some(r) = button {
        click(&mut step, app, r.center());
        for _ in 0..4 {
            step(app, vec![]);
        }
        let g = shared.lock().unwrap();
        did = g.journal.len() > journal;
    }
    {
        let mut g = shared.lock().unwrap();
        let me = g.me;
        g.h.home_mut(me).treasury = saved;
        g.h.home_mut(me).tax = state;
    }
    close(&mut step, app);
    say(
        button.is_some() && did,
        "обзор: у казны в долгах первая кнопка совета («Налог +1 п.») — решение в журнале",
        match button {
            Some(_) => format!("в журнале {}", if did { "новое решение" } else { "ничего нового" }),
            None => "кнопки совета не видно".into(),
        },
    );
}

/// 42. the economist: a click on «Поставить советника», then «Назначить» at the first of those who
/// might — the decision in the journal, the economy in his hands; then the way back.
pub(super) fn appoint(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let mut step = stepper(ctx);
    {
        let mut g = shared.lock().unwrap();
        g.adviser = None;
    }
    super::super::panels::forget_vital(ctx);
    to_top(&mut step, app, super::super::panels::economy::OVERVIEW, ctx);
    let button = find(&mut step, app, ctx, Tab::Econ, "govern_button");
    let before = shared.lock().unwrap().journal.len();
    let mut appointed = false;
    let mut listed = false;
    if let Some(r) = button {
        click(&mut step, app, r.center());
        for _ in 0..4 {
            step(app, vec![]);
        }
        let spot = find(&mut step, app, ctx, Tab::Econ, "appoint");
        listed = spot.is_some();
        if let Some(r) = spot {
            click(&mut step, app, r.center());
            let g = shared.lock().unwrap();
            appointed = g.adviser == Some(0) && g.journal[before..].iter().any(|x| matches!(&x.2, Act::Form(f) if f.iter().any(|(a, b)| a == "econ" && b == "adv0")));
        }
    }
    {
        let mut g = shared.lock().unwrap();
        g.act(Act::Form(vec![("econ".to_string(), "me".to_string())]));
        g.journal.truncate(before);
    }
    close(&mut step, app);
    say(
        button.is_some() && listed && appointed,
        "советник: щелчок «Поставить советника», затем «Назначить» — решение в журнале, хозяйство у него",
        format!("кнопка {}; список {}; назначен {}", if button.is_some() { "на виду" } else { "не видна" }, if listed { "открылся" } else { "не открылся" }, if appointed { "да" } else { "нет" }),
    );
}

/// 43. the forecast under the levers: with the breakdown of the treasury open, the tax's lever tells what
/// a year of five points more would do (the forecast is reckoned in the background: it is waited for).
pub(super) fn forecast(shared: &Arc<Mutex<Game>>, app: &mut App, ctx: &egui::Context, say: &mut dyn FnMut(bool, &str, String) -> bool) {
    let mut step = stepper(ctx);
    {
        let mut g = shared.lock().unwrap();
        g.adviser = None;
    }
    super::super::panels::forget_vital(ctx);
    let mut seen = false;
    let mut waited = 0;
    to_top(&mut step, app, super::super::panels::economy::OVERVIEW, ctx);
    // (the folder turned down till the tax's lever is in sight; the forecast waited for, for a minute at most)
    let _ = find(&mut step, app, ctx, Tab::Econ, "ov_tax_plus");
    while waited < 240 && !seen {
        std::thread::sleep(std::time::Duration::from_millis(250));
        waited += 1;
        for _ in 0..3 {
            step(app, vec![]);
        }
        let out = frame_at(ctx, app, 1600.0, 900.0, vec![], ctx.input(|i| i.time) + 1.0 / 60.0);
        seen = texts_of(&out.shapes, true).iter().any(|t| t.text.contains("+5 п.:"));
    }
    close(&mut step, app);
    say(seen, "обзор: у рычага налога сказано, что даст год «+5 п.» (прогноз посчитан в фоне)", format!("строка есть: {}; ждали {:.0} с", if seen { "да" } else { "нет" }, waited as f32 * 0.25));
}
