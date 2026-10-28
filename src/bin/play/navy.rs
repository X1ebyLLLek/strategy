//! The navy in the game: each side's fleet in squadrons by their place (the line, the cruisers, the
//! light forces, the submarines, the raiders), its shipyards and what they build, the zones of the
//! sea and the ways of trade through them; a month at sea — the fleet's supply and its building paid
//! out of the army's money (`side` orders: the market sells the steel and the fuel), ships mended,
//! new ones commissioned with crews called up like soldiers; at war, battles where the fleets meet,
//! raiders against convoys, blockades of ports — and what is sunk on the way is goods that never
//! arrive: merchants see the losses and carry less (`trade::World::hamper`).

pub mod admiralty;
pub mod seas;

use std::sync::Arc;

use strategy::military::navy::{self, Clash, Mission, Squadron, Vessel, MEND_PORT, MEND_SEA, RETREAT};
use strategy::rng::{hash64, Rng};
use strategy::world::names;

use crate::game::Note;
use crate::Game;
use seas::{SeaWay, Seas};

/// The share of the army's money a coastal state gives its navy by custom (more with a long coast
/// and much trade by sea, none without a coast).
pub const NAVY_SHARE: f32 = 0.15;
/// Months of the navy's money the years before the game built the fleet with.
const START_MONTHS: f32 = 24.0;
/// Battles at sea kept for the record.
const CLASHES: usize = 80;
/// Of a ship's crew, the share that goes down with her.
const DROWNED: f32 = 0.5;

/// A ship on the slipway: her kind, the name she will bear, how far she is built (0 … 1), the month
/// she was laid down.
#[derive(Clone, Debug)]
pub struct Build {
    pub kind: usize,
    pub name: String,
    pub done: f32,
    pub since: u32,
}

/// A battle at sea for the record: the month, what it did, the zone's name.
#[derive(Clone, Debug)]
pub struct Battle {
    pub month: u32,
    pub clash: Clash,
    pub zone: String,
}

/// Both sides' navies, the seas and the trade through them.
#[derive(Clone, Default)]
pub struct Navy {
    pub seas: Arc<Seas>,
    pub fleets: [Vec<Squadron>; 2],
    pub yards: [Vec<Build>; 2],
    /// Ships each side's yards build at once.
    pub slipways: [usize; 2],
    /// The share of the army's money the government gives the navy (none: the custom, or the
    /// admiralty's).
    pub share: [Option<f32>; 2],
    /// Whether the admiralty chooses what the yards build (the player's own order takes it).
    pub yard_staff: [bool; 2],
    /// Each side's ways of trade to every country of the map; the zones on its coast; its coastal
    /// provinces.
    pub ways: [Vec<SeaWay>; 2],
    pub home: [Vec<usize>; 2],
    pub coast: [Vec<usize>; 2],
    /// Share of each country's trade with the side that the enemy sank this month; the worth of
    /// what was sunk (at usual prices), of the side's trade by sea and of all of it, this month.
    pub lost: [Vec<f32>; 2],
    pub sunk_trade: [f32; 2],
    pub sea_trade: [f32; 2],
    pub all_trade: [f32; 2],
    /// Money by good the navy asks of the market this month; the goods it got (`mil_after`); what of
    /// it was for the fleet's supply and for the yards (the full needs, and what was paid for).
    pub ask: [Vec<f32>; 2],
    pub got: [Vec<f32>; 2],
    need_supply: [Vec<f32>; 2],
    paid_supply: [Vec<f32>; 2],
    paid_build: [Vec<f32>; 2],
    need_build: [Vec<f32>; 2],
    /// Share of its supply the fleet got last month.
    pub fed: [f32; 2],
    /// Battles at sea, newest last.
    pub clashes: Vec<Battle>,
    /// Each side's share of the sea in each zone (by the ships that sail it), last month.
    pub control: [Vec<f32>; 2],
    /// Each side's raiders' power in each zone, last month (for the map).
    pub raids: [Vec<f32>; 2],
    /// Ships each side lost, and the worth of its merchants' goods sunk, since the war began.
    pub ships_lost: [u32; 2],
    pub goods_lost: [f32; 2],
    /// Whether the ways of trade are hampered now (they are set free at peace).
    pub hampered: bool,
    /// Names given so far (for the next ship's).
    pub named: u32,
}

