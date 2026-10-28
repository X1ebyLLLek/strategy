//! The engine's self-checks and chronicles, a suite for each part of the engine:
//! `checks <suite> [its keys]`, the suites — `battle` (a battle), `doctrine` (doctrines grown from
//! experience), `front` (the front on the map, `--groups`: groups of armies), `corps` (the war of
//! divisions, `--checks`), `nation` (a country's market), `trade` (trade between countries),
//! `advisor` (economists), `homefront` (the home front and its sets: `--army`, `--econ`, …). What
//! each takes is said at the head of its file. `python tools/checks.py OUT` runs them all, each
//! era, one after another.
mod advisor;
mod common;
mod battle;
mod corps;
mod doctrine;
mod front;
mod homefront;
mod nation;
mod trade;

/// The program's arguments as the suite's own program would see them (its name left out).
pub fn args() -> Vec<String> {
    let mut a: Vec<String> = std::env::args().collect();
    if a.len() > 1 {
        a.remove(1);
    }
    a
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("advisor") => advisor::main(),
        Some("battle") => battle::main(),
        Some("corps") => corps::main(),
        Some("doctrine") => doctrine::main(),
        Some("front") => front::main(),
        Some("homefront") => homefront::main(),
        Some("nation") => nation::main(),
        Some("trade") => trade::main(),
        _ => {
            eprintln!("checks <suite> [keys]: {}", SUITES.join(", "));
            std::process::exit(2);
        }
    }
}

const SUITES: [&str; 8] = ["advisor", "battle", "corps", "doctrine", "front", "homefront", "nation", "trade"];
