//! Saving and loading a game. The world is the same for the same map, era and decisions (the
//! clock's self-check shows it to the bit), so a save is what made it: the pack, the map's seed,
//! the player's side and every decision of the player with the day it was taken (the journal);
//! and the world's fingerprint on the day of the save. Loading builds the same world, lives it
//! again with the same decisions on the same days, and checks the fingerprint.

use strategy::rng::{hash64, Rng};

use crate::Game;

/// A decision of the player (whatever he decides goes through `Game::act`).
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    /// The levers and orders, as keys and values (`Game::apply`).
    Form(Vec<(String, String)>),
    /// An event answered: its number, the choice.
    Answer(u64, usize),
    /// A claim on the neighbour begun; a peace offered; a depot laid in a province of the theatre.
    Claim,
    Peace,
    Depot(usize),
    /// A line of supply begun: a railway (or a road) through these provinces of the theatre.
    Line(bool, Vec<usize>),
    /// The government turns on another neighbour (a map country).
    Rival(usize),
    /// Divisions (their ids) sent to a province of the theatre by the player himself; told to stop
    /// where they are; given to a group of armies (usize::MAX: the reserve), back under its
    /// general.
    Send(Vec<u32>, usize),
    Halt(Vec<u32>),
    Assign(Vec<u32>, usize),
    /// Divisions sent on from where their ways end (one stop more); a new group of armies formed
    /// of these divisions; a group given its own stretch of the front, drawn through these
    /// provinces of the theatre.
    SendOn(Vec<u32>, usize),
    Raise(Vec<u32>),
    Sector(usize, Vec<usize>),
    /// Groups of armies given a line through these provinces of the theatre, cut among them along
    /// it; the country of the world it faces when not the enemy of the war.
    Front(Vec<usize>, Option<u32>, Vec<usize>),
}

