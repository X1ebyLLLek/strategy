//! The game in a window of its own (`play --native`, built with `--features native`), drawn by the
//! video card (egui): the map on the whole window, the country's bar on top (the flag, the spheres
//! of the state, how the country lives in words with their icons, the clock), one folder on the
//! left for the sphere that is open (it slides out of the window's edge), the groups of armies
//! along the bottom in a war, an event as a sheet in the middle of the window (the clock waits for
//! a choice); every tooltip a card of the era (`chrome::tip_at`), the cursor the era's own
//! (`cursor`). The world lives in `Game` (its clock runs in its own thread); the window only reads it and sends it decisions.
//! `--shot FILE [--tab NAME] [--frames N]` — a picture of the window into FILE, then it closes
//! (to look at the window without playing).

mod armybar;
mod briefing;
mod charts;
mod chrome;
mod command;
mod cursor;
mod designer;
mod flags;
mod frame;
mod guide;
mod hud;
mod kit;
mod look;
mod map;
mod panels;
mod realm;
mod reports;
mod screens;
mod techtree;
mod terms;
mod top;
mod uitest;
mod units;
mod widgets;

pub use uitest::uitest;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use eframe::egui::{self, Color32, RichText};

use chrome::{Card, Tip};
use look::Skin;
use crate::Game;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Nation,
    Econ,
    Arms,
    Science,
    Army,
    Navy,
    Politics,
    People,
    Society,
    Ledger,
    News,
    Battle(usize),
}

/// The page of a folder open now (28th session: the economy's and the army's folders in pages —
/// one scroll of four screens hid the armies at its foot).
fn page_id(tab: Tab) -> egui::Id {
    egui::Id::new(("folder-page", match tab {
        Tab::Econ => 1,
        Tab::Army => 2,
        Tab::Nation => 3,
        Tab::Politics => 4,
        Tab::Society => 5,
        _ => 0,
    }))
}

fn page_of(ctx: &egui::Context, tab: Tab) -> usize {
    ctx.data(|d| d.get_temp::<usize>(page_id(tab))).unwrap_or(0)
}

fn set_page(ctx: &egui::Context, tab: Tab, k: usize) {
    ctx.data_mut(|d| d.insert_temp(page_id(tab), k));
}

impl Tab {
    /// A folder by its name (`army:2` — its third page).
    fn of(name: &str) -> Option<Tab> {
        let name = name.split(':').next().unwrap_or(name);
        Some(match name {
            "nation" => Tab::Nation,
            "econ" => Tab::Econ,
            "arms" => Tab::Arms,
            "science" => Tab::Science,
            "army" => Tab::Army,
            "navy" => Tab::Navy,
            "politics" => Tab::Politics,
            "news" => Tab::News,
            "people" => Tab::People,
            "society" => Tab::Society,
            "ledger" => Tab::Ledger,
            // (for pictures: the latest battle explained)
            "battle" => Tab::Battle(usize::MAX),
            _ => return None,
        })
    }
}

/// A picture of the window to take: into which file, after how many frames.
pub struct Shot {
    pub file: PathBuf,
    pub frames: u32,
    pub tab: Option<String>,
    pub zoom: f32,
    pub mode: Option<String>,
    /// A template open in the designer.
    pub design: Option<usize>,
    /// The menu open.
    pub menu: bool,
    /// Where the view looks (world pixels `X,Y`, or `high` — the highest land of the war).
    pub center: Option<String>,
    /// Where the hand rests (points `X,Y`; its card and the cursor in the picture), and whether
    /// the cursors of the era are laid out in a row (to look at them).
    pub hover: Option<(f32, f32)>,
    pub cursors: bool,
    /// The window's size (points), if not the usual 1600×900.
    pub size: Option<(f32, f32)>,
    /// The largest stack of our divisions on the screen chosen (their panel, their ways).
    pub pick: bool,
    /// A group chosen (its plans over the cards); an arrow of its attack drawn and not carried
    /// out (at the enemy's province nearest the middle of its stretch).
    pub choose: Option<usize>,
    pub draft: bool,
    /// The fronts shown off: the first two groups on the whole front with the enemy, the third
    /// facing another neighbour; the tool of the front in hand.
    pub fronts: bool,
    /// More folders after the first, each in a picture of its own (`FILE_econ_1.png`…): one world,
    /// one start of the window for a whole round of pictures (29th session).
    pub tour: Vec<String>,
    /// The guide open at a page.
    pub guide: Option<usize>,
}

