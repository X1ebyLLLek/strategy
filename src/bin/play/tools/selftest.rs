//! The clock's self-checks (`play --selftest`).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use strategy::pack::Pack;

use crate::{Game, SPEEDS};

/// The clock's self-checks: living by the clock is the same world as turn by turn; the clock
/// stops by itself when something matters, and only then; the speeds run as fast as they say.
pub fn selftest(pack: Pack, seed: u64, side: usize, path: &str) -> bool {
    let started = Instant::now();
    let mut checks: Vec<(String, bool, String)> = Vec::new();
    let base = Game::new(pack, seed, side);
    let fingerprint = |g: &Game| -> Vec<u32> {
        let mut v = Vec::new();
        for s in 0..2 {
            let n = g.h.home(s);
            v.push(n.gdp_real.to_bits());
            v.push(n.treasury.to_bits());
            v.extend(n.pop.iter().map(|x| x.to_bits()));
            v.extend(g.h.war.sides[s].army.count.iter().map(|x| x.to_bits()));
        }
        v
    };
    let levers = |army: &str| vec![("army".to_string(), army.to_string()), ("ctrl".into(), "staff".into())];
    // (the player answers his events at once, with their first choice; to see a war to its end,
    // with their last — a peace offered is refused)
    let answer = |g: &mut Game| {
        while let Some(id) = g.ev.waiting.first().map(|e| e.id) {
            g.answer(id, 0);
        }
    };
    let answer_last = |g: &mut Game| {
        while let Some((id, k)) = g.ev.waiting.first().map(|e| (e.id, e.choices.len().saturating_sub(1))) {
            g.answer(id, k);
        }
    };
    // 1. six months by the clock = six turns of a month
    let months = 6;
    let mut turns = base.clone();
    turns.apply(&levers("45"));
    for _ in 0..months {
        turns.advance();
        answer(&mut turns);
    }
    let mut live = base.clone();
    live.apply(&levers("45"));
    live.paused = false;
    while live.month() < months {
        live.tick();
        answer(&mut live);
        live.paused = false;
    }
    let mut other = base.clone();
    other.apply(&levers("10"));
    for _ in 0..months {
        other.advance();
        answer(&mut other);
    }
    let same = fingerprint(&turns) == fingerprint(&live);
    let differs = fingerprint(&turns) != fingerprint(&other);
    checks.push(("по часам — тот же мир, что по ходам".into(), same && differs, format!("до бита: {}; контроль (армии 10% вместо 45%) отличается: {}", same, differs)));
    // 2. the clock stops by itself exactly when something matters: a war to the end at top speed
    let mut war = base.clone();
    war.apply(&[("declare".to_string(), "1".to_string())]);
    war.paused = false;
    war.speed = 5;
    let (mut stops, mut silent, mut missed, mut alerts) = (0, 0, 0, Vec::new());
    let ended = |g: &Game| g.news.iter().any(|l| l.text.contains("капитулирует"));
    for _ in 0..(48 * 31) {
        if ended(&war) {
            break;
        }
        let before = (war.news.len(), war.month());
        war.tick();
        answer(&mut war);
        let month_done = war.month() != before.1 || ended(&war);
        if war.paused {
            stops += 1;
            if !month_done || war.alert.is_none() {
                silent += 1;
            }
            alerts.push(war.alert.clone().unwrap_or_default());
            war.paused = false;
            war.alert = None;
        } else if month_done {
            // a month that did not stop the clock: nothing in it should have
            let mine = war.h.home(war.me).name.clone();
            // (news of us, a capitulation, a capital fallen — not a crown fallen somewhere abroad)
            if war.news[before.0..].iter().any(|l| l.text.starts_with(&mine) || l.text.contains("капитулирует") || l.text.contains("пала столица")) {
                missed += 1;
            }
        }
    }
    let over = ended(&war);
    checks.push((
        "часы сами встают, когда важно, и только тогда".into(),
        stops > 0 && silent == 0 && missed == 0 && (!over || alerts.last().is_some_and(|a| a.contains("капитулирует"))),
        format!("остановок {stops} за {} мес войны (без причины {silent}, пропущено {missed}); война окончена: {over}; первая: {}", war.month(), alerts.first().cloned().unwrap_or_default()),
    ));
    // 4. peace after a capitulation: the war is over and the clock goes on; the land each side
    // holds is its own; the winner's people grew by what it holds of the loser's, which the loser
    // has no more (it had left the loser's people with the occupation); the world's money is what
    // it was (control: with occupation off nothing goes over, the winner's people stay)
    let to_peace = |occupation: bool| -> Option<(Game, f32, f32, f64, f64)> {
        let mut g = base.clone();
        g.h.occupation = occupation;
        // (the player's side strong, the other's army starved: a war that ends in a capitulation
        // whatever pair the map made)
        let foe = 1 - g.me;
        g.h.army_share[foe] = Some(0.1);
        g.h.home_mut(foe).mil_share = 0.1;
        g.apply(&[("army".to_string(), "80".to_string()), ("declare".to_string(), "1".to_string())]);
        for _ in 0..120 {
            // (the bot's course would give its army back its share: kept starved month by month)
            g.h.army_share[foe] = Some(0.1);
            g.h.home_mut(foe).mil_share = 0.1;
            let ppl = [g.h.home(0).people(), g.h.home(1).people()];
            let money = g.h.world.money_total();
            g.advance();
            answer_last(&mut g);
            if ended(&g) {
                let loser = if g.news.iter().any(|l| l.text.starts_with(&g.h.home(0).name) && l.text.contains("капитулирует")) { 0 } else { 1 };
                let w = 1 - loser;
                let grew = g.h.home(w).people() / ppl[w].max(1e-9);
                let fell = g.h.home(loser).people() / ppl[loser].max(1e-9);
                let gone = if g.h.home(loser).away.is_none() { 1.0 } else { 0.0 };
                return Some((g, grew, fell, money, gone));
            }
        }
        None
    };
    match (to_peace(true), to_peace(false)) {
        (Some((mut g, grew, fell, money, gone)), Some((_, ctrl, _, _, _))) => {
            let after = g.h.world.money_total();
            let same_money = ((after - money) / money.max(1e-9)).abs() < 1e-3;
            let land = g.h.war.th.home == g.h.war.owner;
            let quiet = !g.h.at_war && g.h.war.fallen.is_none();
            let m0 = g.month();
            g.paused = false;
            for _ in 0..40 {
                g.tick();
                g.paused = false;
            }
            let goes = g.month() > m0;
            checks.push((
                "после капитуляции — мир, и время идёт".into(),
                quiet && land && goes && same_money && grew > 1.02 && gone > 0.5 && (ctrl - 1.0).abs() < 0.02,
                format!(
                    "война кончилась на {}-м месяце; мир: {quiet}, земля чья занята: {land}, часы идут: {goes}; людей у победителя ×{grew:.2}, занятое у побеждённого ушло насовсем: {} (дома у него ×{fell:.2}); деньги мира те же: {same_money}; контроль (без оккупации) — у победителя ×{ctrl:.2}",
                    m0,
                    gone > 0.5
                ),
            ));
        }
        _ => checks.push(("после капитуляции — мир, и время идёт".into(), false, "война не дошла до капитуляции за 10 лет".into())),
    }
    // 3. the speeds: days a second as they say (a world that is quick to reckon: peace)
    let shared = Arc::new(Mutex::new(base.clone()));
    let mut rates = Vec::new();
    for speed in 1..=SPEEDS.len() {
        {
            let mut g = shared.lock().unwrap();
            g.speed = speed;
            g.paused = false;
        }
        let days0 = { let g = shared.lock().unwrap(); g.month() * 31 + g.day };
        let t0 = Instant::now();
        let span = Duration::from_millis(1500);
        while t0.elapsed() < span {
            let w = SPEEDS[shared.lock().unwrap().speed - 1];
            let t = Instant::now();
            {
                let mut g = shared.lock().unwrap();
                g.tick();
                answer(&mut g);
                // (what stops the clock in peace — an event, a claim — is not what is measured)
                g.paused = false;
            }
            std::thread::sleep(Duration::from_secs_f32(w).saturating_sub(t.elapsed()).max(Duration::from_millis(1)));
        }
        let g = shared.lock().unwrap();
        let days = (g.month() * 31 + g.day).saturating_sub(days0) as f32;
        rates.push(days / t0.elapsed().as_secs_f32());
    }
    let want: Vec<f32> = SPEEDS[..4].iter().map(|w| 1.0 / w).collect();
    let near = (0..4).all(|k| (rates[k] / want[k] - 1.0).abs() < 0.35);
    let rising = rates.windows(2).all(|w| w[1] > w[0]);
    checks.push((
        "скорости идут, как обещано".into(),
        near && rising,
        format!("дней в секунду: {} (обещано {}, 5-я — как успевает)", rates.iter().map(|r| format!("{r:.1}")).collect::<Vec<_>>().join(" / "), want.iter().map(|r| format!("{r:.0}")).collect::<Vec<_>>().join(" / ")),
    ));
    // 5. the player plays the country he chose at the start: a neighbour of the rival (not of the
    // pair the map makes) — the world built is his, a save keeps the choice and lives it again;
    // the world without the choice (the control: a start that forgets it) is not his
    {
        let mine = base.h.war.th.countries[base.me];
        let rival = base.h.war.th.countries[1 - base.me];
        let pick = base.neighbours().into_iter().find(|&c| c != mine && c != rival);
        match (pick, Pack::load(path)) {
            (Some(c), Ok(pack)) => {
                let mut g = Game::new_as(pack, seed, 0, Some(c));
                g.path = path.to_string();
                let ours = g.h.war.th.countries[g.me];
                let text = g.save_text();
                let back = Game::load_text(&text, &mut |_, _| {}).ok();
                let again = back.as_ref().map(|b| b.h.war.th.countries[b.me]);
                let forgot = base.h.war.th.countries[base.me];
                checks.push((
                    "играете за страну, выбранную в начале".into(),
                    ours == c && again == Some(c) && forgot != c && text.contains(&format!("country {c}")),
                    format!("выбрана {}, построена {}, из сохранения {}, без выбора {}", base.countries[c].0, base.countries[ours].0, again.map_or("—".to_string(), |x| base.countries[x].0.clone()), base.countries[forgot].0),
                ));
            }
            _ => checks.push(("играете за страну, выбранную в начале".into(), false, "у мира нет третьей страны рядом (или набор не открылся)".into())),
        }
    }
    println!("\nпроверки часов ({:.0} с):", started.elapsed().as_secs_f32());
    let mut ok = true;
    for (name, pass, detail) in &checks {
        ok &= pass;
        println!("  [{}] {:<44} {}", if *pass { "OK" } else { "FAIL" }, name, detail);
    }
    ok
}