fn ids(v: &[u32]) -> String {
    v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

fn ids_of(s: &str) -> Option<Vec<u32>> {
    s.split(',').filter(|x| !x.is_empty()).map(|x| x.parse::<u32>().ok()).collect()
}

/// The folder of the saves (beside the packs).
pub const DIR: &str = "saves";

impl Game {
    /// The player decides: the decision is written into the journal with the day, then done;
    /// whether it was done (a claim, a peace, a depot may be refused).
    pub fn act(&mut self, a: Act) -> bool {
        self.journal.push((self.month(), self.day, a.clone()));
        match a {
            Act::Form(f) => {
                self.apply(&f);
                true
            }
            Act::Answer(id, k) => {
                self.answer(id, k);
                true
            }
            Act::Claim => self.claim(),
            Act::Peace => {
                let me = self.me;
                self.offer_peace(me)
            }
            Act::Depot(q) => self.build_depot(q),
            Act::Line(rail, path) => {
                let me = self.me;
                self.begin_line(me, rail, path)
            }
            Act::Rival(c) => self.set_rival(c),
            Act::Send(v, q) => {
                let me = self.me;
                self.h.war.send(me, &v, q) > 0
            }
            Act::Halt(v) => {
                let me = self.me;
                self.h.war.halt(me, &v);
                true
            }
            Act::Assign(v, g) => {
                let me = self.me;
                self.h.war.assign(me, &v, g);
                true
            }
            Act::SendOn(v, q) => {
                let me = self.me;
                self.h.war.send_on(me, &v, q) > 0
            }
            Act::Raise(v) => self.raise(&v),
            Act::Sector(g, line) => {
                let me = self.me;
                self.h.war.set_sector(me, g, &line)
            }
            Act::Front(gs, facing, line) => {
                let me = self.me;
                self.h.war.set_front(me, &gs, &line, facing)
            }
        }
    }

    /// The world's fingerprint: the day, who holds what, each side's economy and army, the
    /// record, the news, the events, the templates (a different past gives a different one).
    pub fn fingerprint(&self) -> u64 {
        let mut h = hash64(self.month() as u64 ^ (self.day as u64) << 32);
        let mut mix = |x: u64| h = hash64(h ^ x);
        for &o in &self.h.war.owner {
            mix(o as u64);
        }
        for s in 0..2 {
            let n = self.h.home(s);
            for v in [n.gdp_real, n.treasury, n.pop.iter().sum::<f32>(), n.money.iter().sum::<f32>()] {
                mix(v.to_bits() as u64);
            }
            for p in &n.price {
                mix(p.to_bits() as u64);
            }
            for c in &self.h.war.sides[s].army.count {
                mix(c.to_bits() as u64);
            }
            mix(self.state.stability[s].to_bits() as u64);
        }
        if let Some(c) = &self.h.war.corps {
            for d in &c.divs {
                mix(d.id as u64 ^ (d.at as u64) << 32);
                mix(d.org.to_bits() as u64 ^ (d.has.iter().sum::<f32>().to_bits() as u64) << 32);
            }
        }
        mix(self.rows.len() as u64);
        mix(self.news.len() as u64);
        mix(self.ev.next_id);
        for t in &self.templates {
            for &b in &t.bat {
                mix(b as u64);
            }
            mix(t.want.to_bits() as u64);
            for &k in &t.norm {
                mix(k.to_bits() as u64);
            }
            for &k in &t.support {
                mix(k as u64 ^ 0x5A);
            }
        }
        h
    }

    /// The game as a save (text).
    pub fn save_text(&self) -> String {
        let mut o = format!("strategy-save 1\npack {}\nseed {}\nside {}\n", self.path, self.seed, self.me);
        if let Some(c) = self.chosen {
            o += &format!("country {c}\n");
        }
        for (m, d, a) in &self.journal {
            o += &format!("act {m} {d} ");
            o += &match a {
                Act::Form(f) => format!("form {}", f.iter().map(|(k, v)| format!("{}={}", enc(k), enc(v))).collect::<Vec<_>>().join("&")),
                Act::Answer(id, k) => format!("answer {id} {k}"),
                Act::Claim => "claim".into(),
                Act::Peace => "peace".into(),
                Act::Depot(q) => format!("depot {q}"),
                Act::Line(rail, path) => format!("line {} {}", if *rail { "rail" } else { "road" }, path.iter().map(|q| q.to_string()).collect::<Vec<_>>().join(",")),
                Act::Rival(c) => format!("rival {c}"),
                Act::Send(v, q) => format!("send {} {q}", ids(v)),
                Act::Halt(v) => format!("halt {}", ids(v)),
                Act::Assign(v, g) => format!("assign {} {}", ids(v), if *g == usize::MAX { "-".to_string() } else { g.to_string() }),
                Act::SendOn(v, q) => format!("sendon {} {q}", ids(v)),
                Act::Raise(v) => format!("raise {}", ids(v)),
                Act::Sector(g, line) => format!("sector {g} {}", line.iter().map(|q| q.to_string()).collect::<Vec<_>>().join(",")),
                Act::Front(gs, facing, line) => format!(
                    "front {} {} {}",
                    gs.iter().map(|q| q.to_string()).collect::<Vec<_>>().join(","),
                    facing.map_or("-".to_string(), |c| c.to_string()),
                    line.iter().map(|q| q.to_string()).collect::<Vec<_>>().join(",")
                ),
            };
            o += "\n";
        }
        o += &format!("now {} {}\nfingerprint {:016x}\n", self.month(), self.day, self.fingerprint());
        o
    }

    /// The game saved in the text: built, lived again, checked. `say` hears how far it got
    /// (months lived of all).
    pub fn load_text(text: &str, say: &mut dyn FnMut(u32, u32)) -> Result<Game, String> {
        let mut lines = text.lines();
        if lines.next().map(str::trim) != Some("strategy-save 1") {
            return Err("это не сохранение этой игры".into());
        }
        let (mut path, mut seed, mut side) = (String::new(), None, 0usize);
        let mut country: Option<usize> = None;
        let mut acts: Vec<(u32, u32, Act)> = Vec::new();
        let (mut now, mut print) = (None, None);
        for l in lines {
            let l = l.trim_end_matches('\r');
            let (key, rest) = l.split_once(' ').unwrap_or((l, ""));
            match key {
                "pack" => path = rest.to_string(),
                "seed" => seed = rest.parse::<u64>().ok(),
                "side" => side = rest.parse().unwrap_or(0),
                "country" => country = rest.parse::<usize>().ok(),
                "now" => now = rest.split_once(' ').and_then(|(m, d)| Some((m.parse::<u32>().ok()?, d.parse::<u32>().ok()?))),
                "fingerprint" => print = u64::from_str_radix(rest, 16).ok(),
                "act" => {
                    let mut w = rest.splitn(4, ' ');
                    let (Some(m), Some(d), Some(kind)) = (w.next().and_then(|x| x.parse().ok()), w.next().and_then(|x| x.parse().ok()), w.next()) else {
                        return Err(format!("непонятная строка: {l}"));
                    };
                    let arg = w.next().unwrap_or("");
                    let a = match kind {
                        "form" => Act::Form(form(arg)),
                        "answer" => {
                            let (id, k) = arg.split_once(' ').ok_or_else(|| format!("непонятная строка: {l}"))?;
                            Act::Answer(id.parse().map_err(|_| format!("непонятная строка: {l}"))?, k.parse().map_err(|_| format!("непонятная строка: {l}"))?)
                        }
                        "claim" => Act::Claim,
                        "peace" => Act::Peace,
                        "depot" => Act::Depot(arg.parse().map_err(|_| format!("непонятная строка: {l}"))?),
                        "rival" => Act::Rival(arg.parse().map_err(|_| format!("непонятная строка: {l}"))?),
                        "send" | "assign" | "sendon" => {
                            let (v, x) = arg.split_once(' ').ok_or_else(|| format!("непонятная строка: {l}"))?;
                            let v = ids_of(v).ok_or_else(|| format!("непонятная строка: {l}"))?;
                            let x = if x == "-" { usize::MAX } else { x.parse::<usize>().map_err(|_| format!("непонятная строка: {l}"))? };
                            match kind {
                                "send" => Act::Send(v, x),
                                "sendon" => Act::SendOn(v, x),
                                _ => Act::Assign(v, x),
                            }
                        }
                        "halt" => Act::Halt(ids_of(arg).ok_or_else(|| format!("непонятная строка: {l}"))?),
                        "raise" => Act::Raise(ids_of(arg).ok_or_else(|| format!("непонятная строка: {l}"))?),
                        "front" => {
                            let bad = || format!("непонятная строка: {l}");
                            let mut w = arg.splitn(3, ' ');
                            let (Some(gs), Some(facing), Some(line)) = (w.next(), w.next(), w.next()) else { return Err(bad()) };
                            let gs: Result<Vec<usize>, _> = gs.split(',').map(|x| x.parse::<usize>()).collect();
                            let line: Result<Vec<usize>, _> = line.split(',').map(|x| x.parse::<usize>()).collect();
                            let facing = if facing == "-" { None } else { Some(facing.parse::<u32>().map_err(|_| bad())?) };
                            Act::Front(gs.map_err(|_| bad())?, facing, line.map_err(|_| bad())?)
                        }
                        "sector" => {
                            let (g, line) = arg.split_once(' ').ok_or_else(|| format!("непонятная строка: {l}"))?;
                            let line: Result<Vec<usize>, _> = line.split(',').map(|x| x.parse::<usize>()).collect();
                            Act::Sector(g.parse().map_err(|_| format!("непонятная строка: {l}"))?, line.map_err(|_| format!("непонятная строка: {l}"))?)
                        }
                        "line" => {
                            let (kind, path) = arg.split_once(' ').ok_or_else(|| format!("непонятная строка: {l}"))?;
                            let path: Result<Vec<usize>, _> = path.split(',').map(|x| x.parse::<usize>()).collect();
                            Act::Line(kind == "rail", path.map_err(|_| format!("непонятная строка: {l}"))?)
                        }
                        _ => return Err(format!("непонятная строка: {l}")),
                    };
                    acts.push((m, d, a));
                }
                _ => {}
            }
        }
        let seed = seed.ok_or("в сохранении нет номера карты")?;
        let pack = strategy::pack::Pack::load(&path).map_err(|e| format!("набор эпохи {path} не открылся: {e}"))?;
        let mut g = Game::new_as(pack, seed, side, country);
        g.path = path;
        let (end_m, end_d) = now.unwrap_or((0, 0));
        let mut live_to = |g: &mut Game, m: u32, d: u32| -> Result<(), String> {
            while (g.month(), g.day) < (m, d) {
                let before = (g.month(), g.day);
                g.paused = false;
                g.tick();
                if (g.month(), g.day) == before {
                    return Err(format!("мир разошёлся с сохранением: часы встали на {} (ждёт ответа: {})", g.date(), g.ev.waiting.first().map_or("ничего".into(), |e| e.title.clone())));
                }
                if g.day == 0 {
                    say(g.month(), end_m);
                }
            }
            Ok(())
        };
        for (m, d, a) in acts {
            live_to(&mut g, m, d)?;
            g.act(a);
        }
        live_to(&mut g, end_m, end_d)?;
        g.paused = true;
        g.alert = None;
        if let Some(p) = print {
            if g.fingerprint() != p {
                return Err(format!("мир разошёлся с сохранением: на {} он не тот, что был", g.date()));
            }
        }
        Ok(g)
    }

    /// The game saved into the folder of saves, named by the country and the day; the file.
    pub fn save_file(&self) -> std::io::Result<std::path::PathBuf> {
        self.save_named(&format!("{}, {}", self.h.home(self.me).name, self.date()))
    }

    /// The game saved into the folder of saves under a name; the file.
    pub fn save_named(&self, name: &str) -> std::io::Result<std::path::PathBuf> {
        std::fs::create_dir_all(DIR)?;
        let name: String = name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == ',' || c == '-' { c } else { '_' }).collect();
        let file = std::path::Path::new(DIR).join(format!("{name}.save"));
        std::fs::write(&file, self.save_text())?;
        Ok(file)
    }
}

