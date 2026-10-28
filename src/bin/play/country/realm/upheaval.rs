//! What breaks: a revolt, an upheaval as an event with its choices, the bot's answer, a reform or a
//! change of the form of government, what the law allows.

use super::*;

/// What a favour bought of a force costs side `s` (a share of the state's monthly income).
pub fn favour_cost(g: &Game, s: usize) -> f32 {
    1.5 * g.income(s).max(0.0)
}

/// Months a reform of `law` to level `to` takes.
pub fn reform_months(law: usize, to: u8) -> u32 {
    let base = match law {
        polity::LABOUR | polity::LAND => 18,
        polity::SCHOOLS => 24,
        polity::SERVICE => 12,
        _ => 8,
    };
    base + 4 * to as u32
}

/// The form a revolution led by movement `by` gives country `p`.
pub fn revolution_to(p: &polity::Polity, by: usize) -> Regime {
    use polity::*;
    match by {
        LIBERALS if p.regime.crowned() && p.piety > 0.5 => Regime::Constitutional,
        LIBERALS => Regime::Republic,
        LEVELLERS if p.industry > 0.35 => Regime::Party,
        LEVELLERS => Regime::Republic,
        ZEALOTS => Regime::Theocracy,
        GUARDIANS => Regime::Autocracy,
        _ => Regime::Junta,
    }
}

/// The chance a government with order `p` falls this month if it does nothing.
pub fn revolution_chance(p: &polity::Polity) -> f32 {
    ((p.unrest - 0.4) * 1.2 * (1.0 - p.legit)).clamp(0.02, 0.8)
}

impl Game {
    /// Side `s`'s government falls to movement `by`, and takes form `to`: the new laws, the news,
    /// a new ruler (the rival's: a person of the world). What should stop the clock.
    pub fn revolt(&mut self, s: usize, to: Regime, by: usize) -> Vec<String> {
        let c = self.country(s);
        let name = self.h.home(s).name.clone();
        let now = self.month();
        let mut r = Rng::new(hash64(self.seed ^ 0x9E70 ^ (now as u64) << 3 ^ s as u64));
        let from = self.polity.of[c].regime;
        self.polity.change(c, &name, to, by, now as i32, &mut r);
        self.realm.pending[s] = None;
        self.realm.changed[s] = now;
        self.realm.election[s] = to.elected().map_or(0, |y| now + (y * 12.0) as u32);
        if s != self.me {
            let t = self.times_now(c);
            let pack = self.pack.clone();
            if let Some(id) = self.folk.overthrow(&pack, c, &t, now as i32, if to == Regime::Junta { "военный переворот" } else { "революция" }, to.soldiers(), &mut r) {
                let nm = self.folk.get(id).name.clone();
                self.polity.crown(c, &nm, now as i32);
            }
        } else {
            self.polity.crown(c, &strategy::world::names::person(&mut r), now as i32);
        }
        // (a revolution shakes the government and the owners)
        self.state.mods[s] -= 0.15;
        self.fiscal.confidence[s] = (self.fiscal.confidence[s] - 0.2).max(0.3);
        let line = if to == Regime::Junta {
            format!("военный переворот: {} {}, власть у генералов", from.name().to_lowercase(), strategy::world::names::agree(from.name(), "пала"))
        } else {
            format!("революция ({}): {} {} — теперь {}", MOVE[by].name.to_lowercase(), from.name().to_lowercase(), strategy::world::names::agree(from.name(), "пала"), to.name().to_lowercase())
        };
        let text = if s == self.me { line.clone() } else { format!("{name}: {line}") };
        self.news.push(Note::new(now, &text, true, "society"));
        if s == self.me {
            vec![line]
        } else {
            Vec::new()
        }
    }

