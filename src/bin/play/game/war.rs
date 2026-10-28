//! The war in the game: peace made and the war ended, a day of the war of divisions, the
//! player's groups of armies in peace, new groups, the army mustered.

use super::*;
use strategy::military::homefront::WAR_TAX;

/// Months a country takes to go fully onto its war footing: factories turned to arms, the war tax
/// gathered, reserves called up — a war's first year is fought mostly with what the peace built.
/// (At once, the army had four times the money from the first month, and its divisions grew from
/// two hundred to four hundred and fifty in a year of war.)
pub const MOBILISE: f32 = 12.0;
/// The army's share of the state's spending on a full war footing, by the order of the economy
/// (`policy::LAWS`: a free market gives the army least, a total mobilisation most).
pub const WAR_SHARE: [f32; 4] = [0.5, 0.6, 0.7, 0.8];

impl Game {
    /// Each side's war footing this month (`Homefront::footing`): from the custom of peace towards
    /// the full footing of its order of the economy, a twelfth of the way more each month of the
    /// war — never below where the country stood (a country arming before the war does not cut its
    /// army's money when the war comes, a high tax is not lowered). Called at the declaration too
    /// (before the war is on: what the country stands at then is where its footing starts). The
    /// army's share and the tax of a side keeping to the footing follow it; the player's own
    /// levers (an army's share he set, a tax he changed) stay his.
    pub fn war_footing(&mut self) {
        let pack = self.pack.clone();
        let k = ((self.h.war.month as f32 + 1.0) / MOBILISE).min(1.0);
        for s in 0..2 {
            // (where a parliament votes the war's credits, it gives as much as the people back the war)
            let c = self.country(s);
            let vote = if self.h.at_war && self.polity.of[c].regime.elected().is_some() { (0.55 + 0.6 * self.war_mood.support[s]).min(1.0) } else { 1.0 };
            let full = (WAR_SHARE[self.policy.law[s].min(WAR_SHARE.len() - 1)] * vote).max(pack.military);
            let was = self.h.footing[s];
            let n = self.h.home(s);
            let from = if self.h.at_war { was } else { (n.mil_share, n.tax / pack.tax.max(1e-6)) };
            let ramp = (pack.military + (full - pack.military) * k, 1.0 + (WAR_TAX - 1.0) * k);
            let now = (ramp.0.max(from.0), ramp.1.max(from.1));
            self.h.footing[s] = now;
            if !self.h.at_war {
                continue;
            }
            if self.h.army_share[s].is_none() {
                self.h.home_mut(s).mil_share = now.0;
            }
            let n = self.h.home_mut(s);
            if (n.tax - pack.tax * was.1).abs() < 1e-6 {
                n.tax = pack.tax * now.1;
            }
        }
    }

    /// The war is over, side `loser` gave up: what each holds of the other's land is its own now;
    /// the generals go home, veterans for the next war; the state asks of the people what it
    /// asks in peace (the player's own levers stay his). The peace in words.
    pub fn peace(&mut self, loser: usize) -> String {
        let now = self.month();
        let before = [self.h.home(0).people(), self.h.home(1).people()];
        self.end_war();
        let winner = 1 - loser;
        let gained = self.h.home(winner).people() - before[winner];
        // (in words: the names stay as they are, in no case of their own; the numbers on hover)
        let share = gained.max(0.0) / before[loser].max(1e-9);
        let what = match share {
            x if x < 0.1 => "земли, где жила малая часть людей страны",
            x if x < 0.3 => "земли, где жила заметная часть людей страны",
            x if x < 0.6 => "земли, где жила большая часть людей страны",
            _ => "земли почти со всеми людьми страны",
        };
        let line = format!("мир: {} отдаёт победителю ({}) {what}", self.h.home(loser).name, self.h.home(winner).name);
        self.news.push(Note::new(now, &line, true, "army").with_tip(format!("ушло {:.1} млн людей из {:.1}", gained.max(0.0), before[loser])));
        self.stamp += 1;
        line
    }

