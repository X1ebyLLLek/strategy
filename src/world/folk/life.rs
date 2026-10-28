//! How the world's people live and die: who is born into which country and class, schooled by whom,
//! what his youth taught him; deaths; a year of everyone.

use super::*;

/// How many notable people a country of `pop` millions brings forth a year.
fn births(pop: f32, dev: f32) -> f32 {
    0.22 + 0.05 * pop.max(0.0).sqrt() + 0.25 * dev
}

/// Chance of dying within a year at `age` (in peace).
fn mortality(age: f32) -> f32 {
    (0.0025 * ((age - 30.0) / 10.5).exp()).min(0.6)
}

/// An index by weights.
fn pick(w: &[f32], rng: &mut Rng) -> usize {
    let all: f32 = w.iter().sum();
    let mut r = rng.f32() * all;
    for (i, &x) in w.iter().enumerate() {
        r -= x;
        if r <= 0.0 {
            return i;
        }
    }
    w.len() - 1
}

impl Folk {
    /// A new person of country `c` comes of age in month `month` (born twenty-odd years before):
    /// his class, his schooling, his beliefs from the times, his gifts by luck. His id.
    pub fn born(&mut self, pack: &Pack, c: usize, t: &Times, month: i32, calling: Option<Calling>, age: f32, rng: &mut Rng) -> u32 {
        let id = self.people.len() as u32;
        // his calling: a country at war breeds soldiers, a developed one scientists
        let calling = calling.unwrap_or_else(|| {
            let war = match t.last_war {
                Some((y, _)) if y < 20.0 => 0.6,
                _ => 0.0,
            };
            let w = [1.0 + war + if t.at_war { 0.5 } else { 0.0 }, 0.5 + 1.8 * t.dev, 0.45 + 0.9 * t.dev, 0.8];
            Calling::ALL[pick(&w, rng)]
        });
        // his family: the names a country remembers come more often from above
        let origin = pick(&[(0.7 - 0.4 * t.dev).max(0.1), 0.25 + 0.25 * t.dev, 0.25 + 0.5 * t.dev, 0.35], rng) as u8;
        // his schooling: his class and his country's, and a teacher if a great one takes him
        let base = [0.18, 0.3, 0.6, 0.55][origin as usize];
        let mut education = base + 0.3 * t.dev + 0.14 * rng.normal();
        let teacher = self.teacher(c, calling, rng);
        if let Some(tid) = teacher {
            education += 0.25 * self.get(tid).education;
        }
        let education = education.clamp(0.02, 1.0);
        // his gift: luck alone (now and then a genius)
        let mut talent = (0.48 + 0.17 * rng.normal()).clamp(0.03, 0.9);
        if rng.f32() < 0.035 {
            talent = 0.92 + 0.08 * rng.f32();
        }
        // his beliefs: what the years of his youth taught him
        let (lost, won) = match t.last_war {
            Some((y, Some(false))) if y < 25.0 => (1.0 - y / 25.0, 0.0),
            Some((y, Some(true))) if y < 25.0 => (0.0, 1.0 - y / 25.0),
            _ => (0.0, 0.0),
        };
        let poor = (0.45 - t.dev).max(0.0);
        let reform = (0.3 * rng.normal() + 0.45 * lost - 0.25 * won + 0.3 * poor + if origin == 2 { 0.15 } else { 0.0 }).clamp(-1.0, 1.0);
        let hawk = (0.35 * rng.normal() + 0.3 * lost + 0.15 * won + if t.grudge { 0.35 } else { 0.0 } + if calling == Calling::General { 0.2 } else { 0.0 } - if calling == Calling::Economist { 0.1 } else { 0.0 }).clamp(-1.0, 1.0);
        let state = (0.38 * rng.normal() + [0.25, 0.2, 0.0, -0.35][origin as usize] + 0.4 * poor + 0.15 * lost).clamp(-1.0, 1.0);
        let honesty = (1.0 - rng.f32().powi(2) * (1.0 - 0.3 * education)).clamp(0.0, 1.0);
        let bold = (0.45 * rng.normal() + 0.25 * hawk).clamp(-1.0, 1.0);
        // the sphere he loves: a soldier the arm his country is proud of (or any), a scientist any
        let nu = pack.units.len();
        let love = match calling {
            Calling::General if !t.glory.is_empty() && rng.f32() < 0.6 => (0..nu).max_by(|&a, &b| t.glory[a].total_cmp(&t.glory[b])).unwrap_or(0),
            Calling::Economist | Calling::Statesman => nu,
            _ => rng.below(nu + 1),
        };
        // a son of a famous father, now and then: his name, his calling
        let mut name = names::person(rng);
        let father = self.people.iter().filter(|p| p.country == c && p.calling == calling && p.fame > 1.0 && month - p.born > 45 * 12 && month - p.born < 75 * 12).map(|p| p.id).max_by_key(|&id| (self.get(id).fame * 100.0) as i64);
        let mut son_of = None;
        if let Some(f) = father {
            if rng.f32() < 0.12 {
                let surname = self.get(f).name.split_once(' ').map_or(String::new(), |x| x.1.to_string());
                let initial = name.split_once(' ').map_or("А.".to_string(), |x| x.0.to_string());
                if !surname.is_empty() {
                    name = format!("{initial} {surname}");
                    son_of = Some(f);
                }
            }
        }
        let born = month - (age * 12.0) as i32;
        self.people.push(Person {
            id,
            name,
            country: c,
            born,
            died: None,
            origin,
            calling,
            talent,
            education,
            experience: (0.05 + 0.02 * (age - 20.0).max(0.0)).min(0.7),
            honesty,
            state,
            hawk,
            reform,
            bold,
            love,
            fame: 0.0,
            posts: Vec::new(),
            deeds: Vec::new(),
            ties: Vec::new(),
            won: 0,
            lost: 0,
            took: 0,
            wars: 0,
            works: 0,
            end: String::new(),
        });
        if let Some(tid) = teacher {
            self.tie(id, tid, Tie::Pupil);
        }
        if let Some(f) = son_of {
            self.tie(id, f, Tie::Child);
            self.people[id as usize].fame += 0.15;
            // (a famous name opens doors)
            self.people[id as usize].education = (self.people[id as usize].education + 0.1).min(1.0);
        }
        id
    }

