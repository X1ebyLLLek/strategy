//! How the forces of society judge the government — each lever's part in each force's mood, with
//! its reason — and what the order does to the country: its form of government, laws, faith,
//! legitimacy and unrest as effects.

use super::*;

/// The order of labour: how readily idle people of each law leave for a group that pays better.
pub const MOBILITY: [f32; 3] = [0.15, 0.5, 1.0];
/// The order of land: the landowner's rent (his share of what the land adds) against the custom, by
/// the law of land, and times again under bondage.
pub const RENT: [f32; 3] = [1.6, 1.0, 0.45];
pub const SERF_RENT: [f32; 3] = [1.25, 1.0, 1.0];

/// What a form of government does.
pub fn regime_eff(r: Regime) -> Eff {
    let e = Eff::default;
    match r {
        Regime::Chiefs => Eff { out_industry: 0.92, attack: 1.04, leak: 0.02, ..e() },
        Regime::Autocracy => Eff { leak: 0.01, stability: 0.001, ..e() },
        Regime::Estates => Eff { out_land: 1.02, leak: 0.008, ..e() },
        Regime::Constitutional => Eff { out_civil: 1.02, science: 1.03, ..e() },
        Regime::Patricians => Eff { out_civil: 1.04, out_industry: 1.02, anger: [0.002, 0.002, 0.0, -0.002], ..e() },
        Regime::Republic => Eff { science: 1.05, anger: [-0.001, -0.001, -0.001, 0.0], stability: -0.001, ..e() },
        Regime::Junta => Eff { attack: 1.04, draft: 1.1, out_military: 1.05, science: 0.94, leak: 0.015, ..e() },
        Regime::Party => Eff { out_industry: 1.04, make_cost: 0.96, out_civil: 0.94, science: 0.97, draft: 1.15, ..e() },
        Regime::Theocracy => Eff { science: 0.88, stability: 0.002, out_civil: 0.97, ..e() },
    }
}

/// What law `k` at `level` does (`piety`: how much the faith's law weighs).
pub fn law_eff(k: usize, level: u8, piety: f32) -> Eff {
    let e = Eff::default;
    // (the order of labour and of land works through the market — `laws_apply`: serfs may not leave
    // for the factories, estates take the tiller's harvest, and the purse does the rest; the schools
    // work through literacy and the learned it makes — `civil`, `demography`)
    match (k, level) {
        (polity::LABOUR, 0) => Eff { out_land: 1.02, ..e() },
        (polity::LAND, 2) => Eff { out_land: 1.03, ..e() },
        (polity::VOICE, 0) => Eff { stability: -0.001, ..e() },
        (polity::VOICE, 2) => Eff { anger: [-0.002, -0.002, -0.001, 0.0], stability: 0.001, ..e() },
        (polity::CHURCH, 0) => Eff { science: 0.94, stability: 0.002 * piety, ..e() },
        (polity::CHURCH, 2) => Eff { science: 1.05, stability: -0.002 * piety, ..e() },
        (polity::PRESS, 0) => Eff { science: 0.92, stability: 0.001, ..e() },
        (polity::PRESS, 2) => Eff { science: 1.06, out_civil: 1.02, stability: -0.001, ..e() },
        (polity::SERVICE, 0) => Eff { draft: 0.6, attack: 1.05, exp: 1.1, ..e() },
        (polity::SERVICE, 2) => Eff { draft: 1.35, exp: 0.9, ..e() },
        _ => e(),
    }
}

/// What a faith's tenets do to its people (by how pious they are).
pub fn faith_eff(t: &[f32; strategy::world::polity::faith::TENETS], piety: f32) -> Eff {
    use strategy::world::polity::faith::*;
    let k = piety.clamp(0.0, 1.0);
    Eff {
        science: 1.0 + 0.08 * t[LEARNING] * k,
        out_civil: 1.0 + 0.04 * t[COMMERCE] * k,
        out_industry: 1.0 + 0.03 * t[THRIFT] * k,
        attack: 1.0 + 0.04 * t[WAR].max(0.0) * k,
        anger: [-0.002 * t[CHARITY] * k, -0.002 * t[CHARITY] * k, 0.0, 0.0],
        ..Eff::default()
    }
}

/// What legitimacy and unrest do to the government's standing a month.
pub fn mood_eff(legit: f32, unrest: f32) -> Eff {
    Eff { stability: 0.05 * (0.4 * (legit - 0.5) - 0.5 * (unrest - 0.2)), ..Eff::default() }
}

