//! The admiralty: the bot that leads a side's navy (the rival's always; the player's squadrons it
//! leads till he gives one an order of his own, and his yards till he lays down a ship himself).
//!
//! It wants what any admiralty wants — the most of the sea for the least: what the yards build
//! answers the enemy and the trade (a country living by its sea trade guards it with escorts; an
//! enemy living by his is hunted with submarines and raiders; a battle line weaker than the enemy's
//! is not worth finishing), and where the squadrons go answers the balance of the fleets (a fleet
//! much stronger blockades the enemy's ports, an even one holds its own coast, a weaker one stays in
//! port as a fleet in being and fights the war with raiders). Nothing about a country is written
//! beforehand: what it builds comes from its coast, its trade and its enemy.

use strategy::military::navy::{self, Mission};

use crate::Game;

/// A squadron this hurt (its mean health) goes home to be mended.
const MEND_AT: f32 = 0.55;
/// The fleet much stronger than the enemy's (by weight at sea): it blockades; even with it: it holds
/// its coast.
const STRONG: f32 = 1.3;
const EVEN: f32 = 0.8;

impl Game {
    /// A month of both admiralties: what the yards lay down, where the squadrons go.
    pub fn admiralty(&mut self) {
        if self.pack.ships.is_empty() {
            return;
        }
        self.navy_trade_now();
        for s in 0..2 {
            if self.navy.coast[s].is_empty() && self.navy.fleets[s].iter().all(|q| q.ships.is_empty()) {
                continue;
            }
            if self.navy.yard_staff[s] {
                self.admiralty_yards(s);
            }
            self.admiralty_orders(s);
        }
    }

    /// The share of its trade side `s` carries by sea (0 … 1).
    pub fn sea_dependence(&self, s: usize) -> f32 {
        if self.navy.all_trade[s] > 0.0 {
            self.navy.sea_trade[s] / self.navy.all_trade[s]
        } else {
            0.0
        }
    }

    /// The weight of side `s`'s ships of a place (`role`) at sea.
    fn role_power(&self, s: usize, role: &str) -> f32 {
        self.navy.fleets[s].iter().flat_map(|q| q.ships.iter()).filter(|v| self.pack.ships[v.kind].role == role).map(|v| navy::power(&self.pack, v)).sum()
    }

