//! What stands on the land close up, drawn by the video card in three dimensions: trees in the
//! forests — spruces and pines where it is cold, broad trees, birches and bushes where it is warm,
//! mixed between; dead and burnt in the wastes — the houses of the towns (walls and roofs of the
//! era) and at their edges the era's own buildings — factories with chimneys, windmills turning
//! their sails. One model of each, drawn as many times as there are trees or houses (instancing);
//! the trees laid out by squares of the world, so that only the squares in view are drawn. They
//! grow out of the land as the camera comes close, and are not drawn from afar at all.
//!
//! A tree's crown is clumps of leaves, their light bent towards the crown's own middle (so that
//! it reads as one body of leaves with clumps in it), darker inside and underneath, speckled as
//! leaves are. The trees farther from the middle of the view are pictures of the same models,
//! drawn once at the start into a sheet (an atlas: each kind from four sides) — a card a tree,
//! two triangles instead of a thousand, and the same tree. Each casts its shadow on the land.

use eframe::glow::{self, HasContext};
use strategy::world::map::{Terrain, World};

mod baked;
mod models;

use models::{barn, birch, broad, bush, card, church, cottage, factory, flats, hiphouse, mill, pine, shade, spruce, townhouse};
use strategy::rng::mix64;
use super::mesh::{norm, push, Model};

/// The side of a square of the world the trees are kept by (world pixels).
pub const TILE: usize = 32;

/// The kinds of trees: spruce, broad tree, pine, birch, bush.
pub const KINDS: usize = 5;

/// The sides each kind is drawn from in the atlas (the full models turn the same ways).
const SIDES: usize = 4;

/// The atlas's pixels for one picture (its frame and the camera's tilt it is drawn at — the
/// vertex shader's `CARD_*`, `PIC_PITCH`).
const PIC_W: i32 = 256;
const PIC_H: i32 = 456;

/// The trees of the world: for each kind each tree as (x, y, size, seed), sorted by square; each
/// square's first tree and count.
#[derive(Default)]
pub struct Trees {
    pub kinds: [Vec<[f32; 4]>; KINDS],
    pub tiles: [Vec<(u32, u32)>; KINDS],
    pub tiles_x: usize,
    pub tiles_y: usize,
}

