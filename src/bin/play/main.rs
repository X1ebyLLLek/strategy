//! The game: you rule one of two neighbouring countries of a generated map, in a world that
//! trades; bots rule the other. Time runs by itself, day by day (the world is reckoned month by
//! month): space pauses it, 1–5 set the speed, and what matters — a war, a capital lost, a
//! capitulation, unrest at home, a new design — pauses it by itself. You set the army's share of
//! what the state spends; the economy (the tax, how much of the rest goes into building and what
//! is built) you run yourself or give to an economist — one of three people whose qualities
//! nobody knows: you see only what he does and how the country fares; what the army builds you
//! choose yourself or leave to the general staff, which also leads the troops. You declare the
//! war. The other country's economy is run by an economist of its own. Science (when the pack has
//! it): you set its share of the state's spending; what the laboratories work on you choose
//! yourself or leave to a scientist — one of three, as hidden as the economists. Designers'
//! claims are news; what a design really is, only battles show.
//!
//! `cargo run --release --features native --bin play -- --native [--pack packs/x.toml] [--seed N]`
//! — the game in its window; on Windows: `play.bat`. `--selftest` — the clock's self-checks;
//! `--savetest` — the save's; `--auto [--war-at 36] [--war-months 60] [--log DIR]` — bots play
//! both sides, as fast as the world is reckoned, and the game is written down; `--world` — a
//! report of the world's past.
//!
//! Where what is: `game` (the state of the game, a day, a month; `game/apply` — the player's
//! decisions, `game/war` — war and peace, the war of divisions day by day); `country/` — what goes
//! on inside a country (features, society, the great task, a bot's course, economic order,
//! finances, people, techniques, events); `army/` — templates, models and factories, the draft;
//! `world/` — relations, neighbours, statecraft, the land; `tools/` — bot games, saves, self-checks,
//! the world's report; `native/` — the window; `words` — numbers said in words.
// (built without the window, much of what the game keeps for its screens is never read)
#![cfg_attr(not(feature = "native"), allow(dead_code))]

mod army;
mod country;
mod game;
#[cfg(feature = "native")]
mod native;
mod navy;
mod portraits;
mod tools;
mod words;
mod world;

/// The window's spoiled control is on (`play --native --uitest --control`): the parts made to be
/// checked behave as a naive version would.
#[cfg(feature = "native")]
pub fn spoiled() -> bool {
    native::spoiled()
}
#[cfg(not(feature = "native"))]
pub fn spoiled() -> bool {
    false
}

/// A laptop with two video cards gives the window the strong one when the program asks for it
/// by these numbers (read by the NVIDIA and AMD drivers; exported by `build.rs`).
#[no_mangle]
#[used]
pub static NvOptimusEnablement: u32 = 1;
#[no_mangle]
#[used]
pub static AmdPowerXpressRequestHighPerformance: u32 = 1;

#[cfg(feature = "native")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "native")]
use std::time::Duration;
use std::time::Instant;
use strategy::military::homefront::pick;
use strategy::pack::Pack;
use strategy::world::countries;
use strategy::world::map;

pub use game::Game;
use tools::{auto, saves, selftest, worldreport};

/// Months the world lives before the game: each country alone, then trading.
pub const ALONE: u32 = 240;
pub const TOGETHER: u32 = 240;
/// Months the two countries live in peace with their armies before the game begins.
pub const PREWAR: u32 = 48;

/// Real seconds a day takes at speeds 1–5 (at 5, as fast as the world can be reckoned).
pub const SPEEDS: [f32; 5] = [0.5, 0.25, 0.1, 0.04, 0.0];
/// Provinces of the front a sector holds, about.
pub const SECTOR: usize = 6;

/// The land provinces of the game's world (28th session: as HOI4 has them — about three times as
/// many, each a third the area, as the 3000 the core's checks keep). `PROVS=N` — another number,
/// to compare.
pub const PROVINCES: usize = 9000;

/// The parameters of the game's world map `seed`.
pub fn map_params(seed: u64) -> strategy::world::map::Params {
    let n = std::env::var("PROVS").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(PROVINCES);
    strategy::world::map::Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: n }
}