/// The window spoiled on purpose (`SPOIL=1`), for the negative control of its self-checks: the
/// map is not fitted again when a folder opens, the picture is always painted whole.
pub fn spoiled() -> bool {
    SPOILED.load(std::sync::atomic::Ordering::Relaxed) || std::env::var("SPOIL").is_ok_and(|v| v == "1")
}

static SPOILED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The folder's width (points).
const FOLDER: f32 = 430.0;

struct App {
    shared: Arc<Mutex<Game>>,
    skin: Skin,
    theme: String,
    icons: HashMap<String, Arc<[u8]>>,
    chrome: chrome::Chrome,
    map: Option<map::MapView>,
    tab: Option<Tab>,
    /// The folder last open (drawn while it slides back into the window's edge).
    shown_tab: Option<Tab>,
    chosen: Option<usize>,
    /// When a group was last chosen by its key (twice quickly: the map goes to it); the menu's
    /// page of keys open.
    chose_at: f64,
    keys_open: bool,
    /// The other chosen groups (added with Shift): the orders of the bar go to all of them.
    more: Vec<usize>,
    shot: Option<Shot>,
    frame: u32,
    asked: bool,
    /// The folders still to be taken in a round of pictures, and the one being taken (none: the first).
    tour: std::collections::VecDeque<String>,
    tour_now: Option<String>,
    /// A new world being built (in its own thread), and whether the player is asked to confirm it.
    building: Option<Building>,
    confirm_new: bool,
    /// The menu open (save, load, a new world, leave); its list of saves open; what it says (a
    /// save written, a load that failed); the month of the last autosave.
    menu: bool,
    loading: bool,
    said: Option<String>,
    autosaved: u32,
    /// The division template open in the designer.
    edit: Option<designer::Edit>,
    /// Where the groups' cards were drawn (for the self-checks).
    cards: Vec<egui::Rect>,
    /// When the last frame began, and the gaps between frames (seconds; for pictures of the window).
    last: Option<std::time::Instant>,
    gaps: Vec<f32>,
    /// How long each frame took the processor to make (seconds; for pictures of the window).
    cpu: Vec<f32>,
    /// The start of a game: the player chooses the age, the map and his country over the map
    /// of the world; the settings window open; the settings.
    start: bool,
    settings_open: bool,
    settings: Settings,
    /// How the country lived month by month, as the bar shows it (anger, output, stability,
    /// supply, the world's unease): its arrows compare with three months back.
    hist: Vec<(u32, [f32; 5])>,
    /// How many lines of the news the player has seen (the newspaper's medallion is marked while
    /// hot news waits unread).
    news_seen: usize,
    /// The war's briefing over the map unfolded (29th session).
    brief_open: bool,
    /// The guide (F1) open, and its page.
    guide_open: bool,
    guide_page: usize,
}

/// A mark on a sphere's medallion: its folder and how grave (from the reports, 28th session).
struct Alert {
    tone: u8,
    tab: Tab,
}

/// The marks of the moment: every report's folder and gravity.
fn alerts(g: &Game) -> Vec<Alert> {
    reports::reports(g, usize::MAX).into_iter().map(|r| Alert { tone: r.tone, tab: r.tab }).collect()
}

/// The window's settings (kept in `saves/settings.txt`): the scale of the interface, the full
/// screen, how long the hand rests before a card comes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub scale: f32,
    pub full: bool,
    pub tip: f64,
    /// The guide has been shown once (it opens by itself only the first time).
    pub guide_seen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { scale: 1.0, full: false, tip: chrome::TIP_WAIT, guide_seen: true }
    }
}

impl Settings {
    fn file() -> PathBuf {
        PathBuf::from(crate::tools::saves::DIR).join("settings.txt")
    }

    pub fn load() -> Settings {
        let mut s = Settings::default();
        // (no file yet: the first time the game is played)
        s.guide_seen = false;
        if let Ok(text) = std::fs::read_to_string(Self::file()) {
            for l in text.lines() {
                match l.split_once('=') {
                    Some(("scale", v)) => s.scale = v.trim().parse().unwrap_or(1.0f32).clamp(0.7, 1.6),
                    Some(("full", v)) => s.full = v.trim() == "1",
                    Some(("guide", v)) => s.guide_seen = v.trim() == "1",
                    Some(("tip", v)) => s.tip = v.trim().parse().unwrap_or(chrome::TIP_WAIT).clamp(0.0, 2.0),
                    _ => {}
                }
            }
        }
        s
    }

