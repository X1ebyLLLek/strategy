//! Battles. The engine knows no era: what kinds of units there are, what each is when shot
//! at, whom it hits how hard, how it uses the ground and what it uses up — all of that comes
//! from the content pack.
//!
//! A battle goes day by day. Only so many units of the front line fight side by side: the
//! width of a plain, less in forests and far less in mountains; the rest wait in reserve.
//! Units of the second line fire from behind it and draw little fire while it holds; units in
//! the air need no place in it. Every unit fires at what it can see and harm, the more at a
//! class of targets the more of it there is and the better it hits it. A defender keeps part
//! of the harm off by the cover of the ground, as far as the unit can use it.
//!
//! Harm kills some men and wears out more: a worn-out unit leaves the line and a fresh one
//! from the reserve takes its place. A side gives way when it can no longer hold its line
//! with fresh units, and a faster winner cuts down more of it as it goes. (A first version
//! shook the morale of a whole army by the share of it lost: a big army in the mountains
//! never broke, and no attack there won even at sixteen to one.) Without its supply a unit
//! fires far less. A day has its luck, and so has a whole battle.

use crate::pack::{Line, Pack};
use crate::rng::Rng;
use crate::world::map::Terrain;

/// Units killed and units worn out of the line per unit of harm against their toughness: an
/// army four lines deep meeting its equal on a plain gives way in about three weeks.
pub(crate) const KILL: f32 = 0.03;
pub(crate) const WEAR: f32 = 0.15;
/// A side gives way when its fresh units fill less than this share of the line it held.
pub(crate) const HOLD: f32 = 0.3;
/// Share of the harm that reaches it a fully dug-in defender keeps off.
pub const DIG: f32 = 0.3;
/// Fire of a unit with none of its supply, against one fully supplied.
pub(crate) const UNSUPPLIED: f32 = 0.3;
/// Fire drawn by a unit of the second line while the front holds, against one in the line.
pub(crate) const BEHIND: f32 = 0.3;
/// Luck of a day and of a whole battle: spread of the log of a side's fire.
pub(crate) const DAY_LUCK: f32 = 0.25;
pub(crate) const BATTLE_LUCK: f32 = 0.2;
/// Share of its units a side loses in giving way, and more per unit of how much faster the
/// winner is (at most three times as fast counts).
pub(crate) const RETREAT: f32 = 0.03;
pub(crate) const PURSUIT: f32 = 0.05;

#[derive(Clone, Debug)]
pub struct Army {
    /// Units of each kind (a fraction is a worn-down unit).
    pub count: Vec<f32>,
    /// Share of the supply it needs that it has, 0 to 1.
    pub supplied: f32,
    /// What its equipment of each kind is really worth against the pack's figures: fire and
    /// toughness (a country's own designs, better or worse than the era's average).
    pub fire: Vec<f32>,
    pub tough: Vec<f32>,
    /// How dug in it is where it stands, 0 to 1: a defender dug in fully keeps `DIG` more of
    /// the harm off, besides what the ground gives.
    pub dug: f32,
}

impl Army {
    pub fn new(pack: &Pack) -> Army {
        let nu = pack.units.len();
        Army { count: vec![0.0; nu], supplied: 1.0, fire: vec![1.0; nu], tough: vec![1.0; nu], dug: 0.0 }
    }

    /// Worth at starting prices.
    pub fn worth(&self, pack: &Pack) -> f32 {
        self.count.iter().zip(&pack.units).map(|(n, u)| n * u.cost).sum()
    }

