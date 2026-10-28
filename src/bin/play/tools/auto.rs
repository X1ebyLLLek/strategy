//! Bots play both sides (`play --auto`): the player's spheres go to bots — the economy to one of
//! his three economists, the laboratories to one of his three scientists, what the army builds and
//! the front to the general staff — and the war is declared after so many months of peace (a bot
//! does not decide on war itself yet: the watcher declares it). The game is written down month by
//! month for reading afterwards, in the folder `dir`: `chronicle.txt` (in words; the hidden
//! qualities of the people are shown there, and what the designs really are), `months.csv`,
//! `battles.csv`, `map_MMM.png` (the front), and a line in `../summary.txt`.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use strategy::economy::advisor::Mind;
use strategy::military::front::{General, Order};
use strategy::rng::{hash64, Rng};

use crate::words::terrain_ru;
use crate::words::{group_name, mix};
use crate::{Game, MONTHS};
use strategy::military::homefront::{lines, shares};

#[derive(Clone)]
pub struct Auto {
    /// The war is declared at the end of this month; the game ends at the end of month `end`
    /// (or at a capitulation).
    pub war_at: u32,
    pub end: u32,
    pub dir: PathBuf,
    pub done: bool,
    /// What is already written: lines of the news, battles, deeds of each side's economist,
    /// works of each side's laboratories; the groups' orders last written.
    news: usize,
    fought: usize,
    deeds: [usize; 2],
    works: [usize; 2],
    orders: [String; 2],
    /// Each side's output and people the month before the war; provinces at its start.
    before: [(f32, f32); 2],
    provs: [usize; 2],
}

impl Auto {
    pub fn new(war_at: u32, war_months: u32, dir: PathBuf) -> Auto {
        Auto { war_at, end: war_at + war_months, dir, done: false, news: 0, fought: 0, deeds: [0; 2], works: [0; 2], orders: Default::default(), before: [(0.0, 0.0); 2], provs: [0; 2] }
    }
}

fn mind_words(m: &Mind) -> String {
    format!(
        "образование {:.2}, опыт {:.2}, талант {:.2}, честность {:.2}, вера в государство {:+.2}",
        m.education, m.experience, m.talent, m.beliefs.honesty, m.beliefs.state
    )
}

fn general_words(g: &General) -> String {
    format!(
        "{} (образование {:.2}, опыт {:.2}, талант {:.2}, смелость {:+.2}): побед {}, поражений {}, взял провинций {}",
        g.name, g.education, g.experience, g.talent, g.bold, g.won, g.lost, g.took
    )
}

/// "раз", "раза" after a number.
fn times(n: usize) -> &'static str {
    match (n % 10, n % 100) {
        (2..=4, x) if !(12..=14).contains(&x) => "раза",
        _ => "раз",
    }
}

fn pct(x: f32) -> String {
    format!("{:+.1}%", x * 100.0)
}