    fn save(&self) {
        let _ = std::fs::create_dir_all(crate::tools::saves::DIR);
        let _ = std::fs::write(Self::file(), format!("scale={}\nfull={}\ntip={}\nguide={}\n", self.scale, self.full as u8, self.tip, self.guide_seen as u8));
    }

    /// The settings put into the window.
    fn apply(&self, ctx: &egui::Context) {
        ctx.set_zoom_factor(self.scale);
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("tip-wait"), self.tip));
    }
}

impl App {
    fn new(shared: Arc<Mutex<Game>>, ctx: &egui::Context, theme: &str, shot: Option<Shot>) -> App {
        let tour: std::collections::VecDeque<String> = shot.as_ref().map_or_else(Default::default, |x| x.tour.iter().cloned().collect());
        look::fonts(ctx, theme);
        let skin = Skin::of(theme);
        look::style(ctx, &skin);
        let chrome = chrome::Chrome::load(ctx, theme);
        let icons = look::icons();
        chrome::keep(ctx, &chrome, skin, &icons);
        App {
            shared,
            skin,
            theme: theme.to_string(),
            icons,
            chrome,
            map: None,
            tab: None,
            shown_tab: None,
            chosen: None,
            chose_at: -1.0,
            keys_open: false,
            more: Vec::new(),
            shot,
            frame: 0,
            asked: false,
            tour,
            tour_now: None,
            building: None,
            confirm_new: false,
            menu: false,
            loading: false,
            said: None,
            autosaved: 0,
            edit: None,
            cards: Vec::new(),
            last: None,
            gaps: Vec::new(),
            cpu: Vec::new(),
            start: false,
            settings_open: false,
            settings: Settings::default(),
            hist: Vec::new(),
            news_seen: 0,
            brief_open: true,
            guide_open: false,
            guide_page: 0,
        }
    }

    /// The window dressed anew for another age (a world of another pack was built).
    fn restyle(&mut self, ctx: &egui::Context, theme: &str) {
        if theme == self.theme {
            return;
        }
        look::fonts(ctx, theme);
        self.skin = Skin::of(theme);
        look::style(ctx, &self.skin);
        self.chrome = chrome::Chrome::load(ctx, theme);
        chrome::keep(ctx, &self.chrome, self.skin, &self.icons);
        self.theme = theme.to_string();
    }
}

/// A world being made in a thread of its own (a new one, or a save lived again): what the window
/// says meanwhile, months lived of how many.
struct Building {
    rx: std::sync::mpsc::Receiver<Result<Game, String>>,
    title: &'static str,
    done: Arc<std::sync::atomic::AtomicU32>,
    of: u32,
}

/// The file the game saves itself into, every new year and when the window closes.
const AUTOSAVE: &str = "автосохранение";

/// The window, till it is closed.
pub fn run(shared: Arc<Mutex<Game>>, shot: Option<Shot>, start: bool) -> eframe::Result {
    let (name, theme) = {
        let g = shared.lock().unwrap();
        (g.h.home(g.me).name.clone(), g.pack.theme.clone())
    };
    let size = shot.as_ref().and_then(|s| s.size).unwrap_or((1600.0, 900.0));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([size.0, size.1]).with_min_inner_size([1100.0, 640.0]).with_title(format!("{name} — стратегия")),
        // (the land in three dimensions: a depth buffer, smooth edges of the mountains)
        depth_buffer: 24,
        // (`MSAA=0` or `STRAT_NOMSAA=1`: for a screen with no smoothing in its visuals — a virtual one taking pictures)
        multisampling: if std::env::var_os("STRAT_NOMSAA").is_some() { 0 } else { std::env::var("MSAA").ok().and_then(|v| v.parse().ok()).unwrap_or(4) },
        // (pictures measure the frames as fast as they go, not at the screen's pace)
        vsync: shot.is_none(),
        ..Default::default()
    };
    eframe::run_native(
        "strategy",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let tab = shot.as_ref().and_then(|s| s.tab.as_deref()).and_then(Tab::of);
            let page = shot.as_ref().and_then(|s| s.tab.as_deref()).and_then(|x| x.split_once(':')).and_then(|x| x.1.parse::<usize>().ok());
            let with_menu = shot.as_ref().is_some_and(|s| s.menu);
            let with_guide = shot.as_ref().and_then(|s| s.guide);
            let edit = shot.as_ref().and_then(|s| s.design).and_then(|k| {
                let g = shared.lock().unwrap();
                (k < g.templates.len()).then(|| designer::Edit::of(&g, Some(k)))
            });
            let mut app = App::new(shared, &cc.egui_ctx, &theme, shot);
            app.start = start;
            if app.shot.is_none() {
                app.settings = Settings::load();
                app.settings.apply(&cc.egui_ctx);
                if app.settings.full {
                    cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                }
            }
            app.tab = tab;
            if let (Some(t), Some(k)) = (tab, page) {
                set_page(&cc.egui_ctx, t, k);
            }
            app.menu = with_menu;
            if let Some(p) = with_guide {
                app.guide_open = true;
                app.guide_page = p;
            }
            app.keys_open = with_menu && std::env::args().any(|a| a == "--keys");
            app.loading = with_menu && !app.keys_open;
            app.edit = edit;
            Ok(Box::new(app))
        }),
    )
}

