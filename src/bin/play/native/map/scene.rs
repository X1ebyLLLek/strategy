//! The land in three dimensions, drawn by the video card (OpenGL, inside egui's frame): a grid of
//! the world's heights lit by the sun, the sea with waves, glints and surf, the countries' colours
//! laid over the land from a small table of provinces (a month of war sends the card only that
//! table), their borders, haze far up a tilted view. What lies over it (counters, names, arrows)
//! egui draws, with the same camera.

use std::sync::{Arc, Mutex};
use eframe::egui;
use eframe::egui_glow;
use eframe::glow::{self, HasContext};

use super::figures;
use super::objects;

/// The layers of the world sent to the card once: heights, the light's normals (twice as fine),
/// the ground's colour, the provinces.
pub struct Layers {
    pub w: usize,
    pub h: usize,
    /// The land's height as it is shown (of the highest peak; a basin may lie below the sea, < 0),
    /// the water 0 (its surface).
    pub height: Vec<f32>,
    /// The world's own: land 0..1 (of its highest), water −1..0 (of the deepest deep) — where the
    /// water is and how deep.
    pub sea: Vec<f32>,
    /// The ground's normals, 2w × 2h, RGBA (xyz in 0..255; a — unused).
    pub normal: Vec<u8>,
    /// The ground's colour, RGBA (a — the kind of ground).
    pub base: Vec<u8>,
    /// The province of each pixel + 1 (0 — none), RGBA (low byte, high byte).
    pub ids: Vec<u8>,
    /// How much of each kind of ground lies at each pixel: two pictures RGBA one after the other
    /// (kinds 0–3, 4–7 of `ground::kind`).
    pub splat: Vec<u8>,
    /// The ground's photographs: 8 colours then 8 normal maps, `tex_size` square, RGBA.
    pub tex: Vec<u8>,
    pub tex_size: usize,
    /// The land seen from above (`ground::aerials`): forests broad-leaved and coniferous, a marsh.
    pub aerial: Vec<u8>,
    pub aerial_size: usize,
    /// The trees of the forests (drawn close up).
    pub trees: objects::Trees,
    /// Twice as fine as the world, RGBA: the folds of the land (128 — even; above — a crest,
    /// below — a gully), how much the land is tilled, the nearest river's half-width (255 —
    /// `RIVER_HW`).
    pub detail: Vec<u8>,
    /// Twice as fine as the world, RGBA: the steps to the nearest river's middle line and to
    /// the nearest main road (`ways::near`).
    pub lines: Vec<u8>,
    /// Twice as fine as the world, RGBA: the steps to the nearest village road and railway
    /// (the railways the players build among them).
    pub ways: Vec<u8>,
    /// Twice as fine as the world, RGBA: the sun's light (0 — in a range's shadow), the sky's
    /// (less deep in the valleys) — `ground::light`; the forests' canopy (`ground::layers`).
    pub light: Vec<u8>,
}

/// The table of provinces: 3 rows of `PAL_W` for each `PAL_W` provinces (id = province + 1):
/// row 0 — the colour laid over the land (a: 255 if any), row 1 — the colour of the stripes (a:
/// 255 if striped), row 2 — the country shown (+1, low, high byte), the side in the war (9: none),
/// a: 255 — sea.
pub const PAL_W: usize = 256;

/// What one frame of the land needs (moved into the painting).
#[derive(Clone)]
pub struct Frame {
    /// The copies of the world in view: their shift (world pixels), matrix, and the camera's
    /// place in the copy's own numbers.
    pub copies: [(f32, [f32; 16], [f32; 3]); 4],
    pub n: usize,
    pub k: f32,
    pub time: f32,
    /// How strongly the countries' colours lie over the land; borders drawn by the card (far) or
    /// not (close up, egui draws them smooth); clouds (far away).
    pub overlay: f32,
    pub borders: f32,
    pub clouds: f32,
    pub war: bool,
    /// World pixels of height of the highest peak; the camera's distance (for the haze).
    pub relief: f32,
    pub dist: f32,
    /// The stripes' width (world pixels).
    pub stripe: f32,
    /// The era's ground: 0 — as it is, 1 — the wastes (scorched, ashen), 2 — old paintings (warm).
    pub era: f32,
    /// How much the land shows its heights in colour, as an atlas does (the mode «Рельеф»), with
    /// lines of equal height.
    pub hypso: f32,
    /// The troops' figures by their stacks (filled while the frame is drawn, read when the card
    /// paints it).
    pub figures: Arc<Mutex<Vec<figures::Figure>>>,
    /// How far the trees and the houses have grown out of the land (0 — not drawn: far away), and
    /// the world's bounds seen (x not wrapped).
    pub trees: f32,
    pub houses: f32,
    pub seen: [f32; 4],
    pub focus: [f32; 3],
    /// The camera's tilt (radians from straight down): the far trees face it.
    pub pitch: f32,
    /// The time of year as the land shows it: snow (winter), yellowing (autumn), mud (spring),
    /// each 0..1.
    pub season: [f32; 3],
}