/// How many times smaller a province of the game's world is across than one of the core's (3000
/// on the world): the war's march, supply and depth reckoned in provinces are scaled by it.
pub fn prov_scale() -> f32 {
    (3000.0 / map_params(0).land_provinces as f32).sqrt()
}
/// Days of the months of a year.
pub const DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
pub const MONTHS: [&str; 12] = ["января", "февраля", "марта", "апреля", "мая", "июня", "июля", "августа", "сентября", "октября", "ноября", "декабря"];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str| args.iter().position(|a| a == k).map(|i| args[i + 1].clone());
    let path = arg("--pack").unwrap_or("packs/industrial.toml".into());
    // a new map every time, unless one is asked for
    let seed_asked: Option<u64> = arg("--seed").map(|v| v.parse().expect("--seed N"));
    let side: usize = arg("--side").map_or(0, |v| v.parse().expect("--side 0|1")).min(1);
    let pack = Pack::load(&path).unwrap_or_else(|e| panic!("{e}"));
    if args.iter().any(|a| a == "--world") {
        let first = arg("--seed").map_or(1, |v| v.parse().expect("--seed N"));
        let k = arg("--seeds").map_or(3, |v| v.parse().expect("--seeds N"));
        worldreport::report(&pack, first, k);
        return;
    }
    if args.iter().any(|a| a == "--selftest") {
        // (always the same map, unless one is asked for)
        let seed = arg("--seed").map_or(1, |v| v.parse().expect("--seed N"));
        std::process::exit(if selftest::selftest(pack, seed, side, &path) { 0 } else { 1 });
    }
    // the save's self-checks; a save loaded and checked (by the self-checks, in a process of its own)
    // (`ECON_SPOIL=1`: the 27th session's mechanisms off in any mode — for comparing bot games)
    if std::env::var_os("ECON_SPOIL").is_some() {
        country::ECON_SPOIL.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    if args.iter().any(|a| a == "--econtest") {
        let seed = arg("--seed").map_or(1, |v| v.parse().expect("--seed N"));
        if args.iter().any(|a| a == "--control") {
            country::ECON_SPOIL.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        std::process::exit(if tools::econtest::econtest(pack, seed) { 0 } else { 1 });
    }
    if args.iter().any(|a| a == "--savetest") {
        let seed = arg("--seed").map_or(1, |v| v.parse().expect("--seed N"));
        std::process::exit(if saves::savetest(pack, &path, seed) { 0 } else { 1 });
    }
    if let Some(file) = arg("--load-check") {
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        match Game::load_text(&text, &mut |_, _| {}) {
            Ok(g) => {
                println!("совпало: {}, отпечаток {:016x}", g.date(), g.fingerprint());
                std::process::exit(0);
            }
            Err(e) => {
                println!("не совпало: {e}");
                std::process::exit(1);
            }
        }
    }
    let auto = args.iter().any(|a| a == "--auto");
    let war_at: u32 = arg("--war-at").map_or(36, |v| v.parse().expect("--war-at N"));
    let war_months: u32 = arg("--war-months").map_or(60, |v| v.parse().expect("--war-months N"));
    if auto {
        // bots play, nobody looks: month after month, as fast as the world is reckoned
        let seed = seed_asked.unwrap_or_else(|| fresh_seed(&pack));
        let dir = arg("--log").map_or_else(|| log_dir(&path, seed), std::path::PathBuf::from);
        println!("строю мир (карта {seed}, {}), ~10 с...", pack.name);
        let t0 = Instant::now();
        let mut game = Game::new(pack, seed, side);
        game.path = path.clone();
        game.start_bots(auto::Auto::new(war_at, war_months, dir.clone()));
        println!("мир готов за {:.0} с; играют боты, лог: {}", t0.elapsed().as_secs_f32(), dir.display());
        let t1 = Instant::now();
        loop {
            game.advance();
            game.after_month();
            if game.month() % 12 == 0 {
                println!("  {}-й месяц, {:.0} с", game.month(), t1.elapsed().as_secs_f32());
            }
            if game.auto.as_ref().is_some_and(|a| a.done) {
                break;
            }
        }
        println!("игра окончена на {}-м месяце за {:.0} с", game.month(), t1.elapsed().as_secs_f32());
        return;
    }
    // the game in a window of its own (built with `--features native`)
    #[cfg(not(feature = "native"))]
    if args.iter().any(|a| a == "--native") {
        eprintln!("эта сборка без окна игры: соберите с --features native (так собирает play.bat)");
        std::process::exit(2);
    }
    #[cfg(feature = "native")]
    if args.iter().any(|a| a == "--native") {
        // a saved game (`--load FILE`), or a new world
        let mut game = match arg("--load") {
            Some(file) => {
                println!("загружаю {file}: мир строится и проживается заново...");
                let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
                Game::load_text(&text, &mut |m, of| {
                    if m % 12 == 0 {
                        println!("  {m} мес. из {of}");
                    }
                })
                .unwrap_or_else(|e| panic!("{e}"))
            }
            None => {
                let seed = seed_asked.unwrap_or_else(|| fresh_seed(&pack));
                println!("строю мир (карта {seed}, {}), ~15 с...", pack.name);
                let mut game = Game::new(pack, seed, side);
                game.path = path.clone();
                game
            }
        };
        // (for pictures: a war declared and some months of it lived, the player's events answered;
        // the window's self-checks look at a war of four months)
        let uitest = args.iter().any(|a| a == "--uitest");
        // (for pictures: the government turned on another neighbour; a line of supply laid from
        // the capital to the province of ours farthest from it)
        if args.iter().any(|a| a == "--rival") {
            let mine = game.h.war.th.countries[game.me];
            let now = game.h.war.th.countries[1 - game.me];
            if let Some(&c) = game.neighbours().iter().find(|&&c| c != now && c != mine) {
                game.act(saves::Act::Rival(c));
            }
        }
        if args.iter().any(|a| a == "--lay") {
            let me = game.me;
            let w = &game.h.war;
            let cap = w.th.capital[me];
            let d = w.distance_from(cap);
            let far = (0..w.owner.len()).filter(|&q| w.owner[q] == me && d[q] != strategy::military::front::NO && d[q] as usize <= 10).max_by_key(|&q| d[q]);
            if let Some(path) = far.and_then(|q| world::land::plan(w, me, cap, q)) {
                let rail = game.has_rails();
                game.h.home_mut(me).treasury += 1e3;
                game.act(saves::Act::Line(rail, path));
            }
        }
        // (for pictures: an economist put in charge of the economy — `adv0`…`adv2` — or the player himself, `me`)
        if let Some(v) = arg("--econ") {
            game.act(saves::Act::Form(vec![("econ".to_string(), v)]));
        }
        // (for pictures: months of peace lived first — another time of year)
        if let Some(m) = arg("--peace").and_then(|v| v.parse::<u32>().ok()) {
            for _ in 0..m {
                game.advance();
                while let Some(id) = game.ev.waiting.first().map(|e| e.id) {
                    game.act(saves::Act::Answer(id, 0));
                }
            }
            game.alert = None;
        }
        if let Some(m) = arg("--prep-war").and_then(|v| v.parse::<u32>().ok()).or(uitest.then_some(4)) {
            // (decided as the player decides: into the journal, so that a save of it holds)
            game.act(saves::Act::Form(vec![("declare".to_string(), "1".to_string())]));
            for _ in 0..m {
                // (the player's economy goes on a war footing as the other's does: else the side the
                // bots lead for him fought on a peace economy against a mobilised one)
                let law = game.bot_law(game.me);
                if law != game.policy.law[game.me] {
                    game.act(saves::Act::Form(vec![("ecolaw".to_string(), law.to_string())]));
                }
                game.advance();
                // (the war goes on: a peace is refused, the rest takes the first choice)
                while let Some((id, kind)) = game.ev.waiting.first().map(|e| (e.id, e.kind)) {
                    let keep = matches!(kind, country::events::Kind::PeaceOffer | country::events::Kind::Weary);
                    game.act(saves::Act::Answer(id, keep as usize));
                }
            }
            game.alert = None;
        }
        // (for pictures: an event waiting for the player)
        if args.iter().any(|a| a == "--event") {
            let ev = game.event_of(game.me, country::events::Kind::Treasury);
            game.ev.waiting.push(ev);
        }
        // (for pictures: the game saved into a file before the window opens)
        if let Some(file) = arg("--write-save") {
            std::fs::write(&file, game.save_text()).unwrap_or_else(|e| panic!("{file}: {e}"));
        }
        // (for pictures: the clock running at the fastest)
        if args.iter().any(|a| a == "--play") {
            game.paused = false;
            game.speed = 5;
        }
        let shot = arg("--shot").map(|f| native::Shot {
            file: f.into(),
            frames: arg("--frames").and_then(|v| v.parse().ok()).unwrap_or(40),
            tab: arg("--tab"),
            zoom: arg("--zoom").and_then(|v| v.parse().ok()).unwrap_or(1.0),
            mode: arg("--mode"),
            design: arg("--design").and_then(|v| v.parse().ok()),
            menu: args.iter().any(|a| a == "--menu"),
            center: arg("--center"),
            hover: arg("--hover").and_then(|v| {
                let (x, y) = v.split_once(',')?;
                Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
            }),
            cursors: args.iter().any(|a| a == "--cursors"),
            pick: args.iter().any(|a| a == "--pick"),
            choose: arg("--choose").and_then(|v| v.parse().ok()),
            draft: args.iter().any(|a| a == "--draft"),
            fronts: args.iter().any(|a| a == "--fronts"),
            tour: arg("--tour").map_or_else(Vec::new, |v| v.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()),
            guide: arg("--guide").and_then(|v| v.parse().ok()),
            size: arg("--size").and_then(|v| {
                let (x, y) = v.split_once('x')?;
                Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
            }),
        });
        let shared = Arc::new(Mutex::new(game));
        if uitest {
            let ok = native::uitest(shared, args.iter().any(|a| a == "--control"));
            std::process::exit(if ok { 0 } else { 1 });
        }
        let clock = Arc::clone(&shared);
        std::thread::spawn(move || loop {
            let wait = {
                let g = clock.lock().unwrap();
                if g.paused { None } else { Some(SPEEDS[g.speed - 1]) }
            };
            match wait {
                None => std::thread::sleep(Duration::from_millis(30)),
                Some(w) => {
                    let t0 = Instant::now();
                    clock.lock().unwrap().tick();
                    std::thread::sleep(Duration::from_secs_f32(w).saturating_sub(t0.elapsed()).max(Duration::from_millis(1)));
                }
            }
        });
        // (the start of a game: the player chooses the age, the map, his country — unless a save is
        // loaded, a picture taken or the window checks itself; `--start` shows it in a picture)
        let start = (shot.is_none() && arg("--load").is_none() && !args.iter().any(|a| a == "--uitest" || a == "--no-start")) || args.iter().any(|a| a == "--start");
        if let Err(e) = native::run(shared, shot, start) {
            eprintln!("окно игры не открылось: {e}");
        }
        return;
    }
    eprintln!("игра — в окне: play.bat (или --native в сборке с --features native); --auto — партия ботов, --selftest, --savetest — самопроверки");
    std::process::exit(2);
}

/// Where a game of bots is written down: out/botgames/<pack>_<map>.
fn log_dir(pack: &str, seed: u64) -> std::path::PathBuf {
    let stem = std::path::Path::new(pack).file_stem().map_or("pack".into(), |s| s.to_string_lossy().into_owned());
    std::path::PathBuf::from("out").join("botgames").join(format!("{stem}_{seed}"))
}

/// A map not played before (by the clock), with two neighbours to play.
pub fn fresh_seed(pack: &Pack) -> u64 {
    let mut seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64 % 1_000_000 + 1);
    while pick_map(pack, seed).is_none() {
        seed += 1;
    }
    seed
}

/// Whether map `seed` has two neighbours to play.
fn pick_map(pack: &Pack, seed: u64) -> Option<(usize, usize)> {
    let world = map::generate(&map_params(seed)).0;
    let pol = countries::generate(&world, seed, strategy::world::countries::COUNTRIES, &pack);
    pick(&world, &pol)
}
