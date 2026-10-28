//! The card of the province the hand rests on.

use super::*;

impl MapView {
    /// The card of a province: whose it is and its ground (a town's name if one stands there); in
    /// a war — who holds it, our troops and how dug in, its people; what the map's mode shows; what
    /// a click there would do.
    pub(super) fn province_card(&self, g: &Game, p: usize, chosen: Option<usize>, at: (f32, f32)) -> Card {
        let me = g.me;
        let c = g.country_of[p];
        let (name, colour) = if c == u32::MAX {
            ("Ничья земля".to_string(), Color32::from_gray(150))
        } else {
            let k = &g.countries[c as usize];
            (k.0.clone(), Color32::from_rgb(k.1[0], k.1[1], k.1[2]))
        };
        let terrain = crate::words::terrain_ru(g.world.provinces[p].terrain);
        let town = g.towns.iter().find(|t| t.0 == p).map(|t| t.1.clone());
        let sub = if town.is_some() { format!("{name} · {terrain}") } else { terrain.to_string() };
        let mut card = if c == u32::MAX { Card::new(town.unwrap_or(name)).flag(colour).sub(sub) } else { Card::new(town.unwrap_or(name)).banner(c as usize, colour).sub(sub) };
        let war = &g.h.war;
        let q = war.th.at.get(p).copied().unwrap_or(NO);
        if q != NO {
            let q = q as usize;
            if war.owner[q] != war.th.home[q] {
                card = card.bad("занята", g.h.home(war.owner[q]).name.clone());
            }
            if let Some(c) = war.corps.as_ref() {
                // (a war of divisions: how many stand here, how many it feeds, what the chosen would do)
                let here: Vec<&strategy::military::corps::Division> = c.divs.iter().filter(|d| d.side == me && d.at == q).collect();
                if !here.is_empty() {
                    let eat: f32 = here.iter().map(|d| d.has.iter().zip(&g.pack.units).filter(|(_, u)| !u.supply.is_empty()).map(|(x, _)| x).sum::<f32>()).sum();
                    let cap = war.cap_at(&g.pack, me, q);
                    card = card.row("наших дивизий", here.len().to_string()).row("окопы", format!("{:.0}%", war.dug[me][q] * 100.0));
                    card = card.tone("прокормит", format!("{:.0}%", (cap / eat.max(1e-6) * 100.0).min(100.0)), if eat <= cap { Tone::Good } else { Tone::Bad });
                }
                if !self.picked.is_empty() && g.h.at_war {
                    card = card.note(if war.owner[q] == me { "правой кнопкой — выбранным идти сюда" } else { "правой кнопкой — выбранным атаковать" });
                } else if !self.picked.is_empty() {
                    card = card.note(if war.owner[q] == me { "правой кнопкой — выбранным идти сюда" } else { "в мирное время на чужую землю не войти" });
                }
            } else {
                let mine: f32 = war.troops[me][q].iter().sum();
                if mine > 0.0 {
                    card = card.row("наших дивизий", format!("{mine:.1}")).row("окопы", format!("{:.0}%", war.dug[me][q] * 100.0));
                }
            }
            card = card.row("людей", format!("{:.2} млн", war.th.pop[q]));
            if self.mode == Mode::Supply && war.owner[q] == me {
                let s = war.supply(me)[q];
                card = card.tone("снабжение", format!("{:.0}%", s * 100.0), if s > 0.85 { Tone::Good } else { Tone::Bad });
            }
        }
        // (the height of the land under the hand, as shown: of the highest peak — `PEAK_M` metres)
        let metres = self.ground.z(at.0, at.1) / RELIEF * PEAK_M;
        if metres < 0.0 {
            card = card.tone("ниже моря", format!("{:.0} м", -metres), Tone::Bad);
        } else if self.mode == Mode::Terrain {
            card = card.row("высота", format!("{:.0} м", (metres / 10.0).round() * 10.0));
        }
        match self.mode {
            Mode::Dev => {
                let most = g.prov_dev.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
                card = card.row("развитие", format!("{:.0}% лучшего края", g.prov_dev[p] / most * 100.0));
            }
            Mode::People => {
                card = card.row("людей", format!("{:.2} млн", g.prov_people[p]));
            }
            Mode::Res => {
                let res: Vec<&str> = g.prov_res[p].iter().map(|&r| g.pack.resources[r].name.as_str()).collect();
                card = card.row("недра", if res.is_empty() { "нет".to_string() } else { res.join(", ") });
            }
            Mode::Faith => {
                let st = g.prov_state[p] as usize;
                match g.polity.region_faith.get(st).and_then(|&f| g.polity.faiths.get(f)) {
                    Some(f) => {
                        card = card.row("вера", f.name.clone());
                        let creed = f.creed();
                        if !creed.is_empty() {
                            card = card.text(format!("учит: {}", creed.join("; ")));
                        }
                    }
                    None => card = card.note("здесь никто не живёт"),
                }
            }
            Mode::Relations => {
                let c = g.country_of[p] as usize;
                let mine = g.country(g.me);
                if c < g.diplo.n && c != mine {
                    let op = g.diplo.of(c, mine);
                    card = card.row("их мнение о нас", format!("{:+.0}", op * 100.0));
                    for (why, x) in g.reasons_words(c, mine).iter().take(4) {
                        card = card.row(*why, format!("{:+.0}", x * 100.0));
                    }
                    let pacts: Vec<String> = g.diplo.pacts_of(c).iter().filter(|x| x.has(mine)).map(|x| x.kind.name().to_lowercase()).collect();
                    if !pacts.is_empty() {
                        card = card.row("договоры с нами", pacts.join(", "));
                    }
                    let allies: Vec<String> = g.diplo.allies(c).iter().map(|&a| g.countries[a].0.clone()).collect();
                    if !allies.is_empty() {
                        card = card.row("её союзники", allies.join(", "));
                    }
                }
            }
            Mode::Regime => {
                if let Some(x) = g.polity.of.get(g.country_of[p] as usize) {
                    card = card.row("строй", x.regime.name().to_lowercase()).row("законность", format!("{:.0}%", x.legit * 100.0)).row("смута", format!("{:.0}%", x.unrest * 100.0));
                    if let Some(h) = &x.house {
                        card = card.row("правит", h.clone());
                    }
                }
            }
            Mode::Lost => {
                let st = g.prov_state[p];
                let mut past: Vec<&(u32, u32, u32)> = g.lost_lands.iter().filter(|l| l.0 == st).collect();
                past.sort_by_key(|l| l.2);
                if past.is_empty() {
                    card = card.note("войны прошлого этих земель не меняли");
                } else {
                    card = card.head("Былое");
                }
                for l in past.iter().take(3) {
                    card = card.row(format!("прежде — {}", g.countries[l.1 as usize].0), format!("{} лет назад", l.2));
                }
            }
            _ => {}
        }
        if self.picking {
            // (what the country is like: its government, its faith, its strongest movement — every
            // world grows its own)
            if let Some(x) = g.polity.of.get(c as usize) {
                let fa = g.polity.faiths.get(x.state_faith).map_or(String::new(), |f| f.name.clone());
                let (m, ms) = x.top_move();
                card = card.head("Какая это страна").row("строй", x.regime.name().to_lowercase()).row("вера", fa).row("законность", format!("{:.0}%", x.legit * 100.0)).row("смута", format!("{:.0}%", x.unrest * 100.0));
                if ms > 0.25 {
                    card = card.row("сильнее всех", format!("{} ({:.0}%)", strategy::world::polity::MOVE[m].name.to_lowercase(), ms * 100.0));
                }
                if let Some(k) = g.know.get(c as usize) {
                    card = card.row("стадия", strategy::world::progress::stage_name(&g.pack, k.stage(&g.pack)).to_string());
                }
                if let Some((_, line)) = x.story.last() {
                    card = card.text(format!("Последнее в летописи: {line}"));
                }
            }
            let me = g.h.war.th.countries[g.me];
            card = card.note(if c != u32::MAX && c as usize == me { "ваша страна".to_string() } else if c != u32::MAX { "щёлкните — играть за эту страну".to_string() } else { "ничья земля".to_string() });
            return card;
        }
        if let Build::Line { rail, from } = self.build {
            let name = g.line_name(rail);
            match from {
                None => card = card.note(format!("{name}: щёлкните по своей провинции — откуда")),
                Some(a) if q != NO && war.owner[q as usize] == me => {
                    if let Some(path) = crate::world::land::plan(war, me, a, q as usize) {
                        let (cost, months) = g.line_cost(me, rail, path.len());
                        card = card.head(name).row("провинций", path.len().to_string()).row("стоит", format!("{cost:.2}")).row("месяцев", months.to_string());
                        card = if path.len() > crate::world::statecraft::LINE_MOST { card.bad("длиннее", format!("{} провинций", crate::world::statecraft::LINE_MOST)) } else { card.note("щёлкните — проложить; правой — отмена") };
                    }
                }
                Some(_) => card = card.note("только по своей земле"),
            }
        } else if self.build == Build::Depot {
            card = card.note("щёлкните по своей провинции — там встанет склад");
        } else if self.build == Build::Muster {
            card = card.note("щёлкните по своей провинции — новые дивизии будут собираться здесь");
        } else if chosen.is_some() && g.h.at_war && q != NO {
            card = card.note(if war.owner[q as usize] != me { "правой кнопкой — наступать сюда" } else { "правой кнопкой — держать здесь" });
        }
        card
    }
}
