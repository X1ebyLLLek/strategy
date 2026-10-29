//! Doctrines that grow from experience. Nobody picks "blitzkrieg" from a list: each
//! country's staff orders the army it believes wins, believing what it has seen.
//!
//! A country's equipment is better or worse than the era's figures, and nobody knows by how
//! much: designers claim more than they made, and the staff believes them until battles show
//! the truth. The enemy's army is seen through fog. Every month the staff plays war games
//! with what it believes — its own equipment as it rates it, the enemy's army as it sees it,
//! on the ground the front runs over — and moves a tenth of the budget from one kind of
//! unit to another when the games say that wins more often. A doctrine grows step by step
//! from where the country started, so countries end up in different schools. Factories
//! retool slowly. Battles are fought by the truth, and after each the staff learns a little
//! more of what its units and the enemy's are really worth and of what the enemy fields.
//!
//! What pushes countries apart: the ground of their front, what equipment costs them (a
//! country without steel pays dearly for tanks), how good their designs really are, what
//! they wrongly believe, and how well they see the enemy.
//!
//! A country's character is not set anywhere: it grows from its history. The kinds of unit
//! its victories were won with gain glory, those its defeats were lost with take the blame,
//! and glory colours what the staff believes they are worth for years after. Victories make
//! a staff sure of itself, defeats make it doubt: a doubting staff reopens what it thought it
//! knew and takes to reforms more readily. In peace equipment changes and the staff sees it
//! only at exercises and in what designers claim — a staff sure of itself barely listens.
//! (As tanks after the first world war: they had broken down and stuck in the mud, armies
//! that won with horses kept their cavalry, and the second war showed what tanks had become.)

use crate::military::battle::{battle, Army, Kind};
use crate::pack::{Line, Pack};
use crate::rng::{hash64, Rng};
use crate::world::map::Terrain;

mod duel;
mod games;
mod learn;

pub use duel::Duel;

/// Spread of a country's true equipment quality around the era's figures (log).
const QUALITY_SD: f32 = 0.2;
/// Designers claim this much more than they made, give or take (log).
const CLAIM_MEAN: f32 = 0.1;
const CLAIM_SD: f32 = 0.15;
/// What the staff is unsure of at the start (log sd), of its own equipment and the enemy's.
const PRIOR_SD: f32 = 0.3;
/// Noise of what a battle shows of a kind's worth (log sd, for a single unit in it; more
/// units, sharper view), for one's own units and for the enemy's.
const SEEN_OWN: f32 = 0.6;
const SEEN_FOE: f32 = 1.0;
/// Share of the way to a new view of the enemy's army taken after a battle, when the staff
/// sees the enemy as well as staffs usually do (fog `CLEAR`); the worse it sees, the less it
/// trusts each report. (Trusting a muddled report as much as a clear one, thick fog only
/// added noise and the answer to an enemy's new doctrine came no later.)
const RECALL: f32 = 0.3;
const CLEAR: f32 = 0.5;
/// Share of the budget one step of doctrine moves from a kind to another.
const STEP: f32 = 0.1;
/// A new plan replaces the old only if the war games say it wins this much more often.
const BETTER: f32 = 0.02;
/// Bold reforms the staff weighs each month: this share of the budget moved at once from one
/// kind to another, taken only if the games say it wins clearly more often. (Moving a tenth
/// at a time, nobody ever got to a tank corps: a tank or two among infantry changed nothing
/// in the games, and both sides dropped tanks in the first year for good.)
const BOLD: usize = 3;
const BOLD_STEP: f32 = 0.4;
const BOLD_BETTER: f32 = 0.05;
/// War games per candidate plan, per terrain, attacking and defending each.
const GAMES: usize = 4;
/// Share of the way from what factories make to the plan, per month.
const RETOOL: f32 = 0.2;
/// Battles and war games are fought by this many lines of the front on each side.
const LINES: f32 = 4.0;
/// Glory of a kind of unit, per battle: credit for a victory, blame for a defeat, by how much
/// more of the army it was than an even share (a kind the army did without gains glory from
/// a defeat — "we lost for want of it"). It is added to what the staff believes the kind is
/// worth (log), its own and the enemy's, and fades over a generation of officers (half in
/// about eleven years).
const GLORY: f32 = 0.02;
const GLORY_MAX: f32 = 0.4;
const GLORY_FADE: f32 = 0.005;
/// Doubt is the share of recent battles lost (each battle weighs this much of it); at full
/// doubt a staff's every belief loosens by this much a month (log sd), and it needs only a
/// fifth of the usual evidence for a change of plan. It fades in peace (half in six years).
const DOUBT_STEP: f32 = 0.1;
const DOUBT_NOISE: f32 = 0.05;
const DOUBT_FADE: f32 = 0.01;
/// In peace a staff sees its own equipment once a year: at exercises (kinds it has) and in
/// what designers claim (all kinds), each this unsure (log sd).
const EXERCISE_SD: f32 = 0.5;
/// What fighting the enemy's units of a kind shows of one's own of the same kind (the same
/// era's technique, another country's make): this many times more blurred.
const ENEMY_LESSON: f32 = 2.0;
/// Share of the army that leaves service each month of peace.
const RETIRE: f32 = 0.02;
/// In peace the staff sees the neighbour's army once a year (attachés, parades, budgets),
/// through fog this many times thicker than in war. (Seeing nothing, staffs planned for
/// years against the army the enemy had at the end of the last war.)
const PEACE_FOG: f32 = 2.0;
/// A lost war shakes a country as no single battle does: its staff doubts everything; a won
/// war makes it sure of itself. And the glory of the war's kinds of unit, by how much of the
/// winner's army each was, or the blame of them in the loser's (as a battle's, this many times).
const WAR_GLORY: f32 = 10.0;
/// How much of its war games a staff plays against the enemy's likely answer to its plan
/// rather than against the enemy as it is: the enemy moving `BOLD_STEP` of its budget to the
/// kind that hurts the plan most for the money.
const CAUTION: f32 = 0.3;

