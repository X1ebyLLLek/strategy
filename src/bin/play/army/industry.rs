//! The arms works and the conversion of civil works to arms.
//!
//! What the army buys, the arms works make (the pack's sectors that make military goods: arsenals,
//! gunsmiths, workshops) out of what the market sells them — steel, powder, scrap. In peace they are
//! few; a war asks more of them than they can make, prices run up, owners build more of them (it pays)
//! — slowly. A state at war does more: it turns civil works to arms (machine works make guns, car
//! works tanks, as in life), a share of their workplaces a month, as far as its order of the economy
//! lets it (a free market little, a total mobilisation much); a converted workplace makes a part of
//! what a real arms works would (`RETOOL`: new jigs, new trades), and what it made before is missed
//! in the shops. In peace the works go back to what they made.

use crate::country::policy::kind_of;
use crate::Game;

/// The share of the civil industry's workplaces a state converts to arms, by its order of the
/// economy (`policy::LAWS`): at war, and in peace.
pub const CONVERT_WAR: [f32; 4] = [0.02, 0.08, 0.18, 0.32];
pub const CONVERT_PEACE: [f32; 4] = [0.0, 0.0, 0.04, 0.12];
/// The share of the way to it a month (a year to convert, a year to come back).
const PACE: f32 = 1.0 / 12.0;
/// What a converted workplace makes against a real arms works'.
pub const RETOOL: f32 = 0.6;

/// The arms works of a side: workplaces, against those of before the war; what they made last month
/// (goods by the pack's prices); what the army bought abroad.
#[derive(Clone, Copy, Default, Debug)]
pub struct Works {
    pub jobs: f32,
    pub converted: f32,
    pub share: f32,
    pub made: f32,
    pub bought_abroad: f32,
}

impl Game {
    /// The sectors of the pack that make military goods, and those of the civil industry that can be
    /// converted to them (works that are built, not bound to a deposit, not the land's).
    fn arms_sectors(&self) -> (Vec<usize>, Vec<usize>) {
        let pack = &self.pack;
        let mut arms = Vec::new();
        let mut civil = Vec::new();
        for k in 0..pack.sectors.len() {
            let (land, military, industry, civ) = kind_of(pack, k);
            if military {
                arms.push(k);
            } else if !land && (industry || civ) && !pack.sectors[k].build.is_empty() {
                civil.push(k);
            }
        }
        (arms, civil)
    }

    /// A month of conversion: each side's converted share of the civil industry moves a step towards
    /// what its war and its order of the economy ask; the workplaces go over (or back).
    pub fn convert_month(&mut self) {
        let (arms, civil) = self.arms_sectors();
        if arms.is_empty() || civil.is_empty() {
            return;
        }
        for s in 0..2 {
            let law = self.policy.law[s].min(3);
            let want = if self.h.at_war { CONVERT_WAR[law] } else { CONVERT_PEACE[law] };
            let was = self.mil.convert[s];
            let step = PACE * want.max(was).max(0.01);
            let now = was + (want - was).clamp(-step, step);
            if (now - was).abs() < 1e-5 {
                continue;
            }
            let n = self.h.home_mut(s);
            let finite = |k: usize, n: &strategy::economy::nation::Nation| n.max_cap.get(k).is_some_and(|m| m.is_finite());
            if now > was {
                // more converted: a part of what each civil works still has
                let f = (now - was) / (1.0 - was).max(1e-6);
                let mut moved = 0.0f32;
                for &k in &civil {
                    if finite(k, n) {
                        continue;
                    }
                    let x = n.cap[k] * f;
                    n.cap[k] -= x;
                    moved += x;
                    if self.mil.converted_from[s].len() <= k {
                        self.mil.converted_from[s].resize(k + 1, 0.0);
                    }
                    self.mil.converted_from[s][k] += x;
                }
                let all: f32 = arms.iter().map(|&k| n.cap[k]).sum::<f32>().max(1e-9);
                for &k in &arms {
                    n.cap[k] += moved * RETOOL * n.cap[k] / all;
                }
                self.mil.converted_to[s] += moved * RETOOL;
            } else {
                // back to what they made: a part of what was converted
                let f = ((was - now) / was.max(1e-6)).min(1.0);
                let back_arms = self.mil.converted_to[s] * f;
                let all: f32 = arms.iter().map(|&k| n.cap[k]).sum::<f32>().max(1e-9);
                for &k in &arms {
                    n.cap[k] = (n.cap[k] - back_arms * n.cap[k] / all).max(0.0);
                }
                self.mil.converted_to[s] -= back_arms;
                for &k in &civil {
                    if let Some(x) = self.mil.converted_from[s].get_mut(k) {
                        let r = *x * f;
                        *x -= r;
                        n.cap[k] += r;
                    }
                }
            }
            self.mil.convert[s] = now;
        }
    }

    /// Side `s`'s arms works now.
    pub fn works(&self, s: usize) -> Works {
        let (arms, _) = self.arms_sectors();
        let pack = &self.pack;
        let n = self.h.home(s);
        let jobs: f32 = arms.iter().map(|&k| n.cap.get(k).copied().unwrap_or(0.0)).sum();
        let made: f32 = (0..pack.goods.len()).filter(|&i| pack.goods[i].military).map(|i| n.produced.get(i).copied().unwrap_or(0.0) * pack.goods[i].price).sum();
        let bought_abroad: f32 = (0..pack.goods.len()).filter(|&i| pack.goods[i].military).map(|i| n.import.get(i).copied().unwrap_or(0.0) * pack.goods[i].price).sum();
        Works { jobs, converted: self.mil.converted_to[s], share: self.mil.convert[s], made, bought_abroad }
    }
}