fn unit(h: u64) -> f32 {
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// The trees drawn one by one: the forests themselves are a canopy the card draws over the land
/// (`canopy`, twice as fine as the world: `ground::layers`), so the trees stand where it thins —
/// along its edges and the ways cut through it, a few in its glades — and alone in the open: a
/// few on the grass and the hills, bushes and birches in the marshes. Conifers where it is cold
/// (tundra and snow near, or far from the equator) — spruces mostly, pines, a few birches among
/// them; where it is warm broad trees, birches and bushes; in the jungle broad trees and bushes.
pub fn trees(world: &World, height: &[f32], forest: &[f32], cold: &[f32], rugged: &[f32], canopy: &[f32], clear: &dyn Fn(f32, f32) -> bool) -> Trees {
    let (w, h) = (world.w, world.h);
    let (tx, ty) = (w.div_ceil(TILE), h.div_ceil(TILE));
    let mut by: Vec<Vec<Vec<[f32; 4]>>> = (0..KINDS).map(|_| vec![Vec::new(); tx * ty]).collect();
    let cover = |x: f32, y: f32| canopy[((y * 2.0) as usize).min(2 * h - 1) * 2 * w + ((x * 2.0) as usize).min(2 * w - 1)];
    for y in 0..h {
        let lat = ((y as f32 + 0.5) / h as f32 - 0.5).abs() * 2.0;
        for x in 0..w {
            let i = y * w + x;
            let f = forest[i];
            if height[i] <= 0.0 {
                continue;
            }
            let marsh = world.terrain[i] == Terrain::Marsh;
            let alone = match world.terrain[i] {
                Terrain::Plains => 0.022,
                Terrain::Hills => 0.016,
                Terrain::Marsh => 0.10,
                Terrain::Tundra => 0.010,
                _ => 0.0,
            } * (1.0 - rugged[i].min(1.0));
            let want = if f >= 0.15 { 2.2 * f } else { 0.0 } + alone;
            if want <= 0.0 {
                continue;
            }
            let mut hh = mix64(i as u64 ^ 0x7EE5);
            let n = want.floor() as usize + (unit(hh) < want.fract()) as usize;
            let jungle = world.terrain[i] == Terrain::Jungle;
            let firs = if jungle { 0.0 } else { (cold[i] * 2.0 + (lat - 0.45) * 1.6).clamp(0.08, 0.95) };
            for k in 0..n {
                hh = mix64(hh ^ (k as u64 + 1));
                let (jx, jy) = (unit(hh), unit(mix64(hh ^ 0xA1)));
                let mut size = 0.48 + 0.42 * unit(mix64(hh ^ 0xB2));
                let seed = unit(mix64(hh ^ 0xC3));
                let (r, u) = (unit(mix64(hh ^ 0xD4)), unit(mix64(hh ^ 0xE5)));
                let kind = if marsh {
                    if u < 0.7 {
                        4
                    } else {
                        3
                    }
                } else if r < firs {
                    if u < 0.66 {
                        0
                    } else if u < 0.90 {
                        2
                    } else {
                        3
                    }
                } else if jungle {
                    if u < 0.82 {
                        1
                    } else {
                        4
                    }
                } else if u < 0.56 {
                    1
                } else if u < 0.78 {
                    3
                } else {
                    4
                };
                if kind == 4 {
                    size *= 1.2;
                }
                let (px, py) = (x as f32 + jx, y as f32 + jy);
                // (under the canopy none; along its thin edge all; in its glades and in the open
                // as many as `alone` says)
                let c = cover(px, py);
                let keep = if f < 0.15 { k == 0 } else if c >= 0.45 { false } else if c > 0.03 { true } else { unit(mix64(hh ^ 0xF6)) < 0.06 + alone * 4.0 };
                if !keep || !clear(px, py) {
                    continue;
                }
                by[kind][(py as usize / TILE).min(ty - 1) * tx + (px as usize / TILE).min(tx - 1)].push([px, py, size, seed]);
            }
        }
    }
    let mut out = Trees { tiles_x: tx, tiles_y: ty, ..Default::default() };
    for (kind, cells) in by.iter().enumerate() {
        for cell in cells {
            out.tiles[kind].push((out.kinds[kind].len() as u32, cell.len() as u32));
            out.kinds[kind].extend_from_slice(cell);
        }
    }
    out
}

/// The kinds of buildings: a cottage, a town house (two or three storeys, in rows), a barn, a
/// house under a hipped roof (a hall on the square), a church, a block of flats (the staff's
/// age), a factory, a windmill.
pub const COTTAGE: usize = 0;
pub const TOWNHOUSE: usize = 1;
pub const BARN: usize = 2;
pub const HIPHOUSE: usize = 3;
pub const CHURCH: usize = 4;
pub const BLOCK: usize = 5;
pub const FACTORY: usize = 6;
pub const MILL: usize = 7;
// (32nd session: the houses of the towns now come from free 3D models, baked — `baked.rs`; these are their kinds, after
// the eight made ones; `baked::variant` says which stands where a made one was meant)
pub const ORDINARY_A: usize = 8;
pub const ORDINARY_B: usize = 9;
pub const OLDHOUSE: usize = 10;
pub const COTTAGE_B: usize = 11;
pub const FARM_A: usize = 12;
pub const FARM_B: usize = 13;
pub const BRICK: usize = 14;
pub const PANEL: usize = 15;
pub const CHURCH_B: usize = 16;
pub const BUILDS: usize = 17;

pub use baked::{footprint as baked_footprint, variant as baked_variant};

/// What the shaders need of a kind of building besides its triangles: the walls' height, the whole height and half the
/// width (for the shadow). The made kinds' are the vertex shader's `TOP` and `BSHAPE`.
const MADE: [[f32; 3]; 8] = [[0.19, 0.40, 0.24], [0.34, 0.60, 0.25], [0.15, 0.40, 0.30], [0.30, 0.55, 0.30], [0.32, 0.95, 0.24], [0.64, 0.70, 0.36], [0.30, 0.50, 0.46], [0.70, 0.80, 0.22]];

/// What stands in the towns: each kind of building as (x, y, size, its turn — 0..1 of a full
/// turn: the model's −y side, its front with the door, to the street).
#[derive(Default)]
pub struct Towns {
    pub builds: [Vec<[f32; 4]>; BUILDS],
}

/// A model: vertices of (x, y, z, normal x, y, z, part, light) — part 0 the trunk or the walls,
/// 1 the leaves or the roof, 2 a windmill's sails, 3 a card, 4 a shadow; light — how much of the
/// sky reaches the place (less inside a crown and under it).

const FLOATS: usize = 8;

/// The materials of the buildings and the trees' bark (`assets/objects`, ambientCG, CC0): colour
/// then normals, each 512×512 RGBA — clay tiles, slate, thatch, corrugated iron, brick, plaster,
/// boards, stone, flat tiles, bark (the shader's `MSCALE` order).
pub const MATS: usize = 10;

pub fn materials() -> (Vec<u8>, usize) {
    const FILES: [&[u8]; 2 * MATS] = [
        include_bytes!("../../../../../assets/objects/tiles_col.jpg"),
        include_bytes!("../../../../../assets/objects/slate_col.jpg"),
        include_bytes!("../../../../../assets/objects/thatch_col.jpg"),
        include_bytes!("../../../../../assets/objects/iron_col.jpg"),
        include_bytes!("../../../../../assets/objects/brick_col.jpg"),
        include_bytes!("../../../../../assets/objects/plaster_col.jpg"),
        include_bytes!("../../../../../assets/objects/boards_col.jpg"),
        include_bytes!("../../../../../assets/objects/stone_col.jpg"),
        include_bytes!("../../../../../assets/objects/flat_tiles_col.jpg"),
        include_bytes!("../../../../../assets/objects/bark_col.jpg"),
        include_bytes!("../../../../../assets/objects/tiles_nrm.jpg"),
        include_bytes!("../../../../../assets/objects/slate_nrm.jpg"),
        include_bytes!("../../../../../assets/objects/thatch_nrm.jpg"),
        include_bytes!("../../../../../assets/objects/iron_nrm.jpg"),
        include_bytes!("../../../../../assets/objects/brick_nrm.jpg"),
        include_bytes!("../../../../../assets/objects/plaster_nrm.jpg"),
        include_bytes!("../../../../../assets/objects/boards_nrm.jpg"),
        include_bytes!("../../../../../assets/objects/stone_nrm.jpg"),
        include_bytes!("../../../../../assets/objects/flat_tiles_nrm.jpg"),
        include_bytes!("../../../../../assets/objects/bark_nrm.jpg"),
    ];
    let size = 512;
    let mut out = Vec::with_capacity(size * size * 4 * FILES.len());
    for f in FILES {
        match image::load_from_memory(f) {
            Ok(im) => out.extend_from_slice(im.resize_exact(size as u32, size as u32, image::imageops::FilterType::Triangle).to_rgba8().as_raw()),
            Err(_) => out.extend(std::iter::repeat([128u8, 128, 255, 255]).take(size * size).flatten()),
        }
    }
    (out, size)
}

/// The buildings' kinds as the shaders know them: 10 + the kind (the trees are 0..KINDS).
const BUILT: f32 = 10.0;

/// One model on the card with its copies.
struct Mesh {
    vao: glow::VertexArray,
    inst: glow::Buffer,
    verts: i32,
    copies: u32,
}

pub struct Gpu {
    program: glow::Program,
    /// Each kind of tree whole, its card, its shadow (the three share the copies).
    trees: Vec<Mesh>,
    cards: Vec<Mesh>,
    shades: Vec<Mesh>,
    /// Each kind of building, and its shadow; and each kind's `uBuilt` (the walls' height, the whole height, half the width).
    builds: Vec<Mesh>,
    built: Vec<[f32; 3]>,
    build_shades: Vec<Mesh>,
    /// The trees' pictures (none if the card could not draw them: the cards are left out).
    atlas: Option<glow::Texture>,
    /// The materials' photographs (`materials`), one layer each.
    mats: glow::Texture,
    tiles: [Vec<(u32, u32)>; KINDS],
    tiles_x: usize,
    tiles_y: usize,
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

unsafe fn mesh(gl: &glow::Context, model: &Model) -> Result<Mesh, String> {
    let inst = gl.create_buffer()?;
    mesh_on(gl, model, inst)
}

/// A model drawn with the copies of another's.
unsafe fn mesh_on(gl: &glow::Context, model: &Model, inst: glow::Buffer) -> Result<Mesh, String> {
    let vao = gl.create_vertex_array()?;
    let vbo = gl.create_buffer()?;
    gl.bind_vertex_array(Some(vao));
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
    let bytes: Vec<u8> = model.iter().flat_map(|x| x.to_le_bytes()).collect();
    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
    let stride = (FLOATS * 4) as i32;
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
    gl.enable_vertex_attrib_array(1);
    gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 12);
    gl.enable_vertex_attrib_array(2);
    gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, stride, 24);
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(inst));
    gl.enable_vertex_attrib_array(3);
    gl.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, 16, 0);
    gl.vertex_attrib_divisor(3, 1);
    gl.bind_vertex_array(None);
    Ok(Mesh { vao, inst, verts: (model.len() / FLOATS) as i32, copies: 0 })
}

