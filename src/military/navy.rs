//! The navy: warships the economy builds (`pack::Ship`), squadrons on missions in the zones of the
//! sea, battles between them, and what fleets do to the enemy's trade by sea.
//!
//! Nothing here knows an era: a ship of the line and a battleship differ only in their figures. A
//! fleet does three things a war needs: it fights the enemy's fleet for the sea (a battle is rounds of
//! fire: heavy guns against armour, torpedoes against any hull, depth charges against what hides
//! under water), it sinks the enemy's merchants (raiders against escorts: what is lost on the way
//! is goods that never arrive, and merchants who see it carry less), and it shuts the enemy's ports
//! (a blockade against the fleet that guards them). A ship badly hit leaves the battle and goes home
//! to be mended; one hit harder sinks with her crew.

use crate::pack::Pack;
use crate::rng::Rng;

/// What a squadron is told to do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mission {
    /// In port: mended, kept for a better day; a third of its supply. It fights only an enemy who
    /// comes to its own coast (the fleet in being).
    Port,
    /// Seeks out the enemy's fleet in its zone: the sea is held by those who patrol it.
    Patrol,
    /// Guards the convoys of its own country passing its zone.
    Escort,
    /// Hunts the enemy's merchants (and those of anyone trading with him) passing its zone.
    Raid,
    /// Stands off the enemy's ports in its zone: his ships do not get in or out.
    Blockade,
}

/// The missions: what each is, its name in words and its icon.
pub const MISSIONS: [(Mission, &str, &str, &str); 5] = [
    (Mission::Port, "В порту", "anchor", "Стоит в порту: чинится, тратит треть снабжения; бьётся, только если враг придёт к нашим берегам."),
    (Mission::Patrol, "Господство", "binoculars", "Ищет флот врага в своей зоне: море держит тот, кто в нём ходит."),
    (Mission::Escort, "Конвои", "shield", "Охраняет наши суда, идущие через зону: рейдерам труднее их топить."),
    (Mission::Raid, "Рейды", "target", "Топит суда врага (и тех, кто торгует с ним), идущие через зону."),
    (Mission::Blockade, "Блокада", "wall", "Стоит у портов врага в зоне: ни войти, ни выйти. Нужен перевес над тем, кто их охраняет."),
];

impl Mission {
    pub fn index(self) -> usize {
        MISSIONS.iter().position(|m| m.0 == self).unwrap_or(0)
    }
    pub fn name(self) -> &'static str {
        MISSIONS[self.index()].1
    }
    pub fn icon(self) -> &'static str {
        MISSIONS[self.index()].2
    }
    pub fn about(self) -> &'static str {
        MISSIONS[self.index()].3
    }
    pub fn at_sea(self) -> bool {
        self != Mission::Port
    }
}

/// A ship in service: its kind (`pack.ships`), name, how whole she is (0 sunk … 1 whole), what her
/// crew has learned (0 … 1), the month she was commissioned, the enemy ships she helped to sink.
#[derive(Clone, Debug)]
pub struct Vessel {
    pub kind: usize,
    pub name: String,
    pub health: f32,
    pub exp: f32,
    pub since: u32,
    pub sank: u32,
}

impl Vessel {
    pub fn new(kind: usize, name: String, since: u32) -> Vessel {
        Vessel { kind, name, health: 1.0, exp: 0.0, since, sank: 0 }
    }
}

/// A squadron: its name and its admiral, its ships, its mission and the zone of the sea it is sent
/// to; whether the admiralty (the bot) leads it — the player's order takes it from the admiralty.
#[derive(Clone, Debug)]
pub struct Squadron {
    pub name: String,
    pub admiral: String,
    pub ships: Vec<Vessel>,
    pub mission: Mission,
    pub zone: usize,
    pub staff: bool,
}

