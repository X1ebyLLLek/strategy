//! «Как играть» (29th session; his words: «игрок должен понимать, как играть, во что, как
//! управлять»): a sheet over the map that tells in a few steps what the screen is made of and
//! what to do first — the clock, the reports, the folders, the four words of the bar, the war —
//! then the map's hand, what the signs mean, the keys. Opens by itself the first time the game
//! is played, then by the compass in the bar or F1.

use super::*;
use eframe::egui::{pos2, vec2, FontFamily, FontId, Rect, Sense, Stroke};

/// The pages of the guide.
pub(super) const PAGES: [&str; 4] = ["Начало", "Карта и война", "Знаки", "Клавиши"];

/// A step: its number in a seal, its heading, what to do, in plain words.
struct Step {
    icon: &'static str,
    head: &'static str,
    text: &'static str,
}

const START: [Step; 5] = [
    Step { icon: "play", head: "Пустите время", text: "Пробел или большая кнопка справа вверху. Месяцы идут сами; чёрточки под датой — скорость (клавиши + и −). Игра останавливается сама, когда случилось важное." },
    Step { icon: "scroll", head: "Читайте «Доклады»", text: "Справа под полосой: цель страны и то, что важно сейчас — что случилось, почему и в каких числах. Щёлкните доклад: откроется нужная страница, а нужный рычаг подсветится." },
    Step { icon: "econ", head: "Правьте страну папками", text: "Значки вверху слева: Страна · Экономика, Производство, Наука · Армия, Флот · Политика, Общество, Деятели · Сводка, События. Папка открывается слева; Esc закрывает её." },
    Step { icon: "chart", head: "Следите за четырьмя словами", text: "Народ, Казна, Власть и (в войну) Снабжение — в верхней полосе. Зелёный — хорошо, жёлтый — внимание, красный — беда. Щёлкните слово — откроется папка, где это чинят." },
    Step { icon: "army", head: "Война идёт армиями", text: "Внизу карточки армий. Выберите армию, затем F — линия фронта (проведите вдоль границы), A — стрела наступления, Enter — исполнить. Остальное делает генерал и штаб." },
];

const MAP: [Step; 5] = [
    Step { icon: "compass", head: "Двигайте карту", text: "Перетащите карту (в войну — правой кнопкой: левая выбирает войска). Колесо — ближе и дальше. Щелчок по маленькой карте в углу переносит туда вид." },
    Step { icon: "binoculars", head: "Режимы карты", text: "В правом нижнем углу, две строки значков: Страны, Рельеф, Снабжение, Развитие, Люди, Недра, Былое, Веры, Строй, Отношения, Армии, Моря. Над ними написано, какой показан, и что значат цвета." },
    Step { icon: "pointing", head: "Подсказки на всём", text: "Задержите руку над чем угодно — значком, словом, плиткой, провинцией: появится карточка с объяснением и числами. Подсказка в «Настройках» — как скоро она приходит." },
    Step { icon: "helmet", head: "Войска на карте", text: "Фишка — стопка дивизий: число — сколько их. Щелчок или рамка выбирает, правая кнопка — идти или атаковать. Красная линия — фронт, зелёная — наша сторона." },
    Step { icon: "stop", head: "Если запутались", text: "Esc: отменить план, снять выбор, закрыть папку, потом — меню. Пробел остановит время в любой миг. Сохранить игру можно в меню; раз в год игра сохраняется сама." },
];

/// What the signs mean: a sign, its meaning.
const SIGNS: [(&str, &str, &str); 9] = [
    ("people", "Народ", "настроение людей: «спокоен», «ворчит», «зол». Злой народ бастует и бунтует."),
    ("coins", "Казна", "сколько денег в казне и итог месяца. Красная казна — тратите больше, чем берёте."),
    ("crown", "Власть", "насколько прочно правительство. Слабая власть — перевороты и мятежи."),
    ("crate", "Снабжение", "сколько нужного доходит до войск. Голодная армия слабее и тает."),
    ("alarm", "В мире", "напряжённость между странами: растёт, когда кто-то готовит войну."),
    ("hourglass", "Пять полосок", "рядом со словом: целиком светятся — всё хорошо; стрелка — как менялось за три месяца."),
    ("alarm", "Красная точка на папке", "в этой папке что-то требует вашего внимания — загляните в «Доклады»."),
    ("army", "Карточка армии", "портрет генерала, приказ, пять делений снабжения, число дивизий; две полоски справа — численность и организация."),
    ("staff", "Резерв и флот", "дивизии вне армий (их бросают, где прорыв) и корабли; щелчок открывает папку."),
];