/// What country `c`'s order does (the iron hand, the purge: in the game).
pub fn realm_eff_of(pol: &polity::Polities, c: usize, crackdown: bool, purge: bool) -> Eff {
    let p = &pol.of[c];
    let mut e = regime_eff(p.regime);
    for k in 0..LAWS {
        e = e.and(&law_eff(k, p.laws[k], p.piety));
    }
    if let Some(fa) = pol.faiths.get(p.state_faith) {
        e = e.and(&faith_eff(&fa.tenets, p.piety));
    }
    e = e.and(&mood_eff(p.legit, p.unrest));
    if crackdown {
        e = e.and(&Eff { stability: 0.006, science: 0.95, ..Eff::default() });
    }
    if purge {
        e = e.and(&Eff { attack: 0.93, defend: 0.95, ..Eff::default() });
    }
    let _ = MOVES;
    e
}

impl Game {
    /// The order of labour and of land laid on each side's economy: how freely idle people may leave
    /// for a group that pays better (serfs may not, the dependent hardly), and the landowner's share
    /// of what the land adds — the rent: estates take more of it, land of his own leaves the tiller
    /// all but a little (as after a buy-out), serfs owe labour besides. (As a share of the rent, not of
    /// the wage: so a pack whose fields pay the tiller 90% and one whose pay him 75% lose the same
    /// part of the rent to a reform.)
    pub fn laws_apply(&mut self) {
        let pack = self.pack.clone();
        for s in 0..2 {
            let p = self.polity_of(s);
            let labour = p.laws[polity::LABOUR].min(2) as usize;
            let land = p.laws[polity::LAND].min(2) as usize;
            let (mobility, rent) = if crate::country::econ_spoiled() { (1.0, 1.0) } else { (MOBILITY[labour], RENT[land] * SERF_RENT[labour]) };
            let n = self.h.home_mut(s);
            n.mobility_k = mobility;
            n.wage_by = if rent == 1.0 {
                Vec::new()
            } else {
                pack.sectors.iter().map(|x| if x.land && x.wage_share > 0.0 { (1.0 - (1.0 - x.wage_share) * rent).max(0.0) / x.wage_share } else { 1.0 }).collect()
            };
        }
    }