/// Rounds of fire in a battle (a day of manoeuvres and gunnery).
const ROUNDS: u32 = 5;
/// Harm a gun does in a round, of its figure.
const ROUND_K: f32 = 0.45;
/// Share of torpedoes that hit.
const HIT: f32 = 0.3;
/// A ship this hurt leaves the battle (and goes home to be mended).
pub const RETREAT: f32 = 0.35;
/// A ship this hurt or worse sinks with this chance at the end of a round.
const SINKING: f32 = 0.15;
const SINK_CHANCE: f32 = 0.5;
/// Submarines and raiders this hard to find or harder are struck by anti-submarine weapons only.
const UNDER: f32 = 0.5;
/// Of what a raider does in a month, the most of a zone's passing trade lost, and the floor of
/// raiders and escorts against which their power counts (in ships of the pack's usual raid).
const RAID_MOST: f32 = 0.35;
const BLOCK_MOST: f32 = 0.85;
const FLOOR_SHIPS: f32 = 10.0;

/// How much harm a gun of `gun` does to armour `armor`: in full when it is as heavy as the armour,
/// a fraction when lighter (destroyers' guns scratch a battleship).
pub fn pierce(gun: f32, armor: f32) -> f32 {
    if armor <= 0.0 || gun >= armor {
        1.0
    } else {
        let r = gun / armor;
        0.15 + 0.85 * r * r
    }
}

/// How a crew of experience `exp` fights against an ordinary one.
fn skill(exp: f32) -> f32 {
    0.85 + 0.3 * exp.clamp(0.0, 1.0)
}

/// Whether a kind of ship hides under water (or behind islands): only anti-submarine weapons find it.
pub fn hidden(pack: &Pack, kind: usize) -> bool {
    pack.ships[kind].stealth >= UNDER
}

/// A ship's weight in a fight for the sea (guns and torpedoes, by how whole she is).
pub fn power(pack: &Pack, v: &Vessel) -> f32 {
    let k = &pack.ships[v.kind];
    (k.gun + 0.6 * k.torpedo + 0.3 * k.asw) * (0.4 + 0.6 * k.hull.sqrt() / 6.0).min(1.5) * v.health * skill(v.exp)
}

/// A ship's weight against merchants, and her weight in guarding them.
pub fn raid_power(pack: &Pack, v: &Vessel) -> f32 {
    pack.ships[v.kind].raid * v.health * skill(v.exp)
}

pub fn escort_power(pack: &Pack, v: &Vessel) -> f32 {
    pack.ships[v.kind].escort * v.health * skill(v.exp)
}

/// What one side's ships in a zone bring to a search for the enemy: speed of the ships that can be
/// seen (and so can look), by how whole they are.
pub fn search(pack: &Pack, ships: &[&Vessel]) -> f32 {
    ships.iter().map(|v| pack.ships[v.kind].speed * (1.0 - pack.ships[v.kind].stealth) * v.health).sum()
}

/// The chance that two fleets in one zone meet in a month: by what both bring to the search (the sea
/// is wide: two squadrons of a few ships may cruise a month without meeting).
pub fn meet_chance(search_a: f32, search_b: f32) -> f32 {
    1.0 - (-(search_a + search_b) / 6.0).exp()
}

/// The floor against which raiders and escorts count in this pack (`FLOOR_SHIPS` ships of the usual
/// raid or escort).
pub fn floor(pack: &Pack) -> f32 {
    if pack.ships.is_empty() {
        return 1.0;
    }
    let mean = pack.ships.iter().map(|k| k.raid.max(k.escort)).sum::<f32>() / pack.ships.len() as f32;
    (FLOOR_SHIPS * mean).max(1e-3)
}

/// Share of a zone's passing trade the raiders sink in a month, against the escorts.
pub fn raid_loss(pack: &Pack, raid: f32, escort: f32) -> f32 {
    if raid <= 0.0 {
        return 0.0;
    }
    RAID_MOST * raid / (raid + escort + floor(pack))
}

/// Share of the trade of the ports on a zone that a blockade stops, against the fleet that guards
/// them (a blockade weaker than half the guard is driven off).
pub fn blockade_loss(pack: &Pack, block: f32, guard: f32) -> f32 {
    if block <= 0.0 || block < 0.5 * guard {
        return 0.0;
    }
    BLOCK_MOST * block / (block + guard + floor(pack))
}