    /// A great man of `calling` in country `c` who takes a pupil, if one does.
    pub(super) fn teacher(&self, c: usize, calling: Calling, rng: &mut Rng) -> Option<u32> {
        if !matches!(calling, Calling::Scientist | Calling::Economist | Calling::General) {
            return None;
        }
        let masters: Vec<u32> = self.people.iter().filter(|p| p.country == c && p.alive() && p.calling == calling && p.fame > 0.8 && p.education > 0.6).map(|p| p.id).collect();
        if masters.is_empty() || rng.f32() > 0.35 {
            return None;
        }
        Some(masters[rng.below(masters.len())])
    }

    /// Person `id` dies in month `month` (`how`, in words): his posts fall vacant.
    pub fn die(&mut self, id: u32, month: i32, how: &str) {
        let c = self.people[id as usize].country;
        let posts: Vec<Post> = self.people[id as usize].posts.iter().filter(|s| s.to.is_none()).map(|s| s.post).collect();
        for p in posts {
            self.vacate(c, p, id, month);
        }
        let p = &mut self.people[id as usize];
        p.died = Some(month);
        p.end = how.to_string();
        if p.fame >= 1.0 {
            let line = format!("умер {} — {}, {}", p.name, p.calling.word(), p.fame_word());
            self.chronicle.push((month, c, line));
        }
    }

