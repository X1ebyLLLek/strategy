//! The staff's war games: what it believes each kind's mix would do against the enemy it sees, at
//! what odds; the norm of equipment by the country's prices; rethinking the plan in small steps or
//! a bold reform.

use super::*;

impl Country {
    /// How often the staff believes an army bought by `shares` beats the enemy as it sees
    /// it, over the ground of the front: attacking and defending, the same luck for every
    /// plan it compares.
    pub(super) fn games(&self, pack: &Pack, front: &[(Terrain, f32)], shares: &[f32], seed: u64) -> f32 {
        self.games_with(pack, front, shares, seed, None, 1.0)
    }

    /// How often the staff's war games would be won with its plan if one kind of its units were
    /// `better.1` times as good as it believes (fire and toughness), or its army `more` times as
    /// big: what a scientist asks the staff before he chooses what to work on.
    pub fn games_if(&self, pack: &Pack, front: &[(Terrain, f32)], seed: u64, better: Option<(usize, f32)>, more: f32) -> f32 {
        self.games_with(pack, front, &self.plan, seed, better, more)
    }

    fn games_with(&self, pack: &Pack, front: &[(Terrain, f32)], shares: &[f32], seed: u64, better: Option<(usize, f32)>, more: f32) -> f32 {
        self.games_norm(pack, front, shares, seed, better, more, None)
    }

    /// The war games with the kinds equipped by `norms` (None: by the country's own; a standard
    /// kit changes nothing — the old world to the bit).
    #[allow(clippy::too_many_arguments)]
    fn games_norm(&self, pack: &Pack, front: &[(Terrain, f32)], shares: &[f32], seed: u64, better: Option<(usize, f32)>, more: f32, norms: Option<&[f32]>) -> f32 {
        let nu = pack.units.len();
        let worth = Country::battle_worth(pack);
        let k = |u: usize| norms.map_or(self.norm_of(u), |v| v[u]);
        let mut ours = Army {
            count: (0..nu).map(|u| worth * shares[u] / if norms.is_some() { self.cost_at(u, k(u)) } else { self.cost[u] }).collect(),
            supplied: 1.0,
            fire: self.rated(&self.own_fire),
            tough: self.rated(&self.own_tough),
            dug: 0.0,
        };
        for u in 0..nu {
            ours.fire[u] *= norm::fire(k(u));
            ours.tough[u] *= norm::tough(k(u));
        }
        if let Some((k, m)) = better {
            ours.fire[k] *= m;
            ours.tough[k] *= m;
        }
        if more != 1.0 {
            ours.count.iter_mut().for_each(|c| *c *= more);
        }
        // the enemy's army of the same worth by our reckoning, as it is and as it would be if
        // it answered this plan: a share of its money moved to the kind that hurts it most
        let per: f32 = (0..nu).map(|u| self.foe_mix[u] * self.cost[u]).sum::<f32>().max(1e-9);
        let money: Vec<f32> = (0..nu).map(|u| self.foe_mix[u] * self.cost[u] / per).collect();
        let classes = pack.war.as_ref().unwrap().classes.len();
        let mut here = vec![0.0f32; classes];
        for u in 0..nu {
            here[pack.units[u].class] += ours.count[u];
        }
        let foe_fire = self.rated(&self.foe_fire);
        let foe_tough = self.rated(&self.foe_tough);
        let bite = |k: usize| (0..classes).map(|c| pack.units[k].fire[c] * here[c]).sum::<f32>() * foe_fire[k] / self.cost[k];
        let answer = (0..nu).max_by(|&a, &b| bite(a).total_cmp(&bite(b))).unwrap();
        let army_of = |money: &[f32]| Army {
            count: (0..nu).map(|u| worth * money[u] / self.cost[u]).collect(),
            supplied: 1.0,
            fire: foe_fire.clone(),
            tough: foe_tough.clone(),
            dug: 0.0,
        };
        let theirs = army_of(&money);
        let mut moved: Vec<f32> = money.iter().map(|m| m * (1.0 - BOLD_STEP)).collect();
        moved[answer] += BOLD_STEP;
        let answered = army_of(&moved);
        let play = |foe: &Army| {
            let mut won = 0.0;
            let mut all = 0.0;
            for (ti, &(t, w)) in front.iter().enumerate() {
                for g in 0..GAMES {
                    let mut rng = Rng::new(hash64(seed ^ ((ti * GAMES + g) as u64 * 0x9E37_79B9)));
                    let attack = battle(pack, t, [&ours, foe], Kind::Attack, &mut rng, false);
                    let defend = battle(pack, t, [foe, &ours], Kind::Attack, &mut rng, false);
                    won += w * ((attack.winner == Some(0)) as u32 as f32 + (defend.winner == Some(1)) as u32 as f32);
                    all += 2.0 * w;
                }
            }
            won / all
        };
        let now = play(&theirs);
        if self.caution > 0.0 {
            (1.0 - self.caution) * now + self.caution * play(&answered)
        } else {
            now
        }
    }