impl Game {
    /// The player's spheres go to bots, and the chronicle begins.
    pub fn start_bots(&mut self, mut auto: Auto) {
        let mut r = Rng::new(hash64(self.seed ^ 0xB075));
        let k = r.below(self.pool.len());
        let i = r.below(self.scholars.len());
        self.apply(&[("econ".to_string(), format!("adv{k}")), ("scihead".to_string(), format!("sci{i}")), ("ctrl".to_string(), "staff".to_string())]);
        std::fs::create_dir_all(&auto.dir).expect("папка лога");
        auto.news = self.news.len();
        // (what the years before the game made is not news of the game)
        for s in 0..2 {
            auto.deeds[s] = self.h.econ[s].as_ref().map_or(0, |e| e.record.len());
            auto.works[s] = self.h.labs[s].as_ref().map_or(0, |l| l.works.len());
        }
        auto.provs = self.h.war.held();
        let mut o = String::new();
        let pack = &self.pack;
        let _ = writeln!(o, "ИГРА БОТОВ: карта {}, эпоха «{}» ({})", self.seed, pack.name, self.path);
        let _ = writeln!(o, "Мир прожил {} мес. до игры ({} порознь, {} с торговлей, {} — эти две страны с армиями в мире). За обе стороны играют боты.", crate::ALONE + crate::TOGETHER + crate::PREWAR, crate::ALONE, crate::TOGETHER, crate::PREWAR);
        let _ = writeln!(
            o,
            "Войну объявит наблюдатель в конце {}-го месяца (бот пока не решает о войне сам); игра кончится капитуляцией или в конце {}-го месяца.",
            auto.war_at, auto.end
        );
        let _ = writeln!(o, "Скрытые качества людей (0..1; в игре их не видно никому) и правда о разработках здесь — для разбора.\n");
        if !self.past.is_empty() {
            let _ = writeln!(o, "Летопись мира ({} войн за {} лет; * — войны этих двух стран):", self.past.len(), strategy::world::history::YEARS);
            for w in &self.past {
                let ours = w.sides.iter().any(|&c| c == self.h.at[0] || c == self.h.at[1]);
                let _ = writeln!(o, "  {}{}", if ours { "* " } else { "" }, w.words());
            }
            for s in 0..2 {
                let _ = writeln!(o, "  {}: потеряно областей в прошлых войнах {}; генералов-ветеранов {}", self.h.home(s).name, self.lost[s].len(), self.veterans[s].len());
                for v in &self.veterans[s] {
                    let _ = writeln!(o, "    {} — {}", general_words(v), v.past.join("; "));
                }
            }
            o.push('\n');
        }
        // the world's order: its faiths, its governments, what broke over two hundred years
        {
            use strategy::world::polity::Regime;
            let mut by: Vec<(Regime, usize)> = Regime::ALL.iter().map(|&r| (r, self.polity.of.iter().zip(&self.countries).filter(|(p, _)| p.regime == r).count())).filter(|x| x.1 > 0).collect();
            by.sort_by(|a, b| b.1.cmp(&a.1));
            let regimes: Vec<String> = by.iter().map(|(r, k)| format!("{} {k}", r.name().to_lowercase())).collect();
            let _ = writeln!(o, "Строй стран мира: {}.", regimes.join(", "));
            let mut faiths: Vec<(usize, usize)> = (0..self.polity.faiths.len()).map(|f| (f, self.polity.region_faith.iter().filter(|&&x| x == f).count())).filter(|x| x.1 > 0).collect();
            faiths.sort_by(|a, b| b.1.cmp(&a.1));
            let fw: Vec<String> = faiths.iter().map(|&(f, k)| format!("«{}» {k} обл.", self.polity.faiths[f].name)).collect();
            let _ = writeln!(o, "Веры мира ({} всего основано): {}.", self.polity.faiths.len(), fw.join(", "));
            let techs: Vec<usize> = self.know.iter().map(|k| k.count()).collect();
            if !techs.is_empty() {
                let mut t = techs.clone();
                t.sort();
                let _ = writeln!(o, "Техник у стран мира (из {}): меньше всех {}, в середине {}, больше всех {}.", pack.techs.len(), t[0], t[t.len() / 2], t[t.len() - 1]);
            }
            let cn = |c: usize| self.countries.get(c).map_or("?", |x| x.0.as_str());
            for s in 0..2 {
                let c = self.country(s);
                let ps: Vec<String> = self.diplo.pacts_of(c).iter().map(|p| format!("{} — {}", p.kind.name().to_lowercase(), cn(p.other(c)))).collect();
                let _ = writeln!(o, "Договоры {}: {}.", cn(c), if ps.is_empty() { "нет".to_string() } else { ps.join("; ") });
            }
            let _ = writeln!(o, "Мнение правительств друг о друге: {} о {} {:+.0}, {} о {} {:+.0}.", cn(self.country(0)), cn(self.country(1)), self.diplo.of(self.country(0), self.country(1)) * 100.0, cn(self.country(1)), cn(self.country(0)), self.diplo.of(self.country(1), self.country(0)) * 100.0);
            for w in &self.diplo.wars {
                let side = |k: usize| w.sides[k].iter().map(|&c| cn(c)).collect::<Vec<_>>().join(" + ");
                let _ = writeln!(o, "Идёт война в мире: {} против {} — {}.", side(0), side(1), w.cause);
            }
            let _ = writeln!(o, "Летопись обществ ({} записей; последние 40):", self.polity.chronicle.len());
            let k = self.polity.chronicle.len().saturating_sub(40);
            for (m, _, line) in &self.polity.chronicle[k..] {
                let _ = writeln!(o, "  {} лет назад: {line}", (-m).max(0) / 12);
            }
            o.push('\n');
        }
        for s in 0..2 {
            let n = self.h.home(s);
            let c = &self.h.war.sides[s];
            let _ = writeln!(
                o,
                "{}: людей {:.1} млн, выпуск {:.1} в мес, провинций {}, казна {:.1}, налог {:.0}%, армии {:.0}% трат; армия {:.0} дивизий = {:.1} линии вдоль границы ({})",
                n.name,
                n.people(),
                n.gdp_real,
                auto.provs[s],
                n.treasury,
                n.tax * 100.0,
                n.mil_share * 100.0,
                c.army.count.iter().sum::<f32>(),
                lines(pack, &c.army.count, self.border),
                mix(pack, &shares(pack, &c.army.count))
            );
            let (econ, sci) = if s == self.me {
                (self.pool[k].0.clone(), self.scholars[i].0.clone())
            } else {
                (self.foe_adviser.clone(), self.foe_scholar.clone())
            };
            if let Some(e) = self.h.econ[s].as_ref() {
                let _ = writeln!(o, "  экономист {econ}: {}", mind_words(&e.mind));
            }
            if let Some(sc) = self.h.scientists[s].as_ref() {
                let _ = writeln!(o, "  учёный {sci}: {}", mind_words(&sc.mind));
            }
            let _ = writeln!(o, "  армию строит и войсками ведёт генштаб");
            // what it knows, who rules it, its course, its people of note (with what is hidden)
            let c = self.h.war.th.countries[s];
            if let Some(k) = self.know.get(c) {
                let locked: Vec<&str> = k.locked(pack).iter().enumerate().filter(|x| *x.1).map(|(u, _)| pack.units[u].name.as_str()).collect();
                let _ = writeln!(
                    o,
                    "  стадия: {}; знает техник {} из {}{}",
                    strategy::world::progress::stage_name(pack, k.stage(pack)),
                    k.count(),
                    pack.techs.len(),
                    if locked.is_empty() { String::new() } else { format!("; нельзя выставить: {}", locked.join(", ")) }
                );
            }
            if let Some(r) = self.folk.offices.get(c).and_then(|x| x.ruler) {
                let p = self.folk.get(r);
                let _ = writeln!(o, "  правитель {} ({}): война {:+.2}, реформы {:+.2}, государство {:+.2}, честность {:.2}", p.name, p.calling.word(), p.hawk, p.reform, p.state, p.honesty);
            }
            if let Some(cur) = self.courses.now[s].as_ref() {
                let _ = writeln!(o, "  курс: «{}»{} — {}", cur.kind.name(), cur.favour.map_or(String::new(), |u| format!(", ставка на {}", pack.units[u].name)), cur.why.join("; "));
            }
            {
                use strategy::world::polity::{BLOC, LAW, LAWS, MOVE, MOVES};
                let p = &self.polity.of[c];
                let fa = self.polity.faiths.get(p.state_faith).map_or(String::new(), |f| f.name.clone());
                let laws: Vec<String> = (0..LAWS).map(|k| format!("{} — {}", LAW[k].name.to_lowercase(), p.law_word(k))).collect();
                let _ = writeln!(o, "  строй: {}{}; законность {:.2}, смута {:.2}; вера «{fa}» (набожность {:.2}), грамотность {:.2}, заводской уклад {:.2}", p.regime.name(), p.house.as_ref().map_or(String::new(), |h| format!(" ({h})")), p.legit, p.unrest, p.piety, p.literacy, p.industry);
                let _ = writeln!(o, "  законы: {}", laws.join("; "));
                let blocs: Vec<String> = (0..strategy::world::polity::BLOCS).map(|b| format!("{} {:.0}%/{:+.0}", BLOC[b].name.to_lowercase(), p.clout[b] * 100.0, p.mood[b] * 100.0)).collect();
                let _ = writeln!(o, "  силы (сила/настроение): {}", blocs.join(", "));
                let moves: Vec<String> = (0..MOVES).filter(|&m| p.moves[m] > 0.1).map(|m| format!("{} {:.0}%", MOVE[m].name.to_lowercase(), p.moves[m] * 100.0)).collect();
                let _ = writeln!(o, "  движения: {}; революций {}, переворотов {}, реформ {} за двести лет", if moves.is_empty() { "нет заметных".to_string() } else { moves.join(", ") }, p.revolutions, p.coups, p.reforms);
                let offers: Vec<String> = self.agenda.offers[s].iter().map(|t| self.task_title(t)).collect();
                if !offers.is_empty() {
                    let _ = writeln!(o, "  предложенные задачи: {}", offers.join("; "));
                }
            }
            for &id in self.folk.all_living(c).iter().take(6) {
                let p = self.folk.get(id);
                let _ = writeln!(o, "    {} — {}, {}: талант {:.2}, образование {:.2}, опыт {:.2}, честность {:.2}", p.name, p.calling.word(), p.fame_word(), p.talent, p.education, p.experience, p.honesty);
            }
        }
        o.push('\n');
        self.auto = Some(auto);
        self.write("chronicle.txt", &o, false);
        let mut head = String::from("month,war_month,war");
        for side in ["a", "b"] {
            for col in ["gdp", "gdp_rel", "people", "drafted", "anger", "treasury", "level", "tax", "mil", "sci", "units", "lines", "fed", "dead_k", "provinces", "own_held", "attacks", "attacks_won", "lost_worth"] {
                let _ = write!(head, ",{side}_{col}");
            }
        }
        head.push('\n');
        self.write("months.csv", &head, false);
        self.write("battles.csv", "month,war_month,attacker,terrain,ratio,odds,won,supply_att,supply_def,lost_att,lost_def\n", false);
        let staff = self.staff_words();
        self.write("chronicle.txt", &staff, true);
    }

