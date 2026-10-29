//! The player's decisions as the window (and the journal of a saved game) sends them: a form
//! of keys and values — levers, orders, answers, treaties — applied by the same rules whoever
//! asks. The parts go in a fixed order (a saved game relives them so): who runs what, the levers,
//! the templates and what the army builds, the front, the war, statecraft, policy, diplomacy,
//! society.

use super::*;

/// The form's value of key `k`.
type Get<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The number under key `k`, within `lo`…`hi` (a comma as the decimal point too).
fn num(get: Get, k: &str, lo: f32, hi: f32) -> Option<f32> {
    get(k).and_then(|v| v.trim().replace(',', ".").parse::<f32>().ok()).map(|x| x.clamp(lo, hi))
}

impl Game {
    /// What the player chose: the levers, who runs what, the war.
    pub fn apply(&mut self, form: &[(String, String)]) {
        self.version += 1;
        self.stamp += 1;
        let get = |k: &str| form.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
        self.apply_heads(&get);
        self.apply_levers(&get);
        self.apply_army_money(&get);
        self.apply_navy(&get);
        self.apply_templates(&get);
        self.apply_building(&get);
        self.apply_front(&get);
        if get("declare").is_some() && !self.h.at_war {
            self.declare_war();
        }
        self.apply_statecraft(&get);
        // the economic policy and the factories: always yours
        self.apply_policy(&get);
        // diplomacy: pacts proposed and broken, envoys, a war of the world left
        self.relations_form(form);
        // the order of society, the great task
        let me = self.me;
        self.realm_form(me, form);
        self.agenda_form(me, form);
        // (the laws of the draft at once: the player's choice, or a war just declared)
        self.keep_laws();
    }

    /// Who runs the economy (you, or one of the three) and who heads science.
    fn apply_heads(&mut self, get: Get) {
        let me = self.me;
        let pack = self.pack.clone();
        let month = self.h.peace_months + self.h.war.month;
        let chosen = match get("econ").as_deref() {
            Some("me") => Some(None),
            Some(v) => v.strip_prefix("adv").and_then(|k| k.parse::<usize>().ok()).filter(|&k| k < self.pool.len()).map(Some),
            None => None,
        };
        if let Some(who) = chosen.filter(|&who| who != self.adviser) {
            self.adviser = who;
            self.h.econ[me] = who.map(|k| Economist::new(self.pool[k].1, hash64(self.seed ^ 0xAD2 ^ (k as u64) << 8 ^ month as u64), &pack));
            self.since[0] = month;
            let text = match who {
                Some(k) => format!("экономикой правит {}", self.pool[k].0),
                None => "экономикой вы правите сами".to_string(),
            };
            self.news.push(Note::new(month, &text, false, "econ"));
            self.folk_appoint(strategy::world::folk::Post::Economy, who);
        }
        if self.h.labs[me].is_none() {
            return;
        }
        let chosen = match get("scihead").as_deref() {
            Some("me") => Some(None),
            Some(v) => v.strip_prefix("sci").and_then(|k| k.parse::<usize>().ok()).filter(|&k| k < self.scholars.len()).map(Some),
            None => None,
        };
        if let Some(who) = chosen.filter(|&who| who != self.scholar) {
            self.scholar = who;
            self.h.scientists[me] = who.map(|k| Scientist::new(self.scholars[k].1, hash64(self.seed ^ 0x5C2 ^ (k as u64) << 8 ^ month as u64), &pack));
            self.since[1] = month;
            let text = match who {
                Some(k) => format!("науку возглавил {}", self.scholars[k].0),
                None => "науку вы ведёте сами".to_string(),
            };
            self.news.push(Note::new(month, &text, false, "science"));
            self.folk_appoint(strategy::world::folk::Post::Science, who);
        }
    }

