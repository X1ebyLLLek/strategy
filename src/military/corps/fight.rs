//! The fights of divisions: who fits the width, the harm done by a day of battle, who leaves
//! it (a defender falls back or is lost), the province taken; each fight's record.

use super::*;

impl Campaign {
    /// A day of each fight.
    pub(super) fn fight(&mut self, pack: &Pack, c: &mut Corps, rng: &mut Rng, reach: &[Vec<u32>; 2], fighting: &mut [bool]) {
        let war = pack.war.as_ref().unwrap();
        let units = &pack.units;
        let (nu, nc) = (units.len(), war.classes.len());
        let mut done: Vec<(usize, bool)> = Vec::new();
        // (a division attacked where it stands defends there and attacks nowhere else)
        let attacked: Vec<(usize, usize)> = c.fights.iter().map(|f| (f.at, 1 - f.side)).collect();
        // (how many divisions each general leads)
        let size: [Vec<usize>; 2] = std::array::from_fn(|s| {
            let k = self.command[s].as_ref().map_or(0, |g| g.len());
            let mut v = vec![0usize; k];
            for d in c.divs.iter().filter(|d| d.side == s && d.group < k) {
                v[d.group] += 1;
            }
            v
        });
        for fi in 0..c.fights.len() {
            let q = c.fights[fi].at;
            let a = c.fights[fi].side;
            let o = 1 - a;
            let ti = self.th.terrain[q] as usize;
            let atts: Vec<usize> = (0..c.divs.len())
                .filter(|&i| {
                    let d = &c.divs[i];
                    d.side == a && d.rest <= 0.0 && d.org > ORG_OUT && d.way.first() == Some(&q) && self.th.nbr[d.at].contains(&q) && !attacked.contains(&(d.at, a))
                })
                .collect();
            let defs: Vec<usize> = (0..c.divs.len()).filter(|&i| c.divs[i].side == o && c.divs[i].at == q && c.divs[i].rest <= 0.0).collect();
            if self.owner[q] == a || defs.is_empty() || atts.is_empty() {
                if atts.is_empty() && !defs.is_empty() {
                    // (nobody fit to attack is left: those still told to stop)
                    for d in c.divs.iter_mut().filter(|d| d.side == a && d.way.first() == Some(&q)) {
                        d.way.clear();
                        d.gone = 0.0;
                    }
                }
                done.push((fi, defs.is_empty()));
                continue;
            }
            let sides = [atts, defs];
            for list in &sides {
                for &i in list {
                    fighting[i] = true;
                }
            }
            // each division as it is today: what of it is fit to fight, who leads it
            let lead_of = |d: &Division| -> f32 { self.command[d.side].as_ref().and_then(|gs| gs.get(d.group)).and_then(|g| g.general.as_ref()).map_or(1.0, |x| x.lead_of(size[d.side].get(d.group).copied().unwrap_or(0))) };
            let mut fr: [Vec<Vec<f32>>; 2] = std::array::from_fn(|x| sides[x].iter().map(|&i| c.divs[i].has.iter().map(|v| v * c.divs[i].org).collect()).collect());
            let mut has: [Vec<Vec<f32>>; 2] = std::array::from_fn(|x| sides[x].iter().map(|&i| c.divs[i].has.clone()).collect());
            // (each division's kit of each kind, and what its companies give it)
            let mut kit: [Vec<Vec<f32>>; 2] = std::array::from_fn(|x| sides[x].iter().map(|&i| c.divs[i].kit.clone()).collect());
            let qk: [Vec<Vec<f32>>; 2] = std::array::from_fn(|x| sides[x].iter().map(|&i| (0..nu).map(|u| c.divs[i].q(u)).collect()).collect());
            let gift: [Vec<Gift>; 2] = std::array::from_fn(|x| sides[x].iter().map(|&i| c.gift(pack, &c.divs[i])).collect());
            let mut lost_men: [Vec<f32>; 2] = std::array::from_fn(|x| vec![0.0; sides[x].len()]);
            let lead: [Vec<f32>; 2] = std::array::from_fn(|x| sides[x].iter().map(|&i| lead_of(&c.divs[i])).collect());
            let dug: Vec<f32> = sides[1].iter().map(|&i| c.divs[i].dug).collect();
            let exp: [Vec<f32>; 2] = std::array::from_fn(|x| sides[x].iter().map(|&i| c.divs[i].exp).collect());
            let trained: [Vec<f32>; 2] = std::array::from_fn(|x| sides[x].iter().map(|&i| c.divs[i].trained()).collect());
            let fed: [Vec<f32>; 2] = std::array::from_fn(|x| sides[x].iter().map(|&i| c.divs[i].supplied.min(1.0)).collect());
            let pace = |x: usize| -> f32 { sides[x].iter().map(|&i| Corps::speed(pack, &c.divs[i].has)).sum::<f32>() / sides[x].len().max(1) as f32 };
            let fast = pace(0) / pace(1).max(1e-6);
            // the width: more when the attack comes from more sides
            let mut froms: Vec<usize> = sides[0].iter().map(|&i| c.divs[i].at).collect();
            froms.sort_unstable();
            froms.dedup();
            let width = war.width * war.frontage[ti] * (1.0 + EXTRA_SIDE * (froms.len() as f32 - 1.0)).min(WIDEST);
            // what holds each side's line: its front kinds, or its second line if it has no front
            let holder: [Vec<bool>; 2] = std::array::from_fn(|x| {
                let has_front = has[x].iter().any(|v| (0..nu).any(|u| units[u].line == Line::Front && v[u] > 0.0));
                (0..nu).map(|u| units[u].line == if has_front { Line::Front } else { Line::Support }).collect()
            });
            let line = |x: usize, fr: &[Vec<Vec<f32>>; 2]| -> f32 { fr[x].iter().map(|v| (0..nu).filter(|&u| holder[x][u]).map(|u| v[u] * units[u].width).sum::<f32>()).sum() };
            let mut f = c.fights[fi].clone();
            if f.days == 0 {
                for x in 0..2 {
                    let mut k = vec![0.0f32; nu];
                    for v in &has[x] {
                        for u in 0..nu {
                            k[u] += v[u];
                        }
                    }
                    f.start[x] = Corps::worth(pack, &k);
                    f.kinds[x] = k;
                    f.lead[x] = lead[x].iter().sum::<f32>() / lead[x].len() as f32;
                    f.supply[x] = fed[x].iter().sum::<f32>() / fed[x].len() as f32;
                    let k = sides[x].len().max(1) as f32;
                    f.kit[x] = sides[x].iter().map(|&i| c.equipped(pack, &c.divs[i])).sum::<f32>() / k;
                    f.exp[x] = exp[x].iter().sum::<f32>() / k;
                    f.companies[x] = sides[x].iter().map(|&i| c.shapes[c.divs[i].side].get(c.divs[i].shape).map_or(0, |sh| sh.support.len()) as f32).sum::<f32>() / k;
                }
                f.dug = dug.iter().sum::<f32>() / dug.len() as f32;
            }
            for x in 0..2 {
                f.divs[x] = f.divs[x].max(sides[x].len() as u32);
            }
            f.sides = f.sides.max(froms.len() as u32);
            for x in 0..2 {
                f.held[x] = f.held[x].max(line(x, &fr).min(width));
            }
            f.days += 1;
            if f.lost_kinds[0].len() != nu {
                f.lost_kinds = [vec![0.0; nu], vec![0.0; nu]];
            }
            let real = [&self.sides[a], &self.sides[o]];
            // what each side's country brings (the attacker's fire on the attack, the defender's on
            // the defence); whether any kind wears armour (else fire pierces all, as before)
            let mods = [c.mods[a], c.mods[o]];
            let armoured = units.iter().any(|u| u.armor > 0.0);
            // the generals' tactics: chosen anew every few days by what each side has and the
            // ground; an able general reads the other's last one and answers it
            if f.days % TACTIC_DAYS == 1 {
                let has_guns = |x: usize| has[x].iter().any(|v| (0..nu).any(|u| units[u].line == Line::Support && v[u] > 0.0));
                let has_armour = |x: usize| has[x].iter().any(|v| (0..nu).any(|u| units[u].armor > 0.0 && v[u] > 0.0));
                let rough_ground = rough(self.th.terrain[q]);
                for x in 0..2 {
                    let ok = |t: &Tactic| {
                        t.side as usize == x
                            && match t.needs {
                                NEED_SIDES => froms.len() >= 2,
                                NEED_GUNS => has_guns(x),
                                NEED_ARMOUR => has_armour(x),
                                NEED_ROUGH => rough_ground,
                                _ => true,
                            }
                    };
                    let can: Vec<u8> = (0..TACTICS.len() as u8).filter(|&t| ok(&TACTICS[t as usize])).collect();
                    if can.is_empty() {
                        continue;
                    }
                    let theirs = f.tactic[1 - x];
                    let able = lead[x].iter().sum::<f32>() / lead[x].len().max(1) as f32;
                    let answer: Vec<u8> = can.iter().copied().filter(|&t| TACTICS[t as usize].counters.contains(&theirs)).collect();
                    let read = (0.25 + 1.5 * (able - 1.0)).clamp(0.1, 0.7);
                    f.tactic[x] = if !answer.is_empty() && rng.f32() < read { answer[(rng.f32() * answer.len() as f32) as usize % answer.len()] } else { can[(rng.f32() * can.len() as f32) as usize % can.len()] };
                }
            }
            for x in 0..2 {
                let t = f.tactic[x] as usize;
                if t < 8 {
                    f.tactics[x][t] += 1;
                }
            }
            // (a tactic countered does nothing; the one that counters it does more)
            let countered = |x: usize| TACTICS[f.tactic[x] as usize].counters.contains(&f.tactic[1 - x]);
            let tac: [&Tactic; 2] = [&TACTICS[f.tactic[0] as usize], &TACTICS[f.tactic[1] as usize]];
            let beaten = [countered(1), countered(0)];
            let tac_fire = |x: usize, u: usize| -> f32 {
                if beaten[x] {
                    return 1.0;
                }
                let t = tac[x];
                let mut k = t.fire;
                if units[u].line == Line::Support {
                    k *= t.support;
                }
                if units[u].armor > 0.0 {
                    k *= t.armour;
                }
                if countered(x) {
                    k *= 1.1;
                }
                k
            };
            let tac_harm = |x: usize| -> f32 { if beaten[x] { 1.08 } else { tac[x].harm } };
            // who fights and how much fire each draws
            let mut fu: [Vec<Vec<f32>>; 2] = [Vec::new(), Vec::new()];
            let mut draws: [Vec<Vec<f32>>; 2] = [Vec::new(), Vec::new()];
            let mut seen = [vec![0.0f32; nc], vec![0.0f32; nc]];
            for x in 0..2 {
                let room = (width / line(x, &fr).max(1e-9)).min(1.0);
                for v in &fr[x] {
                    let mut a_ = vec![0.0f32; nu];
                    let mut d_ = vec![0.0f32; nu];
                    for u in 0..nu {
                        let (k, w) = if holder[x][u] {
                            (room, 1.0)
                        } else {
                            match units[u].line {
                                Line::Air => (1.0, 1.0),
                                _ => (1.0, battle::BEHIND),
                            }
                        };
                        a_[u] = v[u] * k;
                        d_[u] = a_[u] * w;
                        seen[x][units[u].class] += d_[u];
                    }
                    fu[x].push(a_);
                    draws[x].push(d_);
                }
            }
            // the fire of each side on each class of the other's; of it, what pierces the armour
            // of each kind of the other's
            let mut harm = [vec![0.0f32; nc], vec![0.0f32; nc]];
            let mut pierced = [vec![0.0f32; nu], vec![0.0f32; nu]];
            for x in 0..2 {
                let y = 1 - x;
                let day_luck = (battle::DAY_LUCK * rng.normal()).exp();
                for j in 0..fu[x].len() {
                    let fed_k = battle::UNSUPPLIED + (1.0 - battle::UNSUPPLIED) * fed[x][j];
                    for u in 0..nu {
                        let n_ = fu[x][j][u];
                        if n_ <= 0.0 {
                            continue;
                        }
                        let unit = &units[u];
                        let aim: Vec<f32> = (0..nc).map(|cl| seen[y][cl] * unit.fire[cl]).collect();
                        let all: f32 = aim.iter().sum();
                        if all <= 0.0 {
                            continue;
                        }
                        let fd = if unit.supply.is_empty() { 1.0 } else { fed_k };
                        let k = n_ * real[x].truth_fire[u] * lead[x][j] * (1.0 + EXP_FIRE * exp[x][j]) * trained[x][j] * unit.terrain[ti] * fd * day_luck * f.luck[x] * norm::fire(qk[x][j][u]) * gift[x][j].fire;
                        // (breakthrough and defence: a kind made to strike, or to hold)
                        let role = if x == 0 { unit.attack * mods[0].attack } else { unit.defend * mods[1].defend } * tac_fire(x, u);
                        let k = if role == 1.0 { k } else { k * role };
                        for cl in 0..nc {
                            let h = k * unit.fire[cl] * aim[cl] / all;
                            harm[y][cl] += h;
                            if armoured {
                                for v in 0..nu {
                                    if units[v].class == cl && unit.pierce >= units[v].armor {
                                        pierced[y][v] += h;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // the harm spread over each side's divisions by the fire they draw: the defender's
            // ground and trenches keep part of it off; it kills some and wears out more
            for x in 0..2 {
                for j in 0..fu[x].len() {
                    let mut dead_w = 0.0;
                    for u in 0..nu {
                        let cl = units[u].class;
                        let dr = draws[x][j][u];
                        if dr <= 0.0 || seen[x][cl] <= 0.0 {
                            continue;
                        }
                        let mut h = harm[x][cl] * dr / seen[x][cl];
                        // (fire that does not pierce its armour glances off, most of it)
                        if armoured && units[u].armor > 0.0 && harm[x][cl] > 0.0 {
                            let ok = (pierced[x][u] / harm[x][cl]).clamp(0.0, 1.0);
                            h *= ok + (1.0 - ok) * BOUNCE;
                        }
                        let side_harm = mods[x].harm * tac_harm(x);
                        if side_harm != 1.0 {
                            h *= side_harm;
                        }
                        if x == 1 {
                            let cov = units[u].cover.clamp(0.0, 1.0);
                            h *= 1.0 - war.cover[ti] * cov;
                            h *= 1.0 - battle::DIG * dug[j].clamp(0.0, 1.0) * cov;
                        }
                        let hit = h / (units[u].toughness * real[x].truth_tough[u] * lead[x][j] * norm::tough(qk[x][j][u]));
                        let v = fr[x][j][u];
                        let dead = (hit * battle::KILL).min(v);
                        let tired = (hit * battle::WEAR).min(v - dead);
                        fr[x][j][u] = v - dead - tired;
                        has[x][j][u] = (has[x][j][u] - dead).max(0.0);
                        // (the dead take their equipment; the wounded a hospital saves go back to
                        // the depot, without it)
                        kit[x][j][u] = (kit[x][j][u] - dead * qk[x][j][u]).max(0.0);
                        let side = if x == 0 { a } else { o };
                        c.pool[side][u] += dead * gift[x][j].saved;
                        c.tally[side][T_DEAD] += dead * (1.0 - gift[x][j].saved);
                        lost_men[x][j] += dead * (1.0 - gift[x][j].saved);
                        f.lost_kinds[x][u] += dead * (1.0 - gift[x][j].saved);
                        if c.lost[side].len() == nu {
                            c.lost[side][u] += dead * (1.0 - gift[x][j].saved);
                            c.lost_kit[side][u] += dead * qk[x][j][u];
                        }
                        f.men[x] += dead * (1.0 - gift[x][j].saved) * units[u].men * war.crew;
                        dead_w += dead * units[u].cost;
                    }
                    f.lost[x] += dead_w;
                }
            }
            let fresh = |x: usize| -> f32 { fr[x].iter().map(|v| Corps::worth(pack, v)).sum() };
            f.balance = fresh(0) / (fresh(0) + fresh(1)).max(1e-9);
            // a side gives way when its fresh divisions no longer hold its line
            let broke: Vec<bool> = (0..2).map(|x| line(x, &fr) < battle::HOLD * f.held[x]).collect();
            let over = f.days >= war.days;
            c.fights[fi] = f;
            // the day written into the divisions
            for x in 0..2 {
                for (j, &i) in sides[x].iter().enumerate() {
                    let d = &mut c.divs[i];
                    d.has = has[x][j].clone();
                    d.kit = kit[x][j].clone();
                    d.dead += lost_men[x][j];
                    d.days += 1;
                    let all: f32 = d.has.iter().sum();
                    d.org = if all > 1e-6 { (fr[x][j].iter().sum::<f32>() / all).clamp(0.0, 1.0) } else { 0.0 };
                    let gain = if mods[x].exp_gain == 1.0 { EXP_GAIN } else { EXP_GAIN * mods[x].exp_gain };
                    d.exp += gain * (1.0 - d.exp);
                }
            }
            // a worn-out division leaves the fight: the attacker stops; the defender falls back,
            // losing more the faster the winner is — or, with nowhere to go, is lost
            let mut gone_def = 0;
            // (the pursuers' repairmen take more of what is left behind)
            let take = CAPTURE * (1.0 + gift[0].iter().map(|g| g.salvage).sum::<f32>() / gift[0].len().max(1) as f32);
            for (x, list) in sides.iter().enumerate() {
                for (j, &i) in list.iter().enumerate() {
                    let out = broke[x] || c.divs[i].org < ORG_OUT || (x == 0 && over);
                    if !out {
                        continue;
                    }
                    if x == 0 {
                        if c.divs[i].org < ORG_OUT && !broke[1] {
                            c.tally[a][T_STOP] += 1.0 / list.len() as f32;
                        }
                        c.divs[i].way.clear();
                        c.divs[i].gone = 0.0;
                    } else {
                        let cut = battle::RETREAT + battle::PURSUIT * (fast - 1.0).clamp(0.0, 3.0);
                        c.divs[i].has.iter_mut().for_each(|v| *v *= 1.0 - cut);
                        // its equipment: more of it is left behind than of its men, less with
                        // repairmen; the pursuer takes a share
                        let leave = (cut * ABANDON * (1.0 - gift[1][j].salvage)).min(1.0);
                        gone_def += 1;
                        let lost = !self.fall_back(c, i, q, reach);
                        let leave = if lost { 1.0 } else { leave };
                        for u in 0..nu {
                            let left = c.divs[i].kit[u] * leave;
                            c.divs[i].kit[u] -= left;
                            c.divs[i].left += left;
                            c.abandoned[o][u] += left;
                            c.tally[o][T_LEFT] += left;
                            let got = left * take;
                            c.kit[a][u] += got;
                            c.trophies[a][u] += got;
                            c.fights[fi].captured += got;
                        }
                        if lost {
                            c.divs[i].has.iter_mut().for_each(|v| *v = 0.0);
                            let (side, no) = (c.divs[i].side, c.divs[i].no);
                            c.news.push((c.day, side, format!("{no}-я дивизия окружена и уничтожена")));
                        }
                    }
                }
            }
            if gone_def == sides[1].len() {
                done.push((fi, true));
            } else if sides[0].iter().all(|&i| c.divs[i].way.first() != Some(&q)) {
                done.push((fi, false));
            }
        }
        // fights over: for the record, the generals' books, the staffs' lessons
        done.sort_by_key(|x| std::cmp::Reverse(x.0));
        done.dedup_by_key(|x| x.0);
        for (fi, won) in done {
            let f = c.fights.remove(fi);
            if f.days == 0 {
                continue;
            }
            self.record(pack, c, &f, won, reach, rng);
        }
    }

    /// A fight over: it goes into the record; the generals of the groups that fought it count it;
    /// the staffs learn from it.
    fn record(&mut self, pack: &Pack, c: &mut Corps, f: &Fight, won: bool, reach: &[Vec<u32>; 2], rng: &mut Rng) {
        let (s, o) = (f.side, 1 - f.side);
        c.tally[s][T_ATT] += 1.0;
        if !won {
            c.tally[s][T_FAIL] += 1.0;
        }
        let q = f.at;
        self.fought.push(Fought {
            month: self.month,
            attacker: s,
            province: q,
            from: f.from,
            terrain: self.th.terrain[q],
            ratio: f.start[0] / f.start[1].max(1e-6),
            odds: f.odds,
            won,
            supply: f.supply,
            lost: f.lost,
            cut: reach[o][q] == NO,
            plan: [f.start[0], f.start[1], f.start[0], f.start[1]],
            kinds: f.kinds.clone(),
            lead: f.lead,
            dug: f.dug,
            days: f.days,
            divs: f.divs,
            kit: f.kit,
            exp: f.exp,
            companies: f.companies,
            sides: f.sides,
            men: f.men,
            captured: f.captured,
            lost_kinds: f.lost_kinds.clone(),
            tactics: f.tactics,
        });
        // the generals whose divisions fought it
        for (side, w) in [(s, won), (o, !won)] {
            let mut groups: Vec<usize> = c.divs.iter().filter(|d| d.side == side && (d.at == q || d.way.first() == Some(&q))).map(|d| d.group).filter(|&g| g != NONE).collect();
            groups.sort_unstable();
            groups.dedup();
            if let Some(gs) = self.command[side].as_mut() {
                for g in groups {
                    if let Some(gen) = gs.get_mut(g).and_then(|g| g.general.as_mut()) {
                        if w {
                            gen.won += 1;
                        } else {
                            gen.lost += 1;
                        }
                        gen.experience += crate::military::front::SEASON * (1.0 - gen.experience);
                    }
                }
            }
        }
        let w = &mut self.sides[s].wariness;
        *w = (*w + crate::military::front::WARY * (f.odds - if won { 1.0 } else { 0.0 })).clamp(-1.0, 2.0);
        for side in 0..2 {
            self.sides[side].battles += 1;
        }
        self.sides[if won { s } else { o }].battles_won += 1;
        let army = |side: usize, x: usize| Army { count: f.kinds[x].clone(), supplied: f.supply[x], ..self.sides[side].real(f.kinds[x].clone()) };
        let (att, def) = (army(s, 0), army(o, 1));
        for (me, ours, theirs, w) in [(s, &att, &def, won), (o, &def, &att, !won)] {
            let [a, b] = &mut self.sides;
            let (me, foe) = if me == 0 { (a, &*b) } else { (b, &*a) };
            me.learn(pack, ours, theirs, foe, w, rng);
        }
    }

    /// Division `i` falls back from province `q` to a neighbouring one of its own — one no enemy
    /// attacks, nearest its capital, if it can; false when there is none (it is lost).
    fn fall_back(&self, c: &mut Corps, i: usize, q: usize, reach: &[Vec<u32>; 2]) -> bool {
        let s = c.divs[i].side;
        let under = |r: usize| c.fights.iter().any(|f| f.at == r && f.side != s);
        let back = self.th.nbr[q].iter().copied().filter(|&r| self.owner[r] == s).min_by_key(|&r| (under(r), reach[s][r], r));
        let Some(r) = back else { return false };
        let d = &mut c.divs[i];
        d.at = r;
        d.way.clear();
        d.gone = 0.0;
        d.rest = RETREAT_DAYS;
        d.dug = 0.0;
        true
    }
}