struct Gpu {
    program: glow::Program,
    vao: glow::VertexArray,
    _vbo: glow::Buffer,
    _ebo: glow::Buffer,
    count: i32,
    height: glow::Texture,
    normal: glow::Texture,
    base: glow::Texture,
    ids: glow::Texture,
    pal: glow::Texture,
    splat_a: glow::Texture,
    splat_b: glow::Texture,
    photos: glow::Texture,
    aerial: glow::Texture,
    detail: glow::Texture,
    lines: glow::Texture,
    ways: glow::Texture,
    light: glow::Texture,
    size: (usize, usize),
    /// The trees and the houses (none if the card would not take their drawing); the troops'
    /// figures.
    objects: Option<objects::Gpu>,
    figures: Option<figures::Gpu>,
    /// The card's own clocks: how long the land and the objects took (read some frames later,
    /// not to wait for the card).
    timers: Vec<[glow::Query; 2]>,
    timer_next: usize,
}

/// The land on the card: made on the first painting; the world's layers and the table of
/// provinces wait there till then.
#[derive(Default)]
pub struct Scene {
    gpu: Option<Gpu>,
    layers: Option<Arc<Layers>>,
    palette: Option<(Vec<u8>, usize)>,
    houses: Option<objects::Towns>,
    /// The ways anew (a railway or a road laid): sent with the next painting.
    ways: Option<Vec<u8>>,
    lines: Option<Vec<u8>>,
    pub failed: Option<String>,
    /// What the video card spent on each frame drawn: the land, the trees and the houses
    /// (seconds; for the pictures' measures).
    pub spent: Vec<(f32, f32)>,
}

impl Scene {
    pub fn new(layers: Arc<Layers>) -> Scene {
        Scene { layers: Some(layers), ..Default::default() }
    }

    /// A new table of provinces (sent with the next painting); its bytes.
    pub fn set_palette(&mut self, bytes: Vec<u8>, rows: usize) -> usize {
        let n = bytes.len();
        self.palette = Some((bytes, rows));
        n
    }

    /// The houses of the towns and the era's buildings (sent with the next painting).
    pub fn set_houses(&mut self, h: objects::Towns) {
        self.houses = Some(h);
    }

    /// The ways anew (sent with the next painting).
    pub fn set_ways(&mut self, ways: Vec<u8>) {
        self.ways = Some(ways);
    }

    /// The rivers and the main roads anew (sent with the next painting).
    pub fn set_lines(&mut self, lines: Vec<u8>) {
        self.lines = Some(lines);
    }

    /// The world's layers sent anew (the spoiled control: everything resent on every change).
    pub fn resend(&mut self, layers: Arc<Layers>) {
        self.layers = Some(layers);
    }
}

