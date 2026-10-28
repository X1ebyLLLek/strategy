//! The posts of each country and who holds them: appointed and leaving, the government choosing
//! among those it knows, the posts filled each year; generals of a war and what a war leaves in
//! their lives; a ruler overthrown; what the ruler and the government lean to.

use super::*;

impl Folk {
    /// Person `id` takes post `post` in month `month` (the one who held it leaves it).
    pub fn appoint(&mut self, c: usize, post: Post, id: u32, month: i32) {
        let old = match post {
            Post::Ruler => self.offices[c].ruler.replace(id),
            Post::Economy => self.offices[c].economy.replace(id),
            Post::Science => self.offices[c].science.replace(id),
            Post::Staff => self.offices[c].staff.replace(id),
            Post::Command => {
                if !self.offices[c].command.contains(&id) {
                    self.offices[c].command.push(id);
                }
                None
            }
            Post::Chair => {
                if !self.offices[c].chairs.contains(&id) {
                    self.offices[c].chairs.push(id);
                }
                None
            }
        };
        if old == Some(id) {
            return;
        }
        if let Some(o) = old {
            self.leave(o, post, month);
        }
        // (one high post at a time: a minister made ruler leaves his ministry)
        let held: Vec<Post> = self.people[id as usize].posts.iter().filter(|s| s.to.is_none() && s.post != post && !matches!(s.post, Post::Command | Post::Chair)).map(|s| s.post).collect();
        for p in held {
            self.vacate(c, p, id, month);
        }
        self.people[id as usize].posts.push(Stint { post, from: month, to: None });
        // (whoever appointed him is his patron)
        if post != Post::Ruler {
            if let Some(r) = self.offices[c].ruler {
                if r != id && matches!(post, Post::Economy | Post::Science | Post::Staff) {
                    self.tie(r, id, Tie::Patron);
                }
            }
        }
    }

    /// Person `id` leaves `post` (his stint ends).
    pub(super) fn leave(&mut self, id: u32, post: Post, month: i32) {
        for s in self.people[id as usize].posts.iter_mut().filter(|s| s.post == post && s.to.is_none()) {
            s.to = Some(month);
        }
    }

    /// Post `post` of country `c` held by `id` falls vacant.
    pub fn vacate(&mut self, c: usize, post: Post, id: u32, month: i32) {
        let o = &mut self.offices[c];
        match post {
            Post::Ruler if o.ruler == Some(id) => o.ruler = None,
            Post::Economy if o.economy == Some(id) => o.economy = None,
            Post::Science if o.science == Some(id) => o.science = None,
            Post::Staff if o.staff == Some(id) => o.staff = None,
            Post::Command => o.command.retain(|&x| x != id),
            Post::Chair => o.chairs.retain(|&x| x != id),
            _ => {}
        }
        self.leave(id, post, month);
    }

    /// The ruler of country `c` chooses one of `candidates` for a post: an able and honest ruler the
    /// ablest (as their deeds and schooling show him, with his own error), another the most loyal
    /// (those already tied to him, those who flatter — the dishonest).
    pub(super) fn choose(&self, c: usize, candidates: &[u32], rng: &mut Rng) -> Option<u32> {
        if candidates.is_empty() {
            return None;
        }
        let (judge, honest) = match self.offices[c].ruler {
            Some(r) => {
                let r = self.get(r);
                (r.talent * r.education, r.honesty)
            }
            None => (0.3, 0.7),
        };
        let ruler = self.offices[c].ruler;
        let score = |id: u32, rng: &mut Rng| {
            let p = self.get(id);
            let able = 0.6 * p.talent + 0.4 * p.education + 0.3 * p.experience + 0.15 * p.fame;
            let seen = able + (0.35 * (1.0 - judge)) * rng.normal();
            let loyal = if ruler.is_some_and(|r| p.ties.iter().any(|t| t.0 == r)) { 0.4 } else { 0.0 } + 0.3 * (1.0 - p.honesty);
            honest * seen + (1.0 - honest) * (loyal + 0.3 * seen)
        };
        let mut best = None;
        let mut top = f32::MIN;
        for &id in candidates {
            let s = score(id, rng);
            if s > top {
                top = s;
                best = Some(id);
            }
        }
        best
    }