/// What a battle at sea did: ships sunk on each side (kind, name), ships that left it hurt, the
/// side that held the sea after it (none: neither), rounds fought.
#[derive(Clone, Debug, Default)]
pub struct Clash {
    pub zone: usize,
    pub sunk: [Vec<(usize, String)>; 2],
    pub hurt: [u32; 2],
    pub held: Option<usize>,
    pub rounds: u32,
    /// Each side's weight at sea when the battle began.
    pub weight: [f32; 2],
}

/// A battle between the ships `a` and `b` (side 0 and side 1): rounds of fire till one side has no
/// ship left in the fight or the rounds are done. `ambush[s]`: side `s`'s hidden ships strike first
/// (in the first round, twice). Ships keep what they suffered; the sunk are at health 0.
pub fn battle(pack: &Pack, a: &mut [Vessel], b: &mut [Vessel], ambush: [bool; 2], rng: &mut Rng) -> Clash {
    let mut clash = Clash { weight: [a.iter().map(|v| power(pack, v)).sum(), b.iter().map(|v| power(pack, v)).sum()], ..Clash::default() };
    let fleets: [&mut [Vessel]; 2] = [a, b];
    let mut fighting: [Vec<bool>; 2] = [fleets[0].iter().map(|v| v.health > RETREAT).collect(), fleets[1].iter().map(|v| v.health > RETREAT).collect()];
    for round in 0..ROUNDS {
        let alive = |f: &[bool]| f.iter().any(|&x| x);
        if !alive(&fighting[0]) || !alive(&fighting[1]) {
            break;
        }
        clash.rounds = round + 1;
        // the harm each side takes this round (by ship), dealt by the other
        let mut harm: [Vec<f32>; 2] = [vec![0.0; fleets[0].len()], vec![0.0; fleets[1].len()]];
        for x in 0..2 {
            let y = 1 - x;
            let (fx, fy) = (&*fleets[x], &*fleets[y]);
            // targets: the ships of the other side still in the fight, those seen to guns and
            // torpedoes (by their size), those under water to depth charges (if found)
            let seen: Vec<usize> = (0..fy.len()).filter(|&t| fighting[y][t] && !hidden(pack, fy[t].kind)).collect();
            let under: Vec<usize> = (0..fy.len()).filter(|&t| fighting[y][t] && hidden(pack, fy[t].kind)).collect();
            let size = |t: usize| pack.ships[fy[t].kind].hull * fy[t].health.max(0.1);
            let big = |t: usize| size(t).powf(1.5);
            let asw_all: f32 = (0..fx.len()).filter(|&i| fighting[x][i]).map(|i| pack.ships[fx[i].kind].asw * fx[i].health).sum();
            // (each ship lays its guns on one target, the bigger ones drawing more of them: fire
            // spread evenly over a fleet left every ship hurt and none sunk)
            let pick = |w: &dyn Fn(usize) -> f32, among: &[usize], rng: &mut Rng| -> Option<usize> {
                let all: f32 = among.iter().map(|&t| w(t)).sum();
                if all <= 0.0 {
                    return None;
                }
                let mut r = rng.f32() * all;
                for &t in among {
                    r -= w(t);
                    if r <= 0.0 {
                        return Some(t);
                    }
                }
                among.last().copied()
            };
            for i in (0..fx.len()).filter(|&i| fighting[x][i]) {
                let k = &pack.ships[fx[i].kind];
                let mut w = fx[i].health * skill(fx[i].exp) * (0.8 + 0.4 * rng.f32());
                // (the hidden strike first, unseen)
                if round == 0 && ambush[x] && hidden(pack, fx[i].kind) {
                    w *= 2.0;
                }
                if k.gun > 0.0 {
                    if let Some(t) = pick(&size, &seen, rng) {
                        harm[y][t] += k.gun * ROUND_K * w * pierce(k.gun, pack.ships[fy[t].kind].armor);
                    }
                }
                if k.torpedo > 0.0 && rng.f32() < HIT {
                    if let Some(t) = pick(&big, &seen, rng) {
                        harm[y][t] += k.torpedo * ROUND_K * w;
                    }
                }
                if k.asw > 0.0 && !under.is_empty() {
                    // a hidden ship is found by the depth charges of the whole side against how well
                    // it hides; then this ship's charges fall on it
                    let t = under[rng.below(under.len())];
                    let st = pack.ships[fy[t].kind].stealth;
                    if rng.f32() < asw_all / (asw_all + 4.0 * st + 1e-6) {
                        harm[y][t] += k.asw * ROUND_K * w;
                    }
                }
            }
        }
        for s in 0..2 {
            for t in 0..fleets[s].len() {
                if harm[s][t] <= 0.0 {
                    continue;
                }
                let hull = pack.ships[fleets[s][t].kind].hull.max(1e-3);
                let v = &mut fleets[s][t];
                v.health = (v.health - harm[s][t] / hull).max(0.0);
                if v.health > 0.0 && v.health <= SINKING && rng.f32() < SINK_CHANCE {
                    v.health = 0.0;
                }
                if v.health <= RETREAT {
                    fighting[s][t] = false;
                }
            }
        }
    }
    // who sank what, who went home hurt, who held the sea
    let mut left = [0.0f32; 2];
    for s in 0..2 {
        for v in fleets[s].iter_mut() {
            v.exp = (v.exp + 0.05).min(1.0);
        }
        for v in fleets[s].iter() {
            if v.health <= 0.0 {
                clash.sunk[s].push((v.kind, v.name.clone()));
            } else if v.health <= RETREAT {
                clash.hurt[s] += 1;
            }
        }
        left[s] = (0..fleets[s].len()).filter(|&t| fighting[s][t]).map(|t| power(pack, &fleets[s][t])).sum();
    }
    // (credit for the sunk to the ships that fought)
    for s in 0..2 {
        let n = clash.sunk[1 - s].len() as u32;
        if n > 0 {
            if let Some(v) = fleets[s].iter_mut().filter(|v| v.health > 0.0).max_by(|a, b| power(pack, a).total_cmp(&power(pack, b))) {
                v.sank += n;
            }
        }
    }
    clash.held = if left[0] > 1.5 * left[1] {
        Some(0)
    } else if left[1] > 1.5 * left[0] {
        Some(1)
    } else {
        None
    };
    clash
}