    /// The levers: the law of the draft, the army's and science's shares of spending, what the
    /// laboratories work on (with no scientist), the tax and building (with no economist).
    fn apply_levers(&mut self, get: Get) {
        let me = self.me;
        let pack = self.pack.clone();
        // the law of the draft is yours too (`custom`: as the custom of peace and war has it)
        if let Some(v) = get("draft") {
            self.draft[me] = v.trim().parse::<usize>().ok().filter(|&k| k < strategy::economy::nation::LAWS.len());
        }
        // the army's share of the state's spending is always yours
        if let Some(x) = num(get, "army", 0.0, 100.0) {
            self.h.army_share[me] = Some(x / 100.0);
            self.h.home_mut(me).mil_share = x / 100.0;
        }
        // science: its share of spending is yours; what it works on, yours or a scientist's
        if self.h.labs[me].is_some() {
            if let Some(x) = num(get, "science", 0.0, 50.0) {
                self.h.home_mut(me).science_share = x / 100.0;
            }
            if self.scholar.is_none() && get("scifocus").is_some() {
                let k = pack.units.len() + 1;
                let focus: Vec<f32> = match get("scifocus").and_then(|v| v.parse::<usize>().ok()).filter(|&i| i < k) {
                    Some(i) => (0..k).map(|j| if j == i { 1.0 } else { 0.0 }).collect(),
                    None => vec![1.0 / k as f32; k],
                };
                if let Some(lab) = self.h.labs[me].as_mut() {
                    lab.focus = focus;
                }
            }
        }
        if self.adviser.is_none() {
            let n = self.h.home_mut(me);
            if let Some(x) = num(get, "tax", 0.0, 80.0) {
                n.tax = x / 100.0;
            }
            if let Some(x) = num(get, "build", 0.0, 100.0) {
                n.build_share = x / 100.0;
            }
            if let Some(v) = get("focus") {
                n.build_focus = v.parse::<usize>().ok().filter(|&s| s < pack.sectors.len());
            }
            n.plan = 0.0;
        }
    }

    /// The army as the government keeps it: its peace establishment (`custom`: as the custom has it),
    /// a mobilisation ordered or called off.
    fn apply_army_money(&mut self, get: Get) {
        let me = self.me;
        if let Some(v) = get("establish") {
            self.mil.chosen[me] = v.trim().parse::<usize>().ok().filter(|&k| k < crate::army::budget::ESTABLISHMENTS.len());
        }
        if let Some(v) = get("mobilise") {
            self.order_mobilisation(me, v.trim() == "1");
        }
    }

    /// The navy: its share of the army's money (`custom`: the custom one), ships laid down (the yards
    /// are then the player's) or struck off the slipways, the yards given back to the admiralty; a
    /// squadron's mission and zone (it is then the player's), a squadron given back.
    fn apply_navy(&mut self, get: Get) {
        let me = self.me;
        if let Some(v) = get("navy_share") {
            self.navy.share[me] = v.trim().replace(',', ".").parse::<f32>().ok().map(|x| (x / 100.0).clamp(0.0, 0.6));
        }
        if let Some(k) = get("lay").and_then(|v| v.trim().parse::<usize>().ok()) {
            self.lay_down(me, k);
            self.navy.yard_staff[me] = false;
        }
        if let Some(i) = get("unlay").and_then(|v| v.trim().parse::<usize>().ok()) {
            if i < self.navy.yards[me].len() {
                self.navy.yards[me].remove(i);
            }
            self.navy.yard_staff[me] = false;
        }
        if get("yards").as_deref() == Some("staff") {
            self.navy.yard_staff[me] = true;
        }
        if let Some(q) = get("sq").and_then(|v| v.trim().parse::<usize>().ok()).filter(|&q| q < self.navy.fleets[me].len()) {
            let nz = self.navy.seas.zones.len();
            if get("sq_staff").is_some() {
                self.navy.fleets[me][q].staff = true;
            }
            if let Some(m) = get("mission").and_then(|v| v.trim().parse::<usize>().ok()).filter(|&m| m < strategy::military::navy::MISSIONS.len()) {
                let sq = &mut self.navy.fleets[me][q];
                sq.mission = strategy::military::navy::MISSIONS[m].0;
                sq.staff = false;
            }
            if let Some(z) = get("zone").and_then(|v| v.trim().parse::<usize>().ok()).filter(|&z| z < nz) {
                let sq = &mut self.navy.fleets[me][q];
                sq.zone = z;
                sq.staff = false;
            }
        }
    }