    /// The posts of country `c` filled where they stand empty (and a ruler's term ended, or a lost
    /// war, brings a new one).
    pub(super) fn fill(&mut self, pack: &Pack, c: usize, t: &Times, month: i32, rng: &mut Rng) {
        let adult = |p: &Person| (month - p.born) as f32 / 12.0 >= 28.0;
        // the ruler: when he dies, when his term ends, when a war is lost (a hero may take power)
        let lost_now = matches!(t.last_war, Some((y, Some(false))) if y < 1.0);
        let won_now = matches!(t.last_war, Some((y, Some(true))) if y < 1.0);
        let o = self.offices[c].clone();
        let mine = self.player == Some(c);
        // (a monarch or a dictator rules until he dies — a lost war unseats him less often than an
        // elected one)
        let falls = !mine && match o.ruler {
            None => true,
            Some(_) if lost_now && rng.f32() < if t.lifelong { 0.25 } else { 0.6 } => true,
            Some(_) if t.lifelong => false,
            Some(_) => month - o.ruler_since >= o.ruler_term,
        };
        if falls {
            let old = o.ruler;
            // (the one who held the seat last, alive or not: a crown passes to his son)
            let before = old.or_else(|| self.people.iter().filter(|p| p.country == c && p.posts.iter().any(|s| s.post == Post::Ruler)).max_by_key(|p| p.posts.iter().filter(|s| s.post == Post::Ruler).map(|s| s.to.unwrap_or(i32::MAX)).max().unwrap_or(i32::MIN)).map(|p| p.id));
            // candidates: statesmen; a famous general after a war (a hero takes power — or a coup)
            let mut cand: Vec<u32> = self.people.iter().filter(|p| p.country == c && p.alive() && p.calling == Calling::Statesman && adult(p) && Some(p.id) != old).map(|p| p.id).collect();
            if t.soldiers {
                let gens: Vec<u32> = self.people.iter().filter(|p| p.country == c && p.alive() && p.calling == Calling::General && adult(p) && Some(p.id) != old).map(|p| p.id).collect();
                if !gens.is_empty() {
                    cand = gens;
                }
            }
            // a crown: the old monarch's son (born, if the house has none grown — most houses go on)
            let mut heir = false;
            if t.crowned && !lost_now {
                if let Some(b) = before {
                    let sons: Vec<u32> = self.get(b).ties.iter().filter(|x| x.1 == Tie::Child).map(|x| x.0).filter(|&s| self.get(s).alive() && adult(self.get(s))).collect();
                    let son = match sons.first() {
                        Some(&s) => Some(s),
                        None if rng.f32() < 0.85 => {
                            let surname = self.get(b).name.split_once(' ').map_or(String::new(), |x| x.1.to_string());
                            let id = self.born(pack, c, t, month, Some(Calling::Statesman), 24.0 + 18.0 * rng.f32(), rng);
                            if !surname.is_empty() {
                                let initial = self.get(id).name.split_once(' ').map_or("А.".to_string(), |x| x.0.to_string());
                                self.people[id as usize].name = format!("{initial} {surname}");
                            }
                            self.tie(id, b, Tie::Child);
                            Some(id)
                        }
                        None => None,
                    };
                    if let Some(s) = son {
                        cand = vec![s];
                        heir = true;
                    }
                }
            }
            let heroes: Vec<u32> = self
                .people
                .iter()
                .filter(|p| p.country == c && p.alive() && p.calling == Calling::General && p.fame > 1.2 && adult(p) && Some(p.id) != old)
                .map(|p| p.id)
                .collect();
            if (lost_now || won_now) && !heroes.is_empty() && rng.f32() < 0.4 {
                cand = heroes.clone();
            }
            // (no statesmen at all: whoever is known best — a country may be ruled by a soldier or
            // a professor for want of politicians)
            if cand.is_empty() {
                cand = self.people.iter().filter(|p| p.country == c && p.alive() && adult(p) && p.fame > 0.2 && Some(p.id) != old).map(|p| p.id).collect();
            }
            // (a ruler whose term ends may stay on, if he is still there and nobody better comes)
            if let (Some(r), false) = (old, lost_now) {
                if self.get(r).alive() && rng.f32() < 0.3 {
                    self.offices[c].ruler_since = month;
                    self.offices[c].ruler_term = (6.0 + 10.0 * rng.f32()) as i32 * 12;
                    cand.clear();
                }
            }
            if let Some(new) = self.choose(c, &cand, rng) {
                let how = if self.get(new).calling == Calling::General {
                    if lost_now { "взял власть после поражения" } else { "герой войны стал во главе страны" }
                } else if lost_now {
                    "пришёл к власти после проигранной войны"
                } else {
                    "стал во главе страны"
                };
                if let Some(r) = old {
                    if self.get(r).alive() {
                        self.tie(new, r, Tie::Rival);
                        let ruler = &mut self.people[r as usize];
                        ruler.deeds.push(Deed { month, text: if lost_now { "ушёл после поражения в войне".into() } else { "оставил власть".into() } });
                    }
                }
                self.appoint(c, Post::Ruler, new, month);
                self.offices[c].ruler_since = month;
                self.offices[c].ruler_term = if t.term > 0.0 { (t.term * 12.0) as i32 * (1 + (rng.f32() < 0.4) as i32) } else { ((5.0 + 14.0 * rng.f32()) * 12.0) as i32 };
                let surname = self.get(new).name.split_once(' ').map_or(String::new(), |x| x.1.to_string());
                self.changed[c] = Some((surname, heir));
                let p = &mut self.people[new as usize];
                p.deed(month, how.to_string(), 0.6);
                let mood = if p.reform > 0.35 {
                    " — обещает реформы"
                } else if p.hawk > 0.45 {
                    " — грозит соседям"
                } else if p.hawk < -0.4 {
                    " — обещает мир"
                } else {
                    ""
                };
                let line = format!("{}: {how}{mood}", p.name);
                self.chronicle.push((month, c, line));
                // a new ruler may bring his own men
                let o = self.offices[c].clone();
                for (post, holder) in [(Post::Economy, o.economy), (Post::Science, o.science), (Post::Staff, o.staff)] {
                    if let Some(h) = holder {
                        if rng.f32() < 0.5 && !self.get(h).ties.iter().any(|t| t.0 == new) {
                            self.vacate(c, post, h, month);
                            self.people[h as usize].deeds.push(Deed { month, text: format!("отставлен новой властью ({})", post.word()) });
                        }
                    }
                }
            }
        }
        // the ministries
        for (post, calling) in [(Post::Economy, Calling::Economist), (Post::Science, Calling::Scientist), (Post::Staff, Calling::General)] {
            if self.holder(c, post).is_some() || (mine && post != Post::Staff) {
                continue;
            }
            let cand: Vec<u32> = self
                .people
                .iter()
                .filter(|p| p.country == c && p.alive() && p.calling == calling && adult(p) && p.post().is_none_or(|q| matches!(q, Post::Command | Post::Chair)))
                .map(|p| p.id)
                .collect();
            if let Some(id) = self.choose(c, &cand, rng) {
                self.appoint(c, post, id, month);
                let fame = if post == Post::Staff { 0.25 } else { 0.3 };
                self.people[id as usize].deed(month, format!("назначен: {}", post.word()), fame);
                // (the others who wanted it remember)
                for &o in cand.iter().filter(|&&o| o != id).take(1) {
                    if self.get(o).fame > 0.5 {
                        self.tie(o, id, Tie::Rival);
                    }
                }
            }
        }
        // generals in command and chairs of science: as many as the country is big
        let want = (2.0 + t.pop.sqrt() * 0.5).min(8.0) as usize;
        self.offices[c].command.retain(|&id| self.people[id as usize].alive());
        while self.offices[c].command.len() < want {
            let cand: Vec<u32> = self.people.iter().filter(|p| p.country == c && p.alive() && p.calling == Calling::General && adult(p) && p.post().is_none()).map(|p| p.id).collect();
            let Some(id) = self.choose(c, &cand, rng) else { break };
            self.appoint(c, Post::Command, id, month);
        }
        let chairs = (1.0 + t.dev * 5.0) as usize;
        self.offices[c].chairs.retain(|&id| self.people[id as usize].alive());
        while self.offices[c].chairs.len() < chairs {
            let cand: Vec<u32> = self.people.iter().filter(|p| p.country == c && p.alive() && p.calling == Calling::Scientist && adult(p) && p.post().is_none()).map(|p| p.id).collect();
            let Some(id) = self.choose(c, &cand, rng) else { break };
            self.appoint(c, Post::Chair, id, month);
            self.people[id as usize].deed(month, "получил кафедру".into(), 0.1);
        }
    }