    /// The war is over, by a capitulation or a peace: what each holds of the other's land is its
    /// own now; the generals go home, veterans for the next war; the player's levers come back;
    /// the truce begins.
    pub fn end_war(&mut self) {
        // (who won it, for the features that remember wars)
        self.war_ended(self.h.war.fallen);
        // (what the generals did goes into their lives)
        self.folk_after_war(self.h.war.fallen);
        // (the world remembers it; the allies go home)
        self.pair_war_ends(self.h.war.fallen);
        let me = self.me;
        let now = self.month();
        let foe_name = [self.h.home(1).name.clone(), self.h.home(0).name.clone()];
        for s in 0..2 {
            if let Some(gs) = self.h.war.command[s].as_ref() {
                for g in gs {
                    if let Some(gen) = &g.general {
                        let mut v = gen.clone();
                        v.past.push(format!("воевал против {} в {}-м году: побед {}, поражений {}, взял провинций {}", foe_name[s], now / 12 + 1, gen.won, gen.lost, gen.took));
                        self.veterans[s].push(v);
                    }
                }
            }
        }
        self.h.make_peace(&self.pack);
        // (the divisions stay, back in their garrisons under the staffs)
        self.h.war.staff_led = [true; 2];
        // the player's levers as they were before the war
        if let Some((tax, mil)) = self.prewar.take() {
            let n = self.h.home_mut(me);
            if self.adviser.is_none() {
                n.tax = tax;
            }
            if self.h.army_share[me].is_none() {
                self.h.home_mut(me).mil_share = mil;
            }
        }
        self.border = (self.h.war.front(0).len() + self.h.war.front(1).len()) as f32 / 2.0 * crate::prov_scale();
        self.after_war();
        // (the army in groups along the new border again)
        self.peace_groups();
        self.stamp += 1;
    }

    /// Groups of side `s` facing the enemy whose stretch of the front is empty (their anchors lay on
    /// the border of peace, and the war's front runs elsewhere; or the front moved away from them) are
    /// given the front anew: it is cut into as many stretches as such groups and the groups take them
    /// in order (their generals, orders and plans stay). The player's own groups too: a group with no
    /// front is of no use to him either.
    pub fn reanchor(&mut self, s: usize) {
        let parts = self.h.war.sectors_of(s);
        let Some(gs) = self.h.war.command[s].as_ref() else { return };
        let facing_enemy: Vec<usize> = (0..gs.len()).filter(|&k| gs[k].facing.is_none()).collect();
        if facing_enemy.len() < 2 || !facing_enemy.iter().any(|&k| parts.get(k).is_none_or(|p| p.is_empty())) {
            return;
        }
        let front = self.h.war.front(s).len().max(1);
        let cut = self.h.war.sectors(s, front.div_ceil(facing_enemy.len()).max(1));
        if cut.is_empty() {
            return;
        }
        if let Some(gs) = self.h.war.command[s].as_mut() {
            for (i, &k) in facing_enemy.iter().enumerate() {
                gs[k].anchor = cut[i.min(cut.len() - 1)].clone();
            }
        }
    }

    /// How hard each side's troops fight this month, for what they burn (`Homefront::war_use`): a
    /// quiet front burns little more than exercises, a front in battle all of a month of fighting's
    /// supply, offensives more.
    pub fn war_use_month(&mut self) {
        let Some(c) = self.h.war.corps.as_ref() else { return };
        for s in 0..2 {
            let mine: Vec<&strategy::military::corps::Division> = c.divs.iter().filter(|d| d.side == s).collect();
            if mine.is_empty() {
                continue;
            }
            let busy = mine.iter().filter(|d| d.busy || d.days > 0 && d.org < 0.6).count() as f32 / mine.len() as f32;
            let attacking = c.blows[s].len() as f32 / mine.len().max(1) as f32;
            self.h.war_use[s] = (0.45 + 1.1 * busy + 1.5 * attacking).clamp(0.45, 1.6);
        }
    }

    /// A day of the divisions: of the war, or of the peace (they march to where they are sent and
    /// to their garrisons; they are refilled).
    pub fn war_day(&mut self) {
        if self.h.war.corps.is_some() && self.h.war.fallen.is_none() {
            self.h.war.day(&self.pack);
        }
    }

    /// The templates the player builds his divisions by (none: the staff builds by its plan) and
    /// how much of each.
    pub fn shapes(&self) -> (Vec<strategy::military::corps::Shape>, Vec<f32>) {
        if !self.h.war.sides[self.me].fixed {
            return (Vec::new(), Vec::new());
        }
        self.templates.iter().filter(|t| t.want > 0.0 && t.battalions() > 0).map(|t| (t.shape(), t.want)).unzip()
    }

    /// In peace too the player's army stands in groups of armies along the border with the rival,
    /// a general at the head of each (as in HOI4: the armies, their stretches of the front and
    /// their plans are made before a war): the staff forms them and leads them till the player
    /// takes one; the divisions go to them from the reserve. (The rival's staff keeps its army in
    /// garrisons, as before.)
    pub fn peace_groups(&mut self) {
        let me = self.me;
        if self.h.at_war || self.h.war.corps.is_none() || self.h.war.command[me].is_some() || self.h.war.front(me).is_empty() {
            return;
        }
        self.h.war.by_armies(me, true);
        let mut r = Rng::new(hash64(self.seed ^ 0x9E4C ^ ((self.month() as u64) << 4) ^ self.h.war.th.countries[1 - me] as u64));
        let k = self.h.war.command[me].as_ref().map_or(0, |gs| gs.len());
        let gens: Vec<General> = (0..k).map(|_| self.new_general(me, &mut r)).collect();
        if let Some(gs) = self.h.war.command[me].as_mut() {
            for (g, x) in gs.iter_mut().zip(gens) {
                g.general = Some(x);
            }
        }
        if self.spare.is_empty() {
            let spare: Vec<General> = (0..3).map(|_| self.new_general(me, &mut r)).collect();
            self.spare = spare;
        }
        self.h.war.regroup_side(me);
    }