/// The name of the squadron of a role.
fn squadron_name(role: &str) -> &'static str {
    match role {
        "line" => "Линейная эскадра",
        "cruiser" => "Крейсерская эскадра",
        "screen" => "Флотилия лёгких сил",
        "sub" => "Подводные силы",
        _ => "Рейдерская флотилия",
    }
}

/// Ships sunk in words: a name when one or two of a kind, else how many of each kind.
pub fn sunk_words(pack: &strategy::pack::Pack, v: &[(usize, String)]) -> String {
    let mut kinds: Vec<usize> = v.iter().map(|x| x.0).collect();
    kinds.sort();
    kinds.dedup();
    kinds
        .iter()
        .map(|&k| {
            let these: Vec<&String> = v.iter().filter(|x| x.0 == k).map(|x| &x.1).collect();
            match these.len() {
                1 => format!("{} {}", pack.ships[k].name, these[0]),
                2 => format!("{} {} и {}", pack.ships[k].name, these[0], these[1]),
                n => format!("{} × {n}", pack.ships[k].name),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

impl Game {
    /// The seas of the map, each side's coast, ports, ways of trade and yards; the fleets the years
    /// before built (`START_MONTHS` of the custom share), in squadrons by their place.
    pub fn navy_start(&mut self) {
        let pack = self.pack.clone();
        self.navy.seas = Arc::new(Seas::of(&self.world, self.seed));
        self.navy.yard_staff = [true; 2];
        for s in 0..2 {
            self.navy_coast(s);
            self.navy_ways(s);
            let n = self.navy.coast[s].len();
            let dev = self.navy.coast[s].iter().map(|&p| self.prov_dev.get(p).copied().unwrap_or(0.5)).sum::<f32>() / n.max(1) as f32;
            self.navy.slipways[s] = if n == 0 || pack.ships.is_empty() { 0 } else { (1.0 + n as f32 * (0.5 + dev) / 6.0).round().clamp(1.0, 8.0) as usize };
            self.navy.fed[s] = 1.0;
            self.navy.lost[s] = vec![0.0; self.capitals.len()];
            let nz = self.navy.seas.zones.len();
            self.navy.control[s] = vec![0.0; nz];
            self.navy.raids[s] = vec![0.0; nz];
        }
        if pack.ships.is_empty() {
            return;
        }
        let scale = pack.war.as_ref().map_or(0.02, |w| w.scale);
        let mut rng = Rng::new(hash64(self.seed ^ 0xF1EE7));
        for s in 0..2 {
            if self.navy.coast[s].is_empty() {
                continue;
            }
            let money = self.navy_share(s) * self.h.home(s).army_money() * START_MONTHS;
            let prices = self.h.home(s).price.clone();
            // the custom mix of the years before: by the worth spent on each place
            let mix = [("line", 0.35f32), ("cruiser", 0.25), ("screen", 0.25), ("sub", 0.1), ("raider", 0.05)];
            let have: Vec<&str> = pack.ships.iter().map(|k| k.role.as_str()).collect();
            let all: f32 = mix.iter().filter(|m| have.contains(&m.0)).map(|m| m.1).sum::<f32>().max(1e-6);
            for (role, w) in mix {
                let Some(k) = pack.ships.iter().position(|x| x.role == role) else { continue };
                let price: f32 = pack.ships[k].equip.iter().map(|&(i, a)| a * scale * prices[i]).sum::<f32>().max(1e-9);
                let count = (money * w / all / price).floor() as usize;
                for _ in 0..count.min(16) {
                    let name = self.ship_name(k, &mut rng);
                    self.navy_commission(s, Vessel::new(k, name, 0));
                }
            }
            // their crews serve
            let men: f32 = self.navy.fleets[s].iter().flat_map(|q| q.ships.iter()).map(|v| pack.ships[v.kind].crew / 1000.0).sum();
            self.h.home_mut(s).draft(men);
        }
        self.admiralty();
    }

    /// Side `s`'s coastal provinces now and the zones of the sea on their coast.
    pub fn navy_coast(&mut self, s: usize) {
        let war = &self.h.war;
        let coast: Vec<usize> = (0..war.owner.len())
            .filter(|&q| war.owner[q] == s)
            .map(|q| war.th.provs[q])
            .filter(|&p| self.world.provinces[p].neighbors.iter().any(|&n| self.world.provinces[n as usize].sea))
            .collect();
        self.navy.home[s] = self.navy.seas.zones_by(&self.world, &coast);
        self.navy.coast[s] = coast;
    }

    /// Side `s`'s ways of trade from its capital now.
    pub fn navy_ways(&mut self, s: usize) {
        let Some(t) = self.pack.trade.as_ref() else { return };
        let war = &self.h.war;
        let from = war.th.provs.get(war.th.capital[s]).copied().unwrap_or(usize::MAX);
        let mut caps = self.capitals.clone();
        for x in 0..2 {
            if let Some(&p) = war.th.provs.get(war.th.capital[x]) {
                if let Some(c) = caps.get_mut(war.th.countries[x]) {
                    *c = p;
                }
            }
        }
        self.navy.ways[s] = seas::ways(&self.world, &self.navy.seas, from, &caps, t);
    }

    /// A name for a new ship of kind `k`: a place for the big ones, a letter and a number for the
    /// small.
    pub fn ship_name(&mut self, k: usize, rng: &mut Rng) -> String {
        self.navy.named += 1;
        let role = self.pack.ships[k].role.as_str();
        match role {
            "sub" => format!("П-{}", self.navy.named),
            "raider" => format!("Р-{}", self.navy.named),
            _ => {
                let mut taken = std::collections::HashSet::new();
                format!("«{}»", names::place(rng, &mut taken))
            }
        }
    }

    /// A new ship joins the squadron of her place (a new squadron when it has none), in its zone.
    fn navy_commission(&mut self, s: usize, v: Vessel) {
        let role = self.pack.ships[v.kind].role.clone();
        let home = self.navy.home[s].first().copied().unwrap_or(0);
        if let Some(q) = self.navy.fleets[s].iter_mut().find(|q| q.ships.first().is_some_and(|x| self.pack.ships[x.kind].role == role) || (q.ships.is_empty() && q.name == squadron_name(&role))) {
            q.ships.push(v);
            return;
        }
        let mut rng = Rng::new(hash64(self.seed ^ 0xAD31 ^ (s as u64) << 8 ^ self.navy.fleets[s].len() as u64));
        self.navy.fleets[s].push(Squadron { name: squadron_name(&role).to_string(), admiral: names::person(&mut rng), ships: vec![v], mission: Mission::Port, zone: home, staff: true });
    }

    /// The share of the army's money side `s`'s government gives its navy: its own, else the custom
    /// one (none without a coast; more with a long coast and much trade by sea).
    pub fn navy_share(&self, s: usize) -> f32 {
        if let Some(x) = self.navy.share[s] {
            return x;
        }
        if self.navy.coast[s].is_empty() || self.pack.ships.is_empty() {
            return 0.0;
        }
        let all = self.h.war.owner.iter().filter(|&&o| o == s).count().max(1) as f32;
        let coast = self.navy.coast[s].len() as f32 / all;
        let sea = if self.navy.all_trade[s] > 0.0 { self.navy.sea_trade[s] / self.navy.all_trade[s] } else { 0.5 };
        (NAVY_SHARE * (0.5 + 2.0 * coast).clamp(0.5, 1.6) * (0.6 + 0.8 * sea)).clamp(0.03, 0.4)
    }

    /// The pay of side `s`'s sailors (they are paid as soldiers: it is in the army's pay).
    pub fn navy_crew_pay(&self, s: usize) -> f32 {
        let n = self.h.home(s);
        let men: f32 = self.navy.fleets[s].iter().flat_map(|q| q.ships.iter()).map(|v| self.pack.ships[v.kind].crew / 1000.0).sum();
        men * n.soldier_pay() * n.draft_pay
    }

    /// Ships in service on side `s`, and their weight at sea.
    pub fn navy_power(&self, s: usize) -> (usize, f32) {
        let ships: Vec<&Vessel> = self.navy.fleets[s].iter().flat_map(|q| q.ships.iter()).collect();
        (ships.len(), ships.iter().map(|v| navy::power(&self.pack, v)).sum::<f32>() + 0.0)
    }

    /// Before the world's month: what each navy asks of the market — the supply of its ships (at sea
    /// in full, in port a third) first, then the building on its slipways, as far as its share of the
    /// army's money goes.
    pub fn navy_before(&mut self) {
        let pack = self.pack.clone();
        let nk = pack.goods.len();
        let scale = pack.war.as_ref().map_or(0.02, |w| w.scale);
        for s in 0..2 {
            self.navy.ask[s] = vec![0.0; nk];
            self.navy.need_supply[s] = vec![0.0; nk];
            self.navy.paid_supply[s] = vec![0.0; nk];
            self.navy.need_build[s] = vec![0.0; nk];
            self.navy.paid_build[s] = vec![0.0; nk];
            if pack.ships.is_empty() {
                continue;
            }
            let n = self.h.home(s);
            let wages: f32 = n.soldier_wages().iter().sum();
            let money = self.navy_share(s) * (n.state_budget() * n.mil_share - wages).max(0.0);
            let mut sup = vec![0.0f32; nk];
            for q in &self.navy.fleets[s] {
                let k = if q.mission.at_sea() { 1.0 } else { 1.0 / 3.0 };
                for v in &q.ships {
                    for &(i, a) in &pack.ships[v.kind].supply {
                        sup[i] += a * scale * k;
                    }
                }
            }
            let mut bld = vec![0.0f32; nk];
            for b in self.navy.yards[s].iter().take(self.navy.slipways[s]) {
                let k = &pack.ships[b.kind];
                for &(i, a) in &k.equip {
                    bld[i] += a * scale / k.months as f32;
                }
            }
            let p = &n.price;
            let sup_money: f32 = (0..nk).map(|i| sup[i] * p[i]).sum();
            let bld_money: f32 = (0..nk).map(|i| bld[i] * p[i]).sum();
            let k_sup = if sup_money > money { money / sup_money.max(1e-9) } else { 1.0 };
            let k_bld = if bld_money > 0.0 { ((money - sup_money).max(0.0) / bld_money).min(1.0) } else { 0.0 };
            for i in 0..nk {
                self.navy.ask[s][i] = (sup[i] * k_sup + bld[i] * k_bld) * p[i];
                self.navy.paid_supply[s][i] = sup[i] * k_sup;
                self.navy.paid_build[s][i] = bld[i] * k_bld;
            }
            self.navy.need_supply[s] = sup;
            self.navy.need_build[s] = bld;
        }
    }

    /// After the world's month: the fleets' supply and their yards' work from what the navy got; ships
    /// mended, new ones commissioned (their crews called up); at war, the war at sea; the admiralty
    /// decides.
    pub fn navy_month(&mut self) -> Vec<String> {
        let pack = self.pack.clone();
        let mut alerts = Vec::new();
        if pack.ships.is_empty() {
            return alerts;
        }
        let nk = pack.goods.len();
        let now = self.month();
        let mut rng = Rng::new(hash64(self.seed ^ 0x5EA ^ (now as u64) << 12));
        // (the coasts change with a war; the ways with a capital, and yearly)
        if self.h.at_war || now % 12 == 0 {
            for s in 0..2 {
                self.navy_coast(s);
                if now % 12 == 0 || self.h.at_war && now % 3 == 0 {
                    self.navy_ways(s);
                }
            }
        }
        for s in 0..2 {
            let got = self.navy.got[s].clone();
            let (mut sup_got, mut bld_got) = (vec![0.0f32; nk], vec![0.0f32; nk]);
            for i in 0..nk.min(got.len()) {
                let (a, b) = (self.navy.paid_supply[s][i], self.navy.paid_build[s][i]);
                if a + b > 0.0 {
                    sup_got[i] = got[i] * a / (a + b);
                    bld_got[i] = got[i] * b / (a + b);
                }
            }
            let worth = |v: &[f32]| (0..nk).map(|i| v.get(i).copied().unwrap_or(0.0) * pack.goods[i].price).sum::<f32>();
            let need = worth(&self.navy.need_supply[s]);
            self.navy.fed[s] = if need > 0.0 { (worth(&sup_got) / need).clamp(0.0, 1.0) } else { 1.0 };
            // the yards: each ship on a slipway gets her part of what came
            let want = worth(&self.navy.need_build[s]);
            let k = if want > 0.0 { (worth(&bld_got) / want).clamp(0.0, 1.0) } else { 0.0 };
            let ways = self.navy.slipways[s];
            for b in self.navy.yards[s].iter_mut().take(ways) {
                b.done = (b.done + k / pack.ships[b.kind].months as f32).min(1.0);
            }
            // ready ships take their crews and join their squadrons
            let mut i = 0;
            while i < self.navy.yards[s].len() {
                if self.navy.yards[s][i].done < 1.0 {
                    i += 1;
                    continue;
                }
                let kind = self.navy.yards[s][i].kind;
                let men = pack.ships[kind].crew / 1000.0;
                if self.h.home(s).draftable() < men {
                    i += 1;
                    continue;
                }
                let b = self.navy.yards[s].remove(i);
                self.h.home_mut(s).draft(men);
                let line = format!("в строй вошёл {} {}", pack.ships[kind].name, b.name);
                if s == self.me {
                    self.news.push(Note::new(now, &line, false, "navy"));
                }
                self.navy_commission(s, Vessel::new(kind, b.name, now));
            }
            // mending
            let fed = self.navy.fed[s];
            for q in self.navy.fleets[s].iter_mut() {
                let mend = if q.mission == Mission::Port { MEND_PORT } else { MEND_SEA };
                for v in q.ships.iter_mut() {
                    v.health = (v.health + mend * fed).min(1.0);
                }
            }
        }
        if self.h.at_war {
            alerts.extend(self.sea_war(&mut rng));
        } else if self.navy.hampered {
            self.navy_free_ways();
        }
        self.admiralty();
        alerts
    }

    /// The ways of trade of both sides free again (the war at sea is over).
    pub fn navy_free_ways(&mut self) {
        let pack = self.pack.clone();
        let nc = self.capitals.len();
        for s in 0..2 {
            let a = self.h.war.th.countries[s];
            for j in 0..nc {
                if j != a {
                    self.h.world.hamper(&pack, a, j, 1.0);
                }
            }
            self.navy.lost[s] = vec![0.0; nc];
        }
        self.navy.sunk_trade = [0.0; 2];
        self.navy.hampered = false;
    }

    /// The war at sea this month: where the fleets meet they fight; raiders against escorts; blockades
    /// against the guard of the ports; what the enemy sinks of each side's trade.
    fn sea_war(&mut self, rng: &mut Rng) -> Vec<String> {
        let pack = self.pack.clone();
        let now = self.month();
        let me = self.me;
        let nz = self.navy.seas.zones.len();
        let mut alerts = Vec::new();
        // the squadrons at sea in each zone, and the fleets in being on their own coast
        let at = |g: &Game, s: usize, z: usize| -> Vec<usize> {
            (0..g.navy.fleets[s].len()).filter(|&k| {
                let q = &g.navy.fleets[s][k];
                !q.ships.is_empty() && (q.zone == z && q.mission.at_sea() || q.mission == Mission::Port && g.navy.home[s].first() == Some(&z))
            }).collect()
        };
        for z in 0..nz {
            let (a, b) = (at(self, 0, z), at(self, 1, z));
            if a.is_empty() || b.is_empty() {
                continue;
            }
            // (two fleets in port on their own coasts do not meet)
            let sailing = |s: usize, v: &[usize], g: &Game| v.iter().any(|&k| g.navy.fleets[s][k].mission.at_sea());
            if !sailing(0, &a, self) && !sailing(1, &b, self) {
                continue;
            }
            let ships = |s: usize, v: &[usize], g: &Game| -> Vec<Vessel> { v.iter().flat_map(|&k| g.navy.fleets[s][k].ships.iter().filter(|x| x.health > RETREAT).cloned()).collect() };
            let (mut fa, mut fb) = (ships(0, &a, self), ships(1, &b, self));
            if fa.is_empty() || fb.is_empty() {
                continue;
            }
            let ra: Vec<&Vessel> = fa.iter().collect();
            let rb: Vec<&Vessel> = fb.iter().collect();
            let chance = navy::meet_chance(navy::search(&pack, &ra), navy::search(&pack, &rb));
            if rng.f32() >= chance {
                continue;
            }
            let ambush = [fa.iter().any(|v| navy::hidden(&pack, v.kind)), fb.iter().any(|v| navy::hidden(&pack, v.kind))];
            let mut clash = navy::battle(&pack, &mut fa, &mut fb, ambush, rng);
            clash.zone = z;
            // the ships as the battle left them back to their squadrons; the sunk gone, their crews
            // with many of them
            for (s, list, fought) in [(0usize, &a, &fa), (1usize, &b, &fb)] {
                let mut j = 0;
                let mut drowned = 0.0f32;
                for &k in list.iter() {
                    for v in self.navy.fleets[s][k].ships.iter_mut().filter(|x| x.health > RETREAT) {
                        if let Some(w) = fought.get(j) {
                            *v = w.clone();
                        }
                        j += 1;
                    }
                    let before = self.navy.fleets[s][k].ships.len();
                    let crew: f32 = self.navy.fleets[s][k].ships.iter().filter(|v| v.health <= 0.0).map(|v| pack.ships[v.kind].crew / 1000.0).sum();
                    drowned += crew;
                    self.navy.fleets[s][k].ships.retain(|v| v.health > 0.0);
                    self.navy.ships_lost[s] += (before - self.navy.fleets[s][k].ships.len()) as u32;
                }
                if drowned > 0.0 {
                    let n = self.h.home_mut(s);
                    n.bury(drowned * DROWNED, true);
                    n.release(drowned * (1.0 - DROWNED));
                    self.h.dead[s] += drowned * DROWNED;
                }
            }
            let zone = self.navy.seas.zones[z].name.clone();
            let ours = &clash.sunk[me];
            let theirs = &clash.sunk[1 - me];
            let held = match clash.held {
                Some(x) if x == me => "море за нами",
                Some(_) => "море за врагом",
                None => "ни одна сторона не взяла верх",
            };
            let list = |v: &[(usize, String)]| sunk_words(&pack, v);
            let line = format!(
                "бой на море ({zone}): {}{}; {held}",
                if theirs.is_empty() { "враг не потерял ни одного корабля".to_string() } else { format!("потоплены вражеские: {}", list(theirs)) },
                if ours.is_empty() { String::new() } else { format!("; мы потеряли: {}", list(ours)) }
            );
            alerts.push(line.clone());
            self.news.push(Note::new(now, &line, true, "navy").with_tip(format!("сила флотов перед боем: наш {:.0}, вражеский {:.0}; раундов {}", clash.weight[me], clash.weight[1 - me], clash.rounds)));
            self.navy.clashes.push(Battle { month: now, clash, zone });
            if self.navy.clashes.len() > CLASHES {
                self.navy.clashes.remove(0);
            }
        }
        // each side's weight in each zone: at sea for the sea, against merchants, for convoys, at the
        // ports; and the fleet in being on its own coast
        let mut patrol = [vec![0.0f32; nz], vec![0.0f32; nz]];
        let mut raid = [vec![0.0f32; nz], vec![0.0f32; nz]];
        let mut escort = [vec![0.0f32; nz], vec![0.0f32; nz]];
        let mut block = [vec![0.0f32; nz], vec![0.0f32; nz]];
        for s in 0..2 {
            let fed = self.navy.fed[s].clamp(0.2, 1.0);
            for q in &self.navy.fleets[s] {
                if q.zone >= nz {
                    continue;
                }
                let p: f32 = q.ships.iter().map(|v| navy::power(&pack, v)).sum::<f32>() * fed;
                let r: f32 = q.ships.iter().map(|v| navy::raid_power(&pack, v)).sum::<f32>() * fed;
                let e: f32 = q.ships.iter().map(|v| navy::escort_power(&pack, v)).sum::<f32>() * fed;
                match q.mission {
                    Mission::Port => {
                        for &z in &self.navy.home[s] {
                            patrol[s][z] += 0.3 * p / self.navy.home[s].len().max(1) as f32;
                        }
                    }
                    Mission::Patrol => {
                        patrol[s][q.zone] += p;
                        escort[s][q.zone] += 0.5 * e;
                    }
                    Mission::Escort => {
                        escort[s][q.zone] += e;
                        patrol[s][q.zone] += 0.5 * p;
                    }
                    Mission::Raid => raid[s][q.zone] += r,
                    Mission::Blockade => {
                        block[s][q.zone] += p + r;
                        patrol[s][q.zone] += 0.5 * p;
                    }
                }
            }
        }
        for s in 0..2 {
            for z in 0..nz {
                let all = patrol[0][z] + patrol[1][z];
                self.navy.control[s][z] = if all > 0.0 { patrol[s][z] / all } else { 0.0 };
            }
            self.navy.raids[s] = raid[s].clone();
        }
        // escorts sink raiders: each raider in a guarded zone may meet the guard
        for s in 0..2 {
            let e = 1 - s;
            for k in 0..self.navy.fleets[s].len() {
                let q = &self.navy.fleets[s][k];
                if q.mission != Mission::Raid || q.zone >= nz {
                    continue;
                }
                let z = q.zone;
                let guard = escort[e][z] + 0.3 * patrol[e][z];
                if guard <= 0.0 {
                    continue;
                }
                let p = 0.12 * guard / (guard + raid[s][z] + navy::floor(&pack));
                let mut drowned = 0.0f32;
                let mut lost: Vec<(usize, String)> = Vec::new();
                for v in self.navy.fleets[s][k].ships.iter_mut() {
                    if rng.f32() < p {
                        v.health -= 0.4 + 0.6 * rng.f32();
                        if v.health <= 0.0 {
                            drowned += pack.ships[v.kind].crew / 1000.0;
                            lost.push((v.kind, v.name.clone()));
                        }
                    }
                }
                let before = self.navy.fleets[s][k].ships.len();
                self.navy.fleets[s][k].ships.retain(|v| v.health > 0.0);
                self.navy.ships_lost[s] += (before - self.navy.fleets[s][k].ships.len()) as u32;
                if drowned > 0.0 {
                    let n = self.h.home_mut(s);
                    n.bury(drowned * DROWNED, true);
                    n.release(drowned * (1.0 - DROWNED));
                    self.h.dead[s] += drowned * DROWNED;
                }
                if !lost.is_empty() {
                    let zone = &self.navy.seas.zones[z].name;
                    let lost = sunk_words(&pack, &lost);
                    let line = if s == me { format!("конвойные корабли врага потопили наши рейдеры ({zone}): {lost}") } else { format!("наши конвои отбились от рейдеров ({zone}): потоплены {lost}") };
                    self.news.push(Note::new(now, &line, true, "navy"));
                }
            }
        }
        // what the enemy sinks of each side's trade: on every way of it through the zones he raids,
        // and at its ports he blockades
        let nc = self.capitals.len();
        for e in 0..2 {
            let s = 1 - e;
            let a = self.h.war.th.countries[e];
            let foe = self.h.war.th.countries[s];
            let (mut sunk, mut sea, mut all) = (0.0f32, 0.0f32, 0.0f32);
            let mut lost = vec![0.0f32; nc];
            for j in 0..nc {
                if j == a || j == foe {
                    continue;
                }
                let Some(way) = self.navy.ways[e].get(j) else { continue };
                let mut keep = 1.0f32;
                for &z in &way.zones {
                    keep *= 1.0 - navy::raid_loss(&pack, raid[s][z], escort[e][z] + 0.5 * patrol[e][z]);
                    if self.navy.home[e].contains(&z) {
                        keep *= 1.0 - navy::blockade_loss(&pack, block[s][z], patrol[e][z]);
                    }
                }
                lost[j] = (1.0 - keep).clamp(0.0, 0.99);
                self.h.world.hamper(&pack, a, j, keep);
                let w = &self.h.world;
                let trade = w.carried.get(a).and_then(|r| r.get(j)).copied().unwrap_or(0.0) + w.carried.get(j).and_then(|r| r.get(a)).copied().unwrap_or(0.0);
                all += trade;
                if !way.zones.is_empty() {
                    sea += trade;
                    sunk += trade * lost[j] / (1.0 - lost[j]).max(0.05);
                }
            }
            self.navy.lost[e] = lost;
            self.navy.sunk_trade[e] = sunk;
            self.navy.sea_trade[e] = sea;
            self.navy.all_trade[e] = all;
            self.navy.goods_lost[e] += sunk;
        }
        self.navy.hampered = true;
        alerts
    }

    /// The war begins at sea: the losses are counted from now.
    pub fn navy_war_begins(&mut self) {
        self.navy.ships_lost = [0; 2];
        self.navy.goods_lost = [0.0; 2];
    }

    /// The trade of side `s` with the world now (by sea and all of it), in peace too (for the
    /// admiralty and the panel).
    pub fn navy_trade_now(&mut self) {
        let nc = self.capitals.len();
        for s in 0..2 {
            let a = self.h.war.th.countries[s];
            let (mut sea, mut all) = (0.0f32, 0.0f32);
            for j in 0..nc {
                if j == a {
                    continue;
                }
                let w = &self.h.world;
                let trade = w.carried.get(a).and_then(|r| r.get(j)).copied().unwrap_or(0.0) + w.carried.get(j).and_then(|r| r.get(a)).copied().unwrap_or(0.0);
                all += trade;
                if self.navy.ways[s].get(j).is_some_and(|w| !w.zones.is_empty()) {
                    sea += trade;
                }
            }
            if !self.h.at_war {
                self.navy.sea_trade[s] = sea;
                self.navy.all_trade[s] = all;
            }
        }
    }

    /// The worth of trade of side `s` passing each zone of the sea (at usual prices, a month).
    pub fn trade_by_zone(&self, s: usize) -> Vec<f32> {
        let nz = self.navy.seas.zones.len();
        let mut v = vec![0.0f32; nz];
        let a = self.h.war.th.countries[s];
        let w = &self.h.world;
        for way in &self.navy.ways[s] {
            let j = way.to;
            if j == a {
                continue;
            }
            let trade = w.carried.get(a).and_then(|r| r.get(j)).copied().unwrap_or(0.0) + w.carried.get(j).and_then(|r| r.get(a)).copied().unwrap_or(0.0);
            for &z in &way.zones {
                if z < nz {
                    v[z] += trade;
                }
            }
        }
        v
    }

    /// Side `s` lays down a ship of kind `k` (the player's order takes the yards from the admiralty).
    pub fn lay_down(&mut self, s: usize, k: usize) {
        if k >= self.pack.ships.len() || self.navy.coast[s].is_empty() {
            return;
        }
        let mut rng = Rng::new(hash64(self.seed ^ 0x1A1D ^ (self.month() as u64) << 10 ^ self.navy.named as u64));
        let name = self.ship_name(k, &mut rng);
        self.navy.yards[s].push(Build { kind: k, name, done: 0.0, since: self.month() });
    }
}
