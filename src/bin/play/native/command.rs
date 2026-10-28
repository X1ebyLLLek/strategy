//! The orders of the war as HOI4 gives them: over the groups' cards, the bar of the chosen group's
//! plans — its stretch of the front drawn over the map, the arrow of its attack, the plan carried
//! out or stopped, deep defence, back to the staff, its divisions chosen — each order a plate
//! with its icon over its word and its key on a cap in the corner; the keys of the war (the
//! groups by their numbers, Ctrl and a number — the chosen divisions to that group; the chosen
//! divisions' orders; the clock's speed by + and −).

use eframe::egui::{self, pos2, vec2, Color32, FontFamily, FontId, Rect, Response, Sense, Shape, Stroke, Ui};

use crate::Game;
use crate::tools::saves::Act;
use strategy::military::front::Order;
use super::chrome::{self, Card, Chrome, Tip};
use super::look::Skin;
use super::map::{self, Tool};
use super::{hud, look, App};

/// Height of an order's plate.
pub const H: f32 = 58.0;

/// An order: a plate with its icon over its word, its key on a cap in the top right corner (on —
/// a state that holds now: a rim of gold light, the icon gold; can't be given — dimmed). Its
/// response.
#[allow(clippy::too_many_arguments)]
pub fn order(ui: &mut Ui, ch: &Chrome, skin: &Skin, icon: &str, word: &str, key: &str, on: bool, enabled: bool) -> Response {
    let font = FontId::new(13.0, FontFamily::Name("head".into()));
    // (a word that can't be given now: dimmer, but still read — the icon says it more)
    let ink = if enabled { skin.desk_ink } else { skin.desk_ink.gamma_multiply(0.72) };
    let gal = ui.painter().layout_no_wrap(word.to_string(), font, ink);
    let kg = (!key.is_empty()).then(|| ui.painter().layout_no_wrap(key.to_string(), FontId::new(12.0, FontFamily::Name("bold".into())), Color32::from_rgb(236, 228, 206)));
    let s = 28.0;
    // (wide enough for the word, and for the icon beside a wide cap)
    let cap_w = kg.as_ref().map_or(0.0, |k| k.size().x + 8.0);
    let w = (gal.size().x + 18.0).max(72.0).max(cap_w + 11.0 + s + 14.0);
    let (r, resp) = ui.allocate_exact_size(vec2(w, H), if enabled { Sense::click() } else { Sense::hover() });
    let resp = if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
    if !ui.is_rect_visible(r) {
        return resp;
    }
    let hot = enabled && resp.hovered();
    let down = enabled && resp.is_pointer_button_down_on();
    let sink = if down { 1.0 } else { 0.0 };
    let mut v = chrome::Out::new();
    ch.plate(&mut v, r, skin, true, false, hot || on, down);
    if on {
        // (lit from inside: a glow of gold along the rim, a bar of it at the foot)
        let gold = hud::lighten(ch.trim, 0.25);
        v.push(egui::epaint::Shadow { offset: vec2(0.0, 0.0), blur: 10.0, spread: 0.0, color: gold.gamma_multiply(0.35) }.as_shape(r.shrink(3.0), 2.0).into());
        v.push(Shape::rect_stroke(r.shrink(1.5), 2.0, Stroke::new(2.0, gold)));
        v.push(Shape::rect_filled(Rect::from_min_max(pos2(r.min.x + 6.0, r.max.y - 4.0), pos2(r.max.x - 6.0, r.max.y - 2.0)), 1.0, gold));
    }
    if !enabled {
        v.push(Shape::rect_filled(r, 2.0, Color32::from_black_alpha(60)));
    }
    ui.painter().extend(v);
    // the cap of the key, top right; the icon, a shade under it (left of a wide cap)
    let cap = kg.as_ref().map(|k| Rect::from_min_size(pos2(r.max.x - k.size().x - 11.0, r.min.y + 3.0), k.size() + vec2(8.0, 2.0)));
    let icons = chrome::icons(ui.ctx());
    let cx = cap.map_or(r.center().x, |c| r.center().x.min(c.min.x - 5.0 - s / 2.0));
    let c = pos2(cx, r.min.y + 21.0 + sink);
    look::icon(&icons, icon, s, Color32::from_black_alpha(210)).paint_at(ui, Rect::from_center_size(c + vec2(1.0, 1.5), vec2(s, s)));
    let tint = if on { hud::lighten(ch.trim, 0.35) } else if hot { hud::lighten(skin.desk_ink, 0.3) } else { skin.desk_ink };
    look::icon(&icons, icon, s, if enabled { tint } else { tint.gamma_multiply(0.45) }).paint_at(ui, Rect::from_center_size(c, vec2(s, s)));
    ui.painter().galley(pos2(r.center().x - gal.size().x / 2.0, r.max.y - gal.size().y - 5.0 + sink), gal, ink);
    // the key on its cap
    if let (Some(kg), Some(cap)) = (kg, cap) {
        let p = ui.painter();
        p.rect_filled(cap.translate(vec2(0.0, 1.5)), 3.0, Color32::from_black_alpha(160));
        p.rect_filled(cap, 3.0, Color32::from_rgb(34, 31, 26));
        p.rect_stroke(cap, 3.0, Stroke::new(1.0, ch.trim.gamma_multiply(if enabled { 0.9 } else { 0.4 })));
        p.galley(cap.min + vec2(4.0, 1.0), kg, Color32::from_rgb(236, 228, 206));
    }
    resp
}