/// The painting of the land into `rect`, for egui to call when it paints the frame.
pub fn paint(scene: Arc<Mutex<Scene>>, rect: egui::Rect, f: Frame) -> egui::PaintCallback {
    egui::PaintCallback {
        rect,
        callback: Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
            let gl = painter.gl();
            let mut s = scene.lock().unwrap();
            if s.failed.is_some() {
                return;
            }
            unsafe {
                if s.gpu.is_none() {
                    match make(gl) {
                        Ok(g) => s.gpu = Some(g),
                        Err(e) => {
                            eprintln!("карта: видеокарта не приняла рисование: {e}");
                            s.failed = Some(e);
                            return;
                        }
                    }
                }
                if let Some(l) = s.layers.take() {
                    let g = s.gpu.as_mut().unwrap();
                    upload_layers(gl, g, &l);
                    if let Some(o) = g.objects.as_mut() {
                        objects::upload_trees(gl, o, &l.trees);
                    }
                }
                if let Some(h) = s.houses.take() {
                    if let Some(o) = s.gpu.as_mut().unwrap().objects.as_mut() {
                        objects::upload_houses(gl, o, &h);
                    }
                }
                if let Some(v) = s.lines.take() {
                    let g = s.gpu.as_ref().unwrap();
                    gl.bind_texture(glow::TEXTURE_2D, Some(g.lines));
                    gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
                    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, 2 * g.size.0 as i32, 2 * g.size.1 as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&v));
                }
                if let Some(v) = s.ways.take() {
                    let g = s.gpu.as_ref().unwrap();
                    gl.bind_texture(glow::TEXTURE_2D, Some(g.ways));
                    gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
                    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, 2 * g.size.0 as i32, 2 * g.size.1 as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&v));
                }
                if let Some((bytes, rows)) = s.palette.take() {
                    let g = s.gpu.as_ref().unwrap();
                    gl.bind_texture(glow::TEXTURE_2D, Some(g.pal));
                    gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
                    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, PAL_W as i32, rows as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&bytes));
                }
                // (the card's clocks of a frame some frames back, if they are ready)
                let g = s.gpu.as_mut().unwrap();
                let slot = g.timer_next % g.timers.len().max(1);
                let mut got = None;
                if g.timer_next >= g.timers.len() && !g.timers.is_empty() {
                    let [a, b] = g.timers[slot];
                    if gl.get_query_parameter_u32(b, glow::QUERY_RESULT_AVAILABLE) != 0 {
                        got = Some((gl.get_query_parameter_u32(a, glow::QUERY_RESULT) as f32 * 1e-9, gl.get_query_parameter_u32(b, glow::QUERY_RESULT) as f32 * 1e-9));
                    }
                }
                let timers = g.timers.get(slot).copied();
                g.timer_next += 1;
                draw(gl, s.gpu.as_ref().unwrap(), &f, timers);
                if let Some(t) = got {
                    s.spent.push(t);
                }
            }
        })),
    }
}

unsafe fn texture(gl: &glow::Context, filter: u32, wrap_s: u32) -> Result<glow::Texture, String> {
    let t = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(t));
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, if filter == glow::LINEAR_MIPMAP_LINEAR { glow::LINEAR } else { filter } as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap_s as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
    Ok(t)
}

unsafe fn compile(gl: &glow::Context, kind: u32, src: &str) -> Result<glow::Shader, String> {
    let sh = gl.create_shader(kind)?;
    gl.shader_source(sh, src);
    gl.compile_shader(sh);
    if !gl.get_shader_compile_status(sh) {
        return Err(gl.get_shader_info_log(sh));
    }
    Ok(sh)
}

/// The rows of the grid (world pixels): every 2 over the world, coarser over the open sea beyond
/// its top and bottom.
fn rows(h: usize) -> Vec<f32> {
    let h = h as f32;
    let mut v: Vec<f32> = Vec::new();
    let mut y = -h * 0.75;
    while y < -16.0 {
        v.push(y);
        y += 32.0;
    }
    let mut y = -16.0;
    while y <= h + 16.0 {
        v.push(y);
        y += 2.0;
    }
    let mut y = h + 48.0;
    while y <= h * 1.75 {
        v.push(y);
        y += 32.0;
    }
    v
}