/// What a staff believes of one figure: the log of its multiplier, and how unsure it is.
#[derive(Clone, Copy, Debug)]
pub struct Guess {
    pub mean: f32,
    pub var: f32,
}

impl Guess {
    fn see(&mut self, obs: f32, sd: f32) {
        let k = self.var / (self.var + sd * sd);
        self.mean += k * (obs - self.mean);
        self.var *= 1.0 - k;
    }
}

#[derive(Clone)]
pub struct Country {
    pub name: String,
    /// What each kind of unit costs this country (its prices of the goods, and its men).
    pub cost: Vec<f32>,
    /// Hidden: what its equipment really is, against the era's figures.
    pub truth_fire: Vec<f32>,
    pub truth_tough: Vec<f32>,
    /// What its staff believes of its own equipment and of the enemy's.
    pub own_fire: Vec<Guess>,
    pub own_tough: Vec<Guess>,
    pub foe_fire: Vec<Guess>,
    pub foe_tough: Vec<Guess>,
    /// The enemy's army as the staff sees it: share of each kind among its units.
    pub foe_mix: Vec<f32>,
    /// How badly it sees the enemy: spread of the log of each kind's count as seen.
    pub fog: f32,
    /// Shares of the budget the staff wants for each kind, and what factories make now.
    pub plan: Vec<f32>,
    pub making: Vec<f32>,
    pub army: Army,
    /// Worth of units made per month.
    pub budget: f32,
    /// A doctrine that never changes (a control).
    pub fixed: bool,
    /// Weight of the enemy's likely answer in the staff's war games.
    pub caution: f32,
    /// What the staff has learned to add to the enemy it sees when it weighs an attack (log):
    /// it grows when attacks fail more often than its war games promised, and shrinks when they
    /// succeed more often (the enemy brings up reserves the staff does not see).
    pub wariness: f32,
    /// What its history made of the staff: glory of each kind (log, added to its worth as the
    /// staff sees it) and doubt (share of recent battles lost).
    pub glory: Vec<f32>,
    pub doubt: f32,
    /// Its designs as they were made, before technique matured or aged (`mature`).
    pub base_fire: Vec<f32>,
    pub base_tough: Vec<f32>,
    pub battles_won: u32,
    pub battles: u32,
    /// Norm of equipment of each kind (`norm::LEVELS`): the share of the standard kit each of its
    /// units carries — what the factories make it with. Empty: standard (the old world, to the
    /// bit). Set by the player's templates, or by the staff when it may choose (`norm_free`).
    pub norm: Vec<f32>,
    pub norm_free: bool,
    /// What a unit of each kind costs the country by its parts: equipment (standard kit), the
    /// supply its kit burns (six months), its men and their rations — so that a lighter or
    /// heavier kit can be priced.
    pub parts: Vec<[f32; 3]>,
    /// How much more (or less) of each kind the government wants its factories to make than the
    /// staff's shares say (the game's priorities of production; empty: the staff's shares).
    pub prio: Vec<f32>,
    /// Kinds of units the country cannot field: it does not know the technique that makes them
    /// (`progress::Know::locked`). Empty: none.
    pub locked: Vec<bool>,
    /// Months of full-intensity fighting the stores of supply are kept stocked for, beyond this month's and the next's
    /// (a war reserve laid up in peace, so that a war does not begin with empty stores; 0 — none, the old way).
    pub reserve: f32,
}

