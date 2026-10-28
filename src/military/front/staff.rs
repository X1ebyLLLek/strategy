//! The staff's month on the map: the orders to the groups and where the army stands — the
//! line held by the enemy seen, the striking force placed by war games, the reserve.

use super::*;

impl Campaign {
    /// The staff of side `s`, leading by groups, gives them shares and orders for next month by
    /// what it sees through fog, as it would place its army leading as one: half of it holds the
    /// line, each group's sector by the enemy it sees across it (a fifth evenly); with the other
    /// half it looks for blows its war games promise, the best first, the smallest group that
    /// gives the odds — each blow goes to the group of the sector it is struck from, which is told
    /// to attack towards its best target with that much more; what is left is its reserve. A group
    /// that will not strike and faces an enemy outweighing its line is told to hold deep. The
    /// blows found (group, from, target, share of the striking half, odds) go to the groups as they
    /// are.
    pub(crate) fn staff_orders(&mut self, pack: &Pack, s: usize, pool: &[f32], seen: &[f32], supply: &[Vec<f32>; 2], reach: &[Vec<u32>; 2], seed: u64) -> Vec<(usize, usize, usize, f32, f32)> {
        let parts = self.sectors_of(s);
        let k = parts.len();
        let mut found = Vec::new();
        if k == 0 {
            return found;
        }
        let o = 1 - s;
        // the groups the player took: the staff does not strike from their sectors, and leads only
        // what they leave of the army
        let mine: Vec<bool> = self.command[s].as_ref().map_or(vec![true; k], |gs| gs.iter().map(|g| g.staff).collect());
        let taken: f32 = self.command[s].as_ref().map_or(0.0, |gs| gs.iter().filter(|g| !g.staff).map(|g| g.share).sum::<f32>()).min(1.0);
        let scale = 1.0 - taken;
        let mut lab = vec![usize::MAX; self.owner.len()];
        for (g, part) in parts.iter().enumerate() {
            if mine[g] {
                for &p in part {
                    lab[p] = g;
                }
            }
        }
        // the blows
        let strike: Vec<f32> = pool.iter().map(|x| x * (1.0 - HOLD)).collect();
        let mut scored: Vec<(usize, usize, f32)> = Vec::new();
        for q in 0..self.owner.len() {
            if self.owner[q] != o {
                continue;
            }
            let Some(from) = self.th.nbr[q].iter().copied().filter(|&r| self.owner[r] == s && reach[s][r] != NO && lab[r] != usize::MAX).max_by(|&a, &b| supply[s][a].total_cmp(&supply[s][b])) else {
                continue;
            };
            scored.push((q, from, self.odds(pack, s, q, &strike, supply[s][from], seen[q], supply[o][q], seed ^ q as u64)));
        }
        scored.sort_by(|a, b| b.2.total_cmp(&a.2));
        let mut left = 1.0f32;
        let mut blows = vec![0.0f32; k];
        let mut head: Vec<Option<usize>> = vec![None; k];
        for &(q, from, best) in &scored {
            if best < ODDS || left <= 0.01 {
                break;
            }
            for &f in SPLITS.iter().rev() {
                if f > left + 1e-6 {
                    continue;
                }
                let g: Vec<f32> = strike.iter().map(|x| x * f).collect();
                let got = self.odds(pack, s, q, &g, supply[s][from], seen[q], supply[o][q], seed ^ q as u64 ^ 7);
                if got >= ODDS {
                    left -= f;
                    blows[lab[from]] += f;
                    head[lab[from]].get_or_insert(q);
                    found.push((lab[from], from, q, f * scale, got));
                    break;
                }
            }
        }
        // the line
        let threat: Vec<f32> = parts.iter().map(|part| part.iter().map(|&p| self.threat(s, p, seen)).sum()).collect();
        let all: f32 = threat.iter().sum();
        let even = 1.0 / k as f32;
        let worth = Campaign::worth(pack, pool);
        if let Some(groups) = self.command[s].as_mut() {
            for (g, gr) in groups.iter_mut().enumerate() {
                if !gr.staff {
                    continue;
                }
                let line = scale * HOLD * (0.2 * even + 0.8 * if all > 0.0 { threat[g] / all } else { even });
                let blow = scale * (1.0 - HOLD) * blows[g];
                gr.share = line + blow;
                gr.strike = blow / gr.share.max(1e-9);
                gr.order = match head[g] {
                    Some(q) => Order::Attack(q),
                    None if threat[g] > DEEP_AT * worth * line => Order::Deep,
                    None => Order::Hold,
                };
            }
        }
        found
    }