    /// Generals for a war of country `c`: those in command first, then new men (who become people
    /// of the world). `k` of them, each with his id.
    pub fn war_generals(&mut self, pack: &Pack, c: usize, t: &Times, k: usize, month: i32, rng: &mut Rng) -> Vec<(u32, General)> {
        let mut ids: Vec<u32> = self.offices[c].command.iter().copied().filter(|&id| self.get(id).alive()).collect();
        let mut spare: Vec<u32> = self.people.iter().filter(|p| p.country == c && p.alive() && p.calling == Calling::General && !ids.contains(&p.id) && p.age(month) < 66.0).map(|p| p.id).collect();
        spare.sort_by(|&a, &b| self.get(b).fame.total_cmp(&self.get(a).fame));
        ids.extend(spare);
        while ids.len() < k {
            let age = 32.0 + 15.0 * rng.f32();
            let id = self.born(pack, c, t, month, Some(Calling::General), age, rng);
            ids.push(id);
        }
        ids.truncate(k);
        ids.into_iter().map(|id| (id, self.get(id).general())).collect()
    }

    /// A war of country `c` against `enemy` is over: its generals' records (as the war left them),
    /// fame for the victors, comrades, the fallen.
    pub fn after_war(&mut self, c: usize, generals: &[(u32, General)], won: Option<bool>, enemy: &str, month: i32, rng: &mut Rng) {
        let end = match won {
            Some(true) => "победа",
            Some(false) => "поражение",
            None => "мир без победителя",
        };
        let mut best: Option<(u32, i64)> = None;
        for &(id, ref g) in generals {
            let p = &mut self.people[id as usize];
            let (dw, dl, dt) = (g.won.saturating_sub(p.won), g.lost.saturating_sub(p.lost), g.took.saturating_sub(p.took));
            p.won = g.won;
            p.lost = g.lost;
            p.took = g.took;
            p.experience = g.experience.max(p.experience);
            p.wars += 1;
            let fame = 0.1 + 0.06 * dw as f32 - 0.03 * dl as f32 + 0.02 * dt as f32 + if won == Some(true) { 0.3 } else { 0.0 };
            p.deed(month, format!("воевал против {enemy} ({end}): побед {dw}, поражений {dl}, взял провинций {dt}"), fame.max(0.0));
            let score = dw as i64 * 2 + dt as i64 - dl as i64;
            if best.is_none_or(|b| score > b.1) {
                best = Some((id, score));
            }
        }
        // the hero of the war
        if let (Some((id, score)), Some(true)) = (best, won) {
            if score >= 4 {
                let p = &mut self.people[id as usize];
                p.deed(month, format!("герой войны против {enemy}"), 0.8);
                self.chronicle.push((month, c, format!("{} — герой войны против {enemy}", p.name)));
            }
        }
        // comrades of one war
        let ids: Vec<u32> = generals.iter().map(|g| g.0).collect();
        for i in 0..ids.len() {
            for j in i + 1..ids.len() {
                if rng.f32() < 0.3 {
                    self.tie(ids[i], ids[j], Tie::Comrade);
                }
            }
        }
        // some fall
        for &id in &ids {
            if rng.f32() < 0.05 {
                self.die(id, month, "пал на войне");
            }
        }
        // the beaten staff's chief goes
        if won == Some(false) {
            if let Some(s) = self.offices[c].staff {
                if rng.f32() < 0.7 {
                    self.vacate(c, Post::Staff, s, month);
                    self.people[s as usize].deeds.push(Deed { month, text: "снят после поражения".into() });
                }
            }
        }
    }