pub unsafe fn make(gl: &glow::Context) -> Result<Gpu, String> {
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
    let models = [spruce(), broad(), pine(), birch(), bush()];
    let mut trees = Vec::new();
    let mut cards = Vec::new();
    let mut shades = Vec::new();
    for m in &models {
        let t = mesh(gl, m)?;
        cards.push(mesh_on(gl, &card(), t.inst)?);
        shades.push(mesh_on(gl, &shade(), t.inst)?);
        trees.push(t);
    }
    let mut builds = Vec::new();
    let mut build_shades = Vec::new();
    let mut built: Vec<[f32; 3]> = MADE.to_vec();
    let mut models: Vec<Model> = vec![cottage(), townhouse(), barn(), hiphouse(), church(), flats(), factory(), mill()];
    for k in 8..BUILDS {
        match baked::load(k) {
            Some(b) => {
                built.push([b.top, b.height, (b.hx + b.hy) / 2.0]);
                models.push(b.model);
            }
            None => {
                // (a baked file that cannot be read: a made house stands there)
                built.push(MADE[1]);
                models.push(townhouse());
            }
        }
    }
    for m in &models {
        let b = mesh(gl, m)?;
        build_shades.push(mesh_on(gl, &shade(), b.inst)?);
        builds.push(b);
    }
    let mats = gl.create_texture()?;
    {
        let (bytes, size) = materials();
        gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(mats));
        gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
        let ext = gl.supported_extensions();
        if ext.contains("GL_EXT_texture_filter_anisotropic") || ext.contains("GL_ARB_texture_filter_anisotropic") {
            gl.tex_parameter_f32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MAX_ANISOTROPY_EXT, 8.0);
        }
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_3d(glow::TEXTURE_2D_ARRAY, 0, glow::RGBA8 as i32, size as i32, size as i32, (2 * MATS) as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&bytes));
        gl.generate_mipmap(glow::TEXTURE_2D_ARRAY);
        gl.bind_texture(glow::TEXTURE_2D_ARRAY, None);
    }
    let mut g = Gpu { program, trees, cards, shades, builds, built, build_shades, atlas: None, mats, tiles: Default::default(), tiles_x: 0, tiles_y: 0 };
    match bake(gl, &g) {
        Ok(t) => g.atlas = Some(t),
        Err(e) => eprintln!("карта: картинки дальних деревьев не нарисовать: {e}"),
    }
    Ok(g)
}