impl eframe::App for App {
    /// The window closes: the game saves itself (not a picture's game).
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.shot.is_none() {
            if let Ok(g) = self.shared.lock() {
                if g.auto.is_none() {
                    let _ = g.save_named(AUTOSAVE);
                }
            }
        }
    }

    /// (for pictures: the hand set where the picture asks, resting there)
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        if let Some((x, y)) = self.shot.as_ref().and_then(|s| s.hover) {
            raw.events.retain(|e| !matches!(e, egui::Event::PointerMoved(_) | egui::Event::PointerGone));
            raw.events.push(egui::Event::PointerMoved(egui::pos2(x, y)));
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame += 1;
        let now = std::time::Instant::now();
        if let Some(t) = self.last {
            self.gaps.push((now - t).as_secs_f32());
        }
        self.last = Some(now);
        // the world moves by itself: look again soon
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
        let t0 = std::time::Instant::now();
        self.draw(ctx);
        self.cpu.push(t0.elapsed().as_secs_f32());
        // a picture of the window, if one was asked for
        let mut next_tab: Option<String> = None;
        if let Some(shot) = &self.shot {
            if let (Some(k), 10) = (shot.choose, self.frame) {
                self.chosen = Some(k);
                if shot.draft {
                    let g = self.shared.lock().unwrap();
                    let war = &g.h.war;
                    let parts = war.sectors_of(g.me);
                    let head = parts.get(k).and_then(|p| p.get(p.len() / 2)).and_then(|&q| war.th.nbr[q].iter().copied().find(|&r| war.owner[r] != g.me));
                    if let (Some(q), Some(m)) = (head, self.map.as_mut()) {
                        m.drafts.push((k, q));
                    }
                }
            }
            if shot.fronts && self.frame == 11 {
                let mut g = self.shared.lock().unwrap();
                let foe = g.h.war.th.countries[1 - g.me] as u32;
                let mine = g.h.war.th.countries[g.me] as u32;
                let line = map::border_with(&g, foe);
                g.act(crate::tools::saves::Act::Front(vec![0, 1], None, line));
                let other = (0..g.world.provinces.len()).map(|p| map::country_now(&g, p)).find(|&c| c != foe && c != mine && c != u32::MAX && !map::border_with(&g, c).is_empty());
                if let Some(c) = other {
                    let line = map::border_with(&g, c);
                    g.act(crate::tools::saves::Act::Front(vec![2], Some(c), line));
                }
                self.chosen = Some(0);
                self.more = vec![1];
                if let Some(m) = self.map.as_mut() {
                    m.tool = map::Tool::Front { gs: vec![0, 1], drawn: Vec::new() };
                }
            }
            if shot.pick && self.frame == 12 {
                if let Some(m) = self.map.as_mut() {
                    let best = m.stacks_drawn.lock().unwrap().iter().max_by_key(|x| x.1.len()).map(|x| x.1.clone());
                    if let Some(ids) = best {
                        m.picked = ids;
                    }
                }
            }
            if shot.cursors {
                // (the era's cursors in a row, on the map, to look at)
                let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Debug, egui::Id::new("cursor-sheet")));
                for (i, k) in cursor::Kind::ALL.iter().enumerate() {
                    let r = egui::Rect::from_min_size(egui::pos2(520.0 + i as f32 * 70.0, 300.0), egui::vec2(64.0, 64.0));
                    if let Ok(egui::load::TexturePoll::Ready { texture }) = cursor::image(&self.theme, *k).load_for_size(ctx, r.size()) {
                        p.image(texture.id, r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                    }
                }
            }
            if self.frame == shot.frames && !self.asked {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
                self.asked = true;
            }
            let got = ctx.input(|i| i.raw.events.iter().find_map(|e| if let egui::Event::Screenshot { image, .. } = e { Some(image.clone()) } else { None }));
            if let Some(img) = got {
                let rgb: Vec<u8> = img.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect();
                let path = match &self.tour_now {
                    Some(label) => {
                        let stem = shot.file.file_stem().map_or(String::new(), |x| x.to_string_lossy().into_owned());
                        shot.file.with_file_name(format!("{stem}_{}.png", label.replace(':', "_")))
                    }
                    None => shot.file.clone(),
                };
                let _ = std::fs::write(&path, strategy::png::encode_rgb(img.size[0], img.size[1], &rgb));
                println!("снимок окна: {}", path.display());
                if let Some(m) = &self.map {
                    for (how, n, t, a) in &m.painted {
                        println!("  в видеокарту: {how} (сменили хозяина {n} провинций), {:.3} с, {:.2}% слоёв земли", t, a * 100.0);
                    }
                }
                let mut gaps = self.gaps.clone();
                gaps.sort_by(|a, b| a.total_cmp(b));
                let mut worst: Vec<(usize, f32)> = self.gaps.iter().copied().enumerate().collect();
                worst.sort_by(|a, b| b.1.total_cmp(&a.1));
                println!("  долгие кадры (номер, с): {:?}", worst.iter().take(6).map(|(i, t)| (i + 2, (t * 1000.0).round() / 1000.0)).collect::<Vec<_>>());
                let mut cpu = self.cpu.clone();
                cpu.sort_by(|a, b| a.total_cmp(b));
                if !cpu.is_empty() {
                    println!("  кадр на процессоре (без видеокарты): медиана {:.4} с, 95% {:.4} с", cpu[cpu.len() / 2], cpu[cpu.len() * 95 / 100]);
                }
                if let Some(&most) = gaps.last() {
                    println!("  кадров {}, между кадрами: медиана {:.3} с, 95% {:.3} с, самое большее {:.3} с", gaps.len() + 1, gaps[gaps.len() / 2], gaps[gaps.len() * 95 / 100], most);
                }
                // (what the video card itself spent on the map: the frames of the second half —
                // the camera settled)
                if let Some(m) = &self.map {
                    let spent = m.spent();
                    let tail = &spent[spent.len() / 2..];
                    if !tail.is_empty() {
                        let med = |f: &dyn Fn(&(f32, f32)) -> f32| {
                            let mut v: Vec<f32> = tail.iter().map(f).collect();
                            v.sort_by(|a, b| a.total_cmp(b));
                            v[v.len() / 2]
                        };
                        println!("  видеокарта на карту: земля {:.2} мс, деревья и дома {:.2} мс (медиана {} кадров)", med(&|t| t.0) * 1e3, med(&|t| t.1) * 1e3, tail.len());
                    }
                }
                match self.tour.front() {
                    Some(next) => next_tab = Some(next.clone()),
                    None => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                }
            }
        }
        // (the next folder of a round of pictures: opened, waited for, taken)
        if let Some(next) = next_tab {
            self.tour.pop_front();
            if let Some(t) = Tab::of(&next) {
                self.tab = Some(t);
                if let Some(k) = next.split_once(':').and_then(|x| x.1.parse::<usize>().ok()) {
                    set_page(ctx, t, k);
                }
            }
            self.tour_now = Some(next);
            self.frame = 0;
            self.asked = false;
            self.gaps.clear();
            self.cpu.clear();
        }
    }
}

/// A window coming into view: it rises a little and grows to its size as it fades in (its layer
/// moved and scaled about its middle); called every frame, open or not, so that it knows when it
/// opens.
fn appear(ctx: &egui::Context, anim: egui::Id, window: egui::Id, open: bool) {
    let t = ctx.animate_bool_with_time(anim, open, 0.22);
    let layer = egui::LayerId::new(egui::Order::Middle, window);
    let k = 1.0 - (1.0 - t).powi(3);
    match ctx.memory(|m| m.area_rect(window)) {
        Some(r) if open && k < 1.0 => {
            let s = 0.94 + 0.06 * k;
            let c = r.center();
            let shift = egui::vec2(c.x * (1.0 - s), c.y * (1.0 - s) + (1.0 - k) * 16.0);
            ctx.set_transform_layer(layer, egui::emath::TSTransform::new(shift, s));
        }
        _ => ctx.set_transform_layer(layer, egui::emath::TSTransform::IDENTITY),
    }
}
