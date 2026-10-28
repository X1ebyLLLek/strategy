//! What goes on inside a country, the player's and every other: its features (as HOI4's
//! national spirits, grown from facts), its society and order, its great task, a bot's course,
//! the order of its economy and its finances, its people of note, its techniques, the events
//! that come of all that.

pub mod spirits;
pub mod realm;
pub mod agenda;
pub mod course;
pub mod policy;
pub mod fiscal;
pub mod folk;
pub mod research;
pub mod events;
pub mod war_mood;

/// The economy's self-checks' control (`play --econtest --control`): the 27th session's mechanisms
/// switched off — the civil budget, the people's own births and deaths, the banks and bonds, each
/// force's measured lot, the order of labour and land — so every check of them must fail.
pub static ECON_SPOIL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn econ_spoiled() -> bool {
    ECON_SPOIL.load(std::sync::atomic::Ordering::Relaxed)
}
pub mod civil;
pub mod demography;
pub mod credit;