    /// What force `b` of side `s` feels about the government's doings now, thing by thing (its
    /// laws aside: `law_parts`).
    pub fn mood_parts(&self, s: usize, b: usize) -> Vec<Part> {
        use polity::*;
        let n = self.h.home(s);
        let f = &self.spirits.facts[s];
        let mut v: Vec<Part> = Vec::new();
        let mut add = |what: &str, x: f32| {
            if x.abs() >= 0.01 {
                v.push((what.to_string(), x));
            }
        };
        let poor = matches!(b, WORKERS | PEASANTS);
        // (what the tax, the dole, credit and the press do to a force's purse it feels through the
        // market — `interest_parts`; what is left here is belief: free trade is the merchants' creed,
        // the union the worker's right, sound money the creditor's faith)
        match self.fiscal.tariff[s] {
            0 => add("свободная торговля", if b == MERCHANTS { 0.1 } else { 0.0 }),
            2 => add("защита промышленности", if b == CAPITAL { 0.1 } else if b == MERCHANTS { -0.08 } else { 0.0 }),
            3 => add("автаркия", if b == CAPITAL { 0.05 } else if b == MERCHANTS { -0.2 } else { 0.0 }),
            _ => {}
        }
        match self.fiscal.labour[s] {
            0 => add("без защиты труда", if b == WORKERS { -0.15 } else { 0.0 }),
            2 => add("сильные профсоюзы", if b == WORKERS { 0.1 } else if b == CAPITAL { -0.05 } else { 0.0 }),
            3 => add("рабочий контроль", if b == WORKERS { 0.15 } else if b == CAPITAL { -0.25 } else if b == NOBLES { -0.05 } else { 0.0 }),
            _ => {}
        }
        if self.fiscal.rate[s] == 3 {
            add("золотой стандарт", if b == MERCHANTS { 0.08 } else { 0.0 });
        }
        // the army and science
        let mil = n.mil_share - 0.3;
        add("доля армии в тратах", match b {
            OFFICERS => (mil * 1.5).clamp(-0.4, 0.4),
            PEASANTS | MERCHANTS => (-mil * 0.5).clamp(-0.2, 0.2),
            _ => 0.0,
        });
        let custom = self.pack.science.as_ref().map_or(0.03, |x| x.share);
        if b == LEARNED {
            add("доля науки в тратах", ((n.science_share - custom) * 8.0).clamp(-0.3, 0.3));
        }
        let draft = n.draft_cap - 0.03;
        add("призыв", match b {
            OFFICERS => (draft * 3.0).clamp(-0.1, 0.3),
            PEASANTS | WORKERS => (-draft * 2.0).clamp(-0.3, 0.1),
            _ => 0.0,
        });
        // the order of the economy and the programmes
        match self.policy.law[s] {
            0 => add("вольный рынок", if b == CAPITAL || b == MERCHANTS { 0.1 } else { 0.0 }),
            2 => add("военная экономика", if b == OFFICERS { 0.15 } else if b == CAPITAL { 0.05 } else if b == WORKERS { -0.1 } else { 0.0 }),
            3 => add("тотальная мобилизация", if b == OFFICERS { 0.25 } else if poor { -0.2 } else if b == MERCHANTS { -0.15 } else { 0.0 }),
            _ => {}
        }
        for r in &self.policy.running[s] {
            let id = crate::country::policy::PLANS[r.plan].id;
            let x = match (id, b) {
                ("industry", CAPITAL) | ("industry", WORKERS) => 0.1,
                ("land", PEASANTS) => 0.15,
                ("land", NOBLES) => -0.15,
                ("schools", LEARNED) => 0.15,
                ("schools", CLERGY) => -0.05,
                ("rearm", OFFICERS) | ("forts", OFFICERS) => 0.12,
                ("austerity", WORKERS) | ("austerity", PEASANTS) => -0.12,
                ("austerity", MERCHANTS) => 0.08,
                ("works", WORKERS) => 0.12,
                ("patriots", OFFICERS) | ("patriots", CLERGY) => 0.08,
                _ => 0.0,
            };
            add(&format!("программа «{}»", crate::country::policy::PLANS[r.plan].name), x);
        }
        // the army as the government keeps it: its establishment, a mobilisation in peace, the arms
        // orders (the industrialists live on them), the war's course as each force sees it
        let est = self.establishment(s) / crate::army::budget::PEACE_PER_FRONT - 1.0;
        add("штат армии", match b {
            OFFICERS => (0.25 * est).clamp(-0.2, 0.25),
            PEASANTS | WORKERS => (-0.1 * est).clamp(-0.1, 0.05),
            _ => 0.0,
        });
        if !self.h.at_war && self.mil.mobilised[s].is_some() {
            add("мобилизация", match b {
                OFFICERS => 0.12,
                PEASANTS | WORKERS => -0.18,
                MERCHANTS => -0.12,
                _ => -0.04,
            });
        }
        if b == CAPITAL {
            let book = &self.mil.book[s];
            let orders = (book.equip + book.mend + book.navy) / n.gdp.max(1e-6);
            add("военные заказы", (orders * 4.0).clamp(0.0, 0.2));
        }
        add("ход войны", self.war_mood_of_bloc(s, b));
        // the war
        if self.h.at_war {
            let held = self.h.war.people();
            let (ours, theirs) = (held[s], held[1 - s]);
            add("идёт война", match b {
                OFFICERS => 0.1,
                MERCHANTS => -0.2,
                PEASANTS | WORKERS => -0.1,
                _ => -0.03,
            });
            if ours < 0.92 {
                add("враг на нашей земле", -0.25 * (1.0 - ours) * 4.0 * if b == OFFICERS { 1.3 } else { 1.0 });
            }
            if theirs < 0.92 {
                add("победы", 0.2 * (1.0 - theirs) * 4.0 * if b == OFFICERS { 1.5 } else { 0.5 });
            }
        }
        if f.last_won == Some(false) && f.last_war.is_some_and(|y| y < 10.0) {
            add("проигранная война", if b == OFFICERS { -0.3 } else { -0.08 });
        }
        // how they live
        let role = |r: usize| -> f32 {
            let gs: Vec<usize> = (0..self.pack.groups.len()).filter(|&g| crate::country::policy::role(&self.pack, g) == r).collect();
            let all: f32 = gs.iter().map(|&g| n.pop[g]).sum::<f32>().max(1e-6);
            gs.iter().map(|&g| n.discontent[g] * n.pop[g]).sum::<f32>() / all
        };
        let lives = match b {
            PEASANTS => Some(role(0)),
            WORKERS => Some(role(1)),
            LEARNED => Some(role(2)),
            CAPITAL | NOBLES | MERCHANTS => Some(role(3)),
            _ => None,
        };
        if let Some(d) = lives {
            add("как живут", (0.2 - 0.3 * d).clamp(-0.6, 0.2));
        }
        if poor {
            add("голод", -0.6 * f.hunger);
        }
        // the government's own doings
        let fav = self.realm.favour[s][b];
        add("уступки им", fav);
        if self.realm.crackdown[s] > 0 {
            add("твёрдая рука", match b {
                LEARNED => -0.3,
                WORKERS => -0.2,
                MERCHANTS => -0.1,
                OFFICERS => 0.1,
                _ => 0.0,
            });
        }
        if self.realm.purge[s] > 0 && b == OFFICERS {
            add("чистка офицеров", -0.3);
        }
        if let Some(p) = &self.realm.pending[s] {
            let pol = self.polity_of(s);
            let d = law_fit(b, p.law, p.to) - law_fit(b, p.law, pol.laws[p.law]);
            add(&format!("реформа: {}", LAW[p.law].name.to_lowercase()), 0.3 * d);
        }
        // faith
        if b == CLERGY {
            let pol = self.polity_of(s);
            let c = self.country(s);
            let shares = self.polity.shares(self.realm.regions.get(c).map_or(&[][..], |v| &v[..]));
            let state = shares.iter().find(|x| x.0 == pol.state_faith).map_or(0.0, |x| x.1);
            add("паства государственной веры", (state - 0.7) * 0.6);
        }
        // what they have: income, the tax they pay, work — measured; the civil budget's beliefs
        v.extend(self.interest_parts(s, b));
        let civil = self.civil_moods(s, b);
        if civil.abs() >= 0.01 {
            v.push(("гражданский бюджет".into(), civil));
        }
        v
    }