pub mod norm;

impl Country {
    /// The norm of kind `u` (standard when none was set).
    pub fn norm_of(&self, u: usize) -> f32 {
        self.norm.get(u).copied().unwrap_or(1.0)
    }

    /// What a unit of kind `u` with kit `k` costs the country (by its parts; without them, the
    /// cost as it stands).
    pub fn cost_at(&self, u: usize, k: f32) -> f32 {
        match self.parts.get(u) {
            Some(p) => p[0] * k + p[1] * norm::eat(k) + p[2],
            None => self.cost[u],
        }
    }

    /// The costs of its kinds by their norms.
    pub fn price_norms(&mut self) {
        if self.parts.len() == self.cost.len() {
            for u in 0..self.cost.len() {
                self.cost[u] = self.cost_at(u, self.norm_of(u));
            }
        }
    }

    /// A country whose goods cost `prices` (against the pack's), whose men cost `men`, with
    /// equipment drawn around the era's figures by `rng`.
    pub fn new(pack: &Pack, name: &str, prices: &[f32], rng: &mut Rng) -> Country {
        let war = pack.war.as_ref().expect("в наборе нет раздела [war]");
        let nu = pack.units.len();
        let cost: Vec<f32> = pack
            .units
            .iter()
            .map(|u| {
                let worth = |v: &[(usize, f32)]| v.iter().map(|&(g, a)| a * pack.goods[g].price * prices[g]).sum::<f32>();
                worth(&u.equip) + 6.0 * worth(&u.supply) + u.men * war.man_cost
            })
            .collect();
        // (by parts: rations go with the men — they eat by their number, whatever their kit)
        let parts: Vec<[f32; 3]> = pack
            .units
            .iter()
            .map(|u| {
                let worth = |v: &[(usize, f32)], food: bool| v.iter().filter(|x| pack.ration(x.0) == food).map(|&(g, a)| a * pack.goods[g].price * prices[g]).sum::<f32>();
                [worth(&u.equip, false) + worth(&u.equip, true), 6.0 * worth(&u.supply, false), u.men * war.man_cost + 6.0 * worth(&u.supply, true)]
            })
            .collect();
        let truth_fire: Vec<f32> = (0..nu).map(|_| (QUALITY_SD * rng.normal()).exp()).collect();
        let truth_tough: Vec<f32> = (0..nu).map(|_| (QUALITY_SD * rng.normal()).exp()).collect();
        let claim = |t: f32, rng: &mut Rng| Guess { mean: t.ln() + CLAIM_MEAN + CLAIM_SD * rng.normal(), var: PRIOR_SD * PRIOR_SD };
        let own_fire = truth_fire.iter().map(|&t| claim(t, rng)).collect();
        let own_tough = truth_tough.iter().map(|&t| claim(t, rng)).collect();
        let unknown = vec![Guess { mean: 0.0, var: PRIOR_SD * PRIOR_SD }; nu];
        let even = vec![1.0 / nu as f32; nu];
        Country {
            name: name.into(),
            cost,
            truth_fire: truth_fire.clone(),
            truth_tough: truth_tough.clone(),
            own_fire,
            own_tough,
            foe_fire: unknown.clone(),
            foe_tough: unknown,
            foe_mix: even.clone(),
            fog: CLEAR,
            plan: even.clone(),
            making: even,
            army: Army::new(pack),
            budget: 0.0,
            fixed: false,
            caution: CAUTION,
            wariness: 0.0,
            glory: vec![0.0; nu],
            doubt: 0.0,
            base_fire: truth_fire.clone(),
            base_tough: truth_tough.clone(),
            battles_won: 0,
            battles: 0,
            norm: Vec::new(),
            norm_free: false,
            parts,
            prio: Vec::new(),
            locked: Vec::new(),
            reserve: 0.0,
        }
    }

    /// The era's technique of a kind has come to `m` times what it was: its designs are worth
    /// that much more (or less). Nobody is told.
    pub fn mature(&mut self, u: usize, m: f32) {
        self.truth_fire[u] = self.base_fire[u] * m;
        self.truth_tough[u] = self.base_tough[u] * m;
    }

