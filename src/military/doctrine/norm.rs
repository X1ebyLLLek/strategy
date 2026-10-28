//! Norms of equipment: how much of the standard kit a soldier carries — and what it does.
//! A unit carries kit `q` (its equipment against the standard, 1): it fires `fire(q)` times as
//! hard, takes `tough(q)` times the harm to lose, and burns `eat(q)` times the ammunition and fuel.

/// The norms a template may give a kind, and their words.
pub const LEVELS: [f32; 3] = [0.5, 1.0, 1.5];
pub const WORDS: [&str; 3] = ["лёгкое", "штатное", "усиленное"];
// (the standard kit is exactly 1: the old world keeps its bits)
pub fn fire(q: f32) -> f32 {
    if q == 1.0 {
        1.0
    } else {
        q.max(0.0).powf(0.75)
    }
}
pub fn tough(q: f32) -> f32 {
    if q == 1.0 {
        1.0
    } else {
        (0.4 + 0.6 * q.max(0.0)).sqrt()
    }
}
pub fn eat(q: f32) -> f32 {
    if q == 1.0 {
        1.0
    } else {
        q.max(0.0).powf(0.75)
    }
}
/// The norm's word (nearest level).
pub fn word(k: f32) -> &'static str {
    let i = (0..LEVELS.len()).min_by(|&a, &b| (LEVELS[a] - k).abs().total_cmp(&(LEVELS[b] - k).abs())).unwrap_or(1);
    WORDS[i]
}