    /// The ruler of country `c` is overthrown (`how`: a revolution, a coup): a general takes his
    /// seat (`soldiers`), else a statesman who wants change (whoever is known, if none). The new
    /// ruler (none in the player's country: its government is his).
    #[allow(clippy::too_many_arguments)]
    pub fn overthrow(&mut self, pack: &Pack, c: usize, t: &Times, month: i32, how: &str, soldiers: bool, rng: &mut Rng) -> Option<u32> {
        if self.player == Some(c) {
            return None;
        }
        let old = self.offices[c].ruler;
        if let Some(r) = old {
            self.vacate(c, Post::Ruler, r, month);
            self.people[r as usize].deeds.push(Deed { month, text: format!("свергнут: {how}") });
        }
        let adult = |p: &Person| (month - p.born) as f32 / 12.0 >= 28.0;
        let calling = if soldiers { Calling::General } else { Calling::Statesman };
        let mut cand: Vec<u32> = self.people.iter().filter(|p| p.country == c && p.alive() && p.calling == calling && adult(p) && Some(p.id) != old && (soldiers || p.reform > 0.1)).map(|p| p.id).collect();
        if cand.is_empty() {
            cand = self.people.iter().filter(|p| p.country == c && p.alive() && adult(p) && Some(p.id) != old && p.fame > 0.1).map(|p| p.id).collect();
        }
        let new = match cand.iter().copied().max_by(|&a, &b| (self.get(a).fame + 0.3 * self.get(a).reform.abs()).total_cmp(&(self.get(b).fame + 0.3 * self.get(b).reform.abs()))) {
            Some(id) => id,
            None => self.born(pack, c, t, month, Some(calling), 35.0 + 15.0 * rng.f32(), rng),
        };
        self.appoint(c, Post::Ruler, new, month);
        self.offices[c].ruler_since = month;
        self.offices[c].ruler_term = if t.term > 0.0 { (t.term * 12.0) as i32 } else { ((8.0 + 14.0 * rng.f32()) * 12.0) as i32 };
        if let Some(r) = old {
            self.tie(new, r, Tie::Rival);
        }
        let p = &mut self.people[new as usize];
        p.deed(month, format!("пришёл к власти: {how}"), 0.8);
        let line = format!("{}: пришёл к власти ({how})", p.name);
        let surname = p.name.split_once(' ').map_or(String::new(), |x| x.1.to_string());
        self.chronicle.push((month, c, line));
        self.changed[c] = Some((surname, false));
        Some(new)
    }