    /// Designers have made a new design of kind `u`: really `gain` better (log, fire and
    /// toughness; it may be worse), and they claim `claim`. The army is refitted with it; the staff
    /// believes the claim, less sure of the kind than it was (`doubt`, added variance).
    pub fn redesign(&mut self, u: usize, gain: (f32, f32), claim: (f32, f32), doubt: f32) {
        self.base_fire[u] *= gain.0.exp();
        self.base_tough[u] *= gain.1.exp();
        self.truth_fire[u] *= gain.0.exp();
        self.truth_tough[u] *= gain.1.exp();
        self.own_fire[u].mean += claim.0;
        self.own_tough[u].mean += claim.1;
        self.own_fire[u].var += doubt;
        self.own_tough[u].var += doubt;
    }

    /// The share of the army's money for new units of kind `u`: the staff's, times the
    /// government's priority for it.
    pub fn make_share(&self, u: usize) -> f32 {
        if self.is_locked(u) {
            return 0.0;
        }
        let m = self.making[u];
        match self.prio.get(u) {
            Some(&p) if p != 1.0 => m * p,
            _ => m,
        }
    }

    /// `make_share` of every kind.
    pub fn shares_made(&self) -> Vec<f32> {
        (0..self.making.len()).map(|u| self.make_share(u)).collect()
    }

    /// What the staff takes a kind to be worth: what it believes, coloured by glory.
    pub(crate) fn rated(&self, g: &[Guess]) -> Vec<f32> {
        g.iter().zip(&self.glory).map(|(g, l)| (g.mean + l).exp()).collect()
    }

    /// Its equipment as it really is.
    pub(crate) fn real(&self, count: Vec<f32>) -> Army {
        Army { count, supplied: 1.0, fire: self.truth_fire.clone(), tough: self.truth_tough.clone(), dug: 0.0 }
    }

    /// Worth of a battle: so many lines of the front of the era's cheapest soldiers.
    pub(crate) fn battle_worth(pack: &Pack) -> f32 {
        let war = pack.war.as_ref().unwrap();
        let cheap = pack.units.iter().filter(|u| u.line == Line::Front).map(|u| u.cost / u.width).fold(f32::INFINITY, f32::min);
        LINES * war.width * cheap
    }

    /// Shares of each kind among the units of its army.
    pub fn mix(&self) -> Vec<f32> {
        let all: f32 = self.army.count.iter().sum::<f32>().max(1e-9);
        self.army.count.iter().map(|c| c / all).collect()
    }

    /// Shares of each kind in the worth of its army (by its own costs).
    pub fn worth_mix(&self) -> Vec<f32> {
        let w: Vec<f32> = self.army.count.iter().zip(&self.cost).map(|(c, k)| c * k).collect();
        let all: f32 = w.iter().sum::<f32>().max(1e-9);
        w.iter().map(|x| x / all).collect()
    }

    /// Factories retool towards the plan and make this month's units.
    pub(crate) fn produce(&mut self) {
        self.retool();
        for u in 0..self.plan.len() {
            self.army.count[u] += self.budget * self.making[u] / self.cost[u];
        }
    }

    /// Factories go a step of the way from what they make to what the staff wants.
    pub(crate) fn retool(&mut self) {
        for u in 0..self.plan.len() {
            self.making[u] += RETOOL * (self.plan[u] - self.making[u]);
            if self.is_locked(u) {
                self.making[u] = 0.0;
            }
        }
    }

    /// Whether the country cannot field kind `u`.
    pub fn is_locked(&self, u: usize) -> bool {
        self.locked.get(u).copied().unwrap_or(false)
    }

    /// The kinds the country can field are `locked`'s falses: the shares of the others go to them
    /// (by their shares, or evenly when none had any).
    pub fn lock(&mut self, locked: Vec<bool>) {
        if locked.iter().all(|&l| !l) {
            self.locked = locked;
            return;
        }
        self.locked = locked;
        for v in [&mut self.plan, &mut self.making] {
            let free: Vec<usize> = (0..v.len()).filter(|&u| !self.locked.get(u).copied().unwrap_or(false)).collect();
            if free.is_empty() {
                continue;
            }
            let gone: f32 = (0..v.len()).filter(|u| !free.contains(u)).map(|u| v[u]).sum();
            if gone <= 0.0 {
                continue;
            }
            let kept: f32 = free.iter().map(|&u| v[u]).sum();
            for u in 0..v.len() {
                if !free.contains(&u) {
                    v[u] = 0.0;
                }
            }
            for &u in &free {
                v[u] += if kept > 1e-9 { gone * v[u] / kept } else { gone / free.len() as f32 };
            }
        }
    }
}

