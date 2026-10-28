//! War on the map: two neighbouring countries of a generated map fight province by province.
//! A chronicle, a picture of the front year by year (`out/front.png`), and self-checks.
//!
//! `checks front [--seed N] [--pack packs/x.toml] [--months M]`; `--groups` —
//! the checks of groups of armies (orders, the staff that leads by groups).

use strategy::military::doctrine::Country;
use strategy::military::front::{Campaign, General, Order, Theatre};
use strategy::pack::Pack;
use strategy::png;
use strategy::rng::{hash64, Rng};
use strategy::world::countries::{self, Politics};
use strategy::world::map::{self, Params, Terrain, World};
use strategy::world::render;

mod groups;
mod suite;

use groups::groups_checks;
use suite::checks;
use crate::common::{pick, doctrine, mean_se};

/// A war between the chosen pair of map `seed`; with `twins`, both countries' equipment and
/// what their designers claim are the same.
fn campaign_of(pack: &Pack, world: &World, pol: &Politics, pair: (usize, usize), money: [f32; 2], seed: u64, twins: bool) -> Campaign {
    let th = Theatre::new(world, pol, pair.0, pair.1);
    let mut ra = Rng::new(hash64(seed));
    let mut rb = Rng::new(hash64(seed ^ if twins { 0 } else { 0xB0B }));
    let ones = vec![1.0; pack.goods.len()];
    let a = Country::new(pack, &pol.countries[pair.0].name, &ones, &mut ra);
    let b = Country::new(pack, &pol.countries[pair.1].name, &ones, &mut rb);
    Campaign::new(pack, th, [a, b], money, seed)
}

/// Who won: the side that made the other give up, or that holds more of its people at the end
/// (None when they hold nearly as much).
fn winner(c: &Campaign) -> Option<usize> {
    if let Some(s) = c.fallen {
        return Some(1 - s);
    }
    let p = c.people();
    let h = c.held();
    let _ = h;
    if (p[0] - p[1]).abs() < 0.05 {
        None
    } else if p[0] > p[1] {
        Some(0)
    } else {
        Some(1)
    }
}

/// One map: its pair, ready to fight.
struct Theatre0 {
    world: World,
    pol: Politics,
    pair: (usize, usize),
}

/// Lucks each war of the checks is fought with.
const LUCKS: [u64; 6] = [11, 12, 13, 14, 15, 16];

pub fn main() {
    let args: Vec<String> = crate::args().into_iter().collect();
    let arg = |k: &str| args.iter().position(|a| a == k).map(|i| args[i + 1].clone());
    let path = arg("--pack").unwrap_or("packs/industrial.toml".into());
    let seed: u64 = arg("--seed").map_or(1, |v| v.parse().expect("--seed N"));
    let months: u32 = arg("--months").map_or(48, |v| v.parse().expect("--months M"));
    let pack = Pack::load(&path).unwrap_or_else(|e| panic!("{e}"));
    if args.iter().any(|a| a == "--groups") {
        std::process::exit(if groups_checks(&pack, months) { 0 } else { 1 });
    }
    let world = map::generate(&Params { seed, w: 2048, h: 1024, land_frac: 0.32, land_provinces: 3000 }).0;
    let pol = countries::generate(&world, seed, 30, &pack);
    let Some(pair) = pick(&world, &pol) else {
        println!("на карте {seed} нет пары соседей сравнимой силы");
        return;
    };
    let pair = if args.iter().any(|a| a == "--swap") { (pair.1, pair.0) } else { pair };
    let money = if args.iter().any(|a| a == "--even") { [1.0, 1.0] } else { [pol.gdp(pair.0), pol.gdp(pair.1)] };
    let luck: u64 = arg("--luck").map_or(seed, |v| v.parse().expect("--luck N"));
    let mut c = campaign_of(&pack, &world, &pol, pair, money, luck, args.iter().any(|a| a == "--twins"));
    let every: u32 = arg("--every").map_or(6, |v| v.parse().expect("--every N"));
    let names = [c.sides[0].name.clone(), c.sides[1].name.clone()];
    println!("══════ {} — война на карте {seed}: {} (синие) против {} (красные) ══════", pack.name, names[0], names[1]);
    let held = c.held();
    println!(
        "  провинций {} и {}, граница {} и {} провинций, ВВП {:.0} и {:.0}",
        held[0],
        held[1],
        c.front(0).len(),
        c.front(1).len(),
        money[0],
        money[1]
    );
    let fr = render::frame(&world, &c.th);
    let mut frames = vec![render::front(&world, &c.th, &c.owner, fr)];
    let mut last = 0;
    for m in 1..=months {
        c.step(&pack);
        if m % every == 0 || c.fallen.is_some() {
            let held = c.held();
            let period: Vec<_> = c.fought[last..].to_vec();
            last = c.fought.len();
            let (att, won): (Vec<usize>, Vec<usize>) = (0..2)
                .map(|s| (period.iter().filter(|f| f.attacker == s).count(), period.iter().filter(|f| f.attacker == s && f.won).count()))
                .unzip();
            let pockets: Vec<usize> = (0..2)
                .map(|s| {
                    let sup = c.supply(s);
                    (0..c.owner.len()).filter(|&p| c.owner[p] == s && sup[p] <= 0.5 && c.troops[s][p].iter().sum::<f32>() > 0.05).count()
                })
                .collect();
            let people = c.people();
            println!(
                "  мес {m:>2}: провинций {} / {} (своих людей {:.0}% / {:.0}%) | наступлений {} (успешных {}) / {} ({}) | без снабжения {} / {} | доктрины: {} // {}",
                held[0],
                held[1],
                people[0] * 100.0,
                people[1] * 100.0,
                att[0],
                won[0],
                att[1],
                won[1],
                pockets[0],
                pockets[1],
                doctrine(&pack, &c.sides[0].plan),
                doctrine(&pack, &c.sides[1].plan)
            );
        }
        if m % 12 == 0 {
            frames.push(render::front(&world, &c.th, &c.owner, fr));
        }
        for l in std::mem::take(&mut c.log) {
            println!("  {l}");
        }
        if let Some(s) = c.fallen {
            println!("  мес {m}: {} капитулирует — у неё меньше трети её людей", names[s]);
            frames.push(render::front(&world, &c.th, &c.owner, fr));
            break;
        }
    }
    // the frames side by side, two to a row
    let (_, cw, _, ch) = fr;
    let cols = 2.min(frames.len());
    let rows = frames.len().div_ceil(cols);
    let (iw, ih) = (cols * cw + (cols - 1) * 6, rows * ch + (rows - 1) * 6);
    let mut img = vec![255u8; iw * ih * 3];
    for (k, f) in frames.iter().enumerate() {
        let (ox, oy) = ((k % cols) * (cw + 6), (k / cols) * (ch + 6));
        for y in 0..ch {
            let dst = ((oy + y) * iw + ox) * 3;
            img[dst..dst + cw * 3].copy_from_slice(&f[y * cw * 3..(y + 1) * cw * 3]);
        }
    }
    std::fs::create_dir_all("out").ok();
    png::write_rgb("out/front.png", iw, ih, &img).unwrap();
    println!("  карта фронта по годам: out/front.png ({} кадров, {}×{})", frames.len(), iw, ih);
    if !args.iter().any(|a| a == "--quick") && !checks(&pack, months) {
        std::process::exit(1);
    }
}
