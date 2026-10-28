//! The chamber in the folder of society: who holds its seats, how many of them stand for the
//! government now (each force by its mood, the economy by prices, work and growth), when the next
//! election comes. A country where nobody is asked has none.

use strategy::world::polity::{BLOC, BLOCS};

use super::*;
use super::super::charts::pct;
use super::super::kit::{self, Stat};

/// The chamber's block (nothing where the country has none).
pub(super) fn chamber(ui: &mut egui::Ui, s: &Skin, g: &mut Game) {
    let me = g.me;
    let Some(seats) = g.seats(me) else { return };
    kit::ribbon(ui, "Собрание и выборы", Some("vote"));
    let share = g.vote_share(me).unwrap_or(0.5);
    let econ = g.economy_vote(me);
    let now = g.month();
    let next = g.realm.election[me];
    let elected = g.polity_of(me).regime.elected().is_some();
    let sup = g.support(me);
    let items = vec![
        Stat::new("vote", "За правительство", pct(share), if share >= 0.55 { kit::GOOD } else if share < 0.5 { kit::BAD } else { kit::MIXED }),
        Stat::new("chart", "Хозяйство на выборах", format!("{:+.0}", econ * 100.0), if econ > 0.05 { kit::GOOD } else if econ < -0.05 { kit::BAD } else { kit::PLAIN }),
        Stat::new("hourglass", "До выборов", if elected && next > now { format!("{} мес.", next - now) } else { "не выбирают".to_string() }, kit::PLAIN),
    ];
    let resp = kit::stats(ui, &items, 3);
    let mut order: Vec<usize> = (0..BLOCS).filter(|&b| seats[b] > 0.005).collect();
    order.sort_by(|&a, &b| seats[b].total_cmp(&seats[a]));
    let rows: Vec<(String, f32, f32)> = order.iter().map(|&b| (BLOC[b].name.to_string(), seats[b], sup[b])).collect();
    let r0 = rows.clone();
    let cards: [Box<dyn Fn() -> Card>; 3] = [
        Box::new(move || {
            let mut c = Card::new("Собрание").icon("vote").row("за правительство", pct(share));
            for (name, seat, s) in &r0 {
                c = c.tone(format!("{name}: мест {}", pct(*seat)), format!("за {}", pct(*s)), if *s >= 0.5 { Tone::Good } else { Tone::Bad });
            }
            c.note("каждая сила голосует по своему настроению и по хозяйству; реформу без большинства можно провести только указом")
        }),
        Box::new(move || Card::new("Хозяйство на выборах").icon("chart").row("сдвиг голосов", format!("{:+.0}", econ * 100.0)).text("Цены и безработица против правительства, рост — за него: так голосуют везде, где голосуют.")),
        Box::new(move || Card::new("Выборы").icon("hourglass").text("Проиграв, правительство уступает кабинет оппозиции — или распускает собрание и теряет законность.")),
    ];
    for (r, cd) in resp.into_iter().zip(cards.iter()) {
        r.tip(|| cd());
    }
    // the seats as a band
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 14.0), egui::Sense::hover());
    let mut x = r.min.x;
    for (name, seat, sp) in &rows {
        let ww = w * seat;
        let col = if *sp >= 0.5 { kit::GOOD } else { kit::BAD };
        ui.painter().rect_filled(egui::Rect::from_min_size(pos2(x, r.min.y), vec2((ww - 1.0).max(0.0), r.height())), 2.0, col.gamma_multiply(0.35 + 0.65 * (sp - 0.5).abs() * 2.0));
        let _ = name;
        x += ww;
    }
    kit::words(ui, "Зелёным — места тех, кто сейчас за правительство, красным — против.", 12.0, s.soft);
}