    /// The order's hardest moment of side `s` now, if any (an event).
    pub fn realm_happens(&self, s: usize) -> Option<Ev> {
        use polity::*;
        let c = self.country(s);
        let p = &self.polity.of[c];
        let now = self.month();
        let h = hash64(self.seed ^ 0x9EA ^ (now as u64) << 5 ^ s as u64);
        let roll = (h % 1000) as f32 / 1000.0;
        if p.unrest > RESTLESS && p.legit < 0.5 {
            let (by, _) = [LIBERALS, LEVELLERS, ZEALOTS, GUARDIANS, HAWKS].iter().map(|&m| (m, p.moves[m])).max_by(|a, b| a.1.total_cmp(&b.1)).unwrap_or((LIBERALS, 0.0));
            return Some(Ev::Revolution(by));
        }
        if p.regime != Regime::Junta && p.clout[OFFICERS] > 0.12 && p.mood[OFFICERS] < -0.35 && self.realm.purge[s] == 0 {
            return Some(Ev::Officers);
        }
        if p.regime.elected().is_some() && self.realm.election[s] > 0 && now >= self.realm.election[s] {
            return Some(Ev::Election);
        }
        if p.moves[SEPARATISTS] > 0.5 && roll < 0.3 {
            return Some(Ev::Separatists);
        }
        if self.realm.pending[s].is_none() {
            for m in [LIBERALS, LEVELLERS, ZEALOTS, HAWKS] {
                if p.moves[m] < 0.45 {
                    continue;
                }
                let closed = matches!(p.regime, Regime::Party | Regime::Junta | Regime::Theocracy);
                if let Some(&(law, want)) = MOVE[m].aims.iter().find(|&&(law, want)| p.laws[law] != want && !(closed && (law == PRESS || law == VOICE) && want > p.laws[law])) {
                    let to = if want > p.laws[law] { p.laws[law] + 1 } else { p.laws[law] - 1 };
                    if roll < 0.35 {
                        return Some(Ev::Demand(m, law, to));
                    }
                }
            }
        }
        let tenets = self.polity.faiths.get(p.state_faith).map_or([0.0; strategy::world::polity::faith::TENETS], |f| f.tenets);
        if p.piety > 0.3 && p.piety < 0.85 && p.literacy > 0.22 && p.mood[CLERGY] < 0.1 && p.clout[LEARNED] + p.clout[MERCHANTS] > 0.18 && tenets[strategy::world::polity::faith::HIERARCHY] > -0.2 && roll < 0.0025 {
            return Some(Ev::Schism);
        }
        None
    }

