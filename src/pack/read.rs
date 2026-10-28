//! Reading a pack: its TOML form (`Raw*`, with the defaults of what a pack may leave out) and how
//! it becomes a `Pack` — ids looked up, every mistake told to the modder in words.

use super::*;

pub(super) fn one() -> f32 {
    1.0
}

fn d_patience() -> f32 {
    6.0
}

fn d_wage() -> f32 {
    0.7
}

fn d_tax() -> f32 {
    0.15
}

fn d_military() -> f32 {
    0.4
}

fn d_margin() -> f32 {
    0.05
}

pub(super) fn yes() -> bool {
    true
}

fn d_width() -> f32 {
    12.0
}

fn d_days() -> u32 {
    30
}

fn d_line() -> String {
    "front".into()
}

fn d_theme() -> String {
    "staff".into()
}

fn d_icon() -> String {
    "helmet".into()
}

fn d_good_icon() -> String {
    "crate".into()
}

fn d_symbol() -> String {
    "infantry".into()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPack {
    name: String,
    #[serde(default)]
    about: String,
    #[serde(default)]
    government: RawGov,
    #[serde(rename = "good")]
    goods: Vec<RawGood>,
    #[serde(rename = "group")]
    groups: Vec<RawGroup>,
    #[serde(rename = "sector")]
    sectors: Vec<RawSector>,
    #[serde(rename = "resource", default)]
    resources: Vec<RawResource>,
    #[serde(default)]
    trade: Option<RawTrade>,
    #[serde(default)]
    war: Option<RawWar>,
    #[serde(rename = "unit", default)]
    units: Vec<RawUnit>,
    #[serde(rename = "support", default)]
    support: Vec<RawSupport>,
    #[serde(rename = "ship", default)]
    ships: Vec<RawShip>,
    #[serde(default)]
    science: Option<RawScience>,
    #[serde(rename = "tech", default)]
    techs: Vec<RawTech>,
    #[serde(default)]
    progress: RawProgress,
    #[serde(default)]
    ui: RawUi,
    #[serde(default)]
    world: RawWorld,
}

/// The world of the era: how many people live in it, the least and the most (millions; each world
/// draws its own between them).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWorld {
    #[serde(default = "d_people")]
    people: [f32; 2],
}

impl Default for RawWorld {
    fn default() -> Self {
        RawWorld { people: d_people() }
    }
}