    /// Side `s`'s groups take their places for next month from `pool` (all it can move): each its
    /// share, on its sector, as told; what no group takes stays back as the staff's reserve. A
    /// group told to attack looks for its blows itself — its general does — towards the arrow's
    /// head (told by the staff: towards the targets its war games found, `planned`: group, from,
    /// target, share of the army's striking half, odds), judging the enemy with his own eyes
    /// (`truth`: the worth of the enemy's troops in each province, which he sees with his own
    /// error; without a general, what the staff sees, `seen`).
    pub(super) fn place_groups(&mut self, pack: &Pack, s: usize, pool: Vec<f32>, seen: &[f32], truth: &[f32], supply: &[Vec<f32>; 2], reach: &[Vec<u32>; 2], seed: u64, planned: &[(usize, usize, usize, f32, f32)]) {
        let o = 1 - s;
        let n = self.owner.len();
        let parts = self.sectors_of(s);
        let groups = self.command[s].clone().unwrap_or_default();
        let mut reserve = pool.clone();
        self.deep[s] = vec![false; n];
        for (g, group) in groups.iter().enumerate() {
            let line: Vec<usize> = parts[g].iter().copied().filter(|&p| reach[s][p] != NO).collect();
            if line.is_empty() {
                continue;
            }
            let mine: Vec<f32> = pool.iter().map(|x| x * group.share).collect();
            add(&mut reserve, &mine, -1.0);
            // just behind the line: own provinces next to it that touch no enemy
            let mut behind: Vec<usize> = line
                .iter()
                .flat_map(|&p| self.th.nbr[p].iter().copied())
                .filter(|&r| self.owner[r] == s && reach[s][r] != NO && !self.th.nbr[r].iter().any(|&x| self.owner[x] != s))
                .collect();
            behind.sort_unstable();
            behind.dedup();
            if behind.is_empty() {
                behind = line.clone();
            }
            let threat: Vec<f32> = line.iter().map(|&p| self.threat(s, p, seen)).collect();
            let all: f32 = threat.iter().sum();
            
            // Идеальное решение: сглаживаем веса, чтобы избежать микро-суеты
            let weights: Vec<f32> = (0..line.len()).map(|i| {
                if all > 0.0 {
                    let raw_weight = 0.8 * threat[i] / all + 0.2 / line.len() as f32;
                    // Округляем до ближайших 0.2 (20%), чтобы убрать микро-колебания
                    (raw_weight * 5.0).round() / 5.0
                } else {
                    1.0 / line.len() as f32
                }
            }).collect();
            
            // Нормализуем веса после округления, чтобы сумма была строго 1.0
            let w_sum: f32 = weights.iter().sum::<f32>().max(1e-9);
            let weights: Vec<f32> = weights.iter().map(|w| w / w_sum).collect();

            
            // Нормализуем веса после округления, чтобы сумма была строго 1.0
            let w_sum: f32 = weights.iter().sum::<f32>().max(1e-9);
            let weights: Vec<f32> = weights.iter().map(|w| w / w_sum).collect();

            // told to attack: the share of the group that strikes (a bold general throws in more)
            let striking = match group.order {
                Order::Attack(_) => (group.strike * group.general.as_ref().map_or(1.0, |x| 1.0 + BOLD_PUSH * x.bold)).clamp(0.0, 0.9),
                _ => 0.0,
            };
            let first = match group.order {
                Order::Hold => LINE_HOLD,
                Order::Deep => LINE_DEEP,
                Order::Attack(_) => 1.0 - striking,
            };
            for (i, &p) in line.iter().enumerate() {
                add(&mut self.troops[s][p], &mine, first * weights[i]);
            }
            match group.order {
                Order::Attack(head) => {
                    // blows at the enemy provinces next to the sector nearest the arrow's head (led
                    // by the staff: nearest the targets its war games found), each the smallest
                    // group that gives the general the odds he wants, as he judges the enemy there;
                    // what is not used keeps the line
                    let strike: Vec<f32> = mine.iter().map(|x| x * striking).collect();
                    let mut heads: Vec<usize> = planned.iter().filter(|b| b.0 == g).map(|b| b.2).collect();
                    if heads.is_empty() {
                        heads.push(head);
                    }
                    let d = self.distance_from_all(&heads);
                    let mut targets: Vec<(usize, usize)> = Vec::new();
                    for &p in &line {
                        for &q in &self.th.nbr[p] {
                            if self.owner[q] == o && !targets.iter().any(|t| t.0 == q) {
                                let from = self.th.nbr[q].iter().copied().filter(|r| line.contains(r)).max_by(|&a, &b| supply[s][a].total_cmp(&supply[s][b]).then(b.cmp(&a))).unwrap_or(p);
                                targets.push((q, from));
                            }
                        }
                    }
                    targets.sort_by_key(|&(q, _)| (d[q], q));
                    let nearest = targets.first().map_or(NO, |t| d[t.0]);
                    let mut left = 1.0f32;
                    // the general: the odds he wants (a staff's order asks for more than the
                    // player's), how truly he sees, how finely he weighs
                    let general = group.general.clone();
                    let base = if group.staff { ODDS } else { ODDS_ORDERED };
                    let wants = general.as_ref().map_or(base, |x| x.wants(base));
                    let rough = general.as_ref().is_some_and(|x| x.education < 0.4);
                    for &(q, from) in &targets {
                        if d[q] > nearest.saturating_add(AROUND) || left <= 0.01 {
                            break;
                        }
                        let judged = match &general {
                            Some(x) => truth[q] * (x.blur() * Rng::new(hash64(seed ^ 0x6E7 ^ q as u64)).normal()).exp(),
                            None => seen[q],
                        };
                        for (i, &f) in SPLITS.iter().rev().enumerate() {
                            if f > left + 1e-6 || (rough && i % 2 == 1) {
                                continue;
                            }
                            let part: Vec<f32> = strike.iter().map(|x| x * f).collect();
                            let got = self.odds(pack, s, q, &part, supply[s][from], judged, supply[o][q], seed ^ q as u64 ^ 0xA77);
                            if got >= wants {
                                left -= f;
                                add(&mut self.troops[s][from], &part, 1.0);
                                if let Some(x) = &general {
                                    self.hidden[s][from] += Campaign::worth(pack, &part) * HIDE * x.talent;
                                }
                                self.staged.push((s, from, q, part, got));
                                self.staged_def.push(truth[q]);
                                break;
                            }
                        }
                    }
                    for (i, &p) in line.iter().enumerate() {
                        add(&mut self.troops[s][p], &strike, left.max(0.0) * weights[i]);
                    }
                }
                order => {
                    for &p in &behind {
                        add(&mut self.troops[s][p], &mine, (1.0 - first) / behind.len() as f32);
                    }
                    if order == Order::Deep {
                        for &p in line.iter().chain(&behind) {
                            self.deep[s][p] = true;
                        }
                    }
                }
            }
        }
        self.reserve[s] = reserve.iter().map(|x| x.max(0.0)).collect();
    }
}