    /// The order's event in words, with its choices.
    pub fn realm_event(&self, s: usize, e: Ev) -> (String, String, Vec<(String, String)>) {
        use polity::*;
        let p = self.polity_of(s);
        match e {
            Ev::Revolution(by) => {
                let to = revolution_to(p, by);
                let (law, lv) = MOVE[by].aims.iter().copied().find(|&(l, w)| p.laws[l] != w).unwrap_or((VOICE, 2));
                (
                    format!("Революция у порога: {}", MOVE[by].name.to_lowercase()),
                    format!(
                        "Смута {:.0}%, законность власти {:.0}%. {} поднимают людей: если власть падёт, страна станет — {}.",
                        p.unrest * 100.0,
                        p.legit * 100.0,
                        MOVE[by].name,
                        to.name().to_lowercase()
                    ),
                    vec![
                        ("Твёрдая рука".into(), "год жёсткого порядка: смута быстро спадает, интеллигенция и рабочие злы, офицеры довольны; законность −5".into()),
                        (format!("Уступить: {}", LAW[law].levels[lv as usize]), format!("{} — сразу; {} притихнут", LAW[law].name, MOVE[by].name.to_lowercase())),
                        ("Ничего не делать".into(), format!("шанс переворота сейчас ~{:.0}%", revolution_chance(p) * 100.0)),
                    ],
                )
            }
            Ev::Officers => (
                "Заговор офицеров".into(),
                format!("Офицерство (сила {:.0}%, настроение {:+.0}) ропщет: деньги, служба, поражения. В штабах говорят о «спасении отечества».", p.clout[OFFICERS] * 100.0, p.mood[OFFICERS] * 100.0),
                vec![
                    ("Деньги армии".into(), "доля армии в тратах +8 пунктов; офицеры довольны".into()),
                    ("Чистка офицеров".into(), "заговор раскрыт, но год армия слабее (атака −7%, оборона −5%); офицеры злы".into()),
                    ("Ничего не делать".into(), "шанс переворота ~30%".into()),
                ],
            ),
            Ev::Demand(m, law, to) => (
                format!("{} требуют: {}", MOVE[m].name, LAW[law].levels[to as usize]),
                format!("Движение «{}» (поддержка {:.0}%) требует: {} — «{}» вместо «{}».", MOVE[m].name, p.moves[m] * 100.0, LAW[law].name.to_lowercase(), LAW[law].levels[to as usize], p.law_word(law)),
                vec![
                    ("Начать реформу".into(), format!("реформа займёт {} мес.; те, кому она по нраву, довольны уже сейчас", reform_months(law, to))),
                    ("Обещать позже".into(), "движение притихнет на время; законность −3".into()),
                    ("Отказать".into(), "движение крепнет, смута растёт".into()),
                ],
            ),
            Ev::Schism => {
                let fa = self.polity.faiths.get(p.state_faith).map_or(String::new(), |f| f.name.clone());
                (
                    "Раскол в церкви".into(),
                    format!("Учёные богословы и купцы требуют очистить «{fa}» от власти священства. Половина приходов готова уйти за ними."),
                    vec![
                        ("Признать новую церковь".into(), "государственная вера — новый толк; духовенство в ярости, интеллигенция и купцы довольны".into()),
                        ("Терпимость".into(), "вера и государство: терпимость; обе церкви живут рядом".into()),
                        ("Подавить".into(), "раскольников преследуют: духовенство довольно, отделенцы и вольнодумцы растут".into()),
                    ],
                )
            }
            Ev::Election => {
                let coalition: Vec<&str> = p.coalition().iter().map(|&b| BLOC[b].name).collect();
                let share = self.vote_share(s).unwrap_or(0.5);
                let econ = self.economy_vote(s);
                let won = share >= 0.5;
                let why = if econ < -0.05 { "цены и безработица против правительства" } else if econ > 0.05 { "рост и работа за правительство" } else { "хозяйство ни за, ни против" };
                (
                    "Выборы".into(),
                    format!(
                        "Страна выбрала: за правительство {:.0}% мест ({why}). Сильнейшие в собрании — {}. {}",
                        share * 100.0,
                        coalition.join(", ").to_lowercase(),
                        if won { "Большинство за нами." } else { "Большинство у оппозиции: кабинет должен уйти." }
                    ),
                    if won {
                        vec![
                            ("Принять волю собрания".into(), "законность +8; силы большинства довольнее".into()),
                            ("Распустить собрание".into(), "законность −15; вольнодумцы крепнут; правительство остаётся".into()),
                        ]
                    } else {
                        vec![
                            ("Уступить кабинет оппозиции".into(), "законность +5; власть меняется по закону (у бота — новый глава правительства)".into()),
                            ("Распустить собрание".into(), "законность −20; вольнодумцы крепнут; правительство остаётся вопреки выборам".into()),
                        ]
                    },
                )
            }
            Ev::Separatists => (
                "Области хотят уйти".into(),
                format!("Отделенцы (поддержка {:.0}%) поднимают взятые у соседей и иноверные области.", p.moves[SEPARATISTS] * 100.0),
                vec![
                    ("Дать автономию".into(), "отделенцев вдвое меньше; законность −5".into()),
                    ("Ввести войска".into(), "отделенцы −20%, смута выше".into()),
                    ("Ничего не делать".into(), "пусть шумят".into()),
                ],
            ),
        }
    }

    /// What a bot government chooses in the order's event.
    pub fn realm_bot(&self, s: usize, e: Ev) -> usize {
        use polity::*;
        let p = self.polity_of(s);
        let c = self.country(s);
        let reform = self.folk.offices[c].ruler.map_or(0.0, |r| self.folk.get(r).reform);
        let n = self.h.home(s);
        match e {
            Ev::Revolution(_) => {
                if p.regime.repression() >= 0.5 && p.mood[OFFICERS] > -0.1 {
                    0
                } else {
                    1
                }
            }
            Ev::Officers => {
                if n.treasury > 0.05 * n.gdp.max(1e-6) {
                    0
                } else {
                    1
                }
            }
            Ev::Demand(..) => {
                if reform > 0.15 {
                    0
                } else if reform > -0.2 {
                    1
                } else {
                    2
                }
            }
            Ev::Schism => {
                let tol = self.polity.faiths.get(p.state_faith).map_or(0.0, |f| f.tenets[faith::TOLERANCE]);
                if tol > 0.2 {
                    1
                } else if reform > 0.3 {
                    0
                } else {
                    2
                }
            }
            Ev::Election => 0,
            Ev::Separatists => {
                if self.h.at_war {
                    1
                } else {
                    0
                }
            }
        }
    }

