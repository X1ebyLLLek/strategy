//! An event in words: its title, its text with the numbers that made it, its choices and what each
//! will do.

use super::*;

impl Game {
    /// The event of kind `kind` for side `s`, in words, with its choices.
    pub fn event_of(&mut self, s: usize, kind: Kind) -> Event {
        let pack = self.pack.clone();
        let n = self.h.home(s);
        let foe = self.h.home(1 - s).name.clone();
        let money = |k: f32| k * n.treasury;
        let (title, text, choices): (String, String, Vec<(String, String)>) = match kind {
            Kind::Unrest(g) => (
                format!("Брожение: {}", pack.groups[g].name),
                format!("{} недовольны, и терпение их на исходе: {}.", pack.groups[g].name, n.cause(&pack, g)),
                vec![
                    ("Пойти навстречу — раздать из казны".into(), format!("из казны {:.1}; недовольство −40%", money(0.3))),
                    ("Ввести войска".into(), "недовольство −25%; устойчивость правительства падает".into()),
                    ("Ничего не делать".into(), "пусть терпят".into()),
                ],
            ),
            Kind::Famine(g) => (
                format!("Голод: {}", pack.groups[g].name),
                format!("{} голодают: {}.", pack.groups[g].name, n.cause(&pack, g)),
                vec![
                    ("Хлеб из казны".into(), format!("из казны {:.1}; недовольство −15%", money(0.25))),
                    ("Ввести нормы".into(), "недовольство −10%; устойчивость немного падает".into()),
                    ("Ничего не делать".into(), "рынок рассудит".into()),
                ],
            ),
            Kind::Treasury => (
                "Казна пуста".into(),
                "Денег в казне почти не осталось: армия и стройка встанут.".into(),
                vec![
                    ("Принудительный заём у собственников".into(), "у самой богатой группы — десятая часть денег; она недовольна".into()),
                    ("Урезать армию".into(), "траты на армию — на треть меньше".into()),
                    ("Ничего не делать".into(), "как-нибудь".into()),
                ],
            ),
            Kind::Thief => (
                "Казнокрад".into(),
                {
                    let (months, took) = self.h.econ[s].as_ref().map_or((0, 0.0), |e| (e.months, e.took - self.ev.took_seen[s]));
                    format!(
                        "Ревизия нашла: деньги казны уходят к людям, близким к тому, кто правит хозяйством. За {months} мес. ушло около {took:.1}: каждый месяц — {:.0}% того, что казна тратит. Ревизоры у нас {}.",
                        self.ev.theft[s] * 100.0,
                        match self.audit(s) {
                            a if a < 0.7 => "слабые: печать и собрания молчат",
                            a if a < 1.1 => "обычные",
                            _ => "зоркие: печать свободна, собрание спрашивает",
                        }
                    )
                },
                vec![("Отдать под суд и снять".into(), "хозяйством снова правит правительство само".into()), ("Закрыть глаза".into(), "устойчивость немного падает".into())],
            ),
            Kind::General(k) => {
                let name = self.h.war.command[s].as_ref().and_then(|gs| gs.get(k)).and_then(|g| g.general.as_ref()).map_or_else(|| format!("Командующий {}", group_name(k).to_lowercase()), |x| x.name.clone());
                (
                    format!("Неудачливый генерал: {}", group_name(k).to_lowercase()),
                    format!("{name} проигрывает бой за боем."),
                    vec![("Снять и поставить другого".into(), "на его место — новый человек".into()), ("Оставить".into(), "может, ещё научится".into())],
                )
            }
            Kind::Claim => (
                format!("{foe} готовит войну"),
                format!("Разведка доносит: {foe} обосновывает претензии к нам и через несколько месяцев может напасть."),
                vec![
                    ("Мобилизация".into(), "траты на армию +15 пунктов".into()),
                    ("Переговоры".into(), "их обоснование затянется на три месяца; мир спокойнее".into()),
                    ("Ничего не делать".into(), "ждать".into()),
                ],
            ),
            Kind::Weary => (
                "Страна устала от войны".into(),
                "Потери, занятая земля, тяготы — люди ропщут, правительство шатается.".into(),
                vec![
                    ("Искать мира".into(), "предложить мир, каждому — то, что держит; враг согласится, если не побеждает".into()),
                    ("Держаться".into(), "устойчивость падает".into()),
                ],
            ),
            Kind::Coup => (
                "Правительство на грани".into(),
                "Против правительства — заговор, на улицах — толпы.".into(),
                vec![
                    ("Опереться на армию".into(), "траты на армию +10 пунктов; устойчивость +0,15".into()),
                    ("Уступки народу".into(), "недовольство всех −30%; налог ниже".into()),
                    ("Ничего не делать".into(), "будь что будет".into()),
                ],
            ),
            Kind::Lenders => (
                "Кредиторы у ворот".into(),
                format!("Долг казны вырос до {:.0} месяцев её доходов: банкиры требуют своё и грозят закрыть кредит.", self.spirits.facts[s].debt),
                vec![
                    ("Просить отсрочки".into(), "долг растянется на два года дольше; процент выше".into()),
                    ("Объявить дефолт".into(), "долг списан; власть шатается; три года никто не даст взаймы".into()),
                    ("Платить, как условлено".into(), "казна пуста ещё долго".into()),
                ],
            ),
            Kind::Dear => (
                "Цены растут".into(),
                {
                    let inf = self.spirits.facts[s].inflation;
                    let how = if inf < 0.25 { "заработки за ними не поспевают, лавочники придерживают товар" } else { "лавочники прячут товар, люди стоят в очередях" };
                    format!("За год цены выросли на {:.0}%: {how}.", inf * 100.0)
                },
                vec![
                    ("Твёрдые цены".into(), "недовольство −15%; хозяева злы, власть немного шатается".into()),
                    ("Раздать пособия".into(), format!("из казны {:.1}; недовольство −10%", money(0.15))),
                    ("Ничего не делать".into(), "рынок рассудит".into()),
                ],
            ),
            Kind::Reform => {
                let schools = self.spirits.list[s].iter().any(|sp| sp.def().id == "illiterate");
                let hunger = self.spirits.list[s].iter().any(|sp| sp.def().id == "hunger");
                let (a, b) = if hunger { ("Аграрная реформа", "Индустриализация") } else if schools { ("Всеобщая грамотность", "Индустриализация") } else { ("Индустриализация", "Всеобщая грамотность") };
                (
                    "Реформаторы при дворе".into(),
                    "Молодые министры приносят записку: страна отстаёт, и они знают, с чего начать.".into(),
                    vec![
                        (format!("Начать: {a}"), "программа на годы; платит казна".into()),
                        (format!("Начать: {b}"), "программа на годы; платит казна".into()),
                        ("Не сейчас".into(), "реформаторы уйдут ни с чем".into()),
                    ],
                )
            }
            Kind::Genius(id, t) => {
                let p = self.folk.get(id).clone();
                let tech = &pack.techs[t];
                (
                    format!("Учёный просит денег: {}", p.name),
                    format!("{} ({}) просит у казны денег на работу: «{}». Говорят, он знает, как сделать это быстрее всех.", p.name, p.fame_word(), tech.name),
                    vec![
                        ("Дать денег".into(), format!("из казны {:.1}; работа над «{}» продвинется", money(0.12), tech.name)),
                        ("Отказать".into(), "денег не тратим; учёный обижен — может уехать к соседу".into()),
                    ],
                )
            }
            Kind::Hero(id) => {
                let p = self.folk.get(id).clone();
                (
                    format!("Герой войны: {}", p.name),
                    format!("{} — {} — без дела и без должности. Его речи слушают толпы, газеты пишут, что правительство боится его.", p.name, p.fame_word()),
                    vec![
                        ("Дать ему армию".into(), "он поведёт группу армий; власть крепче".into()),
                        ("Отправить в отставку".into(), "его голос стихнет, но обиженные военные запомнят; власть шатается".into()),
                        ("Пусть говорит".into(), "ничего не делать".into()),
                    ],
                )
            }
            Kind::Panic => (
                "Банковская паника".into(),
                "Годы дешёвого кредита и бегущие цены кончились: банки не могут вернуть вклады, у дверей толпы.".into(),
                vec![
                    ("Спасти банки за счёт казны".into(), format!("из казны {:.1}; собственники спокойнее", money(0.2))),
                    ("Пусть лопнут".into(), "собственники теряют деньги и злы; стройка встанет".into()),
                    ("Взять банки в казну".into(), "казна берёт десятую часть денег собственников; они злы надолго".into()),
                ],
            ),
            Kind::Flight => (
                "Бегство капитала".into(),
                "Собственники увозят деньги за границу: строить им страшно, хранить дома — тоже.".into(),
                vec![
                    ("Запретить вывоз денег".into(), "бегство остановится; собственники злы, климат хуже".into()),
                    ("Успокоить богатых: налог ниже".into(), "плоский налог; климат лучше; бедные недовольны".into()),
                    ("Ничего не делать".into(), "сами вернутся, когда станет спокойнее".into()),
                ],
            ),
            Kind::Realm(e) => self.realm_event(s, e),
            Kind::AllyCalls(head) => {
                let name = self.countries.get(head).map_or(String::new(), |c| c.0.clone());
                let war = self.diplo.war_of(head).map(|w| self.diplo.wars[w].clone());
                let text = match &war {
                    Some(w) => format!(
                        "{name}, {} с нами договором, воюет: {} против {} — {}. Посол просит нас вступить в войну.",
                        strategy::world::names::agree(&name, "связанная"),
                        self.countries[w.sides[0][0]].0,
                        self.countries[w.sides[1][0]].0,
                        w.cause
                    ),
                    None => format!("{name} просит нас о помощи."),
                };
                let cost = crate::world::relations::HELP * self.h.home(s).state_budget();
                let near = self.neighbours();
                let border = war.as_ref().and_then(|w| {
                    let side = w.side_of(head).unwrap_or(0);
                    w.sides[1 - side].iter().copied().find(|e| near.contains(e))
                });
                let join_note = match border {
                    Some(e) if !self.h.at_war => format!("{} — наш сосед: начнётся наша война с ним на карте; его союзники станут нашими врагами", self.countries[e].0),
                    _ => format!("враг далеко: каждый месяц — {cost:.2} из казны союзнику (6% трат), торговля с врагом закрыта; выйти можно в папке «Политика»"),
                };
                (
                    "Союзник просит помощи".into(),
                    text,
                    vec![
                        ("Вступить в войну".into(), join_note),
                        ("Отказать".into(), "союз расторгнут; о нашем вероломстве будут помнить".into()),
                    ],
                )
            }
            Kind::Offer(k, from) => {
                let name = self.countries.get(from).map_or(String::new(), |c| c.0.clone());
                let kind = crate::world::relations::kind_of(k).unwrap_or(strategy::world::diplomacy::Kind::Trade);
                let me_c = self.country(s);
                let op = self.diplo.of(from, me_c);
                (
                    format!("Посольство: {name}"),
                    format!("{name} предлагает нам: {}. {} Мнение их правительства о нас: {:+.0}.", kind.name().to_lowercase(), kind.about(), op * 100.0),
                    vec![
                        ("Принять".into(), format!("{} на {} лет; расторгнуть можно в папке «Политика»", kind.name(), kind.years())),
                        ("Отказаться".into(), "их правительство немного охладеет".into()),
                    ],
                )
            }
            Kind::PeaceOffer => (
                format!("{foe} просит мира"),
                if self.h.war.people()[1 - s] < self.h.war.people()[s] {
                    format!("{foe} проигрывает и предлагает мир: каждой стороне — то, что она держит.")
                } else {
                    format!("{foe} {} от войны и предлагает мир: каждой стороне — то, что она держит.", strategy::world::names::agree(&foe, "устала"))
                },
                vec![("Принять мир".into(), "война кончится, занятое — наше".into()), ("Воевать до победы".into(), "до капитуляции".into())],
            ),
        };
        self.ev.next_id += 1;
        Event { id: self.ev.next_id, side: s, kind, title, text, choices }
    }
}