    fn write(&self, file: &str, text: &str, append: bool) {
        let a = self.auto.as_ref().unwrap();
        let path = a.dir.join(file);
        let mut f = std::fs::OpenOptions::new().create(true).write(true).append(append).truncate(!append).open(&path).expect("файл лога");
        let _ = f.write_all(text.as_bytes());
    }

    /// The front as a picture (`map_MMM.png`).
    fn picture(&self) {
        let a = self.auto.as_ref().unwrap();
        let (_, cw, _, ch) = self.frame;
        let rgb = strategy::world::render::front(&self.world, &self.h.war.th, &self.h.war.owner, self.frame);
        let _ = std::fs::write(a.dir.join(format!("map_{:03}.png", self.month())), strategy::png::encode_rgb(cw, ch, &rgb));
    }

    /// How each group of people lives: how many, discontent against patience, want, the worst of it.
    fn people_words(&self) -> String {
        let pack = &self.pack;
        let mut o = String::new();
        for s in 0..2 {
            let n = self.h.home(s);
            let parts: Vec<String> = pack
                .groups
                .iter()
                .enumerate()
                .map(|(g, grp)| {
                    format!(
                        "{} {:.1} млн — недовольство {:.2} из {:.0}, нужда {:.0}%{} ({})",
                        grp.name,
                        n.pop[g],
                        n.discontent[g],
                        grp.patience,
                        n.hardship[g] * 100.0,
                        n.unemployed[g].map_or(String::new(), |u| format!(", без работы {:.0}%", u * 100.0)),
                        n.cause(pack, g)
                    )
                })
                .collect();
            let _ = writeln!(o, "  {} — люди: {}", n.name, parts.join("; "));
            // the state's household, the people, the banks, the chamber (27th session)
            let income = self.income(s).max(1e-6);
            let lv = self.civil.level[s];
            let f = &self.spirits.facts[s];
            let _ = writeln!(
                o,
                "    хозяйство: выпуск {:.1}, рост {:+.1}%, цены {:+.1}%, деньги {:+.1}%; налог {:.0}%, уклоняются {:.0}%; статьи [упр {} шк {} мед {} пор {} приз {}] выплачено {:.0}%, охват {:.2}, школы {:.2}, врачи {:.2}, порядок {:.2}; грамотность {:.2}, образованных {:.1}%; рожд {:.1}% смерт {:.1}% прирост {:+.2}%{}; кредит {:.0}% денег{}; облигации {:.1} мес. под {:.2}%, долг за границей {:.1} мес.; казна {:.1} мес.{}",
                n.gdp_real,
                f.growth * 100.0,
                f.inflation * 100.0,
                self.money_growth(s) * 100.0,
                n.tax * 100.0,
                self.fiscal.evaded[s] * 100.0,
                lv[0], lv[1], lv[2], lv[3], lv[4],
                if self.civil.asked[s].iter().sum::<f32>() > 1e-9 { 100.0 * self.civil.paid[s].iter().sum::<f32>() / self.civil.asked[s].iter().sum::<f32>() } else { 100.0 },
                self.civil.reach[s],
                self.civil.schools[s],
                self.civil.health[s],
                self.civil.order[s],
                self.polity_of(s).literacy,
                self.learned_share(s) * 100.0,
                self.demo.births[s] * 100.0,
                self.demo.deaths[s] * 100.0,
                self.people_growth(s) * 100.0,
                if self.demo.epidemic[s] > 0 { format!(", МОР {:.2}%/мес", self.demo.plague[s] * 100.0) } else { String::new() },
                100.0 * self.credit.loans[s] / self.coin_base(s).max(1e-6),
                if self.credit.crunch[s] > 0 { format!(" (КРИЗИС, {} мес.)", self.credit.crunch[s]) } else { String::new() },
                self.credit.bonds[s] / income,
                self.credit.bond_rate[s] * 100.0,
                self.policy.debt[s] / income,
                n.treasury / income,
                self.vote_share(s).map_or(String::new(), |v| format!("; за правительство {:.0}% мест", v * 100.0))
            );
        }
        o
    }