    /// The staff weighs the kit of each kind it builds (when it may choose, `norm_free`): a step
    /// lighter or heavier, by its war games, at what each costs this country — where goods are
    /// dear and men cheap a light kit pays (many men, few guns), where goods are cheap and men
    /// dear a heavy one. Nothing is set by a character: only prices and the games. Returns
    /// whether a norm changed.
    pub fn renorm(&mut self, pack: &Pack, front: &[(Terrain, f32)], seed: u64) -> bool {
        if !self.norm_free || self.fixed || front.is_empty() || self.parts.len() != pack.units.len() {
            return false;
        }
        let nu = pack.units.len();
        if self.norm.len() != nu {
            self.norm = vec![1.0; nu];
        }
        let mut cur = self.norm.clone();
        let mut best = self.games_norm(pack, front, &self.plan, seed, None, 1.0, Some(&cur));
        let mut changed = false;
        for u in 0..nu {
            if self.plan[u] < 0.03 || !pack.units[u].norms {
                continue;
            }
            let i = (0..norm::LEVELS.len()).min_by(|&a, &b| (norm::LEVELS[a] - cur[u]).abs().total_cmp(&(norm::LEVELS[b] - cur[u]).abs())).unwrap_or(1);
            for j in [i.wrapping_sub(1), i + 1] {
                if j >= norm::LEVELS.len() {
                    continue;
                }
                let mut v = cur.clone();
                v[u] = norm::LEVELS[j];
                let score = self.games_norm(pack, front, &self.plan, seed, None, 1.0, Some(&v));
                if score > best + BETTER * self.ready() {
                    best = score;
                    cur = v;
                    changed = true;
                    break;
                }
            }
        }
        if changed {
            self.norm = cur;
            self.price_norms();
        }
        changed
    }

    /// The staff's month: try moving a tenth of the budget from each kind to each other,
    /// keep the best if the war games say it is clearly better.
    pub(crate) fn rethink(&mut self, pack: &Pack, front: &[(Terrain, f32)], seed: u64) {
        if self.fixed {
            return;
        }
        let nu = pack.units.len();
        let now = self.games(pack, front, &self.plan, seed);
        let mut best = (now, self.plan.clone());
        for a in 0..nu {
            if self.plan[a] <= 1e-6 {
                continue;
            }
            for b in 0..nu {
                if a == b || self.is_locked(b) {
                    continue;
                }
                let mut s = self.plan.clone();
                let d = s[a].min(STEP);
                s[a] -= d;
                s[b] += d;
                let score = self.games(pack, front, &s, seed);
                if score > best.0 {
                    best = (score, s);
                }
            }
        }
        let mut rng = Rng::new(hash64(seed ^ 0xB01D));
        for _ in 0..BOLD {
            let (a, b) = (rng.below(nu), rng.below(nu));
            if a == b || self.plan[a] <= 1e-6 || self.is_locked(b) {
                continue;
            }
            let mut s = self.plan.clone();
            let d = s[a].min(BOLD_STEP);
            s[a] -= d;
            s[b] += d;
            let score = self.games(pack, front, &s, seed);
            if score > now + BOLD_BETTER * self.ready() && score > best.0 {
                best = (score, s);
            }
        }
        if best.0 > now + BETTER * self.ready() {
            self.plan = best.1;
        }
    }

    /// How much evidence the staff needs for a change, against a staff that never lost: a
    /// doubting staff needs only a fifth of it.
    pub(super) fn ready(&self) -> f32 {
        1.0 - 0.8 * self.doubt
    }
}