/// The trees' pictures: each kind from each of its sides, drawn once by the same shader into a
/// sheet — what the light makes of it (the sun, the sky, whether leaves, how much it faces up),
/// premultiplied by its cover so that the smaller copies of the sheet keep their edges' light.
unsafe fn bake(gl: &glow::Context, g: &Gpu) -> Result<glow::Texture, String> {
    let (w, h) = (PIC_W * SIDES as i32, PIC_H * KINDS as i32);
    let tex = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, w, h, 0, glow::RGBA, glow::UNSIGNED_BYTE, None);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
    let depth = gl.create_renderbuffer()?;
    gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth));
    gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH_COMPONENT24, w, h);
    // (what egui had bound, to give it back)
    let prev_fb = gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING);
    let mut vp = [0i32; 4];
    gl.get_parameter_i32_slice(glow::VIEWPORT, &mut vp);
    let scissor = gl.is_enabled(glow::SCISSOR_TEST);
    let fb = gl.create_framebuffer()?;
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fb));
    gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
    gl.framebuffer_renderbuffer(glow::FRAMEBUFFER, glow::DEPTH_ATTACHMENT, glow::RENDERBUFFER, Some(depth));
    let ok = gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE;
    if ok {
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.disable(glow::CULL_FACE);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LESS);
        gl.depth_mask(true);
        gl.viewport(0, 0, w, h);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear_depth_f32(1.0);
        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        gl.use_program(Some(g.program));
        let u = |name: &str| gl.get_uniform_location(g.program, name);
        gl.uniform_1_i32(u("uMode").as_ref(), 1);
        gl.uniform_1_f32(u("uGrow").as_ref(), 1.0);
        gl.uniform_1_f32(u("uEra").as_ref(), 0.0);
        // (one copy a side: its seed turns the model that way)
        let one = gl.create_buffer()?;
        let seeds: Vec<u8> = (0..SIDES).flat_map(|s| [0.0f32, 0.0, 1.0, side_seed(s)]).flat_map(|x| x.to_le_bytes()).collect();
        for (k, t) in g.trees.iter().enumerate() {
            gl.uniform_1_f32(u("uKind").as_ref(), k as f32);
            gl.bind_vertex_array(Some(t.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(one));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &seeds, glow::STATIC_DRAW);
            for s in 0..SIDES {
                gl.viewport(s as i32 * PIC_W, k as i32 * PIC_H, PIC_W, PIC_H);
                gl.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, 16, (s * 16) as i32);
                gl.draw_arrays_instanced(glow::TRIANGLES, 0, t.verts, 1);
            }
            // (the copies are the tree's own again)
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(t.inst));
            gl.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, 16, 0);
        }
        gl.bind_vertex_array(None);
        gl.delete_buffer(one);
        gl.uniform_1_i32(u("uMode").as_ref(), 0);
    }
    gl.bind_framebuffer(glow::FRAMEBUFFER, std::num::NonZeroU32::new(prev_fb as u32).map(glow::NativeFramebuffer));
    gl.viewport(vp[0], vp[1], vp[2], vp[3]);
    if scissor {
        gl.enable(glow::SCISSOR_TEST);
    }
    gl.delete_framebuffer(fb);
    gl.delete_renderbuffer(depth);
    if !ok {
        gl.delete_texture(tex);
        return Err("рамка для рисования не собралась".into());
    }
    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
    gl.generate_mipmap(glow::TEXTURE_2D);
    Ok(tex)
}