    /// What each staff builds, wants, sees and believes.
    fn staff_words(&self) -> String {
        let pack = &self.pack;
        let mut o = String::new();
        for s in 0..2 {
            let c = &self.h.war.sides[s];
            let _ = writeln!(
                o,
                "  {} — генштаб: армия {}; хочет {}; врага видит так: {}; сомнение {:.2}, настороженность {:+.2}",
                c.name,
                mix(pack, &shares(pack, &c.army.count)),
                mix(pack, &c.plan),
                mix(pack, &c.foe_mix),
                c.doubt,
                c.wariness
            );
            for (u, unit) in pack.units.iter().enumerate() {
                if c.army.count[u] < 0.05 && c.plan[u] < 0.01 {
                    continue;
                }
                let _ = writeln!(
                    o,
                    "    {:<22} {:>6.1} дивизий; огонь: верит ×{:.2}, на деле ×{:.2}; стойкость: верит ×{:.2}, на деле ×{:.2}; слава {:+.2}",
                    unit.name,
                    c.army.count[u],
                    c.own_fire[u].mean.exp(),
                    c.truth_fire[u],
                    c.own_tough[u].mean.exp(),
                    c.truth_tough[u],
                    c.glory[u]
                );
            }
        }
        o
    }

    /// After a month of the world: what happened goes into the chronicle; the war is declared
    /// when its month comes; the game ends. Whether the clock should stop for a look.
    pub fn after_month(&mut self) -> bool {
        let Some(mut a) = self.auto.take() else { return false };
        let pack = self.pack.clone();
        let m = self.month();
        let wm = self.h.war.month;
        let mut o = String::new();
        let phase = if self.h.at_war { "война" } else { "мир" };
        let _ = writeln!(o, "── {}-й месяц (конец {} {}-го года), {} ──", m, MONTHS[(m as usize + 11) % 12], (m + 11) / 12, phase);
        // how each side stands (its staff and generals, judged)
        for s in 0..2 {
            let w = self.h.war.stand_words(&pack, s);
            if !w.is_empty() {
                let _ = writeln!(o, "  {w}");
            }
        }
        // the battles of the month
        let fresh: Vec<_> = self.h.war.fought[a.fought.min(self.h.war.fought.len())..].to_vec();
        a.fought = self.h.war.fought.len();
        let mut att = [0u32; 2];
        let mut won = [0u32; 2];
        let mut lost = [0.0f32; 2];
        let mut rows = String::new();
        for f in &fresh {
            att[f.attacker] += 1;
            won[f.attacker] += f.won as u32;
            lost[f.attacker] += f.lost[0];
            lost[1 - f.attacker] += f.lost[1];
            let _ = writeln!(
                rows,
                "{m},{},{},{},{:.3},{:.3},{},{:.3},{:.3},{:.4},{:.4}",
                f.month,
                f.attacker,
                terrain_ru(f.terrain),
                f.ratio,
                f.odds,
                f.won as u8,
                f.supply[0],
                f.supply[1],
                f.lost[0],
                f.lost[1]
            );
        }
        let held = self.h.war.held();
        let own = self.h.war.people();
        let mut csv = format!("{m},{wm},{}", self.h.at_war as u8);
        for s in 0..2 {
            let n = self.h.home(s);
            let c = &self.h.war.sides[s];
            let units: f32 = c.army.count.iter().sum();
            let ppl = n.people();
            let _ = writeln!(
                o,
                "{}: выпуск {:.1} ({} к началу), людей {:.2} млн, служат {:.1}%, недовольство {:.2}, казна {:.1}, цены {:.2}, налог {:.0}%, армии {:.0}%, науке {:.0}%; армия {:.1} дивизий = {:.2} линии",
                n.name,
                n.gdp_real,
                pct(n.gdp_real / self.gdp0[s].max(1e-9) - 1.0),
                ppl,
                n.drafted.iter().sum::<f32>() / ppl.max(1e-9) * 100.0,
                n.anger(),
                n.treasury,
                n.level,
                n.tax * 100.0,
                n.mil_share * 100.0,
                n.science_share * 100.0,
                units,
                lines(&pack, &c.army.count, self.border)
            );
            // the army's money as the ministry spent it; the depots and reservists; the navy; the war's cost
            let b = &self.mil.book[s];
            let pc = |x: f32| x / b.allotted.max(1e-9) * 100.0;
            let _ = writeln!(
                o,
                "    военный бюджет {:.1}: жалованье {:.0}% (кадровые {:.0}%), обслуживание {:.0}% (получено {:.0}%), снабжение {:.0}%, новое снаряжение {:.0}%, флот {:.0}%, не потрачено {:.0}% | дивизий {} (штат {:.0}{}), на складах {:.0} батальонов, резервистов {:.2} млн",
                b.allotted,
                pc(b.pay_men + b.pay_cadre),
                pc(b.pay_cadre),
                pc(b.mend),
                b.mended * 100.0,
                pc(b.supply),
                pc(b.equip),
                pc(b.navy),
                pc(b.unspent),
                b.divisions,
                self.est_divisions(s),
                if self.mobilised(s) { ", мобилизация" } else { "" },
                b.stored / strategy::military::corps::BATTALION,
                b.reservists
            );
            if !self.pack.ships.is_empty() {
                let (ships, power) = self.navy_power(s);
                let build: Vec<String> = self.navy.yards[s].iter().take(4).map(|x| format!("{} {:.0}%", self.pack.ships[x.kind].name, x.done * 100.0)).collect();
                let missions: Vec<String> = self.navy.fleets[s].iter().filter(|q| !q.ships.is_empty()).map(|q| format!("{} {}: {} ({})", q.name, q.ships.len(), q.mission.name(), self.navy.seas.zones.get(q.zone).map_or("?", |z| z.name.as_str()))).collect();
                let _ = writeln!(
                    o,
                    "    флот: кораблей {ships}, сила {power:.0}, снабжение {:.0}%, доля {:.0}% армейских денег; верфи ({}): {}; {}; торговля морем {:.0}% (топят {:.1} в мес){}",
                    self.navy.fed[s] * 100.0,
                    self.navy_share(s) * 100.0,
                    self.navy.slipways[s],
                    if build.is_empty() { "пусто".to_string() } else { build.join(", ") },
                    missions.join("; "),
                    self.sea_dependence(s) * 100.0,
                    self.navy.sunk_trade[s],
                    if self.h.at_war { format!(", потеряно кораблей {}", self.navy.ships_lost[s]) } else { String::new() }
                );
            }
            {
                let w = self.works(s);
                let owed: f32 = self.war_mood.owed[s].iter().sum();
                let _ = writeln!(
                    o,
                    "    военные заводы {:.2} млн мест (конверсия {:.0}%), делают {:.1}, ввоз {:.1}; займы: {} (долг {:.1}, взято {:.1}){}",
                    w.jobs,
                    w.share * 100.0,
                    w.made,
                    w.bought_abroad,
                    crate::country::war_mood::BONDS[self.bond_level(s)].name,
                    owed,
                    self.war_mood.took[s],
                    if self.h.at_war {
                        let parts: Vec<String> = self.war_mood.parts[s].iter().map(|(n, x)| format!("{n} {:+.0}", x * 100.0)).collect();
                        format!("; поддержка войны {:.0}% ({}), забастовки {:.0}%", self.war_mood.support[s] * 100.0, parts.join(", "), self.war_mood.strike[s] * 100.0)
                    } else {
                        String::new()
                    }
                );
            }
            if self.h.at_war {
                let w = &self.mil.war_cost[s];
                let _ = writeln!(o, "    цена войны: {:.0} денег сверх мирного, убито {:.0} тыс., снаряжения потеряно {:.0} батальонов, долга {:.0}", w.money, w.dead * 1000.0, w.kit_lost, w.debt);
            }
            let _ = write!(
                csv,
                ",{:.3},{:.4},{:.4},{:.4},{:.4},{:.3},{:.4},{:.4},{:.4},{:.4},{:.3},{:.4},{:.4},{:.2},{},{:.4},{},{},{:.4}",
                n.gdp_real,
                n.gdp_real / self.gdp0[s].max(1e-9),
                ppl,
                n.drafted.iter().sum::<f32>() / ppl.max(1e-9),
                n.anger(),
                n.treasury,
                n.level,
                n.tax,
                n.mil_share,
                n.science_share,
                units,
                lines(&pack, &c.army.count, self.border),
                self.h.fed[s],
                self.h.dead[s] * 1000.0,
                held[s],
                own[s],
                att[s],
                won[s],
                lost[s]
            );
        }
        csv.push('\n');
        let names = [self.h.home(0).name.clone(), self.h.home(1).name.clone()];
        if self.h.at_war {
            let _ = writeln!(
                o,
                "фронт: провинций {} {} · {} {}; своих людей под своей властью {:.0}% · {:.0}%; снабжение {:.0}% · {:.0}%; погибло всего {:.0} · {:.0} тыс.; сдалось окружённых провинций всего {} · {}",
                names[0],
                held[0],
                names[1],
                held[1],
                own[0] * 100.0,
                own[1] * 100.0,
                self.h.fed[0] * 100.0,
                self.h.fed[1] * 100.0,
                self.h.dead[0] * 1000.0,
                self.h.dead[1] * 1000.0,
                self.h.war.surrendered[0],
                self.h.war.surrendered[1]
            );
            if fresh.is_empty() {
                let _ = writeln!(o, "боёв не было");
            } else {
                let side = |s: usize| {
                    if att[s] == 0 {
                        format!("{} {}", names[s], strategy::world::names::agree(&names[s], "не наступала"))
                    } else {
                        let ratio: f32 = fresh.iter().filter(|f| f.attacker == s).map(|f| f.ratio).sum::<f32>() / att[s] as f32;
                        format!("{} {} {} {}, {} {} (сил против обороны в среднем ×{:.2})", names[s], strategy::world::names::agree(&names[s], "наступала"), att[s], times(att[s] as usize), strategy::world::names::agree(&names[s], "взяла"), won[s], ratio)
                    }
                };
                let _ = writeln!(o, "боёв {}: {}; {}; потеряно сил {:.2} · {:.2}", fresh.len(), side(0), side(1), lost[0], lost[1]);
            }
            for s in 0..2 {
                if let Some(gs) = self.h.war.command[s].as_ref() {
                    let line: Vec<String> = gs
                        .iter()
                        .enumerate()
                        .map(|(k, g)| {
                            let order = match g.order {
                                Order::Hold => "держать",
                                Order::Deep => "в глубину",
                                Order::Attack(_) => "наступать",
                            };
                            let gen = g.general.as_ref().map_or("без генерала".to_string(), |x| x.name.clone());
                            format!("{} ({gen}) — {order}, {:.0}% сил", group_name(k), g.share * 100.0)
                        })
                        .collect();
                    let line = line.join("; ");
                    if line != a.orders[s] {
                        let _ = writeln!(o, "{} — группы: {}", names[s], line);
                        a.orders[s] = line;
                    }
                }
            }
        }
        // the news (what the laboratories made is told below, with the truth)
        for note in &self.news[a.news.min(self.news.len())..] {
            if note.tab == "science" {
                continue;
            }
            if note.tip.is_empty() {
                let _ = writeln!(o, "• {}", note.text);
            } else {
                let _ = writeln!(o, "• {} ({})", note.text, note.tip);
            }
        }
        a.news = self.news.len();
        for s in 0..2 {
            if let Some(e) = self.h.econ[s].as_ref() {
                for d in &e.record[a.deeds[s].min(e.record.len())..] {
                    let _ = writeln!(o, "{} — экономист: {}", names[s], d.what);
                }
                a.deeds[s] = e.record.len();
            }
            if let Some(lab) = self.h.labs[s].as_ref() {
                for w in &lab.works[a.works[s].min(lab.works.len())..] {
                    let _ = match w.kind {
                        Some(u) => writeln!(
                            o,
                            "{} — разработка: {} — обещают огонь {}, стойкость {}; на деле огонь {}, стойкость {}; {}",
                            names[s],
                            pack.units[u].name,
                            pct(w.claim.0.exp() - 1.0),
                            pct(w.claim.1.exp() - 1.0),
                            pct(w.gain.0.exp() - 1.0),
                            pct(w.gain.1.exp() - 1.0),
                            if w.adopted { "приняли" } else { "после испытаний не приняли" }
                        ),
                        None => writeln!(o, "{} — разработка: хозяйство, производительность {}", names[s], pct(w.gain.0.exp() - 1.0)),
                    };
                }
                a.works[s] = lab.works.len();
            }
        }
        // a year: how the staffs see things
        if m % 12 == 0 {
            o += "итоги года:\n";
            for s in 0..2 {
                if let Some(e) = self.h.econ[s].as_ref() {
                    if let Some(y) = e.years.last() {
                        let _ = writeln!(o, "  {} — экономист: за {}-й год люди {}, выпуск {}, волнений {}", names[s], y.year, pct(y.welfare), pct(y.output), y.unrest);
                    }
                }
            }
            o += &self.people_words();
            o += &self.staff_words();
        }
        // the war is declared when its month comes
        let mut stop = false;
        if !self.h.at_war && m >= a.war_at && self.h.war.fallen.is_none() {
            for s in 0..2 {
                a.before[s] = (self.h.home(s).gdp_real, self.h.home(s).people());
            }
            self.apply(&[("declare".to_string(), "1".to_string())]);
            a.news = self.news.len();
            a.provs = self.h.war.held();
            let _ = writeln!(o, "\n════ ВОЙНА ОБЪЯВЛЕНА (объявил наблюдатель) ════");
            for s in 0..2 {
                if let Some(gs) = self.h.war.command[s].as_ref() {
                    let _ = writeln!(o, "{}: групп армий {}", names[s], gs.len());
                    for (k, g) in gs.iter().enumerate() {
                        if let Some(gen) = g.general.as_ref() {
                            let _ = writeln!(o, "  {}: {}", group_name(k), general_words(gen));
                        }
                    }
                }
            }
            stop = true;
        } else if self.h.at_war && wm > 0 && wm % 6 == 0 {
            stop = true;
        }
        let over = self.h.war.fallen.is_some() || m >= a.end;
        o.push('\n');
        self.auto = Some(a);
        self.write("chronicle.txt", &o, true);
        self.write("months.csv", &csv, true);
        if !rows.is_empty() {
            self.write("battles.csv", &rows, true);
        }
        if stop || over {
            self.picture();
        }
        if over {
            self.finish();
            self.paused = true;
            return true;
        }
        false
    }