/// The keys of the game and what each does (the menu's page of keys).
pub const KEYS: [(&str, &str); 19] = [
    ("Пробел", "пустить или остановить время"),
    ("+  −", "быстрее, медленнее"),
    ("Esc", "отменить план, снять выбор, закрыть папку; меню"),
    ("1 … 9", "выбрать группу армий (дважды — показать её на карте)"),
    ("Shift 1 … 9", "добавить группу к выбранным (или убрать): приказы — всем сразу"),
    ("Tab", "следующая группа"),
    ("Ctrl 1 … 9", "выбранные дивизии — в эту группу"),
    ("F", "фронт: щелчок по соседней стране — весь фронт с ней (на несколько армий — по их дивизиям); или провести по своим провинциям"),
    ("A", "стрела наступления армии (план)"),
    ("Enter", "исполнить план или остановить наступление"),
    ("D", "оборона в глубину"),
    ("R", "новые дивизии — в эту армию"),
    ("G", "армию — генштабу; выбранные дивизии — их генералам"),
    ("S", "выбрать все дивизии армии"),
    ("H", "выбранным дивизиям стоять"),
    ("N", "новая армия из выбранных дивизий"),
    ("ЛКМ", "выбрать стопку, рамкой — несколько; с Shift — добавить"),
    ("ПКМ", "идти туда или атаковать; с Shift — и дальше оттуда"),
    ("ПКМ, СКМ", "тянуть — двигать карту; колесо — ближе, дальше"),
];

/// The page of keys: each on its cap, what it does beside it (on paper).
pub fn keys_page(ui: &mut Ui, ch: &Chrome, skin: &Skin) {
    let kf = FontId::new(12.5, FontFamily::Name("bold".into()));
    for (k, what) in KEYS {
        ui.horizontal(|ui| {
            let kg = ui.painter().layout_no_wrap(k.to_string(), kf.clone(), Color32::from_rgb(236, 228, 206));
            let (r, _) = ui.allocate_exact_size(vec2(84.0, 20.0), Sense::hover());
            let cap = Rect::from_min_size(pos2(r.max.x - kg.size().x - 10.0, r.center().y - kg.size().y / 2.0 - 1.0), kg.size() + vec2(8.0, 2.0));
            let p = ui.painter();
            p.rect_filled(cap.translate(vec2(0.0, 1.5)), 3.0, Color32::from_black_alpha(120));
            p.rect_filled(cap, 3.0, Color32::from_rgb(34, 31, 26));
            p.rect_stroke(cap, 3.0, Stroke::new(1.0, ch.trim));
            p.galley(cap.min + vec2(4.0, 1.0), kg, Color32::from_rgb(236, 228, 206));
            ui.add(egui::Label::new(egui::RichText::new(what).size(13.5).color(skin.ink)).wrap());
        });
    }
}

