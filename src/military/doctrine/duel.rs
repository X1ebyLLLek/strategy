//! Two countries at war along one front, month by month (the doctrine's own sandbox: the staffs
//! rethink, factories make units, battles are fought by the truth).

use super::*;

/// Two countries at war along one front.
pub struct Duel {
    pub sides: [Country; 2],
    /// The ground of the front: terrains and how much of it each is.
    pub front: Vec<(Terrain, f32)>,
    pub month: u32,
    pub seed: u64,
}

impl Duel {
    /// Both sides start with an army of `months` of their budget, bought by their plan.
    pub fn new(pack: &Pack, sides: [Country; 2], front: Vec<(Terrain, f32)>, seed: u64) -> Duel {
        let mut d = Duel { sides, front, month: 0, seed };
        let worth = Country::battle_worth(pack);
        for s in &mut d.sides {
            s.budget = worth / 6.0;
            for _ in 0..12 {
                s.produce();
            }
        }
        d
    }

    /// A month of war: the staffs rethink, factories make units, and a battle is fought by the
    /// truth, on ground drawn from the front, the sides attacking in turn.
    pub fn step(&mut self, pack: &Pack) {
        self.month += 1;
        let seed = hash64(self.seed ^ self.month as u64);
        let mut rng = Rng::new(seed);
        let front = self.front.clone();
        {
            let [a, b] = &mut self.sides;
            rayon::join(|| a.rethink(pack, &front, seed ^ 1), || b.rethink(pack, &front, seed ^ 2));
        }
        for s in &mut self.sides {
            s.produce();
        }
        let all: f32 = front.iter().map(|f| f.1).sum();
        let mut pick = rng.f32() * all;
        let mut t = front[0].0;
        for &(ft, w) in &front {
            if pick < w {
                t = ft;
                break;
            }
            pick -= w;
        }
        // each side sends so many lines of the front into battle, or all it has
        let worth = Country::battle_worth(pack);
        let slice: Vec<f32> = self.sides.iter().map(|s| (worth / s.army.worth(pack).max(1e-9)).min(1.0)).collect();
        let sent: Vec<Army> = (0..2).map(|i| self.sides[i].real(self.sides[i].army.count.iter().map(|c| c * slice[i]).collect())).collect();
        let first = (self.month % 2) as usize;
        let out = battle(pack, t, [&sent[first], &sent[1 - first]], Kind::Attack, &mut rng, false);
        let back = [&out.left[0], &out.left[1]];
        for (k, i) in [first, 1 - first].into_iter().enumerate() {
            let s = &mut self.sides[i];
            for u in 0..s.army.count.len() {
                s.army.count[u] += back[k].count[u] - sent[i].count[u];
                s.army.count[u] = s.army.count[u].max(0.0);
            }
            s.battles += 1;
            if out.winner == Some(k) {
                s.battles_won += 1;
            }
        }
        let (x, y) = (sent[0].clone(), sent[1].clone());
        let won_a = out.winner == Some(if first == 0 { 0 } else { 1 });
        let [a, b] = &mut self.sides;
        a.learn(pack, &x, &y, b, won_a, &mut rng);
        b.learn(pack, &y, &x, a, !won_a, &mut rng);
        a.age();
        b.age();
    }

    /// The war is over, won by `winner`: the loser's staff doubts everything it thought it knew,
    /// the winner's is sure of itself, and the kinds of unit each fought with take the glory or
    /// the blame.
    pub fn end_war(&mut self, pack: &Pack, winner: usize) {
        for (i, s) in self.sides.iter_mut().enumerate() {
            s.conclude(pack, i == winner);
        }
    }

    /// A month of peace: the staffs rethink each season with what they believe, factories make a little,
    /// old units leave service, doubt and glory fade; once a year, exercises.
    pub fn peace(&mut self, pack: &Pack, budget_share: f32) {
        self.month += 1;
        let seed = hash64(self.seed ^ self.month as u64 ^ 0x9EACE);
        let mut rng = Rng::new(seed);
        let front = self.front.clone();
        // in peace the staff sits once a season
        if self.month % 3 == 0 {
            let [a, b] = &mut self.sides;
            rayon::join(|| a.rethink(pack, &front, seed ^ 1), || b.rethink(pack, &front, seed ^ 2));
        }
        for s in &mut self.sides {
            let full = s.budget;
            s.budget = full * budget_share;
            s.produce();
            s.budget = full;
            for c in &mut s.army.count {
                *c *= 1.0 - RETIRE;
            }
            s.doubt *= 1.0 - DOUBT_FADE;
            s.age();
            if self.month % 12 == 0 {
                s.exercise(&mut rng);
            }
        }
        if self.month % 12 == 0 {
            let (x, y) = (self.sides[1].army.clone(), self.sides[0].army.clone());
            let fogs = [self.sides[0].fog * PEACE_FOG, self.sides[1].fog * PEACE_FOG];
            self.sides[0].spy(&x, fogs[0], &mut rng);
            self.sides[1].spy(&y, fogs[1], &mut rng);
        }
    }
}