/// The saves in the folder, newest first: the file, its name, when it was written.
pub fn list() -> Vec<(std::path::PathBuf, String, std::time::SystemTime)> {
    let mut out: Vec<_> = std::fs::read_dir(DIR)
        .map(|d| {
            d.flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "save"))
                .map(|e| {
                    let t = e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                    (e.path(), e.path().file_stem().map_or(String::new(), |s| s.to_string_lossy().into_owned()), t)
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| b.2.cmp(&a.2));
    out
}

/// A word of a form, safe on a line of a save (`%XX` for all but letters and digits of ASCII).
/// `a+b%20c` → `a b c`.
fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                if let Ok(x) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(x);
                    i += 2;
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `k=v&k2=v2` (as written by `enc`) → the pairs.
fn form(body: &str) -> Vec<(String, String)> {
    body.split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut kv = p.splitn(2, '=');
            (decode(kv.next().unwrap_or("")), decode(kv.next().unwrap_or("")))
        })
        .collect()
}

fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.' { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// A player of chance for the save's self-check: on a day of each month one or two decisions of
/// any kind (levers, advisers, a template with an awkward name, what the army builds, a claim, the
/// war, the groups' orders, depots, the state's building list, now and then a peace); the events
/// answered at once by a choice of chance. Lives `months` months.
fn play_by_chance(g: &mut Game, r: &mut Rng, months: u32) {
    let end = g.month() + months;
    let mut decided = u32::MAX;
    let mut day = r.below(28) as u32;
    let f = |pairs: &[(&str, String)]| Act::Form(pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect());
    while g.month() < end {
        while let Some((id, n)) = g.ev.waiting.first().map(|e| (e.id, e.choices.len())) {
            g.act(Act::Answer(id, r.below(n.max(1))));
        }
        if decided != g.month() && g.day >= day.min(g.days() - 1) {
            decided = g.month();
            day = r.below(28) as u32;
            for _ in 0..1 + r.below(2) {
                let a = match r.below(15) {
                    0 => f(&[("tax", (5 + r.below(35)).to_string())]),
                    1 => f(&[("army", (10 + r.below(50)).to_string())]),
                    // (the state's household too: a line of the civil budget, bonds, the bank rate)
                    2 => match r.below(4) {
                        0 => f(&[("civ", format!("{}:{}", r.below(5), r.below(4)))]),
                        1 => f(&[("bonds_sell", "1".into())]),
                        2 => f(&[("fx_rate", r.below(4).to_string())]),
                        _ => f(&[("build", r.below(90).to_string())]),
                    },
                    3 => f(&[("econ", if r.below(3) == 0 { "me".into() } else { format!("adv{}", r.below(3)) })]),
                    4 => f(&[("science", r.below(20).to_string()), ("scihead", if r.below(2) == 0 { "me".into() } else { format!("sci{}", r.below(3)) })]),
                    5 => {
                        let bat: Vec<String> = (0..g.pack.units.len()).map(|_| r.below(5).to_string()).collect();
                        let norm: Vec<String> = (0..g.pack.units.len()).map(|_| r.below(3).to_string()).collect();
                        let sup: Vec<String> = (0..g.pack.support.len()).filter(|_| r.below(2) == 0).map(|k| k.to_string()).collect();
                        f(&[
                            ("tpl", if r.below(3) == 0 { "new".into() } else { "0".into() }),
                            ("tpl_bat", bat.join(",")),
                            ("tpl_name", format!("Дивизия №{} & «ко» = 100%", r.below(9))),
                            ("tpl_want", r.below(5).to_string()),
                            ("tpl_norm", norm.join(",")),
                            ("tpl_sup", sup.join(",")),
                        ])
                    }
                    6 => f(&[("ctrl", if r.below(2) == 0 { "me".into() } else { "staff".into() })]),
                    7 if !g.h.at_war => Act::Claim,
                    8 if !g.h.at_war => f(&[("declare", "1".into())]),
                    9 | 10 if g.h.at_war => {
                        let war = &g.h.war;
                        let me = g.me;
                        let groups = war.command[me].as_ref().map_or(0, |x| x.len());
                        let foe: Vec<usize> = (0..war.owner.len()).filter(|&q| war.owner[q] != me).collect();
                        let mine: Vec<usize> = (0..war.owner.len()).filter(|&q| war.owner[q] == me).collect();
                        if r.below(2) == 0 && war.corps.is_some() && !mine.is_empty() {
                            // (divisions: sent somewhere, stopped, given to a group or the reserve)
                            let c = war.corps.as_ref().unwrap();
                            let ours: Vec<u32> = c.divs.iter().filter(|d| d.side == me).map(|d| d.id).collect();
                            if ours.is_empty() {
                                continue;
                            }
                            let k = 1 + r.below(4);
                            let pick: Vec<u32> = (0..k).map(|_| ours[r.below(ours.len())]).collect();
                            // (and sent on, formed into a new group, a group given its stretch of the front)
                            match r.below(6) {
                                0 => Act::Send(pick, if r.below(2) == 0 && !foe.is_empty() { foe[r.below(foe.len())] } else { mine[r.below(mine.len())] }),
                                1 => Act::Halt(pick),
                                2 => Act::SendOn(pick, if r.below(2) == 0 && !foe.is_empty() { foe[r.below(foe.len())] } else { mine[r.below(mine.len())] }),
                                3 if groups < 8 => Act::Raise(pick),
                                4 if groups > 0 => {
                                    let front = war.front(me);
                                    if front.is_empty() {
                                        continue;
                                    }
                                    let a = r.below(front.len());
                                    Act::Sector(r.below(groups), front[a..(a + 1 + r.below(4)).min(front.len())].to_vec())
                                }
                                _ => Act::Assign(pick, if r.below(4) == 0 { usize::MAX } else { r.below(groups.max(1)) }),
                            }
                        } else if r.below(4) == 0 && mine.len() > 1 {
                            // (a line from one province of ours to another, the shortest way)
                            let (a, b) = (mine[r.below(mine.len())], mine[r.below(mine.len())]);
                            match crate::world::land::plan(war, me, a, b) {
                                Some(p) if p.len() >= 2 && p.len() <= crate::world::statecraft::LINE_MOST => Act::Line(r.below(2) == 0, p),
                                _ => continue,
                            }
                        } else if r.below(3) == 0 && !mine.is_empty() {
                            Act::Depot(mine[r.below(mine.len())])
                        } else if groups > 0 && !foe.is_empty() {
                            let order = ["hold", "deep", "attack", "staff"][r.below(4)];
                            let target = war.th.provs[foe[r.below(foe.len())]];
                            f(&[("grp", r.below(groups).to_string()), ("order", order.into()), ("target", target.to_string())])
                        } else {
                            continue;
                        }
                    }
                    11 => Act::Form(vec![("queue_set".into(), "1".into()), (format!("q{}", r.below(g.pack.sectors.len())), "1".into())]),
                    12 => f(&[("draft", if r.below(4) == 0 { "custom".into() } else { r.below(5).to_string() })]),
                    // (the economic policy: a law, a programme begun or stopped, a loan taken or paid
                    // off, a trade modernised)
                    13 => match r.below(6) {
                        0 => f(&[("ecolaw", r.below(crate::country::policy::LAWS.len()).to_string())]),
                        1 => f(&[("plan_go", r.below(crate::country::policy::PLANS.len()).to_string())]),
                        2 => f(&[("plan_stop", r.below(crate::country::policy::PLANS.len()).to_string())]),
                        3 => f(&[(if r.below(2) == 0 { "loan" } else { "repay" }, "1".into())]),
                        4 => f(&[("modern", r.below(g.pack.sectors.len()).to_string())]),
                        _ => f(&[("modern_stop", "1".into())]),
                    },
                    // (the factories: a new model taken, tried or turned down; a priority of production)
                    14 => {
                        let offers: Vec<u64> = g.arms.offers[g.me].iter().map(|o| o.id).collect();
                        if !offers.is_empty() && r.below(2) == 0 {
                            let id = offers[r.below(offers.len())];
                            f(&[(["model_ok", "model_try", "model_no"][r.below(3)], id.to_string())])
                        } else {
                            f(&[("prio", format!("{}:{}", r.below(g.pack.units.len()), r.below(crate::army::arms::PRIO.len())))])
                        }
                    }
                    _ if g.h.at_war && r.below(8) == 0 => Act::Peace,
                    _ if !g.h.at_war && r.below(12) == 0 => {
                        let nb = g.neighbours();
                        if nb.is_empty() {
                            continue;
                        }
                        Act::Rival(nb[r.below(nb.len())])
                    }
                    _ => continue,
                };
                g.act(a);
            }
        }
        let before = (g.month(), g.day);
        g.paused = false;
        g.tick();
        if (g.month(), g.day) == before && g.ev.waiting.is_empty() {
            break;
        }
    }
}