    /// What force `b` of side `s` feels about each law (only those it cares for).
    pub fn law_parts(&self, s: usize, b: usize) -> Vec<Part> {
        let pol = self.polity_of(s);
        (0..LAWS)
            .filter_map(|k| {
                let x = polity::law_fit(b, k, pol.laws[k]);
                (x.abs() >= 0.01).then(|| (format!("{}: {}", LAW[k].name.to_lowercase(), pol.law_word(k)), x * 0.6))
            })
            .collect()
    }

    /// Each force's mood from the government's doings now (−1 … +1).
    pub fn moods_now(&self, s: usize) -> [f32; BLOCS] {
        let mut m = [0.0f32; BLOCS];
        for (b, x) in m.iter_mut().enumerate() {
            *x = self.mood_parts(s, b).iter().map(|p| p.1).sum::<f32>().clamp(-1.0, 1.0);
        }
        m
    }

    /// What side `s`'s order does: its form, laws, faith, unrest and legitimacy.
    pub fn realm_eff(&self, s: usize) -> Eff {
        realm_eff_of(&self.polity, self.country(s), self.realm.crackdown[s] > 0, self.realm.purge[s] > 0)
    }

    /// What side `s`'s order does, thing by thing (for the panel: where each multiplier comes from).
    pub fn realm_eff_parts(&self, s: usize) -> Vec<(String, Eff)> {
        let c = self.country(s);
        let p = &self.polity.of[c];
        let mut v = vec![(format!("строй: {}", p.regime.name().to_lowercase()), regime_eff(p.regime))];
        for k in 0..LAWS {
            v.push((format!("{}: {}", LAW[k].name.to_lowercase(), p.law_word(k)), law_eff(k, p.laws[k], p.piety)));
        }
        if let Some(fa) = self.polity.faiths.get(p.state_faith) {
            v.push((format!("вера: {}", fa.name), faith_eff(&fa.tenets, p.piety)));
        }
        v.push(("законность и смута".into(), mood_eff(p.legit, p.unrest)));
        if self.realm.crackdown[s] > 0 {
            v.push(("твёрдая рука".into(), Eff { stability: 0.006, science: 0.95, ..Eff::default() }));
        }
        if self.realm.purge[s] > 0 {
            v.push(("чистка офицеров".into(), Eff { attack: 0.93, defend: 0.95, ..Eff::default() }));
        }
        v
    }
}