    /// What a choice in the order's event does.
    pub fn realm_choose(&mut self, s: usize, e: Ev, k: usize) {
        use polity::*;
        let c = self.country(s);
        let now = self.month();
        let name = self.h.home(s).name.clone();
        let mut r = Rng::new(hash64(self.seed ^ 0x9ECC ^ (now as u64) << 4 ^ (s as u64) << 1 ^ k as u64));
        match (e, k) {
            (Ev::Revolution(_), 0) => {
                self.realm.crackdown[s] = 12;
                let p = &mut self.polity.of[c];
                p.unrest *= 0.7;
                p.legit = (p.legit - 0.05).max(0.05);
            }
            (Ev::Revolution(by), 1) => {
                let p = &self.polity.of[c];
                if let Some(&(law, want)) = MOVE[by].aims.iter().find(|&&(l, w)| p.laws[l] != w) {
                    let to = if want > p.laws[law] { p.laws[law] + 1 } else { p.laws[law] - 1 };
                    self.polity.reform(c, &name, law, to, Some(by), now as i32);
                }
                let p = &mut self.polity.of[c];
                p.unrest *= 0.75;
                p.moves[by] *= 0.7;
            }
            (Ev::Revolution(by), _) => {
                let p = &self.polity.of[c];
                if r.f32() < revolution_chance(p) {
                    let to = revolution_to(p, by);
                    let lines = self.revolt(s, to, by);
                    if s == self.me {
                        self.alert = lines.first().cloned().or(self.alert.take());
                    }
                }
            }
            (Ev::Officers, 0) => {
                let x = (self.h.home(s).mil_share + 0.08).min(0.9);
                self.h.army_share[s] = Some(x);
                self.h.home_mut(s).mil_share = x;
                self.realm.favour[s][OFFICERS] += 0.3;
            }
            (Ev::Officers, 1) => {
                self.realm.purge[s] = 12;
                let p = &mut self.polity.of[c];
                p.clout[OFFICERS] *= 0.7;
            }
            (Ev::Officers, _) => {
                if r.f32() < 0.3 {
                    let lines = self.revolt(s, Regime::Junta, HAWKS);
                    if s == self.me {
                        self.alert = lines.first().cloned().or(self.alert.take());
                    }
                }
            }
            (Ev::Demand(_, law, to), 0) => {
                self.realm.pending[s] = Some(Pending { law, to, done: 0, months: reform_months(law, to) });
            }
            (Ev::Demand(m, ..), 1) => {
                let p = &mut self.polity.of[c];
                p.moves[m] *= 0.8;
                p.legit = (p.legit - 0.03).max(0.05);
            }
            (Ev::Demand(m, ..), _) => {
                let p = &mut self.polity.of[c];
                p.moves[m] = (p.moves[m] + 0.1).min(1.0);
                p.unrest = (p.unrest + 0.05).min(1.0);
            }
            (Ev::Schism, 0) => {
                let regions: Vec<usize> = self.realm.regions.get(c).map_or(Vec::new(), |v| v.iter().map(|x| x.0).collect());
                let cap = self.capitals.get(c).copied().filter(|&p| p != usize::MAX).map_or(usize::MAX, |p| self.prov_state[p] as usize);
                let founder = strategy::world::names::person(&mut r);
                let old = self.polity.of[c].state_faith;
                let f = self.polity.split(c, &regions, cap, false, now as i32, founder, &mut r);
                let line = format!("{name}: раскол — «{}» отделился от «{}» и стал государственной верой", self.polity.faiths[f].name, self.polity.faiths[old].name);
                self.polity.chronicle.push((now as i32, c, line.clone()));
                let p = &mut self.polity.of[c];
                p.story.push((now as i32, line));
                p.mood[CLERGY] = (p.mood[CLERGY] - 0.5).max(-1.0);
                p.mood[LEARNED] = (p.mood[LEARNED] + 0.2).min(1.0);
                p.mood[MERCHANTS] = (p.mood[MERCHANTS] + 0.2).min(1.0);
            }
            (Ev::Schism, 1) => {
                if self.polity.of[c].laws[CHURCH] == 0 {
                    self.polity.reform(c, &name, CHURCH, 1, None, now as i32);
                }
            }
            (Ev::Schism, _) => {
                let p = &mut self.polity.of[c];
                p.moves[SEPARATISTS] = (p.moves[SEPARATISTS] + 0.1).min(1.0);
                p.moves[LIBERALS] = (p.moves[LIBERALS] + 0.1).min(1.0);
                p.mood[CLERGY] = (p.mood[CLERGY] + 0.2).min(1.0);
                p.mood[LEARNED] = (p.mood[LEARNED] - 0.2).max(-1.0);
            }
            (Ev::Election, 0) => {
                let won = self.vote_share(s).unwrap_or(0.5) >= 0.5;
                let p = &mut self.polity.of[c];
                p.legit = (p.legit + if won { 0.08 } else { 0.05 }).min(1.0);
                for b in p.coalition() {
                    self.realm.favour[s][b] += 0.15;
                }
                self.realm.election[s] = now + (self.polity.of[c].regime.elected().unwrap_or(4.0) * 12.0) as u32;
                // a lost election: the opposition's man heads the government (the rival's)
                if !won && s != self.me {
                    let t = self.times_now(c);
                    let pack = self.pack.clone();
                    if let Some(id) = self.folk.elect(&pack, c, &t, now as i32, &mut r) {
                        let who = self.folk.get(id).name.clone();
                        self.news.push(Note::new(now, &format!("{name}: правительство проиграло выборы, кабинет возглавил {who}"), false, "politics"));
                    }
                }
            }
            (Ev::Election, _) => {
                let won = self.vote_share(s).unwrap_or(0.5) >= 0.5;
                let p = &mut self.polity.of[c];
                p.legit = (p.legit - if won { 0.15 } else { 0.2 }).max(0.05);
                p.moves[LIBERALS] = (p.moves[LIBERALS] + 0.15).min(1.0);
                self.realm.election[s] = now + (self.polity.of[c].regime.elected().unwrap_or(4.0) * 12.0) as u32;
            }
            (Ev::Separatists, 0) => {
                let p = &mut self.polity.of[c];
                p.moves[SEPARATISTS] *= 0.5;
                p.legit = (p.legit - 0.05).max(0.05);
            }
            (Ev::Separatists, 1) => {
                let p = &mut self.polity.of[c];
                p.moves[SEPARATISTS] *= 0.8;
                p.unrest = (p.unrest + 0.06).min(1.0);
            }
            _ => {}
        }
        self.realm.moods[s] = self.moods_now(s);
    }