    /// Division templates: made, changed, dropped; the staff's taken as one's own.
    fn apply_templates(&mut self, get: Get) {
        let me = self.me;
        let pack = self.pack.clone();
        let nu = pack.units.len();
        if get("tpl_take").is_some() {
            let c = &self.h.war.sides[me];
            self.templates.insert(0, templates::of_plan("Дивизия генштаба", &c.plan, c));
        }
        let Some(which) = get("tpl") else { return };
        let k = which.parse::<usize>().ok().filter(|&k| k < self.templates.len());
        if get("tpl_del").is_some() {
            if let Some(k) = k {
                self.templates.remove(k);
            }
            return;
        }
        let bat: Option<Vec<u32>> = get("tpl_bat").map(|v| v.split(',').map(|x| x.trim().parse::<u32>().unwrap_or(0)).collect());
        let bat = bat.filter(|b| b.len() == nu).map(|mut b| {
            // no more than a division holds
            while b.iter().sum::<u32>() > MOST {
                if let Some(u) = (0..nu).max_by_key(|&u| b[u]) {
                    b[u] -= 1;
                }
            }
            b
        });
        let want = get("tpl_want").and_then(|v| v.parse::<usize>().ok()).map(|i| WANT[i.min(WANT.len() - 1)].1);
        let name = get("tpl_name").map(|v| v.trim().chars().take(40).collect::<String>()).filter(|v| !v.is_empty());
        // (the kit of each kind as a level of `norm::LEVELS`, the kinds without norms standard;
        // the companies by their numbers in the pack)
        let norm: Option<Vec<f32>> = get("tpl_norm").map(|v| {
            let lv: Vec<usize> = v.split(',').map(|x| x.trim().parse::<usize>().unwrap_or(1)).collect();
            (0..nu).map(|u| if pack.units[u].norms { strategy::military::doctrine::norm::LEVELS[lv.get(u).copied().unwrap_or(1).min(2)] } else { 1.0 }).collect()
        });
        let support: Option<Vec<usize>> = get("tpl_sup").map(|v| {
            let mut s: Vec<usize> = v.split(',').filter_map(|x| x.trim().parse::<usize>().ok()).filter(|&k| k < pack.support.len()).collect();
            s.sort_unstable();
            s.dedup();
            s
        });
        match k {
            Some(k) => {
                let t = &mut self.templates[k];
                if let Some(b) = bat {
                    t.bat = b;
                }
                if let Some(x) = norm {
                    t.norm = x;
                }
                if let Some(x) = support {
                    t.support = x;
                }
                if let Some(w) = want {
                    t.want = w;
                }
                if let Some(n) = name {
                    t.name = n;
                }
            }
            None if which == "new" => {
                let b = bat.unwrap_or_else(|| vec![0; nu]);
                if b.iter().sum::<u32>() > 0 {
                    let mut t = Template::new(&name.unwrap_or_else(|| "Новая дивизия".into()), b, want.unwrap_or(1.0));
                    t.norm = norm.unwrap_or_default();
                    t.support = support.unwrap_or_default();
                    self.templates.push(t);
                }
            }
            None => {}
        }
    }

    /// Who decides what the army builds (the player's templates, or the staff), the kit the
    /// factories make, the templates new divisions are formed by.
    fn apply_building(&mut self, get: Get) {
        let me = self.me;
        let nu = self.pack.units.len();
        let c = &mut self.h.war.sides[me];
        match get("ctrl").as_deref() {
            None => {}
            Some("me") => {
                // (shares set directly, as the old page of tables did; else by the templates)
                let plan: Vec<f32> = (0..nu).map(|u| num(get, &format!("plan{u}"), 0.0, 100.0).unwrap_or(0.0)).collect();
                let all: f32 = plan.iter().sum();
                if all > 0.0 {
                    c.plan = plan.iter().map(|x| x / all).collect();
                    c.fixed = true;
                } else if let Some(p) = templates::plan_of(&self.templates, c) {
                    c.plan = p;
                    c.fixed = true;
                }
            }
            Some(_) => c.fixed = false,
        }
        // the player's templates give the plan while he decides what the army builds; saving a
        // template of his own is deciding it (the staff no longer builds as it likes)
        if get("tpl").is_some() && !c.fixed && templates::plan_of(&self.templates, c).is_some() {
            c.fixed = true;
            self.news.push(Note::new(self.h.peace_months + self.h.war.month, "армию теперь строят по вашим шаблонам дивизий", false, "army"));
        }
        let c = &mut self.h.war.sides[me];
        let touched = get("tpl").is_some() || get("tpl_take").is_some();
        if c.fixed && touched {
            if let Some(p) = templates::plan_of(&self.templates, c) {
                c.plan = p;
            }
        }
        if !touched && get("ctrl").is_none() {
            return;
        }
        // the kit the factories make: by his templates while he decides what the army builds,
        // else as the staff chooses
        let norm = templates::norm_of(&self.templates, nu);
        let c = &mut self.h.war.sides[me];
        c.norm_free = !c.fixed;
        if c.fixed {
            c.norm = norm;
            c.price_norms();
        }
        // (the new divisions are formed by the templates he builds now)
        if self.h.war.corps.is_some() {
            let (shapes, wants) = self.shapes();
            self.h.war.set_shapes(me, shapes, wants);
        }
    }