/// The seed that turns a tree to side `s` (the shader's `side` of it is `s`).
fn side_seed(s: usize) -> f32 {
    (s as f32 + 0.5) / SIDES as f32 / 7.77
}

unsafe fn fill(gl: &glow::Context, m: &mut Mesh, copies: &[[f32; 4]]) {
    let bytes: Vec<u8> = copies.iter().flatten().flat_map(|x| x.to_le_bytes()).collect();
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(m.inst));
    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
    m.copies = copies.len() as u32;
}

pub unsafe fn upload_trees(gl: &glow::Context, g: &mut Gpu, t: &Trees) {
    for k in 0..KINDS {
        fill(gl, &mut g.trees[k], &t.kinds[k]);
        g.cards[k].copies = g.trees[k].copies;
        g.shades[k].copies = g.trees[k].copies;
    }
    g.tiles = t.tiles.clone();
    g.tiles_x = t.tiles_x;
    g.tiles_y = t.tiles_y;
}

pub unsafe fn upload_houses(gl: &glow::Context, g: &mut Gpu, t: &Towns) {
    for (k, list) in t.builds.iter().enumerate() {
        fill(gl, &mut g.builds[k], list);
        g.build_shades[k].copies = g.builds[k].copies;
    }
}

/// What the objects need of a frame: the copies of the world in view (shift, matrix), how far
/// the trees and the houses have grown (0 — not drawn), the world's bounds seen (x not wrapped),
/// the middle of the view and how far from it trees are drawn (beyond — the haze: none), the
/// world's size, the land's height scale, the camera's distance, the era.
pub struct View<'a> {
    pub copies: &'a [(f32, [f32; 16], [f32; 3])],
    pub trees: f32,
    pub houses: f32,
    pub seen: [f32; 4],
    pub focus: [f32; 3],
    pub size: (f32, f32),
    pub relief: f32,
    pub dist: f32,
    pub era: f32,
    pub time: f32,
    pub pitch: f32,
    pub season: [f32; 3],
    /// The share of the trees' circle where they are drawn whole (the rest are cards).
    pub full: f32,
    /// The light of the land (the sun where no range shades it).
    pub light: glow::Texture,
    /// Parts left out, to measure what they cost (`scene::dbg`: 32 — the trees' shadows, 64 —
    /// the far trees, 128 — the near trees whole).
    pub dbg: i32,
}

