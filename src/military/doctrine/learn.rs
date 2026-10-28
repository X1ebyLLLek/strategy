//! How a staff learns: from battles (by how much they surprised it), from what it spies of the
//! enemy, from a war won or lost (glory, doubt), from exercises and the years of peace.

use super::*;

impl Country {
    /// What a battle showed: a little of what its units and the enemy's are worth, and the
    /// enemy's army through the fog.
    pub(crate) fn learn(&mut self, pack: &Pack, ours: &Army, theirs: &Army, foe: &Country, won: bool, rng: &mut Rng) {
        let nu = pack.units.len();
        for u in 0..nu {
            if ours.count[u] > 0.05 {
                let sd = SEEN_OWN / (1.0 + ours.count[u]).sqrt();
                self.own_fire[u].see(self.truth_fire[u].ln() + sd * rng.normal(), sd);
                self.own_tough[u].see(self.truth_tough[u].ln() + sd * rng.normal(), sd);
            }
            if theirs.count[u] > 0.05 {
                let sd = SEEN_FOE / (1.0 + theirs.count[u]).sqrt();
                self.foe_fire[u].see(foe.truth_fire[u].ln() + sd * rng.normal(), sd);
                self.foe_tough[u].see(foe.truth_tough[u].ln() + sd * rng.normal(), sd);
                // what the enemy's tanks can do, ours of the same era may do too
                let far = sd * ENEMY_LESSON;
                self.own_fire[u].see(foe.truth_fire[u].ln() + far * rng.normal(), far);
                self.own_tough[u].see(foe.truth_tough[u].ln() + far * rng.normal(), far);
            }
        }
        // glory and blame by how much of the army each kind was
        let worth: Vec<f32> = (0..nu).map(|u| ours.count[u] * self.cost[u]).collect();
        let all: f32 = worth.iter().sum::<f32>().max(1e-9);
        let sign = if won { 1.0 } else { -1.0 };
        for u in 0..nu {
            self.glory[u] = (self.glory[u] + GLORY * sign * (worth[u] / all - 1.0 / nu as f32)).clamp(-GLORY_MAX, GLORY_MAX);
        }
        self.doubt += DOUBT_STEP * (if won { 0.0 } else { 1.0 } - self.doubt);
        self.spy(theirs, self.fog, rng);
    }

    /// A look at the enemy's army through fog `fog`: the worse it is, the less the staff
    /// trusts what it saw.
    pub(crate) fn spy(&mut self, theirs: &Army, fog: f32, rng: &mut Rng) {
        let nu = theirs.count.len();
        let seen: Vec<f32> = (0..nu).map(|u| if theirs.count[u] > 0.05 { theirs.count[u] * (fog * rng.normal()).exp() } else { 0.0 }).collect();
        let all: f32 = seen.iter().sum();
        if all > 0.0 {
            let trust = RECALL * 2.0 * CLEAR * CLEAR / (CLEAR * CLEAR + fog * fog);
            for u in 0..nu {
                self.foe_mix[u] += trust * (seen[u] / all - self.foe_mix[u]);
            }
        }
    }

    /// Its war is over, won or lost: the loser's staff doubts everything it thought it knew, the
    /// winner's is sure of itself, and the kinds of unit it fought with take the glory or the blame.
    pub fn conclude(&mut self, pack: &Pack, won: bool) {
        let nu = pack.units.len();
        self.doubt = if won { 0.0 } else { 1.0 };
        let worth: Vec<f32> = (0..nu).map(|u| self.army.count[u] * self.cost[u]).collect();
        let all: f32 = worth.iter().sum::<f32>().max(1e-9);
        let sign = if won { 1.0 } else { -1.0 };
        for u in 0..nu {
            self.glory[u] = (self.glory[u] + WAR_GLORY * GLORY * sign * (worth[u] / all - 1.0 / nu as f32)).clamp(-GLORY_MAX, GLORY_MAX);
        }
    }

    /// A month of peace: doubt fades, and so does glory.
    pub fn quiet(&mut self) {
        self.doubt *= 1.0 - DOUBT_FADE;
        self.age();
    }

    /// A month passes: doubt loosens what the staff thought it knew, glory fades.
    pub(crate) fn age(&mut self) {
        let loose = DOUBT_NOISE * DOUBT_NOISE * self.doubt;
        for g in self.own_fire.iter_mut().chain(self.own_tough.iter_mut()).chain(self.foe_fire.iter_mut()).chain(self.foe_tough.iter_mut()) {
            g.var += loose;
        }
        for l in &mut self.glory {
            *l *= 1.0 - GLORY_FADE;
        }
    }

    /// A year of peace shows the staff its own equipment at exercises (kinds it has) and in
    /// what designers claim (every kind), both blurred.
    pub(crate) fn exercise(&mut self, rng: &mut Rng) {
        for u in 0..self.plan.len() {
            let claim = CLAIM_MEAN + CLAIM_SD * rng.normal();
            self.own_fire[u].see(self.truth_fire[u].ln() + claim, EXERCISE_SD);
            self.own_tough[u].see(self.truth_tough[u].ln() + claim, EXERCISE_SD);
            if self.army.count[u] > 0.05 {
                self.own_fire[u].see(self.truth_fire[u].ln() + EXERCISE_SD * rng.normal(), EXERCISE_SD);
                self.own_tough[u].see(self.truth_tough[u].ln() + EXERCISE_SD * rng.normal(), EXERCISE_SD);
            }
        }
    }
}