    /// The front: orders, shares and generals of the player's groups; the whole front to the
    /// staff; where new divisions gather and which group they join.
    fn apply_front(&mut self, get: Get) {
        let me = self.me;
        if let Some(k) = get("grp").and_then(|v| v.parse::<usize>().ok()) {
            let w = &mut self.h.war;
            let n = w.command[me].as_ref().map_or(0, |g| g.len());
            if k < n {
                match get("order").as_deref() {
                    Some("hold") => w.tell(me, k, Order::Hold),
                    Some("deep") => w.tell(me, k, Order::Deep),
                    Some("attack") => {
                        let head = get("target")
                            .and_then(|v| v.parse::<usize>().ok())
                            .and_then(|p| w.th.at.get(p).copied())
                            .filter(|&q| q != front::NO && w.owner[q as usize] != me)
                            .map_or(w.th.capital[1 - me], |q| q as usize);
                        w.tell(me, k, Order::Attack(head));
                    }
                    Some("staff") => w.command[me].as_mut().unwrap()[k].staff = true,
                    _ => {}
                }
                if let Some(x) = num(get, "share", 0.0, 80.0) {
                    let others: f32 = w.command[me].as_ref().unwrap().iter().enumerate().filter(|(j, g)| *j != k && !g.staff).map(|(_, g)| g.share).sum();
                    let g = &mut w.command[me].as_mut().unwrap()[k];
                    g.share = (x / 100.0).min((0.95 - others).max(0.0));
                    g.staff = false;
                }
                // (the general relieved: he goes to the reserve, the army fights without one)
                if get("general_off").is_some() {
                    if let Some(old) = self.h.war.command[me].as_mut().unwrap()[k].general.take() {
                        self.news.push(Note::new(self.month(), &format!("{} снят с командования: {}", old.name, group_name(k).to_lowercase()), false, "army"));
                        self.spare.push(old);
                    }
                }
                let w = &mut self.h.war;
                if let Some(i) = get("general").and_then(|v| v.parse::<usize>().ok()).filter(|&i| i < self.spare.len()) {
                    // (an army without a general takes the man from the reserve; one with a
                    // general gives him to the reserve in exchange)
                    let new = match w.command[me].as_mut().unwrap()[k].general.take() {
                        Some(old) => std::mem::replace(&mut self.spare[i], old),
                        None => self.spare.remove(i),
                    };
                    self.news.push(Note::new(self.month(), &format!("{} принял {}", new.name, group_name(k).to_lowercase()), false, "army"));
                    self.h.war.command[me].as_mut().unwrap()[k].general = Some(new);
                }
                // (the army disbanded: its divisions to the staff's reserve, its general to the
                // reserve of generals)
                // (the spoiled control of the window's self-checks: nothing is disbanded)
                if get("disband").is_some() && !crate::spoiled() {
                    if let Some(gen) = self.h.war.disband(me, k) {
                        self.news.push(Note::new(self.month(), &format!("{} расформирована, дивизии — в резерв ставки", group_name(k)), false, "army"));
                        if let Some(gen) = gen {
                            self.spare.push(gen);
                        }
                    }
                }
            }
        }
        // a new man called to the reserve of generals (the staff names a colonel it trusts)
        if get("general_new").is_some() && self.spare.len() < 12 {
            let mut r = Rng::new(hash64(self.seed ^ 0x6E7E ^ ((self.month() as u64) << 8) ^ self.day as u64 ^ ((self.journal.len() as u64) << 32)));
            let more = self.new_general(me, &mut r);
            self.news.push(Note::new(self.month(), &format!("в резерв генералов вызван {}", more.name), false, "army"));
            self.spare.push(more);
        }
        if get("front").as_deref() == Some("staff") {
            if let Some(gs) = self.h.war.command[me].as_mut() {
                for g in gs.iter_mut() {
                    g.staff = true;
                }
            }
        }
        // where the new divisions gather (a province of the theatre, or "-": as the staff likes)
        if let Some(v) = get("muster") {
            let q = v.parse::<usize>().ok().and_then(|p| self.h.war.th.at.get(p).copied()).filter(|&q| q != front::NO && self.h.war.owner[q as usize] == me);
            if let Some(c) = self.h.war.corps.as_mut() {
                c.muster[me] = q.map(|q| q as usize);
            }
        }
        if let Some(v) = get("muster_grp") {
            // (the group is the player's from now: his order, as any other; else the staff would
            // even its share out again)
            let k = v.parse::<usize>().ok().filter(|&k| self.h.war.command[me].as_ref().is_some_and(|gs| k < gs.len()));
            if let Some(gr) = k.and_then(|k| self.h.war.command[me].as_mut().map(|gs| &mut gs[k])) {
                gr.staff = false;
            }
            if let Some(c) = self.h.war.corps.as_mut() {
                c.muster_group[me] = k;
            }
        }
    }