/// The trees in view and the houses, over the land just drawn (its height map bound to unit 1,
/// its light to unit 12).
pub unsafe fn draw(gl: &glow::Context, g: &Gpu, v: &View) {
    if v.trees <= 0.0 && v.houses <= 0.0 {
        return;
    }
    gl.use_program(Some(g.program));
    let u = |name: &str| gl.get_uniform_location(g.program, name);
    gl.uniform_1_i32(u("uHeight").as_ref(), 1);
    gl.active_texture(glow::TEXTURE14);
    gl.bind_texture(glow::TEXTURE_2D, Some(v.light));
    gl.uniform_1_i32(u("uLight").as_ref(), 14);
    if let Some(a) = g.atlas {
        gl.active_texture(glow::TEXTURE13);
        gl.bind_texture(glow::TEXTURE_2D, Some(a));
        gl.uniform_1_i32(u("uAtlas").as_ref(), 13);
    }
    gl.active_texture(glow::TEXTURE15);
    gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(g.mats));
    gl.uniform_1_i32(u("uMat").as_ref(), 15);
    gl.active_texture(glow::TEXTURE0);
    gl.uniform_2_f32(u("uSize").as_ref(), v.size.0, v.size.1);
    gl.uniform_1_f32(u("uRelief").as_ref(), v.relief);
    gl.uniform_1_f32(u("uDist").as_ref(), v.dist);
    gl.uniform_1_f32(u("uEra").as_ref(), v.era);
    gl.uniform_1_f32(u("uTime").as_ref(), v.time);
    gl.uniform_3_f32(u("uSeason").as_ref(), v.season[0], v.season[1], v.season[2]);
    let (sp, cp) = v.pitch.sin_cos();
    gl.uniform_3_f32(u("uUp").as_ref(), 0.0, -cp, sp);
    gl.uniform_3_f32(u("uToward").as_ref(), 0.0, sp, cp);
    gl.enable(glow::DEPTH_TEST);
    gl.depth_func(glow::LEQUAL);
    // (the models are wound either way round where they were made by hand: cull nothing)
    gl.disable(glow::CULL_FACE);
    let tile = TILE as f32;
    // the runs of copies of a kind in view: (first, count) — whole near the middle of the view
    // (`inner`), cards beyond it, or both (`None`: the shadows)
    let runs = |k: usize, shift: f32, inner: Option<bool>| -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        if g.tiles_x == 0 || g.tiles[k].is_empty() {
            return out;
        }
        let x0 = (((v.seen[0] - shift) / tile).floor().max(0.0)) as usize;
        let x1 = (((v.seen[2] - shift) / tile).floor().max(-1.0) as isize).min(g.tiles_x as isize - 1);
        let y0 = ((v.seen[1] / tile).floor().max(0.0)) as usize;
        let y1 = ((v.seen[3] / tile).floor() as isize).min(g.tiles_y as isize - 1);
        if x1 < x0 as isize || y1 < y0 as isize {
            return out;
        }
        // (the squares near enough the view's middle; the squares of a row next to each other
        // are one run of copies — they are kept row by row)
        let near = |tx: usize, ty: usize| {
            let (x, y) = (tx as f32 * tile + tile / 2.0 + shift, ty as f32 * tile + tile / 2.0);
            let d = (x - v.focus[0]).hypot(y - v.focus[1]);
            let whole = v.full > 0.0 && d <= v.focus[2] * v.full + tile * 0.71;
            d <= v.focus[2] + tile * 0.71 && inner.map_or(true, |i| i == whole)
        };
        for ty in y0..=y1 as usize {
            let mut tx = x0;
            while tx <= x1 as usize {
                if !near(tx, ty) {
                    tx += 1;
                    continue;
                }
                let first = tx;
                while tx < x1 as usize && near(tx + 1, ty) {
                    tx += 1;
                }
                let (a, b) = (g.tiles[k][ty * g.tiles_x + first], g.tiles[k][ty * g.tiles_x + tx]);
                let (start, end) = (a.0, b.0 + b.1);
                tx += 1;
                if end > start {
                    out.push((start, end - start));
                }
            }
        }
        out
    };
    let draw_runs = |mesh: &Mesh, list: &[(u32, u32)]| {
        gl.bind_vertex_array(Some(mesh.vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(mesh.inst));
        for &(start, n) in list {
            gl.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, 16, start as i32 * 16);
            gl.draw_arrays_instanced(glow::TRIANGLES, 0, mesh.verts, n as i32);
        }
    };
    let all = |mesh: &Mesh| {
        if mesh.copies > 0 {
            gl.bind_vertex_array(Some(mesh.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(mesh.inst));
            gl.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, 16, 0);
            gl.draw_arrays_instanced(glow::TRIANGLES, 0, mesh.verts, mesh.copies as i32);
        }
    };
    for (shift, m, _) in v.copies {
        gl.uniform_matrix_4_f32_slice(u("uMvp").as_ref(), false, m);
        // the shadows first, darkening the land (not written into the depth: they lie on it)
        gl.enable(glow::BLEND);
        gl.blend_func(glow::ZERO, glow::ONE_MINUS_SRC_ALPHA);
        gl.depth_mask(false);
        gl.enable(glow::POLYGON_OFFSET_FILL);
        gl.polygon_offset(-1.0, -4.0);
        gl.uniform_1_i32(u("uMode").as_ref(), 3);
        if v.trees > 0.0 && v.dbg & 32 == 0 {
            gl.uniform_1_f32(u("uGrow").as_ref(), v.trees);
            for k in 0..KINDS {
                gl.uniform_1_f32(u("uKind").as_ref(), k as f32);
                draw_runs(&g.shades[k], &runs(k, *shift, None));
            }
        }
        if v.houses > 0.0 {
            gl.uniform_1_f32(u("uGrow").as_ref(), v.houses);
            for k in 0..BUILDS {
                gl.uniform_1_f32(u("uKind").as_ref(), BUILT + k as f32);
                gl.uniform_3_f32_slice(u("uBuilt").as_ref(), &g.built[k]);
                all(&g.build_shades[k]);
            }
        }
        gl.disable(glow::POLYGON_OFFSET_FILL);
        gl.disable(glow::BLEND);
        gl.depth_mask(true);
        if v.trees > 0.0 {
            gl.uniform_1_f32(u("uGrow").as_ref(), v.trees);
            for k in 0..KINDS {
                gl.uniform_1_f32(u("uKind").as_ref(), k as f32);
                gl.uniform_1_i32(u("uMode").as_ref(), 0);
                if v.dbg & 128 == 0 {
                    draw_runs(&g.trees[k], &runs(k, *shift, Some(true)));
                }
                // (very close, near the middle of the view, the trees whole; elsewhere their
                // pictures — a tree there is a few points tall)
                if g.atlas.is_some() && v.dbg & 64 == 0 {
                    gl.uniform_1_i32(u("uMode").as_ref(), 2);
                    gl.enable(glow::SAMPLE_ALPHA_TO_COVERAGE);
                    draw_runs(&g.cards[k], &runs(k, *shift, Some(false)));
                    gl.disable(glow::SAMPLE_ALPHA_TO_COVERAGE);
                }
            }
        }
        if v.houses > 0.0 {
            gl.uniform_1_i32(u("uMode").as_ref(), 0);
            gl.uniform_1_f32(u("uGrow").as_ref(), v.houses);
            for k in 0..BUILDS {
                gl.uniform_1_f32(u("uKind").as_ref(), BUILT + k as f32);
                gl.uniform_3_f32_slice(u("uBuilt").as_ref(), &g.built[k]);
                all(&g.builds[k]);
            }
        }
    }
    gl.bind_vertex_array(None);
}

const VERTEX: &str = include_str!("shaders/objects.vert");

const FRAGMENT: &str = include_str!("shaders/objects.frag");