    /// The government of country `c` lost an election: a statesman of the other camp (his belief
    /// in reform the other way from the old head's) takes the seat — none in the player's country.
    /// The new head.
    pub fn elect(&mut self, pack: &Pack, c: usize, t: &Times, month: i32, rng: &mut Rng) -> Option<u32> {
        if self.player == Some(c) {
            return None;
        }
        let old = self.offices[c].ruler;
        let side = old.map_or(0.0, |r| self.get(r).reform);
        if let Some(r) = old {
            self.vacate(c, Post::Ruler, r, month);
            self.people[r as usize].deeds.push(Deed { month, text: "проиграл выборы".into() });
        }
        let adult = |p: &Person| (month - p.born) as f32 / 12.0 >= 30.0;
        let other = |p: &Person| side == 0.0 || p.reform * side < 0.0;
        let cand: Vec<u32> = self.people.iter().filter(|p| p.country == c && p.alive() && p.calling == Calling::Statesman && adult(p) && Some(p.id) != old && other(p)).map(|p| p.id).collect();
        let new = match cand.iter().copied().max_by(|&a, &b| self.get(a).fame.total_cmp(&self.get(b).fame)) {
            Some(id) => id,
            None => self.born(pack, c, t, month, Some(Calling::Statesman), 40.0 + 15.0 * rng.f32(), rng),
        };
        self.appoint(c, Post::Ruler, new, month);
        self.offices[c].ruler_since = month;
        self.offices[c].ruler_term = if t.term > 0.0 { (t.term * 12.0) as i32 } else { 48 };
        if let Some(r) = old {
            self.tie(new, r, Tie::Rival);
        }
        let p = &mut self.people[new as usize];
        p.deed(month, "победил на выборах".into(), 0.5);
        let line = format!("{}: победил на выборах", p.name);
        let surname = p.name.split_once(' ').map_or(String::new(), |x| x.1.to_string());
        self.chronicle.push((month, c, line));
        self.changed[c] = Some((surname, false));
        Some(new)
    }

    /// How warlike the government of country `c` is now (its ruler's belief, his chief of staff's a
    /// little): −1 … +1.
    pub fn hawk(&self, c: usize) -> f32 {
        let r = self.offices[c].ruler.map_or(0.0, |r| self.get(r).hawk);
        let s = self.offices[c].staff.map_or(0.0, |s| self.get(s).hawk);
        (0.8 * r + 0.2 * s).clamp(-1.0, 1.0)
    }

    /// How country `c`'s government leans in choosing what its science works on: by its ruler's
    /// beliefs and its head of science's love.
    pub fn leaning(&self, pack: &Pack, c: usize) -> crate::world::progress::Leaning {
        let mut l = crate::world::progress::Leaning::even(pack);
        if let Some(r) = self.offices[c].ruler {
            let r = self.get(r);
            l.sphere[1] *= 1.0 + 0.8 * r.hawk.max(0.0);
            l.sphere[0] *= 1.0 + 0.6 * r.state.max(0.0) + 0.3 * (-r.hawk).max(0.0);
            l.sphere[2] *= 1.0 + 0.7 * r.reform.max(0.0);
            l.sphere[3] *= 1.0 + 0.4 * r.reform.abs();
        }
        if let Some(s) = self.offices[c].science {
            let s = self.get(s);
            if s.love < pack.units.len() {
                l.kinds[s.love] *= 2.0;
                l.sphere[1] *= 1.3;
            } else {
                l.sphere[0] *= 1.3;
            }
        }
        if let Some(s) = self.offices[c].staff {
            let s = self.get(s);
            if s.love < pack.units.len() {
                l.kinds[s.love] *= 1.5;
            }
        }
        l
    }
}
