//! The law of the draft in the game (the laws: `strategy::economy::nation::LAWS`). The player chooses his
//! law; a bot's state keeps the custom one — the peace law in peace, the war law at war (as it
//! goes on a war footing with its spending).

use strategy::economy::nation::{LAWS, PEACE_LAW, WAR_LAW};

use crate::Game;

impl Game {
    /// The law side `s` lives by now: the player's choice, else the custom one.
    pub fn law(&self, s: usize) -> usize {
        self.draft[s].unwrap_or_else(|| self.course_draft(s).unwrap_or(self.custom_law()))
    }

    /// The law of the draft a bot's course asks for in peace: a course of arms keeps more men under
    /// the colours, a thrifty one fewer (at war — the custom of war).
    pub fn course_draft(&self, s: usize) -> Option<usize> {
        if self.h.at_war || (s == self.me && self.auto.is_none()) {
            return None;
        }
        match self.courses.now[s].as_ref().map(|c| c.kind) {
            Some(crate::country::course::Kind::Army) | Some(crate::country::course::Kind::Revanche) => Some(1),
            Some(crate::country::course::Kind::Fortress) => Some(1),
            _ => None,
        }
    }

    /// The custom law of the times: of peace, or of war.
    pub fn custom_law(&self) -> usize {
        if self.h.at_war {
            WAR_LAW
        } else {
            PEACE_LAW
        }
    }

    /// Each side's country set to the law it lives by (the spoiled control of the window: the
    /// law changes nothing — every country as before it, all may serve at the pay of work).
    pub fn keep_laws(&mut self) {
        for s in 0..2 {
            let (cap, pay) = if crate::spoiled() { (strategy::economy::nation::DRAFT_MAX, 1.0) } else { (LAWS[self.law(s)].cap, LAWS[self.law(s)].pay) };
            let n = self.h.home_mut(s);
            n.draft_cap = cap;
            n.draft_pay = pay;
        }
    }
}