unsafe fn make(gl: &glow::Context) -> Result<Gpu, String> {
    if std::env::var("MAPGPU").is_ok() {
        eprintln!("карта: видеокарта {} ({})", gl.get_parameter_string(glow::RENDERER), gl.get_parameter_string(glow::VERSION));
    }
    let vs = compile(gl, glow::VERTEX_SHADER, VERTEX)?;
    let fs = compile(gl, glow::FRAGMENT_SHADER, FRAGMENT)?;
    let program = gl.create_program()?;
    gl.attach_shader(program, vs);
    gl.attach_shader(program, fs);
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        return Err(gl.get_program_info_log(program));
    }
    gl.delete_shader(vs);
    gl.delete_shader(fs);
    let vao = gl.create_vertex_array()?;
    let vbo = gl.create_buffer()?;
    let ebo = gl.create_buffer()?;
    let height = texture(gl, glow::LINEAR, glow::REPEAT)?;
    let normal = texture(gl, glow::LINEAR_MIPMAP_LINEAR, glow::REPEAT)?;
    let base = texture(gl, glow::LINEAR_MIPMAP_LINEAR, glow::REPEAT)?;
    let ids = texture(gl, glow::NEAREST, glow::REPEAT)?;
    let pal = texture(gl, glow::NEAREST, glow::CLAMP_TO_EDGE)?;
    let splat_a = texture(gl, glow::LINEAR_MIPMAP_LINEAR, glow::REPEAT)?;
    let splat_b = texture(gl, glow::LINEAR_MIPMAP_LINEAR, glow::REPEAT)?;
    let detail = texture(gl, glow::LINEAR, glow::REPEAT)?;
    let lines = texture(gl, glow::LINEAR, glow::REPEAT)?;
    let ways = texture(gl, glow::LINEAR, glow::REPEAT)?;
    let light = texture(gl, glow::LINEAR_MIPMAP_LINEAR, glow::REPEAT)?;
    let aerial = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(aerial));
    gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
    let photos = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(photos));
    gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
    // (sharp when the view tilts: anisotropic filtering, where the card has it)
    let ext = gl.supported_extensions();
    if ext.contains("GL_EXT_texture_filter_anisotropic") || ext.contains("GL_ARB_texture_filter_anisotropic") {
        gl.tex_parameter_f32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MAX_ANISOTROPY_EXT, 8.0);
    }
    let objects = match objects::make(gl) {
        Ok(o) => Some(o),
        Err(e) => {
            eprintln!("карта: деревья и дома не нарисовать: {e}");
            None
        }
    };
    let figures = match figures::make(gl) {
        Ok(o) => Some(o),
        Err(e) => {
            eprintln!("карта: фигуры войск не нарисовать: {e}");
            None
        }
    };
    let mut timers = Vec::new();
    for _ in 0..4 {
        if let (Ok(a), Ok(b)) = (gl.create_query(), gl.create_query()) {
            timers.push([a, b]);
        }
    }
    if timers.len() < 4 {
        timers.clear();
    }
    Ok(Gpu { program, vao, _vbo: vbo, _ebo: ebo, count: 0, height, normal, base, ids, pal, splat_a, splat_b, photos, aerial, detail, lines, ways, light, size: (0, 0), objects, figures, timers, timer_next: 0 })
}