    /// The end: what the war came to, who did what.
    fn finish(&mut self) {
        let a = self.auto.clone().unwrap();
        let pack = &self.pack;
        let names = [self.h.home(0).name.clone(), self.h.home(1).name.clone()];
        let mut o = String::from("════ ИТОГ ════\n");
        let held = self.h.war.held();
        let own = self.h.war.people();
        let wm = self.h.war.month;
        let outcome = match self.h.war.fallen {
            Some(s) => format!("{} {} на {}-м месяце войны", names[s], strategy::world::names::agree(&names[s], "капитулировала"), wm),
            None if self.h.at_war => format!("война не кончилась за {wm} мес."),
            None => "войны не было".to_string(),
        };
        let _ = writeln!(o, "{outcome}");
        let f = &self.h.war.fought;
        let att = |s: usize| f.iter().filter(|x| x.attacker == s).count();
        let won = |s: usize| f.iter().filter(|x| x.attacker == s && x.won).count();
        for s in 0..2 {
            let n = self.h.home(s);
            let (g0, p0) = a.before[s];
            let _ = writeln!(
                o,
                "{}: провинций {} → {}, своих людей под своей властью {:.0}%, погибло {:.0} тыс.; выпуск к довоенному {}, людей к довоенному {}; наступала {} {}, взяла {} ({:.0}%)",
                names[s],
                a.provs[s],
                held[s],
                own[s] * 100.0,
                self.h.dead[s] * 1000.0,
                pct(n.gdp_real / g0.max(1e-9) - 1.0),
                pct(n.people() / p0.max(1e-9) - 1.0),
                att(s),
                times(att(s)),
                won(s),
                won(s) as f32 / att(s).max(1) as f32 * 100.0
            );
            if let Some(lab) = self.h.labs[s].as_ref() {
                let adopted = lab.works.iter().filter(|w| w.adopted).count();
                let _ = writeln!(o, "  разработок {} (приняли {}); армия в конце: {}", lab.works.len(), adopted, mix(pack, &shares(pack, &self.h.war.sides[s].army.count)));
            }
            if let Some(gs) = self.h.war.command[s].as_ref() {
                for (k, g) in gs.iter().enumerate() {
                    if let Some(gen) = g.general.as_ref() {
                        let _ = writeln!(o, "  {}: {}", group_name(k), general_words(gen));
                    }
                }
            }
        }
        o += &self.staff_words();
        // the world's relations over the game
        let cn = |c: usize| self.countries.get(c).map_or("?", |x| x.0.as_str());
        let lines: Vec<&(i32, usize, String)> = self.diplo.chronicle.iter().filter(|x| x.0 >= 0).collect();
        let _ = writeln!(o, "Мир за игру ({} записей о договорах и войнах):", lines.len());
        for (m, _, line) in lines.iter().rev().take(40).collect::<Vec<_>>().into_iter().rev() {
            let _ = writeln!(o, "  {}-й год: {line}", m / 12 + 1);
        }
        for s in 0..2 {
            let helpers: Vec<String> = self.rel.helpers[s].iter().map(|&(c, x)| format!("{} ({x:.2} в мес.)", cn(c))).collect();
            if !helpers.is_empty() {
                let _ = writeln!(o, "  на стороне {} воюют: {}", names[s], helpers.join(", "));
            }
        }
        self.write("chronicle.txt", &o, true);
        let line = format!(
            "карта {} | {} | {} против {} | {} | провинций {}→{} / {}→{} | погибло {:.0} / {:.0} тыс. | наступлений {} ({}) / {} ({}) | папка {}\n",
            self.seed,
            pack.name,
            names[0],
            names[1],
            outcome,
            a.provs[0],
            held[0],
            a.provs[1],
            held[1],
            self.h.dead[0] * 1000.0,
            self.h.dead[1] * 1000.0,
            att(0),
            won(0),
            att(1),
            won(1),
            a.dir.display()
        );
        if let Some(parent) = a.dir.parent() {
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(parent.join("summary.txt")) {
                let _ = f.write_all(line.as_bytes());
            }
        }
        if let Some(x) = self.auto.as_mut() {
            x.done = true;
        }
    }
}