    /// The player's decisions of the order: a reform (`reform=law:level`), the iron hand
    /// (`crackdown`), a force's favour bought (`favour=b`), a new form from above (`regime=k`).
    pub fn realm_form(&mut self, s: usize, form: &[(String, String)]) {
        let get = |k: &str| form.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
        let now = self.month();
        let c = self.country(s);
        if let Some(v) = get("reform") {
            if let Some((l, to)) = v.split_once(':') {
                if let (Ok(law), Ok(to)) = (l.trim().parse::<usize>(), to.trim().parse::<u8>()) {
                    if law < LAWS && to <= 2 && self.reform_allowed(s, law, to).is_ok() {
                        // where there is a chamber, it votes; a reform it rejects goes by decree —
                        // slower, and the government pays for it in legitimacy
                        let vote = self.reform_vote(s, law, to);
                        let decree = vote.is_some_and(|v| v < 0.5);
                        let months = if decree { reform_months(law, to) * 3 / 2 } else { reform_months(law, to) };
                        self.realm.pending[s] = Some(Pending { law, to, done: 0, months });
                        let p = &mut self.polity.of[c];
                        p.legit = (p.legit - if decree { 0.09 } else { 0.03 }).max(0.05);
                        let what = format!("{}: {}", LAW[law].name.to_lowercase(), LAW[law].levels[to as usize]);
                        let line = match vote {
                            Some(v) if decree => format!("собрание отвергло реформу ({what}) — {:.0}% за; проводим указом", v * 100.0),
                            Some(v) => format!("собрание приняло реформу ({what}) — {:.0}% за", v * 100.0),
                            None => format!("начата реформа — {what}"),
                        };
                        self.news.push(Note::new(now, &line, false, "society"));
                    }
                }
            }
        }
        if get("reform_stop").is_some() && self.realm.pending[s].is_some() {
            self.realm.pending[s] = None;
            let p = &mut self.polity.of[c];
            p.legit = (p.legit - 0.04).max(0.05);
        }
        if get("crackdown").is_some() && self.realm.crackdown[s] == 0 {
            self.realm.crackdown[s] = 12;
            let p = &mut self.polity.of[c];
            p.legit = (p.legit - 0.05).max(0.05);
            self.news.push(Note::new(now, "твёрдая рука: год жёсткого порядка", false, "society"));
        }
        if let Some(b) = get("favour").and_then(|v| v.trim().parse::<usize>().ok()).filter(|&b| b < BLOCS) {
            let cost = favour_cost(self, s);
            if self.h.home(s).treasury >= cost {
                self.h.home_mut(s).treasury -= cost;
                self.realm.favour[s][b] = (self.realm.favour[s][b] + 0.35).min(0.8);
                for o in 0..BLOCS {
                    if o != b {
                        self.realm.favour[s][o] -= 0.03;
                    }
                }
                self.news.push(Note::new(now, &format!("уступки: {} (из казны {cost:.1})", polity::BLOC[b].name.to_lowercase()), false, "society"));
            }
        }
        if let Some(k) = get("regime").and_then(|v| v.trim().parse::<usize>().ok()).filter(|&k| k < FROM_ABOVE.len()) {
            if self.from_above_allowed(s, k).is_ok() {
                let to = FROM_ABOVE[k].0;
                let name = self.h.home(s).name.clone();
                let by = match to {
                    Regime::Junta => polity::HAWKS,
                    Regime::Theocracy => polity::ZEALOTS,
                    Regime::Party => polity::LEVELLERS,
                    _ => polity::LIBERALS,
                };
                let mut r = Rng::new(hash64(self.seed ^ 0xAB0E ^ now as u64));
                self.polity.change(c, &name, to, by, now as i32, &mut r);
                let p = &mut self.polity.of[c];
                // (a change from above costs the government's standing, less than a revolution)
                p.legit = 0.4;
                self.realm.changed[s] = now;
                self.realm.pending[s] = None;
                self.realm.election[s] = to.elected().map_or(0, |y| now + (y * 12.0) as u32);
                self.polity.crown(c, &strategy::world::names::person(&mut r), now as i32);
                self.state.mods[s] -= 0.08;
                self.news.push(Note::new(now, &format!("сверху: {} — теперь {}", FROM_ABOVE[k].1.to_lowercase(), to.name().to_lowercase()), true, "society"));
            }
        }
    }