    /// A year of country `c`'s people: new ones come of age, the old die, posts are filled, those in
    /// office do what they do: scientists find things out (research for `know`, a genius sometimes
    /// a breakthrough), economists better or worsen the economy and may rob it, the ruler reforms.
    /// What its scientists add to its research this year (years of an ordinary country's science).
    pub fn year(&mut self, pack: &Pack, c: usize, t: &Times, month: i32, know: &mut Know, seen: &dyn Fn(usize) -> f32, rng: &mut Rng) -> f32 {
        // the newly come of age
        let lam = births(t.pop, t.dev);
        let mut k = lam.floor() as usize;
        if rng.f32() < lam.fract() {
            k += 1;
        }
        for _ in 0..k {
            let age = 21.0 + 6.0 * rng.f32();
            self.born(pack, c, t, month, None, age, rng);
        }
        // the old die (the dead do nothing more)
        let living: Vec<u32> = self.people.iter().filter(|p| p.country == c && p.alive()).map(|p| p.id).collect();
        for &id in &living {
            let age = self.get(id).age(month);
            if rng.f32() < mortality(age) {
                let how = if age > 70.0 { "от старости" } else { "от болезни" };
                self.die(id, month, how);
            }
        }
        self.fill(pack, c, t, month, rng);
        let living: Vec<u32> = self.people.iter().filter(|p| p.country == c && p.alive()).map(|p| p.id).collect();
        let mut research = 0.0f32;
        for &id in &living {
            let p = self.get(id).clone();
            let post = p.post();
            // experience comes with the post, and slowly without
            let pace = if post.is_some() { 1.0 / 12.0 } else { 1.0 / 40.0 };
            self.people[id as usize].experience += pace * (1.0 - p.experience);
            match p.calling {
                Calling::Scientist => {
                    // a discovery: the gifted, the schooled, those with a chair and laboratories
                    let chair = matches!(post, Some(Post::Science) | Some(Post::Chair));
                    let chance = (0.02 + 0.3 * p.talent.powi(3) * (0.4 + p.education)) * (0.5 + t.dev) * if chair { 1.5 } else { 0.7 };
                    if rng.f32() < chance {
                        let open = know.open(pack);
                        // (on what he loves, if anything there is open)
                        let mine: Vec<usize> = open.iter().copied().filter(|&tt| loves(pack, tt, p.love)).collect();
                        let pool = if mine.is_empty() { open.clone() } else { mine };
                        if let Some(&tt) = pool.get(rng.below(pool.len().max(1)).min(pool.len().saturating_sub(1))) {
                            let genius = p.talent > 0.9;
                            if genius && rng.f32() < 0.45 {
                                know.learn(tt, month, Some(id));
                                let q = &mut self.people[id as usize];
                                q.works += 1;
                                q.deed(month, format!("сделал открытие: {}", pack.techs[tt].name), 1.0);
                                self.chronicle.push((month, c, format!("{}: открытие — {}", q.name, pack.techs[tt].name)));
                            } else {
                                let pts = 0.5 + 2.5 * p.talent * p.education;
                                know.progress[tt] += pts;
                                if know.progress[tt] >= Know::cost(pack, tt, seen(tt)) {
                                    know.learn(tt, month, Some(id));
                                    let q = &mut self.people[id as usize];
                                    q.works += 1;
                                    q.deed(month, format!("завершил работу: {}", pack.techs[tt].name), 0.6);
                                } else {
                                    let q = &mut self.people[id as usize];
                                    q.deed(month, format!("его труды двинули вперёд: {}", pack.techs[tt].name), 0.25);
                                }
                            }
                        }
                    }
                    // a scientist heading science adds to the country's research by his ability
                    if post == Some(Post::Science) {
                        research += 0.25 * (p.talent * (0.5 + p.education) - 0.35);
                        if p.honesty < 0.4 {
                            self.taken[c] += 0.05 * (1.0 - p.honesty);
                        }
                    }
                }
                Calling::Economist if post == Some(Post::Economy) => {
                    // his year over the economy: the able better it, the dogmatic in the wrong place
                    // harm it, the dishonest rob it
                    let able = p.talent * (0.4 + p.education) + 0.2 * p.experience - 0.45;
                    let fit = if t.dev < 0.4 { p.state } else { -p.state } * 0.3 * p.state.abs();
                    let luck = 0.01 * rng.normal();
                    let gain = 0.006 * able + 0.003 * fit + luck * (1.0 - p.experience);
                    self.econ[c] += gain;
                    let took = 0.25 * (1.0 - p.honesty).powi(2);
                    self.taken[c] += took;
                    let years = (month - p.posts.iter().rev().find(|s| s.post == Post::Economy).map_or(month, |s| s.from)) / 12;
                    if years > 0 && years % 4 == 0 {
                        let line = if gain > 0.006 {
                            "провёл удачную хозяйственную реформу"
                        } else if gain < -0.004 {
                            "его меры привели к кризису"
                        } else {
                            "правил хозяйством без потрясений"
                        };
                        self.people[id as usize].deed(month, line.into(), if gain > 0.006 { 0.5 } else { 0.1 });
                    }
                    // a thief may be found out
                    if took > 0.08 && rng.f32() < 0.15 {
                        self.people[id as usize].deed(month, "уличён в казнокрадстве и смещён".into(), -0.2);
                        self.vacate(c, Post::Economy, id, month);
                        self.chronicle.push((month, c, format!("{}: казнокрадство раскрыто", p.name)));
                    }
                }
                _ => {}
            }
            if post == Some(Post::Ruler) {
                // the ruler: schools if he believes in reform, the treasury if he steals
                if p.reform > 0.3 && p.honesty > 0.5 {
                    self.schools[c] += 0.004 * p.reform;
                    if rng.f32() < 0.08 {
                        self.people[id as usize].deed(month, "открыл школы и университеты".into(), 0.3);
                    }
                }
                if p.honesty < 0.4 {
                    self.taken[c] += 0.15 * (1.0 - p.honesty);
                }
                // years in power make a name
                self.people[id as usize].fame += 0.05;
            }
        }
        research
    }
}