/// A month in port or at sea mends a ship by this much (in port: a yard; at sea: her own crew).
pub const MEND_PORT: f32 = 0.15;
pub const MEND_SEA: f32 = 0.01;

#[cfg(test)]
mod tests {
    use super::*;

    fn pack() -> Pack {
        Pack::load("packs/industrial.toml").expect("индустриальный набор")
    }

    fn kind(p: &Pack, id: &str) -> usize {
        p.ships.iter().position(|k| k.id == id).unwrap_or_else(|| panic!("нет корабля {id}"))
    }

    fn fleet(p: &Pack, mix: &[(&str, usize)]) -> Vec<Vessel> {
        let mut v = Vec::new();
        for &(id, n) in mix {
            for i in 0..n {
                v.push(Vessel::new(kind(p, id), format!("{id}{i}"), 0));
            }
        }
        v
    }

    /// Twin fleets: each side keeps about half of what is left after the battle (the control); twice
    /// the fleet holds the sea nearly always; ships do sink.
    #[test]
    fn the_bigger_fleet_holds_the_sea() {
        let p = pack();
        let mix: [(&str, usize); 3] = [("battleship", 2), ("cruiser", 3), ("destroyer", 6)];
        let (mut first, mut big, mut sunk) = (0.0f32, 0u32, 0usize);
        for seed in 0..400u64 {
            let mut rng = Rng::new(seed);
            let (mut a, mut b) = (fleet(&p, &mix), fleet(&p, &mix));
            let c = battle(&p, &mut a, &mut b, [false; 2], &mut rng);
            sunk += c.sunk[0].len() + c.sunk[1].len();
            let left = |f: &[Vessel]| f.iter().map(|v| power(&p, v)).sum::<f32>();
            first += left(&a) / (left(&a) + left(&b)).max(1e-6);
            let double: Vec<(&str, usize)> = mix.iter().map(|&(k, n)| (k, n * 2)).collect();
            let (mut a, mut b) = (fleet(&p, &double), fleet(&p, &mix));
            if battle(&p, &mut a, &mut b, [false; 2], &mut rng).held == Some(0) {
                big += 1;
            }
        }
        let first = first / 400.0;
        assert!((0.46..0.54).contains(&first), "близнецы: у первой стороны после боя {first:.3} оставшейся силы");
        assert!(big >= 300, "вдвое больший флот держал море в {big} боях из 400");
        assert!(sunk >= 200, "за 400 боёв близнецов потоплено лишь {sunk} кораблей");
    }