    /// The player declares the war: both countries go onto a war footing (more of the state's
    /// spending to the army, a war tax — month by month, `war_footing`; the player may change
    /// it, and the peace gives his levers back); both sides fight by groups of armies, a general
    /// at the head of each, their staffs leading them till the player takes his.
    fn declare_war(&mut self) {
        let me = self.me;
        let pack = self.pack.clone();
        self.state.aggressor.get_or_insert(me);
        self.state.claim = [None, None];
        let n = self.h.home(me);
        self.prewar = Some((n.tax, n.mil_share));
        // (the first month of the footing: the rest comes month by month)
        self.war_footing();
        self.h.declare(&pack);
        self.pair_war_begins();
        let mut r = Rng::new(hash64(self.seed ^ 0x6E4 ^ self.month() as u64));
        // (the player's groups of peace go to war as they stand: their stretches, generals,
        // orders and plans)
        let ready = self.h.war.command[me].clone();
        for s in 0..2 {
            if s == me && ready.is_some() {
                continue;
            }
            self.h.war.by_armies(s, true);
            // the generals of its last wars first, then its soldiers of note, then new men
            let k = self.h.war.command[s].as_ref().map_or(0, |gs| gs.len());
            let gens: Vec<General> = (0..k).map(|_| self.new_general(s, &mut r)).collect();
            if let Some(gs) = self.h.war.command[s].as_mut() {
                for (g, x) in gs.iter_mut().zip(gens) {
                    g.general = Some(x);
                }
            }
        }
        if ready.is_none() || self.spare.is_empty() {
            let spare: Vec<General> = (0..3).map(|_| self.new_general(me, &mut r)).collect();
            self.spare = spare;
        }
        // the armies' divisions go to their groups (formed anew if there were none)
        if self.h.war.corps.is_some() {
            self.h.war.mobilise();
            // (groups whose stretch of the border of peace is not on the war's front take it anew)
            for s in 0..2 {
                self.reanchor(s);
            }
        } else {
            self.muster();
        }
        if let Some(x) = self.h.army_share[me] {
            // (an army's share the player set himself stays his)
            self.h.home_mut(me).mil_share = x;
        }
        self.news.push(Note::new(self.month(), "война объявлена", true, "army"));
        self.news.push(Note::new(self.month(), "страна переходит на военное положение: за год армии всё больше трат, налог выше", true, "econ"));
    }