    pub fn describe(&self, pack: &Pack) -> String {
        let parts: Vec<String> =
            self.count.iter().zip(&pack.units).filter(|(n, _)| **n > 0.05).map(|(n, u)| format!("{} {:.1}", u.name, n)).collect();
        if parts.is_empty() {
            "никого".into()
        } else {
            parts.join(", ")
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    /// The first side attacks, the second holds its ground and has its cover.
    Attack,
    /// Both sides meet in the open, neither has cover.
    Meeting,
}

pub struct Outcome {
    /// The side that held the field; None when neither gave way in a meeting.
    pub winner: Option<usize>,
    pub days: u32,
    /// What is left of each army (worn-out units are back in it once the battle is over).
    pub left: [Army; 2],
    pub log: Vec<String>,
}

/// One battle on terrain `t` between two armies. With `chronicle`, a line a day.
pub fn battle(pack: &Pack, t: Terrain, armies: [&Army; 2], kind: Kind, rng: &mut Rng, chronicle: bool) -> Outcome {
    let war = pack.war.as_ref().expect("в наборе нет раздела [war]");
    let units = &pack.units;
    let (nu, nc, ti) = (units.len(), war.classes.len(), t as usize);
    let width = war.width * war.frontage[ti];
    let mut fresh = [armies[0].count.clone(), armies[1].count.clone()];
    let mut worn = [vec![0.0f32; nu], vec![0.0f32; nu]];
    let start = [armies[0].worth(pack), armies[1].worth(pack)];
    // nobody on one side: the other has the field without a fight (an empty province is simply
    // taken — fought for a month against nobody, it never gave way)
    if start[0] <= 1e-6 || start[1] <= 1e-6 {
        let winner = match (start[0] > 1e-6, start[1] > 1e-6) {
            (true, false) => Some(0),
            (false, true) => Some(1),
            _ if kind == Kind::Attack => Some(1),
            _ => None,
        };
        return Outcome { winner, days: 0, left: [armies[0].clone(), armies[1].clone()], log: Vec::new() };
    }
    // what holds a side's line: its front, or its second line if it has no front at all
    let holder: [Vec<bool>; 2] = std::array::from_fn(|s| {
        let has_front = (0..nu).any(|u| units[u].line == Line::Front && armies[s].count[u] > 0.0);
        (0..nu).map(|u| units[u].line == if has_front { Line::Front } else { Line::Support }).collect()
    });
    let place = |s: usize, n: &[Vec<f32>; 2]| (0..nu).filter(|&u| holder[s][u]).map(|u| n[s][u] * units[u].width).sum::<f32>();
    let held = [place(0, &fresh).min(width), place(1, &fresh).min(width)];
    let luck = [(BATTLE_LUCK * rng.normal()).exp(), (BATTLE_LUCK * rng.normal()).exp()];
    let mut log = Vec::new();
    let mut winner = None;
    let mut days = 0;
    for day in 1..=war.days {
        days = day;
        // who fights, and how much of the enemy's fire each draws
        let mut fighting = [vec![0.0f32; nu], vec![0.0f32; nu]];
        let mut draws = [vec![0.0f32; nu], vec![0.0f32; nu]];
        let mut seen = [vec![0.0f32; nc], vec![0.0f32; nc]];
        for s in 0..2 {
            let line = place(s, &fresh);
            let room = (width / line.max(1e-9)).min(1.0);
            for u in 0..nu {
                let (f, d) = if holder[s][u] {
                    (fresh[s][u] * room, 1.0)
                } else {
                    match units[u].line {
                        Line::Air => (fresh[s][u], 1.0),
                        _ => (fresh[s][u], BEHIND),
                    }
                };
                fighting[s][u] = f;
                draws[s][u] = f * d;
                seen[s][units[u].class] += f * d;
            }
        }
        // the fire of each side on each class of the other's units
        let mut harm = [vec![0.0f32; nc], vec![0.0f32; nc]];
        for s in 0..2 {
            let o = 1 - s;
            let day_luck = (DAY_LUCK * rng.normal()).exp();
            for u in 0..nu {
                if fighting[s][u] <= 0.0 {
                    continue;
                }
                let unit = &units[u];
                let aim: Vec<f32> = (0..nc).map(|c| seen[o][c] * unit.fire[c]).collect();
                let all: f32 = aim.iter().sum();
                if all <= 0.0 {
                    continue;
                }
                let fed = if unit.supply.is_empty() { 1.0 } else { UNSUPPLIED + (1.0 - UNSUPPLIED) * armies[s].supplied };
                let k = fighting[s][u] * armies[s].fire[u] * unit.terrain[ti] * fed * day_luck * luck[s];
                for c in 0..nc {
                    harm[o][c] += k * unit.fire[c] * aim[c] / all;
                }
            }
        }
        // each class's harm spread over its units by the fire they draw; a defender's cover
        // keeps part of it off; it kills some and wears out more
        let mut fell = [vec![0.0f32; nu], vec![0.0f32; nu]];
        for s in 0..2 {
            for u in 0..nu {
                let c = units[u].class;
                if draws[s][u] <= 0.0 || seen[s][c] <= 0.0 {
                    continue;
                }
                let mut h = harm[s][c] * draws[s][u] / seen[s][c];
                if kind == Kind::Attack && s == 1 {
                    h *= 1.0 - war.cover[ti] * units[u].cover.clamp(0.0, 1.0);
                    h *= 1.0 - DIG * armies[1].dug.clamp(0.0, 1.0) * units[u].cover.clamp(0.0, 1.0);
                }
                let hit = h / (units[u].toughness * armies[s].tough[u]);
                let dead = (hit * KILL).min(fresh[s][u]);
                let tired = (hit * WEAR).min(fresh[s][u] - dead);
                fresh[s][u] -= dead + tired;
                worn[s][u] += tired;
                fell[s][u] = dead;
            }
        }
        if chronicle {
            let side = |s: usize| {
                let dead: Vec<String> = (0..nu).filter(|&u| fell[s][u] > 0.005).map(|u| format!("{} −{:.2}", units[u].name, fell[s][u])).collect();
                format!("убиты {}; держат линию {:.1} из {:.1}", if dead.is_empty() { "—".into() } else { dead.join(", ") }, place(s, &fresh).min(width), held[s])
            };
            log.push(format!("  день {day:>2}: первые: {}; вторые: {}", side(0), side(1)));
        }
        // a side gives way when its fresh units no longer hold its line
        let broke: Vec<usize> = (0..2).filter(|&s| place(s, &fresh) < HOLD * held[s]).collect();
        if !broke.is_empty() {
            let loser = if broke.len() == 2 {
                if place(0, &fresh) / held[0].max(1e-9) < place(1, &fresh) / held[1].max(1e-9) { 0 } else { 1 }
            } else {
                broke[0]
            };
            let w = 1 - loser;
            winner = Some(w);
            // giving way costs men, the more the faster the winner is
            let speed = |n: &[f32]| {
                let all: f32 = n.iter().sum::<f32>().max(1e-9);
                (0..nu).map(|u| n[u] * units[u].speed).sum::<f32>() / all
            };
            let all_loser: Vec<f32> = (0..nu).map(|u| fresh[loser][u] + worn[loser][u]).collect();
            let fast = speed(&fighting[w]) / speed(&all_loser).max(1e-9);
            let cut = RETREAT + PURSUIT * (fast - 1.0).clamp(0.0, 3.0);
            for u in 0..nu {
                fresh[loser][u] *= 1.0 - cut;
                worn[loser][u] *= 1.0 - cut;
            }
            if chronicle {
                log.push(format!(
                    "  {} отходят на {}-й день: свежими держат {:.1} из {:.1}; при отходе теряют ещё {:.0}%",
                    if loser == 0 { "первые" } else { "вторые" },
                    day,
                    place(loser, &fresh),
                    held[loser],
                    cut * 100.0
                ));
            }
            break;
        }
    }
    if winner.is_none() && kind == Kind::Attack {
        // the attack stalled: the defender holds
        winner = Some(1);
        if chronicle {
            log.push(format!("  наступление выдохлось за {days} дней — вторые удержались"));
        }
    }
    let after = |s: usize| Army { count: (0..nu).map(|u| fresh[s][u] + worn[s][u]).collect(), ..armies[s].clone() };
    let left = [after(0), after(1)];
    Outcome { winner, days, left, log }
}