    /// Light guns barely scratch armour; torpedoes do not care.
    #[test]
    fn armour_turns_light_guns() {
        assert!(pierce(1.5, 10.0) < 0.2);
        assert_eq!(pierce(10.0, 10.0), 1.0);
        assert_eq!(pierce(2.0, 0.0), 1.0);
        let p = pack();
        let mut hurt_by_guns = 0.0;
        let mut hurt_by_torps = 0.0;
        for seed in 0..100u64 {
            let mut rng = Rng::new(seed);
            let mut bb = fleet(&p, &[("battleship", 1)]);
            let mut cr = fleet(&p, &[("cruiser", 2)]);
            battle(&p, &mut bb, &mut cr, [false; 2], &mut rng);
            hurt_by_guns += 1.0 - bb[0].health;
            let mut bb = fleet(&p, &[("battleship", 1)]);
            let mut dd = fleet(&p, &[("destroyer", 5)]);
            battle(&p, &mut bb, &mut dd, [false; 2], &mut rng);
            hurt_by_torps += 1.0 - bb[0].health;
        }
        assert!(hurt_by_torps > hurt_by_guns, "торпеды эсминцев ({hurt_by_torps:.1}) должны ранить линкор сильнее пушек крейсеров ({hurt_by_guns:.1})");
    }

    /// Without escorts, raiders sink more; a blockade weaker than half the guard does nothing.
    #[test]
    fn escorts_and_guards_count() {
        let p = pack();
        let f = floor(&p);
        assert!(raid_loss(&p, f, 0.0) > raid_loss(&p, f, 2.0 * f));
        assert_eq!(raid_loss(&p, 0.0, f), 0.0);
        assert!(raid_loss(&p, 1e6, 0.0) <= RAID_MOST);
        assert_eq!(blockade_loss(&p, f, 3.0 * f), 0.0);
        assert!(blockade_loss(&p, 3.0 * f, f) > 0.3);
    }

    /// Submarines with nothing to hunt them come home; against escorts with depth charges they die.
    #[test]
    fn only_depth_charges_find_submarines() {
        let p = pack();
        let mut lost_vs_cruisers = 0usize;
        let mut lost_vs_destroyers = 0usize;
        for seed in 0..200u64 {
            let mut rng = Rng::new(seed);
            let mut subs = fleet(&p, &[("submarine", 4)]);
            let mut cr = fleet(&p, &[("battleship", 1), ("cruiser", 2)]);
            battle(&p, &mut subs, &mut cr, [true, false], &mut rng);
            lost_vs_cruisers += subs.iter().filter(|v| v.health <= 0.0).count();
            let mut subs = fleet(&p, &[("submarine", 4)]);
            let mut dd = fleet(&p, &[("destroyer", 4)]);
            battle(&p, &mut subs, &mut dd, [true, false], &mut rng);
            lost_vs_destroyers += subs.iter().filter(|v| v.health <= 0.0).count();
        }
        assert_eq!(lost_vs_cruisers, 0, "у крейсеров и линкора нет глубинных бомб");
        assert!(lost_vs_destroyers > 40, "эсминцы потопили лишь {lost_vs_destroyers} лодок за 200 боёв");
    }
}