/// What the keys of the war asked this frame.
#[derive(Default)]
struct Keys {
    group: Option<usize>,
    add: bool,
    to_group: Option<usize>,
    next: bool,
    faster: bool,
    slower: bool,
    halt: bool,
    back: bool,
    raise: bool,
    front: bool,
    attack: bool,
    go: bool,
    deep: bool,
    pick: bool,
    feed: bool,
}

impl App {
    /// The bar of the chosen group's plans, over the groups' cards in the middle of the map.
    pub(super) fn planbar(&mut self, ctx: &egui::Context, g: &mut Game) {
        let me = g.me;
        // (the spoiled control of the self-checks: no bar)
        let (Some(k), true, false) = (self.chosen, g.h.war.command[g.me].is_some(), super::spoiled()) else { return };
        let at_war = g.h.at_war;
        let ks = self.groups_chosen(g);
        let Some(gr) = g.h.war.command[me].as_ref().and_then(|gs| gs.get(k)).cloned() else { return };
        let Some(m) = self.map.as_ref() else { return };
        let corps = g.h.war.corps.is_some();
        let tool = m.tool.clone();
        let draft = m.drafts.iter().any(|d| d.0 == k);
        let free = ctx.available_rect();
        let bar = egui::containers::panel::PanelState::load(ctx, egui::Id::new("armybar")).map_or(free.max.y, |p| p.rect.min.y);
        let ch = self.chrome.clone();
        let s = self.skin;
        let hue = map::HUES[k % map::HUES.len()];
        let gen = gr.general.as_ref().map_or(String::new(), |x| x.name.clone());
        let attacking = matches!(gr.order, Order::Attack(_));
        let status = match &tool {
            Tool::Front { .. } => "щелчок по соседу — весь фронт; протянуть вдоль границы — участок".to_string(),
            Tool::Attack { .. } => "щёлкните по провинции врага — цель".to_string(),
            _ if ks.len() > 1 => format!("и ещё: {}", ks[1..].iter().map(|&j| crate::words::group_name(j)).collect::<Vec<_>>().join(", ")),
            _ if draft && !at_war => "план готов — исполнить в войну".to_string(),
            _ if draft => "план готов — «Исполнить»".to_string(),
            _ => {
                let order = match gr.order {
                    Order::Hold => "держит участок",
                    Order::Deep => "держит в глубину",
                    Order::Attack(_) => "наступает",
                };
                format!("{order} · {}", if gr.staff { "ведёт генштаб" } else { "ведёте вы" })
            }
        };
        let mut todo: Vec<&'static str> = Vec::new();
        let id = egui::Id::new("planbar");
        // (in the middle of the map, but never over the small map in the corner)
        let mini = m.last_mini;
        let at = ctx.memory(|mem| mem.area_rect(id)).map_or(pos2(free.center().x - 330.0, bar - 90.0), |r| {
            let mut x = free.center().x - r.width() / 2.0;
            if mini.is_positive() && bar - r.height() - 12.0 < mini.max.y && x + r.width() > mini.min.x - 10.0 {
                x = (mini.min.x - 10.0 - r.width()).max(free.min.x + 8.0);
            }
            pos2(x, bar - r.height() - 12.0)
        });
        egui::Area::new(id).order(egui::Order::Middle).fixed_pos(at).show(ctx, |ui| {
            let back = chrome::hold(ui);
            let fr = egui::Frame::none().inner_margin(egui::Margin { left: 16.0, right: 10.0, top: 8.0, bottom: 8.0 }).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    // the group: its general's portrait, its name, what it does
                    let (face, _) = ui.allocate_exact_size(vec2(36.0, 46.0), Sense::hover());
                    ui.painter().rect_filled(face.expand(2.5), 2.0, Color32::from_black_alpha(220));
                    if let Some(img) = look::portrait(&g.face_of(&gen, "general"), 36.0) {
                        img.paint_at(ui, face);
                    }
                    ui.painter().rect_stroke(face.expand(1.0), 2.0, Stroke::new(1.5, ch.trim));
                    ui.vertical(|ui| {
                        ui.set_width(230.0);
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new(crate::words::group_name(k).to_uppercase()).family(FontFamily::Name("head".into())).size(17.0).color(s.desk_ink));
                        ui.label(egui::RichText::new(if gen.is_empty() { "без генерала".to_string() } else { format!("генерал {gen}") }).size(13.0).color(s.desk_ink.gamma_multiply(0.85)));
                        let mut job = egui::text::LayoutJob::simple_singleline(status.clone(), FontId::proportional(13.0), hud::lighten(hue, 0.35));
                        job.wrap = egui::text::TextWrapping::truncate_at_width(230.0);
                        ui.label(job);
                    });
                    let (sep, _) = ui.allocate_exact_size(vec2(10.0, H), Sense::hover());
                    let mut v = chrome::Out::new();
                    hud::divider(&mut v, sep.center().x, sep.min.y + 4.0, sep.max.y - 4.0, &ch);
                    ui.painter().extend(v);
                    // the plans
                    let on_front = matches!(&tool, Tool::Front { gs, .. } if gs.contains(&k));
                    if order(ui, &ch, &s, "wire", "Линия фронта", "F", on_front, corps)
                        .tip(|| {
                            Card::new("Линия фронта")
                                .icon("wire")
                                .text("Щёлкните по соседней стране — вся граница с ней станет фронтом выбранных групп; несколько групп (Shift + клик по карточке или Shift + цифра) делят его поровну, участок за участком. Против любого соседа, не только соперника.")
                                .text("Или проведите левой кнопкой по своим провинциям — только этот участок. Соседние группы уступают взятое.")
                                .note("клавиша F; правая кнопка — отменить")
                        })
                        .clicked()
                    {
                        todo.push("front");
                    }
                    let on_attack = matches!(&tool, Tool::Attack { gs } if gs.contains(&k));
                    if order(ui, &ch, &s, "strike", "Наступление", "A", on_attack, true)
                        .tip(|| Card::new("Стрела наступления").icon("strike").text("Щёлкните по провинции врага: туда пойдёт удар группы. Это план — наступление начнётся, когда нажмёте «Исполнить».").note("клавиша A; правая кнопка по карте при выбранной группе — приказ сразу"))
                        .clicked()
                    {
                        todo.push("attack");
                    }
                    let (word, icon) = if !draft && attacking { ("Остановить", "stop") } else { ("Исполнить", "play") };
                    let can = (draft && at_war) || attacking;
                    if order(ui, &ch, &s, icon, word, "Enter", false, can)
                        .tip(|| {
                            if !at_war {
                                Card::new("Исполнить план").icon("play").text("Сейчас мир: план ждёт войны. Когда она начнётся, «Исполнить» — и группа пойдёт по стреле.").note("клавиша Enter")
                            } else if draft {
                                Card::new("Исполнить план").icon("play").text("Группа начинает наступление по стреле: часть её дивизий бьёт, остальные держат линию.").note("клавиша Enter")
                            } else if attacking {
                                Card::new("Остановить наступление").icon("stop").text("Группа перестаёт наступать и держит свой участок.").note("клавиша Enter")
                            } else {
                                Card::new("Исполнить план").icon("play").text("Сначала проведите стрелу наступления (A).").note("клавиша Enter")
                            }
                        })
                        .clicked()
                    {
                        todo.push("go");
                    }
                    if order(ui, &ch, &s, "wall", "В глубину", "D", gr.order == Order::Deep, true)
                        .tip(|| Card::new("Оборона в глубину").icon("wall").text("Вторая линия за первой, почти такая же сильная; обе окапываются быстрее. Нажмите ещё раз — снова держать участок.").note("клавиша D"))
                        .clicked()
                    {
                        todo.push("deep");
                    }
                    if order(ui, &ch, &s, "staff", if gr.staff { "У генштаба" } else { "Генштабу" }, "G", gr.staff, true)
                        .tip(|| {
                            if gr.staff {
                                Card::new("Группу ведёт генштаб").icon("staff").text("Генштаб сам решает, держать ей участок или наступать, и сколько у неё дивизий. Нажмите — группа ваша: она делает то же, что сейчас, пока не прикажете другое.").note("клавиша G; любой ваш приказ тоже забирает её у генштаба")
                            } else {
                                Card::new("Отдать генштабу").icon("staff").text("Генштаб снова сам решает, держать группе участок или наступать, и сколько у неё дивизий.").note("клавиша G")
                            }
                        })
                        .clicked()
                    {
                        todo.push("staff");
                    }
                    let fed = g.h.war.corps.as_ref().is_some_and(|c| c.muster_group[me] == Some(k));
                    if order(ui, &ch, &s, "rally", "Пополнение", "R", fed, corps)
                        .tip(|| {
                            if fed {
                                Card::new("Новые дивизии — сюда").icon("rally").text("Все новые дивизии идут в эту группу. Нажмите ещё раз — группы им снова даёт генштаб.").note("клавиша R; место сбора — в папке «Армия»")
                            } else {
                                Card::new("Пополнение").icon("rally").text("Все новые дивизии пойдут в эту группу (где они собираются — в папке «Армия», «Место сбора»). Группа станет вашей: генштаб не будет забирать у неё дивизии.").note("клавиша R")
                            }
                        })
                        .clicked()
                    {
                        todo.push("feed");
                    }
                    if order(ui, &ch, &s, "army", "Дивизии", "S", false, corps)
                        .tip(|| Card::new("Выбрать дивизии группы").icon("army").text("Все её дивизии выбраны: правой кнопкой по карте — вести их самому.").note("клавиша S; двойной щелчок по карточке — то же"))
                        .clicked()
                    {
                        todo.push("pick");
                    }
                });
            });
            let r = fr.response.rect;
            let mut v = chrome::Out::new();
            hud::plate(&mut v, r, &ch, 4.0);
            // (the group's colour down the left edge)
            v.push(Shape::rect_filled(Rect::from_min_size(r.min + vec2(4.0, 4.0), vec2(5.0, r.height() - 8.0)), 1.0, hue));
            chrome::under(ui, back, v);
        });
        for t in todo {
            self.plan_order(g, &ks, t);
        }
    }

    /// The chosen groups: the first chosen, then those added with Shift (that still exist).
    pub(super) fn groups_chosen(&self, g: &Game) -> Vec<usize> {
        let n = g.h.war.command[g.me].as_ref().map_or(0, |v| v.len());
        let mut v: Vec<usize> = self.chosen.into_iter().filter(|&k| k < n).collect();
        for &k in &self.more {
            if k < n && !v.contains(&k) {
                v.push(k);
            }
        }
        v
    }

    /// A group added to the chosen or taken out of them (Shift); the first chosen when none was.
    pub(super) fn toggle_group(&mut self, k: usize) {
        match self.chosen {
            None => {
                self.chosen = Some(k);
                self.more.clear();
            }
            Some(c) if c == k => {
                // (the first taken out: the next added becomes first)
                self.chosen = if self.more.is_empty() { None } else { Some(self.more.remove(0)) };
            }
            Some(_) => {
                if let Some(i) = self.more.iter().position(|&j| j == k) {
                    self.more.remove(i);
                } else {
                    self.more.push(k);
                }
            }
        }
    }

    /// An order of the bar (or its key) to the chosen groups `ks` (the first leads: what it does
    /// decides a switch, as deep defence).
    fn plan_order(&mut self, g: &mut Game, ks: &[usize], what: &str) {
        let me = g.me;
        let Some(&k) = ks.first() else { return };
        let Some(gr) = g.h.war.command[me].as_ref().and_then(|gs| gs.get(k)).cloned() else { return };
        let form = |j: usize, o: &str| Act::Form(vec![("grp".into(), j.to_string()), ("order".into(), o.into())]);
        let attack = |j: usize, q: usize, g: &Game| Act::Form(vec![("grp".into(), j.to_string()), ("order".into(), "attack".into()), ("target".into(), g.h.war.th.provs[q].to_string())]);
        let Some(m) = self.map.as_mut() else { return };
        match what {
            "front" if g.h.war.corps.is_some() => {
                m.tool = if matches!(&m.tool, Tool::Front { gs, .. } if gs == ks) { Tool::None } else { Tool::Front { gs: ks.to_vec(), drawn: Vec::new() } };
                m.picked.clear();
            }
            "attack" => {
                m.tool = if m.tool == (Tool::Attack { gs: ks.to_vec() }) { Tool::None } else { Tool::Attack { gs: ks.to_vec() } };
                m.picked.clear();
            }
            "go" if !g.h.at_war => {}
            "go" => {
                for &j in ks {
                    let attacking = g.h.war.command[me].as_ref().and_then(|gs| gs.get(j)).is_some_and(|x| matches!(x.order, Order::Attack(_)));
                    if let Some(i) = m.drafts.iter().position(|d| d.0 == j) {
                        let (_, q) = m.drafts.remove(i);
                        g.act(attack(j, q, g));
                    } else if attacking {
                        g.act(form(j, "hold"));
                    }
                }
            }
            "deep" => {
                let o = if gr.order == Order::Deep { "hold" } else { "deep" };
                for &j in ks {
                    g.act(form(j, o));
                }
            }
            "staff" => {
                for &j in ks {
                    let Some(x) = g.h.war.command[me].as_ref().and_then(|gs| gs.get(j)).cloned() else { continue };
                    if !x.staff {
                        g.act(form(j, "staff"));
                        m.drafts.retain(|d| d.0 != j);
                    } else {
                        // (taken from the staff: the same order, his now)
                        match x.order {
                            Order::Hold => g.act(form(j, "hold")),
                            Order::Deep => g.act(form(j, "deep")),
                            Order::Attack(q) => g.act(attack(j, q, g)),
                        };
                    }
                }
            }
            "feed" => {
                let now = g.h.war.corps.as_ref().and_then(|c| c.muster_group[me]);
                g.act(Act::Form(vec![("muster_grp".into(), if now == Some(k) { "-".into() } else { k.to_string() })]));
            }
            "pick" => {
                if let Some(c) = g.h.war.corps.as_ref() {
                    m.picked = c.divs.iter().filter(|d| d.side == me && ks.contains(&d.group)).map(|d| d.id).collect();
                    if !m.picked.is_empty() {
                        self.chosen = None;
                        self.more.clear();
                    }
                }
            }
            _ => {}
        }
    }

    /// The keys of the war (not while words are typed): 1–9 choose a group (twice — the map goes
    /// to it), Ctrl with a number gives it the chosen divisions, Tab the next group; + and − the
    /// clock's speed; the chosen divisions' orders (H stand, G back to their generals, N a new
    /// group of them); the chosen group's plans (F the front, A the attack, Enter carry it out or
    /// stop, D deep defence, G back to the staff, S its divisions).
    pub(super) fn war_keys(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() || super::spoiled() {
            return;
        }
        let k: Keys = ctx.input(|i| {
            use egui::Key::*;
            let num = [Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9].iter().position(|x| i.key_pressed(*x));
            let ctrl = i.modifiers.command;
            Keys {
                group: num.filter(|_| !ctrl),
                add: i.modifiers.shift,
                to_group: num.filter(|_| ctrl),
                next: i.key_pressed(Tab),
                faster: i.key_pressed(Plus) || i.key_pressed(Equals),
                slower: i.key_pressed(Minus),
                halt: i.key_pressed(H),
                back: i.key_pressed(G),
                raise: i.key_pressed(N),
                front: i.key_pressed(F),
                attack: i.key_pressed(A),
                go: i.key_pressed(Enter),
                deep: i.key_pressed(D),
                pick: i.key_pressed(S),
                feed: i.key_pressed(R),
            }
        });
        let shared = self.shared.clone();
        let mut g = shared.lock().unwrap();
        if k.faster {
            g.speed = (g.speed + 1).min(5);
        }
        if k.slower {
            g.speed = g.speed.saturating_sub(1).max(1);
        }
        let groups = g.h.war.command[g.me].as_ref().map_or(0, |v| v.len());
        let picked: Vec<u32> = self.map.as_ref().map_or(Vec::new(), |m| m.picked.clone());
        // the chosen divisions' orders
        if !picked.is_empty() {
            if k.halt {
                g.act(Act::Halt(picked.clone()));
            }
            if k.back {
                self.back_to_generals(&mut g, &picked);
            }
            if k.raise && groups > 0 && g.act(Act::Raise(picked.clone())) {
                self.chosen = g.h.war.command[g.me].as_ref().map(|v| v.len() - 1);
                if let Some(m) = self.map.as_mut() {
                    m.picked.clear();
                }
            }
            if let Some(t) = k.to_group.filter(|&t| t < groups) {
                g.act(Act::Assign(picked.clone(), t));
            }
        }
        // the groups
        if groups > 0 {
            let pick = k.group.filter(|&t| t < groups).or_else(|| k.next.then(|| self.chosen.map_or(0, |c| (c + 1) % groups)));
            if let (Some(t), true) = (pick, k.add && k.group.is_some()) {
                self.toggle_group(t);
                if let Some(m) = self.map.as_mut() {
                    m.picked.clear();
                }
            } else if let Some(t) = pick {
                self.more.clear();
                let now = ctx.input(|i| i.time);
                let again = self.chosen == Some(t) && now - self.chose_at < 0.45;
                self.chosen = Some(t);
                self.chose_at = now;
                if let Some(m) = self.map.as_mut() {
                    m.picked.clear();
                    if again {
                        // (twice: the map goes to the middle of its stretch of the front)
                        let parts = g.h.war.sectors_of(g.me);
                        if let Some(&q) = parts.get(t).and_then(|p| p.get(p.len() / 2)) {
                            m.look_at(&g, g.h.war.th.provs[q]);
                        }
                    }
                }
            }
            if let (true, true) = (self.chosen.is_some(), picked.is_empty()) {
                let ks = self.groups_chosen(&g);
                for (on, what) in [(k.front, "front"), (k.attack, "attack"), (k.go, "go"), (k.deep, "deep"), (k.back, "staff"), (k.pick, "pick"), (k.feed, "feed")] {
                    if on {
                        self.plan_order(&mut g, &ks, what);
                    }
                }
            }
        }
    }

    /// Each of the divisions back under the general of its own group (or the staff, in peace).
    pub(super) fn back_to_generals(&mut self, g: &mut Game, ids: &[u32]) {
        let Some(c) = g.h.war.corps.as_ref() else { return };
        let mut by: Vec<(usize, Vec<u32>)> = Vec::new();
        for &id in ids {
            let Some(i) = c.find(id) else { continue };
            let k = c.divs[i].group;
            match by.iter_mut().find(|x| x.0 == k) {
                Some(x) => x.1.push(id),
                None => by.push((k, vec![id])),
            }
        }
        for (k, ids) in by {
            g.act(Act::Assign(ids, k));
        }
    }
}

/// The two bars of a group's divisions on its card (strength green, organisation yellow), upright
/// at its right edge.
pub fn upright(v: &mut chrome::Out, r: Rect, strength: f32, org: f32) {
    for (i, (x, col)) in [(strength, Color32::from_rgb(112, 196, 84)), (org, Color32::from_rgb(236, 200, 64))].into_iter().enumerate() {
        let b = Rect::from_min_size(pos2(r.min.x + i as f32 * 8.0, r.min.y), vec2(5.0, r.height()));
        v.push(Shape::rect_filled(b.expand(1.0), 1.0, Color32::from_black_alpha(200)));
        let h = b.height() * x.clamp(0.0, 1.0);
        v.push(Shape::rect_filled(Rect::from_min_max(pos2(b.min.x, b.max.y - h), b.max), 0.5, col));
    }
}