    /// Statecraft: a claim to work up, a peace to offer, an event answered, a depot, the list of
    /// what the state builds.
    fn apply_statecraft(&mut self, get: Get) {
        let me = self.me;
        if get("claim").is_some() {
            self.claim();
        }
        if get("peace").is_some() {
            self.offer_peace(me);
        }
        if let (Some(id), Some(k)) = (get("ev").and_then(|v| v.parse::<u64>().ok()), get("k").and_then(|v| v.parse::<usize>().ok())) {
            self.answer(id, k);
        }
        if let Some(q) = get("hub").and_then(|v| v.parse::<usize>().ok()).and_then(|p| self.h.war.th.at.get(p).copied()).filter(|&q| q != front::NO) {
            self.build_depot(q as usize);
        }
        if let Some(v) = get("logistics") {
            self.state.staff_logistics = v == "1";
        }
        if get("queue_set").is_some() {
            self.state.queue = (0..self.pack.sectors.len()).filter(|k| get(&format!("q{k}")).is_some()).collect();
            if self.state.queue.is_empty() {
                self.h.home_mut(me).build_focus = None;
            }
        }
    }

    /// The player's word on the economic order, the programmes, loans, new models and the
    /// priorities of production.
    fn apply_policy(&mut self, get: Get) {
        let me = self.me;
        let num = |k: &str| get(k).and_then(|v| v.trim().parse::<u64>().ok());
        if let Some(k) = num("ecolaw") {
            self.set_eco_law(me, k as usize);
        }
        if let Some(k) = num("plan_go") {
            self.begin_plan(me, k as usize);
        }
        if let Some(k) = num("plan_stop") {
            self.stop_plan(me, k as usize);
        }
        if let Some(k) = num("modern") {
            self.modernise(me, k as usize);
        }
        if get("modern_stop").is_some() {
            self.policy.modern[me] = None;
        }
        if get("loan").is_some() {
            self.take_loan(me);
        }
        if get("repay").is_some() {
            self.repay(me);
        }
        // the civil budget (`civ=line:level`), the bonds at home
        if let Some((k, lv)) = get("civ").as_deref().and_then(|v| v.split_once(':')) {
            if let (Ok(k), Ok(lv)) = (k.trim().parse::<usize>(), lv.trim().parse::<u8>()) {
                self.set_civil(me, k, lv);
            }
        }
        if get("bonds_sell").is_some() {
            self.sell_bonds(me);
        }
        if get("bonds_redeem").is_some() {
            self.redeem_bonds(me);
        }
        if get("bonds_repudiate").is_some() {
            self.repudiate(me);
        }
        // what the laboratories work on; the state's finances
        if let Some(t) = num("tech") {
            self.set_tech(me, t as usize);
        }
        for what in ["tax", "sales", "profit", "estate", "dole", "tariff", "caps", "labour", "rate", "print"] {
            if let Some(k) = num(&format!("fx_{what}")) {
                self.set_fiscal(me, what, k as usize);
            }
        }
        if let Some(v) = get("fx_bonds") {
            self.war_mood.bonds[me] = v.trim().parse::<usize>().ok().filter(|&k| k < crate::country::war_mood::BONDS.len());
        }
        if let Some(k) = num("fx_embargo") {
            self.set_embargo(me, k > 0);
        }
        for (key, public) in [("fx_sub", false), ("fx_pub", true)] {
            if let Some((k, lv)) = get(key).as_deref().and_then(|v| v.split_once(':')) {
                if let (Ok(k), Ok(lv)) = (k.trim().parse::<usize>(), lv.trim().parse::<u8>()) {
                    self.set_sector(me, k, lv, public);
                }
            }
        }
        if let Some(id) = num("model_ok") {
            self.adopt(me, id);
        }
        if let Some(id) = num("model_try") {
            self.try_model(me, id);
        }
        if let Some(id) = num("model_no") {
            self.refuse(me, id);
        }
        if let Some((u, lv)) = get("prio").filter(|_| !crate::spoiled()).as_deref().and_then(|v| v.split_once(':')) {
            if let (Ok(u), Ok(lv)) = (u.trim().parse::<usize>(), lv.trim().parse::<usize>()) {
                if let Some(l) = self.arms.lines[me].get_mut(u) {
                    l.prio = lv.min(crate::army::arms::PRIO.len() - 1);
                }
            }
        }
    }
}