    /// A new group of armies of the player's divisions `ids` (in a war of divisions): the first
    /// of the spare generals at its head (a new man takes his place among them). Whether formed.
    pub fn raise(&mut self, ids: &[u32]) -> bool {
        let me = self.me;
        let mut r = Rng::new(hash64(self.seed ^ 0x7A15E ^ ((self.month() as u64) << 8) ^ self.day as u64 ^ ((self.journal.len() as u64) << 32)));
        let general = if self.spare.is_empty() { self.new_general(me, &mut r) } else { self.spare.remove(0) };
        let name = general.name.clone();
        let Some(k) = self.h.war.raise(me, ids, Some(general.clone())) else {
            self.spare.insert(0, general);
            return false;
        };
        let more = self.new_general(me, &mut r);
        self.spare.push(more);
        let n = ids.len();
        let line = format!("сформирована {}: {n} {}, во главе {name}", crate::words::group_name(k).to_lowercase(), crate::words::plural(n, "дивизия", "дивизии", "дивизий"));
        self.news.push(Note::new(self.month(), &line, false, "army"));
        true
    }

    /// The war begins with divisions: both armies formed into divisions, each by its own make-up;
    /// the player's new ones by his templates when he builds by them.
    pub fn muster(&mut self) {
        let me = self.me;
        let (shapes, wants) = self.shapes();
        let mut sh = [Vec::new(), Vec::new()];
        let mut wt = [Vec::new(), Vec::new()];
        sh[me] = shapes;
        wt[me] = wants;
        let pack = self.pack.clone();
        self.h.war.muster(&pack, ["Кадровая дивизия", "Кадровая дивизия"], sh, wt);
        // (they stay divisions in peace too)
        if let Some(c) = self.h.war.corps.as_mut() {
            c.keep = true;
            c.peace = !self.h.at_war;
        }
        if !self.h.at_war {
            self.h.war.staff_led = [true; 2];
        }
    }

    /// `DIVDBG=1`: each month, where each side's divisions stand (the biggest stacks, why they
    /// are there) and the staff's pattern — for finding out why a front stands still.
    pub fn dump_divisions(&self) {
        if let Some(c) = self.h.war.corps.as_ref() {
            for s in 0..2 {
                let n = self.h.war.owner.len();
                let mut by = vec![Vec::new(); n];
                for d in c.divs.iter().filter(|d| d.side == s) {
                    by[d.at].push(d);
                }
                let mut qs: Vec<usize> = (0..n).filter(|&q| !by[q].is_empty()).collect();
                qs.sort_by_key(|&q| std::cmp::Reverse(by[q].len()));
                let front = self.h.war.front(s);
                eprintln!("DIVDBG м{} войной {} сторона {s}: дивизий {}, в провинциях {}", self.month(), self.h.at_war, c.divs.iter().filter(|d| d.side == s).count(), qs.len());
                if let Some(sh) = c.shapes[s].last() {
                    let ks: Vec<String> = (0..sh.units.len()).filter(|&u| sh.units[u] > 0.0).map(|u| format!("{}×{} {:.1}", (sh.units[u] / 0.1).round(), self.pack.units[u].id, sh.norm_of(u))).collect();
                    eprintln!("   шаблон «{}»: {} | роты {:?} | нехватки {:?}", sh.name, ks.join(", "), sh.support, c.lack[s]);
                }
                for &q in qs.iter().take(4) {
                    let v = &by[q];
                    let groups: Vec<usize> = v.iter().map(|d| d.group).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
                    eprintln!(
                        "   пров {q}: {} див., фронт {}, столица {}, ручных {}, в пути {}, голодных {}, группы {:?}, прокорм {:.0}%",
                        v.len(),
                        front.contains(&q),
                        q == self.h.war.th.capital[s],
                        v.iter().filter(|d| d.manual).count(),
                        v.iter().filter(|d| !d.way.is_empty()).count(),
                        v.iter().filter(|d| d.supplied < 0.5).count(),
                        groups,
                        100.0 * self.h.war.cap_at(&self.pack, s, q) / v.iter().map(|d| strategy::military::front::Campaign::eats(&self.pack, d)).sum::<f32>().max(1e-6)
                    );
                }
            }
        }
    }
}