    /// Whether side `s` may start reforming `law` to `to` now, else why not (in words).
    pub fn reform_allowed(&self, s: usize, law: usize, to: u8) -> Result<(), String> {
        let p = self.polity_of(s);
        if self.realm.pending[s].is_some() {
            return Err("уже идёт другая реформа".into());
        }
        let now = p.laws[law];
        if to == now || (to as i32 - now as i32).abs() != 1 {
            return Err("реформа — на одну ступень".into());
        }
        let closed = matches!(p.regime, Regime::Party | Regime::Junta | Regime::Theocracy);
        if closed && (law == polity::PRESS || law == polity::VOICE) && to > now {
            return Err(format!("{} не даёт ни печати, ни голоса — нужен другой строй", p.regime.name().to_lowercase()));
        }
        if p.regime == Regime::Theocracy && law == polity::CHURCH && to > now {
            return Err("теократия не отделит веру от государства".into());
        }
        if p.legit < 0.15 {
            return Err("у власти слишком мало законности".into());
        }
        Ok(())
    }

    /// Whether side `s` may give itself form `FROM_ABOVE[k]` now, else why not.
    pub fn from_above_allowed(&self, s: usize, k: usize) -> Result<(), String> {
        use polity::*;
        let p = self.polity_of(s);
        let to = FROM_ABOVE[k].0;
        if p.regime == to {
            return Err("это и есть нынешний строй".into());
        }
        if self.month() < self.realm.changed[s] + 60 && self.realm.changed[s] > 0 {
            return Err("строй менялся меньше пяти лет назад".into());
        }
        match to {
            Regime::Constitutional if !matches!(p.regime, Regime::Autocracy | Regime::Estates) => Err("конституцию дарует монарх".into()),
            Regime::Republic if p.moves[LIBERALS] + p.moves[LEVELLERS] < 0.5 => Err("за республику мало кто встанет (вольнодумцы и уравнители вместе — меньше половины)".into()),
            Regime::Junta if p.mood[OFFICERS] < 0.0 || p.clout[OFFICERS] < 0.1 => Err("офицерство не поддержит".into()),
            Regime::Party if p.industry < 0.3 || p.moves[LEVELLERS] < 0.35 => Err("нужны заводы и сильные уравнители".into()),
            Regime::Theocracy if p.piety < 0.55 || p.moves[ZEALOTS] < 0.3 => Err("народ недостаточно набожен, ревнителей мало".into()),
            _ => Ok(()),
        }
    }
}
