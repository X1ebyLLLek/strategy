//! The forecast shared by the ledger («Сводка») and the economy's overview: the same world lived a
//! year ahead with one decision changed against none (the rival and luck live too: it is the game
//! itself, not a formula; only that no war begins in it, which would put its own footing over every
//! decision), so that the player sees what a decision does before he takes it. It is
//! reckoned in the background, on every core; the folders read what it has found.

use std::sync::{Mutex, OnceLock};

use rayon::prelude::*;
use super::*;

/// What a decision did after a year.
#[derive(Clone, Debug, Default)]
pub(super) struct Outcome {
    /// What the overview looks the decision up by.
    pub key: &'static str,
    pub name: String,
    pub about: String,
    pub gdp: f32,
    pub treasury: f32,
    pub anger: f32,
    pub stability: f32,
    pub legit: f32,
    pub unrest: f32,
    pub techs: f32,
    pub army: f32,
    pub level: f32,
    pub science: f32,
    pub idle: f32,
    pub hunger: f32,
    /// What the treasury took in taxes and in all, and what it gave the army, in the last month.
    pub tax: f32,
    pub income: f32,
    pub army_money: f32,
    /// The tax rate it ended with (for the record).
    pub rate: f32,
}

/// What the forecast has: the game month and the stamp of decisions it was reckoned from, whether it
/// is being reckoned, the outcomes (the first is «nothing changed»), the seconds it took.
#[derive(Clone, Default)]
pub(super) struct Forecast {
    pub month: u32,
    pub stamp: u64,
    pub running: bool,
    pub rows: Vec<Outcome>,
    pub secs: f32,
}

impl Forecast {
    /// The outcome of the decision `key` and the one of doing nothing, when both are reckoned.
    pub fn pair(&self, key: &str) -> Option<(&Outcome, &Outcome)> {
        let base = self.rows.first()?;
        Some((self.rows.iter().find(|o| o.key == key)?, base))
    }

    /// Reckoned for another month or other decisions than the game has now.
    pub fn stale(&self, g: &Game) -> bool {
        !self.rows.is_empty() && (self.month != g.month() || self.stamp != g.stamp)
    }
}

fn shared() -> &'static Mutex<Forecast> {
    static F: OnceLock<Mutex<Forecast>> = OnceLock::new();
    F.get_or_init(|| Mutex::new(Forecast::default()))
}

/// The forecast as it stands.
pub(super) fn state() -> Forecast {
    shared().lock().unwrap().clone()
}

type Form = Vec<(String, String)>;

