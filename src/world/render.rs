use crate::military::front::Theatre;
use crate::world::map::{Terrain, World, NONE};

fn shade(t: Terrain) -> f32 {
    match t {
        Terrain::Mountains => 0.72,
        Terrain::Hills => 0.86,
        Terrain::Forest | Terrain::Jungle => 0.84,
        Terrain::Marsh => 0.9,
        _ => 1.0,
    }
}

/// The theatre at one moment: each side's land, what it took from the other paler, the
/// front dark, capitals white.
pub fn front(world: &World, th: &Theatre, owner: &[usize], frame: (usize, usize, usize, usize)) -> Vec<u8> {
    let (x0, cw, y0, ch) = frame;
    let w = world.w;
    let mut out = vec![0u8; cw * ch * 3];
    let side_of = |i: usize| {
        let p = world.owner[i];
        if p == NONE || world.provinces[p as usize].sea {
            return None;
        }
        let t = th.at[p as usize];
        (t != crate::military::front::NO).then_some(t as usize)
    };
    for yy in 0..ch {
        for xx in 0..cw {
            let (x, y) = ((x0 + xx) % w, y0 + yy);
            let i = y * w + x;
            let c: [u8; 3] = match side_of(i) {
                None => {
                    if matches!(world.terrain[i], Terrain::Ocean | Terrain::Lake) {
                        [52, 82, 125]
                    } else {
                        [205, 205, 200]
                    }
                }
                Some(t) => {
                    let s = owner[t];
                    let r = side_of(y * w + (x + 1) % w);
                    let d = if y + 1 < world.h { side_of(i + w) } else { Some(t) };
                    let edge = |o: Option<usize>| o.is_some_and(|o| owner[o] != s);
                    if edge(r) || edge(d) {
                        [15, 15, 15]
                    } else {
                        let base: [f32; 3] = match (s, th.home[t] == s) {
                            (0, true) => [70.0, 110.0, 190.0],
                            (0, false) => [150.0, 185.0, 235.0],
                            (_, true) => [190.0, 80.0, 70.0],
                            (_, false) => [240.0, 165.0, 150.0],
                        };
                        let k = shade(th.terrain[t]);
                        [(base[0] * k) as u8, (base[1] * k) as u8, (base[2] * k) as u8]
                    }
                }
            };
            out[(yy * cw + xx) * 3..(yy * cw + xx) * 3 + 3].copy_from_slice(&c);
        }
    }
    for s in 0..2 {
        let seed = world.provinces[th.provs[th.capital[s]]].seed;
        let (cx, cy) = ((seed % w + w - x0) % w, (seed / w).wrapping_sub(y0));
        for dy in 0..7usize {
            for dx in 0..7usize {
                let (x, y) = ((cx + dx).wrapping_sub(3), (cy + dy).wrapping_sub(3));
                if x < cw && y < ch {
                    let edge = dx == 0 || dy == 0 || dx == 6 || dy == 6;
                    out[(y * cw + x) * 3..(y * cw + x) * 3 + 3].copy_from_slice(if edge { &[0, 0, 0] } else { &[255, 255, 255] });
                }
            }
        }
    }
    out
}

/// Where on the map the theatre lies (the map wraps around east to west): (x0, width, y0, height).
pub fn frame(world: &World, th: &Theatre) -> (usize, usize, usize, usize) {
    let (w, h) = (world.w, world.h);
    let mut col = vec![false; w];
    let (mut y0, mut y1) = (h, 0);
    for y in 0..h {
        for x in 0..w {
            let p = world.owner[y * w + x];
            if p != NONE && th.at[p as usize] != crate::military::front::NO {
                col[x] = true;
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    // the longest run of empty columns, around the seam too, is where the picture is cut
    let (mut best, mut best_end, mut run) = (0usize, 0usize, 0usize);
    for i in 0..2 * w {
        if !col[i % w] {
            run += 1;
            if run > best {
                best = run;
                best_end = i % w;
            }
        } else {
            run = 0;
        }
    }
    let best = best.min(w);
    let pad = 8;
    let x0 = (best_end + 1 + w - pad) % w;
    let mut cw = (w - best + 2 * pad).min(w);
    let y0 = y0.saturating_sub(pad);
    let mut ch = (y1 + pad).min(h - 1) - y0 + 1;
    
    // Округляем размеры до кратности 16 пикселей, чтобы картинку не корежило при сдвигах фронта
    cw = ((cw + 15) / 16) * 16;
    ch = ((ch + 15) / 16) * 16;
    
    (x0, cw.min(w), y0, ch.min(h - 1))
}

/// Water a land pixel gathers before it shows as a river (pixels of land upstream, itself
/// included).
const RIVER: f32 = 450.0;

/// The rivers of the world: each land pixel where enough water gathers, the pixel its water runs
/// on to, and how much gathers there. The way down: the land is flooded from the water inward,
/// lowest first (a pit fills up to where it spills, so every drop finds the sea or a lake), each
/// pixel's water running on to the one it was reached from; the water gathers along the way
/// (each land pixel gives one). Inside a filled pit the way runs straight across the flat: there
/// no river is drawn (the river runs into the basin and comes out of it lower down, as through a
/// lake or a marsh).
pub fn rivers(world: &World) -> Vec<(usize, usize, f32)> {
    rivers_with(world, false)
}

/// The rivers, and (`through_pits`) their ways across the filled pits too — whole lines from
/// source to sea, for a map that draws them as lines.
pub fn rivers_with(world: &World, through_pits: bool) -> Vec<(usize, usize, f32)> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let (w, h) = (world.w, world.h);
    let ht = &world.height;
    let water = |i: usize| matches!(world.terrain[i], Terrain::Ocean | Terrain::Lake);
    let n = w * h;
    let mut parent = vec![usize::MAX; n];
    let mut level = vec![f32::NEG_INFINITY; n];
    let mut seen = vec![false; n];
    let mut heap: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();
    let key = |v: f32| -> u32 {
        // a float to an order-keeping integer (heights are finite)
        let b = v.to_bits();
        if b >> 31 == 1 { !b } else { b | 0x8000_0000 }
    };
    let around = |i: usize| -> [usize; 8] {
        let (x, y) = (i % w, i / w);
        let xl = (x + w - 1) % w;
        let xr = (x + 1) % w;
        let yu = y.saturating_sub(1);
        let yd = (y + 1).min(h - 1);
        [yu * w + xl, yu * w + x, yu * w + xr, y * w + xl, y * w + xr, yd * w + xl, yd * w + x, yd * w + xr]
    };
    for i in 0..n {
        if water(i) {
            seen[i] = true;
            level[i] = ht[i];
            if around(i).iter().any(|&j| !water(j)) {
                heap.push(Reverse((key(ht[i]), i)));
            }
        }
    }
    let mut order: Vec<usize> = Vec::new();
    while let Some(Reverse((_, i))) = heap.pop() {
        for j in around(i) {
            if seen[j] {
                continue;
            }
            seen[j] = true;
            parent[j] = i;
            level[j] = ht[j].max(level[i] + 1e-5);
            order.push(j);
            heap.push(Reverse((key(level[j]), j)));
        }
    }
    let mut flow = vec![0.0f32; n];
    for &i in order.iter().rev() {
        flow[i] += 1.0;
        let p = parent[i];
        if p != usize::MAX && !water(p) {
            flow[p] += flow[i];
        }
    }
    order.into_iter().filter(|&i| flow[i] >= RIVER && parent[i] != usize::MAX && (through_pits || level[i] - ht[i] < 1e-4)).map(|i| (i, parent[i], flow[i])).collect()
}