unsafe fn upload_layers(gl: &glow::Context, g: &mut Gpu, l: &Layers) {
    gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
    let bytes = |v: &[f32]| -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() };
    gl.bind_texture(glow::TEXTURE_2D, Some(g.height));
    // (the land as shown, and the world's own: where the water is)
    let hs: Vec<f32> = l.height.iter().zip(&l.sea).flat_map(|(&a, &b)| [a, b]).collect();
    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RG32F as i32, l.w as i32, l.h as i32, 0, glow::RG, glow::FLOAT, Some(&bytes(&hs)));
    gl.bind_texture(glow::TEXTURE_2D, Some(g.normal));
    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, 2 * l.w as i32, 2 * l.h as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&l.normal));
    gl.generate_mipmap(glow::TEXTURE_2D);
    gl.bind_texture(glow::TEXTURE_2D, Some(g.base));
    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, l.w as i32, l.h as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&l.base));
    gl.generate_mipmap(glow::TEXTURE_2D);
    gl.bind_texture(glow::TEXTURE_2D, Some(g.ids));
    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, l.w as i32, l.h as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&l.ids));
    let half = l.w * l.h * 4;
    for (t, part) in [(g.splat_a, &l.splat[..half]), (g.splat_b, &l.splat[half..])] {
        gl.bind_texture(glow::TEXTURE_2D, Some(t));
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, l.w as i32, l.h as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(part));
        gl.generate_mipmap(glow::TEXTURE_2D);
    }
    for (t, v) in [(g.detail, &l.detail), (g.lines, &l.lines), (g.ways, &l.ways), (g.light, &l.light)] {
        gl.bind_texture(glow::TEXTURE_2D, Some(t));
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, 2 * l.w as i32, 2 * l.h as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(v));
    }
    gl.bind_texture(glow::TEXTURE_2D, Some(g.light));
    gl.generate_mipmap(glow::TEXTURE_2D);
    let n = l.tex.len() / (l.tex_size * l.tex_size * 4);
    gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(g.photos));
    gl.tex_image_3d(glow::TEXTURE_2D_ARRAY, 0, glow::RGBA8 as i32, l.tex_size as i32, l.tex_size as i32, n as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&l.tex));
    gl.generate_mipmap(glow::TEXTURE_2D_ARRAY);
    let n = l.aerial.len() / (l.aerial_size * l.aerial_size * 4);
    gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(g.aerial));
    gl.tex_image_3d(glow::TEXTURE_2D_ARRAY, 0, glow::RGBA8 as i32, l.aerial_size as i32, l.aerial_size as i32, n as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&l.aerial));
    gl.generate_mipmap(glow::TEXTURE_2D_ARRAY);
    let ext = gl.supported_extensions();
    if ext.contains("GL_EXT_texture_filter_anisotropic") || ext.contains("GL_ARB_texture_filter_anisotropic") {
        gl.tex_parameter_f32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MAX_ANISOTROPY_EXT, 8.0);
    }
    if g.size != (l.w, l.h) {
        // the grid: one copy of the world
        let ys = rows(l.h);
        let xs: Vec<f32> = (0..=l.w / 2).map(|i| (i * 2) as f32).collect();
        let mut v: Vec<u8> = Vec::with_capacity(xs.len() * ys.len() * 8);
        for &y in &ys {
            for &x in &xs {
                v.extend_from_slice(&x.to_le_bytes());
                v.extend_from_slice(&y.to_le_bytes());
            }
        }
        let cols = xs.len() as u32;
        let mut e: Vec<u8> = Vec::with_capacity((xs.len() - 1) * (ys.len() - 1) * 24);
        for r in 0..ys.len() as u32 - 1 {
            for c in 0..cols - 1 {
                let a = r * cols + c;
                for i in [a, a + 1, a + cols, a + 1, a + cols + 1, a + cols] {
                    e.extend_from_slice(&i.to_le_bytes());
                }
            }
        }
        gl.bind_vertex_array(Some(g.vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(g._vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &v, glow::STATIC_DRAW);
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(g._ebo));
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, &e, glow::STATIC_DRAW);
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);
        gl.bind_vertex_array(None);
        g.count = (e.len() / 4) as i32;
        g.size = (l.w, l.h);
    }
}

/// Parts of the drawing left out, to measure what they cost (`MAPDBG`: 1 — trees and houses,
/// 2 — the ground's photographs, 4 — the new lines and fields, 8 — the sea's waves).
fn dbg() -> i32 {
    static D: std::sync::OnceLock<i32> = std::sync::OnceLock::new();
    *D.get_or_init(|| std::env::var("MAPDBG").ok().and_then(|v| v.parse().ok()).unwrap_or(0))
}

