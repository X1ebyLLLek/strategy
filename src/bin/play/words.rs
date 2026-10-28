//! The numbers of the game said in words, and small helpers the screens share.

use strategy::economy::advisor::Levers;
use strategy::pack::Pack;
use strategy::world::map;
pub use strategy::world::names::plural;

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub fn mix(pack: &Pack, sh: &[f32]) -> String {
    let mut order: Vec<usize> = (0..sh.len()).filter(|&u| sh[u] >= 0.03).collect();
    order.sort_by(|&a, &b| sh[b].total_cmp(&sh[a]));
    if order.is_empty() {
        return "—".into();
    }
    order.iter().map(|&u| format!("{} {:.0}%", esc(&pack.units[u].name), sh[u] * 100.0)).collect::<Vec<_>>().join(", ")
}

/// A division's name: its number and its template's ("12-я линейная дивизия").
pub fn division_name(no: u32, template: &str) -> String {
    let mut chars = template.trim().chars();
    let rest = match chars.next() {
        Some(c) => c.to_lowercase().collect::<String>() + chars.as_str(),
        None => "дивизия".to_string(),
    };
    format!("{no}-я {rest}")
}

/// An army (a general and his divisions, as in HOI4) by its number: `3-я армия`.
pub fn group_name(k: usize) -> String {
    format!("{}-я армия", k + 1)
}

/// The name of army `k` as what one goes into (`в 3-ю армию`), in small letters.
pub fn group_into(k: usize) -> String {
    format!("{}-ю армию", k + 1)
}

/// The Russian word for `n` things (`дивизия`, `дивизии`, `дивизий`).
/// «год», «года» or «лет» after a number of years.
pub fn years_word(n: u32) -> &'static str {
    plural(n as usize, "год", "года", "лет")
}

/// A month of the game as a date: "май 2-го года".
pub fn date_words(month: u32) -> String {
    const NAMES: [&str; 12] = ["январь", "февраль", "март", "апрель", "май", "июнь", "июль", "август", "сентябрь", "октябрь", "ноябрь", "декабрь"];
    format!("{} {}-го года", NAMES[month as usize % 12], month / 12 + 1)
}

/// How long someone has been in office, in words.
pub fn tenure(months: u32) -> String {
    const YEARS: [&str; 9] = ["два года", "три года", "четыре года", "пять лет", "шесть лет", "семь лет", "восемь лет", "девять лет", "десять лет"];
    match months {
        0..=1 => "первый месяц".into(),
        2..=4 => "несколько месяцев".into(),
        5..=8 => "около полугода".into(),
        9..=17 => "около года".into(),
        18..=119 => YEARS[((months + 6) / 12).clamp(2, 10) as usize - 2].into(),
        _ => "больше десяти лет".into(),
    }
}

/// What an economist does with the levers, in words against the custom (the numbers are
/// `Levers::describe`).
pub fn levers_words(l: &Levers, pack: &Pack) -> String {
    let tax = match l.tax {
        t if t < 0.8 => "налог ниже обычного",
        t if t <= 1.25 => "налог обычный",
        t if t <= 2.0 => "налог тяжёлый",
        _ => "налог грабительский",
    };
    if l.build <= 0.0 {
        return format!("{tax}, казна не строит");
    }
    let how = if l.build < 0.5 { "немного" } else if l.build < 0.9 { "много" } else { "сколько может" };
    let what = l.focus.map_or("где людям хуже".to_string(), |s| esc(&pack.sectors[s].name));
    format!("{tax}, казна строит {how}: {what}{}", if l.plan > 0.0 { ", по плану" } else { "" })
}

/// The kind of land, in words.
pub fn terrain_ru(t: map::Terrain) -> &'static str {
    use map::Terrain::*;
    match t {
        Ocean => "море",
        Lake => "озеро",
        Plains => "равнина",
        Forest => "лес",
        Hills => "холмы",
        Mountains => "горы",
        Desert => "пустыня",
        Marsh => "болото",
        Jungle => "джунгли",
        Tundra => "тундра",
        Arctic => "льды",
    }
}