    /// What side `s`'s yards should lay down next: the place the fleet lacks most for its enemy and
    /// its trade (by what it has spent on each against what it wants), its cheapest kind for it.
    fn admiralty_yards(&mut self, s: usize) {
        let pack = self.pack.clone();
        if self.navy.yards[s].len() > self.navy.slipways[s] || self.navy.slipways[s] == 0 || self.navy_share(s) <= 0.0 {
            return;
        }
        let e = 1 - s;
        let ours = self.sea_dependence(s);
        let theirs = self.sea_dependence(e);
        let line = (self.role_power(s, "line") + 1e-3) / (self.role_power(e, "line") + 1e-3);
        let foe_subs = self.role_power(e, "sub") + self.role_power(e, "raider");
        let foe_all = self.navy_power(e).1.max(1e-3);
        // what each place is wanted for
        let want = |role: &str| -> f32 {
            match role {
                "line" => if line < 0.5 { 0.3 } else { 1.0 },
                "cruiser" => 0.5 + theirs + 0.5 * ours,
                "screen" => 0.5 + 1.5 * ours * (0.5 + foe_subs / foe_all),
                "sub" => 0.2 + 2.0 * theirs,
                _ => 0.2 + 1.2 * theirs,
            }
        };
        let roles: Vec<&str> = pack.ships.iter().map(|k| k.role.as_str()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        let all_want: f32 = roles.iter().map(|r| want(r)).sum::<f32>().max(1e-6);
        // spent so far on each place: ships in service and on the slipways
        let spent = |role: &str| -> f32 {
            let afloat: f32 = self.navy.fleets[s].iter().flat_map(|q| q.ships.iter()).filter(|v| pack.ships[v.kind].role == role).map(|v| pack.ships[v.kind].cost).sum();
            let building: f32 = self.navy.yards[s].iter().filter(|b| pack.ships[b.kind].role == role).map(|b| pack.ships[b.kind].cost).sum();
            afloat + building
        };
        let all_spent: f32 = roles.iter().map(|r| spent(r)).sum::<f32>().max(1e-6);
        let Some(role) = roles.iter().copied().max_by(|a, b| {
            let gap = |r: &str| want(r) / all_want - spent(r) / all_spent;
            gap(a).total_cmp(&gap(b)).then(b.cmp(a))
        }) else {
            return;
        };
        let Some(k) = (0..pack.ships.len()).filter(|&k| pack.ships[k].role == role).min_by(|&a, &b| pack.ships[a].cost.total_cmp(&pack.ships[b].cost)) else { return };
        self.lay_down(s, k);
    }

    /// Where side `s`'s squadrons led by the admiralty go: by the balance of the fleets, the trade by
    /// zone, the state of the ships and their supply.
    fn admiralty_orders(&mut self, s: usize) {
        let e = 1 - s;
        let nz = self.navy.seas.zones.len();
        if nz == 0 {
            return;
        }
        let ours = self.trade_by_zone(s);
        let theirs = self.trade_by_zone(e);
        let busiest = |v: &[f32], among: &[usize]| -> Option<usize> { among.iter().copied().filter(|&z| z < v.len()).max_by(|&a, &b| v[a].total_cmp(&v[b]).then(b.cmp(&a))) };
        let all_zones: Vec<usize> = (0..nz).collect();
        let home = self.navy.home[s].clone();
        let foe_home = self.navy.home[e].clone();
        let my_home = busiest(&ours, &home).or(home.first().copied()).unwrap_or(0);
        let my_lane = busiest(&ours, &all_zones).filter(|&z| ours[z] > 0.0).unwrap_or(my_home);
        let foe_lane = busiest(&theirs, &all_zones).filter(|&z| theirs[z] > 0.0);
        let foe_port = busiest(&theirs, &foe_home).or(foe_home.first().copied());
        let ratio = self.navy_power(s).1 / self.navy_power(e).1.max(1e-3);
        let fed = self.navy.fed[s];
        let war = self.h.at_war;
        let pack = self.pack.clone();
        let line_order = if !war {
            (Mission::Port, my_home)
        } else if ratio >= STRONG && foe_port.is_some() {
            (Mission::Blockade, foe_port.unwrap())
        } else if ratio >= EVEN {
            (Mission::Patrol, my_home)
        } else {
            (Mission::Port, my_home)
        };
        for q in self.navy.fleets[s].iter_mut().filter(|q| q.staff && !q.ships.is_empty()) {
            let role = pack.ships[q.ships[0].kind].role.clone();
            let health = q.ships.iter().map(|v| v.health).sum::<f32>() / q.ships.len() as f32;
            let (m, z) = if health < MEND_AT || fed < 0.4 && role != "sub" {
                (Mission::Port, my_home)
            } else if !war {
                match role.as_str() {
                    "line" | "sub" => (Mission::Port, my_home),
                    _ => (Mission::Patrol, my_lane),
                }
            } else {
                match role.as_str() {
                    "line" => line_order,
                    "cruiser" => match foe_lane {
                        Some(z) if theirs[z] > 0.5 * ours.get(my_lane).copied().unwrap_or(0.0) => (Mission::Raid, z),
                        _ => (Mission::Escort, my_lane),
                    },
                    "screen" => {
                        if line_order.0 == Mission::Blockade || line_order.0 == Mission::Patrol {
                            if ours.get(my_lane).copied().unwrap_or(0.0) > 0.0 && self.navy.raids[e].iter().sum::<f32>() > 0.0 {
                                (Mission::Escort, my_lane)
                            } else {
                                (line_order.0, line_order.1)
                            }
                        } else {
                            (Mission::Escort, my_lane)
                        }
                    }
                    _ => match foe_lane.or(foe_port) {
                        Some(z) => (Mission::Raid, z),
                        None => (Mission::Port, my_home),
                    },
                }
            };
            q.mission = m;
            q.zone = z;
        }
    }
}