unsafe fn draw(gl: &glow::Context, g: &Gpu, f: &Frame, timers: Option<[glow::Query; 2]>) {
    if g.count == 0 {
        return;
    }
    if let Some([a, _]) = timers {
        gl.begin_query(glow::TIME_ELAPSED, a);
    }
    gl.clear_color(0.07, 0.17, 0.30, 1.0);
    gl.clear_depth_f32(1.0);
    gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
    gl.enable(glow::DEPTH_TEST);
    gl.depth_func(glow::LEQUAL);
    gl.depth_mask(true);
    gl.disable(glow::BLEND);
    gl.use_program(Some(g.program));
    let u = |name: &str| gl.get_uniform_location(g.program, name);
    for (i, (name, t)) in [("uHeight", g.height), ("uNormal", g.normal), ("uBase", g.base), ("uIds", g.ids), ("uPal", g.pal), ("uSplatA", g.splat_a), ("uSplatB", g.splat_b)].iter().enumerate() {
        gl.active_texture(glow::TEXTURE1 + i as u32);
        gl.bind_texture(glow::TEXTURE_2D, Some(*t));
        gl.uniform_1_i32(u(name).as_ref(), 1 + i as i32);
    }
    gl.active_texture(glow::TEXTURE8);
    gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(g.photos));
    gl.uniform_1_i32(u("uPhotos").as_ref(), 8);
    gl.active_texture(glow::TEXTURE13);
    gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(g.aerial));
    gl.uniform_1_i32(u("uAerial").as_ref(), 13);
    for (i, (name, t)) in [("uDetail", g.detail), ("uLines", g.lines), ("uWays", g.ways), ("uLight", g.light)].iter().enumerate() {
        gl.active_texture(glow::TEXTURE9 + i as u32);
        gl.bind_texture(glow::TEXTURE_2D, Some(*t));
        gl.uniform_1_i32(u(name).as_ref(), 9 + i as i32);
    }
    gl.uniform_2_f32(u("uSize").as_ref(), g.size.0 as f32, g.size.1 as f32);
    gl.uniform_1_f32(u("uK").as_ref(), f.k);
    gl.uniform_1_f32(u("uTime").as_ref(), f.time);
    gl.uniform_1_f32(u("uOverlay").as_ref(), f.overlay);
    gl.uniform_1_f32(u("uBorders").as_ref(), f.borders);
    gl.uniform_1_f32(u("uClouds").as_ref(), f.clouds);
    gl.uniform_1_f32(u("uWar").as_ref(), if f.war { 1.0 } else { 0.0 });
    gl.uniform_1_f32(u("uRelief").as_ref(), f.relief);
    gl.uniform_1_f32(u("uDist").as_ref(), f.dist);
    gl.uniform_1_f32(u("uStripe").as_ref(), f.stripe);
    gl.uniform_1_f32(u("uEra").as_ref(), f.era);
    gl.uniform_1_f32(u("uHypso").as_ref(), f.hypso);
    gl.uniform_1_i32(u("uDbg").as_ref(), dbg());
    gl.uniform_3_f32(u("uSeason").as_ref(), f.season[0], f.season[1], f.season[2]);
    gl.uniform_1_i32(u("uPalW").as_ref(), PAL_W as i32);
    gl.bind_vertex_array(Some(g.vao));
    for (shift, m, eye) in f.copies.iter().take(f.n) {
        gl.uniform_matrix_4_f32_slice(u("uMvp").as_ref(), false, m);
        gl.uniform_3_f32(u("uEye").as_ref(), eye[0], eye[1], eye[2]);
        gl.uniform_1_f32(u("uShift").as_ref(), *shift);
        gl.draw_elements(glow::TRIANGLES, g.count, glow::UNSIGNED_INT, 0);
    }
    gl.bind_vertex_array(None);
    if let Some([a, b]) = timers {
        gl.end_query(glow::TIME_ELAPSED);
        let _ = a;
        gl.begin_query(glow::TIME_ELAPSED, b);
    }
    if let Some(o) = &g.objects {
        let view = objects::View { copies: &f.copies[..f.n], trees: f.trees, houses: f.houses, seen: f.seen, focus: f.focus, size: (g.size.0 as f32, g.size.1 as f32), relief: f.relief, dist: f.dist, era: f.era, time: f.time, pitch: f.pitch, season: f.season, light: g.light, dbg: dbg(), full: 0.42 * ((f.k - 6.0) / 4.0).clamp(0.0, 1.0) };
        if dbg() & 1 == 0 {
            objects::draw(gl, o, &view);
        }
    }
    if let Some(fg) = &g.figures {
        let list = f.figures.lock().unwrap().clone();
        let view = figures::View { copies: &f.copies[..f.n], size: (g.size.0 as f32, g.size.1 as f32), relief: f.relief, dist: f.dist, time: f.time, pitch: f.pitch, light: g.light };
        figures::draw(gl, fg, &view, &list);
    }
    if timers.is_some() {
        gl.end_query(glow::TIME_ELAPSED);
    }
    gl.disable(glow::DEPTH_TEST);
    gl.active_texture(glow::TEXTURE0);
}

const VERTEX: &str = include_str!("shaders/land.vert");

const FRAGMENT: &str = include_str!("shaders/land.frag");