impl App {
    /// The guide over the map, when it is open: a sheet in the middle with its pages.
    pub(super) fn guide(&mut self, ctx: &egui::Context, g: &mut Game) {
        if !self.guide_open {
            return;
        }
        let s = self.skin;
        let ch = self.chrome.clone();
        let shadow = ctx.style().visuals.window_shadow;
        let mut close = false;
        let mut page = self.guide_page;
        let screen = ctx.screen_rect();
        let w = (screen.width() - 80.0).clamp(480.0, 700.0);
        let body_h = (screen.height() - 250.0).clamp(220.0, 500.0);
        egui::Window::new("КАК ИГРАТЬ")
            .id(egui::Id::new("guide"))
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 10.0])
            .collapsible(false)
            .resizable(false)
            .fixed_size([w, body_h + 168.0])
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(26.0, 20.0)).shadow(shadow).rounding(3.0))
            .show(ctx, |ui| {
                let back = chrome::hold(ui);
                ui.set_width(w - 52.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("КАК ИГРАТЬ").family(FontFamily::Name("head".into())).size(26.0).color(s.accent));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if chrome::press(ui, &ch, &s, chrome::Press { text: "×", size: vec2(30.0, 28.0), font: 20.0, ..chrome::Press::default() }).tip(|| Card::new("Закрыть").text("Esc или F1 — то же самое.")).clicked() {
                            close = true;
                        }
                    });
                });
                ch.heading(ui, &s, "");
                if let Some(k) = widgets::switch(ui, &PAGES, page) {
                    page = k;
                }
                ui.add_space(8.0);
                let inner_w = ui.available_width();
                widgets::scroll(ui, &format!("guide-scroll-{page}"), body_h, false, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    match page {
                        0 => {
                            ui.label(RichText::new(format!("Вы правите страной «{}»: деньги, люди, армия и отношения с соседями. Победа — выжить и стать сильнее; как — выбирать вам.", g.h.home(g.me).name)).size(14.5).color(s.ink));
                            for (i, st) in START.iter().enumerate() {
                                step(ui, &ch, &s, i + 1, st, inner_w);
                            }
                        }
                        1 => {
                            for (i, st) in MAP.iter().enumerate() {
                                step(ui, &ch, &s, i + 1, st, inner_w);
                            }
                        }
                        2 => {
                            for (icon, head, text) in SIGNS {
                                sign(ui, &ch, &s, icon, head, text, inner_w);
                            }
                        }
                        _ => {
                            ui.spacing_mut().item_spacing.y = 3.0;
                            command::keys_page(ui, &ch, &s);
                        }
                    }
                });
                ui.add_space(6.0);
                ch.heading(ui, &s, "");
                ui.horizontal(|ui| {
                    if chrome::button(ui, &ch, &s, if page + 1 < PAGES.len() { "Дальше" } else { "Понятно, играть" }, vec2(170.0, 32.0), false, true).clicked() {
                        if page + 1 < PAGES.len() {
                            page += 1;
                        } else {
                            close = true;
                        }
                    }
                    if page + 1 < PAGES.len() && chrome::button(ui, &ch, &s, "Играть", vec2(110.0, 32.0), false, false).clicked() {
                        close = true;
                    }
                });
                let mut v = chrome::Out::new();
                ch.sheet(&mut v, ui.min_rect().expand2(vec2(26.0, 20.0)), 8.0);
                chrome::under_window(ui, back, v);
            });
        self.guide_page = page;
        if close {
            self.guide_open = false;
        }
    }
}

/// A numbered step: the number in a seal, its heading in bold, its words under it.
fn step(ui: &mut egui::Ui, ch: &chrome::Chrome, s: &Skin, n: usize, st: &Step, w: f32) {
    let text_w = w - 64.0;
    let gal = ui.painter().layout(st.text.to_string(), FontId::proportional(14.0), s.ink, text_w);
    let head = ui.painter().layout_no_wrap(st.head.to_string(), FontId::new(16.0, FontFamily::Name("bold".into())), s.ink);
    let h = (head.size().y + gal.size().y + 12.0).max(52.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    if !ui.is_rect_visible(r) {
        return;
    }
    let c = pos2(r.min.x + 24.0, r.min.y + 26.0);
    hud::seal(ui, ch, c, 20.0, st.icon, ch.trim);
    // (its number on a small cap at the seal's corner)
    let cap = Rect::from_center_size(c + vec2(15.0, -15.0), vec2(18.0, 18.0));
    ui.painter().circle_filled(cap.center(), 9.0, s.accent);
    ui.painter().circle_stroke(cap.center(), 9.0, Stroke::new(1.0, ch.trim));
    ui.painter().text(cap.center(), egui::Align2::CENTER_CENTER, n.to_string(), FontId::new(12.0, FontFamily::Name("bold".into())), s.on_accent());
    let x = r.min.x + 56.0;
    ui.painter().galley(pos2(x, r.min.y + 2.0), head, s.ink);
    let hy = r.min.y + 2.0 + 20.0;
    ui.painter().galley(pos2(x, hy), gal, s.ink);
}

/// A sign and its meaning in one line or two.
fn sign(ui: &mut egui::Ui, ch: &chrome::Chrome, s: &Skin, icon: &str, head: &str, text: &str, w: f32) {
    let text_w = w - 64.0;
    let gal = ui.painter().layout(text.to_string(), FontId::proportional(14.0), s.ink, text_w);
    let h = (gal.size().y + 24.0).max(44.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    if !ui.is_rect_visible(r) {
        return;
    }
    hud::seal(ui, ch, pos2(r.min.x + 22.0, r.min.y + 22.0), 17.0, icon, ch.trim);
    let x = r.min.x + 52.0;
    let hg = ui.painter().layout_no_wrap(head.to_string(), FontId::new(15.0, FontFamily::Name("bold".into())), s.ink);
    ui.painter().galley(pos2(x, r.min.y + 1.0), hg, s.ink);
    ui.painter().galley(pos2(x, r.min.y + 20.0), gal, s.ink);
}