/// From the Earth of the 1700s to that of the first years of the 1900s.
fn d_people() -> [f32; 2] {
    [600.0, 1700.0]
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUi {
    #[serde(default = "d_theme")]
    theme: String,
}

impl Default for RawUi {
    fn default() -> Self {
        RawUi { theme: d_theme() }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScience {
    learned: Vec<String>,
    #[serde(default = "d_science")]
    share: f32,
}

fn d_science() -> f32 {
    0.05
}

/// A technique of the era (`[[tech]]`): what a country knows or does not. Not a tree to click
/// through: countries come to know techniques over the world's past and the game, by their
/// scientists, by trade and wars with those who know them; a country without one lives without it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTech {
    id: String,
    name: String,
    #[serde(default)]
    about: String,
    #[serde(default = "d_tech_icon")]
    icon: String,
    /// The stage of development it belongs to: 0 — every country of the era knows it at the
    /// beginning of the world's past; the higher, the later.
    #[serde(default)]
    tier: usize,
    /// "economy", "military", "science", "state".
    #[serde(default = "d_sphere")]
    sphere: String,
    #[serde(default)]
    needs: Vec<String>,
    /// Years of an ordinary country's science it takes (by its tier if not given).
    #[serde(default)]
    cost: Option<f32>,
    /// What it adds to the productivity of sectors (a share: 0.08 is +8%).
    #[serde(default)]
    sectors: BTreeMap<String, f32>,
    /// What it adds to the fire and toughness of kinds of units (a share).
    #[serde(default)]
    units: BTreeMap<String, f32>,
    /// Kinds of units nobody can field without it.
    #[serde(default)]
    unlocks: Vec<String>,
    /// What it adds to the power of science, to how fast factories learn models, to the
    /// government's stability a year (shares).
    #[serde(default)]
    science: f32,
    #[serde(default)]
    learn: f32,
    #[serde(default)]
    stability: f32,
    /// What it adds to how many people the state can draft, to the supply troops get (shares).
    #[serde(default)]
    draft: f32,
    #[serde(default)]
    supply: f32,
}

fn d_tech_icon() -> String {
    "science".into()
}

fn d_sphere() -> String {
    "economy".into()
}

/// The stages of development of the era, their names from the least to the most developed.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawProgress {
    #[serde(default)]
    stages: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWar {
    #[serde(default = "d_width")]
    width: f32,
    #[serde(default = "d_days")]
    days: u32,
    #[serde(default = "one")]
    man_cost: f32,
    #[serde(default = "one")]
    scale: f32,
    #[serde(default = "one")]
    crew: f32,
    #[serde(default)]
    frontage: BTreeMap<String, f32>,
    #[serde(default)]
    cover: BTreeMap<String, f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUnit {
    id: String,
    name: String,
    class: String,
    #[serde(default = "d_line")]
    line: String,
    #[serde(default = "one")]
    men: f32,
    #[serde(default)]
    equip: BTreeMap<String, f32>,
    #[serde(default)]
    supply: BTreeMap<String, f32>,
    fire: BTreeMap<String, f32>,
    #[serde(default = "one")]
    toughness: f32,
    #[serde(default = "one")]
    cover: f32,
    #[serde(default = "one")]
    speed: f32,
    #[serde(default = "one")]
    width: f32,
    #[serde(default)]
    terrain: BTreeMap<String, f32>,
    #[serde(default = "d_icon")]
    icon: String,
    #[serde(default = "d_symbol")]
    symbol: String,
    #[serde(default)]
    norms: bool,
    #[serde(default)]
    model: String,
    #[serde(default)]
    armor: f32,
    #[serde(default)]
    pierce: f32,
    #[serde(default = "one")]
    attack: f32,
    #[serde(default = "one")]
    defend: f32,
    #[serde(default = "one")]
    complexity: f32,
}

/// A company of support a division may have (one to a division, as in HOI4): its men and goods,
/// and what it gives the whole division (each figure neutral when left out).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSupport {
    id: String,
    name: String,
    #[serde(default)]
    about: String,
    #[serde(default = "d_support_icon")]
    icon: String,
    #[serde(default)]
    men: f32,
    #[serde(default)]
    equip: BTreeMap<String, f32>,
    #[serde(default)]
    supply: BTreeMap<String, f32>,
    #[serde(default = "one")]
    dig: f32,
    #[serde(default = "one")]
    rough: f32,
    #[serde(default)]
    see: f32,
    #[serde(default = "one")]
    org: f32,
    #[serde(default)]
    saved: f32,
    #[serde(default = "one")]
    eat: f32,
    #[serde(default = "one")]
    fire: f32,
    #[serde(default)]
    salvage: f32,
}

fn d_support_icon() -> String {
    "gears".into()
}

/// A kind of warship (`[[ship]]`): what it costs to build and keep, how it fights.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawShip {
    id: String,
    name: String,
    #[serde(default)]
    about: String,
    #[serde(default = "d_ship_icon")]
    icon: String,
    role: String,
    equip: BTreeMap<String, f32>,
    #[serde(default)]
    supply: BTreeMap<String, f32>,
    months: u32,
    crew: f32,
    #[serde(default)]
    gun: f32,
    #[serde(default)]
    armor: f32,
    hull: f32,
    #[serde(default)]
    torpedo: f32,
    #[serde(default)]
    asw: f32,
    #[serde(default)]
    stealth: f32,
    #[serde(default = "one")]
    speed: f32,
    #[serde(default)]
    raid: f32,
    #[serde(default)]
    escort: f32,
}

fn d_ship_icon() -> String {
    "anchor".into()
}

fn parse_ships(raw: &RawPack, gi: &HashMap<String, usize>, goods: &[Good]) -> Result<Vec<Ship>, String> {
    if raw.ships.is_empty() {
        return Ok(Vec::new());
    }
    if raw.war.is_none() {
        return Err("есть [[ship]], но нет раздела [war]".into());
    }
    index(raw.ships.iter().map(|u| &u.id), "корабль")?;
    let mut out = Vec::new();
    for s in &raw.ships {
        let place = format!("корабль «{}»", s.id);
        if !ROLES.iter().any(|r| r.0 == s.role) {
            let known: Vec<&str> = ROLES.iter().map(|r| r.0).collect();
            return Err(format!("{place}: неизвестное место в флоте «{}» (есть: {})", s.role, known.join(", ")));
        }
        let goods_of = |m: &BTreeMap<String, f32>| m.iter().map(|(k, &v)| Ok((look(gi, k, "товар", &place)?, v))).collect::<Result<Vec<_>, String>>();
        let equip = goods_of(&s.equip)?;
        let supply = goods_of(&s.supply)?;
        if equip.is_empty() {
            return Err(format!("{place}: equip пуст — кораблю не из чего строиться"));
        }
        if s.months == 0 || s.hull <= 0.0 || s.crew <= 0.0 {
            return Err(format!("{place}: months, hull и crew должны быть больше нуля"));
        }
        if !(0.0..1.0).contains(&s.stealth) {
            return Err(format!("{place}: stealth = {} — доля, от 0 до 1", s.stealth));
        }
        let worth = |v: &[(usize, f32)]| v.iter().map(|&(g, a)| a * goods[g].price).sum::<f32>();
        out.push(Ship {
            id: s.id.clone(),
            name: s.name.clone(),
            about: s.about.clone(),
            icon: s.icon.clone(),
            role: s.role.clone(),
            cost: worth(&equip) + 6.0 * worth(&supply),
            equip,
            supply,
            months: s.months,
            crew: s.crew,
            gun: s.gun,
            armor: s.armor,
            hull: s.hull,
            torpedo: s.torpedo,
            asw: s.asw,
            stealth: s.stealth,
            speed: s.speed,
            raid: s.raid,
            escort: s.escort,
        });
    }
    Ok(out)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTrade {
    owner: String,
    #[serde(default = "d_margin")]
    margin: f32,
    sea: f32,
    land: f32,
    #[serde(default)]
    port: f32,
    sea_speed: f32,
    land_speed: f32,
    #[serde(default)]
    terrain: BTreeMap<String, f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGov {
    #[serde(default = "d_tax")]
    tax: f32,
    #[serde(default = "d_military")]
    military: f32,
}

impl Default for RawGov {
    fn default() -> Self {
        RawGov { tax: d_tax(), military: d_military() }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGood {
    id: String,
    name: String,
    #[serde(default = "one")]
    price: f32,
    #[serde(default)]
    military: bool,
    #[serde(default = "one")]
    bulk: f32,
    #[serde(default = "yes")]
    tradable: bool,
    #[serde(default = "d_good_icon")]
    icon: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGroup {
    id: String,
    name: String,
    share: f32,
    #[serde(default = "d_patience")]
    patience: f32,
    #[serde(default)]
    needs: Vec<RawNeed>,
    #[serde(default)]
    becomes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNeed {
    good: String,
    amount: f32,
    #[serde(default = "one")]
    weight: f32,
    #[serde(default)]
    vital: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBoost {
    amount: f32,
    gain: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSector {
    id: String,
    name: String,
    workers: BTreeMap<String, f32>,
    #[serde(default)]
    inputs: BTreeMap<String, f32>,
    #[serde(default)]
    boost: BTreeMap<String, RawBoost>,
    outputs: BTreeMap<String, f32>,
    owner: String,
    #[serde(default = "d_wage")]
    wage_share: f32,
    #[serde(default)]
    start: f32,
    #[serde(default)]
    land: bool,
    #[serde(default)]
    deposit: Option<String>,
    #[serde(default)]
    per_deposit: f32,
    #[serde(default)]
    build: BTreeMap<String, f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResource {
    id: String,
    name: String,
    chance: BTreeMap<String, f32>,
}

/// How much harder than plains a terrain is to cross, unless the pack says otherwise.
fn d_terrain(t: Terrain) -> f32 {
    match t {
        Terrain::Ocean | Terrain::Lake | Terrain::Plains => 1.0,
        Terrain::Forest => 1.5,
        Terrain::Hills | Terrain::Desert | Terrain::Tundra => 2.0,
        Terrain::Marsh | Terrain::Jungle => 3.0,
        Terrain::Mountains => 4.0,
        Terrain::Arctic => 8.0,
    }
}

/// How much of the width of a plain the front has on each terrain, unless the pack says otherwise.
fn d_frontage(t: Terrain) -> f32 {
    match t {
        Terrain::Ocean | Terrain::Lake | Terrain::Arctic => 0.3,
        Terrain::Plains | Terrain::Desert => 1.0,
        Terrain::Tundra => 0.8,
        Terrain::Forest | Terrain::Hills => 0.7,
        Terrain::Marsh | Terrain::Jungle => 0.5,
        Terrain::Mountains => 0.4,
    }
}

/// Share of harm the ground keeps off a defender, unless the pack says otherwise.
fn d_cover(t: Terrain) -> f32 {
    match t {
        Terrain::Ocean | Terrain::Lake => 0.0,
        Terrain::Plains | Terrain::Desert => 0.1,
        Terrain::Tundra | Terrain::Arctic => 0.15,
        Terrain::Forest => 0.35,
        Terrain::Hills | Terrain::Marsh => 0.4,
        Terrain::Jungle => 0.45,
        Terrain::Mountains => 0.6,
    }
}

fn terrain_index(t: &str, place: &str) -> Result<usize, String> {
    TERRAINS.iter().position(|x| x.name() == t).ok_or_else(|| {
        let names: Vec<&str> = TERRAINS.iter().map(|x: &Terrain| x.name()).collect();
        format!("{place}: неизвестная местность «{t}» (есть: {})", names.join(", "))
    })
}

pub(super) fn index<'a>(ids: impl Iterator<Item = &'a String>, kind: &str) -> Result<HashMap<String, usize>, String> {
    let mut m = HashMap::new();
    for (i, id) in ids.enumerate() {
        if m.insert(id.clone(), i).is_some() {
            return Err(format!("{kind} «{id}» объявлен дважды"));
        }
    }
    Ok(m)
}

pub(super) fn look(m: &HashMap<String, usize>, id: &str, what: &str, place: &str) -> Result<usize, String> {
    m.get(id).copied().ok_or_else(|| {
        let mut known: Vec<&String> = m.keys().collect();
        known.sort();
        format!("{place}: неизвестный {what} «{id}» (есть: {})", known.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "))
    })
}

fn parse_support(raw: &RawPack, gi: &HashMap<String, usize>, goods: &[Good]) -> Result<Vec<Support>, String> {
    if raw.support.is_empty() {
        return Ok(Vec::new());
    }
    let Some(w) = &raw.war else {
        return Err("есть [[support]], но нет раздела [war]".into());
    };
    index(raw.support.iter().map(|u| &u.id), "рота поддержки")?;
    let mut out = Vec::new();
    for s in &raw.support {
        let place = format!("рота поддержки «{}»", s.id);
        let goods_of = |m: &BTreeMap<String, f32>| m.iter().map(|(k, &v)| Ok((look(gi, k, "товар", &place)?, v))).collect::<Result<Vec<_>, String>>();
        let equip = goods_of(&s.equip)?;
        let supply = goods_of(&s.supply)?;
        for (what, v) in [("dig", s.dig), ("rough", s.rough), ("org", s.org), ("eat", s.eat), ("fire", s.fire)] {
            if v <= 0.0 {
                return Err(format!("{place}: {what} должно быть больше нуля"));
            }
        }
        for (what, v) in [("saved", s.saved), ("salvage", s.salvage)] {
            if !(0.0..1.0).contains(&v) {
                return Err(format!("{place}: {what} = {v} — доля, от 0 до 1"));
            }
        }
        let worth = |v: &[(usize, f32)]| v.iter().map(|&(g, a)| a * goods[g].price).sum::<f32>();
        out.push(Support {
            id: s.id.clone(),
            name: s.name.clone(),
            about: s.about.clone(),
            icon: s.icon.clone(),
            men: s.men,
            cost: worth(&equip) + 6.0 * worth(&supply) + s.men * w.man_cost,
            equip,
            supply,
            dig: s.dig,
            rough: s.rough,
            see: s.see,
            org: s.org,
            saved: s.saved,
            eat: s.eat,
            fire: s.fire,
            salvage: s.salvage,
        });
    }
    Ok(out)
}

fn parse_war(raw: &RawPack, gi: &HashMap<String, usize>, goods: &[Good]) -> Result<(Option<War>, Vec<Unit>), String> {
    let Some(w) = &raw.war else {
        if !raw.units.is_empty() {
            return Err("есть [[unit]], но нет раздела [war]".into());
        }
        return Ok((None, Vec::new()));
    };
    if raw.units.is_empty() {
        return Err("раздел [war] есть, а ни одного [[unit]] нет".into());
    }
    index(raw.units.iter().map(|u| &u.id), "род войск")?;
    // classes of targets are what units say they are
    let mut classes: Vec<String> = Vec::new();
    for u in &raw.units {
        if !classes.contains(&u.class) {
            classes.push(u.class.clone());
        }
    }
    let mut frontage = TERRAINS.map(d_frontage);
    for (t, &v) in &w.frontage {
        frontage[terrain_index(t, "война, frontage")?] = v;
    }
    let mut cover = TERRAINS.map(d_cover);
    for (t, &v) in &w.cover {
        if !(0.0..1.0).contains(&v) {
            return Err(format!("война, cover: укрытие «{t}» = {v}, а должно быть от 0 до 1"));
        }
        cover[terrain_index(t, "война, cover")?] = v;
    }
    let mut units = Vec::new();
    for u in &raw.units {
        let place = format!("род войск «{}»", u.id);
        let goods_of = |m: &BTreeMap<String, f32>| {
            m.iter().map(|(k, &v)| Ok((look(gi, k, "товар", &place)?, v))).collect::<Result<Vec<_>, String>>()
        };
        let line = match u.line.as_str() {
            "front" => Line::Front,
            "support" => Line::Support,
            "air" => Line::Air,
            other => return Err(format!("{place}: line = «{other}», а бывает front, support или air")),
        };
        let mut fire = vec![0.0; classes.len()];
        for (c, &v) in &u.fire {
            let k = classes.iter().position(|x| x == c).ok_or_else(|| {
                format!("{place}: огонь по «{c}», но такого класса ни у кого нет (есть: {})", classes.join(", "))
            })?;
            fire[k] = v;
        }
        if u.width <= 0.0 {
            return Err(format!("{place}: width должна быть больше нуля"));
        }
        if u.toughness <= 0.0 {
            return Err(format!("{place}: toughness должна быть больше нуля"));
        }
        if !SYMBOLS.contains(&u.symbol.as_str()) {
            return Err(format!("{place}: неизвестный знак symbol = «{}» (есть: {})", u.symbol, SYMBOLS.join(", ")));
        }
        let mut terrain = [1.0; 11];
        for (t, &v) in &u.terrain {
            terrain[terrain_index(t, &place)?] = v;
        }
        let equip = goods_of(&u.equip)?;
        let supply = goods_of(&u.supply)?;
        let worth = |v: &[(usize, f32)]| v.iter().map(|&(g, a)| a * goods[g].price).sum::<f32>();
        units.push(Unit {
            id: u.id.clone(),
            name: u.name.clone(),
            class: classes.iter().position(|x| *x == u.class).unwrap(),
            line,
            men: u.men,
            cost: worth(&equip) + 6.0 * worth(&supply) + u.men * w.man_cost,
            equip,
            supply,
            fire,
            toughness: u.toughness,
            cover: u.cover,
            speed: u.speed,
            width: u.width,
            terrain,
            icon: u.icon.clone(),
            symbol: u.symbol.clone(),
            norms: u.norms,
            model: if u.model.is_empty() { u.name.clone() } else { u.model.clone() },
            armor: u.armor,
            pierce: u.pierce,
            attack: u.attack,
            defend: u.defend,
            complexity: u.complexity.max(0.1),
        });
    }
    if !units.iter().any(|u| u.line == Line::Front) {
        return Err("ни один род войск не держит линию фронта (line = \"front\")".into());
    }
    Ok((Some(War { classes, width: w.width, frontage, cover, days: w.days, man_cost: w.man_cost, scale: w.scale, crew: w.crew }), units))
}

/// The pack's techniques, checked: every name it uses must exist, a technique may need only those
/// of its tier or lower, none may need itself.
fn parse_techs(raw: &RawPack, units: &[Unit]) -> Result<Vec<Tech>, String> {
    let ti = index(raw.techs.iter().map(|t| &t.id), "техника")?;
    let si = index(raw.sectors.iter().map(|s| &s.id), "отрасль")?;
    let ui: HashMap<String, usize> = units.iter().enumerate().map(|(i, u)| (u.id.clone(), i)).collect();
    let mut out = Vec::new();
    for t in &raw.techs {
        let place = format!("техника «{}»", t.id);
        let sphere = match t.sphere.as_str() {
            "economy" => Sphere::Economy,
            "military" => Sphere::Military,
            "science" => Sphere::Science,
            "state" => Sphere::State,
            x => return Err(format!("{place}: неизвестная сфера «{x}» (есть: economy, military, science, state)")),
        };
        if t.tier >= TIER_COST.len() {
            return Err(format!("{place}: tier {} — не больше {}", t.tier, TIER_COST.len() - 1));
        }
        let needs = t.needs.iter().map(|n| look(&ti, n, "техника", &place)).collect::<Result<Vec<_>, String>>()?;
        for &n in &needs {
            if raw.techs[n].id == t.id {
                return Err(format!("{place}: нужна сама себе"));
            }
            if raw.techs[n].tier > t.tier {
                return Err(format!("{place}: нужна «{}» более позднего яруса", raw.techs[n].id));
            }
        }
        let log = |x: f32, what: &str| -> Result<f32, String> {
            if x <= -0.9 {
                Err(format!("{place}: {what} {x} — не меньше −0,9"))
            } else {
                Ok((1.0 + x).ln())
            }
        };
        let sectors = t.sectors.iter().map(|(k, &x)| Ok((look(&si, k, "отрасль", &place)?, log(x, "отрасль")?))).collect::<Result<Vec<_>, String>>()?;
        let unit_gains = t.units.iter().map(|(k, &x)| Ok((look(&ui, k, "род войск", &place)?, log(x, "род войск")?))).collect::<Result<Vec<_>, String>>()?;
        let unlocks = t.unlocks.iter().map(|k| look(&ui, k, "род войск", &place)).collect::<Result<Vec<_>, String>>()?;
        out.push(Tech {
            id: t.id.clone(),
            name: t.name.clone(),
            about: t.about.clone(),
            icon: t.icon.clone(),
            tier: t.tier,
            sphere,
            needs,
            cost: t.cost.unwrap_or(TIER_COST[t.tier]).max(0.0),
            sectors,
            units: unit_gains,
            unlocks,
            science: t.science,
            learn: t.learn,
            stability: t.stability,
            draft: t.draft,
            supply: t.supply,
        });
    }
    Ok(out)
}

/// Techniques for a pack that names none: for each sector, two better ways of working it; for each
/// kind of units, two better designs; schools and a science of its own.
fn generic_techs(raw: &RawPack, units: &[Unit]) -> Vec<Tech> {
    let blank = |id: String, name: String, tier: usize, sphere: Sphere| Tech {
        id,
        name,
        about: String::new(),
        icon: "science".into(),
        tier,
        sphere,
        needs: Vec::new(),
        cost: TIER_COST[tier],
        sectors: Vec::new(),
        units: Vec::new(),
        unlocks: Vec::new(),
        science: 0.0,
        learn: 0.0,
        stability: 0.0,
        draft: 0.0,
        supply: 0.0,
    };
    let mut out = Vec::new();
    let mut school = blank("schools".into(), "Школы и учёные общества".into(), 1, Sphere::Science);
    school.science = 0.1;
    school.icon = "graduate".into();
    out.push(school);
    for (k, s) in raw.sectors.iter().enumerate() {
        for tier in 1..=2 {
            let mut t = blank(format!("{}_{tier}", s.id), format!("{}: {}", s.name, if tier == 1 { "новые приёмы" } else { "новая техника" }), tier, Sphere::Economy);
            t.sectors.push((k, 0.07f32.ln_1p()));
            t.icon = "gears".into();
            if tier == 2 {
                t.needs.push(out.len() - 1);
            }
            out.push(t);
        }
    }
    for (u, unit) in units.iter().enumerate() {
        for tier in 1..=2 {
            let mut t = blank(format!("{}_{tier}", unit.id), format!("{}: {}", unit.name, if tier == 1 { "новые приёмы" } else { "новое поколение" }), tier + 1, Sphere::Military);
            t.units.push((u, 0.08f32.ln_1p()));
            t.icon = unit.icon.clone();
            if tier == 2 {
                t.needs.push(out.len() - 1);
            }
            out.push(t);
        }
    }
    out
}

impl Pack {
    pub fn parse(text: &str) -> Result<Pack, String> {
        let raw: RawPack = toml::from_str(text).map_err(|e| e.to_string())?;
        let gi = index(raw.goods.iter().map(|g| &g.id), "товар")?;
        let pi = index(raw.groups.iter().map(|g| &g.id), "группа")?;
        let ri = index(raw.resources.iter().map(|r| &r.id), "ресурс")?;
        index(raw.sectors.iter().map(|s| &s.id), "отрасль")?;

        let share: f32 = raw.groups.iter().map(|g| g.share).sum();
        if (share - 1.0).abs() > 0.02 {
            return Err(format!("доли групп в сумме {share:.3}, а должно быть 1"));
        }

        let goods: Vec<Good> = raw
            .goods
            .iter()
            .map(|g| Good {
                id: g.id.clone(),
                name: g.name.clone(),
                price: g.price,
                military: g.military,
                bulk: g.bulk,
                tradable: g.tradable,
                icon: g.icon.clone(),
            })
            .collect();

        let mut groups = Vec::new();
        for g in &raw.groups {
            let place = format!("группа «{}»", g.id);
            let needs = g
                .needs
                .iter()
                .map(|n| Ok(Need { good: look(&gi, &n.good, "товар", &place)?, amount: n.amount, weight: n.weight, vital: n.vital }))
                .collect::<Result<Vec<_>, String>>()?;
            for (k, n) in needs.iter().enumerate() {
                if needs[..k].iter().any(|m: &Need| m.good == n.good) {
                    return Err(format!("{place}: товар «{}» в нуждах дважды", g.needs[k].good));
                }
            }
            let becomes = g.becomes.iter().map(|b| look(&pi, b, "группа", &place)).collect::<Result<Vec<_>, String>>()?;
            groups.push(Group {
                id: g.id.clone(),
                name: g.name.clone(),
                share: g.share,
                patience: g.patience,
                needs,
                becomes,
            });
        }

        let mut sectors = Vec::new();
        for s in &raw.sectors {
            let place = format!("отрасль «{}»", s.id);
            let goods_of = |m: &BTreeMap<String, f32>| {
                m.iter()
                    .map(|(k, &v)| Ok((look(&gi, k, "товар", &place)?, v)))
                    .collect::<Result<Vec<_>, String>>()
            };
            let workers = s
                .workers
                .iter()
                .map(|(k, &v)| Ok((look(&pi, k, "группа", &place)?, v)))
                .collect::<Result<Vec<_>, String>>()?;
            let wsum: f32 = workers.iter().map(|w| w.1).sum();
            if (wsum - 1.0).abs() > 0.02 {
                return Err(format!("{place}: доли работников в сумме {wsum:.3}, а должно быть 1"));
            }
            let boost = s
                .boost
                .iter()
                .map(|(k, b)| Ok((look(&gi, k, "товар", &place)?, b.amount, b.gain)))
                .collect::<Result<Vec<_>, String>>()?;
            let deposit = match &s.deposit {
                Some(d) => Some(look(&ri, d, "ресурс", &place)?),
                None => None,
            };
            if deposit.is_some() && s.per_deposit <= 0.0 {
                return Err(format!("{place}: есть deposit, но per_deposit не задан"));
            }
            sectors.push(Sector {
                id: s.id.clone(),
                name: s.name.clone(),
                workers,
                inputs: goods_of(&s.inputs)?,
                boost,
                outputs: goods_of(&s.outputs)?,
                owner: look(&pi, &s.owner, "группа", &place)?,
                wage_share: s.wage_share,
                start: s.start,
                land: s.land,
                deposit,
                per_deposit: s.per_deposit,
                build: goods_of(&s.build)?,
            });
        }

        let mut resources = Vec::new();
        for r in &raw.resources {
            let mut chance = [0.0; 11];
            for (t, &c) in &r.chance {
                chance[terrain_index(t, &format!("ресурс «{}»", r.id))?] = c;
            }
            resources.push(Resource { id: r.id.clone(), name: r.name.clone(), chance });
        }

        let trade = match &raw.trade {
            None => None,
            Some(t) => {
                let mut terrain = TERRAINS.map(d_terrain);
                for (name, &k) in &t.terrain {
                    terrain[terrain_index(name, "торговля")?] = k;
                }
                if t.sea_speed <= 0.0 || t.land_speed <= 0.0 {
                    return Err("торговля: скорости sea_speed и land_speed должны быть больше нуля".into());
                }
                Some(Trade {
                    owner: look(&pi, &t.owner, "группа", "торговля")?,
                    margin: t.margin,
                    sea: t.sea,
                    land: t.land,
                    port: t.port,
                    sea_speed: t.sea_speed,
                    land_speed: t.land_speed,
                    terrain,
                })
            }
        };

        let (war, units) = parse_war(&raw, &gi, &goods)?;
        let support = parse_support(&raw, &gi, &goods)?;
        let ships = parse_ships(&raw, &gi, &goods)?;
        let science = match &raw.science {
            None => None,
            Some(s) => {
                let learned = s.learned.iter().map(|g| look(&pi, g, "группа", "наука")).collect::<Result<Vec<_>, String>>()?;
                if learned.is_empty() {
                    return Err("наука: в learned нет ни одной группы".into());
                }
                if !(0.0..=1.0).contains(&s.share) {
                    return Err(format!("наука: share {} — доля трат казны, от 0 до 1", s.share));
                }
                Some(Science { learned, share: s.share })
            }
        };

        if !THEMES.contains(&raw.ui.theme.as_str()) {
            return Err(format!("[ui] theme: неизвестная тема «{}» (есть: {})", raw.ui.theme, THEMES.join(", ")));
        }
        let techs = if raw.techs.is_empty() { generic_techs(&raw, &units) } else { parse_techs(&raw, &units)? };
        let stages = if raw.progress.stages.is_empty() {
            ["отсталая", "развивающаяся", "развитая", "передовая", "ведущая"].iter().map(|s| s.to_string()).collect()
        } else {
            raw.progress.stages.clone()
        };
        Ok(Pack {
            techs,
            stages,
            theme: raw.ui.theme.clone(),
            name: raw.name,
            about: raw.about,
            tax: raw.government.tax,
            military: raw.government.military,
            goods,
            groups,
            sectors,
            resources,
            trade,
            war,
            units,
            support,
            ships,
            science,
            people: raw.world.people,
        })
    }
}