/// The save's self-checks (`play --savetest`): a game of chance saved and loaded is the same world
/// to the bit — in this process and in another — and goes on the same; a save with one decision
/// left out is not (the control).
pub fn savetest(pack: strategy::pack::Pack, path: &str, seed: u64) -> bool {
    let t0 = std::time::Instant::now();
    let mut checks: Vec<(String, bool, String)> = Vec::new();
    let mut g = Game::new(pack, seed, 0);
    g.path = path.to_string();
    // (the world as it begins: the orders of the war are checked on it if the game played by
    // chance left the player no land to order his divisions over)
    let fresh = g.clone();
    let mut r = Rng::new(hash64(seed ^ 0x5A7E));
    play_by_chance(&mut g, &mut r, 30);
    let text = g.save_text();
    let print = g.fingerprint();
    let acts = g.journal.len();
    let kinds = |a: &Act| match a {
        Act::Form(_) => 0,
        Act::Answer(..) => 1,
        Act::Claim => 2,
        Act::Peace => 3,
        Act::Depot(_) => 4,
        Act::Line(..) => 5,
        Act::Rival(_) => 5,
        Act::Send(..) | Act::Halt(_) | Act::Assign(..) | Act::SendOn(..) | Act::Raise(_) | Act::Sector(..) | Act::Front(..) => 6,
    };
    let mut by_kind = [0usize; 7];
    for (_, _, a) in &g.journal {
        by_kind[kinds(a)] += 1;
    }
    // 1. loaded here
    let t1 = std::time::Instant::now();
    let loaded = Game::load_text(&text, &mut |_, _| {});
    let took = t1.elapsed().as_secs_f32();
    let same = loaded.as_ref().map_or(false, |l| l.fingerprint() == print && l.journal.len() == acts && l.month() == g.month() && l.day == g.day);
    checks.push((
        "сохранение и загрузка — тот же мир до бита".into(),
        same,
        format!(
            "{} мес., решений {acts} (рычаги и приказы {}, ответы на события {}, цели войны {}, мир {}, склады {}, дивизиям {}), файл {} КБ, загрузка {took:.0} с{}",
            g.month(),
            by_kind[0],
            by_kind[1],
            by_kind[2],
            by_kind[3],
            by_kind[4],
            by_kind[6],
            text.len() / 1024,
            loaded.as_ref().err().map_or(String::new(), |e| format!("; ошибка: {e}"))
        ),
    ));
    // 2. loaded by another process (the same program)
    let file = std::env::temp_dir().join(format!("strategy_savetest_{}.save", std::process::id()));
    let other = std::fs::write(&file, &text).ok().and_then(|_| std::process::Command::new(std::env::current_exe().ok()?).arg("--load-check").arg(&file).output().ok());
    let _ = std::fs::remove_file(&file);
    let (ok2, said) = match other {
        Some(o) => (o.status.success(), String::from_utf8_lossy(&o.stdout).lines().last().unwrap_or("").to_string()),
        None => (false, "не запустился".into()),
    };
    checks.push(("загрузка в другом процессе — тот же мир".into(), ok2, said));
    // 3. the loaded game goes on the same
    let (ok3, d3) = match loaded {
        Ok(mut l) => {
            let mut a = g.clone();
            let (mut ra, mut rb) = (Rng::new(77), Rng::new(77));
            play_by_chance(&mut a, &mut ra, 6);
            play_by_chance(&mut l, &mut rb, 6);
            (a.fingerprint() == l.fingerprint() && a.journal.len() == l.journal.len(), format!("ещё 6 мес. теми же решениями: до {}, решений {} и {}", a.date(), a.journal.len(), l.journal.len()))
        }
        Err(e) => (false, e),
    };
    checks.push(("загруженная игра идёт дальше так же".into(), ok3, d3));
    // 4. the control: one decision about money left out — the army's share, which the government
    // sets whoever runs the economy (a tax set while an adviser runs it does nothing: that control
    // was blind once)
    let lines: Vec<&str> = text.lines().collect();
    let drop = lines.iter().position(|l| l.starts_with("act ") && l.contains(" form army="));
    let (ok4, d4) = match drop {
        Some(i) => {
            let spoiled: String = lines.iter().enumerate().filter(|(k, _)| *k != i).map(|(_, l)| format!("{l}\n")).collect();
            match Game::load_text(&spoiled, &mut |_, _| {}) {
                Err(e) => (true, format!("без решения «{}» — {e}", lines[i])),
                Ok(_) => (false, format!("без решения «{}» мир сошёлся — проверка слепа", lines[i])),
            }
        }
        None => (false, "в игре не было решений о деньгах — контроль не поставить".into()),
    };
    checks.push(("контроль: без одного решения мир расходится".into(), ok4, d4));
    // 5. the orders of the war as the window gives them — a new group of divisions, its own
    // stretch of the front, a way with a stop more, where the new divisions gather and to which
    // group — saved and loaded are the same orders (the groups, their stretches, each division's
    // group and way); the control: the save without the new group is not
    let (ok5, d5) = {
        let rear = |x: &Game| {
            let war = &x.h.war;
            let front = war.front(x.me);
            (0..war.owner.len()).filter(|&q| war.owner[q] == x.me && !front.contains(&q)).count()
        };
        let mut w = if rear(&g) >= 2 { g.clone() } else { fresh.clone() };
        if !w.h.at_war {
            w.act(Act::Form(vec![("declare".into(), "1".into())]));
            w.advance();
        }
        let me = w.me;
        let war = &w.h.war;
        let picked: Vec<u32> = war.corps.as_ref().map_or(Vec::new(), |c| c.divs.iter().filter(|d| d.side == me).take(5).map(|d| d.id).collect());
        let front = war.front(me);
        // (where to send: the rear, else — a country of a few provinces — any of its own beyond the
        // stretch given to the new group)
        let mut mine: Vec<usize> = (0..war.owner.len()).filter(|&q| war.owner[q] == me && !front.contains(&q)).collect();
        if mine.len() < 2 {
            let more: Vec<usize> = front.iter().skip(2).copied().filter(|q| !mine.contains(q)).collect();
            mine.extend(more);
        }
        if picked.len() < 2 || front.is_empty() || mine.len() < 2 {
            (false, format!("нет войны дивизиями — проверить нечем (война {}, дивизий {}, фронт {}, тыл {}, дивизии вообще {})", w.h.at_war, picked.len(), front.len(), mine.len(), war.corps.as_ref().map_or(0, |c| c.divs.len())))
        } else {
            let k = war.command[me].as_ref().map_or(0, |v| v.len());
            let p = war.th.provs[mine[0]];
            w.act(Act::Raise(picked.clone()));
            w.act(Act::Sector(k, front[..front.len().min(2)].to_vec()));
            w.act(Act::Front(vec![0, k], None, front.clone()));
            w.act(Act::Send(picked[..2].to_vec(), mine[0]));
            w.act(Act::SendOn(picked[..2].to_vec(), mine[1]));
            w.act(Act::Form(vec![("muster".into(), p.to_string()), ("muster_grp".into(), k.to_string())]));
            let state = |x: &Game| -> String {
                let war = &x.h.war;
                let gs: Vec<String> = war.command[x.me].as_ref().map_or(Vec::new(), |v| v.iter().map(|gr| format!("{:?}/{}/{:?}/{:?}", gr.anchor, gr.staff, gr.order, gr.facing)).collect());
                let c = war.corps.as_ref().unwrap();
                let ds: Vec<String> = c.divs.iter().filter(|d| d.side == x.me).map(|d| format!("{}:{}:{:?}", d.id, d.group, d.way)).collect();
                format!("{}|{}|{:?}{:?}", gs.join(";"), ds.join(";"), c.muster, c.muster_group)
            };
            let text = w.save_text();
            let want = state(&w);
            let back = Game::load_text(&text, &mut |_, _| {});
            let same = back.as_ref().is_ok_and(|b| state(b) == want);
            let lines: Vec<&str> = text.lines().collect();
            let drop = lines.iter().rposition(|l| l.starts_with("act ") && l.contains(" raise "));
            let blind = drop.is_some_and(|i| {
                let spoiled: String = lines.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, l)| format!("{l}
")).collect();
                Game::load_text(&spoiled, &mut |_, _| {}).is_ok_and(|b| state(&b) == want)
            });
            let ok = same && drop.is_some() && !blind;
            (ok, format!("групп {} → {}, приказы после загрузки {}; без строки «raise» {}", k, k + 1, if same { "те же" } else { "другие" }, if blind { "те же — проверка слепа" } else { "другие" }))
        }
    };
    checks.push(("приказы войны (новая группа, участок, путь дальше, сбор) сохраняются".into(), ok5, d5));
    println!("\nпроверки сохранения ({:.0} с):", t0.elapsed().as_secs_f32());
    for (name, ok, detail) in &checks {
        println!("  [{}] {name:<45} {detail}", if *ok { "OK" } else { "FAIL" });
    }
    checks.iter().all(|c| c.1)
}