/// The decisions tried: what the overview looks them up by, a name, what it is, the form the window
/// would send.
fn options(g: &Game) -> Vec<(&'static str, String, String, Form)> {
    let me = g.me;
    let n = g.h.home(me);
    let mut v: Vec<(&'static str, String, String, Form)> = vec![("base", "Ничего не менять".into(), "как сейчас — с чем сравнивается всё остальное".into(), Vec::new())];
    if g.adviser.is_none() {
        let t = (n.tax * 100.0).round();
        v.push(("tax+", "Налог +5".into(), format!("налог {:.0}% вместо {t:.0}%", t + 5.0), vec![("tax".into(), format!("{}", t + 5.0))]));
        if t >= 5.0 {
            v.push(("tax-", "Налог −5".into(), format!("налог {:.0}% вместо {t:.0}%", t - 5.0), vec![("tax".into(), format!("{}", t - 5.0))]));
        }
        if n.build_share < 0.95 {
            v.push(("build+", "Всё в стройку".into(), format!("стройка 100% свободных денег казны вместо {:.0}%", n.build_share * 100.0), vec![("build".into(), "100".into())]));
        }
        if n.build_share > 0.05 {
            v.push(("build-", "Не строить".into(), format!("казна не строит вместо {:.0}%", n.build_share * 100.0), vec![("build".into(), "0".into())]));
        }
    }
    if g.h.labs[me].is_some() {
        let sci = (n.science_share * 100.0).max(0.5);
        let to = (sci * 2.0).max(sci + 3.0).min(50.0);
        v.push(("sci+", "Наука ×2".into(), format!("доля науки {to:.0}% вместо {sci:.0}%"), vec![("science".into(), format!("{to:.1}"))]));
    }
    let army = (n.mil_share * 100.0).round();
    v.push(("army+", "Армия +10".into(), format!("доля армии {:.0}% вместо {army:.0}%", (army + 10.0).min(90.0)), vec![("army".into(), format!("{}", (army + 10.0).min(90.0)))]));
    if army >= 10.0 {
        v.push(("army-", "Армия −10".into(), format!("доля армии {:.0}% вместо {army:.0}%", army - 10.0), vec![("army".into(), format!("{}", army - 10.0))]));
    }
    if g.fiscal.tariff[me] != 2 {
        v.push(("tariff+", "Защита промышленности".into(), "высокие пошлины на чужие товары".into(), vec![("fx_tariff".into(), "2".into())]));
    }
    if g.fiscal.tariff[me] != 0 {
        v.push(("tariff-", "Свободная торговля".into(), "без пошлин".into(), vec![("fx_tariff".into(), "0".into())]));
    }
    if g.fiscal.rate[me] != 0 {
        v.push(("rate-", "Дешёвый кредит".into(), "банки дают в долг почти даром".into(), vec![("fx_rate".into(), "0".into())]));
    }
    if g.fiscal.rate[me] < 2 {
        v.push(("rate+", "Дорогие деньги".into(), "банки требуют долги назад, кредит дорог".into(), vec![("fx_rate".into(), "2".into())]));
    }
    if !g.fiscal.caps[me] {
        v.push(("caps+", "Твёрдые цены на хлеб".into(), "цена нужного не выше положенной".into(), vec![("fx_caps".into(), "1".into())]));
    }
    if g.realm.crackdown[me] == 0 {
        v.push(("crack", "Твёрдая рука".into(), "год жёсткого порядка".into(), vec![("crackdown".into(), "1".into())]));
    }
    v
}

/// The world `g` lived `months` ahead with `form` decided now; what it came to.
fn live(mut w: Game, form: &[(String, String)], months: u32) -> Outcome {
    w.auto = None;
    // (a lever is judged by what it does to the economy, all else alike: no war is begun in the
    // years ahead — a war's footing would put its own tax and army over every decision)
    if !w.h.at_war {
        w.state.truce_until = w.state.truce_until.max(w.month() + months + 1);
        w.state.claim = [None, None];
    }
    if !form.is_empty() {
        w.apply(form);
    }
    for _ in 0..months {
        w.advance();
        if w.h.war.fallen.is_some() {
            break;
        }
    }
    let me = w.me;
    let c = w.country(me);
    let b = w.rows.last().map(|r| r.book[me]).unwrap_or_default();
    Outcome {
        gdp: w.h.home(me).gdp_real,
        treasury: w.h.home(me).treasury,
        anger: w.h.home(me).anger(),
        stability: w.state.stability[me],
        legit: w.polity.of[c].legit,
        unrest: w.polity.of[c].unrest,
        techs: w.know.get(c).map_or(0.0, |k| k.count() as f32),
        army: w.h.war.sides[me].army.count.iter().sum(),
        level: w.h.home(me).level,
        science: w.research.points[me],
        idle: w.figures(me).idle,
        hunger: w.spirits.facts[me].hunger,
        tax: b.tax,
        income: b.income(),
        army_money: b.army + b.soldiers,
        rate: w.h.home(me).tax,
        ..Default::default()
    }
}

/// Start reckoning the forecast for game `g` (in the background); nothing if it is being reckoned.
pub(super) fn start(g: &Game) {
    {
        let mut f = shared().lock().unwrap();
        if f.running {
            return;
        }
        f.running = true;
        f.month = g.month();
        f.stamp = g.stamp;
    }
    let base = g.clone();
    std::thread::spawn(move || {
        let t0 = std::time::Instant::now();
        let opts = options(&base);
        // (a world of its own for each decision: the game is not to be shared between threads)
        let worlds: Vec<(Game, (&'static str, String, String, Form))> = opts.into_iter().map(|o| (base.clone(), o)).collect();
        let rows: Vec<Outcome> = worlds
            .into_par_iter()
            .map(|(w, (key, name, about, form))| {
                let mut o = live(w, &form, 12);
                o.key = key;
                o.name = name;
                o.about = about;
                o
            })
            .collect();
        if std::env::var("FCDBG").is_ok() {
            for o in &rows {
                eprintln!("прогноз {:<24} выпуск {:.2} казна {:.2} ставка {:.3} налоги {:.2} доход {:.2} армии {:.2} гнев {:.3} без работы {:.3} цены {:.3} нужда {:.3}", o.name, o.gdp, o.treasury, o.rate, o.tax, o.income, o.army_money, o.anger, o.idle, o.level, o.hunger);
            }
        }
        let mut f = shared().lock().unwrap();
        f.rows = rows;
        f.running = false;
        f.secs = t0.elapsed().as_secs_f32();
    });
}
